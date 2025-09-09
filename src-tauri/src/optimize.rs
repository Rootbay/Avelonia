use std::env;
use std::path::PathBuf;
use walkdir::WalkDir;
use serde::{Serialize, Deserialize};

#[tauri::command]
pub fn list_startup_shortcuts() -> Result<Vec<String>, String> {
    let mut items = Vec::new();
    // User startup folder
    if let Some(appdata) = env::var_os("APPDATA") {
        let user_startup = PathBuf::from(appdata).join("Microsoft/Windows/Start Menu/Programs/Startup");
        if user_startup.exists() && user_startup.is_dir() {
            for entry in WalkDir::new(&user_startup).min_depth(1).max_depth(1).into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_file() {
                    items.push(entry.path().display().to_string());
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
                    items.push(entry.path().display().to_string());
                }
            }
        }
    }
    Ok(items)
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
        if let Ok(subkey) = hive.open_subkey_with_flags(&e.key, KEY_SET_VALUE) {
            if subkey.delete_value(&e.name).is_ok() { count += 1; }
        }
    }
    Ok(count)
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
            Ok(_) => count += 1,
            Err(e) => eprintln!("Failed to move startup item to Trash {}: {}", f, e),
        }
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
