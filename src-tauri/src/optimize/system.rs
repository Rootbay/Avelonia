use crate::AppError;
use std::env;
use std::path::PathBuf;
use sysinfo::System;
use super::shell_helpers::{run_cmd_elevated, run_powershell_elevated};

#[tauri::command]
pub async fn get_startup_folders() -> Result<Vec<String>, AppError> {
    let mut out = Vec::new();
    if let Some(appdata) = env::var_os("APPDATA") {
        let user_startup =
            PathBuf::from(appdata).join(r"Microsoft\Windows\Start Menu\Programs\Startup");
        if user_startup.exists() && user_startup.is_dir() {
            out.push(user_startup.display().to_string());
        }
    }
    if let Some(programdata) = env::var_os("PROGRAMDATA") {
        let all_startup = PathBuf::from(programdata.clone())
            .join(r"Microsoft\Windows\Start Menu\Programs\StartUp");
        if all_startup.exists() && all_startup.is_dir() {
            out.push(all_startup.display().to_string());
        } else {
            let alt =
                PathBuf::from(programdata).join(r"Microsoft\Windows\Start Menu\Programs\Startup");
            if alt.exists() && alt.is_dir() {
                out.push(alt.display().to_string());
            }
        }
    }
    Ok(out)
}

#[tauri::command]
pub async fn is_process_running(image: String) -> Result<bool, AppError> {
    let target = image.to_lowercase();
    let sys = System::new_all();
    let running = sys.processes().values().any(|p| {
        let name = p.name().to_string_lossy().to_string();
        name.eq_ignore_ascii_case(&image) || name.to_lowercase() == target
    });
    Ok(running)
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub async fn block_process_ifeo(images: Vec<String>, enable: bool) -> Result<usize, AppError> {
    let mut script = String::from("$ErrorActionPreference='SilentlyContinue'\n");
    for img in &images {
        let name = img.replace("'", "''");
        script.push_str(&format!(
            "$k='HKLM:\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Image File Execution Options\\{}'\n",
            name
        ));
        if enable {
            script.push_str("New-Item -Path $k -Force | Out-Null\n");
            script.push_str(
                "Set-ItemProperty -Path $k -Name Debugger -Value 'cmd.exe /c exit 0' -Force\n",
            );
        } else {
            script.push_str(
                "Remove-ItemProperty -Path $k -Name Debugger -ErrorAction SilentlyContinue\n",
            );
            script.push_str("Remove-Item -Path $k -ErrorAction SilentlyContinue\n");
        }
    }
    let mut tmp = env::temp_dir();
    tmp.push("avelonia_ifeo.ps1");
    let tmp_str = tmp.to_string_lossy().to_string();
    std::fs::write(&tmp, &script).map_err(AppError::Io)?;
    let _ok = run_powershell_elevated(&[
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        &tmp_str,
    ]);
    let _ = std::fs::remove_file(&tmp);
    Ok(images.len())
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub async fn block_process_ifeo(_images: Vec<String>, _enable: bool) -> Result<usize, AppError> {
    Err(AppError::System("Only available on Windows".into()))
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub async fn schedule_delete_on_reboot(paths: Vec<String>) -> Result<usize, AppError> {
    let mut script = String::from("$ErrorActionPreference='SilentlyContinue'\n");
    script.push_str("$sm='HKLM:\\SYSTEM\\CurrentControlSet\\Control\\Session Manager'\n");
    script.push_str("$val=(Get-ItemProperty -Path $sm -Name PendingFileRenameOperations -ErrorAction SilentlyContinue).PendingFileRenameOperations\n");
    script.push_str("if(-not $val){ $val=@() }\n");
    for p in &paths {
        let clean = p.replace("'", "''");
        script.push_str(&format!(
            "$pp=[Environment]::ExpandEnvironmentVariables('{}')\n$val += ('\\??\\' + $pp), ''\n",
            clean
        ));
    }
    script.push_str("Set-ItemProperty -Path $sm -Name PendingFileRenameOperations -Value $val -Type MultiString\n");
    let mut tmp = env::temp_dir();
    tmp.push("avelonia_delete_on_reboot.ps1");
    let tmp_str = tmp.to_string_lossy().to_string();
    std::fs::write(&tmp, &script).map_err(AppError::Io)?;
    let _ok = run_powershell_elevated(&[
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        &tmp_str,
    ]);
    let _ = std::fs::remove_file(&tmp);
    Ok(paths.len())
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub async fn schedule_delete_on_reboot(_paths: Vec<String>) -> Result<usize, AppError> {
    Err(AppError::System("Only available on Windows".into()))
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub async fn remove_wmi_subscriptions_by_match(
    images: Vec<String>,
    paths: Vec<String>,
) -> Result<usize, AppError> {
    let mut script = String::from("$ErrorActionPreference='SilentlyContinue'\n");
    script.push_str("$images = @()\n$paths = @()\n");
    for i in &images {
        script.push_str(&format!("$images += '{}'\n", i.replace("'", "''")));
    }
    for p in &paths {
        script.push_str(&format!("$paths += '{}'\n", p.replace("'", "''")));
    }
    script.push_str(r#"
function Hit([string]$s){ $t=$s.ToLower(); foreach($x in $images){ if($t.Contains($x.ToLower())){ return $true } } foreach($y in $paths){ if($t.Contains($y.ToLower())){ return $true } } return $false }
$removed = 0
$cons = Get-CimInstance -Namespace root\subscription -ClassName CommandLineEventConsumer
foreach($c in $cons){ if(Hit([string]$c.CommandLineTemplate)){ Remove-CimInstance $c; $removed++ } }
$ascons = Get-CimInstance -Namespace root\subscription -ClassName ActiveScriptEventConsumer
foreach($c in $ascons){ if(Hit([string]$c.ScriptText)){ Remove-CimInstance $c; $removed++ } }
$binds = Get-CimInstance -Namespace root\subscription -ClassName __FilterToConsumerBinding
foreach($b in $binds){ try { $fc = (Get-CimAssociatedInstance -InputObject $b -Association __FilterToConsumerBinding); if(-not $fc){ Remove-CimInstance $b; $removed++ } } catch {} }
$filters = Get-CimInstance -Namespace root\subscription -ClassName __EventFilter
foreach($f in $filters){ if(Hit([string]$f.Query)){ Remove-CimInstance $f; $removed++ } }
Write-Output $removed
"#);
    let mut tmp = env::temp_dir();
    tmp.push("avelonia_wmi_cleanup.ps1");
    let tmp_str = tmp.to_string_lossy().to_string();
    std::fs::write(&tmp, &script).map_err(AppError::Io)?;
    let _ok = run_powershell_elevated(&[
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        &tmp_str,
    ]);
    let _ = std::fs::remove_file(&tmp);
    Ok(1)
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub async fn remove_wmi_subscriptions_by_match(
    _images: Vec<String>,
    _paths: Vec<String>,
) -> Result<usize, AppError> {
    Ok(0)
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub async fn restart_system() -> Result<(), AppError> {
    let ok = run_cmd_elevated(&["/C", "shutdown", "/r", "/t", "0"]);
    if ok {
        Ok(())
    } else {
        Err(AppError::System("failed to trigger restart".into()))
    }
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub async fn restart_system() -> Result<(), AppError> {
    Err(AppError::System("Only available on Windows".into()))
}