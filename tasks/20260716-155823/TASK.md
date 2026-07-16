# Explicit content generator bin; make content_ron_parity assert-only

- STATUS: CLOSED
- PRIORITY: 90
- TAGS: refactor, testing

## Goal

Keep authoring base scenarios/sections as .rs builders, but stop
generating the committed RON as a test side effect. An explicit bin
writes the files; the parity test only asserts. Decision from audit task
20260716-141620 (FINDINGS.md section 1), user confirmed.

## Steps

- [x] Extract shared helpers into `nova_assets::scenario_generation`
      (crates/nova_assets/src/lib.rs:48-115): a `serialize_content` fn
      (pretty_config + trailing newline, currently duplicated in the
      test) and a fn returning the full file map
      `Vec<(relative_path, contents)>` covering
      `base/sections/base.content.ron` + every builder-backed
      `base/scenarios/<id>.content.ron`, so bin and test cannot diverge.
- [x] Add `crates/nova_assets/src/bin/gen_content.rs`: writes every file
      in the map under the workspace `assets/` dir
      (env!("CARGO_MANIFEST_DIR")/../../assets, same resolution as the
      parity test), creating parent dirs, printing what it wrote. Run as
      `cargo run -p nova_assets --bin gen_content`.
- [x] Rewrite `crates/nova_assets/tests/content_ron_parity.rs`
      assert-only: a MISSING file is a failure (today it silently writes
      and passes), drift is a failure; both failure messages name the
      exact regen command.
- [x] Prove generator stability (verify-generator-stability lesson): run
      the bin twice, `git diff --exit-code` after each - byte-identical
      and matching the committed files.
- [x] Update prose: the scenario_generation module doc (lib.rs:40-42 says
      it exists "for the content_ron_parity test"), the parity test
      header, and grep docs/ + web/src/wiki for the old "delete the file
      and re-run the test" regen instruction.
- [x] Verify: `cargo check --workspace --all-targets`, `cargo fmt`, run
      content_ron_parity (full suite is CI's job).

## Notes

- Builders remain the single source; the game still loads only the RON.
- Bin file named gen_content.rs (underscore) so no [[bin]] table needed.
- The RON serialization is deterministic by construction (fixed
  PrettyConfig, no maps in output order?) - the double-run check is the
  proof, do not skip it.
- generate-data-from-code corollary applies from here on: any builder
  change regenerates the RON in the SAME commit via the bin.
- Review finding R1.2 from 20260716-155816: also assert that
  base.bundle.ron's content list equals the generated file map exactly,
  so "every base content file is builder-backed" is enforced, not just
  currently true.

## Close notes (2026-07-16)

What changed: scenario_generation gained `serialize_content` and
`content_files()` (the single (path, body) map); the new
`crates/nova_assets/src/bin/gen_content.rs` walks that map and writes
`assets/base/**/*.content.ron`; `content_ron_parity` was rewritten
assert-only with two tests - `committed_content_matches_builders`
(missing = failure, drift = failure, both name the regen command) and
`base_bundle_ships_exactly_the_generated_files` (the uniformity guard
from review R1.2 of 155816: bundle content list == generated set).
Docs: modding-ron.md pipeline paragraph updated (also fixed the stale
`scenario_ron_parity` test name), LESSONS.md write-on-missing recovery
clause repointed at the bin.

Evidence: generator run twice - byte-identical, zero diff vs committed
files. A/B both guards: deleting menu_ambience.content.ron FAILED the
test with the regen message and did NOT recreate the file (the old
behavior would have silently written and passed); deleting the bundle
entry FAILED the set guard; both restored, 2/2 green.
cargo check --workspace --all-targets green, fmt clean. Full suite is
CI's job per the standing instruction.

Alternatives considered: an xtask crate (rejected - the builders and
their engine deps live in nova_assets, a bin target there is the thin
path); asserting bundle ORDER as well as set (rejected - base ids are
unique so order does not affect the merge, and set equality is the
uniformity invariant actually claimed).

Reflection: smooth cycle; the R1.2 routing from the previous task's
review meant the uniformity guard was already specced when work started.
