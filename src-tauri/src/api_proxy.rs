use reqwest::header::{HeaderMap, HeaderValue, HeaderName};
use serde_json::{json, Value};
use tauri::{command, AppHandle};
use std::str::FromStr;

#[command]
pub async fn proxy_api(
    url: String,
    method: String,
    headers: Option<Value>,
    body: Option<Value>,
) -> Result<Value, String> {
    let client = reqwest::Client::new();
    
    // 构建请求
    let base_url = "https://api.openfrp.net/frp/api";
    let full_url = format!("{}/{}", base_url, url);
    
    let mut request_builder = match method.to_lowercase().as_str() {
        "get" => client.get(&full_url),
        "post" => client.post(&full_url),
        "put" => client.put(&full_url),
        "delete" => client.delete(&full_url),
        _ => return Err("不支持的HTTP方法".into()),
    };
    
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
    
    // 添加请求体
    if let Some(body_value) = body {
        request_builder = request_builder.json(&body_value);
    }
    
    // 发送请求
    let response = request_builder.send().await.map_err(|e| e.to_string())?;
    
    // 解析响应
    let response_body = response.json::<Value>().await.map_err(|e| e.to_string())?;
    
    Ok(response_body)
}