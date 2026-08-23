# Get master green and ship v0.11.0

- STATUS: IN_PROGRESS
- PRIORITY: 100
- TAGS: v0.11.0,ci,release

## Goal

Get `master` green again, then release v0.11.0. The release checklist opens with
"confirm master is green", so the CI repair is the gate on everything else.

## Why CI broke

Three independent causes, all landing right after the v0.10.0 tag (2026-08-13,
the last green run). Each one was masked by the one above it, so they surfaced
in sequence rather than together:

1. **Toolchain drift.** `rust-toolchain.toml` said `channel = "nightly"`, so CI
   installed whatever nightly was current that morning. Nightly 1.100.0 changed
   method resolution, and `bevy_render` calls `.run_if(..)` unqualified where
   both `IntoScheduleConfigs::run_if` and the preludeed
   `ObserverSystemExt::run_if` are now applicable candidates - E0034, 8 errors,
   inside the dependency. It killed all three build jobs, so no job ever reached
   our own code and the lints below stayed hidden behind it.

   Not fixable from bevy: `ObserverSystemExt` is in the prelude of bevy_ecs
   0.19.0 *and* 0.19.1, and bevy_render 0.19.1 has the same unqualified call
   sites. The toolchain is the thing to pin.

2. **Real lints of ours**, which only became visible once the dependency built:
   `doc_lazy_continuation` in `nova_gameplay` and `nova_scenario`, dead code in
   `nova_ship`, four `--features debug` leaks in examples that the
   default-features job exists to catch, and two `#[expect(..)]` that had gone
   unfulfilled.

3. **A stale test**, reachable only once clippy passed. `d20a37c4` (2026-08-19)
   dropped the Asteroid Field sandbox and its relay, and updated every reference
   except `nova_assets/tests/example_scenario.rs`, which still asserted
   `asteroid_field` and `asteroid_next` were registered built-ins. CI had been
   dark for six days by then, so nothing caught it. The list now mirrors
   `base_content::scenarios::catalog` - four carousel backdrops plus the five
   nova_protocol chapters.

4. **Two probe-sweep failures**, reachable only once the tests passed - the last
   layer of the same onion:

   - `system_section_severing` tripped `log_clean` on one ERROR line:
     `insert_controller_section_render: entity .. not found in q_controller`.
     The example marks a part it already gave its own cube art with a bare
     `ControllerSectionMarker`, and the render observer treated a marker with no
     authored render data as a bug. It is not one, and it is not even avoidable
     from outside: `ControllerSectionRenderMesh` is crate-private, so an external
     caller can ONLY add the bare marker. The guard now fires on the case it was
     really written for - a render mesh present with its
     `SectionRenderMeshTransform` missing, which renders silently at identity and
     drops the authored pose - and stays quiet for a marker-only controller.

   - `screenshot_editor` blew the 170s autopilot deadline, reaching its last of
     76 steps at 161s and dying nine seconds short. Structural, not a perf
     regression: the walk grew this cycle with the collider-comparison captures,
     and every step pays its own settle under lavapipe. Both CI bounds are hang
     detectors rather than performance gates, so the deadline moves to 280s and
     probe's supervisor `--timeout` to 300s (it defaults to 180 and must stay
     above the deadline, or the child dies before the harness can say what it
     was waiting on).

## Direction

- Pin the nightly explicitly, in `rust-toolchain.toml` and `flake.nix` together,
  so the devshell and CI are the same compiler. `nightly.latest` in the flake
  floated with the rust-overlay input: `nix flake update` could move local off
  the CI toolchain without touching a tracked file.
- Fix the lints properly rather than suppressing them. The doc lints are prose
  that wrapped onto a `+ ` / `- ` line start and so read as Markdown list
  markers; rewrap instead of `#[expect]`.
- Revisit the pin once nightly or bevy resolves the ambiguity. The pin is the
  fix for today, not a permanent freeze.

## Done when

- All four CI jobs pass on `master`.
- `cargo fmt --check`, the debug clippy pass, the default-features check, and the
  wasm clippy pass are green locally on the pinned toolchain.
- v0.11.0 is tagged and released per `RELEASE.md`, with the changelog, News, and
  docs current.
