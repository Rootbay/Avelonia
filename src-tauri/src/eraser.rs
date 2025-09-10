use rand::rngs::OsRng;
use rand::RngCore;
use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};
use std::path::PathBuf;

#[tauri::command]
pub fn secure_erase(files: Vec<String>, passes: u32) -> Result<usize, String> {
    if passes == 0 { return Err("passes must be >= 1".into()); }
    let mut erased = 0usize;

    for path_str in files {
        let path = PathBuf::from(&path_str);
        if !path.exists() || !path.is_file() {
            eprintln!("secure_erase: skipping non-file {}", path_str);
            continue;
        }
        let metadata = match std::fs::metadata(&path) {
            Ok(m) => m,
            Err(e) => { eprintln!("metadata failed {}: {}", path_str, e); continue; }
        };
        let len = metadata.len();
        // If size is zero, just delete
        if len == 0 {
            if let Err(e) = std::fs::remove_file(&path) { eprintln!("remove_file failed {}: {}", path_str, e); }
            else { erased += 1; }
            continue;
        }

        let mut file = match OpenOptions::new().write(true).open(&path) {
            Ok(f) => f,
            Err(e) => { eprintln!("open for write failed {}: {}", path_str, e); continue; }
        };

        let mut buffer = vec![0u8; 1024 * 1024];
        for _ in 0..passes {
            let mut remaining = len;
            file.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
            while remaining > 0 {
                let chunk = std::cmp::min(remaining, buffer.len() as u64) as usize;
                OsRng.fill_bytes(&mut buffer[..chunk]);
                if let Err(e) = file.write_all(&buffer[..chunk]) { eprintln!("write failed {}: {}", path_str, e); break; }
                remaining -= chunk as u64;
            }
            if let Err(e) = file.flush() { eprintln!("flush failed {}: {}", path_str, e); }
        }
        drop(file);

        match std::fs::remove_file(&path) {
            Ok(_) => erased += 1,
            Err(e) => eprintln!("remove_file failed {}: {}", path_str, e),
        }
    }

    Ok(erased)
}

