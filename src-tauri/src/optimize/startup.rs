use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;
use walkdir::WalkDir;
use lnk::ShellLink;
use lnk::encoding::WINDOWS_1252;
use super::shell_helpers::{run_cmd_elevated, run_reg, run_reg_elevated, run_powershell_elevated};

#[derive(Serialize, Clone)]
pub struct StartupShortcut {
    pub path: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StartupRegItem {
    pub hive: String,
    pub key: String,
    pub name: String,
    pub command: String,
}

use crate::AppError;

#[tauri::command]
pub async fn list_startup_shortcuts() -> Result<Vec<StartupShortcut>, AppError> {
    let mut items: Vec<StartupShortcut> = Vec::new();
    let wp = crate::paths::WindowsPaths::get();
    
    let user_startup = wp.startup_user();
    if user_startup.exists() && user_startup.is_dir() {
        for entry in WalkDir::new(&user_startup)
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                let p = entry.path();
                if let Some(fname) = p.file_name().and_then(|s| s.to_str()) {
                    if fname.eq_ignore_ascii_case("desktop.ini") {
                        continue;
                    }
                }
                let mut allowed = false;
                if let Some(ext) = p.extension() {
                    if ext.eq_ignore_ascii_case("lnk")
                        || ext.eq_ignore_ascii_case("url")
                        || ext.eq_ignore_ascii_case("exe")
                    {
                        allowed = true;
                    }
                }
                if !allowed {
                    continue;
                }

                let mut name = p
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();
                if let Some(ext) = p.extension() {
                    if ext.eq_ignore_ascii_case("lnk") {
                        if let Ok(link) = ShellLink::open(p, WINDOWS_1252) {
                            if let Some(li) = link.link_info() {
                                let target = li.common_path_suffix();
                                let target_name = PathBuf::from(target.to_string())
                                    .file_stem()
                                    .and_then(|s| s.to_str())
                                    .map(|s| s.to_string());
                                if let Some(n) = target_name {
                                    name = n;
                                }
                            }
                        }
                    }
                }
                if name.is_empty() {
                    name = p.display().to_string();
                }
                items.push(StartupShortcut {
                    path: p.display().to_string(),
                    name,
                });
            }
        }
    }

    let all_startup = wp.startup_common();
    if all_startup.exists() && all_startup.is_dir() {
        for entry in WalkDir::new(&all_startup)
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                let p = entry.path();
                if let Some(fname) = p.file_name().and_then(|s| s.to_str()) {
                    if fname.eq_ignore_ascii_case("desktop.ini") {
                        continue;
                    }
                }
                let mut allowed = false;
                if let Some(ext) = p.extension() {
                    if ext.eq_ignore_ascii_case("lnk")
                        || ext.eq_ignore_ascii_case("url")
                        || ext.eq_ignore_ascii_case("exe")
                    {
                        allowed = true;
                    }
                }
                if !allowed {
                    continue;
                }

                let mut name = p
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();
                if let Some(ext) = p.extension() {
                    if ext.eq_ignore_ascii_case("lnk") {
                        if let Ok(link) = ShellLink::open(p, WINDOWS_1252) {
                            if let Some(li) = link.link_info() {
                                let target = li.common_path_suffix();
                                let target_name = PathBuf::from(target.to_string())
                                    .file_stem()
                                    .and_then(|s| s.to_str())
                                    .map(|s| s.to_string());
                                if let Some(n) = target_name {
                                    name = n;
                                }
                            }
                        }
                    }
                }
                if name.is_empty() {
                    name = p.display().to_string();
                }
                items.push(StartupShortcut {
                    path: p.display().to_string(),
                    name,
                });
            }
        }
    }
    Ok(items)
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub async fn list_registry_run() -> Result<Vec<StartupRegItem>, AppError> {
    use winreg::RegKey;
    use winreg::enums::*;

    let mut out: Vec<StartupRegItem> = Vec::new();

    let to_items =
        |hive_label: &str, key_path: &str, hive: &RegKey, out: &mut Vec<StartupRegItem>| {
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
        r"Software\Microsoft\Windows\CurrentVersion\Run",
        r"Software\Microsoft\Windows\CurrentVersion\RunOnce",
        r"Software\Wow6432Node\Microsoft\Windows\CurrentVersion\Run",
        r"Software\Wow6432Node\Microsoft\Windows\CurrentVersion\RunOnce",
    ];

    for k in keys.iter() {
        to_items("HKCU", k, &hku, &mut out);
        to_items("HKLM", k, &hklm, &mut out);
    }

    Ok(out)
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub async fn list_registry_run() -> Result<Vec<StartupRegItem>, AppError> {
    Err(AppError::System("list_registry_run is only implemented on Windows".into()))
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub async fn remove_registry_run(entries: Vec<StartupRegItem>) -> Result<usize, AppError> {
    use winreg::RegKey;
    use winreg::enums::*;
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
            {
                let _ = run_cmd_elevated(&["/C", "taskkill", "/IM", &img, "/F"]);
            }
        }

        let key_path = format!(r"{}\{}", e.hive, e.key);
        let args_base = ["delete", &key_path, "/v", &e.name, "/f"];
        if run_reg(&args_base)
            || run_reg(&["delete", &key_path, "/v", &e.name, "/f", "/reg:64"])
            || run_reg(&["delete", &key_path, "/v", &e.name, "/f", "/reg:32"])
        {
            if !registry_value_exists(&e) {
                count += 1;
                continue;
            }
        }

        let _ = run_reg_elevated(&args_base);
        let _ = run_reg_elevated(&["delete", &key_path, "/v", &e.name, "/f", "/reg:64"]);
        let _ = run_reg_elevated(&["delete", &key_path, "/v", &e.name, "/f", "/reg:32"]);
        if !registry_value_exists(&e) {
            count += 1;
        }
    }
    Ok(count)
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub async fn remove_registry_run(_entries: Vec<StartupRegItem>) -> Result<usize, AppError> {
    Err(AppError::System("remove_registry_run is only implemented on Windows".into()))
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub async fn force_remove_registry_run(entries: Vec<StartupRegItem>) -> Result<usize, AppError> {
    use std::env;
    let mut script = String::from("$ErrorActionPreference='SilentlyContinue'\n");
    for e in &entries {
        let root = if e.hive.eq_ignore_ascii_case("HKLM") {
            "HKLM:"
        } else {
            "HKCU:"
        };
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
    std::fs::write(&tmp, &script).map_err(|e| AppError::Io(e))?;
    let _ok = run_powershell_elevated(&[
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        &tmp_str,
    ]);
    let _ = std::fs::remove_file(&tmp);
    let mut removed = 0usize;
    for e in &entries {
        if !registry_value_exists(e) {
            removed += 1;
        }
    }
    Ok(removed)
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub async fn force_remove_registry_run(_entries: Vec<StartupRegItem>) -> Result<usize, AppError> {
    Err(AppError::System("Only available on Windows".into()))
}

#[tauri::command]
pub async fn remove_startup_shortcuts(files: Vec<String>) -> Result<usize, AppError> {
    let mut count = 0usize;
    for f in files {
        match trash::delete(&f) {
            Ok(_) => {
                count += 1;
                continue;
            }
            Err(e) => {
                eprintln!("[startup] trash delete failed {}: {}", f, e);
            }
        }
        if std::fs::remove_file(&f).is_ok() {
            count += 1;
            continue;
        }
        #[cfg(target_os = "windows")]
        {
            let quoted = format!("\"{}\"", f.replace('"', "\\\""));
            if run_cmd_elevated(&["/C", "del", "/F", "/Q", &quoted]) {
                count += 1;
                continue;
            }
        }
        eprintln!("[startup] failed to remove {} (even elevated)", f);
    }
    Ok(count)
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub async fn purge_startup_approved(names: Vec<String>) -> Result<usize, AppError> {
    use winreg::RegKey;
    use winreg::enums::*;
    let hives = [
        ("HKCU", RegKey::predef(HKEY_CURRENT_USER)),
        ("HKLM", RegKey::predef(HKEY_LOCAL_MACHINE)),
    ];
    let subkeys = [
        r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run",
        r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run32",
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
pub async fn purge_startup_approved(_names: Vec<String>) -> Result<usize, AppError> {
    Ok(0)
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub async fn open_registry_key(hive: String, key: String) -> Result<(), AppError> {
    use winreg::RegKey;
    use winreg::enums::*;

    let hive_label = match hive.as_str() {
        "HKCU" | "HKEY_CURRENT_USER" => "HKEY_CURRENT_USER",
        "HKLM" | "HKEY_LOCAL_MACHINE" => "HKEY_LOCAL_MACHINE",
        other => other,
    };
    let full = format!(r"{}\{}", hive_label, key);

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (regedit_key, _) = hkcu
        .create_subkey(r"Software\Microsoft\Windows\CurrentVersion\Applets\Regedit")
        .map_err(|e| AppError::Io(e))?;
    regedit_key
        .set_value("LastKey", &full)
        .map_err(|e| AppError::Io(e))?;

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
                    .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &ps])
                    .spawn();
                return Ok(())
            }
            Err(AppError::System(format!("failed to launch regedit: {}", e)))
        }
    }
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub async fn open_registry_key(_hive: String, _key: String) -> Result<(), AppError> {
    Err(AppError::System("open_registry_key is only implemented on Windows".into()))
}

pub fn extract_image_from_command(cmd: &str) -> Option<String> {
    let s = cmd.trim();
    if s.is_empty() {
        return None;
    }
    let first = if s.starts_with('"') {
        s.split('"').nth(1).unwrap_or("")
    } else {
        s.split_whitespace().next().unwrap_or("")
    };
    if first.is_empty() {
        return None;
    }
    let token = if first.to_lowercase().contains(".exe") {
        first
    } else {
        if let Some(idx) = s.to_lowercase().find(".exe") {
            let start = s[..=idx]
                .rfind(|c| c == ' ' || c == '"')
                .map(|i| i + 1)
                .unwrap_or(0);
            &s[start..=idx + 3]
        } else {
            first
        }
    };
    let file = std::path::Path::new(token)
        .file_name()?
        .to_string_lossy()
        .to_string();
    if file.to_lowercase().ends_with(".exe") {
        Some(file)
    } else {
        None
    }
}

#[cfg(target_os = "windows")]
pub fn registry_value_exists(e: &StartupRegItem) -> bool {
    use winreg::RegKey;
    use winreg::enums::*;
    let hive = match e.hive.as_str() {
        "HKCU" => RegKey::predef(HKEY_CURRENT_USER),
        "HKLM" => RegKey::predef(HKEY_LOCAL_MACHINE),
        _ => return false,
    };
    let views = [0u32, KEY_WOW64_64KEY, KEY_WOW64_32KEY];
    for v in views {
        if let Ok(sub) = hive.open_subkey_with_flags(&e.key, KEY_READ | v) {
            if sub.get_value::<String, _>(&e.name).is_ok() {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_image_from_command() {
        assert_eq!(extract_image_from_command("\"C:\\Program Files\\App\\app.exe\" --start"), Some("app.exe".to_string()));
        assert_eq!(extract_image_from_command("C:\\Windows\\system32\\cmd.exe /c echo"), Some("cmd.exe".to_string()));
        assert_eq!(extract_image_from_command("powershell.exe -NoProfile"), Some("powershell.exe".to_string()));
        assert_eq!(extract_image_from_command("malicious.vbs"), None);
    }
}
