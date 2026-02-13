#!/bin/bash
# sierra-launcher wrapper script
# Handles IPC communication with daemon

SOCKET_PATH="${XDG_RUNTIME_DIR:-/tmp}/sierra-launcher.sock"
BINARY="/usr/bin/sierra-launcher-daemon"

# Check if daemon is running
if [ -S "$SOCKET_PATH" ]; then
    # Try socat first, then nc as fallback
    if command -v socat &>/dev/null; then
        echo "SHOW" | socat - UNIX-CONNECT:"$SOCKET_PATH" 2>/dev/null && exit 0
    elif command -v nc &>/dev/null; then
        echo "SHOW" | nc -U "$SOCKET_PATH" 2>/dev/null && exit 0
    fi
fi

# Daemon not running or IPC failed, start it
exec "$BINARY"
