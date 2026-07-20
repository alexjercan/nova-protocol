# Review: ephemeral docs/ model (compile-to-LESSONS + wipe + release guard)

- TASK: 20260718-175424
- BRANCH: docs/ephemeral-docs

## Round 1

- VERDICT: APPROVE

A large, multi-part process/tooling change, executed against the user's two
decisions (agent-distill model; full wipe now). Verified each load-bearing
piece rather than trusting the plan:

- **The mechanism works and is idempotent.** `scripts/wipe-docs.sh` cleared the
  2 scratch dirs and a re-run was a no-op (ran it). `scripts/check-docs-clean.sh`
  exits 0 on the clean docs/ and 1 on an injected junk file, listing offenders
  (ran both). `release.yaml` has a `guard-docs` job and `get-version needs:
  guard-docs`, so every build waits on it - a dirty docs/ blocks the tag
  (structure re-read; indentation matches the sibling jobs).
- **The DoD's internal tension is resolved, not ignored.** "docs/README.md
  describes the model" vs "only LESSONS.md survives" can't both hold literally,
  so docs/ permanently keeps exactly TWO meta files (LESSONS.md + README.md) and
  the wipe/guard allow both. Named explicitly so it's a decision, not a fudge.
- **No durable content was lost in the wipe.** The 9 design docs were assessed by
  an out-of-context pass and I spot-verified: 6 were already-in-wiki/superseded
  (deleted safely), 2 (craft-ships) had a genuine gap migrated to
  guide-author-section.md (verified `SectionSource::Inline/Prototype` in
  spaceship.rs before writing), 1 distilled to the `asset-meta-always-web-cost`
  lesson. The live v0.8.0 plan folded into the 20260720-142428 tracker task
  (git records it as a rename, so the content demonstrably moved, not vanished);
  old plans + the already-APPLIED sdlc-suggestions rely on git history.
- **The relocation swept its references.** After deleting docs/design + docs/plans
  I grepped the whole repo and redirected every LIVE dangling pointer (AGENTS.md,
  development.md, keeping-docs-in-sync.md, Trunk.toml, nova_meta_gen x3,
  mod_refs.rs, the example mod README) to the wiki/lesson; a final grep is clean
  outside frozen tasks/* history. This is the `sweep-then-delete` lesson applied.
- **npm run ci green** - the three wiki pages render (webpack build ok). The
  code-comment redirects are plain-text backticks, not intra-doc `[links]`, so
  `cargo doc`'s warning-free state is preserved (a broken `[link]` would warn;
  these are not links).
- **Absorbed 20260718-152225** (the CI-guard task) - built here, closed there
  with a pointer.

- [ ] R1.1 (NIT) The `guard-docs` step NAME in release.yaml reads "docs/ must
  hold only LESSONS.md" though the guard also allows README.md; the script's own
  message is accurate. Cosmetic step label. Take it or leave it.
- [ ] R1.2 (NIT) v0.4-v0.7 release plans were deleted relying on git history as
  the archive (their process lessons are already distilled into LESSONS.md via
  the per-task retros). A deliberate call under "full wipe now"; noted in case a
  future reader expects a plans archive in-tree.
