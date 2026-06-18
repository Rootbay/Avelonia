pub mod fix_actions;
pub mod network;
pub mod services;
pub mod shell_helpers;
pub mod startup;
pub mod system;
pub mod tasks;
pub mod tweaks;
pub mod update_profiles;

pub use startup::*;
pub use network::*;
pub use tasks::*;
pub use system::*;
pub use services::*;

#[tauri::command]
pub async fn apply_tweaks(
    tweaks: Vec<String>,
    configs: Vec<String>,
    update_profile: Option<String>,
) -> Result<tweaks::TweakApplyResponse, String> {
    tweaks::apply_tweaks(tweaks::TweakApplyRequest {
        tweaks,
        configs,
        update_profile,
    })
}

#[tauri::command]
pub async fn run_fix_action(action_id: String) -> Result<String, String> {
    fix_actions::run_fix_action(action_id)
}

#[tauri::command]
pub async fn apply_update_profile(profile: String) -> Result<String, String> {
    update_profiles::apply_update_profile(profile)
}

#[tauri::command]
pub async fn get_update_profile() -> Result<String, String> {
    update_profiles::get_update_profile()
}

#[tauri::command]
pub async fn get_tweaks_status() -> Result<std::collections::HashMap<String, bool>, String> {
    tweaks::get_tweaks_status()
}

#[tauri::command]
pub fn is_elevated_command() -> bool {
    #[cfg(target_os = "windows")]
    {
        shell_helpers::is_elevated()
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

#[tauri::command]
pub async fn apply_tweak_state(id: String, enabled: bool) -> Result<(), String> {
    tweaks::apply_tweak_state(&id, enabled)
}

#[tauri::command]
pub async fn apply_tweaks_state_batch(changes: Vec<tweaks::TweakStateChange>) -> Result<(), String> {
    tweaks::apply_tweaks_state_batch(changes)
}

#[tauri::command]
pub async fn restart_explorer() -> Result<(), String> {
    tweaks::restart_explorer()
}

#[tauri::command]
pub async fn is_windows_activated() -> Result<bool, String> {
    #[cfg(target_os = "windows")]
    {
        let script = "Get-CimInstance -ClassName SoftwareLicensingProduct | Where-Object PartialProductKey | Select-Object Name, LicenseStatus | ConvertTo-Json";
        match shell_helpers::run_powershell_json(script) {
            Ok(values) => {
                for value in values {
                    let is_windows = value.get("Name")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_lowercase().contains("windows"))
                        .unwrap_or(false);
                    if is_windows {
                        let status = value.get("LicenseStatus")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        if status == 1 {
                            return Ok(true);
                        }
                    }
                }
                Ok(false)
            }
            Err(e) => {
                eprintln!("Failed to check activation status: {}", e);
                Ok(true)
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(true)
    }
}

#[tauri::command]
pub async fn is_watermark_removed() -> Result<bool, String> {
    #[cfg(target_os = "windows")]
    {
        use winreg::RegKey;
        use winreg::enums::HKEY_LOCAL_MACHINE;
        let key = RegKey::predef(HKEY_LOCAL_MACHINE);

        let mut svsvc_disabled = true;
        if let Ok(subkey) = key.open_subkey("SYSTEM\\CurrentControlSet\\Services\\svsvc") {
            let start: u32 = subkey.get_value("Start").unwrap_or(3);
            svsvc_disabled = start == 4;
        }

        if let Ok(subkey) = key.open_subkey("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\SoftwareProtectionPlatform\\Activation") {
            let manual: u32 = subkey.get_value("Manual").unwrap_or(0);
            let notif: u32 = subkey.get_value("NotificationDisabled").unwrap_or(0);
            return Ok(svsvc_disabled && manual == 1 && notif == 1);
        }
        Ok(false)
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(false)
    }
}