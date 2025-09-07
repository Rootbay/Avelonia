use sysinfo::System;
use std::thread;
use std::time::Duration;

#[tauri::command]
pub fn get_cpu_usage() -> Result<f32, String> {
    let mut system = System::new();
    system.refresh_all();
    thread::sleep(Duration::from_millis(500));
    system.refresh_all();
    Ok(system.global_cpu_usage())
}

#[tauri::command]
pub fn get_memory_usage() -> Result<u64, String> {
    let mut system = System::new();
    system.refresh_memory();
    Ok(system.used_memory())
}

#[tauri::command]
pub fn get_total_memory() -> Result<u64, String> {
    let mut system = System::new();
    system.refresh_memory();
    Ok(system.total_memory())
}