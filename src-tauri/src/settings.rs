use crate::AppError;
use serde::Serialize;
use std::fs;
use std::path::{Component, Path, PathBuf};
use tauri::{AppHandle, Manager};

#[derive(Serialize)]
pub struct SettingsLocation {
    pub path: String,
    pub portable: bool,
}

const SETTINGS_FILENAME: &str = "avelonia_settings_v1.json";

fn exe_dir() -> Result<PathBuf, AppError> {
    let exe = std::env::current_exe().map_err(AppError::Io)?;
    exe.parent()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| AppError::Internal("Executable directory not found".into()))
}

#[cfg(target_os = "windows")]
fn is_removable_drive(path: &Path) -> bool {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Storage::FileSystem::GetDriveTypeW;
    use windows::core::PCWSTR;

    let root = match drive_root(path) {
        Some(root) => root,
        None => return false,
    };

    let mut wide: Vec<u16> = root.as_os_str().encode_wide().collect();
    wide.push(0);
    let drive_type = unsafe { GetDriveTypeW(PCWSTR::from_raw(wide.as_ptr())) };
    // DRIVE_REMOVABLE == 2
    drive_type == 2
}

#[cfg(not(target_os = "windows"))]
fn is_removable_drive(_path: &Path) -> bool {
    false
}

#[cfg(target_os = "windows")]
fn drive_root(path: &Path) -> Option<PathBuf> {
    let mut components = path.components();
    match components.next() {
        Some(Component::Prefix(prefix)) => match prefix.kind() {
            std::path::Prefix::Disk(disk) | std::path::Prefix::VerbatimDisk(disk) => {
                let letter = (disk as char).to_ascii_uppercase();
                Some(PathBuf::from(format!("{letter}:\\",)))
            }
            _ => None,
        },
        _ => None,
    }
}

#[cfg(not(target_os = "windows"))]
fn drive_root(_path: &Path) -> Option<PathBuf> {
    None
}

fn settings_base_dir(app: &AppHandle) -> Result<(PathBuf, bool), AppError> {
    let exe_dir = exe_dir()?;
    if is_removable_drive(&exe_dir) {
        return Ok((exe_dir, true));
    }
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e: tauri::Error| AppError::Tauri(e.to_string()))?;
    Ok((dir, false))
}

fn settings_path(app: &AppHandle) -> Result<(PathBuf, bool), AppError> {
    let (base, portable) = settings_base_dir(app)?;
    Ok((base.join(SETTINGS_FILENAME), portable))
}

#[tauri::command]
pub fn settings_location(app: AppHandle) -> Result<SettingsLocation, AppError> {
    let (path, portable) = settings_path(&app)?;
    Ok(SettingsLocation {
        path: path.to_string_lossy().to_string(),
        portable,
    })
}

#[tauri::command]
pub fn settings_read(app: AppHandle) -> Result<Option<String>, AppError> {
    let (path, _portable) = settings_path(&app)?;
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path).map_err(AppError::Io)?;
    Ok(Some(raw))
}

#[tauri::command]
pub fn settings_write(app: AppHandle, contents: String) -> Result<(), AppError> {
    let (path, _portable) = settings_path(&app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(AppError::Io)?;
    }
    fs::write(&path, contents).map_err(AppError::Io)?;
    Ok(())
}
