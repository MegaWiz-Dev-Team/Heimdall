#!/usr/bin/env bash
# Rotate Google Gemini API key across Heimdall .env and K8s asgard-secrets.
#
# Workflow:
#   1. Prompt for new AIzaSy key (no echo to terminal)
#   2. Validate format (AIzaSy + 33 chars)
#   3. Test new key against Google before touching live config
#   4. Backup Heimdall .env + K8s secret
#   5. Update both, sync atomically
#   6. Reload heimdall-gateway (launchd) and bifrost (k8s)
#   7. Round-trip verify via Heimdall → Gemini
#
# Re-runnable. Aborts on any error. Old key never logged.

set -euo pipefail

# ── Config ──────────────────────────────────────────────────────────────
HEIMDALL_ENV="/Users/mimir/Developer/Heimdall/.env"
HEIMDALL_PLIST="/Users/mimir/Library/LaunchAgents/com.asgard.heimdall-gateway.plist"
HEIMDALL_LAUNCHD_LABEL="com.asgard.heimdall-gateway"
K8S_NS="asgard"
K8S_SECRET="asgard-secrets"
K8S_KEY="GEMINI_API_KEY"
K8S_CONSUMER_DEPLOY="bifrost"

TIMESTAMP=$(date +%Y%m%d-%H%M%S)
BACKUP_DIR="/Users/mimir/Developer/Asgard/logs/incidents/2026-05-23-gitguardian-google-api-key/rotation-${TIMESTAMP}"

# ── Cleanup ─────────────────────────────────────────────────────────────
cleanup() {
  unset NEW_KEY KEY B64 HEIMDALL_AUTH
}
trap cleanup EXIT INT TERM

# ── Pre-flight ──────────────────────────────────────────────────────────
echo "=== Rotate GEMINI_API_KEY ==="
echo "  Heimdall .env  : $HEIMDALL_ENV"
echo "  K8s            : secret/$K8S_SECRET key/$K8S_KEY in ns/$K8S_NS"
echo "  Consumer       : deploy/$K8S_CONSUMER_DEPLOY (k8s) + $HEIMDALL_LAUNCHD_LABEL (launchd)"
echo "  Backup         : $BACKUP_DIR"
echo

for f in "$HEIMDALL_ENV" "$HEIMDALL_PLIST"; do
  [[ -f "$f" ]] || { echo "ERROR: $f not found"; exit 1; }
done
command -v kubectl >/dev/null || { echo "ERROR: kubectl not on PATH"; exit 1; }
command -v plutil  >/dev/null || { echo "ERROR: plutil not on PATH";  exit 1; }
command -v python3 >/dev/null || { echo "ERROR: python3 not on PATH"; exit 1; }
kubectl -n "$K8S_NS" get secret "$K8S_SECRET" >/dev/null \
  || { echo "ERROR: secret/$K8S_SECRET not accessible in ns/$K8S_NS"; exit 1; }

# ── Read key (no echo, works in bash and zsh via /bin/bash shebang) ────
read -s -p "Paste new AIzaSy key (won't echo): " NEW_KEY
echo

# ── Validate format ─────────────────────────────────────────────────────
if [[ ! "$NEW_KEY" =~ ^AIzaSy[A-Za-z0-9_-]{33}$ ]]; then
  echo "ERROR: Key doesn't match Google API Key format (AIzaSy + 33 alnum/_/-)"
  exit 1
fi
echo "  ✓ Format OK"

# ── Live-test against Google before rotating anything ──────────────────
echo "→ Testing new key against generativelanguage.googleapis.com..."
HTTP=$(curl -s -o /dev/null -w "%{http_code}" \
  "https://generativelanguage.googleapis.com/v1beta/models?key=$NEW_KEY")
if [[ "$HTTP" != "200" ]]; then
  echo "ERROR: Google rejected the key (HTTP $HTTP). Aborting before rotation."
  echo "  Common causes: key not enabled for Generative Language API,"
  echo "  IP/referrer restriction blocking this host, or typo."
  exit 1
fi
echo "  ✓ Google accepted the key (HTTP 200)"

# ── Backup ──────────────────────────────────────────────────────────────
mkdir -p "$BACKUP_DIR"
cp "$HEIMDALL_ENV" "$BACKUP_DIR/heimdall.env.bak"
kubectl -n "$K8S_NS" get secret "$K8S_SECRET" -o yaml > "$BACKUP_DIR/k8s-secret.bak.yaml"
chmod 600 "$BACKUP_DIR"/*
echo "→ Backed up to $BACKUP_DIR"

# ── Update Heimdall .env (in-place, no echo of value) ──────────────────
KEY="$NEW_KEY" python3 - <<'PY'
import os, pathlib
p = pathlib.Path(os.environ.get("HEIMDALL_ENV", "/Users/mimir/Developer/Heimdall/.env"))
lines = p.read_text().splitlines()
found = False
new = []
for l in lines:
    if l.startswith("GEMINI_API_KEY="):
        new.append("GEMINI_API_KEY=" + os.environ["KEY"])
        found = True
    else:
        new.append(l)
if not found:
    new.append("GEMINI_API_KEY=" + os.environ["KEY"])
p.write_text("\n".join(new) + "\n")
print(f"  ✓ Heimdall .env updated ({'replaced' if found else 'appended'})")
PY

# ── Update K8s secret ──────────────────────────────────────────────────
B64=$(printf %s "$NEW_KEY" | base64)
kubectl -n "$K8S_NS" patch secret "$K8S_SECRET" --type=json \
  -p="[{\"op\":\"replace\",\"path\":\"/data/$K8S_KEY\",\"value\":\"$B64\"}]" >/dev/null
unset B64
echo "  ✓ K8s secret/$K8S_SECRET[$K8S_KEY] updated"

# ── Reload services ────────────────────────────────────────────────────
echo "→ Reloading services..."
launchctl kickstart -k "gui/$(id -u)/$HEIMDALL_LAUNCHD_LABEL"
echo "  ✓ Heimdall gateway kicked"

kubectl -n "$K8S_NS" rollout restart deploy/"$K8S_CONSUMER_DEPLOY" >/dev/null
kubectl -n "$K8S_NS" rollout status deploy/"$K8S_CONSUMER_DEPLOY" --timeout=120s
echo "  ✓ $K8S_CONSUMER_DEPLOY restarted"

# ── Verify Heimdall → Gemini round-trip ────────────────────────────────
echo "→ Verifying via Heimdall round-trip..."
sleep 3
HEIMDALL_AUTH=$(plutil -extract EnvironmentVariables.API_KEYS raw "$HEIMDALL_PLIST" | cut -d',' -f1)
if [[ -z "$HEIMDALL_AUTH" ]]; then
  echo "  ⚠ Could not extract Heimdall API key from plist; skipping round-trip"
else
  RESP=$(curl -s --max-time 30 -X POST http://localhost:8080/v1/chat/completions \
    -H "Authorization: Bearer $HEIMDALL_AUTH" \
    -H "Content-Type: application/json" \
    -d '{"model":"gemini/gemini-2.5-flash-lite","messages":[{"role":"user","content":"reply with the single word OK"}],"max_tokens":10}')
  unset HEIMDALL_AUTH
  if echo "$RESP" | grep -qE '"content"\s*:\s*"[^"]*OK'; then
    echo "  ✓ Heimdall → Gemini round-trip OK"
  else
    echo "  ⚠ Round-trip response did not contain 'OK'. Raw response (first 300 chars):"
    echo "$RESP" | head -c 300
    echo
    echo "  Manually inspect Heimdall logs: tail -f /var/log/asgard/heimdall-gateway.log"
    exit 1
  fi
fi

echo
echo "=== DONE ==="
echo "  Backup       : $BACKUP_DIR"
echo "  Next steps   : if old key was real, revoke it in GCP Console now."
echo "  Verify spend : https://console.cloud.google.com/billing"