use chrono::Local;
use std::process::Command;

fn main() {
    // 获取构建时间
    let now = Local::now();
    let build_time = now.format("%y%m%d_%H%M").to_string();

    // 获取 Git 提交 ID（无 git 环境或非 git 仓库时容错为 "unknown"，避免 panic 中断构建）
    let git_hash = Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // 组合构建信息
    let build_info = format!("{}_{}", build_time, git_hash);

    // 注入环境变量
    println!("cargo:rustc-env=BUILD_TIME={}", build_info);
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
    println!("cargo:rerun-if-changed=.git/HEAD");
    tauri_build::build();
}
