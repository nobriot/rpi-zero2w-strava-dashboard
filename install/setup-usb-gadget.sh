#!/bin/bash
# setup-usb-gadget.sh — Configure the Raspberry Pi Zero 2W as a USB serial gadget.
#
# This makes the Pi appear as a serial port (/dev/ttyACM0) when plugged into
# a host machine via USB. On the Pi side, the serial port is /dev/ttyGS0.
#
# Usage: sudo bash setup-usb-gadget.sh
#
# After running this script and rebooting:
#   - On the Pi: /dev/ttyGS0 will be available
#   - On the host: /dev/ttyACM0 (Linux) or /dev/tty.usbmodem* (macOS)

set -euo pipefail

BOOT_CONFIG="/boot/firmware/config.txt"
MODULES_FILE="/etc/modules"

if [ "$(id -u)" -ne 0 ]; then
    echo "Error: This script must be run as root (sudo)." >&2
    exit 1
fi

# Make a back-upu of the BOOT_CONFIG
# TODO: Stop making back-ups if there is a back-up already
cp $BOOT_CONFIG "$BOOT_CONFIG.$(date +%Y_%m_%d_).bkp"

echo "=== USB Gadget Serial Setup ==="

# 1. Update config.txt: switch dwc2 from host to peripheral mode
if grep -q 'dtoverlay=dwc2,dr_mode=host' "$BOOT_CONFIG"; then
    echo "[1/3] Switching dwc2 to peripheral mode in $BOOT_CONFIG..."
    sed -i 's/dtoverlay=dwc2,dr_mode=host/dtoverlay=dwc2,dr_mode=peripheral/' "$BOOT_CONFIG"
elif grep -q 'dtoverlay=dwc2,dr_mode=peripheral' "$BOOT_CONFIG"; then
    echo "[1/3] dwc2 already in peripheral mode — skipping."
elif grep -q 'dtoverlay=dwc2' "$BOOT_CONFIG"; then
    echo "[1/3] Updating dwc2 overlay to peripheral mode..."
    sed -i 's/dtoverlay=dwc2.*/dtoverlay=dwc2,dr_mode=peripheral/' "$BOOT_CONFIG"
else
    echo "[1/3] Adding dwc2 peripheral overlay to $BOOT_CONFIG..."
    echo "dtoverlay=dwc2,dr_mode=peripheral" >> "$BOOT_CONFIG"
fi

# 2. Ensure dwc2 and g_serial are loaded at boot
echo "[2/3] Ensuring kernel modules are configured..."
for mod in dwc2 g_serial; do
    if ! grep -q "^${mod}$" "$MODULES_FILE" 2>/dev/null; then
        echo "$mod" >> "$MODULES_FILE"
        echo "  Added $mod to $MODULES_FILE"
    else
        echo "  $mod already in $MODULES_FILE"
    fi
done

# 3. Remove otg_mode=1 if present (conflicts with gadget mode)
if grep -q 'otg_mode=1' "$BOOT_CONFIG"; then
    echo "[3/3] Removing otg_mode=1 (conflicts with gadget mode)..."
    sed -i '/^otg_mode=1/d' "$BOOT_CONFIG"
else
    echo "[3/3] No conflicting otg_mode setting found."
fi

echo ""
echo "Done! Reboot the Pi for changes to take effect."
echo "After reboot, /dev/ttyGS0 will be available for the USB daemon."
echo ""
echo "On the host machine, connect via USB and look for:"
echo "  Linux:  /dev/ttyACM0"
echo "  macOS:  /dev/tty.usbmodem*"
