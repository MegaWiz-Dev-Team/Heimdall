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

# 2. Kill current python mlx backend securely
PID_FILE="$PROJECT_DIR/.pids/backend.pid"
if [ -f "$PID_FILE" ]; then
    PID=$(cat "$PID_FILE")
    if kill -0 "$PID" 2>/dev/null; then
        echo "🛑 Terminating running backend (PID: $PID)..."
        kill "$PID" || true
        # Wait up to 5 seconds for graceful shutdown
        for i in {1..5}; do
            if ! kill -0 "$PID" 2>/dev/null; then break; fi
            sleep 1
        done
        # Forcefully terminate if still hanging
        if kill -0 "$PID" 2>/dev/null; then
            kill -9 "$PID" || true
        fi
    fi
    rm -f "$PID_FILE"
fi

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
echo "⏳ Polling backend health endpoint..."
for _ in {1..60}; do
    if curl -s http://127.0.0.1:8081/v1/models >/dev/null 2>&1; then
        echo "✅ Model $NEW_MODEL successfully loaded and ready!"
        exit 0
    fi
    sleep 2
done

echo "❌ Hot-swap failed or timed out during model loading."
# If failed, kill it so it doesn't hang zombie
kill -9 $NEW_PID 2>/dev/null || true
rm -f "$PID_FILE"
exit 1
