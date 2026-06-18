#[cfg(target_os = "windows")]
use super::shell_helpers::run_powershell_commands;

#[cfg(target_os = "windows")]
pub(crate) fn apply_update_profile_impl(profile: &str) -> Result<String, String> {
    let commands = match profile {
        "default" => vec![
            r#"Remove-ItemProperty -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate' -Name 'NoAutoUpdate' -ErrorAction SilentlyContinue"#.to_string(),
            r#"Remove-ItemProperty -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate' -Name 'AUPowerMode' -ErrorAction SilentlyContinue"#.to_string(),
            r#"Remove-ItemProperty -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate\AU' -Name 'DeferFeatureUpdatesPeriodInDays' -ErrorAction SilentlyContinue"#.to_string(),
            r#"Remove-ItemProperty -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate\AU' -Name 'DeferQualityUpdatesPeriodInDays' -ErrorAction SilentlyContinue"#.to_string(),
            r#"Remove-ItemProperty -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate\AU' -Name 'DeferFeatureUpdates' -ErrorAction SilentlyContinue"#.to_string(),
            r#"Remove-ItemProperty -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate\AU' -Name 'AUOptions' -ErrorAction SilentlyContinue"#.to_string(),
        ],
        "security" => vec![
            r#"if (-not (Test-Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate')) { New-Item -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate' | Out-Null }"#.to_string(),
            r#"if (-not (Test-Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate\AU')) { New-Item -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate\AU' | Out-Null }"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate\AU' -Name 'DeferFeatureUpdatesPeriodInDays' -Value 730 -Type DWord -Force"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate\AU' -Name 'DeferQualityUpdatesPeriodInDays' -Value 4 -Type DWord -Force"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate\AU' -Name 'DeferFeatureUpdates' -Value 1 -Type DWord -Force"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate\AU' -Name 'NoAutoUpdate' -Value 0 -Type DWord -Force"#.to_string(),
        ],
        "disable" => vec![
            r#"if (-not (Test-Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate')) { New-Item -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate' | Out-Null }"#.to_string(),
            r#"if (-not (Test-Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate\AU')) { New-Item -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate\AU' | Out-Null }"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate' -Name 'NoAutoUpdate' -Value 1 -Type DWord -Force"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate\AU' -Name 'AUOptions' -Value 1 -Type DWord -Force"#.to_string(),
        ],
        _ => return Err(format!("Unknown update profile '{}'", profile)),
    };
    run_powershell_commands(&commands, "update_profile")?;
    Ok(format!("Update profile '{}' applied", profile))
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn apply_update_profile_impl(_profile: &str) -> Result<String, String> {
    Err("Only available on Windows".into())
}

pub fn apply_update_profile(profile: String) -> Result<String, String> {
    apply_update_profile_impl(&profile)
}

#[cfg(target_os = "windows")]
pub(crate) fn get_update_profile_impl() -> Result<String, String> {
    use winreg::RegKey;
    use winreg::enums::HKEY_LOCAL_MACHINE;
    let key = RegKey::predef(HKEY_LOCAL_MACHINE);

    let wu_key = match key.open_subkey("SOFTWARE\\Policies\\Microsoft\\Windows\\WindowsUpdate") {
        Ok(k) => k,
        Err(_) => return Ok("default".to_string()),
    };

    let au_key = match key.open_subkey("SOFTWARE\\Policies\\Microsoft\\Windows\\WindowsUpdate\\AU")
    {
        Ok(k) => Some(k),
        Err(_) => None,
    };

    // Check for "disable" profile
    let no_auto_update: u32 = wu_key.get_value("NoAutoUpdate").unwrap_or(0);
    let au_options: u32 = au_key
        .as_ref()
        .and_then(|k| k.get_value("AUOptions").ok())
        .unwrap_or(0);

    if no_auto_update == 1 && au_options == 1 {
        return Ok("disable".to_string());
    }

    // Check for "security" profile
    if let Some(ref k) = au_key {
        let defer_features: u32 = k.get_value("DeferFeatureUpdates").unwrap_or(0);
        let defer_feature_days: u32 = k.get_value("DeferFeatureUpdatesPeriodInDays").unwrap_or(0);
        let defer_quality_days: u32 = k.get_value("DeferQualityUpdatesPeriodInDays").unwrap_or(0);
        let no_auto_update_au: u32 = k.get_value("NoAutoUpdate").unwrap_or(0);

        if defer_features == 1
            && defer_feature_days == 730
            && defer_quality_days == 4
            && no_auto_update_au == 0
        {
            return Ok("security".to_string());
        }
    }

    Ok("default".to_string())
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn get_update_profile_impl() -> Result<String, String> {
    Ok("default".to_string())
}

pub fn get_update_profile() -> Result<String, String> {
    get_update_profile_impl()
}
