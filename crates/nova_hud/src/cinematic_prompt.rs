//! The one affordance a cutscene owes the player: a way out, and a way to know
//! there is one.
//!
//! Data path: a scenario's `Cinematic` action starts a skippable scene, and the
//! event-world sync (nova_scenario) writes [`CinematicPrompt`] here. The prompt
//! shows the key while the scene will take it and goes when the scene ends.
//!
//! Deliberately NOT tagged with a `HudTier`. A scene almost always drops the
//! HUD to its cinematic level, and a skip prompt hidden by the very thing it
//! is offering to skip is worse than no prompt: an untagged widget is not
//! HUD-managed, so it survives the level and drives its own visibility.

use bevy::prelude::*;
use nova_input::prelude::InputBindings;
use nova_ui::{hud::ChipTone, theme};

/// The `CinematicPrompt` resource.
pub mod prelude {
    pub use super::CinematicPrompt;
}

/// Whether a scene the player may leave is playing right now, and the action
/// whose key leaves it.
///
/// Written every frame by nova_scenario's event-world sync. Lives here rather
/// than in the scenario crate because the HUD cannot depend on nova_scenario -
/// the same split `StoryFeed` and `GameObjectives` make.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct CinematicPrompt {
    /// The input action that skips, or `None` when nothing skippable is
    /// playing.
    pub skip_action: Option<String>,
}

/// Marker for the prompt row.
#[derive(Component, Debug)]
struct CinematicPromptMarker;

/// Marker for the text inside it.
#[derive(Component, Debug)]
struct CinematicPromptText;

/// Bottom-centre, above the comms stack.
const PROMPT_BOTTOM_PX: f32 = 18.0;

/// The skip prompt.
pub struct CinematicPromptPlugin;

impl Plugin for CinematicPromptPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CinematicPrompt>();
        app.add_systems(Startup, spawn_cinematic_prompt);
        app.add_systems(Update, sync_cinematic_prompt.in_set(super::NovaHudSystems));
    }
}

fn spawn_cinematic_prompt(mut commands: Commands) {
    let tone = ChipTone::Amber;
    commands
        .spawn((
            Name::new("CinematicSkipPrompt"),
            CinematicPromptMarker,
            Visibility::Hidden,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(PROMPT_BOTTOM_PX),
                left: Val::Percent(50.0),
                margin: UiRect::left(Val::Px(-90.0)),
                width: Val::Px(180.0),
                justify_content: JustifyContent::Center,
                padding: UiRect::axes(Val::Px(10.0), Val::Px(4.0)),
                border: UiRect::all(Val::Px(theme::BORDER_W)),
                border_radius: BorderRadius::all(Val::Px(theme::RADIUS)),
                ..default()
            },
            BorderColor::all(tone.border()),
            BackgroundColor(tone.fill()),
        ))
        .with_children(|prompt| {
            prompt.spawn((
                CinematicPromptText,
                Text::new(""),
                TextFont::from_font_size(12.0),
                TextColor(tone.text()),
            ));
        });
}

/// Show the prompt with the live key while a skippable scene runs.
///
/// The key is read from the registry every frame rather than snapshotted: a
/// player who rebinds Advance mid-run must not be told the old key.
fn sync_cinematic_prompt(
    prompt: Res<CinematicPrompt>,
    bindings: Res<InputBindings>,
    mut q_row: Query<&mut Visibility, With<CinematicPromptMarker>>,
    mut q_text: Query<&mut Text, With<CinematicPromptText>>,
) {
    let label = prompt
        .skip_action
        .as_deref()
        .and_then(|action| bindings.get(action))
        .and_then(|action| action.keyboard.first())
        .map(|source| source.label());
    for mut visibility in &mut q_row {
        *visibility = match label {
            Some(_) => Visibility::Inherited,
            None => Visibility::Hidden,
        };
    }
    let Some(label) = label else {
        return;
    };
    for mut text in &mut q_text {
        let wanted = format!("{}  SKIP SCENE", label.to_uppercase());
        if text.0 != wanted {
            text.0 = wanted;
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::input::keyboard::KeyCode;
    use nova_input::prelude::{ActionBinding, BindingSpec, InputSource};

    use super::*;

    /// The scenario's own binding, as `scenario_bindings` registers it: the
    /// skip FOLLOWS Advance, so the prompt has to print whatever Advance is
    /// bound to right now.
    fn prompt_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<CinematicPrompt>();
        app.insert_resource(InputBindings::from_actions([
            ActionBinding::new("scenario_advance", "SCENARIO", "Advance")
                .keyboard([InputSource::Keyboard(KeyCode::Enter)]),
            ActionBinding::new("cinematic_skip", "SCENARIO", "Skip Scene")
                .follows("scenario_advance")
                .keyboard([InputSource::Keyboard(KeyCode::Enter)]),
        ]));
        app.add_systems(Startup, spawn_cinematic_prompt);
        app.add_systems(Update, sync_cinematic_prompt);
        app
    }

    fn offer(app: &mut App, action: Option<&str>) {
        app.world_mut()
            .resource_mut::<CinematicPrompt>()
            .skip_action = action.map(str::to_string);
        app.update();
    }

    fn prompt_visibility(app: &mut App) -> Visibility {
        *app.world_mut()
            .query_filtered::<&Visibility, With<CinematicPromptMarker>>()
            .single(app.world())
            .expect("the prompt row exists")
    }

    fn prompt_text(app: &mut App) -> String {
        app.world_mut()
            .query_filtered::<&Text, With<CinematicPromptText>>()
            .single(app.world())
            .expect("the prompt has text")
            .0
            .clone()
    }

    /// The prompt is up only while a scene is offering the skip, and names the
    /// key that takes it.
    #[test]
    fn the_prompt_follows_the_offer() {
        let mut app = prompt_app();
        app.update();
        assert_eq!(
            prompt_visibility(&mut app),
            Visibility::Hidden,
            "nothing is playing, so nothing is offered"
        );

        offer(&mut app, Some("cinematic_skip"));
        assert_eq!(prompt_visibility(&mut app), Visibility::Inherited);
        assert!(
            prompt_text(&mut app).contains("SKIP SCENE"),
            "the prompt says what the key does: {}",
            prompt_text(&mut app)
        );

        offer(&mut app, None);
        assert_eq!(
            prompt_visibility(&mut app),
            Visibility::Hidden,
            "the offer goes with the scene"
        );
    }

    /// The key is read from the registry every frame. A player who rebinds
    /// Advance mid-run must not be told the old key - and because the skip
    /// FOLLOWS Advance, rebinding Advance is the only way it moves.
    #[test]
    fn the_prompt_prints_the_live_binding_not_a_snapshot() {
        let mut app = prompt_app();
        app.update();
        offer(&mut app, Some("cinematic_skip"));
        let before = prompt_text(&mut app);

        app.world_mut().resource_mut::<InputBindings>().rebind(
            "scenario_advance",
            BindingSpec {
                keyboard: vec![InputSource::Keyboard(KeyCode::Space)],
                gamepad: vec![],
            },
        );
        app.update();

        let after = prompt_text(&mut app);
        assert_ne!(before, after, "the prompt still names the old key");
        assert!(after.contains("SKIP SCENE"));
    }

    /// An action the registry does not know is not a key the player has. The
    /// prompt stays down rather than offering a blank one.
    #[test]
    fn an_unregistered_action_offers_nothing() {
        let mut app = prompt_app();
        app.update();
        offer(&mut app, Some("no_such_action"));
        assert_eq!(prompt_visibility(&mut app), Visibility::Hidden);
    }
}
