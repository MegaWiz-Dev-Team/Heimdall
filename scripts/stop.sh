#!/usr/bin/env bash
set -euo pipefail

echo ""
echo "❌ FATAL: stop.sh is DEPRECATED"
echo "==================================="
echo "Heimdall has been upgraded to a production-grade macOS daemon."
echo "Running this script manually breaks the launchd KeepAlive cycle."
echo ""
echo "To manage Heimdall Gateway, use launchctl:"
echo "  launchctl stop com.asgard.heimdall-gateway"
echo "  launchctl start com.asgard.heimdall-gateway"
echo ""
echo "Or using the install script interface:"
echo "  ./scripts/install_daemons.sh"
echo ""
exit 1
