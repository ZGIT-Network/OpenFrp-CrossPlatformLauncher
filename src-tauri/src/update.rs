use serde::{Deserialize, Serialize};
use std::error::Error;
use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;

#[derive(Debug, Serialize, Deserialize)]
pub struct CplUpdate {
    pub version: String,
    pub notes: String,
    pub pub_date: String,
    pub platforms: Platforms,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Platforms {
    #[serde(rename = "windows-x86_64")]
    pub windows_x86_64: PlatformInfo,
    #[serde(rename = "darwin-x86_64")]
    pub darwin_x86_64: PlatformInfo,
    #[serde(rename = "linux-x86_64")]
    pub linux_x86_64: PlatformInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlatformInfo {
    pub signature: String,
    pub url: String,
}

pub async fn check_update() -> Result<Option<CplUpdate>, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let response = client
        .get("https://api.openfrp.net/cpl/update")
        .send()
        .await?
        .json::<CplUpdate>()
        .await?;

    Ok(Some(response))
}

pub async fn download_and_install_update(app_handle: AppHandle) -> Result<(), Box<dyn Error>> {
    // 检查更新
    let updater = app_handle.updater()?;
    let update = updater.check().await?;

    if let Some(update) = update {
        // 开始下载并安装更新
        let mut downloaded = 0;
        update
            .download_and_install(
                |chunk_length, content_length| {
                    downloaded += chunk_length;
                    println!("已下载: {downloaded} / {content_length:?} 字节");
                },
                || {
                    println!("更新下载完成");
                },
            )
            .await?;
        println!("更新已安装完成");
        Ok(())
    } else {
        Err("没有可用的更新".into())
    }
}
