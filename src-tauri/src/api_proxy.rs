use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde_json::Value;
use std::str::FromStr;
use tauri::command;

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

    // 构建请求
    let base_url = "https://of-dev-api.bfsea.com/frp/api";
    let full_url = format!("{}/{}", base_url, url);

    let mut request_builder = match method.to_lowercase().as_str() {
        "get" => client.get(&full_url),
        "post" => client.post(&full_url),
        "put" => client.put(&full_url),
        "delete" => client.delete(&full_url),
        _ => return Err("不支持的HTTP方法".into()),
    };

    // 添加统一的 User-Agent
    let os_name = std::env::consts::OS;
    let version = env!("CARGO_PKG_VERSION");
    let user_agent = format!("OpenFrp-CPL/{}-{}", os_name, version);
    request_builder = request_builder.header("User-Agent", &user_agent);

    // 添加请求头
    if let Some(headers_value) = headers {
        // 创建一个新的 HeaderMap
        let mut header_map = HeaderMap::new();

        // 直接处理 JSON 对象，避免生命周期问题
        if let Value::Object(obj) = headers_value {
            for (key, value) in obj {
                if let Value::String(value_str) = value {
                    // 将 key 转换为 HeaderName
                    if let Ok(header_name) = HeaderName::from_str(&key) {
                        if let Ok(header_value) = HeaderValue::from_str(&value_str) {
                            header_map.insert(header_name, header_value);
                        }
                    }
                }
            }
        }

        request_builder = request_builder.headers(header_map);
    }

    // // 添加平台特定的请求头
    // #[cfg(target_os = "macos")]
    // {
    //     request_builder = request_builder.header("User-Agent", "OpenFrp-Launcher/macOS");
    // }

    // 添加请求体
    if let Some(body_value) = body {
        request_builder = request_builder.json(&body_value);
    }

    // 发送请求
    let response = request_builder.send().await.map_err(|e| e.to_string())?;

    // 获取响应头
    let response_headers: std::collections::HashMap<String, String> = response
        .headers()
        .iter()
        .map(|(name, value)| (name.to_string(), value.to_str().unwrap_or("").to_string()))
        .collect();

    // 解析响应
    let response_body = response.json::<Value>().await.map_err(|e| e.to_string())?;

    // 返回包含响应头和响应体的对象
    let result = serde_json::json!({
        "data": response_body,
        "headers": response_headers
    });

    Ok(result)
}
