#!/bin/bash
SOCKET_PATH="${XDG_RUNTIME_DIR:-/tmp}/sierra-launcher.sock"
BINARY="/usr/bin/sierra-launcher-daemon"

if [ -S "$SOCKET_PATH" ]; then
    if command -v socat &>/dev/null; then
        echo "SHOW" | socat - UNIX-CONNECT:"$SOCKET_PATH" 2>/dev/null && exit 0
    elif command -v nc &>/dev/null; then
        echo "SHOW" | nc -U "$SOCKET_PATH" 2>/dev/null && exit 0
    fi
fi

exec "$BINARY"
