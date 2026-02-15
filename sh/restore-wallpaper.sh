#!/bin/bash
set -e

CACHE_FILE="$HOME/.cache/sierra/wallpapers/last_wallpaper.json"

sleep 0.5

if [ ! -f "$CACHE_FILE" ]; then
    echo "[Wallpaper] No cache file found at $CACHE_FILE"
    exit 0
fi

if ! command -v jq &>/dev/null; then
    echo "[Wallpaper] jq not installed, using fallback parser"
    WALLPAPER=$(grep -oP '"last_wallpaper":\s*"\K[^"]+' "$CACHE_FILE")
else
    WALLPAPER=$(jq -r '.last_wallpaper' "$CACHE_FILE")
fi

if [ -z "$WALLPAPER" ] || [ ! -f "$WALLPAPER" ]; then
    echo "[Wallpaper] Invalid or missing wallpaper: $WALLPAPER"
    exit 0
fi

pkill -9 gslapper 2>/dev/null || true
sleep 0.1

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
