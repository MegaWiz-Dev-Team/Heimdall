#!/usr/bin/env bash
# 🛡️ Heimdall — Quick Setup & Deploy
# One-command setup for local development + production launchd
set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SCRIPT_DIR="$PROJECT_DIR/scripts"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

info()  { echo -e "${BLUE}ℹ️  $1${NC}"; }
ok()    { echo -e "${GREEN}✅ $1${NC}"; }
warn()  { echo -e "${YELLOW}⚠️  $1${NC}"; }
fail()  { echo -e "${RED}❌ $1${NC}"; exit 1; }
step()  { echo -e "${BLUE}── $1 ──${NC}"; }

echo ""
echo "╔════════════════════════════════════════╗"
echo "║   🛡️  Heimdall Quick Setup              ║"
echo "║   LLM Gateway for Asgard AI Platform   ║"
echo "╚════════════════════════════════════════╝"
echo ""

# ─── Preflight checks ───────────────────────────────────────────
step "Preflight checks"
[[ "$(uname)" == "Darwin" ]] || fail "macOS only (requires launchd)"
command -v cargo >/dev/null 2>&1 || fail "Rust not installed (brew install rust)"
command -v python3 >/dev/null 2>&1 || fail "Python 3 not installed"
ok "Preflight OK"

# ─── Environment setup ──────────────────────────────────────────
step "Environment setup"
if [ ! -f "$PROJECT_DIR/.env" ]; then
    info "Creating .env from template..."
    cp "$PROJECT_DIR/.env.example" "$PROJECT_DIR/.env"
    warn ".env created — EDIT NOW with your API keys before proceeding"
    warn "Required keys: API_KEYS, GEMINI_API_KEY, OPENROUTER_API_KEY (optional)"
    echo ""
    read -p "Press ENTER after editing .env, or Ctrl+C to abort: "
fi
ok ".env found"

# ─── Python venv ───────────────────────────────────────────────
step "Python virtual environment"
if [ ! -d "$PROJECT_DIR/.venv" ]; then
    info "Creating venv..."
    python3 -m venv "$PROJECT_DIR/.venv"
fi
# shellcheck disable=SC1091
source "$PROJECT_DIR/.venv/bin/activate"
ok "venv activated"

# ─── Build Rust components ─────────────────────────────────────
step "Building Heimdall Rust components"
cd "$PROJECT_DIR"
info "Building gateway (this may take 2-3 minutes)..."
cargo build --release -p heimdall-gateway 2>&1 | tail -5
ok "Gateway built"

info "Building embedding engine..."
cargo build --release -p heimdall-embedding 2>&1 | tail -3
ok "Embedding built"

# ─── Install MLX dependencies ──────────────────────────────────
step "Installing MLX Python dependencies"
if ! python3 -c "import mlx" 2>/dev/null; then
    info "Installing mlx-lm..."
    pip install -q mlx-lm mlx-whisper
fi
ok "MLX dependencies installed"

# ─── Install daemons ───────────────────────────────────────────
step "Installing launchd services"
bash "$SCRIPT_DIR/install_daemons.sh"

echo ""
echo "╔════════════════════════════════════════╗"
echo "║   ✅ Setup Complete!                   ║"
echo "╚════════════════════════════════════════╝"
echo ""

# ─── Usage info ─────────────────────────────────────────────────
step "Manage Heimdall services"
cat << 'EOF'
Start/stop services:
  ./scripts/manage-heimdall.sh start gateway
  ./scripts/manage-heimdall.sh stop gateway
  ./scripts/manage-heimdall.sh status

Or use launchctl directly:
  launchctl start  com.asgard.heimdall-gateway
  launchctl stop   com.asgard.heimdall-gateway
  launchctl start  com.asgard.heimdall-mlx
  launchctl stop   com.asgard.heimdall-mlx

Test gateway is running:
  curl http://localhost:8080/health

Logs:
  tail -f logs/gateway-stdout.log
  tail -f logs/gateway-stderr.log

Download models:
  ./heimdall pull mlx-community/Qwen3.5-9B-MLX-4bit
EOF
echo ""
