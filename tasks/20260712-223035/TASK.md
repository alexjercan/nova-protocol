# Travel/combat lock slots: TravelLock + CombatLock, seed-on-raise, view-routed consumers

- STATUS: OPEN
- PRIORITY: 55
- TAGS: v0.5.0, targeting, input, hud, spike

## Goal

Split the single lock into two slots per spike 20260712-222610 (rounds
1-3): a TRAVEL lock (auto-cast from the live look ray, sticky,
scroll-cycled while lowered, feeds GOTO) and a COMBAT lock (seeded on
raise by an incumbent-hysteresis rule, enemy-only scroll while raised,
persists when lowered, feeds guns/torpedoes/fine-lock/inset). The
look-ray and mode/raised-state infrastructure this builds on lands first
in task 20260712-231141; this task consumes it. Also carries the
componentization (resources port straight to the end-state shape).

Body rewritten after the round-3 adversarial review; the earlier layered
notes live in git history and the spike.

## Steps

- [ ] Consume the 20260712-231141 infrastructure: travel casting and the
      wide-cone list use the ACTIVE look ray every frame (live in
      Normal/FreeLook per user directive); raise-frame seeding evaluates
      against the press-frame look (the accessor's documented property);
      all routing reads the RAISED flag, never the camera enum. Turret
      slewing keeps the turret rig. Extend the faithful split-rig test
      fixtures from 231141 rather than re-inventing single-rig ones.
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
- [ ] COMBAT side (round-4 decisions): maintain `HostileContacts`
      (edge-indicator threat set + auto-seed pool ONLY - combat scroll
      does NOT walk it). Seed/re-seed ON RAISE by the incumbent-
      hysteresis rule (spike rounds 2b + 3 deltas 5-6): evaluate the
      best enemy by angle from the press-frame look ray over a
      CONE/ON-SCREEN pool; the incumbent (current CombatLock, else the
      hostile TravelLock IF designated or in-cone) holds unless a
      challenger is clearly nearer (cos-ratio band const). A NON-hostile
      TravelLock inside the TIGHT pick cone at raise seeds instead
      (aimed-raise, questionnaire Q4). Committed torpedoes are excluded
      from ALL automatic pools (scroll still reaches them). Clears:
      death, out-of-range, allegiance flip to non-hostile, optional
      ~20 s lowered-decay const (flag).
- [ ] Scroll routing on RAISED (builds on 20260712-223034; round-4
      decision 1: ONE pool, two slots): both modes' scroll walks the
      SAME `AvailableTargets` cone list (wide cone of the ACTIVE look
      ray - normal/freelook ray lowered, turret ray raised; STRICT cone,
      no past-edge reach, questionnaire Q3; all classes including
      neutrals/friendlies - "Combat mode is Combat mode"). RAISED
      decides the slot written: lowered -> TravelLock (+designated),
      raised -> CombatLock. Precedence (round 3 delta 9): scroll sets
      lock + 4 s pin + freezes order; a raise re-seed that switches
      REPLACES the pin; auto-seed only fills an EMPTY slot and sets no
      pin; a valid lock is never auto re-picked. Add a small debounce
      const so a wheel flick spanning the raise/lower transition does
      not land on the wrong slot (UX m2).
- [ ] Auto-seed-on-kill (const flag, default on): when the CombatLock
      dies while RAISED, seed the next ENEMY by angle (from
      `HostileContacts`, hostile-only - deliberate scroll is the only
      path to non-hostiles), ON-SCREEN only; while lowered, the slot
      stays empty. Default ON (questionnaire Q2). (The held-trigger
      interrupt lives in 20260712-223036.)
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
- [ ] Tests (state-per-step for gestures; on the 231141 split-rig
      fixtures): travel cast follows the look ray in Normal AND
      FreeLook; raise out of FreeLook seeds toward the flanker being
      looked at; seed cases - designated hostile incumbent holds /
      stale undesignated behind-you hostile loses to the on-screen enemy /
      empty-space raise keeps the designated incumbent / non-hostile
      travel lock in tight cone seeds; scroll routes per raised state;
      pin vs raise precedence; combat scroll reaches a neutral rock in
      the cone and a guided torpedo then commits to it (round-4 reversal
      of the recorded loss); combat scroll does NOT reach an off-cone
      enemy (strict cone); auto-seed only when raised+on-screen+empty
      and only onto hostiles;
      combat lock persists on lowering; allegiance flip clears it;
      G reads travel while combat points elsewhere; torpedo commits on
      CombatLock; behind-player hostile in `HostileContacts`, absent
      from `AvailableTargets`, edge arrow shown.
- [ ] cargo fmt + cargo check + run targeting/input/hud test modules.

## Notes

- Spike: docs/spikes/20260712-222610-travel-combat-lock-slots.md - rounds
  2b (incumbent rule, user-confirmed), 2c (raised-gating), 3 (adversarial
  deltas), 4 (user directives + questionnaire decisions).
- Depends on: 20260712-223034 (scroll rebind) AND 20260712-231141
  (look-ray/mode infrastructure).
- Round-4 reversal: the once-recorded loss (guided torpedoes at nav
  bodies) is GONE - deliberate combat scroll locks any cone member, so
  torpedo-at-rock stays guided via scroll+launch.
- Playtest knobs as consts: wide-cone half-angle, hysteresis band, decay
  seconds + flag, auto-seed flag, debounce.
