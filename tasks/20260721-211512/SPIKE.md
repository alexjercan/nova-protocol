# Spike: the Tab ship-computer drawer - contents + interaction model

- DATE: 20260721-211512
- STATUS: RECOMMENDED
- TAGS: spike, ui, hud, v0.9.0

## Current status note

This spike answered the first drawer question and remains the source for the
accepted freeze/input decision: the drawer is a third `PauseStates::Drawer`
variant on the single pause axis. Its content/layout recommendation is now
historical. Post-playtest feedback in `tasks/20260725-104330/SPIKE.md`
supersedes the permanent side-panel model with one inset NOVA OS terminal
monitor. Existing objectives/log work becomes terminal output, and the 3D
minimap remains v0.9.0 stretch as a terminal-launched `map` app, not a center
drawer panel. Visual reference:
`examples/ui/nova_os_terminal_poc.html`.

## Question

The owner wants a "Tab" ship-computer DRAWER: a right-side surface that opens on
a keybind, pauses the game, frees the cursor, and shows more detail than the
in-flight HUD. Two things are undefined and this spike must fix them before
211520 (diegetic objectives) and 211526 (comms stack) can be planned:

1. **Contents** - what ships in the drawer for v0.9.0, and what is deferred. The
   owner questionnaire (2026-07-21) named four "core" sections: expanded
   objectives, the full comms log, a 3D minimap, and ship status/damage, plus
   "other cooler things". The v0.9.0 release tracker (20260724-083631, groomed
   three days later) narrows that: the spike "must FIX the drawer's v0.9.0
   contents (recommend: objectives + comms log in, defer the 3D minimap) so the
   release cannot balloon." Reconciling those two statements is the first job.
2. **Interaction model** - the Tab keybind, the open/close animation, pause
   semantics, cursor handling, how the diegetic objective (211520) lands INTO
   the drawer's tab handle, and how comms history renders.

A good answer fixes the v0.9.0 section list with reasons, and specifies the
drawer's state/input/animation wiring concretely enough that `/plan` can expand
it into steps without re-litigating the architecture.

## Context

Grounded in the current tree (Bevy 0.19, `bevy_common_systems` = "bcs" for
tweens/objectives/sfx, `bevy_enhanced_input` for flight actions):

**HUD.** `NovaHudPlugin` (`crates/nova_gameplay/src/hud/mod.rs:152`) spawns all
in-flight widgets as `HudTier::Instrument` (always on) or `HudTier::Chrome`
(togglable), all parented to the player ship and despawned with it. Grave/tilde
cycles `HudVisibility` All -> Minimal -> None (`hud/mod.rs:306`). Native
`bevy::ui`, no egui.

**Objectives (data EXISTS).** `bevy_common_systems::GameObjectives` resource is
synced write-on-diff from the scenario event-world by
`nova_scenario/src/world.rs:51` and rendered as a fixed 280px top-right panel
(`hud/mod.rs:273`). Completion feedback + on-screen gold objective-marker chips
already exist (`hud/objective_feedback.rs`, `hud/objective_markers.rs`).

**Comms (full log ALREADY EXISTS).** `StoryFeed`
(`hud/comms_panel.rs:57`) is an append-only log of every `StoryLine {speaker,
text, dwell}` in delivery order, synced by `nova_scenario`. The in-flight panel
(`comms_panel.rs`) only shows ONE line at a time via `CommsQueue`/`CommsDisplay`
with dwell timers; the full transcript is already sitting in `StoryFeed`
unused. Cast speaker constants live at
`crates/nova_assets/src/scenario/cast.rs`. This means the drawer's comms-log
section is mostly a RENDERING task over data that already exists.

**Pause + cursor (READY to reuse, built for this).** `PauseStates {Unpaused,
Paused}` is nested under `GameStates::Playing`
(`crates/nova_gameplay/src/lib.rs:98`). Entering `Paused`: `pause_clocks()`
freezes `Time<Virtual>`+`Time<Physics>` and `release_cursor()` frees+shows the
cursor (`crates/nova_menu/src/lib.rs:333,351`); flight input + section systems
are gated `.run_if(in_state(PauseStates::Unpaused))`
(`nova_gameplay/src/plugin.rs:166`). Task 20260721-211500 (CLOSED) deliberately
made the cursor state-driven "so a future drawer (Tab) gets it for free".

**Input.** Flight verbs go through `bevy_enhanced_input` action bindings
(`nova_gameplay/src/input/player.rs`). O is bound to `AutopilotOrbitInput`
(ORBIT) - hence the owner's Tab-only decision. Tab is UNBOUND anywhere. The
pause toggle is the ONE exception to the action-map: a hard-coded
`KeyCode::Escape` check in `nova_menu/src/lib.rs:296` (comment: "no existing
Escape binding anywhere"). This is the pattern the Tab toggle should copy - it
must fire while the sim is frozen, so it cannot live in the Unpaused-gated
flight rig.

**Minimap.** None exists. The nearest spatial UI is the objective-marker chips
and the radar-lock box; neither is a map. A "3D minimap" is net-new.

**Animation.** `bevy_common_systems::TweenPlugin` is already wired into the HUD
(`hud/mod.rs:198`) and drives the comms fade - the slide-in vehicle is already
present. (Prior spike 20260717-155740 noted bcs Tween/UiAnimate as unadopted;
Tween is in fact adopted for comms; a richer drawer slide is the next use.)

## Options considered

### A. Pause/overlay wiring - how the drawer freezes the sim

- **A1. Reuse `PauseStates::Paused` as-is, add a "which overlay" resource.**
  Tab sets `NextState(Paused)` (freeze + cursor free for free) and a
  `DrawerOpen` marker so the drawer UI shows and the ESC pause-menu UI does not.
  Pros: zero change to the freeze/cursor hooks. Cons: two independent inputs
  (ESC, Tab) both drive one `Paused` state through a side-channel resource;
  ESC-vs-Tab precedence and "close drawer -> where do we land" become
  implicit/fragile; the pause overlay's `setup_pause_ui` must learn to stay
  hidden when the drawer opened it.

- **A2. Add a third variant to the sim-gate: `PauseStates {Unpaused, Paused,
  Drawer}`.** The variant IS the overlay identity. Generalize the freeze +
  cursor-free hooks to run on "any non-Unpaused" (a `not(in_state(Unpaused))`
  run-condition, or an `OnEnter` for each variant); flight/section gating is
  already `in_state(Unpaused)` so it needs no change. Tab toggles
  Unpaused<->Drawer; ESC: Unpaused->Paused, Drawer->Unpaused (Tab-drawer is
  cheap to dismiss), Paused->Unpaused. Pros: overlays are mutually exclusive by
  construction (you cannot be in the pause menu AND the drawer); each surface
  owns its own `OnEnter/OnExit` UI; precedence is explicit in one enum. Cons:
  touches the `PauseStates` enum and the two freeze/cursor systems' run
  conditions; one migration point.

- **A3. A separate `Camera`/UI stack with manual clock+cursor control.** The
  drawer manages its own `Time` pause and `CursorOptions` writes, independent of
  `PauseStates`. Pros: fully decoupled. Cons: re-implements exactly the
  freeze+cursor logic 211500 centralized; two code paths for "the sim is
  frozen" is the bug farm this project already avoided once.

### B. v0.9.0 contents - what actually ships in the drawer

- **B1. Objectives + comms log only** (the tracker's recommendation). Both data
  sources already exist (`GameObjectives`, `StoryFeed`); the work is the shell +
  two rendering sections. Smallest, ships the headline, cannot balloon.
- **B2. B1 + ship status/damage.** Section health data exists
  (`SpaceshipSectionSystems`); a status readout is moderate. But it overlaps the
  STRETCH Strand B (critical-damage model, 20260722-092320) and risks
  double-work if that lands differently.
- **B3. All four (add the 3D minimap).** The minimap is net-new and the biggest
  unknown (see C). Including it is what "balloons the release" per the tracker.

### C. 3D minimap - how it would be built (if/when pulled in)

- **C1. Render the real scene to a texture** via a second `Camera3d` with a
  render target, shown in the drawer. Highest fidelity, highest cost (second
  render pass over live geometry every open frame; culling/scale headaches).
- **C2. Schematic orrery: a lightweight proxy scene of blips** (player, gravity
  wells, objectives, radar contacts) at scaled positions, rendered by a small
  dedicated camera to a texture and rotatable. Cheaper, mod-friendly, reads as
  "3D" without touching real geometry. The data (contacts, wells, objective
  targets) is already enumerable from existing components.
- **C3. 2D top-down radar plot.** Cheapest, but the owner explicitly said "3D".
- All three share a plottable-contacts data model; the render mode is a
  swappable back layer, so starting 2D and upgrading to the orrery is a
  contained change, not a rewrite.

### D. Do nothing / defer the whole drawer

Always a candidate. Cost: 211520 and 211526 both cite this drawer as their home
for the objective tuck-target and the comms log view; deferring the drawer
strands both and guts the v0.9.0 "Cockpit & Command" headline. Not recommended.

## Recommendation

**Build the drawer for v0.9.0 with two sections - expanded objectives and the
full comms log - and defer the 3D minimap and ship-status sections to backlog.**
This matches the release tracker's explicit "objectives + comms log in, defer
the 3D minimap" and rests on the fact that both sections render data that
ALREADY exists (`GameObjectives`, `StoryFeed`), so the release ships the
headline interaction without a net-new subsystem. (See Open Questions for the
one owner reconciliation this needs.)

**Interaction model (the load-bearing choice - a DECISION.md on the shell task
should record it, citing this spike):**

- **Wiring: option A2** - add `PauseStates::Drawer` as a third sim-gate variant.
  The variant carries the overlay identity, overlays stay mutually exclusive,
  each owns its `OnEnter/OnExit` UI, and the existing `Unpaused`-gated flight
  systems need no change. Generalize `pause_clocks`/`release_cursor` (and their
  exit partners) to fire on any non-`Unpaused` state. Rejected A1 (implicit
  precedence via a side resource) and A3 (re-implements the freeze this repo
  already centralized).
- **Keybind:** a hard-coded `KeyCode::Tab` toggle system in the same spirit as
  `toggle_pause` (`nova_menu/src/lib.rs:296`), run in `GameStates::Playing`
  regardless of pause substate so it can also CLOSE the drawer while frozen. NOT
  in the flight input rig (that rig is `Unpaused`-gated and could not close the
  drawer). Tab: Unpaused<->Drawer. ESC from Drawer closes to Unpaused.
- **Pause + cursor:** free via A2 - entering `Drawer` freezes the clocks and
  frees the cursor through the generalized hooks; no bespoke cursor code.
- **Surface + animation:** a right-side panel that slides in from the right edge
  via `bevy_common_systems::TweenPlugin` (already wired for comms), animating the
  panel's X offset with a backdrop fade. Collapsed, it presents a **tab handle**
  on the right edge; the handle has a known screen anchor.
- **Sections:** the drawer hosts named sections (Objectives, Comms Log now;
  Map, Ship later) in a simple vertical or tabbed layout - a section framework
  the deferred sections slot into without reopening the shell.
- **Diegetic objective hand-off (feeds 211520):** the tab handle's screen anchor
  is the tween TARGET for 211520's "big cockpit objective animates into the
  tab". The shell task must expose that anchor (a component/resource with the
  handle's screen rect) so 211520 tweens to it without hard-coding coordinates.
- **Comms log render (feeds 211526):** the Comms Log section renders the full
  `StoryFeed` as a scrollable, speaker-grouped transcript (bevy_ui scroll),
  reusing the per-speaker icons 211526 introduces. The stacking in-flight panel
  (211526) and this log view are two views of the same `StoryFeed`.

**Extras explored ("what else?"):** a **codex/dossier** of speakers and
factions (rides the comms cast constants), a **nav/contacts list** paired with
the eventual map, and an **autopilot/maneuver readout**. All are genuinely nice
but none are v0.9.0 - they are captured here and as backlog tasks so the release
stays tight. The honest answer to "what else" is: the drawer's section framework
is the real deliverable; extra sections are cheap to add once it exists, so
resist front-loading them.

## Open questions

- **Owner reconciliation (contents) - RESOLVED 2026-07-24.** The 2026-07-21
  questionnaire called the 3D minimap and ship-status "core"; this spike
  recommended the tracker's tighter set (objectives + comms log). Owner call:
  keep the **minimap in v0.9.0 as a STRETCH item** (do it LAST, "curious how it
  will look", cut first if the core runs long) - pulled in as task
  20260724-102320 (p30, stretch). **Ship-status stays deferred** to backlog
  (20260724-102332). So v0.9.0 drawer = objectives + comms log (committed) +
  minimap (stretch).
- **ESC precedence from the drawer.** Recommended: ESC in `Drawer` closes to
  `Unpaused` (not into the pause menu). Trivially reversible; confirm in
  playtest.
- **Section navigation.** Vertical stack (all sections visible, scroll) vs
  tabbed (one at a time). Lean vertical for two sections; revisit if the map +
  ship sections land and it gets crowded. A one-line layout choice, not an
  architecture decision.
- **Minimap fidelity (only if pulled in).** C2 (schematic orrery) vs C1 (real
  scene) - deferred with the minimap task; the plottable-contacts data model
  makes the render mode a swappable back layer.

## Next steps

Direction-level tasks this spike seeded, for `/plan` to break into steps. The
shell task is the gate for the drawer sections and for 211520's tuck-target.

**In v0.9.0:**

- tatr 20260724-102304 (p72): Drawer shell + interaction model + objectives
  section (Tab keybind, `PauseStates::Drawer`, slide animation, tab-handle
  screen anchor, section framework, expanded objectives section). Carries the
  A2/keybind DECISION.md. GATES 211520 (tuck-target anchor) and the comms-log
  section.
- tatr 20260724-102309 (p50): Drawer comms-log section - render the full
  `StoryFeed` as a scrollable speaker-grouped transcript in the drawer; reuses
  211526's speaker icons. Depends on the shell.

**v0.9.0 STRETCH (owner call 2026-07-24, post-spike - pulled in but LAST, cut
first if the core runs long):**

- tatr 20260724-102320 (p30, stretch): Drawer 3D minimap / nav section -
  schematic-orrery design (option C2) captured; net-new subsystem, the release's
  largest single unknown. Owner wants it in v0.9.0 but at the end - "curious how
  it will look".

**Deferred to backlog:**

- tatr 20260724-102332 (p0, backlog): Drawer ship-status / damage section -
  overlaps the STRETCH critical-damage model (20260722-092320); sequence after
  it lands.

Pre-existing tasks that ride this spike (already tagged v0.9.0, now unblocked by
this design): 20260721-211520 (diegetic objectives -> tween to the shell's tab
handle anchor), 20260721-211526 (comms stack -> its icons feed the log section).

## Fix record

(Appended by each implementing task as it lands - keeps this doc the family's
single source of current state.)

- 20260724-102304 drawer shell + interaction model - LANDED 2026-07-24. Tab
  drawer as a third `PauseStates::Drawer` variant on the one freeze axis (reuses
  the pause freeze + cursor-free, minus the pause menu); slide via `Time<Real>`
  (bcs Tween is virtual-clocked); tab-handle anchor `DrawerTabAnchor` exposed for
  211520; expanded objectives section. 2 review rounds (out-of-context R1.1 caught
  an audio-loop freeze gap). See tasks/20260724-102304/.
- 20260721-211520 diegetic objectives - LANDED 2026-07-24. A new objective's big
  cockpit card tucks into the shell's `DrawerTabAnchor` (the spike's diegetic
  hand-off), superseding the gold posting ghost. Rides the drawer shell above.
  1 review round (out-of-context APPROVE). See tasks/20260721-211520/.
- (pending) 20260724-102309 drawer comms-log section
