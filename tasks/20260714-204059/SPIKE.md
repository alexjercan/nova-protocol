# Spike: editor UI rework - a wiki-style drawer for placing components (baseline slice)

- DATE: 20260714-204059
- STATUS: RECOMMENDED
- TAGS: spike, v0.6.0, editor, ui

## Question

The sandbox editor today is a flat 400px scroll panel of pill buttons
(`crates/nova_editor/src/lib.rs`, one 2001-line file). We want to re-work it to
feel like the web wiki's `/wiki/sections/` page: browse placeable content by
CATEGORY, see each item as a card with a placeholder icon and a tooltip
(name + HP + description), pick one, and place it. The user framed the target UI
as a DRAWER: click a category (e.g. "Components") and a panel slides open with
that category's items; select one to arm placement and do simple per-item
tweaks (keybinds).

This spike scopes and designs the BASELINE slice only. It must answer:

1. What UI structure delivers the wiki "look nice" (category browse + card grid +
   icon + tooltip) inside a Bevy `bevy_ui` editor, without eating the 3D
   viewport the editor exists to show?
2. Where do the card data (name, HP, description, icon) come from, given no
   real icons exist yet?
3. What is the smallest end-to-end baseline that is worth shipping, and what is
   explicitly deferred to "the rest"?

A good answer is concrete enough that `/plan` can break it into steps without
re-litigating the layout, the data source, or the baseline/deferred cut.

## Context

- **Editor today** (`crates/nova_editor/src/lib.rs`, 2001 lines, no submodules):
  a left 400px `EditorScrollPanel` (`Overflow::scroll_y()` + `ScrollPosition`)
  holds a title, two "Create New Spaceship" buttons, a flat column of tool
  buttons (`Select / Rebind`, one button per section id from `GameSections`,
  `Delete Section`), and a `Play` button. Tool selection writes a `SectionChoice`
  resource (`None` | `Section(id)` | `Delete`) via a `button_on_setting` observer;
  placement is a raycast (`PointerInteraction` nearest hit + `hit.normal`) that
  spawns the section as a child of `SpaceshipPreviewMarker`, offset by `normal * 1.0`.
  Per-section keybind chips (`SectionKeybindLabel`) are `bevy_ui` nodes projected
  to screen space (`Node.left/top`) - a proven floating-overlay pattern we can
  reuse for tooltips. Buttons come from a local `button(text)` factory; colors are
  a state observer (`button_on_interaction`). `PlayerSpaceshipConfig` holds the
  built ship (sections + input bindings) handed to the scenario on `Play`.
- **Placement / scenario today**: on `Play`, `test_scenario()` spawns 20 random
  asteroids, an ENEMY ship, and the player ship at `(0,0,50)` with a
  "destroy_spaceship" objective. There is NO planetoid in the editor scenario.
- **Data we can show on cards** (grounded): `GameSections(Vec<SectionConfig>)`;
  each `SectionConfig { base: BaseSectionConfig { id, name, description, mass,
  health }, kind: SectionKind::{Hull,Thruster,Controller,Turret,Torpedo} }`
  (`crates/nova_gameplay/src/sections/base_section.rs:27-54`). So name + HP
  (`health`) + description + a category (`kind`) are ALL already available per
  section - tooltips need no new data model. There is NO `icon` field.
- **Planetoid exists** (grounded): `crates/nova_scenario/src/objects/asteroid.rs`
  has a full procedural planet (`PlanetHeight`, seed/scale/noise) plus asteroid
  spawning. "Asteroid field with a planetoid" is a spawn-composition task, not
  new tech. (The user's "like 03_scenario" is a mislabel: `03_hull_section` is a
  section demo and the scenario demo is `08_scenario`; the intent is
  asteroids + a planet backdrop, player ship only.)
- **The wiki `/wiki/sections/` look** (from `web/src/style.css`, `wiki.ts`,
  `wiki-pages.ts`): a sticky sidebar (232px) of monospace uppercase CYAN category
  labels with left-border active/hover rails, next to a content column whose
  "sections" render as a CARD GRID (`repeat(auto-fill, minmax(220px,1fr))`,
  14px gap). Each card = a 44x44 icon (placeholder: 1px DASHED border-bright +
  45-degree diagonal cyan-hatch gradient) beside a title (Rajdhani, cyan-bright on
  hover) + a muted summary. Palette: bg `#0b0f1c`, panel `#141a2e`, border
  `#233052`, border-bright `#3a4d7a`, cyan `#5cc8ff`, cyan-bright `#8fe0ff`,
  amber `#ffb877`, text `#e8eefc`, muted `#8b95b0`; 2px radius, 1px borders,
  crisp hover (border brighten + text color shift, NO glow). Coming-soon entries
  render greyed (opacity ~0.7) with an amber "soon" badge and are non-clickable.
  This exact vocabulary - category rail, card grid, hatched placeholder icon,
  crisp hover, coming-soon badge - is what we mirror in `bevy_ui`.

## Options considered

### A. Category rail + sliding drawer (RECOMMENDED)

A thin vertical RAIL pinned to the left edge lists categories as buttons:
`Components` (active), and greyed coming-soon rows `Ships`, `Objects`, `Events`,
`Objectives` (matching the user's "other things unclickable buttons"). Clicking a
category opens a DRAWER panel beside the rail showing that category's CARD GRID
(the wiki `.wiki-children` grid). Selecting a card arms `SectionChoice::Section(id)`
and (for bindable kinds) surfaces the existing keybind chip as the "simple
modification". One category open at a time; the drawer can be dismissed to free
the viewport. Top-of-rail keeps `Create New Ship` and `Play`.

- Pros: matches the user's explicit "drawer" language; keeps the 3D viewport
  clear except while browsing; the rail-plus-content split is a faithful analog
  of the wiki sidebar+content; card grid + tooltip reuse the wiki structure
  directly; coming-soon rows advertise "the rest" for free.
- Cons: most new UI code (rail, drawer open/close state, card grid builder,
  tooltip). Drawer show/hide is new interaction state to get right.
- Unknowns: exact open/close affordance (toggle vs auto-close on select) - a
  detail for `/plan`, not a blocker.

### B. Persistent two-pane sidebar (wiki port, always-on)

Port the wiki 1:1: a permanent 232px category list + an always-visible card grid
column. No drawer animation.

- Pros: simplest interaction model (nothing opens/closes); closest pixel match to
  the web page.
- Cons: permanently consumes a wide strip of the screen in a tool whose entire
  point is the 3D scene; contradicts the user's "drawer" ask; the always-on grid
  competes with the viewport for attention.

### C. Tabbed panel (keep the 400px panel, add tabs)

Keep today's single left panel; add a tab strip (`Components` / `Ships` / ...) at
its top; each tab swaps the panel body.

- Pros: least new code; reuses the existing scroll panel and button factory
  almost verbatim.
- Cons: not a drawer and not the wiki card-grid feel; tabs inside a narrow column
  can't show a real card grid with icons; least of the three on "look nice".

### D. Radial / hotbar palette

A bottom hotbar or radial of component icons.

- Rejected: no category browse, no room for tooltips/descriptions, and nothing
  like the wiki. Wrong reference entirely.

### Sub-question: where do card data + icons come from?

- Name / HP / description: read straight from `SectionConfig.base` and
  `SectionKind` - already present, no model change. (RECOMMENDED.)
- Icon: no `icon` field exists. Reproduce the wiki PLACEHOLDER exactly - a 44x44
  node with a bright dashed border and a diagonal cyan-hatch fill, tinted by
  `SectionKind` so kinds are visually distinct. Wrap it behind a small
  `component_icon(kind)` helper so a later task can swap in a real texture
  (mirroring the wiki's optional `icon?` field) without touching call sites.
  Adding a real `icon: Option<String>` to the content RON + real art is "the rest"
  (and gated on the screenshot/asset tooling from task 20260714-081706).

### Sub-question: tooltip mechanism in `bevy_ui`

Reuse the editor's existing screen-projected overlay pattern
(`SectionKeybindLabel` positions a node via `Node.left/top`): on
`Pointer<Over>` of a card, show/position a floating `Tooltip` node populated with
name + HP + description; on `Pointer<Out>`, hide it. This is a proven, low-risk
pattern already in the crate - no new mechanism. (Showing stats inline in the card
instead was considered but the user explicitly asked for a tooltip, and inline
stats bloat the grid.)

## Recommendation

**Build option A (category rail + sliding drawer), baseline slice only.** It is
the only option that honors the user's "drawer" framing while protecting the 3D
viewport, and it maps the wiki vocabulary (category rail, card grid, hatched
placeholder icon, crisp hover, coming-soon badge) onto `bevy_ui` one-for-one. Cards
read `SectionConfig` (name/HP/description/kind) - no new data model - and the
tooltip and placeholder-icon patterns already exist in the crate or are trivial
nodes. Because the UI is being substantially reworked, split the 2001-line
`nova_editor/src/lib.rs` into modules as part of the work (theme, rail, drawer,
card, tooltip, placement, scenario) so the crate stays maintainable and "the rest"
can extend it cleanly.

**Baseline scope (ship this):**

- Restyle the editor to the wiki palette/typography (industrial HUD: `#0b0f1c`
  bg, `#141a2e` panels, `#233052`/`#3a4d7a` borders, cyan/amber accents, 2px
  radius, 1px borders, crisp hover) via a shared `theme` module. `nova_menu` can
  later adopt the same theme, but this task only touches the editor.
- Category RAIL: `Components` active; `Ships`, `Objects`, `Events`, `Objectives`
  rendered as greyed coming-soon rows (non-clickable, amber "soon" badge) to
  advertise "the rest".
- DRAWER with a CARD GRID for Components: one card per `GameSections` entry -
  placeholder hatched icon tinted by `SectionKind` + prettified name; hover shows
  a tooltip with name + HP + description. Selecting a card arms placement
  (replacing today's flat tool-button list). Keep `Select/Rebind` and `Delete`
  as tools (rail buttons or drawer controls) and keep the keybind chip as the one
  "simple modification".
- Placement unchanged mechanically (raycast + normal offset), just driven from the
  card selection.
- Baseline SCENARIO on `Play`: an asteroid field + a planetoid backdrop (reuse
  `nova_scenario::objects::asteroid`) with the PLAYER ship only. Drop the enemy
  ship and the destroy objective for baseline (they belong to
  events/factions/objectives = "the rest").
- Split `nova_editor` into modules as part of the rework.

**Explicitly DEFERRED to "the rest" (do NOT build now):**

- Export/load scenarios to/from `*.scenario.ron` (the original 081703 goal).
- Placing non-ship objects (asteroids, planetoids, beacons, salvage) as editor
  content - the other rail categories become real.
- Events, objectives wiring, factions (player vs enemy), and modifications beyond
  keybinds.
- Real component icons + an `icon` field in the content RON (gated on the
  screenshot/asset tooling, 20260714-081706).

## Open questions

- Drawer dismissal affordance (toggle button vs auto-close on select vs stays
  pinned) - resolve in `/plan` or first review; not load-bearing for the design.
- Whether the rail should also expose `Select/Rebind` + `Delete` as tools or fold
  them into the drawer - a layout detail for `/plan`.
- Should `nova_menu` share the new `theme` module now or later? Recommend later
  (keep this task editor-only); note it so a future task unifies them.
- Prettifying section ids into display names: `SectionConfig.base.name` already
  holds a human name, so prefer it directly; only fall back to id-prettifying if a
  name is blank.

## Next steps

Direction-level tasks this spike seeds, for `/plan` to break into steps:

- tatr 20260714-204059 is this spike (closed once written).
- Baseline task (build now, via `/flow`): "Editor UI rework - wiki-style category
  rail + component drawer + tooltips; player-only asteroid+planetoid scenario".
  See the seeded task below.
- "The rest" umbrella stays task 20260714-081703 (export/load + objects + events +
  factions + modifications + real icons), re-pointed at this spike; it plans once
  the baseline lands.

## Fix record

(Appended by each implementing task as it lands.)
