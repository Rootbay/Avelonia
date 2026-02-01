use dashmap::DashMap;
use futures_util::StreamExt;
use reqwest::{Client, Url};
use serde::Serialize;
use std::fs;
use std::io::{ErrorKind, Write};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};

#[derive(Default)]
pub struct DownloadState(pub Arc<DashMap<u64, bool>>);

#[derive(Clone, Serialize)]
pub struct DownloadProgress {
    pub id: u64,
    pub downloaded: u64,
    pub total: u64,
}

#[derive(Clone, Serialize)]
pub struct ProbeResult {
    pub filename: String,
    pub ext: String,
    pub size: Option<u64>,
}

use crate::AppError;

#[tauri::command]
pub async fn probe_download(url: String) -> Result<ProbeResult, AppError> {
    let client = Client::builder()
        .user_agent("Avelonia/0.1 (tauri)")
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| AppError::Internal(format!("Failed to build HTTP client: {}", e)))?;

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

pub async fn download_file_inner(
    app: &AppHandle,
    id: u64,
    url: &str,
    path: &str,
    state: &State<'_, DownloadState>,
) -> Result<(), AppError> {
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
        .map_err(|e| AppError::Internal(format!("Failed to build HTTP client: {}", e)))?;

    let mut request = client.get(url);
    if downloaded > 0 {
        request = request.header(reqwest::header::RANGE, format!("bytes={}-", downloaded));
    }

    let res = request.send().await.map_err(|e| AppError::Internal(format!("Failed to get response: {}", e)))?;
    
    // Check for HTTP errors (4xx, 5xx)
    let res = res.error_for_status().map_err(|e| AppError::Internal(format!("Server returned error: {}", e)))?;

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
    .map_err(|e| AppError::Io(e))?;

    let mut stream = res.bytes_stream();
    while let Some(item) = stream.next().await {
        if state.0.get(&id).map_or(false, |r| *r) {
            return Ok(());
        }

        let chunk = item.map_err(|e: reqwest::Error| AppError::Internal(format!("Failed to download chunk: {}", e)))?;
        file.write_all(&chunk).map_err(|e| AppError::Io(e))?;
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
        std::fs::rename(&temp_path, &path).map_err(|e| AppError::Io(e))?;
        Ok(())
    } else {
        Err(AppError::Internal("Download interrupted or incomplete".to_string()))
    }
}

#[tauri::command]
pub async fn download_file(
    app: AppHandle,
    id: u64,
    url: String,
    path: String,
    state: State<'_, DownloadState>,
) -> Result<(), AppError> {
    let mut last_err = AppError::Internal("Unknown error".to_string());
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
    Err(last_err)
}

#[tauri::command]
pub async fn cleanup_orphaned_downloads(
    download_dir: String,
    active_paths: Vec<String>,
) -> Result<u64, AppError> {
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
pub async fn cancel_download(id: u64, state: State<'_, DownloadState>) -> Result<(), AppError> {
    state.0.insert(id, true);
    Ok(())
}

#[tauri::command]
pub async fn read_download_catalog(path: String) -> Result<String, AppError> {
    match fs::read_to_string(&path) {
        Ok(content) => Ok(content),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(AppError::Io(e)),
    }
}

#[tauri::command]
pub async fn write_download_catalog(path: String, contents: String) -> Result<(), AppError> {
    if let Some(parent) = Path::new(&path).parent() {
        fs::create_dir_all(parent).map_err(|e| AppError::Io(e))?;
    }
    fs::write(&path, contents).map_err(|e| AppError::Io(e))
}

#[tauri::command]
pub async fn move_download_catalog(from: String, to: String) -> Result<(), AppError> {
    if from.is_empty() || to.is_empty() || from == to {
        return Ok(());
    }
    if let Some(parent) = Path::new(&to).parent() {
        fs::create_dir_all(parent).map_err(|e| AppError::Io(e))?;
    }
    if !Path::new(&from).exists() {
        return Ok(());
    }
    fs::rename(&from, &to).map_err(|e| AppError::Io(e))
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
