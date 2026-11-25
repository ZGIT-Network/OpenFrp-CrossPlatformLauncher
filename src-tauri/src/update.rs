use serde::Serialize;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::{Update, UpdaterExt};

#[derive(Serialize)]
pub struct UpdateInfo {
    title: String,
    latest: String,
    msg: String,
}

async fn available_update(app: &AppHandle) -> Result<Option<Update>, String> {
    let bypass = crate::proxy_bypassed();
    let updater = app
        .updater_builder()
        .header("User-Agent", crate::api_proxy::user_agent())
        .map_err(|e| e.to_string())?
        .timeout(Duration::from_secs(15))
        .configure_client(move |client| if bypass { client.no_proxy() } else { client })
        .build()
        .map_err(|e| e.to_string())?;
    updater.check().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn check_update(app: AppHandle) -> Result<Option<UpdateInfo>, String> {
    Ok(available_update(&app).await?.map(|update| UpdateInfo {
        title: "发现新版本".to_string(),
        latest: update.version,
        msg: update.body.unwrap_or_default(),
    }))
}

static INSTALLING: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[tauri::command]
pub async fn download_and_install_update(
    app_handle: AppHandle,
    version: String,
) -> Result<(), String> {
    let _guard = INSTALLING
        .try_lock()
        .map_err(|_| "正在安装更新".to_string())?;
    let mut update = available_update(&app_handle)
        .await?
        .ok_or("没有可用的更新")?;
    if update.version != version {
        return Err("可用版本已变化，请重新检查更新".into());
    }

    update.timeout = Some(Duration::from_secs(300));
    let mut downloaded = 0u64;
    let bytes = update
        .download(
            |chunk, total| {
                downloaded += chunk as u64;
                if let Some(total) = total.filter(|total| *total > 0) {
                    let percent = (100.0 * downloaded as f64 / total as f64).min(100.0);
                    let _ = app_handle.emit("update-progress", percent);
                }
            },
            || {},
        )
        .await
        .map_err(|e| format!("下载或验证更新失败: {e}"))?;
    update
        .install(bytes)
        .map_err(|e| format!("安装更新失败: {e}"))?;
    Ok(())
}
