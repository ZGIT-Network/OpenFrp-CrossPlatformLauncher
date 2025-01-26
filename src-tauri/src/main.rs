// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use flate2::read::GzDecoder;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Cursor;
use std::io::Write;
// use std::io::{BufRead, BufReader};
//use std::path::Path;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use tar::Archive;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{TrayIcon, TrayIconBuilder};
use tauri::Manager;
use tauri::{command, Emitter, Runtime, State};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
// use std::sync::atomic;
// use std::sync::Arc;
// use tauri::PhysicalPosition;
// use tauri::WindowEvent;

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
            self.cpl_version = self.cpl_version.or_else(|| Some("0.1.3".to_string()));
        }

        // 更新版本号
        self.config_version = Some(CONFIG_VERSION);
        self
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

    let response = reqwest::get("https://api.openfrp.net/commonQuery/get?key=software")
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

    let response = reqwest::get(&download_url)
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

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&target_path)
            .map_err(|e| e.to_string())?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&target_path, perms).map_err(|e| e.to_string())?;
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
async fn stop_frpc_instance<R: Runtime>(
    _app: tauri::AppHandle<R>,
    processes: State<'_, FrpcProcesses>,
    id: String,
) -> Result<(), String> {
    if let Ok(mut map) = processes.0.lock() {
        if let Some(process_info) = map.remove(&id) {
            #[cfg(target_os = "windows")]
            {
                let mut cmd = Command::new("taskkill");
                cmd.creation_flags(CREATE_NO_WINDOW);
                cmd.args(&["/F", "/T", "/PID"])
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

    let frpc_path = app_dir.join(&config.frpc_filename.as_ref().unwrap());

    if !frpc_path.exists() {
        return Err("frpc 可执行文件不存在".to_string());
    }

    let mut cmd = Command::new(&frpc_path);
    #[cfg(target_os = "windows")]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    
    let child = cmd
        .arg("-v")
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;

    let output = child.wait_with_output().map_err(|e| e.to_string())?;

    if output.status.success() {
        let version_str = String::from_utf8(output.stdout).map_err(|e| e.to_string())?;

        if let Some(version) = version_str.lines().next() {
            if let Some(ver) = version.split_whitespace().last() {
                config.frpc_version = Some(ver.to_string());
                save_config(&config)?;
            }
        }

        app.emit(
            "log",
            LogPayload {
                message: format!("检测到本地版本:{}", version_str),
            },
        )
        .map_err(|e| e.to_string())?;

        Ok("版本信息已更新".to_string())
    } else {
        Err("获取版本信息失败".to_string())
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

// 添加一个检查进程状态的命令
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
                        for (_, process_info) in map.drain() {
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
#[derive(Serialize, Deserialize, Debug)]
struct CplUpdate {
    latest: String,
    url: String,
    msg: String,
    title: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct UpdateData {
    latest: String,
    latest_full: String,
    latest_ver: String,
    latest_msg: String,
    common_details: String,
    #[serde(rename = "cplUpdate")]
    cpl_update: CplUpdate,
    // 其他字段可以暂时忽略
    #[serde(skip)]
    launcherUpdate: Option<serde_json::Value>,
    #[serde(skip)]
    source: Option<serde_json::Value>,
    #[serde(skip)]
    soft: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ApiResponse {
    data: UpdateData,
    flag: bool,
    msg: String,
}

#[command]
async fn check_update() -> Result<Option<CplUpdate>, String> {
    let response = reqwest::get("https://api.openfrp.net/commonQuery/get?key=software")
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    let text = response.text().await.map_err(|e| format!("读取响应失败: {}", e))?;
    // println!("API Response: {}", text); // 输出完整响应

    if text.is_empty() {
        return Err("API 响应为空".to_string());
    }

    let api_response: ApiResponse =
        serde_json::from_str(&text).map_err(|e| format!("解析错误: {} \n原始数据: {}", e, text))?;

    let current_version = env!("CARGO_PKG_VERSION");
    println!("Current version: {}, Latest version: {}", current_version, api_response.data.cpl_update.latest);

    if current_version != api_response.data.cpl_update.latest {
        Ok(Some(api_response.data.cpl_update))
    } else {
        Ok(None)
    }
}

// 添加下载和更新命令
#[command]
async fn download_and_update(app: tauri::AppHandle) -> Result<(), String> {
    let response = reqwest::get("https://api.openfrp.net/commonQuery/get?key=software")
        .await
        .map_err(|e| e.to_string())?;

    let api_response: ApiResponse = response.json().await.map_err(|e| e.to_string())?;
    let download_url = format!(
        "{}ofcpl_{}_{}.zip",
        api_response.data.cpl_update.url,
        std::env::consts::OS,
        std::env::consts::ARCH
    );

    // 下载更新包
    let response = reqwest::get(&download_url)
        .await
        .map_err(|e| e.to_string())?;

    let bytes = response.bytes().await.map_err(|e| e.to_string())?;

    // 获取临时目录
    let app_dir = get_app_dir();
    let temp_dir = app_dir.join("temp");
    fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;

    // 保存并解压更新包
    let temp_zip = temp_dir.join("update.zip");
    fs::write(&temp_zip, &bytes).map_err(|e| e.to_string())?;

    // 解压更新包
    let file = File::open(&temp_zip).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    archive.extract(&temp_dir).map_err(|e| e.to_string())?;

    // 准备重启命令
    let current_exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let new_exe = temp_dir.join("openfrp-crossplatformlauncher");

    // 创建更新脚本
    #[cfg(target_os = "windows")]
    {
        let bat_content = format!(
            "@echo off\n\
             timeout /t 1 /nobreak >nul\n\
             copy /y \"{}\" \"{}\"\n\
             start \"\" \"{}\"\n",
            new_exe.display(),
            current_exe.display(),
            current_exe.display()
        );
        let bat_path = temp_dir.join("update.bat");
        fs::write(&bat_path, bat_content).map_err(|e| e.to_string())?;

        Command::new("cmd")
            .args(["/C", "start", "/b", bat_path.to_str().unwrap()])
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_family = "unix")]
    {
        let sh_content = format!(
            "#!/bin/sh\n\
             sleep 1\n\
             cp -f \"{}\" \"{}\"\n\
             \"{}\" &",
            new_exe.display(),
            current_exe.display(),
            current_exe.display()
        );
        let sh_path = temp_dir.join("update.sh");
        fs::write(&sh_path, &sh_content).map_err(|e| e.to_string())?;
        fs::set_permissions(
            &sh_path,
            std::os::unix::fs::PermissionsExt::from_mode(0o755),
        )
        .map_err(|e| e.to_string())?;

        Command::new("sh")
            .arg(sh_path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    // 更新配置文件中的版本号
    let mut config = load_config()?;
    config.cpl_version = Some(api_response.data.cpl_update.latest);
    save_config(&config)?;

    // 退出当前进程
    app.exit(0);

    Ok(())
}

#[command]
fn get_cpl_version() -> Result<String, String> {
    let config = load_config()?;
    Ok(config.cpl_version.unwrap_or_else(|| "0.1.0".to_string()))
}

#[tauri::command]
async fn toggle_auto_start(app: tauri::AppHandle, enable: bool) -> Result<(), String> {
    let autostart_manager = app.autolaunch();
    let current_state = autostart_manager.is_enabled().map_err(|e| format!("检查状态失败: {}", e))?;
    
    // 只在状态不一致时进行切换
    if current_state != enable {
        match enable {
            true => autostart_manager.enable().map_err(|e| format!("启用失败: {}", e))?,
            false => autostart_manager.disable().map_err(|e| format!("禁用失败: {}", e))?,
        }
        
        // 验证操作是否成功
        let new_state = autostart_manager.is_enabled().map_err(|e| e.to_string())?;
        if new_state != enable {
            return Err(format!("操作未生效: 目标状态 {} 但当前状态为 {}", enable, new_state));
        }
    }
    
    Ok(())
}

#[tauri::command]
async fn check_auto_start(app: tauri::AppHandle) -> Result<bool, String> {
    let autostart_manager = app.autolaunch();
    // 多次检查以确保状态稳定
    let state1 = autostart_manager.is_enabled().map_err(|e| e.to_string())?;
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    let state2 = autostart_manager.is_enabled().map_err(|e| e.to_string())?;
    
    if state1 != state2 {
        return Err("状态不稳定，请重试".to_string());
    }
    
    Ok(state1)
}

// 修改 main 函数
fn main() {
    let instance = single_instance::SingleInstance::new("openfrp-cpl").unwrap();

    if !instance.is_single() {
        let builder = tauri::Builder::default().plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![])
        ));
        let app = builder.build(tauri::generate_context!()).unwrap();

        app.dialog()
            .message("OpenFrp 跨平台启动器已经在运行中。\n请检查系统托盘或任务栏。")
            .title("程序已在运行")
            .kind(MessageDialogKind::Warning)
            .blocking_show();

        std::process::exit(1);
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![])
        ))
        .setup(|app| {
            let app_dir = init_app_directory(app)?;
            println!("应用程序目录: {:?}", app_dir);

            let _tray = create_tray_menu(app)?;

            // 检查 frpc 是否存在
            let frpc_path = app_dir.join("frpc").join(if cfg!(target_os = "windows") {
                "frpc.exe"
            } else {
                "frpc"
            });

            if !frpc_path.exists() {
                // 如果 frpc 不存在，发送事件通知前端
                let window = app.get_webview_window("main").unwrap();
                // 发送一个带有路由信息的事件
                window
                    .emit("redirect_to_settings", "need_download")
                    .unwrap();
            }

            Ok(())
        })
        .manage(FrpcProcesses::default())
        .invoke_handler(tauri::generate_handler![
            check_update,
            download_and_update,
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
        ])
        .run(tauri::generate_context!())
        .expect("运行时出错");
}
