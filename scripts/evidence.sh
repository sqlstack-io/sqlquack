#!/usr/bin/env bash
# Fetches the recorded evidence SQLQuack's checks read, from the evidence repository evidence.toml pins.
#
# Usage: scripts/evidence.sh path | refresh | clone | pin [REV]
#   path     Makes sure the snapshot of the commit evidence.toml pins is there and verified, then prints
#            its absolute path, and nothing else, on stdout. REFRESH=1 does what refresh does.
#            EVIDENCE=working prints the clone's unpinned working tree instead, to check recordings
#            before they are pinned.
#   refresh  Fetches the pinned commit, extracts and verifies its snapshot again, and prints its path.
#   clone    Clones the evidence repository, unless the clone is there.
#   pin      Fetches the clone, then writes REV's commit (default origin/main) and its content hash
#            into evidence.toml.
#
# The clone is $EVIDENCE_DIR (default .evidence/clone), cloned with --no-checkout from evidence.toml's
# repository, or from $EVIDENCE_REPOSITORY when set. Checks read only .evidence/pinned/<commit>/, the
# read-only files of the pinned commit, never the clone's branches or working tree. A snapshot is
# verified once against content_sha256, and .evidence/pinned/<commit>.verified records that; after
# that it is read without hashing or the network. content_sha256 is the SHA-256 of one
# `<sha256>  <path>` line per regular file, sorted bytewise by path: what
# `find . -type f -printf '%P\0' | LC_ALL=C sort -z | xargs -0 sha256sum | sha256sum` prints.
# SQLQUACK_ROOT (default this script's checkout) is the checkout whose evidence.toml and .evidence/
# are used. Messages go to stderr; exits 2 when the evidence cannot be had or verified.
set -euo pipefail

root=$(cd "${SQLQUACK_ROOT:-$(dirname "${BASH_SOURCE[0]}")/..}" && pwd)
pin_file=$root/evidence.toml
clone=${EVIDENCE_DIR:-$root/.evidence/clone}
case $clone in /*) ;; *) clone=$PWD/$clone ;; esac
pinned=$root/.evidence/pinned
# evidence.toml holds this commit until the first pin.
placeholder_commit=0000000000000000000000000000000000000000
# The extraction in progress, removed if the script stops before it is in place.
work=
trap '[ -z "$work" ] || discard "$work"' EXIT

say() { echo "evidence: $*" >&2; }
fail() {
  say "$@"
  exit 2
}
# A path as messages show it: relative to the checkout when inside it.
shown() { case $1 in "$root"/*) echo "${1#"$root"/}" ;; *) echo "$1" ;; esac }
# Removes a directory tree, read-only or not.
discard() {
  chmod -R u+w "$1" 2>/dev/null || true
  rm -rf "$1"
}

# The value of `key = "value"` in evidence.toml.
pin_value() {
  sed -n -E "s/^$1[[:space:]]*=[[:space:]]*\"([^\"]*)\"[[:space:]]*$/\1/p" "$pin_file" | head -n 1
}

read_pin() {
  [ -f "$pin_file" ] || fail "$(shown "$pin_file") is missing"
  repository=${EVIDENCE_REPOSITORY:-$(pin_value repository)}
  commit=$(pin_value commit)
  content_sha256=$(pin_value content_sha256)
  [ -n "$repository" ] || fail "evidence.toml names no repository"
  [[ $commit =~ ^[0-9a-f]{40}$ ]] || fail "evidence.toml's commit is not a 40-digit lower-case hex SHA"
  [[ $content_sha256 =~ ^[0-9a-f]{64}$ ]] ||
    fail "evidence.toml's content_sha256 is not a 64-digit lower-case hex SHA-256"
  snapshot=$pinned/$commit
  stamp=$pinned/$commit.verified
}

require_pinned() {
  [ "$commit" != "$placeholder_commit" ] ||
    fail "no evidence is pinned yet: evidence.toml holds the placeholder commit. Run 'make evidence-pin' to pin the evidence repository's main."
}

ensure_clone() {
  [ ! -e "$clone/.git" ] || return 0
  mkdir -p "$(dirname "$clone")"
  git clone --quiet --no-checkout "$repository" "$clone" 2>/dev/null ||
    fail "cloning $repository into $(shown "$clone") failed. Connect to the network with access to the evidence repository and try again, or set EVIDENCE_DIR to an existing clone."
}

has_commit() { git -C "$clone" cat-file -e "$commit^{commit}" 2>/dev/null; }

# Fetches the pinned commit by its SHA, or every branch when the server refuses that.
fetch_commit() {
  git -C "$clone" fetch --quiet origin "$commit" 2>/dev/null ||
    git -C "$clone" fetch --quiet origin 2>/dev/null
}

verified() { [ -d "$snapshot" ] && [ -f "$stamp" ] && [ "$(cat "$stamp")" = "$content_sha256" ]; }

content_hash() {
  (cd "$1" && find . -type f -printf '%P\0' | LC_ALL=C sort -z | xargs -0 -r sha256sum -- |
    sha256sum | cut -d ' ' -f 1)
}

# Extracts the files of $commit into $work, a new directory beside the snapshots.
extract() {
  mkdir -p "$pinned"
  work=$(mktemp -d "$pinned/.tmp-$commit.XXXXXX")
  chmod u=rwx,go=rx "$work"
  git -C "$clone" archive --format=tar "$commit" | tar -x -C "$work" ||
    fail "extracting commit ${commit:0:7} from $(shown "$clone") failed"
}

# Puts the verified extraction in $work in place of any older snapshot, read-only, and stamps it.
install_snapshot() {
  local old=$pinned/.old-$commit.$$
  chmod -R a-w "$work"
  rm -f "$stamp"
  if [ -e "$snapshot" ]; then mv -T "$snapshot" "$old"; fi
  # Another run that put the same commit in place first leaves this extraction unused.
  if mv -T "$work" "$snapshot" 2>/dev/null; then work=; fi
  [ ! -e "$old" ] || discard "$old"
  [ -d "$snapshot" ] || fail "could not put the snapshot of commit ${commit:0:7} in $(shown "$snapshot")"
  printf '%s\n' "$content_sha256" >"$stamp.$$"
  mv -f "$stamp.$$" "$stamp"
}

# Makes sure the pinned snapshot is in place and verified, fetching and extracting it when it is not,
# or always when $1 is 1, and prints its path.
ensure_pinned() {
  local actual
  read_pin
  require_pinned
  if [ "$1" = 0 ] && verified; then
    echo "$snapshot"
    return
  fi
  ensure_clone
  if [ "$1" = 1 ] || ! has_commit; then
    if ! fetch_commit; then
      has_commit ||
        fail "commit ${commit:0:7} (pinned in evidence.toml) is not in $(shown "$clone"), and fetching it failed. Connect to the network and run 'make evidence-refresh', or set EVIDENCE_DIR to a clone that contains it."
      say "fetching commit ${commit:0:7} failed; extracting it again from $(shown "$clone")"
    fi
    has_commit ||
      fail "commit ${commit:0:7} (pinned in evidence.toml) is not in the evidence repository $(git -C "$clone" remote get-url origin). Push it there, or pin a commit that is."
  fi
  extract
  actual=$(content_hash "$work")
  if [ "$actual" != "$content_sha256" ]; then
    discard "$work"
    work=
    fail "content hash mismatch for commit ${commit:0:7}: evidence.toml expects $content_sha256, but its files hash to $actual"
  fi
  install_snapshot
  echo "$snapshot"
}

# Pins the commit $1 names (default origin/main) and the content hash of its files in evidence.toml.
pin() {
  local rev=${1:-origin/main} updated=$pin_file.$$
  read_pin
  ensure_clone
  git -C "$clone" fetch --quiet origin 2>/dev/null ||
    fail "fetching $(shown "$clone") from origin failed. Connect to the network and try again."
  commit=$(git -C "$clone" rev-parse --verify --quiet "$rev^{commit}") ||
    fail "'$rev' names no commit in $(shown "$clone")"
  extract
  content_sha256=$(content_hash "$work")
  snapshot=$pinned/$commit
  stamp=$pinned/$commit.verified
  if verified; then
    discard "$work"
    work=
  else
    install_snapshot
  fi
  sed -E -e "s/^(commit[[:space:]]*=[[:space:]]*)\"[^\"]*\"/\1\"$commit\"/" \
    -e "s/^(content_sha256[[:space:]]*=[[:space:]]*)\"[^\"]*\"/\1\"$content_sha256\"/" \
    "$pin_file" >"$updated"
  mv -f "$updated" "$pin_file"
  if [ "$(pin_value commit)" != "$commit" ] || [ "$(pin_value content_sha256)" != "$content_sha256" ]; then
    fail "could not write the pin into evidence.toml: it needs commit and content_sha256 lines"
  fi
  say "pinned commit $commit, content_sha256 $content_sha256, in evidence.toml"
}

case ${1:-} in
path)
  case ${EVIDENCE:-} in
  working)
    read_pin
    ensure_clone
    say "EVIDENCE=working: reading the unpinned working tree of $(shown "$clone"), not the commit evidence.toml pins"
    echo "$clone"
    ;;
  '') if [ "${REFRESH:-}" = 1 ]; then ensure_pinned 1; else ensure_pinned 0; fi ;;
  *) fail "EVIDENCE is '$EVIDENCE': set it to 'working' to read the clone's working tree, or unset it" ;;
  esac
  ;;
refresh) ensure_pinned 1 ;;
clone)
  read_pin
  ensure_clone
  ;;
pin) pin "${2:-}" ;;
*)
  echo "usage: scripts/evidence.sh path | refresh | clone | pin [REV]" >&2
  exit 2
  ;;
esac
