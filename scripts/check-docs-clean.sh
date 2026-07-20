#!/usr/bin/env bash
# Release guard for the ephemeral-docs model (task 20260718-175424): at a
# release tag, docs/ must hold NOTHING but LESSONS.md. During a development
# cycle docs/ is free scratch; before tagging you distill it into LESSONS.md
# (and the wiki) and run scripts/wipe-docs.sh. This check - run by the
# release-flow workflow - fails the release if any scratch remains, so a tag
# can never ship with a junk-drawer docs/.
#
# Exits 0 when clean, non-zero listing the offenders otherwise. Run from
# anywhere in the repo.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

# The only permanent files are the ledger and the model's README; anything
# else under docs/ is leftover scratch and blocks the release.
offenders=$(find docs -mindepth 1 ! -name LESSONS.md ! -name README.md 2>/dev/null || true)

if [ -n "$offenders" ]; then
    echo "docs/ is NOT clean for a release - only docs/LESSONS.md and docs/README.md may remain." >&2
    echo "Offending entries:" >&2
    echo "$offenders" | sed 's/^/  /' >&2
    echo >&2
    echo "Distill anything durable into docs/LESSONS.md (lessons) or the wiki" >&2
    echo "(reference), then run scripts/wipe-docs.sh, and re-tag." >&2
    exit 1
fi

echo "docs/ is clean (only LESSONS.md + README.md) - ok to release."
