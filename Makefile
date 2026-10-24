SHELL := /bin/bash

.PHONY: format rust-lint python-check fensu check-ci test census

format:
	cargo fmt --all
	uv run ruff format .

rust-lint:
	cargo fmt --all --check
	cargo clippy --workspace --all-targets --locked -- -D warnings

python-check:
	uv run ruff format --check .
	uv run ruff check .
	uv run ty check

fensu:
	uv run fensu check

# The static gate: CI and the pre-push verify hook run exactly this.
check-ci: rust-lint python-check fensu

test:
	cargo test --workspace --locked

# Example: make census CENSUS_ARGS="path/to/sql --json"
census:
	cargo run --release --locked -p sqlquack -- census $(CENSUS_ARGS)
