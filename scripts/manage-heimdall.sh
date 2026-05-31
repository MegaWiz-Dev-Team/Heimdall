#!/usr/bin/env bash
# 🛡️ Heimdall — Service Management
# Simplified launchctl wrapper for Heimdall services
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

ok()    { echo -e "${GREEN}✅ $1${NC}"; }
warn()  { echo -e "${YELLOW}⚠️  $1${NC}"; }
fail()  { echo -e "${RED}❌ $1${NC}"; exit 1; }

LAUNCHD_DIR="$HOME/Library/LaunchAgents"
SERVICES=(
    "com.asgard.heimdall-gateway"
    "com.asgard.heimdall-mlx"
    "com.asgard.heimdall-vlm-q4"
    "com.asgard.heimdall-vlm-q8"
    "com.asgard.heimdall-logshipper"
)

SERVICE_NAMES=(
    "gateway"
    "mlx"
    "vlm-q4"
    "vlm-q8"
    "logshipper"
)

ACTION="${1:-status}"
TARGET="${2:-all}"

# ─── Helper functions ───────────────────────────────────────────
service_start() {
    local svc=$1
    if launchctl start "$svc" 2>/dev/null; then
        ok "Started $svc"
        return 0
    else
        warn "$svc already running or failed to start"
        return 1
    fi
}

service_stop() {
    local svc=$1
    if launchctl stop "$svc" 2>/dev/null; then
        ok "Stopped $svc"
        return 0
    else
        warn "$svc not running or failed to stop"
        return 1
    fi
}

service_status() {
    local svc=$1
    if launchctl list "$svc" >/dev/null 2>&1; then
        echo -e "  ${GREEN}●${NC} $svc (loaded)"
        return 0
    else
        echo -e "  ${RED}○${NC} $svc (not loaded)"
        return 1
    fi
}

# ─── Main logic ─────────────────────────────────────────────────
[[ "$(uname)" == "Darwin" ]] || fail "macOS only"
[ -d "$LAUNCHD_DIR" ] || fail "launchd not found at $LAUNCHD_DIR"

case "$ACTION" in
    start)
        if [ "$TARGET" = "all" ]; then
            echo "Starting all Heimdall services..."
            for svc in "${SERVICES[@]}"; do
                service_start "$svc" || true
            done
        else
            svc="com.asgard.heimdall-$TARGET"
            service_start "$svc"
        fi
        ;;
    stop)
        if [ "$TARGET" = "all" ]; then
            echo "Stopping all Heimdall services..."
            for svc in "${SERVICES[@]}"; do
                service_stop "$svc" || true
            done
        else
            svc="com.asgard.heimdall-$TARGET"
            service_stop "$svc"
        fi
        ;;
    restart)
        if [ "$TARGET" = "all" ]; then
            echo "Restarting all Heimdall services..."
            for svc in "${SERVICES[@]}"; do
                service_stop "$svc" || true
                sleep 1
                service_start "$svc" || true
            done
        else
            svc="com.asgard.heimdall-$TARGET"
            service_stop "$svc" || true
            sleep 1
            service_start "$svc"
        fi
        ;;
    status)
        echo "Heimdall Service Status:"
        echo ""
        if [ "$TARGET" = "all" ]; then
            for svc in "${SERVICES[@]}"; do
                service_status "$svc" || true
            done
        else
            svc="com.asgard.heimdall-$TARGET"
            service_status "$svc"
        fi
        echo ""
        ;;
    logs)
        PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
        if [ "$TARGET" = "gateway" ]; then
            echo "Tailing gateway logs..."
            tail -f "$PROJECT_DIR/logs/gateway-stdout.log"
        elif [ "$TARGET" = "mlx" ]; then
            echo "Tailing MLX logs..."
            tail -f "$PROJECT_DIR/logs/mlx-stdout.log"
        elif [ "$TARGET" = "all" ]; then
            echo "Log files available at: $PROJECT_DIR/logs/"
            ls -lh "$PROJECT_DIR/logs/"
        else
            tail -f "$PROJECT_DIR/logs/${TARGET}-stdout.log"
        fi
        ;;
    *)
        cat << 'EOF'
🛡️  Heimdall Service Manager

Usage: ./scripts/manage-heimdall.sh <action> [service]

Actions:
  start <service>    — Start a service (gateway|mlx|vlm-q4|vlm-q8|logshipper|all)
  stop <service>     — Stop a service
  restart <service>  — Restart a service
  status <service>   — Check service status
  logs <service>     — Tail service logs

Examples:
  ./scripts/manage-heimdall.sh start all
  ./scripts/manage-heimdall.sh stop gateway
  ./scripts/manage-heimdall.sh status
  ./scripts/manage-heimdall.sh logs mlx
  ./scripts/manage-heimdall.sh restart gateway

Manage directly with launchctl:
  launchctl start  com.asgard.heimdall-gateway
  launchctl stop   com.asgard.heimdall-gateway
  launchctl list   com.asgard.heimdall-gateway
EOF
        ;;
esac
