#!/usr/bin/env bash
#
# Integration test for .githooks/pre-commit: proves the shipped hook refuses a
# commit that introduces rustfmt drift and accepts a clean one. Hermetic - it
# builds a throwaway git repo + tiny crate in a tempdir and runs the REAL hook
# there, so it never touches this checkout. cargo fmt --check does not compile,
# so this is fast and needs only the nightly toolchain rustfmt.
#
# Run: scripts/test-fmt-hook.sh   (exit 0 = pass, non-zero = fail)

set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
hook="$repo_root/.githooks/pre-commit"
[[ -f $hook ]] || { echo "FAIL: $hook missing"; exit 1; }

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# A minimal but real cargo crate so `cargo fmt` has a package to format.
mkdir -p "$tmp/src" "$tmp/.githooks"
cat > "$tmp/Cargo.toml" <<'EOF'
[package]
name = "fmt_hook_probe"
version = "0.0.0"
edition = "2021"

[[bin]]
name = "fmt_hook_probe"
path = "src/main.rs"
EOF
cp "$hook" "$tmp/.githooks/pre-commit"
chmod +x "$tmp/.githooks/pre-commit"

git -C "$tmp" init -q
git -C "$tmp" config user.email test@example.com
git -C "$tmp" config user.name "fmt hook test"
git -C "$tmp" config commit.gpgsign false
git -C "$tmp" config core.hooksPath .githooks

fail() { echo "FAIL: $1"; exit 1; }

# 1. Drift must be REFUSED. Deliberately misformatted (no spaces, jammed body).
printf 'fn main(){let x=1;println!("{}",x);}\n' > "$tmp/src/main.rs"
git -C "$tmp" add -A
if git -C "$tmp" commit -q -m "drift" > /dev/null 2>&1; then
    fail "hook accepted a misformatted commit (should have refused)"
fi
echo "ok: misformatted commit refused"

# 2. A clean tree must be ACCEPTED.
( cd "$tmp" && cargo fmt )
git -C "$tmp" add -A
if ! git -C "$tmp" commit -q -m "clean" > /dev/null 2>&1; then
    fail "hook refused a rustfmt-clean commit (should have accepted)"
fi
echo "ok: clean commit accepted"

# 3. A docs-only commit must NOT be gated (no .rs staged), even with the crate
#    left dirty - proves the .rs-staged trigger, not a whole-tree veto.
printf 'fn main(){let y=2;println!("{}",y);}\n' > "$tmp/src/main.rs"   # drift, unstaged
echo "hello" > "$tmp/README.md"
git -C "$tmp" add README.md
if ! git -C "$tmp" commit -q -m "docs only" > /dev/null 2>&1; then
    fail "hook gated a docs-only commit (no .rs staged; should skip)"
fi
echo "ok: docs-only commit not gated"

echo "PASS: fmt pre-commit hook guards drift"
