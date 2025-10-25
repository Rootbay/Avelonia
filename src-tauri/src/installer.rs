#[tauri::command]
#[cfg(target_os = "windows")]
pub fn silent_install(path: String, elevate: bool) -> Result<(), String> {
    use std::path::PathBuf;
    use std::process::Command;
    let pb = PathBuf::from(&path);
    if !pb.exists() { return Err(format!("installer not found: {}", path)); }
    let ext = pb.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();

    // Helper: Start-Process -Verb RunAs and wait
    fn run_process_elevated(file: &str, args: &[&str]) -> bool {
        let arglist = {
            let items: Vec<String> = args.iter().map(|a| format!("'{}'", a.replace('\'', "''"))).collect();
            format!("@({})", items.join(", "))
        };
        let ps = format!(
            "Start-Process -FilePath '{}' -ArgumentList {} -Verb RunAs -Wait; exit $LASTEXITCODE",
            file.replace('\'', "''"), arglist
        );
        Command::new("powershell")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &ps])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    if ext == "msi" {
        let args = ["/i", &path, "/qn", "/norestart", "ALLUSERS=1"]; // machine-wide when possible
        let ok = if elevate { run_process_elevated("msiexec", &args) } else {
            Command::new("msiexec").args(args).status().map(|s| s.success()).unwrap_or(false)
        };
        if ok { return Ok(()); }
        return Err("msiexec failed".into());
    }

    if ext == "exe" {
        let combos: Vec<Vec<&str>> = vec![
            vec!["/S"],
            vec!["/SILENT"],
            vec!["/silent"],
            vec!["/VERYSILENT", "/NORESTART", "/SP-", "/SUPPRESSMSGBOXES"],
            vec!["/quiet"],
        ];
        for combo in combos {
            let ok = if elevate { run_process_elevated(&path, &combo) } else {
                Command::new(&path).args(&combo).status().map(|s| s.success()).unwrap_or(false)
            };
            if ok { return Ok(()); }
        }
        return Err("no silent flag combination succeeded".into());
    }

    Err("unsupported installer type (expecting .msi or .exe)".into())
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn silent_install(_path: String, _elevate: bool) -> Result<(), String> {
    Err("Silent install is only supported on Windows in this build".into())
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub fn launch_installer(path: String, elevate: bool) -> Result<(), String> {
    use std::path::PathBuf;
    use std::process::Command;
    let pb = PathBuf::from(&path);
    if !pb.exists() { return Err(format!("installer not found: {}", path)); }
    let ext = pb.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();

    fn run_process_elevated(file: &str, args: &[&str]) -> bool {
        let arglist = {
            let items: Vec<String> = args.iter().map(|a| format!("'{}'", a.replace('\'', "''"))).collect();
            format!("@({})", items.join(", "))
        };
        let ps = format!(
            "Start-Process -FilePath '{}' -ArgumentList {} -Verb RunAs -Wait; exit $LASTEXITCODE",
            file.replace('\'', "''"), arglist
        );
        Command::new("powershell")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &ps])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    if ext == "msi" {
        let args = ["/i", &path];
        let ok = if elevate { run_process_elevated("msiexec", &args) } else {
            Command::new("msiexec").args(args).status().map(|s| s.success()).unwrap_or(false)
        };
        if ok { return Ok(()); }
        return Err("msiexec launch failed".into());
    }
    // EXE: no args
    let ok = if elevate { run_process_elevated(&path, &[]) } else {
        Command::new(&path).status().map(|s| s.success()).unwrap_or(false)
    };
    if ok { Ok(()) } else { Err("failed to start installer".into()) }
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn launch_installer(_path: String, _elevate: bool) -> Result<(), String> {
    Err("Installer launch is only supported on Windows in this build".into())
}
