//! The New Game setup modal: the name a new open world is saved under and the
//! seed it is generated from.
//!
//! New Game does not leave the menu. It opens this modal over it with a fresh
//! seed drawn from the game's entropy, and only Create starts the world. On
//! the desktop build Create first makes the world's folder under
//! [`WorldsRoot`]; a name that is taken or a folder that cannot be made is
//! refused in the modal and starts nothing. The world then saves itself as
//! it is played, and the Load screen brings it back. The web build saves
//! nothing: the modal says so, and the seed is the whole handle on a world.

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
#[cfg(not(target_arch = "wasm32"))]
use nova_world_base::prelude::{create_world, world_slug, WorldSaveSession};
use rand::Rng as _;

use crate::{
    scenarios::NewGameScenario,
    widgets::{back_button, button, button_variant},
};

/// The widest seed: `u32::MAX` is ten digits.
const SEED_DIGITS: usize = 10;

/// The message under the seed field when what is typed is not a seed.
const SEED_REFUSAL: &str = "Enter a whole number from 0 to 4294967295";

/// The longest name the field takes: where `world_slug` refuses a name.
#[cfg(not(target_arch = "wasm32"))]
const NAME_CHARS: usize = 32;

/// The refusal under the name field when there is no worlds folder.
#[cfg(not(target_arch = "wasm32"))]
const NO_WORLDS_ROOT: &str = "This system has no folder for saved worlds";

/// The folder saved worlds live in, or `None` when this system names none:
/// Create and Load then refuse visibly and start nothing.
///
/// `NovaMenuPlugin` inserts it from [`nova_assets::storage::worlds_root`]
/// unless the app already has one, so a test points it at a scratch folder
/// and never at the player's own worlds.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Resource, Clone, Debug)]
pub(crate) struct WorldsRoot(pub(crate) Option<std::path::PathBuf>);

/// Marker for the modal root.
#[derive(Component)]
pub(crate) struct WorldSetupOverlay;

/// Marker for the world name field. Desktop only: the web build saves no
/// world.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Component)]
pub(crate) struct WorldNameField;

/// Marker for the seed field.
#[derive(Component)]
pub(crate) struct WorldSeedField;

/// Marker for the Create button, greyed while the name or the seed is
/// refused.
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
                            "Ship: Line Warship. World: sparse asteroid clusters, planetoids and \
                             derelicts, generated from the seed and streamed in as you fly. The same seed \
                             gives the same world on the same build.",
                        ),
                        TextFont {
                            font_size: FontSize::Px(13.0),
                            ..default()
                        },
                        TextColor(Color::NONE),
                        ThemedText::new(UiColor::Label),
                    ));
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        parent.spawn((
                            Name::new("World Name Label"),
                            Text::new("World name"),
                            TextFont {
                                font_size: FontSize::Px(13.0),
                                ..default()
                            },
                            TextColor(Color::NONE),
                            ThemedText::new(UiColor::Label),
                        ));
                        parent.spawn((
                            Name::new("World Name Field"),
                            WorldNameField,
                            text_field(TextFieldSpec::new("").max_chars(NAME_CHARS)),
                        ));
                    }
                    #[cfg(target_arch = "wasm32")]
                    parent.spawn((
                        Name::new("Web Saves Note"),
                        Text::new("Saved worlds need the desktop build."),
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

/// Refuse a typed name or seed inline, and grey Create while either stands.
///
/// Reads the fields, not the press: a bad name or seed shows as refused while
/// it is typed, and Create cannot look like it works on it. A name is checked
/// for its form here; a taken name is refused by Create itself, which is the
/// one check that cannot go stale.
pub(crate) fn read_world_setup(
    mut commands: Commands,
    #[cfg(not(target_arch = "wasm32"))] names: Query<
        (Entity, Ref<TextFieldValue>),
        (With<WorldNameField>, Without<WorldSeedField>),
    >,
    seeds: Query<(Entity, Ref<TextFieldValue>), With<WorldSeedField>>,
    buttons: Query<Entity, With<CreateWorldButton>>,
) {
    let Some((seed_field, seed)) = seeds.iter().next() else {
        return;
    };
    #[cfg_attr(
        target_arch = "wasm32",
        expect(unused_mut, reason = "only the native build adds the name check")
    )]
    let mut checks = vec![(
        seed_field,
        seed.is_changed(),
        parse_world_seed(&seed.0)
            .map(|_| ())
            .ok_or_else(|| SEED_REFUSAL.to_string()),
    )];
    #[cfg(not(target_arch = "wasm32"))]
    if let Some((name_field, name)) = names.iter().next() {
        checks.push((
            name_field,
            name.is_changed(),
            world_slug(&name.0)
                .map(|_| ())
                .map_err(|refusal| refusal.to_string()),
        ));
    }
    if !checks.iter().any(|(_, changed, _)| *changed) {
        return;
    }
    for (field, changed, check) in &checks {
        if !changed {
            continue;
        }
        match check {
            Ok(()) => {
                commands.entity(*field).remove::<TextFieldError>();
            }
            Err(refusal) => {
                commands
                    .entity(*field)
                    .insert(TextFieldError(refusal.clone()));
            }
        }
    }
    let valid = checks.iter().all(|(_, _, check)| check.is_ok());
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
/// On the desktop build the world's folder is made first, under the typed
/// name: a taken name, an invalid one, a missing worlds folder or a folder
/// that cannot be made is refused under the name field, and nothing starts.
/// The new folder's session then saves the world from its first frame.
///
/// Clears the Scenarios picker's override, so `start_new_game_scenario` loads
/// the base bundle's declared start. The modal goes with the press, so a
/// second press in the same frame has nothing to land on.
pub(crate) fn on_create_world(
    _activate: On<Activate>,
    mut commands: Commands,
    field: Single<&TextFieldValue, With<WorldSeedField>>,
    #[cfg(not(target_arch = "wasm32"))] name_field: Single<
        (Entity, &TextFieldValue),
        (With<WorldNameField>, Without<WorldSeedField>),
    >,
    #[cfg(not(target_arch = "wasm32"))] root: Res<WorldsRoot>,
    overlays: Query<Entity, With<WorldSetupOverlay>>,
    mut mode: ResMut<GameMode>,
    mut state: ResMut<NextState<GameStates>>,
    mut pick: ResMut<NewGameScenario>,
) {
    let Some(seed) = parse_world_seed(&field.0) else {
        return;
    };
    #[cfg(not(target_arch = "wasm32"))]
    {
        let (name_entity, name) = *name_field;
        let created = match &root.0 {
            Some(root) => create_world(root, &name.0).map_err(|refusal| refusal.to_string()),
            None => Err(NO_WORLDS_ROOT.to_string()),
        };
        let (folder, lock) = match created {
            Ok(created) => created,
            Err(refusal) => {
                commands.entity(name_entity).insert(TextFieldError(refusal));
                return;
            }
        };
        commands.insert_resource(WorldSaveSession::created(
            folder,
            lock,
            name.0.trim().to_string(),
            seed,
        ));
    }
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
