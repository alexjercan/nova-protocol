# Add moddable RON UI themes

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: backlog

## Objective

Replace the closed `UiSkin` choice with moddable, data-only UI themes authored
as normal RON mod content. Base provides `base/phosphor` and `base/hardware`.
Settings discovers themes from enabled mods and persists one stable theme ID.

The first release covers the same visual scope as the current skins. It changes
paint and simple shape, not UI structure or placement.

## Grounding

- `UiSkin` is currently a two-variant enum in
  `crates/nova_ui/src/skin.rs:33`.
- Shared widget reconcilers cover buttons, list rows, panels, segmented
  controls, and sliders in `crates/nova_ui/src/widget/mod.rs:119`.
- The HUD explicitly does not follow `UiSkin` in
  `crates/nova_ui/src/hud.rs:11`.
- NOVA OS has a separate palette in
  `crates/nova_os_ui/src/terminal/style.rs:1`.
- Settings persists `UiSkin` in
  `crates/nova_menu/src/settings_store.rs:61`.
- Mod bundles already declare resources in
  `crates/nova_mod_format/src/lib.rs:89`.

## Decisions

- Use RON, not CSS or a CSS-like parser.
- Add a normal mod content kind such as `Content::UiTheme`.
- Give every player-facing visual entity a stable semantic UI role.
- Resolve style from the active theme, semantic role, and control state.
- Base themes use `inherit: None`. Derived themes use
  `inherit: Some("base/phosphor")` or another known theme ID.
- A role override replaces the complete inherited role definition. Do not
  inherit individual interaction states.
- All states required by a role are explicit. Do not use optional state paint.
- The semantic role determines which states are required. Do not add a separate
  `Static` or `Interactive` classification.
- Enabling a mod does not select its theme. Selection is a separate setting.
- A missing, disabled, or invalid selected theme falls back to
  `base/phosphor` with a player-visible diagnostic.

## Initial style scope

Allow:

- foreground, background, and border colors;
- border width and corner radius;
- solid and gradient fills;
- simple glow and shadow effects; and
- paint for required interaction states.

Do not initially allow:

- position, width, height, padding, or gaps;
- flex or grid layout;
- z order, visibility, or pointer behavior;
- font size; or
- screen-specific geometry.

## Color format

Use one color wrapper with either a palette variable or literal source:

```ron
background: Color((
    value: Variable("phosphor"),
    alpha: 0.12,
))
foreground: Color((
    value: Literal("#d6ffe4"),
))
```

`alpha` is an `f32` with a serde default of `1.0`, not an `Option`. Palette
values and literals use six-digit RGB. An explicit wrapper alpha overwrites the
source alpha. Transparent paint uses alpha `0.0`.

## Required control behavior

Buttons initially resolve these required states:

1. disabled;
2. selected and pressed;
3. pressed;
4. selected;
5. hovered; and
6. normal.

The list is precedence order. The serialized fields are `normal`, `hovered`,
`pressed`, `selected`, `selected_pressed`, and `disabled`.

## Coverage contract

- Every player-facing entity with visual UI components has a semantic role or
  a documented exemption.
- Every active semantic role resolves through the selected theme's inheritance
  chain.
- Unknown theme IDs, role IDs, variables, fields, and resources are lint
  errors.
- Unknown parents and inheritance cycles are lint errors.
- Unmatched or obsolete role entries are reported instead of ignored.
- The debug egui inspector is exempt because it is developer UI.
- Keep the earliest loading/error screen on a built-in bootstrap theme unless
  an early safe theme-loading stage is designed separately.

## Change

Proposed surfaces include:

- serializable `UiThemeConfig`, theme role, paint, and color value types;
- `Content::UiTheme` and a merged theme registry;
- an `ActiveUiTheme` resource keyed by stable theme ID;
- semantic role components and one style resolver;
- RON definitions for base Phosphor and Hardware;
- settings discovery, selection, persistence, and fallback;
- migration of menu, editor, HUD, and NOVA OS paint to semantic roles; and
- content lint and visual coverage checks.

## Blast radius

This affects `nova_mod_format`, `nova_modding`, `nova_assets`, `nova_ui`,
`nova_menu`, `nova_editor`, `nova_hud`, `nova_os_ui`, settings persistence,
content generation and lint, tests, screenshots, and UI documentation.

## Verification

- Unit-test RON decode, alpha defaulting, color resolution, inheritance,
  complete-role replacement, cycle rejection, and state precedence.
- Test theme merge order and fallback after a selected mod is disabled.
- Add a visual-component census that fails on unclassified player UI.
- Assert every required role resolves for both base themes.
- Run focused screenshot scenarios for menu, settings, mods, pause, outcomes,
  editor, HUD states, and all NOVA OS apps under both base themes.
- Inspect rendered frames. Headless tests do not prove appearance.

## Delivery order

1. Define the typed RON format and semantic role inventory.
2. Register and merge theme content, inheritance, and lint.
3. Express Phosphor and Hardware as base RON themes.
4. Add active-theme selection, persistence, and fallback.
5. Migrate shared widgets, then menu and editor.
6. Migrate HUD and NOVA OS through role-specific adapters.
7. Add the complete coverage census and screenshot matrix.
