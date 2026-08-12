use crate::domain::ssh::SshConnectionManager;
use crate::errors::{AppError, AppResult};
use serde::Serialize;
use std::collections::HashMap;
use std::time::Duration;

const PROBE_COMMAND: &str = r#"
printf '__OS__\n'; cat /etc/os-release 2>/dev/null || true
printf '__HOSTNAME__\n'; hostname 2>/dev/null || true
printf '__UNAME__\n'; uname -srm 2>/dev/null || true
printf '__ARCH__\n'; uname -m 2>/dev/null || true
printf '__UPTIME__\n'; cut -d. -f1 /proc/uptime 2>/dev/null || printf '0\n'
printf '__CPU_MODEL__\n'; awk -F: '/model name|Hardware/ {gsub(/^[ \t]+/,"",$2); print $2; exit}' /proc/cpuinfo 2>/dev/null
printf '__CORES__\n'; getconf _NPROCESSORS_ONLN 2>/dev/null || printf '1\n'
printf '__MEMINFO__\n'; cat /proc/meminfo 2>/dev/null || true
printf '__LOAD__\n'; cat /proc/loadavg 2>/dev/null || true
printf '__DF__\n'; df -B1 -P 2>/dev/null || true
printf '__CPU_A__\n'; head -n1 /proc/stat 2>/dev/null || true
sleep 0.25
printf '__CPU_B__\n'; head -n1 /proc/stat 2>/dev/null || true
printf '__DOCKER__\n'; if command -v docker >/dev/null 2>&1; then printf 'installed\t'; docker version --format '{{.Server.Version}}' 2>/dev/null || docker --version 2>/dev/null || true; if command -v systemctl >/dev/null 2>&1 && systemctl is-active --quiet docker 2>/dev/null; then printf 'running\n'; else printf 'stopped\n'; fi; else printf 'missing\n'; fi
printf '__NGINX__\n'; if command -v nginx >/dev/null 2>&1; then printf 'installed\t'; nginx -v 2>&1 | sed 's#nginx version: nginx/##'; if command -v systemctl >/dev/null 2>&1 && systemctl is-active --quiet nginx 2>/dev/null; then printf 'running\n'; else printf 'stopped\n'; fi; else printf 'missing\n'; fi
printf '__CAPABILITIES__\n'; for name in systemctl sudo docker nginx ss ip journalctl lsof tar gzip; do if command -v "$name" >/dev/null 2>&1; then printf '%s=1\n' "$name"; else printf '%s=0\n' "$name"; fi; done
"#;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemOverview {
    pub hostname: String,
    pub os_name: String,
    pub os_version: String,
    pub kernel: String,
    pub architecture: String,
    pub uptime_seconds: u64,
    pub cpu_model: String,
    pub logical_cores: u32,
    pub cpu_usage_percent: Option<f64>,
    pub load: [f64; 3],
    pub memory_total_bytes: u64,
    pub memory_available_bytes: u64,
    pub swap_total_bytes: u64,
    pub swap_free_bytes: u64,
    pub disks: Vec<DiskUsage>,
    pub docker: RuntimeStatus,
    pub nginx: RuntimeStatus,
    pub capabilities: HashMap<String, bool>,
    pub sampled_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskUsage {
    pub mount: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub usage_percent: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeStatus {
    pub installed: bool,
    pub running: bool,
    pub version: Option<String>,
}

pub async fn probe(manager: &SshConnectionManager, server_id: &str) -> AppResult<SystemOverview> {
    let result = manager
        .execute(server_id, PROBE_COMMAND, Duration::from_secs(15))
        .await?;
    if result.exit_code != 0 && result.stdout.is_empty() {
        return Err(
            AppError::new("REMOTE_COMMAND_FAILED", "metrics", "系统状态探测失败")
                .details(result.stderr)
                .for_server(server_id),
        );
    }
    parse_overview(&result.stdout).map_err(|error| error.for_server(server_id))
}

pub fn parse_overview(output: &str) -> AppResult<SystemOverview> {
    let sections = split_sections(output);
    let os_release = parse_key_values(section(&sections, "OS"));
    let meminfo = parse_meminfo(section(&sections, "MEMINFO"));
    let uname: Vec<_> = section(&sections, "UNAME").split_whitespace().collect();
    let load_values: Vec<f64> = section(&sections, "LOAD")
        .split_whitespace()
        .take(3)
        .filter_map(|value| value.parse().ok())
        .collect();
    Ok(SystemOverview {
        hostname: first_line(section(&sections, "HOSTNAME"))
            .unwrap_or("unknown")
            .to_string(),
        os_name: os_release
            .get("NAME")
            .cloned()
            .unwrap_or_else(|| "Linux".to_string()),
        os_version: os_release.get("VERSION_ID").cloned().unwrap_or_default(),
        kernel: uname.get(1).copied().unwrap_or("unknown").to_string(),
        architecture: first_line(section(&sections, "ARCH"))
            .or_else(|| uname.get(2).copied())
            .unwrap_or("unknown")
            .to_string(),
        uptime_seconds: first_line(section(&sections, "UPTIME"))
            .and_then(|value| value.parse().ok())
            .unwrap_or(0),
        cpu_model: first_line(section(&sections, "CPU_MODEL"))
            .unwrap_or("Unknown CPU")
            .to_string(),
        logical_cores: first_line(section(&sections, "CORES"))
            .and_then(|value| value.parse().ok())
            .unwrap_or(1),
        cpu_usage_percent: cpu_delta(section(&sections, "CPU_A"), section(&sections, "CPU_B")),
        load: [
            *load_values.first().unwrap_or(&0.0),
            *load_values.get(1).unwrap_or(&0.0),
            *load_values.get(2).unwrap_or(&0.0),
        ],
        memory_total_bytes: meminfo.get("MemTotal").copied().unwrap_or(0) * 1024,
        memory_available_bytes: meminfo
            .get("MemAvailable")
            .or_else(|| meminfo.get("MemFree"))
            .copied()
            .unwrap_or(0)
            * 1024,
        swap_total_bytes: meminfo.get("SwapTotal").copied().unwrap_or(0) * 1024,
        swap_free_bytes: meminfo.get("SwapFree").copied().unwrap_or(0) * 1024,
        disks: parse_df(section(&sections, "DF")),
        docker: parse_runtime(section(&sections, "DOCKER")),
        nginx: parse_runtime(section(&sections, "NGINX")),
        capabilities: parse_capabilities(section(&sections, "CAPABILITIES")),
        sampled_at: chrono::Utc::now().to_rfc3339(),
    })
}

fn split_sections(output: &str) -> HashMap<String, String> {
    let mut sections = HashMap::new();
    let mut current: Option<String> = None;
    for line in output.lines() {
        if line.starts_with("__") && line.ends_with("__") {
            current = Some(line.trim_matches('_').to_string());
            sections
                .entry(current.clone().unwrap_or_default())
                .or_insert_with(String::new);
        } else if let Some(name) = &current {
            let value = sections.entry(name.clone()).or_insert_with(String::new);
            value.push_str(line);
            value.push('\n');
        }
    }
    sections
}

fn section<'a>(sections: &'a HashMap<String, String>, name: &str) -> &'a str {
    sections.get(name).map(String::as_str).unwrap_or("")
}
fn first_line(input: &str) -> Option<&str> {
    input.lines().map(str::trim).find(|value| !value.is_empty())
}

pub fn parse_key_values(input: &str) -> HashMap<String, String> {
    input
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| {
            (
                key.trim().to_string(),
                value.trim().trim_matches('"').to_string(),
            )
        })
        .collect()
}

pub fn parse_meminfo(input: &str) -> HashMap<String, u64> {
    input
        .lines()
        .filter_map(|line| {
            let (key, value) = line.split_once(':')?;
            Some((
                key.to_string(),
                value.split_whitespace().next()?.parse().ok()?,
            ))
        })
        .collect()
}

pub fn cpu_delta(first: &str, second: &str) -> Option<f64> {
    fn values(line: &str) -> Option<Vec<u64>> {
        let values: Vec<u64> = line
            .split_whitespace()
            .skip(1)
            .filter_map(|value| value.parse().ok())
            .collect();
        (values.len() >= 4).then_some(values)
    }
    let first = values(first_line(first)?)?;
    let second = values(first_line(second)?)?;
    let first_idle = first.get(3).copied().unwrap_or(0) + first.get(4).copied().unwrap_or(0);
    let second_idle = second.get(3).copied().unwrap_or(0) + second.get(4).copied().unwrap_or(0);
    let first_total: u64 = first.iter().sum();
    let second_total: u64 = second.iter().sum();
    let total_delta = second_total.saturating_sub(first_total);
    if total_delta == 0 {
        return None;
    }
    let idle_delta = second_idle.saturating_sub(first_idle);
    Some(
        ((total_delta.saturating_sub(idle_delta)) as f64 / total_delta as f64 * 100.0)
            .clamp(0.0, 100.0),
    )
}

pub fn parse_df(input: &str) -> Vec<DiskUsage> {
    input
        .lines()
        .skip(1)
        .filter_map(|line| {
            let values: Vec<_> = line.split_whitespace().collect();
            if values.len() < 6 {
                return None;
            }
            Some(DiskUsage {
                total_bytes: values[1].parse().ok()?,
                used_bytes: values[2].parse().ok()?,
                usage_percent: values[4].trim_end_matches('%').parse().ok()?,
                mount: values[5..].join(" "),
            })
        })
        .collect()
}

fn parse_runtime(input: &str) -> RuntimeStatus {
    let value = input.trim();
    if value.starts_with("missing") {
        return RuntimeStatus {
            installed: false,
            running: false,
            version: None,
        };
    }
    let running = value
        .lines()
        .last()
        .is_some_and(|line| line.trim() == "running");
    let version = value
        .strip_prefix("installed\t")
        .map(|value| value.lines().next().unwrap_or("").trim())
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    RuntimeStatus {
        installed: true,
        running,
        version,
    }
}

fn parse_capabilities(input: &str) -> HashMap<String, bool> {
    input
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(name, value)| (format!("has_{}", name), value == "1"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{cpu_delta, parse_df, parse_key_values, parse_meminfo};

    #[test]
    fn parses_os_release_quotes() {
        let values = parse_key_values(include_str!("../../../../fixtures/os-release-ubuntu.txt"));
        assert_eq!(values["NAME"], "Ubuntu");
        assert_eq!(values["VERSION_ID"], "24.04");
    }

    #[test]
    fn parses_memory_kibibytes() {
        let values = parse_meminfo(include_str!("../../../../fixtures/proc-meminfo.txt"));
        assert_eq!(values["MemTotal"], 16_384_256);
    }

    #[test]
    fn computes_cpu_usage_from_two_samples() {
        let value = cpu_delta(
            include_str!("../../../../fixtures/proc-stat-a.txt"),
            include_str!("../../../../fixtures/proc-stat-b.txt"),
        )
        .unwrap();
        assert!((value - 46.666).abs() < 0.01);
    }

    #[test]
    fn parses_posix_df() {
        let disks = parse_df(include_str!("../../../../fixtures/df-posix.txt"));
        assert_eq!(disks[0].mount, "/");
        assert_eq!(disks[0].usage_percent, 85.0);
    }
}
