.PHONY: build wasm test clippy fmt fmt-check check clean deploy verify

NETWORK ?= testnet
IDENTITY ?= subs-deployer

# Build all contracts to wasm with the Stellar CLI.
build:
	stellar contract build

# Build all contracts to wasm with cargo only (what CI runs).
wasm:
	cargo build --workspace --release --target wasm32v1-none

# Run every contract's test suite, including the invariant tests.
test:
	cargo test --workspace

clippy:
	cargo clippy --workspace --all-targets -- -D warnings

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

# Everything CI checks, in CI's order.
check: test clippy fmt-check wasm

clean:
	cargo clean

# Deploy, initialize and wire all three contracts.
deploy:
	NETWORK=$(NETWORK) IDENTITY=$(IDENTITY) ./scripts/deploy.sh

# Prove CAP, INTERVAL and REVOCATION against the live deployment
# (run `DEMO=1 make deploy` first).
verify:
	NETWORK=$(NETWORK) ./scripts/verify-testnet.sh
