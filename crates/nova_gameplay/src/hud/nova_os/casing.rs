use bevy::{
    prelude::*,
    ui_widgets::{observe, Button},
};

use super::{components::*, shell::*, style::*};

/// The PoC `.case` body: a 168deg gradient from a lit top through the mid body to
/// an almost-black undercut, with a 1px top highlight catching the moulding lip.
pub(crate) fn nova_os_case_gradient() -> BackgroundGradient {
    BackgroundGradient(vec![
        LinearGradient::degrees(
            168.0,
            vec![
                ColorStop::percent(NOVA_OS_CASE_LIT, 0.0),
                ColorStop::percent(NOVA_OS_CASE_MID, 26.0),
                ColorStop::percent(NOVA_OS_CASE_DEEP, 88.0),
                ColorStop::percent(Color::srgb_u8(4, 6, 8), 100.0),
            ],
        )
        .into(),
        // 1px lit moulding lip along the very top edge.
        LinearGradient::degrees(
            180.0,
            vec![
                ColorStop::px(NOVA_OS_CASE_HIGHLIGHT, 0.0),
                ColorStop::px(NOVA_OS_CASE_HIGHLIGHT, 1.0),
                ColorStop::px(Color::NONE, 1.0),
            ],
        )
        .into(),
    ])
}

/// The PoC `.bezel`: a dark vertical gradient giving the recessed lip its depth.
pub(crate) fn nova_os_bezel_gradient() -> BackgroundGradient {
    BackgroundGradient(vec![LinearGradient::degrees(
        180.0,
        vec![
            ColorStop::percent(Color::srgb_u8(18, 24, 29), 0.0),
            ColorStop::percent(Color::srgb_u8(7, 10, 13), 100.0),
        ],
    )
    .into()])
}

/// Four moulded corner screws (PoC `.screw`): a spherical head via a diagonal
/// light -> dark gradient over a full-radius disc, with a rotated slot line. The
/// slot is a FILLED bar, not a coloured border on a zero-content node, so it
/// dodges the border-collapse trap in the ledger
/// (`bevy-css-border-triangle-needs-contentbox`).
pub(crate) fn spawn_nova_os_casing_screws(parent: &mut ChildSpawnerCommands) {
    const DIAM_PX: f32 = 12.0;
    const INSET_PX: f32 = 15.0;
    for (name, left, top) in [
        ("NovaOsScrewTL", true, true),
        ("NovaOsScrewTR", false, true),
        ("NovaOsScrewBL", true, false),
        ("NovaOsScrewBR", false, false),
    ] {
        let mut node = Node {
            position_type: PositionType::Absolute,
            width: Val::Px(DIAM_PX),
            height: Val::Px(DIAM_PX),
            border: UiRect::all(Val::Px(1.0)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border_radius: BorderRadius::MAX,
            ..default()
        };
        if left {
            node.left = Val::Px(INSET_PX);
        } else {
            node.right = Val::Px(INSET_PX);
        }
        if top {
            node.top = Val::Px(INSET_PX);
        } else {
            node.bottom = Val::Px(INSET_PX);
        }
        parent
            .spawn((
                Name::new(name),
                NovaOsScrewMarker,
                node,
                BorderColor::all(Color::srgba(0.0, 0.0, 0.0, 0.7)),
                BackgroundColor(NOVA_OS_SCREW_DARK),
                BackgroundGradient(vec![LinearGradient::degrees(
                    135.0,
                    vec![
                        ColorStop::percent(NOVA_OS_SCREW_LIT, 0.0),
                        ColorStop::percent(Color::srgb_u8(27, 33, 38), 62.0),
                        ColorStop::percent(NOVA_OS_SCREW_DARK, 100.0),
                    ],
                )
                .into()]),
                Pickable::IGNORE,
            ))
            .with_children(|screw| {
                screw.spawn((
                    Node {
                        width: Val::Px(8.0),
                        height: Val::Px(1.5),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.75)),
                    UiTransform::from_rotation(Rot2::degrees(38.0)),
                    Pickable::IGNORE,
                ));
            });
    }
}

/// The moulding seam running just inside the shell edge (PoC `.case::after`): a
/// 1px rounded outline, light along the top/left, dark along the bottom/right,
/// so the plastic reads as a moulded part with a parting line.
pub(crate) fn spawn_nova_os_moulding_seam(parent: &mut ChildSpawnerCommands) {
    parent.spawn((
        Name::new("NovaOsMouldingSeam"),
        NovaOsSeamMarker,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(5.0),
            bottom: Val::Px(5.0),
            left: Val::Px(5.0),
            right: Val::Px(5.0),
            border: UiRect::all(Val::Px(1.0)),
            border_radius: BorderRadius {
                top_left: Val::Px(NOVA_OS_CASE_RADIUS_TOP_PX - 4.0),
                top_right: Val::Px(NOVA_OS_CASE_RADIUS_TOP_PX - 4.0),
                bottom_left: Val::Px(NOVA_OS_CASE_RADIUS_BOTTOM_PX - 4.0),
                bottom_right: Val::Px(NOVA_OS_CASE_RADIUS_BOTTOM_PX - 4.0),
            },
            ..default()
        },
        BorderColor {
            top: Color::srgba(1.0, 1.0, 1.0, 0.05),
            left: Color::srgba(1.0, 1.0, 1.0, 0.05),
            bottom: Color::srgba(0.0, 0.0, 0.0, 0.5),
            right: Color::srgba(0.0, 0.0, 0.0, 0.5),
        },
        Pickable::IGNORE,
    ));
}

/// The top-centre vent grille (PoC `.vents`): a centred row of thin dark slats,
/// the case gradient showing through the gaps.
pub(crate) fn spawn_nova_os_casing_vents(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Name::new("NovaOsVents"),
            NovaOsVentMarker,
            Node {
                align_self: AlignSelf::Center,
                height: Val::Px(10.0),
                margin: UiRect::bottom(Val::Px(8.0)),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(4.0),
                overflow: Overflow::clip(),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|vents| {
            for _ in 0..28 {
                vents.spawn((
                    Node {
                        width: Val::Px(4.0),
                        height: Val::Percent(100.0),
                        border_radius: BorderRadius::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.75)),
                    Pickable::IGNORE,
                ));
            }
        });
}

/// A faint phosphor halo tracing the screen edge (PoC `.rim`): a wider low-alpha
/// glow under a thin line, two nested rounded-border nodes at the screen
/// rounding. Drawn above the CRT overlay, below the glass. Kept deliberately
/// faint - the crisp, tube-bowed screen edge is now the shader's barrel-warped
/// rim (see DECISION.md); this is only the soft outer bloom, and the headless
/// fallback's sole edge cue.
pub(crate) fn spawn_nova_os_phosphor_rim(screen: &mut ChildSpawnerCommands) {
    for (name, border_px, color) in [
        (
            "NovaOsPhosphorRimGlow",
            3.0,
            NOVA_OS_PHOSPHOR.with_alpha(0.09),
        ),
        (
            "NovaOsPhosphorRimLine",
            1.0,
            NOVA_OS_PHOSPHOR.with_alpha(0.16),
        ),
    ] {
        screen.spawn((
            Name::new(name),
            NovaOsPhosphorRimMarker,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                border: UiRect::all(Val::Px(border_px)),
                border_radius: BorderRadius::all(Val::Px(NOVA_OS_SCREEN_RADIUS_PX)),
                ..default()
            },
            BorderColor::all(color),
            ZIndex(NOVA_OS_RIM_Z),
            Pickable::IGNORE,
        ));
    }
}

/// The glass specular sheen over the screen (PoC `.glass`): a diagonal white
/// gradient fading to clear, plus one soft angled highlight rectangle. The
/// frontmost surface layer; ignores picking so it never eats terminal input.
pub(crate) fn spawn_nova_os_glass_sheen(screen: &mut ChildSpawnerCommands) {
    screen
        .spawn((
            Name::new("NovaOsGlass"),
            NovaOsGlassMarker,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                overflow: Overflow::clip(),
                border_radius: BorderRadius::all(Val::Px(NOVA_OS_SCREEN_RADIUS_PX)),
                ..default()
            },
            BackgroundGradient(vec![LinearGradient::degrees(
                118.0,
                vec![
                    ColorStop::percent(Color::srgba(1.0, 1.0, 1.0, 0.055), 0.0),
                    ColorStop::percent(Color::srgba(1.0, 1.0, 1.0, 0.016), 17.0),
                    ColorStop::percent(Color::NONE, 33.0),
                ],
            )
            .into()]),
            ZIndex(NOVA_OS_GLASS_Z),
            Pickable::IGNORE,
        ))
        .with_children(|glass| {
            // A soft upper-left reflection. A RADIAL gradient (not a solid fill)
            // fades to transparent at the edges, so it reads as a soft glass
            // catch instead of the hard-edged card a blur-less solid node gives.
            glass.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(6.0),
                    top: Val::Percent(7.0),
                    width: Val::Percent(26.0),
                    height: Val::Percent(40.0),
                    ..default()
                },
                BackgroundGradient(vec![Gradient::from(RadialGradient::new(
                    UiPosition::CENTER,
                    RadialGradientShape::ClosestSide,
                    vec![
                        ColorStop::percent(Color::srgba(0.82, 0.92, 1.0, 0.06), 0.0),
                        ColorStop::percent(Color::srgba(0.82, 0.92, 1.0, 0.02), 55.0),
                        ColorStop::percent(Color::NONE, 100.0),
                    ],
                ))]),
                UiTransform::from_rotation(Rot2::degrees(-14.0)),
                Pickable::IGNORE,
            ));
        });
}

/// The bottom casing chin (PoC `.chin`): the recessed brand plate on the left
/// and the BRIGHT/SCAN/SND/PWR controls row on the right.
pub(crate) fn spawn_nova_os_chin(
    parent: &mut ChildSpawnerCommands,
    font: Handle<Font>,
    crt_mark: Option<Handle<Image>>,
    settings: &NovaOsMonitorSettings,
) {
    parent
        .spawn((
            Name::new("NovaOsChin"),
            NovaOsChinMarker,
            Node {
                width: Val::Percent(100.0),
                min_height: Val::Px(NOVA_OS_CHIN_HEIGHT_PX),
                // Wide left/right padding so the plate + controls clear the
                // bottom corner screws (screws inset ~15px, ~12px wide).
                padding: UiRect {
                    left: Val::Px(40.0),
                    right: Val::Px(40.0),
                    top: Val::Px(11.0),
                    bottom: Val::Px(4.0),
                },
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: Val::Px(14.0),
                ..default()
            },
        ))
        .with_children(|chin| {
            chin.spawn((
                Name::new("NovaOsBrandPlate"),
                NovaOsBrandPlateMarker,
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(11.0),
                    padding: UiRect {
                        left: Val::Px(11.0),
                        right: Val::Px(14.0),
                        top: Val::Px(7.0),
                        bottom: Val::Px(7.0),
                    },
                    border: UiRect::all(Val::Px(1.0)),
                    border_radius: BorderRadius::all(Val::Px(5.0)),
                    ..default()
                },
                // Recessed badge, matching the PoC `.plate`: a base DARKER than
                // the surrounding case, really dark edges, and a top(dark) ->
                // bottom(light-ish grey) gradient with a light lower catch, so it
                // reads pressed a little INTO the plastic (a 3D inset).
                BorderColor {
                    top: Color::srgba(0.0, 0.0, 0.0, 0.8),
                    left: Color::srgb_u8(3, 4, 6),
                    right: Color::srgb_u8(3, 4, 6),
                    bottom: Color::srgba(1.0, 1.0, 1.0, 0.11),
                },
                BackgroundColor(NOVA_OS_CASE_EDGE),
                BackgroundGradient(vec![LinearGradient::degrees(
                    180.0,
                    vec![
                        ColorStop::percent(Color::srgba(0.0, 0.0, 0.0, 0.45), 0.0),
                        ColorStop::percent(Color::srgba(0.82, 0.86, 0.91, 0.16), 100.0),
                    ],
                )
                .into()]),
            ))
            .with_children(|plate| {
                // Logo mark: SVG rendered to a PNG asset (Bevy UI cannot draw SVG).
                // Preloaded into NovaHudAssets; absent only on bare-app rigs.
                if let Some(crt_mark) = crt_mark {
                    plate.spawn((
                        Name::new("NovaOsBrandMark"),
                        ImageNode::new(crt_mark),
                        Node {
                            width: Val::Px(22.0),
                            height: Val::Px(22.0),
                            ..default()
                        },
                    ));
                }
                plate
                    .spawn((
                        Name::new("NovaOsBrandText"),
                        Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(3.0),
                            ..default()
                        },
                    ))
                    .with_children(|text| {
                        // Dark glyphs stamped INTO the plastic, with a light catch
                        // along the lower edge (a hard 1px offset, no blur - the
                        // pressed-in look, not the doubled text a blur would give).
                        text.spawn((
                            Name::new("NovaOsBrandWordmark"),
                            Text::new("NOVACRT 9000"),
                            nova_os_text_font(12.0, font.clone()),
                            TextColor(Color::srgb_u8(12, 16, 19)),
                            TextShadow {
                                offset: Vec2::new(0.0, 1.0),
                                color: Color::srgba(1.0, 1.0, 1.0, 0.12),
                            },
                        ));
                        text.spawn((
                            Name::new("NovaOsBrandSpec"),
                            Text::new("P22 GREEN PHOSPHOR . 15 IN . TYPE CQ-4"),
                            nova_os_text_font(8.0, font.clone()),
                            TextColor(Color::srgb_u8(16, 23, 27)),
                            TextShadow {
                                offset: Vec2::new(0.0, 1.0),
                                color: Color::srgba(1.0, 1.0, 1.0, 0.085),
                            },
                        ));
                    });
            });
            // Controls row: the working BRIGHT/SCAN knobs and SND/PWR buttons.
            chin.spawn((
                Name::new("NovaOsControlsRow"),
                NovaOsControlsRowMarker,
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::FlexEnd,
                    column_gap: Val::Px(14.0),
                    min_width: Val::Px(120.0),
                    min_height: Val::Px(26.0),
                    ..default()
                },
            ))
            .with_children(|controls| {
                spawn_nova_os_knob(
                    controls,
                    font.clone(),
                    settings,
                    NovaOsKnob::Bright,
                    "BRIGHT",
                );
                spawn_nova_os_knob(controls, font.clone(), settings, NovaOsKnob::Scan, "SCAN");
                spawn_nova_os_sound_button(controls, font.clone(), settings);
                spawn_nova_os_power_button(controls, font.clone());
            });
        });
}

/// One rotary knob (BRIGHT or SCAN): a clickable dial that cycles its 4 detents
/// on each press (PoC `.knob`), the pointer rotating to the detent angle, with a
/// small caption beneath. Spawns with the dial already rotated to the current
/// detent so a reopen shows the saved position; live turns are re-synced by
/// [`sync_nova_os_monitor_controls`].
pub(crate) fn spawn_nova_os_knob(
    controls: &mut ChildSpawnerCommands,
    font: Handle<Font>,
    settings: &NovaOsMonitorSettings,
    knob: NovaOsKnob,
    caption: &str,
) {
    let mut knob_cmd = controls.spawn((
        Name::new(format!("NovaOsKnob({caption})")),
        knob,
        Button,
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(3.0),
            ..default()
        },
    ));
    // Each knob cycles its own detent; the observer type differs per knob, so
    // attach it via EntityCommands rather than a shared bundle.
    match knob {
        NovaOsKnob::Bright => knob_cmd.observe(on_nova_os_bright_knob),
        NovaOsKnob::Scan => knob_cmd.observe(on_nova_os_scan_knob),
    };
    knob_cmd.with_children(|knob_node| {
        // The dial face: a dark moulded disc with a raised rim.
        knob_node
            .spawn((
                Name::new("NovaOsKnobDial"),
                NovaOsKnobDialMarker,
                knob,
                Node {
                    width: Val::Px(26.0),
                    height: Val::Px(26.0),
                    border: UiRect::all(Val::Px(1.0)),
                    border_radius: BorderRadius::MAX,
                    ..default()
                },
                BackgroundColor(NOVA_OS_DIAL_DARK),
                // Domed knob body: an off-centre highlight (PoC `circle at 34% 28%`)
                // falling to the dark disc rim reads as a rounded rotary.
                BackgroundGradient(vec![Gradient::from(RadialGradient::new(
                    UiPosition::anchor(Vec2::new(-0.16, -0.22)),
                    RadialGradientShape::ClosestSide,
                    vec![
                        ColorStop::percent(NOVA_OS_DIAL_LIT, 0.0),
                        ColorStop::percent(NOVA_OS_DIAL_MID, 58.0),
                        ColorStop::percent(NOVA_OS_DIAL_DARK, 100.0),
                    ],
                ))]),
                // A near-black inner rim (PoC `inset 0 0 0 1px rgba(0,0,0,0.8)`).
                BorderColor::all(NOVA_OS_BUTTON_BORDER),
                UiTransform::from_rotation(Rot2::degrees(settings.dial_angle(knob))),
                // The knob click is owned by the parent Button; the dial and
                // its pointer must not intercept the pick.
                Pickable::IGNORE,
            ))
            .with_children(|dial| {
                // Pointer: a bright phosphor tick near the top, sweeping as the
                // dial rotates around its centre.
                dial.spawn((
                    Name::new("NovaOsKnobPointer"),
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Px(2.0),
                        height: Val::Px(9.0),
                        top: Val::Px(2.0),
                        left: Val::Px(11.0),
                        ..default()
                    },
                    BackgroundColor(NOVA_OS_PHOSPHOR),
                    Pickable::IGNORE,
                ));
            });
        knob_node.spawn((
            Name::new("NovaOsKnobCaption"),
            Text::new(caption),
            nova_os_text_font(7.0, font),
            TextColor(NOVA_OS_PHOSPHOR_MUTED),
            Pickable::IGNORE,
        ));
    });
}

/// The SND speaker toggle (PoC `#soundBtn`): flips
/// [`NovaOsMonitorSettings::sound_enabled`], its indicator lit when armed and the
/// label reading "SND ON"/"SND OFF". Spawns matching the current state; live
/// flips are re-synced by [`sync_nova_os_monitor_controls`].
pub(crate) fn spawn_nova_os_sound_button(
    controls: &mut ChildSpawnerCommands,
    font: Handle<Font>,
    settings: &NovaOsMonitorSettings,
) {
    let on = settings.sound_enabled;
    controls.spawn((
        Name::new("NovaOsSoundButton"),
        NovaOsSoundButtonMarker,
        Button,
        nova_os_chin_button_node(),
        // The button chrome is static now; the bulb (below), not the border or
        // the label, carries the on/off state (owner playtest).
        BorderColor::all(NOVA_OS_BUTTON_BORDER),
        BackgroundColor(NOVA_OS_BUTTON_DEEP),
        nova_os_chin_button_gradient(),
        observe(on_nova_os_sound_button),
        children![
            (
                Name::new("NovaOsSoundIndicator"),
                NovaOsSoundIndicatorMarker,
                nova_os_bulb_node(),
                BackgroundColor(nova_os_bulb_color(on)),
                nova_os_bulb_gradient(),
                Pickable::IGNORE,
            ),
            (
                // A fixed legend: it never swaps text, so the bulb is the only
                // moving part reporting state.
                Name::new("NovaOsSoundLabel"),
                Text::new("SND"),
                nova_os_text_font(9.0, font),
                TextColor(NOVA_OS_TEXT),
                Pickable::IGNORE,
            ),
        ],
    ));
}

/// The PWR button + green power LED (PoC `#powerBtn`): pressing it drives the
/// existing animated close, the diegetic twin of the `exit` command.
pub(crate) fn spawn_nova_os_power_button(controls: &mut ChildSpawnerCommands, font: Handle<Font>) {
    controls.spawn((
        Name::new("NovaOsPowerButton"),
        NovaOsPowerButtonMarker,
        Button,
        nova_os_chin_button_node(),
        BorderColor::all(NOVA_OS_BUTTON_BORDER),
        BackgroundColor(NOVA_OS_BUTTON_DEEP),
        nova_os_chin_button_gradient(),
        observe(on_nova_os_power_button),
        children![
            (
                Name::new("NovaOsPowerLed"),
                NovaOsPowerLedMarker,
                nova_os_bulb_node(),
                // Lit green while powered; flashes orange during the power-down
                // close (see `drive_nova_os_power_led`).
                BackgroundColor(NOVA_OS_PHOSPHOR),
                nova_os_bulb_gradient(),
                Pickable::IGNORE,
            ),
            (
                Name::new("NovaOsPowerLabel"),
                Text::new("PWR"),
                nova_os_text_font(9.0, font),
                TextColor(NOVA_OS_TEXT),
                Pickable::IGNORE,
            ),
        ],
    ));
}

/// Shared node style for the SND/PWR chin buttons (PoC `.power-btn`): a small
/// pill with an indicator glyph beside a caption.
pub(crate) fn nova_os_chin_button_node() -> Node {
    Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: Val::Px(6.0),
        padding: UiRect::axes(Val::Px(8.0), Val::Px(5.0)),
        border: UiRect::all(Val::Px(1.0)),
        border_radius: BorderRadius::all(Val::Px(4.0)),
        ..default()
    }
}

/// The moulded 3D fill of a chin button (PoC `.power-btn`): a top-lit vertical
/// gradient over the dark base, plus a 1px inner top-highlight lip so the key
/// catches the light like raised plastic.
pub(crate) fn nova_os_chin_button_gradient() -> BackgroundGradient {
    BackgroundGradient(vec![
        LinearGradient::degrees(
            180.0,
            vec![
                ColorStop::percent(NOVA_OS_BUTTON_LIT, 0.0),
                ColorStop::percent(NOVA_OS_BUTTON_DEEP, 100.0),
            ],
        )
        .into(),
        // 1px lit lip along the top edge (PoC `inset 0 1px 0 rgba(255,255,255,.12)`).
        LinearGradient::degrees(
            180.0,
            vec![
                ColorStop::px(NOVA_OS_CASE_HIGHLIGHT, 0.0),
                ColorStop::px(NOVA_OS_CASE_HIGHLIGHT, 1.0),
                ColorStop::px(Color::NONE, 1.0),
            ],
        )
        .into(),
    ])
}

/// A 7px round indicator bulb inside a chin button (the SND / PWR LED). Its base
/// colour reports state; the glassy cap is fixed by [`nova_os_bulb_gradient`].
pub(crate) fn nova_os_bulb_node() -> Node {
    Node {
        width: Val::Px(7.0),
        height: Val::Px(7.0),
        border_radius: BorderRadius::MAX,
        ..default()
    }
}

/// A fixed upper-left glassy highlight over an indicator bulb, so the lit and
/// unlit states both read as a rounded glass LED rather than a flat dot.
pub(crate) fn nova_os_bulb_gradient() -> BackgroundGradient {
    BackgroundGradient(vec![Gradient::from(RadialGradient::new(
        UiPosition::anchor(Vec2::new(-0.2, -0.25)),
        RadialGradientShape::ClosestSide,
        vec![
            ColorStop::percent(Color::srgba(1.0, 1.0, 1.0, 0.5), 0.0),
            ColorStop::percent(Color::srgba(1.0, 1.0, 1.0, 0.12), 45.0),
            ColorStop::percent(Color::NONE, 100.0),
        ],
    ))])
}

/// Lit phosphor vs unlit dark-green for the SND bulb: the bulb going dark is how
/// the muted state reads now that the label never changes.
pub(crate) fn nova_os_bulb_color(on: bool) -> Color {
    if on {
        NOVA_OS_PHOSPHOR
    } else {
        NOVA_OS_BULB_OFF
    }
}
