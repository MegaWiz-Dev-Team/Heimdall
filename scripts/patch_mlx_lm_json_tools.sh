#!/usr/bin/env bash
# Sprint 51c Path B — patch mlx_lm/tool_parsers/json_tools.py in the
# Heimdall venv to swallow JSON-decode errors instead of crashing the
# whole /v1/chat/completions request.
#
# STATUS: PARTIAL FIX
#   ✅ Prevents the 500 crash on malformed/empty tool_call JSON
#      (useful defensively for any model that emits irregular
#       tool_call payloads, not just Typhoon).
#   ❌ Does NOT restore message.content for typhoon-si-med-thinking-4b
#      because mlx_lm.server's state machine (server.py L640+) routes
#      <tool_call> tokens to the "tool" channel regardless of whether
#      the request actually passed `tools`. With this patch, the chat
#      completion returns 200 with finish_reason="tool_calls" and
#      empty message body — server doesn't crash but content is gone.
#
# A FULL fix needs a deeper patch on _build_state_machine to skip
# tool-call transitions when the request has no tools. That's bigger
# scope (re-creating the SM per-request, not cached at load), tracked
# as Sprint 51d work.
#
# eir-research agent stays on Ollama meanwhile — see
# Mimir/docs/04_evaluation_and_testing/04_03_HealthBench_Pro_Baseline_2026-05-04.md
# § Sprint 51c.
#
# Re-apply this script after every `pip install --upgrade mlx-lm`. Drop
# entirely once upstream lands a --no-tools flag or per-request tool
# gating.
#
# Usage:
#   bash scripts/patch_mlx_lm_json_tools.sh           # apply or no-op
#   bash scripts/patch_mlx_lm_json_tools.sh --check   # verify already patched
#   bash scripts/patch_mlx_lm_json_tools.sh --revert  # restore stock

set -euo pipefail

VENV_FILE=/Users/mimir/Developer/Heimdall/.venv/lib/python3.14/site-packages/mlx_lm/tool_parsers/json_tools.py
MARKER="Sprint 51c Path B local patch"

case "${1:-apply}" in
    --check)
        if grep -q "$MARKER" "$VENV_FILE"; then
            echo "✅ patch present"
            exit 0
        else
            echo "❌ patch NOT present — re-apply with: $0"
            exit 1
        fi
        ;;
    --revert)
        cat > "$VENV_FILE" <<'PY'
# Copyright © 2025 Apple Inc.

import json

tool_call_start = "<tool_call>"

tool_call_end = "</tool_call>"


def parse_tool_call(text, tools=None):
    return json.loads(text.strip())
PY
        echo "↩ reverted to stock"
        exit 0
        ;;
    apply|"")
        if grep -q "$MARKER" "$VENV_FILE"; then
            echo "✅ already patched (no-op)"
            exit 0
        fi
        cat > "$VENV_FILE" <<'PY'
# Copyright © 2025 Apple Inc.
#
# Sprint 51c Path B local patch (Asgard Heimdall venv only) — return None
# instead of raising when text isn't valid JSON. Models like
# typhoon-ai/typhoon-si-med-thinking-4b emit a trailing <tool_call> token
# (a known model-card quirk) that this parser was crashing on, killing
# the whole /v1/chat/completions request. Returning None lets the caller
# treat the malformed segment as "no tool call detected" and continue.
#
# Re-apply this patch via Heimdall/scripts/patch_mlx_lm_json_tools.sh
# after every `pip install --upgrade mlx-lm`. Drop entirely once
# upstream lands a --no-tools flag or fixes the parser.

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
        echo "✅ patch applied"
        exit 0
        ;;
    *)
        echo "usage: $0 [apply|--check|--revert]" >&2
        exit 2
        ;;
esac
