#!/usr/bin/env bash
# Compile-and-wipe for the ephemeral-docs model (task 20260718-175424).
#
# The model: docs/ is FREE SCRATCH during a development cycle - write whatever
# notes you like, no structure required. The only DURABLE outputs are
# docs/LESSONS.md (the lessons ledger) and the wiki (web/src/wiki/); docs/ also
# keeps a permanent README.md describing this model. At release time the scratch
# is compiled into the ledger (reference detail into the wiki) and the folder is
# wiped, so docs/ holds only LESSONS.md + README.md at every tag.
#
# The "compile" is an AGENT step, not this script: a human/agent reads docs/,
# distills anything worth keeping into LESSONS.md entries (the /compound format:
# slug, one-two sentences, task ids) - reference-grade detail goes to a wiki
# dev page instead - because a script cannot summarize free-form notes into good
# lessons. THEN run this to clear the scratch.
#
# This step removes everything under docs/ except LESSONS.md + README.md. Idempotent: a
# no-op on an already-clean docs/. Run from anywhere in the repo.
#
#     scripts/wipe-docs.sh
#
# The release guard (scripts/check-docs-clean.sh, run by release-flow CI) fails
# a tag build if docs/ still holds anything else.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

if [ ! -f docs/LESSONS.md ]; then
    echo "wipe-docs: docs/LESSONS.md not found - refusing to wipe (is this the repo root?)." >&2
    exit 1
fi

# The two PERMANENT files are the ledger and this model's own README; all
# other entries under docs/ are ephemeral scratch and get cleared.
removed=0
while IFS= read -r -d '' entry; do
    rm -rf "$entry"
    echo "  removed $entry"
    removed=$((removed + 1))
done < <(find docs -mindepth 1 -maxdepth 1 ! -name LESSONS.md ! -name README.md -print0)

if [ "$removed" -eq 0 ]; then
    echo "wipe-docs: docs/ already clean (only LESSONS.md + README.md); nothing to do."
else
    echo "wipe-docs: cleared $removed entr(ies); docs/ now holds only LESSONS.md + README.md."
fi
