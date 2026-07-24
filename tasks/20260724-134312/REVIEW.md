# Review: Flight objective HUD rework

- TASK: 20260724-134312
- BRANCH: feat/objective-hint

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

Verification (not findings):

- Panel removal is safe. bcs `ObjectivesPlugin` is kept and still owns
  `GameObjectives`: verified in the actually-locked rev (Cargo.lock ->
  bevy-common-systems tag v0.19.5, checkout `30d1bef`) that `build` calls
  `init_resource::<GameObjectives>()` (objectives.rs:102) and `rebuild_lines`
  takes `Single<(Entity, Option<&Children>), With<ObjectivesPanelMarker>>`
  (objectives.rs:115). A `Single` system silently skips when the entity is
  absent, so dropping the nova panel spawn leaves `GameObjectives` populated for
  the drawer/reveal/hint. Claim confirmed.
- Hint visibility interacts correctly with `apply_hud_visibility`. `update_hint`
  (Update, in NovaHudSystems) sets Hidden at count 0 / Inherited otherwise;
  `apply_hud_visibility` (PostUpdate, mod.rs:284) skips self-driven widgets on a
  level-change restore (line 301 `!self_driven`) but still forces Hidden when the
  tier is off (line 300). PostUpdate runs after Update, so when the HUD is
  minimized the hint is forced Hidden every frame and does not leak visible - the
  correct precedence. `HudSelfDrivenVisibility` is set on the hint root.
- `DrawerTabAnchor` is now sourced from the hint (objective_hint.rs
  `update_tab_anchor` from `ObjectiveHintMarker`'s GlobalTransform); the drawer's
  handle-based `update_tab_anchor` + `DrawerTabHandleMarker` + spawn + the
  `remove_drawer` Or-clause are all removed. No dangling handle refs.
- `GamepadButton::RightThumb` is genuinely free: the only other Thumb consumer is
  `nova_editor` LeftThumb; grep of `GamepadButton::` across all crates shows
  RightThumb used only by the drawer toggle (+ its test). The toggle mirrors the
  keyboard path via `Option<Res<ButtonInput<GamepadButton>>>` with `.unwrap_or(false)`,
  same shape as `toggle_pause`.
- Reveal retune is consistent. The base-position test now derives
  `expected_base_left = 960.0 - REVEAL_WIDTH_PX / 2.0` instead of a hardcoded 780,
  so it tracks the new width (260). Scale/width/font reduced as specced.
- does-the-old-element-survive: grep of `spawn_objectives_panel`,
  `DrawerTabHandleMarker`, `OBJECTIVES_PANEL_WIDTH_PX`, `style_objective_lines`,
  `setup_hud_objectives`, `remove_hud_objectives`, `OBJECTIVES_FONT_PX`,
  `objectives_panel`, `ObjectivesPanelConfig`, `ObjectivesPluginSystems` across
  crates+examples returns only the two removal-note comments in mod.rs. Clean.
  The one live cross-consumer (objective_feedback's ghost column, which used
  `OBJECTIVES_PANEL_WIDTH_PX`) is correctly re-homed to a local
  `GHOST_COLUMN_WIDTH_PX = 280.0`.
- Removed tests correctly deleted, not weakened: the panel-styling test
  (`objective_lines_get_novas_font_and_wrap`) is gone with its element; the
  handle-anchor test moved to `objective_hint_provides_the_drawer_anchor` (still
  fails if the publishing system is deleted -> anchor stays None); the two hint
  tests and the pad test are new and meaningful; `drawer_renders_above_the_hud`
  updated to assert only panel + backdrop z.
- Docs: CHANGELOG, keybinds.md (pad glyph), hud.md and the scenario-authoring
  guide are all swept off "compact objectives panel"/"tab handle" - grep of
  web/src/wiki for that prose is empty.
- Tests: `cargo test -p nova_gameplay --lib -- objective_hint:: drawer::
  objective_reveal:: objective_feedback::` -> 13 passed, 0 failed (compiled clean,
  exit 0).
- Manual items (hint reads minimal + hints Tab/pad; reveal smaller + slides
  up-right; pad opens the drawer) are correctly batched for owner acceptance.

No BLOCKER/MAJOR/MINOR/NIT findings.
