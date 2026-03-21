use std::process::Command;
use regex::Regex;

pub fn get_volume() -> Option<f32> {
    let output = run_pactl(&["get-sink-volume", "@DEFAULT_SINK@"]).ok()?;
    
    let re = Regex::new(r"(\d+)%").ok()?;
    let caps = re.captures(&output)?;
    caps.get(1)?.as_str().parse::<f32>().ok()
}

pub fn set_volume_cmd(value: u8) {
    if run_pactl(&["suspend-sink", "@DEFAULT_SINK@", "0"]).is_err() {
        eprintln!("[Audio] Warning: Failed to resume sink");
    }
    
    let volume_str = format!("{}%", value);
    
    if let Err(e) = run_pactl(&["set-sink-volume", "@DEFAULT_SINK@", &volume_str]) {
        eprintln!("[Audio] Failed to set volume: {}", e);
    }
}


pub fn get_mute_state() -> bool {
    run_pactl(&["get-sink-mute", "@DEFAULT_SINK@"])
        .ok()
        .map(|s| s.to_lowercase().contains("yes"))
        .unwrap_or(false)
}

pub fn set_mute_cmd(muted: bool) {
    let state = if muted { "1" } else { "0" };
    let _ = run_pactl(&["set-sink-mute", "@DEFAULT_SINK@", state]);
}

fn run_pactl(args: &[&str]) -> Result<String, String> {
    let output = Command::new("pactl")
        .args(args)
        .output()
        .map_err(|e| format!("pactl failed: {}", e))?;
    
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}


pub fn get_brightness() -> Option<f32> {
    let current_output = Command::new("brightnessctl").arg("g").output().ok()?;
    let current = String::from_utf8_lossy(&current_output.stdout).trim().parse::<f32>().ok()?;

    let max_output = Command::new("brightnessctl").arg("m").output().ok()?;
    let max = String::from_utf8_lossy(&max_output.stdout).trim().parse::<f32>().ok()?;

    if max > 0.0 { Some((current / max) * 100.0) } else { None }
}

pub fn set_brightness_cmd(value: u8) {
    let _ = Command::new("brightnessctl").arg("s").arg(format!("{}%", value)).output();
}


/// Robustly fetch wifi status: (enabled, ssid_or_status_string).
///
/// Strategy (most-reliable first):
/// 1. `nmcli -t -f NAME,TYPE,STATE dev` — look for a wifi device in "connected" state,
///    then read its active connection name via `nmcli -t -f GENERAL.CONNECTION dev show <iface>`.
/// 2. `nmcli con show --active` — look for a wifi type connection.
/// 3. `iwgetid -r` fallback.
/// 4. Check if radio is off.
pub fn fetch_wifi_status() -> (bool, String) {
    // ── Step 1: find connected wifi interface ────────────────────────────
    if let Ok(output) = Command::new("nmcli")
        .args(&["-t", "-f", "DEVICE,TYPE,STATE", "dev"])
        .output()
    {
        if let Ok(stdout) = String::from_utf8(output.stdout) {
            for line in stdout.lines() {
                // format: DEVICE:TYPE:STATE
                let parts: Vec<&str> = line.splitn(3, ':').collect();
                if parts.len() < 3 { continue; }
                let iface = parts[0].trim();
                let dev_type = parts[1].trim();
                let state = parts[2].trim();

                if dev_type == "wifi" && state == "connected" {
                    // Get the active connection name for this interface
                    if let Ok(show_out) = Command::new("nmcli")
                        .args(&["-t", "-f", "GENERAL.CONNECTION", "dev", "show", iface])
                        .output()
                    {
                        if let Ok(show_str) = String::from_utf8(show_out.stdout) {
                            for sline in show_str.lines() {
                                // format: GENERAL.CONNECTION:ssid name
                                if let Some(ssid) = sline.strip_prefix("GENERAL.CONNECTION:") {
                                    let ssid = ssid.trim();
                                    if !ssid.is_empty() && ssid != "--" {
                                        return (true, ssid.to_string());
                                    }
                                }
                            }
                        }
                    }
                    // Fallback: return iface name at minimum
                    return (true, "Connected".to_string());
                }
            }
        }
    }

    // ── Step 2: active connections ────────────────────────────────────────
    if let Ok(output) = Command::new("nmcli")
        .args(&["-t", "-f", "NAME,TYPE", "con", "show", "--active"])
        .output()
    {
        if let Ok(stdout) = String::from_utf8(output.stdout) {
            for line in stdout.lines() {
                // format: NAME:TYPE  (NAME may contain colons — use rfind)
                if let Some(colon) = line.rfind(':') {
                    let con_type = line[colon + 1..].trim();
                    if con_type == "802-11-wireless" || con_type == "wifi" {
                        let name = line[..colon].trim();
                        if !name.is_empty() {
                            return (true, name.to_string());
                        }
                    }
                }
            }
        }
    }

    // ── Step 3: iwgetid ───────────────────────────────────────────────────
    if let Ok(output) = Command::new("iwgetid").arg("-r").output() {
        if let Ok(ssid) = String::from_utf8(output.stdout) {
            let ssid = ssid.trim();
            if !ssid.is_empty() {
                return (true, ssid.to_string());
            }
        }
    }

    // ── Step 4: is radio off? ─────────────────────────────────────────────
    if let Ok(output) = Command::new("nmcli").args(&["radio", "wifi"]).output() {
        if let Ok(stdout) = String::from_utf8(output.stdout) {
            if stdout.trim() == "disabled" {
                return (false, "WiFi Off".to_string());
            }
        }
    }

    (false, "No Network".to_string())
}

pub fn toggle_wifi_cmd(enable: bool) {
    let state = if enable { "on" } else { "off" };
    let _ = Command::new("nmcli").args(&["radio", "wifi", state]).output();
}


pub fn fetch_bluetooth_status() -> (bool, String) {
    if let Ok(output) = Command::new("bluetoothctl").arg("show").output() {
        if let Ok(stdout) = String::from_utf8(output.stdout) {
            let powered = stdout.lines()
                .find(|line| line.contains("Powered:"))
                .and_then(|line| line.split(':').nth(1))
                .map(|s| s.trim() == "yes")
                .unwrap_or(false);

            if !powered { return (false, "Bluetooth Off".to_string()); }

            if let Ok(devices_output) = Command::new("bluetoothctl").args(&["devices", "Connected"]).output() {
                if let Ok(devices_str) = String::from_utf8(devices_output.stdout) {
                    if let Some(first_device) = devices_str.lines().next() {
                        let parts: Vec<&str> = first_device.split_whitespace().collect();
                        if parts.len() >= 3 {
                            return (true, parts[2..].join(" "));
                        }
                    }
                }
            }

            return (true, "No Device".to_string());
        }
    }

    (false, "Bluetooth Off".to_string())
}

pub fn toggle_bluetooth_cmd(enable: bool) {
    let state = if enable { "on" } else { "off" };
    let _ = Command::new("bluetoothctl").args(&["power", state]).output();
}


pub fn toggle_eye_care_cmd(enable: bool) {
    if enable {
        let _ = Command::new("redshift").args(&["-P", "-O", "3500"]).output();
    } else {
        let _ = Command::new("redshift").args(&["-x"]).output();
    }
}


/// A scanned wifi network entry.
#[derive(Debug, Clone)]
pub struct WifiNetwork {
    /// SSID truncated to 30 chars max
    pub ssid: String,
    /// Signal strength 0–100
    pub signal: u8,
    /// Whether this is the currently connected network
    pub connected: bool,
    /// Whether the network requires a password (has security)
    pub secured: bool,
}

/// Scan for available wifi networks using nmcli.
/// Returns them sorted: connected first, then by signal descending.
pub fn fetch_wifi_networks() -> Vec<WifiNetwork> {
    // nmcli -t -f IN-USE,SSID,SIGNAL,SECURITY dev wifi list
    // IN-USE is "*" for the active network
    let output = match Command::new("nmcli")
        .args(&["-t", "-f", "IN-USE,SSID,SIGNAL,SECURITY", "dev", "wifi", "list"])
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            eprintln!("[Wifi] nmcli failed: {}", e);
            return Vec::new();
        }
    };

    let stdout = match String::from_utf8(output.stdout) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let mut seen_ssids = std::collections::HashSet::new();
    let mut networks: Vec<WifiNetwork> = Vec::new();

    for line in stdout.lines() {
        // Format: IN-USE:SSID:SIGNAL:SECURITY
        // SSID may contain ':' — so split from left (IN-USE), then from right (SECURITY, SIGNAL)
        let (in_use_char, rest) = match line.chars().next() {
            Some(c) => (c, &line[c.len_utf8()..]),
            None => continue,
        };
        let rest = rest.strip_prefix(':').unwrap_or(rest);

        // Split from the RIGHT to get SECURITY (last field) and SIGNAL (second-to-last)
        let rfind_colon = |s: &str| s.rfind(':');
        let security_split = match rfind_colon(rest) {
            Some(i) => i,
            None => continue,
        };
        let security_raw = rest[security_split + 1..].trim();
        let without_security = &rest[..security_split];

        let signal_split = match rfind_colon(without_security) {
            Some(i) => i,
            None => continue,
        };
        let signal_str = without_security[signal_split + 1..].trim();
        let ssid_raw = without_security[..signal_split].trim();

        // Skip hidden networks (empty SSID)
        if ssid_raw.is_empty() || ssid_raw == "--" {
            continue;
        }

        // Deduplicate by SSID
        if seen_ssids.contains(ssid_raw) {
            if let Some(existing) = networks.iter_mut().find(|n| n.ssid == truncate_ssid(ssid_raw)) {
                if let Ok(sig) = signal_str.parse::<u8>() {
                    if sig > existing.signal {
                        existing.signal = sig;
                    }
                }
            }
            continue;
        }
        seen_ssids.insert(ssid_raw.to_string());

        let signal: u8 = signal_str.parse().unwrap_or(0);
        let connected = in_use_char == '*';
        let secured = !security_raw.is_empty() && security_raw != "--";

        networks.push(WifiNetwork {
            ssid: truncate_ssid(ssid_raw),
            signal,
            connected,
            secured,
        });
    }

    // Sort: connected first, then by signal descending
    networks.sort_by(|a, b| {
        b.connected.cmp(&a.connected)
            .then(b.signal.cmp(&a.signal))
    });

    networks
}

/// Truncate an SSID to at most 30 characters.
fn truncate_ssid(ssid: &str) -> String {
    const MAX: usize = 30;
    if ssid.chars().count() <= MAX {
        ssid.to_string()
    } else {
        let truncated: String = ssid.chars().take(MAX - 2).collect();
        format!("{}…", truncated)
    }
}

/// Signal strength (0–100) → one of four wifi icon strings (nerd font).
pub fn signal_icon(signal: u8) -> &'static str {
    match signal {
        75..=100 => "󰤨", // full
        50..=74  => "󰤥", // good
        25..=49  => "󰤢", // fair
        _        => "󰤟", // weak
    }
}

pub fn connect_wifi_cmd(ssid: &str, password: &str) {
    eprintln!("[Wifi] connect attempt: {}", ssid);

    // Try connecting with nmcli
    let success = if password.is_empty() {
        Command::new("nmcli")
            .args(&["dev", "wifi", "connect", ssid])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    } else {
        Command::new("nmcli")
            .args(&["dev", "wifi", "connect", ssid, "password", password])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    };

    if success || true {
        // Save credential regardless — nmcli exit code is unreliable on some setups
        crate::utils::wifi_credentials::save_password(ssid, password);
        eprintln!("[Wifi] saved credential for '{}'", ssid);
    }
}

/// Forget a wifi network: remove from nmcli saved connections AND local credential store.
pub fn forget_wifi_cmd(ssid: &str) {
    eprintln!("[Wifi] forgetting network: {}", ssid);

    // Remove from NetworkManager
    // nmcli connection delete "<ssid>"
    let _ = Command::new("nmcli")
        .args(&["connection", "delete", ssid])
        .output();

    // Also remove from our local credential cache
    crate::utils::wifi_credentials::forget_password(ssid);
}

/// Disconnect from the currently active wifi network.
pub fn disconnect_wifi_cmd(ssid: &str) {
    eprintln!("[Wifi] disconnecting from '{}'", ssid);
    // nmcli dev disconnect <iface>  -- works even without knowing the iface name
    // The reliable approach is to use connection down for the SSID profile:
    let _ = Command::new("nmcli")
        .args(&["connection", "down", ssid])
        .output();
    // Fallback: disconnect the wifi device entirely
    let _ = Command::new("nmcli")
        .args(&["dev", "disconnect", "wlan0"])
        .output();
}

/// Try to auto-connect to the best known network in the scan results.
/// Returns the SSID it attempted to connect to, or None.
pub fn auto_connect_best(networks: &[WifiNetwork]) -> Option<String> {
    // Find the highest-signal network we have a saved password for
    let best = networks.iter()
        .filter(|n| !n.connected)
        .filter(|n| crate::utils::wifi_credentials::has_saved(&n.ssid))
        .max_by_key(|n| n.signal)?;

    let ssid = best.ssid.clone();
    let password = crate::utils::wifi_credentials::get_password(&ssid)
        .unwrap_or_default();

    eprintln!("[Wifi] auto-connecting to best known network: '{}' (signal {})", ssid, best.signal);

    std::thread::spawn({
        let ssid = ssid.clone();
        move || { connect_wifi_cmd(&ssid, &password); }
    });

    Some(ssid)
}
