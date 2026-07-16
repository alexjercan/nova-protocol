# Explicit content generator bin; make content_ron_parity assert-only

- STATUS: OPEN
- PRIORITY: 90
- TAGS: refactor, testing

## Goal

Keep authoring base scenarios/sections as .rs builders, but stop
generating the committed RON as a test side effect. An explicit bin
writes the files; the parity test only asserts. Decision from audit task
20260716-141620 (FINDINGS.md section 1), user confirmed.

## Steps

- [ ] Extract shared helpers into `nova_assets::scenario_generation`
      (crates/nova_assets/src/lib.rs:48-115): a `serialize_content` fn
      (pretty_config + trailing newline, currently duplicated in the
      test) and a fn returning the full file map
      `Vec<(relative_path, contents)>` covering
      `base/sections/base.content.ron` + every builder-backed
      `base/scenarios/<id>.content.ron`, so bin and test cannot diverge.
- [ ] Add `crates/nova_assets/src/bin/gen_content.rs`: writes every file
      in the map under the workspace `assets/` dir
      (env!("CARGO_MANIFEST_DIR")/../../assets, same resolution as the
      parity test), creating parent dirs, printing what it wrote. Run as
      `cargo run -p nova_assets --bin gen_content`.
- [ ] Rewrite `crates/nova_assets/tests/content_ron_parity.rs`
      assert-only: a MISSING file is a failure (today it silently writes
      and passes), drift is a failure; both failure messages name the
      exact regen command.
- [ ] Prove generator stability (verify-generator-stability lesson): run
      the bin twice, `git diff --exit-code` after each - byte-identical
      and matching the committed files.
- [ ] Update prose: the scenario_generation module doc (lib.rs:40-42 says
      it exists "for the content_ron_parity test"), the parity test
      header, and grep docs/ + web/src/wiki for the old "delete the file
      and re-run the test" regen instruction.
- [ ] Verify: `cargo check --workspace --all-targets`, `cargo fmt`, run
      content_ron_parity (full suite is CI's job).

## Notes

- Builders remain the single source; the game still loads only the RON.
- Bin file named gen_content.rs (underscore) so no [[bin]] table needed.
- The RON serialization is deterministic by construction (fixed
  PrettyConfig, no maps in output order?) - the double-run check is the
  proof, do not skip it.
- generate-data-from-code corollary applies from here on: any builder
  change regenerates the RON in the SAME commit via the bin.
