#!/bin/bash
# Script to convert and quantize Qwen3.5 27B Destilled model to MLX 4-bit

# Resolve the root project directory automatically
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# Allow overriding HF CACHE directory (e.g. for external SSDs)
export HF_HUB_CACHE="${HF_HUB_CACHE:-$HOME/.cache/huggingface/hub}"
mkdir -p "$HF_HUB_CACHE"
mkdir -p "$PROJECT_DIR/models"

# Source virtual environment if exists
if [ -f "$PROJECT_DIR/.venv/bin/activate" ]; then
    source "$PROJECT_DIR/.venv/bin/activate"
fi

# Update mlx-lm to ensure compatibility
pip install -U mlx-lm

MODEL_ID="Jackrong/Qwen3.5-27B-Claude-4.6-Opus-Reasoning-Distilled"
OUTPUT_DIR="$PROJECT_DIR/models/Qwen3.5-27B-Opus-Reasoning-MLX-4bit"

echo "Starting download and conversion of $MODEL_ID..."
echo "Raw safetensors will be cached to: $HF_HUB_CACHE"
echo "4-bit MLX model will be saved to: $OUTPUT_DIR"

mlx_lm.convert \
  --hf-path "$MODEL_ID" \
  -q \
  --q-bits 4 \
  --mlx-path "$OUTPUT_DIR"

echo "Conversion complete!"
