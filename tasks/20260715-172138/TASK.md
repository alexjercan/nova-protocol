# Fix content_ron_parity drift: regenerate shakedown_run.content.ron after 713ac855 changed the builder

- STATUS: OPEN
- PRIORITY: 16
- TAGS: bug,modding

Found by the 142906 implementation (20260715): `cargo test -p nova_assets
--test content_ron_parity` fails on `built_in_scenario_content_matches_
committed_ron` - PRE-EXISTING on local master (A/B verified via stash on the
clean branch head): commit 713ac855 changed shakedown crate positions in the
scenario BUILDER without regenerating the committed
`assets/base/scenarios/shakedown_run.content.ron`. Local master will go red on
the next push unless this lands first.

Goal: re-align the committed RON with the builder - run the parity generator's
regeneration path (the generate-data-from-code lesson: serialize the builder
output, never hand-edit), diff-review the regenerated file (positions only),
confirm the parity test and the headless load path pass.

## Plan (20260715)

Verified: 713ac855 ("per-crate salvage pickup cue + wider Shakedown spacing")
deliberately changed the shakedown builder's crate positions; the committed
`assets/base/scenarios/shakedown_run.content.ron` lagged. The parity test
regenerates on missing (content_ron_parity.rs:51, write-on-missing; its own
failure message says "delete the file and re-run").

Steps:
- [ ] 1. Delete assets/base/scenarios/shakedown_run.content.ron; run
  `cargo test -p nova_assets --test content_ron_parity` (regenerates + passes).
- [ ] 2. Diff-review the regenerated file: position-only changes expected
  (wider crate spacing), no structural drift.
- [ ] 3. Verify the load path: `cargo test -p nova_assets --test demo_scenario`
  (loads the real base bundle recursively). fmt no-op (data file).
- [ ] 4. No CHANGELOG (713ac855 already owns the user-visible change; this
  realigns data). Close-out notes the root cause for the retro.
