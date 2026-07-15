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
confirm the parity test and the headless load path pass, and check 713ac855's
intent (the position change was presumably deliberate; the data file just
lagged). Steps for /plan when picked up.
