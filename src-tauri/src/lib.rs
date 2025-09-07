// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use std::env;
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Serialize};
use std::fs::File;
use std::io::Write;
use tauri::{AppHandle, State, Emitter};
use dashmap::DashMap;
use std::sync::Arc;

mod cleaner;
mod system_info;

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

    let client = Client::new();
    let res = client
        .get(&url)
        .send()
        .await
        .map_err(|e| {
            println!("Rust Error: Failed to get response: {}", e);
            format!("Failed to get response: {}", e)
        })?;
    println!("Rust: Got response for ID: {}", id);

    let total = res
        .content_length()
        .ok_or_else(|| {
            println!("Rust Error: Failed to get content length for ID: {}", id);
            "Failed to get content length".to_string()
        })?;
    println!("Rust: Content length for ID {}: {}", id, total);

    let mut file =
        File::create(&path).map_err(|e| {
            println!("Rust Error: Failed to create file {}: {}", path, e);
            format!("Failed to create file: {}", e)
        })?;
    println!("Rust: File created at {}", path);
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

        app.emit(
            "download-progress",
            DownloadProgress {
                id,
                downloaded,
                total,
            },
        )
        .unwrap();
        // println!("Rust: Emitted progress for ID {}: {}/{}", id, downloaded, total);
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
        .invoke_handler(tauri::generate_handler![
            greet,
            download_file,
            cancel_download,
            cleaner::get_temp_files,
            cleaner::clean_temp_files,
            cleaner::delete_files, // New command
            cleaner::empty_recycle_bin,
            cleaner::find_large_files,
            cleaner::find_duplicate_files,
            cleaner::find_empty_folders,
            cleaner::find_broken_shortcuts,
            cleaner::get_drive_info,
            system_info::get_cpu_usage,
            system_info::get_memory_usage,
            system_info::get_total_memory
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}