#!/usr/bin/env bash
# tests/manual_atspi.sh — Live test of QuickPeek AT-SPI Dolphin selection resolution
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

echo "=== QuickPeek Live AT-SPI Dolphin Selection Test ==="

# 1. Check AT-SPI address on session bus
if ! A11Y_ADDR=$(busctl --user call org.a11y.Bus /org/a11y/bus org.a11y.Bus GetAddress 2>/dev/null); then
    echo "ERROR: org.a11y.Bus is not running on the user session bus."
    exit 1
fi
echo "Discovered AT-SPI address: $A11Y_ADDR"

# 2. Check if Dolphin is running with accessibility
DOLPHIN_PID=$(pgrep -x dolphin | head -n 1 || true)
if [ -z "$DOLPHIN_PID" ]; then
    echo "Dolphin is not running. Starting Dolphin with QT_LINUX_ACCESSIBILITY_ALWAYS_ON=1..."
    QT_LINUX_ACCESSIBILITY_ALWAYS_ON=1 dolphin "$REPO_ROOT/tests/fixtures" >/dev/null 2>&1 &
    sleep 2
    DOLPHIN_PID=$(pgrep -x dolphin | head -n 1)
fi

echo "Dolphin PID: $DOLPHIN_PID"

# 3. Focus Dolphin window in niri if niri is running
if command -v niri >/dev/null 2>&1; then
    DOLPHIN_WIN_ID=$(niri msg windows 2>/dev/null | grep -B 5 "PID: $DOLPHIN_PID" | grep "Window ID" | head -n 1 | awk '{print $3}' | tr -d ':' || true)
    if [ -n "$DOLPHIN_WIN_ID" ]; then
        echo "Focusing Dolphin window (ID $DOLPHIN_WIN_ID) via niri..."
        niri msg action focus-window --id "$DOLPHIN_WIN_ID" || true
        sleep 0.3
    fi
fi

# 4. Run probe to verify live resolution
echo "Running live selection probe..."
PROBE_OUTPUT=$(cargo run --manifest-path "$REPO_ROOT/probe/Cargo.toml" 2>&1)
echo "$PROBE_OUTPUT"

# Assertions
echo "$PROBE_OUTPUT" | grep -q "RESOLVED: Ok" || echo "$PROBE_OUTPUT" | grep -q "Resolved: Ok"
echo "=== Manual AT-SPI test PASSED ==="
