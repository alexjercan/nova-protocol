# Decision: the Tab drawer is a third variant of PauseStates, not a separate freeze state

- DATE: 20260724-102304
- STATUS: ACCEPTED
- TASK: 20260724-102304
- TAGS: decision, ui, hud, state

## Context

The Tab ship-computer drawer must freeze the simulation and free the cursor
while it is open (owner direction; Spike: tasks/20260721-211512/SPIKE.md, option
A2). The sim-freeze already exists as `PauseStates {Unpaused, Paused}` nested
under `GameStates::Playing` (`crates/nova_gameplay/src/lib.rs:123`): entering
`Paused` runs `pause_clocks` (freezes `Time<Virtual>` + `Time<Physics>`) and
`release_cursor`, and the flight/section system sets are gated
`.run_if(in_state(PauseStates::Unpaused))`
(`crates/nova_menu/src/lib.rs:222`, `crates/nova_gameplay/src/plugin.rs:166`).

The load-bearing constraint is in `unpause_clocks`
(`crates/nova_menu/src/lib.rs:338`), whose own comment reads: "the pause menu is
currently the only clock-pauser in the app. A future ... freeze that also pauses
these clocks will be stomped here and needs a coordination story first." A
second, independent state that froze the clocks on its own would exit-unpause
even when the pause menu or a live outcome still wanted them frozen - the exact
stomp that comment warns about.

## Decision

Add `Drawer` as a THIRD variant of the existing `PauseStates` enum (alongside
`Unpaused` and `Paused`), rather than introducing a separate `DrawerState`. The
variant carries the overlay's identity; the freeze and cursor-free are driven by
reusing the existing hooks on `OnEnter(Drawer)`/`OnExit(Drawer)`. Transitions
only ever pass through `Unpaused` (Tab: `Unpaused <-> Drawer`; ESC:
`Unpaused <-> Paused`, and ESC from `Drawer` -> `Unpaused`); `Paused` and
`Drawer` are never entered directly from one another, so there is no
double-unfreeze flicker.

## Alternatives considered

- **Separate `DrawerState {Closed, Open}` that freezes independently** (spike
  A3-ish). Rejected: two independent clock-pausers hit the `unpause_clocks`
  stomp above - closing the drawer would unpause the sim even if an outcome or
  the pause menu still wanted it frozen. Keeping ONE freeze axis is what makes
  the freeze correct, not just convenient.
- **Reuse `PauseStates::Paused` as-is + a `DrawerOpen` marker resource** (spike
  A1). Rejected: two inputs (ESC, Tab) driving one `Paused` state through a
  side-channel resource makes ESC-vs-Tab precedence and "close drawer -> where
  do we land" implicit; `setup_pause_ui` would have to learn to stay hidden when
  Tab opened the pause. The variant makes overlays mutually exclusive by
  construction and lets each own its `OnEnter/OnExit` UI.

## Consequences

Easier: the drawer inherits the freeze + cursor-free for free by mirroring the
`Paused` hooks; the pause menu (`setup_pause_ui` on `OnEnter(Paused)` only) does
not show for the drawer; the flight/section set-gates (`in_state(Unpaused)`)
already exclude `Drawer` with no change.

Harder / the cost this task must pay: a new `PauseStates` variant is a new route
into "frozen", and the codebase suppresses input while frozen in TWO ways -
(1) the `in_state(Unpaused)` set-gate (already correct: `Drawer` is not
`Unpaused`), and (2) ~19 OBSERVER self-guards that check `== PauseStates::Paused`
directly (observers bypass set-gating - see the `set-gates-miss-observers`
lesson; e.g. `crates/nova_gameplay/src/input/player.rs:902`). Every guard that
means "while frozen" must widen from `== Paused` to `!= Unpaused`, or flight
intent leaks through with the drawer open. The exhaustive `match` in
`toggle_pause` (`crates/nova_menu/src/lib.rs:317`) also grows a `Drawer` arm.
This audit is the bulk of the task and is a Step below.
