#!/usr/bin/env bash
# E2E smoke test: Docker Compose full stack
#
# Builds and starts the production stack, verifies all endpoints respond
# correctly, then tears everything down.
#
# Usage: ./tests/e2e/test_docker_stack.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$ROOT_DIR"

COMPOSE_FILE="docker-compose.yml"
PASSED=0
FAILED=0
ENGINE_URL="http://localhost:8080"
SIDECAR_URL="http://localhost:8100"

# ── Helpers ───────────────────────────────────────────────────────────

cleanup() {
    echo ""
    echo "==> Tearing down containers..."
    docker compose -f "$COMPOSE_FILE" down --volumes --remove-orphans 2>/dev/null || true
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

wait_for_health() {
    local url="$1"
    local name="$2"
    local max_attempts=30

    for i in $(seq 1 "$max_attempts"); do
        if curl -sf "$url" > /dev/null 2>&1; then
            echo "  $name healthy (attempt $i)"
            return 0
        fi
        sleep 2
    done
    echo "  $name failed to become healthy after $((max_attempts * 2))s"
    docker compose -f "$COMPOSE_FILE" logs "$name" 2>/dev/null | tail -20
    return 1
}

assert_status() {
    local description="$1"
    local expected_status="$2"
    local actual_status="$3"

    if [ "$actual_status" -eq "$expected_status" ]; then
        pass "$description (HTTP $actual_status)"
    else
        fail "$description" "expected HTTP $expected_status, got $actual_status"
    fi
}

assert_json_field() {
    local description="$1"
    local json="$2"
    local field="$3"

    if echo "$json" | python3 -c "import sys,json; d=json.load(sys.stdin); assert '$field' in d" 2>/dev/null; then
        pass "$description (has .$field)"
    else
        fail "$description" "missing field '$field' in response"
    fi
}

# ── Build and start ───────────────────────────────────────────────────

echo "==> Building and starting Docker stack..."
docker compose -f "$COMPOSE_FILE" build --quiet
docker compose -f "$COMPOSE_FILE" up -d

echo ""
echo "==> Waiting for services..."
wait_for_health "$SIDECAR_URL/health" "chemiq-sidecar"
wait_for_health "$ENGINE_URL/api/health" "protein-engine"

# ── Test: Health endpoint ─────────────────────────────────────────────

echo ""
echo "==> Testing endpoints..."

STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$ENGINE_URL/api/health")
assert_status "GET /api/health" 200 "$STATUS"

BODY=$(curl -sf "$ENGINE_URL/api/health")
assert_json_field "GET /api/health body" "$BODY" "status"

# ── Test: Sidecar health ──────────────────────────────────────────────

STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$SIDECAR_URL/health")
assert_status "GET sidecar /health" 200 "$STATUS"

# ── Test: Score a variant ─────────────────────────────────────────────

SCORE_BODY='{"name":"test-variant","sequence":"MKWVTFISLLLLFSSAYS","target_factor":"OCT4"}'
STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST -H "Content-Type: application/json" \
    -d "$SCORE_BODY" \
    "$ENGINE_URL/api/variants/score")
assert_status "POST /api/variants/score" 200 "$STATUS"

RESPONSE=$(curl -sf \
    -X POST -H "Content-Type: application/json" \
    -d "$SCORE_BODY" \
    "$ENGINE_URL/api/variants/score")
assert_json_field "Score response" "$RESPONSE" "composite"
assert_json_field "Score response" "$RESPONSE" "reprogramming_efficiency"

# ── Test: Evolution cycle ─────────────────────────────────────────────

EVOLVE_BODY='{"generation":0,"population_size":10,"mutation_rate":0.3,"crossover_rate":0.2,"quantum_enabled":false,"top_k":5}'
STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST -H "Content-Type: application/json" \
    -d "$EVOLVE_BODY" \
    "$ENGINE_URL/api/evolution/cycle")
assert_status "POST /api/evolution/cycle" 200 "$STATUS"

RESPONSE=$(curl -sf \
    -X POST -H "Content-Type: application/json" \
    -d "$EVOLVE_BODY" \
    "$ENGINE_URL/api/evolution/cycle")
assert_json_field "Evolution response" "$RESPONSE" "generation"
assert_json_field "Evolution response" "$RESPONSE" "promoted"

# ── Test: Ledger verify ───────────────────────────────────────────────

STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$ENGINE_URL/api/ledger/verify")
assert_status "GET /api/ledger/verify" 200 "$STATUS"

RESPONSE=$(curl -sf "$ENGINE_URL/api/ledger/verify")
assert_json_field "Ledger verify response" "$RESPONSE" "valid"

# ── Summary ───────────────────────────────────────────────────────────

echo ""
echo "========================================"
echo "  E2E Docker Stack: $PASSED passed, $FAILED failed"
echo "========================================"

if [ "$FAILED" -gt 0 ]; then
    echo ""
    echo "Container logs:"
    docker compose -f "$COMPOSE_FILE" logs --tail=30
    exit 1
fi
