# Review: Canonical enforcement - lint rejects bare asset refs in content

- TASK: 20260717-002133
- BRANCH: feat/bare-ref-lint

Proportional review of a contained static-lint addition. The reviewable substance
is the DESIGN DECISION (why an extension heuristic rather than a type-level ban)
and whether the heuristic is sound in both directions. I re-derived the design
constraint and verified both false-positive and false-negative behavior.

## Round 1

- VERDICT: APPROVE

### The design decision (verified sound)

A hard, type-level bare-ban is architecturally unavailable here, for two reasons
that both hold up:
1. The generic content walk (serde_json::Value) cannot distinguish an `AssetRef`
   string from a `SectionId`, message, or name - all are JSON strings. So the walk
   can't say "this string is an asset ref that must be schemed".
2. `AssetRef::deserialize` can't reject scheme-less strings, because the
   merge-rewrite (`rewrite_refs`) serializes Content -> Value, rewrites `self://X`
   -> `base/X` (a bare RESOLVED path), and deserializes BACK via `from_value` -
   which goes through `AssetRef::deserialize`. A strict deserialize would break the
   rewrite round-trip. Confirmed by reading `rewrite_refs`.

So the HARD guarantee is structural and already in place from task 002105: base
art moved under `assets/base/`, so a bare `textures/x.png` resolves against the
default source to a nonexistent path and 404s at load. This lint is the
AUTHOR-TIME catch, using an asset-extension heuristic. That framing is honest and
correct - it is not claiming a guarantee it can't provide.

### Heuristic soundness (verified both directions)

- **Catches real bare refs**: a scheme-less string ending in a binary-asset
  extension (`.png`/`.glb`/...) is flagged, `#label` preserved. Pinned by
  `a_bare_asset_ref_is_flagged...`, `a_bare_labeled_model_ref_is_flagged`
  (mod_refs), `a_bare_asset_ref_is_a_lint_error` (lint_walk), and
  `bare_asset_ref_in_content_is_rejected` (portal). All would fail if the detector
  were reverted.
- **No false positives**: ids (`shakedown_run`), names, and messages - including a
  slashed message (`and/or`) - are NOT flagged, because none end in an asset
  extension. Pinned by `non_asset_strings_are_not_flagged`. `self://`/`dep://`
  refs are excluded by prefix.
- **Residual false-negative** (accepted, documented): an asset ref with NO
  extension would slip the heuristic - but no such ref exists (all game art has an
  extension), and the structural 404 backstops any that ever did. Honest limit,
  not a hole.

### Consistency + coverage

- Both static domains mirror the check: `lint_walk` (typed, via
  `mod_refs::bare_asset_refs`) and the portal generator (engine-free `ron::Value`,
  `collect_bare_refs`), with a shared `ASSET_EXTENSIONS` list duplicated by
  necessity (portal must not pull in bevy) - the same deliberate mirror pattern as
  `collect_self_refs`/`collect_dep_refs`. No runtime gate, per the user decision.
- The two synthetic fixtures that carried an incidental bare ref
  (`content_lint_gate.rs`, `portal_install.rs`) were made canonical
  (`dep://base/...`), and one lint_walk fixture likewise - so they test their
  intended thing without tripping the new gate.
- Repo tree verified scheme-clean (no `*.content.ron` has a bare asset-ext ref),
  so `content_lint_gate`'s repo-tree test stays green.

Suites green (run as targeted builds - the full nova_assets suite build was being
killed by transient resource pressure): nova_assets lib 68 (incl. 4 new
bare-ref tests), content_lint_gate 2, portal_install 9, nova_portal_gen generate
22. Unaffected suites (mod_binary_resources, example_scenario, content_ron_parity)
don't exercise the lint and use schemed/synthetic content. No findings.
