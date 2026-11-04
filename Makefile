SHELL := /bin/bash

# CI sets RECORDER=0: the Snowflake recorder is never installed, type-checked or tested there.
RECORDER ?= 1
# The revision recorded results are compared against by the append-only guard.
EVIDENCE_BASE ?= origin/main
# The DuckShop benchmarks and the SQLBuild release that compiles them, pinned together as in CI.
SQLBUILD_REF ?= a7bc60e0abee5bb19d67e5bdc948e048a1df82e0
SQLBUILD_VERSION ?= 0.187.2
DUCKSHOP_DIR ?= target/duckshop
# Compiled DuckShop projects the exact parser check reads; `make duckshop-compiled` builds them.
DUCKSHOP_COMPILED ?= $(DUCKSHOP_DIR)/sqlbuild/benchmarks/duckshop/project/target/compiled $(DUCKSHOP_DIR)/duckshop-10k/target/compiled
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

.PHONY: format rust-lint python-check fensu syntax-coverage evidence-guard polyglot-guard generate-cases generated-cases generate-referee-cases referee-cases referee-pending check-ci check-merge test test-rust test-python verify-recorded polyglot-duckshop census-duckshop fuzz-smoke gate gate-parallel census duckshop-compiled

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

# Fails on any unresolved coverage reference, a form or obligation the baseline claims and the
# current parser no longer supports, or a catalogue change the baseline was not re-recorded for.
syntax-coverage:
	cargo run --locked -p sqlquack-syntax-coverage -- diff coverage/syntax/baseline.json

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

# Rewrites referee/generated/ and referee/environments/ from scripts/referee_cases/.
generate-referee-cases:
	uv run python -m scripts.generate_referee_cases

# Fails when a generated referee file differs from what its generator writes.
referee-cases:
	uv run python -m scripts.generate_referee_cases --check

# Writes evidence/pending/referee/ for the recorder. Example:
# make referee-pending REFEREE_ARGS="--select 'generated/*.json' --catalogue"
referee-pending:
	uv run python -m scripts.write_referee_pending $(REFEREE_ARGS)

# The static gate, the first step of `gate`.
check-ci: rust-lint python-check fensu syntax-coverage evidence-guard polyglot-guard generated-cases referee-cases

# Rust tests build with the gate profile: release speed, debug assertions and overflow checks on.
# Its test binaries run RUST_TEST_JOBS at a time; any failure fails the target.
RUST_TEST_PROFILE ?= gate
RUST_TEST_JOBS ?= 4

test: test-rust test-python

test-rust:
	RUST_TEST_JOBS=$(RUST_TEST_JOBS) scripts/run_rust_tests.sh $(RUST_TEST_PROFILE)

# What every merge to main must pass, on the exact head: the static gate and the Rust tests.
check-merge: check-ci test-rust

test-python:
	uv run pytest $(PYTHON_TESTS) -n auto --dist loadfile

# Replays every recorded case on DuckDB; every rule and equivalence needs a passed case.
# Backup pushes may drop --require-verified; failed cases and error findings still block.
VERIFY_FLAGS ?= --require-verified --strict-errors
# Verify reports evidence coverage (docs/coverage.md) and writes it to COVERAGE_REPORT as JSON.
# REQUIRE_COVERAGE=1 fails on any gap; it stays 0 until the generated hazard cases are recorded.
COVERAGE_REPORT ?= target/coverage-report.json
REQUIRE_COVERAGE ?= 0
# The interpreter that replays cases calling Python functions; it needs the dev group's duckdb.
VERIFY_PYTHON ?= $(CURDIR)/.venv/bin/python
COVERAGE_FLAGS := --coverage-json $(COVERAGE_REPORT) $(if $(filter 1,$(REQUIRE_COVERAGE)),--require-coverage,)

verify-recorded:
	mkdir -p $(dir $(COVERAGE_REPORT))
	cargo run --release --locked -p sqlquack -- verify $(VERIFY_FLAGS) $(COVERAGE_FLAGS) --python $(VERIFY_PYTHON)

# Compiles DuckShop and DuckShop 10K at SQLBUILD_REF with sqlbuild SQLBUILD_VERSION, as CI does.
# Skipped when DUCKSHOP_DIR already holds that revision.
duckshop-compiled:
	@if [ "$$(cat $(DUCKSHOP_DIR)/.ref 2>/dev/null)" = "$(SQLBUILD_REF) $(SQLBUILD_VERSION)" ]; then \
	  echo "duckshop-compiled: $(DUCKSHOP_DIR) is current"; exit 0; fi; \
	set -e; rm -rf $(DUCKSHOP_DIR); mkdir -p $(DUCKSHOP_DIR); \
	git clone -q --filter=blob:none --no-checkout https://github.com/sqlstack-io/sqlbuild $(DUCKSHOP_DIR)/sqlbuild; \
	git -C $(DUCKSHOP_DIR)/sqlbuild sparse-checkout set benchmarks; \
	git -C $(DUCKSHOP_DIR)/sqlbuild checkout -q $(SQLBUILD_REF); \
	uv venv -q -p 3.12 $(DUCKSHOP_DIR)/venv; \
	VIRTUAL_ENV=$(abspath $(DUCKSHOP_DIR)/venv) uv pip install -q "sqlbuild==$(SQLBUILD_VERSION)"; \
	python3 $(DUCKSHOP_DIR)/sqlbuild/benchmarks/duckshop-10k/generate.py --out $(DUCKSHOP_DIR)/duckshop-10k; \
	(cd $(DUCKSHOP_DIR)/sqlbuild/benchmarks/duckshop/project && $(abspath $(DUCKSHOP_DIR)/venv/bin/sqb) compile); \
	(cd $(DUCKSHOP_DIR)/duckshop-10k && $(abspath $(DUCKSHOP_DIR)/venv/bin/sqb) compile); \
	echo "$(SQLBUILD_REF) $(SQLBUILD_VERSION)" > $(DUCKSHOP_DIR)/.ref

# The exact parser check over each compiled DuckShop project in DUCKSHOP_COMPILED.
polyglot-duckshop:
	@for compiled in $(DUCKSHOP_COMPILED); do test -d "$$compiled" || \
	  { echo "polyglot-duckshop: $$compiled is missing; run make duckshop-compiled" >&2; exit 1; }; done
	for compiled in $(DUCKSHOP_COMPILED); do \
	  cargo run --locked --release -p sqlquack-polyglot-check -- directory "$$compiled" --dialect snowflake || exit 1; \
	done

# Converting each compiled DuckShop project, with chained models, must finish within
# DUCKSHOP_CENSUS_SECONDS. DuckShop 10K takes about 25 s on a development machine; 300 s is about
# twelve times that, room for slower CI runners, so only a pathological slowdown fails the gate.
DUCKSHOP_CENSUS_SECONDS ?= 300

census-duckshop:
	@for compiled in $(DUCKSHOP_COMPILED); do test -d "$$compiled" || \
	  { echo "census-duckshop: $$compiled is missing; run make duckshop-compiled" >&2; exit 1; }; done
	cargo build --locked --release -p sqlquack
	for compiled in $(DUCKSHOP_COMPILED); do \
	  timeout $(DUCKSHOP_CENSUS_SECONDS) cargo run --locked --release -q -p sqlquack -- \
	    census --chain-models --json "$$compiled" > /dev/null \
	    || { echo "census-duckshop: converting $$compiled failed or took over $(DUCKSHOP_CENSUS_SECONDS) s" >&2; exit 1; }; \
	done

# Fuzzes tolerant parsing from the exact check's statements for FUZZ_SECONDS.
fuzz-smoke:
	uv run python -m scripts.seed_fuzz_corpus --out $(FUZZ_CORPUS)
	cd fuzz && $(FUZZ_CARGO) run --target x86_64-unknown-linux-gnu tolerant $(abspath $(FUZZ_CORPUS)) -- \
	  -max_total_time=$(FUZZ_SECONDS) -max_len=4096 -timeout=10 -rss_limit_mb=2048

# Everything CI runs: the static gate, the tests, the recorded-case replay, the exact check and the
# timed conversion on DuckShop, and the fuzz smoke. The pre-push verify hook runs this.
# After the static gate, the independent targets run in parallel, each target's output kept
# together; any failure fails the gate. The verify hook builds what they need beforehand. The fuzz
# smoke runs alone afterwards, so its time-boxed run gets the machine to itself.
gate: check-ci $(if $(filter command line,$(origin DUCKSHOP_COMPILED)),,duckshop-compiled)
	$(MAKE) -j --output-sync=target gate-parallel
	$(MAKE) fuzz-smoke

gate-parallel: test-rust test-python verify-recorded polyglot-duckshop census-duckshop

# Example: make census CENSUS_ARGS="path/to/sql --json"
census:
	cargo run --release --locked -p sqlquack -- census $(CENSUS_ARGS)
