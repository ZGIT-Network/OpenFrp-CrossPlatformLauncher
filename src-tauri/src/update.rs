use serde::{Deserialize, Serialize};
use std::error::Error;
use std::sync::atomic::{AtomicUsize, Ordering};
use tauri::AppHandle;
use tauri::Emitter;
use tauri::Listener;
use tauri_plugin_updater::UpdaterExt;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UpdateInfo {
    pub title: String,
    pub latest: String,
    pub msg: String,
}

// 检查更新并返回更新信息
pub async fn check_update() -> Result<Option<UpdateInfo>, Box<dyn Error>> {
    // 获取当前版本
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    println!("当前版本: {}", current_version);
    
    // 使用 Tauri 配置的更新源
    let client = reqwest::Client::new();
    let response = client
        .get("https://api.zyghit.cn/updater/ofcpl")
        .send()
        .await?;
    
    if !response.status().is_success() {
        return Err(format!("API 请求失败: {}", response.status()).into());
    }
    
    let update_data: serde_json::Value = response.json().await?;
    
    // 解析更新信息
    if let Some(version) = update_data.get("version").and_then(|v| v.as_str()) {
        println!("服务器版本: {}", version);
        
        // 比较版本号
        if compare_versions(&current_version, version) {
            let notes = update_data.get("notes").and_then(|n| n.as_str()).unwrap_or("");
            
            // 构建更新信息
            let update_info = UpdateInfo {
                title: "发现新版本".to_string(),
                latest: version.to_string(),
                msg: notes.to_string(),
            };
            
            return Ok(Some(update_info));
        }
    }
    
    // 没有更新或版本相同
    Ok(None)
}

// 版本比较函数：如果远程版本更新则返回 true
fn compare_versions(current: &str, remote: &str) -> bool {
    // 移除可能的 'v' 前缀
    let current = current.trim_start_matches('v');
    let remote = remote.trim_start_matches('v');
    
    // 将版本号分割为组件
    let current_parts: Vec<&str> = current.split('.').collect();
    let remote_parts: Vec<&str> = remote.split('.').collect();
    
    // 比较主要版本号
    for i in 0..std::cmp::min(current_parts.len(), remote_parts.len()) {
        let current_part = current_parts[i].parse::<u32>().unwrap_or(0);
        let remote_part = remote_parts[i].parse::<u32>().unwrap_or(0);
        
        if remote_part > current_part {
            return true;
        } else if remote_part < current_part {
            return false;
        }
    }
    
    // 如果前面的版本号都相同，但远程版本有更多的组件，则认为远程版本更新
    remote_parts.len() > current_parts.len()
}

// 下载并安装更新
pub async fn download_and_install_update(app_handle: AppHandle) -> Result<(), Box<dyn Error>> {
    // 使用 Tauri 的更新器 API
    let updater = app_handle.updater()?;
    
    // 检查更新
    let update = updater.check().await?;
    
    if let Some(update) = update {
        // 设置进度监听器
        let app_handle_clone = app_handle.clone();
        
        // 监听更新状态事件
        app_handle.listen("tauri://update-status", move |event| {
            println!("收到更新状态事件: {}", event.payload());
            
            if let Ok(status) = serde_json::from_str::<serde_json::Value>(event.payload()) {
                // 直接发送原始状态，便于调试
                let _ = app_handle_clone.emit("update-raw-status", event.payload());
                
                // 处理不同状态
                if let Some(status_str) = status.get("status").and_then(|s| s.as_str()) {
                    match status_str {
                        "PENDING" => {
                            let _ = app_handle_clone.emit("update-status", "正在准备更新...");
                        },
                        "DOWNLOADING" => {
                            if let Some(progress) = status.get("progress").and_then(|p| p.as_f64()) {
                                let _ = app_handle_clone.emit("update-progress", format!("{:.1}%", progress * 100.0));
                            } else {
                                let _ = app_handle_clone.emit("update-status", "正在下载更新...");
                            }
                        },
                        "DOWNLOADED" => {
                            let _ = app_handle_clone.emit("update-status", "下载完成，准备安装...");
                        },
                        _ if status_str.contains("ERROR") => {
                            if let Some(error) = status.get("error").and_then(|e| e.as_str()) {
                                let _ = app_handle_clone.emit("update-error", error);
                            } else {
                                let _ = app_handle_clone.emit("update-error", "更新过程中发生未知错误");
                            }
                        },
                        _ => {
                            let _ = app_handle_clone.emit("update-status", format!("更新状态: {}", status_str));
                        }
                    }
                }
            } else {
                println!("无法解析更新状态: {}", event.payload());
            }
        });
        
        // 开始下载并安装更新
        println!("开始下载更新...");
        
        // 使用原子计数器跟踪下载进度
        let downloaded_size = AtomicUsize::new(0);
        let app_handle_for_download = app_handle.clone();
        
        match update.download(
            move |chunk_size, total_size| {
                // 下载进度回调
                let current_size = downloaded_size.fetch_add(chunk_size, Ordering::SeqCst) + chunk_size;
                if let Some(total) = total_size {
                    let progress = (current_size as f64 / total as f64) * 100.0;
                    println!("下载进度: {:.2}% ({} / {})", progress, current_size, total);
                    let _ = app_handle_for_download.emit("update-progress", format!("{:.1}%", progress));
                } else {
                    println!("接收到 {} 字节", chunk_size);
                }
            },
            || {
                // 下载完成回调
                println!("下载完成");
            }
        ).await {
            Ok(bytes) => {
                println!("更新下载完成，准备安装");
                let _ = app_handle.emit("update-status", "下载完成，准备安装...");
                
                // 安装更新
                match update.install(bytes) {
                    Ok(_) => {
                        println!("更新安装成功");
                        let _ = app_handle.emit("update-status", "更新安装成功，即将重启应用...");
                        Ok(())
                    },
                    Err(e) => {
                        println!("更新安装失败: {}", e);
                        let _ = app_handle.emit("update-error", format!("更新安装失败: {}", e));
                        Err(e.into())
                    }
                }
            },
            Err(e) => {
                println!("更新下载失败: {}", e);
                let _ = app_handle.emit("update-error", format!("更新下载失败: {}", e));
                Err(e.into())
            }
        }
    } else {
        Err("没有可用的更新".into())
    }
}
