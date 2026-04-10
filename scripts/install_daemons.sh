#!/usr/bin/env bash
set -euo pipefail

# ============================================
# LLM Server — Daemon Installer (macOS)
# Registers Heimdall components to launchd
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

HOMEBREW_PREFIX=$(brew --prefix 2>/dev/null || echo "/opt/homebrew")

mkdir -p "$LAUNCHD_DIR"
mkdir -p "$PROJECT_DIR/logs"

# Install Gateway Service
echo "📦 Installing Gateway Service..."
sed -e "s|{{PROJECT_DIR}}|$PROJECT_DIR|g" \
    "$PROJECT_DIR/deploy/launchd/com.asgard.heimdall-gateway.plist" \
    > "$LAUNCHD_DIR/com.asgard.heimdall-gateway.plist"

# Install Embedding Services
echo "📦 Installing Embedding Services..."
sed -e "s|{{PROJECT_DIR}}|$PROJECT_DIR|g" -e "s|{{HOMEBREW_PREFIX}}|$HOMEBREW_PREFIX|g" \
    "$PROJECT_DIR/deploy/launchd/com.asgard.heimdall-llama.plist" \
    > "$LAUNCHD_DIR/com.asgard.heimdall-llama.plist"
    
sed -e "s|{{PROJECT_DIR}}|$PROJECT_DIR|g" \
    "$PROJECT_DIR/deploy/launchd/com.asgard.heimdall-embedding.plist" \
    > "$LAUNCHD_DIR/com.asgard.heimdall-embedding.plist"

# Unload existing
echo "🔄 Reloading launchd services..."
launchctl unload "$LAUNCHD_DIR/com.asgard.heimdall-gateway.plist" 2>/dev/null || true
launchctl unload "$LAUNCHD_DIR/com.asgard.heimdall-llama.plist" 2>/dev/null || true
launchctl unload "$LAUNCHD_DIR/com.asgard.heimdall-embedding.plist" 2>/dev/null || true

# Load new
launchctl load "$LAUNCHD_DIR/com.asgard.heimdall-gateway.plist"
launchctl load "$LAUNCHD_DIR/com.asgard.heimdall-llama.plist"
launchctl load "$LAUNCHD_DIR/com.asgard.heimdall-embedding.plist"

echo ""
echo "✅ Heimdall Daemons Installed Successfully!"
echo ""
echo "Manage services using launchctl:"
echo "  launchctl stop com.asgard.heimdall-gateway"
echo "  launchctl start com.asgard.heimdall-gateway"
echo ""
echo "⚠️  NOTE: By default, the MLX Python Text Backend (Port 8081) is NOT managed"
echo "   by launchd to allow for decoupling. If using heavy parameter models like"
echo "   Qwen3.5-397B, you must start the Flash-MoE server manually on 8081."
