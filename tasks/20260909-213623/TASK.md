# Systems ranges: combat and destruction at both hull sizes

- STATUS: CLOSED
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

- [x] Stage an inline one-section shuttle and `block_carrier` at two materially
      different window aspect ratios.
- [x] Assert every visible screen indicator is contained by the viewport at
      both hull sizes and both shapes.
- [x] Assert one component marker per attached section at minimum and capital
      scale.
- [x] Assert the target inset frames the carrier's complete live hull.
- [x] Assert the lead pip remains on the projected intercept at capital scale.
- [x] Do not repeat velocity-sphere containment. `system_hud_shell` already
      proves both shells enclose both reference hulls.
- [x] Do not repeat the generic lock, readout, dwell, GOTO, inset-presence or
      anchor-cleanup chain from `system_hud_indicators`.

### Existing `system_collision_damage`

- [x] A touch below the universal safe speed is free at both hull sizes.
- [x] A ship-to-ship ram spends hit points on both bodies.
- [x] The bite grows with closing speed.
- [x] Add a ship-to-rock ram and assert both damage-bearing structures pay.
- [x] Add a destructive ram and assert the destroyed section leaves a wreck.
- [x] Pin same-rigid-body contacts never becoming self-damage with the cheapest
      focused test unless an exact test already exists. Do not manufacture a
      second live rigid body and call it the same hull.

### `system_wreck_lock`

- [x] Lock a multi-section hull and pin one attached section.
- [x] Sever a different section. Assert the combat lock and existing pin remain
      on the original hull.
- [x] Sever the pinned section. Assert the component pin clears cleanly while
      the combat lock remains on the original hull.
- [x] Assert the severed fragment appears as its own lockable close contact.
- [x] Clear and explicitly acquire the fragment. Assert the reticle and inset
      follow that fragment.
- [x] Fire the same gun and assert the fragment takes damage. This is projectile
      obstruction, not owner immunity.
- [x] Do not repeat generic fragment damageability or defeated-hull gun proof.
- [x] Do not assert that neutralization clears the combat lock.

### `system_torpedo_capital`

- [x] Fire one bay at a large multi-section hull, then at an inline one-section
      drone.
- [x] Assert the proximity fuze beats each ship hull it is closing on.
- [x] DROPPED, no such behavior exists. A torpedo may only be ordered at a
      ship ROOT (`commit_scripted_torpedo` filters the order through
      `q_ship_root.contains(target)`,
      `crates/nova_ship/src/sections/torpedo_section/scripted.rs:62`) and homes
      on the target's live structure anchor, not on a section
      (`update_target_position`,
      `crates/nova_ship/src/sections/torpedo_section/projectile.rs:43`). The
      per-section pin, `ComponentLock`, reaches TURRET aim only
      (`crates/nova_ship/src/input/player/intent.rs:100-129`). What the range
      asserts instead is where the warhead stops: on the face it reached, not
      on the aim point behind it.
- [x] Assert the capital absorbs the blast in the layer that was hit.
- [x] Assert the one-section target is not overflown.
- [x] Do not repeat launcher-clearance, generic guidance, cruise, or gate
      detonation claims from `system_torpedo_launch`.
- [x] Do not assert that target death retires a torpedo. The opposite policy is
      intentional and already has focused proof.

### `system_railgun_hulls`

- [x] Mount the shipped lance prototype at the same off-axis location on a light
      hull and a heavy hull.
- [x] Measure live mass and assert translational recoil follows the inverse-mass
      relationship within a tolerance derived from fixed-step sampling.
- [x] Assert the off-axis impulse produces the expected immediate rotational
      response without exceeding live structural/controller limits.
- [x] Assert the controller reconverges the heading after recoil on both hulls.
- [x] Assert the live bore sight and the slug's actual path agree on where the
      shot went.
- [x] Do not repeat one-shell reload coverage.
- [x] Do not repeat the lance's generic recoil, rake, charge, or penetration
      budget claims.

### `system_mission_hud`

- [x] In one live scenario, post an objective with a world marker, one comms
      line, and a variable readout while the player flies.
- [x] Assert the objective produces both its HUD chip and world marker.
- [x] Assert the comms card displays its authored speaker attribution.
- [x] Change the scenario variable and assert the visible readout follows it.
- [x] Move or frame the objective offscreen and assert its edge indicator points
      toward the projected target direction.
- [x] Kill the marker target and assert its marker hides.
- [x] Do not repeat scenario objective-state transitions or the focused
      completion-chime-once test.
- [x] Do not define or test comms queue overflow here.

### `system_ship_audio`

- [x] Stage one hull firing, taking a burst of hits, burning its main drive, and
      dry-firing after its magazine empties.
- [x] Assert an authored cue becomes a live `SfxVoice` on the route its authored
      track selects.
- [x] Assert an exterior cue resolves to a spatial source relative to the live
      listener.
- [x] Assert the per-source throttle collapses the staged hit burst to its
      allowed live voice count.
- [x] Assert the engine-hum voice's requested level follows live throttle and
      returns toward silence when throttle is released.
- [x] Assert one dry-trigger edge produces one live click request and a held
      empty trigger does not repeat it.
- [x] Keep `HarnessMute` active and do not assert audible output or sink-device
      behavior.
- [x] Do not drive the Settings UI, repeat mixer arithmetic, or cover teardown
      races here.

### Existing `system_turret_gunnery`

- [x] Extend the live range with the shipped retractable turret's deploy, quiet
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

## Delivery, 2026-09-15

Branch `combat-ranges`, from `8fe2f9c90`. Eleven files, +7307 / -147.

### Commits

| commit | subject |
| - | - |
| `f1864942a` | Range a ram on a rock and a ram that leaves a wreck |
| `ae78ca372` | Range the retractable mount over a whole stow cycle |
| `6f2787ace` | Range a wreck through lock, pin and fragment acquisition |
| `555170e02` | Range the mission HUD from posted objective to marker chip |
| `925410b52` | Range the ship's soundtrack from one burst to a closing throttle |
| `dadc548a8` | Range the HUD at both ends of the hull scale in two window shapes |
| `27ce16c6a` | Range one warhead onto a capital hull and onto the minimum hull |
| `56618ce08` | Range one lance on a light hull and a heavy one |
| `50ec874cf` | Census the ram contacts per fixed step |
| `9a10986a4` | Read the mission readout again after the mission moves it |

### Lanes

| lane | range | outcome | evidence |
| - | - | - | - |
| 1 | `system_collision_damage` | extended, 4 -> 7 claims | rock ram reads two ledgers (13.57 hp / 0.23 carve), the 800 m/s ram leaves 2 wrecks; same-body self-damage pinned by a focused test, not a second body |
| 2 | `system_turret_gunnery` | extended, 5 -> 8 claims | deploy, quiet delay and a completed stow cycle, live behavior only |
| 3 | `system_mission_hud` | new, 8 claims | chip, marker, range, edge chevron, comms voice, bound readout and its move, marker teardown |
| 4 | `system_ship_audio` | new, 5 claims | route, spatial source, burst throttle, dry-click edge, hum against throttle - `HarnessMute` left on throughout |
| 5 | `system_hud_scales` | new, 4 claims | containment, one marker per section, inset framing, lead pip, at 1280x600 and 1024x768 |
| 6 | `system_wreck_lock` | new, 6 claims | three phases, explicit fragment acquisition, never automatic |
| 7 | `system_torpedo_capital` | new, 4 claims | fuze stand-off both sizes, stop on the face, capital pays on that face, one-section hull not overflown |
| 8 | `system_railgun_hulls` | new, 5 claims | one lance on 33 kg and 2364 kg: station, inverse-mass step, off-axis spin inside live limits, heading recovery, sight/slug agreement |

Roster: `SYSTEMS_INVARIANTS` 364 -> 370, 47 slugs over the eight ranges, each
one seen in its own `timeline.jsonl`.

### Three consecutive direct runs on the final source

`DISPLAY=:99 ALSA_CONFIG_PATH=empty NOVA_AUTOPILOT=1 cargo run --features debug
--example <range>`, run one at a time on a quiet host, load sampled either side
of every run. Every run exited 0 and closed with `cycle complete, no panic`.

| range | run 1 | run 2 | run 3 | host load, min / max over the set |
| - | - | - | - | - |
| `system_collision_damage` | 19.1 s | 19.1 s | 19.1 s | 0.45 - 2.47 |
| `system_turret_gunnery` | pass | pass | pass | 1.98 - 3.02 |
| `system_mission_hud` | 2.7 s | 2.8 s | 2.8 s | 0.44 - 0.52 |
| `system_ship_audio` | 10.3 s | 10.3 s | 10.3 s | 1.78 - 2.40 |
| `system_hud_scales` | 1.8 s | 1.9 s | 1.9 s | 2.00 - 2.25 |
| `system_wreck_lock` | 9.5 s | 9.5 s | 9.6 s | 1.66 - 2.06 |
| `system_torpedo_capital` | 13.2 s | 13.2 s | 13.1 s | 1.60 - 2.16 |
| `system_railgun_hulls` | 16.7 s | 16.6 s | 16.8 s | 1.51 - 2.41 |

The times are the harness's own sim seconds, quoted as a reader and graded on
nothing. `system_railgun_hulls` is the set that measures mass: its readings are
identical across all three runs - light 33 kg, inertia 1.9512e2, dv 1.3717 u/s,
dspin 0.96022 rad/s; heavy 2364 kg, inertia 2.1957e5, dv 0.0190 u/s, dspin
0.00117 rad/s; a 72.07x velocity split against a 72.07x mass split, modelled
error 0.00e0 on both - with only the settled residuals moving in the fourth
decimal.

### Checks

| check | result |
| - | - |
| `cargo fmt --all -- --check` | clean |
| `cargo build --features debug --example <each of the 8>` | clean |
| `cargo test -p nova_probe_cli --test catalog_drift` | 2 passed, re-run after every roster change |
| `cargo test -p nova_gameplay --lib one_body_wearing_two_overlapping_colliders_never_damages_itself` | 1 passed |
| `probe run <the 8> --correctness-only` | 8/8 OK, 6/8 measured each, 0 invariant violations, 0 offending log lines, every artifact loadable |

Deliberate skips: no workspace-wide `cargo test` and no Clippy sweep (the full
suite exhausts this box); no frame-time or `--release` pass, because no range
here claims a frame cost - `capture_simulated` and `fps_within_baseline` report
`N/A - not claimed` on all eight.

### Defects found and fixed

1. `50ec874cf`, found by this lane's own extension. The contact census ran in
   `Last`, once a RENDER frame, while `deal_contact_impact_damage` runs once a
   fixed step after the solver
   (`crates/nova_gameplay/src/integrity/core.rs:109-112`). Every pre-existing
   staged contact presses for seconds and never noticed. The wrecking ram does
   not press: at 800 m/s it holds for about a dozen steps inside one
   software-rasterized frame, so the frame-end sample saw it once or not at
   all. Reproduced before the fix - runs 2 and 3 of three hung `let every pair
   meet` until the harness completion deadline expired
   (`deadline (120s) expired with collectors still pending: ["autopilot"]`),
   while run 1 recorded exactly one touching frame. After sampling with the
   damage system the ram reads 12 steps and peak 38 contacts, identically in
   three runs, and every other pair's count is stable too (17 / 9 / 14 / 23 /
   42). The counts are named `steps` now, because that is what they are.
2. `9a10986a4`, a proof gap rather than a game defect. The mission readout was
   read once, against the value the mission posted with it, so a strip that
   latched its first value would have passed. The variable is now moved the way
   `VariableSetActionConfig::action` moves it - `insert_variable` - and the row
   is read again: `SALVAGE 12.3` -> `SALVAGE 87.6`.

No production behavior changed, so no changelog entry is owed.

### Rendered output inspected

Captured with `NOVA_CAPTURE=1` under `Xvfb :99` and looked at, not just graded:

- `mission_hud.png` (1024x768): the green `SALVAGE 12.3` readout row and both
  amber objective rows (`HAUL THE PLATE HOME`, `CUT THE DERELICT FREE`) with
  hollow diamond bullets; the near mark's gold chip `DERELICT 600 m` standing
  over the rock; the far mark's chip `PLATE 9.49 km` pinned to the left edge
  under a chevron pointing off-frame; the comms card headed `MERIDIAN` reading
  the authored line. The centred marker chip overlaps the mission title band -
  they are independent layers and a mark at screen centre lands on the title.
- `hud_scales_wide.png` (1280x600) and `hud_scales_tall.png` (1024x768): the
  carrier inside a red target bracket that grows with the window, the target
  inset framing the whole carrier over `Fleet Carrier - NEUTRAL`, the component
  ring on the one-section shuttle, and the hint bar. Both frames keep every
  indicator on the window, which is what the range claims.

### For the HUD owner, not changed here

In `hud_scales_tall.png` the locked-target readout column (`DST` / `CLS` plus
its health bar) is drawn UNDER the target inset card and is unreadable; only a
sliver of its left border shows at the inset's edge. In the wide frame the same
readout is clear. The readout is a child of the reticle node at `left: 100%`
(`crates/nova_hud/src/torpedo_target.rs:9-12`), so it rides the reticle's right
edge with no awareness of the top-right inset. Nothing in this task claims it -
`every visible indicator lands on the live window` is a containment claim, and
both frames hold it - and where the readout should go when the reticle grows
into the inset is a HUD layout decision, so it is left for the HUD owner rather
than settled here.

### Shard

`20260909-213100` owns the `combat` / `structure` split and is still open, so
there is no such shard to register into. The shipped matrix is
`category: [screenshots, systems, playable]` (`.github/workflows/ci.yaml:245`)
and `probe run <category>` takes the example's DIRECTORY under `examples/`
(`.github/workflows/ci.yaml:224`), so all eight ranges join the `systems` shard
by living in `examples/systems/`. No CI file was edited. The CI pass gives each
example 300 s (`.github/workflows/ci.yaml:317`); the slowest of these measured
28 s.
