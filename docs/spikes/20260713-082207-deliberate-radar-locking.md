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

## Fix record

(tasks not started yet)
