use crate::AppError;
use serde::{Deserialize, Serialize};
use std::process::Command;
use super::shell_helpers::run_cmd_elevated;

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
        .map_err(|e| AppError::System(format!("failed to run powershell Get-CimInstance: {}", e)))?;
    if !out.status.success() {
        return Ok(Vec::new());
    }
    let stdout: String = String::from_utf8_lossy(&out.stdout).to_string();
    let result: serde_json::Value =
        serde_json::from_str(&stdout).map_err(|e| AppError::Internal(format!("json parse failed: {}", e)))?;
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
    Ok(Vec::new())
}
#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub async fn stop_services(_names: Vec<String>) -> Result<usize, AppError> {
    Err(AppError::System("Only on Windows".into()))
}
#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub async fn disable_services(_names: Vec<String>) -> Result<usize, AppError> {
    Err(AppError::System("Only on Windows".into()))
}