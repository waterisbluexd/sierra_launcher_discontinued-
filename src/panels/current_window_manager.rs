use std::collections::HashSet;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Compositor {
    Hyprland,
    Sway,
    Unknown,
}

fn detect_compositor() -> Compositor {
    static COMPOSITOR: OnceLock<Compositor> = OnceLock::new();
    *COMPOSITOR.get_or_init(|| {
        if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() {
            eprintln!("[WM] Detected compositor: Hyprland");
            Compositor::Hyprland
        } else if std::env::var("SWAYSOCK").is_ok() {
            eprintln!("[WM] Detected compositor: Sway");
            Compositor::Sway
        } else {
            eprintln!("[WM] Unknown compositor, workspace features disabled");
            Compositor::Unknown
        }
    })
}

pub fn get_current_workspace() -> usize {
    match detect_compositor() {
        Compositor::Hyprland => hyprland_current_workspace(),
        Compositor::Sway     => sway_current_workspace(),
        Compositor::Unknown  => 1,
    }
}

pub fn get_workspaces_with_windows() -> HashSet<usize> {
    match detect_compositor() {
        Compositor::Hyprland => hyprland_workspaces_with_windows(),
        Compositor::Sway     => sway_workspaces_with_windows(),
        Compositor::Unknown  => HashSet::new(),
    }
}

pub fn switch_workspace(num: usize) {
    match detect_compositor() {
        Compositor::Hyprland => {
            std::thread::spawn(move || {
                let out = std::process::Command::new("hyprctl")
                    .args(["dispatch", "workspace", &num.to_string()])
                    .output();
                match out {
                    Ok(o) if o.status.success() => eprintln!("[WM] Hyprland: switched to workspace {}", num),
                    Ok(o) => eprintln!("[WM] Hyprland: hyprctl error: {}", String::from_utf8_lossy(&o.stderr)),
                    Err(e) => eprintln!("[WM] Hyprland: failed to spawn hyprctl: {}", e),
                }
            });
        }
        Compositor::Sway => {
            std::thread::spawn(move || {
                let out = std::process::Command::new("swaymsg")
                    .args(["workspace", &num.to_string()])
                    .output();
                match out {
                    Ok(o) if o.status.success() => eprintln!("[WM] Sway: switched to workspace {}", num),
                    Ok(o) => eprintln!("[WM] Sway: swaymsg error: {}", String::from_utf8_lossy(&o.stderr)),
                    Err(e) => eprintln!("[WM] Sway: failed to spawn swaymsg: {}", e),
                }
            });
        }
        Compositor::Unknown => eprintln!("[WM] Cannot switch workspace: unknown compositor"),
    }
}

fn hyprland_current_workspace() -> usize {
    let output = match std::process::Command::new("hyprctl").args(["activeworkspace", "-j"]).output() {
        Ok(o) if o.status.success() => o,
        Ok(o) => { eprintln!("[WM] hyprctl activeworkspace failed: {}", String::from_utf8_lossy(&o.stderr)); return 1; }
        Err(e) => { eprintln!("[WM] hyprctl spawn failed: {}", e); return 1; }
    };
    serde_json::from_str::<serde_json::Value>(&String::from_utf8_lossy(&output.stdout))
        .ok().and_then(|j| j.get("id")?.as_u64()).map(|id| id as usize).unwrap_or(1)
}

fn hyprland_workspaces_with_windows() -> HashSet<usize> {
    let mut result = HashSet::new();
    let output = match std::process::Command::new("hyprctl").args(["workspaces", "-j"]).output() {
        Ok(o) if o.status.success() => o,
        _ => return result,
    };
    if let Ok(serde_json::Value::Array(arr)) = serde_json::from_str::<serde_json::Value>(&String::from_utf8_lossy(&output.stdout)) {
        for ws in arr {
            if let Some(id) = ws.get("id").and_then(|v| v.as_u64()) {
                if ws.get("windows").and_then(|v| v.as_u64()).unwrap_or(0) > 0 {
                    result.insert(id as usize);
                }
            }
        }
    }
    result
}

fn sway_current_workspace() -> usize {
    let output = match std::process::Command::new("swaymsg").args(["-t", "get_workspaces"]).output() {
        Ok(o) if o.status.success() => o,
        Ok(o) => { eprintln!("[WM] swaymsg get_workspaces failed: {}", String::from_utf8_lossy(&o.stderr)); return 1; }
        Err(e) => { eprintln!("[WM] swaymsg spawn failed: {}", e); return 1; }
    };
    if let Ok(serde_json::Value::Array(arr)) = serde_json::from_str::<serde_json::Value>(&String::from_utf8_lossy(&output.stdout)) {
        for ws in &arr {
            if ws.get("focused").and_then(|v| v.as_bool()).unwrap_or(false) {
                if let Some(num) = ws.get("num").and_then(|v| v.as_i64()) {
                    return num as usize;
                }
            }
        }
    }
    1
}

fn sway_workspaces_with_windows() -> HashSet<usize> {
    let mut result = HashSet::new();
    let output = match std::process::Command::new("swaymsg").args(["-t", "get_workspaces"]).output() {
        Ok(o) if o.status.success() => o,
        _ => return result,
    };
    if let Ok(serde_json::Value::Array(arr)) = serde_json::from_str::<serde_json::Value>(&String::from_utf8_lossy(&output.stdout)) {
        for ws in arr {
            if let Some(num) = ws.get("num").and_then(|v| v.as_i64()) {
                if num <= 0 { continue; }
                let has_windows = ws.get("representation").map(|v| !v.is_null()).unwrap_or(false)
                    || ws.get("nodes").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0) > 0
                    || ws.get("floating_nodes").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0) > 0;
                if has_windows { result.insert(num as usize); }
            }
        }
    }
    result
}
