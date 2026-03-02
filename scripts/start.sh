#!/usr/bin/env bash
set -euo pipefail

# ============================================
# LLM Server — Start Script
# Starts vllm-mlx backend + Rust Gateway
# ============================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
LOGS_DIR="$PROJECT_DIR/logs"
PIDS_DIR="$PROJECT_DIR/.pids"

mkdir -p "$LOGS_DIR" "$PIDS_DIR"

# Load config
if [ -f "$PROJECT_DIR/.env" ]; then
    set -a
    source "$PROJECT_DIR/.env"
    set +a
fi

LLM_MODEL="${LLM_MODEL:-mlx-community/Qwen3.5-35B-A3B-Instruct-4bit}"
BACKEND_PORT="${BACKEND_PORT:-8000}"
GATEWAY_PORT="${GATEWAY_PORT:-3000}"

# --- Check if already running ---
if [ -f "$PIDS_DIR/backend.pid" ] && kill -0 "$(cat "$PIDS_DIR/backend.pid")" 2>/dev/null; then
    echo "⚠️  Backend already running (PID $(cat "$PIDS_DIR/backend.pid"))"
    echo "   Stop first with: ./scripts/stop.sh"
    exit 1
fi

echo "🛡️ Starting Heimdall"
echo "======================"
echo ""
echo "📦 Model: $LLM_MODEL"
echo "🔌 Backend: 127.0.0.1:$BACKEND_PORT"
echo "🌐 Gateway: 0.0.0.0:$GATEWAY_PORT"
echo ""

# --- Start vllm-mlx Backend ---
echo "1️⃣  Starting vllm-mlx backend..."
if [ -d "$PROJECT_DIR/.venv" ]; then
    source "$PROJECT_DIR/.venv/bin/activate"
elif [ -d "$HOME/.venv-vllm-metal" ]; then
    source "$HOME/.venv-vllm-metal/bin/activate"
fi

nohup vllm serve "$LLM_MODEL" \
    --port "$BACKEND_PORT" \
    > "$LOGS_DIR/backend.log" 2>&1 &
echo $! > "$PIDS_DIR/backend.pid"
echo "   PID: $(cat "$PIDS_DIR/backend.pid") — Log: logs/backend.log"

# --- Wait for backend to be ready ---
echo "   Waiting for backend..."
for i in $(seq 1 60); do
    if curl -s "http://127.0.0.1:$BACKEND_PORT/v1/models" > /dev/null 2>&1; then
        echo "   ✅ Backend ready!"
        break
    fi
    if [ "$i" -eq 60 ]; then
        echo "   ⚠️  Backend not ready after 60s — check logs/backend.log"
        echo "   Starting gateway anyway..."
    fi
    sleep 2
done

# --- Start Rust Gateway ---
echo ""
echo "2️⃣  Starting Rust gateway..."
GATEWAY_BIN="$PROJECT_DIR/gateway/target/release/heimdall-gateway"
if [ ! -f "$GATEWAY_BIN" ]; then
    echo "   Gateway not built. Building..."
    (cd "$PROJECT_DIR/gateway" && source "$HOME/.cargo/env" && cargo build --release 2>&1 | tail -1)
fi

nohup "$GATEWAY_BIN" \
    > "$LOGS_DIR/gateway.log" 2>&1 &
echo $! > "$PIDS_DIR/gateway.pid"
echo "   PID: $(cat "$PIDS_DIR/gateway.pid") — Log: logs/gateway.log"

sleep 1

echo ""
echo "✅ Heimdall started!"
echo ""
echo "Usage:"
echo "  curl http://localhost:$GATEWAY_PORT/health"
echo "  curl http://localhost:$GATEWAY_PORT/v1/models"
echo "  curl http://localhost:$GATEWAY_PORT/v1/chat/completions \\"
echo "    -H 'Content-Type: application/json' \\"
echo "    -d '{\"model\":\"$LLM_MODEL\",\"messages\":[{\"role\":\"user\",\"content\":\"Hello!\"}]}'"
