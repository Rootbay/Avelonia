use super::shell_helpers::run_powershell_commands;
use super::update_profiles::apply_update_profile_impl;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use winreg::RegKey;
use winreg::enums::*;
use winreg::HKEY;

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

pub enum TweakAction {
    RegistryValue {
        hive: HKEY,
        path: &'static str,
        name: &'static str,
        value: u32,
    },
    #[allow(dead_code)]
    Script(Vec<String>),
}

struct TweakDefinition {
    id: &'static str,
    action: TweakAction,
}

impl TweakDefinition {
    fn check_applied(&self) -> bool {
        match &self.action {
            TweakAction::RegistryValue { hive, path, name, value } => {
                let key = RegKey::predef(*hive);
                if let Ok(subkey) = key.open_subkey(path) {
                    if let Ok(actual_value) = subkey.get_value::<u32, _>(name) {
                        return actual_value == *value;
                    }
                }
                false
            }
            TweakAction::Script(_) => false,
        }
    }

    fn get_commands(&self) -> Vec<String> {
        match &self.action {
            TweakAction::RegistryValue { hive, path, name, value } => {
                let hive_str = if *hive == HKEY_LOCAL_MACHINE { "HKLM" } else { "HKCU" };
                vec![
                    format!("New-Item -Path '{}:\\{}' -Force | Out-Null", hive_str, path),
                    format!("Set-ItemProperty -Path '{}:\\{}' -Name '{}' -Value {} -Type DWord -Force", hive_str, path, name, value),
                ]
            }
            TweakAction::Script(cmds) => cmds.clone(),
        }
    }
}

#[cfg(target_os = "windows")]
fn get_tweak_definitions() -> Vec<TweakDefinition> {
    vec![
        TweakDefinition {
            id: "disable_telemetry",
            action: TweakAction::RegistryValue {
                hive: HKEY_LOCAL_MACHINE,
                path: "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Policies\\DataCollection",
                name: "AllowTelemetry",
                value: 0,
            },
        },
        TweakDefinition {
            id: "disable_gamedvr",
            action: TweakAction::RegistryValue {
                hive: HKEY_LOCAL_MACHINE,
                path: "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\GameDVR",
                name: "AppCaptureEnabled",
                value: 0,
            },
        },
        TweakDefinition {
            id: "disable_recall",
            action: TweakAction::RegistryValue {
                hive: HKEY_CURRENT_USER,
                path: "Software\\Microsoft\\Windows\\CurrentVersion\\Recall",
                name: "Disabled",
                value: 1,
            },
        },
        TweakDefinition {
            id: "show_file_extensions",
            action: TweakAction::RegistryValue {
                hive: HKEY_CURRENT_USER,
                path: "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced",
                name: "HideFileExt",
                value: 0,
            },
        },
    ]
}

#[cfg(target_os = "windows")]
fn script_for_tweak(id: &str) -> Vec<String> {
    if let Some(def) = get_tweak_definitions().into_iter().find(|d| d.id == id) {
        if def.check_applied() {
            return Vec::new();
        }
        return def.get_commands();
    }

    match id {
        "create_restore_point" => {
            vec![r#"Checkpoint-Computer -Description 'Avelonia Tweaks' -RestorePointType 'MODIFY_SETTINGS' -ErrorAction SilentlyContinue | Out-Null"#.to_string()]
        }
        "disable_consumer_features" => vec![r#"@('dmwappushservice','DiagTrack','RetailDemo','XblAuthManager','XblGameSave','WaaSMedicSvc') | ForEach-Object { Stop-Service -Name $_ -Force -ErrorAction SilentlyContinue; Set-Service -Name $_ -StartupType Disabled -ErrorAction SilentlyContinue }"#.to_string()],
        "set_hibernation_default" => vec![r#"powercfg -h on"#.to_string()],
        "services_manual" => vec![r#"@('WaaSMedicSvc','XblGameSave','XblAuthManager','DiagTrack') | ForEach-Object { Set-Service -Name $_ -StartupType Manual -ErrorAction SilentlyContinue }"#.to_string()],
        "debloat_brave" => vec![
            r#"if (Get-Command winget -ErrorAction SilentlyContinue) { Start-Process winget -ArgumentList 'uninstall','--id','Brave.Brave','--exact','--accept-source-agreements','--accept-package-agreements' -Wait | Out-Null }"#.to_string(),
            r#"if (Get-Command winget -ErrorAction SilentlyContinue) { Start-Process winget -ArgumentList 'uninstall','--id','BraveSoftware.Brave','--exact','--accept-source-agreements','--accept-package-agreements' -Wait | Out-Null }"#.to_string(),
        ],
        "debloat_edge" => vec![
            r#"@('MicrosoftEdgeUpdateTaskMachineCore','MicrosoftEdgeUpdateTaskMachineUA','MicrosoftEdgeUpdateTaskUserS-1-5-18') | ForEach-Object { schtasks /Change /TN $_ /Disable | Out-Null }"#.to_string(),
            r#"Get-Service -Name edgeupdate -ErrorAction SilentlyContinue | Stop-Service -Force -ErrorAction SilentlyContinue"#.to_string(),
            r#"Set-Service -Name edgeupdate -StartupType Disabled -ErrorAction SilentlyContinue"#.to_string(),
        ],
        "block_adobe_network" => vec![r#"$hosts = "$env:SystemRoot\System32\drivers\etc\hosts"
$entries = @('127.0.0.1 activate.adobe.com','127.0.0.1 ims-na1.adobelogin.com','127.0.0.1 practivate.adobe.com','127.0.0.1 oobe.adobe.com')
foreach ($entry in $entries) {
  if (-not (Select-String -Path $hosts -SimpleMatch $entry -Quiet)) {
    Add-Content -Path $hosts -Value $entry
  }
}"#.to_string()],
        "debloat_adobe" => vec![
            r#"@('AdobeARMservice','AdobeUpdateService','Adobe Genuine Monitor Service') | ForEach-Object { Stop-Service -Name $_ -Force -ErrorAction SilentlyContinue; Set-Service -Name $_ -StartupType Disabled -ErrorAction SilentlyContinue }"#.to_string(),
            r#"Get-ScheduledTask | Where-Object TaskName -Match 'Adobe' | ForEach-Object { Disable-ScheduledTask -TaskName $_.TaskName }"#.to_string(),
        ],
        "disable_ipv6" => vec![
            r#"New-Item -Path 'HKLM:\SYSTEM\CurrentControlSet\Services\Tcpip6\Parameters' -Force | Out-Null"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SYSTEM\CurrentControlSet\Services\Tcpip6\Parameters' -Name 'DisabledComponents' -Value 0xFFFFFFFF -Type DWord -Force"#.to_string(),
        ],
        "prefer_ipv4" => vec![
            r#"New-Item -Path 'HKLM:\SYSTEM\CurrentControlSet\Services\Tcpip6\Parameters' -Force | Out-Null"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SYSTEM\CurrentControlSet\Services\Tcpip6\Parameters' -Name 'DisabledComponents' -Value 32 -Type DWord -Force"#.to_string(),
        ],
        "remove_settings_home" => vec![
            r#"New-Item -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\StartPage' -Force | Out-Null"#.to_string(),
            r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\StartPage' -Name 'HideSettingsFromStartMenu' -Value 1 -Type DWord -Force"#.to_string(),
        ],
        "disable_background_apps" => vec![
            r#"New-Item -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\AppPrivacy' -Force | Out-Null"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\AppPrivacy' -Name 'LetAppsRunInBackground' -Value 2 -Type DWord -Force"#.to_string(),
        ],
        "detailed_bso_d" => vec![
            r#"New-Item -Path 'HKLM:\SYSTEM\CurrentControlSet\Control\CrashControl' -Force | Out-Null"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SYSTEM\CurrentControlSet\Control\CrashControl' -Name 'VerboseStatus' -Value 1 -Type DWord -Force"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SYSTEM\CurrentControlSet\Control\CrashControl' -Name 'CrashDumpEnabled' -Value 2 -Type DWord -Force"#.to_string(),
        ],
        "s3_sleep" => vec![r#"Set-ItemProperty -Path 'HKLM:\SYSTEM\CurrentControlSet\Control\Power' -Name 'CsEnabled' -Value 0 -Type DWord -Force"#.to_string()],
        "cross_device_resume" => vec![r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\Advanced' -Name 'EnableSharedExperiences' -Value 1 -Type DWord -Force"#.to_string()],
        "new_outlook" => vec![
            r#"New-Item -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Office\16.0\outlook\newoutlook' -Force | Out-Null"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Office\16.0\outlook\newoutlook' -Name 'ForceNewOutlook' -Value 1 -Type DWord -Force"#.to_string(),
        ],
        "show_hidden_files" => vec![
            r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\Advanced' -Name 'Hidden' -Value 1 -Type DWord -Force"#.to_string(),
            r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\Advanced' -Name 'ShowSuperHidden' -Value 1 -Type DWord -Force"#.to_string(),
        ],
        "show_file_extensions" => vec![r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\Advanced' -Name 'HideFileExt' -Value 0 -Type DWord -Force"#.to_string()],
        "taskbar_search_button" => vec![r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Search' -Name 'SearchboxTaskbarMode' -Value 1 -Type DWord -Force"#.to_string()],
        "taskbar_task_view" => vec![r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\Advanced' -Name 'ShowTaskViewButton' -Value 1 -Type DWord -Force"#.to_string()],
        "center_taskbar_items" => vec![r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\Advanced' -Name 'TaskbarAl' -Value 1 -Type DWord -Force"#.to_string()],
        "widgets_button" => vec![r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\Advanced' -Name 'TaskbarDa' -Value 1 -Type DWord -Force"#.to_string()],
        _ => Vec::new(),
    }
}

#[cfg(target_os = "windows")]
fn script_for_config(id: &str) -> Vec<String> {
    match id {
        "dotnet_framework" => vec![r#"Start-Process dism -ArgumentList '/Online','/Enable-Feature','/FeatureName:NetFx3','/All','/NoRestart' -Wait | Out-Null"#.to_string()],
        "hyperv_virtualization" => vec![r#"Start-Process dism -ArgumentList '/Online','/Enable-Feature','/FeatureName:Microsoft-Hyper-V-All','/All','/NoRestart' -Wait | Out-Null"#.to_string()],
        "legacy_media" => vec![
            r#"Start-Process dism -ArgumentList '/Online','/Enable-Feature','/FeatureName:LegacyComponents','/All','/NoRestart' -Wait | Out-Null"#.to_string(),
            r#"Start-Process dism -ArgumentList '/Online','/Enable-Feature','/FeatureName:DirectPlay','/All','/NoRestart' -Wait | Out-Null"#.to_string(),
            r#"Start-Process dism -ArgumentList '/Online','/Enable-Feature','/FeatureName:WindowsMediaPlayer','/All','/NoRestart' -Wait | Out-Null"#.to_string(),
        ],
        "nfs_network_file_system" => vec![r#"Start-Process dism -ArgumentList '/Online','/Enable-Feature','/FeatureName:ServicesForNFS-ClientOnly','/All','/NoRestart' -Wait | Out-Null"#.to_string()],
        "search_box_web_suggestions_enable" => vec![
            r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Search' -Name 'BingSearchEnabled' -Value 1 -Type DWord -Force"#.to_string(),
            r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Search' -Name 'AllowSearchToUseLocation' -Value 1 -Type DWord -Force"#.to_string(),
            r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Search' -Name 'CortanaConsent' -Value 1 -Type DWord -Force"#.to_string(),
        ],
        "search_box_web_suggestions_disable" => vec![
            r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Search' -Name 'BingSearchEnabled' -Value 0 -Type DWord -Force"#.to_string(),
            r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Search' -Name 'AllowSearchToUseLocation' -Value 0 -Type DWord -Force"#.to_string(),
            r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Search' -Name 'CortanaConsent' -Value 0 -Type DWord -Force"#.to_string(),
        ],
        "daily_registry_backup" => vec![r#"$taskName = 'AveloniaRegistryBackup'
$action = New-ScheduledTaskAction -Execute 'powershell.exe' -Argument '-NoProfile -WindowStyle Hidden -Command "& { $date = Get-Date -Format yyyyMMdd; reg export HKLM\SOFTWARE ""C:\Users\Public\AveloniaRegistryBackup-$date.reg"" /y }"'
$trigger = New-ScheduledTaskTrigger -Daily -At 00:30
Register-ScheduledTask -TaskName $taskName -Action $action -Trigger $trigger -Force"#.to_string()],
        "legacy_f8_boot_enable" => vec![r#"Start-Process bcdedit -ArgumentList '/set','{current}','bootmenupolicy','legacy' -Wait | Out-Null"#.to_string()],
        "legacy_f8_boot_disable" => vec![r#"Start-Process bcdedit -ArgumentList '/set','{current}','bootmenupolicy','standard' -Wait | Out-Null"#.to_string()],
        "wsl" => vec![
            r#"Start-Process dism -ArgumentList '/Online','/Enable-Feature','/FeatureName:Microsoft-Windows-Subsystem-Linux','/All','/NoRestart' -Wait | Out-Null"#.to_string(),
            r#"Start-Process dism -ArgumentList '/Online','/Enable-Feature','/FeatureName:VirtualMachinePlatform','/All','/NoRestart' -Wait | Out-Null"#.to_string(),
            r#"Start-Process wsl -ArgumentList '--set-default-version','2' -Wait | Out-Null"#.to_string(),
        ],
        "windows_sandbox" => vec![r#"Start-Process dism -ArgumentList '/Online','/Enable-Feature','/FeatureName:Containers-DisposableClientVM','/All','/NoRestart' -Wait | Out-Null"#.to_string()],
        _ => Vec::new(),
    }
}

pub fn apply_tweaks(payload: TweakApplyRequest) -> Result<TweakApplyResponse, String> {
    let mut commands = Vec::new();
    let mut seen = HashSet::new();
    for id in &payload.tweaks {
        if seen.insert(id.clone()) {
            commands.extend(script_for_tweak(id));
        }
    }
    seen.clear();
    for id in &payload.configs {
        if seen.insert(id.clone()) {
            commands.extend(script_for_config(id));
        }
    }
    if !commands.is_empty() {
        run_powershell_commands(&commands, "tweaks")?;
    }
    if let Some(ref profile) = payload.update_profile {
        apply_update_profile_impl(profile)?;
    }
    Ok(TweakApplyResponse {
        tweaks_applied: payload.tweaks.len(),
        configs_applied: payload.configs.len(),
        profile_applied: payload.update_profile,
    })
}
