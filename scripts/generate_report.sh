#!/usr/bin/env bash
set -euo pipefail

# ============================================
# Heimdall — Generate Visual HTML Report
# Supports single & multi-model comparison
# Usage: ./generate_report.sh <results.json>
# ============================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
REPORT_DIR="$PROJECT_DIR/reports"

INPUT_FILE="${1:-$(ls -t "$REPORT_DIR"/benchmark_*.json 2>/dev/null | head -1)}"

if [ -z "$INPUT_FILE" ] || [ ! -f "$INPUT_FILE" ]; then
    echo "❌ No benchmark results found. Run ./scripts/benchmark.sh first."
    exit 1
fi

BASENAME=$(basename "$INPUT_FILE" .json)
HTML_FILE="$REPORT_DIR/${BASENAME}.html"

python3 "$SCRIPT_DIR/report_template.py" "$INPUT_FILE" "$HTML_FILE"

echo ""
echo "✅ HTML Report: ${HTML_FILE}"
echo "   Open with: open \"${HTML_FILE}\""
