# Editor UI rework (baseline): wiki-style category rail + component drawer with icons/tooltips; player-only asteroid+planetoid scenario

- STATUS: OPEN
- PRIORITY: 55
- TAGS: v0.6.0,editor,ui,spike

Spike: tasks/20260714-204059/SPIKE.md

## Goal

Re-work the sandbox editor UI to feel like the web wiki's `/wiki/sections/` page:
a left category RAIL, a component DRAWER of cards (placeholder icon + name, with a
hover TOOLTIP showing name/HP/description) that arms placement, restyled to the
wiki palette, with the `nova_editor` crate split into modules. The `Play` scenario
becomes a player-only asteroid field with a planetoid backdrop (no enemy, no
objective). "The rest" (export/load, other objects, events, factions,
modifications, real icons) is explicitly OUT (stays on task 20260714-081703).

Done = launching the editor shows the rail + Components drawer of cards with
tooltips; selecting a card places that section (autopilot flow still green); the
rail shows greyed coming-soon rows for Ships/Objects/Events/Objectives; and
pressing Play drops the built player ship into an asteroid+planetoid scene with no
enemy ship and no destroy objective.

## Steps

### Crate split + theme

- [ ] Split `crates/nova_editor/src/lib.rs` (2001 lines) into modules under
  `crates/nova_editor/src/`: `plugin.rs` (plugin wiring, states), `scene.rs`
  (`setup_editor_scene`, camera/light/skybox), `placement.rs` (raycast place /
  hover-preview / delete: `on_click/hover/move/out_spaceship_section`,
  `SectionChoice`, `PlayerSpaceshipConfig`), `keybind.rs`
  (`SectionKeybindLabel`, rebind state machine), `scenario.rs` (`test_scenario`),
  and `ui/` (`mod.rs`, `theme.rs`, `rail.rs`, `drawer.rs`, `card.rs`,
  `tooltip.rs`, `widget.rs`). Keep `lib.rs` as the crate root re-exporting the
  plugin. Pure move + re-export first; no behavior change in this step so the
  `09_editor` example still passes.
- [ ] Add `ui/theme.rs` with the wiki palette as `Color` consts (bg #0b0f1c,
  panel #141a2e, panel-2 #1a2138, border #233052, border-bright #3a4d7a, cyan
  #5cc8ff, cyan-bright #8fe0ff, amber #ffb877, text #e8eefc, muted #8b95b0) plus
  small spacing/border consts (2px radius, 1px border, card 44px icon, 14px gap).
  Replace the ad-hoc `NORMAL_BUTTON`/`HOVERED_BUTTON`/... consts with theme
  references. Restyle the existing panels/buttons to 1px borders + 2px radius +
  crisp hover (border brighten + text color shift), dropping the pill
  `BorderRadius::MAX`.

### Category rail

- [ ] Add `ui/rail.rs`: a left vertical rail (`RailPanel` marker) of category
  rows. `Components` is active (opens the drawer); `Ships`, `Objects`, `Events`,
  `Objectives` render greyed (opacity ~0.7, non-`Button`) with an amber "soon"
  badge (coming-soon = "the rest"). Keep `Create New Ship` (fold the two existing
  V1/V2 create buttons - decide in review whether to keep both) and `Play` on the
  rail. Category rows use a monospace-ish uppercase label styled like the wiki
  `.wiki-nav__cat`.
- [ ] Add a `DrawerState` resource/marker so clicking `Components` toggles the
  drawer open/closed (one category open at a time). Resolve the exact affordance
  (toggle vs auto-close on select) here or at first review.

### Component drawer + cards + placeholder icons

- [ ] Add `ui/card.rs` + `ui/drawer.rs`: the drawer (`DrawerPanel` marker, beside
  the rail) holds a CARD GRID built from `GameSections` - one `ComponentCard`
  (carrying the section `id`) per entry, laid out to mimic the wiki
  `.wiki-children` grid (wrap of ~220px min cards, 14px gap, scrollable via the
  existing `ScrollPosition`/`Overflow::scroll_y()` pattern). Each card = a
  placeholder icon + the section's `base.name`.
- [ ] Add `component_icon(kind: &SectionKind) -> impl Bundle` in `ui/card.rs`: a
  44x44 node with a bright dashed-ish border and a fill tinted per `SectionKind`
  (Hull/Thruster/Controller/Turret/Torpedo distinct), standing in for the wiki's
  hatched placeholder. Isolate it so a later task can swap in a real texture
  without touching call sites.

### Tooltip

- [ ] Add `ui/tooltip.rs`: on `Pointer<Over>` of a `ComponentCard`, show/position a
  floating `Tooltip` node (reuse the screen-projected `Node.left/top` overlay
  pattern from `keybind.rs`) populated from the card's `SectionConfig`: `base.name`
  (title), `HP {base.health}`, and `base.description`. On `Pointer<Out>` hide it.
  Style it as a small themed panel.

### Wire selection to placement

- [ ] Replace the flat per-section tool buttons: selecting a `ComponentCard` sets
  `SectionChoice::Section(id)` (reuse the existing `button_on_setting` path or a
  card-specific observer) and marks the card selected (themed active state). Keep
  `Select/Rebind` and `Delete Section` as tools (rail buttons or drawer controls)
  and keep the `SectionKeybindLabel` chip as the one "simple modification".
  Placement mechanics (`placement.rs` raycast + `normal * 1.0`) are unchanged.

### Player-only asteroid + planetoid Play scenario

- [ ] In `scenario.rs::test_scenario`, keep the 20-asteroid loop; ADD one large
  `ScenarioObjectKind::Asteroid` as the "planetoid" - big radius (~40-60),
  `invulnerable: true`, `surface_gravity: Some(...)` for a gravity well,
  positioned as a distant backdrop (e.g. well below/beside the field). REMOVE the
  `other_spaceship` object entirely.
- [ ] In the same fn, drop the enemy-related events: remove the
  `destroy_spaceship` Objective action, the `other_spaceship` `OnDestroyed`
  event, keep only the `OnStart` spawn of (asteroids + planetoid + player) and the
  optional player-destroyed `DebugMessage`. Player ship spawn/config unchanged.

### Verify

- [ ] Run `cargo check -p nova_editor` and `cargo fmt`. Run the `09_editor`
  example headless (`DISPLAY=:0 BCS_AUTOPILOT=1 cargo run --example 09_editor
  --features debug`) and confirm the autopilot place-a-section flow still passes
  against the new drawer/card UI (update the autopilot's `button_by_name`/card
  lookup in `examples/09_editor.rs` if the section button became a card). Launch
  interactively once (menu -> Sandbox) to eyeball the rail, drawer, tooltips, and
  the planetoid scene.
- [ ] Update `examples/09_editor.rs` autopilot if the section-selection affordance
  changed (flat button -> card): find the card by section name, click it, then
  place as before. Keep the `Play` -> scenario assertion valid (no enemy ship).

## Notes

- Relevant files: `crates/nova_editor/src/lib.rs` (the whole current editor);
  `examples/09_editor.rs` (autopilot: `button_by_name` line ~207,
  `aim_at_a_section`, phases); `crates/nova_gameplay/src/sections/base_section.rs`
  (`SectionConfig { base: BaseSectionConfig { id, name, description, mass, health },
  kind: SectionKind }`, `GameSections(Vec<SectionConfig>)`, `get_section`);
  `crates/nova_scenario/src/objects/asteroid.rs` (`AsteroidConfig { radius,
  texture, health, surface_gravity, invulnerable, lock_signature }`; every asteroid
  is a `PlanetHeight`-displaced sphere scaled by radius - a big invulnerable one IS
  the planetoid, verified insert_asteroid_render:256-273);
  `crates/nova_scenario/src/actions.rs:1158` (`ScenarioObjectKind`:
  Asteroid/Spaceship/Beacon/SalvageCrate - NO dedicated Planet kind, so use a large
  Asteroid); `web/src/style.css` + `web/src/wiki.ts` (palette, `.wiki-children`
  card grid, `.wiki-child__icon` placeholder) as the visual reference.
- Reuse from the crate: local `button()` factory, `observe(Activate)`,
  `button_on_setting`/`button_on_interaction` observers, `ScrollPosition` +
  `Overflow::scroll_y()` scroll, and the `SectionKeybindLabel` screen-projection
  (`position_section_keybind_labels`) - the tooltip copies this projection pattern.
- Tooltip/card data need NO new model: name/HP/description are already on
  `BaseSectionConfig`; category is `SectionKind`.
- Placeholder icon is deliberately behind `component_icon(kind)` so "the rest" can
  drop in a real `icon` texture (and an `icon` field in content RON) without
  touching call sites.
- Assumption (from spike): keep the drawer editor-only; do NOT retheme `nova_menu`
  in this task.
- Still `spike`-tagged only because it was seeded by a spike; this plan IS the
  step breakdown, so implementation can start.
- Explicitly OUT (task 20260714-081703): export/load `*.scenario.ron`, placing
  non-ship objects, events/objectives authoring, factions, modifications beyond
  keybinds, real icons.
