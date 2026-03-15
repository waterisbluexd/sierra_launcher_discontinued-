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


pub fn fetch_wifi_status() -> (bool, String) {
    if let Ok(output) = Command::new("nmcli").args(&["-t", "-f", "ACTIVE,SSID", "dev", "wifi"]).output() {
        if let Ok(stdout) = String::from_utf8(output.stdout) {
            for line in stdout.lines() {
                if line.starts_with("yes:") {
                    return (true, line.strip_prefix("yes:").unwrap_or("Connected").to_string());
                }
            }
        }
    }

    if let Ok(output) = Command::new("iwgetid").arg("-r").output() {
        if let Ok(ssid) = String::from_utf8(output.stdout) {
            let ssid = ssid.trim();
            if !ssid.is_empty() { return (true, ssid.to_string()); }
        }
    }

    if let Ok(output) = Command::new("nmcli").args(&["radio", "wifi"]).output() {
        if let Ok(stdout) = String::from_utf8(output.stdout) {
            if stdout.trim() == "disabled" { return (false, "WiFi Off".to_string()); }
        }
    }

    (true, "No Network".to_string())
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
        let parts: Vec<&str> = line.splitn(4, ':').collect();
        if parts.len() < 4 {
            continue;
        }

        let in_use = parts[0].trim();
        let ssid_raw = parts[1];
        let signal_str = parts[2].trim();
        let security_raw = parts[3].trim();

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
        let connected = in_use == "*";
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
