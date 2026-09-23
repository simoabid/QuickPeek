#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

DAEMON_LOG="$(mktemp /tmp/qp_daemon_XXXXXX.log)"
CLIENT_ERR="$(mktemp /tmp/qp_client_err_XXXXXX.log)"

cleanup() {
    if [ -n "${DAEMON_PID:-}" ] && kill -0 "$DAEMON_PID" 2>/dev/null; then
        kill -CONT "$DAEMON_PID" 2>/dev/null || true
        kill -9 "$DAEMON_PID" 2>/dev/null || true
    fi
    rm -f "$DAEMON_LOG" "$CLIENT_ERR"
}
trap cleanup EXIT INT TERM

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
grep -q "SHOW_MS" "$DAEMON_LOG"
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
echo "=== ALL E2E D-BUS CHECKS PASSED ==="
