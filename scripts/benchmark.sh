#!/usr/bin/env bash
set -euo pipefail

# ============================================
# Heimdall — Benchmark Script
# Supports multiple models for comparison
# Usage:
#   ./scripts/benchmark.sh                      # benchmark default model
#   ./scripts/benchmark.sh --runs 5             # 5 runs per test
#   ./scripts/benchmark.sh --models model1 model2  # compare models
#   ./scripts/benchmark.sh --all                # benchmark all loaded models
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
DEFAULT_MODEL="${LLM_MODEL:-mlx-community/Qwen3.5-35B-A3B-Instruct-4bit}"
RUNS=3

# Version info
VERSION=$(cat "$PROJECT_DIR/VERSION" 2>/dev/null | tr -d '[:space:]' || echo "0.0.0")
GIT_COMMIT=$(git -C "$PROJECT_DIR" rev-parse --short HEAD 2>/dev/null || echo "unknown")

# ============================================
# Parse arguments
# ============================================

MODELS=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        --runs|-r)     RUNS="$2"; shift 2 ;;
        --models|-m)   shift; while [[ $# -gt 0 && ! "$1" =~ ^-- ]]; do MODELS+=("$1"); shift; done ;;
        --all|-a)      FETCH_ALL=true; shift ;;
        --help|-h)
            echo "Usage: ./scripts/benchmark.sh [options]"
            echo "  --runs N          Number of runs per test (default: 3)"
            echo "  --models M1 M2    Specific models to benchmark"
            echo "  --all             Benchmark all models from /v1/models"
            echo "  --help            Show this help"
            exit 0 ;;
        *)             RUNS="$1"; shift ;;  # backward compat: first arg = runs
    esac
done

# ============================================
# Helper functions
# ============================================

check_server() {
    if ! curl -s "${BASE_URL}/health" > /dev/null 2>&1; then
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

get_available_models() {
    curl -s "${BASE_URL}/v1/models" 2>/dev/null | python3 -c "
import sys, json
try:
    data = json.load(sys.stdin)
    for m in data.get('data', []):
        print(m['id'])
except: pass
" 2>/dev/null
}

get_memory_mb() {
    ps aux | grep -E "(vllm|mlx|python)" | grep -v grep | awk '{sum += $6} END {printf "%.0f", sum/1024}' 2>/dev/null || echo "0"
}

run_single_request() {
    local model="$1"
    local prompt="$2"
    local max_tokens="${3:-100}"

    local response
    response=$(curl -s -w "\n%{time_starttransfer}|%{time_total}" \
        "${BASE_URL}/v1/chat/completions" \
        -H "Content-Type: application/json" \
        -d "{
            \"model\": \"${model}\",
            \"messages\": [{\"role\": \"user\", \"content\": \"${prompt}\"}],
            \"max_tokens\": ${max_tokens},
            \"temperature\": 0.1,
            \"stream\": false
        }" 2>/dev/null)

    local body=$(echo "$response" | head -n -1)
    local timings=$(echo "$response" | tail -1)
    local ttfb=$(echo "$timings" | cut -d'|' -f1)
    local total=$(echo "$timings" | cut -d'|' -f2)

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

    local tps="0"
    if [ "$completion_tokens" -gt 0 ] && [ "$(echo "$total > 0" | bc)" -eq 1 ]; then
        tps=$(echo "scale=2; $completion_tokens / $total" | bc)
    fi

    echo "${ttfb}|${total}|${completion_tokens}|${prompt_tokens}|${tps}"
}

# Run a full test suite for a single model
benchmark_model() {
    local model="$1"
    local model_short=$(echo "$model" | sed 's|.*/||')  # strip org prefix

    echo ""
    echo "┌──────────────────────────────────────────────────┐"
    echo "│ 📦 Model: ${model_short}"
    echo "└──────────────────────────────────────────────────┘"

    # Test 1: Short
    echo "  📊 Short Prompt (max 20 tokens)"
    local SHORT_RESULTS=()
    for i in $(seq 1 "$RUNS"); do
        result=$(run_single_request "$model" "Say hello" 20)
        SHORT_RESULTS+=("$result")
        ttfb=$(echo "$result" | cut -d'|' -f1)
        tps=$(echo "$result" | cut -d'|' -f5)
        printf "     Run %d: TTFT=%.3fs  TPS=%.1f\n" "$i" "$ttfb" "$tps"
    done

    # Test 2: Medium
    echo "  📊 Medium Generation (max 200 tokens)"
    local MEDIUM_RESULTS=()
    for i in $(seq 1 "$RUNS"); do
        result=$(run_single_request "$model" "Explain quantum computing in 3 paragraphs" 200)
        MEDIUM_RESULTS+=("$result")
        ttfb=$(echo "$result" | cut -d'|' -f1)
        tokens=$(echo "$result" | cut -d'|' -f3)
        tps=$(echo "$result" | cut -d'|' -f5)
        printf "     Run %d: TTFT=%.3fs  Tokens=%s  TPS=%.1f\n" "$i" "$ttfb" "$tokens" "$tps"
    done

    # Test 3: Long
    echo "  📊 Long Generation (max 500 tokens)"
    local LONG_RESULTS=()
    for i in $(seq 1 "$RUNS"); do
        result=$(run_single_request "$model" "Write a detailed essay about the history of artificial intelligence from 1950 to present" 500)
        LONG_RESULTS+=("$result")
        ttfb=$(echo "$result" | cut -d'|' -f1)
        tokens=$(echo "$result" | cut -d'|' -f3)
        tps=$(echo "$result" | cut -d'|' -f5)
        printf "     Run %d: TTFT=%.3fs  Tokens=%s  TPS=%.1f\n" "$i" "$ttfb" "$tokens" "$tps"
    done

    # Output results as single-line JSON arrays (for python to parse)
    echo "MODEL_DATA|${model}|$(IFS=,; echo "${SHORT_RESULTS[*]}")|$(IFS=,; echo "${MEDIUM_RESULTS[*]}")|$(IFS=,; echo "${LONG_RESULTS[*]}")"
}

# ============================================
# Main
# ============================================

echo ""
echo "╔══════════════════════════════════════════╗"
echo "║   🛡️  Heimdall Benchmark Suite           ║"
echo "╚══════════════════════════════════════════╝"
echo ""
echo "  Version: v${VERSION} (${GIT_COMMIT})"
echo "  Server:  ${BASE_URL}"
echo "  Runs:    ${RUNS} per test"
echo "  Time:    $(date '+%Y-%m-%d %H:%M:%S')"

check_server

# Determine models to benchmark
if [ "${FETCH_ALL:-false}" = true ]; then
    while IFS= read -r m; do
        [ -n "$m" ] && MODELS+=("$m")
    done <<< "$(get_available_models)"
fi

if [ ${#MODELS[@]} -eq 0 ]; then
    MODELS=("$DEFAULT_MODEL")
fi

echo "  Models:  ${#MODELS[@]}"
for m in "${MODELS[@]}"; do
    echo "           - $m"
done

MEMORY_BEFORE=$(get_memory_mb)
echo "  Memory:  ~${MEMORY_BEFORE} MB"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Run benchmarks and capture MODEL_DATA lines
ALL_MODEL_DATA=()
for model in "${MODELS[@]}"; do
    output=$(benchmark_model "$model")
    # Print non-data lines to terminal
    echo "$output" | grep -v "^MODEL_DATA|"
    # Capture data line
    data_line=$(echo "$output" | grep "^MODEL_DATA|" || true)
    if [ -n "$data_line" ]; then
        ALL_MODEL_DATA+=("$data_line")
    fi
done

MEMORY_AFTER=$(get_memory_mb)

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# ============================================
# Generate JSON with all models
# ============================================

# Pass all model data to python
JOINED_DATA=$(printf '%s\n' "${ALL_MODEL_DATA[@]}")

python3 -c "
import json, sys

def parse_results(results_str):
    parsed = []
    for r in results_str.split(','):
        if not r.strip():
            continue
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

# Parse all model data
models_data = {}
raw_lines = '''$JOINED_DATA'''.strip().split('\n')

for line in raw_lines:
    parts = line.split('|', 4)
    if parts[0] != 'MODEL_DATA':
        continue
    model_name = parts[1]
    # The remaining parts contain results separated by pipes within commas
    # Format: MODEL_DATA|model|short1|t|c|p|tps,short2...|med1,...|long1,...
    rest = '|'.join(parts[2:])
    sections = rest.split('|', 2)

    # Re-parse: after model name, we have 3 sections separated by the benchmark_model output
    # Actually the format from bash is: MODEL_DATA|model|short_csv|medium_csv|long_csv
    # where csv items are pipe-separated within commas
    raw = line[len('MODEL_DATA|' + model_name + '|'):]
    csv_parts = raw.split('|')

    # Each result is: ttfb|total|tokens|prompt|tps joined by comma
    # So we need to rejoin them properly
    # The data comes as: t1|tot1|c1|p1|tps1,t2|tot2|c2|p2|tps2|...
    # Split by the outer separator (|) between test types
    # Actually from bash: IFS=, joined, so commas separate runs
    # The three sections (short, medium, long) are separated by |
    # but each run within a section also uses | internally
    # Let me re-parse from the original line more carefully

    # The original bash outputs:
    # MODEL_DATA|model|short_run1_pipe_data,short_run2_pipe_data|med_run1,med_run2|long_run1,long_run2
    # where each run data is: ttfb|total|tokens|prompt|tps
    # So after MODEL_DATA|model| we have three |-separated groups
    pass

# Simpler approach: re-read from the structured line
models_data = {}
for line in raw_lines:
    if not line.startswith('MODEL_DATA|'):
        continue
    # Split: MODEL_DATA | model | short_csv | medium_csv | long_csv
    _, model_name, short_csv, medium_csv, long_csv = line.split('|', 4)

    # But each csv item contains pipes! e.g. '0.1|0.5|20|5|40.0,0.2|0.6|20|5|33.3'
    # So short_csv = '0.1|0.5|20|5|40.0,0.2|0.6|20|5|33.3' - this is fine for parse_results

    short = parse_results(short_csv)
    medium = parse_results(medium_csv)
    long = parse_results(long_csv)

    model_short = model_name.split('/')[-1] if '/' in model_name else model_name

    models_data[model_name] = {
        'name': model_name,
        'short_name': model_short,
        'tests': {
            'short': {'name': 'Short Prompt (TTFT)', 'max_tokens': 20, **calc_stats(short)},
            'medium': {'name': 'Medium Generation', 'max_tokens': 200, **calc_stats(medium)},
            'long': {'name': 'Long Generation', 'max_tokens': 500, **calc_stats(long)}
        }
    }

report = {
    'timestamp': '$(date -u +%Y-%m-%dT%H:%M:%SZ)',
    'version': '${VERSION}',
    'git_commit': '${GIT_COMMIT}',
    'server': '${BASE_URL}',
    'runs_per_test': int('${RUNS}'),
    'memory_mb': {'before': int('${MEMORY_BEFORE}'), 'after': int('${MEMORY_AFTER}')},
    'hardware': {
        'chip': 'Apple M4 Pro',
        'ram': '64GB',
        'bandwidth': '273 GB/s'
    },
    'models': models_data
}

with open('${RESULTS_FILE}', 'w') as f:
    json.dump(report, f, indent=2)

# Print summary
print()
print('📊 Summary')
print('=' * 60)
for mname, mdata in models_data.items():
    t = mdata['tests']
    best_tps = max(t[k]['tps_avg'] for k in t)
    print(f'  {mdata[\"short_name\"]}')
    print(f'    TTFT: {t[\"short\"][\"ttfb_avg\"]:.3f}s  |  Best TPS: {best_tps:.1f} tok/s')
print()
" 2>&1

echo "📄 Results saved: ${RESULTS_FILE}"
echo ""

# Generate HTML report
"${SCRIPT_DIR}/generate_report.sh" "${RESULTS_FILE}"
