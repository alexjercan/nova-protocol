# Spike: stations, ship inventory, and the NOVA OS/UI direction

- STATUS: CLOSED
- PRIORITY: 80
- TAGS: v0.15.0,spike,ui,stations,inventory,design

Rewritten in place on 2026-09-21 at owner direction. This was a one-paragraph
ideation note ("dockable space stations as first-class scenario objects");
it is now the highest-priority v0.15.0 spike. Epic: `20260921-231507`.

This is a SPIKE. It produces decisions, recorded open choices, and child
tasks. It does not implement stations, inventory, or the UI rework.

Closed on 2026-09-26 after the owner accepted PR #77's UI look and split the
remaining work into `20260926-174836` (breaking TAB/app/command migration) and
`20260926-174806` (station/inventory research). PR #77 merged separately as
`2bbd743ca`, an example-only sketch. Closing this research spike does not ship
the new UI or approve station mechanics. The original direction and
open-choice lists below record earlier research; the newer owner decision in
"Closure and handoff" supersedes them where they conflict.

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

## Progression and world-system research, 2026-09-25

`PROGRESSION-RESEARCH.md` audits the current streamed bootstrap, cluster
composition, free repair/reload and idle-batch ammo, and the missing station,
inventory, persistence and refit seams. It records the owner's proposed
P* A+ S* composition as a question for **nonempty clusters**, not a shipped
sector rule; derelict-only currently has no rocks, and placement skips can
leave no rock. It compares tangible salvage/dock/ship-project loops with
faction, encounter, station UI and WFC alternatives. All entries remain
research candidates, not approved generator policy or schemas. Four
research-only child task specifications now exist; none authorizes a feature.
It also flags that `20260824-125938/RESEARCH.md` predates the current one-
bootstrap streamed world and must not be treated as implemented architecture.

## Deeper comparative research, 2026-09-25

- `COMPARATIVE-RESEARCH.md` uses official game descriptions to compare
  physics-driven mining, physical salvage, modular ship upgrades and broad
  sandbox economies. It tests the idea of one deliberate field-to-dock-to-
  changed-ship loop against Nova's lore. Crew simulation, skill trees and
  galaxy-scale breadth remain outside the owner's goal.
- `ASSET-SCOUT.md` separates already credited local source art, original
  game assets, and one verified external CC0 candidate from license-unknown
  suggestions. No third-party asset was imported.
- The gravity backlog task `20260925-182711` separately researches how to
  remove the gravity exemptions for *all mobile* rocks and neutral ships while
  keeping static planet/anchor wells. Mining of moving asteroids depends on
  that lifetime/ownership decision; this spike does not choose its physics.
- The owner rejected the two HTML concept pages from this research as bad
  design, and they were deleted before merge. They endorse no screen, layout
  or interaction. Only the behavioral lessons below are kept.

## Owner UI review, 2026-09-25

Recorded direction for future design. No runtime code and no replacement
mockup exist; any new visual is future design work.

**Approved direction**
- Map and ship visuals stay similar to the existing NOVA OS `map` and `ship`
  apps, but are hosted in normal themed UI tabs, not the CRT/terminal. Today
  both render an offscreen `Camera3d` image through the NOVA OS CRT composite
  with projected clickable UI blips (`crates/nova_os_ui/src/map/mod.rs:1-19`,
  `crates/nova_os_ui/src/ship/mod.rs:12-27`), in phosphor colors
  (`crates/nova_os_ui/src/map/scene.rs:86-94`).
- Less text. No visible logs. Today the apps write result and error rows to
  the terminal scrollback (`crates/nova_os_ui/src/map/app.rs:52-68`,
  `crates/nova_os_ui/src/ship/app.rs:216-224`).
- The map may show sector boundary lines, the ship-forward orientation and
  more meaningful contact labels. Today the scene draws distance rings and a
  central hub (`map/scene.rs:86-94`); a search of `map/scene.rs` finds no
  sector or heading geometry. Labels are a kind prefix plus a per-kind index
  such as `AST-2` (`crates/nova_os_ui/src/map/contacts.rs:66-75,445-456`).
  Forward orientation reaches the player only as the bearing number in the
  readout text (`contacts.rs:121-155`).
- A ship projection with clickable components. Today section blips are
  clickable and the app has Repair, Reload and Rebind buttons
  (`crates/nova_os_ui/src/ship/scene.rs:661-662`,
  `crates/nova_os_ui/src/ship/app.rs:109-122`).
- Inventory must be interesting to look at, not a text ledger.
- Undocked, the player may view map, inventory and ship. Transfers, trades
  and other station services are dock-gated.

**Open decisions**
- Ship appearance: the existing block wireframe (`ship/mod.rs:12-19`)
  against an editor-style 3D appearance. Consequence: the wireframe reuses
  the current proxy blocks; the 3D look needs a render path that shows real
  section models.
- Station UI entry: a dedicated key against shared tabs opened with TAB.
  Input conflict: TAB is the `novaos_toggle` keyboard binding
  (`crates/nova_os_ui/src/bindings.rs:48-51`). In flight it opens the NOVA OS
  shell (`crates/nova_os_ui/src/terminal/input.rs:62-80`); at the NOVA OS
  prompt, TAB is command completion (`terminal/input.rs:235-247,308`). Consequence: a
  dedicated key leaves TAB with NOVA OS; TAB-opened shared tabs take the
  flight press from NOVA OS, so the terminal needs another entry or becomes a
  tab, and completion at its prompt must not also switch tabs.

**Behavioral lessons kept from the deleted pages**
- A service is available only while docked to the right partner with the
  port intact. Undocked, a lost port and an unloaded station record all
  refuse and show no default services.
- A broken dock port releases the joint and fires `OnUndocked`; it is not a
  docked-fault state. A transaction in progress at undock needs a rollback or
  refusal decision.
- Free ship-app Repair and idle ammo refill leave a station nothing
  exclusive to offer until the owner changes one of them.
- A repeat salvage claim must refuse. Claim lifetime is a persistence
  choice: none lets the player farm by retiring a sector, session resets on
  restart, save keeps the claim across restart.

## UI sketch example, 2026-09-26

`UI-APP-VARIANTS.md` records a playable, example-local sketch of themed map,
ship and inventory screens: `examples/playable/ui_app_variants.rs`, with
frames in `ui-app-variants/`. It runs over a paused fixture scenario with
three mock contexts: undocked, at a mock station, and boarded on a parked
raider. Station trade is paid, boarded loot is free, and repair works only at
the station. All of it is example-local fixture state that changes no
gameplay state. It adds no station, inventory, docking, boarding or UI
runtime, content or schema, and leaves both open decisions above open. Its
limits are listed in that file.

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

Each entry names the child task it would become. Research-only child task
specifications now exist, but their classification and feature scope remain
**proposals for owner review**, not a v0.15.0 delivery commitment:

- **Needed for the game to feel complete at 1.0 (candidate):**
  `20260925-190156` shared streamed-world mutation and save contract;
  `20260925-190219` design of one dock-salvage-to-physical-ship-payoff loop,
  including station identity and free repair/reload policy. A refit is one
  option against design swap or another visible payoff; durable saving is a
  candidate, not an approved format or launch feature.
- **Nice to have (candidate):** `20260925-190207` P* A+ S* asteroid-group
  composition and field readability; `20260923-110307` existing catalog
  derelict-selection task; `20260925-190131` situated faction encounters and
  station jobs. The owner has not approved any generator change.
- **Later / post-1.0 (candidate):** runtime WFC-generated stations or
  derelicts, a multi-station economy, continuous faction conflict and a
  broader ship/service cast. These are research options, not new tasks or
  priority claims; they need measured cost and a coherent first loop.

## Closure and handoff, 2026-09-26

- **UI direction decided:** the themed, 3D map/ship and illustrative inventory
  sketch in `UI-APP-VARIANTS.md` / PR #77 is the accepted *visual direction*.
  Its fixture economy, stock, loot, repair, credits and contexts are not live
  gameplay or approved service rules. PR #77 merged as `2bbd743ca`; the
  example stays example-only and adds no gameplay runtime.
- **Breaking input/interface decision:** TAB will open the new themed panels,
  not the old NOVA OS computer. `:` must continue to open the command entry
  with its existing capabilities. Migrate NOVA OS apps and commands into the
  new interface and command entry; delete the old app host/CRT screen and
  obsolete paths, adapters, aliases and defaults. No backward compatibility.
  The earlier "terminal stays" and dedicated-key-versus-TAB choice above are
  superseded by this decision; the command *capability* stays, not the old
  terminal-first UI. The old command registry's split between launching apps
  and performing gameplay actions needs a code-backed migration design in
  `20260926-174836`, not an assumed rewrite of command effects.
- **Station/inventory choices delegated:** station identity, visits, inventory
  and market ownership, transaction/undock failure, repair/reload policy,
  physical payoff and UI integration belong to `20260926-174806` with existing
  `20260925-190219`. Persistent player/world mutations and shared save policy
  remain with `20260925-190156`. Neither child inherits approved fixture
  numbers or services. This is an explicit recorded open decision, not a
  missing implementation claimed as done.
- **Classification:** UI replacement is a v0.15.0 follow-up; station/inventory
  mechanics remain v0.15.0 research, not a release commitment to implement.
  Other child proposals and their candidate 1.0/nice-to-have/later categories
  remain recorded above. The research handoff, not their delivery, closes the
  spike.

## Done when (original spike acceptance)

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
