use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use tauri::{AppHandle, Emitter};

static TEMP_CANCEL: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));
use sysinfo::Disks;

use lnk::ShellLink;
use lnk::encoding::WINDOWS_1252;
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use walkdir::WalkDir;

#[cfg(target_os = "windows")]
use windows::Win32::UI::Shell::{
    SHERB_NOCONFIRMATION, SHERB_NOPROGRESSUI, SHERB_NOSOUND, SHEmptyRecycleBinA,
};

#[derive(serde::Serialize, serde::Deserialize, Default, Clone)]
pub struct CleanupStats {
    pub files_deleted: u64,
    pub bytes_deleted: u64,
}

fn clear_directory_contents(root: &Path) -> Result<CleanupStats, String> {
    if !root.exists() || !root.is_dir() {
        return Ok(CleanupStats::default());
    }
    let mut stats = CleanupStats::default();
    for entry in WalkDir::new(root)
        .min_depth(1)
        .contents_first(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if entry.file_type().is_file() {
            if let Ok(meta) = fs::metadata(path) {
                if fs::remove_file(path).is_ok() {
                    stats.files_deleted += 1;
                    stats.bytes_deleted += meta.len();
                }
            } else {
                let _ = fs::remove_file(path);
            }
        } else if entry.file_type().is_dir() {
            let _ = fs::remove_dir(path);
        }
    }
    Ok(stats)
}

#[tauri::command]
pub fn get_temp_files(app_handle: tauri::AppHandle) -> Result<Vec<String>, String> {
    let mut temp_files = Vec::new();
    let mut scanned_count = 0;

    let common_temp_paths: Vec<PathBuf> = vec![
        env::var_os("TEMP").map(PathBuf::from).unwrap_or_default(),
        env::var_os("LOCALAPPDATA")
            .map(|p| PathBuf::from(p).join("Temp"))
            .unwrap_or_default(),
        PathBuf::from("/tmp"),
        PathBuf::from("/private/tmp"),
        PathBuf::from("/var/tmp"),
    ]
    .into_iter()
    .filter(|p| p.exists() && p.is_dir())
    .collect();

    for temp_dir_path in common_temp_paths {
        for entry in WalkDir::new(&temp_dir_path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                temp_files.push(entry.path().display().to_string());
                scanned_count += 1;
                if scanned_count % 100 == 0 {
                    app_handle
                        .emit(
                            "scan_progress",
                            format!("Scanned {} temporary files...", scanned_count),
                        )
                        .map_err(|e| e.to_string())?;
                }
            }
        }
    }

    Ok(temp_files)
}

#[tauri::command]
pub fn get_temp_files_stream(
    app_handle: AppHandle,
    batch_size: Option<usize>,
    max: Option<usize>,
) -> Result<usize, String> {
    let bs = batch_size.unwrap_or(250).max(50).min(1000);
    let limit = max.unwrap_or(500_000);
    TEMP_CANCEL.store(false, Ordering::Relaxed);

    let mut roots: Vec<PathBuf> = Vec::new();
    let wp = crate::paths::WindowsPaths::get();

    roots.push(wp.temp.clone());
    roots.push(wp.local_app_data.join("Temp"));

    roots.push(PathBuf::from("/tmp"));
    roots.push(PathBuf::from("/private/tmp"));
    roots.push(PathBuf::from("/var/tmp"));

    let local_app_data = wp.local_app_data;
    roots.push(local_app_data.join("Google/Chrome/User Data/Default/Cache"));
    roots.push(local_app_data.join("Google/Chrome/User Data/Default/Code Cache"));

    roots.push(local_app_data.join("Microsoft/Edge/User Data/Default/Cache"));
    roots.push(local_app_data.join("Microsoft/Edge/User Data/Default/Code Cache"));

    let firefox_profiles = local_app_data.join("Mozilla/Firefox/Profiles");
    if firefox_profiles.exists() {
        if let Ok(entries) = fs::read_dir(firefox_profiles) {
            for entry in entries.filter_map(|e| e.ok()) {
                roots.push(entry.path().join("cache2"));
                roots.push(entry.path().join("jumpListCache"));
            }
        }
    }

    roots.push(local_app_data.join("Microsoft/Windows/WER"));

    let windir = wp.windir;
    if !windir.as_os_str().is_empty() {
        roots.push(windir.join("Logs"));
        roots.push(windir.join("SoftwareDistribution/Download"));
        roots.push(windir.join("Minidump"));
    }

    roots.sort();
    roots.dedup();

    let active_roots: Vec<PathBuf> = roots
        .into_iter()
        .filter(|p| p.exists() && p.is_dir())
        .collect();
    let (tx, rx) = mpsc::channel::<String>();
    let mut handles = Vec::new();

    for root in active_roots {
        let txc = tx.clone();
        let ah = app_handle.clone();
        handles.push(thread::spawn(move || {
            let mut scanned = 0usize;
            for entry in walkdir::WalkDir::new(&root)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if TEMP_CANCEL.load(Ordering::Relaxed) {
                    break;
                }
                if entry.file_type().is_file() {
                    let _ = txc.send(entry.path().display().to_string());
                }
                scanned += 1;
                if scanned % 2000 == 0 {
                    let _ = ah.emit(
                        "scan_progress",
                        format!(
                            "Scanned {} entries in {}...",
                            scanned,
                            root.file_name().and_then(|n| n.to_str()).unwrap_or("dir")
                        ),
                    );
                }
            }
        }));
    }
    drop(tx);

    let mut total = 0usize;
    let mut batch: Vec<String> = Vec::with_capacity(bs);
    while let Ok(p) = rx.recv() {
        if TEMP_CANCEL.load(Ordering::Relaxed) {
            break;
        }
        total += 1;
        if total >= limit {
            TEMP_CANCEL.store(true, Ordering::Relaxed);
        }
        batch.push(p);
        if batch.len() >= bs {
            let _ = app_handle.emit("cleaner-temp-batch", batch.clone());
            batch.clear();
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        if total >= limit {
            break;
        }
    }
    if !batch.is_empty() {
        let _ = app_handle.emit("cleaner-temp-batch", batch);
    }
    for h in handles {
        let _ = h.join();
    }
    let _ = app_handle.emit("cleaner-temp-done", serde_json::json!({"total": total}));
    Ok(total)
}

#[tauri::command]
pub fn cancel_temp_scan() -> Result<(), String> {
    TEMP_CANCEL.store(true, Ordering::Relaxed);
    Ok(())
}

fn emit_chunked_pairs(app: &AppHandle, event: &str, items: &[(String, u64)], chunk: usize) {
    if items.is_empty() {
        return;
    }
    let mut start = 0usize;
    let n = items.len();
    while start < n {
        let end = std::cmp::min(start + chunk, n);
        let slice: Vec<(String, u64)> = items[start..end].to_vec();
        let _ = app.emit(event, slice);
        std::thread::sleep(std::time::Duration::from_millis(8));
        start = end;
        if TEMP_CANCEL.load(Ordering::Relaxed) {
            break;
        }
    }
}

fn emit_chunked_strings(app: &AppHandle, event: &str, items: &[String], chunk: usize) {
    if items.is_empty() {
        return;
    }
    let mut start = 0usize;
    let n = items.len();
    while start < n {
        let end = std::cmp::min(start + chunk, n);
        let slice: Vec<String> = items[start..end].to_vec();
        let _ = app.emit(event, slice);
        std::thread::sleep(std::time::Duration::from_millis(8));
        start = end;
        if TEMP_CANCEL.load(Ordering::Relaxed) {
            break;
        }
    }
}

#[tauri::command]
pub fn start_cleaner_scan(
    app_handle: AppHandle,
    min_size_bytes: Option<u64>,
    max_temp: Option<usize>,
) -> Result<(), String> {
    let app = app_handle.clone();
    TEMP_CANCEL.store(false, Ordering::Relaxed);
    std::thread::spawn(move || {
        let _ = app.emit("cleaner-progress", "Starting scan...");
        let _ = app.emit("cleaner-progress", "Scanning temporary files...");
        let _ = get_temp_files_stream(app.clone(), Some(250), Some(max_temp.unwrap_or(500_000)));

        if TEMP_CANCEL.load(Ordering::Relaxed) {
            let _ = app.emit("cleaner-stopped", serde_json::json!({"scope":"temp"}));
            return;
        }

        let min = min_size_bytes.unwrap_or(100 * 1024 * 1024);
        let _ = app.emit("cleaner-progress", "Scanning large files...");
        match find_large_files_min(min, app.clone()) {
            Ok(list) => {
                emit_chunked_pairs(&app, "cleaner-large-batch", &list, 300);
                let _ = app.emit("cleaner-done", serde_json::json!({"scope":"large"}));
            }
            Err(e) => {
                let _ = app.emit("cleaner-error", format!("large: {}", e));
            }
        }

        if TEMP_CANCEL.load(Ordering::Relaxed) {
            let _ = app.emit("cleaner-stopped", serde_json::json!({"scope":"large"}));
            return;
        }

        let _ = app.emit("cleaner-progress", "Scanning duplicates...");
        match find_duplicate_groups(app.clone()) {
            Ok(groups) => {
                let mut start = 0usize;
                let chunk = 60usize;
                while start < groups.len() {
                    let end = std::cmp::min(start + chunk, groups.len());
                    let slice = &groups[start..end];
                    let _ = app.emit("cleaner-dup-groups-batch", slice);
                    start = end;
                    if TEMP_CANCEL.load(Ordering::Relaxed) {
                        break;
                    }
                }
                let _ = app.emit("cleaner-done", serde_json::json!({"scope":"duplicate"}));
            }
            Err(e) => {
                let _ = app.emit("cleaner-error", format!("duplicate: {}", e));
            }
        }

        if TEMP_CANCEL.load(Ordering::Relaxed) {
            let _ = app.emit("cleaner-stopped", serde_json::json!({"scope":"duplicate"}));
            return;
        }

        let _ = app.emit("cleaner-progress", "Scanning empty folders...");
        match find_empty_folders(app.clone()) {
            Ok(list) => {
                emit_chunked_strings(&app, "cleaner-empty-batch", &list, 500);
                let _ = app.emit("cleaner-done", serde_json::json!({"scope":"empty"}));
            }
            Err(e) => {
                let _ = app.emit("cleaner-error", format!("empty: {}", e));
            }
        }

        if TEMP_CANCEL.load(Ordering::Relaxed) {
            let _ = app.emit("cleaner-stopped", serde_json::json!({"scope":"empty"}));
            return;
        }

        let _ = app.emit("cleaner-progress", "Scanning broken shortcuts...");
        match find_broken_shortcuts(app.clone()) {
            Ok(list) => {
                emit_chunked_strings(&app, "cleaner-shortcut-batch", &list, 500);
                let _ = app.emit("cleaner-done", serde_json::json!({"scope":"shortcuts"}));
            }
            Err(e) => {
                let _ = app.emit("cleaner-error", format!("shortcuts: {}", e));
            }
        }

        let _ = app.emit("cleaner-done", serde_json::json!({"scope":"all"}));
    });
    Ok(())
}

#[tauri::command]
pub fn cancel_cleaner_scan() -> Result<(), String> {
    TEMP_CANCEL.store(true, Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
pub fn start_large_scan(app_handle: AppHandle, min_size_bytes: Option<u64>) -> Result<(), String> {
    let app = app_handle.clone();
    TEMP_CANCEL.store(false, Ordering::Relaxed);
    std::thread::spawn(move || {
        let min = min_size_bytes.unwrap_or(100 * 1024 * 1024);
        let _ = app.emit("cleaner-progress", "Scanning large files...");
        match find_large_files_min(min, app.clone()) {
            Ok(list) => {
                emit_chunked_pairs(&app, "cleaner-large-batch", &list, 300);
                let _ = app.emit("cleaner-done", serde_json::json!({"scope":"large"}));
            }
            Err(e) => {
                let _ = app.emit("cleaner-error", format!("large: {}", e));
            }
        }
    });
    Ok(())
}

#[tauri::command]
pub fn start_duplicate_groups_scan(app_handle: AppHandle) -> Result<(), String> {
    let app = app_handle.clone();
    TEMP_CANCEL.store(false, Ordering::Relaxed);
    std::thread::spawn(move || {
        let _ = app.emit("cleaner-progress", "Scanning duplicates...");
        match find_duplicate_groups(app.clone()) {
            Ok(groups) => {
                let mut start = 0usize;
                let chunk = 60usize;
                while start < groups.len() {
                    let end = std::cmp::min(start + chunk, groups.len());
                    let slice = &groups[start..end];
                    let _ = app.emit("cleaner-dup-groups-batch", slice);
                    start = end;
                    if TEMP_CANCEL.load(Ordering::Relaxed) {
                        break;
                    }
                }
                let _ = app.emit("cleaner-done", serde_json::json!({"scope":"duplicate"}));
            }
            Err(e) => {
                let _ = app.emit("cleaner-error", format!("duplicate: {}", e));
            }
        }
    });
    Ok(())
}

#[tauri::command]
pub fn start_empty_scan(app_handle: AppHandle) -> Result<(), String> {
    let app = app_handle.clone();
    TEMP_CANCEL.store(false, Ordering::Relaxed);
    std::thread::spawn(move || {
        let _ = app.emit("cleaner-progress", "Scanning empty folders...");
        match find_empty_folders(app.clone()) {
            Ok(list) => {
                emit_chunked_strings(&app, "cleaner-empty-batch", &list, 500);
                let _ = app.emit("cleaner-done", serde_json::json!({"scope":"empty"}));
            }
            Err(e) => {
                let _ = app.emit("cleaner-error", format!("empty: {}", e));
            }
        }
    });
    Ok(())
}

#[tauri::command]
pub fn start_shortcut_scan(app_handle: AppHandle) -> Result<(), String> {
    let app = app_handle.clone();
    TEMP_CANCEL.store(false, Ordering::Relaxed);
    std::thread::spawn(move || {
        let _ = app.emit("cleaner-progress", "Scanning broken shortcuts...");
        match find_broken_shortcuts(app.clone()) {
            Ok(list) => {
                emit_chunked_strings(&app, "cleaner-shortcut-batch", &list, 500);
                let _ = app.emit("cleaner-done", serde_json::json!({"scope":"shortcuts"}));
            }
            Err(e) => {
                let _ = app.emit("cleaner-error", format!("shortcuts: {}", e));
            }
        }
    });
    Ok(())
}

fn _delete_files_helper(files: Vec<String>) -> Result<usize, String> {
    let mut deleted_count = 0;
    let mut errors: Vec<String> = Vec::new();
    for file_path_str in files {
        let path = PathBuf::from(&file_path_str);
        if path.exists() {
            if path.is_file() {
                match fs::remove_file(&path) {
                    Ok(_) => deleted_count += 1,
                    Err(e) => {
                        let msg = format!("Failed to delete file {}: {}", file_path_str, e);
                        eprintln!("{}", msg);
                        errors.push(msg);
                    }
                }
            } else if path.is_dir() {
                match fs::remove_dir_all(&path) {
                    Ok(_) => deleted_count += 1,
                    Err(e) => {
                        let msg = format!("Failed to delete directory {}: {}", file_path_str, e);
                        eprintln!("{}", msg);
                        errors.push(msg);
                    }
                }
            }
        }
    }
    if errors.is_empty() {
        Ok(deleted_count)
    } else {
        Err(errors.join("; "))
    }
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

#[tauri::command]
#[cfg(target_os = "windows")]
pub fn quick_clear_user_temp() -> Result<CleanupStats, String> {
    let user_temp = env::temp_dir();
    clear_directory_contents(&user_temp)
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub fn quick_clear_system_temp() -> Result<CleanupStats, String> {
    if let Some(windir) = env::var_os("WINDIR") {
        let p = PathBuf::from(windir).join("Temp");
        return clear_directory_contents(&p);
    }
    Err("WINDIR not set".into())
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub fn quick_clear_prefetch() -> Result<CleanupStats, String> {
    if let Some(windir) = env::var_os("WINDIR") {
        let p = PathBuf::from(windir).join("Prefetch");
        return clear_directory_contents(&p);
    }
    Err("WINDIR not set".into())
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub fn quick_clear_recent() -> Result<CleanupStats, String> {
    if let Some(appdata) = env::var_os("APPDATA") {
        let recent = PathBuf::from(appdata).join("Microsoft/Windows/Recent");
        if !recent.exists() || !recent.is_dir() {
            return Ok(CleanupStats::default());
        }
        let mut stats = CleanupStats::default();
        for entry in WalkDir::new(&recent)
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if entry.file_type().is_file() {
                if let Some(ext) = path.extension() {
                    if ext.eq_ignore_ascii_case("lnk") {
                        if let Ok(meta) = fs::metadata(path) {
                            if fs::remove_file(path).is_ok() {
                                stats.files_deleted += 1;
                                stats.bytes_deleted += meta.len();
                            }
                        } else {
                            let _ = fs::remove_file(path);
                        }
                    }
                }
            }
        }
        return Ok(stats);
    }
    Err("APPDATA not set".into())
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn quick_clear_user_temp() -> Result<CleanupStats, String> {
    Err("Only on Windows".into())
}
#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn quick_clear_system_temp() -> Result<CleanupStats, String> {
    Err("Only on Windows".into())
}
#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn quick_clear_prefetch() -> Result<CleanupStats, String> {
    Err("Only on Windows".into())
}
#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn quick_clear_recent() -> Result<CleanupStats, String> {
    Err("Only on Windows".into())
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
    let min_size_bytes: u64 = 100 * 1024 * 1024;
    find_large_files_min(min_size_bytes, app_handle)
}

#[tauri::command]
pub fn find_large_files_min(
    min_size_bytes: u64,
    app_handle: tauri::AppHandle,
) -> Result<Vec<(String, u64)>, String> {
    let wp = crate::paths::WindowsPaths::get();
    let user_profile = wp.user_profile;
    if user_profile.as_os_str().is_empty() {
        return Err("USERPROFILE not found".to_string());
    }
    let downloads_path = user_profile.join("Downloads");
    let desktop_path = user_profile.join("Desktop");
    let documents_path = user_profile.join("Documents");
    let paths_to_scan = vec![downloads_path, desktop_path, documents_path];

    let scanned_count = std::sync::atomic::AtomicUsize::new(0);

    let results: Vec<(String, u64)> = paths_to_scan
        .into_iter()
        .flat_map(|root| {
            walkdir::WalkDir::new(root)
                .into_iter()
                .filter_map(|e| e.ok())
        })
        .par_bridge()
        .filter_map(|entry| {
            if TEMP_CANCEL.load(Ordering::Relaxed) {
                return None;
            }

            let c = scanned_count.fetch_add(1, Ordering::Relaxed);
            if c % 1000 == 0 {
                let _ = app_handle.emit(
                    "scan_progress",
                    format!("Scanned {} files for large files...", c),
                );
            }

            if entry.file_type().is_file() {
                let p = entry.path();
                if let Ok(meta) = fs::metadata(p) {
                    if meta.is_file() && meta.len() >= min_size_bytes {
                        return Some((p.display().to_string(), meta.len()));
                    }
                }
            }
            None
        })
        .collect();

    Ok(results)
}

#[tauri::command]
pub fn find_large_files_top(
    k: Option<usize>,
    min_size_bytes: Option<u64>,
    app_handle: tauri::AppHandle,
) -> Result<Vec<(String, u64)>, String> {
    let k = k.unwrap_or(1000).clamp(100, 10_000);
    let min_size = min_size_bytes.unwrap_or(100 * 1024 * 1024);

    let user_profile = env::var_os("USERPROFILE").ok_or("USERPROFILE not found".to_string())?;
    let downloads_path = PathBuf::from(&user_profile).join("Downloads");
    let desktop_path = PathBuf::from(&user_profile).join("Desktop");
    let documents_path = PathBuf::from(&user_profile).join("Documents");
    let paths_to_scan = vec![downloads_path, desktop_path, documents_path];

    let mut files: Vec<PathBuf> = Vec::new();
    let mut scanned_count = 0usize;
    let enum_cap: usize = 300_000;
    for scan_path in paths_to_scan {
        if scan_path.exists() && scan_path.is_dir() {
            for entry in walkdir::WalkDir::new(&scan_path)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if files.len() >= enum_cap {
                    break;
                }
                if entry.file_type().is_file() {
                    files.push(entry.path().to_path_buf());
                }
                scanned_count += 1;
                if scanned_count % 1000 == 0 {
                    let _ = app_handle.emit(
                        "scan_progress",
                        format!("Scanned {} entries...", scanned_count),
                    );
                }
            }
        }
    }

    let sized: Vec<(u64, String)> = files
        .par_iter()
        .filter_map(|p| {
            fs::metadata(p)
                .ok()
                .map(|m| (m.len(), p.display().to_string()))
        })
        .filter(|(len, _)| *len >= min_size)
        .collect();

    let mut heap: BinaryHeap<Reverse<(u64, String)>> = BinaryHeap::with_capacity(k + 1);
    for (len, path) in sized.into_iter() {
        if heap.len() < k {
            heap.push(Reverse((len, path)));
        } else if let Some(mut smallest) = heap.peek_mut() {
            if len > (smallest.0).0 {
                *smallest = Reverse((len, path));
            }
        }
    }
    let mut out: Vec<(String, u64)> = heap
        .into_iter()
        .map(|Reverse((len, path))| (path, len))
        .collect();
    out.sort_by(|a, b| b.1.cmp(&a.1));
    Ok(out)
}

#[tauri::command]
pub fn find_duplicate_files(app_handle: tauri::AppHandle) -> Result<Vec<(String, u64)>, String> {
    let mut scanned_count = 0usize;
    let user_profile = env::var_os("USERPROFILE").ok_or("USERPROFILE not found".to_string())?;
    let downloads_path = PathBuf::from(&user_profile).join("Downloads");
    let documents_path = PathBuf::from(&user_profile).join("Documents");
    let paths_to_scan = vec![downloads_path, documents_path];
    let mut files: Vec<PathBuf> = Vec::new();
    for scan_path in &paths_to_scan {
        if scan_path.exists() && scan_path.is_dir() {
            for entry in WalkDir::new(&scan_path).into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_file() {
                    files.push(entry.path().to_path_buf());
                }
                scanned_count += 1;
                if scanned_count % 500 == 0 {
                    let _ = app_handle.emit(
                        "scan_progress",
                        format!("Scanned {} files...", scanned_count),
                    );
                }
            }
        }
    }

    let sized: Vec<(u64, String)> = files
        .par_iter()
        .filter_map(|p| {
            fs::metadata(p)
                .ok()
                .map(|m| (m.len(), p.display().to_string()))
        })
        .collect();
    let mut size_buckets: HashMap<u64, Vec<String>> = HashMap::new();
    for (sz, p) in sized {
        size_buckets.entry(sz).or_default().push(p);
    }

    let mut file_hashes: HashMap<String, Vec<String>> = HashMap::new();
    for (_size, group) in size_buckets.into_iter().filter(|(_, v)| v.len() > 1) {
        let hashed: Vec<(String, String)> = group
            .par_iter()
            .filter_map(|p| {
                let path = PathBuf::from(p);
                match calculate_file_hash(&path) {
                    Ok(h) => Some((h, p.clone())),
                    Err(_) => None,
                }
            })
            .collect();
        for (h, p) in hashed {
            file_hashes.entry(h).or_default().push(p);
        }
    }

    let mut out = Vec::new();
    for (_h, paths) in file_hashes.into_iter().filter(|(_, v)| v.len() > 1) {
        if let Some(first) = paths.first() {
            if let Ok(meta) = fs::metadata(first) {
                for p in paths {
                    out.push((p, meta.len()));
                }
            }
        }
    }
    Ok(out)
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct DuplicateGroup {
    pub hash: String,
    pub size: u64,
    pub files: Vec<String>,
}

fn calculate_partial_hash(path: &Path) -> Result<String, io::Error> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 16384]; // 16KB prefix

    let bytes_read = file.read(&mut buffer)?;
    if bytes_read > 0 {
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

#[tauri::command]
pub fn find_duplicate_groups(app_handle: tauri::AppHandle) -> Result<Vec<DuplicateGroup>, String> {
    let first_pass_limit: usize = 200_000;
    let hash_limit: usize = 15_000;

    let wp = crate::paths::WindowsPaths::get();
    let user_profile = wp.user_profile;
    if user_profile.as_os_str().is_empty() {
        return Err("USERPROFILE not found".to_string());
    }
    let downloads_path = user_profile.join("Downloads");
    let documents_path = user_profile.join("Documents");
    let desktop_path = user_profile.join("Desktop");
    let paths_to_scan = vec![downloads_path, documents_path, desktop_path];

    let scanned_count = std::sync::atomic::AtomicUsize::new(0);
    let size_buckets: DashMap<u64, Vec<String>> = DashMap::new();

    paths_to_scan
        .into_iter()
        .flat_map(|root| {
            walkdir::WalkDir::new(root)
                .into_iter()
                .filter_map(|e| e.ok())
        })
        .par_bridge()
        .for_each(|entry| {
            if TEMP_CANCEL.load(Ordering::Relaxed) {
                return;
            }

            let current = scanned_count.fetch_add(1, Ordering::Relaxed);
            if current >= first_pass_limit {
                return;
            }

            if current % 1000 == 0 {
                let _ = app_handle.emit("scan_progress", format!("Scanned {} files...", current));
            }

            if entry.file_type().is_file() {
                let p = entry.path();
                if let Ok(meta) = fs::metadata(p) {
                    let len = meta.len();
                    if len > 0 {
                        size_buckets
                            .entry(len)
                            .or_default()
                            .push(p.display().to_string());
                    }
                }
            }
        });

    if TEMP_CANCEL.load(Ordering::Relaxed) {
        return Ok(Vec::new());
    }

    // Second pass: Partial hash (first 16KB)
    let mut partial_hash_buckets: HashMap<String, Vec<String>> = HashMap::new();
    let potential_dupes: Vec<(u64, Vec<String>)> = size_buckets
        .into_iter()
        .filter(|(_, v)| v.len() > 1)
        .collect();

    for (size, group) in potential_dupes {
        let partial_hashes: Vec<(String, String)> = group
            .par_iter()
            .filter_map(|p| {
                if TEMP_CANCEL.load(Ordering::Relaxed) {
                    return None;
                }
                let path = PathBuf::from(p);
                match calculate_partial_hash(&path) {
                    Ok(h) => Some((format!("{}_{}", size, h), p.clone())),
                    Err(_) => None,
                }
            })
            .collect();

        for (h, p) in partial_hashes {
            partial_hash_buckets.entry(h).or_default().push(p);
        }
    }

    // Third pass: Full hash
    let mut file_hashes: HashMap<String, Vec<String>> = HashMap::new();
    let mut hashed_count = 0usize;

    let final_potential_dupes: Vec<Vec<String>> = partial_hash_buckets
        .into_iter()
        .filter(|(_, v)| v.len() > 1)
        .map(|(_, v)| v)
        .collect();

    for group in final_potential_dupes {
        if hashed_count >= hash_limit {
            break;
        }

        let remaining = hash_limit.saturating_sub(hashed_count);
        let batch: Vec<String> = group.into_iter().take(remaining).collect();

        let hashed_batch: Vec<(String, String)> = batch
            .par_iter()
            .filter_map(|p| {
                if TEMP_CANCEL.load(Ordering::Relaxed) {
                    return None;
                }
                let path = PathBuf::from(p);
                match calculate_file_hash(&path) {
                    Ok(h) => Some((h, p.clone())),
                    Err(_) => None,
                }
            })
            .collect();

        hashed_count += hashed_batch.len();
        for (h, p) in hashed_batch {
            file_hashes.entry(h).or_default().push(p);
        }

        if TEMP_CANCEL.load(Ordering::Relaxed) {
            break;
        }
    }

    let mut out: Vec<DuplicateGroup> = Vec::new();
    for (hash, files) in file_hashes.into_iter().filter(|(_, v)| v.len() > 1) {
        let size = files
            .first()
            .and_then(|p| fs::metadata(p).ok())
            .map(|m| m.len())
            .unwrap_or(0);
        out.push(DuplicateGroup { hash, size, files });
    }
    Ok(out)
}

#[tauri::command]
pub fn move_to_trash(files: Vec<String>) -> Result<usize, String> {
    let mut count = 0usize;
    for file in files {
        match trash::delete(&file) {
            Ok(_) => count += 1,
            Err(e) => eprintln!("Failed to move to trash {}: {}", file, e),
        }
    }
    Ok(count)
}

fn calculate_file_hash(path: &Path) -> Result<String, io::Error> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 65536];

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
    let mut scanned_count = 0usize;

    let wp = crate::paths::WindowsPaths::get();
    let user_profile = wp.user_profile;
    if user_profile.as_os_str().is_empty() {
        return Err("USERPROFILE not found".to_string());
    }
    let downloads_path = user_profile.join("Downloads");
    let desktop_path = user_profile.join("Desktop");
    let documents_path = user_profile.join("Documents");
    let paths_to_scan = vec![downloads_path, desktop_path, documents_path];

    let mut dirs: Vec<PathBuf> = Vec::new();
    for scan_path in paths_to_scan {
        if scan_path.exists() && scan_path.is_dir() {
            for entry in WalkDir::new(&scan_path).into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_dir() {
                    dirs.push(entry.path().to_path_buf());
                }
                scanned_count += 1;
                if scanned_count % 500 == 0 {
                    let _ = app_handle.emit(
                        "scan_progress",
                        format!("Scanned {} directories for empty folders...", scanned_count),
                    );
                }
            }
        }
    }
    let results: Vec<String> = dirs
        .par_iter()
        .filter_map(|d| {
            if let Ok(mut rd) = fs::read_dir(d) {
                if rd.next().is_none() {
                    return Some(d.display().to_string());
                }
            }
            None
        })
        .collect();
    Ok(results)
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub fn find_broken_shortcuts(app_handle: tauri::AppHandle) -> Result<Vec<String>, String> {
    let mut scanned_count = 0usize;

    let wp = crate::paths::WindowsPaths::get();
    let user_profile = wp.user_profile;
    if user_profile.as_os_str().is_empty() {
        return Err("USERPROFILE not found".to_string());
    }
    let downloads_path = user_profile.join("Downloads");
    let desktop_path = user_profile.join("Desktop");
    let documents_path = user_profile.join("Documents");

    let mut paths_to_scan = vec![downloads_path, desktop_path, documents_path];

    if !wp.app_data.as_os_str().is_empty() {
        paths_to_scan.push(wp.app_data.join("Microsoft/Windows/Start Menu"));
    }
    if !wp.program_data.as_os_str().is_empty() {
        paths_to_scan.push(wp.program_data.join("Microsoft/Windows/Start Menu"));
    }
    let public = env::var_os("PUBLIC").map(PathBuf::from);
    if let Some(p) = public {
        paths_to_scan.push(p.join("Desktop"));
    }

    let mut lnk_files: Vec<PathBuf> = Vec::new();
    for scan_path in paths_to_scan {
        if scan_path.exists() && scan_path.is_dir() {
            for entry in WalkDir::new(&scan_path).into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_file() {
                    let path = entry.path();
                    if let Some(ext) = path.extension() {
                        if ext.eq_ignore_ascii_case("lnk") {
                            lnk_files.push(path.to_path_buf());
                        }
                    }
                }
                scanned_count += 1;
                if scanned_count % 400 == 0 {
                    let _ = app_handle.emit(
                        "scan_progress",
                        format!("Scanned {} files for broken shortcuts...", scanned_count),
                    );
                }
            }
        }
    }

    let _ = app_handle.emit(
        "scan_progress",
        format!("Found {} shortcuts. Verifying...", lnk_files.len()),
    );

    let results: Vec<String> = lnk_files
        .par_iter()
        .filter_map(|p| {
            if TEMP_CANCEL.load(Ordering::Relaxed) {
                return None;
            }
            match ShellLink::open(p, WINDOWS_1252) {
                Ok(shell_link) => {
                    if let Some(link_info) = shell_link.link_info() {
                        let common_path_str = link_info.common_path_suffix();
                        let target_path = PathBuf::from(common_path_str.to_string());

                        if !target_path.exists() {
                            if common_path_str.contains('\\') || common_path_str.contains('/') {
                                return Some(p.display().to_string());
                            }
                        }
                    }
                }
                Err(_) => {}
            }
            None
        })
        .collect();

    Ok(results)
}

#[tauri::command]
pub fn move_files(files: Vec<String>, destination: String) -> Result<usize, String> {
    let dest = PathBuf::from(&destination);
    if !dest.exists() {
        fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
    }
    if !dest.is_dir() {
        return Err("Destination is not a directory".into());
    }
    let mut moved = 0usize;
    for f in files {
        let from = PathBuf::from(&f);
        if !from.exists() || !from.is_file() {
            continue;
        }
        let file_name = match from.file_name() {
            Some(n) => n,
            None => continue,
        };
        let mut to = dest.join(file_name);
        if to.exists() {
            let mut idx = 1u32;
            let stem = to
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("file")
                .to_string();
            let ext = to.extension().and_then(|s| s.to_str()).unwrap_or("");
            loop {
                let candidate = if ext.is_empty() {
                    dest.join(format!("{} ({})", stem, idx))
                } else {
                    dest.join(format!("{} ({}).{}", stem, idx, ext))
                };
                if !candidate.exists() {
                    to = candidate;
                    break;
                }
                idx += 1;
                if idx > 9999 {
                    break;
                }
            }
        }
        if fs::rename(&from, &to).is_ok() {
            moved += 1;
            continue;
        }
        match fs::copy(&from, &to) {
            Ok(_) => {
                let _ = fs::remove_file(&from);
                moved += 1;
            }
            Err(e) => eprintln!("copy failed {} -> {}: {}", from.display(), to.display(), e),
        }
    }
    Ok(moved)
}

#[tauri::command]
pub fn stat_paths(paths: Vec<String>) -> Result<Vec<(String, u64)>, String> {
    let out: Vec<(String, u64)> = paths
        .par_iter()
        .map(|p| {
            let pb = PathBuf::from(p);
            if let Ok(meta) = fs::metadata(&pb) {
                if meta.is_file() {
                    return (p.clone(), meta.len());
                }
            }
            (p.clone(), 0)
        })
        .collect();
    Ok(out)
}
