//! The two UI themes the base mod ships, as builders.
//!
//! A theme is content like a section is content, so this file is a builder and
//! not a table of constants: `content gen` serializes both into
//! `assets/base/ui_themes/base.content.ron`, the content parity test asserts
//! those bytes, and a mod overlays either by declaring its id.
//!
//! Two looks ship:
//!
//! - [`phosphor_theme`] (`base/phosphor`, the default): the CLI-drawn terminal.
//!   Flat phosphor-on-black fills, 1px phosphor borders, inverted selection,
//!   2px corners, a block-meter slider and bracketed `[TAG]` badges. Controls
//!   read as elements a terminal drew.
//! - [`hardware_theme`] (`base/hardware`): the light-3D moulded casing.
//!   Case-gradient faces, drop-shadow bevels, amber selection, 7px corners, a
//!   solid slider fill and chip badges.
//!
//! Both are ROOT themes (`inherit: None`) and therefore complete. Hardware does
//! not derive from phosphor: the two disagree about nearly every role, so
//! inheriting would have been a parent whose every role a child replaced.
//!
//! [`phosphor_theme`] is also the BOOTSTRAP theme - `ActiveUiTheme::default()`
//! resolves it - so the loading screen paints before any content exists and the
//! built-in look and the shipped RON can never drift apart.
//!
//! # Where the values came from
//!
//! Verbatim from the accepted PoC `web/design/nova_ui_rework_poc.html` (its
//! `:root` tokens and control rules), by way of the per-widget paint functions
//! this theme replaced. Moving them here changed nothing visually - that is the
//! point, and the round-trip tests pin it.

use std::collections::BTreeMap;

use super::config::{
    BadgeRole, BarRole, ButtonRole, CheckboxRole, Effect, FieldRole, Fill, HeadPaint, RadialAnchor,
    RowPaint, RowRole, SliderMeter, SliderRole, StatePaint, Stop, SurfacePaint, ThemeColor,
    ToggleRole, UiMetrics, UiRoles, UiThemeConfig, HARDWARE_THEME_ID, PHOSPHOR_THEME_ID,
};

/// Both shipped themes, in the order the generated content file carries them -
/// the default first, so the Settings list reads default-first too.
pub fn base_ui_themes() -> Vec<UiThemeConfig> {
    vec![phosphor_theme(), hardware_theme()]
}

// -- shared shorthand --------------------------------------------------------

fn var(name: &str) -> ThemeColor {
    ThemeColor::var(name)
}

fn va(name: &str, alpha: f32) -> ThemeColor {
    ThemeColor::var_alpha(name, alpha)
}

/// The moulded-face drop shadow (PoC `--drop`): outset only, since bevy 0.19
/// `BoxShadow` has no inset shadows - the demo's inner rim and undercut are
/// approximated by the face gradient instead.
fn drop_shadow() -> Effect {
    Effect {
        color: va("black", 0.55),
        offset_y: 2.0,
        blur: 8.0,
    }
}

/// A coloured glow (selected and primary faces). Kept subtle so it reads as a
/// lit element, not a halo that fights the text.
fn glow(variable: &str) -> Effect {
    Effect {
        color: va(variable, 0.22),
        offset_y: 0.0,
        blur: 7.0,
    }
}

/// A three-stop moulded face over a base: lit top, body, shaded bottom.
fn bevel3(base: &str, top: &str, mid: &str, bottom: &str) -> Fill {
    Fill::linear(
        var(base),
        vec![
            Stop::new(var(top), 0.0),
            Stop::new(var(mid), 55.0),
            Stop::new(var(bottom), 100.0),
        ],
    )
}

/// A two-stop face over a base.
fn bevel2(base: &str, top: &str, bottom: &str) -> Fill {
    Fill::linear(
        var(base),
        vec![Stop::new(var(top), 0.0), Stop::new(var(bottom), 100.0)],
    )
}

fn solid(color: ThemeColor) -> Fill {
    Fill::Solid(color)
}

fn state(fill: Fill, border: ThemeColor, text: ThemeColor, effect: Option<Effect>) -> StatePaint {
    StatePaint {
        fill,
        border,
        text,
        effect,
    }
}

fn palette(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
    entries
        .iter()
        .map(|(name, hex)| ((*name).to_string(), (*hex).to_string()))
        .collect()
}

// -- base/phosphor -----------------------------------------------------------

/// The phosphor terminal theme: the shipped default, and the bootstrap theme
/// the earliest UI paints in.
pub fn phosphor_theme() -> UiThemeConfig {
    UiThemeConfig {
        id: PHOSPHOR_THEME_ID.to_string(),
        name: "Phosphor".to_string(),
        inherit: None,
        palette: palette(&[
            // The twelve semantic names every theme must carry.
            ("primary", "#36ff79"),
            ("secondary", "#19a64f"),
            ("label", "#0d6e35"),
            ("body", "#b9ffc9"),
            ("accent", "#ffb84a"),
            ("accent_high", "#ffd07a"),
            ("accent_low", "#e6952f"),
            ("inverted", "#04140a"),
            ("surface", "#001304"),
            ("void", "#03060b"),
            ("danger", "#ff4e42"),
            ("info", "#36a3ff"),
            // On a phosphor screen the ink and the lamp are the same green.
            ("nominal", "#36ff79"),
            // This look's own tones, referenced only by its role table.
            ("primary_high", "#7dffab"),
            ("primary_low", "#12b552"),
            ("screen_lifted", "#002b0f"),
            ("hover_text", "#d6ffe4"),
            ("danger_text_hot", "#ffd9d5"),
            ("black", "#000000"),
            ("white", "#ffffff"),
        ]),
        metrics: Some(UiMetrics {
            border_width: 1.0,
            radius: 2.0,
            panel_radius: 10.0,
        }),
        roles: UiRoles {
            button: Some(phosphor_button(
                // Neutral: a lit wash that brightens on hover and deepens on
                // press, the label staying phosphor throughout.
                state(
                    solid(va("primary", 0.05)),
                    va("primary", 0.4),
                    var("primary"),
                    None,
                ),
                state(
                    solid(va("primary", 0.12)),
                    var("primary"),
                    var("hover_text"),
                    None,
                ),
                state(
                    solid(va("primary", 0.2)),
                    var("primary"),
                    var("primary"),
                    None,
                ),
            )),
            button_primary: Some(phosphor_primary_button()),
            button_danger: Some(phosphor_button(
                state(
                    solid(va("danger", 0.06)),
                    va("danger", 0.5),
                    var("danger"),
                    None,
                ),
                state(
                    solid(va("danger", 0.16)),
                    var("danger"),
                    var("danger_text_hot"),
                    None,
                ),
                state(solid(va("danger", 0.2)), var("danger"), var("danger"), None),
            )),
            button_ghost: Some(phosphor_button(
                state(Fill::none(), va("primary", 0.25), var("primary"), None),
                state(
                    solid(va("primary", 0.06)),
                    va("primary", 0.4),
                    var("primary"),
                    None,
                ),
                state(
                    solid(va("primary", 0.14)),
                    var("primary"),
                    var("primary"),
                    None,
                ),
            )),
            panel: Some(SurfacePaint {
                // A dark screen face under a phosphor glow blooming down from
                // the top edge (PoC `radial-gradient(ellipse 70% 60% at 50% 0%,
                // phosphor .10, transparent 70%)`).
                fill: Fill::Radial {
                    base: var("surface"),
                    anchor: RadialAnchor::Top,
                    stops: vec![
                        Stop::new(va("primary", 0.06), 0.0),
                        Stop::new(va("black", 0.0), 70.0),
                    ],
                },
                border: va("primary", 0.16),
                effect: None,
                radius: None,
            }),
            panel_head: Some(HeadPaint {
                border: va("primary", 0.18),
                title: var("primary"),
                rule: va("label", 0.6),
                tag: var("primary"),
            }),
            list_row: Some(RowRole {
                normal: RowPaint {
                    fill: Fill::none(),
                    border: va("primary", 0.14),
                },
                hovered: RowPaint {
                    fill: solid(va("primary", 0.06)),
                    border: va("primary", 0.2),
                },
                // Selection INVERTS on this look rather than tinting.
                selected: RowPaint {
                    fill: solid(va("primary", 0.14)),
                    border: var("primary"),
                },
            }),
            segmented: Some(SurfacePaint {
                fill: solid(va("black", 0.35)),
                border: va("primary", 0.25),
                effect: None,
                radius: None,
            }),
            slider_track: Some(SliderRole {
                surface: SurfacePaint {
                    fill: solid(va("black", 0.5)),
                    border: va("primary", 0.32),
                    effect: None,
                    radius: None,
                },
                height: 14.0,
                meter: SliderMeter::Blocks {
                    segments: 24,
                    gap: 2.0,
                },
                lit: var("primary"),
                unlit: va("primary", 0.16),
            }),
            text_field: Some(washed_field("primary")),
            checkbox: Some(CheckboxRole {
                radius: None,
                off: state(Fill::none(), va("primary", 0.4), var("primary"), None),
                on: state(solid(var("primary")), var("primary"), var("inverted"), None),
            }),
            toggle: Some(ToggleRole {
                radius: None,
                knob_radius: 1.0,
                off: state(
                    solid(va("black", 0.4)),
                    va("primary", 0.35),
                    var("label"),
                    None,
                ),
                on: state(
                    solid(va("primary", 0.14)),
                    var("primary"),
                    var("primary"),
                    Some(Effect {
                        color: va("primary", 0.35),
                        offset_y: 0.0,
                        blur: 5.0,
                    }),
                ),
            }),
            badge: Some(BadgeRole {
                bracketed: true,
                padding_x: 2.0,
                border_width: 0.0,
                border_alpha: 0.0,
                fill_alpha: 0.0,
            }),
            scroll_bar: Some(BarRole {
                track: va("primary", 0.07),
                thumb: va("primary", 0.35),
            }),
            key_chip: Some(key_chip()),
            separator: Some(va("label", 0.5)),
        },
    }
}

/// The three phosphor button states that differ per variant, wrapped with the
/// four this look shares.
///
/// Disabled and the two selected states are IDENTICAL across variants here, and
/// that is the look rather than an accident: a selected phosphor control is
/// solid phosphor with inverted glyphs whatever it does, and a dead one is a
/// ghost of itself. Writing them once keeps the four variants honest about
/// where they actually differ.
fn phosphor_button(normal: StatePaint, hovered: StatePaint, pressed: StatePaint) -> ButtonRole {
    ButtonRole {
        normal,
        hovered,
        pressed,
        selected: phosphor_inverted(),
        selected_pressed: phosphor_inverted_pressed(),
        disabled: phosphor_disabled(),
    }
}

/// Selected: solid phosphor, inverted glyphs, lit.
fn phosphor_inverted() -> StatePaint {
    state(
        solid(var("primary")),
        var("primary"),
        var("inverted"),
        Some(glow("primary")),
    )
}

/// Selected and held: the lit face SINKS (an inverted control is already at
/// full phosphor, so dimming is the only move left) and the glow goes out.
fn phosphor_inverted_pressed() -> StatePaint {
    state(
        solid(var("primary_low")),
        var("primary"),
        var("inverted"),
        None,
    )
}

fn phosphor_disabled() -> StatePaint {
    state(
        solid(va("primary", 0.02)),
        va("primary", 0.12),
        va("primary", 0.3),
        None,
    )
}

/// Primary reads as a PERMANENT selection: solid phosphor in every live state,
/// so hovering it changes nothing and pressing it sinks it.
fn phosphor_primary_button() -> ButtonRole {
    ButtonRole {
        normal: phosphor_inverted(),
        hovered: phosphor_inverted(),
        pressed: phosphor_inverted_pressed(),
        selected: phosphor_inverted(),
        selected_pressed: phosphor_inverted_pressed(),
        disabled: phosphor_disabled(),
    }
}

/// A text field over one ink colour: the terminal look types in phosphor, the
/// hardware look in the casing's own ink. Everything else about the field is
/// shared, which is why one builder serves both.
///
/// The ERROR state carries the focused fill on purpose: a field only becomes
/// invalid because somebody typed into it, so it is focused in practice, and
/// giving error the idle wash made a live field flicker darker as it went bad.
fn washed_field(ink: &str) -> FieldRole {
    FieldRole {
        normal: state(solid(va(ink, 0.035)), va(ink, 0.32), var(ink), None),
        hovered: state(solid(va(ink, 0.035)), va(ink, 0.65), var(ink), None),
        focused: state(solid(va(ink, 0.08)), var("accent"), var(ink), None),
        error: state(solid(va(ink, 0.08)), var("danger"), var(ink), None),
    }
}

/// The keycap chip: an amber legend in a dark well. The one role both shipped
/// looks paint identically - a keycap is a keycap.
fn key_chip() -> StatePaint {
    state(
        solid(va("black", 0.3)),
        va("accent", 0.5),
        var("accent"),
        None,
    )
}

// -- base/hardware -----------------------------------------------------------

/// The light-3D hardware casing theme: moulded faces, amber selection.
pub fn hardware_theme() -> UiThemeConfig {
    UiThemeConfig {
        id: HARDWARE_THEME_ID.to_string(),
        name: "Hardware".to_string(),
        // A ROOT theme, not a derivation of phosphor: it disagrees with
        // phosphor about nearly every role, so inheriting would name a parent
        // whose every role the child replaced.
        inherit: None,
        palette: palette(&[
            // The twelve semantic names in the CASING's own tones. Copying
            // phosphor's greens here is what made every label, title and body
            // line on this look read green: the screens paint through these
            // names, so a look that shares them cannot differ from the one it
            // copied, whatever its role table says.
            ("primary", "#dcefe0"),
            ("secondary", "#8b9aa3"),
            ("label", "#69777f"),
            ("body", "#c3d2d8"),
            ("accent", "#ffb84a"),
            ("accent_high", "#ffd07a"),
            ("accent_low", "#e6952f"),
            ("inverted", "#0a0d10"),
            ("surface", "#161b20"),
            ("void", "#05070a"),
            ("danger", "#ff4e42"),
            ("info", "#36a3ff"),
            // The ink is bone white, but a lamp that is ON is still green.
            ("nominal", "#36ff79"),
            ("nominal_high", "#7dffab"),
            ("nominal_low", "#12b552"),
            ("black", "#000000"),
            ("white", "#ffffff"),
            // The moulded casing tones (PoC `--case-*`).
            ("case_0", "#0a0d10"),
            ("case_1", "#161b20"),
            ("case_2", "#232a31"),
            ("case_3", "#2f383f"),
            ("case_edge", "#05070a"),
            ("face_hot_high", "#3a444c"),
            ("face_hot_mid", "#222a31"),
            ("face_hot_low", "#12171b"),
            ("case_text", "#dcefe0"),
            ("danger_text_hot", "#ffd9d5"),
            ("danger_face_lit", "#6b2a26"),
            ("danger_face_dark", "#3a1512"),
            ("knob_off", "#55636c"),
        ]),
        metrics: Some(UiMetrics {
            border_width: 1.0,
            radius: 7.0,
            panel_radius: 10.0,
        }),
        roles: UiRoles {
            button: Some(hardware_button(
                hardware_face(var("case_text")),
                state(
                    bevel3(
                        "face_hot_mid",
                        "face_hot_high",
                        "face_hot_mid",
                        "face_hot_low",
                    ),
                    var("case_edge"),
                    var("white"),
                    Some(drop_shadow()),
                ),
                // Pressed SINKS: the bevel inverts and the drop shadow goes.
                state(
                    bevel2("case_0", "case_0", "case_1"),
                    var("case_edge"),
                    var("case_text"),
                    None,
                ),
            )),
            button_primary: Some(hardware_primary_button()),
            button_danger: Some(hardware_button(
                hardware_face(var("danger_text_hot")),
                state(
                    bevel2("danger", "danger_face_lit", "danger_face_dark"),
                    var("case_edge"),
                    var("white"),
                    Some(drop_shadow()),
                ),
                state(
                    bevel2("danger", "danger_face_dark", "danger_face_lit"),
                    var("case_edge"),
                    var("white"),
                    None,
                ),
            )),
            // Ghost stays fill-less by contract, so its press cannot be a
            // bevel: it is a dark wash under a brightening border instead.
            button_ghost: Some(hardware_button(
                state(Fill::none(), va("white", 0.12), var("case_text"), None),
                state(
                    solid(va("white", 0.04)),
                    va("white", 0.22),
                    var("case_text"),
                    None,
                ),
                state(
                    solid(va("black", 0.22)),
                    va("white", 0.3),
                    var("case_text"),
                    None,
                ),
            )),
            panel: Some(SurfacePaint {
                // PoC panel: linear-gradient(168deg, case-2, case-0 88%, edge).
                fill: Fill::Linear {
                    base: var("case_1"),
                    degrees: 168.0,
                    stops: vec![
                        Stop::new(var("case_2"), 0.0),
                        Stop::new(var("case_0"), 88.0),
                        Stop::new(var("case_edge"), 100.0),
                    ],
                },
                border: var("case_edge"),
                effect: Some(drop_shadow()),
                radius: None,
            }),
            panel_head: Some(HeadPaint {
                border: var("case_edge"),
                title: var("white"),
                rule: var("case_3"),
                tag: va("white", 0.7),
            }),
            list_row: Some(RowRole {
                normal: RowPaint {
                    fill: solid(va("white", 0.02)),
                    border: va("white", 0.05),
                },
                hovered: RowPaint {
                    fill: solid(va("white", 0.06)),
                    border: va("white", 0.1),
                },
                // Selection TINTS amber on this look rather than inverting.
                selected: RowPaint {
                    fill: solid(var("case_2")),
                    border: va("accent", 0.5),
                },
            }),
            segmented: Some(SurfacePaint {
                fill: solid(va("black", 0.4)),
                border: var("case_edge"),
                effect: None,
                radius: None,
            }),
            slider_track: Some(SliderRole {
                surface: SurfacePaint {
                    fill: solid(va("black", 0.55)),
                    border: var("case_edge"),
                    effect: None,
                    radius: Some(6.0),
                },
                height: 10.0,
                meter: SliderMeter::Fill,
                lit: var("nominal_low"),
                // The solid fill draws nothing behind itself: the well IS the
                // unlit part.
                unlit: va("black", 0.0),
            }),
            text_field: Some(washed_field("case_text")),
            checkbox: Some(CheckboxRole {
                radius: Some(5.0),
                off: state(solid(var("case_0")), var("case_edge"), var("nominal"), None),
                // Checked is the same lit square in both looks: a checkbox
                // reads as ON or it does not.
                on: state(solid(var("nominal")), var("nominal"), var("inverted"), None),
            }),
            toggle: Some(ToggleRole {
                radius: Some(11.0),
                knob_radius: 9.0,
                off: state(
                    solid(va("black", 0.5)),
                    var("case_edge"),
                    var("knob_off"),
                    None,
                ),
                on: state(
                    solid(va("nominal", 0.14)),
                    var("nominal"),
                    var("nominal"),
                    None,
                ),
            }),
            badge: Some(BadgeRole {
                bracketed: false,
                padding_x: 8.0,
                border_width: 1.0,
                border_alpha: 0.4,
                fill_alpha: 0.06,
            }),
            scroll_bar: Some(BarRole {
                track: va("white", 0.04),
                thumb: var("case_3"),
            }),
            key_chip: Some(key_chip()),
            separator: Some(va("label", 0.5)),
        },
    }
}

/// The idle moulded face: a lit top falling to a shaded bottom, standing on its
/// drop shadow. The look's default surface - a button, a disabled button and a
/// resting danger button are all this face under different legends.
fn hardware_face(text: ThemeColor) -> StatePaint {
    state(
        bevel3("case_1", "case_3", "case_1", "case_0"),
        var("case_edge"),
        text,
        Some(drop_shadow()),
    )
}

/// The three hardware button states that differ per variant, wrapped with the
/// four this look shares.
///
/// Selection is AMBER on every variant - a selected hardware control is an
/// amber-lit moulded face whatever it does - and disabled is the idle face
/// under a dimmed legend.
fn hardware_button(normal: StatePaint, hovered: StatePaint, pressed: StatePaint) -> ButtonRole {
    ButtonRole {
        normal,
        hovered,
        pressed,
        selected: hardware_amber(),
        selected_pressed: hardware_amber_pressed(),
        disabled: hardware_face(va("case_text", 0.34)),
    }
}

fn hardware_amber() -> StatePaint {
    state(
        bevel3("accent", "accent_high", "accent", "accent_low"),
        var("case_edge"),
        var("inverted"),
        Some(glow("accent")),
    )
}

fn hardware_amber_pressed() -> StatePaint {
    state(
        bevel3("accent", "accent_low", "accent", "accent_high"),
        var("case_edge"),
        var("inverted"),
        None,
    )
}

/// Primary wears the LAMP gradient the way the neutral button wears the case
/// one - lit in every live state, sinking on press. Selection still wins: a
/// selected primary button is amber, like every other selected control here.
fn hardware_primary_button() -> ButtonRole {
    let lit = || {
        state(
            bevel3("nominal", "nominal_high", "nominal", "nominal_low"),
            var("case_edge"),
            var("inverted"),
            Some(glow("nominal")),
        )
    };
    ButtonRole {
        normal: lit(),
        hovered: lit(),
        pressed: state(
            bevel3("nominal", "nominal_low", "nominal", "nominal_high"),
            var("case_edge"),
            var("inverted"),
            None,
        ),
        selected: hardware_amber(),
        selected_pressed: hardware_amber_pressed(),
        disabled: hardware_face(va("case_text", 0.34)),
    }
}
