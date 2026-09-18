# Prevent UI layout panic during fullscreen startup

- STATUS: CLOSED
- PRIORITY: 100
- TAGS: v0.14.0, bug, ui, fullscreen, crash

## User facts

- Starting Nova while another application, such as a YouTube video, already owns fullscreen can crash during game startup.
- The observed panic is `BorderRadius::resolve`: `min > max, or either was NaN. min = 0.0, max = -12.0` inside `bevy_ui::layout::ui_layout_system`.
- Investigation and task capture are approved. No fix is approved yet.

## Agent findings

- Bevy 0.19 computes the radius upper bound as half the UI node's shortest dimension. `max = -12.0` indicates layout supplied a `-24 px` node dimension; the radius is the crash site, not the source.
- Nova defaults fresh installs to borderless fullscreen in `crates/nova_menu/src/settings.rs` and applies the setting from `crates/nova_menu/src/settings_store.rs` during `Update`.
- The menu already owns a dedicated UI camera in `crates/nova_menu/src/ambience.rs`. That prevents scenario-camera teardown from invalidating menu layout, but it does not guard a transient zero-area window or render target.
- Bevy 0.19.1 retains the same `radius.clamp(0., 0.5 * min_length)` behavior.
- CONFIRMED cause: a zero-area UI render target. `propagate_ui_target_cameras` writes `physical_size = UVec2::ZERO` when a camera has no viewport size (`bevy_ui-0.19.0/src/update.rs:142`).
- CONFIRMED crash tree: `Scenario Row: <id>` under `Scenarios List`. The list is a Column (`crates/nova_menu/src/menu_ui.rs:407`); a campaign row is indented by a 24 px LEFT margin over `width: Auto` (`crates/nova_menu/src/scenarios.rs:299,380-386`). taffy sizes a stretched child as `line_cross_size - cross margins` with NO floor at zero (`taffy-0.10.1/src/compute/flexbox.rs:1632`), so a zero-wide line gives `-24`. A leaf clamps its own size; the row has children, so it does not.
- Reproduced deterministically with no window manager: `BEVY_ASSET_ROOT=$PWD NOVA_NORENDER=1 ./target/debug/nova-protocol` panics verbatim with `min = 0.0, max = -12.0` at `BorderRadius::resolve` inside `ui_layout_system`.
- The Scenarios screen is built at boot, before any click, so every launch is exposed.

## Decisions

Approved 2026-09-18 and delivered in one commit on `master`.

- OWNER: `AppBuilder::assemble` in `crates/nova_core/src/lib.rs` configures `bevy::ui::UiSystems::Layout` with a private run condition, `every_ui_root_has_a_drawable_target`.
- SCOPE: EVERY assembly, not `Assembly::Windowed` alone. The reproduction is headless, so a windowed-only gate would have left a proven crash in place. No `Assembly` branch exists in the condition.
- MULTI-TARGET CONSEQUENCE, accepted: the gate is global, so one permanently zero-area root would hold ALL UI geometry. Nova's only second UI root is `NovaOsImageContentRoot`, whose image is pinned at `>= UVec2::ONE` (`crates/nova_os_ui/src/terminal/crt.rs:198`), so it cannot deadlock the gate.
- POSITIVE AREA, not a pixel floor: a target narrower than a node's own margins still goes negative. Native cannot reach it (`MIN_WINDOW_WIDTH` is a window-manager resize floor); a canvas in a column under 24 px can. Recorded in the condition's doc comment rather than patched with an invented number.
- Rejected: changing authored radii. Their values do not create the negative node dimension.
- Rejected: changing `CAMPAIGN_MEMBER_INDENT_PX`. That moves the crash to the next margined row and is a widget workaround.
- Headless ranges that click UI keep working because they spawn their own window (`examples/systems/system_headless_drag.rs:63-69`). Only a windowless app is held, and its geometry was all zeros already.

## Delivery

Intended result: Nova can start, switch window mode, minimize, and restore while another application owns fullscreen, without sending a degenerate render target through Bevy UI layout.

Owner and interface, as delivered:

- `crates/nova_core/src/lib.rs`
- Private run condition `every_ui_root_has_a_drawable_target` over root `ComputedUiRenderTargetInfo` values.
- Applied to `bevy::ui::UiSystems::Layout` in `AppBuilder::assemble`, for every assembly.

What dies:

- The assumption that a windowed UI render target always has positive area.
- The startup panic path through `BorderRadius::resolve`.

Blast radius:

- A global layout gate freezes all UI geometry while any root target is invalid.
- A permanently invalid secondary UI target could therefore block otherwise valid UI.
- Headless and offscreen assemblies must retain their current behavior.

Work sequence:

1. Reproduce while recording primary-window size, root UI target sizes, target cameras, and the first negative computed node.
2. Preserve the failing evidence and confirm whether the target is zero-area.
3. Stop for approval of global gating versus a narrower owner-specific delay.
4. Change the owning integration point, not individual widgets.
5. Verify startup and recovery across windowed, borderless, minimize, and restore flows.
6. Check offscreen, headless, and multi-camera callers for scheduling regressions.

## Verification

Named invariant: Bevy UI layout runs only when every root UI target has positive physical width and height.

Observed:

- `NOVA_NORENDER=1 ./target/debug/nova-protocol` panicked before, reaches the menu and runs to the timeout after. Same command both sides.
- Permanent proof, `crates/nova_core/src/lib.rs`: `ui_layout_is_held_while_a_root_target_has_no_area` and `ui_layout_resumes_once_the_target_has_area`. Both PANIC with the reported message when the gate is removed from the test rig; both pass with it.
- Borderless launch under Xvfb, `RUST_LOG=info,nova_core=debug`: the gate closes and reopens TWICE on every one of five launches - at window creation, then again across the borderless mode switch, 60-150 ms after `GameAssetsStates::Loading` finishes. Those are the frames that crash.
- Menu screenshot inspected: panel, buttons, radii, status bar and backdrop all laid out correctly.
- `system_menu_boot` (windowed, real pointer click by screen position) passes.
- `system_headless_drag` passes, both sliders: a windowless-gate regression would have stalled every `ui_node_present` beat.
- `system_pause_settings` passes, including containment at a 500x600 window.

Not verified here: the live handoff from another fullscreen application, a runtime window-mode flip, and minimize/restore. Those need a real window manager. Five borderless Xvfb launches on the pre-fix binary did not hit the race - the held frames landed just before the menu spawned.

## Done when

- The reported fullscreen-startup flow no longer panics.
- UI appears after the window receives a valid target without requiring a restart.
- Normal window-mode changes and minimize/restore recover correctly.
- No individual widget or radius value carries a workaround.
- Headless and offscreen behavior is unchanged.
- The chosen global or narrower ownership policy and its multi-target consequence are recorded as an approved decision.
