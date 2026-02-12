#!/bin/bash
# Installation script for Sierra Launcher daemon mode

set -e

echo "=== Sierra Launcher Daemon Installer ==="
echo

# Check if running as root
if [ "$EUID" -eq 0 ]; then 
   echo "ERROR: Do not run as root! Run as your normal user."
   exit 1
fi

# Check for required commands
for cmd in cargo socat; do
    if ! command -v $cmd &> /dev/null; then
        echo "ERROR: $cmd not found. Please install it first."
        exit 1
    fi
done

echo "Step 1: Building sierra-launcher in release mode..."
cargo build --release

if [ ! -f "target/release/sierra_launcher" ]; then
    echo "ERROR: Build failed or binary not found"
    exit 1
fi

echo "Step 2: Installing binary..."
sudo cp target/release/sierra_launcher /usr/bin/sierra-launcher-daemon
sudo chmod +x /usr/bin/sierra-launcher-daemon

echo "Step 3: Installing wrapper script..."
sudo cp sierra-launcher-wrapper.sh /usr/bin/sierra-launcher
sudo chmod +x /usr/bin/sierra-launcher

echo "Step 4: Installing systemd service..."
mkdir -p ~/.config/systemd/user/
cp sierra-launcher.service ~/.config/systemd/user/
systemctl --user daemon-reload

echo "Step 5: Enabling autostart..."
systemctl --user enable sierra-launcher.service

echo
echo "=== Installation Complete! ==="
echo
echo "To start the daemon now:"
echo "  systemctl --user start sierra-launcher"
echo
echo "To open the launcher (daemon will start if not running):"
echo "  sierra-launcher"
echo
echo "To stop the daemon:"
echo "  systemctl --user stop sierra-launcher"
echo
echo "To check daemon status:"
echo "  systemctl --user status sierra-launcher"
echo
echo "Your Hyprland keybind should be:"
echo "  bind = \$mainMod, F, exec, sierra-launcher"
echo
