#!/usr/bin/env bash
set -euo pipefail

# ============================================
# LLM Server — Health Check
# ============================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

if [ -f "$PROJECT_DIR/.env" ]; then
    set -a
    source "$PROJECT_DIR/.env"
    set +a
fi

GATEWAY_PORT="${GATEWAY_PORT:-3000}"
BACKEND_PORT="${BACKEND_PORT:-8000}"

echo "🏥 LLM Server Health Check"
echo "=========================="
echo ""

# --- Gateway ---
echo "Gateway (:$GATEWAY_PORT):"
if curl -s "http://127.0.0.1:$GATEWAY_PORT/ready" > /dev/null 2>&1; then
    echo "  ✅ Healthy"
else
    echo "  ❌ Unreachable"
fi

# --- Backend ---
echo ""
echo "Backend (:$BACKEND_PORT):"
MODELS=$(curl -s "http://127.0.0.1:$BACKEND_PORT/v1/models" 2>/dev/null || echo "")
if [ -n "$MODELS" ]; then
    echo "  ✅ Healthy"
    echo "  📦 Models: $(echo "$MODELS" | python3 -c "import sys,json; data=json.load(sys.stdin); [print(f'     - {m[\"id\"]}') for m in data.get('data',[])]" 2>/dev/null || echo "$MODELS")"
else
    echo "  ❌ Unreachable"
fi

# --- Full health via gateway ---
echo ""
echo "Full status via /health:"
curl -s "http://127.0.0.1:$GATEWAY_PORT/health" 2>/dev/null | python3 -m json.tool 2>/dev/null || echo "  ❌ Gateway not available"
