#!/usr/bin/env bash
set -euo pipefail

# ============================================
# Heimdall — Daemon Installer (macOS)
# Registers Heimdall components to launchd
#
# Services managed:
#   1. heimdall-gateway   — Rust API Gateway (:8080)
#   2. heimdall-mlx       — MLX LLM Backend  (:8081)
#   3. heimdall-logshipper — Host Log Shipper → Tyr/Wazuh
# ============================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
LAUNCHD_DIR="$HOME/Library/LaunchAgents"

echo "🛡️ Heimdall Production Setup"
echo "=============================="
echo ""

if [[ "$(uname)" != "Darwin" ]]; then
    echo "❌ This script is intended for macOS launchd only."
    exit 1
fi

mkdir -p "$LAUNCHD_DIR"
mkdir -p "$PROJECT_DIR/logs"

# ── Clean up legacy daemons ──────────────────────────────────
echo "🧹 Removing legacy daemons..."
for legacy in com.asgard.heimdall-llama com.asgard.heimdall-embedding com.asgard.colima; do
    if [ -f "$LAUNCHD_DIR/$legacy.plist" ]; then
        launchctl unload "$LAUNCHD_DIR/$legacy.plist" 2>/dev/null || true
        rm "$LAUNCHD_DIR/$legacy.plist"
        echo "   Removed $legacy"
    fi
done

# ── Load secrets from .env (gateway needs API_KEYS + provider keys) ──
# Sourcing .env lets us substitute {{API_KEYS}}, {{GEMINI_API_KEY}}, etc.
# in the gateway plist. The .env stays gitignored; the rendered plist
# lives only in ~/Library/LaunchAgents (per-user, not world-readable).
if [ -f "$PROJECT_DIR/.env" ]; then
    # shellcheck disable=SC1091
    set -a
    source "$PROJECT_DIR/.env"
    set +a
fi

# ── Install Gateway Service ──────────────────────────────────
echo "📦 Installing Heimdall Gateway (:8080)..."
sed -e "s|{{PROJECT_DIR}}|$PROJECT_DIR|g" \
    -e "s|{{API_KEYS}}|${API_KEYS:-}|g" \
    -e "s|{{MARIADB_URL}}|${MARIADB_URL:-}|g" \
    -e "s|{{SKUGGI_LOCAL_ONLY_TENANTS}}|${SKUGGI_LOCAL_ONLY_TENANTS:-}|g" \
    -e "s|{{PYTHAINLP_URL}}|${PYTHAINLP_URL:-}|g" \
    -e "s|{{GEMINI_API_KEY}}|${GEMINI_API_KEY:-}|g" \
    -e "s|{{OPENROUTER_API_KEY}}|${OPENROUTER_API_KEY:-}|g" \
    -e "s|{{OPENAI_API_KEY}}|${OPENAI_API_KEY:-}|g" \
    "$PROJECT_DIR/deploy/launchd/com.asgard.heimdall-gateway.plist" \
    > "$LAUNCHD_DIR/com.asgard.heimdall-gateway.plist"
chmod 600 "$LAUNCHD_DIR/com.asgard.heimdall-gateway.plist"

# ── Install MLX Backend Service ──────────────────────────────
echo "📦 Installing MLX LLM Backend (:8081)..."
sed -e "s|{{PROJECT_DIR}}|$PROJECT_DIR|g" \
    "$PROJECT_DIR/deploy/launchd/com.asgard.heimdall-mlx.plist" \
    > "$LAUNCHD_DIR/com.asgard.heimdall-mlx.plist"

# ── Install MLX-VLM q4 Service (Sprint 51) ───────────────────
echo "📦 Installing MLX-VLM q4 Backend (:8082)..."
sed -e "s|{{PROJECT_DIR}}|$PROJECT_DIR|g" \
    -e "s|{{HOME}}|$HOME|g" \
    "$PROJECT_DIR/deploy/launchd/com.asgard.heimdall-vlm-q4.plist" \
    > "$LAUNCHD_DIR/com.asgard.heimdall-vlm-q4.plist"

# ── Install MLX-VLM q8 Service (Sprint 51) ───────────────────
echo "📦 Installing MLX-VLM q8 Backend (:8083)..."
sed -e "s|{{PROJECT_DIR}}|$PROJECT_DIR|g" \
    -e "s|{{HOME}}|$HOME|g" \
    "$PROJECT_DIR/deploy/launchd/com.asgard.heimdall-vlm-q8.plist" \
    > "$LAUNCHD_DIR/com.asgard.heimdall-vlm-q8.plist"

# ── Install Log Shipper Service ──────────────────────────────
echo "📦 Installing Heimdall Log Shipper (→ Tyr/Wazuh)..."
# No placeholders in this one, just direct copy if exists, or assume it's created manually
# (Since we wrote it directly to Library, let's copy it back to deploy/ for backup)
cp "$LAUNCHD_DIR/com.asgard.heimdall-logshipper.plist" "$PROJECT_DIR/deploy/launchd/" 2>/dev/null || true
cp "$PROJECT_DIR/deploy/launchd/com.asgard.heimdall-logshipper.plist" "$LAUNCHD_DIR/com.asgard.heimdall-logshipper.plist"

# ── Reload services ─────────────────────────────────────────
echo "🔄 Reloading launchd services..."
launchctl unload "$LAUNCHD_DIR/com.asgard.heimdall-gateway.plist" 2>/dev/null || true
launchctl unload "$LAUNCHD_DIR/com.asgard.heimdall-mlx.plist" 2>/dev/null || true
launchctl unload "$LAUNCHD_DIR/com.asgard.heimdall-vlm-q4.plist" 2>/dev/null || true
launchctl unload "$LAUNCHD_DIR/com.asgard.heimdall-vlm-q8.plist" 2>/dev/null || true
launchctl unload "$LAUNCHD_DIR/com.asgard.heimdall-logshipper.plist" 2>/dev/null || true

launchctl load "$LAUNCHD_DIR/com.asgard.heimdall-gateway.plist"
launchctl load "$LAUNCHD_DIR/com.asgard.heimdall-mlx.plist"
launchctl load "$LAUNCHD_DIR/com.asgard.heimdall-vlm-q4.plist"
launchctl load "$LAUNCHD_DIR/com.asgard.heimdall-vlm-q8.plist"
launchctl load "$LAUNCHD_DIR/com.asgard.heimdall-logshipper.plist"

echo ""
echo "✅ Heimdall Daemons Installed Successfully!"
echo ""
echo "Services:"
echo "  🌐 Gateway    :8080  — Rust API Gateway + built-in ONNX Embedding/Rerank"
echo "  🧠 MLX Backend :8081  — MLX Python LLM server (chat/completions)"
echo "  👁️ MLX-VLM q4 :8082  — Typhoon-OCR 3b q4 (throughput-default OCR)"
echo "  👁️ MLX-VLM q8 :8083  — Typhoon-OCR 3b q8 (escalation tier)"
echo "  📡 Log Shipper        — Ships host logs to Tyr (Wazuh:30920)"
echo ""
echo "Manage services using launchctl:"
echo "  launchctl stop  com.asgard.heimdall-gateway"
echo "  launchctl start com.asgard.heimdall-gateway"
echo "  launchctl stop  com.asgard.heimdall-mlx"
echo "  launchctl start com.asgard.heimdall-mlx"
echo "  launchctl stop  com.asgard.heimdall-vlm-q4"
echo "  launchctl start com.asgard.heimdall-vlm-q4"
echo "  launchctl stop  com.asgard.heimdall-vlm-q8"
echo "  launchctl start com.asgard.heimdall-vlm-q8"
echo ""
echo "Logs: $PROJECT_DIR/logs/"
echo ""
