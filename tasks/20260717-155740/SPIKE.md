# Spike: storytelling and pacing - what makes the scenarios readable and gives them rhythm?

- DATE: 20260717-155740
- STATUS: RECOMMENDED
- TAGS: spike, scenario, gameplay, hud, pacing

## Question

Playtest feedback (2026-07-17, after the difficulty rework landed): the
game feels rushed - every event fires immediately with no breaks, enemies
appear out of nowhere, and objectives/story messages are hard to read
mid-fight. What combination of content pacing, engine mechanics and HUD
presentation fixes readability and rhythm - and what authoring convention
do future scenarios follow so this stays fixed?

A good answer names the mechanisms, splits engine from content work, and
leaves a beat-sheet convention a scenario author can apply without
re-deriving any of this.

## Context

Everything here was verified in source this cycle (files cited inline).

Why it feels rushed - four compounding causes:

1. **Zero-delay event chains.** Every shipped beat fires the frame its
   gate flips: wave 2 spawns the frame wave 1 dies, the Victory overlay
   the frame the last kill lands. The scenario clock
   (`scenario_elapsed`, task 20260717-112647) exists precisely for
   authored delays, but only the example mod uses it - the campaign and
   ledger predate it.
2. **Story lines destroy each other.** The comms panel renders ONE line,
   latest-wins, fixed 8s dwell, no queue, no fade
   (crates/nova_gameplay/src/hud/comms_panel.rs:36-138). Any handler that
   fires two StoryMessages (ledger_ch2's OnStart fires two back to back)
   makes the first invisible; any mid-fight line stomps whatever the
   player had not read yet. Checkpoint handlers fire their story line and
   the pause-everything Victory overlay in the SAME action list, so the
   line lands already covered.
3. **Ships materialize.** SpawnScenarioObject places a fully hostile
   ship instantly; the AI engages within 800u immediately. The only
   telegraph the difficulty rework could author was DISTANCE (spawns
   600u+ out). There is no arrival state, no warning cue, and no
   scenario action can play a sound at all (the audio layer is
   authored-or-silent per gameplay event; crates/nova_gameplay/src/audio.rs).
4. **Transitions are binary.** `linger: true` = modal full-freeze
   overlay; `linger: false` = hard cut that SWALLOWS a same-handler
   Outcome before it displays (NovaEventWorld::clear's documented
   footgun, world.rs:162). Nothing in between - no timed auto-advance,
   no non-modal beat (user-spotted gap, this playtest).

What already works and should be reused, not rebuilt: the objectives
panel has good change feedback (ObjectiveNew/Complete UI cues with an
anti-masking 1s delay + green ghost-fade lines,
hud/objective_feedback.rs); the hint-emphasis gold pulse is a working
attention cue; and bevy-common-systems ships Tween, UiAnimate and Popup
plugins that Nova's HUD never adopted (hand-rolled timers instead) -
fades and toasts are an integration away, not a build.

## Options considered

- **A. Content-only beat pass** (clock-gated delays, one line per beat,
  staggered spawns). Works today with zero engine risk - but it cannot
  fix latest-wins line replacement (any gameplay-triggered line still
  stomps an unread one), cannot telegraph arrivals beyond distance, and
  puts the whole burden on every future author. Necessary, insufficient.
- **B. Comms pacing queue** (engine/HUD): queue story lines, display in
  arrival order with a minimum on-screen time, optional per-line dwell
  override on the StoryMessage action, fade in/out via the bcs Tween
  helpers, and a comms blip in the UiSfx bank per line shown. Kills the
  latest-wins bug at the root; every scenario benefits without content
  changes. The one open design point: cap the queue depth so a stale
  backlog cannot narrate the previous fight (drop-oldest past ~4 with
  the full log still in StoryFeed).
- **C. Arrival telegraphs** (engine): an `engage_delay`/spawn-passive
  option on AIControllerConfig (ship arrives on patrol, goes hot after N
  seconds or when fired upon - the leash/threat machinery already
  supports the override), letting authors pair a warning line with a
  visible, not-yet-lethal approach. Rejected sub-option: a whole
  entrance-effect system (warp flash, engine-flare cinematic) - high
  effort, feel risk, and distance + delay + warning already read.
- **D. Objective toasts**: mostly ALREADY EXISTS (cues + ghosts); the
  remaining gap is a brief gold flash on newly-posted rows, worth
  folding into B's HUD pass as polish, not its own track.
- **E. Outcome transition pacing** (engine): the middle gear between
  hard cut and modal hold - USER-DIRECTED (2026-07-17): "we can add to
  the pacing by doing linger false in some cases maybe with a time
  delay". Concretely: an authorable delay on the non-lingering switch
  (queue the chain, keep playing or show the outcome banner without
  blocking, advance automatically after N seconds), plus an optional
  timed auto-advance on the modal overlay, plus a content_lint WARN for
  the Outcome + linger:false same-handler swallow trap. Small, closes a
  user-spotted gap with the user's own mechanism.
- **F. Do nothing** - the difficulty rework already spaced the fights;
  rejected: the reported problem is readability and rhythm, which
  distance alone does not fix (the playtest that reported it was played
  AFTER the rework landed).

## Recommendation

Layered, engine-first then one content pass - B, C and E make pacing
POSSIBLE and self-serve; the beat pass makes it ACTUAL and writes the
convention:

1. **Comms pacing queue** (B + D's flash): the readability core.
2. **Arrival telegraphs** (C): `engage_delay` + the authored warning-line
   pattern; enemies arrive instead of appearing.
3. **Outcome transition pacing** (E): the delayed non-lingering switch
   (per the user's directive), timed auto-advance on the overlay, and
   the swallow lint.
4. **Beat-sheet content pass**: apply 1-3 plus the scenario clock across
   shakedown, both broadside parts and the five ledger files, and write
   the authoring convention into the dev wiki (guide-author-scenario):
   announce -> breathe -> arrive -> fight -> confirm -> breathe ->
   next; one story line per beat; every fight gets a lead-in line;
   checkpoint story lines fire BEFORE the outcome beat or ride its
   auto-advance; no OnUpdate gate advances two beats in one frame.
   Acceptance is checkable: no handler fires more than one StoryMessage,
   and the balance audit's spawn groups each trail a warning beat.

## Open questions

- Comms queue depth and drop policy (recommend drop-oldest past 4;
  decide in the task with the HUD in front of the reviewer).
- Should the per-line dwell override be capped (a mod authoring a 10min
  line)? Recommend clamping to [3s, 30s] in the action's apply.
- Does `engage_delay` interact with the damage-override (a telegraphed
  ship fired upon during its grace must go hot immediately - recommend
  yes, the threat machinery already does this for leashes)?
- Music/ambience beds are OUT of scope here (no music system exists;
  a future spike).

## Next steps

Direction-level tasks this spike seeded, for /plan to break into steps:

- tatr 20260717-163033: comms pacing queue - ordered story lines,
  minimum display, per-line dwell, fades, comms blip, objective flash
- tatr 20260717-163042: arrival telegraphs - engage_delay on the AI
  controller + the warning-beat authoring pattern
- tatr 20260717-163050: outcome transition pacing - delayed
  non-lingering switches (user directive), overlay auto-advance, and
  the linger:false swallow lint
- tatr 20260717-163058: beat-sheet pass - apply the rhythm across the
  campaign and ledger; write the convention into the dev wiki

## Fix record

(Each implementing task appends a few lines here as it lands.)
