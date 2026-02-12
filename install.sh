#!/bin/bash
# Sierra Launcher - Complete Installation Script with Daemon Mode
# Installs the launcher as a background daemon for instant window appearance

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
#   Banner
# ─────────────────────────────────────────────
clear
echo ""
echo -e "  ${BOLD}${WHITE}Sierra Launcher${RESET}  ${DIM}${GRAY}v2.0 · Daemon Mode${RESET}"
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
    fail "Wayland session not detected"
    info "Current session: ${XDG_SESSION_TYPE:-unknown}"
    info "Sierra requires a Wayland compositor (Hyprland, Sway, etc.)"
    echo ""
    exit 1
else
    ok "Wayland session detected  ${DIM}${GRAY}($XDG_SESSION_TYPE)${RESET}"
fi

# ─────────────────────────────────────────────
#   Dependencies
# ─────────────────────────────────────────────
header "Dependencies"

# Check for required commands
MISSING_DEPS=""

for cmd in cargo socat; do
    if ! command -v $cmd &>/dev/null; then
        MISSING_DEPS="$MISSING_DEPS $cmd"
    fi
done

if [ -n "$MISSING_DEPS" ]; then
    fail "Missing required commands:$MISSING_DEPS"
    info "Install them with: sudo pacman -S rust socat"
    exit 1
fi

ok "All required commands available"

# Check for optional dependencies
OPTIONAL_DEPS=""
for cmd in brightnessctl gslapper; do
    if ! command -v $cmd &>/dev/null; then
        OPTIONAL_DEPS="$OPTIONAL_DEPS $cmd"
    fi
done

if [ -n "$OPTIONAL_DEPS" ]; then
    warn "Optional dependencies not found:$OPTIONAL_DEPS"
    info "brightnessctl: for screen brightness control"
    info "gslapper: for video wallpapers (AUR: yay -S gslapper)"
fi

# ─────────────────────────────────────────────
#   Stop Existing Service
# ─────────────────────────────────────────────
header "Existing Installation"

if systemctl --user is-active sierra-launcher &>/dev/null; then
    step "Stopping existing sierra-launcher service..."
    systemctl --user stop sierra-launcher 2>/dev/null || true
    ok "Service stopped"
fi

# Remove old socket
if [ -S "${XDG_RUNTIME_DIR:-/tmp}/sierra-launcher.sock" ]; then
    step "Removing old socket..."
    rm -f "${XDG_RUNTIME_DIR:-/tmp}/sierra-launcher.sock"
    ok "Socket removed"
fi

# ─────────────────────────────────────────────
#   Build
# ─────────────────────────────────────────────
header "Build"

step "Compiling Sierra Launcher in release mode..."
info "This may take a few minutes on first build"
echo ""

cargo build --release 2>&1 | grep -E "^   Compiling|^    Finished|^error" | while read -r line; do
    case "$line" in
        *Compiling*)  info "  ${line}" ;;
        *Finished*)   echo -e "  ${GREEN}✓${RESET}  ${DIM}${line}${RESET}" ;;
        *error*)      echo -e "  ${RED}✗${RESET}  ${line}" ;;
    esac
done

if [ ! -f "target/release/sierra_launcher" ]; then
    fail "Build failed or binary not found"
    exit 1
fi

ok "Build complete"

# ─────────────────────────────────────────────
#   Install Binary
# ─────────────────────────────────────────────
header "Installation"

step "Installing daemon binary to /usr/bin/sierra-launcher-daemon..."
sudo cp target/release/sierra_launcher /usr/bin/sierra-launcher-daemon
sudo chmod +x /usr/bin/sierra-launcher-daemon
ok "Daemon binary installed"

step "Installing wrapper script to /usr/bin/sierra-launcher..."
sudo tee /usr/bin/sierra-launcher > /dev/null << 'WRAPPER'
#!/bin/bash
# sierra-launcher wrapper script
# Handles IPC communication with daemon

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
WRAPPER
sudo chmod +x /usr/bin/sierra-launcher
ok "Wrapper script installed"

# ─────────────────────────────────────────────
#   Systemd Service
# ─────────────────────────────────────────────
header "Systemd Service"

step "Installing systemd user service..."
mkdir -p ~/.config/systemd/user/

cat > ~/.config/systemd/user/sierra-launcher.service << 'SERVICE'
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
SERVICE

ok "Service file installed"

step "Reloading systemd daemon..."
systemctl --user daemon-reload
ok "Systemd reloaded"

step "Enabling autostart on login..."
systemctl --user enable sierra-launcher.service
ok "Autostart enabled"

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
#   Complete
# ─────────────────────────────────────────────
echo ""
echo -e "  ${BOLD}${GREEN}✓ Installation Complete!${RESET}"
echo ""
echo -e "  ${WHITE}Usage:${RESET}"
echo -e "      ${CYAN}sierra-launcher${RESET}        Show the launcher"
echo -e "      ${CYAN}Super+F${RESET} (keybind)     Your Hyprland keybind"
echo ""
echo -e "  ${WHITE}Service commands:${RESET}"
echo -e "      ${CYAN}systemctl --user status sierra-launcher${RESET}"
echo -e "      ${CYAN}systemctl --user stop sierra-launcher${RESET}"
echo -e "      ${CYAN}systemctl --user restart sierra-launcher${RESET}"
echo ""
echo -e "  ${WHITE}How it works:${RESET}"
echo -e "      ${DIM}· Daemon starts on login (no window visible)"
echo -e "      ${DIM}· Press Super+F → Window appears instantly (<50ms)"
echo -e "      ${DIM}· Press ESC → Window closes, daemon stays alive"
echo ""
