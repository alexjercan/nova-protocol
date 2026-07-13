# One hit = one cue: dedup HealthApplyDamage propagation in audio + juice

- STATUS: CLOSED
- PRIORITY: 50
- TAGS: v0.4.0,audio,juice,bug,spike

Spike: tasks/20260709-091536/SPIKE.md
Source: PR #54 review F1/F3/F4 (tasks/20260708-162013/REVIEW.md);
supersedes the "key by IntegrityRoot" note in CLOSED task 20260708-215922 and
PR #53 review F3.

## Goal

`HealthApplyDamage` auto-propagates section -> ship root (the game depends on
the bubbling for ship death), and the global cue observers in
`crates/nova_gameplay/src/audio.rs` (`on_damage_play_impact`) and
`crates/nova_gameplay/src/juice.rs` (`on_damage_juice`) fire once per
propagation hop. When the section and root straddle a 6-unit area-cell
boundary, one hit produces two cues: doubled impact sound, 2x camera trauma
(0.16 vs the tuned 0.08), and a phantom flash ring at the ship root's origin
(confirmed empirically in the PR #54 review).

Make one logical hit produce exactly one cue in both modules: guard each
damage-cue observer to react only when `damage.entity ==
damage.original_event_target()` (Bevy 0.19, available on propagating entity
events), leaving the propagation itself untouched. Per the spike, this beats
rekeying the throttle by `IntegrityRoot` (over-collapses distinct hit points,
treats the symptom) and a shared-cue-seam extraction (deferred under the rule
of three until a third cue consumer exists).

In the same change, since it touches the same observers:

- Add observer-level regression tests: a parented hierarchy with the parent
  one cell away must yield a single cue (flash count 1 / single trauma /
  single sound).
- Document the propagation caveat on both observers so a future damage-cue
  observer copies the guard along with the shape.

(Originally this task also carried the attenuation-path observer tests and the
"Three effects" doc-count fix from review findings F3/F4; those landed in the
PR #54 branch itself while addressing the Copilot comments - see the review
addendum - along with flash distance attenuation, F7.)

## Steps

- [x] Guard `on_damage_play_impact` in `crates/nova_gameplay/src/audio.rs`:
      early-return when `damage.entity != damage.original_event_target()`, with
      a doc comment on the observer explaining the propagation caveat (the
      event auto-propagates section -> ship root and ship death depends on the
      bubbling, so only the cue reaction is guarded, and the original target is
      also the better cue position - the actual hit location).
- [x] Guard `on_damage_juice` in `crates/nova_gameplay/src/juice.rs` the same
      way, with the same doc comment shape.
- [x] Juice regression test: in the existing observer-test module, spawn a
      parent one area cell away (`> JUICE_AREA_CELL` offset) with a child via
      `ChildOf`, trigger one `HealthApplyDamage` on the child, assert exactly
      one flash and exactly `hit_trauma` (not 2x) - the literal PR #54 scratch
      test.
- [x] Audio regression test: new observer-level test in `audio.rs` using a
      minimal asset app (`MinimalPlugins` + `AssetPlugin` +
      `init_asset::<AudioSource>` + `SoundBank::load`, as in bcs
      `registry.rs` tests); same straddling hierarchy, one damage trigger,
      assert a single impact fired (one `ThrottleKey::Impact` stamp in
      `SfxThrottle`, keyed by the child's cell).
- [x] Verify: `cargo fmt --check`, `cargo check --workspace --all-targets`
      (force a real recompile of the touched crate), and run the new/module
      tests for `audio` and `juice`.

## Notes

- Facts pinned during planning:
  - `HealthApplyDamage` is `#[derive(EntityEvent)]` with
    `#[entity_event(propagate, auto_propagate)]`
    (bevy-common-systems `src/health/mod.rs:68`).
  - `On<E>` for propagating events exposes `original_event_target()`
    (bevy_ecs-0.19.0 `src/observer/system_param.rs:139`), so the guard reads
    exactly `damage.entity != damage.original_event_target()`.
  - `world.trigger()` in tests does propagate (the trigger impl drives the
    `ChildOf` traversal), so the regression tests exercise the real hop.
  - The destroy observers need no guard: `On<Add, IntegrityDestroyMarker>` is
    a lifecycle event, it does not propagate.
- Out of scope (already landed on the PR #54 branch): attenuation-path
  observer tests, "Three effects" doc count, flash distance attenuation.
- Deferred by the spike: shared combat-cue seam (rule of three - wait for a
  third cue consumer); throttle-by-`IntegrityRoot` rejected.

## Result

Branch fix/one-hit-one-cue (local branch by user request, no worktree).

- Both damage-cue observers now early-return unless
  `damage.entity == damage.original_event_target()`, with a propagation-caveat
  doc comment on each so a future damage-cue observer copies the guard along
  with the shape. Propagation itself is untouched (ship death still bubbles).
- Regression tests added, both modeled on the PR #54 scratch test (parent one
  area cell away, child damaged once):
  - `juice::tests::a_propagated_hit_on_a_straddling_hierarchy_fires_one_cue`
    asserts exactly `hit_trauma` (not 2x), exactly one flash, and that the
    flash sits at the hit location, not the parent's origin.
  - `audio::tests::a_propagated_hit_on_a_straddling_hierarchy_plays_one_impact`
    counts actual `PlaySfx` triggers via a test observer (stronger than
    inspecting throttle stamps) and also asserts the single throttle stamp is
    keyed by the child's cell. Uses a minimal asset app so `SoundBank::load`
    works headless.
- Both tests were mutation-checked: with the guard disabled (`if false && ...`)
  they fail with flashes/sounds = 2, confirming they pin the real regression.
- Difficulty hit: the audio test initially counted 0 sounds - the observer
  plays via `Commands`, so the queued `PlaySfx` trigger only fires after a
  `world.flush()`; the juice observers mutate resources directly, which is why
  the juice test needed no flush. Documented in the test.
- Verification: `cargo fmt --check` clean; `cargo check --workspace
  --all-targets` green after touching the edited files (forced recompile);
  full `audio::` (9) and `juice::` (21) module tests pass. Full workspace test
  suite and clippy deliberately skipped locally per standing instruction - CI
  covers them.
- Self-reflection: the mutation check itself was worth it, but reverting the
  mutation via `git checkout -p < /dev/null` was sloppy and silently did
  nothing; a targeted `sed` restore (or stash) is the right tool. Check the
  working tree state after any scripted revert.
