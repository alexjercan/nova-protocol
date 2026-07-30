# Keybind dock shows only currently-available verbs again

- STATUS: OPEN
- PRIORITY: 50
- TAGS: v0.9.0,ui,hud,feedback

## Story

Owner playtest 2026-07-30 (feedback wave on the v0.9.0 UI rework):

> I think we should keep the old logic of showing only the Keybinds that are
> available NOW (so if something is not enabled/available we do not show it
> instead of showing it as disabled)

The icon dock (task 20260728-175742) deliberately reversed the pre-dock rule:
`update_dock`'s doc says "unlike the text cluster it replaced, a dim chip STAYS
on screen", on the argument that constant chip positions make the row
glanceable. The owner playtested it and wants the old rule back - an
unavailable verb is not on screen.

Only the DOCK is in scope. The anchored ORBIT/GOTO cues are already
offer-gated, and the objective-stack TAB footer is not a verb chip.

## Understanding (2026-07-30)

- `crates/nova_gameplay/src/hud/keybind_dock.rs` - `update_dock` sets
  `node.display` from `!hint.key.is_empty()` only (no flight rig -> hidden);
  availability lands in `DockChipState` (`Dim` / `Available` / `Hot`) which
  `chip_paint` renders as three colour bands. So the hide seam already exists;
  the change is what feeds it.
- `chip_state` checks `Hot` BEFORE availability on purpose: the ORBIT offer is
  retired the moment you are parked, and that is exactly when ORBIT should read
  as the live maneuver. So `Hot` must stay VISIBLE even though its hint is
  unavailable - hiding on `!hint.available` alone would blink the live-maneuver
  chip out.
- `pulse_emphasized_chips` (scenario spotlight, `HintEmphasisSet`) has a
  dedicated `EMPHASIS_ALPHA_UNAVAILABLE` band, i.e. spotlighting a not-yet-
  available verb is an EXISTING supported case (the tutorials point at a verb
  before it lights up). A strict hide would silence it.

## Decision (owner, 2026-07-30)

Hide unavailable chips, EXCEPT when a scenario emphasizes that verb - an
emphasized chip is shown and pulses in the existing dim band. Recorded in
DECISION.md.

Visibility rule, in one line:

    shown = !hint.key.is_empty() && (state != Dim || emphasis.contains(verb))

## Steps

- [ ] Write the failing rig first: a live-tree dock test asserting an
      unavailable verb's chip is `Display::None` while an available one and a
      `Hot`-but-unavailable one are `Display::Flex`; plus one asserting an
      EMPHASIZED unavailable verb is visible and pulsing. Watch both fail.
- [ ] Move the visibility decision into a single helper next to `chip_state` so
      the dock has exactly one place that answers "is this chip on screen".
- [ ] Feed emphasis into `update_dock` (it does not read `HintEmphasis` today)
      and re-check visibility when emphasis changes - the existing early-out
      skips quiet frames on `hints`/`situations`/`assets`/`Added` only, so
      `HintEmphasis` must join that change gate or a spotlight set on an
      otherwise-quiet frame will not reveal the chip.
- [ ] Keep `pulse_emphasized_chips`'s hidden-chip restore correct: a chip that
      goes hidden mid-pulse must not keep its gold when it comes back.
- [ ] Re-check the row's centring: the dock is `justify_content: Center`, so a
      shrinking set re-centres. Confirm that reads as intended and not as the
      row sliding under the eye; note the verdict in NOTES.md.
- [ ] Update the module docs that assert the opposite rule (`update_dock`'s
      "a dim chip STAYS on screen" paragraph and the crate-level bullet listing
      the three states).
- [ ] Doc-surface sweep: grep the live doc surfaces (`web/src/wiki/**`,
      `web/src/tutorial.html`, `README.md`, `CHANGELOG.md`) for any claim that
      the dock shows all verbs / greys unavailable ones. Excludes `tasks/`.
- [ ] Probe a gameplay run for evidence the dock still behaves in flight.

## Definition of Done

1. An unavailable verb's dock chip is not rendered; an available one and a
   `Hot`-but-unavailable one are (test: the live-tree dock visibility rig).
2. A scenario spotlight on an unavailable verb still shows that chip, pulsing
   in the unavailable alpha band (test: the emphasis rig, including the
   emphasis-set-on-a-quiet-frame case).
3. `DockChipState::Dim` no longer paints an on-screen chip in the normal path;
   no live doc surface still claims unavailable verbs are shown greyed
   (cmd: `rg -n 'dim chip|greyed|all seven verbs' web/src README.md CHANGELOG.md crates`).
4. A probe run of a flight example is OK/WARN with no new dock warnings
   (cmd: `cargo run -p nova_probe -- run playable`).
5. Owner playtest: the dock reads as "these are your options right now"
   (manual).

## Notes

Sits under epic 20260728-175719 (UI rework); it is a direct verdict on that
epic's DoD 4 (text density) and DoD 5 (contextual HUD).

## Flow State

- FLOW STEP: PLANNED
