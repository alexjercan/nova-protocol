# Notes: editor UI rework (baseline)

Task 20260714-204219. Design: `tasks/20260714-204059/SPIKE.md`.

## Module map (`crates/nova_editor/src/`)

- `lib.rs` - crate root: `NovaEditorPlugin`, `editor_plugin()` wiring, the
  `ExampleStates` state machine (Loading/Editor/Scenario), state routing,
  cursor grab, `switch_scene_editor` (F1), and the plugin-level state tests.
- `config.rs` - `PlayerSpaceshipConfig` (the ship being built, in serialized
  shape), `SectionChoice` (active tool), `SpaceshipPreviewMarker`,
  `SectionPreviewMarker`.
- `placement.rs` - `create_new_spaceship(_with_controller)`,
  `continue_to_simulation`, and the pointer observers
  `on_click/hover/move/out_spaceship_section` (raycast + surface-normal offset).
- `keybind.rs` - `EditorRebind`, `SectionKeybindLabel`, and the sync / position /
  apply systems for the section keybind chips + click-to-rebind, plus their tests.
- `scenario.rs` - `setup_scenario` + `test_scenario`, split into pure
  `sandbox_objects` / `sandbox_events` (unit-tested without a real `GameAssets`).
- `ui/`
  - `mod.rs` - assembles the scene (`setup_editor_scene`: light, camera, rail,
    drawer), owns the panel scroll (`EditorScrollPanel`, `scroll_editor_panel`),
    and `register`s the UI observers.
  - `theme.rs` - the web-wiki palette (`web/src/style.css` CSS variables) as
    `Color` consts + metrics (radius, border, icon size, rail/drawer widths).
  - `widget.rs` - `EditorButton`, `SelectedOption`, `ButtonValue<T>`,
    `button_on_setting`, the colour observers, and the themed `button` factory.
  - `rail.rs` - the `Components` category (opens the drawer) + greyed coming-soon
    category rows with a "soon" badge.
  - `drawer.rs` - `DrawerPanel`, `panel_header`, `toggle_drawer`.
  - `card.rs` - `ComponentCard`, `component_icon(kind)` (placeholder), and
    `component_card(section)`.
  - `tooltip.rs` - the hover tooltip (name / HP / description from `SectionConfig`).

## How selection stays autopilot-compatible

A component card is a themed button carrying `Name(section.base.name)` +
`EditorButton` + `ButtonValue(SectionChoice::Section(id))`. Pressing it (a real
click, or an autopilot `insert(Pressed)`) fires `button_on_setting::<SectionChoice>`,
which sets the tool and moves the `SelectedOption` highlight. So the existing
`09_editor` / `12_menu_newgame` autopilots - which look sections up by `Name` and
insert `Pressed` - needed no change.

## Deferred to "the rest" (task 20260714-081703)

- Export/load `*.scenario.ron`; the other rail categories (Ships / Objects /
  Events / Objectives) becoming real; factions; modifications beyond keybinds.
- Real component icons (a texture behind `component_icon`, plus an `icon` field in
  the content RON) - gated on the screenshot/asset tooling (task 20260714-081706).

## Follow-up filed

- Unify the WHOLE game UI (menu, HUD, editor) to the web-app theme - task filed
  20260714-2050xx (see the tasks/ tree). The editor now matches the wiki palette;
  `nova_menu` and the rest still use their own ad-hoc colours.
