#!/bin/bash
# Heimdall Gateway Launchd Wrapper
# Fetches secrets from Fafnir (Vault) before launching the Rust binary

export VAULT_ADDR="http://127.0.0.1:30820"
export VAULT_TOKEN="hvs.QLBolhzojhTzetrEuzqZQYEx"

echo "Fetching GEMINI_API_KEY from Fáfnir..."
# Fetch the secret
SECRETS=$(curl -sf -H "X-Vault-Token: ${VAULT_TOKEN}" "${VAULT_ADDR}/v1/secret/data/mimir" || echo "{}")

# Extract using python
export GEMINI_API_KEY=$(echo "$SECRETS" | python3 -c "import sys,json; print(json.load(sys.stdin).get('data',{}).get('data',{}).get('gemini_api_key', ''))")

if [ -z "$GEMINI_API_KEY" ]; then
    echo "Warning: GEMINI_API_KEY not found in Vault!"
fi

echo "Starting Heimdall Gateway..."
exec /Users/mimir/Developer/Heimdall/gateway/target/release/heimdall-gateway
