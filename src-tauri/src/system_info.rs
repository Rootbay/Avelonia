use std::sync::{Arc, Mutex};
use sysinfo::System;
use tauri::State;

pub struct SystemState {
    pub sys: Mutex<System>,
    pub cpu_usage: Mutex<f32>,
}

impl SystemState {
    pub fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_cpu_all();
        Self {
            sys: Mutex::new(sys),
            cpu_usage: Mutex::new(0.0),
        }
    }
}

#[tauri::command]
pub fn get_cpu_usage(state: State<'_, Arc<SystemState>>) -> f32 {
    *state.cpu_usage.lock().unwrap()
}

#[tauri::command]
pub fn get_memory_usage(state: State<'_, Arc<SystemState>>) -> u64 {
    let mut sys = state.sys.lock().unwrap();
    sys.refresh_memory();
    sys.used_memory()
}

#[tauri::command]
pub fn get_total_memory(state: State<'_, Arc<SystemState>>) -> u64 {
    let mut sys = state.sys.lock().unwrap();
    sys.refresh_memory();
    sys.total_memory()
}

#[tauri::command]
pub fn get_boot_time() -> u64 {
    System::boot_time()
}