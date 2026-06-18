use super::shell_helpers::run_cmd_elevated;
use crate::AppError;
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Serialize, Clone, Deserialize)]
pub struct ServiceInfo {
    pub name: String,
    pub display_name: String,
    pub state: String,
    pub start_mode: String,
    pub path: String,
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub async fn list_services() -> Result<Vec<ServiceInfo>, AppError> {
    let ps = r#"Get-CimInstance Win32_Service | Select-Object Name,DisplayName,State,StartMode,PathName | ConvertTo-Json -Depth 3"#;
    let out = Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", ps])
        .output()
        .map_err(|e| {
            AppError::System(format!("failed to run powershell Get-CimInstance: {}", e))
        })?;
    if !out.status.success() {
        return Ok(Vec::new());
    }
    let stdout: String = String::from_utf8_lossy(&out.stdout).to_string();
    let result: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| AppError::Internal(format!("json parse failed: {}", e)))?;
    let mut out_vec: Vec<ServiceInfo> = Vec::new();
    match result {
        serde_json::Value::Array(arr) => {
            for v in arr {
                let name = v
                    .get("Name")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                let display_name = v
                    .get("DisplayName")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                let state = v
                    .get("State")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                let start_mode = v
                    .get("StartMode")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                let path = v
                    .get("PathName")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                out_vec.push(ServiceInfo {
                    name,
                    display_name,
                    state,
                    start_mode,
                    path,
                });
            }
        }
        serde_json::Value::Object(v) => {
            let name = v
                .get("Name")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let display_name = v
                .get("DisplayName")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let state = v
                .get("State")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let start_mode = v
                .get("StartMode")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let path = v
                .get("PathName")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            out_vec.push(ServiceInfo {
                name,
                display_name,
                state,
                start_mode,
                path,
            });
        }
        _ => {}
    }
    Ok(out_vec)
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub async fn stop_services(names: Vec<String>) -> Result<usize, AppError> {
    let mut ok = 0usize;
    for n in names {
        if Command::new("sc")
            .args(["stop", &n])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            ok += 1;
            continue;
        }
        let _ = run_cmd_elevated(&["/C", "sc", "stop", &n]);
    }
    Ok(ok)
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub async fn disable_services(names: Vec<String>) -> Result<usize, AppError> {
    let mut ok = 0usize;
    for n in names {
        if Command::new("sc")
            .args(["config", &n, "start=", "disabled"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            ok += 1;
            continue;
        }
        let _ = run_cmd_elevated(&["/C", "sc", "config", &n, "start=", "disabled"]);
    }
    Ok(ok)
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub async fn list_services() -> Result<Vec<ServiceInfo>, AppError> {
    let mut services = Vec::new();
    #[cfg(target_os = "linux")]
    {
        // Try systemctl list-units --type=service --all --output=json --no-pager
        if let Ok(output) = Command::new("systemctl")
            .args([
                "list-units",
                "--type=service",
                "--all",
                "--output=json",
                "--no-pager",
            ])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&stdout) {
                    if let Some(arr) = parsed.as_array() {
                        for v in arr {
                            let name = v
                                .get("unit")
                                .and_then(|x| x.as_str())
                                .unwrap_or("")
                                .to_string();
                            let display_name = v
                                .get("description")
                                .and_then(|x| x.as_str())
                                .unwrap_or("")
                                .to_string();
                            let state = v
                                .get("sub")
                                .and_then(|x| x.as_str())
                                .unwrap_or("")
                                .to_string(); // e.g. running, dead
                            let start_mode = v
                                .get("load")
                                .and_then(|x| x.as_str())
                                .unwrap_or("")
                                .to_string(); // e.g. loaded
                            services.push(ServiceInfo {
                                name,
                                display_name,
                                state,
                                start_mode,
                                path: String::new(),
                            });
                        }
                        return Ok(services);
                    }
                }
            }
        }
        // Fallback or text parsing if json is not supported or systemctl output differs
        if let Ok(output) = Command::new("systemctl")
            .args([
                "list-units",
                "--type=service",
                "--all",
                "--no-pager",
                "--no-legend",
            ])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    let name = parts[0].to_string();
                    let load = parts[1].to_string();
                    let _active = parts[2];
                    let sub = parts[3].to_string();
                    let desc = parts[4..].join(" ");
                    services.push(ServiceInfo {
                        name,
                        display_name: desc,
                        state: sub,
                        start_mode: load,
                        path: String::new(),
                    });
                }
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = Command::new("launchctl").arg("list").output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines().skip(1) {
                // Skip header: PID Status Label
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    let pid = parts[0];
                    let status = parts[1];
                    let label = parts[2].to_string();
                    let state = if pid == "-" {
                        "stopped".to_string()
                    } else {
                        format!("running (PID {})", pid)
                    };
                    services.push(ServiceInfo {
                        name: label.clone(),
                        display_name: label,
                        state,
                        start_mode: format!("ExitStatus {}", status),
                        path: String::new(),
                    });
                }
            }
        }
    }
    Ok(services)
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub async fn stop_services(names: Vec<String>) -> Result<usize, AppError> {
    let mut ok = 0;
    for n in names {
        #[cfg(target_os = "linux")]
        {
            if Command::new("systemctl")
                .args(["stop", &n])
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
            {
                ok += 1;
            }
        }
        #[cfg(target_os = "macos")]
        {
            if Command::new("launchctl")
                .args(["stop", &n])
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
            {
                ok += 1;
            }
        }
    }
    Ok(ok)
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub async fn disable_services(names: Vec<String>) -> Result<usize, AppError> {
    let mut ok = 0;
    for n in names {
        #[cfg(target_os = "linux")]
        {
            if Command::new("systemctl")
                .args(["disable", &n])
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
            {
                ok += 1;
            }
        }
        #[cfg(target_os = "macos")]
        {
            if Command::new("launchctl")
                .args(["disable", &format!("user/{}", n)])
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
            {
                ok += 1;
            }
        }
    }
    Ok(ok)
}
