# Spike: stations, ship inventory, and the NOVA OS/UI direction

- STATUS: OPEN
- PRIORITY: 80
- TAGS: v0.15.0,spike,ui,stations,inventory,design

Rewritten in place on 2026-09-21 at owner direction. This was a one-paragraph
ideation note ("dockable space stations as first-class scenario objects");
it is now the highest-priority v0.15.0 spike. Epic: `20260921-231507`.

This is a SPIKE. It produces decisions, recorded open choices, and child
tasks. It does not implement stations, inventory, or the UI rework.

## What it explores

Three questions that turn out to be one question:

1. **Stations.** Dockable structures the player visits: what a station IS in
   this game, and what a visit is FOR.
2. **Ship inventory.** What the ship carries, and therefore what a station
   trades, repairs, rearms, or stores.
3. **The NOVA OS / UI direction.** How the player operates any of it.

They are one question because a station's services have no meaning until
there is something to carry, and neither has a shape until the UI that
drives them is decided.

## The UI direction the owner wants

An approachable, click-based, THEMED UI aimed at an average player. Not a
command line as the price of entry.

- The map view and the ship view MIGRATE toward that UI.
- The NOVA OS terminal is RETAINED, primarily for advanced and cheat
  commands. It stays; it stops being the front door.
- Do NOT prescribe terminal-first station services. A station's services
  being typed commands is exactly the thing this direction moves away from.
  If the spike finds a service that genuinely belongs on the terminal, it
  argues for it with evidence.

The spike owns the DIRECTION and the boundary: which surfaces move, which
stay, and what "themed click-based UI" concretely means against the theme
machinery that already exists. It does not build the screens.

## Verified seams, 2026-09-21

What exists today, checked against master:

**Docking exists, as a mechanic and as an event.**
- `crates/nova_events/src/lib.rs:422` `OnDockedEvent`, `:440` its undock
  counterpart, `:453` `DockingEventInfo` (the pair in the roles it docked in).
- `crates/nova_scenario/src/loader/trackers.rs:371` `DockEcho` is held on
  BOTH hulls; `trackers.rs:436` fires `OnDockedEvent` from the live
  connection entity built by the `DOCK` verb.
- `crates/nova_scenario/src/events.rs:130` exposes `OnDocked` to authored
  scenario events, so content can already react to a dock.
- `crates/nova_hud/src/docking_sight.rs` is the existing docking HUD.

Docking is a solved mechanic with no destination. A station is what makes it
worth doing.

**Map and ship are NOVA OS apps, not standalone UI.**
- `crates/nova_os/src/app.rs:84` `NovaOsAppRuntime` is the app seam
  (`spawn_body` / `handle_key`).
- `crates/nova_os_ui/src/map/app.rs:109` `MapApp` implements it.
- `crates/nova_os_ui/src/ship/app.rs:21` `ShipApp` implements it.
- `crates/nova_os/src/command.rs:92` registers an app as a terminal command.

So "migrate map and ship toward the new UI" means moving them OFF this seam,
or redefining what the seam hosts. That is a real interface decision and it
belongs to this spike.

**UI themes already exist.**
- `crates/nova_ui/src/theme/config.rs:51` `UiThemeConfig`, `:97` `ThemeColor`.
- `crates/nova_ui/src/theme/registry.rs:34` `GameUiThemes`, `:57`
  `SelectedUiTheme`, `:77` `UiThemeSystems`.
- Themed widget primitives exist: `ThemedPanel`
  (`crates/nova_ui/src/widget/panel.rs:23`), `ThemedText`
  (`crates/nova_ui/src/widget/chrome.rs:28`), and others.

A themed click-based UI is not starting from nothing. The spike should say
what the existing theme layer already gives and what it is missing.

**Inventory, credits, and save state do NOT exist.**
No inventory, credits, or cargo-hold runtime exists in `crates/`. Nor does
any save or world-persistence runtime; the only persistence is
`crates/nova_assets/src/storage.rs:41`, a platform key-value store used for
settings and training progress. Everything about what the ship owns has to
be invented, and inventing it is a decision about persistence, not just
about a screen.

Do not carry forward any speculative schema or type for these. None is
approved, and a scout's proposed struct is not evidence.

## Decisions the spike must reach or explicitly leave open

- **What a station is**: a scenario object, a world object, or both; who
  spawns it; what identifies it.
- **What a visit does**: which services exist at all, and what each one
  changes in game state.
- **What the ship owns**: inventory shape, credits or no credits, and
  whether either is a mod-extensible content kind.
- **Persistence**: whether any of it survives a session, and if so in what.
  This is shared with `20260824-125938` and must be decided once.
- **The UI boundary**: which surfaces become themed click-based UI, which
  stay on the terminal, and what happens to the `NovaOsAppRuntime` seam when
  map and ship leave it.
- **Migration order**: whether map or ship moves first, and whether the old
  surface is deleted at the same time. Per the change policy, a migrated
  surface's old path is deleted, not kept beside the new one.

Record each as decided-with-evidence or as an open decision with options and
one consequence each. An unsettled choice stays a choice.

## Coordinate with the open-world spike

`20260824-125938` runs beside this one. They SHARE three boundaries:

- **Persistence**: inventory and credits are world state. One format,
  decided once, not twice.
- **Content ownership**: a station is a world anchor and a scenario object.
- **UI surface**: travel, the chart, and docking land in whatever UI
  direction this spike chooses.

Neither task may silently settle the other's question. Where they disagree,
record it as an open decision on BOTH tasks and take it to the owner.

## Output: proposals to validate, not promises

Group the candidate outcomes into three lists. Each list is a PROPOSAL for
the owner to validate. Nothing is committed by appearing in it.

- **Needed for the game to feel complete at 1.0**
- **Nice to have**
- **Later / post-1.0**

Each entry names the child task it would become.

## Done when

- The current docking, UI, and theme seams are captured with verified
  `path:line` evidence, including what is absent.
- Every decision above is either decided with evidence or recorded as an
  open decision with options and consequences.
- The UI direction is concrete enough to tell, for each existing surface,
  whether it migrates, stays, or dies - without having built it.
- The shared boundaries with `20260824-125938` each carry an agreed decision
  or a recorded open decision on both tasks.
- Child tasks exist, each in one of the three classification lists.
- No station, inventory, or UI code, content, or schema has been written by
  this task.
