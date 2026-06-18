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
                    format!("if (-not (Test-Path '{}:\\{}')) {{ New-Item -Path '{}:\\{}' | Out-Null }}", hive_str, path, hive_str, path),
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
            r#"if (-not (Test-Path 'HKLM:\SYSTEM\CurrentControlSet\Services\Tcpip6\Parameters')) { New-Item -Path 'HKLM:\SYSTEM\CurrentControlSet\Services\Tcpip6\Parameters' | Out-Null }"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SYSTEM\CurrentControlSet\Services\Tcpip6\Parameters' -Name 'DisabledComponents' -Value 0xFFFFFFFF -Type DWord -Force"#.to_string(),
        ],
        "prefer_ipv4" => vec![
            r#"if (-not (Test-Path 'HKLM:\SYSTEM\CurrentControlSet\Services\Tcpip6\Parameters')) { New-Item -Path 'HKLM:\SYSTEM\CurrentControlSet\Services\Tcpip6\Parameters' | Out-Null }"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SYSTEM\CurrentControlSet\Services\Tcpip6\Parameters' -Name 'DisabledComponents' -Value 32 -Type DWord -Force"#.to_string(),
        ],
        "remove_settings_home" => vec![
            r#"if (-not (Test-Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\StartPage')) { New-Item -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\StartPage' | Out-Null }"#.to_string(),
            r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\StartPage' -Name 'HideSettingsFromStartMenu' -Value 1 -Type DWord -Force"#.to_string(),
        ],
        "disable_background_apps" => vec![
            r#"if (-not (Test-Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\AppPrivacy')) { New-Item -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\AppPrivacy' | Out-Null }"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\AppPrivacy' -Name 'LetAppsRunInBackground' -Value 2 -Type DWord -Force"#.to_string(),
        ],
        "detailed_bso_d" => vec![
            r#"if (-not (Test-Path 'HKLM:\SYSTEM\CurrentControlSet\Control\CrashControl')) { New-Item -Path 'HKLM:\SYSTEM\CurrentControlSet\Control\CrashControl' | Out-Null }"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SYSTEM\CurrentControlSet\Control\CrashControl' -Name 'VerboseStatus' -Value 1 -Type DWord -Force"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SYSTEM\CurrentControlSet\Control\CrashControl' -Name 'CrashDumpEnabled' -Value 2 -Type DWord -Force"#.to_string(),
        ],
        "s3_sleep" => vec![r#"Set-ItemProperty -Path 'HKLM:\SYSTEM\CurrentControlSet\Control\Power' -Name 'CsEnabled' -Value 0 -Type DWord -Force"#.to_string()],
        "cross_device_resume" => vec![r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\Advanced' -Name 'EnableSharedExperiences' -Value 1 -Type DWord -Force"#.to_string()],
        "new_outlook" => vec![
            r#"if (-not (Test-Path 'HKLM:\SOFTWARE\Policies\Microsoft\Office\16.0\outlook\newoutlook')) { New-Item -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Office\16.0\outlook\newoutlook' | Out-Null }"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Office\16.0\outlook\newoutlook' -Name 'ForceNewOutlook' -Value 1 -Type DWord -Force"#.to_string(),
        ],
        "show_hidden_files" => vec![
            r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\Advanced' -Name 'Hidden' -Value 1 -Type DWord -Force"#.to_string(),
            r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\Advanced' -Name 'ShowSuperHidden' -Value 1 -Type DWord -Force"#.to_string(),
        ],
        "show_file_extensions" => vec![r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\Advanced' -Name 'HideFileExt' -Value 0 -Type DWord -Force"#.to_string()],
        "disable_search" => vec![r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Search' -Name 'SearchboxTaskbarMode' -Value 0 -Type DWord -Force"#.to_string()],
        "disable_task_view" => vec![r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\Advanced' -Name 'ShowTaskViewButton' -Value 0 -Type DWord -Force"#.to_string()],
        "center_taskbar_items" => vec![r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\Advanced' -Name 'TaskbarAl' -Value 1 -Type DWord -Force"#.to_string()],
        "disable_widgets" => vec![r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\Advanced' -Name 'TaskbarDa' -Value 0 -Type DWord -Force"#.to_string()],
        "disable_chat" => vec![r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\Advanced' -Name 'TaskbarMn' -Value 0 -Type DWord -Force"#.to_string()],
        "disable_activity_history" => vec![
            r#"if (-not (Test-Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\System')) { New-Item -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\System' | Out-Null }"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\System' -Name 'PublishUserActivities' -Value 0 -Type DWord -Force"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\System' -Name 'UploadUserActivities' -Value 0 -Type DWord -Force"#.to_string(),
            r#"if (-not (Test-Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Privacy')) { New-Item -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Privacy' | Out-Null }"#.to_string(),
            r#"Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Privacy' -Name 'PublishUserActivities' -Value 0 -Type DWord -Force"#.to_string(),
        ],
        "disable_explorer_discovery" => vec![
            r#"if (-not (Test-Path 'HKCU:\Software\Classes\Local Settings\Software\Microsoft\Windows\Shell\Bags\AllFolders\Shell')) { New-Item -Path 'HKCU:\Software\Classes\Local Settings\Software\Microsoft\Windows\Shell\Bags\AllFolders\Shell' | Out-Null }"#.to_string(),
            r#"Set-ItemProperty -Path 'HKCU:\Software\Classes\Local Settings\Software\Microsoft\Windows\Shell\Bags\AllFolders\Shell' -Name 'FolderType' -Value 'NotSpecified' -Type String -Force"#.to_string(),
        ],
        "disable_hibernation" => vec![r#"powercfg -h off"#.to_string()],
        "disable_homegroup" => vec![
            r#"@('HomeGroupListener','HomeGroupProvider') | ForEach-Object { Stop-Service -Name $_ -Force -ErrorAction SilentlyContinue; Set-Service -Name $_ -StartupType Disabled -ErrorAction SilentlyContinue }"#.to_string()
        ],
        "disable_location_tracking" => vec![
            r#"if (-not (Test-Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\location')) { New-Item -Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\location' | Out-Null }"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\location' -Name 'Value' -Value 'Deny' -Type String -Force"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\location' -Name 'FallbackValue' -Value 'Deny' -Type String -Force"#.to_string(),
            r#"if (-not (Test-Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\AppPrivacy')) { New-Item -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\AppPrivacy' | Out-Null }"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\AppPrivacy' -Name 'LetAppsAccessLocation' -Value 2 -Type DWord -Force"#.to_string(),
            r#"Stop-Service -Name lfsvc -Force -ErrorAction SilentlyContinue; Set-Service -Name lfsvc -StartupType Disabled -ErrorAction SilentlyContinue"#.to_string(),
        ],
        "disable_storage_sense" => vec![
            r#"if (-not (Test-Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\StorageSense\Parameters\StoragePolicy')) { New-Item -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\StorageSense\Parameters\StoragePolicy' | Out-Null }"#.to_string(),
            r#"Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\StorageSense\Parameters\StoragePolicy' -Name '01' -Value 0 -Type DWord -Force"#.to_string(),
        ],
        "disable_wifi_sense" => vec![
            r#"if (-not (Test-Path 'HKLM:\SOFTWARE\Microsoft\WcmSvc\wifisense\Setting')) { New-Item -Path 'HKLM:\SOFTWARE\Microsoft\WcmSvc\wifisense\Setting' | Out-Null }"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Microsoft\WcmSvc\wifisense\Setting' -Name 'Enabled' -Value 0 -Type DWord -Force"#.to_string(),
        ],
        "enable_end_task" => vec![
            r#"if (-not (Test-Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced')) { New-Item -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced' | Out-Null }"#.to_string(),
            r#"Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced' -Name 'TaskbarEndTask' -Value 1 -Type DWord -Force"#.to_string(),
        ],
        "run_disk_cleanup" => vec![
            r#"Remove-Item -Path "$env:TEMP\*" -Recurse -Force -ErrorAction SilentlyContinue"#.to_string(),
            r#"Remove-Item -Path "$env:SystemRoot\Temp\*" -Recurse -Force -ErrorAction SilentlyContinue"#.to_string(),
        ],
        "terminal_powershell7_default" => vec![
            r#"$path = "$env:LOCALAPPDATA\Packages\Microsoft.WindowsTerminal_8wekyb3d8bbwe\LocalState\settings.json"
if (-not (Test-Path $path)) { $path = "$env:LOCALAPPDATA\Microsoft\Windows Terminal\settings.json" }
if (Test-Path $path) {
  $json = Get-Content $path -Raw | ConvertFrom-Json -ErrorAction SilentlyContinue
  if ($json) {
    $pwsh7 = $json.profiles.list | Where-Object { $_.commandline -like '*pwsh.exe*' -or $_.name -like '*PowerShell 7*' } | Select-Object -First 1
    if ($pwsh7 -and $pwsh7.guid) {
      $json.defaultProfile = $pwsh7.guid
      $json | ConvertTo-Json -Depth 100 | Set-Content $path -Force
    }
  }
}"#.to_string()
        ],
        "disable_powershell7_telemetry" => vec![
            r#"Set-ItemProperty -Path 'HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager\Environment' -Name 'POWERSHELL_TELEMETRY_OPTOUT' -Value '1' -Type String -Force"#.to_string(),
            r#"[Environment]::SetEnvironmentVariable('POWERSHELL_TELEMETRY_OPTOUT', '1', 'Machine')"#.to_string(),
        ],
        "disable_teredo" => vec![r#"netsh interface teredo set state disabled"#.to_string()],
        "disable_fullscreen_optimizations" => vec![
            r#"if (-not (Test-Path 'HKCU:\System\GameConfigStore')) { New-Item -Path 'HKCU:\System\GameConfigStore' | Out-Null }"#.to_string(),
            r#"Set-ItemProperty -Path 'HKCU:\System\GameConfigStore' -Name 'GameDVR_FSEBehaviorMode' -Value 2 -Type DWord -Force"#.to_string(),
        ],
        "dark_theme" => vec![
            r#"if (-not (Test-Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Themes\Personalize')) { New-Item -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Themes\Personalize' | Out-Null }"#.to_string(),
            r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Themes\Personalize' -Name 'AppsUseLightTheme' -Value 0 -Type DWord -Force"#.to_string(),
            r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Themes\Personalize' -Name 'SystemUsesLightTheme' -Value 0 -Type DWord -Force"#.to_string(),
        ],
        "bing_search_start" => vec![
            r#"if (-not (Test-Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Search')) { New-Item -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Search' | Out-Null }"#.to_string(),
            r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Search' -Name 'BingSearchEnabled' -Value 1 -Type DWord -Force"#.to_string(),
            r#"if (-not (Test-Path 'HKCU:\SOFTWARE\Policies\Microsoft\Windows\Windows Search')) { New-Item -Path 'HKCU:\SOFTWARE\Policies\Microsoft\Windows\Windows Search' | Out-Null }"#.to_string(),
            r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Policies\Microsoft\Windows\Windows Search' -Name 'DisableWebSearch' -Value 0 -Type DWord -Force"#.to_string(),
        ],
        "numlock_on_startup" => vec![
            r#"if (-not (Test-Path 'HKU:\.DEFAULT\Control Panel\Keyboard')) { New-Item -Path 'HKU:\.DEFAULT\Control Panel\Keyboard' | Out-Null }"#.to_string(),
            r#"Set-ItemProperty -Path 'HKU:\.DEFAULT\Control Panel\Keyboard' -Name 'InitialKeyboardIndicators' -Value '2' -Type String -Force"#.to_string(),
            r#"if (-not (Test-Path 'HKCU:\Control Panel\Keyboard')) { New-Item -Path 'HKCU:\Control Panel\Keyboard' | Out-Null }"#.to_string(),
            r#"Set-ItemProperty -Path 'HKCU:\Control Panel\Keyboard' -Name 'InitialKeyboardIndicators' -Value '2' -Type String -Force"#.to_string(),
        ],
        "verbose_logon" => vec![
            r#"if (-not (Test-Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System')) { New-Item -Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System' | Out-Null }"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System' -Name 'VerboseStatus' -Value 1 -Type DWord -Force"#.to_string(),
        ],
        "start_recommendations" => vec![
            r#"if (-not (Test-Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced')) { New-Item -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced' | Out-Null }"#.to_string(),
            r#"Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced' -Name 'Start_IrisRecommendations' -Value 1 -Type DWord -Force"#.to_string(),
            r#"Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced' -Name 'Start_TrackProgs' -Value 1 -Type DWord -Force"#.to_string(),
        ],
        "snap_window" => vec![
            r#"if (-not (Test-Path 'HKCU:\Control Panel\Desktop')) { New-Item -Path 'HKCU:\Control Panel\Desktop' | Out-Null }"#.to_string(),
            r#"Set-ItemProperty -Path 'HKCU:\Control Panel\Desktop' -Name 'WindowArrangementActive' -Value '1' -Type String -Force"#.to_string(),
        ],
        "snap_assist_flyout" => vec![
            r#"if (-not (Test-Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced')) { New-Item -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced' | Out-Null }"#.to_string(),
            r#"Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced' -Name 'SnapAssist' -Value 1 -Type DWord -Force"#.to_string(),
        ],
        "snap_assist_suggestion" => vec![
            r#"if (-not (Test-Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced')) { New-Item -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced' | Out-Null }"#.to_string(),
            r#"Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced' -Name 'JointResize' -Value 1 -Type DWord -Force"#.to_string(),
        ],
        "mouse_acceleration" => vec![
            r#"if (-not (Test-Path 'HKCU:\Control Panel\Mouse')) { New-Item -Path 'HKCU:\Control Panel\Mouse' | Out-Null }"#.to_string(),
            r#"Set-ItemProperty -Path 'HKCU:\Control Panel\Mouse' -Name 'MouseSpeed' -Value '0' -Type String -Force"#.to_string(),
            r#"Set-ItemProperty -Path 'HKCU:\Control Panel\Mouse' -Name 'MouseThreshold1' -Value '0' -Type String -Force"#.to_string(),
            r#"Set-ItemProperty -Path 'HKCU:\Control Panel\Mouse' -Name 'MouseThreshold2' -Value '0' -Type String -Force"#.to_string(),
        ],
        "sticky_keys" => vec![
            r#"if (-not (Test-Path 'HKCU:\Control Panel\Accessibility\StickyKeys')) { New-Item -Path 'HKCU:\Control Panel\Accessibility\StickyKeys' | Out-Null }"#.to_string(),
            r#"Set-ItemProperty -Path 'HKCU:\Control Panel\Accessibility\StickyKeys' -Name 'Flags' -Value '510' -Type String -Force"#.to_string(),
        ],
        "multiplane_overlay" => vec![
            r#"if (-not (Test-Path 'HKLM:\SOFTWARE\Microsoft\Windows\Dwm')) { New-Item -Path 'HKLM:\SOFTWARE\Microsoft\Windows\Dwm' | Out-Null }"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Microsoft\Windows\Dwm' -Name 'OverlayTestMode' -Value 5 -Type DWord -Force"#.to_string(),
        ],
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

#[cfg(target_os = "windows")]
pub fn check_tweak_applied_by_id(id: &str) -> bool {
    if let Some(def) = get_tweak_definitions().into_iter().find(|d| d.id == id) {
        return def.check_applied();
    }

    match id {
        "disable_consumer_features" => {
            let key = RegKey::predef(HKEY_LOCAL_MACHINE);
            if let Ok(subkey) = key.open_subkey("SYSTEM\\CurrentControlSet\\Services\\DiagTrack") {
                if let Ok(val) = subkey.get_value::<u32, _>("Start") {
                    return val == 4;
                }
            }
            false
        }
        "disable_activity_history" => {
            let key = RegKey::predef(HKEY_LOCAL_MACHINE);
            if let Ok(subkey) = key.open_subkey("SOFTWARE\\Policies\\Microsoft\\Windows\\System") {
                if let Ok(val) = subkey.get_value::<u32, _>("PublishUserActivities") {
                    return val == 0;
                }
            }
            false
        }
        "disable_explorer_discovery" => {
            let key = RegKey::predef(HKEY_CURRENT_USER);
            if let Ok(subkey) = key.open_subkey("Software\\Classes\\Local Settings\\Software\\Microsoft\\Windows\\Shell\\Bags\\AllFolders\\Shell") {
                if let Ok(val) = subkey.get_value::<String, _>("FolderType") {
                    return val == "NotSpecified";
                }
            }
            false
        }
        "disable_hibernation" => {
            let key = RegKey::predef(HKEY_LOCAL_MACHINE);
            if let Ok(subkey) = key.open_subkey("SYSTEM\\CurrentControlSet\\Control\\Power") {
                if let Ok(val) = subkey.get_value::<u32, _>("HibernateEnabled") {
                    return val == 0;
                }
            }
            false
        }
        "set_hibernation_default" => {
            let key = RegKey::predef(HKEY_LOCAL_MACHINE);
            if let Ok(subkey) = key.open_subkey("SYSTEM\\CurrentControlSet\\Control\\Power") {
                if let Ok(val) = subkey.get_value::<u32, _>("HibernateEnabled") {
                    return val == 1;
                }
            }
            false
        }
        "disable_location_tracking" => {
            let key = RegKey::predef(HKEY_LOCAL_MACHINE);
            if let Ok(subkey) = key.open_subkey("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\CapabilityAccessManager\\ConsentStore\\location") {
                if let Ok(val) = subkey.get_value::<String, _>("Value") {
                    return val == "Deny";
                }
            }
            false
        }
        "disable_storage_sense" => {
            let key = RegKey::predef(HKEY_CURRENT_USER);
            if let Ok(subkey) = key.open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\StorageSense\\Parameters\\StoragePolicy") {
                if let Ok(val) = subkey.get_value::<u32, _>("01") {
                    return val == 0;
                }
            }
            false
        }
        "disable_wifi_sense" => {
            let key = RegKey::predef(HKEY_LOCAL_MACHINE);
            if let Ok(subkey) = key.open_subkey("SOFTWARE\\Microsoft\\WcmSvc\\wifisense\\Setting") {
                if let Ok(val) = subkey.get_value::<u32, _>("Enabled") {
                    return val == 0;
                }
            }
            false
        }
        "enable_end_task" => {
            let key = RegKey::predef(HKEY_CURRENT_USER);
            if let Ok(subkey) = key.open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced") {
                if let Ok(val) = subkey.get_value::<u32, _>("TaskbarEndTask") {
                    return val == 1;
                }
            }
            false
        }
        "disable_powershell7_telemetry" => {
            let key = RegKey::predef(HKEY_LOCAL_MACHINE);
            if let Ok(subkey) = key.open_subkey("SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment") {
                if let Ok(val) = subkey.get_value::<String, _>("POWERSHELL_TELEMETRY_OPTOUT") {
                    return val == "1";
                }
            }
            false
        }
        "disable_fullscreen_optimizations" => {
            let key = RegKey::predef(HKEY_CURRENT_USER);
            if let Ok(subkey) = key.open_subkey("System\\GameConfigStore") {
                if let Ok(val) = subkey.get_value::<u32, _>("GameDVR_FSEBehaviorMode") {
                    return val == 2;
                }
            }
            false
        }
        "dark_theme" => {
            let key = RegKey::predef(HKEY_CURRENT_USER);
            if let Ok(subkey) = key.open_subkey("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize") {
                if let Ok(val) = subkey.get_value::<u32, _>("AppsUseLightTheme") {
                    return val == 0;
                }
            }
            false
        }
        "bing_search_start" => {
            let key = RegKey::predef(HKEY_CURRENT_USER);
            if let Ok(subkey) = key.open_subkey("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Search") {
                if let Ok(val) = subkey.get_value::<u32, _>("BingSearchEnabled") {
                    return val == 1;
                }
            }
            false
        }
        "numlock_on_startup" => {
            let key = RegKey::predef(HKEY_CURRENT_USER);
            if let Ok(subkey) = key.open_subkey("Control Panel\\Keyboard") {
                if let Ok(val) = subkey.get_value::<String, _>("InitialKeyboardIndicators") {
                    return val.contains('2');
                }
            }
            false
        }
        "verbose_logon" => {
            let key = RegKey::predef(HKEY_LOCAL_MACHINE);
            if let Ok(subkey) = key.open_subkey("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Policies\\System") {
                if let Ok(val) = subkey.get_value::<u32, _>("VerboseStatus") {
                    return val == 1;
                }
            }
            false
        }
        "start_recommendations" => {
            let key = RegKey::predef(HKEY_CURRENT_USER);
            if let Ok(subkey) = key.open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced") {
                if let Ok(val) = subkey.get_value::<u32, _>("Start_IrisRecommendations") {
                    return val == 1;
                }
            }
            false
        }
        "snap_window" => {
            let key = RegKey::predef(HKEY_CURRENT_USER);
            if let Ok(subkey) = key.open_subkey("Control Panel\\Desktop") {
                if let Ok(val) = subkey.get_value::<String, _>("WindowArrangementActive") {
                    return val == "1";
                }
            }
            false
        }
        "snap_assist_flyout" => {
            let key = RegKey::predef(HKEY_CURRENT_USER);
            if let Ok(subkey) = key.open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced") {
                if let Ok(val) = subkey.get_value::<u32, _>("SnapAssist") {
                    return val == 1;
                }
            }
            false
        }
        "snap_assist_suggestion" => {
            let key = RegKey::predef(HKEY_CURRENT_USER);
            if let Ok(subkey) = key.open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced") {
                if let Ok(val) = subkey.get_value::<u32, _>("JointResize") {
                    return val == 1;
                }
            }
            false
        }
        "mouse_acceleration" => {
            let key = RegKey::predef(HKEY_CURRENT_USER);
            if let Ok(subkey) = key.open_subkey("Control Panel\\Mouse") {
                if let Ok(val) = subkey.get_value::<String, _>("MouseSpeed") {
                    return val == "0";
                }
            }
            false
        }
        "sticky_keys" => {
            let key = RegKey::predef(HKEY_CURRENT_USER);
            if let Ok(subkey) = key.open_subkey("Control Panel\\Accessibility\\StickyKeys") {
                if let Ok(val) = subkey.get_value::<String, _>("Flags") {
                    return val == "510";
                }
            }
            false
        }
        "multiplane_overlay" => {
            let key = RegKey::predef(HKEY_LOCAL_MACHINE);
            if let Ok(subkey) = key.open_subkey("SOFTWARE\\Microsoft\\Windows\\Dwm") {
                if let Ok(val) = subkey.get_value::<u32, _>("OverlayTestMode") {
                    return val == 5;
                }
            }
            false
        }
        "remove_settings_home" => {
            let key = RegKey::predef(HKEY_CURRENT_USER);
            if let Ok(subkey) = key.open_subkey("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\StartPage") {
                if let Ok(val) = subkey.get_value::<u32, _>("HideSettingsFromStartMenu") {
                    return val == 1;
                }
            }
            false
        }
        "disable_background_apps" => {
            let key = RegKey::predef(HKEY_LOCAL_MACHINE);
            if let Ok(subkey) = key.open_subkey("SOFTWARE\\Policies\\Microsoft\\Windows\\AppPrivacy") {
                if let Ok(val) = subkey.get_value::<u32, _>("LetAppsRunInBackground") {
                    return val == 2;
                }
            }
            false
        }
        "detailed_bso_d" => {
            let key = RegKey::predef(HKEY_LOCAL_MACHINE);
            if let Ok(subkey) = key.open_subkey("SYSTEM\\CurrentControlSet\\Control\\CrashControl") {
                if let Ok(val) = subkey.get_value::<u32, _>("VerboseStatus") {
                    return val == 1;
                }
            }
            false
        }
        "s3_sleep" => {
            let key = RegKey::predef(HKEY_LOCAL_MACHINE);
            if let Ok(subkey) = key.open_subkey("SYSTEM\\CurrentControlSet\\Control\\Power") {
                if let Ok(val) = subkey.get_value::<u32, _>("CsEnabled") {
                    return val == 0;
                }
            }
            false
        }
        "cross_device_resume" => {
            let key = RegKey::predef(HKEY_LOCAL_MACHINE);
            if let Ok(subkey) = key.open_subkey("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced") {
                if let Ok(val) = subkey.get_value::<u32, _>("EnableSharedExperiences") {
                    return val == 1;
                }
            }
            false
        }
        "new_outlook" => {
            let key = RegKey::predef(HKEY_LOCAL_MACHINE);
            if let Ok(subkey) = key.open_subkey("SOFTWARE\\Policies\\Microsoft\\Office\\16.0\\outlook\\newoutlook") {
                if let Ok(val) = subkey.get_value::<u32, _>("ForceNewOutlook") {
                    return val == 1;
                }
            }
            false
        }
        "show_hidden_files" => {
            let key = RegKey::predef(HKEY_CURRENT_USER);
            if let Ok(subkey) = key.open_subkey("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced") {
                if let Ok(val) = subkey.get_value::<u32, _>("Hidden") {
                    return val == 1;
                }
            }
            false
        }
        "disable_search" => {
            let key = RegKey::predef(HKEY_CURRENT_USER);
            if let Ok(subkey) = key.open_subkey("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Search") {
                if let Ok(val) = subkey.get_value::<u32, _>("SearchboxTaskbarMode") {
                    return val == 0;
                }
            }
            false
        }
        "disable_task_view" => {
            let key = RegKey::predef(HKEY_CURRENT_USER);
            if let Ok(subkey) = key.open_subkey("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced") {
                if let Ok(val) = subkey.get_value::<u32, _>("ShowTaskViewButton") {
                    return val == 0;
                }
            }
            false
        }
        "center_taskbar_items" => {
            let key = RegKey::predef(HKEY_CURRENT_USER);
            if let Ok(subkey) = key.open_subkey("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced") {
                if let Ok(val) = subkey.get_value::<u32, _>("TaskbarAl") {
                    return val == 1;
                }
            }
            false
        }
        "disable_widgets" => {
            let key = RegKey::predef(HKEY_CURRENT_USER);
            if let Ok(subkey) = key.open_subkey("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced") {
                if let Ok(val) = subkey.get_value::<u32, _>("TaskbarDa") {
                    return val == 0;
                }
            }
            false
        }
        "disable_chat" => {
            let key = RegKey::predef(HKEY_CURRENT_USER);
            if let Ok(subkey) = key.open_subkey("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced") {
                if let Ok(val) = subkey.get_value::<u32, _>("TaskbarMn") {
                    return val == 0;
                }
            }
            false
        }
        "disable_ipv6" => {
            let key = RegKey::predef(HKEY_LOCAL_MACHINE);
            if let Ok(subkey) = key.open_subkey("SYSTEM\\CurrentControlSet\\Services\\Tcpip6\\Parameters") {
                if let Ok(val) = subkey.get_value::<u32, _>("DisabledComponents") {
                    return val == 0xFFFFFFFF;
                }
            }
            false
        }
        "prefer_ipv4" => {
            let key = RegKey::predef(HKEY_LOCAL_MACHINE);
            if let Ok(subkey) = key.open_subkey("SYSTEM\\CurrentControlSet\\Services\\Tcpip6\\Parameters") {
                if let Ok(val) = subkey.get_value::<u32, _>("DisabledComponents") {
                    return val == 32;
                }
            }
            false
        }
        "disable_homegroup" => {
            let key = RegKey::predef(HKEY_LOCAL_MACHINE);
            if let Ok(subkey) = key.open_subkey("SYSTEM\\CurrentControlSet\\Services\\HomeGroupListener") {
                if let Ok(val) = subkey.get_value::<u32, _>("Start") {
                    return val == 4;
                }
            }
            false
        }
        "services_manual" => {
            let key = RegKey::predef(HKEY_LOCAL_MACHINE);
            if let Ok(subkey) = key.open_subkey("SYSTEM\\CurrentControlSet\\Services\\WaSMedicSvc") {
                if let Ok(val) = subkey.get_value::<u32, _>("Start") {
                    return val == 3;
                }
            }
            false
        }
        "debloat_edge" => {
            let key = RegKey::predef(HKEY_LOCAL_MACHINE);
            if let Ok(subkey) = key.open_subkey("SYSTEM\\CurrentControlSet\\Services\\edgeupdate") {
                if let Ok(val) = subkey.get_value::<u32, _>("Start") {
                    return val == 4;
                }
            }
            false
        }
        "debloat_adobe" => {
            let key = RegKey::predef(HKEY_LOCAL_MACHINE);
            if let Ok(subkey) = key.open_subkey("SYSTEM\\CurrentControlSet\\Services\\AdobeARMservice") {
                if let Ok(val) = subkey.get_value::<u32, _>("Start") {
                    return val == 4;
                }
            }
            false
        }
        "daily_registry_backup" => {
            let key = RegKey::predef(HKEY_LOCAL_MACHINE);
            let path = "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Schedule\\TaskCache\\Tree\\AveloniaRegistryBackup";
            key.open_subkey(path).is_ok()
        }
        _ => false,
    }
}

#[cfg(not(target_os = "windows"))]
pub fn check_tweak_applied_by_id(_id: &str) -> bool {
    false
}

pub fn get_tweaks_status() -> Result<std::collections::HashMap<String, bool>, String> {
    #[cfg(target_os = "windows")]
    {
        let mut status = std::collections::HashMap::new();
        let all_tweaks = vec![
            "disable_telemetry", "disable_gamedvr", "disable_recall", "show_file_extensions",
            "show_hidden_files", "disable_activity_history", "disable_explorer_discovery",
            "disable_hibernation", "set_hibernation_default", "disable_location_tracking",
            "disable_storage_sense", "disable_wifi_sense", "enable_end_task",
            "disable_powershell7_telemetry", "disable_fullscreen_optimizations", "dark_theme",
            "bing_search_start", "numlock_on_startup", "verbose_logon", "start_recommendations",
            "snap_window", "snap_assist_flyout", "snap_assist_suggestion", "mouse_acceleration",
            "sticky_keys", "multiplane_overlay", "remove_settings_home", "disable_background_apps",
            "detailed_bso_d", "s3_sleep", "cross_device_resume", "new_outlook",
            "disable_search", "disable_task_view", "center_taskbar_items", "disable_widgets", "disable_chat", "disable_consumer_features",
            "disable_ipv6", "prefer_ipv4", "disable_homegroup", "services_manual", "debloat_edge",
            "debloat_adobe", "daily_registry_backup"
        ];
        for id in all_tweaks {
            status.insert(id.to_string(), check_tweak_applied_by_id(id));
        }
        Ok(status)
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(std::collections::HashMap::new())
    }
}

#[cfg(target_os = "windows")]
fn script_for_tweak_revert(id: &str) -> Vec<String> {
    match id {
        "disable_telemetry" => vec![
            r#"if (-not (Test-Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\DataCollection')) { New-Item -Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\DataCollection' | Out-Null }"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\DataCollection' -Name 'AllowTelemetry' -Value 1 -Type DWord -Force"#.to_string(),
        ],
        "disable_gamedvr" => vec![
            r#"if (-not (Test-Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\GameDVR')) { New-Item -Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\GameDVR' | Out-Null }"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\GameDVR' -Name 'AppCaptureEnabled' -Value 1 -Type DWord -Force"#.to_string(),
        ],
        "disable_recall" => vec![
            r#"if (-not (Test-Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Recall')) { New-Item -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Recall' | Out-Null }"#.to_string(),
            r#"Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Recall' -Name 'Disabled' -Value 0 -Type DWord -Force"#.to_string(),
        ],
        "show_file_extensions" => vec![
            r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\Advanced' -Name 'HideFileExt' -Value 1 -Type DWord -Force"#.to_string(),
        ],
        "show_hidden_files" => vec![
            r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\Advanced' -Name 'Hidden' -Value 2 -Type DWord -Force"#.to_string(),
            r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\Advanced' -Name 'ShowSuperHidden' -Value 0 -Type DWord -Force"#.to_string(),
        ],
        "disable_activity_history" => vec![
            r#"if (-not (Test-Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\System')) { New-Item -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\System' | Out-Null }"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\System' -Name 'PublishUserActivities' -Value 1 -Type DWord -Force"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\System' -Name 'UploadUserActivities' -Value 1 -Type DWord -Force"#.to_string(),
            r#"if (-not (Test-Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Privacy')) { New-Item -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Privacy' | Out-Null }"#.to_string(),
            r#"Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Privacy' -Name 'PublishUserActivities' -Value 1 -Type DWord -Force"#.to_string(),
        ],
        "disable_explorer_discovery" => vec![
            r#"Remove-ItemProperty -Path 'HKCU:\Software\Classes\Local Settings\Software\Microsoft\Windows\Shell\Bags\AllFolders\Shell' -Name 'FolderType' -ErrorAction SilentlyContinue"#.to_string(),
        ],
        "disable_hibernation" => vec![r#"powercfg -h on"#.to_string()],
        "set_hibernation_default" => vec![r#"powercfg -h off"#.to_string()],
        "disable_location_tracking" => vec![
            r#"if (-not (Test-Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\location')) { New-Item -Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\location' | Out-Null }"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\location' -Name 'Value' -Value 'Allow' -Type String -Force"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\location' -Name 'FallbackValue' -Value 'Allow' -Type String -Force"#.to_string(),
            r#"Set-Service -Name lfsvc -StartupType Manual -ErrorAction SilentlyContinue"#.to_string(),
        ],
        "disable_storage_sense" => vec![
            r#"if (-not (Test-Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\StorageSense\Parameters\StoragePolicy')) { New-Item -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\StorageSense\Parameters\StoragePolicy' | Out-Null }"#.to_string(),
            r#"Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\StorageSense\Parameters\StoragePolicy' -Name '01' -Value 1 -Type DWord -Force"#.to_string(),
        ],
        "disable_wifi_sense" => vec![
            r#"if (-not (Test-Path 'HKLM:\SOFTWARE\Microsoft\WcmSvc\wifisense\Setting')) { New-Item -Path 'HKLM:\SOFTWARE\Microsoft\WcmSvc\wifisense\Setting' | Out-Null }"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Microsoft\WcmSvc\wifisense\Setting' -Name 'Enabled' -Value 1 -Type DWord -Force"#.to_string(),
        ],
        "enable_end_task" => vec![
            r#"Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced' -Name 'TaskbarEndTask' -Value 0 -Type DWord -Force"#.to_string(),
        ],
        "disable_powershell7_telemetry" => vec![
            r#"Remove-ItemProperty -Path 'HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager\Environment' -Name 'POWERSHELL_TELEMETRY_OPTOUT' -ErrorAction SilentlyContinue"#.to_string(),
            r#"[Environment]::SetEnvironmentVariable('POWERSHELL_TELEMETRY_OPTOUT', $null, 'Machine')"#.to_string(),
        ],
        "disable_fullscreen_optimizations" => vec![
            r#"Set-ItemProperty -Path 'HKCU:\System\GameConfigStore' -Name 'GameDVR_FSEBehaviorMode' -Value 0 -Type DWord -Force"#.to_string(),
        ],
        "dark_theme" => vec![
            r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Themes\Personalize' -Name 'AppsUseLightTheme' -Value 1 -Type DWord -Force"#.to_string(),
            r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Themes\Personalize' -Name 'SystemUsesLightTheme' -Value 1 -Type DWord -Force"#.to_string(),
        ],
        "bing_search_start" => vec![
            r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Search' -Name 'BingSearchEnabled' -Value 0 -Type DWord -Force"#.to_string(),
            r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Policies\Microsoft\Windows\Windows Search' -Name 'DisableWebSearch' -Value 1 -Type DWord -Force"#.to_string(),
        ],
        "numlock_on_startup" => vec![
            r#"Set-ItemProperty -Path 'HKU:\.DEFAULT\Control Panel\Keyboard' -Name 'InitialKeyboardIndicators' -Value '0' -Type String -Force"#.to_string(),
            r#"Set-ItemProperty -Path 'HKCU:\Control Panel\Keyboard' -Name 'InitialKeyboardIndicators' -Value '0' -Type String -Force"#.to_string(),
        ],
        "verbose_logon" => vec![
            r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System' -Name 'VerboseStatus' -Value 0 -Type DWord -Force"#.to_string(),
        ],
        "start_recommendations" => vec![
            r#"Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced' -Name 'Start_IrisRecommendations' -Value 0 -Type DWord -Force"#.to_string(),
            r#"Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced' -Name 'Start_TrackProgs' -Value 0 -Type DWord -Force"#.to_string(),
        ],
        "snap_window" => vec![
            r#"Set-ItemProperty -Path 'HKCU:\Control Panel\Desktop' -Name 'WindowArrangementActive' -Value '0' -Type String -Force"#.to_string(),
        ],
        "snap_assist_flyout" => vec![
            r#"Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced' -Name 'SnapAssist' -Value 0 -Type DWord -Force"#.to_string(),
        ],
        "snap_assist_suggestion" => vec![
            r#"Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced' -Name 'JointResize' -Value 0 -Type DWord -Force"#.to_string(),
        ],
        "mouse_acceleration" => vec![
            r#"Set-ItemProperty -Path 'HKCU:\Control Panel\Mouse' -Name 'MouseSpeed' -Value '1' -Type String -Force"#.to_string(),
            r#"Set-ItemProperty -Path 'HKCU:\Control Panel\Mouse' -Name 'MouseThreshold1' -Value '6' -Type String -Force"#.to_string(),
            r#"Set-ItemProperty -Path 'HKCU:\Control Panel\Mouse' -Name 'MouseThreshold2' -Value '10' -Type String -Force"#.to_string(),
        ],
        "sticky_keys" => vec![
            r#"Set-ItemProperty -Path 'HKCU:\Control Panel\Accessibility\StickyKeys' -Name 'Flags' -Value '506' -Type String -Force"#.to_string(),
        ],
        "multiplane_overlay" => vec![
            r#"Remove-ItemProperty -Path 'HKLM:\SOFTWARE\Microsoft\Windows\Dwm' -Name 'OverlayTestMode' -ErrorAction SilentlyContinue"#.to_string(),
        ],
        "remove_settings_home" => vec![
            r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\StartPage' -Name 'HideSettingsFromStartMenu' -Value 0 -Type DWord -Force"#.to_string(),
        ],
        "disable_background_apps" => vec![
            r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\AppPrivacy' -Name 'LetAppsRunInBackground' -Value 0 -Type DWord -Force"#.to_string(),
        ],
        "detailed_bso_d" => vec![
            r#"Set-ItemProperty -Path 'HKLM:\SYSTEM\CurrentControlSet\Control\CrashControl' -Name 'VerboseStatus' -Value 0 -Type DWord -Force"#.to_string(),
        ],
        "s3_sleep" => vec![r#"Set-ItemProperty -Path 'HKLM:\SYSTEM\CurrentControlSet\Control\Power' -Name 'CsEnabled' -Value 1 -Type DWord -Force"#.to_string()],
        "cross_device_resume" => vec![r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\Advanced' -Name 'EnableSharedExperiences' -Value 0 -Type DWord -Force"#.to_string()],
        "new_outlook" => vec![
            r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Office\16.0\outlook\newoutlook' -Name 'ForceNewOutlook' -Value 0 -Type DWord -Force"#.to_string(),
        ],
        "disable_search" => vec![r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Search' -Name 'SearchboxTaskbarMode' -Value 3 -Type DWord -Force"#.to_string()],
        "disable_task_view" => vec![r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\Advanced' -Name 'ShowTaskViewButton' -Value 1 -Type DWord -Force"#.to_string()],
        "center_taskbar_items" => vec![r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\Advanced' -Name 'TaskbarAl' -Value 0 -Type DWord -Force"#.to_string()],
        "disable_widgets" => vec![r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\Advanced' -Name 'TaskbarDa' -Value 1 -Type DWord -Force"#.to_string()],
        "disable_chat" => vec![r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\Advanced' -Name 'TaskbarMn' -Value 1 -Type DWord -Force"#.to_string()],
        "disable_consumer_features" => vec![
            r#"@('dmwappushservice','DiagTrack','RetailDemo','XblAuthManager','XblGameSave','WaaSMedicSvc') | ForEach-Object { Set-Service -Name $_ -StartupType Manual -ErrorAction SilentlyContinue }"#.to_string()
        ],
        "disable_ipv6" => vec![
            r#"Remove-ItemProperty -Path 'HKLM:\SYSTEM\CurrentControlSet\Services\Tcpip6\Parameters' -Name 'DisabledComponents' -ErrorAction SilentlyContinue"#.to_string(),
        ],
        "prefer_ipv4" => vec![
            r#"Remove-ItemProperty -Path 'HKLM:\SYSTEM\CurrentControlSet\Services\Tcpip6\Parameters' -Name 'DisabledComponents' -ErrorAction SilentlyContinue"#.to_string(),
        ],
        "disable_homegroup" => vec![
            r#"@('HomeGroupListener','HomeGroupProvider') | ForEach-Object { Set-Service -Name $_ -StartupType Manual -ErrorAction SilentlyContinue }"#.to_string()
        ],
        "services_manual" => vec![
            r#"@('WaaSMedicSvc','XblGameSave','XblAuthManager','DiagTrack') | ForEach-Object { Set-Service -Name $_ -StartupType Automatic -ErrorAction SilentlyContinue }"#.to_string()
        ],
        "debloat_edge" => vec![
            r#"@('MicrosoftEdgeUpdateTaskMachineCore','MicrosoftEdgeUpdateTaskMachineUA','MicrosoftEdgeUpdateTaskUserS-1-5-18') | ForEach-Object { schtasks /Change /TN $_ /Enable | Out-Null }"#.to_string(),
            r#"Set-Service -Name edgeupdate -StartupType Automatic -ErrorAction SilentlyContinue"#.to_string(),
        ],
        "debloat_adobe" => vec![
            r#"@('AdobeARMservice','AdobeUpdateService','Adobe Genuine Monitor Service') | ForEach-Object { Set-Service -Name $_ -StartupType Automatic -ErrorAction SilentlyContinue }"#.to_string(),
            r#"Get-ScheduledTask | Where-Object TaskName -Match 'Adobe' | ForEach-Object { Enable-ScheduledTask -TaskName $_.TaskName }"#.to_string(),
        ],
        "disable_teredo" => vec![r#"netsh interface teredo set state default"#.to_string()],
        "block_adobe_network" => vec![r#"$hosts = "$env:SystemRoot\System32\drivers\etc\hosts"
$entries = @('127.0.0.1 activate.adobe.com','127.0.0.1 ims-na1.adobelogin.com','127.0.0.1 practivate.adobe.com','127.0.0.1 oobe.adobe.com')
$temp = Get-Content $hosts
$filtered = $temp | Where-Object {
  $line = $_.Trim()
  $match = $false
  foreach ($entry in $entries) {
    if ($line -eq $entry) { $match = $true }
  }
  -not $match
}
$filtered | Set-Content $hosts -Force"#.to_string()],
        _ => Vec::new(),
    }
}

#[cfg(not(target_os = "windows"))]
fn script_for_tweak_revert(_id: &str) -> Vec<String> {
    Vec::new()
}

#[cfg(target_os = "windows")]
fn script_for_config_revert(id: &str) -> Vec<String> {
    match id {
        "dotnet_framework" => vec![r#"Start-Process dism -ArgumentList '/Online','/Disable-Feature','/FeatureName:NetFx3' -Wait | Out-Null"#.to_string()],
        "hyperv_virtualization" => vec![r#"Start-Process dism -ArgumentList '/Online','/Disable-Feature','/FeatureName:Microsoft-Hyper-V-All' -Wait | Out-Null"#.to_string()],
        "legacy_media" => vec![
            r#"Start-Process dism -ArgumentList '/Online','/Disable-Feature','/FeatureName:LegacyComponents' -Wait | Out-Null"#.to_string(),
        ],
        "nfs_network_file_system" => vec![r#"Start-Process dism -ArgumentList '/Online','/Disable-Feature','/FeatureName:ServicesForNFS-ClientOnly' -Wait | Out-Null"#.to_string()],
        "search_box_web_suggestions_enable" => vec![
            r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Search' -Name 'BingSearchEnabled' -Value 0 -Type DWord -Force"#.to_string(),
        ],
        "search_box_web_suggestions_disable" => vec![
            r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Search' -Name 'BingSearchEnabled' -Value 1 -Type DWord -Force"#.to_string(),
        ],
        "daily_registry_backup" => vec![r#"Unregister-ScheduledTask -TaskName 'AveloniaRegistryBackup' -Confirm:$false -ErrorAction SilentlyContinue"#.to_string()],
        "legacy_f8_boot_enable" => vec![r#"Start-Process bcdedit -ArgumentList '/set','{current}','bootmenupolicy','standard' -Wait | Out-Null"#.to_string()],
        "legacy_f8_boot_disable" => vec![r#"Start-Process bcdedit -ArgumentList '/set','{current}','bootmenupolicy','legacy' -Wait | Out-Null"#.to_string()],
        "wsl" => vec![
            r#"Start-Process dism -ArgumentList '/Online','/Disable-Feature','/FeatureName:Microsoft-Windows-Subsystem-Linux' -Wait | Out-Null"#.to_string(),
        ],
        "windows_sandbox" => vec![r#"Start-Process dism -ArgumentList '/Online','/Disable-Feature','/FeatureName:Containers-DisposableClientVM' -Wait | Out-Null"#.to_string()],
        _ => Vec::new(),
    }
}

#[cfg(not(target_os = "windows"))]
fn script_for_config_revert(_id: &str) -> Vec<String> {
    Vec::new()
}

pub fn apply_tweak_state(id: &str, enabled: bool) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let commands = if enabled {
            let mut c = script_for_tweak(id);
            if c.is_empty() {
                c = script_for_config(id);
            }
            c
        } else {
            let mut c = script_for_tweak_revert(id);
            if c.is_empty() {
                c = script_for_config_revert(id);
            }
            c
        };

        if !commands.is_empty() {
            run_powershell_commands(&commands, "tweak_state")?;
        }
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = id;
        let _ = enabled;
        Ok(())
    }
}

#[derive(Deserialize)]
pub struct TweakStateChange {
    pub id: String,
    pub enabled: bool,
}

pub fn apply_tweaks_state_batch(changes: Vec<TweakStateChange>) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let mut commands = Vec::new();
        for change in changes {
            let cmds = if change.enabled {
                let mut c = script_for_tweak(&change.id);
                if c.is_empty() {
                    c = script_for_config(&change.id);
                }
                c
            } else {
                let mut c = script_for_tweak_revert(&change.id);
                if c.is_empty() {
                    c = script_for_config_revert(&change.id);
                }
                c
            };
            commands.extend(cmds);
        }
        if !commands.is_empty() {
            run_powershell_commands(&commands, "tweak_batch")?;
        }
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = changes;
        Ok(())
    }
}

pub fn restart_explorer() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        run_powershell_commands(&["Stop-Process -Name explorer -Force".to_string()], "restart_explorer")?;
    }
    Ok(())
}
