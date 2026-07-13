# Travel/combat lock slots: TravelLock + CombatLock, seed-on-raise, view-routed consumers

- STATUS: CLOSED
- PRIORITY: 55
- TAGS: v0.5.0, targeting, input, hud, spike, wontdo

## Goal

Split the single lock into two slots per the FINAL v0.5 design (spike
20260712-222610, rounds 1-4): a TRAVEL lock (auto-cast from the live look
ray, sticky, feeds GOTO, guns never read it) and a COMBAT lock (seeded on
raise, persists when lowered, feeds guns/torpedoes/fine-lock/inset). ONE
cone pool serves both modes - `AvailableTargets`, all classes and
allegiances, strict cone of the active look ray ("Combat mode is Combat
mode") - and the RAISED flag decides which slot a scroll writes. All
AUTOMATIC mechanics (auto-cast, seeding, auto-seed-on-kill, LMB acquire)
are class-restricted; deliberate scroll is not. Carries the
componentization (resources port straight to this end-state shape).
Builds on the 20260712-231141 infrastructure.

## Steps

- [ ] Consume the 20260712-231141 infrastructure: the pool and travel
      casting use the ACTIVE look ray every frame (live in Normal and
      FreeLook per user directive); raise-frame seeding evaluates the
      press-frame look (the accessor's documented property); all routing
      reads the RAISED flag, never the camera enum. Turret slewing keeps
      the turret rig. Extend 231141's faithful split-rig test fixtures.
- [ ] Componentize straight to the end state: replace
      `SpaceshipPlayerTargetLock` (targeting.rs:72, registered :91) with
      `TravelLock { target: Option<Entity>, designated: bool }` and
      `CombatLock(Option<Entity>)`; replace
      `SpaceshipPlayerTargetCandidates` (targeting.rs:235, :92) with
      `AvailableTargets` (entries + pinned_until). Add `HostileContacts`
      (all-directions hostile combat targets, angle-then-distance).
      All on the ship root via `#[require]` on `PlayerSpaceshipMarker`
      (player.rs:288-290). Port surface: targeting.rs, player.rs,
      hud/{torpedo_target,target_candidates,edge_indicators,target_inset,
      component_lock}.rs, examples/12_hud_range.rs. GOTCHAS: verb hints
      (player.rs:157-281) must keep RUNNING shipless (a ship Single would
      freeze stale hints); `drive_reticle_anchor`
      (torpedo_target.rs:245-251) must write None on an empty query, not
      early-return; HUD teardown on ship despawn exists (hud/mod.rs:217-229).
      Port tests from `insert_resource` setups to components on the ship.
- [ ] THE POOL: `AvailableTargets` = every lockable body (any class, any
      allegiance; signature range gates intact, no extra distance cap)
      inside a wide cone (~50 deg half-angle const next to
      `TARGETING_CONE_HALF_ANGLE_DEG`, targeting.rs:128) of the ACTIVE
      look ray, ranked angle-then-distance. STRICT cone (round 4 / Q3):
      no past-edge membership - with ONE exception: the slot lock being
      cycled stays a member while valid even out-of-cone (a cycle press
      must be able to step OFF it; existing reticle-in-own-list rule,
      targeting.rs:563). Keep the 5-cap and the pinned stable-order rule.
- [ ] TRAVEL slot: empty -> auto-cast the nearest-to-ray body inside the
      TIGHT 18 deg pick cone, `designated = false`. Scroll while lowered
      steps the pool and sets `designated = true`. Sticky: aim wander
      never moves it; clears on death/despawn/range. The 550 m
      direction-blind hostile fallback does NOT apply to travel; record
      its final disposition when implemented.
- [ ] COMBAT slot: seed/re-seed ON RAISE by the incumbent-hysteresis rule
      (rounds 2b/3): best ENEMY by angle from the press-frame look over a
      cone/on-screen pool; incumbent (current CombatLock, else hostile
      TravelLock IF designated or in-cone) holds unless a challenger is
      clearly nearer (cos-ratio band const). A NON-hostile TravelLock
      inside the TIGHT cone at raise seeds instead (aimed-raise, Q4).
      Auto-seed-on-kill (Q2, const flag DEFAULT ON): lock dies while
      RAISED -> next ENEMY from `HostileContacts`, ON-SCREEN only;
      lowered -> slot stays empty. Committed torpedoes excluded from ALL
      automatic pools (deliberate scroll reaches them). Clears: death,
      out-of-range, allegiance flip to non-hostile, optional ~20 s
      lowered-decay const (flag).
- [ ] Scroll routing on RAISED (on top of 20260712-223034): lowered ->
      TravelLock step (+designated), raised -> CombatLock step - SAME
      pool, any member incl. neutrals/friends/rocks (Q1: deliberate
      scroll is unrestricted; this is what makes guided-torpedo-at-rock
      work). Precedence: scroll sets lock + 4 s pin + freezes order; a
      raise re-seed that switches REPLACES the pin; auto-seed only fills
      an EMPTY slot, no pin; a valid lock is never auto re-picked. Small
      debounce const so a wheel flick spanning raise/lower does not land
      on the wrong slot.
- [ ] `HostileContacts` consumers: edge indicators
      (hud/edge_indicators.rs:262) and the auto-seed pool ONLY - combat
      scroll does not walk it.
- [ ] Consumer routing: G/GOTO + verb hints (player.rs:232/:841) ->
      TravelLock; turret feed (player.rs:361), torpedo commit
      (player.rs:459), focus dwell (targeting.rs:606), component
      fine-lock (targeting.rs:665) -> CombatLock; inset view ->
      CombatLock, else TravelLock.
- [ ] HUD baseline: reticle = CombatLock; NEW distinct travel
      chevron/diamond = TravelLock; candidate brackets render the pool;
      edge indicators = `HostileContacts` + CombatLock + off-screen
      TravelLock arrow; "guns hot on <target>" banner whenever a
      CombatLock exists while lowered. Unmistakable slot distinction is
      IN scope; polish is not.
- [ ] Tests (state-per-step for gestures; on the 231141 split-rig
      fixtures): travel cast follows the look ray in Normal AND FreeLook;
      raise out of FreeLook seeds toward the looked-at flanker; seeding -
      designated hostile incumbent holds / stale undesignated behind-you
      hostile loses to the on-screen enemy / empty-space raise keeps the
      designated incumbent / non-hostile TravelLock in tight cone seeds;
      scroll routes per raised state; combat scroll reaches a neutral
      rock in the cone and a guided torpedo commits to it; combat scroll
      does NOT reach an off-cone enemy (strict cone); pin vs raise
      precedence; auto-seed only when raised+on-screen+empty and only
      onto hostiles; combat lock persists on lowering; allegiance flip
      clears it; G reads travel while combat points elsewhere;
      behind-player hostile in `HostileContacts`, absent from the pool,
      edge arrow shown.
- [ ] cargo fmt + cargo check + run targeting/input/hud test modules.

## Notes

- Spike: tasks/20260712-222610/SPIKE.md; rounds
  2b (incumbent rule), 2c (raised gating), 3 (adversarial deltas), 4
  (questionnaire - FINAL). Bodies rewritten clean post-round-4 per user
  directive; history lives in git and the spike.
- Depends on: 20260712-223034 (scroll rebind) AND 20260712-231141
  (look-ray/mode infrastructure).
- Playtest knobs as consts: wide-cone half-angle, hysteresis band, decay
  seconds + flag, auto-seed flag, debounce.
