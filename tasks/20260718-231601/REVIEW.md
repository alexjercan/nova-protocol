# Review: document modding meta-conventions

- TASK: 20260718-231601
- BRANCH: docs/modding-meta-conventions (landed f77e8bbe)

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

Round-1 findings from a fresh reviewer with no sight of the implementing
session. The reviewer verified all five documented "why" claims against the
cited code (file:line for each) and re-ran the CI gate; the in-session pass
re-confirmed the load-bearing version-pin claim independently
(`crates/nova_assets/tests/gauntlet_course.rs:540` asserts `version: "1.2.0"`,
matching the doc and the shipped gauntlet bundle).

Verified against code:
1. Version = opaque non-empty string; `bundle_ships_the_bumped_version` pins
   1.2.0; Gauntlet 1.2.0 / Ledger 1.5.0 correct.
2. Merge order (catalog/base-first -> downloaded -> topological with stable
   tiebreak) matches `nova_assets::register_bundles` +
   `nova_mod_format::deps::topological_order`; the demo-drop-is-breaking story
   matches the bundle comment + git history (drop landed at v1.1.0).
3. `.meta` exemption: `example.bundle.ron` lists `nebula.png` but not
   `nebula.png.meta`, both ship.
4. Publish-vs-load split: `gen-portal.py` never deserializes content; the
   pre-publish check `content lint --target <mod>` (ref + balance + input-
   overlap in one pass, task 152240) with `--report` is stated correctly.
5. Bundle-filename rationale was already documented accurately (lib.rs:~208
   full-extension reason); "no change" is the right call.

CI gate: `cd web && npm run ci` GREEN (prettier clean, eslint clean, webpack
compiled). DoD "publish my second version" walkthrough is followable from the
wiki. Honesty: close-notes correctly flag that the gauntlet bundle COMMENT still
says "2.0" while the shipped version is 1.2.0, and the doc was written to the
real versions.

- No BLOCKER/MAJOR/MINOR/NIT findings. A correct, complete, ci-green docs diff.

(Recorded on master post-land: REVIEW.md was authored but not committed on the
branch before `sprout land`, so it did not ride the squash - added here instead.
Process note in RETRO.md.)
