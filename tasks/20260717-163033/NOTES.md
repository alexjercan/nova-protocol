# Comms pacing queue - design record

Task 20260717-163033, spike tasks/20260717-155740/SPIKE.md (option B+D).

## What shipped

- comms_panel.rs rebuilt around a display QUEUE decoupled from StoryFeed:
  lines show in ARRIVAL order (the old panel rendered feed.last() -
  latest-wins - so any burst destroyed its own earlier lines); each line
  holds COMMS_DWELL_SECS (8) alone but yields to a pending line after
  COMMS_MIN_SECS (4); pending capped at 4 drop-oldest (the full log stays
  in StoryFeed); teardown resets queue + fades + visibility instantly
  (the leaked-line pin, kept and extended).
- Fades via bcs Tween (FIRST Nova adoption of the tween stack): 0.25s in
  / 0.4s out mapped onto panel text/border/background alphas;
  TweenPlugin registered once in NovaHudPlugin.
- Per-line dwell: StoryMessageActionConfig.dwell: Option<f32> (serde
  default; strict RON `dwell: Some(12.0)`), carried through the world
  sync into StoryLine, clamped [3,30] at use; content_lint WARNs outside
  the range. Eq dropped from the config/StoryLine/StoryFeed derives (f32).
- Comms blip: UiSfx::CommsLine at 0.22 volume when a line SHOWS
  (placeholder audio: reuses ui_toggle.wav under a distinct key).
- Objective gold flashes: fresh postings spawn OBJECTIVE_GOLD ghost
  lines beside the panel (mirroring the green completion ghosts; the
  marker now carries its base color); scenario teardown despawns LIVE
  ghosts too (previously a dying scenario's ghosts faded over the menu -
  same leak class as the comms line, found by the existing tests
  breaking against the new contract).

## Decisions

- Yield floor 4s vs full dwell 8s: a burst flows without any line
  dropping below readability; a solo line keeps the long hold.
- Drop-oldest at the QUEUE (not the log): stale backlog must not narrate
  the previous fight; a one-frame 6-line dump shows lines 2..5 (cap
  trims before the first pop - deliberate, tested).
- The blip plays at SHOW time, not enqueue: the sound marks "new text on
  screen", which is the attention moment.

## Verification

- 138 HUD tests green including 5 new/rewritten comms tests (arrival
  order - the fail-first vs latest-wins - solo full dwell, clamped
  per-line dwell, cap semantics, teardown reset) and 3 updated
  objective-feedback tests (color-aware: gold postings, green
  completions, teardown despawns live ghosts).
- nova_scenario --features serde: 98 + 1 green (sync carries dwell; lint
  warn test).
- content_lint clean; cargo check --workspace --all-targets green
  (pending final run); fmt last. Full suite on CI.

## Post-review addenda (Round 1, REQUEST_CHANGES -> fixes)

- R1.1 (MAJOR, real): teardown now clears StoryFeed like GameObjectives -
  without it, a retry/reload pushing an equal line count inside one sync
  window slipped the length-only diff and the new scenario's opening line
  silently vanished. The first version of the regression pin was VACUOUS
  (an intermediate update let the sync mask the bug; the sabotage stayed
  green) - tightened to the same-window shape and proven red/green.
- R1.2: the claimed-but-missing dwell tests exist now (sync carry + the
  documented strict-RON syntax parse).
- R1.4: pop starts at alpha 0 and the fade-in tween is kept (end value
  stays applied); fade-out residue hides behind Visibility::Hidden.
- R1.5: clamp constants shared with the lint (pub prelude consts).
