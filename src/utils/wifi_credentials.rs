use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

const CREDS_FILE: &str = ".cache/sierra/wifi_credentials.json";

/// In-memory store protected by a Mutex, loaded once on first access.
static STORE: OnceLock<Arc<Mutex<WifiCredStore>>> = OnceLock::new();

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct WifiCredStore {
    /// ssid → password  (empty string for open networks)
    pub credentials: HashMap<String, String>,
}

fn cache_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(CREDS_FILE)
}

fn load_store() -> WifiCredStore {
    let path = cache_path();
    if let Ok(content) = fs::read_to_string(&path) {
        if let Ok(store) = serde_json::from_str::<WifiCredStore>(&content) {
            eprintln!("[WifiCreds] Loaded {} saved credentials", store.credentials.len());
            return store;
        }
    }
    WifiCredStore::default()
}

fn save_store(store: &WifiCredStore) {
    let path = cache_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(store) {
        let _ = fs::write(&path, json);
    }
}

fn get_store() -> &'static Arc<Mutex<WifiCredStore>> {
    STORE.get_or_init(|| Arc::new(Mutex::new(load_store())))
}

/// Retrieve a saved password for an SSID, if any.
pub fn get_password(ssid: &str) -> Option<String> {
    let store = get_store().lock().ok()?;
    store.credentials.get(ssid).cloned()
}

/// Save / update the password for an SSID.
/// Pass an empty string for open networks (records that we know this network).
pub fn save_password(ssid: &str, password: &str) {
    if let Ok(mut store) = get_store().lock() {
        store.credentials.insert(ssid.to_string(), password.to_string());
        save_store(&store);
        eprintln!("[WifiCreds] Saved credential for '{}'", ssid);
    }
}

/// Remove stored credential for an SSID.
pub fn forget_password(ssid: &str) {
    if let Ok(mut store) = get_store().lock() {
        store.credentials.remove(ssid);
        save_store(&store);
        eprintln!("[WifiCreds] Forgot credential for '{}'", ssid);
    }
}

/// Returns true if we have a saved password (or know it's open) for this SSID.
pub fn has_saved(ssid: &str) -> bool {
    get_store()
        .lock()
        .map(|s| s.credentials.contains_key(ssid))
        .unwrap_or(false)
}

/// Return all saved SSIDs.
pub fn all_saved_ssids() -> Vec<String> {
    get_store()
        .lock()
        .map(|s| s.credentials.keys().cloned().collect())
        .unwrap_or_default()
}
