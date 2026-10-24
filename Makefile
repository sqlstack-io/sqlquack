SHELL := /bin/bash

# CI sets RECORDER=0: the Snowflake recorder is never installed, type-checked or tested there.
RECORDER ?= 1
# The revision recorded results are compared against by the append-only guard.
EVIDENCE_BASE ?= origin/main

ifeq ($(RECORDER),1)
PYTHON_TYPED := .
PYTHON_TESTS := tests
else
PYTHON_TYPED := scripts tests/unit/scripts
PYTHON_TESTS := tests/unit/scripts
endif

.PHONY: format rust-lint python-check fensu evidence-guard check-ci test census

format:
	cargo fmt --all
	uv run ruff format .

rust-lint:
	cargo fmt --all --check
	cargo clippy --workspace --all-targets --locked -- -D warnings

python-check:
	uv run ruff format --check .
	uv run ruff check .
	uv run ty check $(PYTHON_TYPED)

fensu:
	uv run fensu check

# Fails when a change modifies or deletes a recorded result, or adds an invalid one.
evidence-guard:
	uv run python -m scripts.check_evidence_append_only --base $(EVIDENCE_BASE)

# The static gate: CI and the pre-push verify hook run exactly this.
check-ci: rust-lint python-check fensu evidence-guard

test:
	cargo test --workspace --locked
	uv run pytest $(PYTHON_TESTS) -n auto --dist loadfile

# Example: make census CENSUS_ARGS="path/to/sql --json"
census:
	cargo run --release --locked -p sqlquack -- census $(CENSUS_ARGS)
