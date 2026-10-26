SHELL := /bin/bash

# CI sets RECORDER=0: the Snowflake recorder is never installed, type-checked or tested there.
RECORDER ?= 1
# The revision recorded results are compared against by the append-only guard.
EVIDENCE_BASE ?= origin/main
# Compiled DuckShop projects the exact parser check reads, as CI's polyglot-check job compiles them.
DUCKSHOP_COMPILED ?=
# How to run cargo-fuzz, the corpus it starts from and how long it runs, as CI's fuzz job does.
FUZZ_CARGO ?= cargo +nightly fuzz
FUZZ_CORPUS ?= fuzz/corpus/tolerant
FUZZ_SECONDS ?= 90

ifeq ($(RECORDER),1)
PYTHON_TYPED := .
PYTHON_TESTS := tests
else
PYTHON_TYPED := scripts tests/unit/scripts
PYTHON_TESTS := tests/unit/scripts
endif

.PHONY: format rust-lint python-check fensu evidence-guard polyglot-guard generate-cases generated-cases check-ci test verify-recorded polyglot-duckshop fuzz-smoke gate census

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

# Fails when a crate other than the dev-only exact check reaches polyglot at runtime.
polyglot-guard:
	uv run python -m scripts.check_polyglot_guard

# Rewrites cases/generated/ from the generators in scripts/case_generators/.
generate-cases:
	uv run python -m scripts.generate_cases

# Fails when a generated case file differs from what its generator writes.
generated-cases:
	uv run python -m scripts.generate_cases --check

# The static gate, the first step of `gate`.
check-ci: rust-lint python-check fensu evidence-guard polyglot-guard generated-cases

test:
	cargo test --workspace --locked
	uv run pytest $(PYTHON_TESTS) -n auto --dist loadfile

# Replays every recorded case on DuckDB; every rule and equivalence needs a passed case.
# Backup pushes may drop --require-verified; failed cases and error findings still block.
VERIFY_FLAGS ?= --require-verified --strict-errors

verify-recorded:
	cargo run --release --locked -p sqlquack -- verify $(VERIFY_FLAGS)

# The exact parser check over each compiled DuckShop project in DUCKSHOP_COMPILED.
polyglot-duckshop:
	@test -n "$(DUCKSHOP_COMPILED)" || { echo "polyglot-duckshop: set DUCKSHOP_COMPILED" >&2; exit 1; }
	for compiled in $(DUCKSHOP_COMPILED); do \
	  cargo run --locked --release -p sqlquack-polyglot-check -- directory "$$compiled" --dialect snowflake || exit 1; \
	done

# Fuzzes tolerant parsing from the exact check's statements for FUZZ_SECONDS.
fuzz-smoke:
	uv run python -m scripts.seed_fuzz_corpus --out $(FUZZ_CORPUS)
	cd fuzz && $(FUZZ_CARGO) run --target x86_64-unknown-linux-gnu tolerant $(abspath $(FUZZ_CORPUS)) -- \
	  -max_total_time=$(FUZZ_SECONDS) -max_len=4096 -timeout=10 -rss_limit_mb=2048

# Everything CI runs: the static gate, the tests, the recorded-case replay, the exact check on
# DuckShop and the fuzz smoke. The pre-push verify hook runs this.
gate: check-ci test verify-recorded polyglot-duckshop fuzz-smoke

# Example: make census CENSUS_ARGS="path/to/sql --json"
census:
	cargo run --release --locked -p sqlquack -- census $(CENSUS_ARGS)
