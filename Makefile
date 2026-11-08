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
FUZZ_TARGETS ?= tolerant diagnose

# The Python module `make python-module` builds into its package, and the tests of its API.
PYTHON_PACKAGE := python/src/sqlquack
PYTHON_MODULE := $(PYTHON_PACKAGE)/_native.abi3.so
PYTHON_BUILD_JSON = $(or $(CARGO_TARGET_DIR),target)/python-module-build.json
PYTHON_API_TESTS := tests/unit/python

ifeq ($(RECORDER),1)
PYTHON_TYPED := .
PYTHON_TESTS := tests
else
PYTHON_TYPED := scripts tests/unit/scripts python/src $(PYTHON_API_TESTS)
PYTHON_TESTS := tests/unit/scripts $(PYTHON_API_TESTS)
endif

.PHONY: distribution-notices format rust-lint python-check fensu syntax-coverage depth-probe depth-probe-debug gate-build evidence-guard polyglot-guard engine-free generate-cases generated-cases generate-syntax-cases syntax-cases dialect-cases corpus-rerecord generate-referee-cases referee-cases referee-pending translation-results sql-sizes sql-size-baseline sql-benchmark check-ci check-merge test test-rust test-python python-module python-api python-wheel verify-recorded polyglot-duckshop census-duckshop fuzz-smoke gate gate-parallel census duckshop-compiled duckshop-measure

format:
	cargo fmt --all
	uv run ruff format .

# Library and binary code reports failures as values and never panics; tests may.
NO_PANIC_LINTS := -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic \
  -D clippy::unreachable -D clippy::todo -D clippy::unimplemented

# The interpreter the Python module builds against (.cargo/config.toml), before any cargo build.
.venv/bin/python:
	uv venv

rust-lint: | .venv/bin/python
	cargo fmt --all --check
	cargo clippy --workspace --all-targets --locked -- -D warnings
	cargo clippy --workspace --lib --bins --locked -- -D warnings $(NO_PANIC_LINTS)

python-check:
	uv run ruff format --check .
	uv run ruff check .
	uv run ty check $(PYTHON_TYPED)

fensu:
	uv run fensu check

# Fails on any unresolved coverage reference, a form or obligation the baseline claims and the
# current parser no longer supports, or a catalogue change the baseline was not re-recorded for.
syntax-coverage: gate-build
	$(GATE_DIR)/syntax-coverage diff coverage/syntax/baseline.json
	$(GATE_DIR)/syntax-coverage diff --root coverage/syntax-sqlite coverage/syntax-sqlite/baseline.json
	$(GATE_DIR)/syntax-coverage diff --root coverage/syntax-duckdb coverage/syntax-duckdb/baseline.json

# Fails when a native parser crashes, panics or hangs on deeply nested SQL on a 2 MiB thread and
# coverage/deep-nesting.toml does not list it, or when a listed failure no longer happens.
depth-probe: gate-build
	$(GATE_DIR)/syntax-coverage depth-probe --profile $(GATE_PROFILE)

# The same probe on a debug build, whose larger frames fail at shallower depths; not in check-ci.
depth-probe-debug:
	cargo build --locked -p sqlquack-syntax-coverage --bin syntax-coverage
	$(or $(CARGO_TARGET_DIR),target)/debug/syntax-coverage depth-probe --profile dev

# Fails when a change modifies or deletes a recorded result, or adds an invalid one.
evidence-guard:
	uv run python -m scripts.check_evidence_append_only --base $(EVIDENCE_BASE)

# Fails when a crate other than the dev-only exact check reaches polyglot at runtime.
polyglot-guard:
	uv run python -m scripts.check_polyglot_guard

# Fails when the conversion pipeline crate, which builds without DuckDB's engine, reaches it.
engine-free:
	@tree="$$(cargo tree --locked -p sqlquack-convert -e normal --prefix none)" || exit 1; \
	  if grep -E '^(duckdb|libduckdb-sys) ' <<<"$$tree"; then \
	    echo "sqlquack-convert reaches DuckDB's engine" >&2; exit 1; \
	  fi

# Rewrites cases/generated/ from the generators in scripts/case_generators/.
generate-cases:
	uv run python -m scripts.generate_cases

# Rewrites the outcomes recorded in corpora/everyday, everyday-models and basics from the current
# converter. Run it in the PR whose change moves an outcome, and commit the corpora with it.
corpus-rerecord:
	SQLQUACK_RERECORD=1 cargo test --locked -p sqlquack --lib everyday_corpus

# Fails when a generated case file differs from what its generator writes.
generated-cases:
	uv run python -m scripts.generate_cases --check

# The syntax-coverage families whose cases and marks scripts/build_syntax_cases.py writes.
SYNTAX_CASE_ARGS := --receipts coverage/syntax/receipts --held coverage/syntax/held.jsonl \
  classes ddl dml expressions functions query scripting security_admin transaction_session utility

# Rewrites coverage/syntax/cases, marks and arity from the obligations and the committed receipts.
generate-syntax-cases:
	uv run python -m scripts.build_syntax_cases $(SYNTAX_CASE_ARGS)

# Fails when a syntax case, mark or arity file differs from what the committed receipts regenerate.
syntax-cases:
	uv run python -m scripts.build_syntax_cases --check $(SYNTAX_CASE_ARGS)

# Dialect obligation builders whose --check rebuilds the committed obligations and cases into a
# temporary directory and fails on a missing or differing file, or on any file under obligations/ or
# cases/ that the build does not write. A dialect joins once its builder has --check.
DIALECT_CASE_BUILDERS ?= scripts.build_syntax_obligations_databricks

dialect-cases:
	@set -e; for m in $(DIALECT_CASE_BUILDERS); do uv run python -m $$m --check; done

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

# The static gate, the first step of `gate`, and the Python API's tests.
check-ci: rust-lint python-check fensu syntax-coverage depth-probe evidence-guard polyglot-guard engine-free generated-cases syntax-cases dialect-cases referee-cases python-api distribution-notices

# Each distribution ships the licence and third-party notices; its copies must match the root files.
distribution-notices:
	@for dir in python crates/sqlquack; do \
	  for file in LICENSE THIRD_PARTY_NOTICES; do \
	    cmp -s "$$file" "$$dir/$$file" || { echo "$$dir/$$file differs from $$file; copy it again"; exit 1; }; \
	  done; \
	done

# The gate builds everything it runs once, in `gate-build`: one profile, GATE_PROFILE (Cargo.toml's
# `gate`), and one feature set, GATE_FEATURES. Cargo unifies features per invocation, so a build of a
# single package would resolve them differently and compile the workspace again. The test binaries
# run RUST_TEST_JOBS at a time; any failure fails the target.
GATE_PROFILE ?= gate
GATE_FEATURES = sqlquack-python/extension-module
GATE_DIR = $(or $(CARGO_TARGET_DIR),target)/$(GATE_PROFILE)
RUST_TEST_PROFILE ?= $(GATE_PROFILE)
RUST_TEST_JOBS ?= 4

test: test-rust test-python

# Every workspace crate, binary and test target, and the Python module, for the steps after it.
gate-build: | .venv/bin/python
	cargo build --locked --profile $(GATE_PROFILE) --workspace --all-targets --features $(GATE_FEATURES) \
	  --message-format=json-render-diagnostics > $(PYTHON_BUILD_JSON)

test-rust: gate-build
	RUST_TEST_JOBS=$(RUST_TEST_JOBS) RUST_TEST_FEATURES=$(GATE_FEATURES) \
	  scripts/run_rust_tests.sh $(RUST_TEST_PROFILE)

# What every merge to main must pass, on the exact head: the static gate and the Rust tests.
check-merge: check-ci test-rust

test-python: python-api
	uv run pytest $(filter-out $(PYTHON_API_TESTS),$(PYTHON_TESTS)) --ignore=$(PYTHON_API_TESTS) \
	  -n auto --dist loadfile

# The `sqlquack` Python module, built from the workspace into its package, where the tests import it.
# The DuckDB library it links goes next to it, where its rpath looks.
python-module: gate-build
	cp $(GATE_DIR)/lib_native.so $(PYTHON_MODULE)
	cp "$$(jq -r 'select(.reason == "build-script-executed" and (.package_id | contains("libduckdb-sys"))) \
	  | .linked_paths[] | sub("^[a-z]*="; "")' $(PYTHON_BUILD_JSON) | head -1)/libduckdb.so" \
	  $(PYTHON_PACKAGE)/

# The API tests compare the module's reports with the command line's, both built with GATE_PROFILE.
python-api: python-module
	SQLQUACK_BIN=$(abspath $(GATE_DIR)/sqlquack) \
	  LD_LIBRARY_PATH=$(abspath $(PYTHON_PACKAGE)) uv run pytest $(PYTHON_API_TESTS) -n auto --dist loadfile

# A local wheel of the Python module in target/wheels; never published.
python-wheel: | .venv/bin/python
	cd python && uvx --from "maturin[patchelf]>=1.9,<2" maturin build --release --locked

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

# Verify also writes each recorded statement's translation outcome to TRANSLATION_STATEMENTS, which
# `syntax-coverage results` links to catalogue forms. It fails unless
# coverage/translation/results-baseline.json holds the measure exactly, and on any regression
# against the baseline at RESULTS_BASE, the target branch.
TRANSLATION_STATEMENTS ?= target/translation-statements.json
TRANSLATION_REPORT ?= target/translation-results.json
RESULTS_BASE ?= $(EVIDENCE_BASE)
RESULTS_BASE_FILE := target/translation-results-base.json

# Verify also writes the bytes of DuckDB SQL each converted case emits to SQL_SIZES, which must equal
# coverage/sql-size/baseline.json and must not grow against the baseline at RESULTS_BASE beyond what
# coverage/sql-size/allowances.toml rules.
SQL_SIZES ?= target/sql-sizes.json
SQL_SIZE_BASELINE := coverage/sql-size/baseline.json
SQL_SIZE_BASE_FILE := target/sql-size-base.json
SQL_SIZE_ALLOWANCES := coverage/sql-size/allowances.toml

verify-recorded:
	mkdir -p $(dir $(COVERAGE_REPORT))
	cargo run --release --locked -p sqlquack -- verify $(VERIFY_FLAGS) $(COVERAGE_FLAGS) --python $(VERIFY_PYTHON) \
	  --translation-statements $(TRANSLATION_STATEMENTS) --sql-sizes $(SQL_SIZES)
	$(MAKE) translation-results
	$(MAKE) sql-sizes

# The generated-SQL size ratchet over the sizes verify wrote; see verify-recorded.
sql-sizes:
	@if git show $(RESULTS_BASE):$(SQL_SIZE_BASELINE) > $(SQL_SIZE_BASE_FILE) 2>/dev/null; \
	  then against="--against $(SQL_SIZE_BASE_FILE)"; \
	  else echo "sql-sizes: $(RESULTS_BASE) has no baseline; checking the committed one only"; against=; fi; \
	  uv run python -m scripts.check_sql_sizes $(SQL_SIZES) --baseline $(SQL_SIZE_BASELINE) \
	    --allowances $(SQL_SIZE_ALLOWANCES) $$against

# Rewrites coverage/sql-size/baseline.json from the sizes the last verify-recorded wrote.
sql-size-baseline:
	uv run python -m scripts.check_sql_sizes $(SQL_SIZES) --baseline $(SQL_SIZE_BASELINE) --update

# Times the converted VARIANT and decimal division examples over a million rows against hand-written
# DuckDB SQL, and fails when one's time over its reference exceeds twice the ratio
# coverage/sql-size/benchmarks.json records. SQL_BENCHMARK_ARGS=--update records the ratios.
SQL_BENCHMARK_ARGS ?=
sql-benchmark:
	cargo build --locked --release -p sqlquack
	target="$${CARGO_TARGET_DIR:-target}"; \
	  library="$$(ls "$$target"/duckdb-download/*/*/libduckdb.so 2>/dev/null | head -1)"; \
	  test -n "$$library" || { echo "sql-benchmark: no libduckdb.so under $$target/duckdb-download" >&2; exit 1; }; \
	  LD_LIBRARY_PATH="$$(dirname "$$library")" uv run python -m scripts.benchmark_sql --sqlquack "$$target/release/sqlquack" \
	  --budget coverage/sql-size/benchmarks.json $(SQL_BENCHMARK_ARGS)

# The result-matching measure over the statements verify wrote; see verify-recorded.
translation-results:
	@if git show $(RESULTS_BASE):coverage/translation/results-baseline.json > $(RESULTS_BASE_FILE) 2>/dev/null; \
	  then against="--against $(RESULTS_BASE_FILE)"; \
	  else echo "translation-results: $(RESULTS_BASE) has no baseline; checking the committed one only"; against=; fi; \
	  cargo run --locked -q -p sqlquack-syntax-coverage --bin syntax-coverage -- results \
	    $(TRANSLATION_STATEMENTS) --report $(TRANSLATION_REPORT) $$against

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

# Measures DuckShop with this checkout's own binary and policy: its unit tests on DuckDB, and its
# models converted and run on DuckDB over Snowflake's recorded inputs, compared with Snowflake's
# rows. DUCKSHOP_RESULTS is the recorded results directory, with index-<target>.json beside it.
# Reports go to DUCKSHOP_MEASURE_DIR: tests.json and snowflake.json.
DUCKSHOP_MEASURE_DIR ?= target/duckshop-measure
duckshop-measure: duckshop-compiled
	@test -d "$(DUCKSHOP_RESULTS)" || \
	  { echo "duckshop-measure: set DUCKSHOP_RESULTS to the recorded results directory" >&2; exit 1; }
	cargo build --locked --release -p sqlquack
	rm -rf $(DUCKSHOP_MEASURE_DIR)
	mkdir -p $(DUCKSHOP_MEASURE_DIR)/converted $(DUCKSHOP_MEASURE_DIR)/guards
	set -e; target="$${CARGO_TARGET_DIR:-target}"; \
	library="$$(dirname "$$(ls "$$target"/duckdb-download/*/*/libduckdb.so | head -1)")"; \
	compiled="$(abspath $(word 1,$(DUCKSHOP_COMPILED)))"; out="$(DUCKSHOP_MEASURE_DIR)"; \
	uv run python -m scripts.measure_duckshop schema --environment referee/environments/duckshop.json \
	  --recorded "$(DUCKSHOP_RESULTS)" --functions "$$compiled/functions" --out "$$out/schema.json"; \
	SQLQUACK_CONVERTED_DIR="$$out/converted" SQLQUACK_GUARDS_DIR="$$out/guards" \
	  LD_LIBRARY_PATH="$$library" "$$target/release/sqlquack" census --policy sqlquack.toml --json \
	  --names --schema "$$out/schema.json" "$$compiled/models" > "$$out/census.json"; \
	uv run python -m scripts.measure_duckshop tests --compiled "$$compiled" \
	  --sqlquack "$$target/release/sqlquack" --library "$$library" --out "$$out/tests.json"; \
	uv run python -m scripts.measure_duckshop snowflake --results "$(DUCKSHOP_RESULTS)" \
	  --compiled "$$compiled" --census "$$out/census.json" --converted "$$out/converted" \
	  --guards "$$out/guards" --functions "$$compiled/functions" --schema "$$out/schema.json" \
	  --sqlquack "$$target/release/sqlquack" --library "$$library" \
	  --work "$$out/work" --out "$$out/snowflake.json"

# Fuzzes each of FUZZ_TARGETS from the exact check's statements for FUZZ_SECONDS: tolerant parsing,
# and strict parsing through to rendered diagnostics.
fuzz-smoke:
	uv run python -m scripts.seed_fuzz_corpus --out $(FUZZ_CORPUS)
	for target in $(FUZZ_TARGETS); do \
	  (cd fuzz && $(FUZZ_CARGO) run --target x86_64-unknown-linux-gnu $$target $(abspath $(FUZZ_CORPUS)) -- \
	    -max_total_time=$(FUZZ_SECONDS) -max_len=4096 -timeout=10 -rss_limit_mb=2048) || exit 1; \
	done

# Everything CI runs: the static gate, the tests, the recorded-case replay, the exact check and the
# timed conversion on DuckShop, and the fuzz smoke. The pre-push verify hook runs this.
# After the static gate, the independent targets run in parallel, each target's output kept
# together; any failure fails the gate. The verify hook builds what they need beforehand. The fuzz
# smoke and then the SQL timing benchmark run alone afterwards, so each gets the machine to itself.
gate: check-ci $(if $(filter command line,$(origin DUCKSHOP_COMPILED)),,duckshop-compiled)
	$(MAKE) -j --output-sync=target gate-parallel
	$(MAKE) fuzz-smoke
	$(MAKE) sql-benchmark

gate-parallel: test-rust test-python verify-recorded polyglot-duckshop census-duckshop

# Example: make census CENSUS_ARGS="path/to/sql --json"
census:
	cargo run --release --locked -p sqlquack -- census $(CENSUS_ARGS)
