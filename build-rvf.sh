#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RVF_OUT="${SCRIPT_DIR}/protein-engine.rvf"

echo "==> Building native binary..."
cargo build --release --features native --bin protein-engine \
    --manifest-path "${SCRIPT_DIR}/Cargo.toml"

BINARY="${SCRIPT_DIR}/target/release/protein-engine"

echo "==> Assembling RVF file..."
"${BINARY}" rvf build --output "${RVF_OUT}"

echo ""
echo "==> RVF assembled: ${RVF_OUT}"
echo "    Size: $(du -h "${RVF_OUT}" | cut -f1)"
echo ""
echo "==> Segment summary:"
"${BINARY}" rvf inspect --path "${RVF_OUT}"
