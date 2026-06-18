use crate::AppError;
use dashmap::DashMap;
use once_cell::sync::Lazy;
use rayon::prelude::*;
use serde_json::json;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use tauri::{AppHandle, Emitter};
use walkdir::WalkDir;

static TEMP_CANCEL: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));
use sysinfo::Disks;

struct ExclusionFilter {
    custom_patterns: Vec<String>,
}

impl ExclusionFilter {
    fn new(custom_exclusions: Option<Vec<String>>) -> Self {
        let custom_patterns = custom_exclusions
            .unwrap_or_default()
            .into_iter()
            .map(|s| s.trim().to_lowercase().replace('\\', "/"))
            .filter(|s| !s.is_empty())
            .collect();
        Self { custom_patterns }
    }

    fn should_exclude_path(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy().to_lowercase().replace('\\', "/");
        for pattern in &self.custom_patterns {
            if path_str.contains(pattern) {
                return true;
            }
        }
        false
    }

    fn should_prune_dir(&self, entry: &walkdir::DirEntry) -> bool {
        if !entry.file_type().is_dir() {
            return false;
        }

        if let Some(name) = entry.file_name().to_str() {
            let name_lower = name.to_lowercase();
            let is_default_ignored = name_lower == ".git"
                || name_lower == "node_modules"
                || name_lower == "target"
                || name_lower == ".venv"
                || name_lower == "venv"
                || name_lower == "env"
                || name_lower == "__pycache__"
                || name_lower == ".idea"
                || name_lower == ".vscode"
                || name_lower == "bin"
                || name_lower == "obj"
                || name_lower == "dist"
                || name_lower == "build"
                || name_lower == "out"
                || name_lower == "vendor"
                || name_lower == ".next"
                || name_lower == ".svelte-kit"
                || name_lower == ".nuxt"
                || name_lower == ".cargo"
                || name_lower == ".gradle"
                || name_lower == "bower_components";

            if is_default_ignored {
                return true;
            }
        }

        self.should_exclude_path(entry.path())
    }

    fn should_exclude_file(&self, entry: &walkdir::DirEntry) -> bool {
        if !entry.file_type().is_file() {
            return false;
        }
        self.should_exclude_path(entry.path())
    }
}

use lnk::ShellLink;
use lnk::encoding::WINDOWS_1252;
use sha2::{Digest, Sha256};
use std::cmp::Reverse;
use std::collections::BinaryHeap;

#[cfg(target_os = "windows")]
use windows::Win32::UI::Shell::{
    SHERB_NOCONFIRMATION, SHERB_NOPROGRESSUI, SHERB_NOSOUND, SHEmptyRecycleBinA,
};

#[derive(serde::Serialize, serde::Deserialize, Default, Clone)]
pub struct CleanupStats {
    pub files_deleted: u64,
    pub bytes_deleted: u64,
}

fn clear_directory_contents(root: &Path) -> Result<CleanupStats, AppError> {
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
pub async fn get_temp_files(
    app_handle: AppHandle,
    exclusions: Option<Vec<String>>,
) -> Result<Vec<String>, AppError> {
    let mut temp_files = Vec::new();
    let mut scanned_count = 0;

    let wp = crate::paths::WindowsPaths::get();
    let common_temp_paths: Vec<PathBuf> = vec![
        wp.temp,
        wp.local_app_data.join("Temp"),
        PathBuf::from("/tmp"),
        PathBuf::from("/private/tmp"),
        PathBuf::from("/var/tmp"),
    ]
    .into_iter()
    .filter(|p| p.exists() && p.is_dir())
    .collect();

    let filter = Arc::new(ExclusionFilter::new(exclusions));

    for temp_dir_path in common_temp_paths {
        let filter_clone = Arc::clone(&filter);
        let filter_clone2 = Arc::clone(&filter);
        for entry in WalkDir::new(&temp_dir_path)
            .into_iter()
            .filter_entry(move |e| !filter_clone.should_prune_dir(e))
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                if filter_clone2.should_exclude_file(&entry) {
                    continue;
                }
                temp_files.push(entry.path().display().to_string());
                scanned_count += 1;
                if scanned_count % 100 == 0 {
                    app_handle.emit(
                        "scan_progress",
                        format!("Scanned {} temporary files...", scanned_count),
                    )?;
                }
            }
        }
    }

    Ok(temp_files)
}

#[tauri::command]
pub async fn get_temp_files_stream(
    app_handle: AppHandle,
    batch_size: Option<usize>,
    max: Option<usize>,
    exclusions: Option<Vec<String>>,
) -> Result<usize, AppError> {
    let bs = batch_size.unwrap_or(250).max(50).min(1000);
    let limit = max.unwrap_or(500_000);
    TEMP_CANCEL.store(false, Ordering::Relaxed);

    let wp = crate::paths::WindowsPaths::get();
    let mut roots: Vec<PathBuf> = Vec::new();

    roots.push(wp.temp.clone());
    roots.push(wp.local_app_data.join("Temp"));

    roots.push(PathBuf::from("/tmp"));
    roots.push(PathBuf::from("/private/tmp"));
    roots.push(PathBuf::from("/var/tmp"));

    let local_app_data = &wp.local_app_data;

    #[cfg(target_os = "windows")]
    {
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
    }
    #[cfg(target_os = "macos")]
    {
        roots.push(local_app_data.join("Google/Chrome/Default/Cache"));
        roots.push(local_app_data.join("Google/Chrome/Default/Code Cache"));
        roots.push(local_app_data.join("Microsoft Edge/Default/Cache"));
        roots.push(local_app_data.join("Microsoft Edge/Default/Code Cache"));

        let firefox_profiles = local_app_data.join("Firefox/Profiles");
        if firefox_profiles.exists() {
            if let Ok(entries) = fs::read_dir(firefox_profiles) {
                for entry in entries.filter_map(|e| e.ok()) {
                    roots.push(entry.path().join("cache2"));
                    roots.push(entry.path().join("jumpListCache"));
                }
            }
        }
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))] // Linux/Unix
    {
        roots.push(local_app_data.join("google-chrome/Default/Cache"));
        roots.push(local_app_data.join("google-chrome/Default/Code Cache"));
        roots.push(local_app_data.join("microsoft-edge/Default/Cache"));
        roots.push(local_app_data.join("microsoft-edge/Default/Code Cache"));

        let firefox_profiles = local_app_data.join("mozilla/firefox");
        if firefox_profiles.exists() {
            if let Ok(entries) = fs::read_dir(firefox_profiles) {
                for entry in entries.filter_map(|e| e.ok()) {
                    roots.push(entry.path().join("cache2"));
                    roots.push(entry.path().join("jumpListCache"));
                }
            }
        }
    }

    roots.push(local_app_data.join("Microsoft/Windows/WER"));

    if !wp.windir.as_os_str().is_empty() {
        roots.push(wp.windir.join("Logs"));
        roots.push(wp.windir.join("SoftwareDistribution/Download"));
        roots.push(wp.windir.join("Minidump"));
    }

    roots.sort();
    roots.dedup();

    let active_roots: Vec<PathBuf> = roots
        .into_iter()
        .filter(|p| p.exists() && p.is_dir())
        .collect();

    let (tx, rx) = mpsc::channel::<(String, u64)>();
    let scanned_total = std::sync::atomic::AtomicUsize::new(0);

    let filter = Arc::new(ExclusionFilter::new(exclusions));

    active_roots.into_par_iter().for_each(|root| {
        let txc = tx.clone();
        let ah = app_handle.clone();
        let filter_clone = Arc::clone(&filter);
        let filter_clone2 = Arc::clone(&filter);

        for entry in WalkDir::new(&root)
            .into_iter()
            .filter_entry(move |e| !filter_clone.should_prune_dir(e))
            .filter_map(|e| e.ok())
        {
            if TEMP_CANCEL.load(Ordering::Relaxed) {
                break;
            }
            if entry.file_type().is_file() {
                if filter_clone2.should_exclude_file(&entry) {
                    continue;
                }
                let size = entry.metadata().ok().map(|m| m.len()).unwrap_or(0);
                let _ = txc.send((entry.path().display().to_string(), size));
            }

            let c = scanned_total.fetch_add(1, Ordering::Relaxed);
            if c % 2000 == 0 {
                let _ = ah.emit(
                    "scan_progress",
                    format!(
                        "Scanned {} entries in {}...",
                        c,
                        root.file_name().and_then(|n| n.to_str()).unwrap_or("dir")
                    ),
                );
            }
        }
    });
    drop(tx);

    let mut total = 0usize;
    let mut batch: Vec<(String, u64)> = Vec::with_capacity(bs);
    while let Ok((p, s)) = rx.recv() {
        if TEMP_CANCEL.load(Ordering::Relaxed) {
            break;
        }
        total += 1;
        if total >= limit {
            TEMP_CANCEL.store(true, Ordering::Relaxed);
        }
        batch.push((p, s));
        if batch.len() >= bs {
            app_handle.emit("cleaner-temp-batch", &batch)?;
            batch.clear();
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        if total >= limit {
            break;
        }
    }
    if !batch.is_empty() {
        app_handle.emit("cleaner-temp-batch", &batch)?;
    }

    app_handle.emit("cleaner-temp-done", json!({"total": total}))?;
    Ok(total)
}

#[tauri::command]
pub fn cancel_temp_scan() -> Result<(), AppError> {
    TEMP_CANCEL.store(true, Ordering::Relaxed);
    Ok(())
}

fn emit_chunked_pairs(
    app: &AppHandle,
    event: &str,
    items: &[(String, u64)],
    chunk: usize,
) -> Result<(), AppError> {
    if items.is_empty() {
        return Ok(());
    }
    for slice in items.chunks(chunk) {
        app.emit(event, slice)?;
        std::thread::sleep(std::time::Duration::from_millis(5));
        if TEMP_CANCEL.load(Ordering::Relaxed) {
            break;
        }
    }
    Ok(())
}

fn emit_chunked_strings(
    app: &AppHandle,
    event: &str,
    items: &[String],
    chunk: usize,
) -> Result<(), AppError> {
    if items.is_empty() {
        return Ok(());
    }
    for slice in items.chunks(chunk) {
        app.emit(event, slice)?;
        std::thread::sleep(std::time::Duration::from_millis(5));
        if TEMP_CANCEL.load(Ordering::Relaxed) {
            break;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn start_cleaner_scan(
    app_handle: AppHandle,
    min_size_bytes: Option<u64>,
    max_temp: Option<usize>,
    exclusions: Option<Vec<String>>,
) -> Result<(), AppError> {
    let app = app_handle.clone();
    TEMP_CANCEL.store(false, Ordering::Relaxed);
    let exclusions_clone = exclusions.clone();
    tokio::spawn(async move {
        let _ = app.emit("cleaner-progress", "Starting scan...");
        let _ = app.emit("cleaner-progress", "Scanning temporary files...");
        let _ = get_temp_files_stream(
            app.clone(),
            Some(250),
            Some(max_temp.unwrap_or(500_000)),
            exclusions_clone.clone(),
        )
        .await;

        if TEMP_CANCEL.load(Ordering::Relaxed) {
            let _ = app.emit("cleaner-stopped", json!({"scope":"temp"}));
            return;
        }

        let min = min_size_bytes.unwrap_or(100 * 1024 * 1024);
        let _ = app.emit("cleaner-progress", "Scanning large files...");
        match find_large_files_min(min, app.clone(), exclusions_clone.clone()).await {
            Ok(list) => {
                let _ = emit_chunked_pairs(&app, "cleaner-large-batch", &list, 300);
                let _ = app.emit("cleaner-done", json!({"scope":"large"}));
            }
            Err(e) => {
                let _ = app.emit("cleaner-error", format!("large: {:?}", e));
            }
        }

        if TEMP_CANCEL.load(Ordering::Relaxed) {
            let _ = app.emit("cleaner-stopped", json!({"scope":"large"}));
            return;
        }

        let _ = app.emit("cleaner-progress", "Scanning duplicates...");
        match find_duplicate_groups(app.clone(), exclusions_clone.clone()).await {
            Ok(groups) => {
                for slice in groups.chunks(60) {
                    let _ = app.emit("cleaner-dup-groups-batch", slice);
                    if TEMP_CANCEL.load(Ordering::Relaxed) {
                        break;
                    }
                }
                let _ = app.emit("cleaner-done", json!({"scope":"duplicate"}));
            }
            Err(e) => {
                let _ = app.emit("cleaner-error", format!("duplicate: {:?}", e));
            }
        }

        if TEMP_CANCEL.load(Ordering::Relaxed) {
            let _ = app.emit("cleaner-stopped", json!({"scope":"duplicate"}));
            return;
        }

        let _ = app.emit("cleaner-progress", "Scanning empty folders...");
        match find_empty_folders(app.clone(), exclusions_clone.clone()).await {
            Ok(list) => {
                let _ = emit_chunked_strings(&app, "cleaner-empty-batch", &list, 500);
                let _ = app.emit("cleaner-done", json!({"scope":"empty"}));
            }
            Err(e) => {
                let _ = app.emit("cleaner-error", format!("empty: {:?}", e));
            }
        }

        if TEMP_CANCEL.load(Ordering::Relaxed) {
            let _ = app.emit("cleaner-stopped", json!({"scope":"empty"}));
            return;
        }

        let _ = app.emit("cleaner-progress", "Scanning broken shortcuts...");
        match find_broken_shortcuts(app.clone(), exclusions_clone).await {
            Ok(list) => {
                let _ = emit_chunked_strings(&app, "cleaner-shortcut-batch", &list, 500);
                let _ = app.emit("cleaner-done", json!({"scope":"shortcuts"}));
            }
            Err(e) => {
                let _ = app.emit("cleaner-error", format!("shortcuts: {:?}", e));
            }
        }

        let _ = app.emit("cleaner-done", json!({"scope":"all"}));
    });
    Ok(())
}

#[tauri::command]
pub fn cancel_cleaner_scan() -> Result<(), AppError> {
    TEMP_CANCEL.store(true, Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
pub async fn start_large_scan(
    app_handle: AppHandle,
    min_size_bytes: Option<u64>,
    exclusions: Option<Vec<String>>,
) -> Result<(), AppError> {
    let app = app_handle.clone();
    TEMP_CANCEL.store(false, Ordering::Relaxed);
    tokio::spawn(async move {
        let min = min_size_bytes.unwrap_or(100 * 1024 * 1024);
        let _ = app.emit("cleaner-progress", "Scanning large files...");
        match find_large_files_min(min, app.clone(), exclusions).await {
            Ok(list) => {
                let _ = emit_chunked_pairs(&app, "cleaner-large-batch", &list, 300);
                let _ = app.emit("cleaner-done", json!({"scope":"large"}));
            }
            Err(e) => {
                let _ = app.emit("cleaner-error", format!("large: {:?}", e));
            }
        }
    });
    Ok(())
}

#[tauri::command]
pub async fn start_duplicate_groups_scan(
    app_handle: AppHandle,
    exclusions: Option<Vec<String>>,
) -> Result<(), AppError> {
    let app = app_handle.clone();
    TEMP_CANCEL.store(false, Ordering::Relaxed);
    tokio::spawn(async move {
        let _ = app.emit("cleaner-progress", "Scanning duplicates...");
        match find_duplicate_groups(app.clone(), exclusions).await {
            Ok(groups) => {
                for slice in groups.chunks(60) {
                    let _ = app.emit("cleaner-dup-groups-batch", slice);
                    if TEMP_CANCEL.load(Ordering::Relaxed) {
                        break;
                    }
                }
                let _ = app.emit("cleaner-done", json!({"scope":"duplicate"}));
            }
            Err(e) => {
                let _ = app.emit("cleaner-error", format!("duplicate: {:?}", e));
            }
        }
    });
    Ok(())
}

#[tauri::command]
pub async fn start_empty_scan(
    app_handle: AppHandle,
    exclusions: Option<Vec<String>>,
) -> Result<(), AppError> {
    let app = app_handle.clone();
    TEMP_CANCEL.store(false, Ordering::Relaxed);
    tokio::spawn(async move {
        let _ = app.emit("cleaner-progress", "Scanning empty folders...");
        match find_empty_folders(app.clone(), exclusions).await {
            Ok(list) => {
                let _ = emit_chunked_strings(&app, "cleaner-empty-batch", &list, 500);
                let _ = app.emit("cleaner-done", json!({"scope":"empty"}));
            }
            Err(e) => {
                let _ = app.emit("cleaner-error", format!("empty: {:?}", e));
            }
        }
    });
    Ok(())
}

#[tauri::command]
pub async fn start_shortcut_scan(
    app_handle: AppHandle,
    exclusions: Option<Vec<String>>,
) -> Result<(), AppError> {
    let app = app_handle.clone();
    TEMP_CANCEL.store(false, Ordering::Relaxed);
    tokio::spawn(async move {
        let _ = app.emit("cleaner-progress", "Scanning broken shortcuts...");
        match find_broken_shortcuts(app.clone(), exclusions).await {
            Ok(list) => {
                let _ = emit_chunked_strings(&app, "cleaner-shortcut-batch", &list, 500);
                let _ = app.emit("cleaner-done", json!({"scope":"shortcuts"}));
            }
            Err(e) => {
                let _ = app.emit("cleaner-error", format!("shortcuts: {:?}", e));
            }
        }
    });
    Ok(())
}

fn _delete_files_helper(files: Vec<String>) -> Result<usize, AppError> {
    let mut deleted_count = 0;
    let mut errors: Vec<String> = Vec::new();
    for file_path_str in files {
        let path = PathBuf::from(&file_path_str);
        if path.exists() {
            if path.is_file() {
                match fs::remove_file(&path) {
                    Ok(_) => deleted_count += 1,
                    Err(e) => {
                        errors.push(format!("Failed to delete file {}: {}", file_path_str, e));
                    }
                }
            } else if path.is_dir() {
                match fs::remove_dir_all(&path) {
                    Ok(_) => deleted_count += 1,
                    Err(e) => {
                        errors.push(format!(
                            "Failed to delete directory {}: {}",
                            file_path_str, e
                        ));
                    }
                }
            }
        }
    }
    if errors.is_empty() {
        Ok(deleted_count)
    } else {
        Err(AppError::System(errors.join("; ")))
    }
}

#[tauri::command]
pub async fn clean_temp_files(files: Vec<String>) -> Result<usize, AppError> {
    _delete_files_helper(files)
}

#[tauri::command]
pub async fn delete_files(files: Vec<String>) -> Result<usize, AppError> {
    _delete_files_helper(files)
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub async fn empty_recycle_bin() -> Result<(), AppError> {
    unsafe {
        let flags = SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND;
        let result = SHEmptyRecycleBinA(None, None, flags);
        if result.is_ok() {
            Ok(())
        } else {
            Err(AppError::System(format!(
                "Failed to empty recycle bin: {:?}",
                result
            )))
        }
    }
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub async fn empty_recycle_bin() -> Result<(), AppError> {
    Err(AppError::Internal(
        "Emptying recycle bin is only supported on Windows.".to_string(),
    ))
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub async fn quick_clear_user_temp() -> Result<CleanupStats, AppError> {
    let user_temp = env::temp_dir();
    clear_directory_contents(&user_temp)
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub async fn quick_clear_system_temp() -> Result<CleanupStats, AppError> {
    if let Some(windir) = env::var_os("WINDIR") {
        let p = PathBuf::from(windir).join("Temp");
        return clear_directory_contents(&p);
    }
    Err(AppError::System("WINDIR not set".into()))
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub async fn quick_clear_prefetch() -> Result<CleanupStats, AppError> {
    if let Some(windir) = env::var_os("WINDIR") {
        let p = PathBuf::from(windir).join("Prefetch");
        return clear_directory_contents(&p);
    }
    Err(AppError::System("WINDIR not set".into()))
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub async fn quick_clear_recent() -> Result<CleanupStats, AppError> {
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
    Err(AppError::System("APPDATA not set".into()))
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub async fn quick_clear_user_temp() -> Result<CleanupStats, AppError> {
    let mut stats = CleanupStats::default();

    // Clear user temp/cache directory
    let cache_dir = env::var_os("HOME").map(PathBuf::from).map(|h| {
        #[cfg(target_os = "macos")]
        {
            h.join("Library/Caches")
        }
        #[cfg(not(target_os = "macos"))]
        {
            h.join(".cache")
        }
    });

    if let Some(p) = cache_dir {
        if let Ok(s) = clear_directory_contents(&p) {
            stats.files_deleted += s.files_deleted;
            stats.bytes_deleted += s.bytes_deleted;
        }
    }

    // Also clear global temp
    let temp_dir = env::temp_dir();
    if let Ok(s) = clear_directory_contents(&temp_dir) {
        stats.files_deleted += s.files_deleted;
        stats.bytes_deleted += s.bytes_deleted;
    }

    Ok(stats)
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub async fn quick_clear_system_temp() -> Result<CleanupStats, AppError> {
    let mut stats = CleanupStats::default();
    for p in &["/var/tmp", "/tmp", "/private/tmp"] {
        let path = Path::new(p);
        if let Ok(s) = clear_directory_contents(path) {
            stats.files_deleted += s.files_deleted;
            stats.bytes_deleted += s.bytes_deleted;
        }
    }
    Ok(stats)
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub async fn quick_clear_prefetch() -> Result<CleanupStats, AppError> {
    Ok(CleanupStats::default())
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub async fn quick_clear_recent() -> Result<CleanupStats, AppError> {
    Ok(CleanupStats::default())
}

#[tauri::command]
pub async fn get_drive_info() -> Result<(u64, u64), AppError> {
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
pub async fn find_large_files(
    app_handle: AppHandle,
    exclusions: Option<Vec<String>>,
) -> Result<Vec<(String, u64)>, AppError> {
    let min_size_bytes: u64 = 100 * 1024 * 1024;
    find_large_files_min(min_size_bytes, app_handle, exclusions).await
}

#[tauri::command]
pub async fn find_large_files_min(
    min_size_bytes: u64,
    app_handle: AppHandle,
    exclusions: Option<Vec<String>>,
) -> Result<Vec<(String, u64)>, AppError> {
    let wp = crate::paths::WindowsPaths::get();
    let user_profile = wp.user_profile;
    if user_profile.as_os_str().is_empty() {
        return Err(AppError::System("USERPROFILE not found".to_string()));
    }

    let paths_to_scan = vec![
        user_profile.join("Downloads"),
        user_profile.join("Desktop"),
        user_profile.join("Documents"),
    ];

    let scanned_count = std::sync::atomic::AtomicUsize::new(0);
    let filter = Arc::new(ExclusionFilter::new(exclusions));

    let results: Vec<(String, u64)> = paths_to_scan
        .into_par_iter()
        .flat_map(|root| {
            let filter_clone = Arc::clone(&filter);
            WalkDir::new(root)
                .into_iter()
                .filter_entry(move |e| !filter_clone.should_prune_dir(e))
                .filter_map(|e| e.ok())
                .par_bridge()
        })
        .filter_map({
            let filter_clone = Arc::clone(&filter);
            move |entry| {
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
                    if filter_clone.should_exclude_file(&entry) {
                        return None;
                    }
                    if let Ok(meta) = entry.metadata() {
                        if meta.len() >= min_size_bytes {
                            return Some((entry.path().display().to_string(), meta.len()));
                        }
                    }
                }
                None
            }
        })
        .collect();

    Ok(results)
}

#[tauri::command]
pub async fn find_large_files_top(
    app_handle: AppHandle,
    top: usize,
    exclusions: Option<Vec<String>>,
) -> Result<Vec<(String, u64)>, AppError> {
    let k = top.max(1).min(10_000);
    let min_size = 100 * 1024 * 1024;

    let wp = crate::paths::WindowsPaths::get();
    if wp.user_profile.as_os_str().is_empty() {
        return Err(AppError::System("USERPROFILE not found".to_string()));
    }

    let paths_to_scan = vec![
        wp.user_profile.join("Downloads"),
        wp.user_profile.join("Desktop"),
        wp.user_profile.join("Documents"),
    ];

    let scanned_count = std::sync::atomic::AtomicUsize::new(0);
    let filter = Arc::new(ExclusionFilter::new(exclusions));

    let sized: Vec<(u64, String)> = paths_to_scan
        .into_par_iter()
        .flat_map(|root| {
            let filter_clone = Arc::clone(&filter);
            WalkDir::new(root)
                .into_iter()
                .filter_entry(move |e| !filter_clone.should_prune_dir(e))
                .filter_map(|e| e.ok())
                .par_bridge()
        })
        .filter_map({
            let filter_clone = Arc::clone(&filter);
            move |entry| {
                let c = scanned_count.fetch_add(1, Ordering::Relaxed);
                if c % 1000 == 0 {
                    let _ = app_handle.emit("scan_progress", format!("Scanned {} entries...", c));
                }

                if entry.file_type().is_file() {
                    if filter_clone.should_exclude_file(&entry) {
                        return None;
                    }
                    if let Ok(meta) = entry.metadata() {
                        if meta.len() >= min_size {
                            return Some((meta.len(), entry.path().display().to_string()));
                        }
                    }
                }
                None
            }
        })
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
pub async fn find_duplicate_files(
    app_handle: AppHandle,
    exclusions: Option<Vec<String>>,
) -> Result<Vec<(String, u64)>, AppError> {
    let wp = crate::paths::WindowsPaths::get();
    if wp.user_profile.as_os_str().is_empty() {
        return Err(AppError::System("USERPROFILE not found".to_string()));
    }

    let paths_to_scan = vec![
        wp.user_profile.join("Downloads"),
        wp.user_profile.join("Documents"),
    ];

    let scanned_count = std::sync::atomic::AtomicUsize::new(0);
    let filter = Arc::new(ExclusionFilter::new(exclusions));

    let files: Vec<PathBuf> = paths_to_scan
        .into_par_iter()
        .flat_map(|root| {
            let filter_clone = Arc::clone(&filter);
            WalkDir::new(root)
                .into_iter()
                .filter_entry(move |e| !filter_clone.should_prune_dir(e))
                .filter_map(|e| e.ok())
                .par_bridge()
        })
        .filter_map({
            let filter_clone = Arc::clone(&filter);
            move |entry| {
                if entry.file_type().is_file() {
                    if filter_clone.should_exclude_file(&entry) {
                        return None;
                    }
                    let c = scanned_count.fetch_add(1, Ordering::Relaxed);
                    if c % 500 == 0 {
                        let _ = app_handle.emit("scan_progress", format!("Scanned {} files...", c));
                    }
                    return Some(entry.path().to_path_buf());
                }
                None
            }
        })
        .collect();

    let mut size_buckets: HashMap<u64, Vec<String>> = HashMap::new();
    for p in files {
        if let Ok(meta) = fs::metadata(&p) {
            size_buckets
                .entry(meta.len())
                .or_default()
                .push(p.display().to_string());
        }
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
pub async fn find_duplicate_groups(
    app_handle: AppHandle,
    exclusions: Option<Vec<String>>,
) -> Result<Vec<DuplicateGroup>, AppError> {
    let first_pass_limit: usize = 200_000;
    let hash_limit: usize = 15_000;

    let wp = crate::paths::WindowsPaths::get();
    if wp.user_profile.as_os_str().is_empty() {
        return Err(AppError::System("USERPROFILE not found".to_string()));
    }

    let paths_to_scan = vec![
        wp.user_profile.join("Downloads"),
        wp.user_profile.join("Documents"),
        wp.user_profile.join("Desktop"),
    ];

    let scanned_count = std::sync::atomic::AtomicUsize::new(0);
    let size_buckets: Arc<DashMap<u64, Vec<String>>> = Arc::new(DashMap::new());
    let filter = Arc::new(ExclusionFilter::new(exclusions));

    let size_buckets_clone = Arc::clone(&size_buckets);
    paths_to_scan
        .into_par_iter()
        .flat_map(|root| {
            let filter_clone = Arc::clone(&filter);
            WalkDir::new(root)
                .into_iter()
                .filter_entry(move |e| !filter_clone.should_prune_dir(e))
                .filter_map(|e| e.ok())
                .par_bridge()
        })
        .for_each({
            let filter_clone = Arc::clone(&filter);
            let size_buckets_inner = Arc::clone(&size_buckets_clone);
            move |entry| {
                if TEMP_CANCEL.load(Ordering::Relaxed) {
                    return;
                }

                let current = scanned_count.fetch_add(1, Ordering::Relaxed);
                if current >= first_pass_limit {
                    return;
                }

                if current % 1000 == 0 {
                    let _ =
                        app_handle.emit("scan_progress", format!("Scanned {} files...", current));
                }

                if entry.file_type().is_file() {
                    if filter_clone.should_exclude_file(&entry) {
                        return;
                    }
                    if let Ok(meta) = entry.metadata() {
                        let len = meta.len();
                        if len > 0 {
                            size_buckets_inner
                                .entry(len)
                                .or_default()
                                .push(entry.path().display().to_string());
                        }
                    }
                }
            }
        });

    if TEMP_CANCEL.load(Ordering::Relaxed) {
        return Ok(Vec::new());
    }

    let size_buckets = Arc::try_unwrap(size_buckets).ok().unwrap();

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
pub async fn move_to_trash(files: Vec<String>) -> Result<usize, AppError> {
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
pub async fn find_empty_folders(
    app_handle: AppHandle,
    exclusions: Option<Vec<String>>,
) -> Result<Vec<String>, AppError> {
    let wp = crate::paths::WindowsPaths::get();
    if wp.user_profile.as_os_str().is_empty() {
        return Err(AppError::System("USERPROFILE not found".to_string()));
    }

    let paths_to_scan = vec![
        wp.user_profile.join("Downloads"),
        wp.user_profile.join("Desktop"),
        wp.user_profile.join("Documents"),
    ];

    let scanned_count = std::sync::atomic::AtomicUsize::new(0);
    let filter = Arc::new(ExclusionFilter::new(exclusions));

    let dirs: Vec<PathBuf> = paths_to_scan
        .into_par_iter()
        .flat_map(|root| {
            let filter_clone = Arc::clone(&filter);
            WalkDir::new(root)
                .into_iter()
                .filter_entry(move |e| !filter_clone.should_prune_dir(e))
                .filter_map(|e| e.ok())
                .par_bridge()
        })
        .filter_map({
            let filter_clone = Arc::clone(&filter);
            move |entry| {
                if entry.file_type().is_dir() {
                    if filter_clone.should_exclude_path(entry.path()) {
                        return None;
                    }
                    let c = scanned_count.fetch_add(1, Ordering::Relaxed);
                    if c % 500 == 0 {
                        let _ = app_handle
                            .emit("scan_progress", format!("Scanned {} directories...", c));
                    }
                    return Some(entry.path().to_path_buf());
                }
                None
            }
        })
        .collect();

    let results: Vec<String> = dirs
        .into_par_iter()
        .filter_map(|d| {
            if let Ok(mut rd) = fs::read_dir(&d) {
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
pub async fn find_broken_shortcuts(
    app_handle: AppHandle,
    exclusions: Option<Vec<String>>,
) -> Result<Vec<String>, AppError> {
    let wp = crate::paths::WindowsPaths::get();
    if wp.user_profile.as_os_str().is_empty() {
        return Err(AppError::System("USERPROFILE not found".to_string()));
    }

    let mut paths_to_scan = vec![
        wp.user_profile.join("Downloads"),
        wp.user_profile.join("Desktop"),
        wp.user_profile.join("Documents"),
    ];

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

    let scanned_count = std::sync::atomic::AtomicUsize::new(0);
    let filter = Arc::new(ExclusionFilter::new(exclusions));

    let app_handle_clone = app_handle.clone();
    let lnk_files: Vec<PathBuf> = paths_to_scan
        .into_par_iter()
        .flat_map(|root| {
            let filter_clone = Arc::clone(&filter);
            WalkDir::new(root)
                .into_iter()
                .filter_entry(move |e| !filter_clone.should_prune_dir(e))
                .filter_map(|e| e.ok())
                .par_bridge()
        })
        .filter_map({
            let filter_clone = Arc::clone(&filter);
            let app_handle_inner = app_handle_clone.clone();
            move |entry| {
                if entry.file_type().is_file() {
                    if filter_clone.should_exclude_file(&entry) {
                        return None;
                    }
                    let c = scanned_count.fetch_add(1, Ordering::Relaxed);
                    if c % 400 == 0 {
                        let _ = app_handle_inner
                            .emit("scan_progress", format!("Scanned {} files...", c));
                    }

                    let path = entry.path();
                    if let Some(ext) = path.extension() {
                        if ext.eq_ignore_ascii_case("lnk") {
                            return Some(path.to_path_buf());
                        }
                    }
                }
                None
            }
        })
        .collect();

    app_handle.emit(
        "scan_progress",
        format!("Found {} shortcuts. Verifying...", lnk_files.len()),
    )?;

    let results: Vec<String> = lnk_files
        .into_par_iter()
        .filter_map(|p| {
            if TEMP_CANCEL.load(Ordering::Relaxed) {
                return None;
            }
            match ShellLink::open(&p, WINDOWS_1252) {
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
                Err(_) => {} // Ignore errors for now
            }
            None
        })
        .collect();

    Ok(results)
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub async fn find_broken_shortcuts(
    _app_handle: tauri::AppHandle,
    _exclusions: Option<Vec<String>>,
) -> Result<Vec<String>, AppError> {
    Ok(Vec::new())
}

#[tauri::command]
pub async fn move_files(files: Vec<String>, destination: String) -> Result<usize, AppError> {
    let dest = PathBuf::from(&destination);
    if !dest.exists() {
        fs::create_dir_all(&dest)?;
    }
    if !dest.is_dir() {
        return Err(AppError::Internal("Destination is not a directory".into()));
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
pub async fn stat_paths(paths: Vec<String>) -> Result<Vec<(String, u64)>, AppError> {
    let out: Vec<(String, u64)> = paths
        .into_par_iter()
        .map(|p| {
            let pb = PathBuf::from(&p);
            if let Ok(meta) = fs::metadata(&pb) {
                if meta.is_file() {
                    return (p, meta.len());
                }
            }
            (p, 0)
        })
        .collect();
    Ok(out)
}
