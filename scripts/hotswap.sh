#!/usr/bin/env bash
set -e

NEW_MODEL="$1"
if [ -z "$NEW_MODEL" ]; then
    echo "❌ Missing model name."
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

echo "🔄 Hot-swapping model to: $NEW_MODEL"

# 1. Safely update .env file for persistence
if [ "$(uname)" = "Darwin" ]; then
    sed -i '' "s|^LLM_MODEL=.*|LLM_MODEL=\"$NEW_MODEL\"|" "$PROJECT_DIR/.env"
else
    sed -i "s|^LLM_MODEL=.*|LLM_MODEL=\"$NEW_MODEL\"|" "$PROJECT_DIR/.env"
fi

# 1.5 If launchd is supervising mlx_lm.server, drive the swap THROUGH launchd.
#
# Background: a launchd LaunchAgent (com.asgard.heimdall-mlx) keeps mlx_lm.server
# alive with KeepAlive=true and a HARDCODED --model arg in the plist. When
# hotswap.sh used to kill the mlx_lm.server directly, launchd would respawn it
# 30s later with the plist's hardcoded model — racing the manually-spawned new
# server for port 8081. Whichever bound first won, silently flipping the active
# model and breaking subsequent swaps.
#
# Fix: if launchd is managing the process, update the plist's --model arg and
# bounce the agent. Launchd then becomes the single source of truth for the
# process, no race.
LAUNCHD_LABEL="com.asgard.heimdall-mlx"
PLIST="$HOME/Library/LaunchAgents/${LAUNCHD_LABEL}.plist"
LAUNCHD_MANAGED=0
if [ -f "$PLIST" ] && launchctl list "$LAUNCHD_LABEL" >/dev/null 2>&1; then
    LAUNCHD_MANAGED=1
fi

if [ "$LAUNCHD_MANAGED" = "1" ]; then
    echo "🪪 launchd-managed: $LAUNCHD_LABEL → updating plist + bouncing"
    # Find the index in ProgramArguments where the --model flag lives, replace
    # the value at index+1. Robust against future arg-order changes via plutil
    # (no XML-text mutation).
    PLIST_LEN=$(/usr/libexec/PlistBuddy -c "Print :ProgramArguments" "$PLIST" 2>/dev/null \
        | awk '/^Array \{/{count++; next} /^\}$/{exit} count{n++} END{print n-1}')
    MODEL_IDX=""
    for i in $(seq 0 "${PLIST_LEN:-10}"); do
        v=$(/usr/libexec/PlistBuddy -c "Print :ProgramArguments:$i" "$PLIST" 2>/dev/null || true)
        if [ "$v" = "--model" ]; then
            MODEL_IDX=$((i + 1))
            break
        fi
    done
    if [ -n "$MODEL_IDX" ]; then
        /usr/libexec/PlistBuddy -c "Set :ProgramArguments:$MODEL_IDX $NEW_MODEL" "$PLIST"
        echo "   ✓ plist :ProgramArguments:$MODEL_IDX = $NEW_MODEL"
    else
        echo "⚠️  Could not locate --model index in plist; falling back to direct kill"
        LAUNCHD_MANAGED=0
    fi

    if [ "$LAUNCHD_MANAGED" = "1" ]; then
        # Bounce the agent so it picks up the new --model.
        # Important: `launchctl kickstart -k` only restarts the process — it does
        # NOT reload the plist from disk. So our PlistBuddy edit would be ignored.
        # We must `unload` (clears launchd's cached config) then `load` (re-reads
        # plist with new args). Modern equivalent: `bootout` + `bootstrap`.
        echo "   ↻ launchctl unload + load (reloads plist from disk)..."
        launchctl unload "$PLIST" 2>/dev/null || true
        # Brief pause to let the prior process fully exit + port free
        for _ in {1..10}; do
            lsof -i :8081 -sTCP:LISTEN >/dev/null 2>&1 || break
            sleep 1
        done
        # Sweep any stragglers (e.g. manually-spawned mlx_lm.server from earlier)
        ORPHANS=$(pgrep -u "$USER" -f "mlx_lm.server" 2>/dev/null || true)
        for p in $ORPHANS; do
            kill "$p" 2>/dev/null || true
        done
        sleep 2
        launchctl load "$PLIST"
        echo "   ✓ launchd reloaded with new --model"

        # Poll for readiness (launchd respawns the process which then loads weights)
        POLL_TIMEOUT_SEC="${HOTSWAP_POLL_TIMEOUT_SEC:-360}"
        echo "⏳ Polling backend health (max ${POLL_TIMEOUT_SEC}s, launchd-managed)..."
        for _ in $(seq 1 $((POLL_TIMEOUT_SEC / 2))); do
            if curl -s http://127.0.0.1:8081/v1/models >/dev/null 2>&1; then
                # Verify the loaded model actually matches what we asked for
                ACTUAL_PID=$(lsof -t -i :8081 -sTCP:LISTEN 2>/dev/null | head -1 || true)
                ACTUAL_MODEL=$(ps -p "$ACTUAL_PID" -o command= 2>/dev/null \
                    | sed -nE 's/.*--model[[:space:]]+([^[:space:]]+).*/\1/p')
                if [ "$ACTUAL_MODEL" = "$NEW_MODEL" ]; then
                    echo "✅ launchd swap successful: $NEW_MODEL (PID $ACTUAL_PID)"
                    exit 0
                fi
            fi
            sleep 2
        done
        echo "❌ launchd swap timeout — model didn't come up as $NEW_MODEL"
        exit 1
    fi
fi

# Fallback path (no launchd or plist edit failed): legacy direct kill+spawn.
# Past failure mode: pidfile pointed to a zombie that failed to bind 8081, while
# the real server ran with no pidfile. We'd kill the zombie, spawn a new server
# that also failed to bind (port still held by the un-killed real owner), and the
# swap appeared "successful" because /v1/models was still answered by the old
# server with the OLD model. So: always go via lsof, then fall back to pidfile.
PID_FILE="$PROJECT_DIR/.pids/backend.pid"

terminate_pid() {
    local p="$1"
    [ -z "$p" ] && return 0
    if kill -0 "$p" 2>/dev/null; then
        echo "🛑 Terminating PID $p ($(ps -p "$p" -o command= 2>/dev/null | head -c 80))"
        kill "$p" 2>/dev/null || true
        for i in {1..5}; do
            kill -0 "$p" 2>/dev/null || return 0
            sleep 1
        done
        kill -9 "$p" 2>/dev/null || true
    fi
}

# Kill whoever actually has port 8081 LISTEN (this is the authoritative owner).
PORT_PID=$(lsof -t -i :8081 -sTCP:LISTEN 2>/dev/null | head -1 || true)
if [ -n "$PORT_PID" ]; then
    terminate_pid "$PORT_PID"
fi

# Kill anything mentioned in the pidfile too (likely a zombie from a prior failed swap).
if [ -f "$PID_FILE" ]; then
    FILE_PID=$(cat "$PID_FILE" 2>/dev/null || true)
    if [ -n "$FILE_PID" ] && [ "$FILE_PID" != "$PORT_PID" ]; then
        terminate_pid "$FILE_PID"
    fi
    rm -f "$PID_FILE"
fi

# Sweep any other lingering mlx_lm.server processes (e.g. orphan zombies that
# never bound 8081). Restrict to current user to be safe.
ORPHANS=$(pgrep -u "$USER" -f "mlx_lm.server" 2>/dev/null || true)
for p in $ORPHANS; do
    terminate_pid "$p"
done

# 3. Wait for port 8081 specifically
echo "⏳ Waiting for port 8081 to be released..."
for _ in {1..10}; do
    if ! lsof -i :8081 >/dev/null 2>&1; then break; fi
    sleep 1
done

# 4. Activate VENV and Start Python Backend in background
cd "$PROJECT_DIR"
source .venv/bin/activate 2>/dev/null || true

BACKEND_PORT=8081
echo "🚀 Spawning new mlx_lm.server backend (Port: $BACKEND_PORT)..."
nohup python3 -m mlx_lm.server --model "$NEW_MODEL" --port "$BACKEND_PORT" > "$PROJECT_DIR/logs/backend.log" 2>&1 &
NEW_PID=$!
echo $NEW_PID > "$PID_FILE"

# 5. Poll for readiness
# Bumped from 60×2s=2min → 180×2s=6min to handle large models (e.g. gemma-4-31b ~19GB)
# whose weight load + MLX graph compile can exceed 2 minutes on first cold start.
POLL_TIMEOUT_SEC="${HOTSWAP_POLL_TIMEOUT_SEC:-360}"
POLL_INTERVAL_SEC=2
ITERATIONS=$((POLL_TIMEOUT_SEC / POLL_INTERVAL_SEC))
echo "⏳ Polling backend health endpoint (max ${POLL_TIMEOUT_SEC}s)..."
for _ in $(seq 1 "$ITERATIONS"); do
    if curl -s http://127.0.0.1:8081/v1/models >/dev/null 2>&1; then
        echo "✅ Model $NEW_MODEL successfully loaded and ready!"
        exit 0
    fi
    sleep "$POLL_INTERVAL_SEC"
done

echo "❌ Hot-swap failed or timed out (${POLL_TIMEOUT_SEC}s) during model loading."
# If failed, kill it so it doesn't hang zombie
kill -9 $NEW_PID 2>/dev/null || true
rm -f "$PID_FILE"
exit 1
