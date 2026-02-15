#!/bin/bash
# Sierra Launcher - Complete Automated Installation Script
# Handles all dependencies, builds, and configures everything automatically

set -e

# ─────────────────────────────────────────────
#   Color Palette
# ─────────────────────────────────────────────
RESET='\033[0m'
BOLD='\033[1m'
DIM='\033[2m'

WHITE='\033[97m'
GRAY='\033[90m'
CYAN='\033[96m'
GREEN='\033[92m'
YELLOW='\033[93m'
RED='\033[91m'
BLUE='\033[94m'

# ─────────────────────────────────────────────
#   Helpers
# ─────────────────────────────────────────────
header() {
    echo ""
    echo -e "  ${BOLD}${CYAN}$1${RESET}"
    echo -e "  ${GRAY}$(printf '%.0s─' $(seq 1 50))${RESET}"
}

step() {
    echo -e "  ${GRAY}·${RESET}  $1"
}

ok() {
    echo -e "  ${GREEN}✓${RESET}  $1"
}

warn() {
    echo -e "  ${YELLOW}⚠${RESET}  $1"
}

fail() {
    echo -e "  ${RED}✗${RESET}  $1"
}

info() {
    echo -e "      ${DIM}${GRAY}$1${RESET}"
}

# ─────────────────────────────────────────────
#   Detect Distribution
# ─────────────────────────────────────────────
detect_distro() {
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        DISTRO=$ID
        DISTRO_LIKE=${ID_LIKE:-$ID}
    else
        fail "Cannot detect Linux distribution"
        exit 1
    fi
}

# ─────────────────────────────────────────────
#   Install System Dependencies
# ─────────────────────────────────────────────
install_system_deps() {
    header "System Dependencies"
    
    detect_distro
    
    case "$DISTRO" in
        arch|manjaro|endeavouros|garuda)
            info "Detected: $PRETTY_NAME"
            step "Installing dependencies via pacman..."
            
            # Core dependencies
            sudo pacman -S --needed --noconfirm \
                rust \
                cargo \
                wl-clipboard \
                brightnessctl \
                pulseaudio \
                pulseaudio-alsa \
                networkmanager \
                bluez \
                bluez-utils \
                lm_sensors \
                socat \
                jq \
                ffmpeg \
                imagemagick \
                python-pywal \
                redshift 2>&1 | grep -v "warning:" || true
            
            ok "Core dependencies installed"
            
            # AUR dependencies (gslapper)
            if command -v yay &>/dev/null; then
                step "Installing gslapper from AUR (yay)..."
                yay -S --needed --noconfirm gslapper 2>&1 | grep -v "warning:" || warn "gslapper install failed"
            elif command -v paru &>/dev/null; then
                step "Installing gslapper from AUR (paru)..."
                paru -S --needed --noconfirm gslapper 2>&1 | grep -v "warning:" || warn "gslapper install failed"
            else
                warn "No AUR helper found (install yay or paru for wallpaper support)"
                info "Manual install: yay -S gslapper"
            fi
            ;;
            
        ubuntu|debian|pop|linuxmint|zorin)
            info "Detected: $PRETTY_NAME"
            step "Updating package lists..."
            sudo apt update -qq
            
            step "Installing dependencies via apt..."
            sudo apt install -y \
                curl \
                build-essential \
                pkg-config \
                libssl-dev \
                libdbus-1-dev \
                wl-clipboard \
                brightnessctl \
                pulseaudio \
                pulseaudio-utils \
                network-manager \
                bluez \
                lm-sensors \
                socat \
                jq \
                ffmpeg \
                imagemagick \
                python3-pip \
                redshift 2>&1 | grep -E "Setting up|Unpacking" || true
            
            ok "Core dependencies installed"
            
            # Install Rust if not present
            if ! command -v cargo &>/dev/null; then
                step "Installing Rust..."
                curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
                source "$HOME/.cargo/env"
                ok "Rust installed"
            fi
            
            # Install pywal
            step "Installing pywal..."
            pip3 install --user pywal >/dev/null 2>&1 || warn "pywal install failed"
            
            warn "gslapper not available for Debian/Ubuntu"
            info "Video wallpapers require manual gslapper installation"
            ;;
            
        fedora|rhel|centos)
            info "Detected: $PRETTY_NAME"
            step "Installing dependencies via dnf..."
            
            sudo dnf install -y \
                rust \
                cargo \
                wl-clipboard \
                brightnessctl \
                pulseaudio \
                pulseaudio-utils \
                NetworkManager \
                bluez \
                lm_sensors \
                socat \
                jq \
                ffmpeg \
                ImageMagick \
                python3-pip \
                redshift \
                openssl-devel \
                dbus-devel 2>&1 | grep "Installing" || true
            
            ok "Core dependencies installed"
            
            step "Installing pywal..."
            pip3 install --user pywal >/dev/null 2>&1 || warn "pywal install failed"
            
            warn "gslapper not available for Fedora"
            ;;
            
        opensuse*|suse)
            info "Detected: $PRETTY_NAME"
            step "Installing dependencies via zypper..."
            
            sudo zypper install -y \
                rust \
                cargo \
                wl-clipboard \
                brightnessctl \
                pulseaudio \
                NetworkManager \
                bluez \
                sensors \
                socat \
                jq \
                ffmpeg \
                ImageMagick \
                python3-pip \
                redshift 2>&1 | grep "Installing" || true
            
            ok "Core dependencies installed"
            ;;
            
        *)
            fail "Unsupported distribution: $DISTRO"
            info "Supported: Arch, Ubuntu, Fedora, openSUSE"
            info "Please install dependencies manually and re-run"
            exit 1
            ;;
    esac
}

# ─────────────────────────────────────────────
#   Configure System Services
# ─────────────────────────────────────────────
configure_services() {
    header "System Services"
    
    # Enable bluetooth
    if systemctl is-enabled bluetooth.service &>/dev/null || sudo systemctl enable bluetooth.service &>/dev/null; then
        ok "Bluetooth service enabled"
        sudo systemctl start bluetooth.service 2>/dev/null || true
    fi
    
    # Detect sensors
    if command -v sensors-detect &>/dev/null; then
        step "Detecting hardware sensors..."
        yes | sudo sensors-detect &>/dev/null || true
        ok "Sensors configured"
    fi
}

# ─────────────────────────────────────────────
#   Create Config Directory
# ─────────────────────────────────────────────
create_config() {
    header "Configuration"
    
    CONFIG_DIR="$HOME/.config/sierra"
    CACHE_DIR="$HOME/.cache/sierra"
    
    mkdir -p "$CONFIG_DIR"
    mkdir -p "$CACHE_DIR/wallpapers/thumbs"
    mkdir -p "$HOME/Pictures/Wallpapers"
    
    if [ ! -f "$CONFIG_DIR/Sierra" ]; then
        step "Creating default config file..."
        cat > "$CONFIG_DIR/Sierra" << 'EOF'
# Sierra Launcher Configuration

# Font Settings
font = "Monocraft"
font_size = 14.0

# Title Animation
title_text = " sierra-launcher "
title_animation = "Wave"  # Options: Rainbow, Wave, InOutWave, Pulse, Sparkle, Gradient

# Wallpaper Directory
wallpaper_dir = "~/Pictures/Wallpapers"

# Weather Location (auto-detected if not set)
# Examples: "New York", "London, UK", "Mumbai, India"
# weather_location = "Your City"

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
EOF
        ok "Config file created: $CONFIG_DIR/Sierra"
        info "Edit this file to customize appearance"
    else
        ok "Config file already exists"
    fi
}

# ─────────────────────────────────────────────
#   Banner
# ─────────────────────────────────────────────
clear
echo ""
echo -e "  ${BOLD}${WHITE}Sierra Launcher${RESET}  ${DIM}${GRAY}Automated Installation${RESET}"
echo ""

# Check if running as root
if [ "$EUID" -eq 0 ]; then 
    fail "Do not run as root! Run as your normal user."
    exit 1
fi

# ─────────────────────────────────────────────
#   Wayland Check
# ─────────────────────────────────────────────
header "Environment"

if [ -z "$WAYLAND_DISPLAY" ] && [ "$XDG_SESSION_TYPE" != "wayland" ]; then
    warn "Wayland session not detected"
    info "Current session: ${XDG_SESSION_TYPE:-unknown}"
    info "Sierra requires a Wayland compositor (Hyprland, Sway, etc.)"
    info "Continuing anyway..."
else
    ok "Wayland session detected  ${DIM}${GRAY}($XDG_SESSION_TYPE)${RESET}"
fi

# ─────────────────────────────────────────────
#   Install Dependencies
# ─────────────────────────────────────────────
install_system_deps

# ─────────────────────────────────────────────
#   Configure Services
# ─────────────────────────────────────────────
configure_services

# ─────────────────────────────────────────────
#   Create Config
# ─────────────────────────────────────────────
create_config

# ─────────────────────────────────────────────
#   Stop Existing Service
# ─────────────────────────────────────────────
header "Cleanup"

# Temporarily disable exit on error for cleanup
set +e

# Check if service exists before trying to stop it
if systemctl --user list-unit-files 2>/dev/null | grep -q "sierra-launcher"; then
    if systemctl --user is-active sierra-launcher 2>/dev/null; then
        step "Stopping existing sierra-launcher service..."
        systemctl --user stop sierra-launcher 2>/dev/null
        sleep 0.5  # Give it time to fully stop
        ok "Service stopped"
    fi
fi

# Remove old socket (safe to do even if doesn't exist)
SOCKET_PATH="${XDG_RUNTIME_DIR:-/tmp}/sierra-launcher.sock"
if [ -S "$SOCKET_PATH" ]; then
    step "Removing old socket..."
    rm -f "$SOCKET_PATH" 2>/dev/null
    ok "Socket removed"
fi

# Re-enable exit on error
set -e

# ─────────────────────────────────────────────
#   Build
# ─────────────────────────────────────────────
header "Build"

# Ensure we have cargo in PATH
if ! command -v cargo &>/dev/null; then
    source "$HOME/.cargo/env" 2>/dev/null || true
fi

step "Compiling Sierra Launcher in release mode..."
info "This may take 5-10 minutes on first build"
echo ""

cargo build --release 2>&1 | grep -E "^   Compiling|^    Finished|^error" | while read -r line; do
    case "$line" in
        *Compiling*)  info "  ${line}" ;;
        *Finished*)   echo -e "  ${GREEN}✓${RESET}  ${DIM}${line}${RESET}" ;;
        *error*)      echo -e "  ${RED}✗${RESET}  ${line}" ;;
    esac
done

# Handle both binary names (sierra_launcher and sierra-launcher)
BINARY=""
if [ -f "target/release/sierra_launcher" ]; then
    BINARY="target/release/sierra_launcher"
elif [ -f "target/release/sierra-launcher" ]; then
    BINARY="target/release/sierra-launcher"
fi

if [ -z "$BINARY" ]; then
    fail "Build failed or binary not found"
    info "Expected: target/release/sierra_launcher"
    exit 1
fi

ok "Build complete"

# ─────────────────────────────────────────────
#   Install Binary
# ─────────────────────────────────────────────
header "Installation"

step "Installing daemon binary to /usr/bin/sierra-launcher-daemon..."
sudo cp "$BINARY" /usr/bin/sierra-launcher-daemon
sudo chmod +x /usr/bin/sierra-launcher-daemon
ok "Daemon binary installed"

step "Installing wrapper script to /usr/bin/sierra-launcher..."
if [ -f "sh/sierra-launcher-wrapper.sh" ]; then
    sudo cp sh/sierra-launcher-wrapper.sh /usr/bin/sierra-launcher
else
    # Create wrapper if it doesn't exist
    sudo tee /usr/bin/sierra-launcher >/dev/null << 'EOF'
#!/bin/bash
# sierra-launcher wrapper script
SOCKET_PATH="${XDG_RUNTIME_DIR:-/tmp}/sierra-launcher.sock"
BINARY="/usr/bin/sierra-launcher-daemon"

# Check if daemon is running
if [ -S "$SOCKET_PATH" ]; then
    # Send SHOW command to daemon
    echo "SHOW" | socat - UNIX-CONNECT:"$SOCKET_PATH" 2>/dev/null
    if [ $? -eq 0 ]; then
        exit 0
    fi
fi

# Daemon not running, start it
exec "$BINARY"
EOF
fi
sudo chmod +x /usr/bin/sierra-launcher
ok "Wrapper script installed"

# ─────────────────────────────────────────────
#   Systemd Service
# ─────────────────────────────────────────────
header "Systemd Service"

step "Installing systemd user service..."
mkdir -p ~/.config/systemd/user/

if [ -f "sh/sierra-launcher.service" ]; then
    cp sh/sierra-launcher.service ~/.config/systemd/user/
else
    # Create service file if it doesn't exist
    cat > ~/.config/systemd/user/sierra-launcher.service << 'EOF'
[Unit]
Description=Sierra Launcher Daemon
After=graphical-session.target
Wants=graphical-session.target
PartOf=graphical-session.target

[Service]
Type=simple
ExecStart=/usr/bin/sierra-launcher-daemon
Restart=on-failure
RestartSec=2s

# Clean up socket on exit
ExecStopPost=/bin/rm -f %t/sierra-launcher.sock

[Install]
WantedBy=graphical-session.target
EOF
fi
ok "Service file installed"

step "Reloading systemd daemon..."
systemctl --user daemon-reload
ok "Systemd reloaded"

step "Enabling autostart on login..."
systemctl --user enable sierra-launcher.service
ok "Autostart enabled"

# ─────────────────────────────────────────────
#   Wallpaper Restore Service (Optional)
# ─────────────────────────────────────────────
if command -v gslapper &>/dev/null; then
    header "Wallpaper Restore"
    
    step "Installing wallpaper restore script..."
    if [ -f "sh/restore-wallpaper.sh" ]; then
        sudo cp sh/restore-wallpaper.sh /usr/local/bin/
    else
        sudo tee /usr/local/bin/restore-wallpaper.sh >/dev/null << 'EOF'
#!/bin/bash
# Restore wallpaper from Sierra launcher cache on login
set -e

CACHE_FILE="$HOME/.cache/sierra/wallpapers/last_wallpaper.json"

# Wait for compositor to be ready
sleep 0.5

if [ ! -f "$CACHE_FILE" ]; then
    exit 0
fi

# Extract wallpaper path from JSON
if command -v jq &>/dev/null; then
    WALLPAPER=$(jq -r '.last_wallpaper' "$CACHE_FILE")
else
    WALLPAPER=$(grep -oP '"last_wallpaper":\s*"\K[^"]+' "$CACHE_FILE")
fi

if [ -z "$WALLPAPER" ] || [ ! -f "$WALLPAPER" ]; then
    exit 0
fi

# Kill any existing gSlapper instances
pkill -9 gslapper 2>/dev/null || true
sleep 0.1

# Determine wallpaper type from extension
EXT="${WALLPAPER##*.}"
EXT_LOWER="${EXT,,}"

case "$EXT_LOWER" in
    mp4|mkv|webm|avi)
        gslapper -o "loop no-audio" "*" "$WALLPAPER" &
        ;;
    jpg|jpeg|png|webp|bmp)
        gslapper -o fill "*" "$WALLPAPER" &
        ;;
esac
EOF
    fi
    sudo chmod +x /usr/local/bin/restore-wallpaper.sh
    ok "Wallpaper restore script installed"
    
    step "Installing wallpaper restore service..."
    if [ -f "sh/restore-wallpaper.service" ]; then
        cp sh/restore-wallpaper.service ~/.config/systemd/user/
    else
        cat > ~/.config/systemd/user/restore-wallpaper.service << 'EOF'
[Unit]
Description=Restore Sierra Launcher Wallpaper
After=graphical-session.target
Wants=graphical-session.target

[Service]
Type=oneshot
ExecStart=/usr/local/bin/restore-wallpaper.sh
RemainAfterExit=yes

[Install]
WantedBy=graphical-session.target
EOF
    fi
    
    systemctl --user daemon-reload
    systemctl --user enable restore-wallpaper.service
    ok "Wallpaper restore enabled"
fi

# ─────────────────────────────────────────────
#   Start Service
# ─────────────────────────────────────────────
header "Start Service"

step "Starting sierra-launcher daemon..."
systemctl --user start sierra-launcher
sleep 1

if systemctl --user is-active sierra-launcher &>/dev/null; then
    ok "Daemon started successfully"
else
    fail "Daemon failed to start"
    info "Check logs: journalctl --user -u sierra-launcher"
fi

# ─────────────────────────────────────────────
#   Compositor Keybind Suggestion
# ─────────────────────────────────────────────
header "Compositor Setup"

echo -e "  ${WHITE}Add this keybind to your compositor config:${RESET}"
echo ""
if [ "$XDG_CURRENT_DESKTOP" = "Hyprland" ] || command -v hyprctl &>/dev/null; then
    echo -e "  ${CYAN}# Hyprland (~/.config/hypr/hyprland.conf)${RESET}"
    echo -e "  ${DIM}bind = \$mainMod, F, exec, sierra-launcher${RESET}"
elif command -v swaymsg &>/dev/null; then
    echo -e "  ${CYAN}# Sway (~/.config/sway/config)${RESET}"
    echo -e "  ${DIM}bindsym \$mod+f exec sierra-launcher${RESET}"
else
    echo -e "  ${CYAN}# Add to your compositor config:${RESET}"
    echo -e "  ${DIM}sierra-launcher${RESET}"
fi

# ─────────────────────────────────────────────
#   Complete
# ─────────────────────────────────────────────
echo ""
echo -e "  ${BOLD}${GREEN}✓ Installation Complete!${RESET}"
echo ""
echo -e "  ${WHITE}Quick Start:${RESET}"
echo -e "      ${CYAN}sierra-launcher${RESET}        Launch the interface"
echo -e "      ${CYAN}Super+F${RESET}                Your keybind (after setup)"
echo ""
echo -e "  ${WHITE}Service Management:${RESET}"
echo -e "      ${CYAN}systemctl --user status sierra-launcher${RESET}"
echo -e "      ${CYAN}systemctl --user restart sierra-launcher${RESET}"
echo -e "      ${CYAN}systemctl --user stop sierra-launcher${RESET}"
echo ""
echo -e "  ${WHITE}Configuration:${RESET}"
echo -e "      ${CYAN}~/.config/sierra/Sierra${RESET}   Edit theme and settings"
echo -e "      ${CYAN}~/Pictures/Wallpapers/${RESET}     Add your wallpapers here"
echo ""
echo -e "  ${WHITE}Logs (if issues occur):${RESET}"
echo -e "      ${CYAN}journalctl --user -u sierra-launcher -f${RESET}"
echo ""