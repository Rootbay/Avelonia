use std::
collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use tauri::Emitter; // Added this line
use sysinfo::Disks;

use sha2::{Digest, Sha256};
use walkdir::WalkDir;
use lnk::{ShellLink};
use lnk::encoding::WINDOWS_1252;

#[cfg(target_os = "windows")]
use windows::Win32::UI::Shell::{SHEmptyRecycleBinA, SHERB_NOCONFIRMATION, SHERB_NOPROGRESSUI, SHERB_NOSOUND};

#[tauri::command]
pub fn get_temp_files(app_handle: tauri::AppHandle) -> Result<Vec<String>, String> {
    let mut temp_files = Vec::new();
    let mut scanned_count = 0;

    let common_temp_paths: Vec<PathBuf> = vec![
        // Windows
        env::var_os("TEMP").map(PathBuf::from).unwrap_or_default(),
        env::var_os("LOCALAPPDATA").map(|p| PathBuf::from(p).join("Temp")).unwrap_or_default(),
        // macOS
        PathBuf::from("/tmp"),
        PathBuf::from("/private/tmp"),
        // Linux
        PathBuf::from("/var/tmp"),
    ].into_iter().filter(|p| p.exists() && p.is_dir()).collect();

    for temp_dir_path in common_temp_paths {
        for entry in WalkDir::new(&temp_dir_path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                temp_files.push(entry.path().display().to_string());
                scanned_count += 1;
                if scanned_count % 100 == 0 { // Emit progress every 100 files
                    app_handle.emit("scan_progress", format!("Scanned {} temporary files...", scanned_count))
                        .map_err(|e| e.to_string())?;
                }
            }
        }
    }

    Ok(temp_files)
}

fn _delete_files_helper(files: Vec<String>) -> Result<usize, String> {
    let mut deleted_count = 0;
    for file_path_str in files {
        let path = PathBuf::from(&file_path_str);
        if path.exists() {
            if path.is_file() {
                match fs::remove_file(&path) {
                    Ok(_) => deleted_count += 1,
                    Err(e) => eprintln!("Failed to delete file {}: {}", file_path_str, e),
                }
            } else if path.is_dir() {
                match fs::remove_dir_all(&path) {
                    Ok(_) => deleted_count += 1,
                    Err(e) => eprintln!("Failed to delete directory {}: {}", file_path_str, e),
                }
            }
        }
    }
    Ok(deleted_count)
}

#[tauri::command]
pub fn clean_temp_files(files: Vec<String>) -> Result<usize, String> {
    _delete_files_helper(files)
}

#[tauri::command]
pub fn delete_files(files: Vec<String>) -> Result<usize, String> {
    _delete_files_helper(files)
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub fn empty_recycle_bin() -> Result<(), String> {
    unsafe {
        let flags = SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND;
        let result = SHEmptyRecycleBinA(None, None, flags);
        if result.is_ok() {
            Ok(())
        } else {
            Err(format!("Failed to empty recycle bin: {:?}", result))
        }
    }
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn empty_recycle_bin() -> Result<(), String> {
    Err("Emptying recycle bin is only supported on Windows.".to_string())
}

#[allow(dead_code)]
#[tauri::command]
pub fn get_drive_info() -> Result<(u64, u64), String> {
    let mut total_disk_space: u64 = 0;
    let mut available_disk_space: u64 = 0;

    let disks = Disks::new_with_refreshed_list();
    for disk in disks.list() {
        total_disk_space += disk.total_space();
        available_disk_space += disk.available_space();
    }

    Ok((total_disk_space, available_disk_space))
}


#[tauri::command]
pub fn find_large_files(app_handle: tauri::AppHandle) -> Result<Vec<(String, u64)>, String> {
    let mut large_files = Vec::new();
    let min_size_bytes: u64 = 100 * 1024 * 1024; // 100 MB
    let mut scanned_count = 0;

    // Common directories to scan for large files
    let user_profile = env::var_os("USERPROFILE").ok_or("USERPROFILE not found".to_string())?;
    let downloads_path = PathBuf::from(&user_profile).join("Downloads");
    let desktop_path = PathBuf::from(&user_profile).join("Desktop");
    let documents_path = PathBuf::from(&user_profile).join("Documents");

    let paths_to_scan = vec![downloads_path, desktop_path, documents_path];

    for scan_path in paths_to_scan {
        if scan_path.exists() && scan_path.is_dir() {
            for entry in WalkDir::new(&scan_path)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_type().is_file() {
                    if let Ok(_metadata) = entry.metadata() {
                        if _metadata.len() >= min_size_bytes {
                            large_files.push((entry.path().display().to_string(), _metadata.len()));
                        }
                    }
                }
                scanned_count += 1;
                if scanned_count % 100 == 0 { // Emit progress every 100 files
                    app_handle.emit("scan_progress", format!("Scanned {} files for large files...", scanned_count))
                        .map_err(|e| e.to_string())?;
                }
            }
        }
    }

    Ok(large_files)
}

#[tauri::command]
pub fn find_duplicate_files(app_handle: tauri::AppHandle) -> Result<Vec<(String, u64)>, String> {
    let mut file_hashes: HashMap<String, Vec<String>> = HashMap::new();
    let mut duplicate_files = Vec::new();
    let mut scanned_count = 0;

    // Common directories to scan for duplicate files
    let user_profile = env::var_os("USERPROFILE").ok_or("USERPROFILE not found".to_string())?;
    let downloads_path = PathBuf::from(&user_profile).join("Downloads");
    let documents_path = PathBuf::from(&user_profile).join("Documents");

    let paths_to_scan = vec![downloads_path, documents_path];

    for scan_path in paths_to_scan {
        if scan_path.exists() && scan_path.is_dir() {
            for entry in WalkDir::new(&scan_path)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_type().is_file() {
                    let path = entry.path();
                    if let Ok(_metadata) = entry.metadata() {
                        match calculate_file_hash(path) {
                            Ok(hash) => {
                                file_hashes
                                    .entry(hash)
                                    .or_insert_with(Vec::new)
                                    .push(path.display().to_string());
                            }
                            Err(e) => eprintln!("Failed to hash file {}: {}", path.display(), e),
                        }
                    }
                }
                scanned_count += 1;
                if scanned_count % 100 == 0 { // Emit progress every 100 files
                    app_handle.emit("scan_progress", format!("Scanned {} files for duplicates...", scanned_count))
                        .map_err(|e| e.to_string())?;
                }
            }
        }
    }

    for (_hash, paths) in file_hashes {
        if paths.len() > 1 {
            // For duplicate files, we need to get the size of each file.
            // Assuming all files with the same hash have the same size.
            if let Some(first_path_str) = paths.first() {
                let first_path = PathBuf::from(first_path_str);
                if let Ok(_metadata) = fs::metadata(&first_path) {
                    for p in paths {
                        duplicate_files.push((p, _metadata.len()));
                    }
                }
            }
        }
    }

    Ok(duplicate_files)
}

fn calculate_file_hash(path: &Path) -> Result<String, io::Error> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 1024]; // Read in 1KB chunks

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

#[tauri::command]
pub fn find_empty_folders(app_handle: tauri::AppHandle) -> Result<Vec<String>, String> {
    let mut empty_folders = Vec::new();
    let mut scanned_count = 0;

    let user_profile = env::var_os("USERPROFILE").ok_or("USERPROFILE not found".to_string())?;
    let downloads_path = PathBuf::from(&user_profile).join("Downloads");
    let desktop_path = PathBuf::from(&user_profile).join("Desktop");
    let documents_path = PathBuf::from(&user_profile).join("Documents");

    let paths_to_scan = vec![downloads_path, desktop_path, documents_path];

    for scan_path in paths_to_scan {
        if scan_path.exists() && scan_path.is_dir() {
            for entry in WalkDir::new(&scan_path)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_type().is_dir() {
                    let path = entry.path();
                    if fs::read_dir(path).map_err(|e| e.to_string())?.next().is_none() {
                        empty_folders.push(path.display().to_string());
                    }
                }
                scanned_count += 1;
                if scanned_count % 100 == 0 {
                    app_handle.emit("scan_progress", format!("Scanned {} directories for empty folders...", scanned_count))
                        .map_err(|e| e.to_string())?;
                }
            }
        }
    }

    Ok(empty_folders)
}

#[tauri::command]
pub fn find_broken_shortcuts(app_handle: tauri::AppHandle) -> Result<Vec<String>, String> {
    let mut broken_shortcuts = Vec::new();
    let mut scanned_count = 0;

    let user_profile = env::var_os("USERPROFILE").ok_or("USERPROFILE not found".to_string())?;
    let downloads_path = PathBuf::from(&user_profile).join("Downloads");
    let desktop_path = PathBuf::from(&user_profile).join("Desktop");
    let documents_path = PathBuf::from(&user_profile).join("Documents");

    let paths_to_scan = vec![downloads_path, desktop_path, documents_path];

    for scan_path in paths_to_scan {
        if scan_path.exists() && scan_path.is_dir() {
            for entry in WalkDir::new(&scan_path)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_type().is_file() {
                    let path = entry.path();
                    if let Some(extension) = path.extension() {
                        if extension.eq_ignore_ascii_case("lnk") {
                            match ShellLink::open(path, WINDOWS_1252) {
                                Ok(shell_link) => {
                                    if let Some(link_info) = shell_link.link_info() {
                                        let common_path_str = link_info.common_path_suffix();
                                        let target_path = PathBuf::from(common_path_str.to_string());
                                        if !target_path.exists() {
                                            broken_shortcuts.push(path.display().to_string());
                                        }
                                    }
                                }
                                Err(e) => eprintln!("Failed to parse .lnk file {}: {}", path.display(), e),
                            }
                        }
                    }
                }
                scanned_count += 1;
                if scanned_count % 100 == 0 {
                    app_handle.emit("scan_progress", format!("Scanned {} files for broken shortcuts...", scanned_count))
                        .map_err(|e| e.to_string())?;
                }
            }
        }
    }

    Ok(broken_shortcuts)
}
