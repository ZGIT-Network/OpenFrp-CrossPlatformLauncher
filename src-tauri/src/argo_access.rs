use base64::Engine; // for URL_SAFE_NO_PAD.encode()/decode()
use once_cell::sync::Lazy;
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use tauri::command;

// 纯 Rust：X25519 + XSalsa20-Poly1305
use crypto_box::{PublicKey as BoxPublicKey, SalsaBox, SecretKey as BoxSecretKey};
use rand_core::OsRng;
use x25519_dalek::{PublicKey as X25519Public, StaticSecret as X25519Secret};
use xsalsa20poly1305::{
    aead::{Aead, KeyInit},
    Nonce, XSalsa20Poly1305,
};

// 针对每个 request_uuid 存储一次性密钥对，避免重复使用同一公钥
static REQ_KP_MAP: Lazy<Mutex<HashMap<String, (X25519Secret, X25519Public)>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static CANCEL_SET: Lazy<Mutex<HashSet<String>>> = Lazy::new(|| Mutex::new(HashSet::new()));

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
    let sk = X25519Secret::new(OsRng);
    let pk = X25519Public::from(&sk);
    (sk, pk)
}

fn b64_urlsafe_padded(data: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE.encode(data)
}

fn b64_urlsafe_padded_decode(s: &str) -> Result<Vec<u8>, String> {
    base64::engine::general_purpose::URL_SAFE
        .decode(s)
        .map_err(|e| e.to_string())
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
pub async fn argo_generate_public_key() -> Result<String, String> {
    let (_sk, pk) = generate_keypair();
    Ok(b64_urlsafe_padded(pk.as_bytes()))
}

#[command]
pub async fn argo_request_login() -> Result<RequestLoginData, String> {
    let (sk, pk) = generate_keypair();
    let public_key_b64 = b64_urlsafe_padded(pk.as_bytes());

    let client = reqwest::Client::new();
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
    // 将本次请求的临时密钥对与 request_uuid 关联保存
    REQ_KP_MAP
        .lock()
        .unwrap()
        .insert(data.request_uuid.clone(), (sk, pk));
    Ok(data)
}

#[command]
pub async fn argo_poll_login(request_uuid: String) -> Result<String, String> {
    // 返回明文 Authorization
    let (sk, _pk) = REQ_KP_MAP
        .lock()
        .unwrap()
        .get(&request_uuid)
        .cloned()
        .ok_or_else(|| "未找到对应请求的密钥，请重新发起登录".to_string())?;
    let client = reqwest::Client::new();
    // 按文档使用 GET + query 参数 request_uuid
    let resp = client
        .get("https://access.openfrp.net/argoAccess/pollLogin")
        .query(&[("request_uuid", request_uuid.clone())])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.status().as_u16() == 429 {
        return Err("请求过于频繁 (429)".into());
    }

    // 先抓取响应头，再解析 Body
    let status = resp.status().as_u16();
    let headers: HeaderMap = resp.headers().clone();
    if status == 204 {
        println!("[Argo] 授权未完成 (204)");
        return Err("未授权".into());
    }
    if status == 429 {
        println!("[Argo] 命中阈值限制 (429)");
        return Err("429 Too Many Requests".into());
    }
    if status != 200 {
        let txt = resp.text().await.unwrap_or_default();
        println!("[Argo] 非200状态: {}, 响应文本: {}", status, txt);
        return Err(format!("轮询失败: HTTP {}", status));
    }
    let body: PollLoginResp = resp.json().await.map_err(|e| e.to_string())?;
    if body.code != 200 {
        return Err(format!("轮询失败: {}", body.msg));
    }
    let data = body.data.ok_or_else(|| "响应缺少 data".to_string())?;
    let server_pk_b64 = match headers.get("x-request-public-key") {
        Some(v) => v.to_str().map_err(|e| e.to_string())?.to_string(),
        None => {
            // 调试输出全部响应头，便于定位问题
            println!("[Argo] 缺少服务器公钥，响应头如下：");
            for (k, v) in headers.iter() {
                println!("[Argo] header {}: {}", k, v.to_str().unwrap_or("<bin>"));
            }
            return Err("缺少服务器公钥".into());
        }
    };

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
    match sbox.decrypt(&nonce_bytes.into(), cipher) {
        Ok(plain) => {
            let auth = String::from_utf8_lossy(&plain).to_string();
            REQ_KP_MAP.lock().unwrap().remove(&request_uuid);
            Ok(auth)
        }
        Err(_e) => Err("解密失败".into()),
    }
}

#[command]
pub async fn argo_wait_authorization(request_uuid: String) -> Result<String, String> {
    use tokio::time::{sleep, Duration};
    println!("[Argo] 开始轮询授权，request_uuid={}", request_uuid);
    let mut attempts: u32 = 0;
    let max_attempts: u32 = 60; // 5分钟，每5秒一次
    loop {
        if CANCEL_SET.lock().unwrap().contains(&request_uuid) {
            println!("[Argo] 检测到用户取消，停止轮询");
            REQ_KP_MAP.lock().unwrap().remove(&request_uuid);
            CANCEL_SET.lock().unwrap().remove(&request_uuid);
            return Err("用户已取消".into());
        }
        attempts += 1;
        println!("[Argo] 第 {} 次轮询...", attempts);
        match argo_poll_login(request_uuid.clone()).await {
            Ok(auth) => {
                println!("[Argo] 成功获取授权明文，结束轮询");
                return Ok(auth);
            }
            Err(e) => {
                let msg = e.to_string();
                // 未完成授权或服务端限流等，继续等待
                if msg.contains("429") {
                    println!("[Argo] 命中限流，延迟后重试");
                } else if msg.contains("解密算法未完成") {
                    println!("[Argo] 收到加密数据但后端解密尚未实现，将继续等待...");
                } else if msg.contains("未找到对应请求的密钥") {
                    println!("[Argo] 未找到请求密钥，可能进程已重启或缓存丢失，停止轮询");
                    return Err(msg);
                } else {
                    println!("[Argo] 轮询返回: {}", msg);
                }
            }
        }

        if attempts >= max_attempts {
            println!("[Argo] 轮询超时，结束等待");
            return Err("轮询超时，未完成授权".into());
        }
        sleep(Duration::from_secs(5)).await;
    }
}

#[command]
pub async fn argo_cancel_wait(request_uuid: String) -> Result<(), String> {
    println!("[Argo] 收到取消请求，request_uuid={}", &request_uuid);
    CANCEL_SET.lock().unwrap().insert(request_uuid);
    Ok(())
}
