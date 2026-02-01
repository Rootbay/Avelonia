use crate::AppError;
use super::shell_helpers::{NetworkSummary, NetworkAdapterInfo, run_powershell_json, collect_string_values, format_link_speed, run_command_text};

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
            mac:
                entry
                    .get("MacAddress")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            media:
                entry
                    .get("MediaType")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            link_state:
                entry
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
    Err(AppError::System("Network summary is only implemented on Windows".into()))
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub async fn run_ping(host: String, count: Option<u8>) -> Result<String, AppError> {
    let trimmed = host.trim();
    if trimmed.is_empty() {
        return Err(AppError::Internal("Host is required".into()));
    }
    let ping_count = count.unwrap_or(4).clamp(1, 8);
    let count_arg = ping_count.to_string();
    run_command_text("ping", &["-n", &count_arg, trimmed]).map_err(AppError::System)
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub async fn run_ping(_host: String, _count: Option<u8>) -> Result<String, AppError> {
    Err(AppError::System("Ping is only implemented on Windows".into()))
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub async fn run_traceroute(host: String) -> Result<String, AppError> {
    let trimmed = host.trim();
    if trimmed.is_empty() {
        return Err(AppError::Internal("Host is required".into()));
    }
    run_command_text("tracert", &[trimmed]).map_err(AppError::System)
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub async fn run_traceroute(_host: String) -> Result<String, AppError> {
    Err(AppError::System("Traceroute is only implemented on Windows".into()))
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub async fn run_dns_lookup(host: String) -> Result<String, AppError> {
    let trimmed = host.trim();
    if trimmed.is_empty() {
        return Err(AppError::Internal("Host is required".into()));
    }
    run_command_text("nslookup", &[trimmed]).map_err(AppError::System)
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub async fn run_dns_lookup(_host: String) -> Result<String, AppError> {
    Err(AppError::System("DNS lookup is only implemented on Windows".into()))
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub async fn flush_dns() -> Result<String, AppError> {
    run_command_text("ipconfig", &["/flushdns"])
        .map_err(AppError::System)
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub async fn flush_dns() -> Result<String, AppError> {
    Err(AppError::System("flush_dns is only implemented on Windows".into()))
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub async fn reset_winsock() -> Result<String, AppError> {
    run_command_text("netsh", &["winsock", "reset"])
        .map_err(AppError::System)
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub async fn renew_ip() -> Result<String, AppError> {
    let release = run_command_text("ipconfig", &["/release"])
        .map_err(AppError::System)?;
    let renew = run_command_text("ipconfig", &["/renew"])
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

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub async fn renew_ip() -> Result<String, AppError> {
    Err(AppError::System("Only on Windows".into()))
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub async fn reset_winsock() -> Result<String, AppError> {
    Err(AppError::System("Only on Windows".into()))
}