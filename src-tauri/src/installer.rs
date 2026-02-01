use dashmap::DashMap;
use std::sync::Arc;
use tauri::State;

#[derive(Default)]
pub struct InstallState(pub Arc<DashMap<u64, bool>>);

#[tauri::command]
#[cfg(target_os = "windows")]
pub async fn silent_install(
    id: u64,
    path: String,
    elevate: bool,
    custom_flags: Option<String>,
    state: State<'_, InstallState>,
) -> Result<(), String> {
    use std::path::PathBuf;
    use std::process::Command;

    state.0.insert(id, false);

    let pb = PathBuf::from(&path);
    if !pb.exists() {
        state.0.remove(&id);
        return Err(format!("installer not found: {}", path));
    }
    let ext = pb
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    let is_cancelled = || state.0.get(&id).map_or(false, |r| *r);

    if is_cancelled() {
        state.0.remove(&id);
        return Err("Installation cancelled".into());
    }

    if let Some(flags) = custom_flags {
        let args: Vec<&str> = flags.split_whitespace().collect();
        let ok = if elevate {
            crate::optimize::shell_helpers::run_elevated(&path, &args)
        } else {
            Command::new(&path)
                .args(&args)
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        };
        state.0.remove(&id);
        if ok {
            return Ok(());
        }
        return Err("Installation with custom flags failed".into());
    }

    if ext == "msi" {
        let args = ["/i", &path, "/qn", "/norestart", "ALLUSERS=1"];
        let ok = if elevate {
            crate::optimize::shell_helpers::run_elevated("msiexec", &args)
        } else {
            Command::new("msiexec")
                .args(args)
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        };
        state.0.remove(&id);
        if ok {
            return Ok(());
        }
        return Err("msiexec failed".into());
    }

    if ext == "exe" {
        let combos: Vec<Vec<&str>> = vec![
            vec!["/S"],
            vec!["/SILENT"],
            vec!["/silent"],
            vec!["/VERYSILENT", "/NORESTART", "/SP-", "/SUPPRESSMSGBOXES"],
            vec!["/quiet"],
        ];
        for combo in combos {
            if is_cancelled() {
                state.0.remove(&id);
                return Err("Installation cancelled".into());
            }
            let ok = if elevate {
                crate::optimize::shell_helpers::run_elevated(&path, &combo)
            } else {
                Command::new(&path)
                    .args(&combo)
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false)
            };
            if ok {
                state.0.remove(&id);
                return Ok(());
            }
        }
        state.0.remove(&id);
        return Err("no silent flag combination succeeded".into());
    }

    state.0.remove(&id);
    Err("unsupported installer type (expecting .msi or .exe)".into())
}

#[tauri::command]
pub async fn cancel_install(id: u64, state: State<'_, InstallState>) -> Result<(), String> {
    state.0.insert(id, true);
    Ok(())
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub async fn silent_install(
    _id: u64,
    _path: String,
    _elevate: bool,
    _custom_flags: Option<String>,
    _state: State<'_, InstallState>,
) -> Result<(), String> {
    Err("Silent install is only supported on Windows in this build".into())
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub async fn launch_installer(
    id: u64,
    path: String,
    elevate: bool,
    state: State<'_, InstallState>,
) -> Result<(), String> {
    use std::path::PathBuf;
    use std::process::Command;

    state.0.insert(id, false);

    let pb = PathBuf::from(&path);
    if !pb.exists() {
        state.0.remove(&id);
        return Err(format!("installer not found: {}", path));
    }
    let ext = pb
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    if ext == "msi" {
        let args = ["/i", &path];
        let ok = if elevate {
            crate::optimize::shell_helpers::run_elevated("msiexec", &args)
        } else {
            Command::new("msiexec")
                .args(args)
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        };
        state.0.remove(&id);
        if ok {
            return Ok(());
        }
        return Err("msiexec launch failed".into());
    }
    let ok = if elevate {
        crate::optimize::shell_helpers::run_elevated(&path, &[])
    } else {
        Command::new(&path)
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    };
    state.0.remove(&id);
    if ok {
        Ok(())
    } else {
        Err("failed to start installer".into())
    }
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub async fn launch_installer(
    _id: u64,
    _path: String,
    _elevate: bool,
    _state: State<'_, InstallState>,
) -> Result<(), String> {
    Err("Installer launch is only supported on Windows in this build".into())
}

#[derive(serde::Serialize, Clone, Default)]
pub struct UninstallEntry {
    pub display_name: String,
    pub display_version: String,
    pub install_location: String,
    pub uninstall_string: String,
    pub publisher: String,
    pub key_path: String,
    pub hive: String,
    pub view: String,
}

#[cfg(target_os = "windows")]
fn read_uninstall_entries() -> Vec<UninstallEntry> {
    use winreg::RegKey;
    use winreg::enums::*;
    fn collect_from(
        hive: &RegKey,
        subkey: &str,
        hive_name: &str,
        view: &str,
        out: &mut Vec<UninstallEntry>,
    ) {
        if let Ok(key) = hive.open_subkey_with_flags(subkey, KEY_READ) {
            for sk in key.enum_keys().filter_map(|x| x.ok()) {
                if let Ok(appkey) = key.open_subkey(&sk) {
                    let get = |name: &str| -> String {
                        appkey.get_value::<String, _>(name).unwrap_or_default()
                    };
                    let mut e = UninstallEntry::default();
                    e.display_name = get("DisplayName");
                    e.display_version = get("DisplayVersion");
                    e.install_location = get("InstallLocation");
                    e.uninstall_string = get("UninstallString");
                    e.publisher = get("Publisher");
                    e.key_path = format!("{}\\{}\\{}", hive_name, subkey, sk);
                    e.hive = hive_name.to_string();
                    e.view = view.to_string();
                    if !e.display_name.is_empty() || !e.uninstall_string.is_empty() {
                        out.push(e);
                    }
                }
            }
        }
    }
    let mut out = Vec::new();
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let hku = RegKey::predef(HKEY_CURRENT_USER);
    let base = "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall";
    let base_wow = "Software\\Wow6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall";
    collect_from(&hklm, base, "HKLM", "x64", &mut out);
    collect_from(&hklm, base_wow, "HKLM", "x86", &mut out);
    collect_from(&hku, base, "HKCU", "x64", &mut out);
    collect_from(&hku, base_wow, "HKCU", "x86", &mut out);
    out
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub fn list_uninstall_entries() -> Result<Vec<UninstallEntry>, String> {
    Ok(read_uninstall_entries())
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn list_uninstall_entries() -> Result<Vec<UninstallEntry>, String> {
    Ok(Vec::new())
}

#[derive(serde::Serialize, Clone, Default)]
pub struct VerifyResult {
    pub verified: bool,
    pub matched: Option<UninstallEntry>,
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub async fn verify_install(
    display_name_hint: Option<String>,
    timeout_ms: Option<u64>,
) -> Result<VerifyResult, String> {
    use tokio::time::{Duration, sleep};
    let before = read_uninstall_entries();
    let before_keys: std::collections::HashSet<String> =
        before.iter().map(|e| e.key_path.clone()).collect();
    let hint = display_name_hint.unwrap_or_default().to_lowercase();
    let timeout = timeout_ms.unwrap_or(30_000);
    let mut elapsed = 0u64;
    let step = 1_000u64;

    while elapsed <= timeout {
        let now = read_uninstall_entries();
        for e in &now {
            if before_keys.contains(&e.key_path) {
                continue;
            }
            if hint.is_empty() || is_match(&e.display_name, &hint) {
                return Ok(VerifyResult {
                    verified: true,
                    matched: Some(e.clone()),
                });
            }
        }
        sleep(Duration::from_millis(step)).await;
        elapsed += step;
    }
    let final_entries = read_uninstall_entries();
    if !hint.is_empty() {
        if let Some(e) = final_entries
            .into_iter()
            .find(|e| is_match(&e.display_name, &hint))
        {
            return Ok(VerifyResult {
                verified: true,
                matched: Some(e),
            });
        }
    }
    Ok(VerifyResult {
        verified: false,
        matched: None,
    })
}

fn is_match(display_name: &str, hint: &str) -> bool {
    let dn = display_name.to_lowercase();
    let h = hint.to_lowercase();

    if dn == h {
        return true;
    }

    let pattern = format!(r"\b{}\b", regex::escape(&h));
    if let Ok(re) = regex::Regex::new(&pattern) {
        return re.is_match(&dn);
    }

    dn.contains(&h)
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub fn is_installed(display_name_hint: String) -> Result<bool, String> {
    let hint = display_name_hint.to_lowercase();
    if hint.trim().is_empty() {
        return Ok(false);
    }
    let entries = read_uninstall_entries();
    Ok(entries
        .into_iter()
        .any(|e| is_match(&e.display_name, &hint)))
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn is_installed(_display_name_hint: String) -> Result<bool, String> {
    Ok(false)
}
