use crate::api_proxy::{client_builder, OPENFRP_API_BASES};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand_core::{OsRng, RngCore};
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use std::net::TcpListener;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tiny_http::{Header, Method, Response, Server};
use tokio::sync::oneshot;

struct ReservedAttempt {
    id: String,
    _cancel_on_drop: oneshot::Sender<()>,
    pending: Option<oneshot::Receiver<()>>,
}

#[derive(Default)]
pub struct OAuthAttempt(Mutex<Option<ReservedAttempt>>);

impl OAuthAttempt {
    fn begin(&self) -> String {
        let mut random = [0; 24];
        OsRng.fill_bytes(&mut random);
        let id = URL_SAFE_NO_PAD.encode(random);
        let (cancel, pending) = oneshot::channel();
        *self.0.lock().unwrap() = Some(ReservedAttempt {
            id: id.clone(),
            _cancel_on_drop: cancel,
            pending: Some(pending),
        });
        id
    }

    fn claim(&self, id: &str) -> Option<oneshot::Receiver<()>> {
        let mut current = self.0.lock().unwrap();
        current
            .as_mut()
            .filter(|attempt| attempt.id == id)?
            .pending
            .take()
    }

    fn is_current(&self, id: &str) -> bool {
        self.0
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|attempt| attempt.id == id)
    }

    fn finish(&self, id: &str) -> bool {
        let mut current = self.0.lock().unwrap();
        if current.as_ref().is_some_and(|attempt| attempt.id == id) {
            current.take();
            true
        } else {
            false
        }
    }
}

#[derive(Deserialize)]
struct LoginResponse {
    flag: bool,
    msg: Option<String>,
    data: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginAttempt {
    attempt_id: String,
    login_url: Url,
}

#[derive(Serialize)]
pub struct OAuthResponse {
    authorization: String,
    flag: bool,
    msg: String,
}

fn client() -> Result<Client, String> {
    client_builder(12)
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| error.to_string())
}

async fn login_url(client: &Client, redirect: &str) -> Result<(Url, &'static str), String> {
    let mut last_error = "无法获取 NatayarkID 登录地址".to_string();
    for base in OPENFRP_API_BASES {
        let result = async {
            client
                .get(format!("{base}/oauth2/login"))
                .query(&[("redirect_url", redirect)])
                .send()
                .await?
                .error_for_status()?
                .json::<LoginResponse>()
                .await
        }
        .await;
        match result {
            Ok(response) if response.flag => {
                let url = response
                    .data
                    .as_deref()
                    .and_then(|url| Url::parse(url).ok());
                if let Some(url) = url.filter(|url| {
                    url.scheme() == "https"
                        && matches!(url.host_str(), Some("account.naids.com" | "openid.17a.ink"))
                        && url
                            .query_pairs()
                            .any(|(key, value)| key == "redirect_uri" && value == redirect)
                }) {
                    return Ok((url, base));
                }
                last_error = "登录地址或回调地址无效".into();
            }
            Ok(response) => last_error = response.msg.unwrap_or(last_error),
            Err(error) => last_error = format!("获取登录地址失败: {}", error.without_url()),
        }
    }
    Err(last_error)
}

async fn exchange_code(
    client: &Client,
    bases: &[&str],
    code: &str,
    redirect: &str,
) -> Result<OAuthResponse, String> {
    for (index, base) in bases.iter().enumerate() {
        let response = match client
            .post(format!("{base}/oauth2/callback"))
            .query(&[("code", code), ("redirect_url", redirect)])
            .send()
            .await
        {
            Ok(response) => response,
            Err(error) if error.is_connect() && index + 1 < bases.len() => continue,
            Err(error) => return Err(format!("请求登录服务失败: {}", error.without_url())),
        };
        let status = response.status();
        let authorization = response
            .headers()
            .get(reqwest::header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let body = response
            .json::<serde_json::Value>()
            .await
            .map_err(|_| format!("登录服务返回了无效响应 (HTTP {status})"))?;
        let msg = body["msg"].as_str().unwrap_or("登录失败，请稍后重试");
        if !status.is_success() || body["flag"].as_bool() != Some(true) {
            return Err(msg.to_string());
        }
        let authorization = authorization.ok_or("登录失败：未获得授权信息")?;
        return Ok(OAuthResponse {
            authorization,
            flag: true,
            msg: msg.to_string(),
        });
    }
    Err("无法连接登录服务".into())
}

#[tauri::command]
pub async fn oauth_callback(code: String) -> Result<OAuthResponse, String> {
    const LEGACY_REDIRECT: &str = "https://staticassets.naids.com/ofcpl_login";
    exchange_code(&client()?, &OPENFRP_API_BASES, &code, LEGACY_REDIRECT).await
}

#[tauri::command]
pub fn begin_local_oauth_login<R: Runtime>(app: AppHandle<R>) -> String {
    app.state::<OAuthAttempt>().begin()
}

#[tauri::command]
pub async fn start_local_oauth_server<R: Runtime>(
    app: AppHandle<R>,
    attempt_id: String,
) -> Result<LoginAttempt, String> {
    let canceled = app
        .state::<OAuthAttempt>()
        .claim(&attempt_id)
        .ok_or("登录请求已结束")?;
    let result = serve_local_oauth_callback(app.clone(), attempt_id.clone(), canceled).await;
    if result.is_err() {
        app.state::<OAuthAttempt>().finish(&attempt_id);
    }
    result
}

async fn serve_local_oauth_callback<R: Runtime>(
    app: AppHandle<R>,
    id: String,
    mut canceled: oneshot::Receiver<()>,
) -> Result<LoginAttempt, String> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|error| error.to_string())?;
    let path = format!("/oauth_callback/{id}");
    let local = format!(
        "http://{}",
        listener.local_addr().map_err(|error| error.to_string())?
    );
    let redirect = format!("{local}{path}");
    let client = client()?;
    let (login_url, base) = tokio::select! {
        biased;
        _ = &mut canceled => return Err("登录已取消".into()),
        result = login_url(&client, &redirect) => result?,
    };
    let server = Server::from_listener(listener, None).map_err(|error| error.to_string())?;
    let attempt_id = id.clone();

    std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(600);
        loop {
            if !app.state::<OAuthAttempt>().is_current(&id) {
                return;
            }
            if Instant::now() >= deadline {
                finish(&app, &id, Err("等待登录超时，请重新登录".to_string()));
                return;
            }
            let request = match server.recv_timeout(Duration::from_millis(250)) {
                Ok(Some(request)) => request,
                Ok(None) => continue,
                Err(_) => {
                    finish(&app, &id, Err("本地登录回调服务已关闭".to_string()));
                    return;
                }
            };
            let (route, query) = request.url().split_once('?').unwrap_or((request.url(), ""));
            if request.method() != &Method::Get || route != path {
                let _ = request.respond(Response::from_string("Not Found").with_status_code(404));
                continue;
            }
            let mut code = None;
            let mut error = None;
            if let Ok(callback) = Url::parse(&format!("{redirect}?{query}")) {
                for (key, value) in callback.query_pairs() {
                    match key.as_ref() {
                        "code" if code.is_none() => code = Some(value.into_owned()),
                        "error" if error.is_none() => error = Some(value.into_owned()),
                        _ => {}
                    }
                }
            }
            let _ = request.respond(
                Response::from_string(include_str!("../../src/login.html"))
                    .with_header(
                        Header::from_bytes("Content-Type", "text/html; charset=utf-8").unwrap(),
                    )
                    .with_header(Header::from_bytes("Cache-Control", "no-store").unwrap())
                    .with_header(Header::from_bytes("Referrer-Policy", "no-referrer").unwrap()),
            );
            tauri::async_runtime::spawn(async move {
                if !app.state::<OAuthAttempt>().is_current(&id) {
                    return;
                }
                let result = match (error, code.filter(|code| !code.is_empty())) {
                    (Some(error), _) => Err(format!("登录失败：{error}")),
                    (None, Some(code)) => tokio::select! {
                        biased;
                        _ = &mut canceled => return,
                        result = exchange_code(&client, std::slice::from_ref(&base), &code, &redirect) => result,
                    },
                    (None, None) => Err("登录失败：回调地址缺少授权码".into()),
                };
                finish(&app, &id, result);
            });
            return;
        }
    });
    Ok(LoginAttempt {
        attempt_id,
        login_url,
    })
}

fn finish<R: Runtime>(app: &AppHandle<R>, id: &str, result: Result<OAuthResponse, String>) {
    if app.state::<OAuthAttempt>().finish(id) {
        let payload = match result {
            Ok(response) => serde_json::json!({
                "attemptId": id, "flag": true, "authorization": response.authorization, "msg": response.msg,
            }),
            Err(msg) => serde_json::json!({"attemptId": id, "flag": false, "msg": msg}),
        };
        let _ = app.emit_to("main", "oauth-auth", payload);
    }
}

#[tauri::command]
pub fn cancel_local_oauth_server<R: Runtime>(app: AppHandle<R>, attempt_id: String) {
    app.state::<OAuthAttempt>().finish(&attempt_id);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replacing_an_attempt_cancels_it_without_affecting_the_retry() {
        let attempts = OAuthAttempt::default();
        let first = attempts.begin();
        let mut first_canceled = attempts.claim(&first).unwrap();
        assert!(attempts.claim(&first).is_none());

        let retry = attempts.begin();
        assert!(matches!(
            first_canceled.try_recv(),
            Err(oneshot::error::TryRecvError::Closed)
        ));
        assert!(!attempts.finish(&first));
        assert!(attempts.is_current(&retry));
        let mut retry_canceled = attempts.claim(&retry).unwrap();
        assert!(matches!(
            retry_canceled.try_recv(),
            Err(oneshot::error::TryRecvError::Empty)
        ));
        assert!(attempts.finish(&retry));
        assert!(matches!(
            retry_canceled.try_recv(),
            Err(oneshot::error::TryRecvError::Closed)
        ));
    }

    #[test]
    fn a_canceled_reservation_cannot_start() {
        let attempts = OAuthAttempt::default();
        let id = attempts.begin();
        assert!(attempts.finish(&id));
        assert!(attempts.claim(&id).is_none());
    }
}
