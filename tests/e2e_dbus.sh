#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

DAEMON_LOG="$(mktemp /tmp/qp_daemon_XXXXXX.log)"
CLIENT_ERR="$(mktemp /tmp/qp_client_err_XXXXXX.log)"
TEST_STATE_DIR="$(mktemp -d /tmp/qp_state_XXXXXX)"
export XDG_STATE_HOME="$TEST_STATE_DIR"

cleanup() {
    if [ -n "${DAEMON_PID:-}" ] && kill -0 "$DAEMON_PID" 2>/dev/null; then
        kill -CONT "$DAEMON_PID" 2>/dev/null || true
        kill -9 "$DAEMON_PID" 2>/dev/null || true
    fi
    rm -f "$DAEMON_LOG" "$CLIENT_ERR"
    rm -rf "$TEST_STATE_DIR"
}
trap cleanup EXIT INT TERM

wait_for_name_released() {
    for i in {1..30}; do
        if ! gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus --method org.freedesktop.DBus.GetNameOwner org.quickpeek.QuickPeek >/dev/null 2>&1; then
            return 0
        fi
        sleep 0.05
    done
    echo "ERROR: Name org.quickpeek.QuickPeek still owned after quit"
    return 1
}

echo "=== 1. Private D-Bus Session Address ==="
echo "DBUS_SESSION_BUS_ADDRESS=$DBUS_SESSION_BUS_ADDRESS"

echo "=== 2. Starting Daemon ==="
"$REPO_ROOT/target/release/quickpeek" "$REPO_ROOT/tests/fixtures/sample.png" > "$DAEMON_LOG" 2>&1 &
DAEMON_PID=$!

# Poll for name on bus (<= 5s)
NAME_ACQUIRED=false
for i in {1..50}; do
    if gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus --method org.freedesktop.DBus.GetNameOwner org.quickpeek.QuickPeek >/dev/null 2>&1; then
        NAME_ACQUIRED=true
        break
    fi
    sleep 0.1
done

if [ "$NAME_ACQUIRED" != "true" ]; then
    echo "ERROR: Daemon failed to acquire D-Bus name within 5s"
    cat "$DAEMON_LOG"
    exit 1
fi

# Wait for WINDOW_OPENED log
for i in {1..30}; do
    if grep -q "WINDOW_OPENED" "$DAEMON_LOG" 2>/dev/null; then
        break
    fi
    sleep 0.1
done

grep -q "ROLE DAEMON" "$DAEMON_LOG"
grep -q "WINDOW_OPENED" "$DAEMON_LOG"
echo "Daemon started successfully (PID $DAEMON_PID). Log so far:"
cat "$DAEMON_LOG"

echo "=== 3. Client Invocation (sample2.png swap) ==="
CLIENT_OUT=$("$REPO_ROOT/target/release/quickpeek" "$REPO_ROOT/tests/fixtures/sample2.png")
echo "$CLIENT_OUT"
echo "$CLIENT_OUT" | grep -q "ROLE CLIENT"
echo "$CLIENT_OUT" | grep -q "CALL_MS"

# Give daemon brief moment to log swap
sleep 0.2
grep -q "IMAGE_SWAPPED" "$DAEMON_LOG"
grep -q "WARM_MS" "$DAEMON_LOG"
echo "Image swap verified in daemon log."

echo "=== 4. ShowFile on Nonexistent Path ==="
REPLY_NONEXIST=$(gdbus call --session --dest org.quickpeek.QuickPeek --object-path /org/quickpeek/QuickPeek --method org.quickpeek.QuickPeek.ShowFile "$REPO_ROOT/tests/fixtures/nonexistent_xyz.png")
echo "ShowFile reply: $REPLY_NONEXIST"
echo "$REPLY_NONEXIST" | grep -q "(false, "

echo "=== 5. ShowFile on Non-Image File (notimage.txt) ==="
REPLY_NOTIMAGE=$(gdbus call --session --dest org.quickpeek.QuickPeek --object-path /org/quickpeek/QuickPeek --method org.quickpeek.QuickPeek.ShowFile "$REPO_ROOT/tests/fixtures/notimage.txt")
echo "ShowFile reply: $REPLY_NOTIMAGE"
echo "$REPLY_NOTIMAGE" | grep -q "(false, "

echo "=== 6. Timeout Path (Daemon STOPped) ==="
kill -STOP "$DAEMON_PID"
START_TIME=$(date +%s)
set +e
"$REPO_ROOT/target/release/quickpeek" "$REPO_ROOT/tests/fixtures/sample.png" 2> "$CLIENT_ERR"
CLIENT_EXIT=$?
set -e
END_TIME=$(date +%s)
kill -CONT "$DAEMON_PID"

ELAPSED=$(( END_TIME - START_TIME ))
echo "Client exit code: $CLIENT_EXIT in ${ELAPSED}s"
cat "$CLIENT_ERR"
if [ "$CLIENT_EXIT" -ne 1 ]; then
    echo "ERROR: Expected client exit code 1 on timeout, got $CLIENT_EXIT"
    exit 1
fi
if [ "$ELAPSED" -gt 4 ]; then
    echo "ERROR: Timeout took too long (${ELAPSED}s > 4s)"
    exit 1
fi

echo "=== 7. D-Bus Introspection ==="
INTROSPECT_OUT=$(gdbus introspect --session --dest org.quickpeek.QuickPeek --object-path /org/quickpeek/QuickPeek)
echo "$INTROSPECT_OUT"
echo "$INTROSPECT_OUT" | grep -q "interface org.quickpeek.QuickPeek"
echo "$INTROSPECT_OUT" | grep -q "ShowFile"
echo "$INTROSPECT_OUT" | grep -q "Toggle"
echo "$INTROSPECT_OUT" | grep -q "Quit"

echo "=== 8. Quit Method ==="
gdbus call --session --dest org.quickpeek.QuickPeek --object-path /org/quickpeek/QuickPeek --method org.quickpeek.QuickPeek.Quit
wait "$DAEMON_PID" || true
grep -q "DAEMON_QUIT" "$DAEMON_LOG"

if gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus --method org.freedesktop.DBus.GetNameOwner org.quickpeek.QuickPeek >/dev/null 2>&1; then
    echo "ERROR: Name still owned after Quit"
    exit 1
fi
echo "Daemon quit verified and bus name released."

echo "=== 9. Toggle Cycle ==="
# Restart daemon with sample.png and swap sample2.png so window is open after swap
"$REPO_ROOT/target/release/quickpeek" "$REPO_ROOT/tests/fixtures/sample.png" > "$DAEMON_LOG" 2>&1 &
DAEMON_PID=$!

for i in {1..50}; do
    if gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus --method org.freedesktop.DBus.GetNameOwner org.quickpeek.QuickPeek >/dev/null 2>&1; then
        break
    fi
    sleep 0.1
done

# Wait for initial map
for i in {1..30}; do
    if grep -q "WINDOW_OPENED" "$DAEMON_LOG" 2>/dev/null; then
        break
    fi
    sleep 0.1
done

# Swap with sample2.png
"$REPO_ROOT/target/release/quickpeek" "$REPO_ROOT/tests/fixtures/sample2.png" >/dev/null
for i in {1..30}; do
    if grep -q "IMAGE_SWAPPED" "$DAEMON_LOG" 2>/dev/null; then
        break
    fi
    sleep 0.1
done

# Toggle 1: close window (window was open)
OUT1=$("$REPO_ROOT/target/release/quickpeek")
if [ -n "$OUT1" ]; then
    echo "ERROR: Expected empty stdout from toggle client, got: $OUT1"
    exit 1
fi
for i in {1..30}; do
    if grep -q "TOGGLE_CLOSED" "$DAEMON_LOG" 2>/dev/null; then
        break
    fi
    sleep 0.1
done

# Toggle 2: re-open window with current selection
OUT2=$("$REPO_ROOT/target/release/quickpeek")
if [ -n "$OUT2" ]; then
    echo "ERROR: Expected empty stdout from toggle client, got: $OUT2"
    exit 1
fi
for i in {1..30}; do
    if grep -q "TOGGLE_OPENED" "$DAEMON_LOG" 2>/dev/null; then
        break
    fi
    sleep 0.1
done
grep -q "WARM_MS" "$DAEMON_LOG"

# Toggle 3: close window again
OUT3=$("$REPO_ROOT/target/release/quickpeek")
if [ -n "$OUT3" ]; then
    echo "ERROR: Expected empty stdout from toggle client, got: $OUT3"
    exit 1
fi
echo "Toggle cycle (close -> open -> close) verified with empty client stdout."

echo "=== 10. No-selection Edge (Nested Private Bus) ==="
dbus-run-session bash -c "
set -euo pipefail
INNER_LOG=\$(mktemp /tmp/qp_inner_XXXXXX.log)
INNER_ERR=\$(mktemp /tmp/qp_inner_err_XXXXXX.log)
INNER_STATE_DIR=\$(mktemp -d /tmp/qp_inner_state_XXXXXX)
export XDG_STATE_HOME=\"\$INNER_STATE_DIR\"
trap 'kill -9 \"\$INNER_PID\" 2>/dev/null || true; rm -f \"\$INNER_LOG\" \"\$INNER_ERR\"; rm -rf \"\$INNER_STATE_DIR\"' EXIT

# Cold-start daemon with nonexistent path
'$REPO_ROOT/target/release/quickpeek' '/nonexistent/path/does_not_exist_xyz.png' > \"\$INNER_LOG\" 2> \"\$INNER_ERR\" &
INNER_PID=\$!

for i in {1..50}; do
    if gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus --method org.freedesktop.DBus.GetNameOwner org.quickpeek.QuickPeek >/dev/null 2>&1; then
        break
    fi
    sleep 0.1
done

for i in {1..50}; do
    if grep -q 'Error:' \"\$INNER_ERR\" 2>/dev/null; then
        break
    fi
    sleep 0.1
done

# Expect ROLE DAEMON, Error: on stderr, and NO WINDOW_OPENED
grep -q 'ROLE DAEMON' \"\$INNER_LOG\"
grep -q 'Error:' \"\$INNER_ERR\"
if grep -q 'WINDOW_OPENED' \"\$INNER_LOG\" 2>/dev/null; then
    echo 'ERROR: Window opened on invalid cold start'
    exit 1
fi

# No-args client exits 1 with stderr line
set +e
CLIENT_TEMP_ERR=\$(mktemp /tmp/qp_cli_temp_XXXXXX.log)
'$REPO_ROOT/target/release/quickpeek' 2> \"\$CLIENT_TEMP_ERR\"
CLIENT_STATUS=\$?
set -e
if [ \"\$CLIENT_STATUS\" -ne 1 ]; then
    echo 'ERROR: Expected exit code 1 from no-selection client'
    exit 1
fi
grep -q 'Error: no file to preview' \"\$CLIENT_TEMP_ERR\"
rm -f \"\$CLIENT_TEMP_ERR\"

# ShowFile with sample.png succeeds
SHOW_OUT=\$('$REPO_ROOT/target/release/quickpeek' '$REPO_ROOT/tests/fixtures/sample.png')
echo \"\$SHOW_OUT\" | grep -q 'ROLE CLIENT'

for i in {1..30}; do
    if grep -q 'WINDOW_OPENED' \"\$INNER_LOG\" 2>/dev/null; then
        break
    fi
    sleep 0.1
done

# Toggle closes it
'$REPO_ROOT/target/release/quickpeek' >/dev/null
for i in {1..30}; do
    if grep -q 'TOGGLE_CLOSED' \"\$INNER_LOG\" 2>/dev/null; then
        break
    fi
    sleep 0.1
done

# Toggle re-opens it
'$REPO_ROOT/target/release/quickpeek' >/dev/null
for i in {1..30}; do
    if grep -q 'TOGGLE_OPENED' \"\$INNER_LOG\" 2>/dev/null; then
        break
    fi
    sleep 0.1
done

gdbus call --session --dest org.quickpeek.QuickPeek --object-path /org/quickpeek/QuickPeek --method org.quickpeek.QuickPeek.Quit >/dev/null
wait \"\$INNER_PID\" || true
"
echo "No-selection edge verified in nested private bus."
sleep 0.2

echo "=== 11. Timing (5 No-Args Client Invocations) ==="
# Re-open window if closed so toggle is primed
"$REPO_ROOT/target/release/quickpeek" >/dev/null
sleep 0.1

# One warmup invocation
"$REPO_ROOT/target/release/quickpeek" >/dev/null
sleep 0.1

# 5 timed invocations with TIMEFORMAT='%R'
TIMEFORMAT='%R'
TIMINGS=()
for i in {1..5}; do
    # Time the execution of the no-args toggle client
    ELAPSED_SEC=$( { time "$REPO_ROOT/target/release/quickpeek" >/dev/null; } 2>&1 )
    TIMINGS+=("$ELAPSED_SEC")
    echo "Toggle run $i wall time: ${ELAPSED_SEC}s"
    sleep 0.05
done

# Convert to ms and verify GATE <= 80 ms
for t in "${TIMINGS[@]}"; do
    MS=$(awk -v t="$t" 'BEGIN { printf "%.0f", t * 1000 }')
    if [ "$MS" -gt 80 ]; then
        echo "ERROR: Toggle wall time exceeded 80 ms budget: ${MS}ms ($t s)"
        exit 1
    fi
done
echo "All 5 toggle timings verified within 80 ms budget."

echo "=== 12. Introspection (Toggle Method) ==="
FINAL_INTROSPECT=$(gdbus introspect --session --dest org.quickpeek.QuickPeek --object-path /org/quickpeek/QuickPeek)
echo "$FINAL_INTROSPECT"
echo "$FINAL_INTROSPECT" | grep -q "Toggle"
echo "$FINAL_INTROSPECT" | grep -q "ShowFile"
echo "$FINAL_INTROSPECT" | grep -q "Quit"

# Clean shutdown
gdbus call --session --dest org.quickpeek.QuickPeek --object-path /org/quickpeek/QuickPeek --method org.quickpeek.QuickPeek.Quit
wait "$DAEMON_PID" || true
wait_for_name_released
echo "Check 12 passed: introspection verified."

echo "=== 13. Persistence Round-Trip ==="
> "$DAEMON_LOG"
"$REPO_ROOT/target/release/quickpeek" "$REPO_ROOT/tests/fixtures/sample.png" > "$DAEMON_LOG" 2>&1 &
DAEMON_PID=$!

for i in {1..50}; do
    if gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus --method org.freedesktop.DBus.GetNameOwner org.quickpeek.QuickPeek >/dev/null 2>&1; then
        break
    fi
    sleep 0.1
done

for i in {1..30}; do
    if grep -q "WINDOW_OPENED" "$DAEMON_LOG" 2>/dev/null; then
        break
    fi
    sleep 0.1
done
grep -q "WINDOW_OPENED" "$DAEMON_LOG"

STATE_FILE="$XDG_STATE_HOME/quickpeek/last_selection"
if [ ! -f "$STATE_FILE" ]; then
    echo "ERROR: State file $STATE_FILE was not created"
    exit 1
fi
grep -q "tests/fixtures/sample.png" "$STATE_FILE"

# Wait for open debounce to expire before toggle close
sleep 0.15

# No-args toggle close (TOGGLE_CLOSED)
OUT_CLOSE=$("$REPO_ROOT/target/release/quickpeek")
if [ -n "$OUT_CLOSE" ]; then
    echo "ERROR: Expected empty stdout from toggle close, got: $OUT_CLOSE"
    exit 1
fi
for i in {1..30}; do
    if grep -q "TOGGLE_CLOSED" "$DAEMON_LOG" 2>/dev/null; then
        break
    fi
    sleep 0.1
done
grep -q "TOGGLE_CLOSED" "$DAEMON_LOG"

# Clean Quit
gdbus call --session --dest org.quickpeek.QuickPeek --object-path /org/quickpeek/QuickPeek --method org.quickpeek.QuickPeek.Quit
wait "$DAEMON_PID" || true
wait_for_name_released

# Cold BARE start (no args, bus name free) -> opens persisted selection
> "$DAEMON_LOG"
"$REPO_ROOT/target/release/quickpeek" > "$DAEMON_LOG" 2>&1 &
DAEMON_PID=$!

for i in {1..50}; do
    if gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus --method org.freedesktop.DBus.GetNameOwner org.quickpeek.QuickPeek >/dev/null 2>&1; then
        break
    fi
    sleep 0.1
done

for i in {1..30}; do
    if grep -q "WINDOW_OPENED" "$DAEMON_LOG" 2>/dev/null; then
        break
    fi
    sleep 0.1
done

grep -q "SELECTION_RESTORED" "$DAEMON_LOG"
grep -q "WINDOW_OPENED" "$DAEMON_LOG"
grep -q "WARM_MS" "$DAEMON_LOG"

# Clean Quit
gdbus call --session --dest org.quickpeek.QuickPeek --object-path /org/quickpeek/QuickPeek --method org.quickpeek.QuickPeek.Quit
wait "$DAEMON_PID" || true
wait_for_name_released
echo "Check 13 passed: persistence round-trip verified."

echo "=== 14. Service Mode Autostart (--service) ==="
# Cold --service start -> ROLE DAEMON and NO WINDOW_OPENED within 2 s
> "$DAEMON_LOG"
"$REPO_ROOT/target/release/quickpeek" --service > "$DAEMON_LOG" 2>&1 &
DAEMON_PID=$!

for i in {1..50}; do
    if gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus --method org.freedesktop.DBus.GetNameOwner org.quickpeek.QuickPeek >/dev/null 2>&1; then
        break
    fi
    sleep 0.1
done

grep -q "ROLE DAEMON" "$DAEMON_LOG"
grep -q "SELECTION_RESTORED" "$DAEMON_LOG"

sleep 2
if grep -q "WINDOW_OPENED" "$DAEMON_LOG" 2>/dev/null; then
    echo "ERROR: Service mode opened a window within 2s"
    exit 1
fi

# Second --service call while daemon running -> exit 0 silent
SERVICE_OUT=$("$REPO_ROOT/target/release/quickpeek" --service)
if [ -n "$SERVICE_OUT" ]; then
    echo "ERROR: Expected silent stdout from --service client, got: $SERVICE_OUT"
    exit 1
fi

# Then no-args client -> TOGGLE_OPENED + WINDOW_OPENED (from persisted selection)
TOGGLE_OUT=$("$REPO_ROOT/target/release/quickpeek")
if [ -n "$TOGGLE_OUT" ]; then
    echo "ERROR: Expected empty stdout from toggle client, got: $TOGGLE_OUT"
    exit 1
fi

for i in {1..30}; do
    if grep -q "WINDOW_OPENED" "$DAEMON_LOG" 2>/dev/null; then
        break
    fi
    sleep 0.1
done

grep -q "TOGGLE_OPENED" "$DAEMON_LOG"
grep -q "WINDOW_OPENED" "$DAEMON_LOG"
grep -q "WARM_MS" "$DAEMON_LOG"

# Clean Quit
gdbus call --session --dest org.quickpeek.QuickPeek --object-path /org/quickpeek/QuickPeek --method org.quickpeek.QuickPeek.Quit
wait "$DAEMON_PID" || true
wait_for_name_released
echo "Check 14 passed: service mode verified."

echo "=== 15. Stale Selection Handling ==="
BOGUS_PATH="/nonexistent/bogus_sample_xyz_123.png"
echo "$BOGUS_PATH" > "$XDG_STATE_HOME/quickpeek/last_selection"

> "$DAEMON_LOG"
DAEMON_ERR="$(mktemp /tmp/qp_daemon_err_XXXXXX.log)"
"$REPO_ROOT/target/release/quickpeek" > "$DAEMON_LOG" 2> "$DAEMON_ERR" &
DAEMON_PID=$!

for i in {1..50}; do
    if gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus --method org.freedesktop.DBus.GetNameOwner org.quickpeek.QuickPeek >/dev/null 2>&1; then
        break
    fi
    sleep 0.1
done

sleep 0.5

# Daemon must be running
if ! kill -0 "$DAEMON_PID" 2>/dev/null; then
    echo "ERROR: Daemon died on stale selection"
    cat "$DAEMON_ERR"
    exit 1
fi

grep -q "ROLE DAEMON" "$DAEMON_LOG"
grep -q "SELECTION_RESTORED" "$DAEMON_LOG"

if grep -q "WINDOW_OPENED" "$DAEMON_LOG" 2>/dev/null; then
    echo "ERROR: Window opened on stale selection"
    exit 1
fi

grep -q "Error:" "$DAEMON_ERR"
grep -q "last previewed file no longer exists" "$DAEMON_ERR"

# Clean Quit
gdbus call --session --dest org.quickpeek.QuickPeek --object-path /org/quickpeek/QuickPeek --method org.quickpeek.QuickPeek.Quit
wait "$DAEMON_PID" || true
wait_for_name_released
rm -f "$DAEMON_ERR"
echo "Check 15 passed: stale selection handling verified."

echo "=== 16. AT-SPI Fail-Open and Fallback (Hermetic) ==="
# Under dbus-run-session there is no org.a11y.Bus.
# The daemon must fail open: log ATSPI_UNAVAILABLE, fall back to persisted/memory,
# successfully open the image, and stay <= 80 ms.
mkdir -p "$XDG_STATE_HOME/quickpeek"
echo "$REPO_ROOT/tests/fixtures/sample.png" > "$XDG_STATE_HOME/quickpeek/last_selection"

> "$DAEMON_LOG"
"$REPO_ROOT/target/release/quickpeek" --service > "$DAEMON_LOG" 2>&1 &
DAEMON_PID=$!

for i in {1..50}; do
    if gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus --method org.freedesktop.DBus.GetNameOwner org.quickpeek.QuickPeek >/dev/null 2>&1; then
        break
    fi
    sleep 0.1
done

# Timed toggle invocation
TIMEFORMAT='%R'
TOGGLE_TIME=$( { time "$REPO_ROOT/target/release/quickpeek" >/dev/null; } 2>&1 )
echo "Fail-open toggle wall time: ${TOGGLE_TIME}s"

for i in {1..30}; do
    if grep -q "WINDOW_OPENED" "$DAEMON_LOG" 2>/dev/null; then
        break
    fi
    sleep 0.1
done

grep -q "ATSPI_MS" "$DAEMON_LOG"
grep -q "ATSPI_UNAVAILABLE" "$DAEMON_LOG"
grep -q -E "SELECTION_FALLBACK (memory|persisted)" "$DAEMON_LOG"
grep -q "WINDOW_OPENED" "$DAEMON_LOG"
grep -q "TOGGLE_OPENED" "$DAEMON_LOG"

# Verify timing <= 80ms
TOGGLE_MS=$(awk -v t="$TOGGLE_TIME" 'BEGIN { printf "%.0f", t * 1000 }')
if [ "$TOGGLE_MS" -gt 80 ]; then
    echo "ERROR: Fail-open toggle took ${TOGGLE_MS}ms, exceeded 80ms budget"
    exit 1
fi

# Clean Quit
gdbus call --session --dest org.quickpeek.QuickPeek --object-path /org/quickpeek/QuickPeek --method org.quickpeek.QuickPeek.Quit
wait "$DAEMON_PID" || true
wait_for_name_released
echo "Check 16 passed: AT-SPI fail-open and fallback verified within ${TOGGLE_MS}ms."

# Verify sandboxed state file was used
if [ ! -f "$TEST_STATE_DIR/quickpeek/last_selection" ]; then
    echo "ERROR: Sandboxed state file not created"
    exit 1
fi

echo "=== ALL 16 E2E D-BUS CHECKS PASSED ==="
