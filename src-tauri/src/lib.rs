// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use futures_util::StreamExt;
use reqwest::Client;
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

#[derive(Default)]
struct DownloadState(Arc<DashMap<u64, bool>>);

#[derive(Clone, Serialize)]
struct DownloadProgress {
    id: u64,
    downloaded: u64,
    total: u64,
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
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            download_file,
            cancel_download,
            cleaner::get_temp_files,
            cleaner::clean_temp_files,
            cleaner::delete_files, // New command
            cleaner::move_to_trash,
            cleaner::empty_recycle_bin,
            cleaner::find_large_files,
            cleaner::find_duplicate_files,
            cleaner::find_empty_folders,
            cleaner::find_broken_shortcuts,
            cleaner::get_drive_info,
            system_info::get_cpu_usage,
            system_info::get_memory_usage,
            system_info::get_total_memory,
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
            cleaner::quick_clear_user_temp,
            cleaner::quick_clear_system_temp,
            cleaner::quick_clear_prefetch,
            cleaner::quick_clear_recent
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
