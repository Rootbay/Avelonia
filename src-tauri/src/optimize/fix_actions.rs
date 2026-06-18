use super::shell_helpers::{escape_single_quotes, run_powershell_commands};
use std::env;

#[cfg(target_os = "windows")]
fn run_autologin_impl() -> Result<String, String> {
    let raw_username = env::var("AVELONIA_AUTOLOGIN_USERNAME")
        .unwrap_or_else(|_| env::var("USERNAME").unwrap_or_default());
    if raw_username.is_empty() {
        return Err("Unable to determine username for autologin".into());
    }
    let password = env::var("AVELONIA_AUTOLOGIN_PASSWORD")
        .map_err(|_| "Set AVELONIA_AUTOLOGIN_PASSWORD to configure autologin".to_string())?;
    let (domain, user) = if let Some((dom, user)) = raw_username.split_once('\\') {
        (dom.to_string(), user.to_string())
    } else {
        (
            env::var("USERDOMAIN").unwrap_or_else(|_| ".".into()),
            raw_username.clone(),
        )
    };
    let mut commands = Vec::new();
    commands.push(r#"if (-not (Test-Path 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon')) { New-Item -Path 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon' | Out-Null }"#.to_string());
    commands.push(r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon' -Name 'AutoAdminLogon' -Value '1' -Type String -Force"#.to_string());
    commands.push(format!(
        r"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon' -Name 'DefaultUserName' -Value '{}' -Type String -Force",
        escape_single_quotes(&user)
    ));
    commands.push(format!(
        r"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon' -Name 'DefaultPassword' -Value '{}' -Type String -Force",
        escape_single_quotes(&password)
    ));
    commands.push(format!(
        r"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon' -Name 'DefaultDomainName' -Value '{}' -Type String -Force",
        escape_single_quotes(&domain)
    ));
    commands.push(r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon' -Name 'ForceAutoLogon' -Value 1 -Type DWord -Force"#.to_string());
    run_powershell_commands(&commands, "autologin")?;
    Ok("Autologin configured".into())
}

#[cfg(target_os = "windows")]
fn run_reset_windows_update_impl() -> Result<String, String> {
    let commands = vec![
        r#"@('wuauserv','bits','cryptsvc','TrustedInstaller') | ForEach-Object { Stop-Service -Name $_ -Force -ErrorAction SilentlyContinue }"#.to_string(),
        r#"Remove-Item -Path "$env:windir\SoftwareDistribution" -Recurse -Force -ErrorAction SilentlyContinue"#.to_string(),
        r#"Remove-Item -Path "$env:windir\System32\catroot2" -Recurse -Force -ErrorAction SilentlyContinue"#.to_string(),
        r#"@('wuauserv','bits','cryptsvc','TrustedInstaller') | ForEach-Object { Start-Service -Name $_ -ErrorAction SilentlyContinue }"#.to_string(),
    ];
    run_powershell_commands(&commands, "reset_updates")?;
    Ok("Windows Update reset task queued".into())
}

#[cfg(target_os = "windows")]
fn run_reset_network_impl() -> Result<String, String> {
    let commands = vec![
        r#"netsh winsock reset | Out-Null"#.to_string(),
        r#"netsh int ip reset | Out-Null"#.to_string(),
        r#"ipconfig /flushdns | Out-Null"#.to_string(),
    ];
    run_powershell_commands(&commands, "reset_network")?;
    Ok("Network reset queued".into())
}

#[cfg(target_os = "windows")]
fn run_system_corruption_scan_impl() -> Result<String, String> {
    let commands = vec![
        r#"Start-Process sfc -ArgumentList '/scannow' -Wait | Out-Null"#.to_string(),
        r#"Start-Process dism -ArgumentList '/Online','/Cleanup-Image','/RestoreHealth' -Wait | Out-Null"#.to_string(),
    ];
    run_powershell_commands(&commands, "system_scan")?;
    Ok("System corruption scan started".into())
}

#[cfg(target_os = "windows")]
fn run_winget_reinstall_impl() -> Result<String, String> {
    let command = r#"if (Get-Command winget -ErrorAction SilentlyContinue) { Start-Process winget -ArgumentList 'install','--id','Microsoft.Winget','--exact','--silent','--accept-source-agreements','--accept-package-agreements' -Wait | Out-Null } else { Write-Output 'winget unavailable' }"#;
    run_powershell_commands(&[command.to_string()], "winget_reinstall")?;
    Ok("WinGet reinstall requested".into())
}

#[cfg(target_os = "windows")]
fn run_remove_adobe_cc_impl() -> Result<String, String> {
    let command = r#"if (Get-Command winget -ErrorAction SilentlyContinue) { Start-Process winget -ArgumentList 'uninstall','--id','Adobe.AdobeCreativeCloud','--exact','--accept-source-agreements','--accept-package-agreements' -Wait | Out-Null } else { Write-Output 'winget unavailable' }"#;
    run_powershell_commands(&[command.to_string()], "remove_adobe_cc")?;
    Ok("Adobe Creative Cloud cleanup queued".into())
}

#[cfg(target_os = "windows")]
fn run_remove_activation_watermark_impl() -> Result<String, String> {
    let commands = vec![
        r#"if (-not (Test-Path 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\SoftwareProtectionPlatform\Activation')) { New-Item -Path 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\SoftwareProtectionPlatform\Activation' | Out-Null }"#.to_string(),
        r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\SoftwareProtectionPlatform\Activation' -Name 'Manual' -Value 1 -Type DWord -Force"#.to_string(),
        r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\SoftwareProtectionPlatform\Activation' -Name 'NotificationDisabled' -Value 1 -Type DWord -Force"#.to_string(),
        r#"Stop-Process -Name explorer -Force"#.to_string(),
    ];
    run_powershell_commands(&commands, "remove_watermark")?;
    Ok("Windows activation watermark removal script executed".into())
}

#[cfg(target_os = "windows")]
fn run_fix_action_impl(action_id: &str) -> Result<String, String> {
    match action_id {
        "autologin" => run_autologin_impl(),
        "reset_windows_update" => run_reset_windows_update_impl(),
        "reset_network" => run_reset_network_impl(),
        "system_corruption_scan" => run_system_corruption_scan_impl(),
        "winget_reinstall" => run_winget_reinstall_impl(),
        "remove_adobe_cc" => run_remove_adobe_cc_impl(),
        "remove_activation_watermark" => run_remove_activation_watermark_impl(),
        other => Err(format!("Unknown fix action '{}'", other)),
    }
}

#[cfg(not(target_os = "windows"))]
fn run_fix_action_impl(_action_id: &str) -> Result<String, String> {
    Err("Only available on Windows".into())
}

pub fn run_fix_action(action_id: String) -> Result<String, String> {
    run_fix_action_impl(&action_id)
}
