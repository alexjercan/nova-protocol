# Systems ranges: combat and destruction at both hull sizes

- STATUS: OPEN
- PRIORITY: 71
- TAGS: v0.14.0, testing, examples, combat

## Goal

Add only the live combat, destruction, mission-HUD and ship-audio proof that is
still missing after the v0.14.0 hull-size sweeps. Do not repeat proof already
landed since v0.13.0, and do not change decisions owned by sibling sprint tasks.

Rules: `examples/systems/README.md`. Each new range gets its `[[example]]`
block and its roster slugs in `crates/nova_probe_cli/tests/catalog_drift.rs`.
Extend an existing range when it already owns the subject. A range that finds a
defect fixes it in the same lane and records it here.

The reference pair is `block_skiff` and `block_carrier`. A range may use an
inline one-section fixture where the invariant explicitly needs the minimum
possible hull.

## Re-check, 2026-09-12

Reviewed commits and current code from `v0.13.0` through `27010fb39`, then
compared this task with `20260909-213441`, `20260909-213525`,
`20260909-214629`, and `20260909-214706`.

Work landed after this task was written:

- `3374155f6` and `563be0915` added `system_hud_shell` proof on the skiff and
  carrier. The velocity and gravity shells, camera clearance, and
  world-anchored chip clearance are done.
- `5fa49826f` extended `system_hud_indicators` with measured stack and inset
  placement at 1x and 2x. Its existing small fixture already proves the generic
  indicator chain, component count, lead point, inset presence, and cleanup.
- `33e5393fe` created `system_collision_damage`. It proves safe contact on both
  reference hull sizes, damage to both participants, and damage growth with
  closing speed.
- `system_section_severing` already proves that a severed fragment is inert and
  damageable. `bug_neutralized_quiet` proves that a defeated hull remains
  damageable and may retain a working gun.
- `system_torpedo_launch` proves contact-fuze margin against a moving gate,
  launcher clearance, guidance, detonation, and authored cruise delivery.
  `torpedo_survives_target_loss_and_freezes_position` already proves the
  target-loss policy.
- `system_railgun_lance` proves one recoil impulse and the one-shell reload.
  Focused railgun tests also pin reload and off-axis torque. `25b63c26f` made
  bore-sight penetration use the round's power rule.
- Scenario and HUD tests already pin objective state, one completion chime,
  comms speaker formatting, and readout internals. They do not prove the
  composed mission HUD in a live app.
- Audio has focused coverage for routing, mixing, placement, throttling, hum
  levels, dry-fire edges, and teardown. It still has no composed ship-audio
  range.

The current unstaged simulation-defect work is not counted as landed proof.
Preserve it and let `20260909-214706` finish its own files and checks.

## Decisions

### Sprint composition

- A torpedo whose target dies does not retire. It drops the dead entity link and
  continues toward the frozen last-known position until it detonates or expires.
  Do not add the opposite assertion, and do not duplicate its focused test.
- Neutralizing a hull does not drop an existing combat lock. The player clears
  it manually. Do not add a neutralization-driven lock-drop assertion.
- A severed fragment is an obstacle for everyone. The former hull and its
  existing and future projectiles may hit it. Do not add owner immunity or a
  grace window.
- Settings UI and persistence belong to `20260909-213441`. An audio range may
  set resources directly but must not repeat the Settings UI journey.
- Audio teardown and command-flush races belong to `20260909-214706`.
- The comms queue cap and overflow policy belong to `20260909-213525`.
- Scenario reload, refusal, outcome, pause and focus lifecycle belong to
  `20260909-214629`.

### Range ownership

- Keep `system_hud_scales` as a narrow dedicated range. Do not replay every
  `system_hud_indicators` invariant.
- Extend the existing `system_collision_damage`; do not create another range
  with that name or a parallel collision range.
- `system_wreck_lock` uses three phases: an unrelated sever preserves the hull
  lock and pin; severing the pinned section clears only the pin; after an
  explicit acquisition of the fragment, the reticle and inset follow it and
  the same gun can hit it. The fragment is not selected automatically.
- Mount the same shipped lance prototype off-axis on the light and heavy
  railgun fixtures. Assert response and recovery against measured mass and live
  structural/controller limits, not a hand-authored angle threshold.
- Keep harness audio muted. Inspect live `SfxVoice` route, source, counts and
  requested loop levels. Do not require audible output or an audio device.
- Turret stow belongs in `system_turret_gunnery`, not `system_ship_audio`.

## Remaining ranges

### `system_hud_scales`

- [ ] Stage an inline one-section shuttle and `block_carrier` at two materially
      different window aspect ratios.
- [ ] Assert every visible screen indicator is contained by the viewport at
      both hull sizes and both shapes.
- [ ] Assert one component marker per attached section at minimum and capital
      scale.
- [ ] Assert the target inset frames the carrier's complete live hull.
- [ ] Assert the lead pip remains on the projected intercept at capital scale.
- [x] Do not repeat velocity-sphere containment. `system_hud_shell` already
      proves both shells enclose both reference hulls.
- [x] Do not repeat the generic lock, readout, dwell, GOTO, inset-presence or
      anchor-cleanup chain from `system_hud_indicators`.

### Existing `system_collision_damage`

- [x] A touch below the universal safe speed is free at both hull sizes.
- [x] A ship-to-ship ram spends hit points on both bodies.
- [x] The bite grows with closing speed.
- [ ] Add a ship-to-rock ram and assert both damage-bearing structures pay.
- [ ] Add a destructive ram and assert the destroyed section leaves a wreck.
- [ ] Pin same-rigid-body contacts never becoming self-damage with the cheapest
      focused test unless an exact test already exists. Do not manufacture a
      second live rigid body and call it the same hull.

### `system_wreck_lock`

- [ ] Lock a multi-section hull and pin one attached section.
- [ ] Sever a different section. Assert the combat lock and existing pin remain
      on the original hull.
- [ ] Sever the pinned section. Assert the component pin clears cleanly while
      the combat lock remains on the original hull.
- [ ] Assert the severed fragment appears as its own lockable close contact.
- [ ] Clear and explicitly acquire the fragment. Assert the reticle and inset
      follow that fragment.
- [ ] Fire the same gun and assert the fragment takes damage. This is projectile
      obstruction, not owner immunity.
- [x] Do not repeat generic fragment damageability or defeated-hull gun proof.
- [x] Do not assert that neutralization clears the combat lock.

### `system_torpedo_capital`

- [ ] Fire one bay at a large multi-section hull, then at an inline one-section
      drone.
- [ ] Assert the proximity fuze beats each ship hull it is closing on.
- [ ] Assert the warhead lands on the explicitly aimed section.
- [ ] Assert the capital absorbs the blast in the layer that was hit.
- [ ] Assert the one-section target is not overflown.
- [x] Do not repeat launcher-clearance, generic guidance, cruise, or gate
      detonation claims from `system_torpedo_launch`.
- [x] Do not assert that target death retires a torpedo. The opposite policy is
      intentional and already has focused proof.

### `system_railgun_hulls`

- [ ] Mount the shipped lance prototype at the same off-axis location on a light
      hull and a heavy hull.
- [ ] Measure live mass and assert translational recoil follows the inverse-mass
      relationship within a tolerance derived from fixed-step sampling.
- [ ] Assert the off-axis impulse produces the expected immediate rotational
      response without exceeding live structural/controller limits.
- [ ] Assert the controller reconverges the heading after recoil on both hulls.
- [ ] Assert the live bore sight and the slug's actual path agree on where the
      shot went.
- [x] Do not repeat one-shell reload coverage.
- [x] Do not repeat the lance's generic recoil, rake, charge, or penetration
      budget claims.

### `system_mission_hud`

- [ ] In one live scenario, post an objective with a world marker, one comms
      line, and a variable readout while the player flies.
- [ ] Assert the objective produces both its HUD chip and world marker.
- [ ] Assert the comms card displays its authored speaker attribution.
- [ ] Change the scenario variable and assert the visible readout follows it.
- [ ] Move or frame the objective offscreen and assert its edge indicator points
      toward the projected target direction.
- [ ] Kill the marker target and assert its marker hides.
- [x] Do not repeat scenario objective-state transitions or the focused
      completion-chime-once test.
- [x] Do not define or test comms queue overflow here.

### `system_ship_audio`

- [ ] Stage one hull firing, taking a burst of hits, burning its main drive, and
      dry-firing after its magazine empties.
- [ ] Assert an authored cue becomes a live `SfxVoice` on the route its authored
      track selects.
- [ ] Assert an exterior cue resolves to a spatial source relative to the live
      listener.
- [ ] Assert the per-source throttle collapses the staged hit burst to its
      allowed live voice count.
- [ ] Assert the engine-hum voice's requested level follows live throttle and
      returns toward silence when throttle is released.
- [ ] Assert one dry-trigger edge produces one live click request and a held
      empty trigger does not repeat it.
- [x] Keep `HarnessMute` active and do not assert audible output or sink-device
      behavior.
- [x] Do not drive the Settings UI, repeat mixer arithmetic, or cover teardown
      races here.

### Existing `system_turret_gunnery`

- [ ] Extend the live range with the shipped retractable turret's deploy, quiet
      delay, and completed stow cycle. Add named roster outcomes for the live
      behavior only; do not copy the state machine's focused unit assertions.

## Proof and workflow

- Every new assertion has an `outcome: <slug>` marker and matching
  `catalog_drift.rs` roster slug.
- Every new example has an explicit root `Cargo.toml` `[[example]]` block.
- Register ranges in the `combat` or `structure` shard from `20260909-213100`;
  do not redesign the shard split here.
- Reproduce any newly found defect before changing production code. Fix it in
  the same range lane and record the defect and fix in this task.
- Run only affected crate checks and ranges. Inspect rendered output for HUD
  work and `checks.json` plus `report.html` for every probe run.
- Run every affected range green three consecutive times in its shard before
  closing the task.
- Documentation and changelog changes are required only for a behavior defect
  found and fixed. Pure proof additions do not need a player-facing changelog
  entry.

## Done when

- Every remaining checkbox is complete or is dropped with evidence recorded
  here.
- No duplicate or contradictory assertion listed under the checked exclusions
  has been restored.
- All new ranges and extensions are cataloged, rostered, assigned to their
  existing shard, and green three times in a row.
- Every defect found by the ranges is fixed, proved, documented where behavior
  changed, and listed here.
