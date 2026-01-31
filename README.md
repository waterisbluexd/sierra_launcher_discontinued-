<p align="center">
  <img
    src="https://github.com/user-attachments/assets/318f1eba-055e-43bb-a7bc-de934152b2c3"
    alt="Sierra logo"
    width="500"
  />
</p>
<p align="center">
  <img src="https://img.shields.io/badge/language-rust-orange?style=for-the-badge&logo=rust" alt="Rust" />
  <img src="https://img.shields.io/badge/platform-linux-blue?style=for-the-badge&logo=linux&logoColor=white" alt="Linux" />
  <img src="https://img.shields.io/badge/wayland-only-00ADD8?style=for-the-badge&logo=wayland&logoColor=white" alt="Wayland Only" />
  <img src="https://img.shields.io/badge/license-MIT-green?style=for-the-badge" alt="License MIT" />
  <img src="https://img.shields.io/badge/code-bad-red?style=for-the-badge" alt="Code quality" />
</p>
<p align="center">
  <strong>
    Feature-rich application launcher for Wayland compositors built with Rust and the Iced GUI framework
  </strong>
</p>
<p align="center">
  <img
    src="https://github.com/user-attachments/assets/b1d8a11e-9753-48a1-a4ca-f98b777c9d56"
    alt="Sierra screenshot"
    width="484"
    height="714"
  />
</p>
<p align="center">
  <a href="#features">Features</a> •
<a href="#installation">Installation</a> •
<a href="#configuration">Configuration</a>

</p>

---

## Requirements

### Build Requirements:

- **Rust** (latest stable version recommended)
  - Install: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
  - Verify: `rustc --version`
- **Cargo** (comes with Rust)

### Runtime Requirements:

- **Wayland compositor** (Hyprland, Sway, etc.)
- **Linux** system :)
- **wl-clipboard**  
  Clipboard support  
  - `wl-copy`
  - `wl-paste`

### System Controls  
- **brightnessctl**  
  Screen brightness control

- **PulseAudio or PipeWire**  
  Volume control via `pactl`

- **NetworkManager**  
  WiFi management via `nmcli`

- **BlueZ**  
  Bluetooth control via `bluetoothctl`

- **lm_sensors**  
  Hardware monitoring (CPU temperature & fan speeds)  
  - `sensors`

### Wallpaper Features  
- **gslapper**  
  Wallpaper daemon for image & video wallpapers  

  **Arch Linux**
  ```bash
  yay -S gslapper
  # or
  paru -S gslapper

### Font
It is better to use Monocraft due to UI and how everything fits 
```bash
https://github.com/IdreesInc/Monocraft
```
---

<p align="center">
  <img
    src="https://github.com/user-attachments/assets/51c56096-dfe2-4472-8531-ac1da684d05a"
    alt="Sierra titles"
    width="1000"
  />
</p>

##  Features

- **Application Launcher** - Fast fuzzy search to launch any installed application
- **Clipboard Manager** - Access and manage clipboard history
- **System Monitor** - Real-time CPU, memory, and disk usage visualization
- **Media Controls** - Control music playback via MPRIS
- **Wallpaper Manager** - Quick wallpaper switching with preview
- **Weather Widget** - Live weather information display
- **System Services** - Quick controls for WiFi, Bluetooth, audio, and brightness

##  Theming & Customization

- **Pywal Integration** - Dynamic theme generation from your wallpaper colors also can be turned off
- **Custom Themes** - Define your own color schemes via TOML config or just change it from config
- **Title Animations** - Multiple animation styles for the launcher title
- **Font Customization** - Configure fonts and sizes for UI elements

##  Installation
```bash
git clone https://github.com/waterisbluexd/sierra_launcher.git
cd sierra_launcher
chmod +x install.sh
./install.sh
```

After installation, launch Sierra by running `sierra-launcher` or bind it to a keyboard shortcut in your compositor config like "bind = $mainMod, F, exec, sierra-launcher" for Hyprland.

##  Configuration

Configuration file: `~/.config/sierra/Sierra`

### Example Configuration
```toml
# Font Settings
font = "Monospace"
font_size = 14.0

# Title Animation
title_text = " sierra-launcher "
title_animation = "Wave"  # Options: Rainbow, Wave, InOutWave, Pulse, Sparkle, Gradient

# Wallpaper Directory
wallpaper_dir = "~/Pictures/Wallpapers"

# Theme Mode
use_pywal = false  # Set to true to use pywal colors

# Custom Theme (only used if use_pywal = false)
[theme]
background = "#1a1b26"
foreground = "#c0caf5"
border     = "#7aa2f7"
accent     = "#7dcfff"

color0  = "#15161e"
color1  = "#f7768e"
color2  = "#9ece6a"
color3  = "#e0af68"
color4  = "#7aa2f7"
color5  = "#bb9af7"
color6  = "#7dcfff"
color7  = "#a9b1d6"
color8  = "#414868"
color9  = "#f7768e"
color10 = "#9ece6a"
color11 = "#e0af68"
color12 = "#7aa2f7"
color13 = "#bb9af7"
color14 = "#7dcfff"
color15 = "#c0caf5"
```
---
### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `Esc` | Exit launcher |
| `Enter` | Launch selected app / Paste clipboard item |
| `↑` / `↓` | Navigate apps / clipboard history |
| `←` / `→` | Cycle panels (Clock → Weather → Music → Wallpaper → System → Services) |
| `Shift + ←/→` | Toggle clipboard panel |
| `Type` | Search applications (auto-focus) |
| `Backspace` | Clear search |
| `Ctrl + D` | Delete clipboard item (in clipboard mode) |
| `Right Click` | Toggle control center (Power/Restart/Sleep) |
