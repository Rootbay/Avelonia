use std::path::PathBuf;
use std::env;

pub struct WindowsPaths {
    pub temp: PathBuf,
    pub local_app_data: PathBuf,
    pub app_data: PathBuf,
    pub windir: PathBuf,
    pub program_data: PathBuf,
    pub user_profile: PathBuf,
}

impl WindowsPaths {
    pub fn get() -> Self {
        Self {
            temp: env::var_os("TEMP").map(PathBuf::from).unwrap_or_default(),
            local_app_data: env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_default(),
            app_data: env::var_os("APPDATA").map(PathBuf::from).unwrap_or_default(),
            windir: env::var_os("WINDIR").map(PathBuf::from).unwrap_or_default(),
            program_data: env::var_os("PROGRAMDATA").or(env::var_os("ALLUSERSPROFILE")).map(PathBuf::from).unwrap_or_default(),
            user_profile: env::var_os("USERPROFILE").map(PathBuf::from).unwrap_or_default(),
        }
    }

    pub fn startup_user(&self) -> PathBuf {
        self.app_data.join("Microsoft/Windows/Start Menu/Programs/Startup")
    }

    pub fn startup_common(&self) -> PathBuf {
        self.program_data.join("Microsoft/Windows/Start Menu/Programs/Startup")
    }
}
