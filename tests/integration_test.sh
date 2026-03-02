#!/usr/bin/env bash
set -euo pipefail

# ============================================
# Heimdall — Integration & Smoke Tests
# Runs against a live server
# Usage: ./tests/integration_test.sh [gateway_url]
# ============================================

GATEWAY_URL="${1:-http://localhost:3000}"
PASS=0
FAIL=0
TOTAL=0

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_pass() {
    PASS=$((PASS + 1))
    TOTAL=$((TOTAL + 1))
    echo -e "  ${GREEN}✅ PASS${NC} — $1"
}

log_fail() {
    FAIL=$((FAIL + 1))
    TOTAL=$((TOTAL + 1))
    echo -e "  ${RED}❌ FAIL${NC} — $1"
    echo -e "       $2"
}

echo ""
echo "🛡️ Heimdall Integration Tests"
echo "=============================="
echo "Target: $GATEWAY_URL"
echo ""

# ============================================
# ST-001: GET /v1/models → 200 + model list
# ============================================
echo "📋 ST-001: GET /v1/models"
RESPONSE=$(curl -s -w "\n%{http_code}" "$GATEWAY_URL/v1/models" 2>/dev/null || echo -e "\n000")
HTTP_CODE=$(echo "$RESPONSE" | tail -1)
BODY=$(echo "$RESPONSE" | sed '$d')

if [ "$HTTP_CODE" = "200" ] && echo "$BODY" | python3 -c "import sys,json; d=json.load(sys.stdin); assert 'data' in d" 2>/dev/null; then
    MODEL_ID=$(echo "$BODY" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['data'][0]['id'])" 2>/dev/null || echo "unknown")
    log_pass "GET /v1/models → 200 (model: $MODEL_ID)"
else
    log_fail "GET /v1/models → expected 200" "Got HTTP $HTTP_CODE"
fi

# ============================================
# ST-004: GET /metrics → 200 + Prometheus
# ============================================
echo "📋 ST-004: GET /metrics"
RESPONSE=$(curl -s -w "\n%{http_code}" "$GATEWAY_URL/metrics" 2>/dev/null || echo -e "\n000")
HTTP_CODE=$(echo "$RESPONSE" | tail -1)
BODY=$(echo "$RESPONSE" | sed '$d')

if [ "$HTTP_CODE" = "200" ] && echo "$BODY" | grep -q "http_requests"; then
    log_pass "GET /metrics → 200 + Prometheus format"
else
    log_fail "GET /metrics → expected 200 + prometheus" "Got HTTP $HTTP_CODE"
fi

# ============================================
# Health Check
# ============================================
echo "📋 Health: GET /health"
RESPONSE=$(curl -s -w "\n%{http_code}" "$GATEWAY_URL/health" 2>/dev/null || echo -e "\n000")
HTTP_CODE=$(echo "$RESPONSE" | tail -1)

if [ "$HTTP_CODE" = "200" ]; then
    log_pass "GET /health → 200"
else
    log_fail "GET /health → expected 200" "Got HTTP $HTTP_CODE"
fi

# ============================================
# ST-002 / IT-001: POST /v1/chat/completions (non-stream)
# ============================================
echo "📋 IT-001 / ST-002: POST /v1/chat/completions (non-stream)"
RESPONSE=$(curl -s -w "\n%{http_code}" "$GATEWAY_URL/v1/chat/completions" \
    -H "Content-Type: application/json" \
    -d '{
        "model": "'"${LLM_MODEL:-mlx-community/Qwen3.5-35B-A3B-Instruct-4bit}"'",
        "messages": [{"role": "user", "content": "Say hello in one word."}],
        "max_tokens": 10,
        "stream": false
    }' 2>/dev/null || echo -e "\n000")
HTTP_CODE=$(echo "$RESPONSE" | tail -1)
BODY=$(echo "$RESPONSE" | sed '$d')

if [ "$HTTP_CODE" = "200" ]; then
    CONTENT=$(echo "$BODY" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['choices'][0]['message']['content'])" 2>/dev/null || echo "")
    if [ -n "$CONTENT" ]; then
        log_pass "Non-stream chat → 200 (response: '$CONTENT')"
    else
        log_fail "Non-stream chat → 200 but no content" "Body: ${BODY:0:200}"
    fi
else
    log_fail "Non-stream chat → expected 200" "Got HTTP $HTTP_CODE — ${BODY:0:200}"
fi

# ============================================
# ST-003 / IT-003: POST /v1/chat/completions (stream)
# ============================================
echo "📋 IT-003 / ST-003: POST /v1/chat/completions (stream=true)"
STREAM_RESPONSE=$(curl -s --max-time 30 "$GATEWAY_URL/v1/chat/completions" \
    -H "Content-Type: application/json" \
    -d '{
        "model": "'"${LLM_MODEL:-mlx-community/Qwen3.5-35B-A3B-Instruct-4bit}"'",
        "messages": [{"role": "user", "content": "Say hi."}],
        "max_tokens": 5,
        "stream": true
    }' 2>/dev/null || echo "TIMEOUT")

if echo "$STREAM_RESPONSE" | grep -q "data:"; then
    CHUNK_COUNT=$(echo "$STREAM_RESPONSE" | grep -c "data:" || echo "0")
    HAS_DONE=$(echo "$STREAM_RESPONSE" | grep -c "\[DONE\]" || echo "0")
    if [ "$HAS_DONE" -gt 0 ]; then
        log_pass "SSE stream → $CHUNK_COUNT chunks + [DONE]"
    else
        log_pass "SSE stream → $CHUNK_COUNT chunks (no [DONE] marker)"
    fi
else
    log_fail "SSE stream → expected data: chunks" "Got: ${STREAM_RESPONSE:0:200}"
fi

# ============================================
# Root endpoint
# ============================================
echo "📋 Root: GET /"
RESPONSE=$(curl -s -w "\n%{http_code}" "$GATEWAY_URL/" 2>/dev/null || echo -e "\n000")
HTTP_CODE=$(echo "$RESPONSE" | tail -1)
BODY=$(echo "$RESPONSE" | sed '$d')

if [ "$HTTP_CODE" = "200" ] && echo "$BODY" | grep -q "Heimdall"; then
    log_pass "GET / → 200 + Heimdall"
else
    log_fail "GET / → expected 200 + Heimdall" "Got HTTP $HTTP_CODE"
fi

# ============================================
# Summary
# ============================================
echo ""
echo "=============================="
echo -e "Results: ${GREEN}$PASS passed${NC}, ${RED}$FAIL failed${NC} / $TOTAL total"
echo "=============================="
echo ""

if [ $FAIL -gt 0 ]; then
    echo -e "${RED}❌ Some tests failed!${NC}"
    exit 1
else
    echo -e "${GREEN}✅ All tests passed!${NC}"
    exit 0
fi
