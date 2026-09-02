//! Builds and tears down the NOVA OS node tree: chrome, header, main body,
//! terminal content and footer.
//!
//! Spawn order is the only place the hierarchy is written down; every other
//! module finds its nodes by the markers attached here.
//!
//! Touch this module when adding or moving a node in the monitor layout.

use bevy::{
    camera::{visibility::RenderLayers, ImageRenderTarget, RenderTarget},
    picking::{
        hover::Hovered,
        pointer::{Location, PointerLocation},
    },
    prelude::*,
    ui::UiTargetCamera,
    ui_render::prelude::MaterialNode,
    ui_widgets::{observe, Button},
};
use nova_gameplay::prelude::*;
use nova_hud::prelude::NovaHudAssets;
use nova_input::prelude::InputBindings;
use nova_os::prelude::*;
use nova_ui::font::UiFont;

use super::{casing::*, components::*, content::*, crt::*, shell::*, style::*};

/// Spawn the NOVA OS shell (backdrop plus inset NOVA OS monitor) once the UI
/// font is loaded, and keep it spawned.
///
/// It used to spawn and despawn with the player ship, like the rest of the HUD.
/// The CRT is no longer a flight surface: it is the terminal EMULATOR, and its
/// Command shell is reachable from the main menu and the editor, neither of
/// which has a ship. So the monitor is app-global, and the ship-scoped parts of
/// it - the session, the flight log, the topbar's ship name - are reset or
/// reconciled instead (see [`reset_nova_os_for_new_ship`]).
///
/// Idempotent: it early-returns once a root exists, so it is safe to run every
/// frame.
pub(crate) fn ensure_nova_os_spawned(
    mut commands: Commands,
    mut crt_materials: Option<ResMut<Assets<NovaOsCrtMaterial>>>,
    mut images: Option<ResMut<Assets<Image>>>,
    ui_font: Option<Res<UiFont>>,
    hud_assets: Option<Res<NovaHudAssets>>,
    settings: Option<Res<NovaOsMonitorSettings>>,
    game_state: Option<Res<State<GameStates>>>,
    q_existing: Query<(), With<NovaOsRootMarker>>,
    q_player: Query<Option<&Name>, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
) {
    if !q_existing.is_empty() {
        return;
    }
    // Nothing is reachable through a half-loaded world, and the fonts the
    // monitor draws with are part of what is still loading.
    if game_state.is_some_and(|state| *state.get() == GameStates::Loading) {
        return;
    }
    // The plugin always inits the resource; tolerate its absence so bare-app
    // rigs that only exercise other parts of the shell still spawn.
    let settings = settings.map(|s| *s).unwrap_or_default();
    let font = nova_os_font(ui_font.as_deref());
    // The brand-mark handle, preloaded into NovaHudAssets. Absent on bare-app
    // rigs without the asset pipeline - the plate then spawns without the logo,
    // as it did before when the AssetServer was missing.
    let crt_mark = hud_assets.map(|a| a.nova_crt_mark.clone());
    // The ship the monitor belongs to, if there is one yet. The monitor now
    // outlives any single ship, so the topbar's ship segment is reconciled
    // against the live one every frame; this is only the first reading.
    let ship_name = nova_os_ship_name(q_player.iter().next().flatten());

    // Render-to-texture pipeline: on render-capable builds route the terminal
    // content to an offscreen image via a dedicated UI camera, so the screen node
    // can sample it through the CRT shader (bloom + curvature). Headless rigs
    // (no image/material assets) fall back to the terminal directly on the screen.
    let rtt = match (crt_materials.as_deref_mut(), images.as_deref_mut()) {
        (Some(_), Some(images)) => {
            let image = images.add(nova_os_new_target_image(UVec2::new(2, 2)));
            let camera = commands
                .spawn((
                    Name::new("NovaOsImageCamera"),
                    NovaOsImageCameraMarker,
                    Camera2d,
                    Camera {
                        order: NOVA_OS_RTT_CAMERA_ORDER,
                        clear_color: ClearColorConfig::Custom(NOVA_OS_SCREEN),
                        is_active: false,
                        ..default()
                    },
                    RenderTarget::Image(ImageRenderTarget {
                        handle: image.clone(),
                        scale_factor: 1.0,
                    }),
                    // Draw ONLY the terminal UI, never stray world 2D sprites.
                    RenderLayers::layer(NOVA_OS_RTT_LAYER),
                ))
                .id();
            let content_root = commands
                .spawn((
                    Name::new("NovaOsImageContentRoot"),
                    NovaOsImageContentRootMarker,
                    Node {
                        position_type: PositionType::Absolute,
                        top: Val::Px(0.0),
                        left: Val::Px(0.0),
                        width: Val::Px(2.0),
                        height: Val::Px(2.0),
                        // Safe-area inset so no content renders in the band the
                        // CRT overscan pushes under the bezel (see the constants).
                        padding: UiRect::axes(
                            Val::Percent(NOVA_OS_CONTENT_SAFE_X_PCT),
                            Val::Percent(NOVA_OS_CONTENT_SAFE_Y_PCT),
                        ),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(12.0),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    BackgroundColor(NOVA_OS_SCREEN),
                    UiTargetCamera(camera),
                    RenderLayers::layer(NOVA_OS_RTT_LAYER),
                    Visibility::Hidden,
                ))
                .id();
            let pointer = commands
                .spawn((
                    Name::new("NovaOsForwardedPointer"),
                    NovaOsForwardedPointerMarker,
                    nova_os_pointer_id(),
                    PointerLocation::new(Location {
                        target: nova_os_image_target(&image),
                        position: Vec2::splat(-1000.0),
                    }),
                ))
                .id();
            commands.insert_resource(NovaOsRtt {
                image: image.clone(),
                camera,
                content_root,
                pointer,
            });
            Some((content_root, image))
        }
        _ => {
            commands.remove_resource::<NovaOsRtt>();
            None
        }
    };

    // Dim backdrop behind the panel (hidden until the NOVA OS opens). NO
    // `HudTier`: the NOVA OS is a modal overlay on its own axis, so the
    // grave/tilde HUD-visibility cycle must not touch it - `apply_hud_visibility`
    // force-hides a non-shown Chrome tier every frame (even self-driven ones),
    // which would blank the NOVA OS if the player opened it with the HUD
    // minimized. The panel's visibility is driven entirely by `drive_nova_os_slide`.
    commands.spawn((
        Name::new("NovaOsBackdrop"),
        NovaOsBackdropMarker,
        GlobalZIndex(DRAWER_BACKDROP_Z),
        Visibility::Hidden,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(0.0),
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            bottom: Val::Px(0.0),
            ..default()
        },
        BackgroundColor(NOVA_OS_BACKDROP.with_alpha(0.0)),
    ));

    // One inset physical monitor. It is hidden until opened by the same
    // real-time openness driver the old NOVA OS panels used.
    commands
        .spawn((
            Name::new("NovaOsMonitor"),
            NovaOsRootMarker,
            NovaOsMonitorMarker,
            NovaOsOpenness(0.0),
            GlobalZIndex(DRAWER_PANEL_Z),
            Visibility::Hidden,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(NOVA_OS_MONITOR_INSET_Y_PX),
                bottom: Val::Px(NOVA_OS_MONITOR_INSET_Y_PX),
                left: Val::Px(NOVA_OS_MONITOR_INSET_X_PX),
                right: Val::Px(NOVA_OS_MONITOR_INSET_X_PX),
                padding: UiRect::all(Val::Px(10.0)),
                border: UiRect::all(Val::Px(1.0)),
                flex_direction: FlexDirection::Column,
                // Injection-moulded shell: larger top radius, tighter bottom.
                border_radius: BorderRadius {
                    top_left: Val::Px(NOVA_OS_CASE_RADIUS_TOP_PX),
                    top_right: Val::Px(NOVA_OS_CASE_RADIUS_TOP_PX),
                    bottom_left: Val::Px(NOVA_OS_CASE_RADIUS_BOTTOM_PX),
                    bottom_right: Val::Px(NOVA_OS_CASE_RADIUS_BOTTOM_PX),
                },
                ..default()
            },
            BorderColor::all(NOVA_OS_CASE_EDGE),
            // Base fill under the gradient (headless/no-gradient fallback).
            BackgroundColor(NOVA_OS_CASE),
            // Injection-moulded shell: a 168deg body gradient (lit top -> deep
            // undercut) plus a 1px top highlight catching the moulding lip.
            nova_os_case_gradient(),
        ))
        .with_children(|monitor| {
            spawn_nova_os_moulding_seam(monitor);
            spawn_nova_os_casing_screws(monitor);
            spawn_nova_os_casing_vents(monitor);
            monitor
                .spawn((
                    Name::new("NovaOsBezel"),
                    NovaOsBezelMarker,
                    Node {
                        flex_grow: 1.0,
                        min_height: Val::Px(0.0),
                        padding: UiRect::all(Val::Px(NOVA_OS_BEZEL_PAD_PX)),
                        border: UiRect::all(Val::Px(1.0)),
                        flex_direction: FlexDirection::Column,
                        border_radius: BorderRadius::all(Val::Px(NOVA_OS_BEZEL_RADIUS_PX)),
                        ..default()
                    },
                    // Recessed bezel lip: dark inner-top shadow, light lower edge.
                    BorderColor {
                        top: Color::srgba(0.0, 0.0, 0.0, 0.6),
                        bottom: Color::srgba(1.0, 1.0, 1.0, 0.06),
                        left: NOVA_OS_CASE_EDGE.with_alpha(0.5),
                        right: NOVA_OS_CASE_EDGE.with_alpha(0.5),
                    },
                    BackgroundColor(NOVA_OS_CASE_RAISED),
                    nova_os_bezel_gradient(),
                ))
                .with_children(|bezel| {
                    bezel
                        .spawn((
                            Name::new("NovaOsScreen"),
                            NovaOsScreenMarker,
                            Node {
                                position_type: PositionType::Relative,
                                flex_grow: 1.0,
                                min_height: Val::Px(0.0),
                                padding: UiRect::all(Val::Px(NOVA_OS_SCREEN_PAD_PX)),
                                border: UiRect::all(Val::Px(1.0)),
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(12.0),
                                overflow: Overflow::clip(),
                                border_radius: BorderRadius::all(Val::Px(NOVA_OS_SCREEN_RADIUS_PX)),
                                ..default()
                            },
                            // A dark recess line, not a bright straight phosphor
                            // frame: the glass sits recessed in the bezel and the
                            // crisp glowing edge now comes from the shader's
                            // barrel-bowed rim (see DECISION.md / feedback item 2).
                            BorderColor::all(NOVA_OS_CASE_EDGE.with_alpha(0.85)),
                            BackgroundColor(NOVA_OS_SCREEN),
                        ))
                        .with_children(|screen| {
                            match (&rtt, crt_materials.as_deref_mut()) {
                                (Some((_, image)), Some(crt_materials)) => {
                                    // Screen surface = the offscreen image sampled
                                    // through the CRT shader. Terminal content is
                                    // populated into the content root below.
                                    let handle = crt_materials.add(NovaOsCrtMaterial {
                                        source: image.clone(),
                                        ..default()
                                    });
                                    screen.spawn((
                                        Name::new("NovaOsCrtSurface"),
                                        NovaOsSamplingSurfaceMarker,
                                        Node {
                                            position_type: PositionType::Absolute,
                                            top: Val::Px(0.0),
                                            bottom: Val::Px(0.0),
                                            left: Val::Px(0.0),
                                            right: Val::Px(0.0),
                                            ..default()
                                        },
                                        MaterialNode(handle),
                                        ZIndex(NOVA_OS_CONTENT_Z),
                                        Pickable::IGNORE,
                                    ));
                                }
                                _ => {
                                    // Headless fallback: header + main + footer
                                    // directly on-screen (no offscreen CRT pass).
                                    spawn_nova_os_chrome(screen, font.clone(), &ship_name);
                                }
                            }
                            spawn_nova_os_phosphor_rim(screen);
                            spawn_nova_os_glass_sheen(screen);
                        });
                });
            spawn_nova_os_chin(monitor, font.clone(), crt_mark.clone(), &settings);
        });

    // Render-capable: populate the offscreen content root with the header + main
    // + footer (the subtree renders through the image camera, not the window).
    if let Some((content_root, _)) = &rtt {
        commands.entity(*content_root).with_children(|root| {
            spawn_nova_os_chrome(root, font.clone(), &ship_name);
        });
    }
}
/// Spawn the three persistent NOVA OS regions into `parent` (the offscreen
/// content root, or the screen node in the headless fallback): the fixed-height
/// header, the flexing `<main>` (seeded with the terminal surface), and the
/// fixed-height footer. The header and footer never move when `<main>` swaps
/// between the terminal and an app.
pub(crate) fn spawn_nova_os_chrome(
    parent: &mut ChildSpawnerCommands,
    font: Handle<Font>,
    ship_name: &str,
) {
    spawn_nova_os_header(parent, font.clone(), ship_name);
    spawn_nova_os_main(parent, font.clone());
    // The footer is a SIBLING of `<main>` so it survives an app hiding the
    // terminal surface, carrying that app's keybinds instead.
    spawn_nova_os_footer(parent, font);
}

/// The persistent header bar (`<header>`): a lit lamp + the brand/breadcrumb on
/// the left, and the app close control + ship/FPS status on the right. Fixed
/// height so it never reflows when `<main>` swaps surfaces.
pub(crate) fn spawn_nova_os_header(
    parent: &mut ChildSpawnerCommands,
    font: Handle<Font>,
    ship_name: &str,
) {
    parent
        .spawn((
            NovaOsTopbarMarker,
            Node {
                height: Val::Px(NOVA_OS_HEADER_HEIGHT_PX),
                flex_shrink: 0.0,
                padding: UiRect::bottom(Val::Px(10.0)),
                border: UiRect::bottom(Val::Px(1.0)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: Val::Px(12.0),
                ..default()
            },
            BorderColor::all(NOVA_OS_PHOSPHOR.with_alpha(0.36)),
            ZIndex(NOVA_OS_CONTENT_Z),
        ))
        .with_children(|topbar| {
            topbar
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(10.0),
                    min_width: Val::Px(0.0),
                    ..default()
                })
                .with_children(|brand| {
                    brand.spawn((
                        NovaOsLampMarker,
                        Node {
                            width: Val::Px(10.0),
                            height: Val::Px(10.0),
                            border: UiRect::all(Val::Px(1.0)),
                            flex_shrink: 0.0,
                            ..default()
                        },
                        BorderColor::all(NOVA_OS_PHOSPHOR),
                        BackgroundColor(NOVA_OS_PHOSPHOR),
                    ));
                    brand.spawn((
                        NovaOsBrandMarker,
                        Text::new(nova_os_header_breadcrumb(
                            ShellKind::NovaOs,
                            TerminalMode::Prompt,
                        )),
                        nova_os_text_font(DRAWER_SECTION_TITLE_FONT_PX, font.clone()),
                        TextColor(NOVA_OS_PHOSPHOR),
                    ));
                });
            topbar
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(12.0),
                    flex_shrink: 0.0,
                    ..default()
                })
                .with_children(|status| {
                    // Clickable close, left of the status, shown only while an app
                    // owns the screen (toggled by `reconcile_nova_os_header`). The
                    // old per-app chrome bar is gone; this is the app's only
                    // on-screen close affordance besides the ESC keybind.
                    status.spawn((
                        // Named because it is the widget a driven run aims at
                        // through the glass: the resolve is by `Name`, so only a
                        // RENAME breaks the run, not a move within the header.
                        Name::new("NovaOsAppClose"),
                        NovaOsAppCloseMarker,
                        Button,
                        Visibility::Hidden,
                        Node {
                            padding: UiRect::axes(Val::Px(10.0), Val::Px(3.0)),
                            border: UiRect::all(Val::Px(1.0)),
                            flex_shrink: 0.0,
                            ..default()
                        },
                        BorderColor::all(NOVA_OS_AMBER.with_alpha(0.7)),
                        children![(
                            Text::new("[ ESC ]"),
                            nova_os_text_font(11.0, font.clone()),
                            TextColor(NOVA_OS_AMBER),
                        )],
                        observe(on_nova_os_app_close),
                    ));
                    status.spawn((
                        NovaOsStatusMarker,
                        Text::new(nova_os_status_text(ship_name, None)),
                        nova_os_text_font(DRAWER_SECTION_TITLE_FONT_PX, font.clone()),
                        TextColor(NOVA_OS_PHOSPHOR_DIM),
                    ));
                });
        });
}

/// The persistent `<main>` region: flex-grows between the header and footer and
/// carries the terminal surface. A launched app is spawned as an absolute-fill
/// child of this node (see [`spawn_nova_os_app`]), so `position_type` is
/// relative here to make it the app root's containing block.
pub(crate) fn spawn_nova_os_main(parent: &mut ChildSpawnerCommands, font: Handle<Font>) {
    parent
        .spawn((
            Name::new("NovaOsMain"),
            NovaOsMainMarker,
            Node {
                position_type: PositionType::Relative,
                flex_grow: 1.0,
                min_height: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            ZIndex(NOVA_OS_CONTENT_Z),
        ))
        .with_children(|main| {
            spawn_nova_os_terminal_content(main, font);
        });
}

/// The terminal surface (scrollback + prompt) that fills `<main>` in Prompt
/// mode. Tagged [`NovaOsTerminalContentMarker`] so `sync_nova_os_app_ui` can hide
/// it (and only it) while an app owns the screen.
pub(crate) fn spawn_nova_os_terminal_content(
    screen: &mut ChildSpawnerCommands,
    font: Handle<Font>,
) {
    screen
        .spawn((
            Name::new("NovaOsTerminalContent"),
            NovaOsTerminalContentMarker,
            Node {
                flex_grow: 1.0,
                min_height: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            ZIndex(NOVA_OS_CONTENT_Z),
            Pickable::IGNORE,
        ))
        .with_children(|terminal| {
            terminal
                .spawn((
                    NovaOsTerminalSurfaceMarker,
                    Node {
                        flex_direction: FlexDirection::Column,
                        flex_grow: 1.0,
                        min_height: Val::Px(0.0),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BorderColor::all(NOVA_OS_PHOSPHOR.with_alpha(0.36)),
                    BackgroundColor(Color::srgba(0.0, 5.0 / 255.0, 2.0 / 255.0, 0.72)),
                ))
                .with_children(|terminal_panel| {
                    terminal_panel
                        .spawn((
                            NovaOsTerminalScrollbackMarker,
                            NovaOsScrollViewportMarker,
                            ScrollPosition::default(),
                            Hovered::default(),
                            Node {
                                flex_direction: FlexDirection::Column,
                                flex_grow: 1.0,
                                min_height: Val::Px(0.0),
                                padding: UiRect::axes(
                                    Val::Px(NOVA_OS_TERMINAL_PAD_X_PX),
                                    Val::Px(NOVA_OS_TERMINAL_PAD_Y_PX),
                                ),
                                overflow: Overflow::scroll_y(),
                                row_gap: Val::Px(5.0),
                                ..default()
                            },
                        ))
                        .with_children(|scrollback| {
                            for row in nova_os_welcome_rows() {
                                spawn_terminal_row(scrollback, &row, font.clone());
                            }
                        });
                    terminal_panel
                        .spawn((
                            NovaOsPromptRowMarker,
                            Node {
                                min_height: Val::Px(NOVA_OS_PROMPT_ROW_HEIGHT_PX),
                                padding: UiRect::axes(
                                    Val::Px(NOVA_OS_TERMINAL_PAD_X_PX),
                                    Val::Px(7.0),
                                ),
                                border: UiRect::top(Val::Px(1.0)),
                                flex_direction: FlexDirection::Column,
                                justify_content: JustifyContent::Center,
                                row_gap: Val::Px(2.0),
                                ..default()
                            },
                            BorderColor::all(NOVA_OS_PHOSPHOR.with_alpha(0.45)),
                            // Near-opaque black-green so the input reads as a
                            // dark box sitting ABOVE the screen (HTML `.prompt-row`).
                            BackgroundColor(Color::srgba(0.0, 0.016, 0.008, 0.97)),
                            ZIndex(NOVA_OS_OVERLAY_Z + 1),
                        ))
                        .with_children(|prompt_row| {
                            prompt_row
                                .spawn((
                                    NovaOsPromptInputLineMarker,
                                    Node {
                                        width: Val::Percent(100.0),
                                        min_height: Val::Px(24.0),
                                        flex_direction: FlexDirection::Row,
                                        align_items: AlignItems::Center,
                                        column_gap: Val::Px(8.0),
                                        min_width: Val::Px(0.0),
                                        ..default()
                                    },
                                ))
                                .with_children(|input_line| {
                                    input_line.spawn((
                                        NovaOsPromptPrefixMarker,
                                        Text::new("nova>"),
                                        nova_os_text_font(DRAWER_LINE_FONT_PX, font.clone()),
                                        TextColor(NOVA_OS_AMBER),
                                        Node {
                                            flex_shrink: 0.0,
                                            ..default()
                                        },
                                    ));
                                    input_line
                                        .spawn((
                                            NovaOsPromptInputWrapMarker,
                                            Node {
                                                flex_grow: 1.0,
                                                min_width: Val::Px(0.0),
                                                // Hold the wrap at a full text line
                                                // box even when every piece is empty:
                                                // the block caret is absolute with
                                                // top:0/bottom:0, so it stretches to
                                                // THIS node. With 0 chars typed all
                                                // three text children are "" and the
                                                // wrap would collapse to 0 height,
                                                // hiding the caret (owner playtest:
                                                // caret invisible before typing). 1.2
                                                // is Bevy's default line-height factor
                                                // (not an arbitrary pad), so this floor
                                                // equals the line box the caret stretches
                                                // to once text is present - empty and
                                                // typed carets stay the same height.
                                                min_height: Val::Px(DRAWER_LINE_FONT_PX * 1.2),
                                                flex_direction: FlexDirection::Row,
                                                align_items: AlignItems::Center,
                                                overflow: Overflow::clip_x(),
                                                ..default()
                                            },
                                        ))
                                        .with_children(|input_wrap| {
                                            // Fish-style inline input: typed text
                                            // left of the caret, a block caret,
                                            // typed text right of it, then the dim
                                            // completion ghost - all NoWrap so the
                                            // completion continues on the SAME line.
                                            input_wrap.spawn((
                                                NovaOsTerminalPromptMarker,
                                                Text::new(""),
                                                nova_os_text_font(
                                                    DRAWER_LINE_FONT_PX,
                                                    font.clone(),
                                                ),
                                                TextColor(NOVA_OS_PHOSPHOR),
                                                nova_os_prompt_text_layout(),
                                                Node {
                                                    flex_shrink: 0.0,
                                                    ..default()
                                                },
                                                ZIndex(1),
                                            ));
                                            input_wrap.spawn((
                                                NovaOsTerminalCaretMarker,
                                                Node {
                                                    // ABSOLUTE so the block does
                                                    // not advance the row: it sits
                                                    // OVER the character at the
                                                    // cursor - the first completion
                                                    // ghost letter when the cursor
                                                    // is at the end - instead of
                                                    // pushing the ghost one cell to
                                                    // the right (owner playtest).
                                                    // `left` is set from the MEASURED
                                                    // typed-text width by
                                                    // `position_nova_os_block_caret`;
                                                    // top + bottom stretch it to the
                                                    // line height (PoC `.caret`, a
                                                    // block one glyph wide).
                                                    position_type: PositionType::Absolute,
                                                    left: Val::Px(0.0),
                                                    top: Val::Px(0.0),
                                                    bottom: Val::Px(0.0),
                                                    width: Val::Px(
                                                        DRAWER_LINE_FONT_PX
                                                            * NOVA_OS_CARET_WIDTH_FRACTION,
                                                    ),
                                                    ..default()
                                                },
                                                // Slightly translucent so the letter
                                                // under the block still reads (PoC
                                                // `.caret` opacity 0.85).
                                                BackgroundColor(NOVA_OS_AMBER.with_alpha(0.85)),
                                                ZIndex(2),
                                            ));
                                            input_wrap.spawn((
                                                NovaOsTerminalPromptAfterMarker,
                                                Text::new(""),
                                                nova_os_text_font(
                                                    DRAWER_LINE_FONT_PX,
                                                    font.clone(),
                                                ),
                                                TextColor(NOVA_OS_PHOSPHOR),
                                                nova_os_prompt_text_layout(),
                                                Node {
                                                    flex_shrink: 0.0,
                                                    ..default()
                                                },
                                                ZIndex(1),
                                            ));
                                            input_wrap.spawn((
                                                NovaOsTerminalGhostMarker,
                                                Text::new(""),
                                                nova_os_text_font(
                                                    DRAWER_LINE_FONT_PX,
                                                    font.clone(),
                                                ),
                                                TextColor(NOVA_OS_TEXT.with_alpha(0.34)),
                                                nova_os_prompt_text_layout(),
                                                Node {
                                                    flex_shrink: 0.0,
                                                    ..default()
                                                },
                                            ));
                                        });
                                });
                            prompt_row.spawn((
                                NovaOsTerminalHintMarker,
                                Text::new(""),
                                nova_os_text_font(12.0, font.clone()),
                                TextColor(NOVA_OS_PHOSPHOR_MUTED),
                                Node {
                                    width: Val::Percent(100.0),
                                    min_height: Val::Px(16.0),
                                    min_width: Val::Px(0.0),
                                    ..default()
                                },
                            ));
                        });
                });
        });
}

/// The footer hint row. Spawned as a SIBLING of the terminal content (not inside
/// it) so it stays visible while an app hides the terminal content - the footer
/// carries each app's own keybinds (`rebuild_nova_os_footer_hints` refills it per
/// active surface). A higher `ZIndex` keeps it above the app overlay.
///
/// Seeded from the SHIPPED defaults, not the live table: the spawn runs in an
/// observer with no world access to thread a resource through, and
/// `rebuild_nova_os_footer_hints` overwrites this on `Added`, so a moved key is
/// wrong for at most the spawn frame.
pub(crate) fn spawn_nova_os_footer(parent: &mut ChildSpawnerCommands, font: Handle<Font>) {
    parent
        .spawn((
            NovaOsFooterHintsMarker,
            Node {
                // Fixed height so the footer stays constant next to the flexing
                // main region (owner: keep header + footer sizes constant).
                height: Val::Px(NOVA_OS_FOOTER_HEIGHT_PX),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                // Clip so a wrapped overflow row never grows the bar past its
                // fixed height; the tuned hint sets fit one row on the near
                // full-screen monitor.
                flex_wrap: FlexWrap::Wrap,
                overflow: Overflow::clip(),
                column_gap: Val::Px(12.0),
                row_gap: Val::Px(2.0),
                // A hairline top border + a little breathing room reads as a
                // distinct footer bar, not loose text over the app.
                border: UiRect::top(Val::Px(1.0)),
                padding: UiRect::top(Val::Px(6.0)),
                ..default()
            },
            BorderColor::all(NOVA_OS_PHOSPHOR.with_alpha(0.28)),
            ZIndex(NOVA_OS_CONTENT_Z + 10),
        ))
        .with_children(|footer| {
            for hint in terminal_hints(&InputBindings::default()) {
                footer.spawn((
                    Text::new(hint),
                    nova_os_text_font(11.0, font.clone()),
                    TextColor(NOVA_OS_PHOSPHOR_MUTED),
                ));
            }
        });
}

/// Forget the ship-scoped half of the NOVA OS when the player ship goes away:
/// the flight log and the NOVA OS session's transcript and history.
///
/// The monitor itself stays. It is app-global now, and the Command shell has no
/// ship to belong to - which is why `reset_session` leaves that shell alone.
pub(crate) fn reset_nova_os_for_new_ship(
    _remove: On<Remove, PlayerSpaceshipMarker>,
    mut log: ResMut<NovaOsFlightLog>,
    mut terminal: ResMut<NovaOsTerminal>,
) {
    log.clear();
    terminal.reset_session();
}
