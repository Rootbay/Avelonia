use std::env;
use std::path::PathBuf;

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
        let temp = env::var_os("TEMP")
            .map(PathBuf::from)
            .unwrap_or_else(|| env::temp_dir());
        let user_profile = env::var_os("USERPROFILE")
            .or_else(|| env::var_os("HOME"))
            .map(PathBuf::from)
            .unwrap_or_default();
        let local_app_data = env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                #[cfg(target_os = "macos")]
                {
                    user_profile.join("Library/Caches")
                }
                #[cfg(not(target_os = "macos"))]
                {
                    user_profile.join(".cache")
                }
            });
        let app_data = env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                #[cfg(target_os = "macos")]
                {
                    user_profile.join("Library/Application Support")
                }
                #[cfg(not(target_os = "macos"))]
                {
                    user_profile.join(".config")
                }
            });
        let windir = env::var_os("WINDIR").map(PathBuf::from).unwrap_or_default();
        let program_data = env::var_os("PROGRAMDATA")
            .or_else(|| env::var_os("ALLUSERSPROFILE"))
            .map(PathBuf::from)
            .unwrap_or_default();

        Self {
            temp,
            local_app_data,
            app_data,
            windir,
            program_data,
            user_profile,
        }
    }

    pub fn startup_user(&self) -> PathBuf {
        #[cfg(target_os = "windows")]
        {
            self.app_data
                .join("Microsoft/Windows/Start Menu/Programs/Startup")
        }
        #[cfg(target_os = "macos")]
        {
            self.user_profile.join("Library/LaunchAgents")
        }
        #[cfg(target_os = "linux")]
        {
            self.app_data.join("autostart")
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            self.app_data.join("Startup")
        }
    }

    pub fn startup_common(&self) -> PathBuf {
        #[cfg(target_os = "windows")]
        {
            self.program_data
                .join("Microsoft/Windows/Start Menu/Programs/Startup")
        }
        #[cfg(target_os = "macos")]
        {
            PathBuf::from("/Library/LaunchAgents")
        }
        #[cfg(target_os = "linux")]
        {
            PathBuf::from("/etc/xdg/autostart")
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            self.program_data.join("Startup")
        }
    }
}
