use chrono::Local;
use std::process::Command;

fn main() {
    // 获取构建时间
    let now = Local::now();
    let build_time = now.format("%y%m%d_%H%M").to_string();

    // 获取 Git 提交 ID
    let output = Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
        .unwrap();
    let git_hash = String::from_utf8(output.stdout).unwrap();
    let git_hash = git_hash.trim();

    // 组合构建信息
    let build_info = format!("{}_{}", build_time, git_hash);

    // 注入环境变量
    println!("cargo:rustc-env=BUILD_TIME={}", build_info);
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
    println!("cargo:rerun-if-changed=.git/HEAD");
    tauri_build::build();
}
