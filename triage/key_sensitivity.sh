#!/usr/bin/env bash
# What changes the model cache's key (SOLIDCRAFT_EVALUATION_SOURCE)?
# Each case writes a changed file into a temporary tree (the branch is not touched) and
# prints the digest the key would have with that change committed.
set -u
ROOT=/home/user/solidcraft-wt/issue-14310
cd "$ROOT" || exit 1

digest_at() { python3 /tmp/opencode/eval_source.py "$ROOT" "$1"; }

commit_with() { # file content-file -> commit id
  local path="$1" content="$2" idx commit
  idx=$(mktemp)
  rm -f "$idx"
  GIT_INDEX_FILE="$idx" git read-tree HEAD
  blob=$(git hash-object -w "$content")
  GIT_INDEX_FILE="$idx" git update-index --add --cacheinfo "100644,$blob,$path"
  tree=$(GIT_INDEX_FILE="$idx" git write-tree)
  commit=$(git commit-tree "$tree" -p HEAD -m "probe")
  rm -f "$idx"
  echo "$commit"
}

BASE=$(digest_at HEAD)
echo "base digest (HEAD):            $BASE"

TMP=$(mktemp -d)
echo "a doc in the kernel's tree" > "$TMP/doc.md"
C=$(commit_with "solidcore/docs/booleans.md" "$TMP/doc.md")
echo "solidcore/docs/booleans.md:   $(digest_at "$C")"

echo "a CI file in the kernel's tree" > "$TMP/ci.yml"
C=$(commit_with "solidcore/.github/workflows/ci.yml" "$TMP/ci.yml")
echo "solidcore/.github/...:         $(digest_at "$C")"

echo "a test in the doc crate" > "$TMP/test.rs"
C=$(commit_with "crates/doc/src/eval_cost_tests.rs" "$TMP/test.rs")
echo "crates/doc/src/eval_cost...:  $(digest_at "$C")"

echo "the kernel's own source" > "$TMP/ops.rs"
C=$(commit_with "solidcore/crates/solidcore-ops/src/lib.rs" "$TMP/ops.rs")
echo "solidcore-ops/src/lib.rs:      $(digest_at "$C")"

echo "the app's UI crate" > "$TMP/ui.rs"
C=$(commit_with "crates/ui-egui/src/lib.rs" "$TMP/ui.rs")
echo "crates/ui-egui/src/lib.rs:     $(digest_at "$C")"

rm -rf "$TMP"
