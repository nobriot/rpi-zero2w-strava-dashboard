#!/usr/bin/env bash
# check-gpio4.sh -- Read the current level of GPIO4 (physical pin 7).
#
# Usage: bash scripts/check-gpio4.sh

set -euo pipefail

GPIO_PIN=4

if ! command -v pinctrl &>/dev/null; then
  echo "[FAIL] pinctrl not found (install raspi-utils)"
  exit 1
fi

level=$(pinctrl get $GPIO_PIN | grep -oP '(hi|lo)')

echo "GPIO$GPIO_PIN = $level"

if [ "$level" = "lo" ]; then
  echo "[LOW]  INT pin is asserted"
else
  echo "[HIGH] INT pin is idle (no pending alarm)"
fi
