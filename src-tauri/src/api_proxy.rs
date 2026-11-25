use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Client, ClientBuilder, Method, StatusCode};
use serde_json::Value;
use std::str::FromStr;
use std::sync::OnceLock;
use std::time::Duration;
use tauri::command;

pub const OPENFRP_API_BASES: [&str; 2] = ["https://api.openfrp.net", "https://ofapi.naids.com"];

pub fn user_agent() -> String {
    format!(
        "OpenFrp-CPL/{}-{}",
        std::env::consts::OS,
        env!("CARGO_PKG_VERSION")
    )
}

fn builder(seconds: u64, bypass: bool) -> ClientBuilder {
    let client = Client::builder()
        .user_agent(user_agent())
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(seconds));
    if bypass {
        client.no_proxy()
    } else {
        client
    }
}

pub fn client_builder(seconds: u64) -> ClientBuilder {
    builder(seconds, super::proxy_bypassed())
}

fn api_client() -> Result<&'static Client, String> {
    static DEFAULT: OnceLock<Client> = OnceLock::new();
    static DIRECT: OnceLock<Client> = OnceLock::new();
    let bypass = super::proxy_bypassed();
    let slot = if bypass { &DIRECT } else { &DEFAULT };
    if let Some(client) = slot.get() {
        return Ok(client);
    }
    let client = builder(12, bypass)
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| e.to_string())?;
    Ok(slot.get_or_init(|| client))
}

#[command]
pub async fn proxy_api(
    url: String,
    method: String,
    headers: Option<Value>,
    body: Option<Value>,
) -> Result<Value, String> {
    let path = url.trim_start_matches('/');
    if path.is_empty() || !path.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return Err("无效的 API 路径".into());
    }
    let method = Method::from_str(&method.to_uppercase()).map_err(|e| e.to_string())?;
    if !matches!(
        method,
        Method::GET | Method::POST | Method::PUT | Method::DELETE
    ) {
        return Err("不支持的 HTTP 方法".into());
    }
    // OpenFrp 的部分只读端点使用 POST。只有这些端点可在超时或服务端异常后回退。
    let read_only = method == Method::GET
        || matches!(
            path,
            "getUserInfo"
                | "getUserProxies"
                | "getNodeList"
                | "getNodeStatus"
                | "getNodeConf"
                | "getSignInfo"
                | "getAllGroups"
                | "getTrafficHistory"
                | "refreshProxyStatus"
        );
    let client = api_client()?;
    let mut request_headers = HeaderMap::new();
    if let Some(Value::Object(headers)) = headers {
        for (key, value) in headers {
            if let Some(value) = value.as_str() {
                let name = HeaderName::from_str(&key).map_err(|_| "无效的请求头名称")?;
                let mut value =
                    HeaderValue::from_str(value.trim()).map_err(|_| "无效的请求头值")?;
                value.set_sensitive(name == reqwest::header::AUTHORIZATION);
                request_headers.insert(name, value);
            }
        }
    }

    let mut last_error = "请求 OpenFrp API 失败".to_string();
    for base in OPENFRP_API_BASES {
        let mut request = client
            .request(method.clone(), format!("{base}/frp/api/{path}"))
            .headers(request_headers.clone());
        if method != Method::GET {
            if let Some(body) = &body {
                request = request.json(body);
            }
        }
        let response = match request.send().await {
            Ok(response) => response,
            Err(error) => {
                let is_connect = error.is_connect();
                last_error = format!("请求 {base} 失败: {}", error.without_url());
                if !read_only && !is_connect {
                    return Err(last_error);
                }
                continue;
            }
        };
        let status = response.status();
        if status.is_redirection() || status.is_server_error() {
            last_error = format!("{base} 返回 HTTP {status}");
            if !read_only {
                return Err(last_error);
            }
            continue;
        }
        let authorization = response
            .headers()
            .get(reqwest::header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        match response.json::<Value>().await {
            Ok(data) if data.get("flag").and_then(Value::as_bool).is_some() => {
                return Ok(serde_json::json!({
                    "data": data,
                    "status": status.as_u16(),
                    "headers": authorization.map(|value| serde_json::json!({"authorization": value}))
                        .unwrap_or_else(|| serde_json::json!({})),
                }));
            }
            _ if status == StatusCode::UNAUTHORIZED => {
                return Ok(serde_json::json!({
                    "data": {"flag": false, "msg": "登录凭证无效"},
                    "status": status.as_u16(),
                    "headers": {},
                }));
            }
            _ => {
                last_error = format!("{base} 返回了无效的 API 响应 (HTTP {status})");
                if !read_only {
                    return Err(last_error);
                }
            }
        }
    }
    Err(last_error)
}

#[command]
pub async fn get_announcement() -> Result<Value, String> {
    let client = api_client()?;
    for base in OPENFRP_API_BASES {
        let result = async {
            client
                .get(format!("{base}/commonQuery/get?key=help_info"))
                .send()
                .await?
                .error_for_status()?
                .json::<Value>()
                .await
        }
        .await;
        if let Ok(response) = result {
            if let Some(data) = response.get("data").filter(|data| data.is_object()) {
                return Ok(data.clone());
            }
        }
    }
    Err("获取公告失败，请稍后重试".into())
}
