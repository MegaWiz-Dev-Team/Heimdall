#!/usr/bin/env bash
# Sprint 51c Path B+ — full mlx_lm patch for typhoon-si-med-thinking-4b
# (and any future model that emits <tool_call> tokens trained-style).
#
# Two files patched in the Heimdall venv:
#   1. mlx_lm/tool_parsers/json_tools.py — try/except, return [] on
#      JSONDecodeError so an empty/malformed tool body never crashes
#      the request (defensive; useful for any model).
#   2. mlx_lm/server.py — _make_state_machine grows a `has_tools` flag
#      that gates the <tool_call> sequence transitions on whether the
#      request actually passed `tools`. Without this, Typhoon's
#      trailing <tool_call> token routes the entire response into the
#      tool channel and message.content comes back empty.
#
# Re-apply this script after every `pip install --upgrade mlx-lm`.
# Drop entirely once upstream lands either:
#   - a per-request --no-tools flag, OR
#   - per-request gating of the tool-call sequence transitions.
#
# Usage:
#   bash scripts/patch_mlx_lm_json_tools.sh           # apply or no-op
#   bash scripts/patch_mlx_lm_json_tools.sh --check   # verify present
#   bash scripts/patch_mlx_lm_json_tools.sh --revert  # restore stock
#
# Verified live (Sprint 51c, 2026-05-08): mlx_lm.server :8083 with
# typhoon-si-med-thinking-4b returns full message.content (947 chars,
# finish_reason="stop") on a STEMI clinical question that previously
# crashed the server (HTTP 500) or returned empty body.

set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VENV_DIR="${HEIMDALL_VENV:-${PROJECT_DIR}/.venv}"
VENV_PY="${VENV_DIR}/bin/python"

# Resolve the mlx_lm package directory under the active venv (any python version).
VENV_ROOT="$("${VENV_PY}" -c 'import mlx_lm, os; print(os.path.dirname(mlx_lm.__file__))')"
JSON_TOOLS=$VENV_ROOT/tool_parsers/json_tools.py
SERVER_FILE=$VENV_ROOT/server.py
JSON_MARKER="Sprint 51c Path B local patch"
SERVER_MARKER="Sprint 51c Path B+"

apply_json_tools() {
    cat > "$JSON_TOOLS" <<'PY'
# Copyright © 2025 Apple Inc.
#
# Sprint 51c Path B local patch (Asgard Heimdall venv only).
# Re-apply via Heimdall/scripts/patch_mlx_lm_json_tools.sh after pip
# upgrades. Drop when upstream fixes per-request tool gating.

import json

tool_call_start = "<tool_call>"

tool_call_end = "</tool_call>"


def parse_tool_call(text, tools=None):
    try:
        return json.loads(text.strip())
    except json.JSONDecodeError:
        # Return empty list (not None) — the caller in mlx_lm/server.py
        # ToolCallFormatter wraps a non-list result in [parsed] then
        # iterates and calls .pop() on each element. None would crash;
        # [] makes the iteration a no-op so the chat completion finishes.
        return []
PY
}

apply_server_py() {
    "${VENV_PY}" - "$SERVER_FILE" <<'PY'
import sys
path = sys.argv[1]
src = open(path).read()

# Add has_tools parameter to _make_state_machine
src = src.replace(
    '    def _make_state_machine(\n'
    '        self, model_key, tokenizer, stop_words, initial_state="normal"\n'
    '    ):',
    '    def _make_state_machine(  # Sprint 51c Path B+\n'
    '        self, model_key, tokenizer, stop_words, initial_state="normal",\n'
    '        has_tools=True,  # Sprint 51c Path B+ gating\n'
    '    ):'
)

# Update cache_key
src = src.replace(
    "        cache_key = (model_key, tuple(stop_words), initial_state)",
    "        cache_key = (model_key, tuple(stop_words), initial_state, bool(has_tools))"
)

# Gate tool-call transitions
src = src.replace(
    "        # Tool calling relating transitions\n        if tokenizer.has_tool_calling:\n",
    "        # Tool calling — Sprint 51c Path B+ gates on has_tools so models\n"
    "        # that emit <tool_call> tokens trained-style don't sink content\n"
    "        # into the tool channel when the request didn't ask for tools.\n"
    "        if tokenizer.has_tool_calling and has_tools:\n"
)

# Update both call sites
src = src.replace(
    "                    sm, sequences = self._make_state_machine(\n"
    "                        self.model_provider.model_key,\n"
    "                        tokenizer,\n"
    "                        args.stop_words,\n"
    "                        initial_state,\n"
    "                    )",
    "                    sm, sequences = self._make_state_machine(\n"
    "                        self.model_provider.model_key,\n"
    "                        tokenizer,\n"
    "                        args.stop_words,\n"
    "                        initial_state,\n"
    "                        has_tools=bool(getattr(request, \"tools\", None)),  # Sprint 51c Path B+\n"
    "                    )"
)
src = src.replace(
    "            sm, sequences = self._make_state_machine(\n"
    "                self.model_provider.model_key,\n"
    "                tokenizer,\n"
    "                args.stop_words,\n"
    "                initial_state=initial_state,\n"
    "            )",
    "            sm, sequences = self._make_state_machine(\n"
    "                self.model_provider.model_key,\n"
    "                tokenizer,\n"
    "                args.stop_words,\n"
    "                initial_state=initial_state,\n"
    "                has_tools=bool(getattr(request, \"tools\", None)),  # Sprint 51c Path B+\n"
    "            )"
)

open(path, "w").write(src)
PY
}

case "${1:-apply}" in
    --check)
        ok=true
        if grep -q "$JSON_MARKER" "$JSON_TOOLS"; then
            echo "✅ json_tools.py: patched"
        else
            echo "❌ json_tools.py: NOT patched"
            ok=false
        fi
        if grep -q "$SERVER_MARKER" "$SERVER_FILE"; then
            echo "✅ server.py: patched"
        else
            echo "❌ server.py: NOT patched"
            ok=false
        fi
        $ok && exit 0 || exit 1
        ;;
    --revert)
        "${VENV_PY}" -m pip install --quiet --force-reinstall --no-deps mlx-lm 2>&1 | tail -2
        echo "↩ reverted via pip --force-reinstall"
        exit 0
        ;;
    apply|"")
        if grep -q "$JSON_MARKER" "$JSON_TOOLS" && grep -q "$SERVER_MARKER" "$SERVER_FILE"; then
            echo "✅ both patches present (no-op)"
            exit 0
        fi
        if ! grep -q "$JSON_MARKER" "$JSON_TOOLS"; then
            apply_json_tools
            echo "✅ json_tools.py patched"
        fi
        if ! grep -q "$SERVER_MARKER" "$SERVER_FILE"; then
            apply_server_py
            echo "✅ server.py patched"
        fi
        exit 0
        ;;
    *)
        echo "usage: $0 [apply|--check|--revert]" >&2
        exit 2
        ;;
esac
