//! The theme format, its resolver and its registry, proved on the two shipped
//! themes and on the smallest hand-written ones that isolate one rule each.
//!
//! The claims here are the ones a mod author depends on: what a theme file
//! decodes to, what an omitted alpha means, which theme a role comes from, what
//! a redeclared role does to the states it does not mention, and what happens
//! when the selected theme goes away.

use bevy::prelude::*;

use super::{
    base::base_ui_themes,
    config::{
        BadgeRole, ColorValue, Fill, RowPaint, RowRole, SliderMeter, ThemeColor, UiMetrics,
        UiRoles, UiThemeConfig, HARDWARE_THEME_ID, PHOSPHOR_THEME_ID,
    },
    registry::{GameUiThemes, SelectedUiTheme, UiThemeDiagnostic},
    resolve::{lint_theme, resolve_theme, RowState, UiColor, UiMetric},
    ActiveUiTheme,
};
use crate::NovaUiPlugin;

/// A derived theme with nothing of its own - the minimum a mod ships to say
/// "the phosphor look, but".
fn derived(id: &str, roles: UiRoles) -> UiThemeConfig {
    UiThemeConfig {
        id: id.to_string(),
        name: id.to_string(),
        inherit: Some(PHOSPHOR_THEME_ID.to_string()),
        palette: std::collections::BTreeMap::new(),
        metrics: None,
        roles,
    }
}

/// A flat row paint, so a test can say "this row, this colour" in one line.
fn row(hex: &str) -> RowPaint {
    RowPaint {
        fill: Fill::solid(ThemeColor::literal(hex)),
        border: ThemeColor::literal_alpha(hex, 0.5),
    }
}

/// A theme file is ORDINARY RON content: the `Content::UiTheme` wrapper, the
/// enum-tagged colour values, and `deny_unknown_fields` on every struct. Decode
/// is what a mod author's file goes through, so it is proved on text rather
/// than on a builder.
#[test]
fn a_theme_decodes_from_authored_ron() {
    let authored = r##"(
        id: "wildcat/amber_crt",
        name: "Amber CRT",
        inherit: Some("base/phosphor"),
        palette: {
            "primary": "#ffb84a",
        },
        metrics: None,
        roles: (
            separator: Some((value: Variable("primary"), alpha: 0.25)),
        ),
    )"##;
    let config: UiThemeConfig = ron::from_str(authored).expect("the theme decodes");
    assert_eq!(config.id, "wildcat/amber_crt");
    assert_eq!(config.inherit.as_deref(), Some(PHOSPHOR_THEME_ID));
    assert_eq!(
        config.palette.get("primary").map(String::as_str),
        Some("#ffb84a")
    );
    assert!(config.metrics.is_none(), "a derived theme may omit metrics");
    let separator = config.roles.separator.expect("the one role it declares");
    assert_eq!(separator.value, ColorValue::Variable("primary".to_string()));
    assert!((separator.alpha - 0.25).abs() < f32::EPSILON);
}

/// An unknown field is REFUSED rather than ignored - a typo in a theme file is
/// an authoring error, not a silently missing colour.
#[test]
fn an_unknown_theme_field_is_refused() {
    let authored = r##"(
        id: "wildcat/typo",
        name: "Typo",
        inherit: Some("base/phosphor"),
        palette: {},
        metrics: None,
        roles: (),
        glow: 3.0,
    )"##;
    ron::from_str::<UiThemeConfig>(authored).expect_err("an unknown field must be refused");
}

/// An omitted alpha means OPAQUE, and a written one OVERWRITES rather than
/// multiplying - so the same variable at `0.12` reads the same in every theme
/// that defines it.
#[test]
fn an_omitted_alpha_is_opaque_and_a_written_one_replaces_it() {
    let opaque: ThemeColor =
        ron::from_str(r#"(value: Variable("primary"))"#).expect("the colour decodes");
    assert!((opaque.alpha - 1.0).abs() < f32::EPSILON);

    let mut themes = base_ui_themes();
    themes.push(derived(
        "t/alpha",
        UiRoles {
            separator: Some(ThemeColor::var_alpha("primary", 0.25)),
            ..UiRoles::default()
        },
    ));
    let theme = resolve_theme("t/alpha", &themes).expect("it resolves");
    let primary = theme.color(UiColor::Primary);
    assert_eq!(
        theme.separator(),
        primary.with_alpha(0.25),
        "the authored alpha replaces the variable's own"
    );
}

/// A palette variable resolves through the chain, and a LITERAL resolves
/// without one - the two halves of `ColorValue`.
#[test]
fn a_colour_resolves_from_the_palette_or_from_a_literal() {
    let mut themes = base_ui_themes();
    themes.push(derived(
        "t/literal",
        UiRoles {
            separator: Some(ThemeColor::literal("#112233")),
            ..UiRoles::default()
        },
    ));
    let theme = resolve_theme("t/literal", &themes).expect("it resolves");
    assert_eq!(theme.separator(), Color::srgb_u8(0x11, 0x22, 0x33));
}

/// An unknown palette name is an ERROR, never a silent black.
#[test]
fn an_unknown_palette_name_is_refused() {
    let mut themes = base_ui_themes();
    themes.push(derived(
        "t/unknown",
        UiRoles {
            separator: Some(ThemeColor::var("nosuchcolour")),
            ..UiRoles::default()
        },
    ));
    let issue = resolve_theme("t/unknown", &themes).expect_err("an unknown name is refused");
    assert!(
        issue.message.contains("nosuchcolour"),
        "the finding must name the variable: {issue}"
    );
}

/// A derived theme inherits every role it omits, and its palette override
/// reaches the INHERITED roles too - that is what makes a two-line recolour a
/// complete theme.
#[test]
fn a_derived_theme_inherits_roles_and_recolours_them() {
    let mut themes = base_ui_themes();
    let mut recolour = derived("t/amber", UiRoles::default());
    recolour
        .palette
        .insert("primary".to_string(), "#ffb84a".to_string());
    themes.push(recolour);

    let base = resolve_theme(PHOSPHOR_THEME_ID, &themes).expect("base resolves");
    let child = resolve_theme("t/amber", &themes).expect("the child resolves");
    assert_eq!(
        child.color(UiColor::Primary),
        Color::srgb_u8(0xff, 0xb8, 0x4a)
    );
    assert_ne!(
        child.list_row(RowState::Selected).fill.base,
        base.list_row(RowState::Selected).fill.base,
        "an inherited role is re-resolved against the child's palette"
    );
    assert_eq!(
        child.metric(UiMetric::Radius),
        base.metric(UiMetric::Radius),
        "metrics it does not declare stay the parent's"
    );
}

/// A role a theme DOES declare replaces the inherited one COMPLETELY - every
/// state of it, not the ones it mentioned. A half-inherited role is how a
/// control ends up painted in two looks at once.
#[test]
fn a_redeclared_role_replaces_every_state_of_it() {
    let mut themes = base_ui_themes();
    themes.push(derived(
        "t/rows",
        UiRoles {
            list_row: Some(RowRole {
                normal: row("#101010"),
                hovered: row("#202020"),
                selected: row("#303030"),
            }),
            ..UiRoles::default()
        },
    ));
    let theme = resolve_theme("t/rows", &themes).expect("it resolves");
    assert_eq!(
        theme.list_row(RowState::Normal).fill.base,
        Color::srgb_u8(0x10, 0x10, 0x10)
    );
    assert_eq!(
        theme.list_row(RowState::Hovered).fill.base,
        Color::srgb_u8(0x20, 0x20, 0x20)
    );
    assert_eq!(
        theme.list_row(RowState::Selected).fill.base,
        Color::srgb_u8(0x30, 0x30, 0x30),
        "the declared role's own selected state, not the parent's"
    );
}

/// An inheritance CYCLE is named and refused rather than hung on.
#[test]
fn an_inheritance_cycle_is_refused() {
    let mut a = derived("t/a", UiRoles::default());
    a.inherit = Some("t/b".to_string());
    let mut b = derived("t/b", UiRoles::default());
    b.inherit = Some("t/a".to_string());
    let issue = resolve_theme("t/a", &[a, b]).expect_err("a cycle is refused");
    assert!(
        issue.message.contains("cycle"),
        "the finding must say what it is: {issue}"
    );
}

/// An unknown PARENT is refused, and the finding names the theme that named it
/// rather than the missing id - that is the file an author has to fix.
#[test]
fn an_unknown_parent_is_refused() {
    let mut orphan = derived("t/orphan", UiRoles::default());
    orphan.inherit = Some("nobody/theme".to_string());
    let issue = resolve_theme("t/orphan", &[orphan]).expect_err("an unknown parent is refused");
    assert_eq!(issue.theme, "t/orphan");
    assert!(
        issue.message.contains("nobody/theme"),
        "the finding must name the parent: {issue}"
    );
}

/// A ROOT theme must be complete: an incomplete one is refused at resolve, and
/// the content lint reports the same sentence.
#[test]
fn an_incomplete_root_theme_is_refused_and_linted() {
    let bare = UiThemeConfig {
        id: "t/bare".to_string(),
        name: "Bare".to_string(),
        inherit: None,
        palette: std::collections::BTreeMap::new(),
        metrics: Some(UiMetrics {
            border_width: 1.0,
            radius: 2.0,
            panel_radius: 10.0,
        }),
        roles: UiRoles::default(),
    };
    let registered = vec![bare.clone()];
    let issue = resolve_theme("t/bare", &registered).expect_err("an empty root is refused");
    assert!(
        issue.message.contains("palette"),
        "the first thing missing is the palette: {issue}"
    );
    assert_eq!(
        lint_theme(&bare, &registered),
        vec![issue],
        "the lint reports the sentence the resolver produced"
    );
}

/// Button state precedence is resolved in ONE place, so an observer and the
/// theme reconciler cannot disagree about a pressed-and-selected-and-disabled
/// button.
#[test]
fn button_state_precedence_is_resolved_once() {
    use super::resolve::ButtonState;

    assert_eq!(
        ButtonState::resolve(true, true, true, true),
        ButtonState::Disabled,
        "disabled outranks everything"
    );
    assert_eq!(
        ButtonState::resolve(false, true, true, true),
        ButtonState::SelectedPressed
    );
    assert_eq!(
        ButtonState::resolve(false, true, false, true),
        ButtonState::Pressed
    );
    assert_eq!(
        ButtonState::resolve(false, false, true, true),
        ButtonState::Selected
    );
    assert_eq!(
        ButtonState::resolve(false, false, false, true),
        ButtonState::Hovered
    );
    assert_eq!(
        ButtonState::resolve(false, false, false, false),
        ButtonState::Normal
    );
}

/// BOTH shipped themes resolve every role and every required palette variable.
/// This is the coverage contract: a role added to `UiRoles` that neither base
/// theme declares fails here rather than on somebody's screen.
#[test]
fn both_base_themes_resolve_completely() {
    let themes = base_ui_themes();
    for id in [PHOSPHOR_THEME_ID, HARDWARE_THEME_ID] {
        let theme = resolve_theme(id, &themes).unwrap_or_else(|issue| panic!("{issue}"));
        assert_eq!(theme.id(), id);
        assert!(!theme.name().is_empty(), "{id} needs a picker label");
        for color in UiColor::ALL {
            assert_ne!(
                theme.color(color),
                Color::NONE,
                "{id} leaves {color:?} unpainted"
            );
        }
        assert!(theme.metric(UiMetric::BorderWidth) > 0.0);
    }
}

/// The two shipped looks differ in SHAPE, not only in colour - the blocks/fill
/// meter and the bracketed/chip badge are the authored difference, and a theme
/// that flattened them would be the same look twice.
#[test]
fn the_two_base_themes_differ_in_shape() {
    let themes = base_ui_themes();
    let phosphor = resolve_theme(PHOSPHOR_THEME_ID, &themes).expect("phosphor resolves");
    let hardware = resolve_theme(HARDWARE_THEME_ID, &themes).expect("hardware resolves");
    assert!(
        matches!(phosphor.slider_track().meter, SliderMeter::Blocks { .. }),
        "the terminal draws a block meter"
    );
    assert_eq!(
        hardware.slider_track().meter,
        SliderMeter::Fill,
        "the hardware look draws a solid fill"
    );
    assert!(phosphor.badge().bracketed, "the terminal writes [TAG]");
    assert!(!hardware.badge().bracketed, "the casing draws a chip");
}

/// A mod declaring an existing id RESTYLES it in place; a new id ADDS a look.
/// The Settings picker reads this order, so the shipped default has to stay
/// first in it.
#[test]
fn a_mod_overlays_by_id_and_keeps_the_order() {
    let mut themes = base_ui_themes();
    let base_count = themes.len();
    let mut restyle = base_ui_themes()
        .into_iter()
        .find(|t| t.id == PHOSPHOR_THEME_ID)
        .expect("phosphor ships");
    restyle.name = "Wildcat Phosphor".to_string();
    // What the merge does: overlay in place by id, push a new id.
    match themes.iter_mut().find(|t| t.id == restyle.id) {
        Some(existing) => *existing = restyle,
        None => themes.push(restyle),
    }
    themes.push(derived("wildcat/amber", UiRoles::default()));

    let registry = GameUiThemes(themes);
    assert_eq!(
        registry.listed().len(),
        base_count + 1,
        "a restyle replaces rather than adds"
    );
    assert_eq!(
        registry.listed()[0],
        (PHOSPHOR_THEME_ID, "Wildcat Phosphor"),
        "the restyled default keeps the first row"
    );
}

/// A headless app with the registry wired the way the game wires it.
fn themed_app(themes: Vec<UiThemeConfig>, selected: &str) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, NovaUiPlugin));
    app.insert_resource(GameUiThemes(themes));
    app.insert_resource(SelectedUiTheme(selected.to_string()));
    app.update();
    app
}

/// The selection resolves into the LIVE theme, and moving it moves the live
/// theme in the SAME frame - that is what a widget reconciler runs after.
#[test]
fn moving_the_selection_publishes_the_new_theme() {
    let mut app = themed_app(base_ui_themes(), PHOSPHOR_THEME_ID);
    assert_eq!(
        app.world().resource::<ActiveUiTheme>().id(),
        PHOSPHOR_THEME_ID
    );

    *app.world_mut().resource_mut::<SelectedUiTheme>() =
        SelectedUiTheme(HARDWARE_THEME_ID.to_string());
    app.update();
    assert_eq!(
        app.world().resource::<ActiveUiTheme>().id(),
        HARDWARE_THEME_ID
    );
    assert!(
        app.world().resource::<UiThemeDiagnostic>().0.is_none(),
        "a selection that resolved reports nothing"
    );
}

/// A selection that no longer resolves - a mod turned off since the setting was
/// saved - falls back to `base/phosphor` and SAYS WHY. A player whose theme
/// silently reverted cannot tell a broken mod from a forgotten setting.
#[test]
fn a_selection_that_is_gone_falls_back_and_says_why() {
    let mut themes = base_ui_themes();
    themes.push(derived("wildcat/amber", UiRoles::default()));
    let mut app = themed_app(themes, "wildcat/amber");
    assert_eq!(
        app.world().resource::<ActiveUiTheme>().id(),
        "wildcat/amber"
    );

    // The mod is disabled: the merge republishes the registry without it.
    *app.world_mut().resource_mut::<GameUiThemes>() = GameUiThemes(base_ui_themes());
    app.update();

    assert_eq!(
        app.world().resource::<ActiveUiTheme>().id(),
        PHOSPHOR_THEME_ID,
        "the fallback paints the shipped default"
    );
    let diagnostic = app.world().resource::<UiThemeDiagnostic>();
    let issue = diagnostic.0.as_ref().expect("the player is told why");
    assert_eq!(issue.theme, "wildcat/amber");
    assert_eq!(
        app.world().resource::<SelectedUiTheme>().0,
        "wildcat/amber",
        "the SELECTION is untouched, so re-enabling the mod restores the look"
    );
}

/// A badge role is pure shape and alpha, and it resolves without touching the
/// palette - so a theme may declare one without repeating a colour.
#[test]
fn a_badge_role_is_shape_and_alpha_alone() {
    let mut themes = base_ui_themes();
    themes.push(derived(
        "t/badge",
        UiRoles {
            badge: Some(BadgeRole {
                bracketed: false,
                padding_x: 9.0,
                border_width: 2.0,
                border_alpha: 0.5,
                fill_alpha: 0.25,
            }),
            ..UiRoles::default()
        },
    ));
    let theme = resolve_theme("t/badge", &themes).expect("it resolves");
    let badge = theme.badge();
    assert!(!badge.bracketed);
    assert!((badge.padding_x - 9.0).abs() < f32::EPSILON);
    assert!((badge.fill_alpha - 0.25).abs() < f32::EPSILON);
}

/// The two shipped looks disagree on the SEMANTIC palette, not only on their
/// role tables.
///
/// The screens paint their text and their surfaces through these names, so two
/// themes that share them cannot differ wherever a screen asks by meaning -
/// which is how the hardware look kept drawing green labels, green body copy
/// and a green mods preview under a moulded grey casing. Every semantic name
/// except the four whose meaning is fixed across looks has to move.
#[test]
fn the_two_shipped_looks_do_not_share_one_semantic_palette() {
    let themes = base_ui_themes();
    let phosphor = resolve_theme(PHOSPHOR_THEME_ID, &themes).expect("phosphor resolves");
    let hardware = resolve_theme(HARDWARE_THEME_ID, &themes).expect("hardware resolves");

    // The amber family is the selection in both, red is danger in both, blue is
    // comms in both, and a lamp that is on is green in both. Those are the
    // meaning, not the look.
    let shared = [
        UiColor::Accent,
        UiColor::AccentHigh,
        UiColor::AccentLow,
        UiColor::Danger,
        UiColor::Info,
        UiColor::Nominal,
    ];
    for color in UiColor::ALL {
        let (a, b) = (phosphor.color(color), hardware.color(color));
        if shared.contains(&color) {
            assert_eq!(a, b, "{color:?} carries a meaning both looks keep");
        } else {
            assert_ne!(
                a, b,
                "{color:?} is the same in both looks, so no screen painting \
                 through it can tell them apart"
            );
        }
    }
}

/// `Nominal` is the lit/healthy tone, and it is NOT the primary ink.
///
/// A screen that wanted "good" used to ask for `Primary` because on a phosphor
/// screen the ink happens to be green. On the casing the ink is bone white, and
/// that turned every ONLINE badge and every enabled dependency white.
#[test]
fn the_lit_tone_is_not_the_ink_on_a_look_whose_ink_is_not_green() {
    let themes = base_ui_themes();
    let hardware = resolve_theme(HARDWARE_THEME_ID, &themes).expect("hardware resolves");
    assert_ne!(
        hardware.color(UiColor::Nominal),
        hardware.color(UiColor::Primary),
        "the hardware lamp and the hardware ink must be two colours"
    );

    let phosphor = resolve_theme(PHOSPHOR_THEME_ID, &themes).expect("phosphor resolves");
    assert_eq!(
        phosphor.color(UiColor::Nominal),
        phosphor.color(UiColor::Primary),
        "on a phosphor screen they are the same green, and that is the trap"
    );
}
