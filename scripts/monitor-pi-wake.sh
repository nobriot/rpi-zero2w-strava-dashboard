#!/usr/bin/env bash
# monitor-pi-wake.sh — Run on a dev machine to monitor RPi wake/sleep cycles.
#
# Pings the Pi every INTERVAL seconds and logs state transitions (awake/asleep)
# with timestamps and battery percentage. Useful for verifying that
# shutdown_after_cycle + rtcwake is working correctly.
#
# Usage: bash scripts/monitor-pi-wake.sh <pi-hostname-or-ip> [interval_secs]
#
# Examples:
#   bash scripts/monitor-pi-wake.sh rpi-dash.local
#   bash scripts/monitor-pi-wake.sh 192.168.1.42 30
#   bash scripts/monitor-pi-wake.sh rpi-dash.local 10 | tee wake-log.txt

set -euo pipefail

PI_HOST="${1:?Usage: $0 <pi-hostname-or-ip> [interval_secs]}"
INTERVAL="${2:-30}"

LOGFILE="wake-log-$(date +%Y%m%d-%H%M%S).csv"
STATE="unknown"
LAST_CHANGE=""
AWAKE_COUNT=0
SLEEP_COUNT=0

# Read battery level using the Waveshare INA219 Python script on the Pi.
read_battery_cmd='python /home/pi/RPi_Zero_PhotoPainter/UPS/INA219.py 2>/dev/null | grep -i "battery level"'

echo "Monitoring $PI_HOST every ${INTERVAL}s (Ctrl-C to stop)"
echo "Logging to $LOGFILE"
echo

# CSV header
echo "timestamp,state,transition,uptime_response,battery" > "$LOGFILE"

while true; do
  ts=$(date "+%Y-%m-%d %H:%M:%S")

  # Try to get uptime + battery from the Pi via SSH, fall back to ping
  uptime_info=""
  battery=""
  if ssh -o ConnectTimeout=5 -o BatchMode=yes pi@"$PI_HOST" "uptime -s" 2>/dev/null; then
    uptime_info=$(ssh -o ConnectTimeout=5 -o BatchMode=yes pi@"$PI_HOST" "uptime -s" 2>/dev/null || true)
    battery=$(ssh -o ConnectTimeout=5 -o BatchMode=yes pi@"$PI_HOST" "$read_battery_cmd" 2>/dev/null || true)
    new_state="awake"
  elif ping -c 1 -W 2 pi@"$PI_HOST" &>/dev/null; then
    new_state="awake"
    uptime_info="ping-only"
  else
    new_state="asleep"
  fi

  # Detect transitions
  transition=""
  if [ "$new_state" != "$STATE" ] && [ "$STATE" != "unknown" ]; then
    if [ "$new_state" = "awake" ]; then
      transition="WOKE UP"
      AWAKE_COUNT=$((AWAKE_COUNT+1))
    else
      transition="WENT TO SLEEP"
      SLEEP_COUNT=$((SLEEP_COUNT+1))
    fi
    duration=""
    if [ -n "$LAST_CHANGE" ]; then
      elapsed=$(( $(date +%s) - LAST_CHANGE ))
      mins=$((elapsed / 60))
      secs=$((elapsed % 60))
      duration=" (was ${STATE} for ${mins}m${secs}s)"
    fi
    echo "[$ts] ** $transition **${duration}"
    LAST_CHANGE=$(date +%s)
  fi

  if [ "$STATE" = "unknown" ]; then
    echo "[$ts] Initial state: $new_state"
    LAST_CHANGE=$(date +%s)
  fi

  STATE="$new_state"

  # Print battery info when available
  if [ -n "$battery" ]; then
    echo "[$ts] $new_state | battery: $battery"
  else
    echo "[$ts] $new_state"
  fi

  # Log to CSV
  echo "$ts,$new_state,$transition,$uptime_info,$battery" >> "$LOGFILE"
  sleep "$INTERVAL"
done
