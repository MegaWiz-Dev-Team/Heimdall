#!/usr/bin/env bash
# Launch llama-server for gemma-4-26b GGUF with N KV slots (Tier-2 production engine).
# Runs on a TEST port (8087) so prod mlx on 8081 is untouched during validation.
# One model in RAM (~15GB) + N slots' KV — same footprint as the current mlx, NOT additive.
set -euo pipefail
GGUF="${GGUF:?set GGUF=/path/to/gemma-4-26B-A4B-it-UD-Q4_K_M.gguf}"
NP="${NP:-4}"          # parallel KV slots (= hot tenants kept warm)
CTX="${CTX:-16384}"    # total context, split across slots (4k each at NP=4)
PORT="${PORT:-8087}"
exec /opt/homebrew/bin/llama-server \
  -m "$GGUF" \
  --host 127.0.0.1 --port "$PORT" \
  -np "$NP" -c "$CTX" \
  -sps 0.5 \
  --slots \
  --jinja \
  -ngl 999
