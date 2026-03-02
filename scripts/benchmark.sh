#!/usr/bin/env bash
set -euo pipefail

# ============================================
# LLM Server — Benchmark Script
# Measures TTFT, TPS, throughput, memory
# Outputs JSON + generates HTML report
# ============================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
REPORT_DIR="$PROJECT_DIR/reports"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
RESULTS_FILE="$REPORT_DIR/benchmark_${TIMESTAMP}.json"

mkdir -p "$REPORT_DIR"

# Load config
if [ -f "$PROJECT_DIR/.env" ]; then
    set -a; source "$PROJECT_DIR/.env"; set +a
fi

HOST="${BENCHMARK_HOST:-127.0.0.1}"
PORT="${GATEWAY_PORT:-3000}"
BACKEND_PORT="${BACKEND_PORT:-8000}"
BASE_URL="http://${HOST}:${PORT}"
MODEL="${LLM_MODEL:-mlx-community/Qwen3.5-35B-A3B-Instruct-4bit}"
RUNS="${1:-3}"  # Number of runs per test (default 3)

# Version info
VERSION=$(cat "$PROJECT_DIR/VERSION" 2>/dev/null | tr -d '[:space:]' || echo "0.0.0")
GIT_COMMIT=$(git -C "$PROJECT_DIR" rev-parse --short HEAD 2>/dev/null || echo "unknown")

# ============================================
# Helper functions
# ============================================

check_server() {
    if ! curl -s "${BASE_URL}/health" > /dev/null 2>&1; then
        # Try direct backend
        if curl -s "http://${HOST}:${BACKEND_PORT}/v1/models" > /dev/null 2>&1; then
            BASE_URL="http://${HOST}:${BACKEND_PORT}"
            echo "⚠️  Gateway not running, benchmarking backend directly at ${BASE_URL}"
        else
            echo "❌ Server not reachable at ${BASE_URL} or port ${BACKEND_PORT}"
            echo "   Start the server first: ./scripts/start.sh"
            exit 1
        fi
    fi
}

get_memory_mb() {
    # Get approximate memory used by vllm/python processes
    ps aux | grep -E "(vllm|mlx|python)" | grep -v grep | awk '{sum += $6} END {printf "%.0f", sum/1024}' 2>/dev/null || echo "0"
}

# Single chat completion request, returns timing info
run_single_request() {
    local prompt="$1"
    local max_tokens="${2:-100}"
    local stream="${3:-false}"

    local start_ns=$(python3 -c "import time; print(int(time.time_ns()))")

    local response
    response=$(curl -s -w "\n%{time_starttransfer}|%{time_total}" \
        "${BASE_URL}/v1/chat/completions" \
        -H "Content-Type: application/json" \
        -d "{
            \"model\": \"${MODEL}\",
            \"messages\": [{\"role\": \"user\", \"content\": \"${prompt}\"}],
            \"max_tokens\": ${max_tokens},
            \"temperature\": 0.1,
            \"stream\": ${stream}
        }" 2>/dev/null)

    local body=$(echo "$response" | head -n -1)
    local timings=$(echo "$response" | tail -1)
    local ttfb=$(echo "$timings" | cut -d'|' -f1)
    local total=$(echo "$timings" | cut -d'|' -f2)

    # Extract token counts from response
    local completion_tokens=$(echo "$body" | python3 -c "
import sys, json
try:
    data = json.load(sys.stdin)
    print(data.get('usage', {}).get('completion_tokens', 0))
except: print(0)
" 2>/dev/null)

    local prompt_tokens=$(echo "$body" | python3 -c "
import sys, json
try:
    data = json.load(sys.stdin)
    print(data.get('usage', {}).get('prompt_tokens', 0))
except: print(0)
" 2>/dev/null)

    # Calculate TPS
    local tps="0"
    if [ "$completion_tokens" -gt 0 ] && [ "$(echo "$total > 0" | bc)" -eq 1 ]; then
        tps=$(echo "scale=2; $completion_tokens / $total" | bc)
    fi

    echo "${ttfb}|${total}|${completion_tokens}|${prompt_tokens}|${tps}"
}

# ============================================
# Main benchmark
# ============================================

echo ""
echo "╔══════════════════════════════════════════╗"
echo "║     🏎️  LLM Server Benchmark Suite      ║"
echo "╚══════════════════════════════════════════╝"
echo ""
echo "  Version: v${VERSION} (${GIT_COMMIT})"
echo "  Server:  ${BASE_URL}"
echo "  Model:   ${MODEL}"
echo "  Runs:    ${RUNS} per test"
echo "  Time:    $(date '+%Y-%m-%d %H:%M:%S')"
echo ""

check_server

# Get model info
MODEL_INFO=$(curl -s "${BASE_URL}/v1/models" 2>/dev/null || echo "{}")
MEMORY_BEFORE=$(get_memory_mb)

echo "  Memory:  ~${MEMORY_BEFORE} MB (before benchmark)"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# --- Test 1: Short prompt (TTFT focus) ---
echo ""
echo "📊 Test 1: Short Prompt (TTFT)"
echo "   Prompt: 'Say hello' | max_tokens: 20"

SHORT_RESULTS=()
for i in $(seq 1 "$RUNS"); do
    result=$(run_single_request "Say hello" 20)
    SHORT_RESULTS+=("$result")
    ttfb=$(echo "$result" | cut -d'|' -f1)
    total=$(echo "$result" | cut -d'|' -f2)
    tps=$(echo "$result" | cut -d'|' -f5)
    printf "   Run %d: TTFT=%.3fs  Total=%.3fs  TPS=%.1f\n" "$i" "$ttfb" "$total" "$tps"
done

# --- Test 2: Medium prompt (TPS focus) ---
echo ""
echo "📊 Test 2: Medium Generation (TPS)"
echo "   Prompt: 'Explain quantum computing in 3 paragraphs' | max_tokens: 200"

MEDIUM_RESULTS=()
for i in $(seq 1 "$RUNS"); do
    result=$(run_single_request "Explain quantum computing in 3 paragraphs" 200)
    MEDIUM_RESULTS+=("$result")
    ttfb=$(echo "$result" | cut -d'|' -f1)
    total=$(echo "$result" | cut -d'|' -f2)
    tokens=$(echo "$result" | cut -d'|' -f3)
    tps=$(echo "$result" | cut -d'|' -f5)
    printf "   Run %d: TTFT=%.3fs  Total=%.3fs  Tokens=%s  TPS=%.1f\n" "$i" "$ttfb" "$total" "$tokens" "$tps"
done

# --- Test 3: Long generation (sustained TPS) ---
echo ""
echo "📊 Test 3: Long Generation (Sustained TPS)"
echo "   Prompt: 'Write a detailed essay about AI history' | max_tokens: 500"

LONG_RESULTS=()
for i in $(seq 1 "$RUNS"); do
    result=$(run_single_request "Write a detailed essay about the history of artificial intelligence from 1950 to present" 500)
    LONG_RESULTS+=("$result")
    ttfb=$(echo "$result" | cut -d'|' -f1)
    total=$(echo "$result" | cut -d'|' -f2)
    tokens=$(echo "$result" | cut -d'|' -f3)
    tps=$(echo "$result" | cut -d'|' -f5)
    printf "   Run %d: TTFT=%.3fs  Total=%.3fs  Tokens=%s  TPS=%.1f\n" "$i" "$ttfb" "$total" "$tokens" "$tps"
done

MEMORY_AFTER=$(get_memory_mb)

# ============================================
# Calculate averages and generate JSON
# ============================================

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

python3 -c "
import json, sys

def parse_results(results):
    parsed = []
    for r in results:
        parts = r.split('|')
        parsed.append({
            'ttfb': float(parts[0]),
            'total': float(parts[1]),
            'completion_tokens': int(parts[2]),
            'prompt_tokens': int(parts[3]),
            'tps': float(parts[4])
        })
    return parsed

def avg(values):
    return sum(values) / len(values) if values else 0

def calc_stats(parsed):
    ttfbs = [r['ttfb'] for r in parsed]
    totals = [r['total'] for r in parsed]
    tps_vals = [r['tps'] for r in parsed if r['tps'] > 0]
    tokens = [r['completion_tokens'] for r in parsed]
    return {
        'ttfb_avg': round(avg(ttfbs), 4),
        'ttfb_min': round(min(ttfbs), 4) if ttfbs else 0,
        'ttfb_max': round(max(ttfbs), 4) if ttfbs else 0,
        'total_avg': round(avg(totals), 4),
        'tps_avg': round(avg(tps_vals), 2) if tps_vals else 0,
        'tps_min': round(min(tps_vals), 2) if tps_vals else 0,
        'tps_max': round(max(tps_vals), 2) if tps_vals else 0,
        'tokens_avg': round(avg(tokens), 0),
        'runs': [{'ttfb': r['ttfb'], 'total': r['total'], 'tokens': r['completion_tokens'], 'tps': r['tps']} for r in parsed]
    }

short = parse_results(sys.argv[1].split(','))
medium = parse_results(sys.argv[2].split(','))
long = parse_results(sys.argv[3].split(','))

report = {
    'timestamp': '$(date -u +%Y-%m-%dT%H:%M:%SZ)',
    'version': '${VERSION}',
    'git_commit': '${GIT_COMMIT}',
    'server': '${BASE_URL}',
    'model': '${MODEL}',
    'runs_per_test': int('${RUNS}'),
    'memory_mb': {'before': int('${MEMORY_BEFORE}'), 'after': int('${MEMORY_AFTER}')},
    'hardware': {
        'chip': 'Apple M4 Pro',
        'ram': '64GB',
        'bandwidth': '273 GB/s'
    },
    'tests': {
        'short': {'name': 'Short Prompt (TTFT)', 'max_tokens': 20, **calc_stats(short)},
        'medium': {'name': 'Medium Generation', 'max_tokens': 200, **calc_stats(medium)},
        'long': {'name': 'Long Generation', 'max_tokens': 500, **calc_stats(long)}
    }
}

with open('${RESULTS_FILE}', 'w') as f:
    json.dump(report, f, indent=2)

print(json.dumps(report, indent=2))
" "$(IFS=,; echo "${SHORT_RESULTS[*]}")" "$(IFS=,; echo "${MEDIUM_RESULTS[*]}")" "$(IFS=,; echo "${LONG_RESULTS[*]}")"

echo ""
echo "📄 Results saved: ${RESULTS_FILE}"

# Generate HTML report
"${SCRIPT_DIR}/generate_report.sh" "${RESULTS_FILE}"
