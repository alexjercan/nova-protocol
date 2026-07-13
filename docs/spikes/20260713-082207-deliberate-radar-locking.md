# Spike: Deliberate radar locking - travel/combat locks, weapons safety

- DATE: 20260713-082207
- STATUS: RECOMMENDED (user-final design capture)
- TAGS: spike, targeting, input, hud, ux

## Question

The round-4 scroll/raise model (spike 20260712-222610) was judged by the user
as "too overcomplicated: I don't like the SCROLL mechanic", and its whole task
family (223034, 223035, 223036, 223345, 231141) was closed wontdo. The user
replaced it with a simpler model built on one primitive - DELIBERATE radar
locking - and answered the clarifying questions (2026-07-13). This spike
captures that design faithfully, grounds it against the code, records what
carries over from the dead family (a lot of its infrastructure analysis
survives), and seeds the v0.5.0 tasks. A good answer is a spec a planner can
expand without re-litigating any decision.

## The design (user-final)

### Principles

1. **Locking is a deliberate action.** Nothing locks passively - no aim-assist
   auto-acquisition, no signature auto-lock. You hold a key to search.
2. **Two coexisting locks**, both STICKY once committed:
   - **TRAVEL lock** - white crosshair; GOTO reads it.
   - **COMBAT lock** - red crosshair, slightly smaller so the two overlap
     cleanly on the same body; guns/torpedoes/focus/inset read it.
3. **Weapons have a safety** ("weapons raised" flag): guns cannot fire while
   safety is ON.
4. **Lock is a computer capability**: like the controller-provided GOTO verb
   flags (spike 20260712-143551), a ship computer carries a lock-capability
   flag; some computers cannot lock at all.
5. **AI ships use the same model** through their controllers - locks, raised
   state and safety are ship-root components, not player resources.

### Gestures

| Gesture | Normal mode (RMB not held) | Combat mode (RMB held) |
|---|---|---|
| Hold CTRL | Radar on: continuously retargets what you look at | same, but for the combat lock |
| Release CTRL | **Travel lock commits** (lock-on-release) | **Combat lock commits** |
| Tap CTRL | Clears **ALL locks** (travel + combat) | Clears the **combat lock only** |
| Hold RMB | -> enter combat mode | safety OFF; **manual turret aim** (turrets follow your look) |
| Release RMB | - | manual aim ends; safety stays OFF only while a combat lock exists |

Decisions folded in (user answers, 2026-07-13):

- **Lock-on-release**, not lock-first-thing: while CTRL is held the radar
  live-retargets to whatever the look ray indicates; releasing commits.
- **Tap in normal mode clears everything** - so you never have to enter combat
  mode just to clear a stale combat lock. Tap in combat mode clears only the
  combat lock (your travel designation survives a fight).
- **Clearing the travel lock disengages an engaged GOTO.**

### Travel flow

```
hold CTRL -> radar on: candidate follows your look (provisional cue)
release CTRL -> travel lock commits (white crosshair, sticky)
[G] -> GOTO the travel lock
tap CTRL -> all locks cleared; engaged GOTO disengages
```

### Combat flow

```
hold RMB -> combat mode: safety OFF, turrets follow your look (MANUAL gunnery)
  |-- fire freely while holding (this absorbs today's CTRL free-aim role)
  |-- also hold CTRL -> radar -> release: COMBAT lock commits (red crosshair)
release RMB -> if a combat lock exists: safety stays OFF, turrets track the lock
              else: safety back ON
tap CTRL (RMB held) -> combat lock cleared -> back to manual; on RMB release safety ON
```

Safety truth table: `safety OFF <=> (RMB held) OR (combat lock exists)`.
The safety gates ALL weapons - turret triggers and torpedo launch (default;
playtest flag). Torpedoes commit against the combat lock; with none, launch is
dumb-fire as today (once safety is off via RMB).

Turret routing default (chosen here, playtest knob): while RMB is HELD the
turrets always follow the look (manual wins - this preserves the side-shot on a
second target without giving up the lock, the job CTRL free-aim used to do);
with RMB released and a combat lock held, turrets auto-track the lock (the
existing three-tier feed).

### Pools and ranges

Anything lockable is eligible for BOTH locks ("combat mode is combat mode" -
carried from round 4 decision 1: deliberate selection may target neutrals,
friendlies, rocks). Range gating stays the LockSignature scanner model
(signature * range_per_unit; ships/wells at full range) with retuned knobs per
the user's examples: small asteroids lockable only within ~200 m, debris only
within ~5 m (today `unsigned_lock_range` is 15 - tighten; exact constants at
plan/playtest time).

### Focus, components, inset

The 1.5 s focus dwell, the component fine-lock, and the target inset all hang
off the COMBAT lock only. Component cycling KEEPS today's wheel binding
(SCROLL = section cycle) - the user's objection was to scroll-cycling TARGETS,
not sections. The travel lock gets no dwell/inset.

### HUD

- White crosshair on the travel lock; red, slightly smaller crosshair on the
  combat lock; sized to overlap on the same body.
- A radar-active cue while CTRL is held (provisional/hollow crosshair on the
  current candidate that solidifies on release).
- A safety indicator ("you know when you can fire") - weapons-hot vs safety-on.
- Keybind hint rows updated (radar, clear, raise).

## Context (verified against the code)

- RMB is already `CombatInput` -> Turret camera view
  (camera_controller.rs:229-230; pad LeftTrigger2) - the combat-mode gesture
  exists, it gains the safety/manual-aim semantics.
- CTRL today = turret free-aim (raw key read, player.rs:434) AND the
  target-cycle wheel modifier (player.rs:625). Both roles are retired by this
  design (free-aim -> RMB manual; CTRL+scroll cycling -> gone), freeing CTRL to
  be a primary radar key. That also defuses the bare-modifier Chord bug class
  (retro 20260711-173237): CTRL stops being a modifier entirely, so hold vs tap
  discrimination is an ordinary tap-threshold gesture (~0.25 s knob).
- The acquisition machinery that survives: the candidate collection with its
  signature range gates and the angle-from-ray pick (targeting.rs) become the
  RADAR's picker, run only while CTRL is held. The auto paths die: the
  every-frame cone auto-pick, the close-range hostile signature auto-acquire,
  and the sticky `held` gate (stickiness is now inherent - nothing ever
  re-picks a committed lock).
- Componentization: locks/raised/safety must live on ship roots (components,
  not resources) both for AI parity (user answer 6) and respawn hygiene - the
  same port the dead 215957/223035 analysis worked out; their findings apply.
- Look-ray infrastructure (the deepest carry-over, from 222610 round 3
  finding 1, verified then at camera_controller.rs:575-631 /
  targeting.rs:314-319): the aim ray the picker reads is FROZEN outside Turret
  view because only the active rig integrates input. The radar requires the
  LIVE look ray in Normal, FreeLook and Turret views - the "active look ray"
  accessor plus robust mode transitions that the closed task 20260712-231141
  specified. That task body (rewritten clean in round 4) is design-agnostic
  infrastructure and is reincarnated below, not re-derived.
- Also carried from the dead family's round-3 adversarial findings: route
  gameplay off a dedicated RAISED flag written by `Start/Complete<CombatInput>`
  (not the last-writer-wins camera enum); fire gating lands in the input
  observers (player.rs:1054/:1128), not bindings data; the componentization
  port surface list (12_hud_range compiles against resources, shipless verb
  hints, reticle None-writes, pause-latch on raise transitions).

## Options considered

- **This design - deliberate radar locking (recommended).** One primitive
  (hold-to-search, release-to-commit, tap-to-clear) covers both locks; the
  mode (RMB) routes which slot; safety is a readable state instead of an
  input rule ("fire only while raised"). Fewer gestures than round 4: no
  scroll rebind, no seed-on-raise/hysteresis rules, no auto-seed-on-kill, no
  unlock-key debate - each of those existed to patch scroll-cycling's input
  tax, and all evaporate when selection is look-driven.
- **The round-4 scroll/raise model** (spike 20260712-222610, 4 rounds +
  adversarial reviews): rejected by the user as overcomplicated - the scroll
  mechanic itself was the objection. Its infrastructure analysis carries over
  (look-ray, raised flag, port surface); its selection mechanics do not.
- **Keep the current shipped model** (sticky combat locks + CTRL+scroll,
  landed 5123576): the do-nothing baseline. Rejected - it keeps passive
  auto-acquisition (locks appear without intent), the dual-purpose lock
  tension, and the scroll cycling the user dislikes.
- In-design alternatives settled: lock-first-thing on radar (rejected for
  lock-on-release - live retarget reads better); tap clearing only the active
  mode's lock (rejected - normal-mode tap clears ALL, so clearing never
  requires entering combat mode); turrets tracking the lock while RMB held
  (rejected for manual-wins-while-held - preserves the side-shot).

## Open questions (playtest knobs, not blockers)

- Tap threshold (~0.25 s) vs radar-flick: a sub-threshold radar hold that
  caught a candidate clears instead of locking - acceptable? (Deliberate
  radar takes longer than a tap by construction.)
- Turret routing default (manual-wins-while-RMB-held) - confirm in playtest.
- Safety gating torpedo launch (default: yes) - or launch-anytime?
- Range knob values (asteroid ~200 m, debris ~5 m examples) - tune against
  04_asteroids and the shakedown scenario.
- Does the AI need staged capability (e.g. the scavenger lock-less, manual
  only) for difficulty flavor? Scenario authoring question.
- Gamepad mapping: LeftTrigger2 = combat (exists); radar hold + tap needs a
  pad key (candidate: LeftTrigger1/shoulder - verify against the binding
  table at plan time).

## Next steps

Seeded, in execution order (all v0.5.0; `/plan` breaks each into steps when
picked up). The wontdo family (223034/-223035/-223036/-223345/-231141) stays
closed; where a new task inherits a dead task's analysis it says so.

- tatr 20260713-082324: look-ray + camera-mode infrastructure (reincarnates
  closed 231141 - live aim in every view, robust Normal/FreeLook/Turret
  transitions, active-look-ray accessor).
- tatr 20260713-082330: travel/combat lock slots + deliberate radar
  (componentized locks, CTRL hold/release/tap, lock-capability flag, unified
  pool with retuned range knobs, crosshair HUD baseline).
- tatr 20260713-082337: weapons safety + RMB manual gunnery + consumer routing
  (raised/safety flags, fire gating, manual turret aim, G/GOTO -> travel with
  disengage-on-clear, guns/torps/focus/inset -> combat, AI parity).
- tatr 20260713-082344: docs reconcile against the shipped radar model.

## Adversarial review round (2026-07-13)

Two independent adversarial reviewers were run on the design above - one UX
(attacking with player scenarios), one feasibility (attacking against the code
at file:line) - plus direct verification of the load-bearing claims in the main
session (bevy_enhanced_input 0.26.0 condition semantics; the pause-ungated
`CombatInput` observers; the latched fire input at player.rs:1053-1073 /
turret_section.rs:948). Verdict frame: the design ships regardless; these are
the sharp edges to spec before/at plan time. Findings condensed; severities
kept.

### State-machine gaps (design decisions needed - see the decision list)

- **[D1] (UX BLOCKER) Radar release with no candidate is undefined.** Sweep
  onto empty space and release: commit-None would silently destroy an existing
  lock (and an engaged GOTO); keep-old means there is no radar abort. Proposed:
  release-with-no-candidate = NO-OP (old lock survives) - "sweep off and
  release" becomes the abort gesture.
- **[D2] (UX MAJOR) Commit slot is routed by mode AT RELEASE**, so a
  mid-gesture RMB change silently re-routes the commit (enemy lands as a white
  travel lock with guns cold, or a beacon lands as a hot combat lock); the
  same-frame CTRL+RMB release race is order-dependent. Proposed: LATCH the
  destination slot at CTRL press and color the provisional crosshair by that
  slot for the whole hold.
- **[D3] (UX MAJOR x2) Tap = clear-ALL is the fast gesture with the biggest
  blast radius** - a too-fast radar flick mid-GOTO clears both locks AND
  disengages the autopilot; and the rule's own rationale inverts under GOTO
  (safing a stale combat lock without killing the autopilot requires entering
  combat mode - the exact thing clear-all was meant to avoid). Proposed
  (fixes both): STAGED clear - tap clears the combat lock if one exists,
  a second tap clears travel/GOTO - plus a "LOCKS CLEARED" toast; alternative:
  keep clear-all but exempt an engaged GOTO (Z already exists for deliberate
  disengage).
- **[D4] (UX MAJOR) Stale combat lock = weapons perma-hot to 20 km.** Safety
  OFF while any combat lock exists + sticky locks + 20 km ship lockability =
  a fled enemy keeps your guns hot for minutes; the predecessor's accepted
  mitigation (stale-lock decay ~20 s outside combat stance) was dropped by the
  new truth table. Proposed: re-add the decay flag and/or a much shorter
  combat out-of-range clear, and make the weapons-hot HUD loud ("HOT: lock on
  X, 14.2 km").
- **[D5] (UX MAJOR) Torpedoes ignore the manual-wins rule** turrets get: while
  RMB is held the guns follow your look but a launch commits to the (possibly
  off-screen) combat lock - the one weapon where a wrong target costs a
  munition is routed by invisible state. Proposed: a commit-target readout on
  the ammo HUD ("TORP -> SCAVENGER" / "TORP: DUMB"), or commit-only-if-lock-
  in-view-cone else dumb-fire.
- **[D6] (UX MAJOR) Gamepad: no free input fits hold+tap radar.** LB is
  FreeLook (camera_controller.rs:225), LT2 is combat, stick clicks fight
  precise aim, and DPadUp (the only freed button) takes the thumb off the
  stick. Proposed: on pad, radar is a press-TOGGLE (press on / press commit /
  long-press clear), or FreeLook moves to reclaim LB.
- **[D7] (UX MAJOR) Candidate flicker at release commits a coin flip** -
  worst for the torpedo-vs-launcher collinear pair in point defense. Proposed
  (fold into 082330, likely no user call needed): incumbent hysteresis on the
  provisional candidate (the existing cos-ratio band pattern) + a name label
  on the hollow crosshair.
- **[D8] (UX MINOR) GOTO reads the travel lock live or captured at [G]?**
  Re-designating mid-flight either leaves the HUD lying or silently re-routes
  the autopilot. Pick one and show the destination name on the GOTO row.
  Proposed default: captured at [G].

### Accepted implementation caveats (fold into the tasks; no user call)

- **Fire gate must be a live predicate, not a press gate** (feasibility MAJOR,
  verified): the trigger LATCHES `TurretSectionInput = true` on `Start`
  (player.rs:1053-1073) and the section fires every cooldown tick while true
  (turret_section.rs:948; torpedo same shape at :1127/torpedo mod.rs:488). A
  safety flip mid-held-trigger must zero the section inputs or be re-checked
  section-side - and a section-side check hits AI ships too, so decide which
  layer owns the gate (082337).
- **bevy_enhanced_input event mapping** (feasibility MAJOR, cross-verified):
  Hold = Start(press) -> Fire(threshold, per-frame) -> **Complete**(release) =
  commit; sub-threshold release emits **Cancel**, not Complete (commit
  observer listens to Complete only). Tap fires **`Fire<Tap>`** once on quick
  release (listen to Fire, not Complete; ignore its t=threshold Cancel). Two
  actions on the same CTRL bindings coexist (consume_input: false). Route the
  slot branch in observers by reading raised state (the `cycle_modifier_held`
  pattern) - never as stacked conditions. Derive Tap release-time and Hold
  threshold from ONE constant and test the boundary frame.
- **Input timers tick on REAL time** (feasibility MAJOR): `TimeKind::Real`
  default - thresholds advance while paused, and a release during pause fires
  Complete into pause-gated observers, silently eating the commit. Decide
  latch-vs-drop across pause for the commit AND the raised flag (082324/082330).
- **The infra task (082324) is a hard prerequisite for MANUAL aim too**
  (feasibility MAJOR, re-verified live): Alt-release while RMB is held sets
  mode Normal (camera_controller.rs:692-697), which would deactivate the
  turret rig while raised - manual gunnery tracking a frozen ray.
- **Test/example rewrite budget** (feasibility MAJOR): ~20 of ~45 targeting
  tests encode auto-acquire/sticky/pin/CTRL-cycle semantics and die or change
  meaning; 12_hud_range's script asserts passive auto-lock + dwell fill and
  needs scripted radar (or component writes); player.rs free-aim + hint tests
  re-key. Pure helpers (pick_target, range gates, rank) survive as radar rules.
- **Candidate-set consumers need a fate** (feasibility MAJOR -> decision
  [D9]): `SpaceshipPlayerTargetCandidates` feeds the candidates HUD and the
  edge-indicator threat arrows; killing the cycle orphans them. Proposed: keep
  the ranked tracker always-on as the threat set (edge arrows survive), retire
  or repurpose the on-screen candidate list.
- **Commit reads last frame's candidate** (observers run PreUpdate, picker in
  Update): store the provisional candidate on the ship root; never recompute
  in the observer. One frame of staleness is fine.
- **Respawn/death mid-gesture**: rigs despawn with the ship/camera and no
  Complete fires - componentized state on the fresh ship root default-clears
  (this is the respawn wart being fixed; do not reintroduce a resource).
- **Lock capability**: mechanism verified (ControllerVerbs is compile-checked,
  Rust-authored, no data migration), but verbs gate only the PLAYER today - AI
  never checks them, so a lock-less AI needs its own AI-side check; and decide
  whether Lock lives in ControllerVerbs (documented as "autopilot maneuvers")
  or a sibling scanner-capability component (082330/082337).
- **AI parity scope**: cheap as "components on ship roots + a thin mirror"
  (ai.rs already keeps AITarget etc. on roots); expensive as "one code path"
  (an AITarget->CombatLock unification drags ~30 test sites; the duplicated
  player/AI torpedo commit is the natural first unification). Also spec: AI
  acquisition timing (instant vs dwell-bound), HUD/audio cues filtered to the
  player ship, and what a LOCK-LESS AI's turrets do (the three-tier feed reads
  the lock - lock-less AI without manual-look gunnery is a harmlessness cliff,
  not difficulty flavor) (082337).
- **Dwell only on the COMMITTED lock**: keep the provisional radar candidate
  in its own field/component so live retargeting neither accrues nor resets
  focus dwell; same-entity re-commit is a NO-OP (lock/focus/component-lock
  survive) (082330).
- **Natural-clear list**: re-add allegiance-flip-to-non-hostile (the round-3
  m5 carry-over that was dropped) (082330).
- **Tutorial/scenario surface**: shakedown's "Lock BEACON 3 and press [G]"
  (shakedown.rs:583) teaches the dead passive lock; combat locking is never
  taught at all (beat 5 is pure manual gunnery). Add shakedown text + a
  teach-the-radar beat to the docs task's scope; guarantee the tutorial ship's
  computer has the lock capability (082344 + scenario).
- **Smaller UX notes**: context-sensitive CTRL hint row + cleared-locks toast
  (the mode-scoped tap is invisible otherwise); a safety-engaged cue on the
  OFF->ON edge (lock death mid-burst silently cuts guns); optional hold-fire-
  while-radar-active flag (sweeping with the trigger down rakes bystanders);
  lowered turrets rest pose (cosmetic); point-defense time budget currently
  viable (~15-25 s vs ~4-6 s of inputs) but COUPLED to torpedo speed/launch
  range - re-run the arithmetic on any torpedo buff.

### What held up under both attacks

The safety truth table (no contradictions once clears are defined); slot
independence (fight-while-fleeing works); combat-tap preserving the travel
designation; manual-wins-while-raised absorbing free-aim; CTRL genuinely
ceasing to be a modifier (tap/hold is an ordinary threshold, no Chord class);
the travel gesture tax (one deliberate hold+release per designation); the
componentization/respawn inheritance; every spike code citation spot-checked
accurate.

## Decisions on the adversarial round (2026-07-13, user)

- **D1 ACCEPTED**: radar release with no candidate is a NO-OP - the old lock
  survives; "sweep off and release" is the radar abort gesture.
- **D2 ACCEPTED**: the commit slot is LATCHED at CTRL press; the provisional
  hollow crosshair renders in the latched slot's color (white/red) for the
  whole hold. Same-frame CTRL+RMB races resolve by the latch.
- **D3 ACCEPTED as (a) staged clear**: tap clears the combat lock if one
  exists; a second tap clears the travel lock (and disengages an engaged
  GOTO). In combat mode (RMB held) tap clears the combat lock only, as
  before. A "LOCKS CLEARED"-style toast names what was cleared.
- **D4 ACCEPTED with tuning**: the combat lock decays - safety back ON - after
  **30 s without combat activity** (const knob; activity = raised or firing).
  Plus the loud weapons-hot indicator.
- **D5 ACCEPTED as (a)**: ammo-HUD commit readout ("TORP -> <target>" /
  "TORP: DUMB").
- **D6 DEFERRED**: do not stress gamepad ergonomics now; keybind improvement +
  customizable bindings in settings are backlog (existing tasks
  20260710-231927 and 20260711-180511 extended with the radar/pad notes -
  press-toggle radar is the candidate remedy when picked up).
- **D7 ACCEPTED** (folded, no veto): incumbent hysteresis on the provisional
  candidate + a name label on the hollow crosshair.
- **D8 ACCEPTED**: GOTO captures its target at [G]; the GOTO row shows the
  actual destination name; re-designating the travel lock does not re-route
  an engaged autopilot.
- **D9 ACCEPTED**: the ranked tracker survives as the always-on threat set
  (edge arrows keep working); the on-screen candidate list HUD is retired.
- Shakedown rework/polish (teach the radar, text updates, lock capability)
  filed as its own post-family task.

## Fix record

- 20260713-082324 LANDED (a123f36): derived camera mode + WeaponsRaised +
  outgoing-rig seeding + ActiveLookRay; acquisition and the section snap read
  the live ray. See its TASK.md.
- 20260713-082330 LANDED (9e655d1): the core - ship-root lock components,
  CTRL radar gestures (hold/commit/tap-staged-clear per D1/D2/D3a), natural
  clears + 30 s decay (D4), threat set (D9), FlightVerb::Lock, crosshair HUD,
  12_hud_range drives the REAL gesture live. Same-frame RMB+CTRL latch edge
  recorded in its TASK.md.
- 20260713-082337 LANDED (74238e1): WeaponsHot safety (3-layer enforcement),
  AI combat mirror, weapons status HUD + torpedo readout (D5a), GOTO capture
  pinned (D8). Audio blip + gesture hint rows deferred to 20260713-090653.
- 20260713-082344 LANDED: this docs sweep (banners on the component-lock,
  signature and inset docs; CHANGELOG coherence; shakedown minimal text fix).
