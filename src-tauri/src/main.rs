// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use flate2::read::GzDecoder;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::fs::File;
use std::io::Cursor;
use std::io::Write;
// use std::io::{BufRead, BufReader};
//use std::path::Path;
// use std::error::Error;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use tar::Archive;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{TrayIcon, TrayIconBuilder};
use tauri::Manager;
mod argo_access;
use tauri::{command, Emitter, Runtime, State};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};

use tauri_plugin_deep_link;
// use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
use tauri_plugin_updater;
use crate::update::UpdateInfo;
use crate::update::download_and_install_update;
use tauri::Listener;
mod api_proxy;
mod update; // 添加这一行

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

// 添加 Windows 特定的常量
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

// 定义进程组结构
#[derive(Default)]
struct FrpcProcesses(Mutex<HashMap<String, ProcessInfo>>);

struct ProcessInfo {
    child: Child,
    #[cfg(target_os = "windows")]
    group_id: u32,
}

#[derive(Serialize, Deserialize)]
struct SoftwareInfo {
    data: SoftwareData,
}

#[derive(Serialize, Deserialize)]
struct SoftwareData {
    latest: String,
    source: Vec<Source>,
}

#[derive(Serialize, Deserialize)]
struct Source {
    label: String,
    value: String,
}

// 添加日志结构体
#[derive(Clone, serde::Serialize)]
struct LogPayload {
    message: String,
}

// 配置文件版本号，用于管理配置文件升级
const CONFIG_VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Default)]
struct Config {
    config_version: Option<u32>, // 配置文件版本号
    frpc_version: Option<String>,
    frpc_filename: Option<String>,
    cpl_version: Option<String>,
}

impl Config {
    fn upgrade(mut self) -> Self {
        let current_version = self.config_version.unwrap_or(0);

        if current_version < 1 {
            // 版本0到版本1的升级
            self.frpc_version = self.frpc_version.or_else(|| Some(String::new()));
            self.frpc_filename = self.frpc_filename.or_else(|| Some(String::new()));
            self.cpl_version = self.cpl_version.or_else(|| Some("0.6.3".to_string()));
        }

        // 更新版本号
        self.config_version = Some(CONFIG_VERSION);
        self
    }
}

#[command]
async fn check_update(_app_handle: tauri::AppHandle) -> Result<Option<UpdateInfo>, String> {
    match crate::update::check_update().await {
        Ok(update) => Ok(update),
        Err(e) => {
            println!("检查更新失败: {}", e);
            Err(e.to_string())
        }
    }
}

#[command]
async fn install_update(app_handle: tauri::AppHandle) -> Result<(), String> {
    match crate::update::download_and_install_update(app_handle).await {
        Ok(_) => Ok(()),
        Err(e) => {
            println!("安装更新失败: {}", e);
            Err(e.to_string())
        }
    }
}

// 添加全局静态变量来存储程序目录
static APP_DIR: Mutex<Option<PathBuf>> = Mutex::new(None);

fn init_app_directory(app: &tauri::App) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let app_local_data_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|e| Box::<dyn std::error::Error>::from(e.to_string()))?;

    // 确保目录存在
    fs::create_dir_all(&app_local_data_dir)?;

    // 创建子目录
    let frpc_dir = app_local_data_dir.join("frpc");
    let config_dir = app_local_data_dir.join("config");
    fs::create_dir_all(&frpc_dir)?;
    fs::create_dir_all(&config_dir)?;

    // 保存目录路径
    *APP_DIR.lock().unwrap() = Some(app_local_data_dir.clone());

    Ok(app_local_data_dir)
}

// 获取程序目录的辅助函数
fn get_app_dir() -> PathBuf {
    APP_DIR
        .lock()
        .unwrap()
        .clone()
        .expect("应用程序目录未初始化")
}

// 获取配置文件路径
fn get_config_path() -> Result<PathBuf, String> {
    let app_dir = get_app_dir();
    Ok(app_dir.join("config.json"))
}

// 加载配置
fn load_config() -> Result<Config, String> {
    let config_path = get_config_path()?;
    let current_version = env!("CARGO_PKG_VERSION").to_string();

    let mut config = if config_path.exists() {
        // 读取现有配置
        let content =
            fs::read_to_string(&config_path).map_err(|e| format!("读取配置文件失败: {}", e))?;

        serde_json::from_str::<Config>(&content)
            .map_err(|e| format!("解析配置文件失败: {}", e))?
            .upgrade() // 升级配置文件结构
    } else {
        // 创建新配置
        Config {
            config_version: Some(CONFIG_VERSION),
            cpl_version: Some(current_version.clone()),
            ..Default::default()
        }
    };

    // 强制更新版本号为当前编译版本
    config.cpl_version = Some(current_version);

    // 保存可能升级后的配置
    save_config(&config)?;

    Ok(config)
}

// 保存配置
fn save_config(config: &Config) -> Result<(), String> {
    let config_path = get_config_path()?;

    // 确保配置目录存在
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建配置目录失败: {}", e))?;
    }

    let content =
        serde_json::to_string_pretty(config).map_err(|e| format!("序列化配置失败: {}", e))?;

    fs::write(&config_path, content).map_err(|e| format!("保存配置文件失败: {}", e))?;

    Ok(())
}

#[command]
async fn download_frpc<R: Runtime>(app: tauri::AppHandle<R>) -> Result<String, String> {
    let os = std::env::consts::OS;
    
    let os_name = os;
    let version = env!("CARGO_PKG_VERSION");
    let user_agent = format!("OpenFrp-CPL/{}-{}", os_name, version);
    let client = reqwest::Client::builder()
        .user_agent(&user_agent)
        .build()
        .map_err(|e| e.to_string())?;

    let os_name = match os {
        "windows" => "windows",
        "linux" => "linux",
        "macos" => "darwin",
        _ => return Err("不支持的操作系统".to_string()),
    };

    let arch_name = match std::env::consts::ARCH {
        "x86" => "386",
        "x86_64" => "amd64",
        "arm" => "arm",
        "aarch64" => "arm64",
        _ => return Err("不支持的系统架构".to_string()),
    };

    // 构建最终的文件名
    let target_filename = if os == "windows" {
        format!("frpc_{}_{}.exe", os_name, arch_name)
    } else {
        format!("frpc_{}_{}", os_name, arch_name)
    };

    app.emit(
        "log",
        LogPayload {
            message: "开始获取最新版本信息...".into(),
        },
    )
    .map_err(|e| e.to_string())?;

    let response = client.get("https://of-dev-api.bfsea.com/commonQuery/get?key=software")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let software_info: SoftwareInfo = response.json().await.map_err(|e| e.to_string())?;

    // 处理版本号，去除两边的斜杠
    let latest_version = software_info.data.latest.trim_matches('/').to_string();

    // 检查版本
    let mut config = load_config()?;
    let app_dir = get_app_dir();
    let target_path = app_dir.join(&target_filename);

    // 先检查文件是否存在
    if target_path.exists() {
        app.emit(
            "log",
            LogPayload {
                message: format!(
                    "当前版本: {}, 最新版本: {}",
                    config.frpc_version.as_ref().map_or("", |s| s.as_str()),
                    latest_version
                ),
            },
        )
        .map_err(|e| e.to_string())?;

        if let Some(current_version) = config.frpc_version.as_ref() {
            if current_version.as_str() == latest_version {
                app.emit(
                    "log",
                    LogPayload {
                        message: "已经是最新版本".into(),
                    },
                )
                .map_err(|e| e.to_string())?;
                return Ok("已经是最新版本".to_string());
            }
        }
    } else {
        app.emit(
            "log",
            LogPayload {
                message: "未检测到 frpc 文件，开始下载...".into(),
            },
        )
        .map_err(|e| e.to_string())?;
    }

    let app_dir = get_app_dir();
    let target_path = app_dir.join(&target_filename);

    // 如果文件已存在，先删除
    if target_path.exists() {
        fs::remove_file(&target_path).map_err(|e| format!("无法删除旧版本: {}", e))?;
    }

    let file_ext = match os {
        "windows" => "zip",
        _ => "tar.gz",
    };

    let download_url = format!(
        "{}{}frpc_{}_{}.{}",
        software_info.data.source[0].value, software_info.data.latest, os_name, arch_name, file_ext
    );

    app.emit(
        "log",
        LogPayload {
            message: format!("开始下载: {}", download_url),
        },
    )
    .map_err(|e| e.to_string())?;

    let response = client.get(&download_url)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let total_size = response.content_length().unwrap_or(0);
    app.emit(
        "log",
        LogPayload {
            message: format!("文件大小: {} bytes", total_size),
        },
    )
    .map_err(|e| e.to_string())?;

    let bytes = response.bytes().await.map_err(|e| e.to_string())?;

    app.emit(
        "log",
        LogPayload {
            message: format!("已下载: {} bytes", bytes.len()),
        },
    )
    .map_err(|e| e.to_string())?;

    if os == "windows" {
        let temp_path = app_dir.join("frpc.zip");
        app.emit(
            "log",
            LogPayload {
                message: format!("临时文件路径: {}", temp_path.display()),
            },
        )
        .map_err(|e| e.to_string())?;

        let mut file = File::create(&temp_path).map_err(|e| e.to_string())?;
        file.write_all(&bytes).map_err(|e| e.to_string())?;
        file.flush().map_err(|e| e.to_string())?;
        drop(file);

        app.emit(
            "log",
            LogPayload {
                message: "开始解压文件...".into(),
            },
        )
        .map_err(|e| e.to_string())?;

        let file = File::open(&temp_path).map_err(|e| format!("无法打开下载的文件: {}", e))?;

        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| format!("无法读取zip文件 (大小: {} bytes): {}", bytes.len(), e))?;

        archive
            .extract(&app_dir)
            .map_err(|e| format!("解压失败: {}", e))?;

        if let Err(e) = std::fs::remove_file(&temp_path) {
            app.emit(
                "log",
                LogPayload {
                    message: format!("警告：无法删除临时文件: {}", e),
                },
            )
            .map_err(|e| e.to_string())?;
        }

        // 移动文件到最终位置
        let extracted_exe = app_dir.join(format!("frpc_{}_{}64/frpc.exe", os_name, arch_name));
        if extracted_exe.exists() {
            fs::rename(&extracted_exe, &target_path)
                .map_err(|e| format!("无法重命名文件: {}", e))?;
        }

        // 清理解压目录
        let extract_dir = app_dir.join(format!("frpc_{}_{}64", os_name, arch_name));
        if extract_dir.exists() {
            fs::remove_dir_all(extract_dir).map_err(|e| format!("无法清理解压目录: {}", e))?;
        }
    } else {
        app.emit(
            "log",
            LogPayload {
                message: "正在解压文件...".into(),
            },
        )
        .map_err(|e| e.to_string())?;

        let cursor = Cursor::new(bytes);
        let gz_decoder = GzDecoder::new(cursor);
        let mut archive = Archive::new(gz_decoder);

        archive.unpack(&app_dir).map_err(|e| e.to_string())?;

        // 移动文件到最终位置
        let extracted_exe = app_dir.join(format!("frpc_{}_{}64/frpc", os_name, arch_name));
        if extracted_exe.exists() {
            fs::rename(&extracted_exe, &target_path)
                .map_err(|e| format!("无法重命名文件: {}", e))?;
        }

        // 清理解压目录
        let extract_dir = app_dir.join(format!("frpc_{}_{}64", os_name, arch_name));
        if extract_dir.exists() {
            fs::remove_dir_all(extract_dir).map_err(|e| format!("无法清理解压目录: {}", e))?;
        }
    }

    // 更新配置
    config.frpc_version = Some(latest_version);
    config.frpc_filename = Some(target_filename);
    save_config(&config)?;

    // 设置文件权限
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        match fs::metadata(&target_path) {
            Ok(metadata) => {
                let mut perms = metadata.permissions();
                perms.set_mode(0o755); // 设置可执行权限
                if let Err(e) = fs::set_permissions(&target_path, perms) {
                    app.emit(
                        "log",
                        LogPayload {
                            message: format!("警告：无法设置文件权限: {}", e),
                        },
                    )
                    .map_err(|e| e.to_string())?;
                } else {
                    app.emit(
                        "log",
                        LogPayload {
                            message: "已设置文件可执行权限".into(),
                        },
                    )
                    .map_err(|e| e.to_string())?;
                }
            }
            Err(e) => {
                app.emit(
                    "log",
                    LogPayload {
                        message: format!("警告：无法获取文件元数据: {}", e),
                    },
                )
                .map_err(|e| e.to_string())?;
            }
        }
    }

    app.emit(
        "log",
        LogPayload {
            message: "下载和安装完成".into(),
        },
    )
    .map_err(|e| e.to_string())?;

    Ok("下载完成".to_string())
}

#[command]
async fn start_frpc_instance<R: Runtime>(
    app: tauri::AppHandle<R>,
    processes: State<'_, FrpcProcesses>,
    id: String,
    token: String,
    tunnel_id: String,
) -> Result<String, String> {
    if let Ok(map) = processes.0.lock() {
        if map.contains_key(&id) {
            return Err("该隧道已经在运行中".to_string());
        }
    }

    let app_dir = get_app_dir();
    let config = load_config()?;
    let frpc_path = app_dir.join(&config.frpc_filename.as_ref().unwrap());

    if !frpc_path.exists() {
        return Err("frpc 程序不存在，请先下载".to_string());
    }

    let mut cmd = Command::new(frpc_path);

    #[cfg(target_os = "windows")]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    cmd.args(&["-u", &token, "-p", &tunnel_id])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    println!("{:?}", cmd);

    // 添加绕过系统代理的环境变量
    let bypass_proxy = std::env::var("BYPASS_PROXY").unwrap_or_else(|_| "false".to_string());
    if bypass_proxy == "true" {
        // 清除所有代理环境变量
        cmd.env("HTTP_PROXY", "");
        cmd.env("HTTPS_PROXY", "");
        cmd.env("http_proxy", "");
        cmd.env("https_proxy", "");
        cmd.env("NO_PROXY", "");
        cmd.env("no_proxy", "");
        cmd.env("ALL_PROXY", "");
        cmd.env("all_proxy", "");
        
        // 在 Windows 上还需要清除系统代理设置
        #[cfg(target_os = "windows")]
        {
            cmd.env("WINHTTP_NO_PROXY", "*");
        }
    }

    let mut child = cmd.spawn().map_err(|e| e.to_string())?;

    // 处理标准输出
    if let Some(stdout) = child.stdout.take() {
        let event_name = format!("frpc-log-{}", id);
        let app_handle = app.clone();

        std::thread::spawn(move || {
            use std::io::{BufRead, BufReader};
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                if let Ok(line) = line {
                    let _ = app_handle.emit(&event_name, LogPayload { message: line });
                }
            }
        });
    }

    // 处理标准错误
    if let Some(stderr) = child.stderr.take() {
        let event_name = format!("frpc-log-{}", id);
        let app_handle = app.clone();

        std::thread::spawn(move || {
            use std::io::{BufRead, BufReader};
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                if let Ok(line) = line {
                    let _ = app_handle.emit(
                        &event_name,
                        LogPayload {
                            message: format!("错误: {}", line),
                        },
                    );
                }
            }
        });
    }

    // 存储进程信息
    if let Ok(mut map) = processes.0.lock() {
        #[cfg(target_os = "windows")]
        {
            let group_id = child.id();
            map.insert(id.clone(), ProcessInfo { child, group_id });
        }
        #[cfg(not(target_os = "windows"))]
        {
            map.insert(id.clone(), ProcessInfo { child });
        }
    }

    Ok("启动成功".to_string())
}

#[command]
fn get_system_info() -> String {
    let arch = std::env::consts::ARCH;
    let os = std::env::consts::OS;
    format!("{}-{}", os, arch)
}

#[command]
fn get_detailed_system_info() -> String {
    let os = std::env::consts::OS;
    
    match os {
        "windows" => get_windows_build_info(), // 使用改进的Windows版本信息函数
        "linux" => get_linux_version(),
        "macos" => get_macos_version(),
        _ => format!("{}-{}", os, std::env::consts::ARCH),
    }
}

fn get_windows_version() -> String {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        
        // 尝试通过systeminfo命令获取Windows版本
        let output = Command::new("cmd")
            .args(&["/C", "systeminfo"])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .output();
            
        if let Ok(output) = output {
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                for line in output_str.lines() {
                    if line.starts_with("OS Name:") {
                        let os_name = line.split(":").nth(1).unwrap_or("").trim();
                        // 尝试获取OS Version
                        if let Some(version_line) = output_str.lines().find(|l| l.starts_with("OS Version:")) {
                            let version = version_line.split(":").nth(1).unwrap_or("").trim();
                            return format!("{} ({})", os_name, version);
                        }
                        return os_name.to_string();
                    }
                }
            }
        }
        
        // 备用方法：通过 wmic 获取更详细的信息
        let output = Command::new("wmic")
            .args(&["os", "get", "Name,Caption,Version,OSArchitecture", "/value"])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .output();
            
        if let Ok(output) = output {
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                let mut caption = "";
                let mut version = "";
                let mut architecture = "";
                
                for line in output_str.lines() {
                    let line = line.trim();
                    if line.starts_with("Caption=") {
                        caption = line.split("=").nth(1).unwrap_or("");
                    } else if line.starts_with("Version=") {
                        version = line.split("=").nth(1).unwrap_or("");
                    } else if line.starts_with("OSArchitecture=") {
                        architecture = line.split("=").nth(1).unwrap_or("");
                    }
                }
                
                if !caption.is_empty() && !version.is_empty() && !architecture.is_empty() {
                    return format!("{} ({}) {}", caption, version, architecture);
                } else if !caption.is_empty() {
                    return caption.to_string();
                }
            }
        }
        
        // 再次备用方法：通过 ver 命令
        let output = Command::new("cmd")
            .args(&["/C", "ver"])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .output();
            
        if let Ok(output) = output {
            if output.status.success() {
                let version_string = String::from_utf8_lossy(&output.stdout);
                return version_string.trim().to_string();
            }
        }
        
        // 最后的备用方法：读取注册表信息
        let output = Command::new("reg")
            .args(&["query", "HKLM\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion", "/v", "ProductName"])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .output();
            
        if let Ok(output) = output {
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                for line in output_str.lines() {
                    if line.contains("ProductName") && line.contains("REG_SZ") {
                        let parts: Vec<&str> = line.split("REG_SZ").collect();
                        if parts.len() > 1 {
                            return parts[1].trim().to_string();
                        }
                    }
                }
            }
        }
        
        "Windows (unknown version)".to_string()
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        "Windows (not available on this platform)".to_string()
    }
}

fn get_windows_build_info() -> String {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        
        // 获取详细的 Windows 版本信息
        let output = Command::new("wmic")
            .args(&["os", "get", "BuildNumber,Version,OSArchitecture,Locale,CurrentBuild", "/value"])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .output();
            
        if let Ok(output) = output {
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                let mut version = "";
                let mut build_number = "";
                let mut architecture = "";
                let mut locale = "";
                
                for line in output_str.lines() {
                    let line = line.trim();
                    if line.starts_with("Version=") {
                        version = line.split("=").nth(1).unwrap_or("");
                    } else if line.starts_with("BuildNumber=") {
                        build_number = line.split("=").nth(1).unwrap_or("");
                    } else if line.starts_with("OSArchitecture=") {
                        architecture = line.split("=").nth(1).unwrap_or("");
                    } else if line.starts_with("Locale=") {
                        locale = line.split("=").nth(1).unwrap_or("");
                    }
                }
                
                // 尝试获取本地化信息
                let locale_str = match locale {
                    "0409" => "en-US",
                    "0804" => "zh-CN",
                    "0404" => "zh-TW",
                    _ => locale,
                };
                
                if !version.is_empty() && !build_number.is_empty() && !architecture.is_empty() {
                    return format!("Microsoft Windows [Version {}.{}]_{}fre_{}", 
                                 version, build_number, 
                                 architecture.to_lowercase().replace("bit", ""),
                                 locale_str);
                }
            }
        }
        
        // 备用方案：通过注册表获取
        let current_version = "HKLM\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion";
        
        // 获取主要版本信息
        let product_name = get_registry_value(current_version, "ProductName");
        let current_build = get_registry_value(current_version, "CurrentBuild");
        let ubr = get_registry_value(current_version, "UBR");
        let arch = std::env::consts::ARCH;
        
        if !product_name.is_empty() && !current_build.is_empty() {
            let ubr_val = if !ubr.is_empty() { 
                format!(".{}", ubr) 
            } else { 
                "".to_string() 
            };
            
            return format!("{} [Version {}.{}{}]_{}fre_{}", 
                          product_name,
                          current_build,
                          current_build,
                          ubr_val,
                          arch,
                          get_windows_locale());
        }
        
        "Windows (unknown build)".to_string()
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        "Windows build info (not available on this platform)".to_string()
    }
}

#[cfg(target_os = "windows")]
fn get_registry_value(key: &str, value_name: &str) -> String {
    use std::process::Command;
    
    let output = Command::new("reg")
        .args(&["query", key, "/v", value_name])
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .output();
        
    if let Ok(output) = output {
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            for line in output_str.lines() {
                if line.contains(value_name) && (line.contains("REG_SZ") || line.contains("REG_DWORD")) {
                    // 根据值类型处理
                    if line.contains("REG_SZ") {
                        let parts: Vec<&str> = line.split("REG_SZ").collect();
                        if parts.len() > 1 {
                            return parts[1].trim().to_string();
                        }
                    } else if line.contains("REG_DWORD") {
                        let parts: Vec<&str> = line.split("REG_DWORD").collect();
                        if parts.len() > 1 {
                            return parts[1].trim().to_string();
                        }
                    }
                }
            }
        }
    }
    
    String::new()
}

#[cfg(target_os = "windows")]
fn get_windows_locale() -> String {
    use std::process::Command;
    
    let output = Command::new("wmic")
        .args(&["os", "get", "Locale", "/value"])
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .output();
        
    if let Ok(output) = output {
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            for line in output_str.lines() {
                let line = line.trim();
                if line.starts_with("Locale=") {
                    let locale = line.split("=").nth(1).unwrap_or("");
                    return match locale {
                        "0409" => "en-US".to_string(),
                        "0804" => "zh-CN".to_string(),
                        "0404" => "zh-TW".to_string(),
                        _ => format!("locale-{}", locale),
                    };
                }
            }
        }
    }
    
    "unknown".to_string()
}

fn get_linux_version() -> String {
    #[cfg(target_os = "linux")]
    {
        use std::fs;
        
        // 尝试读取 /etc/os-release 文件
        if let Ok(content) = fs::read_to_string("/etc/os-release") {
            for line in content.lines() {
                if line.starts_with("PRETTY_NAME=") {
                    if let Some(version) = line.strip_prefix("PRETTY_NAME=") {
                        return version.trim_matches('"').to_string();
                    }
                }
            }
        }
        
        // 尝试读取 /proc/version 文件
        if let Ok(content) = fs::read_to_string("/proc/version") {
            let parts: Vec<&str> = content.split_whitespace().take(3).collect();
            return parts.join(" ");
        }
        
        "Linux (unknown distribution)".to_string()
    }
    
    #[cfg(not(target_os = "linux"))]
    {
        "Linux (not available on this platform)".to_string()
    }
}

fn get_macos_version() -> String {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        
        // 使用 system_profiler 获取 macOS 版本
        let output = Command::new("sw_vers")
            .output();
            
        if let Ok(output) = output {
            if output.status.success() {
                let content = String::from_utf8_lossy(&output.stdout);
                let mut product_name = "macOS";
                let mut version = "";
                
                for line in content.lines() {
                    if line.starts_with("ProductName:") {
                        product_name = line.split(':').nth(1).unwrap_or("macOS").trim();
                    } else if line.starts_with("ProductVersion:") {
                        version = line.split(':').nth(1).unwrap_or("").trim();
                    }
                }
                
                return format!("{} {}", product_name, version);
            }
        }
        
        "macOS (unknown version)".to_string()
    }
    
    #[cfg(not(target_os = "macos"))]
    {
        "macOS (not available on this platform)".to_string()
    }
}

#[command]
fn get_build_info() -> String {
    let build_time = env!("BUILD_TIME", "未知构建时间");
    let _commit_id = env!("GIT_HASH", "未知提交");
    format!("build.{}", build_time)
}

#[command]
async fn tcp_ping(host: String, port: u16) -> Result<serde_json::Value, String> {
    use std::net::{TcpStream, ToSocketAddrs};
    use std::time::{Instant, Duration};
    println!("tcp_ping called with host: {}, port: {}", host, port);

    
    // 解析主机名和端口为SocketAddr
    let addrs = match (host.as_str(), port).to_socket_addrs() {
        Ok(addrs) => addrs,
        Err(e) => {
            return Ok(serde_json::json!({
                "success": false,
                "latency_ms": null,
                "message": format!("地址解析失败: {}", e)
            }));
        }
    };
    
    
    // 取第一个地址进行连接测试
   let addr = match addrs.into_iter().next() {
        Some(addr) => {
            println!("Using address: {:?}", addr);
            addr
        },
        None => {
            println!("No valid address found");
            return Ok(serde_json::json!({
                "success": false,
                "latency_ms": null,
                "message": "未能解析到有效的地址"
            }));
        }
    };
    
    let mut latencies: Vec<u128> = Vec::new();
    let start_time = Instant::now();
    let max_duration = Duration::from_secs(2); // 最多测试2秒钟
    
    // 在1秒钟内进行多次测试
    while start_time.elapsed() < max_duration {
        let start = Instant::now();
        match TcpStream::connect_timeout(&addr, Duration::from_secs(5)) {
            Ok(_) => {
                let duration = start.elapsed();
                latencies.push(duration.as_millis());
                // 短暂休眠以避免过于频繁的连接尝试
                tokio::time::sleep(Duration::from_millis(100)).await;
            },
            Err(e) => {
                println!("Connection failed: {}", e);
                // 即使连接失败也记录时间，但用0表示失败
                latencies.push(0);
                // 短暂休眠
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }
    
    // 计算统计数据
    if latencies.is_empty() {
        return Ok(serde_json::json!({
            "success": false,
            "latency_ms": null,
            "message": "未能执行任何测试"
        }));
    }
    
    // 过滤掉失败的连接(延迟为0的)
    let successful_latencies: Vec<u128> = latencies.iter().filter(|&&x| x > 0).cloned().collect();
    
    if successful_latencies.is_empty() {
        return Ok(serde_json::json!({
            "success": false,
            "latency_ms": 0,
            "message": "所有连接测试均失败",
            "total_tests": latencies.len(),
            "successful_tests": 0
        }));
    }
    
    let total_tests = latencies.len();
    let successful_tests = successful_latencies.len();
    let avg_latency = successful_latencies.iter().sum::<u128>() / successful_tests as u128;
    
    // 计算最小和最大延迟
    let min_latency = *successful_latencies.iter().min().unwrap();
    let max_latency = *successful_latencies.iter().max().unwrap();
    
    println!("Connection successful, average latency: {}ms", avg_latency);
    Ok(serde_json::json!({
        "success": true,
        "latency_ms": avg_latency,
        "message": format!("测试完成，平均延迟 {}ms (最小: {}ms, 最大: {}ms)", avg_latency, min_latency, max_latency),
        "total_tests": total_tests,
        "successful_tests": successful_tests,
        "min_latency": min_latency,
        "max_latency": max_latency
    }))
}

#[command]
async fn stop_frpc_instance<R: Runtime>(
    _app: tauri::AppHandle<R>,
    processes: State<'_, FrpcProcesses>,
    id: String,
) -> Result<(), String> {
    if let Ok(mut map) = processes.0.lock() {
        if let Some(mut process_info) = map.remove(&id) {
            #[cfg(target_os = "windows")]
            {
                let mut cmd = Command::new("taskkill");
                cmd.creation_flags(CREATE_NO_WINDOW);
                let _ = cmd
                    .args(&["/F", "/T", "/PID"])
                    .arg(process_info.group_id.to_string())
                    .output();
            }
            #[cfg(not(target_os = "windows"))]
            {
                let _ = process_info.child.kill();
            }
            return Ok(());
        }
    }
    Err("进程不存在".to_string())
}

#[command]
async fn get_frpc_version() -> Result<String, String> {
    let config = load_config()?;
    Ok(config
        .frpc_version
        .as_ref()
        .unwrap_or(&String::new())
        .clone())
}

// 添加检查 frpc 的函数
#[command]
async fn check_frpc<R: Runtime>(app: tauri::AppHandle<R>) -> Result<bool, String> {
    let app_dir = get_app_dir();
    let config = load_config()?;

    if config.frpc_filename.is_none() {
        app.emit(
            "log",
            LogPayload {
                message: "frpc 文件名为空".into(),
            },
        )
        .map_err(|e| e.to_string())?;
        return Ok(false);
    }

    let frpc_path = app_dir.join(&config.frpc_filename.as_ref().unwrap());
    Ok(frpc_path.exists())
}

// 修改版本获取命令
#[command]
async fn get_frpc_cli_version<R: Runtime>(app: tauri::AppHandle<R>) -> Result<String, String> {
    let app_dir = get_app_dir();
    let mut config = load_config()?;

    // 检查frpc_filename是否存在
    if config.frpc_filename.is_none() {
        // 设置默认文件名
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        
        let filename = if os == "windows" {
            format!("frpc_windows_{}.exe", if arch == "x86_64" { "amd64" } else { arch })
        } else if os == "macos" || os == "darwin" {
            format!("frpc_darwin_{}", if arch == "x86_64" { "amd64" } else if arch == "aarch64" { "arm64" } else { arch })
        } else {
            format!("frpc_linux_{}", if arch == "x86_64" { "amd64" } else if arch == "aarch64" { "arm64" } else { arch })
        };
        
        config.frpc_filename = Some(filename.clone());
        // 保存更新后的配置
        if let Err(e) = save_config(&config) {
            return Err(format!("无法保存配置: {}", e));
        }
        
        // 返回包含路径和文件名信息的JSON对象
        let result = serde_json::json!({
            "version": "未知",
            "path": app_dir.join(&filename).to_string_lossy().to_string(),
            "filename": filename
        });
        
        return Ok(serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string()));
    }
    
    let filename = config.frpc_filename.as_ref().unwrap();
    let frpc_path = app_dir.join(filename);

    if !frpc_path.exists() {
        // 返回包含路径和文件名信息的JSON对象
        let result = serde_json::json!({
            "version": "未知",
            "path": frpc_path.to_string_lossy().to_string(),
            "filename": filename
        });
        
        return Ok(serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string()));
    }

    let mut cmd = Command::new(&frpc_path);
    #[cfg(target_os = "windows")]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let child = match cmd.arg("-v").stdout(Stdio::piped()).spawn() {
        Ok(child) => child,
        Err(e) => {
            // 返回包含路径和文件名信息的JSON对象，但标记为错误
            let result = serde_json::json!({
                "version": format!("错误: {}", e),
                "path": frpc_path.to_string_lossy().to_string(),
                "filename": filename
            });
            
            return Ok(serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string()));
        }
    };

    let output = match child.wait_with_output() {
        Ok(output) => output,
        Err(e) => {
            // 返回包含路径和文件名信息的JSON对象，但标记为错误
            let result = serde_json::json!({
                "version": format!("等待输出错误: {}", e),
                "path": frpc_path.to_string_lossy().to_string(),
                "filename": filename
            });
            
            return Ok(serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string()));
        }
    };

    if output.status.success() {
        let version_str = match String::from_utf8(output.stdout) {
            Ok(s) => s,
            Err(e) => {
                // 返回包含路径和文件名信息的JSON对象，但标记为错误
                let result = serde_json::json!({
                    "version": format!("解析输出错误: {}", e),
                    "path": frpc_path.to_string_lossy().to_string(),
                    "filename": filename
                });
                
                return Ok(serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string()));
            }
        };

        let ver = if let Some(version) = version_str.lines().next() {
            if let Some(v) = version.split_whitespace().last() {
                config.frpc_version = Some(v.to_string());
                save_config(&config)?;
                v.to_string()
            } else {
                "未知版本".to_string()
            }
        } else {
            "未知版本".to_string()
        };

        app.emit(
            "log",
            LogPayload {
                message: format!("检测到本地版本:{}", version_str),
            },
        )
        .map_err(|e| e.to_string())?;

        // 返回包含版本、路径和文件名信息的JSON对象
        let result = serde_json::json!({
            "version": ver,
            "path": frpc_path.to_string_lossy().to_string(),
            "filename": filename
        });
        
        Ok(serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string()))
    } else {
        // 返回包含路径和文件名信息的JSON对象，但标记为错误
        let stderr = String::from_utf8_lossy(&output.stderr);
        let result = serde_json::json!({
            "version": format!("获取版本错误: {}", stderr),
            "path": frpc_path.to_string_lossy().to_string(),
            "filename": filename
        });
        
        Ok(serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string()))
    }
}

// 添加启动检查命令
#[command]
async fn check_and_download<R: Runtime>(app: tauri::AppHandle<R>) -> Result<String, String> {
    if !check_frpc(app.clone()).await? {
        app.emit("need_download", true).map_err(|e| e.to_string())?;
        return Ok("需要下载 frpc".to_string());
    }
    Ok("frpc 已存在".to_string())
}

// 修改检查进程状态的命令
#[command]
async fn check_frpc_status(
    processes: State<'_, FrpcProcesses>,
    id: String,
) -> Result<bool, String> {
    if let Ok(mut map) = processes.0.lock() {
        if let Some(mut process_info) = map.remove(&id) {
            match process_info.child.try_wait() {
                Ok(Some(_)) => {
                    // 进程已结束
                    return Ok(false);
                }
                Ok(None) => {
                    // 进程仍在运行，放回map
                    map.insert(id, process_info);
                    return Ok(true);
                }
                Err(_) => return Ok(false),
            }
        }
    }
    Ok(false)
}

#[command]
async fn kill_all_processes() -> Result<(), String> {
    let os = std::env::consts::OS;
    let os_name = match os {
        "windows" => "windows",
        "linux" => "linux",
        "macos" => "darwin",
        _ => return Err("不支持的操作系统".to_string()),
    };

    let arch_name = match std::env::consts::ARCH {
        "x86" => "386",
        "x86_64" => "amd64",
        "arm" => "arm",
        "aarch64" => "arm64",
        _ => return Err("不支持的系统架构".to_string()),
    };

    // 构建最终的文件名
    let target_filename = if os == "windows" {
        format!("frpc_{}_{}.exe", os_name, arch_name)
    } else {
        format!("frpc_{}_{}", os_name, arch_name)
    };

    #[cfg(target_os = "windows")]
    {
        let mut cmd = Command::new("taskkill");
        cmd.creation_flags(CREATE_NO_WINDOW);
        cmd.args(["/F", "/IM", &target_filename])
            .output()
            .map_err(|e| format!("终止进程失败: {}", e))?;
    }

    #[cfg(target_family = "unix")]
    {
        Command::new("pkill")
            .args(["-f", &target_filename])
            .output()
            .map_err(|e| format!("终止进程失败: {}", e))?;
    }

    Ok(())
}

#[command]
async fn emit_event<R: Runtime>(
    app: tauri::AppHandle<R>,
    event: String,
    payload: serde_json::Value,
) -> Result<(), String> {
    app.emit(&event, payload)
        .map_err(|e| format!("发送事件失败: {}", e))
}

#[command]
async fn exit_app(app_handle: tauri::AppHandle) -> Result<(), String> {
    app_handle.exit(0);
    Ok(())
}

// 在 main 函数前添加这个函数
fn create_tray_menu(app: &tauri::App) -> Result<TrayIcon, Box<dyn std::error::Error>> {
    let menu = Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?,
            &MenuItem::with_id(app, "separator", "", false, None::<&str>)?,
            &MenuItem::with_id(
                app,
                "quit_with_frpc",
                "结束所有隧道并退出",
                true,
                None::<&str>,
            )?,
            &MenuItem::with_id(
                app,
                "quit_keep_frpc",
                "保持隧道运行并退出",
                true,
                None::<&str>,
            )?,
        ],
    )?;

    let tray = TrayIconBuilder::new()
        .menu(&menu)
        .icon(app.default_window_icon().unwrap().clone())
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "quit_with_frpc" => {
                if let Some(processes) = app.try_state::<FrpcProcesses>() {
                    if let Ok(mut map) = processes.0.lock() {
                        for (_, mut process_info) in map.drain() {
                            #[cfg(target_os = "windows")]
                            {
                                let _ = Command::new("taskkill")
                                    .args(&["/F", "/T", "/PID"])
                                    .arg(process_info.group_id.to_string())
                                    .output();
                            }
                            #[cfg(not(target_os = "windows"))]
                            {
                                let _ = process_info.child.kill();
                            }
                        }
                    }
                }
                app.exit(0);
            }
            "quit_keep_frpc" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(move |tray, event| match event {
            tauri::tray::TrayIconEvent::Click {
                button,
                button_state,
                ..
            } => {
                if button == tauri::tray::MouseButton::Left
                    && button_state == tauri::tray::MouseButtonState::Up
                {
                    let app = tray.app_handle();
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
            _ => {}
        })
        .build(app)?;

    Ok(tray)
}

// 添加更新相关的结构体

#[command]
fn get_cpl_version() -> Result<String, String> {
    let config = load_config()?;
    Ok(config.cpl_version.unwrap_or_else(|| "0.6.3".to_string()))
}

#[tauri::command]
async fn toggle_auto_start(app: tauri::AppHandle, enable: bool) -> Result<(), String> {
    let autostart_manager = app.autolaunch();
    
    // 添加重试机制
    let mut retries = 3;
    while retries > 0 {
        match autostart_manager.is_enabled() {
            Ok(current_state) => {
                // 只在状态不一致时进行切换
                if current_state != enable {
                    let result = match enable {
                        true => autostart_manager
                            .enable()
                            .map_err(|e| format!("启用失败: {}", e)),
                        false => autostart_manager
                            .disable()
                            .map_err(|e| format!("禁用失败: {}", e)),
                    };

                    if let Err(e) = result {
                        retries -= 1;
                        if retries > 0 {
                            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                            continue;
                        }
                        return Err(e);
                    }

                    // 验证操作是否成功
                    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                    match autostart_manager.is_enabled() {
                        Ok(new_state) => {
                            if new_state != enable {
                                retries -= 1;
                                if retries > 0 {
                                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                                    continue;
                                }
                                return Err(format!(
                                    "操作未生效: 目标状态 {} 但当前状态为 {}",
                                    enable, new_state
                                ));
                            }
                        }
                        Err(e) => {
                            retries -= 1;
                            if retries > 0 {
                                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                                continue;
                            }
                            return Err(format!("验证状态失败: {}", e));
                        }
                    }
                }
                return Ok(());
            }
            Err(e) => {
                retries -= 1;
                if retries > 0 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    continue;
                }
                return Err(format!("检查状态失败: {}", e));
            }
        }
    }
    
    Err("操作失败，已达到最大重试次数".to_string())
}


#[tauri::command]
async fn check_auto_start(app: tauri::AppHandle) -> Result<bool, String> {
    let autostart_manager = app.autolaunch();
    
    // 多次检查以确保状态稳定，增加重试机制
    let mut retries = 3;
    while retries > 0 {
        match autostart_manager.is_enabled() {
            Ok(state1) => {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                match autostart_manager.is_enabled() {
                    Ok(state2) => {
                        if state1 == state2 {
                            return Ok(state1);
                        } else {
                            retries -= 1;
                            if retries > 0 {
                                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                                continue;
                            }
                            return Err("状态不稳定，请重试".to_string());
                        }
                    }
                    Err(e) => {
                        retries -= 1;
                        if retries > 0 {
                            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                            continue;
                        }
                        return Err(format!("第二次检查失败: {}", e));
                    }
                }
            }
            Err(e) => {
                retries -= 1;
                if retries > 0 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                    continue;
                }
                return Err(format!("检查状态失败: {}", e));
            }
        }
    }
    
    Err("检查自启动状态失败，已达到最大重试次数".to_string())
}

// 添加调试自启动状态的命令
#[tauri::command]
async fn debug_auto_start(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    let autostart_manager = app.autolaunch();
    
    let mut debug_info = serde_json::json!({
        "platform": std::env::consts::OS,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "checks": []
    });
    
    // 进行多次检查以获取详细信息
    for i in 1..=5 {
        match autostart_manager.is_enabled() {
            Ok(enabled) => {
                debug_info["checks"].as_array_mut().unwrap().push(
                    serde_json::json!({
                        "attempt": i,
                        "enabled": enabled,
                        "success": true
                    })
                );
            }
            Err(e) => {
                debug_info["checks"].as_array_mut().unwrap().push(
                    serde_json::json!({
                        "attempt": i,
                        "enabled": null,
                        "success": false,
                        "error": e.to_string()
                    })
                );
            }
        }
        
        if i < 5 {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }
    
    // 计算一致性
    let checks = debug_info["checks"].as_array().unwrap();
    let successful_checks: Vec<_> = checks.iter().filter(|c| c["success"].as_bool().unwrap_or(false)).collect();
    
    if successful_checks.len() >= 3 {
        let enabled_values: Vec<bool> = successful_checks.iter()
            .map(|c| c["enabled"].as_bool().unwrap_or(false))
            .collect();
        
        let all_same = enabled_values.windows(2).all(|w| w[0] == w[1]);
        debug_info["consistent"] = serde_json::Value::Bool(all_same);
        debug_info["final_state"] = serde_json::Value::Bool(enabled_values[0]);
    } else {
        debug_info["consistent"] = serde_json::Value::Bool(false);
        debug_info["final_state"] = serde_json::Value::Null;
    }
    
    Ok(debug_info)
}

// 设置环境变量命令
#[tauri::command]
fn set_env(key: String, value: String) -> Result<(), String> {
    std::env::set_var(&key, &value);
    Ok(())
}

// 获取环境变量命令
#[tauri::command]
fn get_env(key: String) -> Result<Option<String>, String> {
    Ok(std::env::var(&key).ok())
}

// 检查代理绕过状态
#[tauri::command]
fn check_proxy_bypass() -> Result<bool, String> {
    let bypass_proxy = std::env::var("BYPASS_PROXY").unwrap_or_else(|_| "false".to_string());
    Ok(bypass_proxy == "true")
}

// 测试网络连接
#[tauri::command]
async fn test_network_connection() -> Result<serde_json::Value, String> {
    let bypass_proxy = std::env::var("BYPASS_PROXY").unwrap_or_else(|_| "false".to_string());
    
    let client_builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10));
    
    let client = if bypass_proxy == "true" {
        client_builder
            .no_proxy()
            .build()
            .map_err(|e| e.to_string())?
    } else {
        client_builder.build().map_err(|e| e.to_string())?
    };
    
    let mut results = serde_json::json!({
        "bypass_proxy": bypass_proxy == "true",
        "tests": []
    });
    
    // 测试多个端点
    let test_urls = vec![
        ("OpenFrp API", "https://of-dev-api.bfsea.com/frp/api/getUserInfo"),
        ("GitHub", "https://api.github.com"),
        ("Google DNS", "https://8.8.8.8"),
    ];
    
    for (name, url) in test_urls {
        let start = std::time::Instant::now();
        match client.get(url).send().await {
            Ok(response) => {
                let duration = start.elapsed();
                results["tests"].as_array_mut().unwrap().push(
                    serde_json::json!({
                        "name": name,
                        "url": url,
                        "success": true,
                        "status": response.status().as_u16(),
                        "latency_ms": duration.as_millis()
                    })
                );
            }
            Err(e) => {
                let duration = start.elapsed();
                results["tests"].as_array_mut().unwrap().push(
                    serde_json::json!({
                        "name": name,
                        "url": url,
                        "success": false,
                        "error": e.to_string(),
                        "latency_ms": duration.as_millis()
                    })
                );
            }
        }
    }
    
    Ok(results)
}

// 添加状态结构体
#[derive(Default)]
struct DeepLinkState(Mutex<Option<String>>);

#[derive(Serialize, Deserialize)]
struct OAuthResponse {
    authorization: String,
    flag: bool,
    msg: String,
    data: String,
}

#[command]
async fn oauth_callback(code: String) -> Result<OAuthResponse, String> {
    let client = reqwest::Client::new();
    let mut form = std::collections::HashMap::new();
    form.insert("code", code);
    form.insert("redirect_url", "https://staticassets.naids.com/ofcpl_login".to_string());

    let res = client
        .post("https://of-dev-api.bfsea.com/oauth2/callback")
        .form(&form)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let headers = res.headers();
    let auth = headers
        .get("authorization")
        .ok_or("登录失败: 未能找到 Authorization")?
        .to_str()
        .map_err(|e| e.to_string())?
        .to_string();

    let json = res
        .json::<serde_json::Value>()
        .await
        .map_err(|e| e.to_string())?;

    Ok(OAuthResponse {
        authorization: auth,
        flag: json["flag"].as_bool().unwrap_or(false),
        msg: json["msg"].as_str().unwrap_or("").to_string(),
        data: json["data"].as_str().unwrap_or("").to_string(),
    })
}

#[command]
fn get_app_data_dir() -> Result<String, String> {
    let app_dir = get_app_dir();
    Ok(app_dir.to_string_lossy().to_string())
}

#[command]
fn open_app_data_dir() -> Result<(), String> {
    let app_dir = get_app_dir();
    
    // 确保目录存在
    if !app_dir.exists() {
        fs::create_dir_all(&app_dir).map_err(|e| format!("创建目录失败: {}", e))?;
    }
    
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(app_dir)
            .spawn()
            .map_err(|e| format!("打开目录失败: {}", e))?;
    }
    
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(app_dir)
            .spawn()
            .map_err(|e| format!("打开目录失败: {}", e))?;
    }
    
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(app_dir)
            .spawn()
            .map_err(|e| format!("打开目录失败: {}", e))?;
    }
    
    Ok(())
}


#[tauri::command]
async fn get_local_ports() -> Result<serde_json::Value, String> {
    #[cfg(target_os = "windows")]
    {
        use std::collections::HashMap;
        let output = std::process::Command::new("netstat").args(["-ano"]).creation_flags(CREATE_NO_WINDOW).output().map_err(|e| e.to_string())?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut tcp_ports = Vec::new();
        let mut udp_ports = Vec::new();
        let mut pid_map: HashMap<u32, String> = HashMap::new();
        // 获取进程名
        if let Ok(tasklist) = std::process::Command::new("tasklist").creation_flags(CREATE_NO_WINDOW).output() {
            let taskout = String::from_utf8_lossy(&tasklist.stdout);
            for line in taskout.lines().skip(3) {
                let parts: Vec<_> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(pid) = parts[1].parse::<u32>() {
                        pid_map.insert(pid, parts[0].to_string());
                    }
                }
            }
        }
        for line in stdout.lines() {
            let line = line.trim();
            let parts: Vec<_> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let proto = parts[0].to_uppercase();
                let addr = parts[1];
                let pid = parts.last().and_then(|s| s.parse::<u32>().ok());
                if let Some(idx) = addr.rfind(':') {
                    if let Ok(port) = addr[idx+1..].parse::<u16>() {
                        let process = pid.and_then(|p| pid_map.get(&p)).cloned();
                        let entry = serde_json::json!({
                            "port": port,
                            "pid": pid,
                            "process": process,
                        });
                        if proto.starts_with("TCP") {
                            tcp_ports.push(entry);
                        } else if proto.starts_with("UDP") {
                            udp_ports.push(entry);
                        }
                    }
                }
            }
        }
        return Ok(serde_json::json!({ "tcp": tcp_ports, "udp": udp_ports }));
    }
    #[cfg(not(target_os = "windows"))]
    {
        use std::collections::HashMap;
        let output = std::process::Command::new("netstat").args(["-tunlp"]).output().map_err(|e| e.to_string())?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut tcp_ports = Vec::new();
        let mut udp_ports = Vec::new();
        let mut pid_map: HashMap<u32, String> = HashMap::new();
        // 解析 /proc 获取进程名
        if let Ok(entries) = std::fs::read_dir("/proc") {
            for entry in entries.flatten() {
                if let Ok(pid) = entry.file_name().to_string_lossy().parse::<u32>() {
                    if let Ok(cmdline) = std::fs::read_to_string(format!("/proc/{}/cmdline", pid)) {
                        let name = cmdline.split('\0').next().unwrap_or("").split('/').last().unwrap_or("").to_string();
                        pid_map.insert(pid, name);
                    }
                }
            }
        }
        for line in stdout.lines() {
            let line = line.trim();
            let parts: Vec<_> = line.split_whitespace().collect();
            if parts.len() >= 6 {
                let proto = parts[0].to_lowercase();
                let addr = parts[3];
                let pid_info = parts[6];
                let pid = pid_info.split('/').next().and_then(|s| s.parse::<u32>().ok());
                if let Some(idx) = addr.rfind(':') {
                    if let Ok(port) = addr[idx+1..].parse::<u16>() {
                        let process = pid.and_then(|p| pid_map.get(&p)).cloned();
                        let entry = serde_json::json!({
                            "port": port,
                            "pid": pid,
                            "process": process,
                        });
                        if proto.starts_with("tcp") {
                            tcp_ports.push(entry);
                        } else if proto.starts_with("udp") {
                            udp_ports.push(entry);
                        }
                    }
                }
            }
        }
        return Ok(serde_json::json!({ "tcp": tcp_ports, "udp": udp_ports }));
    }
    #[allow(unreachable_code)]
    Ok(serde_json::json!({ "tcp": [], "udp": [] }))
}

// 修改 main 函数
fn main() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--autostart".into()]),
        ))
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            println!("新实例参数: {:?}", argv);
            let _ = app.emit("second-instance", argv);
        }))
        .plugin(tauri_plugin_deep_link::init())
        .setup(|app| {
            let app_dir = init_app_directory(app)?;
            println!("应用程序目录: {:?}", app_dir);

            #[cfg(any(windows, target_os = "linux"))]
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                app.deep_link().register("openfrp")?;
            }

            let _tray = create_tray_menu(app)?;

            #[cfg(target_os = "windows")]
            {
                if let Err(e) = register_app_for_notifications() {
                    eprintln!("Failed to register app for notifications: {}", e);
                }
            }

            app.listen("tauri://update-available", |event| {
                println!("有更新可用: {}", event.payload());
            });

            app.listen("tauri://update-status", |event| {
                println!("更新状态: {}", event.payload());
            });

            Ok(())
        })
        .manage(FrpcProcesses::default())
        .invoke_handler(tauri::generate_handler![
            check_frpc_status,
            download_frpc,
            start_frpc_instance,
            stop_frpc_instance,
            get_frpc_version,
            get_frpc_cli_version,
            check_and_download,
            kill_all_processes,
            emit_event,
            exit_app,
            get_cpl_version,
            toggle_auto_start,
            check_auto_start,
            debug_auto_start,
            set_env,
            get_env,
            check_proxy_bypass,
            test_network_connection,
            oauth_callback,
            check_update,
            install_update,
            get_build_info,
            get_system_info,
            get_detailed_system_info,
            api_proxy::proxy_api,
            get_app_data_dir,
            open_app_data_dir,
            get_local_ports, // 新增端口扫描命令
            download_and_install_update,
            tcp_ping,
            argo_access::argo_generate_public_key,
            argo_access::argo_request_login,
            argo_access::argo_poll_login,
            argo_access::argo_wait_authorization,
            argo_access::argo_cancel_wait,
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    app.run(|app_handle, event| {
        match event {
            tauri::RunEvent::Ready => {
                // 迁移自启动参数、frpc 检查、emit 事件等到这里
                let args: Vec<String> = std::env::args().collect();
                let is_autostart = args.iter().any(|arg| arg == "--autostart");
                if let Some(window) = app_handle.get_webview_window("main") {
                    if is_autostart {
                        let _ = window.eval("window.location.search += window.location.search ? '&autostart=true' : '?autostart=true'");
                        let _ = window.eval("localStorage.setItem('appStartedByAutostart', 'true')");
                        println!("检测到自启动");
                        // window.hide().unwrap(); // 如需隐藏窗口可取消注释
                    }
                    // 检查 frpc 是否存在
                    let app_dir = get_app_dir();
                    let frpc_path = app_dir.join(if cfg!(target_os = "windows") {
                        "frpc_windows_amd64.exe"
                    } else {
                        "frpc_linux_amd64"
                    });
                    if !frpc_path.exists() {
                        let _ = window.emit("redirect_to_settings", "need_download");
                    }
                }
            }
            tauri::RunEvent::Resumed => {}
            tauri::RunEvent::MainEventsCleared => {}
            tauri::RunEvent::Exit => {}
            tauri::RunEvent::ExitRequested { .. } => {}
            _ => {}
        }
    });
}

#[cfg(target_os = "windows")]
fn register_app_for_notifications() -> Result<(), Box<dyn std::error::Error>> {
    use winreg::enums::*;
    use winreg::RegKey;

    let app_path = std::env::current_exe()?;
    let app_name = "OpenFrp Launcher";
    let app_id = "com.openfrp.launcher";

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let path = format!("SOFTWARE\\Classes\\AppUserModelId\\{}", app_id);
    let (key, _) = hkcu.create_subkey(&path)?;

    key.set_value("DisplayName", &app_name)?;
    key.set_value("IconPath", &app_path.to_string_lossy().to_string())?;

    Ok(())
}
