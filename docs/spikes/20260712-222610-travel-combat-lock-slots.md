# Spike: Travel/combat lock slots - raise-to-combat, fire-gated-on-lock

- DATE: 20260712-222610
- STATUS: RECOMMENDED
- TAGS: spike, targeting, input, hud, ux

## Question

The user proposed a new targeting model (2026-07-12, superseding the
single-lock v0.5 compromise of spike 20260712-215733 - "scrap the previous
one OR use what we found for it"). Does it hold together UX- and code-wise,
what are the sharp edges, and how do we sequence it? The model as stated:

1. In Normal or FreeLook view the cone is cast at ANY distance; the first
   hit becomes the TRAVEL lock. Plain SCROLL switches to the next thing in
   the cone. The lock is sticky: re-orienting the ship or moving the cone
   does not move it. This exists to make travel (GOTO) easy.
2. RMB (combat mode / Turret view) RE-USES the travel lock as the COMBAT
   lock if one exists - "like raising the weapon on someone in DayZ: you
   look at them, and when you RMB you raise the weapon".
3. In combat mode, SCROLL moves to the next ENEMY target. Two orderings were
   on the table; the user chose option 2: the next enemy in the camera cone
   (no camera snap; option 1 - closest enemy + camera turns to face - is
   out).
4. Leaving combat mode keeps the combat lock ("without lowering the
   weapon"); a keybind (SHIFT-like) removes the combat lock explicitly.
5. Component fine-lock cycling moves to SHIFT+SCROLL; CTRL stays manual
   (free-aim) targeting.
6. Fire (LMB): if combat-locked, it fires; if NOT combat-locked it does not
   fire - it acquires the combat lock instead.

## Context (verified against the code this session)

- Current input reality: wheel = component cycle, CTRL+wheel = target cycle
  (observer dispatch, targeting.rs:798-821), CTRL also = turret free-aim
  modifier (player.rs:361 feed), RMB hold = `CombatInput` -> Turret camera
  view (camera_controller.rs:230, :699-711; pad: LeftTrigger2), Alt hold =
  FreeLook (pad: LeftTrigger), G = GOTO on the lock (player.rs:841-848),
  DPadUp = target-cycle next, DPadLeft/Right = component cycle, DPadDown =
  ORBIT, Z = autopilot off, O = ORBIT key. SHIFT is UNBOUND in gameplay
  input today. Turret/torpedo triggers are scenario-authored bindings
  (`SpaceshipTurretInputBinding` / `SpaceshipTorpedoInputBinding`,
  nova_scenario/objects/spaceship.rs:167/:185) - the fire-gating change
  lands in the input observers (player.rs:1054/:1128), not in bindings
  data.
- One lock exists today (`SpaceshipPlayerTargetLock` resource; task
  20260712-215957, IN PROGRESS, is porting it plus the candidate list to
  components on the ship root). Acquisition is camera-mode-agnostic (no
  run_if on `SpaceshipTargetingSystems`, targeting.rs:103); "locking happens
  in combat view" is HUD affordance, not mechanism.
- Range gates are the LockSignature scanner model (ships/wells 20 km, signed
  rocks signature*30/u, committed torpedoes 2500, unsigned debris 15;
  docs/2026-07-10-signature-lock.md) - still sound, and "cone at ANY
  distance" reads as "no extra cone-range cap for signed/notable bodies",
  not as repealing the signature model (debris at 500 m stays unlockable,
  which the user explicitly wanted in spike 20260712-215256).
- Prior spikes: 20260712-215256 sketched combat/travel separation (C1
  toggle, C2 two slots); 20260712-215733 recommended the single-lock v0.5
  compromise with a view-routed two-slot future. This model IS that future,
  arriving now, with two new mechanics on top: seed-on-raise and
  fire-gated-on-lock.

## Assessment - how does it sound?

**Sound. This is the right model, and it is buildable on the componentized
state already in flight.** It cleanly dissolves every tension the last three
spikes circled around:

- The dual-purpose lock is gone: G reads the travel slot, guns/torpedoes
  read the combat slot. Traveling to a friend never points guns at them
  (the exact case the user flagged).
- Travel gets its feel back: aim-cast + scroll, sticky, any distance -
  designation without pixel-aiming, no combat semantics bleeding in.
- Combat entry becomes deliberate (raise, then fire), and fire-gated-on-lock
  is a built-in friendly-fire/wasted-torpedo guard.
- Seed-on-raise (DayZ analogy) preserves continuity of intent - the thing
  you were looking at is the thing you raise on - so the extra slot does
  not cost an extra selection step in the common case.
- The view IS the mode: no toggle key, no modal state to forget; the
  screen already looks different in Turret view. This is why C1 (toggle)
  stays retired.

Sharp edges (each has a resolution below): the scroll rebind and LMB gating
rewire muscle memory and MUST ship in one coherent package with the HUD
language, or the interim feels broken; bare-modifier taps are a known bug
class (20260711-173237) so the unlock key must not be a tap of the same
SHIFT that modifies scroll; and dense fields make "next in cone" ordering
quality matter.

## Resolved semantics (defaults chosen, flagged where the user may veto)

**Travel lock (Normal + FreeLook):**
- No travel lock -> auto-acquire: nearest-to-ray lockable body in the cast
  cone (existing 18 deg pick cone), signature gates intact but NO distance
  cap below `TARGETING_MAX_RANGE` (20 km) for full-range classes; signed
  rocks keep their signature range (a big far rock is designable, debris is
  not). Friendly and neutral bodies included - travel is allegiance-blind.
- SCROLL cycles the travel list: everything lockable in a WIDE cone
  (~50 deg half-angle knob), angle-then-distance ranked (the previous
  spike's list, now travel-only).
- Sticky: aim wander never moves it; it clears on death/despawn or scroll.
  Open question: explicit travel unlock (see combat unlock key - default:
  same key clears the ACTIVE view's lock).
- G engages GOTO on it (unchanged consumer, new slot). Guns never read it.

**Combat lock (raised via RMB / Turret view):**
- On RMB press with no combat lock: seed from the travel lock IF it is
  combat-eligible (ship or committed torpedo, ANY allegiance - raising on a
  friend is deliberate, DayZ-faithful); otherwise stay empty.
- SCROLL cycles ENEMIES ordered by angle-from-aim then distance (user's
  option 2). Refinement on top, needs user confirmation: the ordering runs
  over ALL known enemies, cone-first and continuing past the cone edge, so
  a torpedo chasing the tail is reachable by scrolling further while edge
  indicators point at it; the camera never snaps (a separate "face combat
  target" keybind can give option 1's benefit later if wanted).
- Leaving Turret view keeps the combat lock. A dedicated UNLOCK key clears
  it - NOT a bare SHIFT tap (SHIFT already modifies scroll; bare-modifier
  gestures are the 20260711-173237 bug class). Placeholder: X (unbound
  today; verify at plan time).
- LMB (and pad fire): combat-locked -> fire (turrets track lock as today,
  three-tier feed intact). Not combat-locked -> acquire instead of firing
  (best enemy by the combat ordering; no-op when none) - in ANY view, so
  no-lock LMB never blind-fires. CTRL free-aim is the deliberate unlocked
  fire path and bypasses the gate (unchanged).
- Torpedo launch commits against the combat lock; no lock = dumb-fire stays
  (it is already the committed behavior; the gate applies to turrets'
  trigger, torpedo launch keeps its existing no-lock dumb-fire semantics -
  default chosen to preserve the existing dumb-fire tactic, flag for
  playtest).
- Focus dwell, component fine-lock, SHIFT+SCROLL component cycling, and the
  inset view all key off the COMBAT lock.

**HUD language (must ship with the slots):** travel chevron/diamond vs
combat reticle, visually unmistakable; candidate brackets show the active
view's list (travel list in Normal, enemy order in Turret); edge indicators
track combat lock + all hostile contacts (decoupled `HostileContacts` set,
as planned in the previous spike) + the travel lock when off-screen; hint
rows update (SCROLL/SHIFT+SCROLL/unlock).

**Gamepad sketch (plan-time verify against the binding table):**
LeftTrigger2 = combat view (existing), DPadUp = cycle in active view,
DPadLeft/Right = component cycle (existing), unlock = East/B candidate,
fire gating applies to the pad trigger identically.

## Options considered

- **Adopt the two-slot raise model now (recommended).** As resolved above.
- **Stay on the single-lock v0.5 compromise** (spike 20260712-215733):
  simpler, already planned - but the user has explicitly moved past it, and
  its known compromises (guns tracking travel locks, ranking tension) are
  exactly what this model fixes. Its analysis and most of task 215402's
  mechanics (cone list, angle ranking, HostileContacts, stickiness) carry
  over into the new tasks rather than being discarded.
- **Combat scroll option 1** (closest enemy + camera snaps to face): rejected
  by the user; also steals camera control mid-flight and fights the "aim is
  flight" scheme.
- **Bare-SHIFT tap as unlock:** rejected - same-key-as-modifier taps are the
  known Chord bug class (retro 20260711-173237).

## Open questions

- Unlock key final binding (X placeholder) and whether it clears the active
  view's lock or only the combat lock (default: active view's).
- Combat scroll past the cone edge (my refinement of option 2): confirm the
  behind-you tail is reachable by scroll, or strictly cone-only.
- Torpedo dumb-fire without combat lock: keep (default) or gate like
  turrets.
- Travel list clutter in dense fields (04_asteroids): does the wide-cone +
  angle ordering + 5-cap feel right, or does travel want nearest-N by
  distance instead? Playtest.
- Does `PointRotationOutput` (the aim the cone uses) follow the FreeLook
  camera swivel? Verify at plan time; travel casting must follow what the
  player is LOOKING at in both Normal and FreeLook.

## Next steps

Task restructure (user steer: close the stale unified-lock tasks, remove
their worktree). The superseded direction's tasks are CLOSED with closure
notes - 20260712-215402 (unified list; scope redistributed below),
20260712-215957 (neutral componentize; folded into the slot task, since
with no code written porting straight to the end-state shape beats
port-then-rename), 20260712-215958 (docs; reseeded below). Its untouched
worktree (refactor/target-components) was removed.

Seeded, in execution order:

- tatr 20260712-223034: scroll rebind package - SCROLL = target cycle,
  SHIFT+SCROLL = component cycle, CTRL+SCROLL retired (CTRL keeps free-aim
  only). Lands against the current resource state; small, self-contained,
  immediate feel win. No dependencies.
- tatr 20260712-223035: travel/combat slot split - componentize the
  resources straight to TravelLock + CombatLock + AvailableTargets +
  HostileContacts on the ship root, acquisition split (travel
  auto-cast/sticky/scroll-cycle; combat seed-on-raise/enemy-ordering/
  persist), consumer routing (G+hints -> travel; turrets/torpedoes/focus/
  component-lock/inset -> combat), edge-indicator decouple, HUD
  chevron-vs-reticle baseline.
- tatr 20260712-223036: fire gating + unlock - trigger acquires when
  unlocked / fires when locked, CTRL bypass, dedicated unlock key,
  hint-row updates.
- tatr 20260712-223345: docs reconcile against the shipped two-slot model
  (replaces 215958).

## Fix record

(tasks not started yet)

## Round 2 (2026-07-12, user review): scroll fatigue + key conflicts

### User input (recorded verbatim as constraints)

- X is already taken by STOP (`AutopilotStopInput`, player.rs:584, X + pad
  East - verified) - the unlock placeholder was wrong; user floats SHIFT+X.
- Raising the gun on a friendly via seed-on-raise is acceptable ("I guess
  it's fine") but clearly not desired - treat as a tolerated edge, not a
  feature.
- THE core annoyance - too much scrolling to switch targets:
  - Scenario A: traveling toward a friendly, an enemy appears. Path 1:
    scroll off the friend (losing the travel designation), RMB, fire.
    Path 2: RMB (seeds the FRIEND), scroll in combat to the enemy, fire.
    Both are a step too many, and path 1 destroys the travel lock.
  - Scenario B: in combat, locked on enemy 1, LOOKING at enemy 2 - you
    still have to scroll before you can shoot it. The looking should
    count for more.
- Open question (user): is this UX friendly enough? Keep the two lock
  modes, but reduce the input tax.

### Round-2 refinements (two lock modes and stickiness kept)

1. **Hostile-first seed-on-raise** (replaces allegiance-blind seeding): on
   RMB with an empty combat lock, seed from the travel lock ONLY IF it is
   hostile; else the best enemy by angle-from-aim; else empty. Scenario A
   becomes RMB + LMB - zero scrolls, and the friend keeps the travel lock.
   Raise-on-friendly disappears from the normal flow entirely (better than
   "tolerated"). Friendly INSPECTION is preserved without combat-locking:
   the inset view priority becomes combat lock, else travel lock - a
   travel-locked friend still gets the inset/readout.
2. **Re-seed on raise, with hysteresis**: every RMB press re-evaluates -
   if a DIFFERENT enemy is clearly nearer the aim ray than the current
   combat lock (cos-ratio hysteresis, same 0.75-style rule as the
   component snap), the raise re-seeds onto it; otherwise the lock holds.
   Scenario B gains a scroll-free path: release+press RMB while looking at
   enemy 2 ("lower and re-raise the weapon at the new guy" - DayZ-true).
   A hostile TRAVEL lock still wins an empty-lock raise (you designated
   it deliberately); the hysteresis rule only governs raises with a live
   combat lock.
3. **Scroll stays the deterministic fallback**: angle-from-aim ordering
   means the FIRST flick locks the enemy you are looking at (scenario B
   while keeping RMB held = one flick). Past-cone-edge continuation stays
   (point defense behind you).
4. **Auto-seed next on kill while raised** (playtest flag): when the
   combat lock dies while in Turret view, seed the next enemy by
   angle-from-aim automatically; when it dies outside the view, leave the
   slot empty. Kills are the most common mid-fight retarget; this removes
   that scroll entirely. The fire gate still demands a deliberate LMB.
5. **Unlock binding**: MMB - unbound today (verified), chord-free, and on
   the same finger as the wheel it complements; keyboard alternative
   SHIFT+X (user's suggestion; chord-on-non-modifier, not a bare-modifier
   tap, so it avoids the 20260711-173237 bug class). Pad: East and West
   are taken (STOP, autopilot-off); candidate is a stick click
   (RightThumb) - verify at plan time.
6. **CTRL free-aim unchanged**: the no-switch side-shot (shoot enemy 2
   without giving up the lock on enemy 1) already exists and stays.

Input-count check against the user's scenarios: A: was scroll+RMB+LMB (or
RMB+scroll+LMB), now RMB+LMB. B: was scroll+LMB, now flick-RMB+LMB or
scroll+LMB - and after a kill, usually just LMB. Travel designation is
never sacrificed to combat retargeting.

### Round-2 open questions

- Does re-seed-on-raise (2) ever betray intent - e.g. re-raising for the
  SAME enemy while another drifts closer to the aim? The hysteresis band
  is the guard; width is a playtest knob.
- Auto-seed-on-kill (4): does it feel like the computer stealing the hand?
  Playtest; ship it behind a const flag.
- Pad unlock binding (RightThumb?) and whether SHIFT+X should ALSO clear
  the travel lock in Normal view (default: yes, active-view rule from
  round 1).
- User asked for an adversarial review of this round - recorded below.

### Round 2b (2026-07-12, user verdict on seed-on-raise)

User confirmed hostile-first seed-on-raise as "a really good find" and
restated it as "raising the gun re-evaluates the best target", accepting
the residual edge: travel-locked on an ENEMY, raising might lock a
different enemy - "much more acceptable" than the friendly-seed annoyance.

Refinement that shrinks that edge further - ONE incumbent-hysteresis rule
for every raise: the raise evaluates the best enemy by angle-from-aim, and
the INCUMBENT (the current combat lock on a re-raise, or the hostile
travel lock on an empty-lock raise) holds unless another enemy is CLEARLY
nearer the aim (the same cos-ratio band as round-2 rule 2). Outcomes:

- Approach run (aiming at your travel-locked enemy, raise): locks THEM -
  incumbent and aimed-at coincide.
- Ambush (travel-locked on enemy A, looking at attacker B, raise): locks
  B - you get the one you are looking at, which is the correct combat
  reflex.
- Raise while aiming at empty space with a hostile travel lock: incumbent
  wins - your deliberate designation is respected.

So the "might lock on something else" annoyance only remains when the
player raises while genuinely looking at a different enemy - which is the
case where switching IS the intent. Task 20260712-223035 seeding steps
follow this rule.

### Round 2c (2026-07-12, user question): drop the manual unlock?

User: MMB clearing may not be needed at all - could the combat lock clear
NATURALLY (e.g. when the travel lock changes)? What is the best way?

Analysis. A manual unlock exists only to defuse a stale combat lock. The
candidate natural triggers:

- Clear on travel-lock change or on G: rejected - it disarms the
  fight-while-fleeing case (travel-lock the escape station, G, keep the
  turrets on the pursuer). Travel and combat slots must stay independent;
  coupling them re-imports the dual-purpose-lock problem this whole model
  exists to kill.
- The better move is to make a stale lock INERT instead of cleaning it
  eagerly: gate FIRING on combat stance - the trigger only fires (or
  acquires) while RMB is held (Turret view); outside it, LMB is a no-op
  deny cue regardless of lock state, and CTRL free-aim remains the
  any-view manual escape hatch. This kills the adversarial review's
  blind-fire trap (B1) and the stale-lock verb-mismatch (M8) in one rule
  that is also the DayZ metaphor played straight: you cannot fire a
  lowered weapon.
- With stale locks inert, the natural-clear set suffices: death/despawn
  (exists), out-of-range (exists), allegiance flip to non-hostile (new,
  from adversarial finding m5), and an optional slow decay - N seconds
  outside Turret view (generous, ~20 s const, playtest flag) - so old
  locks fade without ever mattering. Re-raising near a new enemy already
  re-targets via the incumbent-hysteresis rule, so switching never needed
  the unlock either.

RESOLVED: no manual unlock key ships. MMB stays unbound (reserved - if
playtest finds a "safe the guns NOW while raised" need, it slots back in
without conflicts). SHIFT+X idea retired. Trade-off accepted and flagged
for playtest: firing always requires holding combat stance - a fleeing
gunfight means flying in Turret view, which is already how combat flying
works today.

## Round 3 (2026-07-12): adversarial reviews - findings and design deltas

Two independent adversarial reviewers (one UX, attacking with player
scenarios; one feasibility, attacking against the code at file:line) were
run on rounds 1-2b. Round 2c (no unlock key, fire gated on raise) landed
between their start and finish and already resolves several findings.
Verified myself before adopting: the frozen-ray chain and the rig-marker
sync (camera_controller.rs:575-631).

### Accepted findings -> design deltas (v3)

1. **The look ray must be re-sourced (feasibility B1/B3/M3 - the deepest
   find).** Today the acquisition cone reads the TURRET rig's
   `PointRotationOutput` (targeting.rs:314-319), which only integrates
   input while that rig holds `SpaceshipRotationInputActiveMarker` -
   Turret mode only (camera_controller.rs:616-629). Outside combat view
   the ray is FROZEN at the last raise's direction: travel casting in
   Normal/FreeLook is impossible as planned, and every raise-frame angle
   evaluation (seeding, hysteresis) compares against the stale ray.
   DELTA: introduce one "active look ray" accessor - the
   `PointRotationOutput` of whichever rig currently holds the active
   marker. Travel casting uses it every frame; raise-time seeding uses it
   on the press frame (the marker still sits on the outgoing rig that
   frame, so it IS the live look). The turret rig keeps feeding turret
   slewing. Bonus fix: Turret entry currently seeds from the NORMAL rig
   even when raising out of FreeLook (camera_controller.rs:586/:623-628) -
   seed from the ACTIVE rig instead, so raising while free-looking at a
   flanker aims at the flanker.
2. **Weapon-raised state, not the camera enum, routes gameplay
   (feasibility M2).** `SpaceshipCameraControlMode` is last-writer-wins
   across four ungated observers; Alt-tap while RMB is held corrupts it.
   DELTA: a dedicated raised flag written only by
   `Start/Complete<CombatInput>`; scroll routing, fire gate, seeding and
   auto-seed all read RAISED. The camera enum stays a camera concern
   (restoring the outer mode on nested releases is noted as a separate
   camera bug, not load-bearing here).
3. **Scroll rebind is a re-binding, not a branch swap (feasibility B2).**
   Wheel, brackets and DPad share the same two actions; swapping observer
   branches would make bare DPadRight retarget the guns. DELTA: move the
   WHEEL bindings onto `TargetCycleNextInput`/`TargetCyclePrevInput`
   (prev is empty today, player.rs:681-688), put the SHIFT dispatch in
   the target-cycle observers, leave brackets/DPad on component cycling
   untouched. Hint plumbing is field-vs-caption paired in
   keybind_hints.rs:242-243 with pinned tests - update both ends.
4. **Acquire/fire only while raised** (round 2c, confirmed by UX B1): a
   no-lock trigger outside combat stance is a deny cue, never an
   acquisition from an all-directions set (the "second click fires
   backwards" trap dies). Seed/acquire pools are cone/on-screen limited.
5. **Designated bit on the travel lock (UX B2).** An auto-cast travel
   lock was never "designated"; only a scrolled (or aimed-at-raise-time)
   one carries intent. The incumbent-hysteresis raise rule (round 2b)
   treats the hostile travel lock as incumbent ONLY if designated or
   currently in the cone; a stale auto-cast hostile behind you never
   wins a raise.
6. **Deliberate raise on a non-hostile (UX M4/M5), no new key.** Round 2's
   hostile-first seeding silently deleted every path to combat-lock a
   neutral/friendly/rock (traitor ships, mining assist). DELTA: a raise
   while the travel lock is non-hostile AND inside the tight pick cone
   seeds it - you are LOOKING at the thing you designated and raising on
   it, which is exactly deliberate. (Replaces the earlier CTRL+RMB
   override idea - no gesture conflict with free-aim, no new input.)
7. **Auto-seed-on-kill guards (UX M1).** Raised-only, on-screen-only,
   and it interrupts a held trigger - continuing fire requires a
   re-press. Behind a const flag, default ON for playtest.
8. **Torpedo protections (UX M3, feasibility m4).** Commit requires the
   combat lock stable for ~0.5 s (knob), else dumb-fire + deny cue;
   committed torpedoes are excluded from all AUTO-seeding (scroll still
   reaches them). RECORDED LOSS, needs user ack: guided torpedoes at nav
   bodies (asteroids/beacons) die with the split - the combat slot never
   holds them; torpedo-at-rock becomes dumb-fire.
9. **Precedence table (feasibility M1)** - deliberate beats automatic,
   last deliberate act wins: (i) scroll sets lock + 4 s pin + freezes
   list order; (ii) a raise re-seed (hysteresis passed) REPLACES any pin
   with a fresh one; (iii) auto-seed-on-kill runs only into an empty
   slot, sets no pin; (iv) stickiness: a valid lock is never auto
   re-picked, either slot. HUD "guns hot on X" banner whenever a combat
   lock exists while lowered (UX M8).
10. **Componentization port surface additions (feasibility m2/m3):**
    examples/12_hud_range.rs compiles against the resources; verb hints
    (player.rs:157-281) must keep running shipless or hints freeze
    stale; `drive_reticle_anchor` must write None on empty query, not
    early-return; mode enum lacks PartialEq (transition detection needs
    a Local prev); mode observers are pause-ungated while the seed
    system is pause-gated - latch raise transitions across pause.
    Good news verified: HUD layers tear down on Remove<PlayerSpaceshipMarker>
    (hud/mod.rs:217-229), and ship-scoped components FIX today's
    stale-resource-across-respawn wart.
11. Free-aim reads raw CTRL keys (player.rs:434) - the fire-gate bypass
    must NOT reuse the (now SHIFT) cycle-modifier helper (feasibility
    m7). Scroll-at-view-transition race (UX m2): debounce const on
    routing flips.

### Rejected / deferred, with reasons

- Soft/hard travel locks (UX M7): contradicts the user's explicit
  sticky-from-cast choice; angle-ordered scroll makes re-designation 1-2
  flicks. Deferred to playtest; the designated bit (delta 5) already
  exists if it becomes wanted.
- Combat-lock time expiry (UX M8 remedy): persistence is the point
  (user model item 4); inertness-while-lowered + banner + natural clears
  cover the risk. The ~20 s decay stays an optional flag.
- LOS hold-fire through a crossing friendly (UX m4): real, but
  pre-existing today and orthogonal to the slot split - future task
  candidate, not this family.
- SHIFT+X unlock collides with X=STOP + Chord class (feasibility M4):
  moot - round 2c removed the unlock key entirely; recorded as further
  evidence for that decision.
- RMB-flick retargeting camera whiplash on pad (UX m6): the scroll path
  remains primary; flick is an alternative, not a requirement.
- UX m1 (input-count arithmetic): fair - the honest claim is parity on
  raw counts vs today with the wins being travel-lock preservation,
  kill-chain continuity, and fewer wrong-target incidents.

### What survived both attacks unscathed

The two-slot invariant itself (G reads travel, guns read only combat),
the angle-from-aim combat scroll with past-cone continuation, torpedo
no-lock dumb-fire, CTRL free-aim as escape hatch, and all binding facts
(SHIFT/MMB free, X=STOP, pad table). The tasks' cited line numbers were
verified accurate.

## Round 4 (2026-07-12): user directives + questionnaire decisions (FINAL for v0.5)

User directives after round 3: fix the point-rotation plumbing and the
Normal/FreeLook/Turret transitions PROPERLY (new infrastructure task
20260712-231141 - mode derived from held inputs with Turret > FreeLook
priority, transition seeding from the outgoing rig, active-look-ray
accessor, faithful split-rig test fixtures); "Normal mode chooses target
for travel" is a hard requirement the code must be changed to allow. All
round-3 findings/solutions approved.

Questionnaire decisions:

1. **Combat slot membership (user's own design, supersedes both offered
   options): nav bodies AND friendlies join the CombatLock via deliberate
   SCROLL in combat mode - "Combat mode is Combat mode".** The combat
   scroll pool is the same cone list the travel scroll walks (lockable
   bodies in the wide cone of the live look ray, angle-ordered), not an
   enemies-only ordering. SIMPLIFICATION: one `AvailableTargets` list
   serves both modes (its ray source is simply the active rig - normal/
   freelook ray lowered, turret ray raised); the RAISED flag decides
   which slot a scroll writes. AUTOMATIC mechanics stay hostile-only:
   seed-on-raise (incumbent rule + the aimed-non-hostile case), LMB
   acquire, auto-seed-on-kill. REVERSES the round-3 recorded loss:
   guided torpedoes at rocks work again - scroll to the rock while
   raised, launch. `HostileContacts` remains, but only as the
   edge-indicator threat set and the auto-seed pool.
2. **Auto-seed-on-kill: default ON** (on-screen only, held fire
   interrupts, const flag).
3. **Combat scroll reach: STRICT CONE ONLY** (rejects the past-cone
   continuation): scroll never selects what you cannot see; a tail
   torpedo requires turning to face it (edge arrows warn). The round-3
   "continuing past the cone edge" delta is dead.
4. **Aimed-raise on a designated non-hostile seeds it: confirmed** (with
   decision 1 this is the raise-path complement to combat scrolling
   non-hostiles).

Design is FINAL for v0.5 implementation. Execution order: 20260712-223034
(scroll rebind) and 20260712-231141 (infrastructure) in either order or
parallel, then 20260712-223035 (slots), 20260712-223036 (fire gating),
20260712-223345 (docs).

### Round 4 addendum: task bodies rewritten clean

Per user directive after round 4, the five v0.5.0 task bodies
(20260712-223034, -231141, -223035, -223036, -223345) were REWRITTEN in
place against the final design - same IDs (all references above stay
valid), fresh coherent Goals/Steps, no layered edit archaeology (that
history lives in git and in this doc's rounds). Notably 223035's Goal had
still said "enemy-only scroll while raised", which round-4 decision 1
overturned.
