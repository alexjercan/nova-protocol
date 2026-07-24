# Notes: drawer shell + interaction model

Design notes and the state-route audit for task 20260724-102304. The
load-bearing choice (drawer = third `PauseStates` variant) is in DECISION.md;
this file holds the audit and the smaller implementation decisions.

## State-route audit: the 19 `== PauseStates::Paused` guards

Adding `PauseStates::Drawer` is a new route into "frozen". Two suppression
mechanisms exist; the first needed no change, the second needed 18 of 19 sites
widened.

1. **`in_state(PauseStates::Unpaused)` set-gates** (flight + section system sets,
   `crates/nova_gameplay/src/plugin.rs:166`). Already correct: `Drawer` is not
   `Unpaused`, so these sets already do not run in the drawer. No change.

2. **Observer self-guards** that compare `== PauseStates::Paused` directly.
   Observers bypass set-gating (`set-gates-miss-observers`), so each re-checks
   the pause state by hand. `Drawer != Paused`, so every guard that means "while
   frozen" had to widen to `!= Unpaused`. Introduced
   `PauseStates::is_frozen()` (`crates/nova_gameplay/src/lib.rs`) and used it at
   the 18 sites below. Note the widen is BEHAVIOR-PRESERVING for the pre-existing
   states: `is_frozen()` equals `== Paused` whenever the state is `Unpaused` or
   `Paused`, so no existing test (none enter `Drawer`) changes behavior.

   Widened (18):
   - `input/player.rs` x10 (flight/autopilot/component intent observers, from :902)
   - `input/targeting.rs` x4 (targeting/lock observers)
   - `camera_controller.rs` x1 (orbit/RCS camera intent)
   - `nova_scenario/src/loader.rs:1037` (`decide_advance`'s `paused` flag - the
     drawer must also hold scenario advance)
   - `nova_menu/src/lib.rs:1016` (`regrab_cursor_on_player_spawn` - do not regrab
     the cursor while the drawer holds it free)
   - `nova_debug/src/lib.rs:171` (`sync_inspector_cursor` - yield the cursor to
     the drawer, same as it yields to the pause overlay)

   Left as-is (1):
   - `nova_menu/src/lib.rs:990` (`sync_outcome_pause`): `*current.get() ==
     PauseStates::Paused` reads "the outcome pause", and an outcome can never be
     live while the drawer is open (the sim is frozen in `Drawer`, so no outcome
     fires; and Tab is inert in `Paused`). Widening it would wrongly unpause a
     drawer on an unrelated outcome clear. Kept precise.

Guard for regressions: the only remaining `== PauseStates::Paused` in the
codebase is that one intentional `current`-based site
(`grep -rn "== .*PauseStates::Paused" crates`).

## Audit gap caught in review (R1.1): audio loop freeze

The initial audit swept `== Paused` observer GUARDS but missed a `Paused`-only
SYSTEM REGISTRATION on the frozen axis: `audio.rs` wired
`pause_loops`/`resume_loops` on `OnEnter/OnExit(PauseStates::Paused)` only. Audio
sinks do not follow `Time<Virtual>`, so opening the drawer while thrusting would
leave the thruster hum / RCS hiss roaring behind it. Fixed by also firing them on
`OnEnter/OnExit(Drawer)`. Lesson for the audit: `audit-state-gates-on-new-entry-
path` must sweep `OnEnter/OnExit(<state>)` REGISTRATIONS across ALL crates, not
just the `== <state>` guard comparisons - a frozen-axis behavior can be wired by
schedule, not only by a runtime guard. (The `DespawnOnExit(Paused)` sites at
nova_menu:425/510 are the pause overlay UI and correctly stay Paused-only; the
drawer despawns its own surface.)

## `setup_pause_ui` stays `Paused`-only

The pause menu overlay (`OnEnter(PauseStates::Paused)`,
`DespawnOnExit(PauseStates::Paused)`) is untouched, so opening the drawer does
NOT spawn it - the drawer draws its own surface. Pinned by
`entering_drawer_freezes_clocks_frees_cursor_and_shows_no_pause_menu`.

## Animation clock: `Time<Real>`, not the bcs `Tween`

The plan suggested the bcs `TweenPlugin` (used by the comms fade). But bcs
`tween::advance_tweens` reads `Res<Time>` (the default clock = `Time<Virtual>`),
which the drawer PAUSES on open - a virtual-clocked tween would freeze mid-slide.
The slide must keep moving while the sim is frozen, so `drive_drawer_slide`
eases a `DrawerOpenness(f32)` with `Time<Real>` instead. (`verify-engine-
guarantees-in-source`: confirmed against the bcs source before wiring.)

## does-the-old-element-survive: drawer vs the compact HUD

The drawer is an INDEPENDENT axis from the grave/tilde `HudVisibility` cycle and
the existing top-right compact objectives panel - both stay. Opening the drawer
overlays them (and pauses); it does not hide or replace them. Grave/tilde still
cycles the flight HUD underneath. The expanded objectives SECTION in the drawer
is a second view of the same `GameObjectives`, not a move of the compact panel.

## HUD-tier: panel/backdrop are NOT Chrome, the handle IS

`apply_hud_visibility` (`hud/mod.rs`) force-hides any node whose `HudTier` the
current `HudVisibility` level does not show - EVERY frame, and the `!shows(tier)`
branch ignores `HudSelfDrivenVisibility`. So a Chrome-tier drawer panel would be
blanked whenever the player has the HUD minimized/off, even mid-open. The drawer
is a modal overlay on its own axis, so the panel and backdrop carry NO `HudTier`
(their visibility is driven solely by `drive_drawer_slide`). The tab HANDLE keeps
`HudTier::Chrome` because it genuinely is flight chrome - hiding it with the HUD
cycle is correct, and it is always in its shown state otherwise. Caught in the
work verify step by reading `apply_hud_visibility` rather than assuming Chrome
was inert.

## Tab-handle anchor for task 20260721-211520

`DrawerTabAnchor { rect: Option<Rect> }` is republished each frame from the
handle's UI global transform + the handle's fixed pixel size (so the math needs
no `ComputedNode`, keeping it unit-testable). This is 211520's tween TARGET.
