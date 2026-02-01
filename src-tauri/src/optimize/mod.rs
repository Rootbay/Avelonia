pub mod fix_actions;
pub mod network;
pub mod services;
pub mod shell_helpers;
pub mod startup;
pub mod system;
pub mod tasks;
pub mod tweaks;
pub mod update_profiles;

pub use startup::*;
pub use network::*;
pub use tasks::*;
pub use system::*;
pub use services::*;

#[tauri::command]
pub fn apply_tweaks(payload: tweaks::TweakApplyRequest) -> Result<tweaks::TweakApplyResponse, String> {
    tweaks::apply_tweaks(payload)
}

#[tauri::command]
pub fn run_fix_action(action_id: String) -> Result<String, String> {
    fix_actions::run_fix_action(action_id)
}

#[tauri::command]
pub fn apply_update_profile(profile: String) -> Result<String, String> {
    update_profiles::apply_update_profile(profile)
}