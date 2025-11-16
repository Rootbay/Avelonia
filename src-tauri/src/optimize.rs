use std::env;
use std::path::PathBuf;
use walkdir::WalkDir;
use serde::{Serialize, Deserialize};
use serde_json::Value;
use lnk::{ShellLink};
use lnk::encoding::WINDOWS_1252;
use std::process::{Command, Stdio};
use std::str;
use std::io;
use std::net::IpAddr;
use std::time::Duration;
use rand::Rng;
use sysinfo::System;

#[derive(Serialize, Clone)]
pub struct ScheduledTask {
    pub name: String,
    pub next_run_time: String,
    pub status: String,
    pub task_to_run: String,
    pub author: String,
    pub is_sus: bool,
}

#[derive(Serialize, Clone)]
pub struct StartupShortcut {
    pub path: String,
    pub name: String,
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

#[derive(Serialize, Deserialize)]
pub struct TweakApplyRequest {
    pub tweaks: Vec<String>,
    pub configs: Vec<String>,
    pub update_profile: Option<String>,
}

#[derive(Serialize)]
pub struct TweakApplyResponse {
    pub tweaks_applied: usize,
    pub configs_applied: usize,
    pub profile_applied: Option<String>,
}

#[tauri::command]
pub fn list_startup_shortcuts() -> Result<Vec<StartupShortcut>, String> {
    let mut items: Vec<StartupShortcut> = Vec::new();
    if let Some(appdata) = env::var_os("APPDATA") {
        let user_startup = PathBuf::from(appdata).join("Microsoft/Windows/Start Menu/Programs/Startup");
        if user_startup.exists() && user_startup.is_dir() {
            for entry in WalkDir::new(&user_startup).min_depth(1).max_depth(1).into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_file() {
                    let p = entry.path();
                    if let Some(fname) = p.file_name().and_then(|s| s.to_str()) {
                        if fname.eq_ignore_ascii_case("desktop.ini") { continue; }
                    }
                    let mut allowed = false;
                    if let Some(ext) = p.extension() { if ext.eq_ignore_ascii_case("lnk") || ext.eq_ignore_ascii_case("url") || ext.eq_ignore_ascii_case("exe") { allowed = true; } }
                    if !allowed { continue; }

                    let mut name = p.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
                    if let Some(ext) = p.extension() {
                        if ext.eq_ignore_ascii_case("lnk") {
                            if let Ok(link) = ShellLink::open(p, WINDOWS_1252) {
                                if let Some(li) = link.link_info() {
                                    let target = li.common_path_suffix();
                                    let target_name = PathBuf::from(target.to_string()).file_stem().and_then(|s| s.to_str()).map(|s| s.to_string());
                                    if let Some(n) = target_name { name = n; }
                                }
                            }
                        }
                    }
                    if name.is_empty() { name = p.display().to_string(); }
                    items.push(StartupShortcut { path: p.display().to_string(), name });
                }
            }
        }
    }
    if let Some(programdata) = env::var_os("PROGRAMDATA") {
        let all_startup = PathBuf::from(programdata).join("Microsoft/Windows/Start Menu/Programs/StartUp");
        if all_startup.exists() && all_startup.is_dir() {
            for entry in WalkDir::new(&all_startup).min_depth(1).max_depth(1).into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_file() {
                    let p = entry.path();
                    if let Some(fname) = p.file_name().and_then(|s| s.to_str()) {
                        if fname.eq_ignore_ascii_case("desktop.ini") { continue; }
                    }
                    let mut allowed = false;
                    if let Some(ext) = p.extension() { if ext.eq_ignore_ascii_case("lnk") || ext.eq_ignore_ascii_case("url") || ext.eq_ignore_ascii_case("exe") { allowed = true; } }
                    if !allowed { continue; }

                    let mut name = p.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
                    if let Some(ext) = p.extension() {
                        if ext.eq_ignore_ascii_case("lnk") {
                            if let Ok(link) = ShellLink::open(p, WINDOWS_1252) {
                                if let Some(li) = link.link_info() {
                                    let target = li.common_path_suffix();
                                    let target_name = PathBuf::from(target.to_string()).file_stem().and_then(|s| s.to_str()).map(|s| s.to_string());
                                    if let Some(n) = target_name { name = n; }
                                }
                            }
                        }
                    }
                    if name.is_empty() { name = p.display().to_string(); }
                    items.push(StartupShortcut { path: p.display().to_string(), name });
                }
            }
        }
    }
    Ok(items)
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub fn list_scheduled_tasks() -> Result<Vec<ScheduledTask>, String> {
    #[derive(Deserialize)]
    struct PsTask {
        name: Option<String>,
        next_run_time: Option<String>,
        status: Option<String>,
        task_to_run: Option<String>,
        author: Option<String>,
    }

    let ps: Result<std::process::Output, io::Error> = Err(io::Error::new(io::ErrorKind::Other, "disabled"));

    let mut tasks: Vec<ScheduledTask> = Vec::new();
    if let Ok(ps_out) = ps {
        if ps_out.status.success() {
            if let Ok(json) = String::from_utf8(ps_out.stdout) {
                let parsed: Result<Vec<PsTask>, _> = serde_json::from_str(&json).or_else(|_| {
                    serde_json::from_str::<PsTask>(&json).map(|one| vec![one])
                });
                if let Ok(list) = parsed {
                    for p in list {
                        let task_to_run = p.task_to_run.unwrap_or_default();
                        let cmd_lower = task_to_run.to_lowercase();
                        let is_sus = cmd_lower.contains("powershell")
                            || cmd_lower.contains("wscript")
                            || cmd_lower.contains("cscript")
                            || cmd_lower.contains("mshta")
                            || cmd_lower.contains("regsvr32")
                            || cmd_lower.contains("rundll32")
                            || cmd_lower.contains("cmd.exe /c")
                            || cmd_lower.contains("/b64")
                            || cmd_lower.contains(" -enc ")
                            || cmd_lower.contains("%temp%")
                            || cmd_lower.contains("appdata")
                            || cmd_lower.contains("http://")
                            || cmd_lower.contains("https://");

                        tasks.push(ScheduledTask {
                            name: p.name.unwrap_or_default(),
                            next_run_time: p.next_run_time.unwrap_or_default(),
                            status: p.status.unwrap_or_default(),
                            task_to_run,
                            author: p.author.unwrap_or_default(),
                            is_sus,
                        });
                    }
                }
            }
        }
    }

    if !tasks.is_empty() {
        return Ok(tasks);
    }

    let output = Command::new("cmd")
        .args(["/C", "chcp 65001>nul & schtasks /Query /FO CSV"])
        .output()
        .map_err(|e| format!("failed to run schtasks: {}", e))?;

    let stdout_bytes = output.stdout;
    let first_line_end = stdout_bytes.iter().position(|&b| b == b'\n').unwrap_or(stdout_bytes.len());
    let first_line = &stdout_bytes[..first_line_end];
    let delim = if first_line.iter().filter(|&&b| b == b';').count() > first_line.iter().filter(|&&b| b == b',').count() {
        b';'
    } else {
        b','
    };

    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .delimiter(delim)
        .from_reader(&*stdout_bytes);

    let headers = rdr.headers().map_err(|e| e.to_string())?.clone();
    let norm: Vec<String> = headers
        .iter()
        .map(|h| h.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase())
        .collect();

    let find_idx_exact = |keys: &[&str]| -> Option<usize> {
        for (i, h) in norm.iter().enumerate() {
            for k in keys {
                if h == *k { return Some(i); }
            }
        }
        None
    };
    let find_idx_contains = |keys: &[&str]| -> Option<usize> {
        for (i, h) in norm.iter().enumerate() {
            for k in keys {
                if h.contains(k) { return Some(i); }
            }
        }
        None
    };

    let idx_name = find_idx_exact(&["taskname"]).or_else(|| find_idx_exact(&["taskpath"]))
        .or_else(|| find_idx_contains(&["taskname", "taskpath"]))
        .unwrap_or(0);
    let idx_next = find_idx_exact(&["nextruntime"]).or_else(|| find_idx_contains(&["nextruntime", "nextrun", "nextstart", "nast"]))
        .unwrap_or(idx_name);
    let idx_status = find_idx_exact(&["status"]).or_else(|| find_idx_contains(&["status"]))
        .unwrap_or(idx_name);
    let idx_run_opt = find_idx_exact(&["tasktorun"]).or_else(|| find_idx_contains(&["tasktorun", "programscript", "tasktoexecute", "aktion", "action", "program", "script"])) ;
    let idx_author_opt = find_idx_exact(&["author"]).or_else(|| find_idx_contains(&["author", "creator", "forfattare", "autor", "skapatav"])) ;

    for result in rdr.records() {
        if let Ok(rec) = result {
            let name = rec.get(idx_name).unwrap_or("").to_string();
            let name_trim = name.trim();
            let name_lower = name_trim.to_lowercase();
            if name_trim.is_empty()
                || name_lower == "taskname"
                || name_lower == "task name"
                || name_lower == "\"taskname\""
            {
                continue;
            }

            let next_run_time = rec.get(idx_next).unwrap_or("").to_string();
            let status = rec.get(idx_status).unwrap_or("").to_string();
            let task_to_run = idx_run_opt.and_then(|i| rec.get(i)).unwrap_or("").to_string();
            let author = idx_author_opt.and_then(|i| rec.get(i)).unwrap_or("").to_string();

            let cmd_lower = task_to_run.to_lowercase();
            let is_sus = cmd_lower.contains("powershell")
                || cmd_lower.contains("wscript")
                || cmd_lower.contains("cscript")
                || cmd_lower.contains("mshta")
                || cmd_lower.contains("regsvr32")
                || cmd_lower.contains("rundll32")
                || cmd_lower.contains("cmd.exe /c")
                || cmd_lower.contains("/b64")
                || cmd_lower.contains(" -enc ")
                || cmd_lower.contains("%temp%")
                || cmd_lower.contains("appdata")
                || cmd_lower.contains("http://")
                || cmd_lower.contains("https://");

            tasks.push(ScheduledTask {
                name,
                next_run_time,
                status,
                task_to_run,
                author,
                is_sus,
            });
        }
    }

    if tasks.is_empty() {
        let output2 = Command::new("cmd")
            .args(["/C", "chcp 65001>nul & schtasks /Query /FO CSV"])
            .output()
            .map_err(|e| format!("failed to run schtasks (fallback): {}", e))?;

        let stdout_bytes2 = output2.stdout;
        let first_line_end2 = stdout_bytes2.iter().position(|&b| b == b'\n').unwrap_or(stdout_bytes2.len());
        let first_line2 = &stdout_bytes2[..first_line_end2];
        let delim2 = if first_line2.iter().filter(|&&b| b == b';').count() > first_line2.iter().filter(|&&b| b == b',').count() { b';' } else { b',' };
        let mut rdr2 = csv::ReaderBuilder::new().has_headers(true).delimiter(delim2).from_reader(&*stdout_bytes2);
        let headers2 = rdr2.headers().map_err(|e| e.to_string())?.clone();
        let norm2: Vec<String> = headers2.iter().map(|h| h.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase()).collect();
        let find_idx_exact2 = |keys: &[&str]| -> Option<usize> { for (i,h) in norm2.iter().enumerate() { for k in keys { if h==*k { return Some(i); } } } None };
        let find_idx_contains2 = |keys: &[&str]| -> Option<usize> { for (i,h) in norm2.iter().enumerate() { for k in keys { if h.contains(k) { return Some(i); } } } None };
        let idx_name2 = find_idx_exact2(&["taskname"]).or_else(|| find_idx_contains2(&["taskname", "taskpath"])) .unwrap_or(0);
        let idx_next2 = find_idx_exact2(&["nextruntime"]).or_else(|| find_idx_contains2(&["nextruntime", "nextrun", "nextstart"])) .unwrap_or(idx_name2);
        let idx_status2 = find_idx_exact2(&["status"]).or_else(|| find_idx_contains2(&["status"])) .unwrap_or(idx_name2);
        for rec in rdr2.records().flatten() {
            let name = rec.get(idx_name2).unwrap_or("").to_string();
            let name_trim = name.trim();
            let name_lower = name_trim.to_lowercase();
            if name_trim.is_empty()
                || name_lower == "taskname"
                || name_lower == "task name"
                || name_lower == "\"taskname\""
            {
                continue;
            }

            let next_run_time = rec.get(idx_next2).unwrap_or("").to_string();
            let status = rec.get(idx_status2).unwrap_or("").to_string();
            tasks.push(ScheduledTask {
                name,
                next_run_time,
                status,
                task_to_run: String::new(),
                author: String::new(),
                is_sus: false,
            });
        }
    }

    if tasks.len() > 5000 { tasks.truncate(5000); }
    Ok(tasks)
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub fn list_suspicious_tasks() -> Result<Vec<String>, String> {
    let output = Command::new("cmd")
        .args(["/C", "chcp 65001>nul & schtasks /Query /V /FO CSV"])
        .output()
        .map_err(|e| format!("failed to run schtasks /V: {}", e))?;

    let stdout_bytes = output.stdout;
    if stdout_bytes.is_empty() {
        return Ok(Vec::new());
    }

    let first_line_end = stdout_bytes
        .iter()
        .position(|&b| b == b'\n')
        .unwrap_or(stdout_bytes.len());
    let first_line = &stdout_bytes[..first_line_end];
    let delim = if first_line
        .iter()
        .filter(|&&b| b == b';')
        .count()
        > first_line.iter().filter(|&&b| b == b',').count()
    {
        b';'
    } else {
        b','
    };

    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .delimiter(delim)
        .from_reader(&*stdout_bytes);

    let headers = rdr.headers().map_err(|e| e.to_string())?.clone();
    let norm: Vec<String> = headers
        .iter()
        .map(|h| h.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase())
        .collect();

    let find_idx_contains = |keys: &[&str]| -> Option<usize> {
        for (i, h) in norm.iter().enumerate() {
            for k in keys {
                if h.contains(k) {
                    return Some(i);
                }
            }
        }
        None
    };

    let idx_name = find_idx_contains(&["taskname", "taskpath"]).unwrap_or(0);
    let idx_run_opt = find_idx_contains(&["tasktorun", "programscript", "tasktoexecute", "aktion", "action", "program", "script"]);

    let mut out: Vec<String> = Vec::new();
    for rec in rdr.records() {
        if let Ok(rec) = rec {
            let name = rec.get(idx_name).unwrap_or("").to_string();
            if name.trim().is_empty() { continue; }
            let task_to_run = idx_run_opt.and_then(|i| rec.get(i)).unwrap_or("");
            let cmd_lower = task_to_run.to_lowercase();
            let sus = cmd_lower.contains("powershell")
                || cmd_lower.contains("wscript")
                || cmd_lower.contains("cscript")
                || cmd_lower.contains("mshta")
                || cmd_lower.contains("regsvr32")
                || cmd_lower.contains("rundll32")
                || cmd_lower.contains("cmd.exe /c")
                || cmd_lower.contains("/b64")
                || cmd_lower.contains(" -enc ")
                || cmd_lower.contains("%temp%")
                || cmd_lower.contains("appdata")
                || cmd_lower.contains("http://")
                || cmd_lower.contains("https://");
            if sus { out.push(name); }
        }
    }
    out.sort();
    out.dedup();
    Ok(out)
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn list_suspicious_tasks() -> Result<Vec<String>, String> { Ok(Vec::new()) }

#[tauri::command]
#[cfg(target_os = "windows")]
pub fn disable_scheduled_tasks(names: Vec<String>) -> Result<OpResult, String> {
    let (mut ok, mut elev) = (0usize, 0usize);
    for n in names {
        let mut tn = n.trim().to_string();
        if !tn.starts_with('\\') { tn.insert(0, '\\'); }
        if Command::new("schtasks").args(["/Change", "/TN", &tn, "/Disable"]).status().map(|s| s.success()).unwrap_or(false) {
            ok += 1;
        } else if run_schtasks(&["/Change", "/TN", &tn, "/Disable"]) { ok += 1; elev += 1; }
    }
    Ok(OpResult { success: ok, elevated: elev, stopped: 0, failures: Vec::new() })
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub fn enable_scheduled_tasks(names: Vec<String>) -> Result<OpResult, String> {
    let (mut ok, mut elev) = (0usize, 0usize);
    for n in names {
        let mut tn = n.trim().to_string();
        if !tn.starts_with('\\') { tn.insert(0, '\\'); }
        if Command::new("schtasks").args(["/Change", "/TN", &tn, "/Enable"]).status().map(|s| s.success()).unwrap_or(false) {
            ok += 1;
        } else if run_schtasks(&["/Change", "/TN", &tn, "/Enable"]) { ok += 1; elev += 1; }
    }
    Ok(OpResult { success: ok, elevated: elev, stopped: 0, failures: Vec::new() })
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub fn delete_scheduled_tasks(names: Vec<String>) -> Result<OpResult, String> {
    let (mut ok, mut elev, mut stopped) = (0usize, 0usize, 0usize);
    let mut failures: Vec<TaskFailure> = Vec::new();
    for n in names {
        let mut tn = n.trim().to_string();
        if !tn.starts_with('\\') { tn.insert(0, '\\'); }
        let (ok1, so1, se1) = run_schtasks_capture(["/Delete", "/TN", &tn, "/F"].as_ref());
        if ok1 { ok += 1; continue; }
        failures.push(TaskFailure { name: tn.clone(), action: "delete".into(), step: "delete1".into(), stdout: so1, stderr: se1, elevated: false });
        let (ok2_end, so2_end, se2_end) = run_schtasks_capture(["/End", "/TN", &tn].as_ref());
        let (ok2, so2, se2) = run_schtasks_capture(["/Delete", "/TN", &tn, "/F"].as_ref());
        let _ = ok2_end;
        if ok2 { ok += 1; stopped += 1; continue; }
        failures.push(TaskFailure { name: tn.clone(), action: "delete".into(), step: "end+delete".into(), stdout: format!("{}\n{}", so2_end, so2), stderr: format!("{}\n{}", se2_end, se2), elevated: false });
        let _ = run_schtasks(&["/End", "/TN", &tn]);
        if run_schtasks(&["/Delete", "/TN", &tn, "/F"]) { ok += 1; elev += 1; stopped += 1; continue; }
        failures.push(TaskFailure { name: tn.clone(), action: "delete".into(), step: "elevated".into(), stdout: String::new(), stderr: String::new(), elevated: true });
        if try_delete_as_system(&tn) { ok += 1; elev += 1; stopped += 1; }
        else {
            failures.push(TaskFailure { name: tn.clone(), action: "delete".into(), step: "final".into(), stdout: String::new(), stderr: "Protected task (TrustedInstaller). Try disabling instead.".into(), elevated: true });
        }
    }
    Ok(OpResult { success: ok, elevated: elev, stopped, failures })
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub fn run_scheduled_tasks(names: Vec<String>) -> Result<OpResult, String> {
    let (mut ok, mut elev) = (0usize, 0usize);
    for n in names {
        let mut tn = n.trim().to_string();
        if !tn.starts_with('\\') { tn.insert(0, '\\'); }
        if Command::new("schtasks").args(["/Run", "/TN", &tn]).status().map(|s| s.success()).unwrap_or(false) {
            ok += 1;
        } else if run_schtasks(&["/Run", "/TN", &tn]) { ok += 1; elev += 1; }
    }
    Ok(OpResult { success: ok, elevated: elev, stopped: 0, failures: Vec::new() })
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub fn end_scheduled_tasks(names: Vec<String>) -> Result<OpResult, String> {
    let (mut ok, mut elev) = (0usize, 0usize);
    for n in names {
        let mut tn = n.trim().to_string();
        if !tn.starts_with('\\') { tn.insert(0, '\\'); }
        if Command::new("schtasks").args(["/End", "/TN", &tn]).status().map(|s| s.success()).unwrap_or(false) {
            ok += 1;
        } else if run_schtasks(&["/End", "/TN", &tn]) { ok += 1; elev += 1; }
    }
    Ok(OpResult { success: ok, elevated: elev, stopped: 0, failures: Vec::new() })
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn disable_scheduled_tasks(_names: Vec<String>) -> Result<OpResult, String> { Err("Only on Windows".into()) }
#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn enable_scheduled_tasks(_names: Vec<String>) -> Result<OpResult, String> { Err("Only on Windows".into()) }
#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn delete_scheduled_tasks(_names: Vec<String>) -> Result<OpResult, String> { Err("Only on Windows".into()) }
#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn run_scheduled_tasks(_names: Vec<String>) -> Result<OpResult, String> { Err("Only on Windows".into()) }
#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn end_scheduled_tasks(_names: Vec<String>) -> Result<OpResult, String> { Err("Only on Windows".into()) }

#[tauri::command]
#[cfg(target_os = "windows")]
pub fn get_task_details(task_name: String) -> Result<(String, String), String> {
    let tn = task_name.replace('"', "\"\"");
    let cmdline = format!("chcp 65001>nul & schtasks /Query /V /FO CSV /TN \"{}\"", tn);
    let output = Command::new("cmd")
        .args(["/C", &cmdline])
        .output()
        .map_err(|e| format!("failed to run schtasks /TN: {}", e))?;

    let stdout_bytes = output.stdout;
    if stdout_bytes.is_empty() {
        return Ok((String::new(), String::new()));
    }

    let first_line_end = stdout_bytes
        .iter()
        .position(|&b| b == b'\n')
        .unwrap_or(stdout_bytes.len());
    let first_line = &stdout_bytes[..first_line_end];
    let delim = if first_line
        .iter()
        .filter(|&&b| b == b';')
        .count()
        > first_line.iter().filter(|&&b| b == b',').count()
    {
        b';'
    } else {
        b','
    };

    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .delimiter(delim)
        .from_reader(&*stdout_bytes);

    let headers = rdr.headers().map_err(|e| e.to_string())?.clone();
    let norm: Vec<String> = headers
        .iter()
        .map(|h| h.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase())
        .collect();

    let find_idx_contains = |keys: &[&str]| -> Option<usize> {
        for (i, h) in norm.iter().enumerate() {
            for k in keys {
                if h.contains(k) {
                    return Some(i);
                }
            }
        }
        None
    };

    let idx_run = find_idx_contains(&["tasktorun", "programscript", "tasktoexecute", "aktion", "action", "program", "script"]);
    let idx_author = find_idx_contains(&["author", "creator", "forfattare", "autor", "skapatav"]);

    for rec in rdr.records() {
        if let Ok(rec) = rec {
            let task_to_run = idx_run.and_then(|i| rec.get(i)).unwrap_or("").to_string();
            let author = idx_author.and_then(|i| rec.get(i)).unwrap_or("").to_string();
            return Ok((task_to_run, author));
        }
    }

    Ok((String::new(), String::new()))
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn get_task_details(_task_name: String) -> Result<(String, String), String> {
    Err("get_task_details is only implemented on Windows".into())
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn list_scheduled_tasks() -> Result<Vec<ScheduledTask>, String> {
    Ok(Vec::new())
}

#[tauri::command]
pub fn get_startup_folders() -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    if let Some(appdata) = env::var_os("APPDATA") {
        let user_startup = PathBuf::from(appdata).join("Microsoft/Windows/Start Menu/Programs/Startup");
        out.push(user_startup.display().to_string());
    }
    if let Some(programdata) = env::var_os("PROGRAMDATA") {
        let all_startup = PathBuf::from(programdata).join("Microsoft/Windows/Start Menu/Programs/StartUp");
        out.push(all_startup.display().to_string());
    }
    Ok(out)
}

#[cfg(target_os = "windows")]
#[derive(Serialize, Deserialize, Clone)]
pub struct StartupRegItem {
    pub hive: String,   // "HKCU"/"HKLM"
    pub key: String,    // registry path
    pub name: String,
    pub command: String
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub fn list_registry_run() -> Result<Vec<StartupRegItem>, String> {
    use winreg::enums::*;
    use winreg::RegKey;

    let mut out: Vec<StartupRegItem> = Vec::new();

    let to_items = |hive_label: &str, key_path: &str, hive: &RegKey, out: &mut Vec<StartupRegItem>| {
        if let Ok(subkey) = hive.open_subkey(key_path) {
            for item in subkey.enum_values().flatten() {
                let (name, value) = (item.0, item.1);
                match value.vtype {
                    REG_SZ | REG_EXPAND_SZ => {
                        if let Ok(cmd) = subkey.get_value::<String, _>(&name) {
                            out.push(StartupRegItem {
                                hive: hive_label.to_string(),
                                key: key_path.to_string(),
                                name,
                                command: cmd,
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
    };

    let hku = RegKey::predef(HKEY_CURRENT_USER);
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

    let keys = [
        "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        "Software\\Microsoft\\Windows\\CurrentVersion\\RunOnce",
        "Software\\Wow6432Node\\Microsoft\\Windows\\CurrentVersion\\Run",
        "Software\\Wow6432Node\\Microsoft\\Windows\\CurrentVersion\\RunOnce",
    ];

    for k in keys.iter() {
        to_items("HKCU", k, &hku, &mut out);
        to_items("HKLM", k, &hklm, &mut out);
    }

    Ok(out)
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub fn remove_registry_run(entries: Vec<StartupRegItem>) -> Result<usize, String> {
    use winreg::enums::*;
    use winreg::RegKey;
    let mut count = 0usize;
    for e in entries {
        let hive = match e.hive.as_str() {
            "HKCU" => RegKey::predef(HKEY_CURRENT_USER),
            "HKLM" => RegKey::predef(HKEY_LOCAL_MACHINE),
            _ => continue,
        };
        let mut deleted = false;
        if let Ok(subkey) = hive.open_subkey_with_flags(&e.key, KEY_SET_VALUE) {
            if subkey.delete_value(&e.name).is_ok() {
                deleted = true;
            }
        }
        if deleted {
            if !registry_value_exists(&e) {
                count += 1;
                continue;
            }
        }

        if let Some(img) = extract_image_from_command(&e.command) {
            let _ = std::process::Command::new("taskkill")
                .args(["/IM", &img, "/F"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            #[cfg(target_os = "windows")]
            { let _ = run_cmd_elevated(&["/C", "taskkill", "/IM", &img, "/F"]); }
        }

        let key_path = format!("{}\\{}", e.hive, e.key);
        let args_base = ["delete", &key_path, "/v", &e.name, "/f"];
        if run_reg(&args_base) || run_reg(&["delete", &key_path, "/v", &e.name, "/f", "/reg:64"]) || run_reg(&["delete", &key_path, "/v", &e.name, "/f", "/reg:32"]) {
            if !registry_value_exists(&e) {
                count += 1;
                continue;
            }
        }

        let _ = run_reg_elevated(&args_base);
        let _ = run_reg_elevated(&["delete", &key_path, "/v", &e.name, "/f", "/reg:64"]);
        let _ = run_reg_elevated(&["delete", &key_path, "/v", &e.name, "/f", "/reg:32"]);
        if !registry_value_exists(&e) { count += 1; }
    }
    Ok(count)
}

fn extract_image_from_command(cmd: &str) -> Option<String> {
    let s = cmd.trim();
    if s.is_empty() { return None; }
    let first = if s.starts_with('"') {
        s.split('"').nth(1).unwrap_or("")
    } else {
        s.split_whitespace().next().unwrap_or("")
    };
    if first.is_empty() { return None; }
    let token = if first.to_lowercase().contains(".exe") { first } else {
        if let Some(idx) = s.to_lowercase().find(".exe") {
            let start = s[..=idx].rfind(|c| c == ' ' || c == '"').map(|i| i+1).unwrap_or(0);
            &s[start..=idx+3]
        } else { first }
    };
    let file = std::path::Path::new(token).file_name()?.to_string_lossy().to_string();
    if file.to_lowercase().ends_with(".exe") { Some(file) } else { None }
}

#[cfg(target_os = "windows")]
fn registry_value_exists(e: &StartupRegItem) -> bool {
    use winreg::enums::*;
    use winreg::RegKey;
    let hive = match e.hive.as_str() {
        "HKCU" => RegKey::predef(HKEY_CURRENT_USER),
        "HKLM" => RegKey::predef(HKEY_LOCAL_MACHINE),
        _ => return false,
    };
    let views = [0u32, KEY_WOW64_64KEY, KEY_WOW64_32KEY];
    for v in views {
        if let Ok(sub) = hive.open_subkey_with_flags(&e.key, KEY_READ | v) {
            if sub.get_value::<String, _>(&e.name).is_ok() { return true; }
        }
    }
    false
}

#[cfg(not(target_os = "windows"))]
fn registry_value_exists(_e: &StartupRegItem) -> bool { false }

#[tauri::command]
pub fn is_process_running(image: String) -> Result<bool, String> {
    let target = image.to_lowercase();
    let sys = System::new_all();
    let running = sys.processes().values().any(|p| {
        let name = p.name().to_string_lossy().to_string();
        name.eq_ignore_ascii_case(&image) || name.to_lowercase() == target
    });
    Ok(running)
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn list_registry_run() -> Result<Vec<StartupRegItem>, String> {
    Err("list_registry_run is only implemented on Windows".into())
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn remove_registry_run(_entries: Vec<StartupRegItem>) -> Result<usize, String> {
    Err("remove_registry_run is only implemented on Windows".into())
}

#[tauri::command]
pub fn remove_startup_shortcuts(files: Vec<String>) -> Result<usize, String> {
    let mut count = 0usize;
    for f in files {
        match trash::delete(&f) {
            Ok(_) => { count += 1; continue; }
            Err(e) => {
                eprintln!("[startup] trash delete failed {}: {}", f, e);
            }
        }
        if std::fs::remove_file(&f).is_ok() { count += 1; continue; }
        #[cfg(target_os = "windows")]
        {
            let quoted = format!("\"{}\"", f.replace('"', "\\\""));
            if run_cmd_elevated(&["/C", "del", "/F", "/Q", &quoted]) { count += 1; continue; }
        }
        eprintln!("[startup] failed to remove {} (even elevated)", f);
    }
    Ok(count)
}

#[cfg(target_os = "windows")]
fn fmt_command_text(output: &std::process::Output) -> String {
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
fn run_command_text(cmd: &str, args: &[&str]) -> Result<String, String> {
    use std::process::Command;
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
fn run_powershell_json(script: &str) -> Result<Vec<Value>, String> {
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
        Value::Array(mut results) => {
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
fn value_to_string(value: &Value) -> Option<String> {
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
fn push_candidate_value(result: &mut Vec<String>, candidate: &str) {
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
fn collect_string_values(value: Option<&Value>, keys: &[&str]) -> Vec<String> {
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
fn format_link_speed(value: &Value) -> Option<String> {
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

fn sanitize_ip_candidate(input: &str) -> Option<String> {
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

fn is_com_instance_string(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.starts_with("MSFT_") && trimmed.contains("Name =") {
        return true;
    }
    false
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub fn get_network_summary() -> Result<NetworkSummary, String> {
    let ip_script = "Get-NetIPConfiguration | Select InterfaceAlias,IPv4Address,IPv6Address,DnsServer,IPv4DefaultGateway | ConvertTo-Json";
    let adapter_script = "Get-NetAdapter | Select InterfaceAlias,Status,LinkSpeed,MacAddress,MediaType,InterfaceDescription,LinkState | ConvertTo-Json";
    let ip_data = run_powershell_json(ip_script)?;
    let adapter_data = run_powershell_json(adapter_script)?;
    let mut summary = NetworkSummary {
        primary_adapter: None,
        ipv4: None,
        ipv6: None,
        dns_servers: Vec::new(),
        gateways: Vec::new(),
        adapters: Vec::new(),
    };
    for entry in &ip_data {
        let alias = entry.get("InterfaceAlias").and_then(|v| v.as_str()).map(|s| s.to_string());
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
        for gw in collect_string_values(entry.get("IPv4DefaultGateway"), &["NextHop", "IPAddress"]) {
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
            status: entry.get("Status").and_then(|v| v.as_str()).map(|s| s.to_string()),
            link_speed: entry.get("LinkSpeed").and_then(format_link_speed),
            mac: entry.get("MacAddress").and_then(|v| v.as_str()).map(|s| s.to_string()),
            media: entry.get("MediaType").and_then(|v| v.as_str()).map(|s| s.to_string()),
            link_state: entry.get("LinkState").and_then(|v| v.as_str()).map(|s| s.to_string()),
        };
        adapters.push(info);
    }
    summary.adapters = adapters;
    Ok(summary)
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn get_network_summary() -> Result<NetworkSummary, String> {
    Err("Network summary is only implemented on Windows".into())
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub fn run_ping(host: String, count: Option<u8>) -> Result<String, String> {
    let trimmed = host.trim();
    if trimmed.is_empty() {
        return Err("Host is required".into());
    }
    let ping_count = count.unwrap_or(4).clamp(1, 8);
    let count_arg = ping_count.to_string();
    run_command_text("ping", &["-n", &count_arg, trimmed])
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn run_ping(_host: String, _count: Option<u8>) -> Result<String, String> {
    Err("Ping is only implemented on Windows".into())
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub fn run_traceroute(host: String) -> Result<String, String> {
    let trimmed = host.trim();
    if trimmed.is_empty() {
        return Err("Host is required".into());
    }
    run_command_text("tracert", &[trimmed])
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn run_traceroute(_host: String) -> Result<String, String> {
    Err("Traceroute is only implemented on Windows".into())
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub fn run_dns_lookup(host: String) -> Result<String, String> {
    let trimmed = host.trim();
    if trimmed.is_empty() {
        return Err("Host is required".into());
    }
    run_command_text("nslookup", &[trimmed])
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn run_dns_lookup(_host: String) -> Result<String, String> {
    Err("DNS lookup is only implemented on Windows".into())
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub fn flush_dns() -> Result<String, String> {
    run_command_text("ipconfig", &["/flushdns"])
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn flush_dns() -> Result<String, String> {
    Err("flush_dns is only implemented on Windows".into())
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub fn reset_winsock() -> Result<String, String> {
    run_command_text("netsh", &["winsock", "reset"])
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub fn renew_ip() -> Result<String, String> {
    let release = run_command_text("ipconfig", &["/release"])?;
    let renew = run_command_text("ipconfig", &["/renew"])?;
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
pub fn renew_ip() -> Result<String, String> { Err("Only on Windows".into()) }

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn reset_winsock() -> Result<String, String> { Err("Only on Windows".into()) }
fn run_schtasks(args: &[&str]) -> bool {
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

fn run_schtasks_capture(args: &[&str]) -> (bool, String, String) {
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

fn try_delete_as_system(target_tn: &str) -> bool {
    let mut rng = rand::thread_rng();
    let temp_name = format!("\\_AveloniaSysDel_{}", rng.r#gen::<u32>());
    let tr = format!("cmd.exe /c schtasks /Delete /TN \"{}\" /F", target_tn.replace('"', "\\\""));
    if !run_schtasks(&["/Create", "/TN", &temp_name, "/TR", &tr, "/SC", "ONCE", "/ST", "23:59", "/RU", "SYSTEM", "/RL", "HIGHEST"]) {
        return false;
    }
    let _ = run_schtasks(&["/Run", "/TN", &temp_name]);
    std::thread::sleep(Duration::from_secs(2));
    let exists = std::process::Command::new("schtasks")
        .args(["/Query", "/TN", target_tn])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    let _ = run_schtasks(&["/Delete", "/TN", &temp_name, "/F"]);
    !exists
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub fn open_registry_key(hive: String, key: String) -> Result<(), String> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hive_label = match hive.as_str() {
        "HKCU" | "HKEY_CURRENT_USER" => "HKEY_CURRENT_USER",
        "HKLM" | "HKEY_LOCAL_MACHINE" => "HKEY_LOCAL_MACHINE",
        other => other,
    };
    let full = format!("{}\\{}", hive_label, key);

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (regedit_key, _) = hkcu
        .create_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Applets\\Regedit")
        .map_err(|e| format!("failed to open Regedit key: {}", e))?;
    regedit_key
        .set_value("LastKey", &full)
        .map_err(|e| format!("failed to set LastKey: {}", e))?;

    match std::process::Command::new("regedit").arg("/m").spawn() {
        Ok(_) => Ok(()),
        Err(e) => {
            if let Some(740) = e.raw_os_error() {
                let arglist = "@('/m')".to_string();
                let ps = format!(
                    "Start-Process -FilePath regedit -ArgumentList {} -Verb RunAs",
                    arglist
                );
                let _ = std::process::Command::new("powershell")
                    .args([
                        "-NoProfile",
                        "-ExecutionPolicy",
                        "Bypass",
                        "-Command",
                        &ps,
                    ])
                    .spawn();
                return Ok(());
            }
            Err(format!("failed to launch regedit: {}", e))
        }
    }
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn open_registry_key(_hive: String, _key: String) -> Result<(), String> {
    Err("open_registry_key is only implemented on Windows".into())
}

#[cfg(target_os = "windows")]
fn run_cmd_elevated(args: &[&str]) -> bool {
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
fn run_reg(args: &[&str]) -> bool {
    std::process::Command::new("reg")
        .args(args)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
fn run_reg_elevated(args: &[&str]) -> bool {
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
fn run_powershell_elevated(args: &[&str]) -> bool {
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

#[tauri::command]
#[cfg(target_os = "windows")]
pub fn force_remove_registry_run(entries: Vec<StartupRegItem>) -> Result<usize, String> {
    use std::env;
    let mut script = String::from("$ErrorActionPreference='SilentlyContinue'\n");
    for e in &entries {
        let root = if e.hive.eq_ignore_ascii_case("HKLM") { "HKLM:" } else { "HKCU:" };
        let path = format!("{}{}", root, e.key);
        let name = e.name.replace("'", "''");
        script.push_str(&format!(
            "try {{ $p='{}'; $acl=Get-Acl $p; $adm=New-Object Security.Principal.NTAccount('Administrators'); $acl.SetOwner($adm); $rule=New-Object Security.AccessControl.RegistryAccessRule('Administrators','FullControl','ContainerInherit,ObjectInherit','None','Allow'); $acl.SetAccessRule($rule); Set-Acl $p $acl }} catch {{}}\n",
            path
        ));
        script.push_str(&format!(
            "try {{ Remove-ItemProperty -Path '{}' -Name '{}' -Force }} catch {{}}\n",
            path, name
        ));
    }
    let mut tmp = env::temp_dir();
    tmp.push("avelonia_force_remove.ps1");
    let tmp_str = tmp.to_string_lossy().to_string();
    std::fs::write(&tmp, script).map_err(|e| format!("write script failed: {}", e))?;
    let ok = run_powershell_elevated(&["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", &tmp_str]);
    let _ = std::fs::remove_file(&tmp);
    let mut removed = 0usize;
    for e in &entries {
        if !registry_value_exists(e) { removed += 1; }
    }
    if ok { Ok(removed) } else { Ok(removed) }
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn force_remove_registry_run(_entries: Vec<StartupRegItem>) -> Result<usize, String> {
    Err("Only available on Windows".into())
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub fn block_process_ifeo(images: Vec<String>, enable: bool) -> Result<usize, String> {
    use std::env;
    let mut script = String::from("$ErrorActionPreference='SilentlyContinue'\n");
    for img in &images {
        let name = img.replace("'", "''");
        script.push_str(&format!(
            "$k='HKLM:\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Image File Execution Options\\{}'\n",
            name
        ));
        if enable {
            script.push_str("New-Item -Path $k -Force | Out-Null\n");
            script.push_str("Set-ItemProperty -Path $k -Name Debugger -Value 'cmd.exe /c exit 0' -Force\n");
        } else {
            script.push_str("Remove-ItemProperty -Path $k -Name Debugger -ErrorAction SilentlyContinue\n");
            script.push_str("Remove-Item -Path $k -ErrorAction SilentlyContinue\n");
        }
    }
    let mut tmp = env::temp_dir();
    tmp.push("avelonia_ifeo.ps1");
    let tmp_str = tmp.to_string_lossy().to_string();
    std::fs::write(&tmp, script).map_err(|e| format!("write script failed: {}", e))?;
    let ok = run_powershell_elevated(&["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", &tmp_str]);
    let _ = std::fs::remove_file(&tmp);
    if ok { Ok(images.len()) } else { Ok(0) }
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn block_process_ifeo(_images: Vec<String>, _enable: bool) -> Result<usize, String> {
    Err("Only available on Windows".into())
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub fn schedule_delete_on_reboot(paths: Vec<String>) -> Result<usize, String> {
    use std::env;
    let mut script = String::from("$ErrorActionPreference='SilentlyContinue'\n");
    script.push_str("$sm='HKLM:\\SYSTEM\\CurrentControlSet\\Control\\Session Manager'\n");
    script.push_str("$val=(Get-ItemProperty -Path $sm -Name PendingFileRenameOperations -ErrorAction SilentlyContinue).PendingFileRenameOperations\n");
    script.push_str("if(-not $val){ $val=@() }\n");
    for p in &paths {
        let clean = p.replace("'", "''");
        script.push_str(&format!(
            "$pp=[Environment]::ExpandEnvironmentVariables('{}')\n$val += ('\\??\\' + $pp), ''\n",
            clean
        ));
    }
    script.push_str("Set-ItemProperty -Path $sm -Name PendingFileRenameOperations -Value $val -Type MultiString\n");
    let mut tmp = env::temp_dir();
    tmp.push("avelonia_delete_on_reboot.ps1");
    let tmp_str = tmp.to_string_lossy().to_string();
    std::fs::write(&tmp, script).map_err(|e| format!("write script failed: {}", e))?;
    let ok = run_powershell_elevated(&["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", &tmp_str]);
    let _ = std::fs::remove_file(&tmp);
    if ok { Ok(paths.len()) } else { Ok(0) }
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn schedule_delete_on_reboot(_paths: Vec<String>) -> Result<usize, String> {
    Err("Only available on Windows".into())
}

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
pub fn list_services() -> Result<Vec<ServiceInfo>, String> {
    let ps = r#"Get-CimInstance Win32_Service | Select-Object Name,DisplayName,State,StartMode,PathName | ConvertTo-Json -Depth 3"#;
    let out = Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", ps])
        .output()
        .map_err(|e| format!("failed to run powershell Get-CimInstance: {}", e))?;
    if !out.status.success() { return Ok(Vec::new()); }
    let stdout: String = String::from_utf8_lossy(&out.stdout).to_string();
    let result: serde_json::Value = serde_json::from_str(&stdout).map_err(|e| format!("json parse failed: {}", e))?;
    let mut out_vec: Vec<ServiceInfo> = Vec::new();
    match result {
        serde_json::Value::Array(arr) => {
            for v in arr {
                let name = v.get("Name").and_then(|x| x.as_str()).unwrap_or("").to_string();
                let display_name = v.get("DisplayName").and_then(|x| x.as_str()).unwrap_or("").to_string();
                let state = v.get("State").and_then(|x| x.as_str()).unwrap_or("").to_string();
                let start_mode = v.get("StartMode").and_then(|x| x.as_str()).unwrap_or("").to_string();
                let path = v.get("PathName").and_then(|x| x.as_str()).unwrap_or("").to_string();
                out_vec.push(ServiceInfo { name, display_name, state, start_mode, path });
            }
        }
        serde_json::Value::Object(v) => {
            let name = v.get("Name").and_then(|x| x.as_str()).unwrap_or("").to_string();
            let display_name = v.get("DisplayName").and_then(|x| x.as_str()).unwrap_or("").to_string();
            let state = v.get("State").and_then(|x| x.as_str()).unwrap_or("").to_string();
            let start_mode = v.get("StartMode").and_then(|x| x.as_str()).unwrap_or("").to_string();
            let path = v.get("PathName").and_then(|x| x.as_str()).unwrap_or("").to_string();
            out_vec.push(ServiceInfo { name, display_name, state, start_mode, path });
        }
        _ => {}
    }
    Ok(out_vec)
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub fn stop_services(names: Vec<String>) -> Result<usize, String> {
    let mut ok = 0usize;
    for n in names {
        if Command::new("sc").args(["stop", &n]).status().map(|s| s.success()).unwrap_or(false) { ok += 1; continue; }
        let _ = run_cmd_elevated(&["/C", "sc", "stop", &n]);
    }
    Ok(ok)
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub fn disable_services(names: Vec<String>) -> Result<usize, String> {
    let mut ok = 0usize;
    for n in names {
        if Command::new("sc").args(["config", &n, "start=", "disabled"]).status().map(|s| s.success()).unwrap_or(false) { ok += 1; continue; }
        let _ = run_cmd_elevated(&["/C", "sc", "config", &n, "start=", "disabled"]);
    }
    Ok(ok)
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn list_services() -> Result<Vec<ServiceInfo>, String> { Ok(Vec::new()) }
#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn stop_services(_names: Vec<String>) -> Result<usize, String> { Err("Only on Windows".into()) }
#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn disable_services(_names: Vec<String>) -> Result<usize, String> { Err("Only on Windows".into()) }

#[tauri::command]
#[cfg(target_os = "windows")]
pub fn purge_startup_approved(names: Vec<String>) -> Result<usize, String> {
    use winreg::enums::*;
    use winreg::RegKey;
    let hives = [
        ("HKCU", RegKey::predef(HKEY_CURRENT_USER)),
        ("HKLM", RegKey::predef(HKEY_LOCAL_MACHINE)),
    ];
    let subkeys = [
        "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\StartupApproved\\Run",
        "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\StartupApproved\\Run32",
    ];
    let mut removed = 0usize;
    for (_label, hive) in hives.iter() {
        for sk in subkeys.iter() {
            if let Ok(sub) = hive.open_subkey_with_flags(sk, KEY_SET_VALUE) {
                for n in &names {
                    let _ = sub.delete_value(n).map(|_| removed += 1);
                }
            }
        }
    }
    Ok(removed)
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn purge_startup_approved(_names: Vec<String>) -> Result<usize, String> { Ok(0) }

#[tauri::command]
#[cfg(target_os = "windows")]
pub fn delete_tasks_by_match(images: Vec<String>, paths: Vec<String>) -> Result<usize, String> {
    let output = Command::new("cmd")
        .args(["/C", "chcp 65001>nul & schtasks /Query /V /FO CSV"])
        .output()
        .map_err(|e| format!("failed to run schtasks /V: {}", e))?;
    if !output.status.success() { return Ok(0); }
    let stdout_bytes = output.stdout;
    let first_line_end = stdout_bytes.iter().position(|&b| b == b'\n').unwrap_or(stdout_bytes.len());
    let first_line = &stdout_bytes[..first_line_end];
    let delim = if first_line.iter().filter(|&&b| b == b';').count() > first_line.iter().filter(|&&b| b == b',').count() { b';' } else { b',' };
    let mut rdr = csv::ReaderBuilder::new().has_headers(true).delimiter(delim).from_reader(&*stdout_bytes);
    let headers = rdr.headers().map_err(|e| e.to_string())?.clone();
    let norm: Vec<String> = headers.iter().map(|h| h.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase()).collect();
    let find_idx_contains = |keys: &[&str]| -> Option<usize> {
        for (i, h) in norm.iter().enumerate() {
            for k in keys { if h.contains(k) { return Some(i); } }
        }
        None
    };
    let idx_name = find_idx_contains(&["taskname", "taskpath"]).unwrap_or(0);
    let idx_run = find_idx_contains(&["tasktorun", "programscript", "tasktoexecute", "aktion", "action", "program", "script"]);
    let mut matches: Vec<String> = Vec::new();
    for rec in rdr.records().flatten() {
        let name = rec.get(idx_name).unwrap_or("").to_string();
        let run = idx_run.and_then(|i| rec.get(i)).unwrap_or("").to_lowercase();
        let hit = images.iter().any(|s| run.contains(&s.to_lowercase())) || paths.iter().any(|s| run.contains(&s.to_lowercase()));
        if hit && !name.trim().is_empty() { matches.push(name); }
    }
    let mut ok = 0usize;
    for tn in matches {
        let mut taskname = tn.trim().to_string();
        if !taskname.starts_with('\\') { taskname.insert(0, '\\'); }
        if Command::new("schtasks").args(["/Delete", "/TN", &taskname, "/F"]).status().map(|s| s.success()).unwrap_or(false) { ok += 1; continue; }
        if run_schtasks(&["/Delete", "/TN", &taskname, "/F"]) { ok += 1; }
    }
    Ok(ok)
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn delete_tasks_by_match(_images: Vec<String>, _paths: Vec<String>) -> Result<usize, String> { Ok(0) }

#[tauri::command]
#[cfg(target_os = "windows")]
pub fn remove_wmi_subscriptions_by_match(images: Vec<String>, paths: Vec<String>) -> Result<usize, String> {
    let mut script = String::from("$ErrorActionPreference='SilentlyContinue'\n");
    script.push_str("$images = @()\n$paths = @()\n");
    for i in &images { script.push_str(&format!("$images += '{}\n'", i.replace("'", "''"))); }
    for p in &paths { script.push_str(&format!("$paths += '{}\n'", p.replace("'", "''"))); }
    script.push_str(r#"
function Hit([string]$s){ $t=$s.ToLower(); foreach($x in $images){ if($t.Contains($x.ToLower())){ return $true } } foreach($y in $paths){ if($t.Contains($y.ToLower())){ return $true } } return $false }
$removed = 0
$cons = Get-CimInstance -Namespace root\subscription -ClassName CommandLineEventConsumer
foreach($c in $cons){ if(Hit([string]$c.CommandLineTemplate)){ Remove-CimInstance $c; $removed++ } }
$ascons = Get-CimInstance -Namespace root\subscription -ClassName ActiveScriptEventConsumer
foreach($c in $ascons){ if(Hit([string]$c.ScriptText)){ Remove-CimInstance $c; $removed++ } }
$binds = Get-CimInstance -Namespace root\subscription -ClassName __FilterToConsumerBinding
foreach($b in $binds){ try { $fc = (Get-CimAssociatedInstance -InputObject $b -Association __FilterToConsumerBinding); if(-not $fc){ Remove-CimInstance $b; $removed++ } } catch {} }
$filters = Get-CimInstance -Namespace root\subscription -ClassName __EventFilter
foreach($f in $filters){ if(Hit([string]$f.Query)){ Remove-CimInstance $f; $removed++ } }
Write-Output $removed
"#);
    use std::env;
    let mut tmp = env::temp_dir();
    tmp.push("avelonia_wmi_cleanup.ps1");
    let tmp_str = tmp.to_string_lossy().to_string();
    std::fs::write(&tmp, script).map_err(|e| format!("write script failed: {}", e))?;
    let ok = run_powershell_elevated(&["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", &tmp_str]);
    let _ = std::fs::remove_file(&tmp);
    Ok(if ok { 1 } else { 0 })
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn remove_wmi_subscriptions_by_match(_images: Vec<String>, _paths: Vec<String>) -> Result<usize, String> { Ok(0) }

#[tauri::command]
#[cfg(target_os = "windows")]
pub fn restart_system() -> Result<(), String> {
    let ok = run_cmd_elevated(&["/C", "shutdown", "/r", "/t", "0"]);
    if ok { Ok(()) } else { Err("failed to trigger restart".into()) }
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn restart_system() -> Result<(), String> { Err("Only available on Windows".into()) }

#[cfg(target_os = "windows")]
fn run_fix_action_impl(action_id: &str) -> Result<String, String> {
    println!("[fix] queued action: {}", action_id);
    Ok(format!("Fix '{}' queued (stub)", action_id))
}

#[cfg(not(target_os = "windows"))]
fn run_fix_action_impl(_action_id: &str) -> Result<String, String> {
    Err("Only on Windows".into())
}

#[cfg(target_os = "windows")]
fn apply_update_profile_impl(profile: &str) -> Result<String, String> {
    println!("[update profile] {}", profile);
    Ok(format!("Update profile '{}' queued (stub)", profile))
}

#[cfg(not(target_os = "windows"))]
fn apply_update_profile_impl(_profile: &str) -> Result<String, String> {
    Err("Only on Windows".into())
}

#[tauri::command]
pub fn apply_tweaks(payload: TweakApplyRequest) -> Result<TweakApplyResponse, String> {
    if !payload.tweaks.is_empty() {
        println!("[tweaks] requested: {:?}", payload.tweaks);
    }
    if !payload.configs.is_empty() {
        println!("[configs] requested: {:?}", payload.configs);
    }
    if let Some(ref profile) = payload.update_profile {
        if let Err(err) = apply_update_profile_impl(profile) {
            eprintln!("[apply_tweaks] update profile failed: {}", err);
        }
    }
    Ok(TweakApplyResponse {
        tweaks_applied: payload.tweaks.len(),
        configs_applied: payload.configs.len(),
        profile_applied: payload.update_profile,
    })
}

#[tauri::command]
pub fn run_fix_action(action_id: String) -> Result<String, String> {
    run_fix_action_impl(&action_id)
}

#[tauri::command]
pub fn apply_update_profile(profile: String) -> Result<String, String> {
    apply_update_profile_impl(&profile)
}
