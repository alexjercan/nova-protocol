# Spike: Unified component-based target computer (cone list + sticky lock)

- DATE: 20260712-215733
- STATUS: RECOMMENDED
- TAGS: spike, targeting, ecs, input, hud

## Question

The user reports the target-selection feel is wrong and states the model they
want:

1. The targeting computer maintains a list of AVAILABLE targets, based on
   facing direction + aiming (the "cone"), stored as a COMPONENT.
2. The locked target is stored as a COMPONENT.
3. No lock -> the computer auto-picks the BEST target from that list.
4. Locked -> the lock NEVER changes on its own; only CTRL+scroll cycles to the
   next entry (or the target dies / leaves range).
5. Steer (mid-spike, 2026-07-12): keep it simple - "locking" serves both
   combat and normal (nav) targets for now, one lock, no combat/travel mode
   split; combat and normal targets are both lockable AS LONG AS they are in
   the cone list. The dual-purpose lock is an accepted compromise for this
   version.

What does the shipped code actually do, where does it diverge, and what is the
smallest coherent redesign that lands the model above? A good answer names the
divergences with code references and seeds direction-level tasks.

## Context

All code in `crates/nova_gameplay/src/input/targeting.rs` unless noted.
State is RESOURCES today, not components:

- `SpaceshipPlayerTargetLock(Option<Entity>)` (targeting.rs:72) - the lock.
  Consumed by turrets + free-aim (input/player.rs:361), torpedo commit
  (input/player.rs:459), GOTO autopilot (input/player.rs:848), and five HUD
  modules (reticle, candidates, edge indicators, inset, component lock).
- `SpaceshipPlayerTargetCandidates { entries, pinned_until }`
  (targeting.rs:235) - top-5 HOSTILE COMBAT targets (ships + committed
  torpedoes), ranked by angle-to-aim then distance, deliberately NOT
  cone-gated ("the cone only decides the aim PICK, not the SET",
  targeting.rs:532) because the edge-indicator overlay points at behind-you
  threats from this same list.
- Acquisition (`update_spaceship_target_input`, targeting.rs:313) runs three
  mechanisms that do not share a list:
  1. Cone pick (18 deg half-angle, 20 km) over ALL lockable bodies, any
     hostility/class - aiming designates anything (targeting.rs:493).
  2. Signature fallback: nearest HOSTILE inside 550 m, direction-blind
     (targeting.rs:502).
  3. The ranked candidate set (hostile combat only, all directions)
     feeding cycle + HUD (targeting.rs:518).
- Stickiness is class-split: the `held` gate (targeting.rs:483) protects
  COMBAT locks only; nav locks (asteroids/beacons/wells) stay aim-driven so
  GOTO could be re-designated by aiming (task 20260712-203353 review R1.1).
- CTRL+scroll (`step_target_lock` via observers, targeting.rs:854) walks
  `entries` - so nav bodies are unreachable by cycle, and cycling can jump to
  a hostile BEHIND the player (entries are not facing-gated).
- Range gating by `LockSignature` (ships/wells 20 km, signed rocks
  signature*30, debris 15 m, committed torpedoes 2500) sits at candidate
  collection and stays as-is; docs/2026-07-10-signature-lock.md is accurate.

### Divergence table (desired vs shipped)

| Desired | Shipped |
|---------|---------|
| One cone-gated available list, combat + nav | Three mechanisms; the only list is hostile-combat-only and not cone-gated |
| List and lock are components | Both are resources (player-singleton baked into 7 consumer files) |
| No lock -> best of the list | Cone pick over all bodies, then 550 m hostile fallback; the ranked list is never consulted |
| Locked -> never auto-switch | Combat locks sticky; nav locks stolen by every aim wander |
| CTRL+scroll cycles the whole list | Cycle reaches hostile combat targets only |

### Relationship to spike 20260712-215256 (combat/travel lock separation)

That sibling spike (same day, parallel session) recommended A1: notable signed
nav bodies join the cycle as NON-STICKY, combat-first entries, keeping aim
re-designation - and sketched a future combat/travel mode toggle (C1). The
user's steer to this spike is NEWER and simpler: one lock, both classes, cone
membership, sticky per the model above. A1's non-sticky detail is therefore
superseded; its motivation (cycle to a far rock you cannot pixel-aim) is
absorbed by this model, and its task 20260712-215402 is REPURPOSED in place to
carry the unified-list behavior change (title, goal and notes updated; the
supersession is recorded in the task body). The C1 mode-toggle and clump
directions stay future work recorded in that spike doc (master a616fc9 dropped
their seeded tasks per user steer); the unified list keeps the C1 door open (a
mode would just swap the list's membership filter and stickiness policy) - see
"Combat vs travel separation" below for the fuller UX analysis.

## Options considered

- **A. Patch the resources in place** (unify pick-from-list, cone-gate the
  set, universal stickiness; keep resources). Least churn, but ignores the
  explicit component requirement, keeps the player-singleton assumption baked
  into every consumer, and a second ship (AI targeting computer, co-op) would
  re-litigate everything.
- **B. Component-based target computer on the ship root (recommended).**
  `AvailableTargets` + `TargetLock` components on the ship root entity
  (`SpaceshipRootMarker` + `PlayerSpaceshipMarker`, targeting.rs:346).
  One acquisition system computes the list from facing + aim; lock rule:
  empty -> best entry, held -> sticky until cycled/dead/out-of-range.
  Consumers query the player ship. Matches the user's stated model verbatim.
- **C. Component-based + separate travel designator slot.** Cleanest for the
  GOTO tension but contradicts the steer ("keep it simple, one lock for
  both") and duplicates HUD affordances. This is effectively the sibling
  spike's C2, parked with C1.
- **Do nothing** - not viable; the user reports the current feel as a bug.

## Recommendation

**Option B**, in three ordered steps (one task each):

1. **Componentize with no behavior change.** Move
   `SpaceshipPlayerTargetLock` -> `TargetLock` component and
   `SpaceshipPlayerTargetCandidates` -> `AvailableTargets` component on the
   ship root; migrate the 7 consumer files (input/player.rs, targeting.rs,
   hud/{torpedo_target,target_candidates,edge_indicators,target_inset,
   component_lock}.rs) and their tests to query the player ship. Pure
   refactor, reviewable on its own. (`SpaceshipPlayerLockFocus` and
   `SpaceshipPlayerComponentLock` stay resources for now - the user asked for
   these two; migrating the rest is a follow-up if it earns its keep.)
2. **Unify acquisition and stickiness on the new components.**
   - Membership: every lockable body (existing signature range gates
     unchanged) inside a NEW list cone around the aim ray - wider than the
     18 deg pick cone (start ~45-60 deg half-angle, playtest knob), because
     the list is for flicking to things you cannot pixel-aim. Both combat
     and nav bodies, per the steer. The current lock is always a member
     while valid (existing rule, targeting.rs:563).
   - Ranking: angle-to-aim then distance (existing `rank_combat_targets`
     logic, generalized) - aim expresses intent; no hostility priority, so
     aiming at an asteroid still designates the asteroid.
   - Auto-pick: no lock -> best entry, where "best" is hostiles first (any
     list member, in rank order), else the best NAV entry inside the tight
     18 deg pick cone. The class asymmetry is deliberate: a threat anywhere
     in the wide cone should auto-acquire (combat urgency), but a rock
     should only self-lock when actually aimed at - otherwise cruising
     through an asteroid field churns the reticle onto rocks the player
     never asked for. Keep the 550 m direction-blind hostile signature
     fallback when the list is empty (the "sensors pick up a heat
     signature" story predates this model). Both cone widths are playtest
     knobs.
   - Stickiness: universal. `held` drops its `is_combat_target` gate; a lock
     of ANY class only leaves by death, range gate, or CTRL+scroll. Aim
     re-designation of nav targets is REMOVED by design (user accepted);
     re-designation = cycle (nav bodies are now in the list) or lose/clear
     the lock. The `pinned_until` window shrinks to its remaining real job:
     freezing list ORDER during a cycle burst (stickiness itself no longer
     needs the pin).
   - Edge indicators DECOUPLE from the list first: they keep their own
     all-directions hostile-combat threat set (plus the lock), so behind-you
     torpedo warnings do not regress when the cycle list becomes cone-gated.
3. **Reconcile the docs** (see below).

Delivered behavior check ("done" for the flow goal): lock a ship, sweep aim
across another ship and an asteroid -> lock unmoved; CTRL+scroll -> steps to
the next cone entry including asteroids/beacons; kill the lock with no other
input -> best cone entry auto-locks; no hostiles, aim at a rock -> rock locks
and GOTO works on it.

### Docs found wrong or stale (fix in step 3)

- docs/spikes/20260711-163800-multi-target-cycle.md: "torpedoes are threats
  to run from, not targets to cycle to" (superseded 20260712-212742) and the
  ships-only, resource-based cycle design - add a SUPERSEDED banner pointing
  here.
- docs/spikes/20260712-203235-lock-stickiness-and-inset-scope.md: B5
  ship-only stickiness and "nav bodies stay aim-driven" - superseded by
  universal stickiness; banner pointing here (inset-scope half still stands).
- docs/spikes/20260712-215256-combat-travel-lock-separation.md: A1's
  non-sticky nav entries - add a note that the sticky unified list supersedes
  A1 while C1/B stay parked.
- docs/2026-07-09-component-lock.md: cone range says 2000 m, code says 20 km
  (`TARGETING_MAX_RANGE`, targeting.rs:119); fallback described as "nearest
  AI ship", code gates on hostile relation. Correct in place.
- docs/2026-07-10-signature-lock.md: still accurate; no change.
- Retros are dated records of what happened and are NOT rewritten.

## Combat vs travel separation - is it worth it? (user question, 2026-07-12)

The user asked to think this through, floating: "RMB switches you into select
mode, and based on something else you can engage in combat with the locked
thing - how to do this nicely and very UX friendly?"

Ground fact the sketches in the sibling spike missed: **a combat mode already
exists as a held camera state.** RMB (pad: LeftTrigger2) is bound to
`CombatInput` (camera_controller.rs:230), and holding it puts
`SpaceshipCameraControlMode` into `Turret` view; releasing returns to
`Normal` (camera_controller.rs:699-711). Locking, aiming and firing all
happen while this view is held - which is exactly the user's "locking happens
in combat mode view". The lock survives leaving the view (the resource is
never cleared on mode change), which is what makes GOTO-on-lock work today.

Second ground fact: **selection and engagement are already separate acts.**
The lock is pure selection; engagement is a verb applied to it - fire
(turrets/torpedoes), G (GOTO), orbit. There is no "engaged" state to enter.
So the real question is not "how do we split selection from engagement" (it
is split) but "does the selection POLICY need to differ by intent" - combat
wants sticky + hostiles-first + auto-pick; travel wants aim-follows-intent +
nav bodies + calm.

Assessment: **a new modal system is not worth building now.**

- The unified list (this spike) already dissolves most of the policy tension:
  nav bodies become cyclable, stickiness is uniform, and the hostiles-first
  auto-pick keeps combat urgency without rock-churn.
- The user has explicitly accepted the dual-purpose lock as this version's
  compromise; building a mode system against that steer buys little.
- Every explicit-toggle design (sibling spike C1) carries the classic modal
  failure: fired-at-the-asteroid-because-I-was-in-travel-mode. A toggle key
  is the least UX-friendly shape available.

The UX-friendly evolution is to make the EXISTING view state the mode - no
new key, no new toggle, the screen already tells you which mode you are in
because turret view looks different. The user sketched the same shape
independently (follow-up steer, 2026-07-12): "lock things in Normal/FreeLook
view, travel to them with G, change with CTRL+scroll; RMB locks for combat" -
plus the key concern that travel-locking a FRIEND must not point the guns at
them. That concern is real in the current model: the turret aim feed tracks
THE lock (input/player.rs:361), so a friendly locked for GOTO gets tracked by
the guns even if you never fire. The leading future shape, then:

- **View-routed two-slot lock (C2-lite, the leading candidate).** Two slots
  on the ship root, and the CURRENT VIEW routes selection input to one of
  them: Normal/FreeLook aiming + CTRL+scroll manipulate the TRAVEL lock (any
  lockable body; G/orbit read it, guns NEVER track it); holding RMB (Turret
  view) manipulates the COMBAT lock (hostiles-only list; turrets and
  torpedoes read ONLY this slot). Zero mode keys, the view disambiguates,
  and the friend case is solved by construction: traveling to a friend is a
  travel-slot action, the guns' slot stays empty. The componentized
  `TargetLock` (task 20260712-215957) should be shaped so the second slot
  is an additive change, not a re-plumb.
  Open UX questions to settle when this is picked up: HUD language for the
  two locks (waypoint chevron vs combat reticle - must read differently at a
  glance); whether G in Turret view falls back to the combat lock (attack
  runs) or stays travel-only for predictability; whether the combat slot
  auto-seeds from the travel lock when it is hostile and RMB is pressed;
  per-slot stickiness (combat sticky per this spike; travel maybe aim-driven
  again since guns no longer follow it - which would quietly restore GOTO
  re-designation-by-aim); and where asteroid clumps plug in (travel slot
  only).
- **Hot-only-in-combat-view (cheap near-term knob):** before any second
  slot, the single-lock computer can auto-pick only while RMB is held; in
  Normal view it keeps the lock and the list but stops re-picking. Cruising
  goes quiet; combat stays instant. Flagged as a plan-time option on task
  20260712-215402 (playtest knob: auto-pick gated on view vs always).
- An explicit "engage" state (turrets weapons-free on the lock without
  holding fire) would be NEW gameplay (autoturret/wingman territory), not a
  targeting refactor - its own spike if ever wanted.

For v0.5 the single dual-purpose lock stays (user-accepted compromise, and
the friendly-gun-tracking quirk is part of that compromise). Decide the
two-slot split after the unified list playtests; nothing here blocks or
changes tasks 20260712-215957 / -215402 / -215958, but 215957 should keep
the door open as noted.

## Open questions

- Behind-you threats vs the cone: a torpedo chasing your tail is not in a
  cone-gated list until you turn to face it. Edge indicators still warn
  (decoupled set), and turning puts it in the list - is that enough, or
  should hostile combat targets bypass the cone gate for membership? Default
  per the steer is the strict cone; decide from playtest.
- List cone half-angle and the 5-entry cap in dense asteroid fields
  (04_asteroids): does nav clutter crowd out combat entries - reserve slots,
  raise the cap, or threshold by signature? Playtest knobs at plan time.
- Explicit unlock input (clear lock without cycling to something else):
  universal stickiness makes "aim away to drop" impossible. Probably wanted
  eventually; not seeded, raise after playtest.
- Whether `SpaceshipPlayerLockFocus` / `SpaceshipPlayerComponentLock` should
  follow onto the ship entity - follow-up refactor, not behavior.

## Next steps

Direction-level tasks seeded (`/plan` breaks into steps when picked up):

- tatr 20260712-215957: componentize targeting state (TargetLock +
  AvailableTargets on the ship root, no behavior change)
- tatr 20260712-215402: unified cone list + universal sticky lock + cycle
  covers nav bodies + edge-indicator decouple (REPURPOSED in place from the
  sibling spike's non-sticky A1 seeding - newer steer supersedes it; the
  task body records the change)
- tatr 20260712-215958: reconcile targeting docs (supersession banners +
  stale-claim fixes listed above)

## Fix record

(tasks not started yet)
