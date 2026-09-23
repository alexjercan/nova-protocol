//! The New Game setup modal: the seed a new open world is generated from.
//!
//! New Game does not leave the menu. It opens this modal over it with a fresh
//! seed drawn from the game's entropy, and only Create starts the world. The
//! seed is the whole handle on the world: a world a player liked is a number
//! they can write down and type back in. Nothing here is saved. A Retry in
//! play keeps the session's seed, and the next New Game draws a new one.

use bevy::{
    prelude::*,
    ui::InteractionDisabled,
    ui_widgets::{observe, Activate},
};
use bevy_rand::prelude::*;
use nova_gameplay::prelude::*;
use nova_ui::{
    prelude::REPORT_Z,
    theme,
    theme::UiColor,
    widget::{
        panel, text_field, ButtonVariant, TextFieldError, TextFieldSpec, TextFieldValue,
        ThemedRadius, ThemedText,
    },
};
use nova_world_base::prelude::OpenWorldSession;
use rand::Rng as _;

use crate::{
    scenarios::NewGameScenario,
    widgets::{back_button, button, button_variant},
};

/// The widest seed: `u32::MAX` is ten digits.
const SEED_DIGITS: usize = 10;

/// The message under the seed field when what is typed is not a seed.
const SEED_REFUSAL: &str = "Enter a whole number from 0 to 4294967295";

/// Marker for the modal root.
#[derive(Component)]
pub(crate) struct WorldSetupOverlay;

/// Marker for the seed field.
#[derive(Component)]
pub(crate) struct WorldSeedField;

/// Marker for the Create button, greyed while the seed does not parse.
#[derive(Component)]
pub(crate) struct CreateWorldButton;

/// The seed `text` spells: decimal digits only, inside `u32`. Blank, signed,
/// and overflowing text is not a seed.
pub(crate) fn parse_world_seed(text: &str) -> Option<u32> {
    let text = text.trim();
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

/// Open the modal with a fresh seed.
pub(crate) fn on_new_game(
    _activate: On<Activate>,
    mut commands: Commands,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
) {
    let seed = rng.next_u32().to_string();
    commands
        .spawn((
            WorldSetupOverlay,
            DespawnOnExit(GameStates::MainMenu),
            Name::new("New Game Overlay"),
            // A modal blocker: the menu behind it takes no click while the
            // player is choosing a world.
            Pickable {
                should_block_lower: true,
                is_hoverable: false,
            },
            // ABSOLUTE for the same reason as the safe-mode report: the menu's
            // hidden screens still take layout space, and a modal in the flow
            // lays out below the viewport.
            Node {
                position_type: PositionType::Absolute,
                top: px(0),
                left: px(0),
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            GlobalZIndex(REPORT_Z),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Name::new("New Game Panel"),
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Stretch,
                        width: px(420),
                        padding: UiRect::all(px(20)),
                        row_gap: px(8),
                        border: UiRect::all(px(theme::BORDER_W)),
                        ..default()
                    },
                    ThemedRadius::control(),
                    panel(),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Name::new("New Game Title"),
                        Text::new("New Game"),
                        TextFont {
                            font_size: FontSize::Px(24.0),
                            ..default()
                        },
                        TextColor(Color::NONE),
                        ThemedText::new(UiColor::Body),
                    ));
                    parent.spawn((
                        Name::new("New Game Profile"),
                        Text::new(
                            "Ship: Line Warship. World: asteroid belts, planetoids and derelicts, \
                             generated from the seed and streamed in as you fly. The same seed \
                             gives the same world on the same build.",
                        ),
                        TextFont {
                            font_size: FontSize::Px(13.0),
                            ..default()
                        },
                        TextColor(Color::NONE),
                        ThemedText::new(UiColor::Label),
                    ));
                    parent.spawn((
                        Name::new("World Seed Label"),
                        Text::new("World seed"),
                        TextFont {
                            font_size: FontSize::Px(13.0),
                            ..default()
                        },
                        TextColor(Color::NONE),
                        ThemedText::new(UiColor::Label),
                    ));
                    parent.spawn((
                        Name::new("World Seed Field"),
                        WorldSeedField,
                        text_field(TextFieldSpec::new(seed).max_chars(SEED_DIGITS)),
                    ));
                    parent.spawn((
                        Name::new("Randomize Seed Button"),
                        button("Randomize"),
                        observe(on_randomize_seed),
                    ));
                    parent.spawn((
                        Name::new("Create World Button"),
                        CreateWorldButton,
                        button_variant("Create", ButtonVariant::Primary, None),
                        observe(on_create_world),
                    ));
                    parent.spawn((
                        Name::new("Cancel New Game Button"),
                        back_button("Cancel"),
                        observe(on_cancel_new_game),
                    ));
                });
        });
}

/// Refuse a typed seed inline, and grey Create while it stands.
///
/// Reads the field, not the press: a bad seed shows as refused while it is
/// typed, and Create cannot look like it works on it.
pub(crate) fn read_world_seed(
    mut commands: Commands,
    fields: Query<(Entity, &TextFieldValue), (With<WorldSeedField>, Changed<TextFieldValue>)>,
    buttons: Query<Entity, With<CreateWorldButton>>,
) {
    let Some((field, value)) = fields.iter().next() else {
        return;
    };
    let valid = parse_world_seed(&value.0).is_some();
    if valid {
        commands.entity(field).remove::<TextFieldError>();
    } else {
        commands
            .entity(field)
            .insert(TextFieldError(SEED_REFUSAL.to_string()));
    }
    for button in &buttons {
        if valid {
            commands.entity(button).remove::<InteractionDisabled>();
        } else {
            commands.entity(button).insert(InteractionDisabled);
        }
    }
}

/// Write a fresh seed into the field.
pub(crate) fn on_randomize_seed(
    _activate: On<Activate>,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
    mut fields: Query<&mut TextFieldValue, With<WorldSeedField>>,
) {
    for mut value in &mut fields {
        value.0 = rng.next_u32().to_string();
    }
}

/// Start the open world on the typed seed.
///
/// Clears the Scenarios picker's override, so `start_new_game_scenario` loads
/// the base bundle's declared start. The modal goes with the press, so a
/// second press in the same frame has nothing to land on.
pub(crate) fn on_create_world(
    _activate: On<Activate>,
    mut commands: Commands,
    field: Single<&TextFieldValue, With<WorldSeedField>>,
    overlays: Query<Entity, With<WorldSetupOverlay>>,
    mut mode: ResMut<GameMode>,
    mut state: ResMut<NextState<GameStates>>,
    mut pick: ResMut<NewGameScenario>,
) {
    let Some(seed) = parse_world_seed(&field.0) else {
        return;
    };
    commands.insert_resource(OpenWorldSession { seed });
    pick.0 = None;
    *mode = GameMode::NewGame;
    state.set(GameStates::Playing);
    for overlay in &overlays {
        commands.entity(overlay).despawn();
    }
}

/// Close the modal and nothing else.
pub(crate) fn on_cancel_new_game(
    _activate: On<Activate>,
    mut commands: Commands,
    overlays: Query<Entity, With<WorldSetupOverlay>>,
) {
    for overlay in &overlays {
        commands.entity(overlay).despawn();
    }
}
