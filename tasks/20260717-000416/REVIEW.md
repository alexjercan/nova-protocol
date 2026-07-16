# Review: Base as a first-class implicit dep://base target (Option A, mechanism)

- TASK: 20260717-000416
- BRANCH: feat/dep-base-mechanism

Proportional review: this is a small, contained INVERSION of the already-reviewed
`dep://` gate (task 20260716-215423) - flip `base` from rejected to the implicit
universal `dep://base` target - not a substantial new subsystem. I re-verified
the load-bearing claim (cross-domain consistency + no silent mis-resolve) by
reading each domain's change and confirming the end-to-end test exercises the
real `register_bundles` path, not just `mod_refs`.

## Round 1

- VERDICT: APPROVE

Verified:

- **Cross-domain consistency.** `base` is exempted from the declared-dep check in
  all four places it is gated: `mod_refs::rewrite_leaf` (`id != "base" && !contains`),
  `mod_refs::violation` (same), and it is supplied in the `deps` map by BOTH
  callers - `register_bundles` (from `by_id["base"]`, base is always the enabled
  `base: true` entry) and `lint_walk::lint_bundle` (from the walked set). The
  portal `build_entry` exempts `base` from its local declared check. No domain
  is left half-migrated.
- **Membership still enforced.** `dep://base/X` is not a blanket pass: base goes
  through the same `deps.get -> resources` membership check, so an undeclared base
  resource is a violation (pinned in `mod_refs`, `lint_walk`, and the integration
  suite). Only the "must be declared in meta.dependencies" half is waived - base
  is the implicit universal dep by design.
- **No silent mis-resolve.** base absent from `deps` (not loaded) -> "not
  available" (violation) + literal ref, never a wrong rewrite. base present ->
  rewrites to `base/X` (its `resource_base`), proven end-to-end by
  `a_dep_ref_to_base_resolves_against_base_folder_without_declaring_base` which
  runs the real `register_bundles_for_test` with a synthetic base catalog entry
  and asserts `base/textures/cubemap.png` + zero content issues.
- **No regression.** No repo content uses `dep://base` yet, so the gate change is
  inert over the tree - `content_lint_gate` (repo tree clean) still passes. Real
  resolution against moved base art is task 20260717-002105.
- **Portal shipped-dep gap.** base membership is not checked at the portal (base
  is shipped; only its id is known) - correctly the same gap as any shipped dep,
  documented in the code comment, backstopped by the repo lint + runtime gate.
- **Decision record.** SPIKE.md updated (Option A chosen; the "declare base as a
  dependency" con corrected - base stays implicit).

Suites green: nova_assets lib 64, mod_binary_resources 7, content_lint_gate 2,
nova_portal_gen generate 21. The flipped tests (base was "rejected", now
"resolves"/"implicit") assert the new behavior and would fail under the old code.
No findings.
