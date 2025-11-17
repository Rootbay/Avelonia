use super::shell_helpers::run_powershell_commands;
use super::update_profiles::apply_update_profile_impl;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

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

#[cfg(target_os = "windows")]
fn script_for_tweak(id: &str) -> Vec<String> {
    match id {
        "create_restore_point" => {
            vec![r#"Checkpoint-Computer -Description 'Avelonia Tweaks' -RestorePointType 'MODIFY_SETTINGS' -ErrorAction SilentlyContinue | Out-Null"#.to_string()]
        }
        "disable_consumer_features" => vec![r#"@('dmwappushservice','DiagTrack','RetailDemo','XblAuthManager','XblGameSave','WaaSMedicSvc') | ForEach-Object { Stop-Service -Name $_ -Force -ErrorAction SilentlyContinue; Set-Service -Name $_ -StartupType Disabled -ErrorAction SilentlyContinue }"#.to_string()],
        "disable_telemetry" => vec![
            r#"New-Item -Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\DataCollection' -Force | Out-Null"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\DataCollection' -Name 'AllowTelemetry' -Value 0 -Type DWord -Force"#.to_string(),
            r#"New-Item -Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\Explorer' -Force | Out-Null"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\Explorer' -Name 'DisableTelemetry' -Value 1 -Type DWord -Force"#.to_string(),
        ],
        "disable_activity_history" => vec![
            r#"New-Item -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\ActivityHistory' -Force | Out-Null"#.to_string(),
            r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\ActivityHistory' -Name 'PublishUserActivities' -Value 0 -Type DWord -Force"#.to_string(),
            r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\ActivityHistory' -Name 'PublishUserActivitiesEnabled' -Value 0 -Type DWord -Force"#.to_string(),
            r#"New-Item -Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\ActivityHistory' -Force | Out-Null"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\ActivityHistory' -Name 'CaptureUserActivities' -Value 0 -Type DWord -Force"#.to_string(),
        ],
        "disable_explorer_discovery" => vec![
            r#"New-Item -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\Advanced' -Force | Out-Null"#.to_string(),
            r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\Advanced' -Name 'Start_TrackDocs' -Value 0 -Type DWord -Force"#.to_string(),
            r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\Advanced' -Name 'Start_TrackProgs' -Value 0 -Type DWord -Force"#.to_string(),
        ],
        "disable_gamedvr" => vec![
            r#"New-Item -Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\GameDVR' -Force | Out-Null"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\GameDVR' -Name 'AppCaptureEnabled' -Value 0 -Type DWord -Force"#.to_string(),
            r#"New-Item -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\GameDVR' -Force | Out-Null"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\GameDVR' -Name 'AllowGameDVR' -Value 0 -Type DWord -Force"#.to_string(),
        ],
        "disable_hibernation" => vec![r#"powercfg -h off"#.to_string()],
        "disable_homegroup" => vec![r#"@('HomeGroupProvider','HomeGroupListener') | ForEach-Object { Stop-Service -Name $_ -Force -ErrorAction SilentlyContinue; Set-Service -Name $_ -StartupType Disabled -ErrorAction SilentlyContinue }"#.to_string()],
        "disable_location_tracking" => vec![
            r#"New-Item -Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\location' -Force | Out-Null"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\location' -Name 'Value' -Value 'Deny' -Type String -Force"#.to_string(),
            r#"New-Item -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\location' -Force | Out-Null"#.to_string(),
            r#"Set-ItemProperty -Path 'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\location' -Name 'Value' -Value 'Deny' -Type String -Force"#.to_string(),
        ],
        "disable_storage_sense" => vec![
            r#"New-Item -Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\StorageSense\Parameters\StorageSenseGlobal' -Force | Out-Null"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\StorageSense\Parameters\StorageSenseGlobal' -Name 'StorageSenseEnabled' -Value 0 -Type DWord -Force"#.to_string(),
        ],
        "disable_wifi_sense" => vec![
            r#"New-Item -Path 'HKLM:\SOFTWARE\Microsoft\WcmSvc\wifinetworkmanager\config' -Force | Out-Null"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Microsoft\WcmSvc\wifinetworkmanager\config' -Name 'AutoConnectAllowedOEM' -Value 0 -Type DWord -Force"#.to_string(),
            r#"Set-ItemProperty -Path 'HKLM:\SOFTWARE\Microsoft\WcmSvc\wifinetworkmanager\config' -Name 'AutoConnectAllowedNonOEM' -Value 0 -Type DWord -Force"#.to_string(),
        ],
        "enable_end_task" => vec![
            r#"$key='HKCR:\DesktopBackground\Shell\EndTask'"#.to_string(),
            r#"New-Item -Path $key -Force | Out-Null"#.to_string(),
            r#"Set-ItemProperty -Path $key -Name '(Default)' -Value 'End Task' -Force"#.to_string(),
            r#"Set-ItemProperty -Path $key -Name 'Icon' -Value 'taskmgr.exe' -Force"#.to_string(),
            r#"New-Item -Path "$key\Command" -Force | Out-Null"#.to_string(),
            r#"Set-ItemProperty -Path "$key\Command" -Name '(Default)' -Value 'taskmgr.exe' -Force"#.to_string(),
        ],
        "run_disk_cleanup" => vec![r#"Start-Process cleanmgr -ArgumentList '/sagerun:99' -Wait | Out-Null"#.to_string()],
        "terminal_powershell7_default" => vec![r#"$terminalPath = Join-Path $env:LOCALAPPDATA 'Packages\Microsoft.WindowsTerminal_8wekyb3d8bbwe\LocalState\settings.json'
if (Test-Path $terminalPath) {
  $settings = Get-Content $terminalPath -Raw | ConvertFrom-Json
  $profile = $settings.profiles.list | Where-Object { $_.name -like 'PowerShell 7*' } | Select-Object -First 1
  if ($profile) {
    $settings.profiles.default = $profile.guid
    $settings | ConvertTo-Json -Depth 5 | Set-Content $terminalPath -Force
  }
}"#.to_string()],
        "disable_powershell7_telemetry" => vec![
            r#"[Environment]::SetEnvironmentVariable('POWERSHELL_TELEMETRY_OPTOUT','1','Machine')"#.to_string(),
            r#"[Environment]::SetEnvironmentVariable('POWERSHELL_UPDATECHECK','0','Machine')"#.to_string(),
        ],
        "disable_recall" => vec![
            r#"New-Item -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Recall' -Force | Out-Null"#.to_string(),
            r#"Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Recall' -Name 'Disabled' -Value 1 -Type DWord -Force"#.to_string(),
            r#"Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Recall' -Name 'NoActivityCollector' -Value 1 -Type DWord -Force"#.to_string(),
        ],
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
