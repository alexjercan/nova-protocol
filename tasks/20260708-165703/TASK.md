# Lock-on acquisition dwell (radar hold-to-lock)

- STATUS: CLOSED
- PRIORITY: 25
- TAGS: v0.7.0,targeting,torpedo,hud

Spike: tasks/20260708-165647/SPIKE.md (origin), superseded/reopened below.

## Goal

Make a radar lock non-instant: a candidate under the look ray must be held for
a per-target dwell (N ms, distance-scaled) before it hard-commits to its lock
slot. You can sweep off before the ring fills to cancel; holding steady on a
distant target is a real skill beat. The dwell duration is a pure function
with a per-target multiplier hook, so future stealth ("harder to lock at a bad
aspect") is a knob, not a rewrite. This task is the MECHANIC only; the radial
"loading ring" visual is 20260717-004302 (depends on this).

## History / why this reopened

The 20260708-165647 weapons-HUD spike seeded this as "lock dwell + acquire
cue". The 20260709-192358 component-lock spike then MOVED the dwell one level
down: the ship lock stayed instant and a 1.5 s focus dwell (`LockFocus`,
FOCUS_TIME) gated the deeper COMPONENT fine-lock instead (shipped in
20260709-192522 / -192523). Separately the deliberate-radar rework
(20260713-082207/-082330) replaced the passive aim-cone auto-pick with an
explicit hold-CTRL radar gesture, and the audio + lock-cue polish shipped
(audio 20260708-162011 CLOSED; lock SFX/inset 20260713-110311 CLOSED).

User decision 2026-07-17 REOPENS the ship-lock dwell after all, as a distinct
mechanic on top of the radar gesture:

- The dwell gates BOTH lock slots (combat AND travel), not combat only.
- It is a SEPARATE stage that runs BEFORE the existing component focus dwell:
  radial ring fills (acquire the ship lock) -> hard lock + LockOn cue -> the
  existing 1.5 s focus bar fills (deepen to components). Two distinct stages;
  `LockFocus` / FOCUS_TIME is untouched.
- Visual is a smooth radial arc via a UiMaterial shader (the -004302 task).

## Current mechanic (verified, targeting.rs)

`crates/nova_gameplay/src/input/targeting.rs`, `update_radar_search`
(lines ~646-755). Hold CTRL raises `RadarState`; past the tap threshold
(`RADAR_TAP_SECS` 0.25 s) the destination slot latches (`RadarState.engaged`:
Combat while `WeaponsRaised`, else Travel). Each held frame `radar_pick`
(pure, lines ~765-796) resolves the best candidate under the 18 deg cone
(`TARGETING_CONE_HALF_ANGLE_DEG`) with `RADAR_PICK_HYSTERESIS`, and the code
writes that candidate to the slot LIVE and instantly (keep-last over empty
space). The FIRST resolve fires `RadarLockAcquired` (-> `NovaSfx::LockOn`),
later changes fire `RadarRetargeted`. `RadarState.acquired` gates once-vs-many.
Torpedo commit reads `CombatLock` at launch only (player.rs
`update_torpedo_target_input`, ~472-506); `WeaponsHot` derives from the combat
lock existing. Systems run chained: contacts/locks -> radar_search ->
weapons_safety -> ... -> tick_lock_focus -> component_lock.

The instant slot write is the single point this task changes: gate it behind a
completed per-target dwell.

## Steps

- [x] Extend `RadarState` (targeting.rs ~128-140) with dwell bookkeeping:
      `dwell_target: Option<Entity>` (the candidate the timer is charging on)
      and `dwell_secs: f32`. Keep `Reflect`/`Default`. Doc-comment that the
      slot is only written at dwell completion now, so keep-last holds the
      prior lock while a NEW candidate charges.
- [x] Add a pure dwell-duration helper
      `fn lock_dwell_secs(distance: f32, effective_range: f32, modifier: f32)
      -> f32` (or a small `LockDwellCtx` struct): base + distance term, so far
      targets take longer, times `modifier`, clamped to
      `[LOCK_DWELL_MIN, LOCK_DWELL_MAX]`. Proposed shape:
      `(LOCK_DWELL_BASE * (1.0 + LOCK_DWELL_RANGE_FACTOR
      * (distance / effective_range).clamp(0,1)) * modifier)
      .clamp(MIN, MAX)`. Keep it pure and camera/physics-free for unit tests.
      `modifier` defaults to 1.0; it is the stealth/aspect extension point
      (read from an OPTIONAL per-target component in a later task, NOT built
      here - just plumb the argument as 1.0 for now).
- [x] Add the dwell tunables to `TargetingSettings` (the reflected tunables
      resource, ~60-92) so they are inspector-tunable like the range knobs:
      `lock_dwell_base` (start 0.6 s), `lock_dwell_range_factor` (start ~1.5,
      so a target at its range edge dwells ~2.5x base), `lock_dwell_min`
      (0.25 s), `lock_dwell_max` (2.5 s). Reference the existing signature/
      range fields for the `effective_range` the formula divides by (the same
      per-candidate gate `collect_lockable` already computes - surface it or
      recompute distance/effective range for the resolved candidate).
- [x] Rewrite the commit path in `update_radar_search` (~705-753): after
      `radar_pick` resolves `radar.candidate`, while the hold has fired,
      charge the dwell instead of writing the slot:
      - If `radar.candidate != radar.dwell_target`: reset
        `dwell_target = radar.candidate`, `dwell_secs = 0.0` (a sweep to a new
        candidate, or off to `None`, CANCELS the in-progress dwell).
      - Else accumulate `dwell_secs += time.delta_secs()`.
      - When `dwell_secs >= lock_dwell_secs(...)` for the current candidate,
        COMMIT: write the engaged slot (the current `combat.0`/`travel.0`
        assignment), then fire the acquire/retarget cue exactly as today
        (first commit of the gesture -> `RadarLockAcquired` + set
        `radar.acquired`; a later commit onto a different target ->
        `RadarRetargeted`). Do not write the slot before completion.
      Preserve the existing combat-sweep decay-hold (F12, ~716-721) and the
      empty-space keep-last (no candidate -> continue, slot untouched).
      Note the `Time` resource must be added to the system's params.
- [x] Confirm the cancel/abort paths still hold: releasing CTRL before
      completion commits nothing (existing abort, `on_radar_cancel`/no write);
      an in-flight dwell is dropped when `RadarState` is removed at gesture
      end. Reset `dwell_target`/`dwell_secs` wherever `engaged` is cleared so a
      new gesture starts clean.
- [x] Tests (world tests, advance `Time` manually, targeting.rs test module):
      dwell accumulates on a held candidate and the slot stays empty until it
      completes; slot commits + `RadarLockAcquired` fires ONCE at completion,
      not before; sweeping to a different candidate mid-dwell resets the timer
      (cancel) and the prior lock is kept (keep-last); a completed re-target
      onto a new candidate moves the slot and fires `RadarRetargeted`; release
      before completion commits nothing; BOTH slots gated (a travel-engaged
      gesture dwells too). Pure `lock_dwell_secs` unit tests: monotonic in
      distance, `modifier` scales linearly, clamps at MIN/MAX.
- [x] Verify: cargo fmt, cargo check --workspace, new + touched targeting
      tests only (report skips per repo policy - CI runs the full suite).

## Notes

- Relevant files: `crates/nova_gameplay/src/input/targeting.rs`
  (`RadarState` ~128, `TargetingSettings` ~60, `update_radar_search` ~646,
  `radar_pick` ~765, `collect_lockable`/`LockableQuery` for effective range,
  cue messages `RadarLockAcquired`/`RadarRetargeted` ~209-224); consumers that
  read the resulting `CombatLock` are unchanged (torpedo commit player.rs
  ~472, `WeaponsHot`, focus/component in targeting.rs).
- The acquire/lock CUE is already fully wired: `NovaSfx::LockOn` fires off
  `RadarLockAcquired`, `LockOff` off `LockClearedToast` (audio.rs). Moving the
  write/cue point to dwell-completion needs NO audio work - the existing cue
  now lands on the satisfying snap for free.
- Decision recorded above (2026-07-17): dwell gates BOTH slots; separate stage
  BEFORE the component focus dwell; `LockFocus`/FOCUS_TIME untouched.
- Design fork resolved: the dwell REPLACES the instant commit (every slot
  write now requires a completed dwell, including mid-gesture re-designation),
  it does not augment a parallel soft-lock. Keep-last means the previous lock
  visibly holds while a new candidate charges.
- Extension hook: `modifier` in `lock_dwell_secs` is where a future
  stealth/aspect mechanic multiplies dwell up ("invisible at a certain
  degree"). Do NOT build stealth here; just leave the seam and pass 1.0.
- Depends on: nothing new (audio + radar already shipped). Blocks the visual
  ring 20260717-004302, which reads the dwell fraction this task exposes.
- Consider exposing a read helper on `RadarState` (e.g. `dwell_fraction(&self,
  needed: f32) -> f32`) so the HUD task and tests read the fill cleanly rather
  than recomputing.

## Resolution (CLOSED 2026-07-17)

Shipped in `input/targeting.rs`:

- `RadarState` gained `dwell_target: Option<Entity>` + `dwell_secs: f32` and a
  `dwell_fraction(needed)` helper (what the ring HUD 20260717-004302 will
  render).
- `TargetingSettings` gained five reflected knobs: `lock_dwell_base` (0.6),
  `lock_dwell_range_factor` (1.5), `lock_dwell_reference_range` (2000),
  `lock_dwell_min` (0.25), `lock_dwell_max` (2.5).
- Pure `lock_dwell_secs(distance, modifier, settings)`: base + a distance term
  scaling to the range factor at the reference range and saturating beyond,
  times `modifier` (the stealth/aspect seam, 1.0 today), clamped to [min, max].
- `update_radar_search` now takes `Res<Time>` and gates every slot write behind
  a completed dwell: settle -> charge on the candidate -> commit only at
  completion; a candidate change or drop-to-None resets the dwell (cancel);
  keep-last holds the prior lock while a new candidate charges; the acquire/
  retarget cues moved to the commit (so `NovaSfx::LockOn` now lands on the snap
  for free).

Tests: 5 new (`lock_dwell_scales_with_distance_and_clamps`,
`the_dwell_gates_the_slot_commit_and_the_ring_fills`,
`sweeping_off_before_the_ring_fills_cancels_and_keeps_last`,
`a_re_designation_earns_its_own_dwell`, `the_combat_slot_is_dwell_gated_too`);
`input::targeting` 47 passed, `hud::lock_crosshairs` 4 passed. cargo fmt +
`cargo check -p nova_gameplay` clean. Full local suite + clippy skipped per
repo policy (CI runs them).

### Deviations from the plan

- **Distance normalized by a tunable `lock_dwell_reference_range`, not the
  per-target effective lock range.** The plan proposed dividing by the
  candidate's effective range (from `collect_lockable`), but that couples dwell
  to object class oddly (a ship lockable at 20 km would dwell ~instantly while a
  torpedo at 2 km dwells near-max). A fixed reference range is simpler, needs no
  `Lockable` tuple change, and matches the user's "depends on distance" intent
  directly. The distance to the pending candidate is read from the already
  collected `candidates` list.
- **Gesture tests neutralize the dwell to zero in `gesture_app`.** The ~10
  existing gesture tests assert latch / keep-last / tap-clear semantics that are
  orthogonal to the dwell; rather than churn each with +13 frames, `gesture_app`
  sets the dwell to zero (instant commit = their original assumption) and the
  dwell GATE has its own five focused tests with real durations. Documented in
  `gesture_app`.

### Docs synced (keep-docs-in-sync lesson)

The change invalidated the "written every frame while you hold" description, so
in the same task: `web/src/wiki/targeting-radar.md` and `getting-started.md`
now describe the hold-to-lock dwell (cancelable, distance-scaled), and a
CHANGELOG `[Unreleased] > Combat & Weapons` entry was added. The ring VISUAL
description (hud.md, the ring image) is deliberately left to 20260717-004302,
which adds the visible feedback - documenting a ring that does not exist yet
would be false at this task's landing.

### Self-reflection

Reading the real `update_radar_search` first paid off: the lock was already a
deliberate hold gesture (not "instant" as the stale spike said), so the change
was a clean one-point gate rather than a new subsystem. One compile round was
lost to a test-only `RadarState { .. }` literal in `hud/lock_crosshairs.rs`
that `cargo check` (non-test) did not catch - a reminder that adding a field to
a `Default` struct still needs a test-target compile. Next time, grep for
`StructName {` across the crate before trusting a green `check`.
