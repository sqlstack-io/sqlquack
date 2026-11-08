#!/usr/bin/env bash
# Runs every workspace test binary built with one Cargo profile, several at once.
#
# Usage: scripts/run_rust_tests.sh <profile>; RUST_TEST_FEATURES, when set, is Cargo's --features.
# RUST_TEST_JOBS (default 4) binaries run at a time, the slowest first. Each runs from its package
# directory with the native library paths of the build scripts, as `cargo test` runs it; its whole
# output is printed when it finishes, and any failing binary fails the run after every binary has
# finished. Doc tests follow, then the tests marked `#[ignore = "timing"]`, one at a time.
set -euo pipefail
profile=${1:?usage: run_rust_tests.sh <profile>}
jobs=${RUST_TEST_JOBS:-4}
# The features the binaries build with: the same set as the build they reuse, or Cargo rebuilds.
features=()
if [ -n "${RUST_TEST_FEATURES:-}" ]; then features=(--features "$RUST_TEST_FEATURES"); fi
# The fewest test binaries the workspace builds; fewer means tests went missing, not that they pass.
minimum=${RUST_TEST_MIN_BINARIES:-21}
# Started first, so the longest binaries overlap the rest.
slowest="cli runs index"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

cargo test --workspace --locked --profile "$profile" "${features[@]}" --no-run \
  --message-format=json-render-diagnostics >"$work/messages.json"
library_path=$(jq -r 'select(.reason == "build-script-executed") | .linked_paths[]' \
  "$work/messages.json" | sed 's/^[a-z]*=//' | sort -u | paste -sd: -)
# Cargo also adds the profile's deps directory and the toolchain's libraries, which proc-macro
# crates' test binaries link against.
deps_directory=$(dirname "$(jq -r 'select(.reason == "compiler-artifact" and .executable != null)
  | .executable' "$work/messages.json" | head -1)")
library_path="$library_path:$deps_directory:$(rustc --print target-libdir)"
export LD_LIBRARY_PATH="$library_path${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
jq -r 'select(.reason == "compiler-artifact" and .profile.test and .executable != null)
  | [.target.name, .executable, (.manifest_path | sub("/[^/]*$"; ""))] | @tsv' \
  "$work/messages.json" | sort -u >"$work/binaries"
found=$(wc -l <"$work/binaries")
if [ "$found" -lt "$minimum" ]; then
  echo "run_rust_tests: FAILED: found $found test binaries, fewer than the $minimum expected" >&2
  exit 1
fi
for name in $slowest; do
  awk -F'\t' -v name="$name" '$1 == name { found = 1 } END { exit !found }' "$work/binaries" || {
    echo "run_rust_tests: FAILED: the $name test binary is missing" >&2
    exit 1
  }
done
{
  for name in $slowest; do awk -F'\t' -v name="$name" '$1 == name' "$work/binaries"; done
  awk -F'\t' -v slowest=" $slowest " 'index(slowest, " " $1 " ") == 0' "$work/binaries"
} | awk '{ print NR "\t" $0 }' >"$work/ordered"
echo "run_rust_tests: $(wc -l <"$work/ordered") test binaries, $jobs at a time"

export work
# shellcheck disable=SC2016
xargs -P "$jobs" -d '\n' -n 1 sh -c '
  IFS="$(printf "\t")" read -r number name executable directory <<EOF
$1
EOF
  started=$(date +%s)
  if (cd "$directory" && "$executable" -q) >"$work/$number.log" 2>&1; then
    status=0
  else
    status=$?
  fi
  echo "$status $(( $(date +%s) - started ))" >"$work/$number.status"
  echo "run_rust_tests: $name finished in $(( $(date +%s) - started )) s (exit $status)"
' run_one <"$work/ordered" || true

failed=0
while IFS="$(printf '\t')" read -r number name executable _; do
  echo "===== $name ($executable)"
  cat "$work/$number.log"
  read -r status _ <"$work/$number.status" 2>/dev/null || status=missing
  if [ "$status" != 0 ]; then
    echo "run_rust_tests: FAILED $name (exit $status)"
    failed=1
  fi
done <"$work/ordered"
cargo test --workspace --locked --profile "$profile" "${features[@]}" --doc || failed=1

# Tests that hold a wall-clock budget, marked `#[ignore = "timing"]`, run last, one at a time and
# single-threaded, so no other test shares the machine with them.
{ grep -rh -A4 --include='*.rs' '#\[ignore = "timing"\]' crates || true; } \
  | sed -n 's/^fn \([a-z0-9_]*\)(.*/\1/p' | sort -u >"$work/timing"
marks=$({ grep -rh --include='*.rs' '#\[ignore = "timing"\]' crates || true; } | wc -l)
if [ "$marks" -ne "$(wc -l <"$work/timing")" ]; then
  echo "run_rust_tests: FAILED: $marks timing marks name $(wc -l <"$work/timing") distinct test functions"
  failed=1
fi
: >"$work/timing-found"
while IFS="$(printf '\t')" read -r _ name executable directory; do
  (cd "$directory" && "$executable" --list --ignored) | sed -n 's/: test$//p' \
    | while read -r path; do
      grep -qxF "${path##*::}" "$work/timing" || continue
      echo "${path##*::}" >>"$work/timing-found"
      started=$(date +%s)
      if (cd "$directory" && "$executable" --exact "$path" --ignored --test-threads=1 -q); then
        echo "run_rust_tests: timing $name $path passed in $(( $(date +%s) - started )) s"
      else
        echo "run_rust_tests: FAILED timing $name $path"
        echo failed >>"$work/timing-failed"
      fi
    done
done <"$work/ordered"
[ -e "$work/timing-failed" ] && failed=1
missing=$(sort -u "$work/timing-found" | comm -23 "$work/timing" -)
if [ -n "$missing" ]; then
  echo "run_rust_tests: FAILED: timing tests not found in any binary: $missing"
  failed=1
fi
echo "run_rust_tests: $(wc -l <"$work/timing") timing tests ran one at a time"
[ "$failed" = 0 ] || { echo "run_rust_tests: FAILED" >&2; exit 1; }
echo "run_rust_tests: passed"
