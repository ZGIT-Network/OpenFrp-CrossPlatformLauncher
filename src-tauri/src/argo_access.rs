use base64::Engine; // for URL_SAFE_NO_PAD.encode()/decode()
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::command;

// 纯 Rust：X25519 + XSalsa20-Poly1305
use crypto_box::{PublicKey as BoxPublicKey, SalsaBox, SecretKey as BoxSecretKey};
use rand_core::OsRng;
use x25519_dalek::{PublicKey as X25519Public, StaticSecret as X25519Secret};
use xsalsa20poly1305::aead::Aead;

const AUTH_WAIT: Duration = Duration::from_secs(300);
struct PendingLogin {
    key: X25519Secret,
    expires_at: Instant,
    waiting: bool,
}
static PENDING_LOGINS: Lazy<Mutex<HashMap<String, PendingLogin>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(Serialize, Deserialize)]
pub struct RequestLoginResp {
    pub code: i32,
    pub msg: String,
    pub data: Option<RequestLoginData>,
}

#[derive(Serialize, Deserialize)]
pub struct RequestLoginData {
    pub authorization_url: String,
    pub request_uuid: String,
}

#[derive(Serialize, Deserialize)]
pub struct PollLoginResp {
    pub code: i32,
    pub msg: String,
    pub data: Option<PollLoginData>,
}

#[derive(Serialize, Deserialize)]
pub struct PollLoginData {
    pub authorization_data: String,
    pub request_uuid: String,
}

fn generate_keypair() -> (X25519Secret, X25519Public) {
    let sk = X25519Secret::random_from_rng(OsRng);
    let pk = X25519Public::from(&sk);
    (sk, pk)
}

fn b64_urlsafe_padded(data: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE.encode(data)
}

fn b64_decode_any(s: &str) -> Result<Vec<u8>, String> {
    // 依次尝试 URL_SAFE(padded), STANDARD(padded), URL_SAFE(no pad), STANDARD(no pad)
    if let Ok(v) = base64::engine::general_purpose::URL_SAFE.decode(s) {
        return Ok(v);
    }
    if let Ok(v) = base64::engine::general_purpose::STANDARD.decode(s) {
        return Ok(v);
    }
    // 手动补齐 padding
    let mut with_pad = s.to_string();
    while with_pad.len() % 4 != 0 {
        with_pad.push('=');
    }
    if let Ok(v) = base64::engine::general_purpose::URL_SAFE.decode(&with_pad) {
        return Ok(v);
    }
    if let Ok(v) = base64::engine::general_purpose::STANDARD.decode(&with_pad) {
        return Ok(v);
    }
    Err("base64 解码失败".into())
}

#[command]
pub async fn argo_request_login() -> Result<RequestLoginData, String> {
    let (sk, pk) = generate_keypair();
    let public_key_b64 = b64_urlsafe_padded(pk.as_bytes());

    let client = crate::api_proxy::client_builder(10)
        .https_only(true)
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .post("https://access.openfrp.net/argoAccess/requestLogin")
        .json(&serde_json::json!({"public_key": public_key_b64}))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.status().as_u16() == 429 {
        return Err("请求过于频繁 (429)".into());
    }

    let body: RequestLoginResp = resp.json().await.map_err(|e| e.to_string())?;
    if body.code != 200 {
        return Err(format!("请求授权失败: {}", body.msg));
    }
    let data = body.data.ok_or_else(|| "响应缺少 data".to_string())?;
    let mut pending = PENDING_LOGINS.lock().unwrap();
    pending.retain(|_, entry| entry.expires_at > Instant::now());
    pending.insert(
        data.request_uuid.clone(),
        PendingLogin {
            key: sk,
            expires_at: Instant::now() + AUTH_WAIT,
            waiting: false,
        },
    );
    Ok(data)
}

async fn poll_login(
    client: &reqwest::Client,
    request_uuid: &str,
    sk: &X25519Secret,
) -> Result<Option<String>, String> {
    let resp = match client
        .get("https://access.openfrp.net/argoAccess/pollLogin")
        .query(&[("request_uuid", request_uuid)])
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) if e.is_connect() || e.is_timeout() => return Ok(None),
        Err(e) => return Err(e.without_url().to_string()),
    };
    let status = resp.status();
    if matches!(status.as_u16(), 204 | 429) || status.is_server_error() {
        return Ok(None);
    }
    if !status.is_success() {
        return Err(format!("轮询失败: HTTP {status}"));
    }

    let server_pk_b64 = resp
        .headers()
        .get("x-request-public-key")
        .and_then(|value| value.to_str().ok())
        .ok_or("缺少服务器公钥")?
        .to_owned();
    let body: PollLoginResp = resp.json().await.map_err(|e| e.without_url().to_string())?;
    if body.code != 200 {
        return Err(format!("轮询失败: {}", body.msg));
    }
    let data = body.data.ok_or("响应缺少 data")?;
    if data.request_uuid != request_uuid {
        return Err("授权请求不匹配".into());
    }
    // 服务器公钥
    let server_pk_bytes = b64_decode_any(&server_pk_b64)?;
    if server_pk_bytes.len() != 32 {
        return Err("服务器公钥长度错误".into());
    }
    let mut pk_arr = [0u8; 32];
    pk_arr.copy_from_slice(&server_pk_bytes);
    let server_pk = X25519Public::from(pk_arr);

    // 解码 authorization_data: 前24字节为 nonce，后面为密文
    let cipher_all = b64_decode_any(&data.authorization_data)?;
    if cipher_all.len() < 24 {
        return Err("密文长度不合法".into());
    }
    let mut nonce_bytes = [0u8; 24];
    nonce_bytes.copy_from_slice(&cipher_all[..24]);
    let cipher = &cipher_all[24..];

    // 使用 NaCl box 兼容实现（Curve25519+XSalsa20-Poly1305）
    let sk_box = BoxSecretKey::from(sk.to_bytes());
    let pk_box = BoxPublicKey::from(server_pk.to_bytes());
    let sbox = SalsaBox::new(&pk_box, &sk_box);
    let plain = sbox
        .decrypt(&nonce_bytes.into(), cipher)
        .map_err(|_| "解密失败")?;
    let auth = String::from_utf8(plain).map_err(|_| "授权信息格式无效")?;
    if auth.trim().is_empty() {
        return Err("授权信息为空".into());
    }
    Ok(Some(auth))
}

#[command]
pub async fn argo_wait_authorization(request_uuid: String) -> Result<String, String> {
    let client = crate::api_proxy::client_builder(10)
        .https_only(true)
        .build()
        .map_err(|e| e.to_string())?;
    let (key, expires_at) = {
        let mut pending = PENDING_LOGINS.lock().unwrap();
        let entry = pending
            .get_mut(&request_uuid)
            .ok_or("授权请求已取消或过期")?;
        if entry.waiting {
            return Err("正在等待本次授权".into());
        }
        entry.waiting = true;
        (entry.key.clone(), entry.expires_at)
    };
    let result = tokio::time::timeout_at(expires_at.into(), async {
        loop {
            if !PENDING_LOGINS.lock().unwrap().contains_key(&request_uuid) {
                return Err("用户已取消".into());
            }
            if let Some(auth) = poll_login(&client, &request_uuid, &key).await? {
                return Ok(auth);
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    })
    .await
    .unwrap_or_else(|_| Err("轮询超时，未完成授权".into()));
    if PENDING_LOGINS
        .lock()
        .unwrap()
        .remove(&request_uuid)
        .is_some()
    {
        result
    } else {
        Err("用户已取消".into())
    }
}

#[command]
pub async fn argo_cancel_wait(request_uuid: String) -> Result<(), String> {
    PENDING_LOGINS.lock().unwrap().remove(&request_uuid);
    Ok(())
}
