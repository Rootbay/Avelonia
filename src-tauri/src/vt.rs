use dashmap::DashMap;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, State};

const DEFAULT_TTL_SECS: u64 = 2 * 24 * 60 * 60;
const PUBLIC_API_INTERVAL_SECS: u64 = 16;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    Clean,
    Suspicious,
    Malicious,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub sha256: String,
    pub verdict: Verdict,
    pub positives: u32,
    pub last_checked: u64,
    pub permalink: Option<String>,
    pub last_alerted: Option<u64>,
    #[serde(default)]
    pub malicious_count: u32,
    #[serde(default)]
    pub suspicious_count: u32,
    #[serde(default)]
    pub harmless_count: u32,
    #[serde(default)]
    pub undetected_count: u32,
}

#[derive(Default)]
pub struct VtState {
    api_key: Mutex<Option<String>>,
    cache: Arc<DashMap<String, CacheEntry>>,
    last_req: Mutex<Option<u64>>,
}

impl VtState {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct VtStatus {
    pub key_set: bool,
    pub cached_items: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct VtItemReport {
    pub subject: String,
    pub sha256: String,
    pub verdict: Verdict,
    pub positives: u32,
    pub permalink: Option<String>,
    pub source: String,
    pub malicious: u32,
    pub suspicious: u32,
    pub harmless: u32,
    pub undetected: u32,
    pub total_vendors: u32,
    pub reason: Option<String>,
}

fn epoch_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_secs()
}

fn get_config_dir() -> PathBuf {
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return PathBuf::from(appdata).join("avelonia");
    }
    std::env::temp_dir().join("avelonia")
}

fn cache_file_path() -> PathBuf { get_config_dir().join("vt_cache.json") }
fn key_file_path() -> PathBuf { get_config_dir().join("vt_key.json") }
fn snapshot_file_path() -> PathBuf { get_config_dir().join("vt_snapshot.json") }

fn load_cache_from_disk() -> HashMap<String, CacheEntry> {
    let path = cache_file_path();
    let mut out = HashMap::new();
    if let Ok(bytes) = fs::read(path) {
        if let Ok(map) = serde_json::from_slice::<HashMap<String, CacheEntry>>(&bytes) {
            out = map;
        }
    }
    out
}

fn save_cache_to_disk(cache: &DashMap<String, CacheEntry>) {
    let dir = get_config_dir();
    let _ = fs::create_dir_all(&dir);
    let path = cache_file_path();
    let mut snap: HashMap<String, CacheEntry> = HashMap::new();
    for kv in cache.iter() {
        snap.insert(kv.key().clone(), kv.value().clone());
    }
    if let Ok(json) = serde_json::to_vec_pretty(&snap) {
        let _ = fs::write(path, json);
    }
}

fn load_key_from_disk() -> Option<String> {
    let path = key_file_path();
    if let Ok(bytes) = fs::read(path) {
        #[derive(Deserialize)]
        struct KeyWrap { key: String }
        if let Ok(v) = serde_json::from_slice::<KeyWrap>(&bytes) { return Some(v.key); }
    }
    None
}

fn save_key_to_disk(key: &str) {
    let dir = get_config_dir();
    let _ = fs::create_dir_all(&dir);
    #[derive(Serialize)]
    struct KeyWrap<'a> { key: &'a str }
    let _ = fs::write(key_file_path(), serde_json::to_vec_pretty(&KeyWrap { key }).unwrap_or_default());
}

fn compute_sha256(path: &Path) -> Result<String, String> {
    let f = File::open(path).map_err(|e| format!("open failed: {}", e))?;
    let mut reader = BufReader::new(f);
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| format!("read failed: {}", e))?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

async fn vt_fetch(client: &Client, key: &str, sha256: &str) -> Result<CacheEntry, String> {
    let url = format!("https://www.virustotal.com/api/v3/files/{}", sha256);
    let resp = client
        .get(&url)
        .header("x-apikey", key)
        .send()
        .await
        .map_err(|e| format!("VT request failed: {}", e))?;

    if resp.status().as_u16() == 404 {
        return Ok(CacheEntry {
            sha256: sha256.to_string(),
            verdict: Verdict::Unknown,
            positives: 0,
            last_checked: epoch_now(),
            permalink: Some(format!("https://www.virustotal.com/gui/file/{}", sha256)),
            last_alerted: None,
            malicious_count: 0,
            suspicious_count: 0,
            harmless_count: 0,
            undetected_count: 0,
        });
    }
    let status = resp.status();
    if !status.is_success() {
        return Err(format!("VT HTTP {}", status));
    }

    #[derive(Deserialize)]
    struct RespRoot {
        data: Option<RespData>,
    }
    #[derive(Deserialize)]
    struct RespData {
        #[allow(dead_code)]
        id: String,
        #[allow(dead_code)]
        links: Option<RespLinks>,
        attributes: Option<RespAttrs>,
    }
    #[derive(Deserialize)]
    struct RespLinks {
        #[serde(rename = "self")]
        #[allow(dead_code)]
        self_: Option<String>,
    }
    #[derive(Deserialize)]
    struct RespAttrs { last_analysis_stats: Option<Stats> }
    #[derive(Deserialize)]
    struct Stats { malicious: Option<u32>, suspicious: Option<u32>, harmless: Option<u32>, undetected: Option<u32> }

    let root: RespRoot = resp
        .json()
        .await
        .map_err(|e| format!("VT parse failed: {}", e))?;

    let mut positives: u32 = 0;
    let mut verdict = Verdict::Unknown;
    let mut malicious: u32 = 0;
    let mut suspicious: u32 = 0;
    let mut harmless: u32 = 0;
    let mut undetected: u32 = 0;
    if let Some(attrs) = root.data.as_ref().and_then(|d| d.attributes.as_ref()) {
        if let Some(st) = attrs.last_analysis_stats.as_ref() {
            malicious = st.malicious.unwrap_or(0);
            suspicious = st.suspicious.unwrap_or(0);
            harmless = st.harmless.unwrap_or(0);
            undetected = st.undetected.unwrap_or(0);
            positives = malicious.saturating_add(suspicious);
            verdict = if malicious > 0 { Verdict::Malicious } else if suspicious > 0 { Verdict::Suspicious } else { Verdict::Clean };
        }
    }
    let link = Some(format!("https://www.virustotal.com/gui/file/{}", sha256));
    Ok(CacheEntry {
        sha256: sha256.to_string(),
        verdict,
        positives,
        last_checked: epoch_now(),
        permalink: link,
        last_alerted: None,
        malicious_count: malicious,
        suspicious_count: suspicious,
        harmless_count: harmless,
        undetected_count: undetected,
    })
}

async fn ensure_rate_limit(state: &VtState) {
    let wait_secs: u64 = {
        let now = epoch_now();
        let guard = state.last_req.lock().unwrap();
        if let Some(prev) = *guard {
            let diff = now.saturating_sub(prev);
            if diff < PUBLIC_API_INTERVAL_SECS {
                PUBLIC_API_INTERVAL_SECS - diff
            } else { 0 }
        } else { 0 }
    };
    if wait_secs > 0 { tokio::time::sleep(Duration::from_secs(wait_secs)).await; }
    let mut guard = state.last_req.lock().unwrap();
    *guard = Some(epoch_now());
}

async fn lookup_or_fetch(state: &State<'_, VtState>, sha256: &str, force: bool) -> Result<CacheEntry, String> {
    if !force {
        if let Some(entry) = state.cache.get(sha256) {
            let age = epoch_now().saturating_sub(entry.last_checked);
            if age < DEFAULT_TTL_SECS {
                return Ok(entry.clone());
            }
        }
    }
    let key_opt = state.api_key.lock().unwrap().clone().or_else(load_key_from_disk);
    let key = match key_opt { Some(k) => k, None => return Err("VirusTotal API key not set".into()) };
    let client = Client::builder()
        .user_agent("Avelonia/0.1 (vt)")
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;
    ensure_rate_limit(&*state).await;
    let fetched = vt_fetch(&client, &key, sha256).await?;
    state.cache.insert(sha256.to_string(), fetched.clone());
    save_cache_to_disk(&state.cache);
    Ok(fetched)
}

fn cache_needs_refresh(state: &VtState, sha256: &str) -> bool {
    if let Some(entry) = state.cache.get(sha256) {
        let age = epoch_now().saturating_sub(entry.last_checked);
        let is_clean = entry.verdict == Verdict::Clean;
        return !(is_clean && age < DEFAULT_TTL_SECS);
    }
    true
}

#[tauri::command]
pub fn vt_get_status(state: State<'_, VtState>) -> Result<VtStatus, String> {
    let key_set = state.api_key.lock().unwrap().is_some() || load_key_from_disk().is_some();
    Ok(VtStatus { key_set, cached_items: state.cache.len() })
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct VtSnapshot {
    last_scan: u64,
    startup: Vec<String>,
    registry: Vec<String>,
}

fn load_snapshot() -> VtSnapshot {
    if let Ok(bytes) = fs::read(snapshot_file_path()) {
        if let Ok(s) = serde_json::from_slice::<VtSnapshot>(&bytes) { return s; }
    }
    VtSnapshot::default()
}

fn save_snapshot(s: &VtSnapshot) {
    let _ = fs::create_dir_all(get_config_dir());
    if let Ok(js) = serde_json::to_vec_pretty(s) { let _ = fs::write(snapshot_file_path(), js); }
}

#[cfg(target_os = "windows")]
fn collect_startup_keys_for_snapshot() -> Vec<String> {
    let mut out = Vec::new();
    let items = resolve_startup_shortcut_targets();
    for (_d, p) in items { out.push(p); }
    out
}
#[cfg(not(target_os = "windows"))]
fn collect_startup_keys_for_snapshot() -> Vec<String> { Vec::new() }

#[cfg(target_os = "windows")]
fn collect_registry_keys_for_snapshot() -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(items) = crate::optimize::list_registry_run() {
        for it in items { out.push(format!("{}|{}|{}", it.hive, it.key, it.name)); }
    }
    out
}
#[cfg(not(target_os = "windows"))]
fn collect_registry_keys_for_snapshot() -> Vec<String> { Vec::new() }

fn build_current_snapshot(prev_last_scan: u64) -> VtSnapshot {
    VtSnapshot { last_scan: prev_last_scan, startup: collect_startup_keys_for_snapshot(), registry: collect_registry_keys_for_snapshot() }
}

fn has_new(prev: &VtSnapshot, cur: &VtSnapshot) -> bool {
    use std::collections::HashSet;
    let ps: HashSet<_> = prev.startup.iter().collect();
    let pr: HashSet<_> = prev.registry.iter().collect();
    cur.startup.iter().any(|k| !ps.contains(k)) || cur.registry.iter().any(|k| !pr.contains(k))
}

#[tauri::command]
pub fn vt_set_api_key(state: State<'_, VtState>, key: Option<String>, persist: Option<bool>) -> Result<(), String> {
    let mut guard = state.api_key.lock().unwrap();
    *guard = key.clone();
    if let Some(true) = persist {
        if let Some(k) = key { save_key_to_disk(&k); }
    }
    Ok(())
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub fn vt_scan_needed(state: State<'_, VtState>, limit: Option<u32>) -> Result<(usize, usize), String> {
    let limit = limit.unwrap_or(50).max(1).min(200) as usize;
    let mut need_startup = 0usize;
    let mut need_registry = 0usize;

    let items = resolve_startup_shortcut_targets();
    for (_display, path) in items.into_iter().take(limit) {
        let pb = PathBuf::from(&path);
        if !pb.exists() || !pb.is_file() { continue; }
        if let Ok(sha) = compute_sha256(&pb) {
            if cache_needs_refresh(&state, &sha) { need_startup += 1; }
        }
    }

    let items = resolve_registry_run_targets();
    for (_display, path, _hive, _key, _name) in items.into_iter().take(limit) {
        let pb = PathBuf::from(&path);
        if !pb.exists() || !pb.is_file() { continue; }
        if let Ok(sha) = compute_sha256(&pb) {
            if cache_needs_refresh(&state, &sha) { need_registry += 1; }
        }
    }

    Ok((need_startup, need_registry))
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub fn vt_scan_needed(_state: State<'_, VtState>, _limit: Option<u32>) -> Result<(usize, usize), String> { Ok((0, 0)) }

#[tauri::command]
pub fn vt_load_cache(state: State<'_, VtState>) -> Result<usize, String> {
    let map = load_cache_from_disk();
    for (k, v) in map.into_iter() { state.cache.insert(k, v); }
    Ok(state.cache.len())
}

#[cfg(target_os = "windows")]
fn resolve_startup_shortcut_targets() -> Vec<(String /*display*/, String /*path*/)> {
    let mut out = Vec::new();
    if let Ok(items) = crate::optimize::list_startup_shortcuts() {
        for it in items {
            let display = it.name.clone();
            let p = std::path::PathBuf::from(&it.path);
            if p.exists() && p.is_file() {
                if let Ok(link) = lnk::ShellLink::open(&p, lnk::encoding::WINDOWS_1252) {
                    if let Some(info) = link.link_info() {
                        let common = info.common_path_suffix().to_string();
                        let tgt = PathBuf::from(common);
                        if tgt.exists() {
                            out.push((display, tgt.display().to_string()));
                            continue;
                        }
                    }
                }
            }
            out.push((display, it.path));
        }
    }
    out
}

#[cfg(target_os = "windows")]
fn extract_exe_from_command(cmd: &str) -> Option<String> {
    fn expand_env_case_insensitive(input: &str) -> String {
        let mut out = input.to_string();
        for (k, v) in std::env::vars() {
            let up = k.to_uppercase();
            let low = k.to_lowercase();
            let pats = [format!("%{}%", k), format!("%{}%", up), format!("%{}%", low)];
            for pat in pats { out = out.replace(&pat, &v); }
        }
        out
    }
    let mut s = cmd.trim().to_string();
    if s.is_empty() { return None; }
    if s.ends_with('"') && !s.starts_with('"') { s = s.trim_end_matches('"').to_string(); }
    let mut first = if s.starts_with('"') {
        s.split('"').nth(1).unwrap_or("")
    } else {
        s.split_whitespace().next().unwrap_or("")
    };
    if first.is_empty() { return None; }
    let expanded = expand_env_case_insensitive(first);
    let mut p = PathBuf::from(&expanded);
    if p.exists() { return Some(p.display().to_string()); }
    first = first.trim_end_matches(&[',', ';', '.'][..]);
    let expanded2 = expand_env_case_insensitive(first);
    p = PathBuf::from(&expanded2);
    if p.exists() { return Some(p.display().to_string()); }

    if Path::new(&expanded2).extension().is_none() {
        let mut cand = PathBuf::from(&expanded2);
        cand.set_extension("exe");
        if cand.exists() { return Some(cand.display().to_string()); }
    }

    let lower = first.to_lowercase();
    if lower.ends_with(".exe") && !lower.contains('\\') && !lower.contains('/') {
        let mut roots: Vec<PathBuf> = Vec::new();
        if let Some(system_root) = std::env::var_os("SystemRoot").or_else(|| std::env::var_os("WINDIR")) {
            roots.push(PathBuf::from(&system_root).join("System32"));
            roots.push(PathBuf::from(&system_root).join("SysWOW64"));
        }
        if let Some(path) = std::env::var_os("PATH") {
            for part in std::env::split_paths(&path) { roots.push(part); }
        }
        for root in roots {
            let cand = root.join(&first);
            if cand.exists() { return Some(cand.display().to_string()); }
        }
    }

    let lower_all = s.to_lowercase();
    if let Some(idx) = lower_all.rfind(".exe") {
        let end = idx + 4;
        let start = s[..end].rfind(|c| c == ' ' || c == '"' || c == '\\' || c == '\t').map(|i| i + 1).unwrap_or(0);
        let token = s[start..end].trim_matches('"');
        if !token.is_empty() {
            let exp = expand_env_case_insensitive(token);
            let q = PathBuf::from(exp.trim_matches('"'));
            if q.exists() { return Some(q.display().to_string()); }
            if let Some(name) = Path::new(token).file_name().map(|x| x.to_string_lossy().to_string()) {
                let mut roots: Vec<PathBuf> = Vec::new();
                if let Some(system_root) = std::env::var_os("SystemRoot").or_else(|| std::env::var_os("WINDIR")) {
                    roots.push(PathBuf::from(&system_root).join("System32"));
                    roots.push(PathBuf::from(&system_root).join("SysWOW64"));
                }
                if let Some(path) = std::env::var_os("PATH") {
                    for part in std::env::split_paths(&path) { roots.push(part); }
                }
                for root in roots {
                    let cand = root.join(&name);
                    if cand.exists() { return Some(cand.display().to_string()); }
                }
            }
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn resolve_registry_run_targets() -> Vec<(String /*display*/, String /*path*/, String /*hive*/, String /*key*/, String /*name*/)> {
    let mut out = Vec::new();
    if let Ok(items) = crate::optimize::list_registry_run() {
        for it in items {
            if let Some(img) = extract_exe_from_command(&it.command) {
                out.push((it.command.clone(), img, it.hive, it.key, it.name));
            }
        }
    }
    out
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub async fn vt_scan_startup(app: AppHandle, state: State<'_, VtState>, limit: Option<u32>, force: Option<bool>) -> Result<Vec<VtItemReport>, String> {
    let _ = vt_load_cache(state.clone());
    let limit = limit.unwrap_or(10).max(1).min(50) as usize;
    let force = force.unwrap_or(false);
    let items = resolve_startup_shortcut_targets();
    let mut out: Vec<VtItemReport> = Vec::new();
    for (display, path) in items.into_iter().take(limit) {
        let mut pb = PathBuf::from(&path);
        if let Some(ext) = pb.extension().and_then(|s| s.to_str()).map(|s| s.to_lowercase()) {
            if ext == "url" {
                let rep = VtItemReport { subject: display.clone(), sha256: String::new(), verdict: Verdict::Unknown, positives: 0, permalink: None, source: "startup".into(), malicious: 0, suspicious: 0, harmless: 0, undetected: 0, total_vendors: 0, reason: Some("no-executable".into()) };
                out.push(rep.clone()); let _ = app.emit("vt-report", &rep); continue;
            }
            if ext == "lnk" {
                if let Ok(link) = lnk::ShellLink::open(&pb, lnk::encoding::WINDOWS_1252) {
                    if let Some(info) = link.link_info() {
                        let tgt = PathBuf::from(info.common_path_suffix().to_string());
                        if tgt.exists() { pb = tgt; } else {
                            let rep = VtItemReport { subject: display.clone(), sha256: String::new(), verdict: Verdict::Unknown, positives: 0, permalink: None, source: "startup".into(), malicious: 0, suspicious: 0, harmless: 0, undetected: 0, total_vendors: 0, reason: Some("no-executable".into()) };
                            out.push(rep.clone()); let _ = app.emit("vt-report", &rep); continue;
                        }
                    } else {
                        let rep = VtItemReport { subject: display.clone(), sha256: String::new(), verdict: Verdict::Unknown, positives: 0, permalink: None, source: "startup".into(), malicious: 0, suspicious: 0, harmless: 0, undetected: 0, total_vendors: 0, reason: Some("no-executable".into()) };
                        out.push(rep.clone()); let _ = app.emit("vt-report", &rep); continue;
                    }
                } else {
                    let rep = VtItemReport { subject: display.clone(), sha256: String::new(), verdict: Verdict::Unknown, positives: 0, permalink: None, source: "startup".into(), malicious: 0, suspicious: 0, harmless: 0, undetected: 0, total_vendors: 0, reason: Some("no-executable".into()) };
                    out.push(rep.clone()); let _ = app.emit("vt-report", &rep); continue;
                }
            }
        }
        if !pb.exists() || !pb.is_file() {
            let rep = VtItemReport {
                subject: display.clone(),
                sha256: String::new(),
                verdict: Verdict::Unknown,
                positives: 0,
                permalink: None,
                source: "startup".into(),
                malicious: 0,
                suspicious: 0,
                harmless: 0,
                undetected: 0,
                total_vendors: 0,
                reason: Some("file-missing".into()),
            };
            out.push(rep.clone());
            let _ = app.emit("vt-report", &rep);
            continue;
        }
        let sha = match compute_sha256(&pb) { Ok(h) => h, Err(_) => {
            let rep = VtItemReport {
                subject: display.clone(),
                sha256: String::new(),
                verdict: Verdict::Unknown,
                positives: 0,
                permalink: None,
                source: "startup".into(),
                malicious: 0,
                suspicious: 0,
                harmless: 0,
                undetected: 0,
                total_vendors: 0,
                reason: Some("hashing-failed".into()),
            };
            out.push(rep.clone());
            let _ = app.emit("vt-report", &rep);
            continue
        } };
        match lookup_or_fetch(&state, &sha, force).await {
            Ok(entry) => {
                let total = entry.malicious_count + entry.suspicious_count + entry.harmless_count + entry.undetected_count;
                let rep = VtItemReport {
                    subject: display.clone(),
                    sha256: entry.sha256.clone(),
                    verdict: entry.verdict.clone(),
                    positives: entry.positives,
                    permalink: entry.permalink.clone(),
                    source: "startup".into(),
                    malicious: entry.malicious_count,
                    suspicious: entry.suspicious_count,
                    harmless: entry.harmless_count,
                    undetected: entry.undetected_count,
                    total_vendors: total,
                    reason: None,
                };
                out.push(rep.clone());
                let _ = app.emit("vt-report", &rep);
                if matches!(entry.verdict, Verdict::Malicious | Verdict::Suspicious) {
                    let mut need_emit = true;
                    if let Some(mut cached) = state.cache.get_mut(&entry.sha256) {
                        let last = cached.last_alerted.unwrap_or(0);
                        if epoch_now().saturating_sub(last) < DEFAULT_TTL_SECS { need_emit = false; }
                        if need_emit { cached.last_alerted = Some(epoch_now()); }
                    }
                    if need_emit {
                        let _ = app.emit("vt-alert", &rep);
                        save_cache_to_disk(&state.cache);
                    }
                }
            }
            Err(e) => {
                let es = e.to_lowercase();
                let reason = if es.contains("api key not set") { "no-api-key" }
                    else if es.contains("http 429") { "rate-limited" }
                    else if es.contains("http ") { "http-error" }
                    else if es.contains("request failed") { "network-error" }
                    else { "unknown-error" };
                let rep = VtItemReport {
                    subject: display.clone(),
                    sha256: sha.clone(),
                    verdict: Verdict::Unknown,
                    positives: 0,
                    permalink: Some(format!("https://www.virustotal.com/gui/file/{}", sha)),
                    source: "startup".into(),
                    malicious: 0,
                    suspicious: 0,
                    harmless: 0,
                    undetected: 0,
                    total_vendors: 0,
                    reason: Some(reason.into()),
                };
                out.push(rep.clone());
                let _ = app.emit("vt-report", &rep);
            }
        }
    }
    Ok(out)
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub async fn vt_scan_registry(app: AppHandle, state: State<'_, VtState>, limit: Option<u32>, force: Option<bool>) -> Result<Vec<VtItemReport>, String> {
    let _ = vt_load_cache(state.clone());
    let limit = limit.unwrap_or(10).max(1).min(50) as usize;
    let force = force.unwrap_or(false);
    let mut out: Vec<VtItemReport> = Vec::new();
    let items_full = crate::optimize::list_registry_run().unwrap_or_default();
    for it in items_full.into_iter().take(limit) {
        let display = it.command.clone();
        let name = it.name.clone();
        let maybe_img = extract_exe_from_command(&it.command);
        if maybe_img.is_none() {
            let subj = if !name.trim().is_empty() { name.clone() } else { display.clone() };
            let rep = VtItemReport {
                subject: subj,
                sha256: String::new(),
                verdict: Verdict::Unknown,
                positives: 0,
                permalink: None,
                source: "registry".into(),
                malicious: 0,
                suspicious: 0,
                harmless: 0,
                undetected: 0,
                total_vendors: 0,
                reason: Some("no-executable".into()),
            };
            out.push(rep.clone());
            let _ = app.emit("vt-report", &rep);
            continue;
        }
        let path = maybe_img.unwrap();
        let pb = PathBuf::from(&path);
        if !pb.exists() || !pb.is_file() {
            let rep = VtItemReport {
                subject: if !name.trim().is_empty() { name.clone() } else { display.clone() },
                sha256: String::new(),
                verdict: Verdict::Unknown,
                positives: 0,
                permalink: None,
                source: "registry".into(),
                malicious: 0,
                suspicious: 0,
                harmless: 0,
                undetected: 0,
                total_vendors: 0,
                reason: Some("file-missing".into()),
            };
            out.push(rep.clone());
            let _ = app.emit("vt-report", &rep);
            continue;
        }
        let sha = match compute_sha256(&pb) { Ok(h) => h, Err(_) => {
            let rep = VtItemReport {
                subject: if !name.trim().is_empty() { name.clone() } else { display.clone() },
                sha256: String::new(),
                verdict: Verdict::Unknown,
                positives: 0,
                permalink: None,
                source: "registry".into(),
                malicious: 0,
                suspicious: 0,
                harmless: 0,
                undetected: 0,
                total_vendors: 0,
                reason: Some("hashing-failed".into()),
            };
            out.push(rep.clone());
            let _ = app.emit("vt-report", &rep);
            continue
        } };
        match lookup_or_fetch(&state, &sha, force).await {
            Ok(entry) => {
                let subj = if !name.trim().is_empty() { name.clone() } else { display.clone() };
                let total = entry.malicious_count + entry.suspicious_count + entry.harmless_count + entry.undetected_count;
                let rep = VtItemReport {
                    subject: subj,
                    sha256: entry.sha256.clone(),
                    verdict: entry.verdict.clone(),
                    positives: entry.positives,
                    permalink: entry.permalink.clone(),
                    source: "registry".into(),
                    malicious: entry.malicious_count,
                    suspicious: entry.suspicious_count,
                    harmless: entry.harmless_count,
                    undetected: entry.undetected_count,
                    total_vendors: total,
                    reason: None,
                };
                out.push(rep.clone());
                let _ = app.emit("vt-report", &rep);
                if matches!(entry.verdict, Verdict::Malicious | Verdict::Suspicious) {
                    let mut need_emit = true;
                    if let Some(mut cached) = state.cache.get_mut(&entry.sha256) {
                        let last = cached.last_alerted.unwrap_or(0);
                        if epoch_now().saturating_sub(last) < DEFAULT_TTL_SECS { need_emit = false; }
                        if need_emit { cached.last_alerted = Some(epoch_now()); }
                    }
                    if need_emit { let _ = app.emit("vt-alert", &rep); save_cache_to_disk(&state.cache); }
                }
            }
            Err(e) => {
                let es = e.to_lowercase();
                let reason = if es.contains("api key not set") { "no-api-key" }
                    else if es.contains("http 429") { "rate-limited" }
                    else if es.contains("http ") { "http-error" }
                    else if es.contains("request failed") { "network-error" }
                    else { "unknown-error" };
                let subj = if !name.trim().is_empty() { name.clone() } else { display.clone() };
                let rep = VtItemReport {
                    subject: subj,
                    sha256: sha.clone(),
                    verdict: Verdict::Unknown,
                    positives: 0,
                    permalink: Some(format!("https://www.virustotal.com/gui/file/{}", sha)),
                    source: "registry".into(),
                    malicious: 0,
                    suspicious: 0,
                    harmless: 0,
                    undetected: 0,
                    total_vendors: 0,
                    reason: Some(reason.into()),
                };
                out.push(rep.clone());
                let _ = app.emit("vt-report", &rep);
            }
        }
    }
    Ok(out)
}

#[tauri::command]
#[cfg(target_os = "windows")]
pub async fn vt_scan_all(app: AppHandle, state: State<'_, VtState>, limit: Option<u32>, force: Option<bool>) -> Result<(usize, usize), String> {
    let n1 = vt_scan_startup(app.clone(), state.clone(), limit, force).await?.len();
    let n2 = vt_scan_registry(app, state, limit, force).await?.len();
    let prev = load_snapshot();
    let mut cur = build_current_snapshot(prev.last_scan);
    cur.last_scan = epoch_now();
    save_snapshot(&cur);
    Ok((n1, n2))
}

#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub async fn vt_scan_startup(_app: AppHandle, _state: State<'_, VtState>, _limit: Option<u32>) -> Result<Vec<VtItemReport>, String> { Ok(Vec::new()) }
#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub async fn vt_scan_registry(_app: AppHandle, _state: State<'_, VtState>, _limit: Option<u32>) -> Result<Vec<VtItemReport>, String> { Ok(Vec::new()) }
#[tauri::command]
#[cfg(not(target_os = "windows"))]
pub async fn vt_scan_all(_app: AppHandle, _state: State<'_, VtState>, _limit: Option<u32>) -> Result<(usize, usize), String> { Ok((0,0)) }

#[tauri::command]
pub async fn vt_auto_maybe_scan(app: AppHandle, state: State<'_, VtState>) -> Result<Option<String>, String> {
    let prev = load_snapshot();
    let mut cur = build_current_snapshot(prev.last_scan);
    let key_present = state.api_key.lock().unwrap().is_some() || load_key_from_disk().is_some();
    let now = epoch_now();
    let reason = if has_new(&prev, &cur) { Some("new-items") } else if prev.last_scan == 0 || now.saturating_sub(prev.last_scan) >= DEFAULT_TTL_SECS { Some("ttl") } else { None };
    if let Some(r) = reason {
        if !key_present { let _ = app.emit("vt-autoscan-skip", &serde_json::json!({"reason": r})); return Ok(None); }
        let _ = app.emit("vt-autoscan-start", &serde_json::json!({"reason": r}));
        let (n1, n2) = vt_scan_all(app.clone(), state, Some(50), Some(false)).await?;
        cur.last_scan = epoch_now();
        save_snapshot(&cur);
        let _ = app.emit("vt-autoscan-done", &serde_json::json!({"reason": r, "startup": n1, "registry": n2}));
        return Ok(Some(r.to_string()));
    }
    let _ = app.emit("vt-autoscan-skip", &serde_json::json!({"reason": "no-change"}));
    Ok(None)
}
