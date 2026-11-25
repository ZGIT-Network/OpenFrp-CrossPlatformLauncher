// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use tar::Archive;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{TrayIcon, TrayIconBuilder};
use tauri::Manager;
mod argo_access;
use tauri::{command, Emitter, Runtime, State};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};

use tauri_plugin_deep_link;
// use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
use tauri_plugin_updater;
mod api_proxy;
mod update;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use tauri_plugin_deep_link::DeepLinkExt;
mod local_ports;
mod oauth;

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
    instance_id: String,
    name: String,
    remote: String,
    external: bool,
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
const CONFIG_VERSION: u32 = 2;

#[derive(Serialize, Deserialize, Default)]
struct Config {
    // 环境变量持久化
    env_vars: Option<HashMap<String, String>>,
    config_version: Option<u32>, // 配置文件版本号
    frpc_version: Option<String>,
    frpc_filename: Option<String>,
    cpl_version: Option<String>,
    // 网络设置持久化（显式字段，避免仅依赖 env map）
    force_tls: Option<bool>,
    use_doh: Option<bool>,
    doh_addr: Option<String>,
}

impl Config {
    fn upgrade(mut self) -> Self {
        // 自动补全新字段
        if self.env_vars.is_none() {
            self.env_vars = Some(HashMap::new());
        }
        let current_version = self.config_version.unwrap_or(0);

        if current_version < 1 {
            // 版本0到版本1的升级
            self.frpc_version = self.frpc_version.or_else(|| Some(String::new()));
            self.frpc_filename = self.frpc_filename.or_else(|| Some(String::new()));
            self.cpl_version = self.cpl_version.or_else(|| Some("0.9.1".to_string()));
        }

        // 与 env_vars 做双向兜底同步，兼容旧版本
        let mut env_map = self.env_vars.take().unwrap_or_default();

        // 先从 env 反填字段（若字段缺失）
        if self.force_tls.is_none() {
            self.force_tls = Some(
                env_map
                    .get("FRPC_FORCE_TLS")
                    .map(|v| v == "true")
                    .unwrap_or(false),
            );
        }
        if self.use_doh.is_none() {
            self.use_doh = Some(env_map.get("USE_DOH").map(|v| v == "true").unwrap_or(false));
        }
        if self.doh_addr.as_deref().unwrap_or("").is_empty() {
            self.doh_addr = Some(env_map.get("DOH_ADDR").cloned().unwrap_or_default());
        }

        // 最终默认值
        if self.force_tls.is_none() {
            self.force_tls = Some(false);
        }
        if self.use_doh.is_none() {
            self.use_doh = Some(false);
        }
        if self.doh_addr.is_none() {
            self.doh_addr = Some(String::new());
        }

        // 再把字段写回 env，保证运行时仍可通过 set_env/get_env 使用
        env_map.insert(
            "FRPC_FORCE_TLS".to_string(),
            if self.force_tls.unwrap_or(false) {
                "true"
            } else {
                "false"
            }
            .to_string(),
        );
        env_map.insert(
            "USE_DOH".to_string(),
            if self.use_doh.unwrap_or(false) {
                "true"
            } else {
                "false"
            }
            .to_string(),
        );
        env_map.insert(
            "DOH_ADDR".to_string(),
            self.doh_addr.clone().unwrap_or_default(),
        );

        self.env_vars = Some(env_map);

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

/// 根据当前 OS/ARCH 生成默认的 frpc 文件名（跨平台正确）
fn default_frpc_filename() -> String {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let os_name = match os {
        "windows" => "windows",
        "linux" => "linux",
        "macos" => "darwin",
        _ => return format!("frpc_{}_{}", os, arch),
    };
    let arch_name = match arch {
        "x86" => "386",
        "x86_64" => "amd64",
        "arm" => "arm",
        "aarch64" => "arm64",
        _ => return format!("frpc_{}_{}", os_name, arch),
    };
    if os == "windows" {
        format!("frpc_{}_{}.exe", os_name, arch_name)
    } else {
        format!("frpc_{}_{}", os_name, arch_name)
    }
}

// 获取配置文件路径
fn get_config_path() -> Result<PathBuf, String> {
    let app_dir = get_app_dir();
    Ok(app_dir.join("config.json"))
}

// 加载配置
static CONFIG_WRITE: Mutex<()> = Mutex::new(());

fn load_config() -> Result<Config, String> {
    let path = get_config_path()?;
    let config = match fs::read_to_string(path) {
        Ok(content) => serde_json::from_str::<Config>(&content)
            .map_err(|e| format!("解析配置文件失败: {}", e))?,
        Err(e) if e.kind() == io::ErrorKind::NotFound => Config::default(),
        Err(e) => return Err(format!("读取配置文件失败: {}", e)),
    };
    let mut config = config.upgrade();
    config.cpl_version = Some(env!("CARGO_PKG_VERSION").to_string());
    Ok(config)
}

fn update_config(update: impl FnOnce(&mut Config)) -> Result<(), String> {
    let _lock = CONFIG_WRITE.lock().map_err(|e| e.to_string())?;
    let mut config = load_config()?;
    update(&mut config);
    let mut staged = tempfile::NamedTempFile::new_in(get_app_dir())
        .map_err(|e| format!("创建临时配置文件失败: {}", e))?;
    serde_json::to_writer_pretty(&mut staged, &config).map_err(|e| e.to_string())?;
    staged.as_file().sync_all().map_err(|e| e.to_string())?;
    staged
        .persist(get_config_path()?)
        .map_err(|e| format!("保存配置文件失败: {}", e))?;
    Ok(())
}

fn frpc_path(config: &Config) -> PathBuf {
    let dir = get_app_dir();
    let default = dir.join(default_frpc_filename());
    let configured = config
        .frpc_filename
        .as_deref()
        .filter(|name| Path::new(name).file_name().and_then(|file| file.to_str()) == Some(*name));
    configured
        .map(|name| dir.join(name))
        .filter(|path| path.is_file())
        .unwrap_or(default)
}

const MAX_FRPC_BYTES: usize = 128 * 1024 * 1024;
static FRPC_DOWNLOAD: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn copy_frpc(mut entry: impl Read, target: &mut impl Write) -> Result<(), String> {
    let size = io::copy(&mut entry.by_ref().take(MAX_FRPC_BYTES as u64 + 1), target)
        .map_err(|e| format!("解压 Frpc 失败: {}", e))?;
    if size == 0 || size > MAX_FRPC_BYTES as u64 {
        return Err("Frpc 可执行文件大小无效".into());
    }
    Ok(())
}

fn extract_frpc(bytes: &[u8], filename: &str, target: &mut impl Write) -> Result<(), String> {
    let binary = if filename.ends_with(".exe") {
        "frpc.exe"
    } else {
        "frpc"
    };
    let is_binary = |path: &Path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == filename || name == binary)
    };
    if filename.ends_with(".exe") {
        let mut archive = zip::ZipArchive::new(io::Cursor::new(bytes))
            .map_err(|e| format!("读取 Frpc 压缩包失败: {}", e))?;
        for i in 0..archive.len() {
            let entry = archive.by_index(i).map_err(|e| e.to_string())?;
            if entry.is_file()
                && entry
                    .unix_mode()
                    .map_or(true, |mode| (mode & 0o170000) != 0o120000)
                && entry.enclosed_name().is_some_and(is_binary)
            {
                return copy_frpc(entry, target);
            }
        }
    } else {
        let mut archive = Archive::new(GzDecoder::new(bytes));
        for entry in archive.entries().map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            if entry.header().entry_type().is_file()
                && is_binary(&entry.path().map_err(|e| e.to_string())?)
            {
                return copy_frpc(entry, target);
            }
        }
    }
    Err("压缩包中没有找到对应的 Frpc 可执行文件".into())
}

fn valid_frpc_image(file: &mut fs::File) -> bool {
    let mut header = [0; 4];
    if file
        .rewind()
        .and_then(|_| file.read_exact(&mut header))
        .is_err()
    {
        return false;
    }
    if cfg!(windows) {
        header.starts_with(b"MZ")
    } else if cfg!(target_os = "linux") {
        header == *b"\x7FELF"
    } else {
        matches!(
            header,
            [0xcf, 0xfa, 0xed, 0xfe] | [0xca, 0xfe, 0xba, 0xbe] | [0xca, 0xfe, 0xba, 0xbf]
        )
    }
}

fn log_message<R: Runtime>(app: &tauri::AppHandle<R>, message: impl Into<String>) {
    let _ = app.emit(
        "log",
        LogPayload {
            message: message.into(),
        },
    );
}

#[command]
async fn download_frpc<R: Runtime>(app: tauri::AppHandle<R>) -> Result<String, String> {
    let _download = FRPC_DOWNLOAD.lock().await;
    let filename = default_frpc_filename();
    let windows = cfg!(target_os = "windows");
    let archive_name = if windows {
        format!("{}.zip", filename.trim_end_matches(".exe"))
    } else {
        format!("{}.tar.gz", filename)
    };
    let client = api_proxy::client_builder(60)
        .https_only(true)
        .build()
        .map_err(|e| e.to_string())?;

    log_message(&app, "开始获取最新 Frpc 版本信息...");
    let mut last_error = "获取软件信息失败".to_string();
    let mut metadata = None;
    for base in api_proxy::OPENFRP_API_BASES {
        let result = async {
            let response = client
                .get(format!("{base}/commonQuery/get?key=software"))
                .timeout(std::time::Duration::from_secs(12))
                .send()
                .await?
                .error_for_status()?;
            response.json::<SoftwareInfo>().await
        }
        .await;
        match result {
            Ok(info)
                if !info.data.latest.trim_matches('/').is_empty()
                    && !info.data.source.is_empty() =>
            {
                metadata = Some(info.data);
                break;
            }
            Ok(_) => last_error = "软件信息缺少版本或下载地址".into(),
            Err(e) => last_error = format!("获取软件信息失败: {}", e),
        }
    }
    let metadata = metadata.ok_or(last_error)?;
    let version = metadata.latest.trim_matches('/');
    let dir = get_app_dir();
    let target_path = dir.join(&filename);
    let config = load_config()?;
    if target_path.is_file()
        && frpc_path(&config) == target_path
        && config.frpc_version.as_deref() == Some(version)
        && fs::File::open(&target_path).is_ok_and(|mut file| valid_frpc_image(&mut file))
    {
        log_message(&app, "已经是最新版本");
        return Ok("已经是最新版本".into());
    }

    let mut last_error = "没有可用的下载镜像".to_string();
    for source in metadata.source {
        let url = format!(
            "{}/{}/{}",
            source.value.trim_end_matches('/'),
            version,
            archive_name
        );
        log_message(&app, format!("正在从 {} 下载 Frpc...", source.label));
        let staged = async {
            let mut response = client
                .get(url)
                .send()
                .await
                .map_err(|e| e.to_string())?
                .error_for_status()
                .map_err(|e| e.to_string())?;
            let mut bytes = Vec::new();
            while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
                if bytes.len() + chunk.len() > MAX_FRPC_BYTES {
                    return Err("Frpc 压缩包过大".into());
                }
                bytes.extend_from_slice(&chunk);
            }
            let mut file = tempfile::Builder::new()
                .suffix(if windows { ".exe" } else { "" })
                .tempfile_in(&dir)
                .map_err(|e| e.to_string())?;
            extract_frpc(&bytes, &filename, &mut file)?;
            if !valid_frpc_image(file.as_file_mut()) {
                return Err("压缩包中的 Frpc 不是当前平台的可执行文件".into());
            }
            file.as_file().sync_all().map_err(|e| e.to_string())?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(file.path(), fs::Permissions::from_mode(0o755))
                    .map_err(|e| format!("无法设置可执行权限: {}", e))?;
            }
            Ok::<_, String>(file)
        }
        .await;
        let file = match staged {
            Ok(file) => file,
            Err(e) => {
                last_error = format!("{} 下载或解压失败: {}", source.label, e);
                log_message(&app, &last_error);
                continue;
            }
        };
        {
            let processes = app.state::<FrpcProcesses>();
            let mut running = processes.0.lock().map_err(|e| e.to_string())?;
            if running
                .values_mut()
                .any(|info| !matches!(info.child.try_wait(), Ok(Some(_))))
            {
                return Err("请停止运行中的隧道后再更新 Frpc".into());
            }
            file.persist(&target_path)
                .map_err(|e| format!("替换 Frpc 失败: {}", e))?;
            update_config(|config| {
                config.frpc_filename = Some(filename);
                config.frpc_version = Some(version.to_string());
            })
            .map_err(|e| format!("Frpc 已安装，保存版本信息失败: {}", e))?;
        }
        log_message(&app, "Frpc 下载和安装完成");
        return Ok("下载完成".into());
    }
    Err(last_error)
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TunnelInstance {
    tunnel_id: String,
    instance_id: String,
}

#[derive(Clone, Serialize)]
struct InstanceLog<'a> {
    #[serde(flatten)]
    instance: &'a TunnelInstance,
    message: &'a str,
}

#[command]
async fn start_frpc_instance<R: Runtime>(
    app: tauri::AppHandle<R>,
    processes: State<'_, FrpcProcesses>,
    id: String,
    instance_id: Option<String>,
    token: String,
    tunnel_id: String,
    name: String,
    remote: String,
    external: Option<bool>,
) -> Result<String, String> {
    if id != tunnel_id
        || !id
            .parse::<u64>()
            .is_ok_and(|value| value > 0 && value.to_string() == id)
    {
        return Err("隧道 ID 无效".into());
    }
    let token = token.trim();
    if token.is_empty() || token.starts_with('-') || token.chars().any(char::is_control) {
        return Err("缺少有效的 Frpc 访问密钥".into());
    }

    let mut map = processes.0.lock().map_err(|e| e.to_string())?;
    if let Some(info) = map.get_mut(&id) {
        if info.child.try_wait().map_err(|e| e.to_string())?.is_none() {
            return Err("该隧道已经在运行中".into());
        }
    }
    map.remove(&id);

    let config = load_config()?;
    let path = frpc_path(&config);
    if !path.is_file() {
        return Err("Frpc 程序不存在，请先配置或下载".into());
    }

    let mut cmd = Command::new(path);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);

    cmd.args(["-u", token, "-p", &tunnel_id]);
    if config_value(&config, "FRPC_DEBUG").as_deref() == Some("true") {
        cmd.arg("--debug");
    }
    if config.force_tls == Some(true) {
        cmd.arg("--force-tls");
    }
    if config.use_doh == Some(true) {
        cmd.arg("--use-doh");
        if let Some(addr) = config.doh_addr.as_deref().filter(|addr| !addr.is_empty()) {
            cmd.args(["--doh-addr", addr]);
        }
    }

    if config_value(&config, "BYPASS_PROXY").as_deref() == Some("true") {
        for key in [
            "HTTP_PROXY",
            "HTTPS_PROXY",
            "ALL_PROXY",
            "http_proxy",
            "https_proxy",
            "all_proxy",
        ] {
            cmd.env_remove(key);
        }
        #[cfg(target_os = "windows")]
        cmd.env("WINHTTP_NO_PROXY", "*");
    }

    // Frpc 可在启动器退出后继续写入日志，避免继承的管道关闭导致子进程退出。
    let (reader, stdout, stderr) = frpc_log_handles(&get_app_dir().join("frpc"))
        .map_err(|e| format!("准备 Frpc 日志失败: {e}"))?;
    cmd.stdin(Stdio::null()).stdout(stdout).stderr(stderr);

    let child = cmd.spawn().map_err(|e| e.to_string())?;
    let pid = child.id();
    let instance = TunnelInstance {
        tunnel_id: id.clone(),
        instance_id: instance_id.unwrap_or_else(|| pid.to_string()),
    };
    map.insert(
        id,
        ProcessInfo {
            child,
            instance_id: instance.instance_id.clone(),
            name,
            remote,
            external: external.unwrap_or(false),
        },
    );
    drop(map);

    rebuild_tray_menu(&app);
    thread::spawn(move || supervise_frpc(app, instance, pid, BufReader::new(reader)));
    Ok("Frpc 进程已启动".to_string())
}

fn frpc_log_handles(dir: &Path) -> io::Result<(fs::File, fs::File, fs::File)> {
    let log = tempfile::NamedTempFile::new_in(dir)?;
    let reader = log.reopen()?;
    let stdout = log.as_file().try_clone()?;
    let stderr = log.as_file().try_clone()?;
    log.close()?;
    Ok((reader, stdout, stderr))
}

fn supervise_frpc<R: Runtime>(
    app: tauri::AppHandle<R>,
    instance: TunnelInstance,
    pid: u32,
    mut reader: impl BufRead,
) {
    let mut bytes = Vec::new();
    loop {
        // 限制每轮工作量，持续输出时仍能及时检测退出。
        for _ in 0..256 {
            match reader
                .by_ref()
                .take(64 * 1024)
                .read_until(b'\n', &mut bytes)
            {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    let line = String::from_utf8_lossy(&bytes);
                    let _ = app.emit_to(
                        "main",
                        "frpc-instance-log",
                        InstanceLog {
                            instance: &instance,
                            message: line.trim_end_matches(['\r', '\n']),
                        },
                    );
                    bytes.clear();
                }
            }
        }
        let exited = {
            let processes = app.state::<FrpcProcesses>();
            let Ok(mut map) = processes.0.lock() else {
                return;
            };
            let Some(info) = map
                .get_mut(&instance.tunnel_id)
                .filter(|info| info.child.id() == pid && info.instance_id == instance.instance_id)
            else {
                return;
            };
            if matches!(info.child.try_wait(), Ok(Some(_))) {
                map.remove(&instance.tunnel_id);
                true
            } else {
                false
            }
        };
        if exited {
            rebuild_tray_menu(&app);
            let _ = app.emit_to("main", "tunnels-stopped", [instance]);
            return;
        }
        thread::sleep(std::time::Duration::from_millis(100));
    }
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

fn get_windows_build_info() -> String {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;

        // 获取详细的 Windows 版本信息
        let output = Command::new("wmic")
            .args(&[
                "os",
                "get",
                "BuildNumber,Version,OSArchitecture,Locale,CurrentBuild",
                "/value",
            ])
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
                    return format!(
                        "Microsoft Windows [Version {}.{}]_{}fre_{}",
                        version,
                        build_number,
                        architecture.to_lowercase().replace("bit", ""),
                        locale_str
                    );
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

            return format!(
                "{} [Version {}.{}{}]_{}fre_{}",
                product_name,
                current_build,
                current_build,
                ubr_val,
                arch,
                get_windows_locale()
            );
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
                if line.contains(value_name)
                    && (line.contains("REG_SZ") || line.contains("REG_DWORD"))
                {
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
        let output = Command::new("sw_vers").output();

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

// ===== Argo stubs removed - now using real argo_access functions =====

#[command]
fn get_build_info() -> String {
    let build_time = env!("BUILD_TIME", "未知构建时间");
    let _commit_id = env!("GIT_HASH", "未知提交");
    format!("build.{}", build_time)
}

#[command]
async fn tcp_ping(host: String, port: u16) -> Result<serde_json::Value, String> {
    use std::time::{Duration, Instant};
    use tokio::net::{lookup_host, TcpStream};
    use tokio::time::timeout;

    let host = host.trim().trim_start_matches('[').trim_end_matches(']');
    if host.is_empty() || port == 0 {
        return Err("节点地址或端口无效".into());
    }
    let addresses: Vec<_> = timeout(Duration::from_secs(3), lookup_host((host, port)))
        .await
        .map_err(|_| "地址解析超时")?
        .map_err(|error| format!("地址解析失败: {error}"))?
        .collect();
    if addresses.is_empty() {
        return Err("未能解析到有效的地址".into());
    }
    let mut latencies = Vec::new();
    for _ in 0..3 {
        let start = Instant::now();
        match timeout(
            Duration::from_secs(2),
            TcpStream::connect(addresses.as_slice()),
        )
        .await
        {
            Ok(Ok(_)) => latencies.push(start.elapsed().as_secs_f64() * 1000.0),
            _ if latencies.is_empty() => break,
            _ => {}
        }
    }
    if latencies.is_empty() {
        return Ok(serde_json::json!({
            "success": false, "latency_ms": null, "message": "无法连接到指定节点和端口",
        }));
    }
    let avg = latencies.iter().sum::<f64>() / latencies.len() as f64;
    let min = latencies.iter().copied().fold(f64::INFINITY, f64::min);
    let max = latencies.iter().copied().fold(0.0, f64::max);
    Ok(serde_json::json!({
        "success": true,
        "latency_ms": avg,
        "message": format!("平均延迟 {avg:.2} ms（最小 {min:.2} ms，最大 {max:.2} ms）"),
    }))
}

fn terminate_child(child: &mut Child) -> io::Result<()> {
    if child.try_wait()?.is_none() {
        if let Err(error) = child.kill() {
            if child.try_wait()?.is_none() {
                return Err(error);
            }
        }
        child.wait()?;
    }
    Ok(())
}

fn stop_managed_tunnel<R: Runtime>(app: &tauri::AppHandle<R>, id: &str) -> Result<(), String> {
    let stopped = {
        let processes = app.state::<FrpcProcesses>();
        let mut map = processes.0.lock().map_err(|e| e.to_string())?;
        if let Some(info) = map.get_mut(id) {
            terminate_child(&mut info.child).map_err(|e| format!("终止隧道 {id} 失败: {e}"))?;
        }
        map.remove(id)
    };
    rebuild_tray_menu(app);
    if let Some(info) = stopped {
        let instance = TunnelInstance {
            tunnel_id: id.to_string(),
            instance_id: info.instance_id,
        };
        let _ = app.emit_to("main", "tunnels-stopped", [instance]);
    }
    Ok(())
}

#[command]
async fn stop_frpc_instance<R: Runtime>(
    app: tauri::AppHandle<R>,
    id: String,
) -> Result<(), String> {
    stop_managed_tunnel(&app, &id)
}

#[command]
fn check_frpc() -> Result<bool, String> {
    Ok(frpc_path(&load_config()?).is_file())
}

#[derive(Serialize)]
struct FrpcFile {
    path: PathBuf,
    filename: String,
    installed: bool,
}

#[command]
fn get_frpc_file() -> Result<FrpcFile, String> {
    let path = frpc_path(&load_config()?);
    Ok(FrpcFile {
        filename: path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string(),
        installed: path.is_file(),
        path,
    })
}

#[command]
async fn get_frpc_cli_version() -> Result<String, String> {
    let file = get_frpc_file()?;
    if !file.installed {
        return Err("未找到 Frpc 可执行文件".into());
    }
    let mut command = tokio::process::Command::new(file.path);
    #[cfg(target_os = "windows")]
    command.creation_flags(CREATE_NO_WINDOW);
    command.arg("-v").kill_on_drop(true);
    let output = tokio::time::timeout(std::time::Duration::from_secs(5), command.output())
        .await
        .map_err(|_| "获取 Frpc 版本超时".to_string())?
        .map_err(|e| format!("无法执行 Frpc: {}", e))?;
    if !output.status.success() {
        return Err(format!("获取 Frpc 版本失败 ({})", output.status));
    }
    let version = String::from_utf8_lossy(&output.stdout);
    version
        .lines()
        .find(|line| !line.trim().is_empty())
        .map(|line| line.trim().to_string())
        .ok_or_else(|| "Frpc 未返回版本信息".to_string())
}

#[command]
fn check_frpc_status(processes: State<'_, FrpcProcesses>, id: String) -> Result<bool, String> {
    let mut map = processes.0.lock().map_err(|e| e.to_string())?;
    match map.get_mut(&id) {
        Some(info) => info
            .child
            .try_wait()
            .map(|status| status.is_none())
            .map_err(|e| e.to_string()),
        None => Ok(false),
    }
}

#[derive(Serialize)]
struct RunningTunnel {
    id: String,
    external: bool,
}

#[command]
fn list_frpc_instances(processes: State<'_, FrpcProcesses>) -> Result<Vec<RunningTunnel>, String> {
    let mut map = processes.0.lock().map_err(|e| e.to_string())?;
    let mut running = Vec::new();
    for (id, info) in map.iter_mut() {
        if info.child.try_wait().map_err(|e| e.to_string())?.is_none() {
            running.push(RunningTunnel {
                id: id.clone(),
                external: info.external,
            });
        }
    }
    Ok(running)
}

fn terminate_managed_tunnels<R: Runtime>(
    app: &tauri::AppHandle<R>,
    notify: bool,
) -> Result<(), String> {
    let mut stopped = Vec::new();
    let mut errors = Vec::new();
    {
        let processes = app.state::<FrpcProcesses>();
        let mut map = processes.0.lock().map_err(|e| e.to_string())?;
        map.retain(|id, info| match terminate_child(&mut info.child) {
            Ok(()) => {
                stopped.push(TunnelInstance {
                    tunnel_id: id.clone(),
                    instance_id: info.instance_id.clone(),
                });
                false
            }
            Err(error) => {
                errors.push(format!("#{id}: {error}"));
                true
            }
        });
    }
    rebuild_tray_menu(app);
    if notify && !stopped.is_empty() {
        let _ = app.emit_to("main", "tunnels-stopped", stopped);
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("部分隧道终止失败: {}", errors.join("；")))
    }
}

#[command]
async fn kill_all_processes<R: Runtime>(app: tauri::AppHandle<R>) -> Result<(), String> {
    terminate_managed_tunnels(&app, true)
}

/// 查询当前是否以自启动模式运行（前端通过 invoke 调用，避免修改 URL 导致页面重载）
#[command]
fn check_is_autostart() -> bool {
    std::env::args().any(|arg| arg == "--autostart")
}

#[command]
async fn exit_app(app_handle: tauri::AppHandle) -> Result<(), String> {
    terminate_managed_tunnels(&app_handle, false)?;
    app_handle.exit(0);
    Ok(())
}

// 动态重建托盘菜单（根据运行中的隧道列表）
fn rebuild_tray_menu<R: Runtime>(app: &tauri::AppHandle<R>) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || set_tray_menu(&handle));
}

fn set_tray_menu<R: Runtime>(app: &tauri::AppHandle<R>) {
    let Some(tray) = app.tray_by_id("main") else {
        return;
    };

    let show = MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>).ok();
    let sep1 = PredefinedMenuItem::separator(app).ok();
    let sep2 = PredefinedMenuItem::separator(app).ok();
    let quit_with = MenuItem::with_id(
        app,
        "quit_with_frpc",
        "结束所有隧道并退出",
        true,
        None::<&str>,
    )
    .ok();
    let quit_keep = MenuItem::with_id(
        app,
        "quit_keep_frpc",
        "保持隧道运行并退出",
        true,
        None::<&str>,
    )
    .ok();

    let mut tunnel_submenus: Vec<Submenu<R>> = Vec::new();
    let mut no_tunnels: Option<MenuItem<R>> = None;

    if let Some(processes) = app.try_state::<FrpcProcesses>() {
        if let Ok(map) = processes.0.lock() {
            if map.is_empty() {
                no_tunnels =
                    MenuItem::with_id(app, "no_tunnels", "（无运行中的隧道）", false, None::<&str>)
                        .ok();
            } else {
                for (id, info) in map.iter() {
                    let copy_item = MenuItem::with_id(
                        app,
                        &format!("copy_link_{}", id),
                        "复制链接地址",
                        true,
                        None::<&str>,
                    )
                    .ok();
                    let stop_item = MenuItem::with_id(
                        app,
                        &format!("stop_{}", id),
                        "停止隧道",
                        true,
                        None::<&str>,
                    )
                    .ok();
                    let label = if info.name.is_empty() {
                        format!("#{}", id)
                    } else {
                        format!("#{} {}", id, info.name)
                    };
                    if let (Some(copy_item), Some(stop_item)) = (copy_item, stop_item) {
                        if let Ok(submenu) =
                            Submenu::with_items(app, &label, true, &[&copy_item, &stop_item])
                        {
                            tunnel_submenus.push(submenu);
                        }
                    }
                }
            }
        }
    }

    let mut items: Vec<&dyn tauri::menu::IsMenuItem<R>> = Vec::new();
    if let Some(ref item) = show {
        items.push(item);
    }
    if let Some(ref item) = sep1 {
        items.push(item);
    }
    if let Some(ref item) = no_tunnels {
        items.push(item);
    }
    for item in &tunnel_submenus {
        items.push(item);
    }
    if let Some(ref item) = sep2 {
        items.push(item);
    }
    if let Some(ref item) = quit_with {
        items.push(item);
    }
    if let Some(ref item) = quit_keep {
        items.push(item);
    }

    if items.is_empty() {
        return;
    }
    if let Ok(menu) = Menu::with_items(app, &items) {
        let _ = tray.set_menu(Some(menu));
    }
}

fn create_tray_menu(app: &tauri::App) -> Result<TrayIcon, Box<dyn std::error::Error>> {
    let menu = Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "no_tunnels", "（无运行中的隧道）", false, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
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

    let tray = TrayIconBuilder::with_id("main")
        .menu(&menu)
        .icon(app.default_window_icon().unwrap().clone())
        .on_menu_event(move |app, event| {
            let eid = event.id.as_ref();
            match eid {
                "show" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                eid if eid.starts_with("copy_link_") => {
                    let tunnel_id = eid.strip_prefix("copy_link_").unwrap();
                    if let Some(processes) = app.try_state::<FrpcProcesses>() {
                        if let Ok(map) = processes.0.lock() {
                            if let Some(info) = map.get(tunnel_id) {
                                let _ = app.emit("tray-copy-link", &info.remote);
                            }
                        }
                    }
                }
                eid if eid.starts_with("stop_") => {
                    let id = eid.trim_start_matches("stop_").to_string();
                    let app = app.clone();
                    tauri::async_runtime::spawn_blocking(move || {
                        if let Err(error) = stop_managed_tunnel(&app, &id) {
                            let _ = app.emit_to("main", "tray-error", error);
                        }
                    });
                }
                "quit_with_frpc" => {
                    let app = app.clone();
                    tauri::async_runtime::spawn_blocking(move || {
                        match terminate_managed_tunnels(&app, false) {
                            Ok(()) => app.exit(0),
                            Err(error) => {
                                let _ = app.emit_to("main", "tray-error", error);
                            }
                        }
                    });
                }
                "quit_keep_frpc" => {
                    app.exit(0);
                }
                _ => {}
            }
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
    Ok(env!("CARGO_PKG_VERSION").to_string())
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
                                    tokio::time::sleep(tokio::time::Duration::from_millis(500))
                                        .await;
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
                debug_info["checks"]
                    .as_array_mut()
                    .unwrap()
                    .push(serde_json::json!({
                        "attempt": i,
                        "enabled": enabled,
                        "success": true
                    }));
            }
            Err(e) => {
                debug_info["checks"]
                    .as_array_mut()
                    .unwrap()
                    .push(serde_json::json!({
                        "attempt": i,
                        "enabled": null,
                        "success": false,
                        "error": e.to_string()
                    }));
            }
        }

        if i < 5 {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }

    // 计算一致性
    let checks = debug_info["checks"].as_array().unwrap();
    let successful_checks: Vec<_> = checks
        .iter()
        .filter(|c| c["success"].as_bool().unwrap_or(false))
        .collect();

    if successful_checks.len() >= 3 {
        let enabled_values: Vec<bool> = successful_checks
            .iter()
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

const SETTING_KEYS: &[&str] = &[
    "BYPASS_PROXY",
    "FRPC_DEBUG",
    "FRPC_FORCE_TLS",
    "USE_DOH",
    "DOH_ADDR",
    "QUICK_START_ENABLED",
];

fn config_value(config: &Config, key: &str) -> Option<String> {
    config
        .env_vars
        .as_ref()
        .and_then(|map| map.get(key))
        .cloned()
}

#[tauri::command]
fn set_env(key: String, value: String) -> Result<(), String> {
    if !SETTING_KEYS.contains(&key.as_str()) || value.chars().any(char::is_control) {
        return Err("未知或无效的设置项".into());
    }
    if key != "DOH_ADDR" && value != "true" && value != "false" {
        return Err("无效的设置值".into());
    }
    update_config(|cfg| {
        match key.as_str() {
            "FRPC_FORCE_TLS" => cfg.force_tls = Some(value == "true"),
            "USE_DOH" => cfg.use_doh = Some(value == "true"),
            "DOH_ADDR" => cfg.doh_addr = Some(value.clone()),
            _ => {}
        }
        cfg.env_vars
            .get_or_insert_with(HashMap::new)
            .insert(key, value);
    })
}

#[tauri::command]
fn get_env(key: String) -> Result<Option<String>, String> {
    if !SETTING_KEYS.contains(&key.as_str()) {
        return Err("未知的设置项".into());
    }
    Ok(config_value(&load_config()?, &key))
}

fn proxy_bypassed() -> bool {
    load_config()
        .ok()
        .and_then(|config| config_value(&config, "BYPASS_PROXY"))
        .as_deref()
        == Some("true")
}

#[tauri::command]
fn check_proxy_bypass() -> Result<bool, String> {
    Ok(config_value(&load_config()?, "BYPASS_PROXY").as_deref() == Some("true"))
}

/// 服务连通性探测结果
#[derive(Serialize)]
struct UrlProbeResult {
    success: bool,
    status: Option<u16>,
    latency_ms: u64,
    body: Option<String>,
    error: Option<String>,
}

/// 由后端探测任意 HTTP/HTTPS 地址。
/// 前端直接 fetch 会受 CORS 限制（例如 cv.naids.com 不返回 CORS 头），
/// 通过后端 reqwest 请求可避免误报“CORS限制或网络问题”。
#[command]
async fn probe_url(
    url: String,
    timeout_ms: Option<u64>,
    read_body: Option<bool>,
) -> Result<UrlProbeResult, String> {
    let url = url.trim().to_string();
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("仅支持 http/https 地址".to_string());
    }

    let timeout = std::time::Duration::from_millis(timeout_ms.unwrap_or(5000).clamp(500, 15_000));
    let client = api_proxy::client_builder(15)
        .timeout(timeout)
        .build()
        .map_err(|e| e.to_string())?;

    let start = std::time::Instant::now();
    match client.get(&url).send().await {
        Ok(mut response) => {
            let status = response.status().as_u16();
            let latency_ms = start.elapsed().as_millis() as u64;
            let body = if read_body.unwrap_or(false) {
                const BODY_LIMIT: usize = 200_000;
                let mut bytes = Vec::new();
                while let Some(chunk) = response
                    .chunk()
                    .await
                    .map_err(|e| e.without_url().to_string())?
                {
                    if bytes.len() + chunk.len() > BODY_LIMIT {
                        return Err("检测响应内容过大".into());
                    }
                    bytes.extend_from_slice(&chunk);
                }
                Some(String::from_utf8_lossy(&bytes).into_owned())
            } else {
                None
            };
            Ok(UrlProbeResult {
                success: true,
                status: Some(status),
                latency_ms,
                body,
                error: None,
            })
        }
        Err(e) => Ok(UrlProbeResult {
            success: false,
            status: None,
            latency_ms: start.elapsed().as_millis() as u64,
            body: None,
            error: Some(e.to_string()),
        }),
    }
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

static STARTUP_LINKS_READ: AtomicBool = AtomicBool::new(false);
static RESTORE_CHECKED: AtomicBool = AtomicBool::new(false);

#[command]
fn take_startup_links(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    if STARTUP_LINKS_READ.swap(true, Ordering::Relaxed) {
        return Ok(Vec::new());
    }
    app.deep_link()
        .get_current()
        .map(|urls| {
            urls.unwrap_or_default()
                .into_iter()
                .map(|url| url.to_string())
                .collect()
        })
        .map_err(|e| e.to_string())
}

#[command]
fn take_autostart_restore() -> bool {
    !RESTORE_CHECKED.swap(true, Ordering::Relaxed) && check_is_autostart()
}

#[command]
fn get_quick_start_enabled(app: tauri::AppHandle) -> Result<bool, String> {
    #[cfg(any(windows, target_os = "linux"))]
    {
        if config_value(&load_config()?, "QUICK_START_ENABLED").as_deref() == Some("false") {
            return Ok(false);
        }
        app.deep_link()
            .is_registered("openfrp")
            .map_err(|e| e.to_string())
    }
    #[cfg(target_os = "macos")]
    return Ok(true);
}

#[command]
fn set_quick_start_enabled(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    #[cfg(any(windows, target_os = "linux"))]
    {
        let link = app.deep_link();
        let was_enabled = link.is_registered("openfrp").map_err(|e| e.to_string())?;
        if was_enabled != enabled {
            if enabled {
                link.register("openfrp")
            } else {
                link.unregister("openfrp")
            }
            .map_err(|e| e.to_string())?;
        }
        if let Err(error) = set_env("QUICK_START_ENABLED".into(), enabled.to_string()) {
            if was_enabled != enabled {
                let _ = if was_enabled {
                    link.register("openfrp")
                } else {
                    link.unregister("openfrp")
                };
            }
            return Err(error);
        }
        Ok(())
    }
    #[cfg(target_os = "macos")]
    return Err("macOS 由系统管理快速启动链接".into());
}

fn main() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_fs::init())
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
        .plugin(tauri_plugin_deep_link::init())
        .setup(|app| {
            init_app_directory(app)?;
            if let Err(error) = update_config(|_| {}) {
                eprintln!("无法初始化配置: {error}");
            }
            #[cfg(any(windows, target_os = "linux"))]
            if config_value(&load_config().unwrap_or_default(), "QUICK_START_ENABLED").as_deref()
                != Some("false")
            {
                if let Err(error) = app.deep_link().register("openfrp") {
                    eprintln!("快速启动链接注册失败: {error}");
                }
            }

            let _tray = create_tray_menu(app)?;

            #[cfg(target_os = "windows")]
            {
                if let Err(e) = register_app_for_notifications(app) {
                    eprintln!("Failed to register app for notifications: {}", e);
                }
            }

            Ok(())
        })
        .manage(FrpcProcesses::default())
        .manage(oauth::OAuthAttempt::default())
        .invoke_handler(tauri::generate_handler![
            check_frpc_status,
            download_frpc,
            start_frpc_instance,
            stop_frpc_instance,
            oauth::begin_local_oauth_login,
            oauth::start_local_oauth_server,
            oauth::cancel_local_oauth_server,
            check_frpc,
            get_frpc_file,
            get_frpc_cli_version,
            list_frpc_instances,
            kill_all_processes,
            exit_app,
            take_autostart_restore,
            take_startup_links,
            get_quick_start_enabled,
            set_quick_start_enabled,
            get_cpl_version,
            toggle_auto_start,
            check_auto_start,
            debug_auto_start,
            set_env,
            get_env,
            check_proxy_bypass,
            probe_url,
            oauth::oauth_callback,
            update::check_update,
            get_build_info,
            get_system_info,
            get_detailed_system_info,
            api_proxy::proxy_api,
            api_proxy::get_announcement,
            get_app_data_dir,
            open_app_data_dir,
            local_ports::get_local_ports,
            update::download_and_install_update,
            tcp_ping,
            argo_access::argo_request_login,
            argo_access::argo_wait_authorization,
            argo_access::argo_cancel_wait,
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    app.run(|_, _| {});
}

#[cfg(target_os = "windows")]
fn register_app_for_notifications(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use winreg::enums::*;
    use winreg::RegKey;

    let app_path = std::env::current_exe()?;
    let app_name = app
        .config()
        .product_name
        .as_deref()
        .unwrap_or("OpenFrp Launcher");
    let app_id = &app.config().identifier;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let path = format!("SOFTWARE\\Classes\\AppUserModelId\\{}", app_id);
    let (key, _) = hkcu.create_subkey(&path)?;

    key.set_value("DisplayName", &app_name)?;
    key.set_value("IconPath", &app_path.to_string_lossy().to_string())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zip_with_entry(name: &str, data: &[u8]) -> Vec<u8> {
        let mut zip = zip::ZipWriter::new(io::Cursor::new(Vec::new()));
        zip.start_file(name, zip::write::FileOptions::default())
            .unwrap();
        zip.write_all(data).unwrap();
        zip.finish().unwrap().into_inner()
    }

    #[test]
    fn frpc_extracts_flat_and_nested_windows_archives() {
        for entry in ["frpc_windows_amd64.exe", "frpc_windows_amd64/frpc.exe"] {
            let zip = zip_with_entry(entry, b"binary");
            let mut output = Vec::new();
            extract_frpc(&zip, "frpc_windows_amd64.exe", &mut output).unwrap();
            assert_eq!(output, b"binary");
        }
    }

    #[test]
    fn frpc_extracts_a_nested_unix_binary() {
        let gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        let mut tar = tar::Builder::new(gzip);
        let mut header = tar::Header::new_gnu();
        header.set_size(6);
        header.set_mode(0o755);
        header.set_cksum();
        tar.append_data(&mut header, "frpc_darwin_arm64/frpc", &b"binary"[..])
            .unwrap();
        let archive = tar.into_inner().unwrap().finish().unwrap();
        let mut output = Vec::new();
        extract_frpc(&archive, "frpc_darwin_arm64", &mut output).unwrap();
        assert_eq!(output, b"binary");
    }

    #[test]
    fn missing_frpc_entry_leaves_the_installed_file_in_place() {
        let directory = tempfile::tempdir().unwrap();
        let installed = directory.path().join("frpc_windows_amd64.exe");
        fs::write(&installed, b"old").unwrap();
        let mut staged = tempfile::NamedTempFile::new_in(directory.path()).unwrap();
        let zip = zip_with_entry("release-notes.txt", b"no binary");
        assert!(extract_frpc(&zip, "frpc_windows_amd64.exe", &mut staged).is_err());
        assert_eq!(fs::read(&installed).unwrap(), b"old");
    }

    #[test]
    fn temporary_log_keeps_independent_read_and_write_handles() {
        let directory = tempfile::tempdir().unwrap();
        let (mut reader, mut stdout, mut stderr) = frpc_log_handles(directory.path()).unwrap();
        stdout.write_all(b"stdout\n").unwrap();
        let mut first_line = [0; 7];
        reader.read_exact(&mut first_line).unwrap();
        assert_eq!(&first_line, b"stdout\n");
        stderr.write_all(b"stderr\n").unwrap();
        drop(stdout);
        drop(stderr);
        let mut rest = String::new();
        reader.read_to_string(&mut rest).unwrap();
        assert_eq!(rest, "stderr\n");
    }

    #[tokio::test]
    async fn tcp_ping_counts_sub_millisecond_local_connections() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let result = tcp_ping("127.0.0.1".into(), listener.local_addr().unwrap().port())
            .await
            .unwrap();
        assert_eq!(result["success"], true);
        assert!(result["latency_ms"].as_f64().unwrap() >= 0.0);
    }
}
