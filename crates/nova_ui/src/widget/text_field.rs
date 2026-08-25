//! Single-line text entry for menus and editor forms.

use bevy::{
    ecs::relationship::RelatedSpawner,
    input::{
        keyboard::{Key, KeyboardInput},
        ButtonState,
    },
    picking::{events::Press, hover::Hovered, pointer::PointerButton, prelude::Pointer},
    prelude::*,
    ui_widgets::Button,
};

use super::UiText;
use crate::{skin::UiSkin, theme};

const FONT_SIZE: f32 = 14.0;
const CHARACTER_WIDTH: f32 = 8.4;

/// Mutable text held by a [`TextField`].
#[derive(Component, Clone, Debug, Default, Deref, DerefMut, PartialEq, Eq)]
pub struct TextFieldValue(pub String);

/// Marks a focused text field.
#[derive(Component, Clone, Debug)]
pub struct TextFieldFocused {
    original: String,
    caret: usize,
}

impl TextFieldFocused {
    /// Focus a field holding `value`, caret at the end - what a screen that
    /// opens with its field ready inserts, and what Escape restores to.
    ///
    /// The fields stay private because the caret is a BYTE index into the
    /// value: one set from outside to a position that is not a character
    /// boundary would panic the next edit.
    pub fn at_end(value: &str) -> Self {
        Self {
            original: value.to_string(),
            caret: value.len(),
        }
    }
}

/// Marks invalid text and supplies the message rendered below the field.
#[derive(Component, Clone, Debug, Deref, DerefMut, PartialEq, Eq)]
pub struct TextFieldError(pub String);

/// Sent when Enter or an outside pointer press commits a field.
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct TextFieldSubmitted {
    /// Submitted field entity.
    pub entity: Entity,
    /// Submitted value.
    pub value: String,
}

/// Build settings for a single-line text field.
#[derive(Clone, Debug)]
pub struct TextFieldSpec {
    /// Initial value.
    pub value: String,
    /// Text shown while an unfocused field is empty.
    pub placeholder: String,
    /// Maximum Unicode scalar count.
    pub max_chars: usize,
    /// Draw it SHORT: a field in a column of fields, rather than one control
    /// on a menu. Same face and the same padding either side - the caret is
    /// found by character width, so only the height gives way.
    pub dense: bool,
}

impl TextFieldSpec {
    /// A field with an initial value and a 256-character limit.
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            placeholder: String::new(),
            max_chars: 256,
            dense: false,
        }
    }

    /// Set the empty, unfocused hint.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Set the character limit.
    pub fn max_chars(mut self, max_chars: usize) -> Self {
        self.max_chars = max_chars;
        self
    }

    /// Draw it short, for a panel that is a column of these.
    pub fn dense(mut self) -> Self {
        self.dense = true;
        self
    }
}

/// Marks a themed single-line text entry root.
#[derive(Component)]
pub struct TextField;

#[derive(Component)]
pub(super) struct TextFieldConfig {
    placeholder: String,
    max_chars: usize,
}

#[derive(Component)]
pub(super) struct TextFieldDisplay;

#[derive(Component)]
pub(super) struct TextFieldErrorDisplay;

/// Build a themed single-line text field.
pub fn text_field(spec: TextFieldSpec) -> impl Bundle {
    // A dense field gives up HEIGHT only. The horizontal padding is what
    // `caret_for_pointer` measures a click against, and the face is what
    // `CHARACTER_WIDTH` is derived from, so neither may move.
    let (min_height, vertical_pad) = if spec.dense {
        (px(26), px(2))
    } else {
        (px(36), px(7))
    };
    (
        TextField,
        TextFieldValue(spec.value),
        TextFieldConfig {
            placeholder: spec.placeholder,
            max_chars: spec.max_chars,
        },
        Button,
        Hovered::default(),
        Node {
            width: percent(100),
            min_height,
            align_items: AlignItems::Center,
            padding: UiRect::axes(px(10), vertical_pad),
            border: UiRect::all(px(theme::BORDER_W)),
            border_radius: BorderRadius::all(px(theme::RADIUS)),
            position_type: PositionType::Relative,
            ..default()
        },
        BorderColor::all(theme::PHOSPHOR.with_alpha(0.32)),
        BackgroundColor(theme::PHOSPHOR.with_alpha(0.035)),
        Children::spawn(SpawnWith(|parent: &mut RelatedSpawner<ChildOf>| {
            parent.spawn((
                TextFieldDisplay,
                UiText,
                Text::new(String::new()),
                TextFont {
                    font_size: FontSize::Px(FONT_SIZE),
                    ..default()
                },
                TextColor(theme::PHOSPHOR),
            ));
            parent.spawn((
                TextFieldErrorDisplay,
                UiText,
                Text::new(String::new()),
                TextFont {
                    font_size: FontSize::Px(11.0),
                    ..default()
                },
                TextColor(theme::semantic::THREAT),
                Node {
                    position_type: PositionType::Absolute,
                    top: percent(100),
                    left: px(2),
                    ..default()
                },
                Visibility::Hidden,
            ));
        })),
    )
}

fn character_boundary_before(value: &str, caret: usize) -> usize {
    value[..caret]
        .char_indices()
        .next_back()
        .map_or(0, |(index, _)| index)
}

fn character_boundary_after(value: &str, caret: usize) -> usize {
    value[caret..]
        .char_indices()
        .nth(1)
        .map_or(value.len(), |(index, _)| caret + index)
}

fn caret_for_pointer(
    value: &str,
    node: &ComputedNode,
    transform: &UiGlobalTransform,
    x: f32,
) -> usize {
    let local = transform.try_inverse().map_or(Vec2::ZERO, |inverse| {
        inverse.transform_point2(Vec2::new(x, 0.0))
    });
    let column = ((local.x + node.size().x * 0.5 - 10.0) / CHARACTER_WIDTH)
        .round()
        .max(0.0) as usize;
    value
        .char_indices()
        .map(|(index, _)| index)
        .nth(column)
        .unwrap_or(value.len())
}

pub(super) fn text_field_on_pointer(
    press: On<Pointer<Press>>,
    mut commands: Commands,
    q_parent: Query<&ChildOf>,
    q_fields: Query<(&TextFieldValue, &ComputedNode, &UiGlobalTransform), With<TextField>>,
    q_focused: Query<(Entity, &TextFieldValue), With<TextFieldFocused>>,
    mut submitted: MessageWriter<TextFieldSubmitted>,
) {
    if press.button != PointerButton::Primary {
        return;
    }
    let mut target = Some(press.entity);
    let mut field = None;
    while let Some(entity) = target {
        if q_fields.contains(entity) {
            field = Some(entity);
            break;
        }
        target = q_parent.get(entity).ok().map(ChildOf::parent);
    }

    for (entity, value) in &q_focused {
        if Some(entity) != field {
            commands.entity(entity).remove::<TextFieldFocused>();
            submitted.write(TextFieldSubmitted {
                entity,
                value: value.0.clone(),
            });
        }
    }

    let Some(entity) = field else {
        return;
    };
    let Ok((value, node, transform)) = q_fields.get(entity) else {
        return;
    };
    let caret = caret_for_pointer(&value.0, node, transform, press.pointer_location.position.x);
    commands.entity(entity).insert(TextFieldFocused {
        original: value.0.clone(),
        caret,
    });
}

pub(super) fn text_field_keyboard(
    mut keyboard: MessageReader<KeyboardInput>,
    mut commands: Commands,
    mut q_field: Query<
        (
            Entity,
            &TextFieldConfig,
            &mut TextFieldValue,
            &mut TextFieldFocused,
        ),
        With<TextField>,
    >,
    mut submitted: MessageWriter<TextFieldSubmitted>,
) {
    let Ok((entity, config, mut value, mut focus)) = q_field.single_mut() else {
        return;
    };
    for event in keyboard.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }
        match &event.logical_key {
            Key::Enter => {
                commands.entity(entity).remove::<TextFieldFocused>();
                submitted.write(TextFieldSubmitted {
                    entity,
                    value: value.0.clone(),
                });
            }
            Key::Escape => {
                value.0.clone_from(&focus.original);
                focus.caret = value.0.len();
                commands.entity(entity).remove::<TextFieldFocused>();
            }
            Key::Backspace if focus.caret > 0 => {
                let before = character_boundary_before(&value.0, focus.caret);
                value.0.replace_range(before..focus.caret, "");
                focus.caret = before;
            }
            Key::Delete if focus.caret < value.0.len() => {
                let after = character_boundary_after(&value.0, focus.caret);
                value.0.replace_range(focus.caret..after, "");
            }
            Key::ArrowLeft => {
                focus.caret = character_boundary_before(&value.0, focus.caret);
            }
            Key::ArrowRight => {
                focus.caret = character_boundary_after(&value.0, focus.caret);
            }
            Key::Home => focus.caret = 0,
            Key::End => focus.caret = value.0.len(),
            Key::Space if value.0.chars().count() < config.max_chars => {
                value.0.insert(focus.caret, ' ');
                focus.caret += 1;
            }
            Key::Character(text) => {
                let room = config.max_chars.saturating_sub(value.0.chars().count());
                let inserted: String = text.chars().take(room).collect();
                value.0.insert_str(focus.caret, &inserted);
                focus.caret += inserted.len();
            }
            _ => {}
        }
    }
}

pub(super) fn paint_text_fields(
    skin: Res<UiSkin>,
    mut q_fields: Query<
        (
            Entity,
            &TextFieldValue,
            &TextFieldConfig,
            Option<&TextFieldFocused>,
            Option<&TextFieldError>,
            &Hovered,
            &mut BorderColor,
            &mut BackgroundColor,
            &Children,
        ),
        With<TextField>,
    >,
    mut q_display: Query<
        (
            &mut Text,
            &mut TextColor,
            Option<&TextFieldErrorDisplay>,
            &mut Visibility,
        ),
        Or<(With<TextFieldDisplay>, With<TextFieldErrorDisplay>)>,
    >,
) {
    for (entity, value, config, focus, error, hovered, mut border, mut background, children) in
        &mut q_fields
    {
        let _ = entity;
        let phosphor = if skin.is_phosphor() {
            theme::PHOSPHOR
        } else {
            theme::SCREEN_TEXT
        };
        let border_color = if error.is_some() {
            theme::semantic::THREAT
        } else if focus.is_some() {
            theme::AMBER_NOVA
        } else if hovered.get() {
            phosphor.with_alpha(0.65)
        } else {
            phosphor.with_alpha(0.32)
        };
        border.set_all(border_color);
        *background = if focus.is_some() {
            phosphor.with_alpha(0.08).into()
        } else {
            phosphor.with_alpha(0.035).into()
        };

        for child in children.iter() {
            let Ok((mut text, mut color, error_display, mut visibility)) = q_display.get_mut(child)
            else {
                continue;
            };
            if error_display.is_some() {
                if let Some(error) = error {
                    text.0.clone_from(&error.0);
                    *visibility = Visibility::Inherited;
                } else {
                    text.0.clear();
                    *visibility = Visibility::Hidden;
                }
                continue;
            }
            *visibility = Visibility::Inherited;
            let mut shown = if value.0.is_empty() && focus.is_none() {
                color.0 = phosphor.with_alpha(0.42);
                config.placeholder.clone()
            } else {
                color.0 = phosphor;
                value.0.clone()
            };
            if let Some(focus) = focus {
                shown.insert(focus.caret, '|');
            }
            text.0 = shown;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{skin::UiSkin, widget::fixtures::skin_app};

    fn press(app: &mut App, key_code: KeyCode, logical_key: Key) {
        app.world_mut().write_message(KeyboardInput {
            key_code,
            logical_key,
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
        app.update();
    }

    #[test]
    fn caret_boundaries_follow_unicode_characters() {
        assert_eq!(character_boundary_after("aéz", 1), 3);
        assert_eq!(character_boundary_before("aéz", 3), 1);
    }

    #[test]
    fn focused_field_edits_at_the_caret() {
        let mut app = skin_app(UiSkin::Phosphor);
        app.add_message::<KeyboardInput>();
        let field = app
            .world_mut()
            .spawn((
                text_field(TextFieldSpec::new("13")),
                TextFieldFocused {
                    original: "13".to_string(),
                    caret: 1,
                },
            ))
            .id();

        press(&mut app, KeyCode::Digit2, Key::Character("2".into()));

        assert_eq!(app.world().get::<TextFieldValue>(field).unwrap().0, "123");
    }

    #[test]
    fn escape_restores_the_focus_entry_value() {
        let mut app = skin_app(UiSkin::Phosphor);
        app.add_message::<KeyboardInput>();
        let field = app
            .world_mut()
            .spawn((
                text_field(TextFieldSpec::new("13")),
                TextFieldFocused {
                    original: "13".to_string(),
                    caret: 2,
                },
            ))
            .id();
        press(&mut app, KeyCode::Digit7, Key::Character("7".into()));
        press(&mut app, KeyCode::Escape, Key::Escape);

        let entity = app.world().entity(field);
        assert_eq!(entity.get::<TextFieldValue>().unwrap().0, "13");
        assert!(!entity.contains::<TextFieldFocused>());
    }
}
