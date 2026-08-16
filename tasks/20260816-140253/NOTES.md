# NOTES

## The audit: every AI behaviour, and whether it honoured neutralization

Neutralization reaches the AI through ONE observer, `on_neutralized_stand_down`
(`crates/nova_ship/src/input/ai/mod.rs`), which inserts `AINonCombatant` on a
neutralized `AISpaceshipMarker` root. So "did this behaviour honour
neutralization" is really "did it read `AINonCombatant`, directly or through
something that does".

| Behaviour | System | Before | How |
| --- | --- | --- | --- |
| Primary target acquisition | `update_ai_target` | YES | Reads `Has<AINonCombatant>` and clears `AITarget` |
| Being targeted by others | `update_ai_target` candidates | YES | Reads `Has<NeutralizedMarker>` on candidates - nobody hunts a wreck |
| Behaviour state machine | `update_behavior_state` | YES, indirectly | Cleared `AITarget` -> no `hostile_distance` -> a passive state |
| Turret aim | `update_turret_target_input` | YES, indirectly | `!state.engages()` and no target -> writes `None` |
| Turret trigger | `on_projectile_input` | YES, indirectly | `!state.engages()` and no target -> writes `false` |
| Torpedo launch | `update_torpedo_section_input` | YES, indirectly | `engaged = state.engages() && ...` -> writes `false` |
| Torpedo commit-on-launch | `update_torpedo_target_input` | YES, vacuously | Nothing launches, so nothing commits |
| Chase / helm | `update_controller_target_rotation_torque` | YES, indirectly | `!state.engages()` -> the command freezes |
| Chase / thrust | `on_thruster_input` | YES, indirectly | No target and no engaging state -> explicit `0.0` |
| Combat mirror | `mirror_ai_combat_state` | NO (fixed downstream) | Published `CombatLock`/`WeaponsRaised` off the PD target, so a wreck stayed HOT |
| **Ship-wide point defence** | **`update_point_defense_target`** | **NO** | **Selected on `AISpaceshipMarker` alone. FIXED.** |
| **Per-turret point defence** | **`update_turret_point_defense`** | **NO** | **Same. FIXED.** |
| PD slot insertion | `insert_turret_defense_target` | n/a | Deliberately left ungated - see below |
| Burst cadence | `update_fire_cadence` | n/a | A free-running clock with no target to fire at |
| Threat memory | `on_damage_track_threat` | n/a | Records the hit; cannot re-engage without a target |
| Passive routine | `update_passive_flight` | NOT COMBAT - see "Left alone" | |
| Gravity opt-in | `insert_gravity_affected_on_ai_ship` | n/a | A wreck should still fall |

The pattern is stark. Everything that hangs off the primary `AITarget` was
already correct for free, because clearing the target drops the behaviour state
into a passive one and every gun system gates on `state.engages()`. Point
defence was the one behaviour that DELIBERATELY bypasses the behaviour state -
that is documented on `AIEngageGrace`, "a graced ship still swats inbound
ordnance (the PD path deliberately bypasses behavior states)" - so the passive
routine that silenced everything else could not silence it. Point defence
landed this release; it was written against a rule the rest of the AI expresses
implicitly, and there was nothing to trip over.

`mirror_ai_combat_state` was a second-order casualty, not a separate bug: it
computes `engaged = pd_target.or(target)`, so the stale PD pick kept a wreck's
`CombatLock` set and its stance raised. Clearing the PD pick fixes it with no
edit; the mirror still RUNS on a neutralized ship, deliberately, because it is
what publishes the cleared lock and lowers the stance.

## The design call: the observer names the rule, the queries read the name

The observer CANNOT carry the whole rule, and this is the honest reason:
`AITarget`, `AIPointDefenseTarget` and every mount's `AITurretDefenseTarget` are
recomputed from the world EVERY FRAME. A one-shot clear at the moment of
neutralization is overwritten on the next tick. There is no state to remove
either - removing `AIPointDefenseTarget` would only drop the ship out of a
`&mut` query while `update_turret_point_defense` (which does not read it) kept
assigning mounts.

Removing `AISpaceshipMarker` on neutralization was considered and rejected. It
reads well ("the AI is gone") and would drop the ship out of every AI query at
once, but the gun systems would then skip it via `q_spaceship.get(*ship) ->
Err -> continue`, which LATCHES the last `TurretSectionInput`. A turret firing
at the instant of neutralization would hold its trigger forever. Same objection
applies to adding `Without<AINonCombatant>` to the gun and torpedo queries:
those systems write an explicit "hold fire", and skipping them would trade a
released trigger for a latched one.

So: the observer stays the single place that says "the crew is gone", and
`AINonCombatant` is the single NAME for it. Two acquisition systems read that
name; everything downstream falls out of the empty picks they leave behind.
Two edits, both one query term:

- `update_point_defense_target` (`acquisition.rs`) - skips a non-combatant and
  CLEARS its `AIPointDefenseTarget`, mirroring what `update_ai_target` already
  does for `AITarget`. Clearing, not skipping: a ship neutralized mid-intercept
  has to let go of the torpedo it was tracking.
- `update_turret_point_defense` (`point_defense.rs`) - `q_ship` gains
  `Without<AINonCombatant>`, so a wreck builds no threat list. Both existing
  passes then hand its mounts `None` for free: pass one finds no threat map
  entry and drops the held target, pass two finds no pick.

`insert_turret_defense_target` is deliberately NOT gated. A turret with no
`AITurretDefenseTarget` component falls back on the ship-wide
`AIPointDefenseTarget`; keeping the slot present is what guarantees an explicit
`None` regardless of that fallback. Denying wrecks the slot would have been the
riskier edit, not the safer one.

The rule now also covers the unarmed `AINonCombatant` hauler, which is strictly
more correct: a ship that cannot fight should not be running fire control.

## What was deliberately LEFT ALONE

**A neutralized hull still flies its passive routine.** With its target cleared
it drops to Idle/Patrol/Orbit and `update_passive_flight` engages the autopilot -
an Idle wreck still runs a station-keeping stop burn. That is the DOCUMENTED
meaning of `AINonCombatant` ("flies its passive routine but never fights"), and
`web/src/wiki/dev/sections.md` describes a neutralized ship as "a derelict to
board, salvage or let limp away", which reads as a hull that can still move.

Making the wreck inert is a separate behavioural decision with real staging
consequences - every scenario ship that is neutralized would stop holding
station and start drifting - so it is flagged for the owner rather than taken
silently. If the answer is "a wreck drifts", the change is one gate on
`update_passive_flight` plus dropping the authored `AIPatrolRoute` /
`AIOrbitDirective` in the same observer.

Also unchanged, on purpose:

- A neutralized PLAYER hull. The observer is keyed on `AISpaceshipMarker`, so a
  player never gains `AINonCombatant`; no AI system touches a player ship, and
  player turrets are aimed and fired by the player. The player is their own
  crew. `only_a_neutralized_ai_ship_stands_down` still pins that split.
- What the scenario layer counts. `OnNeutralizedEvent`, `OnDefeatedEvent` and
  `DefeatedMarker` are all written by `integrity::neutralize`, which this task
  did not touch. The live run asserts it directly (below).

## Evidence

### The bug reproduced

`examples/systems/neutralized_quiet.rs` run against the code with the two query
gates reverted:

```
autopilot: step `assert the wreck is quiet` begins
thread 'main' panicked at examples/systems/neutralized_quiet.rs:
assertion `left == right` failed: neutralized_quiet: a wreck defends against nothing
```

and its snapshot of the live hull, for the "before" reading:

```
raider neutralized=false lock=Torpedo Projectile hot=true
  turret defense_target=Torpedo Projectile firing=true
```

### The fix, live

Same example, with the fix, on the merged tree:

```
Xvfb :77 -screen 0 1280x720x24 &
DISPLAY=:77 BEVY_ASSET_ROOT="$PWD" NOVA_AUTOPILOT=1 \
  NOVA_PERF_SNAPSHOT=/tmp/wreck.jsonl \
  nix develop --command cargo run --example neutralized_quiet --features dev
```

```
neutralized_quiet: live hull defending - mount on torpedo Some(1204v0), trigger down
detect_neutralized: entity 1104v0 neutralized (id: "raider", type: "spaceship")
Variable: raider_neutralized = Number(1.0)
neutralized_quiet: the wreck is quiet with a torpedo 89 u out (3 in the envelope, 1 live mount(s), gun intact)
neutralized_quiet: the wreck still bleeds (199.99 -> 194.99), still defeated once
autopilot: cycle complete, no panic (t=9.7s)
```

The run takes two world snapshots. The raider, before and after:

| | live hull | wreck |
| --- | --- | --- |
| `neutralized` | false | true |
| `combat_lock` | Serpent Torpedo | null |
| `weapons_hot` | true | false |
| turret `defense_target` | Serpent Torpedo | null |
| turret `firing` | true | false |
| turret ammo | 493 / 500 | 473 / 500 |
| ordnance in flight | 13 | 33 |

The ammunition column is the point: the wreck's gun is loaded, working and
NOT firing, with 33 objects in flight and three torpedoes inside its 150 u
point-defence envelope.

### Why the range proves what it claims

The example is built so nothing but the stand-down can explain the silence:

- The raider is neutralized by killing its FLIGHT COMPUTER, not its gun. The
  turret section is asserted still present and still free of
  `SectionInactiveMarker` at the moment of the quiet claim, so this is not a
  disarmed ship going quiet for the boring reason.
- Its authored `engage_range` is 1 u, so it never engages the boat and spends
  the entire run in a passive state - the exact state point defence is
  documented to fire in anyway. The passive routine therefore cannot be
  mistaken for the fix.
- The torpedoes are position-committed to a point 400 u BEYOND the raider,
  30 u off its flank, so they fly through its envelope and fuze far away. The
  wreck is never damaged by them and the stream never stops.
- The quiet assertion FAILS if no torpedo is inside the envelope, so a run that
  stopped staging an intercept cannot pass on an empty sky.

### Checks

All re-run after `git merge master` (which brought in the torpedo-type split -
hence "Serpent Torpedo" in the table above).

- `cargo check --workspace --all-targets` - clean.
- `cargo fmt --check` - clean.
- `cargo test --lib -p nova_ship -p nova_gameplay` - 616 + 142 pass.
- `cargo test -p nova_probe_cli --test catalog_drift` - pass (the new example is
  cataloged).
- NOT run, per the lane brief: the full workspace suite and clippy. The known
  `nova_authoring` shakedown-walk failures on master belong to another lane.

## Tests added

- `acquisition::point_defense_tests::a_neutralized_hull_stops_defending_itself` -
  end to end through the observer and the production chain order: the live hull
  takes the inbound and holds its trigger, the marker lands, and the trigger,
  the aim, the ship pick and the mount pick all go empty while the torpedo
  flies on.
- `point_defense::tests::a_non_combatant_hull_drops_every_mount_it_held` - the
  unit-level half: an existing assignment CLEARS rather than sticking.
- `examples/systems/neutralized_quiet.rs` - the live harness above.

## Next time

Point defence is the only AI behaviour that reads no behaviour state. That is a
deliberate and correct design, but it means it will keep missing rules the rest
of the AI expresses through the state machine. The `AINonCombatant` doc comment
now carries that warning next to the list of who reads it.
