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

use super::config::{
    BadgeRole, BarRole, ButtonRole, CheckboxRole, Effect, FieldRole, Fill, HeadPaint, RadialAnchor,
    RowPaint, RowRole, SliderMeter, SliderRole, StatePaint, Stop, SurfacePaint, ThemeColor,
    ToggleRole, UiColorType as C, UiMetrics, UiPalette, UiRoles, UiThemeConfig, HARDWARE_THEME_ID,
    PHOSPHOR_THEME_ID,
};

/// Both shipped themes, in the order the generated content file carries them -
/// the default first, so the Settings list reads default-first too.
pub fn base_ui_themes() -> Vec<UiThemeConfig> {
    vec![phosphor_theme(), hardware_theme()]
}

// -- shared shorthand --------------------------------------------------------

fn var(color: C) -> ThemeColor {
    ThemeColor::var(color)
}

fn va(color: C, alpha: f32) -> ThemeColor {
    ThemeColor::var_alpha(color, alpha)
}

/// The moulded-face drop shadow (PoC `--drop`): outset only, since bevy 0.19
/// `BoxShadow` has no inset shadows - the demo's inner rim and undercut are
/// approximated by the face gradient instead.
fn drop_shadow() -> Effect {
    Effect {
        color: va(C::Shadow, 0.55),
        offset_y: 2.0,
        blur: 8.0,
    }
}

/// A coloured glow (selected and primary faces). Kept subtle so it reads as a
/// lit element, not a halo that fights the text.
fn glow(color: C) -> Effect {
    Effect {
        color: va(color, 0.22),
        offset_y: 0.0,
        blur: 7.0,
    }
}

/// A three-stop moulded face over a base: lit top, body, shaded bottom.
fn bevel3(base: C, top: C, mid: C, bottom: C) -> Fill {
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
fn bevel2(base: C, top: C, bottom: C) -> Fill {
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

fn hex(value: &str) -> Option<String> {
    Some(value.to_string())
}

// -- base/phosphor -----------------------------------------------------------

/// The phosphor terminal theme: the shipped default, and the bootstrap theme
/// the earliest UI paints in.
pub fn phosphor_theme() -> UiThemeConfig {
    UiThemeConfig {
        id: PHOSPHOR_THEME_ID.to_string(),
        name: "Phosphor".to_string(),
        inherit: None,
        palette: UiPalette {
            text: hex("#36ff79"),
            text_body: hex("#b9ffc9"),
            text_secondary: hex("#19a64f"),
            text_muted: hex("#0d6e35"),
            text_hover: hex("#d6ffe4"),
            text_inverse: hex("#04140a"),
            surface: hex("#001304"),
            // The faces below are the hardware look's. This look's role table
            // does not read them, so each holds the nearest phosphor tone for
            // its job: a derived theme that paints with them stays green.
            surface_sunken: hex("#03060b"),
            surface_raised: hex("#002b0f"),
            surface_lit: hex("#0d6e35"),
            surface_hover: hex("#002b0f"),
            surface_hover_light: hex("#0d6e35"),
            surface_hover_dark: hex("#001304"),
            background: hex("#03060b"),
            border: hex("#0d6e35"),
            accent: hex("#ffb84a"),
            accent_light: hex("#ffd07a"),
            accent_dark: hex("#e6952f"),
            // On a phosphor screen the ink and the lamp are the same green.
            nominal: hex("#36ff79"),
            nominal_light: hex("#7dffab"),
            nominal_dark: hex("#12b552"),
            danger: hex("#ff4e42"),
            danger_text: hex("#ffd9d5"),
            danger_face: hex("#6b2a26"),
            danger_face_dark: hex("#3a1512"),
            info: hex("#36a3ff"),
            control_off: hex("#0d6e35"),
            highlight: hex("#ffffff"),
            shadow: hex("#000000"),
        },
        metrics: Some(UiMetrics {
            border_width: 1.0,
            radius: 2.0,
            // The same hard corner as a control: a phosphor panel is a drawn
            // frame on a screen, and a soft card corner beside 2 px buttons
            // read as two design languages.
            panel_radius: 2.0,
        }),
        roles: UiRoles {
            button: Some(phosphor_button(
                // Neutral: a lit wash that brightens on hover and deepens on
                // press, the label staying phosphor throughout.
                state(
                    solid(va(C::Text, 0.05)),
                    va(C::Text, 0.4),
                    var(C::Text),
                    None,
                ),
                state(
                    solid(va(C::Text, 0.12)),
                    var(C::Text),
                    var(C::TextHover),
                    None,
                ),
                state(solid(va(C::Text, 0.2)), var(C::Text), var(C::Text), None),
            )),
            button_primary: Some(phosphor_primary_button()),
            button_danger: Some(phosphor_button(
                state(
                    solid(va(C::Danger, 0.06)),
                    va(C::Danger, 0.5),
                    var(C::Danger),
                    None,
                ),
                state(
                    solid(va(C::Danger, 0.16)),
                    var(C::Danger),
                    var(C::DangerText),
                    None,
                ),
                state(
                    solid(va(C::Danger, 0.2)),
                    var(C::Danger),
                    var(C::Danger),
                    None,
                ),
            )),
            button_ghost: Some(phosphor_button(
                state(Fill::none(), va(C::Text, 0.25), var(C::Text), None),
                state(
                    solid(va(C::Text, 0.06)),
                    va(C::Text, 0.4),
                    var(C::Text),
                    None,
                ),
                state(solid(va(C::Text, 0.14)), var(C::Text), var(C::Text), None),
            )),
            panel: Some(SurfacePaint {
                // A dark screen face under a phosphor glow blooming down from
                // the top edge (PoC `radial-gradient(ellipse 70% 60% at 50% 0%,
                // phosphor .10, transparent 70%)`).
                fill: Fill::Radial {
                    base: var(C::Surface),
                    anchor: RadialAnchor::Top,
                    stops: vec![
                        Stop::new(va(C::Text, 0.06), 0.0),
                        Stop::new(va(C::Shadow, 0.0), 70.0),
                    ],
                },
                border: va(C::Text, 0.16),
                effect: None,
                radius: None,
            }),
            panel_head: Some(HeadPaint {
                border: va(C::Text, 0.18),
                title: var(C::Text),
                rule: va(C::TextMuted, 0.6),
                tag: var(C::Text),
            }),
            list_row: Some(RowRole {
                normal: RowPaint {
                    fill: Fill::none(),
                    border: va(C::Text, 0.14),
                },
                hovered: RowPaint {
                    fill: solid(va(C::Text, 0.06)),
                    border: va(C::Text, 0.2),
                },
                // Selection INVERTS on this look rather than tinting.
                selected: RowPaint {
                    fill: solid(va(C::Text, 0.14)),
                    border: var(C::Text),
                },
            }),
            segmented: Some(SurfacePaint {
                fill: solid(va(C::Shadow, 0.35)),
                border: va(C::Text, 0.25),
                effect: None,
                radius: None,
            }),
            slider_track: Some(SliderRole {
                surface: SurfacePaint {
                    fill: solid(va(C::Shadow, 0.5)),
                    border: va(C::Text, 0.32),
                    effect: None,
                    radius: None,
                },
                height: 14.0,
                meter: SliderMeter::Blocks {
                    segments: 24,
                    gap: 2.0,
                },
                lit: var(C::Text),
                unlit: va(C::Text, 0.16),
            }),
            text_field: Some(washed_field()),
            checkbox: Some(CheckboxRole {
                radius: None,
                off: state(Fill::none(), va(C::Text, 0.4), var(C::Text), None),
                on: state(solid(var(C::Text)), var(C::Text), var(C::TextInverse), None),
            }),
            toggle: Some(ToggleRole {
                radius: None,
                knob_radius: 1.0,
                off: state(
                    solid(va(C::Shadow, 0.4)),
                    va(C::Text, 0.35),
                    var(C::ControlOff),
                    None,
                ),
                on: state(
                    solid(va(C::Text, 0.14)),
                    var(C::Text),
                    var(C::Text),
                    Some(Effect {
                        color: va(C::Text, 0.35),
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
                track: va(C::Text, 0.07),
                thumb: va(C::Text, 0.35),
            }),
            key_chip: Some(key_chip()),
            separator: Some(va(C::TextMuted, 0.5)),
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
        solid(var(C::Text)),
        var(C::Text),
        var(C::TextInverse),
        Some(glow(C::Text)),
    )
}

/// Selected and held: the lit face SINKS (an inverted control is already at
/// full phosphor, so dimming is the only move left) and the glow goes out.
fn phosphor_inverted_pressed() -> StatePaint {
    state(
        solid(var(C::NominalDark)),
        var(C::Text),
        var(C::TextInverse),
        None,
    )
}

fn phosphor_disabled() -> StatePaint {
    state(
        solid(va(C::Text, 0.02)),
        va(C::Text, 0.25),
        va(C::Text, 0.5),
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

/// A text field in the look's ink: phosphor types in green, hardware in the
/// casing's bone white, both through [`C::Text`]. One builder serves both.
///
/// The ERROR state carries the focused fill on purpose: a field only becomes
/// invalid because somebody typed into it, so it is focused in practice, and
/// giving error the idle wash made a live field flicker darker as it went bad.
fn washed_field() -> FieldRole {
    let ink = C::Text;
    FieldRole {
        normal: state(solid(va(ink, 0.035)), va(ink, 0.32), var(ink), None),
        hovered: state(solid(va(ink, 0.035)), va(ink, 0.65), var(ink), None),
        focused: state(solid(va(ink, 0.08)), var(C::Accent), var(ink), None),
        error: state(solid(va(ink, 0.08)), var(C::Danger), var(ink), None),
    }
}

/// The keycap chip: an amber legend in a dark well. The one role both shipped
/// looks paint identically - a keycap is a keycap.
fn key_chip() -> StatePaint {
    state(
        solid(va(C::Shadow, 0.3)),
        va(C::Accent, 0.5),
        var(C::Accent),
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
        palette: UiPalette {
            // The casing's own ink. Copying phosphor's greens here is what
            // made every label, title and body line on this look read green:
            // the screens paint through these colours, so a look that shares
            // them cannot differ from the one it copied, whatever its role
            // table says.
            text: hex("#dcefe0"),
            text_body: hex("#c3d2d8"),
            text_secondary: hex("#8b9aa3"),
            text_muted: hex("#69777f"),
            text_hover: hex("#ffffff"),
            text_inverse: hex("#0a0d10"),
            // The moulded casing tones (PoC `--case-*`).
            surface: hex("#161b20"),
            surface_sunken: hex("#0a0d10"),
            surface_raised: hex("#232a31"),
            surface_lit: hex("#2f383f"),
            surface_hover: hex("#222a31"),
            surface_hover_light: hex("#3a444c"),
            surface_hover_dark: hex("#12171b"),
            background: hex("#05070a"),
            border: hex("#05070a"),
            accent: hex("#ffb84a"),
            accent_light: hex("#ffd07a"),
            accent_dark: hex("#e6952f"),
            // The ink is bone white, but a lamp that is ON is still green.
            nominal: hex("#36ff79"),
            nominal_light: hex("#7dffab"),
            nominal_dark: hex("#12b552"),
            danger: hex("#ff4e42"),
            danger_text: hex("#ffd9d5"),
            danger_face: hex("#6b2a26"),
            danger_face_dark: hex("#3a1512"),
            info: hex("#36a3ff"),
            control_off: hex("#55636c"),
            highlight: hex("#ffffff"),
            shadow: hex("#000000"),
        },
        metrics: Some(UiMetrics {
            border_width: 1.0,
            radius: 7.0,
            panel_radius: 10.0,
        }),
        roles: UiRoles {
            button: Some(hardware_button(
                hardware_face(var(C::Text)),
                state(
                    bevel3(
                        C::SurfaceHover,
                        C::SurfaceHoverLight,
                        C::SurfaceHover,
                        C::SurfaceHoverDark,
                    ),
                    var(C::Border),
                    var(C::TextHover),
                    Some(drop_shadow()),
                ),
                // Pressed SINKS: the bevel inverts and the drop shadow goes.
                state(
                    bevel2(C::SurfaceSunken, C::SurfaceSunken, C::Surface),
                    var(C::Border),
                    var(C::Text),
                    None,
                ),
            )),
            button_primary: Some(hardware_primary_button()),
            button_danger: Some(hardware_button(
                hardware_face(var(C::DangerText)),
                state(
                    bevel2(C::Danger, C::DangerFace, C::DangerFaceDark),
                    var(C::Border),
                    var(C::TextHover),
                    Some(drop_shadow()),
                ),
                state(
                    bevel2(C::Danger, C::DangerFaceDark, C::DangerFace),
                    var(C::Border),
                    var(C::Highlight),
                    None,
                ),
            )),
            // Ghost stays fill-less by contract, so its press cannot be a
            // bevel: it is a dark wash under a brightening border instead.
            button_ghost: Some(hardware_button(
                state(Fill::none(), va(C::Highlight, 0.12), var(C::Text), None),
                state(
                    solid(va(C::Highlight, 0.04)),
                    va(C::Highlight, 0.22),
                    var(C::Text),
                    None,
                ),
                state(
                    solid(va(C::Shadow, 0.22)),
                    va(C::Highlight, 0.3),
                    var(C::Text),
                    None,
                ),
            )),
            panel: Some(SurfacePaint {
                // PoC panel: linear-gradient(168deg, case-2, case-0 88%, edge).
                fill: Fill::Linear {
                    base: var(C::Surface),
                    degrees: 168.0,
                    stops: vec![
                        Stop::new(var(C::SurfaceRaised), 0.0),
                        Stop::new(var(C::SurfaceSunken), 88.0),
                        Stop::new(var(C::Border), 100.0),
                    ],
                },
                border: var(C::Border),
                effect: Some(drop_shadow()),
                radius: None,
            }),
            panel_head: Some(HeadPaint {
                border: var(C::Border),
                title: var(C::Highlight),
                rule: var(C::SurfaceLit),
                tag: va(C::Highlight, 0.7),
            }),
            list_row: Some(RowRole {
                normal: RowPaint {
                    fill: solid(va(C::Highlight, 0.02)),
                    border: va(C::Highlight, 0.05),
                },
                hovered: RowPaint {
                    fill: solid(va(C::Highlight, 0.06)),
                    border: va(C::Highlight, 0.1),
                },
                // Selection TINTS amber on this look rather than inverting.
                selected: RowPaint {
                    fill: solid(var(C::SurfaceRaised)),
                    border: va(C::Accent, 0.5),
                },
            }),
            segmented: Some(SurfacePaint {
                fill: solid(va(C::Shadow, 0.4)),
                border: var(C::Border),
                effect: None,
                radius: None,
            }),
            slider_track: Some(SliderRole {
                surface: SurfacePaint {
                    fill: solid(va(C::Shadow, 0.55)),
                    border: var(C::Border),
                    effect: None,
                    radius: Some(6.0),
                },
                height: 10.0,
                meter: SliderMeter::Fill,
                lit: var(C::NominalDark),
                // The solid fill draws nothing behind itself: the well IS the
                // unlit part.
                unlit: va(C::Shadow, 0.0),
            }),
            text_field: Some(washed_field()),
            checkbox: Some(CheckboxRole {
                radius: Some(5.0),
                off: state(
                    solid(var(C::SurfaceSunken)),
                    var(C::Border),
                    var(C::Nominal),
                    None,
                ),
                // Checked is the same lit square in both looks: a checkbox
                // reads as ON or it does not.
                on: state(
                    solid(var(C::Nominal)),
                    var(C::Nominal),
                    var(C::TextInverse),
                    None,
                ),
            }),
            toggle: Some(ToggleRole {
                radius: Some(11.0),
                knob_radius: 9.0,
                off: state(
                    solid(va(C::Shadow, 0.5)),
                    var(C::Border),
                    var(C::ControlOff),
                    None,
                ),
                on: state(
                    solid(va(C::Nominal, 0.14)),
                    var(C::Nominal),
                    var(C::Nominal),
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
                track: va(C::Highlight, 0.04),
                thumb: var(C::SurfaceLit),
            }),
            key_chip: Some(key_chip()),
            separator: Some(va(C::TextMuted, 0.5)),
        },
    }
}

/// The idle moulded face: a lit top falling to a shaded bottom, standing on its
/// drop shadow. The look's default surface - a button, a disabled button and a
/// resting danger button are all this face under different legends.
fn hardware_face(text: ThemeColor) -> StatePaint {
    state(
        bevel3(C::Surface, C::SurfaceLit, C::Surface, C::SurfaceSunken),
        var(C::Border),
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
        disabled: hardware_face(va(C::Text, 0.4)),
    }
}

fn hardware_amber() -> StatePaint {
    state(
        bevel3(C::Accent, C::AccentLight, C::Accent, C::AccentDark),
        var(C::Border),
        var(C::TextInverse),
        Some(glow(C::Accent)),
    )
}

fn hardware_amber_pressed() -> StatePaint {
    state(
        bevel3(C::Accent, C::AccentDark, C::Accent, C::AccentLight),
        var(C::Border),
        var(C::TextInverse),
        None,
    )
}

/// Primary wears the LAMP gradient the way the neutral button wears the case
/// one - lit in every live state, sinking on press. Selection still wins: a
/// selected primary button is amber, like every other selected control here.
fn hardware_primary_button() -> ButtonRole {
    let lit = || {
        state(
            bevel3(C::Nominal, C::NominalLight, C::Nominal, C::NominalDark),
            var(C::Border),
            var(C::TextInverse),
            Some(glow(C::Nominal)),
        )
    };
    ButtonRole {
        normal: lit(),
        hovered: lit(),
        pressed: state(
            bevel3(C::Nominal, C::NominalDark, C::Nominal, C::NominalLight),
            var(C::Border),
            var(C::TextInverse),
            None,
        ),
        selected: hardware_amber(),
        selected_pressed: hardware_amber_pressed(),
        disabled: hardware_face(va(C::Text, 0.4)),
    }
}
