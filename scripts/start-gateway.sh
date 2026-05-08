#!/bin/bash
# Heimdall Gateway Launchd Wrapper
# Optionally fetches GEMINI_API_KEY from a Vault server before launching.
# Set VAULT_ADDR and VAULT_TOKEN in the environment to enable.

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if [ -n "${VAULT_ADDR}" ] && [ -n "${VAULT_TOKEN}" ]; then
    echo "Fetching GEMINI_API_KEY from Vault (${VAULT_ADDR})..."
    SECRETS=$(curl -sf -H "X-Vault-Token: ${VAULT_TOKEN}" "${VAULT_ADDR}/v1/secret/data/mimir" || echo "{}")
    export GEMINI_API_KEY=$(echo "$SECRETS" | python3 -c "import sys,json; print(json.load(sys.stdin).get('data',{}).get('data',{}).get('gemini_api_key', ''))")
    if [ -z "$GEMINI_API_KEY" ]; then
        echo "Warning: GEMINI_API_KEY not found in Vault!"
    fi
fi

echo "Starting Heimdall Gateway..."
exec "${PROJECT_DIR}/gateway/target/release/heimdall-gateway"
