#!/usr/bin/env bash
set -euo pipefail

# ============================================
# LLM Server — Stop Script
# Gracefully stops gateway + backend
# ============================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
PIDS_DIR="$PROJECT_DIR/.pids"

echo "🛑 Stopping Heimdall"
echo "======================"

# --- Stop Gateway ---
if [ -f "$PIDS_DIR/gateway.pid" ]; then
    PID=$(cat "$PIDS_DIR/gateway.pid")
    if kill -0 "$PID" 2>/dev/null; then
        kill "$PID"
        echo "  ✅ Gateway stopped (PID $PID)"
    else
        echo "  ⚠️  Gateway was not running"
    fi
    rm -f "$PIDS_DIR/gateway.pid"
else
    echo "  ⚠️  No gateway PID file"
fi

# --- Stop Backend ---
if [ -f "$PIDS_DIR/backend.pid" ]; then
    PID=$(cat "$PIDS_DIR/backend.pid")
    if kill -0 "$PID" 2>/dev/null; then
        kill "$PID"
        echo "  ✅ Backend stopped (PID $PID)"
    else
        echo "  ⚠️  Backend was not running"
    fi
    rm -f "$PIDS_DIR/backend.pid"
else
    echo "  ⚠️  No backend PID file"
fi

echo ""
echo "✅ All services stopped"
