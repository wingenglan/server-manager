use crate::domain::ssh::SshConnectionManager;
use crate::errors::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationsSnapshot {
    pub processes: Vec<ProcessInfo>,
    pub ports: Vec<PortInfo>,
    pub services: Vec<ServiceInfo>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessInfo {
    pub pid: u32,
    pub ppid: u32,
    pub user: String,
    pub state: String,
    pub cpu_percent: f64,
    pub memory_percent: f64,
    pub rss_bytes: u64,
    pub elapsed_seconds: u64,
    pub name: String,
    pub command: String,
    pub systemd_unit: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortInfo {
    pub protocol: String,
    pub local_address: String,
    pub port: u16,
    pub pid: Option<u32>,
    pub process_name: Option<String>,
    pub ipv6: bool,
    pub process_visible: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceInfo {
    pub name: String,
    pub load: String,
    pub active: String,
    pub sub: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminateProcessInput {
    pub server_id: String,
    pub pid: u32,
    pub port: Option<u16>,
    #[serde(default)]
    pub force: bool,
    #[serde(default)]
    pub privileged: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminationResult {
    pub pid: u32,
    pub signal: String,
    pub process_exited: bool,
    pub port_released: Option<bool>,
}

pub async fn snapshot(
    ssh: &SshConnectionManager,
    server_id: &str,
) -> AppResult<OperationsSnapshot> {
    let (processes, ports, services) = tokio::join!(
        ssh.execute(
            server_id,
            "ps -eo pid=,ppid=,user=,stat=,pcpu=,pmem=,rss=,etimes=,comm=,args=",
            Duration::from_secs(30)
        ),
        ssh.execute(server_id, "ss -H -lntup", Duration::from_secs(30)),
        ssh.execute(
            server_id,
            "systemctl list-units --type=service --all --no-legend --no-pager --plain",
            Duration::from_secs(30)
        )
    );
    let processes = processes?;
    let ports = ports?;
    let services = services?;
    if processes.exit_code != 0 {
        return Err(remote_failure(server_id, "ps", processes.stderr));
    }
    if ports.exit_code != 0 {
        return Err(remote_failure(server_id, "ss", ports.stderr));
    }
    Ok(OperationsSnapshot {
        processes: parse_processes(&processes.stdout),
        ports: parse_ports(&ports.stdout),
        services: if services.exit_code == 0 {
            parse_services(&services.stdout)
        } else {
            Vec::new()
        },
    })
}

pub async fn terminate(
    ssh: &SshConnectionManager,
    input: TerminateProcessInput,
) -> AppResult<TerminationResult> {
    if input.pid <= 1 {
        return Err(
            AppError::new("DANGEROUS_PROCESS", "security", "禁止结束 PID 0 或 PID 1")
                .for_server(&input.server_id)
                .fatal(),
        );
    }
    let signal = if input.force { "KILL" } else { "TERM" };
    let command = format!("kill -{signal} -- {}", input.pid);
    let result = if input.privileged {
        ssh.execute_privileged(&input.server_id, &command, Duration::from_secs(10))
            .await?
    } else {
        ssh.execute(&input.server_id, &command, Duration::from_secs(10))
            .await?
    };
    if result.exit_code != 0 {
        return Err(
            AppError::new("PERMISSION_DENIED", "process", "无法向目标进程发送信号")
                .details(result.stderr)
                .for_server(&input.server_id)
                .suggestion("检查进程权限，或通过已配置的 sudo 模式重试"),
        );
    }
    tokio::time::sleep(Duration::from_millis(900)).await;
    let verify = ssh
        .execute(
            &input.server_id,
            &format!("kill -0 -- {} 2>/dev/null", input.pid),
            Duration::from_secs(10),
        )
        .await?;
    let process_exited = verify.exit_code != 0;
    let port_released = if let Some(port) = input.port {
        let ports = ssh
            .execute(&input.server_id, "ss -H -lntup", Duration::from_secs(10))
            .await?;
        Some(
            !parse_ports(&ports.stdout)
                .iter()
                .any(|value| value.port == port),
        )
    } else {
        None
    };
    Ok(TerminationResult {
        pid: input.pid,
        signal: signal.to_string(),
        process_exited,
        port_released,
    })
}

pub async fn service_action(
    ssh: &SshConnectionManager,
    server_id: &str,
    service: &str,
    action: &str,
) -> AppResult<()> {
    if !valid_unit(service) || !matches!(action, "start" | "stop" | "restart") {
        return Err(AppError::new(
            "VALIDATION_FAILED",
            "validation",
            "systemd 服务名或操作无效",
        ));
    }
    let command = format!(
        "systemctl {action} -- {}",
        crate::security::shell_escape(service)
    );
    let result = ssh
        .execute_privileged(server_id, &command, Duration::from_secs(30))
        .await?;
    if result.exit_code != 0 {
        return Err(
            AppError::new("REMOTE_COMMAND_FAILED", "systemd", "systemd 服务操作失败")
                .details(result.stderr)
                .for_server(server_id),
        );
    }
    Ok(())
}

pub fn parse_processes(output: &str) -> Vec<ProcessInfo> {
    output
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let pid = fields.next()?.parse().ok()?;
            let ppid = fields.next()?.parse().ok()?;
            let user = fields.next()?.to_string();
            let state = fields.next()?.to_string();
            let cpu_percent = fields.next()?.parse().unwrap_or(0.0);
            let memory_percent = fields.next()?.parse().unwrap_or(0.0);
            let rss_bytes = fields.next()?.parse::<u64>().unwrap_or(0) * 1024;
            let elapsed_seconds = fields.next()?.parse().unwrap_or(0);
            let name = fields.next()?.to_string();
            let command = fields.collect::<Vec<_>>().join(" ");
            Some(ProcessInfo {
                pid,
                ppid,
                user,
                state,
                cpu_percent,
                memory_percent,
                rss_bytes,
                elapsed_seconds,
                name,
                systemd_unit: infer_unit(&command),
                command,
            })
        })
        .collect()
}

pub fn parse_ports(output: &str) -> Vec<PortInfo> {
    output
        .lines()
        .filter_map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() < 5 {
                return None;
            }
            let protocol = fields[0].to_string();
            let local = fields[4];
            let (local_address, port) = split_socket(local)?;
            let process_text = fields.get(6..).unwrap_or_default().join(" ");
            let pid = extract_number(&process_text, "pid=");
            let process_name = process_text
                .split("((\"")
                .nth(1)
                .and_then(|value| value.split('"').next())
                .map(ToString::to_string);
            Some(PortInfo {
                protocol,
                local_address: local_address.to_string(),
                port,
                pid,
                process_name,
                ipv6: local_address.contains(':'),
                process_visible: pid.is_some(),
            })
        })
        .collect()
}

pub fn parse_services(output: &str) -> Vec<ServiceInfo> {
    output
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            Some(ServiceInfo {
                name: fields.next()?.trim_start_matches('●').to_string(),
                load: fields.next()?.to_string(),
                active: fields.next()?.to_string(),
                sub: fields.next()?.to_string(),
                description: fields.collect::<Vec<_>>().join(" "),
            })
        })
        .collect()
}

fn split_socket(value: &str) -> Option<(&str, u16)> {
    let index = value.rfind(':')?;
    Some((&value[..index], value[index + 1..].parse().ok()?))
}

fn extract_number(value: &str, marker: &str) -> Option<u32> {
    let start = value.find(marker)? + marker.len();
    let end = value[start..]
        .find(|character: char| !character.is_ascii_digit())
        .map(|offset| start + offset)
        .unwrap_or(value.len());
    value[start..end].parse().ok()
}

fn infer_unit(command: &str) -> Option<String> {
    command
        .split_whitespace()
        .find(|value| value.ends_with(".service"))
        .map(|value| {
            value
                .trim_matches(|character: char| {
                    !character.is_alphanumeric()
                        && character != '.'
                        && character != '@'
                        && character != '-'
                        && character != '_'
                })
                .to_string()
        })
}

fn valid_unit(value: &str) -> bool {
    value.ends_with(".service")
        && !value.is_empty()
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '@' | '-' | '_' | '\\')
        })
}

fn remote_failure(server_id: &str, command: &str, stderr: String) -> AppError {
    AppError::new(
        "REMOTE_COMMAND_FAILED",
        "remote_command",
        format!("远程命令 {command} 执行失败"),
    )
    .details(stderr)
    .for_server(server_id)
}

#[cfg(test)]
mod tests {
    use super::{parse_ports, parse_processes, parse_services};

    #[test]
    fn parses_process_fixture() {
        let values = parse_processes(include_str!("../../../../fixtures/ps-linux.txt"));
        assert_eq!(values[0].pid, 1);
        assert_eq!(values[1].rss_bytes, 42_000 * 1024);
        assert!(values[1].command.contains("server.js"));
    }

    #[test]
    fn parses_ss_ipv4_and_ipv6() {
        let values = parse_ports(include_str!("../../../../fixtures/ss-listen.txt"));
        assert_eq!(values[0].port, 8080);
        assert_eq!(values[0].pid, Some(1234));
        assert_eq!(values[0].process_name.as_deref(), Some("node"));
        assert!(values[1].ipv6);
        assert!(!values[1].process_visible);
    }

    #[test]
    fn parses_systemd_units() {
        let values = parse_services(include_str!("../../../../fixtures/systemd-services.txt"));
        assert_eq!(values[0].name, "nginx.service");
        assert_eq!(values[1].active, "failed");
    }
}
