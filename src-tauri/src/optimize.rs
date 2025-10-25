use std::env;
use std::path::PathBuf;
use walkdir::WalkDir;
use serde::{Serialize, Deserialize};
use lnk::{ShellLink};
use lnk::encoding::WINDOWS_1252;
use std::process::{Command, Stdio};
use std::str;
use std::io;
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

#[tauri::command]
pub fn list_startup_shortcuts() -> Result<Vec<StartupShortcut>, String> {
    let mut items: Vec<StartupShortcut> = Vec::new();
    // User startup folder
    if let Some(appdata) = env::var_os("APPDATA") {
        let user_startup = PathBuf::from(appdata).join("Microsoft/Windows/Start Menu/Programs/Startup");
        if user_startup.exists() && user_startup.is_dir() {
            for entry in WalkDir::new(&user_startup).min_depth(1).max_depth(1).into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_file() {
                    let p = entry.path();
                    // Skip system/metadata files like desktop.ini
                    if let Some(fname) = p.file_name().and_then(|s| s.to_str()) {
                        if fname.eq_ignore_ascii_case("desktop.ini") { continue; }
                    }
                    // Only surface typical startup shortcuts (.lnk, .url)
                    let mut allowed = false;
                    if let Some(ext) = p.extension() {
                        if ext.eq_ignore_ascii_case("lnk") || ext.eq_ignore_ascii_case("url") { allowed = true; }
                    }
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
    // All users startup
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
                    if let Some(ext) = p.extension() {
                        if ext.eq_ignore_ascii_case("lnk") || ext.eq_ignore_ascii_case("url") { allowed = true; }
                    }
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
    // First try PowerShell (more reliable across locales) --------------------
    #[derive(Deserialize)]
    struct PsTask {
        name: Option<String>,
        next_run_time: Option<String>,
        status: Option<String>,
        task_to_run: Option<String>,
        author: Option<String>,
    }

    // Disabled PowerShell path to avoid heavy queries that may freeze the UI.
    let ps: Result<std::process::Output, io::Error> = Err(io::Error::new(io::ErrorKind::Other, "disabled"));

    let mut tasks: Vec<ScheduledTask> = Vec::new();
    if let Ok(ps_out) = ps {
        if ps_out.status.success() {
            if let Ok(json) = String::from_utf8(ps_out.stdout) {
                let parsed: Result<Vec<PsTask>, _> = serde_json::from_str(&json).or_else(|_| {
                    // When only one item, PS outputs an object, not an array
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

    // Fallback to schtasks CSV ------------------------------------------------
    // Force UTF-8 code page to avoid mojibake and locale issues
    let output = Command::new("cmd")
        .args(["/C", "chcp 65001>nul & schtasks /Query /FO CSV"])
        .output()
        .map_err(|e| format!("failed to run schtasks: {}", e))?;

    let stdout_bytes = output.stdout;
    // Detect delimiter from the first line (some locales use ';')
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

    // Normalise header labels: lowercase and remove non-alphanumeric
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

    // Try to find best-effort indices across locales, preferring exact matches
    let idx_name = find_idx_exact(&["taskname"]).or_else(|| find_idx_exact(&["taskpath"]))
        .or_else(|| find_idx_contains(&["taskname", "taskpath"]))
        .unwrap_or(0);
    let idx_next = find_idx_exact(&["nextruntime"]).or_else(|| find_idx_contains(&["nextruntime", "nextrun", "nextstart", "nast"]))
        .unwrap_or(idx_name);
    let idx_status = find_idx_exact(&["status"]).or_else(|| find_idx_contains(&["status"]))
        .unwrap_or(idx_name);
    // Optional columns when not using verbose output
    let idx_run_opt = find_idx_exact(&["tasktorun"]).or_else(|| find_idx_contains(&["tasktorun", "programscript", "tasktoexecute", "aktion", "action", "program", "script"])) ;
    let idx_author_opt = find_idx_exact(&["author"]).or_else(|| find_idx_contains(&["author", "creator", "forfattare", "autor", "skapatav"])) ;

    // reuse tasks vec
    for result in rdr.records() {
        if let Ok(rec) = result {
            let name = rec.get(idx_name).unwrap_or("").to_string();
            // Skip any accidental header-echo rows or empties
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

    // If parsing produced nothing, try a simpler query without /V as fallback
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
            // Skip accidental header row or empties
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

    // Safety cap to avoid extremely large payloads freezing the UI
    if tasks.len() > 5000 { tasks.truncate(5000); }
    Ok(tasks)
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub fn list_suspicious_tasks() -> Result<Vec<String>, String> {
    // Scan verbose CSV once and return only names of suspicious tasks
    let output = Command::new("cmd")
        .args(["/C", "chcp 65001>nul & schtasks /Query /V /FO CSV"])
        .output()
        .map_err(|e| format!("failed to run schtasks /V: {}", e))?;

    let stdout_bytes = output.stdout;
    if stdout_bytes.is_empty() {
        return Ok(Vec::new());
    }

    // Detect delimiter from the first line (some locales use ';')
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

    // Normalise headers (lowercase, alphanumeric only)
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
    // dedupe
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
        // try normal delete
        let (ok1, so1, se1) = run_schtasks_capture(["/Delete", "/TN", &tn, "/F"].as_ref());
        if ok1 { ok += 1; continue; }
        failures.push(TaskFailure { name: tn.clone(), action: "delete".into(), step: "delete1".into(), stdout: so1, stderr: se1, elevated: false });
        // try stop then delete (non-elevated)
        let (ok2_end, so2_end, se2_end) = run_schtasks_capture(["/End", "/TN", &tn].as_ref());
        let (ok2, so2, se2) = run_schtasks_capture(["/Delete", "/TN", &tn, "/F"].as_ref());
        let _ = ok2_end; // silence unused var if needed
        if ok2 { ok += 1; stopped += 1; continue; }
        failures.push(TaskFailure { name: tn.clone(), action: "delete".into(), step: "end+delete".into(), stdout: format!("{}\n{}", so2_end, so2), stderr: format!("{}\n{}", se2_end, se2), elevated: false });
        // elevate stop and delete
        let _ = run_schtasks(&["/End", "/TN", &tn]);
        if run_schtasks(&["/Delete", "/TN", &tn, "/F"]) { ok += 1; elev += 1; stopped += 1; continue; }
        failures.push(TaskFailure { name: tn.clone(), action: "delete".into(), step: "elevated".into(), stdout: String::new(), stderr: String::new(), elevated: true });
        // last resort: try delete as SYSTEM using a temporary SYSTEM task
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
    // Query a single task in verbose CSV to fetch Task To Run and Author
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

    // Detect delimiter
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
    pub hive: String,   // "HKCU" or "HKLM"
    pub key: String,    // registry path
    pub name: String,   // value name
    pub command: String // value data
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
                // Accept string values only
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

    // Standard Run and RunOnce
    let keys = [
        "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        "Software\\Microsoft\\Windows\\CurrentVersion\\RunOnce",
        // 32-bit view on 64-bit Windows
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
        // First attempt: direct delete via WinReg
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
            // fallthrough if still present (e.g., re-added immediately)
        }

        // Attempt to terminate likely process if known (reduces instant re-add)
        if let Some(img) = extract_image_from_command(&e.command) {
            let _ = std::process::Command::new("taskkill")
                .args(["/IM", &img, "/F"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            #[cfg(target_os = "windows")]
            { let _ = run_cmd_elevated(&["/C", "taskkill", "/IM", &img, "/F"]); }
        }

        // Fallback 1: try reg.exe non-elevated
        let key_path = format!("{}\\{}", e.hive, e.key);
        let args_base = ["delete", &key_path, "/v", &e.name, "/f"];
        if run_reg(&args_base) || run_reg(&["delete", &key_path, "/v", &e.name, "/f", "/reg:64"]) || run_reg(&["delete", &key_path, "/v", &e.name, "/f", "/reg:32"]) {
            if !registry_value_exists(&e) {
                count += 1;
                continue;
            }
        }

        // If still not removed, try elevated via PowerShell UAC prompt
        let _ = run_reg_elevated(&args_base);
        let _ = run_reg_elevated(&["delete", &key_path, "/v", &e.name, "/f", "/reg:64"]);
        let _ = run_reg_elevated(&["delete", &key_path, "/v", &e.name, "/f", "/reg:32"]);
        if !registry_value_exists(&e) { count += 1; }
    }
    Ok(count)
}

// Try to extract process image name from registry command
fn extract_image_from_command(cmd: &str) -> Option<String> {
    let s = cmd.trim();
    if s.is_empty() { return None; }
    let first = if s.starts_with('"') {
        // quoted
        s.split('"').nth(1).unwrap_or("")
    } else {
        s.split_whitespace().next().unwrap_or("")
    };
    if first.is_empty() { return None; }
    // Find .exe token
    let token = if first.to_lowercase().contains(".exe") { first } else {
        // Sometimes commands are like: cmd.exe /c "path\app.exe ..."
        // Try to find any .exe in full string
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
    // Try both views where relevant
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
    // Build with all data to avoid needing refresh traits/signatures
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
        // Try to move to Recycle Bin
        match trash::delete(&f) {
            Ok(_) => { count += 1; continue; }
            Err(e) => {
                eprintln!("[startup] trash delete failed {}: {}", f, e);
            }
        }
        // Fallback: try direct delete (non-elevated)
        if std::fs::remove_file(&f).is_ok() { count += 1; continue; }
        // Final fallback: elevated forced delete via cmd.exe
        #[cfg(target_os = "windows")]
        {
            let quoted = format!("\"{}\"", f.replace('"', "\\\""));
            if run_cmd_elevated(&["/C", "del", "/F", "/Q", &quoted]) { count += 1; continue; }
        }
        eprintln!("[startup] failed to remove {} (even elevated)", f);
    }
    Ok(count)
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub fn flush_dns() -> Result<(), String> {
    use std::process::Command;
    let status = Command::new("ipconfig").args(["/flushdns"]).status()
        .map_err(|e| format!("failed to run ipconfig: {}", e))?;
    if status.success() { Ok(()) } else { Err(format!("ipconfig exited with status {:?}", status.code())) }
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn flush_dns() -> Result<(), String> {
    Err("flush_dns is only implemented on Windows".into())
}

// quick_clear_* moved to cleaner.rs

#[tauri::command]
#[cfg(target_os = "windows")]
pub fn reset_winsock() -> Result<(), String> {
    use std::process::Command;
    let status = Command::new("netsh").args(["winsock", "reset"]).status()
        .map_err(|e| format!("failed to run netsh: {}", e))?;
    if status.success() { Ok(()) } else { Err(format!("netsh exited with status {:?}", status.code())) }
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub fn renew_ip() -> Result<(), String> {
    use std::process::Command;
    let release = Command::new("ipconfig").args(["/release"]).status()
        .map_err(|e| format!("failed to run ipconfig /release: {}", e))?;
    if !release.success() { return Err(format!("ipconfig /release exited with status {:?}", release.code())); }
    let renew = Command::new("ipconfig").args(["/renew"]).status()
        .map_err(|e| format!("failed to run ipconfig /renew: {}", e))?;
    if renew.success() { Ok(()) } else { Err(format!("ipconfig /renew exited with status {:?}", renew.code())) }
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn reset_winsock() -> Result<(), String> { Err("Only on Windows".into()) }
#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn renew_ip() -> Result<(), String> { Err("Only on Windows".into()) }
fn run_schtasks(args: &[&str]) -> bool {
    // First, try without elevation
    if std::process::Command::new("schtasks")
        .args(args)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        return true;
    }

    // Retry elevated via PowerShell UAC prompt. Build a proper string array for -ArgumentList
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
    // Create a temporary SYSTEM task that deletes the target task, run it, then remove temp task
    let mut rng = rand::thread_rng();
    let temp_name = format!("\\_AveloniaSysDel_{}", rng.gen::<u32>());
    // Command to delete target (quoted)
    let tr = format!("cmd.exe /c schtasks /Delete /TN \"{}\" /F", target_tn.replace('"', "\\\""));
    // Create as SYSTEM
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
    // Use non-localized root (omit the localized "Computer" prefix like "Dator")
    // Regedit honors LastKey when it starts; providing HKEY_* avoids locale issues
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
            // If elevation is required (os error 740), retry elevated
            if let Some(740) = e.raw_os_error() {
                // Build PowerShell -ArgumentList array (no -Wait to avoid blocking UI)
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

// Generic elevated process runner for cmd.exe commands
#[cfg(target_os = "windows")]
fn run_cmd_elevated(args: &[&str]) -> bool {
    // Build PowerShell -ArgumentList array
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

// Try reg.exe without elevation
#[cfg(target_os = "windows")]
fn run_reg(args: &[&str]) -> bool {
    std::process::Command::new("reg")
        .args(args)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

// Try reg.exe with elevation via PowerShell UAC prompt
#[cfg(target_os = "windows")]
fn run_reg_elevated(args: &[&str]) -> bool {
    // Build PowerShell array for -ArgumentList
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
    // Build -ArgumentList array for nested PowerShell
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
    // Generate a PS1 that takes ownership, grants Administrators FullControl, and removes values
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
    // Verify removals
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
        // Expand env vars at runtime
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

// ---------------- Services management (Windows) ----------------

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
    // Prefer PowerShell CIM for reliable JSON across locales
    let ps = r#"Get-CimInstance Win32_Service | Select-Object Name,DisplayName,State,StartMode,PathName | ConvertTo-Json -Depth 3"#;
    let out = Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", ps])
        .output()
        .map_err(|e| format!("failed to run powershell Get-CimInstance: {}", e))?;
    if !out.status.success() { return Ok(Vec::new()); }
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    // PowerShell emits either an array or a single object
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
        // sc.exe expects 'start= disabled' with space after '='; pass as two args to be safe
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

// Purge StartupApproved entries for given value names (both HKCU/HKLM, Run/Run32)
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

// Delete scheduled tasks whose action path or command matches any image/path substring
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
    // Delete matched tasks (try non-elevated then elevated)
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

// Remove WMI permanent event subscriptions/consumers that run suspicious executables
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

// ---------------- System restart (Windows) ----------------

#[tauri::command]
#[cfg(target_os = "windows")]
pub fn restart_system() -> Result<(), String> {
    // Use elevated cmd.exe to invoke shutdown /r /t 0
    let ok = run_cmd_elevated(&["/C", "shutdown", "/r", "/t", "0"]);
    if ok { Ok(()) } else { Err("failed to trigger restart".into()) }
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn restart_system() -> Result<(), String> { Err("Only available on Windows".into()) }
