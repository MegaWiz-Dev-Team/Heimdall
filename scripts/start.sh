#!/usr/bin/env bash
set -euo pipefail

echo ""
echo "❌ FATAL: start.sh is DEPRECATED"
echo "==================================="
echo "Heimdall has been upgraded to a production-grade macOS daemon."
echo "Running this script manually causes AddrInUse conflicts and overlapping MLX instances."
echo ""
echo "To install Heimdall as a system service, run:"
echo "  ./scripts/install_daemons.sh"
echo ""
echo "To manage Heimdall Gateway, use launchctl:"
echo "  launchctl stop com.asgard.heimdall-gateway"
echo "  launchctl start com.asgard.heimdall-gateway"
echo ""
echo "To start a heavy text backend (Port 8081), please use the native engine:"
echo "  e.g., Mimir/scripts/run_flash_moe.sh"
echo ""
exit 1
