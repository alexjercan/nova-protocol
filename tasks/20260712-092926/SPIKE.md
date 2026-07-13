# Spike: What is the starter New Game scenario, beat by beat?

- DATE: 20260712-092926
- STATUS: RECOMMENDED
- TAGS: spike, scenario, content, tutorial, v0.5.0

## Question

Task 20260711-180506: New Game currently drops the player into
"asteroid_field", which is neither a teaching experience nor much fun. What
scenario should New Game actually load? The user's brief: teach the player
how to play by introducing keybinds in context (`Alt` freelook and friends),
make them explore the map (fly A to B, collect things), end with a small
fight, and make it good game design - "pretty", not a checklist of popups.
Also flagged: the keybind stack is growing; the scenario should cope with
that rather than make it worse. A good answer is a concrete beat sheet plus
the (small) list of missing systems, scoped so /plan can break it into steps
without re-deriving the design.

## Context

Grounding facts from the code (verified this spike):

- **Scenarios are code-defined event graphs**, not data files: a
  `ScenarioConfig` (`crates/nova_scenario/src/loader.rs`) carries a skybox
  and `Vec<ScenarioEventConfig>`; handlers listen for OnStart / OnDestroyed
  / OnUpdate / OnEnter / OnExit, gate on filters (entity, expression,
  And/Or/Not), and run actions: spawn objects/areas, set variables (with
  expression evaluation), add/complete objectives, DebugMessage,
  NextScenario (with linger). Built-ins live in
  `crates/nova_assets/src/scenario.rs` (asteroid_field, asteroid_next,
  menu_ambience). asteroid_field already demonstrates counters
  (asteroids_destroyed) and player-death handling.
- **Spawnable objects today**: asteroids (radius, texture, health, optional
  surface gravity -> a planetoid IS an asteroid with gravity), spaceships
  (player or AI, built from section configs), invisible trigger areas.
  There is **no pickup/item/cargo system** and **no player-facing waypoint
  marker** (areas are invisible; AI patrol waypoints are AI-only).
- **The objectives HUD is the only player-facing text channel**
  (`ObjectiveActionConfig { id, message }` -> panel rows; complete removes
  them). No toasts, no comms/dialogue.
- **The keybind-hint substrate shipped** (task 20260710-174646, spike
  20260710-174523): a lower-left cluster of live `[KEY] VERB` rows that
  light up when available (GOTO needs a lock, ORBIT needs a dominant well),
  plus hints anchored on the object the verb applies to. Labels read the
  real bindings. This is the teaching vehicle; the tutorial does not need
  its own keybind UI.
- **Current player verbs** (`input/player.rs`, `camera_controller.rs`):
  W/Space burn, mouse steer, `Alt` freelook, `RMB` combat/turret mode, LMB
  fire (per-section binding), Ctrl+wheel ship-target cycle, `[`/`]`
  component cycle, X STOP, G GOTO, O ORBIT, Z cancel, `~` HUD levels. That
  is ~13 verbs already - the user's "stack grows big" concern is real.
- **Passive-until-provoked AI is built in**: AI ships idle/patrol/orbit and
  only Engage when a hostile is within 800m (`AI_ENGAGE_RANGE`) or they
  took damage in the last 3s. Health is per-section, asteroid health is
  per-config, so a weak "pirate skiff" is pure tuning.
- **Aim-lock works on neutrals**: the aim-cone picker accepts any candidate
  (only signature auto-acquire and Ctrl-cycle are hostile-only,
  `input/targeting.rs:253`), so the player can lock an asteroid/beacon and
  G GOTO it before any enemy exists. (Re-verify at /plan; it is
  load-bearing for the autopilot beat.)
- New Game wiring is done (20260711-180426): the menu loads a canned ship
  into "asteroid_field"; this task swaps in the new scenario id.

## Options considered

- **A. Retune asteroid_field ("gentle sandbox").** Same map, weaker enemy,
  friendlier objective text. Pros: zero new systems, one afternoon of
  tuning. Cons: teaches nothing in sequence, no exploration arc, and the
  task exists precisely because asteroid_field is not fun; this fails the
  brief. Rejected as the destination, but it IS the honest fallback if the
  release runs long.
- **B. Staged tutorial built on the existing event graph (chosen).** A
  beat sheet of objectives gated by trigger areas: each beat introduces one
  verb where it is naturally needed, areas advance the script, variables
  count progress, and the finale spawns a single gentle hostile. Needs two
  small new scenario objects (a visible nav beacon, a collectible salvage
  crate) - both reusable content primitives, not tutorial hacks. Pros:
  rides the proven scenario engine end to end, showcases the v0.5.0
  headline features (wells, ORBIT, hint cluster), degrades gracefully (each
  beat is independent). Cons: teaching is objective-text + hint-cluster
  driven, not input-verified - the script cannot know the player actually
  pressed Alt, only that they reached the next gate. Acceptable: gates
  are placed so the verb is the natural way through.
- **C. A real tutorial engine (input-aware steps, toasts, step manager).**
  Advance beats on actual key presses (OnAction events into the scenario
  event world), transient toast text, scripted camera. Pros: verified
  teaching, the "proper" solution. Cons: a new event source + a new UI
  channel + step sequencing, overlapping the v0.6.0 objectives/modding arc
  (RON scenarios, piccolo VM); building it now front-loads v0.6.0
  architecture into a v0.5.0 content task. Deferred - and if OnAction
  events land later, the beat sheet upgrades in place (swap area gates for
  input gates) without redesign.
- **D. Do nothing (ship asteroid_field).** The menu already points at it,
  so New Game works today. Costs: first-session experience stays weak for
  v0.5.0, and the keybind wall stays unexplained. Rejected; this task is
  the release's front-door content.

## Recommendation

Build **B**: one scenario, working title **"Shakedown Run"** (id
`shakedown_run`), a first-flight fiction that makes each verb the natural
answer to the current situation. Setting: a planetoid with a gravity well
(reuse the menu_ambience planetoid family), a sparse asteroid belt, the
best skybox we have - the same vista the menu sells, now flyable.

Beat sheet (each beat = one objective + one area gate; ids for /plan).
Distances are deliberately short - a few hundred meters between
objectives - both for pacing and because "close enough to see" is the
cheapest possible objective marker (see the conveyance section below):

1. **Underway** - "Your ship is drifting. Burn for Beacon 1." Teaches
   W/Space + mouse steer. Beacon ~300-400m ahead, visible (new beacon
   object, emissive and blinking). Gate: OnEnter beacon area.
2. **Eyes up** - "Beacon 2 is somewhere off your beam. Hold [Alt] to look
   around and find it." Beacon 2 a few hundred meters out, ~120 degrees
   off boresight so freelook is genuinely the answer, not decoration.
   Gate: OnEnter beacon 2.
3. **Salvage sweep** - "Recover 3 supply crates in the debris cluster."
   Three salvage crates (new pickup object) scattered inside a loose
   asteroid cluster right off beacon 2; weaving practice, and X STOP
   earns its keep for the close-quarters stop-and-look. Variable counter,
   objective updates via complete+re-add per pickup (the panel has no
   live counter - open question below). Gate: counter == 3.
4. **Hands off** - "Lock Beacon 3 and let the computer fly: [G] GOTO.
   Then make orbit over the planetoid: [O] ORBIT." Beacon 3 sits inside
   the planetoid's SOI, so arriving lights up the ORBIT hint naturally -
   the hint cluster does the teaching, the objective just points at it.
   Entering this beat also quietly spawns the pirate back in the debris
   cluster (see beat 5). Gate: OnEnter an orbital shell area around the
   planetoid (a thick sphere band approximating "in orbit"; the event
   system cannot see autopilot state - open question below).
5. **Contact** - "A scavenger is picking through the debris field you
   just cleared. Drive it off." The pirate skiff spawned during beat 4
   prowls the crate cluster (patrol route over the debris), so the finale
   sends the player back across ground they know - and they can reuse
   G GOTO to get there, which is the point. Weak sections, one
   low-damage turret, passive until the player closes within 800m or
   fires. Teaches RMB combat mode, LMB fire, Ctrl+wheel lock, [ ]
   component cycle - all via the existing anchored hints once a hostile
   exists. Gate: OnDestroyed pirate -> "Shakedown complete." Then linger
   and leave the player in free flight in a now-safe playground (no
   forced exit; the belt, well, and orbit verbs are theirs). Death at any
   point: OnDestroyed player -> reload shakedown_run (the asteroid_field
   pattern).

Ships are deliberately minimal: the player's shakedown ship is
controller + hull + thruster + a single turret (no torpedo bay - torpedo
verbs are not taught here and every extra section is another thing on
screen); the pirate is the same silhouette with weaker sections. One
turret each keeps the component-cycle lesson trivially readable (there is
exactly one thing to cycle to) and keeps the fight legible.

Why this shape beat the alternatives: every keybind is introduced at the
moment it is the obvious tool (freelook to find something behind you, STOP
in a debris field, ORBIT when the well hint lights up), which is the
"introduce them such that it makes sense" ask; the pretty comes from
setting the whole thing in the well vista the release is about; and the
fight is last, single, and provoked-on-approach, so "gentle" is structural,
not just tuned.

### Conveying objectives: layered, degrades to text

The game has no objective markers, no direction-to-objective indicator,
no item highlighting, and no in-context hint prompts. The scenario is
designed so those are upgrades, not prerequisites - each layer slots in
without touching the beat sheet, because beats reference targets by
scenario entity id and the conveyance attaches to those ids.

**Layer 0 - ships with the scenario (no new features).** Conveyance is
carried by writing and staging:

- Objective text is imperative + key-in-brackets, matching the hint
  cluster labels exactly: "Hold [Alt] to look around", "Press [G] to let
  the computer fly". The panel and the cluster corroborate each other.
- Distances are a few hundred meters, so the current target is on screen
  or one freelook sweep away. Short legs ARE the direction indicator.
- Props self-advertise: beacons are emissive and blink; crates are bright
  against grey rock; the planetoid dominates the sky. "Fly to the
  blinking light" needs no UI.
- One verb per beat, so the newly lit row in the hint cluster is
  unambiguous - the only thing that changed is the thing to press.

**Layer 1 - nav beacon chips (task 20260712-093044).** Beacons carry a
screen-indicator chip (label + distance) with the substrate's existing
ClampToEdge off-screen policy - which means an off-screen beacon's chip
pins to the screen edge in its direction. Direction-to-objective falls
out of a field the indicator substrate already has; no new system.

**Layer 2 - objective conveyance substrate (task 20260712-093831).**
Three reusable pieces, each consumed by the scenario as data:

- **Objective marker action**: a scenario action that attaches/detaches a
  designated indicator (distinct objective styling, label + distance,
  edge-clamped) to any scenario entity. The script marks beacon 1 during
  beat 1, the crate cluster during beat 3, the pirate during beat 5.
- **Item highlight**: pulsing treatment for interactables - emissive
  pulse on the prop and/or an apparent-size bracket chip that tightens as
  the player closes. Applied to salvage crates; reusable for any future
  pickup or interaction point.
- **Hint emphasis**: a small API on the keybind-hint cluster to pulse one
  verb row on request, plus the scenario action to trigger it - so beat 4
  can literally point at [G] without new UI surface. The anchored
  on-object cues ([O] on the well, [G] on the lock) already exist and
  cover the "diegetic prompt" half.

The layering is also the honest answer to "how do we convey press-button
/ collect-items / goto-planet easily": text names the key, the cluster
shows it live, the anchored cue sits on the object, and each later layer
just shortens the distance between reading and doing.

Two small reusable primitives get built for it (separate task, the
scenario depends on it):

- **Nav beacon** scenario object: a small emissive mesh + screen-indicator
  chip (label + distance, the substrate already does this) + optional
  trigger area. This is the game's first player-facing waypoint - needed
  by any future mission content, not tutorial-only.
- **Salvage crate** pickup object: small prop; on player proximity
  (OnEnter of its area) it despawns and fires its handler. A minimal
  pickup built from existing parts (area + spawn + despawn action - check
  whether a DespawnScenarioObject action exists; if not it is one small
  action variant). No inventory; "collected" is a scenario variable.

**On the growing keybind stack** (user's flag): do not add tutorial-only
UI. The hint cluster is the single source of truth and already scales one
row per verb; the tutorial's job is to sequence attention to it, which
this design does by making each beat need exactly one new verb. When the
cluster outgrows the corner (roughly >8 rows), the fix is contextual
paging/grouping in the cluster itself plus the v0.6.0 settings keybind
page (20260711-180511) - noted there rather than solved here.

## Open questions

- **Objective progress text**: can an objective's message be refreshed
  (complete + re-add with "2/3") without visual glitching, or does the
  panel need a small update-in-place API? Resolve at /plan with a code
  read of ObjectivesPlugin (bevy_common_systems).
- **"In orbit" detection**: the orbital-shell area approximates it. If it
  feels wrong in playtest, the honest fix is a scenario event/filter that
  reads autopilot state (OnUpdate + a new expression source) - small, but
  only build it if the area gate fails.
- **Neutral aim-lock**: verified in code (targeting.rs picker takes
  non-hostiles), but confirm in play that lock + G on a beacon works
  end to end before committing beat 4.
- **Crate despawn action**: does the action set already include despawn of
  a scenario object? If not, one new EventActionConfig variant.
- **Difficulty of the finale**: exact pirate section HP / turret damage is
  a playtest knob, not a design decision - tune in /work.

## Next steps

Direction-level tasks this spike seeded, for /plan to break into steps:

- tatr 20260712-093044: nav beacon + salvage crate scenario objects
  (reusable content primitives; blocks the scenario task)
- tatr 20260711-180506 (existing, re-pointed at this spike): build the
  Shakedown Run scenario per the beat sheet above and swap New Game to it
- tatr 20260712-093831: objective conveyance visuals - marker action,
  item highlight, hint emphasis (layer 2; enhances the scenario, does not
  block it)

## Fix record

(Appended by implementing tasks as they land.)

- 20260712-093044 (beacon + crate primitives) LANDED 2026-07-12 (master
  2cac4b3): Beacon and SalvageCrate scenario object kinds, the
  DespawnScenarioObject action (scoped-only id lookup), beacon HUD chip
  with edge-clamp direction chevron, and a targeting-gate amendment
  (Static + authored LockSignature is lockable - Static beacons were
  otherwise unlockable, which would have broken beat 4's GOTO). Layer 1
  of the conveyance plan is therefore live. Details:
  tasks/20260712-093044/TASK.md.
- 20260711-180506 (the scenario) LANDED 2026-07-12 (master 2449120): New
  Game loads `shakedown_run` - all five beats, beat-counter gating, lazy
  per-beat spawns, early-pirate-kill branch, death restart; scavenger
  sections make "gentle" data. Platform pieces it forced: the
  fire_on_update pulse (EventConfig::OnUpdate was dead config), a
  write-on-diff GameObjectives sync, and sweep-pinned
  ASTEROID_GEOMETRIC_FACTOR_MIN/MAX (measured [3.70, 5.64]) that the
  orbit-gate geometry cites. Review 2 rounds, APPROVE. STILL OPEN: the
  human visual playtest (beacon readability, pickup feel, pirate
  difficulty, orbit-gate moment) - the conveyance task 20260712-093831
  should follow it. Details: tasks/20260711-180506/TASK.md.
- 20260712-093831 (conveyance layer 2) LANDED 2026-07-12 (master
  63293fd): gold objective marker chip via ObjectiveMarkerAttach/Detach
  actions (edge-clamp = direction arrow; marked beacons hand their cyan
  chip to the marker), intrinsic salvage-crate highlight (emissive glow
  pulse + WorldRadius HUD bracket on one shared clock), HintEmphasisSet/
  Clear pulsing a keybind row toward gold (teardown clears), and the
  per-beat attach/emphasis map wired into shakedown_run as data. The
  objective-progress piece needed NO new code (the write-on-diff sync +
  no-ghost-on-tally shipped with the scenario). Design spike:
  tasks/20260712-140842/SPIKE.md. Review
  2 rounds APPROVE. The human visual playtest (inherited from
  20260711-180506) now also owns the conveyance feel calls. Details:
  tasks/20260712-093831/TASK.md.
