# Prevent UI layout panic during fullscreen startup

- STATUS: OPEN
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
- Unverified cause: the window manager may expose a transient zero-area camera target while Nova takes fullscreen ownership from the existing fullscreen application. Taffy then produces a negative descendant dimension.

## Decisions

- None yet.
- Recommended direction, pending reproduction and approval: at the `nova_core` window/render integration edge, skip Bevy's global `UiSystems::Layout` while any root UI target has zero width or height. Resume layout when every root target has positive area.
- Rejected as an initial direction: changing authored radii. Their values do not create the negative node dimension.

## Delivery

Intended result: Nova can start, switch window mode, minimize, and restore while another application owns fullscreen, without sending a degenerate render target through Bevy UI layout.

Candidate owner and interface:

- `crates/nova_core/src/lib.rs`
- Add a private run condition over root `ComputedUiRenderTargetInfo` values.
- Apply it to `bevy::ui::UiSystems::Layout` only for `Assembly::Windowed`.

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

Closest proof:

- Reproduce startup while another application owns fullscreen.
- Observe that invalid target frames skip layout instead of entering `BorderRadius::resolve`.
- Observe that layout resumes after the target becomes valid and all computed node dimensions are nonnegative.
- Exercise windowed-to-borderless, borderless-to-windowed, minimize, and restore.
- Run the affected offscreen and headless checks to prove they were not gated.

Do not add a permanent test until the transient target state has been reproduced and the stable invariant is confirmed.

## Done when

- The reported fullscreen-startup flow no longer panics.
- UI appears after the window receives a valid target without requiring a restart.
- Normal window-mode changes and minimize/restore recover correctly.
- No individual widget or radius value carries a workaround.
- Headless and offscreen behavior is unchanged.
- The chosen global or narrower ownership policy and its multi-target consequence are recorded as an approved decision.
