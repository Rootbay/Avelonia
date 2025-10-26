use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use tauri::{Emitter, AppHandle}; // Added this line
use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;

static TEMP_CANCEL: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));
use sysinfo::Disks;

use sha2::{Digest, Sha256};
use walkdir::WalkDir;
use lnk::{ShellLink};
use lnk::encoding::WINDOWS_1252;
use rayon::prelude::*;

#[cfg(target_os = "windows")]
use windows::Win32::UI::Shell::{SHEmptyRecycleBinA, SHERB_NOCONFIRMATION, SHERB_NOPROGRESSUI, SHERB_NOSOUND};

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

// Streamed variant to avoid huge payloads over IPC.
// Emits events:
//  - "cleaner-temp-batch": Vec<String>
//  - "cleaner-temp-done": { total: usize }
#[tauri::command]
pub fn get_temp_files_stream(app_handle: AppHandle, batch_size: Option<usize>, max: Option<usize>) -> Result<usize, String> {
    let bs = batch_size.unwrap_or(250).max(50).min(1000);
    let limit = max.unwrap_or(30_000);
    TEMP_CANCEL.store(false, Ordering::Relaxed);

    let roots: Vec<PathBuf> = vec![
        env::var_os("TEMP").map(PathBuf::from).unwrap_or_default(),
        env::var_os("LOCALAPPDATA").map(|p| PathBuf::from(p).join("Temp")).unwrap_or_default(),
        PathBuf::from("/tmp"),
        PathBuf::from("/private/tmp"),
        PathBuf::from("/var/tmp"),
    ]
    .into_iter()
    .filter(|p| p.exists() && p.is_dir())
    .collect();

    let (tx, rx) = mpsc::channel::<String>();
    let mut handles = Vec::new();
    for root in roots {
        let txc = tx.clone();
        let ah = app_handle.clone();
        handles.push(thread::spawn(move || {
            let mut scanned = 0usize;
            for entry in walkdir::WalkDir::new(&root).into_iter().filter_map(|e| e.ok()) {
                if TEMP_CANCEL.load(Ordering::Relaxed) { break; }
                if entry.file_type().is_file() {
                    let _ = txc.send(entry.path().display().to_string());
                }
                scanned += 1;
                if scanned % 1000 == 0 {
                    let _ = ah.emit("scan_progress", format!("Scanned {} entries...", scanned));
                }
            }
        }));
    }
    drop(tx);

    let mut total = 0usize;
    let mut batch: Vec<String> = Vec::with_capacity(bs);
    while let Ok(p) = rx.recv() {
        if TEMP_CANCEL.load(Ordering::Relaxed) { break; }
        total += 1;
        if total >= limit { TEMP_CANCEL.store(true, Ordering::Relaxed); }
        batch.push(p);
        if batch.len() >= bs {
            let _ = app_handle.emit("cleaner-temp-batch", batch.clone());
            batch.clear();
        }
        if total >= limit { break; }
    }
    if !batch.is_empty() {
        let _ = app_handle.emit("cleaner-temp-batch", batch);
    }
    for h in handles { let _ = h.join(); }
    let _ = app_handle.emit("cleaner-temp-done", serde_json::json!({"total": total}));
    Ok(total)
}

#[tauri::command]
pub fn cancel_temp_scan() -> Result<(), String> {
    TEMP_CANCEL.store(true, Ordering::Relaxed);
    Ok(())
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

// Quick Clean commands (Windows only) ---------------------------------------------------------
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
    // Delete only .lnk shortcuts in Recent
    if let Some(appdata) = env::var_os("APPDATA") {
        let recent = PathBuf::from(appdata).join("Microsoft/Windows/Recent");
        if !recent.exists() || !recent.is_dir() {
            return Ok(CleanupStats::default());
        }
        let mut stats = CleanupStats::default();
        for entry in WalkDir::new(&recent).min_depth(1).max_depth(1).into_iter().filter_map(|e| e.ok()) {
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
pub fn quick_clear_user_temp() -> Result<CleanupStats, String> { Err("Only on Windows".into()) }
#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn quick_clear_system_temp() -> Result<CleanupStats, String> { Err("Only on Windows".into()) }
#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn quick_clear_prefetch() -> Result<CleanupStats, String> { Err("Only on Windows".into()) }
#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn quick_clear_recent() -> Result<CleanupStats, String> { Err("Only on Windows".into()) }

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
    let min_size_bytes: u64 = 100 * 1024 * 1024; // 100 MB
    find_large_files_min(min_size_bytes, app_handle)
}

#[tauri::command]
pub fn find_large_files_min(min_size_bytes: u64, app_handle: tauri::AppHandle) -> Result<Vec<(String, u64)>, String> {
    let user_profile = env::var_os("USERPROFILE").ok_or("USERPROFILE not found".to_string())?;
    let downloads_path = PathBuf::from(&user_profile).join("Downloads");
    let desktop_path = PathBuf::from(&user_profile).join("Desktop");
    let documents_path = PathBuf::from(&user_profile).join("Documents");
    let paths_to_scan = vec![downloads_path, desktop_path, documents_path];

    // Collect files first (sequential walk), then filter in parallel
    let mut files: Vec<PathBuf> = Vec::new();
    let mut scanned_count = 0usize;
    for scan_path in paths_to_scan {
        if scan_path.exists() && scan_path.is_dir() {
            for entry in walkdir::WalkDir::new(&scan_path).into_iter().filter_map(|e| e.ok()) {
                if TEMP_CANCEL.load(Ordering::Relaxed) { break; }
                if entry.file_type().is_file() {
                    files.push(entry.path().to_path_buf());
                }
                scanned_count += 1;
                if scanned_count % 500 == 0 {
                    let _ = app_handle.emit("scan_progress", format!("Scanned {} files for large files...", scanned_count));
                }
            }
        }
    }

    let results: Vec<(String, u64)> = files
        .par_iter()
        .filter_map(|p| {
            if let Ok(meta) = fs::metadata(p) {
                if meta.is_file() && meta.len() >= min_size_bytes {
                    return Some((p.display().to_string(), meta.len()));
                }
            }
            None
        })
        .collect();

    Ok(results)
}

#[tauri::command]
pub fn find_duplicate_files(app_handle: tauri::AppHandle) -> Result<Vec<(String, u64)>, String> {
    let mut scanned_count = 0usize;
    let user_profile = env::var_os("USERPROFILE").ok_or("USERPROFILE not found".to_string())?;
    let downloads_path = PathBuf::from(&user_profile).join("Downloads");
    let documents_path = PathBuf::from(&user_profile).join("Documents");
    let paths_to_scan = vec![downloads_path, documents_path];
    // Enumerate files
    let mut files: Vec<PathBuf> = Vec::new();
    for scan_path in &paths_to_scan {
        if scan_path.exists() && scan_path.is_dir() {
            for entry in WalkDir::new(&scan_path).into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_file() {
                    files.push(entry.path().to_path_buf());
                }
                scanned_count += 1;
                if scanned_count % 500 == 0 { let _ = app_handle.emit("scan_progress", format!("Scanned {} files...", scanned_count)); }
            }
        }
    }

    // Get sizes in parallel, then bucket
    let sized: Vec<(u64, String)> = files
        .par_iter()
        .filter_map(|p| fs::metadata(p).ok().map(|m| (m.len(), p.display().to_string())))
        .collect();
    let mut size_buckets: HashMap<u64, Vec<String>> = HashMap::new();
    for (sz, p) in sized { size_buckets.entry(sz).or_default().push(p); }

    // Hash only buckets with >1, in parallel
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
        for (h, p) in hashed { file_hashes.entry(h).or_default().push(p); }
    }

    let mut out = Vec::new();
    for (_h, paths) in file_hashes.into_iter().filter(|(_, v)| v.len() > 1) {
        if let Some(first) = paths.first() {
            if let Ok(meta) = fs::metadata(first) {
                for p in paths { out.push((p, meta.len())); }
            }
        }
    }
    Ok(out)
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct DuplicateGroup { pub hash: String, pub size: u64, pub files: Vec<String> }

#[tauri::command]
pub fn find_duplicate_groups(app_handle: tauri::AppHandle) -> Result<Vec<DuplicateGroup>, String> {
    let mut scanned_count = 0usize;
    let first_pass_limit: usize = 30_000; // cap number of files enumerated
    let hash_limit: usize = 15_000; // cap number of files hashed

    let user_profile = env::var_os("USERPROFILE").ok_or("USERPROFILE not found".to_string())?;
    let downloads_path = PathBuf::from(&user_profile).join("Downloads");
    let documents_path = PathBuf::from(&user_profile).join("Documents");
    let desktop_path = PathBuf::from(&user_profile).join("Desktop");
    let paths_to_scan = vec![downloads_path, documents_path, desktop_path];

    // Enumerate files with a hard cap
    let mut files: Vec<PathBuf> = Vec::new();
    for scan_path in &paths_to_scan {
        if scan_path.exists() && scan_path.is_dir() {
            for entry in walkdir::WalkDir::new(&scan_path).into_iter().filter_map(|e| e.ok()) {
                if TEMP_CANCEL.load(Ordering::Relaxed) { break; }
                if entry.file_type().is_file() { files.push(entry.path().to_path_buf()); }
                scanned_count += 1;
                if scanned_count % 400 == 0 { let _ = app_handle.emit("scan_progress", format!("Scanned {} files...", scanned_count)); }
                if files.len() >= first_pass_limit { break; }
            }
        }
        if files.len() >= first_pass_limit { break; }
    }

    // Get sizes in parallel and bucket
    let sized: Vec<(u64, String)> = files
        .par_iter()
        .filter_map(|p| fs::metadata(p).ok().map(|m| (m.len(), p.display().to_string())))
        .collect();
    let mut size_buckets: HashMap<u64, Vec<String>> = HashMap::new();
    for (sz, p) in sized { size_buckets.entry(sz).or_default().push(p); }

    // Hash buckets in parallel up to hash_limit items
    let mut file_hashes: HashMap<String, Vec<String>> = HashMap::new();
    let mut hashed = 0usize;
    for (_size, group) in size_buckets.into_iter().filter(|(_, v)| v.len() > 1) {
        if hashed >= hash_limit { break; }
        let remaining = hash_limit.saturating_sub(hashed);
        let batch: Vec<String> = group.into_iter().take(remaining).collect();
        let hashed_batch: Vec<(String, String)> = batch
            .par_iter()
            .filter_map(|p| {
                let path = PathBuf::from(p);
                match calculate_file_hash(&path) {
                    Ok(h) => Some((h, p.clone())),
                    Err(_) => None,
                }
            })
            .collect();
        hashed += hashed_batch.len();
        for (h, p) in hashed_batch { file_hashes.entry(h).or_default().push(p); }
        if TEMP_CANCEL.load(Ordering::Relaxed) { break; }
    }

    let mut out: Vec<DuplicateGroup> = Vec::new();
    for (hash, files) in file_hashes.into_iter().filter(|(_, v)| v.len() > 1) {
        let size = files
            .get(0)
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
    let mut scanned_count = 0usize;

    let user_profile = env::var_os("USERPROFILE").ok_or("USERPROFILE not found".to_string())?;
    let downloads_path = PathBuf::from(&user_profile).join("Downloads");
    let desktop_path = PathBuf::from(&user_profile).join("Desktop");
    let documents_path = PathBuf::from(&user_profile).join("Documents");

    let paths_to_scan = vec![downloads_path, desktop_path, documents_path];

    // Collect directories first
    let mut dirs: Vec<PathBuf> = Vec::new();
    for scan_path in paths_to_scan {
        if scan_path.exists() && scan_path.is_dir() {
            for entry in WalkDir::new(&scan_path).into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_dir() { dirs.push(entry.path().to_path_buf()); }
                scanned_count += 1;
                if scanned_count % 500 == 0 { let _ = app_handle.emit("scan_progress", format!("Scanned {} directories for empty folders...", scanned_count)); }
            }
        }
    }
    // Check emptiness in parallel
    let results: Vec<String> = dirs
        .par_iter()
        .filter_map(|d| {
            if let Ok(mut rd) = fs::read_dir(d) {
                if rd.next().is_none() { return Some(d.display().to_string()); }
            }
            None
        })
        .collect();
    Ok(results)
}

#[tauri::command]
pub fn find_broken_shortcuts(app_handle: tauri::AppHandle) -> Result<Vec<String>, String> {
    let mut scanned_count = 0usize;

    let user_profile = env::var_os("USERPROFILE").ok_or("USERPROFILE not found".to_string())?;
    let downloads_path = PathBuf::from(&user_profile).join("Downloads");
    let desktop_path = PathBuf::from(&user_profile).join("Desktop");
    let documents_path = PathBuf::from(&user_profile).join("Documents");

    let paths_to_scan = vec![downloads_path, desktop_path, documents_path];

    // Collect .lnk files
    let mut lnk_files: Vec<PathBuf> = Vec::new();
    for scan_path in paths_to_scan {
        if scan_path.exists() && scan_path.is_dir() {
            for entry in WalkDir::new(&scan_path).into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_file() {
                    let path = entry.path();
                    if let Some(ext) = path.extension() {
                        if ext.eq_ignore_ascii_case("lnk") { lnk_files.push(path.to_path_buf()); }
                    }
                }
                scanned_count += 1;
                if scanned_count % 400 == 0 { let _ = app_handle.emit("scan_progress", format!("Scanned {} files for broken shortcuts...", scanned_count)); }
            }
        }
    }
    // Parse shortcuts in parallel
    let results: Vec<String> = lnk_files
        .par_iter()
        .filter_map(|p| {
            match ShellLink::open(p, WINDOWS_1252) {
                Ok(shell_link) => {
                    if let Some(link_info) = shell_link.link_info() {
                        let common_path_str = link_info.common_path_suffix();
                        let target_path = PathBuf::from(common_path_str.to_string());
                        if !target_path.exists() {
                            return Some(p.display().to_string());
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
        if !from.exists() || !from.is_file() { continue; }
        let file_name = match from.file_name() { Some(n) => n, None => continue };
        let mut to = dest.join(file_name);
        // handle name collisions
        if to.exists() {
            let mut idx = 1u32;
            let stem = to.file_stem().and_then(|s| s.to_str()).unwrap_or("file").to_string();
            let ext = to.extension().and_then(|s| s.to_str()).unwrap_or("");
            loop {
                let candidate = if ext.is_empty() {
                    dest.join(format!("{} ({})", stem, idx))
                } else {
                    dest.join(format!("{} ({}).{}", stem, idx, ext))
                };
                if !candidate.exists() { to = candidate; break; }
                idx += 1;
                if idx > 9999 { break; }
            }
        }
        if fs::rename(&from, &to).is_ok() {
            moved += 1;
            continue;
        }
        // cross-device fallback: copy then remove
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
