use serde::Serialize;
use std::process::Command;

#[derive(Serialize)]
struct LocalPort {
    port: u16,
    pid: Option<u32>,
    process: Option<String>,
}

#[derive(Default, Serialize)]
pub struct LocalPorts {
    tcp: Vec<LocalPort>,
    udp: Vec<LocalPort>,
}

impl LocalPorts {
    fn push(&mut self, protocol: &str, port: LocalPort) {
        if port.port == 0 {
            return;
        }
        match protocol.to_ascii_lowercase().as_str() {
            "tcp" => self.tcp.push(port),
            "udp" => self.udp.push(port),
            _ => {}
        }
    }
}

fn endpoint_port(address: &str) -> Option<u16> {
    address.rsplit_once(':')?.1.parse().ok()
}

#[tauri::command]
pub async fn get_local_ports() -> Result<LocalPorts, String> {
    tauri::async_runtime::spawn_blocking(scan_ports)
        .await
        .map_err(|error| error.to_string())?
}

#[cfg(windows)]
fn scan_ports() -> Result<LocalPorts, String> {
    use std::os::windows::process::CommandExt;
    let output = Command::new("netstat")
        .args(["-ano"])
        .creation_flags(crate::CREATE_NO_WINDOW)
        .output()
        .map_err(|error| format!("扫描端口失败: {error}"))?;
    if !output.status.success() {
        return Err(format!("扫描端口失败 ({})", output.status));
    }
    let names = Command::new("tasklist")
        .args(["/fo", "csv", "/nh"])
        .creation_flags(crate::CREATE_NO_WINDOW)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).into_owned())
        .unwrap_or_default();
    Ok(parse_windows_ports(
        &String::from_utf8_lossy(&output.stdout),
        &names,
    ))
}

#[cfg(any(windows, test))]
fn parse_windows_ports(netstat: &str, tasklist: &str) -> LocalPorts {
    use std::collections::HashMap;
    let names: HashMap<u32, &str> = tasklist
        .lines()
        .filter_map(|line| {
            let mut fields = line.trim_matches('"').split("\",\"");
            let name = fields.next()?;
            Some((fields.next()?.parse().ok()?, name))
        })
        .collect();

    let mut ports = LocalPorts::default();
    for line in netstat.lines() {
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields.len() < 4 {
            continue;
        }
        let protocol = fields[0];
        // TCP 的监听套接字没有远端端口，避免依赖系统语言中的状态字符串。
        if protocol == "TCP" && endpoint_port(fields[2]) != Some(0) {
            continue;
        }
        if let Some(port) = endpoint_port(fields[1]) {
            let pid = fields.last().and_then(|value| value.parse().ok());
            let process = pid
                .and_then(|pid| names.get(&pid))
                .map(|name| name.to_string());
            ports.push(protocol, LocalPort { port, pid, process });
        }
    }
    ports
}

#[cfg(target_os = "linux")]
fn scan_ports() -> Result<LocalPorts, String> {
    if let Ok(output) = Command::new("ss").args(["-H", "-lntup"]).output() {
        if output.status.success() {
            return Ok(parse_linux_ss(&String::from_utf8_lossy(&output.stdout)));
        }
    }

    // 最小安装的 Linux 可能没有 iproute2，使用内核提供的监听信息。
    let mut ports = LocalPorts::default();
    let mut readable = false;
    for (table, protocol, state) in [
        ("tcp", "tcp", "0A"),
        ("tcp6", "tcp", "0A"),
        ("udp", "udp", "07"),
        ("udp6", "udp", "07"),
    ] {
        let Ok(content) = std::fs::read_to_string(format!("/proc/net/{table}")) else {
            continue;
        };
        readable = true;
        parse_linux_proc(&mut ports, &content, protocol, state);
    }
    if readable {
        Ok(ports)
    } else {
        Err("无法读取系统监听端口".into())
    }
}

#[cfg(any(target_os = "linux", test))]
fn parse_linux_ss(output: &str) -> LocalPorts {
    let mut ports = LocalPorts::default();
    for line in output.lines() {
        let mut fields = line.split_whitespace();
        let protocol = match (fields.next(), fields.next()) {
            (Some("tcp"), Some("LISTEN")) => "tcp",
            (Some("udp"), Some("UNCONN")) => "udp",
            _ => continue,
        };
        let Some(port) = fields.nth(2).and_then(endpoint_port) else {
            continue;
        };
        let identity = line
            .split_once("users:((\"")
            .and_then(|(_, value)| value.split_once("\",pid="));
        let process = identity.map(|(name, _)| name.to_string());
        let pid = identity.and_then(|(_, tail)| tail.split(',').next()?.parse().ok());
        ports.push(protocol, LocalPort { port, pid, process });
    }
    ports
}

#[cfg(any(target_os = "linux", test))]
fn parse_linux_proc(ports: &mut LocalPorts, content: &str, protocol: &str, state: &str) {
    for line in content.lines().skip(1) {
        let mut fields = line.split_whitespace();
        let address = fields.nth(1);
        if fields.nth(1) != Some(state) {
            continue;
        }
        if let Some(port) =
            address.and_then(|address| u16::from_str_radix(address.rsplit_once(':')?.1, 16).ok())
        {
            ports.push(
                protocol,
                LocalPort {
                    port,
                    pid: None,
                    process: None,
                },
            );
        }
    }
}

#[cfg(target_os = "macos")]
fn scan_ports() -> Result<LocalPorts, String> {
    let mut ports = LocalPorts::default();
    for (protocol, args) in [
        ("tcp", &["-nP", "-Fpcn", "-iTCP", "-sTCP:LISTEN"][..]),
        ("udp", &["-nP", "-Fpcn", "-iUDP"][..]),
    ] {
        let output = Command::new("/usr/sbin/lsof")
            .args(args)
            .output()
            .map_err(|error| format!("扫描端口失败: {error}"))?;
        if !output.status.success() && output.stdout.is_empty() && !output.stderr.is_empty() {
            return Err(format!("扫描端口失败 ({})", output.status));
        }
        parse_macos_lsof(
            &mut ports,
            &String::from_utf8_lossy(&output.stdout),
            protocol,
        );
    }
    Ok(ports)
}

#[cfg(any(target_os = "macos", test))]
fn parse_macos_lsof(ports: &mut LocalPorts, output: &str, protocol: &str) {
    let mut pid = None;
    let mut process = None;
    for line in output.lines() {
        if let Some(value) = line.strip_prefix('p') {
            pid = value.parse().ok();
            process = None;
        } else if let Some(value) = line.strip_prefix('c') {
            process = Some(value.to_string());
        } else if let Some(endpoint) = line.strip_prefix('n').filter(|value| !value.contains("->"))
        {
            if let Some(port) = endpoint_port(endpoint) {
                ports.push(
                    protocol,
                    LocalPort {
                        port,
                        pid,
                        process: process.clone(),
                    },
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_ports_include_listeners_and_exclude_connected_clients() {
        let ports = parse_windows_ports(
            "TCP 0.0.0.0:8000 0.0.0.0:0 LISTENING 42\nTCP [::]:8120 [::]:0 LISTENING 42\nTCP 127.0.0.1:8176 127.0.0.1:8000 ESTABLISHED 42\nUDP [::]:19132 *:* 7",
            "\"Game Server.exe\",\"42\",\"Console\"",
        );
        assert_eq!(
            ports.tcp.iter().map(|port| port.port).collect::<Vec<_>>(),
            [8000, 8120]
        );
        assert_eq!(ports.tcp[0].process.as_deref(), Some("Game Server.exe"));
        assert_eq!(ports.udp[0].port, 19132);
    }

    #[test]
    fn linux_ss_keeps_listening_sockets_with_optional_process_details() {
        let ports = parse_linux_ss(concat!(
            "udp UNCONN 0 0 0.0.0.0:19132 0.0.0.0:*\n",
            "tcp LISTEN 0 128 [fe80::1%eth0]:8120 [::]:* users:((\"example app\",pid=42,fd=3))\n",
            "tcp ESTAB 0 0 127.0.0.1:8176 127.0.0.1:8120\n",
            "tcp LISTEN 0 0 invalid\n",
        ));
        assert_eq!(ports.tcp.len(), 1);
        assert_eq!(ports.tcp[0].port, 8120);
        assert_eq!(ports.tcp[0].pid, Some(42));
        assert_eq!(ports.tcp[0].process.as_deref(), Some("example app"));
        assert_eq!(ports.udp.len(), 1);
        assert_eq!(ports.udp[0].port, 19132);
        assert_eq!(ports.udp[0].pid, None);
    }

    #[test]
    fn linux_proc_filters_socket_states_and_decodes_hexadecimal_ports() {
        let mut ports = LocalPorts::default();
        parse_linux_proc(
            &mut ports,
            "sl local_address rem_address st\n0: 0100007F:1FB8 00000000:0000 0A\n1: 0100007F:1FF0 0100007F:1FB8 01\n",
            "tcp",
            "0A",
        );
        parse_linux_proc(
            &mut ports,
            "sl local_address rem_address st\n0: 00000000000000000000000000000000:4ABC 00000000000000000000000000000000:0000 07\n1: 00000000:0000 00000000:0000 07\n",
            "udp",
            "07",
        );
        assert_eq!(
            ports.tcp.iter().map(|port| port.port).collect::<Vec<_>>(),
            [8120]
        );
        assert_eq!(
            ports.udp.iter().map(|port| port.port).collect::<Vec<_>>(),
            [19132]
        );
    }

    #[test]
    fn macos_lsof_keeps_process_records_separate_and_ignores_connected_sockets() {
        let mut ports = LocalPorts::default();
        parse_macos_lsof(
            &mut ports,
            "p42\ncexample app\nn*:8120\np7\nn[::1]:8121\n",
            "tcp",
        );
        parse_macos_lsof(
            &mut ports,
            "p7\ncother app\nn*:19132\nn10.0.0.2:55321->10.0.0.3:53\n",
            "udp",
        );
        assert_eq!(ports.tcp.len(), 2);
        assert_eq!(ports.tcp[0].process.as_deref(), Some("example app"));
        assert_eq!(ports.tcp[1].pid, Some(7));
        assert_eq!(ports.tcp[1].process, None);
        assert_eq!(ports.tcp[1].port, 8121);
        assert_eq!(
            ports.udp.iter().map(|port| port.port).collect::<Vec<_>>(),
            [19132]
        );
    }
}
