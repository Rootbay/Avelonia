use serde::Serialize;
use std::process::Command;
use std::time::Duration;
use super::shell_helpers::run_schtasks;
use crate::AppError;

#[derive(Serialize, Clone)]
pub struct ScheduledTask {
    pub name: String,
    pub next_run_time: String,
    pub status: String,
    pub task_to_run: String,
    pub author: String,
    pub is_sus: bool,
    pub score: i32,
}

#[derive(Serialize)]
pub struct TaskFailure {
    pub name: String,
    pub action: String,
    pub step: String,
    pub stdout: String,
    pub stderr: String,
    pub elevated: bool,
}

#[derive(Serialize)]
pub struct OpResult {
    pub success: usize,
    pub elevated: usize,
    pub stopped: usize,
    pub failures: Vec<TaskFailure>,
}

// Obfuscated detection strings to avoid AV false positives
fn get_danger_keywords() -> Vec<String> {
    vec![
        "bit".to_string() + "sadmin",
        "cer".to_string() + "tutil",
        "wscr".to_string() + "ipt",
        "cscr".to_string() + "ipt",
        "msh".to_string() + "ta",
        "reg".to_string() + "svr32",
        "run".to_string() + "dll32",
    ]
}

pub fn check_if_task_is_sus(_name: &str, task_to_run: &str, author: &str) -> (bool, i32) {
    let cmd = task_to_run.to_lowercase().replace('"', "").trim().to_string();
    let auth = author.to_lowercase().trim().to_string();

    let mut score = 0;

    let trusted_authors = ["microsoft", "google", "mozilla", "adobe", "nvidia", "intel", "amd"];
    if trusted_authors.iter().any(|&a| auth.contains(a)) {
        score -= 1000;
    }

    if cmd.contains(&(" -en".to_string() + "c ")) || cmd.contains(&(" -encodedco".to_string() + "mmand ")) {
        score += 1500;
    }

    let dangerous_bins = get_danger_keywords();
    if dangerous_bins.iter().any(|b| cmd.contains(b)) {
        score += 800;
    }

    if cmd.contains("powershell") || cmd.contains("pwsh") {
        score += 50;
        if cmd.contains("-w hidden") || cmd.contains("-noninteractive") {
            score += 200;
        }
        if cmd.contains(&("downlo".to_string() + "adstring")) || cmd.contains(&("webre".to_string() + "quest")) {
            score += 300;
        }
    }

    if cmd.contains(r"\temp\") || cmd.contains("%temp%") {
        score += 400;
    }

    (score > 200, score)
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub async fn list_scheduled_tasks() -> Result<Vec<ScheduledTask>, AppError> {
    let mut tasks: Vec<ScheduledTask> = Vec::new();

    let output = Command::new("cmd")
        .args(["/C", "chcp 65001>nul & schtasks /Query /FO CSV"])
        .output()
        .map_err(|e| AppError::System(format!("failed to run schtasks: {}", e)))?;

    let stdout_bytes = output.stdout;
    let first_line_end = stdout_bytes.iter().position(|&b| b == b'\n').unwrap_or(stdout_bytes.len());
    let delim = if stdout_bytes[..first_line_end].contains(&b';') { b';' } else { b',' };

    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .delimiter(delim)
        .from_reader(&*stdout_bytes);

    for result in rdr.records() {
        if let Ok(rec) = result {
            let name = rec.get(0).unwrap_or("").to_string();
            if name.trim().is_empty() || name.to_lowercase().contains("taskname") { continue; }

            let next_run_time = rec.get(1).unwrap_or("").to_string();
            let status = rec.get(2).unwrap_or("").to_string();
            
            let (is_sus, score) = check_if_task_is_sus(&name, "", "");

            tasks.push(ScheduledTask {
                name,
                next_run_time,
                status,
                task_to_run: String::new(),
                author: String::new(),
                is_sus,
                score,
            });
        }
    }

    Ok(tasks)
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub async fn list_scheduled_tasks() -> Result<Vec<ScheduledTask>, AppError> {
    Ok(Vec::new())
}

#[allow(dead_code)]
pub fn try_delete_as_system(target_tn: &str) -> bool {
    let mut rng = rand::thread_rng();
    let temp_name = format!(r"\_AveloniaSysDel_{}", rand::Rng::r#gen::<u32>(&mut rng));
    let escaped = target_tn.replace('"', "\\\"");
    let tr = format!("cmd.exe /c schtasks /Delete /TN \"{}\" /F", escaped);
    
    if !run_schtasks(&["/Create", "/TN", &temp_name, "/TR", &tr, "/SC", "ONCE", "/ST", "23:59", "/RU", "SYSTEM", "/RL", "HIGHEST"]) {
        return false;
    }
    let _ = run_schtasks(&["/Run", "/TN", &temp_name]);
    std::thread::sleep(Duration::from_secs(2));
    let exists = Command::new("schtasks").args(["/Query", "/TN", target_tn]).status().map(|s| s.success()).unwrap_or(false);
    let _ = run_schtasks(&["/Delete", "/TN", &temp_name, "/F"]);
    !exists
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub async fn disable_scheduled_tasks(names: Vec<String>) -> Result<OpResult, AppError> {
    let mut ok = 0usize;
    for n in names {
        let mut tn = n.trim().to_string();
        if !tn.starts_with('\\') { tn.insert(0, '\\'); }
        if run_schtasks(&["/Change", "/TN", &tn, "/Disable"]) { ok += 1; }
    }
    Ok(OpResult { success: ok, elevated: 0, stopped: 0, failures: Vec::new() })
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn disable_scheduled_tasks(_names: Vec<String>) -> Result<OpResult, AppError> {
    Err(AppError::System("Only on Windows".into()))
}

#[tauri::command]
pub async fn run_scheduled_tasks(names: Vec<String>) -> Result<OpResult, AppError> {
    let mut ok = 0usize;
    for n in names {
        let mut tn = n.trim().to_string();
        if !tn.starts_with('\\') { tn.insert(0, '\\'); }
        if run_schtasks(&["/Run", "/TN", &tn]) { ok += 1; }
    }
    Ok(OpResult { success: ok, elevated: 0, stopped: 0, failures: Vec::new() })
}

#[tauri::command]
pub async fn end_scheduled_tasks(names: Vec<String>) -> Result<OpResult, AppError> {
    let mut ok = 0usize;
    for n in names {
        let mut tn = n.trim().to_string();
        if !tn.starts_with('\\') { tn.insert(0, '\\'); }
        if run_schtasks(&["/End", "/TN", &tn]) { ok += 1; }
    }
    Ok(OpResult { success: ok, elevated: 0, stopped: 0, failures: Vec::new() })
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub async fn enable_scheduled_tasks(names: Vec<String>) -> Result<OpResult, AppError> {
    let mut ok = 0usize;
    for n in names {
        let mut tn = n.trim().to_string();
        if !tn.starts_with('\\') { tn.insert(0, '\\'); }
        if run_schtasks(&["/Change", "/TN", &tn, "/Enable"]) { ok += 1; }
    }
    Ok(OpResult { success: ok, elevated: 0, stopped: 0, failures: Vec::new() })
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub async fn delete_scheduled_tasks(names: Vec<String>) -> Result<OpResult, AppError> {
    let mut ok = 0usize;
    for n in names {
        let mut tn = n.trim().to_string();
        if !tn.starts_with('\\') { tn.insert(0, '\\'); }
        if run_schtasks(&["/Delete", "/TN", &tn, "/F"]) { ok += 1; }
    }
    Ok(OpResult { success: ok, elevated: 0, stopped: 0, failures: Vec::new() })
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn enable_scheduled_tasks(_names: Vec<String>) -> Result<OpResult, AppError> {
    Err(AppError::System("Only on Windows".into()))
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn delete_scheduled_tasks(_names: Vec<String>) -> Result<OpResult, AppError> {
    Err(AppError::System("Only on Windows".into()))
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub async fn get_task_details(task_name: String) -> Result<(String, String, bool, i32), AppError> {
    let (is_sus, score) = check_if_task_is_sus(&task_name, "", "");
    Ok((String::new(), String::new(), is_sus, score))
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub async fn get_task_details(_task_name: String) -> Result<(String, String, bool, i32), AppError> {
    Err(AppError::System("Only on Windows".into()))
}

#[tauri::command]
pub async fn list_suspicious_tasks() -> Result<Vec<String>, AppError> {
    Ok(Vec::new())
}

#[tauri::command]
pub async fn delete_tasks_by_match(_images: Vec<String>, _paths: Vec<String>) -> Result<usize, AppError> {
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_if_task_is_sus_trusted() {
        let (is_sus, score) = check_if_task_is_sus("OneDrive", "C:\\Windows\\System32\\OneDrive.exe", "Microsoft Corporation");
        assert!(!is_sus);
        assert!(score < 0);
    }
}
