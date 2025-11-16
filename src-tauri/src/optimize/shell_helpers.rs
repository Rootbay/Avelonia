use serde::Serialize;
use serde_json::Value;
use std::env;
use std::net::IpAddr;
#[cfg(target_os = "windows")]
use std::process::Command;

#[cfg(target_os = "windows")]
pub(crate) fn fmt_command_text(output: &std::process::Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let mut parts = Vec::new();
    if !stdout.is_empty() {
        parts.push(stdout);
    }
    if !stderr.is_empty() {
        parts.push(stderr);
    }
    parts.join("\n")
}

#[cfg(target_os = "windows")]
pub(crate) fn run_command_text(cmd: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .map_err(|e| format!("failed to run {}: {}", cmd, e))?;
    let text = fmt_command_text(&output);
    if output.status.success() {
        Ok(text)
    } else {
        let suffix = if text.is_empty() { String::new() } else { format!(": {text}") };
        Err(format!(
            "{} exited with status {:?}{}",
            cmd,
            output.status.code(),
            suffix
        ))
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkAdapterInfo {
    pub name: String,
    pub status: Option<String>,
    pub link_speed: Option<String>,
    pub mac: Option<String>,
    pub media: Option<String>,
    pub link_state: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkSummary {
    pub primary_adapter: Option<String>,
    pub ipv4: Option<String>,
    pub ipv6: Option<String>,
    pub dns_servers: Vec<String>,
    pub gateways: Vec<String>,
    pub adapters: Vec<NetworkAdapterInfo>,
}

#[cfg(target_os = "windows")]
pub(crate) fn run_powershell_json(script: &str) -> Result<Vec<Value>, String> {
    use std::process::Command;
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-WindowStyle",
            "Hidden",
            "-Command",
            script,
        ])
        .output()
        .map_err(|e| format!("failed to run powershell: {}", e))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("powershell failed: {}", stderr));
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        return Ok(Vec::new());
    }
    let parsed: Value =
        serde_json::from_str(&text).map_err(|e| format!("failed to parse powershell output: {}", e))?;
    let items = match parsed {
        Value::Array(results) => {
            if results.is_empty() {
                Vec::new()
            } else {
                results
            }
        }
        Value::Null => Vec::new(),
        other => vec![other],
    };
    Ok(items)
}

#[cfg(target_os = "windows")]
pub(crate) fn value_to_string(value: &Value) -> Option<String> {
    if let Some(s) = value.as_str() {
        return Some(s.to_string());
    }
    if let Some(n) = value.as_u64() {
        return Some(n.to_string());
    }
    if let Some(n) = value.as_i64() {
        return Some(n.to_string());
    }
    if let Some(n) = value.as_f64() {
        return Some(n.to_string());
    }
    if let Some(obj) = value.as_object() {
        for key in ["IPAddress", "ServerAddress", "Address", "NextHop"] {
            if let Some(entry) = obj.get(key) {
                if let Some(s) = entry.as_str() {
                    return Some(s.to_string());
                }
            }
        }
    }
    None
}

#[cfg(target_os = "windows")]
pub(crate) fn push_candidate_value(result: &mut Vec<String>, candidate: &str) {
    let trimmed = candidate.trim();
    if trimmed.is_empty() || is_com_instance_string(trimmed) {
        return;
    }
    if let Some(ip) = sanitize_ip_candidate(trimmed) {
        if !result.contains(&ip) {
            result.push(ip);
        }
        return;
    }
    let normalized = trimmed.to_string();
    if !normalized.is_empty() && !result.contains(&normalized) {
        result.push(normalized);
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn collect_string_values(value: Option<&Value>, keys: &[&str]) -> Vec<String> {
    let mut result = Vec::new();
    if let Some(val) = value {
        if let Some(arr) = val.as_array() {
            for entry in arr {
                for &key in keys {
                    if let Some(item) = entry.get(key).and_then(|v| v.as_str()) {
                        push_candidate_value(&mut result, item);
                        break;
                    }
                }
                if let Some(item) = value_to_string(entry) {
                    push_candidate_value(&mut result, &item);
                }
            }
        } else if let Some(item) = value_to_string(val) {
            push_candidate_value(&mut result, &item);
        }
    }
    result
}

#[cfg(target_os = "windows")]
pub(crate) fn format_link_speed(value: &Value) -> Option<String> {
    if let Some(text) = value.as_str() {
        return Some(text.to_string());
    }
    if let Some(speed) = value.as_u64() {
        let mbps = speed as f64 / 1_000_000.0;
        let formatted = if (mbps - mbps.round()).abs() < 1e-3 {
            format!("{:.0} Mbps", mbps)
        } else {
            format!("{:.1} Mbps", mbps)
        };
        return Some(formatted);
    }
    None
}

pub(crate) fn sanitize_ip_candidate(input: &str) -> Option<String> {
    let mut candidate = input.trim();
    if candidate.is_empty() {
        return None;
    }
    candidate = candidate.trim_matches('"');
    candidate = candidate.trim_matches('\'');
    candidate = candidate.trim();
    if candidate.is_empty() {
        return None;
    }
    if let Some(pos) = candidate.find('%') {
        let base = candidate[..pos].trim();
        if !base.is_empty() {
            if base.parse::<IpAddr>().is_ok() {
                return Some(base.to_string());
            }
        }
    }
    if candidate.parse::<IpAddr>().is_ok() {
        return Some(candidate.to_string());
    }
    if let Some(idx) = candidate.find(' ') {
        let first = candidate[..idx].trim();
        if first.parse::<IpAddr>().is_ok() {
            return Some(first.to_string());
        }
    }
    None
}

pub(crate) fn is_com_instance_string(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.starts_with("MSFT_") && trimmed.contains("Name =") {
        return true;
    }
    false
}



pub(crate) fn run_schtasks(args: &[&str]) -> bool {
    if std::process::Command::new("schtasks")
        .args(args)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        return true;
    }

    let arglist = {
        let items: Vec<String> = args
            .iter()
            .map(|a| format!("'{}'", a.replace('\'', "''")))
            .collect();
        format!("@({})", items.join(", "))
    };
    let ps = format!(
        "Start-Process -FilePath schtasks -ArgumentList {} -Verb RunAs -Wait; exit $LASTEXITCODE",
        arglist
    );
    std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &ps,
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}



pub(crate) fn run_schtasks_capture(args: &[&str]) -> (bool, String, String) {
    match std::process::Command::new("schtasks").args(args).output() {
        Ok(out) => {
            let ok = out.status.success();
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            (ok, stdout, stderr)
        }
        Err(_) => (false, String::new(), String::new()),
    }
}



#[cfg(target_os = "windows")]
pub(crate) fn run_cmd_elevated(args: &[&str]) -> bool {
    let arglist = {
        let items: Vec<String> = args
            .iter()
            .map(|a| format!("'{}'", a.replace('\'', "''")))
            .collect();
        format!("@({})", items.join(", "))
    };
    let ps = format!(
        "Start-Process -FilePath cmd.exe -ArgumentList {} -Verb RunAs -Wait; exit $LASTEXITCODE",
        arglist
    );
    std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &ps,
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}



#[cfg(target_os = "windows")]
pub(crate) fn run_reg(args: &[&str]) -> bool {
    std::process::Command::new("reg")
        .args(args)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}



#[cfg(target_os = "windows")]
pub(crate) fn run_reg_elevated(args: &[&str]) -> bool {
    let arglist = {
        let items: Vec<String> = args
            .iter()
            .map(|a| format!("'{}'", a.replace('\'', "''")))
            .collect();
        format!("@({})", items.join(", "))
    };
    let ps = format!(
        "Start-Process -FilePath reg -ArgumentList {} -Verb RunAs -Wait; exit $LASTEXITCODE",
        arglist
    );
    std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &ps,
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}



#[cfg(target_os = "windows")]
pub(crate) fn run_powershell_elevated(args: &[&str]) -> bool {
    let arglist = {
        let items: Vec<String> = args
            .iter()
            .map(|a| format!("'{}'", a.replace('\'', "''")))
            .collect();
        format!("@({})", items.join(", "))
    };
    let ps = format!(
        "Start-Process -FilePath powershell -ArgumentList {} -Verb RunAs -Wait; exit $LASTEXITCODE",
        arglist
    );
    std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &ps,
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}



#[cfg(target_os = "windows")]
pub(crate) fn escape_single_quotes(value: &str) -> String {
    value.replace('\'', "''")
}



#[cfg(target_os = "windows")]
pub(crate) fn write_temp_ps_script(prefix: &str, contents: &str) -> Result<String, String> {
    let mut path = env::temp_dir();
    let random: u32 = rand::random();
    path.push(format!("avelonia_{}_{}.ps1", prefix, random));
    std::fs::write(&path, contents).map_err(|e| format!("write script failed: {}", e))?;
    Ok(path.to_string_lossy().to_string())
}



#[cfg(target_os = "windows")]
pub(crate) fn run_powershell_commands(commands: &[String], prefix: &str) -> Result<(), String> {
    if commands.is_empty() {
        return Ok(());
    }
    let mut script = String::from("$ErrorActionPreference='SilentlyContinue'\n");
    for cmd in commands {
        script.push_str(cmd);
        if !cmd.ends_with('\n') {
            script.push('\n');
        }
    }
    let script_path = write_temp_ps_script(prefix, &script)?;
    let ok = run_powershell_elevated(&[
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        script_path.as_str(),
    ]);
    let _ = std::fs::remove_file(&script_path);
    if ok {
        Ok(())
    } else {
        Err(format!("powershell script failed for {}", prefix))
    }
}
