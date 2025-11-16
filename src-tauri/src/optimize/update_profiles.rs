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
            r#"New-Item -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate' -Force | Out-Null"#.to_string(),
            r#"New-Item -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate\AU' -Force | Out-Null"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate\AU' -Name 'DeferFeatureUpdatesPeriodInDays' -Value 730 -Type DWord -Force"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate\AU' -Name 'DeferQualityUpdatesPeriodInDays' -Value 4 -Type DWord -Force"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate\AU' -Name 'DeferFeatureUpdates' -Value 1 -Type DWord -Force"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate\AU' -Name 'NoAutoUpdate' -Value 0 -Type DWord -Force"#.to_string(),
        ],
        "disable" => vec![
            r#"New-Item -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate' -Force | Out-Null"#.to_string(),
            r#"New-Item -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate\AU' -Force | Out-Null"#.to_string(),
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
