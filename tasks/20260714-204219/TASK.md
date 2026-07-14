# Editor UI rework (baseline): wiki-style category rail + component drawer with icons/tooltips; player-only asteroid+planetoid scenario

- STATUS: CLOSED
- PRIORITY: 55
- TAGS: v0.6.0,editor,ui,spike

Spike: tasks/20260714-204059/SPIKE.md

## Goal

Re-work the sandbox editor UI to feel like the web wiki's `/wiki/sections/` page:
a left category RAIL, a component DRAWER of cards (placeholder icon + name, with a
hover TOOLTIP showing name/HP/description) that arms placement, restyled to the
wiki palette, with the `nova_editor` crate split into modules. The `Play` scenario
becomes a player-only asteroid field with a planetoid backdrop (no enemy, no
objective). "The rest" stays on task 20260714-081703.

## Steps

- [x] Split `crates/nova_editor/src/lib.rs` (2001 lines) into modules: `config`,
  `placement`, `keybind`, `scenario`, and `ui/{mod,theme,widget,rail,drawer,card,
  tooltip}`. `lib.rs` keeps the plugin + state + cursor + plugin tests.
- [x] `ui/theme.rs`: the wiki palette as `Color` consts + metrics (2px radius, 1px
  border, 44px icon, rail/drawer widths). Restyled buttons to 1px borders + 2px
  radius + crisp hover.
- [x] `ui/rail.rs`: `Components` active category (opens the drawer) + greyed
  coming-soon `Ships/Objects/Events/Objectives` rows with an amber "soon" badge.
- [x] Drawer toggle: `ui/drawer.rs` `DrawerPanel` + `toggle_drawer` observer wired
  to the Components category.
- [x] `ui/card.rs`: `component_card` from `GameSections` (icon + name), one per
  section in a scrollable list; `component_icon(kind)` placeholder isolated for
  later real art.
- [x] `ui/tooltip.rs`: hover a card -> floating panel with name/HP/description
  (read from `SectionConfig`), rebuilt per hover; positioned at the pointer.
- [x] Card selection arms placement via the existing `ButtonValue<SectionChoice>`
  + `button_on_setting` path; Select/Rebind + Delete kept as rail tools; keybind
  chip preserved.
- [x] `scenario.rs`: keep the 20 asteroids, ADD a large invulnerable planetoid
  gravity well, REMOVE the enemy ship + the destroy objective. Player unchanged.
- [x] Verify: `09_editor` autopilot (headless) still creates a ship, selects the
  hull CARD, and places it; workspace + examples compile; two scenario tests guard
  the enemy/objective removal + planetoid.

## Close-out

### What changed and why

- The single 2001-line `nova_editor/src/lib.rs` became a module tree: `config`
  (build-state + markers), `placement` (create ship + pointer place/preview/delete),
  `keybind` (chips + rebind), `scenario` (the Play scene), and a `ui` module with
  `theme`, `widget` (button infra), `rail`, `drawer`, `card`, `tooltip`. `lib.rs` is
  now just the plugin wiring, the `ExampleStates` state machine, cursor grab, and
  the plugin-level state-routing tests.
- The UI is now a left RAIL (categories + ship/tools + Play) next to a component
  DRAWER of cards, styled to the web wiki palette (deep navy panels, cyan/amber
  accents, 1px borders, 2px corners, crisp hover). Each card shows a kind-tinted
  placeholder icon + the section name and hovers a tooltip with name/HP/description
  pulled straight from `SectionConfig` (no new data model). The four unbuilt
  categories render as greyed "soon" rows, advertising "the rest".
- Play now drops into a combat-free scene: the asteroid field, one big invulnerable
  planetoid (a large asteroid - every asteroid is a PlanetHeight-displaced sphere -
  with an explicit gravity well), and the player ship. The enemy ship and the
  destroy objective are gone.

### Key design decisions / alternatives

- Cards ARE buttons: they reuse `EditorButton` + `ButtonValue<SectionChoice>` +
  the `button_on_setting` observer, so selection + highlight machinery is shared
  and the `09_editor`/`12_menu_newgame` autopilots (which find sections by `Name`
  and insert `Pressed`) keep working with ZERO example changes. Confirmed by the
  headless autopilot placing a section against the new card UI.
- Panel footprint (rail 150 + drawer 280 = 430px) is kept under half of the
  1024-wide window so the centred preview ship stays pickable - a wider left panel
  would let the UI block the placement raycast. Verified: the autopilot's
  screen-centre placement click still lands.
- Placeholder icon is isolated behind `component_icon(kind)` so a later task can
  drop in a real texture (and an `icon` field in content RON) without touching the
  card layout - both deferred to "the rest".
- Kept BOTH create-ship buttons (V1 hull / V2 controller) with their exact `Name`s;
  two example autopilots depend on "Create New Spaceship Button V2". Display text
  is friendlier ("New Ship" / "New Hull Ship").

### Difficulties

- `BCS_SHOT` cannot screenshot the editor: `nova_screenshot` force-advances to
  `Playing` on the first frame, before `GameAssets` loads, so `setup_editor_scene`'s
  `Res<GameAssets>` panics. This is a pre-existing incompatibility between BCS_SHOT
  and the editor example (not introduced here), so visual verification was done via
  the autopilot's real load path instead.
- `AssetRef` lives in `nova_gameplay::prelude`, `SpaceshipSectionConfig` in
  `nova_scenario::prelude` - the per-module prelude imports had to be narrowed after
  the split (the compiler's unused-import warnings pinned each one).

### Verification (per repo policy: check/fmt + newly-written tests; full suite is CI's)

- `cargo check --workspace --all-targets --features debug`: clean, no warnings.
- `cargo fmt -p nova_editor`; `cargo test -p nova_editor`: 12 pass (4 keybind, 1
  scroll, 5 plugin-routing, 2 new scenario guards).
- `09_editor` autopilot headless: clicked Sandbox -> created a ship -> selected the
  hull card -> placed a section (1 -> 2) -> cycle complete, no panic.
- NOT run locally (CI covers): the full workspace test suite.

### Self-reflection

- Keeping cards on the existing button/selection path was the highest-leverage
  decision: it made the autopilot pass unchanged and avoided reimplementing
  selection. Worth reaching for a shared mechanism before a bespoke one.
- The window-width / picking-footprint hazard was caught by reading the window
  resolution up front rather than after a failed autopilot run - cheap and it paid.

## Notes

See `tasks/20260714-204219/NOTES.md` for the module map and the follow-ups.
Explicitly OUT (task 20260714-081703): export/load `*.scenario.ron`, placing
non-ship objects, events/objectives authoring, factions, modifications beyond
keybinds, real icons.
