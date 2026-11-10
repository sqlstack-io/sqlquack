#!/usr/bin/env bash
# Makes Cargo download the DuckDB library again when a restored build cache lacks it.
#
# Usage: scripts/refetch_duckdb_library.sh; CARGO_TARGET_DIR, when set, is the target directory.
# libduckdb-sys' build script downloads libduckdb.so into <target>/duckdb-download, beside the
# profile directories, where a build cache such as Swatinem/rust-cache does not keep it. The cache
# does keep the build script's recorded output, so Cargo would not run it again, and the build links
# and copies a library that is not there. For each recorded run whose library is missing, this
# removes its fingerprint, so the next build runs the build script and downloads the library.
set -euo pipefail
target=${CARGO_TARGET_DIR:-target}
for output in "$target"/*/build/libduckdb-sys-*/output; do
  [ -f "$output" ] || continue
  library="$(sed -n 's/^cargo:rustc-link-search=native=\(.*duckdb-download.*\)$/\1/p' "$output" | head -1)"
  if [ -n "$library" ] && [ ! -f "$library/libduckdb.so" ]; then
    unit="$(dirname "$output")"
    echo "refetch DuckDB library: $(basename "$unit") recorded $library/libduckdb.so, which is missing"
    rm -rf "$(dirname "$(dirname "$unit")")/.fingerprint/$(basename "$unit")"
  fi
done
