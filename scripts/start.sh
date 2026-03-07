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

# Load config but don't overwrite existing environment variables
if [ -f "$PROJECT_DIR/.env" ]; then
    set -a
    while IFS='=' read -r key value; do
        if [ -n "$key" ] && [[ ! "$key" =~ ^# ]]; then
            value="${value%\"}"
            value="${value#\"}"
            value="${value%\'}"
            value="${value#\'}"
            if [ -z "${!key:-}" ]; then
                export "$key=$value"
            fi
        fi
    done < "$PROJECT_DIR/.env"
    set +a
fi

LLM_MODEL="${LLM_MODEL:-mlx-community/Qwen3.5-35B-A3B-4bit}"
BACKEND_ENGINE="${BACKEND_ENGINE:-mlx}" # Options: mlx, llama.cpp, ollama
BACKEND_PORT="${BACKEND_PORT:-8081}"
GATEWAY_PORT="${GATEWAY_PORT:-8080}"

# --- Check if already running ---
if [ -f "$PIDS_DIR/backend.pid" ] && kill -0 "$(cat "$PIDS_DIR/backend.pid")" 2>/dev/null; then
    echo "⚠️  Backend already running (PID $(cat "$PIDS_DIR/backend.pid"))"
    echo "   Stop first with: ./scripts/stop.sh"
    exit 1
fi

# --- Pre-flight: Check Python dependencies ---
if [ "$BACKEND_ENGINE" = "mlx" ] || [ "$BACKEND_ENGINE" = "mlx_vlm" ]; then
    # Activate venv
    if [ -d "$PROJECT_DIR/.venv" ]; then
        source "$PROJECT_DIR/.venv/bin/activate"
    elif [ -d "$HOME/.venv-vllm-metal" ]; then
        source "$HOME/.venv-vllm-metal/bin/activate"
    fi

    # Check if MLX packages are installed
    MLX_OK=true
    if ! python3 -c "import mlx_lm" 2>/dev/null; then
        MLX_OK=false
    fi

    if [ "$MLX_OK" = false ]; then
        echo "⚠️  MLX packages not installed. Running setup..."
        echo ""
        "$SCRIPT_DIR/setup.sh"
        echo ""
        # Re-activate venv after setup
        source "$PROJECT_DIR/.venv/bin/activate"
    fi
fi

echo "🛡️ Starting Heimdall"
echo "======================"
echo ""
echo "📦 Model: $LLM_MODEL"
echo "🔌 Backend: 127.0.0.1:$BACKEND_PORT"
echo "🌐 Gateway: 0.0.0.0:$GATEWAY_PORT"
echo ""

# --- Start Backend Engine ---
if [ "$BACKEND_ENGINE" = "llama.cpp" ]; then
    echo "1️⃣  Starting llama.cpp backend..."
    nohup llama-server \
        --model "$LLM_MODEL" \
        --port "$BACKEND_PORT" \
        > "$LOGS_DIR/backend.log" 2>&1 &

elif [ "$BACKEND_ENGINE" = "ollama" ]; then
    echo "1️⃣  Starting Ollama backend..."
    OLLAMA_HOST="127.0.0.1:$BACKEND_PORT" nohup ollama serve \
        > "$LOGS_DIR/backend.log" 2>&1 &

else
    # Default: MLX
    # Auto-detect: vision/multimodal models use mlx_vlm, text models use mlx_lm
    IS_VLM=false
    if echo "$LLM_MODEL" | grep -qiE "vlm|vision"; then
        IS_VLM=true
        echo "1️⃣  Starting mlx_vlm backend (multimodal)..."
    else
        echo "1️⃣  Starting mlx_lm backend..."
    fi

    if [ -d "$PROJECT_DIR/.venv" ]; then
        source "$PROJECT_DIR/.venv/bin/activate"
    elif [ -d "$HOME/.venv-vllm-metal" ]; then
        source "$HOME/.venv-vllm-metal/bin/activate"
    fi

    if [ "$IS_VLM" = true ]; then
        nohup python3 -m mlx_vlm.server \
            --port "$BACKEND_PORT" \
            > "$LOGS_DIR/backend.log" 2>&1 &
    else
        nohup python3 -m mlx_lm.server \
            --model "$LLM_MODEL" \
            --port "$BACKEND_PORT" \
            > "$LOGS_DIR/backend.log" 2>&1 &
    fi
fi
echo $! > "$PIDS_DIR/backend.pid"
echo "   PID: $(cat "$PIDS_DIR/backend.pid") — Log: logs/backend.log"

# --- Wait for backend to be ready ---
echo "   Waiting for backend..."

HEALTH_URL="http://127.0.0.1:$BACKEND_PORT/v1/models"
if [ "$BACKEND_ENGINE" = "ollama" ]; then
    HEALTH_URL="http://127.0.0.1:$BACKEND_PORT/api/tags"
fi

for i in $(seq 1 60); do
    if curl -s "$HEALTH_URL" > /dev/null 2>&1; then
        echo "   ✅ Backend ready!"
        break
    fi
    if [ "$i" -eq 60 ]; then
        echo "   ⚠️  Backend not ready after 60s — check logs/backend.log"
        echo "   Starting gateway anyway..."
    fi
    sleep 2
done

# --- Start Embedding Server (MLX) ---
EMBEDDING_MODEL="${EMBEDDING_MODEL:-}"
EMBEDDING_PORT="${EMBEDDING_PORT:-8001}"
if [ -n "$EMBEDDING_MODEL" ]; then
    echo ""
    echo "2️⃣  Starting MLX embedding server ($EMBEDDING_MODEL)..."

    if [ -d "$PROJECT_DIR/.venv" ]; then
        source "$PROJECT_DIR/.venv/bin/activate"
    fi

    EMBEDDING_MODEL="$EMBEDDING_MODEL" EMBEDDING_PORT="$EMBEDDING_PORT" \
        nohup python3 "$PROJECT_DIR/scripts/embedding_server.py" \
        > "$LOGS_DIR/embedding.log" 2>&1 &
    echo $! > "$PIDS_DIR/embedding.pid"
    echo "   PID: $(cat "$PIDS_DIR/embedding.pid") — Log: logs/embedding.log"

    echo "   Waiting for embedding server..."
    for i in $(seq 1 30); do
        if curl -s "http://127.0.0.1:$EMBEDDING_PORT/health" > /dev/null 2>&1; then
            echo "   ✅ Embedding server ready!"
            break
        fi
        if [ "$i" -eq 30 ]; then
            echo "   ⚠️  Embedding server not ready after 30s — check logs/embedding.log"
        fi
        sleep 2
    done
fi

# --- Start Rust Gateway ---
echo ""
echo "3️⃣  Starting Rust gateway..."
GATEWAY_BIN="$PROJECT_DIR/gateway/target/release/heimdall-gateway"
if [ ! -f "$GATEWAY_BIN" ]; then
    echo "   Gateway not built. Building..."
    (cd "$PROJECT_DIR/gateway" && source "$HOME/.cargo/env" && cargo build --release 2>&1 | tail -1)
fi

BACKEND_ENGINE="$BACKEND_ENGINE" nohup "$GATEWAY_BIN" \
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
