use std::sync::Arc;

mod cleaner;
mod eraser;
mod installer;
mod optimize;
mod system_info;
mod vt;
mod paths;
mod error;
mod downloads;

pub use error::AppError;
pub use downloads::DownloadState;

#[tauri::command]
fn path_exists(path: String) -> Result<bool, AppError> {
    Ok(std::path::Path::new(&path).exists())
}

#[tauri::command]
async fn verify_hash(path: String, expected_hash: String) -> Result<bool, AppError> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    use std::fs::File;

    let mut file =
        File::open(&path).map_err(|e| AppError::Io(e))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65536];

    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|e| AppError::Io(e))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }

    let result = hasher.finalize();
    let actual_hash = format!("{:x}", result);

    Ok(actual_hash.to_lowercase() == expected_hash.to_lowercase())
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
            downloads::download_file,
            downloads::cancel_download,
            downloads::probe_download,
            downloads::read_download_catalog,
            downloads::write_download_catalog,
            downloads::move_download_catalog,
            verify_hash,
            downloads::cleanup_orphaned_downloads,
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