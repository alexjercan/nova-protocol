# Travel/combat lock slots: TravelLock + CombatLock, seed-on-raise, view-routed consumers

- STATUS: OPEN
- PRIORITY: 55
- TAGS: v0.5.0, targeting, input, hud, spike

## Goal

Split the single lock into two slots per spike 20260712-222610 (rounds
1-3): a TRAVEL lock (auto-cast from the live look ray, sticky,
scroll-cycled while lowered, feeds GOTO) and a COMBAT lock (seeded on
raise by an incumbent-hysteresis rule, enemy-only scroll while raised,
persists when lowered, feeds guns/torpedoes/fine-lock/inset). Also
carries the componentization (resources port straight to the end-state
shape) and the look-ray re-sourcing that round 3 proved necessary.

Body rewritten after the round-3 adversarial review; the earlier layered
notes live in git history and the spike.

## Steps

- [ ] Look-ray re-sourcing (round 3 delta 1, feasibility B1/B3/M3): add an
      "active look ray" accessor - the `PointRotationOutput` of the rig
      currently holding `SpaceshipRotationInputActiveMarker`
      (camera_controller.rs:575-631). Travel casting uses it every frame;
      raise-frame seeding uses it on the press frame (the marker still
      sits on the outgoing rig that frame - that IS the live look). The
      turret slewing feed keeps the turret rig. Also seed the turret rig
      from the ACTIVE rig on Turret entry instead of the Normal rig
      (camera_controller.rs:586/:623-628), so raising out of FreeLook
      aims where the player looks. Test rigs must model the SPLIT rigs
      faithfully (today's tests spawn one both-marker entity,
      targeting.rs:1029-1032/:1097-1101, masking the divergence - retro
      rule: non-faithful rigs are not evidence).
- [ ] Raised state (round 3 delta 2, feasibility M2): a public
      weapon-raised flag written ONLY by `Start`/`Complete<CombatInput>`
      (camera_controller.rs:699-711; keep `CombatInput` private by
      writing the flag from those observers). All gameplay routing reads
      RAISED, never `SpaceshipCameraControlMode` (Alt-tap during RMB
      corrupts the enum; it also lacks PartialEq,
      camera_controller.rs:79-85). Latch transitions across pause (mode
      observers are pause-ungated, the consumers are pause-gated -
      feasibility m3). The enum's own restore-on-nested-release bug is
      noted as a separate camera task candidate, not fixed here.
- [ ] Componentize straight to the end state: replace
      `SpaceshipPlayerTargetLock` (targeting.rs:72, registered :91) with
      `TravelLock { target: Option<Entity>, designated: bool }` and
      `CombatLock(Option<Entity>)`; replace
      `SpaceshipPlayerTargetCandidates` (targeting.rs:235, :92) with
      `AvailableTargets` (travel list; entries + pinned_until). Add
      `HostileContacts` (all-directions hostile combat targets,
      angle-then-distance; feeds combat scroll AND edge indicators). All
      on the ship root via `#[require]` on `PlayerSpaceshipMarker`
      (player.rs:288-290). Port surface (feasibility m2): targeting.rs,
      player.rs, hud/{torpedo_target,target_candidates,edge_indicators,
      target_inset,component_lock}.rs, examples/12_hud_range.rs.
      GOTCHAS: verb hints (player.rs:157-281) must keep RUNNING shipless
      (no ship -> no keys -> hints clear; a ship Single would freeze
      them); `drive_reticle_anchor` (torpedo_target.rs:245-251) must
      write None on an empty query, not early-return; HUD teardown on
      ship despawn already exists (hud/mod.rs:217-229). Port tests from
      `insert_resource` setups to components on the spawned ship.
- [ ] TRAVEL side: empty slot -> auto-cast the nearest-to-ray lockable
      body inside the 18 deg pick cone of the ACTIVE look ray (signature
      range gates intact, no extra distance cap), `designated = false`.
      Scroll (lowered) steps the wide-cone (~50 deg const knob)
      angle-ranked `AvailableTargets` and sets `designated = true`.
      Sticky: aim wander never moves it; clears on death/despawn/range.
      The 550 m direction-blind hostile fallback does NOT apply to travel
      (deliberate slot); record its final disposition in this task when
      implemented.
- [ ] COMBAT side: maintain `HostileContacts` (order from the turret ray
      while raised - the only time it is consumed for ordering).
      Seed/re-seed ON RAISE by the incumbent-hysteresis rule (spike
      rounds 2b + 3 deltas 5-6): evaluate the best enemy by angle from
      the press-frame look ray over a CONE/ON-SCREEN pool; the incumbent
      (current CombatLock, else the hostile TravelLock IF designated or
      in-cone) holds unless a challenger is clearly nearer (cos-ratio
      band const). A NON-hostile TravelLock inside the TIGHT pick cone
      at raise seeds instead (deliberate raise on a neutral/friend/rock -
      the only path that puts a non-hostile in the combat slot).
      Committed torpedoes are excluded from ALL automatic pools (scroll
      still reaches them). Clears: death, out-of-range, allegiance flip
      to non-hostile, optional ~20 s lowered-decay const (flag).
- [ ] Scroll routing on RAISED (builds on 20260712-223034): lowered ->
      travel step; raised -> combat step through `HostileContacts`
      (angle order continuing past the cone edge - tail threats stay
      reachable). Precedence (round 3 delta 9): scroll sets lock + 4 s
      pin + freezes order; a raise re-seed that switches REPLACES the
      pin; auto-seed only fills an EMPTY slot and sets no pin; a valid
      lock is never auto re-picked. Add a small debounce const so a
      wheel flick spanning the raise/lower transition does not land on
      the wrong slot (UX m2).
- [ ] Auto-seed-on-kill (const flag, default on): when the CombatLock
      dies while RAISED, seed the next enemy by angle from the CURRENT
      pool, ON-SCREEN only; while lowered, the slot stays empty. (The
      held-trigger interrupt lives in 20260712-223036.)
- [ ] Consumer routing: G/GOTO + verb hints (player.rs:232/:841) ->
      TravelLock; turret feed (player.rs:361), torpedo commit
      (player.rs:459), focus dwell (targeting.rs:606), component
      fine-lock (targeting.rs:665) -> CombatLock; inset view -> CombatLock,
      else TravelLock (friendly inspection without combat-locking).
- [ ] HUD baseline: reticle = CombatLock; NEW distinct travel
      chevron/diamond = TravelLock; candidate brackets render the active
      context (lowered: travel list; raised: enemy order); edge
      indicators (hud/edge_indicators.rs:262) -> `HostileContacts` +
      CombatLock + off-screen TravelLock arrow; "guns hot on <target>"
      banner whenever a CombatLock exists while lowered (UX M8).
      Unmistakable slot distinction is IN scope; polish is not.
- [ ] Tests (state-per-step for gestures; split camera rigs modeled
      faithfully): travel cast follows the look ray in Normal AND
      FreeLook; raise out of FreeLook seeds toward the flanker being
      looked at; seed cases - designated hostile incumbent holds /
      stale undesignated behind-you hostile loses to the on-screen enemy /
      empty-space raise keeps the designated incumbent / non-hostile
      travel lock in tight cone seeds; scroll routes per raised state;
      pin vs raise precedence; auto-seed only when raised+on-screen+empty;
      combat lock persists on lowering; allegiance flip clears it;
      G reads travel while combat points elsewhere; torpedo commits on
      CombatLock; behind-player hostile in `HostileContacts`, absent
      from `AvailableTargets`, edge arrow shown.
- [ ] cargo fmt + cargo check + run targeting/input/hud test modules.

## Notes

- Spike: docs/spikes/20260712-222610-travel-combat-lock-slots.md - rounds
  2b (incumbent rule, user-confirmed), 2c (raised-gating), 3 (adversarial
  deltas). Depends on: 20260712-223034.
- RECORDED LOSS pending user ack (round 3 delta 8): guided torpedoes at
  nav bodies die with the split - CombatLock never holds asteroids/
  beacons; torpedo-at-rock becomes dumb-fire.
- Playtest knobs as consts: wide-cone half-angle, hysteresis band, decay
  seconds + flag, auto-seed flag, debounce.
