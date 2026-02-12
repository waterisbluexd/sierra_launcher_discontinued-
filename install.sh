#!/bin/bash
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
    echo -e "  ${GRAY}$(printf '%.0s─' $(seq 1 40))${RESET}"
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
echo -e "  ${BOLD}${WHITE}Sierra Launcher${RESET}  ${DIM}${GRAY}v2.0 · Wayland only${RESET}"
echo ""


# ─────────────────────────────────────────────
#   Wayland Check
# ─────────────────────────────────────────────
header "Environment"

if [ -z "$WAYLAND_DISPLAY" ] && [ "$XDG_SESSION_TYPE" != "wayland" ]; then
    fail "Wayland session not detected"
    info "Current session: ${XDG_SESSION_TYPE:-unknown}"
    info "Sierra requires a Wayland compositor (e.g. Hyprland, Sway, GNOME on Wayland)"
    echo ""
    exit 1
else
    ok "Wayland session detected  ${DIM}${GRAY}($XDG_SESSION_TYPE)${RESET}"
fi


# ─────────────────────────────────────────────
#   Dependencies
# ─────────────────────────────────────────────
header "Dependencies"

if command -v pacman &>/dev/null; then
    step "Installing packages via pacman..."
    sudo pacman -S --needed \
        rust cargo gcc pkg-config gtk3 \
        brightnessctl pulseaudio redshift ffmpeg \
        lm_sensors jq \
        2>&1 | grep -E "installing|upgrading|up to date" | while read -r line; do
            info "$line"
        done
    ok "Core packages installed"

    if ! command -v gslapper &>/dev/null; then
        step "gSlapper not found — checking AUR helpers..."
        if command -v yay &>/dev/null; then
            yay -S --needed gslapper && ok "gSlapper installed via yay"
        elif command -v paru &>/dev/null; then
            paru -S --needed gslapper && ok "gSlapper installed via paru"
        else
            warn "No AUR helper found"
            info "Install gSlapper manually:"
            info "  yay -S gslapper"
            info "  https://gitlab.com/phoneybadger/gslapper"
        fi
    else
        ok "gSlapper already installed"
    fi

elif command -v apt &>/dev/null; then
    step "Installing packages via apt..."
    sudo apt update -qq
    sudo apt install -y \
        build-essential cargo pkg-config libgtk-3-dev \
        brightnessctl pulseaudio redshift ffmpeg \
        lm-sensors jq &>/dev/null
    ok "Core packages installed"

    if ! command -v gslapper &>/dev/null; then
        warn "gSlapper not in apt repositories"
        info "Install manually: https://gitlab.com/phoneybadger/gslapper"
    else
        ok "gSlapper already installed"
    fi

elif command -v dnf &>/dev/null; then
    step "Installing packages via dnf..."
    sudo dnf install -y \
        rust cargo gcc pkg-config gtk3-devel \
        brightnessctl pulseaudio redshift ffmpeg \
        lm_sensors jq &>/dev/null
    ok "Core packages installed"

    if ! command -v gslapper &>/dev/null; then
        warn "gSlapper not in dnf repositories"
        info "Install manually: https://gitlab.com/phoneybadger/gslapper"
    else
        ok "gSlapper already installed"
    fi

else
    fail "No supported package manager found"
    info "Please install these manually:"
    info "  rust  cargo  gtk3-dev  pkg-config  brightnessctl"
    info "  pulseaudio  redshift  ffmpeg  gslapper  lm_sensors  jq"
    echo ""
    exit 1
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

ok "Build complete"

step "Installing binary to /usr/local/bin/sierra-launcher"
sudo install -Dm755 target/release/sierra_launcher /usr/local/bin/sierra-launcher
ok "Binary installed"


# ─────────────────────────────────────────────
#   Wallpaper Service
# ─────────────────────────────────────────────
header "Wallpaper Restoration Service"

step "Writing restore script..."
sudo tee /usr/local/bin/restore-wallpaper.sh > /dev/null << 'EOF'
#!/bin/bash
set -e

CACHE_FILE="$HOME/.cache/sierra/wallpapers/last_wallpaper.json"

sleep 0.5

if [ ! -f "$CACHE_FILE" ]; then
    echo "[Wallpaper] No cache file found"
    exit 0
fi

if command -v jq &>/dev/null; then
    WALLPAPER=$(jq -r '.last_wallpaper' "$CACHE_FILE")
else
    WALLPAPER=$(grep -oP '"last_wallpaper":\s*"\K[^"]+' "$CACHE_FILE")
fi

if [ -z "$WALLPAPER" ] || [ ! -f "$WALLPAPER" ]; then
    echo "[Wallpaper] Invalid or missing wallpaper: $WALLPAPER"
    exit 0
fi

pkill -9 gslapper 2>/dev/null || true
sleep 0.1

EXT="${WALLPAPER##*.}"
case "${EXT,,}" in
    mp4|mkv|webm|avi)
        gslapper -o "loop no-audio" "*" "$WALLPAPER" &
        ;;
    jpg|jpeg|png|webp|bmp)
        gslapper -o fill "*" "$WALLPAPER" &
        ;;
esac

echo "[Wallpaper] ✓ Restored: $WALLPAPER"
EOF
sudo chmod +x /usr/local/bin/restore-wallpaper.sh
ok "Restore script written"

step "Creating systemd user service..."
mkdir -p "$HOME/.config/systemd/user"
cat > "$HOME/.config/systemd/user/restore-wallpaper.service" << 'EOF'
[Unit]
Description=Restore Sierra Launcher Wallpaper
After=graphical-session.target

[Service]
Type=oneshot
ExecStart=/usr/local/bin/restore-wallpaper.sh
RemainAfterExit=yes

[Install]
WantedBy=graphical-session.target
EOF

systemctl --user daemon-reload
systemctl --user enable restore-wallpaper.service
ok "Service enabled  ${DIM}${GRAY}(restore-wallpaper.service)${RESET}"


# ─────────────────────────────────────────────
#   Config
# ─────────────────────────────────────────────
header "Configuration"

CONFIG_DIR="$HOME/.config/sierra"
CACHE_DIR="$HOME/.cache/sierra"
CONFIG_FILE="$CONFIG_DIR/Sierra"
DEFAULT_WALLPAPER_DIR="$HOME/Pictures/Wallpapers"

mkdir -p "$CONFIG_DIR" "$CACHE_DIR" "$DEFAULT_WALLPAPER_DIR"

if [ ! -f "$CONFIG_FILE" ]; then
    step "Writing default config..."

    cat > "$CONFIG_FILE" << EOF
font = "Monospace"
font_size = 14.0

title_text = " sierra-launcher "
title_animation = "Wave"

wallpaper_dir = "$DEFAULT_WALLPAPER_DIR"

use_pywal = false

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
    ok "Config created"
    info "$CONFIG_FILE"
else
    ok "Config already exists — skipping"
    info "$CONFIG_FILE"
fi


# ─────────────────────────────────────────────
#   Startup Benchmark
# ─────────────────────────────────────────────
header "Performance Check"

step "Measuring startup time..."
START_TIME=$(date +%s%3N)
timeout 5 sierra-launcher &>/dev/null &
LAUNCHER_PID=$!
sleep 0.5
kill $LAUNCHER_PID 2>/dev/null || true
END_TIME=$(date +%s%3N)
STARTUP_MS=$((END_TIME - START_TIME))

if [ $STARTUP_MS -lt 500 ]; then
    ok "Startup time: ${BOLD}${STARTUP_MS}ms${RESET}  ${DIM}${GREEN}excellent${RESET}"
elif [ $STARTUP_MS -lt 800 ]; then
    warn "Startup time: ${BOLD}${STARTUP_MS}ms${RESET}  ${DIM}${YELLOW}acceptable${RESET}"
else
    warn "Startup time: ${BOLD}${STARTUP_MS}ms${RESET}  ${DIM}${RED}slow — expected <500ms${RESET}"
    info "Run: RUST_LOG=debug sierra-launcher  to diagnose"
fi


# ─────────────────────────────────────────────
#   Done
# ─────────────────────────────────────────────
echo ""
echo -e "  ${GRAY}$(printf '%.0s─' $(seq 1 40))${RESET}"
echo ""
echo -e "  ${BOLD}${GREEN}Installation complete${RESET}"
echo ""
echo -e "  ${GRAY}binary   ${RESET}  /usr/local/bin/sierra-launcher"
echo -e "  ${GRAY}config   ${RESET}  $CONFIG_FILE"
echo -e "  ${GRAY}cache    ${RESET}  $CACHE_DIR"
echo -e "  ${GRAY}service  ${RESET}  restore-wallpaper.service"
echo -e "  ${GRAY}walls    ${RESET}  $DEFAULT_WALLPAPER_DIR"
echo ""
echo -e "  ${DIM}${GRAY}Run:  sierra-launcher${RESET}"
echo ""