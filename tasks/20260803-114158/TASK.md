# Clear the nova_debug harness rustdoc warnings and reel nits

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog,tooling,autopilot,docs

## Story

Round-1 review of `20260802-183403` APPROVEd the `nova_autopilot` migration with
six non-blocking findings, all in `crates/nova_debug/src/harness.rs` plus one in
`tests/examples_smoke.rs`. Five are new rustdoc warnings the migration
introduced; the rest are small correctness and readability nits. None block, so
they were deferred rather than folded into the atomic rename commit.

## Steps

- [ ] R1.1 - the new `nova_autopilot` dependency makes bare
      ``[`nova_autopilot`]`` intra-doc links ambiguous (function vs crate),
      emitting four `broken_intra_doc_links` warnings
      (`harness.rs` lines ~83, ~108, ~128 and the module header). Spell them
      ``[`nova_autopilot()`]``, matching line 76.
- [ ] R1.2 - `nova_reel`'s public doc links ``[`reel_freeze_bodies`]``, which is
      private, producing a `rustdoc::private_intra_doc_links` warning. Drop the
      brackets or make the fn `pub`.
- [ ] R1.3 - an empty `beats` list makes `ScreenshotReelPlugin` register
      nothing, but `reel_freeze_bodies` is still added, so an empty reel
      silently statics the scene. Guard the `add_systems` with
      `if !self.beats.is_empty()`.
- [ ] R1.4 - `scenario_camera_present` uses `world.iter_entities().any(..)`, a
      full-world scan every frame. Add a comment noting the `&World` signature
      forbids `query_filtered` and the scan is deliberate.
- [ ] R1.5 - `reel_beat_carries_the_output_path` asserts against
      `NOVA_SCREENSHOT_SETTLE_FRAMES`, which `reel_beat` never sets; the value
      comes from the crate's `DEFAULT_SETTLE_FRAMES` and matches by coincidence.
      Assert against `ReelBeat::new("x").settle_frames`, or drop the line.
- [ ] R1.6 - `tests/examples_smoke.rs` `DRIVERS` includes
      `ScreenshotReelPlugin`, but `bevy_common_systems` has no reel module, so
      it is not one of the "names the bcs prelude ALSO exports" the comment
      describes. Drop it, or amend the comment to say the list also pins the
      crate-side reel type out of examples.

## Definition of Done

- `nova_debug` documents clean.
  (cmd: `nix develop --command cargo doc -p nova_debug --no-deps 2>&1 | grep -c warning` reports 0)
- The workspace still builds and the fleet still smokes.
  (cmd: `nix develop --command cargo check --workspace --all-targets --features debug`)
  (cmd: `nix develop --command cargo test --test examples_smoke`)

## Notes

- Source: `tasks/20260802-183403/REVIEW.md` round 1, findings R1.1-R1.6.
- R1.1/R1.2 were reproduced directly with `cargo doc -p nova_debug --no-deps`
  during review; master did not carry these five warnings.
