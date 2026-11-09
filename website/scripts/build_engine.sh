#!/usr/bin/env bash
# Builds the playground's engine (website/wasm) into wasm/pkg/, which `npm run build` serves under
# public/playground/engine/<hash>/ (scripts/prepare_playground.mjs), and prints its size.
# Needs the wasm32-unknown-unknown target, wasm-bindgen-cli at the crate's wasm-bindgen version,
# and Node (for binaryen's wasm-opt). Run from website/.
set -euo pipefail
here="$(cd "$(dirname "$0")/.." && pwd)"
out="$here/wasm/pkg"
cd "$here/wasm"
cargo build --release --target wasm32-unknown-unknown
raw=target/wasm32-unknown-unknown/release/sqlquack_wasm.wasm
rm -rf "$out"
wasm-bindgen --target web --no-typescript --out-dir "$out" "$raw"
npx --yes -p binaryen@125 wasm-opt -Oz --strip-debug --strip-producers --enable-bulk-memory \
  --enable-nontrapping-float-to-int --enable-sign-ext --enable-mutable-globals \
  "$out/sqlquack_wasm_bg.wasm" -o "$out/sqlquack_wasm_bg.wasm"
wasm="$out/sqlquack_wasm_bg.wasm"
size() { numfmt --to=iec-i --suffix=B "$1"; }
echo "engine: cargo $(size "$(stat -c %s "$raw")"), shipped $(size "$(stat -c %s "$wasm")")," \
  "gzip -9 $(size "$(gzip -9c "$wasm" | wc -c)"), brotli -q 11 $(size "$(node -e 'const z=require("zlib");process.stdout.write(String(z.brotliCompressSync(require("fs").readFileSync(process.argv[1]),{params:{[z.constants.BROTLI_PARAM_QUALITY]:11}}).length))' "$wasm")")"
