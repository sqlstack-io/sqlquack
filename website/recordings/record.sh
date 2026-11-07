#!/usr/bin/env bash
# Record one demo to out/<demo>.gif and out/<demo>.png. Usage: record.sh <demo>
# Uses target/release/sqlquack; set SQLQUACK_BIN to a directory containing another sqlquack to use that.
set -euo pipefail
demo="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
bin="${SQLQUACK_BIN:-$repo/target/release}"
export PATH="$bin:$PATH"
# The binary links the DuckDB shared library the build downloads beside its profile directory.
lib="$(dirname "$(find "$bin/../duckdb-download" -name 'libduckdb.*' | sort | tail -1)")"
export LD_LIBRARY_PATH="$lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export DYLD_LIBRARY_PATH="$lib${DYLD_LIBRARY_PATH:+:$DYLD_LIBRARY_PATH}"
"$here/prepare.sh" "$demo"
mkdir -p "$here/out"
cd "$here"
# Headless Chromium's sandbox is unavailable in many containers and CI runners.
VHS_NO_SANDBOX="${VHS_NO_SANDBOX:-true}" vhs "tapes/$demo.tape"
