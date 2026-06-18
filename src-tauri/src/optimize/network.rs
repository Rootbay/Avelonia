use super::shell_helpers::{
    NetworkAdapterInfo, NetworkSummary, collect_string_values, format_link_speed,
    run_powershell_json,
};
use crate::AppError;

#[tauri::command]
#[cfg(target_os = "windows")]
pub async fn get_network_summary() -> Result<NetworkSummary, AppError> {
    let ip_script = "Get-NetIPConfiguration | Select InterfaceAlias,IPv4Address,IPv6Address,DnsServer,IPv4DefaultGateway | ConvertTo-Json";
    let adapter_script = "Get-NetAdapter | Select InterfaceAlias,Status,LinkSpeed,MacAddress,MediaType,InterfaceDescription,LinkState | ConvertTo-Json";
    let ip_data = run_powershell_json(ip_script).map_err(AppError::System)?;
    let adapter_data = run_powershell_json(adapter_script).map_err(AppError::System)?;
    let mut summary = NetworkSummary {
        primary_adapter: None,
        ipv4: None,
        ipv6: None,
        dns_servers: Vec::new(),
        gateways: Vec::new(),
        adapters: Vec::new(),
    };
    for entry in &ip_data {
        let alias = entry
            .get("InterfaceAlias")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let ipv4_list = collect_string_values(entry.get("IPv4Address"), &["IPAddress"]);
        if summary.ipv4.is_none() && !ipv4_list.is_empty() {
            summary.ipv4 = Some(ipv4_list[0].clone());
            if let Some(name) = alias.clone() {
                summary.primary_adapter = Some(name);
            }
        }
        let ipv6_list = collect_string_values(entry.get("IPv6Address"), &["IPAddress"]);
        if summary.ipv6.is_none() && !ipv6_list.is_empty() {
            summary.ipv6 = Some(ipv6_list[0].clone());
        }
        if summary.primary_adapter.is_none() {
            if let Some(name) = alias.clone() {
                summary.primary_adapter = Some(name);
            }
        }
        for dns in collect_string_values(entry.get("DnsServer"), &["IPAddress", "ServerAddress"]) {
            if !summary.dns_servers.contains(&dns) {
                summary.dns_servers.push(dns);
            }
        }
        for gw in collect_string_values(entry.get("IPv4DefaultGateway"), &["NextHop", "IPAddress"])
        {
            if !summary.gateways.contains(&gw) {
                summary.gateways.push(gw);
            }
        }
    }
    let mut adapters = Vec::new();
    for entry in &adapter_data {
        let name = match entry.get("InterfaceAlias").and_then(|v| v.as_str()) {
            Some(value) => value.to_string(),
            None => continue,
        };
        let info = NetworkAdapterInfo {
            name,
            status: entry
                .get("Status")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            link_speed: entry.get("LinkSpeed").and_then(format_link_speed),
            mac: entry
                .get("MacAddress")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            media: entry
                .get("MediaType")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            link_state: entry
                .get("LinkState")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        };
        adapters.push(info);
    }
    summary.adapters = adapters;
    Ok(summary)
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub async fn get_network_summary() -> Result<NetworkSummary, AppError> {
    Err(AppError::System(
        "Network summary is only implemented on Windows".into(),
    ))
}

#[tauri::command]
pub async fn run_ping(host: String, count: Option<u8>) -> Result<String, AppError> {
    let trimmed = host.trim();
    if trimmed.is_empty() {
        return Err(AppError::Internal("Host is required".into()));
    }
    let ping_count = count.unwrap_or(4).clamp(1, 8);
    let count_arg = ping_count.to_string();
    #[cfg(target_os = "windows")]
    {
        super::shell_helpers::run_command_text("ping", &["-n", &count_arg, trimmed])
            .map_err(AppError::System)
    }
    #[cfg(not(target_os = "windows"))]
    {
        super::shell_helpers::run_command_text("ping", &["-c", &count_arg, trimmed])
            .map_err(AppError::System)
    }
}

#[tauri::command]
pub async fn run_traceroute(host: String) -> Result<String, AppError> {
    let trimmed = host.trim();
    if trimmed.is_empty() {
        return Err(AppError::Internal("Host is required".into()));
    }
    #[cfg(target_os = "windows")]
    {
        super::shell_helpers::run_command_text("tracert", &[trimmed]).map_err(AppError::System)
    }
    #[cfg(not(target_os = "windows"))]
    {
        super::shell_helpers::run_command_text("traceroute", &[trimmed]).map_err(AppError::System)
    }
}

#[tauri::command]
pub async fn run_dns_lookup(host: String) -> Result<String, AppError> {
    let trimmed = host.trim();
    if trimmed.is_empty() {
        return Err(AppError::Internal("Host is required".into()));
    }
    super::shell_helpers::run_command_text("nslookup", &[trimmed]).map_err(AppError::System)
}

#[tauri::command]
pub async fn flush_dns() -> Result<String, AppError> {
    #[cfg(target_os = "windows")]
    {
        super::shell_helpers::run_command_text("ipconfig", &["/flushdns"]).map_err(AppError::System)
    }
    #[cfg(target_os = "macos")]
    {
        let res1 = super::shell_helpers::run_command_text("dscacheutil", &["-flushcache"]);
        let res2 = super::shell_helpers::run_command_text("killall", &["-HUP", "mDNSResponder"]);
        match (res1, res2) {
            (Ok(r1), Ok(r2)) => Ok(format!("dscacheutil: {}\nkillall: {}", r1, r2)),
            (Ok(r1), Err(e)) => Ok(format!("dscacheutil: {}\nkillall failed: {}", r1, e)),
            (Err(e), _) => Err(AppError::System(format!("dscacheutil failed: {}", e))),
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(res) = super::shell_helpers::run_command_text("resolvectl", &["flush-caches"]) {
            Ok(res)
        } else if let Ok(res) =
            super::shell_helpers::run_command_text("systemd-resolve", &["--flush-caches"])
        {
            Ok(res)
        } else {
            Err(AppError::System(
                "Failed to flush DNS cache: resolvectl/systemd-resolve not found".into(),
            ))
        }
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        Err(AppError::System(
            "flush_dns is not supported on this platform".into(),
        ))
    }
}

#[tauri::command]
pub async fn renew_ip() -> Result<String, AppError> {
    #[cfg(target_os = "windows")]
    {
        let release = super::shell_helpers::run_command_text("ipconfig", &["/release"])
            .map_err(AppError::System)?;
        let renew = super::shell_helpers::run_command_text("ipconfig", &["/renew"])
            .map_err(AppError::System)?;
        let mut parts = Vec::new();
        if !release.is_empty() {
            parts.push(release);
        }
        if !renew.is_empty() {
            parts.push(renew);
        }
        Ok(parts.join("\n"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err(AppError::System(
            "IP renewal requires elevated privileges, not supported on this platform".into(),
        ))
    }
}

#[tauri::command]
pub async fn reset_winsock() -> Result<String, AppError> {
    #[cfg(target_os = "windows")]
    {
        super::shell_helpers::run_command_text("netsh", &["winsock", "reset"])
            .map_err(AppError::System)
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err(AppError::System(
            "Winsock reset is only available on Windows".into(),
        ))
    }
}
