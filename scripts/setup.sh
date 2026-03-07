#!/usr/bin/env bash
set -euo pipefail

# ============================================
# LLM Server — Setup Script
# Installs all dependencies and builds gateway
# ============================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

echo "🛡️ Heimdall Setup"
echo "===================="
echo ""

# --- Check Rust ---
echo "📦 Checking Rust..."
if ! command -v cargo &>/dev/null; then
    echo "  ❌ Rust not found. Installing..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
else
    echo "  ✅ Rust $(rustc --version | cut -d' ' -f2)"
fi

# --- Check Python ---
echo "📦 Checking Python..."
if ! command -v python3 &>/dev/null; then
    echo "  ❌ Python3 not found. Please install Python 3.10+"
    exit 1
else
    echo "  ✅ Python $(python3 --version | cut -d' ' -f2)"
fi

# --- Setup Python venv ---
echo ""
echo "📦 Setting up Python environment..."
VENV_DIR="$PROJECT_DIR/.venv"
if [ ! -d "$VENV_DIR" ]; then
    python3 -m venv "$VENV_DIR"
    echo "  ✅ Created virtualenv at $VENV_DIR"
else
    echo "  ✅ Virtualenv exists at $VENV_DIR"
fi

# Use explicit venv pip/python paths (more reliable than source activate)
VPIP="$VENV_DIR/bin/pip"
VPYTHON="$VENV_DIR/bin/python3"

"$VPIP" install --quiet --upgrade pip

# --- Install MLX packages ---
echo ""
echo "📦 Installing MLX packages..."
"$VPIP" install --quiet mlx-lm
echo "  ✅ mlx-lm installed"

"$VPIP" install --quiet mlx-vlm
echo "  ✅ mlx-vlm installed"

"$VPIP" install --quiet mlx-embedding-models
echo "  ✅ mlx-embedding-models installed"

# --- Build Gateway ---
echo ""
echo "🦀 Building Rust Gateway..."
if [ -d "$PROJECT_DIR/gateway" ]; then
    (cd "$PROJECT_DIR/gateway" && cargo build --release 2>&1 | tail -1)
    echo "  ✅ Gateway built"
else
    echo "  ⚠️  Gateway directory not found — skipping build"
fi

# --- Create .env if not exists ---
if [ ! -f "$PROJECT_DIR/.env" ]; then
    if [ -f "$PROJECT_DIR/.env.example" ]; then
        cp "$PROJECT_DIR/.env.example" "$PROJECT_DIR/.env"
        echo ""
        echo "📄 Created .env from .env.example — edit as needed"
    fi
fi

# --- Create directories ---
mkdir -p "$PROJECT_DIR/logs" "$PROJECT_DIR/.pids"

echo ""
echo "✅ Setup complete!"
echo ""
echo "Next steps:"
echo "  1. Edit .env if needed"
echo "  2. Start server:  ./scripts/start.sh"
echo "  3. Check health:  ./scripts/health_check.sh"
