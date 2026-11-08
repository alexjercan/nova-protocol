//! The icon button: the one small glyph button every close, remove and delete
//! uses.
//!
//! The glyph is muted at rest and lights in the primary ink on hover, on press
//! and on keyboard focus. A [`IconButtonType::Delete`] lights in the danger
//! colour instead, so red on the screen always means data goes. Focus also
//! draws a 1px ring. An [`IconButtonArmed`] button shows lit at rest: its verb
//! is the one a key runs now. Every icon button carries a [`Tooltip`]: a glyph alone
//! does not say what it closes.
//!
//! [`paint_icon_buttons`] is the one paint path. It writes only what changed,
//! so it runs every frame without waking change detection.

use bevy::{
    input_focus::InputFocus,
    picking::hover::Hovered,
    prelude::*,
    ui::{InteractionDisabled, Pressed},
    ui_widgets::Button,
};

use super::{Tooltip, UiText};
use crate::theme::{ActiveUiTheme, UiColor};

/// The close mark: MULTIPLICATION X (U+2715). In the cmap of the UI face,
/// `SGr-IosevkaTerm-Medium.ttf`, and drawn larger than the multiplication sign
/// U+00D7, which sits at x-height and reads as a letter at 16px.
pub const CLOSE_GLYPH: &str = "\u{2715}";

/// The glyph size, in px.
const GLYPH_PX: f32 = 16.0;

/// The hit target's side, in px: the smallest a pointer finds without aiming.
pub const ICON_BUTTON_PX: f32 = 22.0;

/// The fill alpha a lit button takes behind its glyph.
const LIT_FILL: f32 = 0.12;

/// The fill alpha of a press.
const PRESS_FILL: f32 = 0.24;

/// The glyph alpha of a disabled button.
const DISABLED_GLYPH: f32 = 0.4;

/// What an icon button does, which picks its lit colour.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconButtonType {
    /// Puts something away and loses no data: a window close, a dismiss, a
    /// reference taken off a list.
    Dismiss,
    /// Deletes data. Lit in the danger colour.
    Delete,
}

/// The glyph span of an icon button.
#[derive(Component)]
pub struct IconButtonGlyph;

/// Marks an icon button whose verb a key runs now, such as the delete of the
/// marked row that Del deletes. Its glyph shows lit at rest, without the hover
/// fill. Also keeps the glyph readable on a selected row's fill, which the
/// muted rest colour does not.
#[derive(Component)]
pub struct IconButtonArmed;

/// One icon button: a [`ICON_BUTTON_PX`] square with `glyph` centred, its hover
/// hint `tooltip`. Unpainted: [`paint_icon_buttons`] paints it the frame it
/// shows. The caller adds its name, its marker and its `Activate` observer.
pub fn icon_button(kind: IconButtonType, glyph: &str, tooltip: impl Into<String>) -> impl Bundle {
    (
        kind,
        Button,
        Hovered::default(),
        Tooltip(tooltip.into()),
        Node {
            width: px(ICON_BUTTON_PX),
            height: px(ICON_BUTTON_PX),
            flex_shrink: 0.0,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::NONE),
        Outline::new(px(1), Val::ZERO, Color::NONE),
        children![(
            IconButtonGlyph,
            UiText,
            Text::new(glyph.to_string()),
            TextLayout {
                linebreak: LineBreak::NoWrap,
                ..default()
            },
            TextFont {
                font_size: FontSize::Px(GLYPH_PX),
                ..default()
            },
            TextColor(Color::NONE),
            Pickable::IGNORE,
        )],
    )
}

/// The state an icon button paints.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct IconState {
    hovered: bool,
    pressed: bool,
    focused: bool,
    armed: bool,
    disabled: bool,
}

/// The colours of one icon button in one state.
#[derive(Clone, Copy, Debug, PartialEq)]
struct IconPaint {
    glyph: Color,
    fill: Color,
    ring: Color,
}

/// The paint of a `kind` button in `state`.
fn icon_paint(theme: &ActiveUiTheme, kind: IconButtonType, state: IconState) -> IconPaint {
    if state.disabled {
        return IconPaint {
            glyph: theme.color_alpha(UiColor::Label, DISABLED_GLYPH),
            fill: Color::NONE,
            ring: Color::NONE,
        };
    }
    let lit = match kind {
        IconButtonType::Dismiss => UiColor::Primary,
        IconButtonType::Delete => UiColor::Danger,
    };
    let fill = if state.pressed {
        theme.color_alpha(lit, PRESS_FILL)
    } else if state.hovered {
        theme.color_alpha(lit, LIT_FILL)
    } else {
        Color::NONE
    };
    IconPaint {
        glyph: if state.hovered || state.pressed || state.focused || state.armed {
            theme.color(lit)
        } else {
            theme.color(UiColor::Label)
        },
        fill,
        ring: if state.focused {
            theme.color(lit)
        } else {
            Color::NONE
        },
    }
}

/// Paint every icon button from its hover, press, focus, armed and disabled
/// state.
#[expect(
    clippy::type_complexity,
    reason = "one query term per icon button visual state"
)]
pub(super) fn paint_icon_buttons(
    theme: Res<ActiveUiTheme>,
    focus: Option<Res<InputFocus>>,
    mut buttons: Query<(
        Entity,
        &IconButtonType,
        &Hovered,
        Has<Pressed>,
        Has<IconButtonArmed>,
        Has<InteractionDisabled>,
        &Children,
        &mut BackgroundColor,
        &mut Outline,
    )>,
    mut glyphs: Query<&mut TextColor, With<IconButtonGlyph>>,
) {
    let focused = focus.and_then(|focus| focus.get());
    for (entity, kind, hovered, pressed, armed, disabled, children, mut fill, mut outline) in
        &mut buttons
    {
        let state = IconState {
            hovered: hovered.get(),
            pressed,
            focused: focused == Some(entity),
            armed,
            disabled,
        };
        let paint = icon_paint(&theme, *kind, state);
        if fill.0 != paint.fill {
            fill.0 = paint.fill;
        }
        if outline.color != paint.ring {
            outline.color = paint.ring;
        }
        for child in children.iter() {
            if let Ok(mut colour) = glyphs.get_mut(child) {
                if colour.0 != paint.glyph {
                    colour.0 = paint.glyph;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::input_focus::InputFocus;

    use super::*;
    use crate::{
        theme::PHOSPHOR_THEME_ID,
        widget::fixtures::{phosphor, themed_app},
    };

    fn glyph(app: &App, button: Entity) -> Color {
        let children = app.world().entity(button).get::<Children>().unwrap();
        let child = children
            .iter()
            .find(|child| app.world().entity(*child).contains::<IconButtonGlyph>())
            .expect("an icon button holds its glyph");
        app.world().entity(child).get::<TextColor>().unwrap().0
    }

    fn ring(app: &App, button: Entity) -> Color {
        app.world().entity(button).get::<Outline>().unwrap().color
    }

    #[test]
    fn an_icon_button_rests_muted_and_lights_primary_on_hover_and_focus() {
        let mut app = themed_app(PHOSPHOR_THEME_ID);
        let theme = phosphor();
        let close = app
            .world_mut()
            .spawn(icon_button(IconButtonType::Dismiss, CLOSE_GLYPH, "Close"))
            .id();
        app.update();
        assert_eq!(glyph(&app, close), theme.color(UiColor::Label));
        assert_eq!(ring(&app, close), Color::NONE);

        app.world_mut().entity_mut(close).insert(Hovered(true));
        app.update();
        assert_eq!(glyph(&app, close), theme.color(UiColor::Primary));

        app.world_mut().entity_mut(close).insert(Hovered(false));
        app.world_mut()
            .insert_resource(InputFocus::from_entity(close));
        app.update();
        assert_eq!(glyph(&app, close), theme.color(UiColor::Primary));
        assert_eq!(
            ring(&app, close),
            theme.color(UiColor::Primary),
            "keyboard focus draws a ring"
        );
    }

    #[test]
    fn only_a_delete_icon_button_lights_in_the_danger_colour() {
        let mut app = themed_app(PHOSPHOR_THEME_ID);
        let theme = phosphor();
        let close = app
            .world_mut()
            .spawn(icon_button(IconButtonType::Dismiss, CLOSE_GLYPH, "Close"))
            .id();
        let delete = app
            .world_mut()
            .spawn(icon_button(IconButtonType::Delete, CLOSE_GLYPH, "Delete"))
            .id();
        app.update();
        assert_eq!(
            glyph(&app, delete),
            theme.color(UiColor::Label),
            "a delete rests muted like any other"
        );

        for button in [close, delete] {
            app.world_mut().entity_mut(button).insert(Hovered(true));
        }
        app.update();
        assert_eq!(glyph(&app, delete), theme.color(UiColor::Danger));
        assert_eq!(glyph(&app, close), theme.color(UiColor::Primary));
        assert_ne!(theme.color(UiColor::Danger), theme.color(UiColor::Primary));
    }

    #[test]
    fn an_armed_delete_shows_danger_at_rest_without_the_hover_fill() {
        let mut app = themed_app(PHOSPHOR_THEME_ID);
        let theme = phosphor();
        let delete = app
            .world_mut()
            .spawn((
                icon_button(IconButtonType::Delete, CLOSE_GLYPH, "Delete"),
                IconButtonArmed,
            ))
            .id();
        app.update();
        assert_eq!(glyph(&app, delete), theme.color(UiColor::Danger));
        assert_eq!(
            app.world()
                .entity(delete)
                .get::<BackgroundColor>()
                .unwrap()
                .0,
            Color::NONE
        );
        assert_eq!(ring(&app, delete), Color::NONE, "armed is not focus");

        app.world_mut()
            .entity_mut(delete)
            .remove::<IconButtonArmed>();
        app.update();
        assert_eq!(glyph(&app, delete), theme.color(UiColor::Label));
    }
}
