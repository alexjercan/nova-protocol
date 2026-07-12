# Review: Objective conveyance visuals: markers, item highlight, hint emphasis

- TASK: 20260712-093831
- BRANCH: objective-conveyance-visuals

## Round 1

- VERDICT: REQUEST_CHANGES
- Reviewed by a fresh-context agent pass over `git diff master...` plus
  spot re-verification in-session (target_world_aabb read directly; the
  attach-after-spawn FIFO ordering and the no-relayout-on-color-write
  claims were independently re-derived by the reviewer from
  bevy_common_systems' handler loop, NovaEventWorld's VecDeque drain, and
  bevy_ui's invalidation inputs). All 8 targeted test suites green
  (objective_markers 4, item_highlights 2, keybind_hints 7,
  beacon_chips 1, actions 8, loader 8, salvage 4, shakedown 10).

- [x] R1.1 (MAJOR) crates/nova_gameplay/src/hud/item_highlights.rs:80 +
  crates/nova_gameplay/src/hud/screen_indicator.rs:296-315 +
  crates/nova_scenario/src/objects/salvage.rs:64 - the bracket sizes to
  the PICKUP SENSOR, not the crate. ApparentSize unions the subtree's
  ColliderAabbs; a salvage crate's only collider is the sensor sphere on
  its root (CRATE_AREA_RADIUS = 8.0 in shakedown.rs), the render child
  has none. So the bracket's world radius is ~13.9u around a 1.5u crate:
  ~70 px at 120m, ~280 px at 30m, ~1000 px (the whole screen) at pickup
  range. The advertised "tightens as you close in" is inverted - the
  bracket swallows the view during the approach. No test pins bracket
  sizing, which is how it slipped through. Suggested change: size the
  bracket from the VISIBLE extent, not collider semantics - e.g. a new
  ScreenIndicatorSize::WorldRadius { radius, min_px } variant fed from
  the crate's authored size (SalvageCrateSize -> ItemHighlight carrying
  a world radius), or exclude Sensor colliders from target_world_aabb
  AND give the highlight an authored-radius path (a sensor-only subtree
  must not silently fall back to min_px and lose the scaling). Do NOT
  fix by adding a solid collider to the crate - collider-class changes
  ripple through the avian pair matrix (lessons ledger). Add a sizing
  test either way.
  - Response: fixed - new ScreenIndicatorSize::WorldRadius { radius,
    min_px } widget mode projects an AUTHORED world radius (shares the
    projection algebra with ApparentSize); ItemHighlight now carries
    world_radius and the bracket uses WorldRadius; salvage crates author
    the crate box half-diagonal (size * sqrt(3)/2 ~ 1.3u), fully
    decoupled from the sensor. No collider change. Tests:
    world_radius_projects_the_authored_radius_not_the_colliders (huge
    sensor AABB on the anchor, exact expected px at 10u, halves at 20u,
    floors at min_px), brackets_size_to_the_authored_radius (the spawned
    indicator's mode), and the salvage contract test pins the authored
    radius against the 6u sensor.

- [x] R1.2 (MINOR) crates/nova_gameplay/src/hud/beacon_chips.rs:141,
  207-225 - dedupe/attach frame order is a schedule tie-break.
  dedupe_marked_beacon_chips runs in Update (NovaHudSystems) unordered
  vs the queued-command drain that inserts/removes ObjectiveMarkerTarget,
  so on attach there can be one frame with BOTH chips clamped at the same
  edge point (the exact jitter the dedupe exists to prevent), and on
  detach one frame with none (marker chip dies via observer immediately,
  beacon anchor restores next dedupe run). Cosmetic and self-healing, but
  cheap to make deterministic: order the dedupe after the scenario drain
  within Update, or run it in PostUpdate before ScreenIndicatorSystems.
  - Response: fixed, via a third option - the polled system is replaced
    by Add/Remove observers on ObjectiveMarkerTarget (beacon-gated), so
    the hand-off lands in the SAME command flush as the marker
    insert/removal; no schedule tie-break exists at all. (Set-ordering
    against the drain was not available cross-crate: the drain is an
    anonymous chained tuple in bevy_common_systems' PostUpdate, and
    nova_gameplay cannot name NovaEventWorld.) The observer shape loses
    the polled self-heal, so the one uncovered ordering - a chip spawned
    for an ALREADY-marked beacon - is closed at the source:
    setup_beacon_chip spawns the chip born-yielded when the beacon
    carries a marker. Tests: the dedupe test now drives the real
    observers flush-by-flush (plus a non-beacon marker guard), and
    chip_spawned_for_an_already_marked_beacon_starts_yielded pins the
    adversarial ordering.

- [x] R1.3 (MINOR) crates/nova_gameplay/src/hud/keybind_hints.rs:552-575 -
  cleared_emphasis_restores_the_base_color does not exercise real change
  detection: every run_system_once registers a fresh system for which
  Res::is_changed is ALWAYS true, so the is_changed gate in
  pulse_emphasized_rows never runs the way it does across real ticks.
  The gate is correct today (traced), but the test would stay green if it
  regressed in tick-visible ways. Suggested change: an App-driven variant
  (like teardown_clears_hint_emphasis) that clears the emphasis, runs
  app.update() twice, and asserts the color stays at base.
  - Response: fixed - emphasis_gates_behave_across_real_frames runs the
    real Update schedule: set -> pulses, clear -> restores next frame,
    then two quiet frames assert the color HOLDS at base (a regressed
    gate would resume pulsing or stick a stale color).

- [x] R1.4 (MINOR) crates/nova_gameplay/src/hud/keybind_hints.rs:314 - a
  row whose key empties while emphasized keeps its mid-pulse gold
  TextColor: the pulse and restore branches both skip key-empty rows, and
  update_hint_cluster clears only the text. Invisible today (empty text),
  but it is exactly the stuck-mid-pulse state the restore guard exists
  for, held together by a side effect. Suggested change: on
  emphasis.is_changed(), restore base for key-empty rows too (drop the
  early continue from the restore branch only).
  - Response: fixed, one step further than suggested - the restore branch
    now fires on emphasis.is_changed() OR hints.is_changed() and includes
    key-empty rows, because the key emptying IS a hints change (emphasis
    untouched), so restoring only on emphasis changes would still have
    left the gold frozen. For non-emphasized keyed rows the restore write
    duplicates update_hint_cluster's identical value and the diffed write
    no-ops. Test: rig_despawn_mid_pulse_restores_the_base_color (pulse,
    then FlightVerbHints::default(), assert DIM restored).

- [x] R1.5 (NIT) crates/nova_assets/src/scenario/shakedown.rs:730 - the
  binding `preexisting` actually holds "ids spawned by THIS handler";
  rename to spawned_by_this_handler.
  - Response: fixed - renamed (replace count asserted: 2 sites).

- [x] R1.6 (NIT) crates/nova_scenario/src/objects/salvage.rs:341 -
  crate_glow_pulses_inside_its_band uses std::thread::sleep(50ms);
  TimeUpdateStrategy::ManualDuration is the deterministic pattern used
  elsewhere (screen_indicator.rs tests).
  - Response: fixed - ManualDuration(period/8), under the Time<Virtual>
    0.25s max-delta clamp (the clamp lesson from 20260525-133025).

- [x] R1.7 (NIT) crates/nova_gameplay/src/hud/beacon_chips.rs:172-198 -
  deduped (anchor-None) chips still format their distance label every
  frame the distance changes. Wasted work only; fine to leave, noted for
  completeness.
  - Response: left as-is per the finding's own assessment - a suppressed
    chip revives with a current label the same frame it un-yields, which
    the skipped formatting would otherwise delay by a frame; the cost is
    one small format per deduped chip per moved frame (at most a
    handful).

Verified clean by the review (no findings): attach-after-spawn FIFO
ordering holds end to end (handler action order -> VecDeque push_back ->
single CommandQueue apply; re-attach is insert-replace, no observer
re-fire, no chip leak); shakedown reachability walk finds no orphaned
marker or emphasis on any path incl. death mid-beat (both teardown paths
clear HintEmphasis and despawn scoped entities); dedupe is the sole
beacon-chip anchor writer and the widget re-shows on restore; the
every-frame color writes cost render extraction only, no UI relayout, and
the tier system does not fight the widget (apply_hud_visibility re-hides
after ScreenIndicatorSystems by design); docs match behavior except the
R1.1 "tightens" claim; no vacuous tests found.

## Round 2

- VERDICT: APPROVE

Every R1 response verified against the new diff (commit 9f3050d):

- R1.1: WorldRadius mode confirmed in screen_indicator.rs (shared
  radius_to_px projection); the widget test plants a deliberately huge
  sensor-sized ColliderAabb on the anchor and asserts the exact authored
  projection at 10u, the half-size at 20u, and the min_px floor -
  mutation-resistant against a regression back to collider-derived
  sizing. ItemHighlight::world_radius flows salvage -> observer ->
  indicator; the salvage contract test pins half-diagonal vs the 6u
  sensor. Ticked.
- R1.2: the polled system is gone; suppress/restore observers are
  beacon-gated and same-flush, and the born-yielded spawn closes the
  chip-after-mark ordering the observers alone could not self-heal.
  Both new tests exercise real observers. The cross-crate set-ordering
  impossibility argument is correct (the drain is an unnamed chained
  tuple in bevy_common_systems). Ticked.
- R1.3: the app-driven test drives the real schedule with set, clear,
  and two quiet frames. Ticked.
- R1.4: the hints.is_changed() restore is the right generalization -
  verified the no-op argument for keyed unemphasized rows (identical
  value, diffed write). The rig-despawn test pins DIM restoration.
  Ticked.
- R1.5/R1.6 verified (rename count 2; ManualDuration under the virtual
  clamp). R1.7 left by agreement.

Checks: cargo check + fmt clean; hud suite 99 passed, salvage 4,
shakedown 10, actions/loader unchanged green. No new findings.
