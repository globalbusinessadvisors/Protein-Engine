.PHONY: test build wasm rvf docker fmt clippy clean e2e e2e-cli e2e-docker e2e-wasm

# ── Development ──────────────────────────────────────────────────────

test: ## Run all tests (native features)
	cargo test --features native

test-unit: ## Run unit tests only (exclude integration)
	cargo test --features native --lib

test-integration: ## Run integration tests only
	cargo test -p pe-integration-tests

test-sidecar: ## Run Python sidecar tests
	cd services/chemiq-sidecar && pytest test_sidecar.py -v

fmt: ## Check formatting
	cargo fmt --check --all

clippy: ## Run clippy lints
	cargo clippy --all-targets --features native -- -D warnings

# ── E2E Tests ────────────────────────────────────────────────────────

e2e: e2e-cli ## Run all E2E smoke tests (CLI only; add e2e-docker e2e-wasm as needed)

e2e-cli: release ## Run CLI E2E smoke tests
	./tests/e2e/test_cli.sh

e2e-docker: ## Run Docker stack E2E smoke tests
	./tests/e2e/test_docker_stack.sh

e2e-wasm: wasm ## Run WASM E2E smoke tests (requires wasm-pack)
	node tests/e2e/test_wasm.mjs

# ── Build ────────────────────────────────────────────────────────────

build: ## Build native binary (debug)
	cargo build --features native --bin protein-engine

release: ## Build native binary (release)
	cargo build --release --features native --bin protein-engine

wasm: ## Build WASM package
	./build-wasm.sh

rvf: ## Build .rvf artifact
	./build-rvf.sh

# ── Docker ───────────────────────────────────────────────────────────

docker: ## Run production stack
	docker compose up --build

docker-dev: ## Run dev stack with hot reload
	docker compose -f docker-compose.dev.yml up --build

docker-down: ## Stop all containers
	docker compose down
	docker compose -f docker-compose.dev.yml down

# ── Serve ────────────────────────────────────────────────────────────

serve: build ## Build and run the HTTP server
	cargo run --features native --bin protein-engine -- serve

# ── Maintenance ──────────────────────────────────────────────────────

clean: ## Remove build artifacts
	cargo clean
	rm -rf web/pkg protein-engine.rvf

help: ## Show this help
	@grep -E '^[a-zA-Z0-9_-]+:.*## .*$$' $(MAKEFILE_LIST) | sort | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2}'

.DEFAULT_GOAL := help
