use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde_json::Value;
use std::str::FromStr;
use tauri::command;

/// 当前可用的 OpenFrp API 地址（主地址 + 备用地址）
pub const OPENFRP_API_BASES: [&str; 2] = [
    "https://api.openfrp.net",
    "https://ofapi.naids.com",
];

#[command]
pub async fn proxy_api(
    url: String,
    method: String,
    headers: Option<Value>,
    body: Option<Value>,
) -> Result<Value, String> {
    let client_builder = reqwest::Client::builder();

    // 获取绕过代理设置
    let bypass_proxy = std::env::var("BYPASS_PROXY").unwrap_or_else(|_| "false".to_string());
    let client = if bypass_proxy == "true" {
        // 绕过系统代理 - 清除所有代理环境变量
        client_builder
            .no_proxy()
            .build()
            .map_err(|e| e.to_string())?
    } else {
        // 使用系统代理
        client_builder.build().map_err(|e| e.to_string())?
    };

    // 添加统一的 User-Agent
    let os_name = std::env::consts::OS;
    let version = env!("CARGO_PKG_VERSION");
    let user_agent = format!("OpenFrp-CPL/{}-{}", os_name, version);

    // 构建请求头
    let mut header_map = HeaderMap::new();
    if let Some(Value::Object(obj)) = &headers {
        // 直接处理 JSON 对象，避免生命周期问题
        for (key, value) in obj {
            if let Value::String(value_str) = value {
                // 将 key 转换为 HeaderName（值去掉首尾空白，避免粘贴带入换行导致请求头被丢弃）
                let trimmed = value_str.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if let Ok(header_name) = HeaderName::from_str(key) {
                    if let Ok(header_value) = HeaderValue::from_str(trimmed) {
                        header_map.insert(header_name, header_value);
                    }
                }
            }
        }
    }

    let path = url.trim_start_matches('/');
    let is_get = method.eq_ignore_ascii_case("get");
    let mut last_error = String::from("请求 OpenFrp API 失败");

    // 依次尝试主地址与备用地址，仅在网络/解析失败时回退。
    // 非 GET 请求只在“连接未建立”时回退，避免请求已到达服务端后重试造成重复操作。
    for base in OPENFRP_API_BASES {
        let full_url = format!("{}/frp/api/{}", base, path);

        let mut request_builder = match method.to_lowercase().as_str() {
            "get" => client.get(&full_url),
            "post" => client.post(&full_url),
            "put" => client.put(&full_url),
            "delete" => client.delete(&full_url),
            _ => return Err("不支持的HTTP方法".into()),
        };

        request_builder = request_builder.header("User-Agent", &user_agent);
        request_builder = request_builder.headers(header_map.clone());

        // 添加请求体
        if let Some(body_value) = &body {
            request_builder = request_builder.json(body_value);
        }

        // 发送请求
        let response = match request_builder.send().await {
            Ok(response) => response,
            Err(e) => {
                last_error = format!("请求 {} 失败: {}", base, e);
                if !is_get && !e.is_connect() {
                    return Err(last_error);
                }
                continue;
            }
        };

        // 获取响应头
        let response_headers: std::collections::HashMap<String, String> = response
            .headers()
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_str().unwrap_or("").to_string()))
            .collect();

        // 解析响应
        match response.json::<Value>().await {
            Ok(response_body) => {
                // 返回包含响应头和响应体的对象
                return Ok(serde_json::json!({
                    "data": response_body,
                    "headers": response_headers
                }));
            }
            Err(e) => {
                last_error = format!("解析 {} 的响应失败: {}", base, e);
                if !is_get {
                    return Err(last_error);
                }
            }
        }
    }

    Err(last_error)
}
