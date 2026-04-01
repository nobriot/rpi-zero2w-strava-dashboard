#!/usr/bin/env bash
# check-rtc-readiness.sh — Run on the RPi to verify DS3231 RTC setup for rtcwake.
#
# Usage: sudo bash scripts/check-rtc-readiness.sh

set -euo pipefail

ok=0
warn=0
fail=0

pass()  { ok=$((ok+1));   echo "  [PASS] $1"; }
warn()  { warn=$((warn+1)); echo "  [WARN] $1"; }
fail()  { fail=$((fail+1)); echo "  [FAIL] $1"; }

echo "=== DS3231 RTC Readiness Check ==="
echo

# 1. I2C device visible?  Scan both bus 0 and bus 1.
echo "1. I2C bus scan"
if command -v i2cdetect &>/dev/null; then
  found_bus=""
  for bus in 0 1; do
    if i2cdetect -y "$bus" 2>/dev/null | grep -q "68"; then
      found_bus="$bus"
      break
    fi
  done
  if [ -n "$found_bus" ]; then
    pass "DS3231 detected at 0x68 on I2C bus $found_bus"
  else
    fail "No device at 0x68 on I2C bus 0 or 1 — check wiring"
  fi
else
  warn "i2cdetect not installed (apt install i2c-tools) — skipping"
fi
echo

# 2. dtoverlay in boot config?
echo "2. Boot config overlay"
found_overlay=false
for f in /boot/firmware/config.txt /boot/config.txt; do
  if [ -f "$f" ]; then
    if grep -q "^dtoverlay=i2c-rtc,ds3231" "$f"; then
      pass "dtoverlay=i2c-rtc,ds3231 found in $f"
      found_overlay=true
    elif grep -q "#.*dtoverlay=i2c-rtc,ds3231" "$f"; then
      fail "dtoverlay=i2c-rtc,ds3231 is commented out in $f"
      echo "        Fix: uncomment the line and reboot"
    else
      fail "dtoverlay=i2c-rtc,ds3231 NOT found in $f"
      echo "        Fix: echo 'dtoverlay=i2c-rtc,ds3231' | sudo tee -a $f && sudo reboot"
    fi
    break
  fi
done
if [ "$found_overlay" = false ] && [ $fail -eq 0 ]; then
  warn "Could not find boot config file"
fi
echo

# 3. /dev/rtc0 exists?
echo "3. RTC device node"
if [ -e /dev/rtc0 ]; then
  pass "/dev/rtc0 exists"
else
  fail "/dev/rtc0 does not exist — reboot after adding dtoverlay"
fi
echo

# 4. RTC chip identity
echo "4. RTC chip identity"
if [ -f /sys/class/rtc/rtc0/name ]; then
  chip=$(cat /sys/class/rtc/rtc0/name)
  # The Linux kernel uses the ds1307 driver for ds3231 — both names are fine
  if echo "$chip" | grep -qiE "ds3231|ds1307"; then
    pass "Chip: $chip (ds1307 driver handles ds3231)"
  else
    warn "Chip: $chip (expected ds3231/ds1307)"
  fi
else
  fail "Cannot read /sys/class/rtc/rtc0/name"
fi
echo

# 5. RTC time vs system time
echo "5. Clock sync"
if [ -f /sys/class/rtc/rtc0/since_epoch ]; then
  rtc_epoch=$(cat /sys/class/rtc/rtc0/since_epoch)
  sys_epoch=$(date +%s)
  drift=$(( sys_epoch - rtc_epoch ))
  drift_abs=${drift#-}
  if [ "$drift_abs" -le 5 ]; then
    pass "RTC in sync with system (drift: ${drift}s)"
  else
    warn "RTC drift: ${drift}s — run 'sudo hwclock -w' to sync"
  fi
else
  warn "Cannot read RTC epoch"
fi
echo

# 6. rtcwake binary
echo "6. rtcwake command"
if command -v rtcwake &>/dev/null; then
  pass "rtcwake found at $(which rtcwake)"
else
  fail "rtcwake not found — install util-linux (apt install util-linux)"
fi
echo

# 7. RTC alarm / IRQ support in kernel driver
echo "7. RTC alarm support"
has_alarm_issue=false

# Check if the driver exposes alarm via sysfs
if [ -f /sys/class/rtc/rtc0/wakealarm ]; then
  pass "/sys/class/rtc/rtc0/wakealarm exists"
else
  fail "/sys/class/rtc/rtc0/wakealarm missing — driver has no alarm/IRQ support"
  has_alarm_issue=true
fi

# Check device tree for interrupt configuration
if [ -d /proc/device-tree ]; then
  rtc_node=""
  for node in /proc/device-tree/soc/i2c*/rtc* /proc/device-tree/soc/i2c*/ds3231*; do
    if [ -d "$node" ] 2>/dev/null; then
      rtc_node="$node"
      break
    fi
  done
  if [ -n "$rtc_node" ]; then
    if [ -f "$rtc_node/interrupts" ] || [ -f "$rtc_node/interrupt-parent" ]; then
      pass "Device tree has interrupt configured at $rtc_node"
    else
      warn "Device tree node $rtc_node has NO interrupt property"
      echo "        The default i2c-rtc overlay may not enable alarm interrupts."
      echo "        rtcwake might still work if wakealarm sysfs is available."
    fi
  else
    warn "Could not find RTC node in device tree"
  fi
fi

# Try setting an alarm via sysfs directly (more diagnostic than rtcwake)
if [ -f /sys/class/rtc/rtc0/wakealarm ]; then
  future=$(($(date +%s) + 30))
  if echo "$future" > /sys/class/rtc/rtc0/wakealarm 2>/dev/null; then
    readback=$(cat /sys/class/rtc/rtc0/wakealarm 2>/dev/null || true)
    if [ -n "$readback" ]; then
      pass "Wake alarm set and readback OK (epoch $readback)"
      # Clear the alarm
      echo 0 > /sys/class/rtc/rtc0/wakealarm 2>/dev/null || true
    else
      warn "Wake alarm written but readback empty"
    fi
  else
    fail "Cannot write to /sys/class/rtc/rtc0/wakealarm"
    has_alarm_issue=true
  fi
fi
echo

# 8. rtcwake dry run
echo "8. rtcwake dry run"
if command -v rtcwake &>/dev/null && [ -e /dev/rtc0 ]; then
  future=$(($(date +%s) + 30))
  output=$(rtcwake -m no -t "$future" 2>&1) && rc=0 || rc=$?
  if [ $rc -eq 0 ]; then
    pass "rtcwake -m no succeeded"
  else
    fail "rtcwake -m no failed (exit $rc): $output"
    if [ "$has_alarm_issue" = true ]; then
      echo "        This is likely because the RTC driver lacks alarm/IRQ support."
      echo "        The default dtoverlay=i2c-rtc,ds3231 may not configure interrupts."
      echo "        Try a custom overlay or check if the DS3231 INT pin is wired to a GPIO."
    fi
  fi
else
  warn "Skipped (rtcwake or /dev/rtc0 missing)"
fi
echo

# 9. GPIO3 / INT pin check
echo "9. INT pin → GPIO3 wiring"
echo "   For wake-from-poweroff, DS3231 INT/SQW must be wired to GPIO3 (pin 7)."
echo "   From the PhotoPainter schematic: INT → R14 → RTC_INT net."
echo "   Check with a multimeter: continuity between DS3231 INT pin and Pi pin 7."
echo
echo "   Quick hardware test (will shut down the Pi!):"
echo "     sudo rtcwake -m off -s 120"
echo "   If it wakes after ~2 min → INT is wired to GPIO3 and everything works."
echo "   If it does NOT wake → INT is not wired to GPIO3; press power or unplug."
echo

# Summary
echo "=== Summary: $ok passed, $warn warnings, $fail failures ==="
if [ $fail -gt 0 ]; then
  echo "Fix the failures above before enabling [power] shutdown_after_cycle."
  exit 1
elif [ $warn -gt 0 ]; then
  echo "Warnings present — rtcwake may still work. Test with: sudo rtcwake -m off -s 120"
  exit 0
else
  echo "All checks passed! Safe to enable [power] shutdown_after_cycle = true"
  exit 0
fi
