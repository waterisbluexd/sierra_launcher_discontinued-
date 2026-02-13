#!/bin/bash
# Restore wallpaper from Sierra launcher cache on login
# Install to: /usr/local/bin/restore-wallpaper.sh

set -e

CACHE_FILE="$HOME/.cache/sierra/wallpapers/last_wallpaper.json"

# Wait for compositor to be ready
sleep 0.5

if [ ! -f "$CACHE_FILE" ]; then
    echo "[Wallpaper] No cache file found at $CACHE_FILE"
    exit 0
fi

# Extract wallpaper path from JSON (requires jq)
if ! command -v jq &>/dev/null; then
    echo "[Wallpaper] jq not installed, using fallback parser"
    # Fallback: simple grep/sed parsing
    WALLPAPER=$(grep -oP '"last_wallpaper":\s*"\K[^"]+' "$CACHE_FILE")
else
    WALLPAPER=$(jq -r '.last_wallpaper' "$CACHE_FILE")
fi

if [ -z "$WALLPAPER" ] || [ ! -f "$WALLPAPER" ]; then
    echo "[Wallpaper] Invalid or missing wallpaper: $WALLPAPER"
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
        echo "[Wallpaper] Restoring video wallpaper: $WALLPAPER"
        gslapper -o "loop no-audio" "*" "$WALLPAPER" &
        ;;
    jpg|jpeg|png|webp|bmp)
        echo "[Wallpaper] Restoring image wallpaper: $WALLPAPER"
        gslapper -o fill "*" "$WALLPAPER" &
        ;;
    *)
        echo "[Wallpaper] Unknown file type: .$EXT_LOWER"
        exit 1
        ;;
esac

echo "[Wallpaper] ✓ Restored successfully"
exit 0
