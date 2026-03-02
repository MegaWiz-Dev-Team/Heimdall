#!/usr/bin/env bash
set -euo pipefail

# ============================================
# Version Manager — Semantic Versioning
# Single source: VERSION file
# Syncs to: Cargo.toml, benchmark reports
# ============================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
VERSION_FILE="$PROJECT_DIR/VERSION"

CURRENT=$(cat "$VERSION_FILE" | tr -d '[:space:]')
IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT"

usage() {
    echo "Usage: ./scripts/version.sh <command>"
    echo ""
    echo "Commands:"
    echo "  show                  Show current version"
    echo "  bump patch            0.1.0 → 0.1.1 (bug fixes)"
    echo "  bump minor            0.1.0 → 0.2.0 (new features)"
    echo "  bump major            0.1.0 → 1.0.0 (breaking changes)"
    echo "  set <version>         Set specific version (e.g., 1.0.0)"
    echo "  tag                   Create git tag for current version"
    echo "  release               bump + sync + commit + tag"
    echo ""
    echo "Current: v${CURRENT}"
}

sync_version() {
    local VERSION="$1"

    # Update Cargo.toml
    if [ -f "$PROJECT_DIR/gateway/Cargo.toml" ]; then
        sed -i '' "s/^version = \".*\"/version = \"${VERSION}\"/" "$PROJECT_DIR/gateway/Cargo.toml"
        echo "  ✅ gateway/Cargo.toml → ${VERSION}"
    fi

    # Update VERSION file
    echo "$VERSION" > "$VERSION_FILE"
    echo "  ✅ VERSION → ${VERSION}"
}

show_version() {
    echo "v${CURRENT}"
    echo ""
    echo "  VERSION file:  ${CURRENT}"

    # Check Cargo.toml
    if [ -f "$PROJECT_DIR/gateway/Cargo.toml" ]; then
        local cargo_ver=$(grep '^version' "$PROJECT_DIR/gateway/Cargo.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')
        local synced="✅"
        [ "$cargo_ver" != "$CURRENT" ] && synced="❌ out of sync!"
        echo "  Cargo.toml:    ${cargo_ver} ${synced}"
    fi

    # Check git tag
    local tag_exists="❌ not tagged"
    git tag -l "v${CURRENT}" | grep -q "v${CURRENT}" 2>/dev/null && tag_exists="✅ tagged"
    echo "  Git tag:       ${tag_exists}"

    # Show recent benchmarks for this version
    echo ""
    echo "  Benchmarks for v${CURRENT}:"
    local found=0
    for f in "$PROJECT_DIR"/reports/benchmark_*.json; do
        [ ! -f "$f" ] && continue
        local bver=$(python3 -c "import json; print(json.load(open('$f')).get('version','???'))" 2>/dev/null || echo "???")
        if [ "$bver" = "$CURRENT" ]; then
            local bdate=$(python3 -c "import json; print(json.load(open('$f')).get('timestamp','?'))" 2>/dev/null)
            local btps=$(python3 -c "import json; d=json.load(open('$f')); print(f\"{max(d['tests'][k]['tps_avg'] for k in d['tests']):.1f} tok/s\")" 2>/dev/null || echo "?")
            echo "    📊 $(basename "$f") — ${btps} (${bdate})"
            found=$((found + 1))
        fi
    done
    [ "$found" -eq 0 ] && echo "    (no benchmarks yet — run ./scripts/benchmark.sh)"
}

bump_version() {
    local TYPE="$1"
    local NEW=""

    case "$TYPE" in
        patch) NEW="${MAJOR}.${MINOR}.$((PATCH + 1))" ;;
        minor) NEW="${MAJOR}.$((MINOR + 1)).0" ;;
        major) NEW="$((MAJOR + 1)).0.0" ;;
        *) echo "❌ Unknown bump type: $TYPE"; usage; exit 1 ;;
    esac

    echo "🔖 Bumping version: v${CURRENT} → v${NEW}"
    sync_version "$NEW"
}

set_version() {
    local NEW="$1"
    # Validate format
    if ! echo "$NEW" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$'; then
        echo "❌ Invalid version format: $NEW (expected X.Y.Z)"
        exit 1
    fi
    echo "🔖 Setting version: v${CURRENT} → v${NEW}"
    sync_version "$NEW"
}

tag_version() {
    local VERSION=$(cat "$VERSION_FILE" | tr -d '[:space:]')
    local TAG="v${VERSION}"

    if git tag -l "$TAG" | grep -q "$TAG" 2>/dev/null; then
        echo "⚠️  Tag $TAG already exists"
        return 0
    fi

    git tag -a "$TAG" -m "Release ${TAG}"
    echo "🏷️  Created tag: ${TAG}"
    echo "   Push with: git push origin ${TAG}"
}

release() {
    local TYPE="${1:-patch}"

    echo "🚀 Release: ${TYPE} bump"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━"
    bump_version "$TYPE"

    local NEW=$(cat "$VERSION_FILE" | tr -d '[:space:]')

    echo ""
    echo "📦 Committing..."
    git add -A
    git commit -m "[RELEASE] v${NEW}"

    echo ""
    tag_version

    echo ""
    echo "✅ Released v${NEW}"
    echo "   Push: git push origin main --tags"
}

# ============================================
# Command dispatch
# ============================================

CMD="${1:-show}"
shift || true

case "$CMD" in
    show|version|v)    show_version ;;
    bump|b)            bump_version "${1:-patch}" ;;
    set|s)             set_version "${1:?Version required}" ;;
    tag|t)             tag_version ;;
    release|r)         release "${1:-patch}" ;;
    help|h|-h|--help)  usage ;;
    *)                 echo "❌ Unknown command: $CMD"; usage; exit 1 ;;
esac
