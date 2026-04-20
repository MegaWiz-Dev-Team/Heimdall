#!/usr/bin/env bash
set -euo pipefail

# ============================================
# Heimdall — Model Manager
# Manage models between internal and external SSD
# Usage:
#   ./scripts/model_manager.sh list              # list all models
#   ./scripts/model_manager.sh archive <model>   # move to external SSD
#   ./scripts/model_manager.sh restore <model>   # move back to internal
#   ./scripts/model_manager.sh status            # show storage summary
# ============================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# Load config
if [ -f "$PROJECT_DIR/.env" ]; then
    set -a; source "$PROJECT_DIR/.env"; set +a
fi

INTERNAL_DIR="${HF_HOME:-$HOME/.cache/huggingface}/hub"
EXTERNAL_DIR="${EXTERNAL_MODEL_DIR:-}"

# ============================================
# Helper functions
# ============================================

human_size() {
    local bytes="$1"
    if [ "$bytes" -gt 1073741824 ]; then
        echo "$(echo "scale=1; $bytes / 1073741824" | bc) GB"
    elif [ "$bytes" -gt 1048576 ]; then
        echo "$(echo "scale=0; $bytes / 1048576" | bc) MB"
    else
        echo "${bytes} B"
    fi
}

get_model_dirs() {
    local base_dir="$1"
    if [ -d "$base_dir" ]; then
        find "$base_dir" -maxdepth 1 -type d -name "models--*" 2>/dev/null | sort
    fi
}

model_dir_to_name() {
    echo "$1" | sed 's|.*/models--||; s|--|/|g'
}

model_name_to_dir() {
    echo "models--$(echo "$1" | sed 's|/|--|g')"
}

check_external() {
    if [ -z "$EXTERNAL_DIR" ]; then
        echo "❌ EXTERNAL_MODEL_DIR ไม่ได้ตั้งค่า"
        echo ""
        echo "เพิ่มใน .env:"
        echo "  EXTERNAL_MODEL_DIR=/Volumes/YOUR_SSD/huggingface/models"
        echo ""
        echo "หรือ set ตรง:"
        echo "  export EXTERNAL_MODEL_DIR=/Volumes/YOUR_SSD/huggingface/models"
        exit 1
    fi
    if [ ! -d "$(dirname "$EXTERNAL_DIR")" ]; then
        echo "❌ External SSD ไม่พบ: $EXTERNAL_DIR"
        echo "   ตรวจสอบว่า SSD เสียบอยู่และ mount แล้ว"
        exit 1
    fi
    mkdir -p "$EXTERNAL_DIR"
}

# ============================================
# Commands
# ============================================

cmd_list() {
    echo ""
    echo "🛡️ Heimdall Model Manager"
    echo "========================="
    echo ""

    echo "📁 Internal SSD: $INTERNAL_DIR"
    if [ -d "$INTERNAL_DIR" ]; then
        local count=0
        while IFS= read -r dir; do
            [ -z "$dir" ] && continue
            local name=$(model_dir_to_name "$dir")
            local size=$(du -s "$dir" 2>/dev/null | awk '{print $1 * 512}')
            local hsize=$(human_size "${size:-0}")
            echo "   ✅ $name ($hsize)"
            count=$((count + 1))
        done <<< "$(get_model_dirs "$INTERNAL_DIR")"
        if [ $count -eq 0 ]; then
            echo "   (ว่าง — ยังไม่มี model)"
        fi
    else
        echo "   (ยังไม่มีโฟลเดอร์)"
    fi

    echo ""
    if [ -n "$EXTERNAL_DIR" ] && [ -d "$EXTERNAL_DIR" ]; then
        echo "💾 External SSD: $EXTERNAL_DIR"
        local count=0
        while IFS= read -r dir; do
            [ -z "$dir" ] && continue
            local name=$(model_dir_to_name "$dir")
            local size=$(du -s "$dir" 2>/dev/null | awk '{print $1 * 512}')
            local hsize=$(human_size "${size:-0}")
            echo "   📦 $name ($hsize)"
            count=$((count + 1))
        done <<< "$(get_model_dirs "$EXTERNAL_DIR")"
        if [ $count -eq 0 ]; then
            echo "   (ว่าง)"
        fi
    else
        echo "💾 External SSD: (ไม่ได้ตั้งค่า — set EXTERNAL_MODEL_DIR)"
    fi
    echo ""
}

cmd_status() {
    echo ""
    echo "🛡️ Heimdall Storage Status"
    echo "=========================="
    echo ""

    # Internal
    local internal_count=0
    local internal_size=0
    if [ -d "$INTERNAL_DIR" ]; then
        while IFS= read -r dir; do
            [ -z "$dir" ] && continue
            internal_count=$((internal_count + 1))
            local s=$(du -s "$dir" 2>/dev/null | awk '{print $1 * 512}')
            internal_size=$((internal_size + ${s:-0}))
        done <<< "$(get_model_dirs "$INTERNAL_DIR")"
    fi

    echo "📁 Internal: $internal_count models ($(human_size $internal_size))"

    # External
    if [ -n "$EXTERNAL_DIR" ] && [ -d "$EXTERNAL_DIR" ]; then
        local external_count=0
        local external_size=0
        while IFS= read -r dir; do
            [ -z "$dir" ] && continue
            external_count=$((external_count + 1))
            local s=$(du -s "$dir" 2>/dev/null | awk '{print $1 * 512}')
            external_size=$((external_size + ${s:-0}))
        done <<< "$(get_model_dirs "$EXTERNAL_DIR")"
        echo "💾 External: $external_count models ($(human_size $external_size))"
    fi

    # Disk
    echo ""
    echo "💿 Disk Usage:"
    local internal_df_path="$INTERNAL_DIR"
    [ ! -d "$internal_df_path" ] && internal_df_path="$HOME"
    df -h "$internal_df_path" 2>/dev/null | tail -1 | awk '{printf "   Internal: %s used / %s total (%s free)\n", $3, $2, $4}'
    if [ -n "$EXTERNAL_DIR" ] && [ -d "$EXTERNAL_DIR" ]; then
        df -h "$EXTERNAL_DIR" 2>/dev/null | tail -1 | awk '{printf "   External: %s used / %s total (%s free)\n", $3, $2, $4}'
    fi
    echo ""
}

cmd_archive() {
    local model_name="${1:-}"
    if [ -z "$model_name" ]; then
        echo "❌ Usage: model_manager.sh archive <model-name>"
        echo "   Example: model_manager.sh archive mlx-community/Qwen2.5-7B-Instruct-4bit"
        exit 1
    fi

    check_external

    local dir_name=$(model_name_to_dir "$model_name")
    local src="$INTERNAL_DIR/$dir_name"
    local dst="$EXTERNAL_DIR/$dir_name"

    if [ ! -d "$src" ]; then
        echo "❌ Model '$model_name' ไม่พบใน internal SSD"
        echo "   Path: $src"
        exit 1
    fi

    if [ -d "$dst" ]; then
        echo "⚠️  Model '$model_name' มีอยู่แล้วใน external SSD"
        read -p "   ต้องการเขียนทับ? (y/N) " confirm
        if [[ ! "$confirm" =~ ^[Yy]$ ]]; then
            echo "   ยกเลิก"
            exit 0
        fi
        rm -rf "$dst"
    fi

    local size=$(du -sh "$src" 2>/dev/null | awk '{print $1}')
    echo ""
    echo "📦 Archiving to external SSD..."
    echo "   Model: $model_name"
    echo "   Size:  $size"
    echo "   From:  $src"
    echo "   To:    $dst"
    echo ""

    # Move with progress
    rsync -ah --progress "$src/" "$dst/"
    if [ $? -eq 0 ]; then
        rm -rf "$src"
        echo ""
        echo "✅ Archived! Model ย้ายไปเก็บใน external SSD แล้ว"
    else
        echo "❌ Error: rsync failed"
        exit 1
    fi
}

cmd_restore() {
    local model_name="${1:-}"
    if [ -z "$model_name" ]; then
        echo "❌ Usage: model_manager.sh restore <model-name>"
        echo "   Example: model_manager.sh restore mlx-community/Qwen2.5-7B-Instruct-4bit"
        exit 1
    fi

    check_external

    local dir_name=$(model_name_to_dir "$model_name")
    local src="$EXTERNAL_DIR/$dir_name"
    local dst="$INTERNAL_DIR/$dir_name"

    if [ ! -d "$src" ]; then
        echo "❌ Model '$model_name' ไม่พบใน external SSD"
        echo "   Path: $src"
        exit 1
    fi

    if [ -d "$dst" ]; then
        echo "⚠️  Model '$model_name' มีอยู่แล้วใน internal SSD"
        read -p "   ต้องการเขียนทับ? (y/N) " confirm
        if [[ ! "$confirm" =~ ^[Yy]$ ]]; then
            echo "   ยกเลิก"
            exit 0
        fi
        rm -rf "$dst"
    fi

    local size=$(du -sh "$src" 2>/dev/null | awk '{print $1}')
    echo ""
    echo "📥 Restoring to internal SSD..."
    echo "   Model: $model_name"
    echo "   Size:  $size"
    echo "   From:  $src"
    echo "   To:    $dst"
    echo ""

    mkdir -p "$INTERNAL_DIR"
    rsync -ah --progress "$src/" "$dst/"
    if [ $? -eq 0 ]; then
        rm -rf "$src"
        echo ""
        echo "✅ Restored! Model พร้อมใช้งานแล้ว"
    else
        echo "❌ Error: rsync failed"
        exit 1
    fi
}

cmd_remove() {
    local model_name="${1:-}"
    if [ -z "$model_name" ]; then
        echo "❌ Usage: model_manager.sh rm <model-name>"
        echo "   Example: model_manager.sh rm mlx-community/Qwen2.5-7B-Instruct-4bit"
        exit 1
    fi

    local dir_name=$(model_name_to_dir "$model_name")
    local src_internal="$INTERNAL_DIR/$dir_name"
    local src_external="$EXTERNAL_DIR/$dir_name"
    local found=0

    echo ""
    echo "🗑️  Removing model: $model_name"

    if [ -d "$src_internal" ]; then
        local size=$(du -sh "$src_internal" 2>/dev/null | awk '{print $1}')
        rm -rf "$src_internal"
        echo "   ✅ Removed from Internal SSD (freed $size)"
        found=1
    fi

    if [ -n "$EXTERNAL_DIR" ] && [ -d "$EXTERNAL_DIR" ] && [ -d "$src_external" ]; then
        local size=$(du -sh "$src_external" 2>/dev/null | awk '{print $1}')
        rm -rf "$src_external"
        echo "   ✅ Removed from External SSD (freed $size)"
        found=1
    fi

    if [ $found -eq 0 ]; then
        echo "   ❌ Model not found in any cache."
        exit 1
    fi
    echo ""
}

cmd_help() {
    echo ""
    echo "🛡️ Heimdall Model Manager"
    echo ""
    echo "Usage: ./scripts/model_manager.sh <command> [args]"
    echo ""
    echo "Commands:"
    echo "  list                     List all models (internal + external)"
    echo "  status                   Show storage summary & disk usage"
    echo "  archive <model-name>     Move model from internal → external SSD"
    echo "  restore <model-name>     Move model from external → internal SSD"
    echo "  rm <model-name>          Permanently delete a model"
    echo "  help                     Show this help"
    echo ""
    echo "Config (.env):"
    echo "  EXTERNAL_MODEL_DIR=/Volumes/YOUR_SSD/huggingface/models"
    echo ""
    echo "Examples:"
    echo "  ./scripts/model_manager.sh list"
    echo "  ./scripts/model_manager.sh archive mlx-community/Qwen2.5-7B-Instruct-4bit"
    echo "  ./scripts/model_manager.sh restore mlx-community/Qwen2.5-7B-Instruct-4bit"
    echo ""
}

# ============================================
# Main
# ============================================

case "${1:-help}" in
    list)     cmd_list ;;
    status)   cmd_status ;;
    archive)  cmd_archive "${2:-}" ;;
    restore)  cmd_restore "${2:-}" ;;
    rm)       cmd_remove "${2:-}" ;;
    help|*)   cmd_help ;;
esac
