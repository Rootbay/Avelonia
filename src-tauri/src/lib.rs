use dashmap::DashMap;
use futures_util::StreamExt;
use reqwest::{Client, Url};
use serde::Serialize;
use std::fs;
use std::fs::File;
use std::io::{ErrorKind, Write};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};

mod cleaner;
mod eraser;
mod installer;
mod optimize;
mod system_info;
mod vt;
mod paths;
mod error;

pub use error::AppError;

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

#[tauri::command]
async fn verify_hash(path: String, expected_hash: String) -> Result<bool, String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;

    let mut file =
        File::open(&path).map_err(|e| format!("Failed to open file for hashing: {}", e))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65536];

    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|e| format!("Failed to read file for hashing: {}", e))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }

    let result = hasher.finalize();
    let actual_hash = format!("{:x}", result);

    Ok(actual_hash.to_lowercase() == expected_hash.to_lowercase())
}

fn filename_from_cd<S: AsRef<str>>(cd: S) -> Option<String> {
    let cd = cd.as_ref();
    if let Some(idx) = cd.to_lowercase().find("filename*=") {
        let rest = &cd[idx + 10..];
        let parts: Vec<&str> = rest.split(';').collect();
        let val = parts[0].trim();
        if let Some(pos) = val.find("''") {
            let enc = &val[pos + 2..];
            if let Ok(decoded) = percent_encoding::percent_decode_str(enc).decode_utf8() {
                return Some(decoded.to_string());
            }
        } else {
            let v = val.trim_matches('"');
            return Some(v.to_string());
        }
    }
    if let Some(idx) = cd.to_lowercase().find("filename=") {
        let rest = &cd[idx + 9..];
        let v = rest
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .trim_matches('"');
        if !v.is_empty() {
            return Some(v.to_string());
        }
    }
    None
}

fn ext_from_content_type<S: AsRef<str>>(ct: S) -> Option<String> {
    let ct = ct.as_ref().to_lowercase();
    let ext = if ct.contains("application/zip") {
        "zip"
    } else if ct.contains("application/x-7z-compressed") {
        "7z"
    } else if ct.contains("application/x-rar-compressed") {
        "rar"
    } else if ct.contains("application/x-msdownload") {
        "exe"
    } else if ct.contains("application/x-msi")
        || ct.contains("application/x-ms-installer")
        || ct.contains("application/x-msdownload")
    {
        "msi"
    } else if ct.contains("application/x-dosexec") {
        "exe"
    } else if ct.contains("application/x-tar") {
        "tar"
    } else if ct.contains("application/gzip") {
        "tar.gz"
    } else if ct.contains("application/x-bzip2") {
        "tar.bz2"
    } else if ct.contains("application/x-xz") {
        "tar.xz"
    } else if ct.contains("application/x-zstd") {
        "tar.zst"
    } else if ct.contains("application/pdf") {
        "pdf"
    } else if ct.contains("application/json") {
        "json"
    } else if ct.contains("text/plain") {
        "txt"
    } else if ct.contains("image/png") {
        "png"
    } else if ct.contains("image/jpeg") {
        "jpg"
    } else {
        ""
    };
    if ext.is_empty() {
        None
    } else {
        Some(ext.to_string())
    }
}

#[tauri::command]
async fn probe_download(url: String) -> Result<ProbeResult, String> {
    let client = Client::builder()
        .user_agent("Avelonia/0.1 (tauri)")
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let mut filename: Option<String> = None;
    let mut ext: Option<String> = None;
    let mut size: Option<u64> = None;

    if let Ok(resp) = client.head(&url).send().await {
        if resp.status().is_success() {
            if let Some(cd) = resp.headers().get(reqwest::header::CONTENT_DISPOSITION) {
                if let Ok(s) = cd.to_str() {
                    filename = filename_from_cd(s);
                }
            }
            if let Some(ct) = resp.headers().get(reqwest::header::CONTENT_TYPE) {
                if let Ok(s) = ct.to_str() {
                    ext = ext_from_content_type(s);
                }
            }
            if let Some(cl) = resp.headers().get(reqwest::header::CONTENT_LENGTH) {
                if let Ok(s) = cl.to_str() {
                    if let Ok(v) = s.parse::<u64>() {
                        size = Some(v);
                    }
                }
            }
        }
    }

    if filename.is_none() || ext.is_none() {
        if let Ok(resp) = client
            .get(&url)
            .header(reqwest::header::RANGE, "bytes=0-0")
            .send()
            .await
        {
            if filename.is_none() {
                if let Some(cd) = resp.headers().get(reqwest::header::CONTENT_DISPOSITION) {
                    if let Ok(s) = cd.to_str() {
                        filename = filename_from_cd(s);
                    }
                }
            }
            if ext.is_none() {
                if let Some(ct) = resp.headers().get(reqwest::header::CONTENT_TYPE) {
                    if let Ok(s) = ct.to_str() {
                        ext = ext_from_content_type(s);
                    }
                }
            }
            if size.is_none() {
                if let Some(cr) = resp.headers().get(reqwest::header::CONTENT_RANGE) {
                    if let Ok(s) = cr.to_str() {
                        if let Some(pos) = s.rfind('/') {
                            if let Ok(v) = s[pos + 1..].trim().parse::<u64>() {
                                size = Some(v);
                            }
                        }
                    }
                }
            }
        }
    }

    if filename.is_none() {
        if let Ok(u) = Url::parse(&url) {
            if let Some(seg) = u
                .path_segments()
                .and_then(|s| s.last())
                .and_then(|s| percent_encoding::percent_decode_str(s).decode_utf8().ok())
                .map(|cow| cow.to_string())
            {
                filename = Some(seg);
            }
        }
    }

    let filename = filename.unwrap_or_else(|| "download".to_string());
    if ext.is_none() {
        let lower = filename.to_lowercase();
        let multi = [".tar.gz", ".tar.bz2", ".tar.xz", ".tar.zst"];
        for m in multi.iter() {
            if lower.ends_with(m) {
                ext = Some(m.trim_start_matches('.').to_string());
                break;
            }
        }
        if ext.is_none() {
            if let Some(idx) = lower.rfind('.') {
                ext = Some(lower[idx + 1..].to_string());
            }
        }
    }

    Ok(ProbeResult {
        filename,
        ext: ext.unwrap_or_default(),
        size,
    })
}

async fn download_file_inner(
    app: &AppHandle,
    id: u64,
    url: &str,
    path: &str,
    state: &State<'_, DownloadState>,
) -> Result<(), String> {
    state.0.insert(id, false);
    let temp_path = format!("{}.part", &path);
    let mut downloaded: u64 = 0;

    if let Ok(metadata) = std::fs::metadata(&temp_path) {
        downloaded = metadata.len();
    }

    let client = Client::builder()
        .user_agent("Avelonia/0.1 (tauri)")
        .timeout(Duration::from_secs(300))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let mut request = client.get(url);
    if downloaded > 0 {
        request = request.header(reqwest::header::RANGE, format!("bytes={}-", downloaded));
    }

    let res = request.send().await.map_err(|e| format!("Failed to get response: {}", e))?;
    
    // Check for HTTP errors (4xx, 5xx)
    let res = res.error_for_status().map_err(|e| format!("Server returned error: {}", e))?;

    let status = res.status();
    let is_partial = status == reqwest::StatusCode::PARTIAL_CONTENT;

    if downloaded > 0 && !is_partial {
        downloaded = 0;
    }

    let content_length = res.content_length().unwrap_or(0);
    let total = if is_partial {
        content_length + downloaded
    } else {
        content_length
    };

    let mut file = if downloaded == 0 {
        std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp_path)
    } else {
        std::fs::OpenOptions::new()
            .write(true)
            .append(true)
            .open(&temp_path)
    }
    .map_err(|e| format!("Failed to open temp file: {}", e))?;

    let mut stream = res.bytes_stream();
    while let Some(item) = stream.next().await {
        if state.0.get(&id).map_or(false, |r| *r) {
            return Ok(());
        }

        let chunk = item.map_err(|e: reqwest::Error| format!("Failed to download chunk: {}", e))?;
        file.write_all(&chunk).map_err(|e| format!("Failed to write to file: {}", e))?;
        downloaded += chunk.len() as u64;

        let _ = app.emit(
            "download-progress",
            DownloadProgress {
                id,
                downloaded,
                total,
            },
        );
    }

    let was_cancelled = state.0.get(&id).map_or(false, |r| *r);
    if was_cancelled {
        return Ok(());
    }

    if (total > 0 && downloaded >= total) || (total == 0 && downloaded > 0) {
        let final_total = if total == 0 { downloaded } else { total };
        let _ = app.emit(
            "download-progress",
            DownloadProgress {
                id,
                downloaded: final_total,
                total: final_total,
            },
        );
        std::fs::rename(&temp_path, &path).map_err(|e| format!("Failed to finalize download: {}", e))?;
        Ok(())
    } else {
        Err("Download interrupted or incomplete".to_string())
    }
}

#[tauri::command]
async fn download_file(
    app: AppHandle,
    id: u64,
    url: String,
    path: String,
    state: State<'_, DownloadState>,
) -> Result<(), String> {
    let mut last_err = String::new();
    for attempt in 1..=3 {
        match download_file_inner(&app, id, &url, &path, &state).await {
            Ok(_) => return Ok(()),
            Err(e) => {
                last_err = e;
                if state.0.get(&id).map_or(false, |r| *r) {
                    break;
                }
                if attempt < 3 {
                    tokio::time::sleep(Duration::from_secs(attempt * 2)).await;
                }
            }
        }
    }
    state.0.remove(&id);
    println!("Rust: Download failed after 3 attempts: {}", last_err);
    Err(format!("Download failed after 3 attempts: {}", last_err))
}

#[tauri::command]

fn cleanup_orphaned_downloads(
    download_dir: String,
    active_paths: Vec<String>,
) -> Result<u64, String> {
    let mut count = 0;

    let active_set: std::collections::HashSet<String> = active_paths.into_iter().collect();

    if let Ok(entries) = std::fs::read_dir(download_dir) {
        for entry in entries.flatten() {
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("part") {
                let path_str = path.to_string_lossy().to_string();

                if !active_set.contains(&path_str) {
                    if std::fs::remove_file(path).is_ok() {
                        count += 1;
                    }
                }
            }
        }
    }

    Ok(count)
}

#[tauri::command]
async fn cancel_download(id: u64, state: State<'_, DownloadState>) -> Result<(), String> {
    state.0.insert(id, true);
    Ok(())
}

#[tauri::command]
fn read_download_catalog(path: String) -> Result<String, String> {
    match fs::read_to_string(&path) {
        Ok(content) => Ok(content),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(format!("Failed to read catalog {}: {}", path, e)),
    }
}

#[tauri::command]
fn write_download_catalog(path: String, contents: String) -> Result<(), String> {
    if let Some(parent) = Path::new(&path).parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "Failed to ensure catalog directory {}: {}",
                parent.display(),
                e
            )
        })?;
    }
    fs::write(&path, contents).map_err(|e| format!("Failed to write catalog {}: {}", path, e))
}

#[tauri::command]
fn move_download_catalog(from: String, to: String) -> Result<(), String> {
    if from.is_empty() || to.is_empty() || from == to {
        return Ok(());
    }
    if let Some(parent) = Path::new(&to).parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "Failed to create catalog directory {}: {}",
                parent.display(),
                e
            )
        })?;
    }
    if !Path::new(&from).exists() {
        return Ok(());
    }
    fs::rename(&from, &to)
        .map_err(|e| format!("Failed to move catalog from {} to {}: {}", from, to, e))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let system_state_arc = Arc::new(system_info::SystemState::new());
    let system_state_clone = Arc::clone(&system_state_arc);

    std::thread::spawn(move || {
        loop {
            {
                if let Ok(mut sys) = system_state_clone.sys.lock() {
                    sys.refresh_cpu_all();
                    let usage = sys.global_cpu_usage();
                    if let Ok(mut cpu_usage) = system_state_clone.cpu_usage.lock() {
                        *cpu_usage = usage;
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(1000));
        }
    });

    tauri::Builder::default()
        .manage(DownloadState::default())
        .manage(installer::InstallState::default())
        .manage(vt::VtState::new())
        .manage(system_state_arc)
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            path_exists,
            download_file,
            cancel_download,
            probe_download,
            read_download_catalog,
            write_download_catalog,
            move_download_catalog,
            verify_hash,
            cleanup_orphaned_downloads,
            cleaner::get_temp_files,
            cleaner::get_temp_files_stream,
            cleaner::cancel_temp_scan,
            cleaner::start_cleaner_scan,
            cleaner::cancel_cleaner_scan,
            cleaner::start_large_scan,
            cleaner::start_duplicate_groups_scan,
            cleaner::start_empty_scan,
            cleaner::start_shortcut_scan,
            cleaner::clean_temp_files,
            cleaner::delete_files,
            cleaner::move_to_trash,
            cleaner::empty_recycle_bin,
            cleaner::find_large_files,
            cleaner::find_large_files_top,
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
            optimize::get_network_summary,
            optimize::run_ping,
            optimize::run_traceroute,
            optimize::run_dns_lookup,
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
            optimize::apply_tweaks,
            optimize::run_fix_action,
            optimize::apply_update_profile,
            cleaner::quick_clear_user_temp,
            cleaner::quick_clear_system_temp,
            cleaner::quick_clear_prefetch,
            cleaner::quick_clear_recent,
            installer::silent_install,
            installer::launch_installer,
            installer::list_uninstall_entries,
            installer::verify_install,
            installer::is_installed,
            installer::cancel_install,
            vt::vt_get_status,
            vt::vt_set_api_key,
            vt::vt_load_cache,
            vt::vt_clear_cache,
            vt::vt_scan_startup,
            vt::vt_scan_registry,
            vt::vt_scan_all,
            vt::vt_scan_needed,
            vt::vt_auto_maybe_scan
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}