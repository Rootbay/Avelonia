// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use futures_util::StreamExt;
use reqwest::{Client, Url};
use serde::{Serialize};
use std::fs::File;
use std::io::Write;
use tauri::{AppHandle, State, Emitter};
use dashmap::DashMap;
use std::sync::Arc;
use std::time::Duration;

mod cleaner;
mod system_info;
mod eraser;
mod optimize;
mod installer;
mod vt;

#[derive(Default)]
struct DownloadState(Arc<DashMap<u64, bool>>);

#[derive(Clone, Serialize)]
struct DownloadProgress {
    id: u64,
    downloaded: u64,
    total: u64,
}

#[derive(Clone, Serialize)]
struct ProbeResult {
    filename: String,
    ext: String,
    size: Option<u64>,
}

#[tauri::command]
fn path_exists(path: String) -> Result<bool, String> {
    Ok(std::path::Path::new(&path).exists())
}

fn filename_from_cd<S: AsRef<str>>(cd: S) -> Option<String> {
    let cd = cd.as_ref();
    // Try RFC 5987 filename*
    if let Some(idx) = cd.to_lowercase().find("filename*=") {
        let rest = &cd[idx+10..];
        let parts: Vec<&str> = rest.split(';').collect();
        let val = parts[0].trim();
        // format: charset''encoded
        if let Some(pos) = val.find("''") {
            let enc = &val[pos+2..];
            if let Ok(decoded) = percent_encoding::percent_decode_str(enc).decode_utf8() {
                return Some(decoded.to_string());
            }
        } else {
            // Sometimes quoted
            let v = val.trim_matches('"');
            return Some(v.to_string());
        }
    }
    // Fallback: filename="..."
    if let Some(idx) = cd.to_lowercase().find("filename=") {
        let rest = &cd[idx+9..];
        let v = rest.split(';').next().unwrap_or("").trim().trim_matches('"');
        if !v.is_empty() { return Some(v.to_string()); }
    }
    None
}

fn ext_from_content_type<S: AsRef<str>>(ct: S) -> Option<String> {
    let ct = ct.as_ref().to_lowercase();
    let ext = if ct.contains("application/zip") { "zip" }
    else if ct.contains("application/x-7z-compressed") { "7z" }
    else if ct.contains("application/x-rar-compressed") { "rar" }
    else if ct.contains("application/x-msdownload") { "exe" }
    else if ct.contains("application/x-msi") || ct.contains("application/x-ms-installer") || ct.contains("application/x-msdownload") { "msi" }
    else if ct.contains("application/x-dosexec") { "exe" }
    else if ct.contains("application/x-tar") { "tar" }
    else if ct.contains("application/gzip") { "tar.gz" }
    else if ct.contains("application/x-bzip2") { "tar.bz2" }
    else if ct.contains("application/x-xz") { "tar.xz" }
    else if ct.contains("application/x-zstd") { "tar.zst" }
    else if ct.contains("application/pdf") { "pdf" }
    else if ct.contains("application/json") { "json" }
    else if ct.contains("text/plain") { "txt" }
    else if ct.contains("image/png") { "png" }
    else if ct.contains("image/jpeg") { "jpg" }
    else { "" };
    if ext.is_empty() { None } else { Some(ext.to_string()) }
}

#[tauri::command]
async fn probe_download(url: String) -> Result<ProbeResult, String> {
    let client = Client::builder()
        .user_agent("Avelonia/0.1 (tauri)")
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    // Try HEAD first
    let mut filename: Option<String> = None;
    let mut ext: Option<String> = None;
    let mut size: Option<u64> = None;
    let head = client.head(&url).send().await;
    if let Ok(resp) = head {
        if resp.status().is_success() {
            if let Some(cd) = resp.headers().get(reqwest::header::CONTENT_DISPOSITION) {
                if let Ok(s) = cd.to_str() { filename = filename_from_cd(s); }
            }
            if let Some(ct) = resp.headers().get(reqwest::header::CONTENT_TYPE) {
                if let Ok(s) = ct.to_str() { ext = ext_from_content_type(s); }
            }
            if let Some(cl) = resp.headers().get(reqwest::header::CONTENT_LENGTH) {
                if let Ok(s) = cl.to_str() { if let Ok(v) = s.parse::<u64>() { size = Some(v); } }
            }
        }
    }
    // Fallback minimal GET (first bytes) if needed
    if filename.is_none() || ext.is_none() {
        let get = client.get(&url).header(reqwest::header::RANGE, "bytes=0-0").send().await;
        if let Ok(resp) = get {
            if filename.is_none() {
                if let Some(cd) = resp.headers().get(reqwest::header::CONTENT_DISPOSITION) {
                    if let Ok(s) = cd.to_str() { filename = filename_from_cd(s); }
                }
            }
            if ext.is_none() {
                if let Some(ct) = resp.headers().get(reqwest::header::CONTENT_TYPE) {
                    if let Ok(s) = ct.to_str() { ext = ext_from_content_type(s); }
                }
            }
            if size.is_none() {
                // Try to parse Content-Range: bytes 0-0/123456
                if let Some(cr) = resp.headers().get(reqwest::header::CONTENT_RANGE) {
                    if let Ok(s) = cr.to_str() {
                        if let Some(pos) = s.rfind('/') {
                            if let Ok(v) = s[pos+1..].trim().parse::<u64>() { size = Some(v); }
                        }
                    }
                }
            }
        }
    }

    // Final fallbacks based on URL path
    if filename.is_none() {
        if let Ok(u) = Url::parse(&url) {
            if let Some(seg) = u.path_segments().and_then(|s| s.last()).and_then(|s| percent_encoding::percent_decode_str(s).decode_utf8().ok()).map(|cow| cow.to_string()) {
                filename = Some(seg);
            }
        }
    }

    let filename = filename.unwrap_or_else(|| "download".to_string());
    // Try to infer extension from filename if still missing
    if ext.is_none() {
        let lower = filename.to_lowercase();
        let multi = [".tar.gz", ".tar.bz2", ".tar.xz", ".tar.zst"];
        for m in multi.iter() {
            if lower.ends_with(m) { ext = Some(m.trim_start_matches('.').to_string()); break; }
        }
        if ext.is_none() {
            if let Some(idx) = lower.rfind('.') { ext = Some(lower[idx+1..].to_string()); }
        }
    }

    Ok(ProbeResult { filename, ext: ext.unwrap_or_default(), size })
}

#[tauri::command]
async fn download_file(app: AppHandle, id: u64, url: String, path: String, state: State<'_, DownloadState>) -> Result<(), String> {
    println!("Rust: download_file called for ID: {}, URL: {}, Path: {}", id, url, path);
    state.0.insert(id, false);

    let client = Client::builder()
        .user_agent("Avelonia/0.1 (tauri)")
        .timeout(Duration::from_secs(300))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;
    let res = client
        .get(&url)
        .send()
        .await
        .map_err(|e| {
            println!("Rust Error: Failed to get response: {}", e);
            format!("Failed to get response: {}", e)
        })?;
    println!("Rust: Got response for ID: {}", id);

    let total = res.content_length().unwrap_or(0);
    if total > 0 {
        println!("Rust: Content length for ID {}: {}", id, total);
    } else {
        println!("Rust: Unknown content length for ID {} (streaming)", id);
    }

    // Write to temporary .part file then rename on success
    let temp_path = format!("{}.part", &path);
    let mut file =
        File::create(&temp_path).map_err(|e| {
            println!("Rust Error: Failed to create file {}: {}", path, e);
            format!("Failed to create file: {}", e)
        })?;
    println!("Rust: Temp file created at {}", temp_path);
    let mut downloaded: u64 = 0;
    let mut stream = res.bytes_stream();

    while let Some(item) = stream.next().await {
        if state.0.get(&id).map_or(false, |r| *r) {
            println!("Rust: Download cancelled for ID: {}", id);
            break;
        }

        let chunk = item.map_err(|e| {
            println!("Rust Error: Failed to download chunk for ID {}: {}", id, e);
            format!("Failed to download chunk: {}", e)
        })?;
        file.write_all(&chunk)
            .map_err(|e| {
                println!("Rust Error: Failed to write to file for ID {}: {}", id, e);
                format!("Failed to write to file: {}", e)
            })?;
        downloaded += chunk.len() as u64;

        if let Err(e) = app.emit(
            "download-progress",
            DownloadProgress {
                id,
                downloaded,
                total,
            },
        ) {
            println!("Rust Error: Failed to emit progress event for ID {}: {}", id, e);
        }
        // println!("Rust: Emitted progress for ID {}: {}/{}", id, downloaded, total);
    }

    // If cancelled before completion, try to remove the partial file
    let was_cancelled = state.0.get(&id).map_or(false, |r| *r);
    if was_cancelled || (total > 0 && downloaded < total) {
        if let Err(e) = std::fs::remove_file(&temp_path) {
            println!("Rust: Failed to remove partial file for ID {} at {}: {}", id, temp_path, e);
        } else {
            println!("Rust: Removed partial file for ID {} at {}", id, temp_path);
        }
    } else {
        // Completed successfully. Emit a final 100% progress and rename .part -> final
        let final_total = if total == 0 { downloaded } else { total };
        if let Err(e) = app.emit(
            "download-progress",
            DownloadProgress {
                id,
                downloaded: final_total,
                total: final_total,
            },
        ) {
            println!("Rust Error: Failed to emit final progress for ID {}: {}", id, e);
        }
        if let Err(e) = std::fs::rename(&temp_path, &path) {
            println!("Rust: Failed to rename {} to {} for ID {}: {}", temp_path, path, id, e);
        } else {
            println!("Rust: Renamed {} to {} for ID {}", temp_path, path, id);
        }
    }

    state.0.remove(&id);
    println!("Rust: Download finished for ID: {}", id);
    Ok(())
}

#[tauri::command]
async fn cancel_download(id: u64, state: State<'_, DownloadState>) -> Result<(), String> {
    println!("Rust: cancel_download called for ID: {}", id);
    state.0.insert(id, true);
    Ok(())
}


#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}



#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(DownloadState::default())
        .manage(vt::VtState::new())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            path_exists,
            download_file,
            cancel_download,
            probe_download,
            cleaner::get_temp_files,
            cleaner::get_temp_files_stream,
            cleaner::cancel_temp_scan,
            cleaner::clean_temp_files,
            cleaner::delete_files, // New command
            cleaner::move_to_trash,
            cleaner::empty_recycle_bin,
            cleaner::find_large_files,
            cleaner::find_duplicate_files,
            cleaner::find_duplicate_groups,
            cleaner::find_empty_folders,
            cleaner::find_broken_shortcuts,
            cleaner::get_drive_info,
            cleaner::find_large_files_min,
            cleaner::move_files,
            cleaner::stat_paths,
            system_info::get_cpu_usage,
            system_info::get_memory_usage,
            system_info::get_total_memory,
            system_info::get_boot_time,
            eraser::secure_erase,
            optimize::list_startup_shortcuts,
            optimize::remove_startup_shortcuts,
            optimize::list_registry_run,
            optimize::remove_registry_run,
            optimize::flush_dns,
            optimize::get_startup_folders,
            optimize::reset_winsock,
            optimize::renew_ip,
            optimize::list_scheduled_tasks,
            optimize::list_suspicious_tasks,
            optimize::get_task_details,
            optimize::disable_scheduled_tasks,
            optimize::enable_scheduled_tasks,
            optimize::delete_scheduled_tasks,
            optimize::run_scheduled_tasks,
            optimize::end_scheduled_tasks,
            optimize::open_registry_key,
            optimize::force_remove_registry_run,
            optimize::is_process_running,
            optimize::block_process_ifeo,
            optimize::schedule_delete_on_reboot,
            optimize::list_services,
            optimize::stop_services,
            optimize::disable_services,
            optimize::purge_startup_approved,
            optimize::delete_tasks_by_match,
            optimize::remove_wmi_subscriptions_by_match,
            optimize::restart_system,
            cleaner::quick_clear_user_temp,
            cleaner::quick_clear_system_temp,
            cleaner::quick_clear_prefetch,
            cleaner::quick_clear_recent,
            installer::silent_install,
            installer::launch_installer,
            installer::list_uninstall_entries,
            installer::verify_install,
            installer::is_installed,
            // VT reputation & config
            vt::vt_get_status,
            vt::vt_set_api_key,
            vt::vt_load_cache,
            vt::vt_scan_startup,
            vt::vt_scan_registry,
            vt::vt_scan_all,
            vt::vt_scan_needed
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

