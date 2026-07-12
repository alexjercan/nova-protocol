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
