# Make the CI clippy job warning-clean

- STATUS: CLOSED
- PRIORITY: 70
- TAGS: v0.8.0,ci,tooling,refactor

## Story

CI's clippy step (`cargo clippy --workspace --all-targets --features debug`,
`.github/workflows/ci.yaml`) currently emits ~40 warnings across
nova_gameplay, nova_scenario, nova_modding, nova_assets, nova_menu,
nova_editor, nova_core and the examples. The step does not run with
`-D warnings`, so the job stays green, but the noise buries real findings
(the `warnings-clean-before-land` lesson) and every new warning rides in
unseen. Bring the workspace to zero warnings under CI's exact invocation.

## Steps

- [x] Baseline: run CI's exact command in the worktree, output to a file
      (never piped), and enumerate every warning - the user's CI paste starts
      mid-stream, so the local run is the authoritative list. (45 sites, an
      exact match with the CI paste; nothing was hidden above its start.)
- [x] Mechanical lint fixes, taking clippy's suggested transform unless noted:
      - `needless_lifetimes`: `flight.rs:969` (choose_group),
        `nova_assets/src/balance.rs:265` (partition_findings).
      - `doc_lazy_continuation` / doc quote markers: `flight.rs:2330-2331`,
        `nova_scenario/src/loader.rs:526-528`,
        `nova_assets/src/mod_cache.rs:338-342`,
        `nova_assets/src/scenario/shakedown.rs:57-59`,
        `nova_core/src/lib.rs:249-250`,
        `nova_assets/tests/portal_install.rs:15-16`,
        `examples/12_menu_newgame.rs:24`.
      - `match_like_matches_macro`: `nova_gameplay/src/hud/mod.rs:67`.
      - `manual_is_multiple_of`: `nova_gameplay/src/input/ai.rs:1174`.
      - `unnecessary_map_or`: `torpedo_section/render.rs:253,379,501`,
        `turret_section.rs:1330,1517`.
      - `while_let_loop`: `turret_section.rs:975`.
      - `field_reassign_with_default`: `turret_section.rs:3371-3372,
        3410-3411, 3455-3456`.
      - `useless_vec`: `nova_assets/src/lib.rs:1276,1280,1347`.
      - `needless_range_loop`: `nova_assets/src/scenario/shakedown.rs:1423-1424`.
      - `useless_conversion`: `nova_editor/src/ui/mod.rs:118`.
      - `unnecessary_lazy_evaluations`: `nova_menu/src/lib.rs:662`.
- [x] `items_after_test_module`: move `pause_loops`/`resume_loops`
      (`nova_gameplay/src/audio.rs:2160-2175`) above the `mod tests` block.
- [x] `assertions_on_constants`: `nova_assets/src/sections.rs:448,452,471` -
      these are balance-invariant pins with formatted messages; decide per
      site between `const { assert!(..) }` (stronger, but const asserts
      cannot format values into the message) and keeping the runtime test
      shape. Record the choice in NOTES.md.
- [x] `large_enum_variant`, two different fixes (verified, not assumed):
      - `SectionSource::Inline` (`nova_scenario/src/objects/spaceship.rs:151`)
        derives `Reflect`, and bevy_reflect 0.19 has NO `Reflect` impl for
        `Box<T>` (checked in the registry source: `src/impls/alloc/` has no
        `boxed.rs`), so boxing cannot compile without stripping Reflect from
        the whole config tree. Add `#[allow(clippy::large_enum_variant)]`
        with a comment citing the constraint.
      - `Content::Section` (`nova_modding/src/lib.rs:68`) has no Reflect -
        box it for real (`Box<SectionConfig>`). Box<T> serdes identically to
        T, so no RON/bundle format change (content parity tests guard this).
        Sweep EVERY constructor and pattern match workspace-wide (grep dump
        in tmp/box-sweep.txt, 83 lines counted) and fix call sites;
        `check-all-targets-for-struct-field` applies: tests and examples
        construct these too.
- [x] Re-run CI's clippy command to zero warnings; `cargo fmt`; then
      `cargo check --all-targets --features debug` stays green. (Both green;
      the only residual line is the upstream future-incompat notice, which
      exists on master too.)
- [x] Run the touched test suites with CI's feature set:
      `cargo test -p nova_assets --features debug` (parity, merge, sections,
      shakedown, mod_refs, portal_install) and
      `cargo test -p nova_gameplay --features debug turret`. nova_modding has
      no debug feature (a solo run would flip bevy/track_location and rebuild
      Bevy); its Box-touched unit test is compile-verified and its machinery
      runs under portal_install. Full workspace suite stays in CI per the
      repo rule. (Both green: nova_assets 83 lib + all integration suites
      incl. portal_install 14/14; gameplay turret filter 52/52.)
- [x] Write NOTES.md (fix record: per-lint choices, especially the
      assertions_on_constants and boxing decisions).

## Definition of Done

- `cargo clippy --workspace --all-targets --features debug` completes with
  zero warnings in this repo's crates (dependency future-incompat notes are
  out of scope).
- No behavior change: pure lint-shape refactors; serde wire formats
  unchanged; the balance assertions still assert the same invariants.
- NOTES.md records the non-mechanical choices.

## Notes

- CI gate: `.github/workflows/ci.yaml` "Clippy" step. It does NOT use
  `-D warnings`, and `rust-toolchain.toml` floats on `channel = "nightly"`
  (no date pin), so enforcing `-D warnings` would let any future nightly's
  new lints redden CI without a code change - this batch of warnings arrived
  exactly that way. Deliberately NOT changing the gate here; surfaced to the
  user as a follow-up fork (pin the nightly date + `-D warnings`, or stay
  advisory).
- Local verification is allowed for this task: the repo rule "do not run
  clippy locally" exists because CI covers it, but this task IS the clippy
  fix, and the user asked for it. Full `cargo test` still stays in CI.
- `Box<SectionConfig>` sites to expect: content builders in nova_assets,
  mod merging in nova_modding, spawn resolution in nova_scenario, plus test
  fixtures (`section(...)` helpers in `nova_assets/src/lib.rs` tests).

## Close-out (2026-07-19)

What changed: all 45 clippy warnings under CI's exact gate fixed to zero;
see NOTES.md for the per-lint choices and REVIEW.md round 1 (out-of-context
pass, APPROVE, 12 verified claims, 2 NITs fixed). Alternatives considered
and rejected: `-D warnings` in CI (floating nightly would redden CI without
code changes - surfaced to the user instead), boxing SectionSource::Inline
(impossible under derive(Reflect) in bevy_reflect 0.19, verified in source).

Difficulties: cargo clippy --fix mis-fixed one doc comment (blockquote
markers baked into shakedown.rs prose) - caught only because the produced
diff was re-read line by line before proceeding; the doc-lint family in
general wants prose rewraps, not clippy's mechanical suggestions.

Self-reflection: the sweep-first discipline (grep dump + compiler as
enumerator via check --all-targets) made the Box refactor uneventful; the
one thing to do differently is to expect auto-fix to be WRONG on prose
lints and budget the manual pass for them from the start. Verification ran
locally because the task IS the lint gate (repo rule exception noted in
TASK.md); the full workspace suite remains CI's job.
