# Travel/combat lock slots: TravelLock + CombatLock, seed-on-raise, view-routed consumers

- STATUS: OPEN
- PRIORITY: 55
- TAGS: v0.5.0, targeting, input, hud, spike

## Goal

Split the single lock into two slots per the two-slot model (spike
20260712-222610): a TRAVEL lock (Normal/FreeLook: auto-cast from the aim
cone at any gated distance, sticky, scroll-cycled, feeds GOTO) and a COMBAT
lock (seeded from the travel lock on RMB raise, enemy-only scroll ordering,
persists across view exits, feeds guns/torpedoes/fine-lock/inset). The view
routes scroll and HUD; guns never read the travel slot. This task ALSO
carries the componentization (absorbed from closed 20260712-215957): the
resources port straight to the end-state component shape, no intermediate
neutral component.

## Steps

- [ ] Componentize straight to the end state: replace the
      `SpaceshipPlayerTargetLock` resource (targeting.rs:72, registered
      :91) with `TravelLock` and `CombatLock` components (Option<Entity>
      newtypes, resource-matching derives) and the
      `SpaceshipPlayerTargetCandidates` resource (targeting.rs:235, :92)
      with an `AvailableTargets` component (travel list; entries +
      pinned_until), all on the ship root - prefer
      `#[require(...)]` on `PlayerSpaceshipMarker` (player.rs:288-290
      already requires SpaceshipRootMarker + Allegiance). Add
      `HostileContacts` (all-directions hostile combat targets,
      angle-to-aim then distance, rebuilt by the acquisition pass) - it
      serves BOTH the combat scroll ordering and the edge indicators.
      GOTCHA: resources always exist, components only while a player ship
      does - every consumer must degrade to its no-lock path when the
      query is empty. Port tests from `world.insert_resource(...)` setups
      (e.g. targeting.rs:1025-1026, edge_indicators.rs:419-420) to
      component insertion on the spawned ship.
- [ ] Verify `PointRotationOutput` (targeting.rs:314) follows the FreeLook
      camera swivel; travel casting must track what the player looks at in
      Normal AND FreeLook. If it does not, source the travel aim from the
      active camera rig.
- [ ] Split acquisition (`update_spaceship_target_input`, targeting.rs:313):
      TRAVEL - when slot empty, nearest-to-ray in the 18 deg pick cone,
      signature gates intact, no extra distance cap (20 km play-area max);
      sticky once held (aim wander never moves it; death/despawn clears);
      wide-cone (~50 deg knob) angle-ranked `AvailableTargets` maintained
      for the scroll. The 550 m hostile signature fallback moves to the
      COMBAT side or is dropped for travel (travel is deliberate; decide in
      implementation, record which). COMBAT - maintain `HostileContacts`
      order; clear the slot on death/out-of-range only.
- [ ] Seed-on-raise: when `SpaceshipCameraControlMode` transitions to
      `Turret` (watch the resource - `CombatInput` is private to
      camera_controller.rs:651; make it pub only if resource-watching
      proves insufficient) and CombatLock is empty: seed from TravelLock if
      it is combat-eligible (ship or committed torpedo, ANY allegiance);
      else leave empty.
- [ ] Route the scroll observers by view: Normal/FreeLook -> step TravelLock
      through `AvailableTargets`; Turret -> step CombatLock through
      `HostileContacts` (cone-first angle order, continuing past the cone
      edge so behind-you threats stay reachable - spike refinement, flagged
      with the user). Keep per-slot pinned_until order-freezing during a
      cycle burst.
- [ ] Route the consumers: G/GOTO + verb hints (player.rs:232, :841) ->
      TravelLock; turret feed (player.rs:361), torpedo commit
      (player.rs:459), focus dwell (targeting.rs:606), component fine-lock
      (targeting.rs:665), inset view -> CombatLock.
- [ ] HUD baseline: combat reticle follows CombatLock (existing reticle);
      NEW distinct travel marker (chevron/diamond) for TravelLock;
      candidate brackets render the ACTIVE view's list; edge indicators ->
      CombatLock + `HostileContacts` + off-screen TravelLock arrow
      (hud/edge_indicators.rs:262 switches source). Full HUD polish stays
      out of scope; unmistakable slot distinction is IN scope.
- [ ] Tests (state-per-step where gestures are involved): travel auto-cast
      picks the aimed body at range; travel lock sticky against aim wander;
      travel scroll walks the wide-cone list; seed-on-raise eligible /
      ineligible / already-locked cases; combat scroll walks enemies by
      angle incl. one behind the player; combat lock persists on view exit;
      G engages GOTO on TravelLock while CombatLock points elsewhere;
      torpedo commits on CombatLock; behind-player hostile present in
      `HostileContacts`, absent from `AvailableTargets`, edge arrow shown.
- [ ] cargo fmt + cargo check + run targeting/input/hud test modules.

## Notes

- Spike: docs/spikes/20260712-222610-travel-combat-lock-slots.md (resolved
  semantics + open questions); carries forward the cone-list mechanics from
  the superseded 20260712-215402 and the componentization plan from the
  superseded 20260712-215957 (its TASK.md holds the original
  consumer-by-consumer port checklist - reuse it).
- Depends on: 20260712-223034 (scroll rebind).
- Fire gating and the unlock key are NOT here - task 20260712-223036.
- Combat-eligibility for seed-on-raise = `is_combat_target` (ship or
  committed torpedo), allegiance-blind (deliberate raise on a friend is
  allowed; fire gating is the safety).
- Clutter knobs (wide-cone half-angle, 5-cap, travel nearest-N alternative)
  are playtest questions - keep them consts next to
  `TARGETING_CONE_HALF_ANGLE_DEG` (targeting.rs:128).

## Round 2 refinements (2026-07-12, spike round 2 - these override the
matching bullets above)

- Seed-on-raise is HOSTILE-FIRST, not allegiance-blind: empty combat lock +
  RMB -> hostile travel lock if any, else best enemy by angle-from-aim,
  else empty. A friendly travel lock is never seeded into combat.
- Re-seed on EVERY raise with aim hysteresis: a raise with a live combat
  lock re-seeds only when a different enemy is clearly nearer the aim
  (cos-ratio band, component-snap style ~0.75; feel knob).
- Auto-seed next enemy (angle order) when the combat lock dies WHILE in
  Turret view; empty when it dies outside. Behind a const flag for
  playtest.
- Inset view priority: combat lock, else travel lock (friendly inspection
  without combat-locking).
