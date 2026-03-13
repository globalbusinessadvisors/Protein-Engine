#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUT_DIR="${SCRIPT_DIR}/web/pkg"

echo "==> Building pe-wasm with wasm-pack..."
wasm-pack build "${SCRIPT_DIR}/crates/pe-wasm" \
    --target web \
    --release \
    --out-dir "${OUT_DIR}"

# Remove wasm-pack metadata file (not needed for deployment)
rm -f "${OUT_DIR}/.gitignore" "${OUT_DIR}/package.json"

echo ""
echo "==> Build complete: ${OUT_DIR}"
echo "    Bundle sizes:"
for f in "${OUT_DIR}"/*.wasm "${OUT_DIR}"/*.js; do
    [ -f "$f" ] && printf "    %-40s %s\n" "$(basename "$f")" "$(du -h "$f" | cut -f1)"
done
