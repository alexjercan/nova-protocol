# Review: Contextual keybind hints - resolver, cluster, anchored cues

- TASK: 20260710-174646
- BRANCH: keybind-hints

## Round 1

- VERDICT: REQUEST_CHANGES

Reviewed commit 59899b5 against master with an independent adversarial
pass. Sound: the change guard (PartialEq, no floats), stale-anchor
handling (indicators hide on unresolved entities; lock recomputed before
the resolver), set ordering (input before HUD, same-frame hints), the
absorption (no OrbitAvailable leftovers anywhere), and the tests match
their claims. Findings:

- [x] R1.1 (MAJOR) input/player.rs (update_flight_verb_hints) - lit hints
  lie on a crippled ship. autopilot_system disengages immediately without
  a live flight computer or live engines, but availability is "ship
  exists" / "lock exists" / "well && !orbiting": on a ship with a dead
  controller the cluster shows lit [X]/[G]/[O] whose presses are visible
  no-ops - violating VerbHint::available's own doc ("pressing the key
  right now would do something"). Fix: AND in the capability the flight
  layer requires (a live controller section with a PD and at least one
  live thruster, mirroring ship_turn_rate's filters); CANCEL stays
  engagement-only (Z always works).
  - Response: fixed - availability now requires a flyable ship (live controller with PD + live thruster child, mirroring ship_turn_rate's filters) for STOP/GOTO/ORBIT; CANCEL stays engagement-only. New truth-table arm: dead controller grounds everything but Z.

- [x] R1.2 (MINOR) hud/keybind_hints.rs (drive_orbit_cue/drive_goto_cue)
  - both write Text/anchor unconditionally every frame (per-frame text
  relayout + two format! allocations while showing), while the cluster
  system next to them got the is_changed guard. Gate both on the
  resource change (their only input) with compare-before-write semantics.
  - Response: fixed - both cue drivers gate on hints.is_changed() with an Added<marker> escape; no writes on quiet frames.

- [x] R1.3 (MINOR) hud/keybind_hints.rs (update_hint_cluster) - the
  is_changed guard races row spawning: rows that appear while the
  resource is quiet stay empty until the next transition. Add an
  Added<KeybindHintRow> (and cue-marker) escape to the guards.
  - Response: fixed - Added<KeybindHintRow> (and cue markers) escape the change guard, so fresh spawns render immediately.

- [x] R1.4 (MINOR) hud/mod.rs (setup_hud_flight_status) - the cluster and
  cue layers are global singletons spawned per Add<PlayerSpaceshipMarker>
  with no one-instance guard (the flight rig has one); a second Add
  stacks duplicates. Mirror the rig's q_existing guard.
  - Response: fixed - setup_hud_flight_status spawns the cluster/cues only when none exist (same one-instance guard as the flight rig).

- [x] R1.5 (MINOR) input/player.rs - stop.available uses !q_ship.is_empty()
  while everything else derives from q_ship.single(): with two player
  ships STOP stays lit while the Single-based observer no-ops. Derive all
  four from the same single() result.
  - Response: fixed - all four verbs derive from the same q_ship.single() result; two ships now ground everything, matching the Single-based observers.

- [x] R1.6 (NIT) hud/keybind_hints.rs (drive_goto_cue) - cancel.available
  as the "engaged" proxy is correct today but silently couples two verbs'
  semantics; an explicit engaged flag on FlightVerbHints costs one field.
  Also noted: with GOTO engaged and the lock lost, the dim GOTO row still
  toggles the trip off on G - accepted as the toggle semantics (the row
  advertises engagement value, not toggle-off value).
  - Response: fixed - FlightVerbHints gains an explicit engaged flag; the goto cue reads it instead of proxying cancel.available. The dim-GOTO-row toggle-off note is accepted as-is.

- [x] R1.7 (NIT) input/player.rs - per-frame label allocation (~8 strings)
  discarded in the common unchanged case; accepted per the task notes
  (bindings are static today), noted for a future Changed<Binding> cache
  if profiling ever cares.
  - Response: accepted - per-frame label allocation stays (bindings are static, ~8 small strings); recorded for a Changed<Binding> cache if profiling ever cares.

## Round 2

- VERDICT: APPROVE

Verified every response against the new diff: the capability gate is in
with its truth-table test (dead controller grounds STOP/GOTO/ORBIT, Z
survives), the guards carry Added escapes, the singleton spawn guard
mirrors the rig, availability derives from one single() result, and the
engaged flag replaces the proxy. input (111), hud (50) green; fmt + check
--workspace --examples clean. No new findings; ready to land.
