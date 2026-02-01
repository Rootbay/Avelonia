use crate::AppError;
use rand::RngCore;
use rand::rngs::OsRng;
use rayon::prelude::*;
use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

#[tauri::command]
pub fn secure_erase(files: Vec<String>, passes: u32) -> Result<usize, AppError> {
    if passes == 0 {
        return Err(AppError::Internal("passes must be >= 1".to_string()));
    }
    let erased_count = AtomicUsize::new(0);

    files.into_par_iter().for_each(|path_str| {
        let path = PathBuf::from(&path_str);
        if !path.exists() || !path.is_file() {
            eprintln!("secure_erase: skipping non-file {}", path_str);
            return;
        }
        let metadata = match std::fs::metadata(&path) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("metadata failed {}: {}", path_str, e);
                return;
            }
        };
        let len = metadata.len();
        if len == 0 {
            if let Err(e) = std::fs::remove_file(&path) {
                eprintln!("remove_file failed {}: {}", path_str, e);
            } else {
                erased_count.fetch_add(1, Ordering::SeqCst);
            }
            return;
        }

        let mut file = match OpenOptions::new().write(true).open(&path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("open for write failed {}: {}", path_str, e);
                return;
            }
        };

        let mut buffer = vec![0u8; 1024 * 1024];
        let mut success = true;
        for _ in 0..passes {
            let mut remaining = len;
            if let Err(e) = file.seek(SeekFrom::Start(0)) {
                eprintln!("seek failed {}: {}", path_str, e);
                success = false;
                break;
            }
            while remaining > 0 {
                let chunk = std::cmp::min(remaining, buffer.len() as u64) as usize;
                OsRng.fill_bytes(&mut buffer[..chunk]);
                if let Err(e) = file.write_all(&buffer[..chunk]) {
                    eprintln!("write failed {}: {}", path_str, e);
                    success = false;
                    break;
                }
                remaining -= chunk as u64;
            }
            if !success {
                break;
            }
            if let Err(e) = file.flush() {
                eprintln!("flush failed {}: {}", path_str, e);
            }
        }
        drop(file);

        if success {
            match std::fs::remove_file(&path) {
                Ok(_) => {
                    erased_count.fetch_add(1, Ordering::SeqCst);
                }
                Err(e) => eprintln!("remove_file failed {}: {}", path_str, e),
            }
        }
    });

    Ok(erased_count.load(Ordering::SeqCst))
}
