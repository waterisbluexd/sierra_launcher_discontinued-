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
