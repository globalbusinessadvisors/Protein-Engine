#!/usr/bin/env bash
# E2E smoke test: pe-cli binary
#
# Builds the native binary and exercises all major commands.
#
# Usage: ./tests/e2e/test_cli.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$ROOT_DIR"

PASSED=0
FAILED=0
RVF_PATH="/tmp/pe-e2e-test-$$.rvf"

# ── Helpers ───────────────────────────────────────────────────────────

cleanup() {
    rm -f "$RVF_PATH"
}
trap cleanup EXIT

pass() {
    PASSED=$((PASSED + 1))
    echo "  PASS: $1"
}

fail() {
    FAILED=$((FAILED + 1))
    echo "  FAIL: $1"
    echo "        $2"
}

assert_exit_zero() {
    local description="$1"
    shift
    if OUTPUT=$("$@" 2>&1); then
        pass "$description"
        echo "$OUTPUT"
    else
        fail "$description" "exit code $?, output: $OUTPUT"
        echo ""
    fi
}

assert_output_contains() {
    local description="$1"
    local expected="$2"
    local output="$3"

    if echo "$output" | grep -qi "$expected"; then
        pass "$description (contains '$expected')"
    else
        fail "$description" "output does not contain '$expected'"
    fi
}

assert_json_valid() {
    local description="$1"
    local output="$2"

    if echo "$output" | python3 -c "import sys,json; json.load(sys.stdin)" 2>/dev/null; then
        pass "$description (valid JSON)"
    else
        fail "$description" "output is not valid JSON"
    fi
}

assert_json_has_field() {
    local description="$1"
    local output="$2"
    local field="$3"

    if echo "$output" | python3 -c "import sys,json; d=json.load(sys.stdin); assert '$field' in d" 2>/dev/null; then
        pass "$description (has .$field)"
    else
        fail "$description" "missing field '$field'"
    fi
}

# ── Build ─────────────────────────────────────────────────────────────

echo "==> Building pe-cli (release)..."
cargo build --release --features native --bin protein-engine
PE_CLI="$ROOT_DIR/target/release/protein-engine"

if [ ! -x "$PE_CLI" ]; then
    echo "FATAL: Binary not found at $PE_CLI"
    exit 1
fi

echo ""
echo "==> Running CLI smoke tests..."

# ── Test: init ────────────────────────────────────────────────────────

echo ""
echo "--- init ---"
OUTPUT=$("$PE_CLI" init --output "$RVF_PATH" 2>&1) || true
if [ -f "$RVF_PATH" ]; then
    pass "init creates RVF file"
else
    fail "init creates RVF file" "file not found at $RVF_PATH"
fi

# ── Test: score ───────────────────────────────────────────────────────

echo ""
echo "--- score ---"
OUTPUT=$("$PE_CLI" --json score "MKWVTFISLLLLFSSAYS" 2>&1)
assert_json_valid "score outputs JSON" "$OUTPUT"
assert_json_has_field "score response" "$OUTPUT" "composite"
assert_json_has_field "score response" "$OUTPUT" "reprogramming_efficiency"
assert_json_has_field "score response" "$OUTPUT" "expression_stability"
assert_json_has_field "score response" "$OUTPUT" "safety_score"

# Also test text mode
TEXT_OUTPUT=$("$PE_CLI" score "MKWVTFISLLLLFSSAYS" 2>&1)
assert_output_contains "score text mode" "composite" "$TEXT_OUTPUT"

# ── Test: evolve ──────────────────────────────────────────────────────

echo ""
echo "--- evolve ---"
OUTPUT=$("$PE_CLI" --json evolve --generations 2 --population-size 10 2>&1)
assert_json_valid "evolve outputs JSON" "$OUTPUT"

TEXT_OUTPUT=$("$PE_CLI" evolve --generations 2 --population-size 10 2>&1)
assert_output_contains "evolve text mode" "Generation 1" "$TEXT_OUTPUT"

# ── Test: search ──────────────────────────────────────────────────────

echo ""
echo "--- search ---"
OUTPUT=$("$PE_CLI" --json search "MKWVTFISLLLLFSSAYS" --k 3 2>&1)
assert_json_valid "search outputs JSON" "$OUTPUT"

# ── Test: quantum vqe ─────────────────────────────────────────────────

echo ""
echo "--- quantum vqe ---"
OUTPUT=$("$PE_CLI" --json quantum vqe H2 2>&1)
assert_json_valid "quantum vqe outputs JSON" "$OUTPUT"
assert_json_has_field "quantum vqe response" "$OUTPUT" "ground_state_energy"
assert_json_has_field "quantum vqe response" "$OUTPUT" "converged"

# ── Test: ledger verify ───────────────────────────────────────────────

echo ""
echo "--- ledger verify ---"
OUTPUT=$("$PE_CLI" --json ledger verify 2>&1)
assert_json_valid "ledger verify outputs JSON" "$OUTPUT"
assert_json_has_field "ledger verify response" "$OUTPUT" "valid"

# ── Test: rvf inspect ─────────────────────────────────────────────────

echo ""
echo "--- rvf inspect ---"
if [ -f "$RVF_PATH" ]; then
    OUTPUT=$("$PE_CLI" --json rvf inspect "$RVF_PATH" 2>&1)
    assert_json_valid "rvf inspect outputs JSON" "$OUTPUT"
    assert_json_has_field "rvf inspect response" "$OUTPUT" "name"
    assert_json_has_field "rvf inspect response" "$OUTPUT" "segment_count"
    assert_json_has_field "rvf inspect response" "$OUTPUT" "file_hash"
else
    fail "rvf inspect" "no RVF file from init step"
fi

# ── Summary ───────────────────────────────────────────────────────────

echo ""
echo "========================================"
echo "  E2E CLI: $PASSED passed, $FAILED failed"
echo "========================================"

[ "$FAILED" -eq 0 ] || exit 1
