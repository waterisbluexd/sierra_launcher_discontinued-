#!/bin/bash
set -e

RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}╔════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║  Sierra Launcher (Wayland Only) v2.0  ║${NC}"
echo -e "${BLUE}║     Optimized for Fast Startup         ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════╝${NC}"
echo ""

if [ -z "$WAYLAND_DISPLAY" ] && [ "$XDG_SESSION_TYPE" != "wayland" ]; then
    echo -e "${RED}ERROR: Sierra Launcher requires Wayland${NC}"
    echo "Current session: ${XDG_SESSION_TYPE:-unknown}"
    exit 1
fi

echo -e "${YELLOW}Installing dependencies...${NC}"

if command -v pacman &>/dev/null; then
    sudo pacman -S --needed \
        rust cargo gcc pkg-config gtk3 \
        brightnessctl pulseaudio redshift ffmpeg \
        lm_sensors jq
    
    if ! command -v gslapper &>/dev/null; then
        echo -e "${YELLOW}gSlapper not found, installing from AUR...${NC}"
        if command -v yay &>/dev/null; then
            yay -S --needed gslapper
        elif command -v paru &>/dev/null; then
            paru -S --needed gslapper
        else
            echo -e "${YELLOW}No AUR helper found (yay/paru)${NC}"
            echo -e "${YELLOW}Install gSlapper manually:${NC}"
            echo "  yay -S gslapper"
            echo "  OR from: https://gitlab.com/phoneybadger/gslapper"
        fi
    fi

elif command -v apt &>/dev/null; then
    sudo apt update
    sudo apt install -y \
        build-essential cargo pkg-config libgtk-3-dev \
        brightnessctl pulseaudio redshift ffmpeg \
        lm-sensors jq

    if ! command -v gslapper &>/dev/null; then
        echo -e "${YELLOW}gSlapper not found in apt, install manually from:${NC}"
        echo "https://gitlab.com/phoneybadger/gslapper"
    fi

elif command -v dnf &>/dev/null; then
    sudo dnf install -y \
        rust cargo gcc pkg-config gtk3-devel \
        brightnessctl pulseaudio redshift ffmpeg \
        lm_sensors jq

    if ! command -v gslapper &>/dev/null; then
        echo -e "${YELLOW}gSlapper not found in dnf, install manually from:${NC}"
        echo "https://gitlab.com/phoneybadger/gslapper"
    fi

else
    echo -e "${RED}ERROR: Unsupported package manager${NC}"
    echo "Please install manually:"
    echo "  rust, cargo, gtk3-dev, pkg-config, brightnessctl, pulseaudio"
    echo "  redshift, ffmpeg, gslapper, lm_sensors, jq"
    exit 1
fi

echo ""
echo -e "${YELLOW}Building Sierra Launcher (release)...${NC}"
echo -e "${BLUE}This may take a few minutes on first build...${NC}"
cargo build --release

echo -e "${YELLOW}Installing binary...${NC}"
sudo install -Dm755 target/release/sierra_launcher /usr/local/bin/sierra-launcher

echo -e "${YELLOW}Installing wallpaper restoration service...${NC}"

# Create restore script
sudo tee /usr/local/bin/restore-wallpaper.sh > /dev/null << 'EOF'
#!/bin/bash
# Restore wallpaper from Sierra launcher cache on login
set -e

CACHE_FILE="$HOME/.cache/sierra/wallpapers/last_wallpaper.json"

# Wait for compositor to be ready
sleep 0.5

if [ ! -f "$CACHE_FILE" ]; then
    echo "[Wallpaper] No cache file found"
    exit 0
fi

# Extract wallpaper path from JSON
if command -v jq &>/dev/null; then
    WALLPAPER=$(jq -r '.last_wallpaper' "$CACHE_FILE")
else
    WALLPAPER=$(grep -oP '"last_wallpaper":\s*"\K[^"]+' "$CACHE_FILE")
fi

if [ -z "$WALLPAPER" ] || [ ! -f "$WALLPAPER" ]; then
    echo "[Wallpaper] Invalid or missing wallpaper: $WALLPAPER"
    exit 0
fi

# Kill existing gSlapper
pkill -9 gslapper 2>/dev/null || true
sleep 0.1

# Determine type and restore
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

echo -e "${GREEN}✓ Wallpaper restoration service installed${NC}"

CONFIG_DIR="$HOME/.config/sierra"
CACHE_DIR="$HOME/.cache/sierra"
CONFIG_FILE="$CONFIG_DIR/Sierra"

mkdir -p "$CONFIG_DIR"
mkdir -p "$CACHE_DIR"

DEFAULT_WALLPAPER_DIR="$HOME/Pictures/Wallpapers"
mkdir -p "$DEFAULT_WALLPAPER_DIR"

if [ ! -f "$CONFIG_FILE" ]; then
    echo -e "${YELLOW}Creating default config...${NC}"

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

    echo -e "${GREEN}✓ Config created at $CONFIG_FILE${NC}"
else
    echo -e "${GREEN}✓ Config already exists at $CONFIG_FILE${NC}"
fi
echo ""
echo -e "${BLUE}╔════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║        Running Performance Test        ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════╝${NC}"

echo -e "${YELLOW}Testing startup time...${NC}"
START_TIME=$(date +%s%3N)
timeout 5 sierra-launcher &>/dev/null &
LAUNCHER_PID=$!
sleep 0.5
kill $LAUNCHER_PID 2>/dev/null || true
END_TIME=$(date +%s%3N)
STARTUP_MS=$((END_TIME - START_TIME))

if [ $STARTUP_MS -lt 500 ]; then
    echo -e "${GREEN}✓ Excellent startup time: ${STARTUP_MS}ms${NC}"
elif [ $STARTUP_MS -lt 800 ]; then
    echo -e "${YELLOW}⚠ Good startup time: ${STARTUP_MS}ms${NC}"
else
    echo -e "${RED}⚠ Slow startup: ${STARTUP_MS}ms (expected <500ms)${NC}"
    echo -e "${YELLOW}  Run: RUST_LOG=debug sierra-launcher to diagnose${NC}"
fi
echo ""
echo -e "${GREEN}╔════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║     Sierra Launcher Installed!         ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════╝${NC}"
echo ""
echo -e "${GREEN}✓ Binary:   /usr/local/bin/sierra-launcher${NC}"
echo -e "${GREEN}✓ Config:   $CONFIG_FILE${NC}"
echo -e "${GREEN}✓ Cache:    $CACHE_DIR${NC}"
echo -e "${GREEN}✓ Service:  restore-wallpaper.service${NC}"