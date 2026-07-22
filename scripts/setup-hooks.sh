#!/usr/bin/env bash
#
# Enable this repo's tracked git hooks (.githooks/) by pointing core.hooksPath
# at them. Run once per fresh clone. Idempotent.
#
# core.hooksPath is local git config (not tracked), so it cannot ship in the
# repo itself; this one-liner is the opt-in. Git worktrees share the main
# repo's config, so running this once also arms the hook in every sprout
# worktree of this checkout.

set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

git config core.hooksPath .githooks
chmod +x .githooks/* 2> /dev/null || true

echo "core.hooksPath -> .githooks"
echo "Active hooks:"
for hook in .githooks/*; do
    [[ -f $hook ]] && echo "  - $(basename "$hook")"
done
