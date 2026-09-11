//! The one affordance a cutscene owes the player: a way out, and a way to know
//! there is one.
//!
//! Data path: a scenario's `Cinematic` action starts a skippable scene, and the
//! event-world sync (nova_scenario) writes [`CinematicPrompt`] here. The prompt
//! shows the key while the scene will take it and goes when the scene ends.
//!
//! Deliberately NOT tagged with a `HudTier`. Nothing drops the HUD level for a
//! scene - `HudVisibility` is the PLAYER's grave/tilde toggle, and the menu's -
//! so the reason is the other way round: a skip prompt the player can hide,
//! while the scene it is offering to skip keeps playing, is worse than no
//! prompt. An untagged widget is not HUD-managed (`apply_hud_visibility` filters
//! `With<HudTier>`), so it survives the level and drives its own visibility.
//!
//! The cost of that is real and deliberate: this widget and the title card are
//! the only two HUD surfaces that do not answer the player's own toggle, which
//! is why `wiki/hud.md` has to name them as the exception rather than promise a
//! clean screen.
//!
//! Being outside the HUD's management is also why it has to place itself. The
//! bottom-centre column belongs to the keybind dock; the prompt measures the
//! dock every frame and rides above it, so the two never print through each
//! other (see `keep_the_prompt_clear_of_the_dock`).

use bevy::prelude::*;
use nova_input::prelude::InputBindings;
use nova_ui::{hud::ChipTone, theme};

use super::keybind_dock::prelude::{KeybindDockMarker, DOCK_BOTTOM_PX};

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

/// Bottom-centre, above the comms stack, when the bottom of the screen is
/// otherwise empty.
const PROMPT_BOTTOM_PX: f32 = 18.0;

/// The clearance left between the keybind dock and the prompt riding above it.
const PROMPT_DOCK_GAP_PX: f32 = 10.0;

/// The skip prompt.
pub struct CinematicPromptPlugin;

impl Plugin for CinematicPromptPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CinematicPrompt>();
        app.add_systems(Startup, spawn_cinematic_prompt);
        app.add_systems(
            Update,
            (sync_cinematic_prompt, keep_the_prompt_clear_of_the_dock)
                .in_set(super::NovaHudSystems),
        );
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
                // No authored width: the pill hugs whatever the live binding
                // label turns out to be. A fixed 180 px box with a half-width
                // margin centred one particular label and clipped the rest -
                // a pad glyph name or a rebound `RIGHT BRACKET` overflowed it.
                justify_content: JustifyContent::Center,
                padding: UiRect::axes(Val::Px(10.0), Val::Px(4.0)),
                border: UiRect::all(Val::Px(theme::BORDER_W)),
                border_radius: BorderRadius::all(Val::Px(theme::RADIUS)),
                ..default()
            },
            // Centring the measured box on the screen midline, which is what
            // the hand-computed negative margin was trying to be.
            UiTransform::from_translation(Val2::percent(-50.0, 0.0)),
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
    // Every bound source, not the keyboard column: `cinematic_skip` ships a pad
    // default, and a rebind can empty the keyboard column while leaving the
    // action perfectly reachable. Reading `keyboard` alone told a pad player to
    // press ENTER, and hid the prompt entirely for a keyboardless binding whose
    // skip still fired.
    let label = prompt
        .skip_action
        .as_deref()
        .and_then(|action| bindings.get(action))
        .and_then(|action| action.sources().next())
        .map(|source| source.glyph_label());
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

/// Ride above the keybind dock rather than through it.
///
/// Both surfaces are bottom-centre and the dock is the taller of the two, so a
/// prompt parked on its own floor prints straight over the verb chips. The
/// dock's height is MEASURED, not assumed: the row is as tall as the keycap
/// pictures in it, and a dock that is hidden or has no available verb to show
/// measures zero, which drops the prompt back to the floor.
///
/// One frame stale by construction - it reads the last layout pass. That is
/// invisible here, because control is suspended for the scene the prompt is
/// offering to leave, so the dock's contents are not changing under it.
fn keep_the_prompt_clear_of_the_dock(
    q_dock: Query<(&ComputedNode, &InheritedVisibility), With<KeybindDockMarker>>,
    mut q_row: Query<&mut Node, With<CinematicPromptMarker>>,
) {
    let dock = q_dock
        .iter()
        .filter(|(_, visible)| visible.get())
        .map(|(node, _)| node.size().y * node.inverse_scale_factor)
        .fold(0.0_f32, f32::max);
    let bottom = Val::Px(if dock > 0.0 {
        DOCK_BOTTOM_PX + dock + PROMPT_DOCK_GAP_PX
    } else {
        PROMPT_BOTTOM_PX
    });
    for mut node in &mut q_row {
        if node.bottom != bottom {
            node.bottom = bottom;
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

    /// A dock of a given LOGICAL height, as the last layout pass would have
    /// left it: `ComputedNode` measures in physical pixels and carries the
    /// inverse scale factor that converts them back.
    fn spawn_dock(app: &mut App, logical_height: f32, scale: f32, visible: bool) -> Entity {
        let computed = ComputedNode {
            size: Vec2::new(320.0, logical_height * scale),
            inverse_scale_factor: 1.0 / scale,
            ..Default::default()
        };
        app.world_mut()
            .spawn((
                KeybindDockMarker,
                computed,
                if visible {
                    InheritedVisibility::VISIBLE
                } else {
                    InheritedVisibility::HIDDEN
                },
            ))
            .id()
    }

    fn prompt_bottom(app: &mut App) -> Val {
        app.world_mut()
            .query_filtered::<&Node, With<CinematicPromptMarker>>()
            .single(app.world())
            .expect("the prompt row exists")
            .bottom
    }

    /// The prompt and the keybind dock want the same strip of screen. The
    /// prompt gives way: it sits a clearance above the dock's measured top,
    /// whatever the display scale.
    #[test]
    fn the_prompt_rides_above_the_keybind_dock() {
        let mut app = prompt_app();
        app.add_systems(Update, keep_the_prompt_clear_of_the_dock);
        app.update();
        assert_eq!(
            prompt_bottom(&mut app),
            Val::Px(PROMPT_BOTTOM_PX),
            "no dock, so the prompt keeps its own floor"
        );

        spawn_dock(&mut app, 44.0, 2.0, true);
        app.update();
        assert_eq!(
            prompt_bottom(&mut app),
            Val::Px(DOCK_BOTTOM_PX + 44.0 + PROMPT_DOCK_GAP_PX),
            "the prompt prints through the verb chips"
        );
    }

    /// A dock that is not on screen is not in the way. Bevy lays a hidden node
    /// out anyway, so its size alone would leave the prompt floating over an
    /// empty strip.
    #[test]
    fn a_dock_that_is_not_showing_does_not_push_the_prompt() {
        let mut app = prompt_app();
        app.add_systems(Update, keep_the_prompt_clear_of_the_dock);
        spawn_dock(&mut app, 44.0, 1.0, false);
        app.update();
        assert_eq!(prompt_bottom(&mut app), Val::Px(PROMPT_BOTTOM_PX));
    }

    /// An empty dock measures zero: every verb is `Display::None`, so the row
    /// collapses and the prompt takes the floor back.
    #[test]
    fn an_empty_dock_gives_the_floor_back() {
        let mut app = prompt_app();
        app.add_systems(Update, keep_the_prompt_clear_of_the_dock);
        spawn_dock(&mut app, 0.0, 1.0, true);
        app.update();
        assert_eq!(prompt_bottom(&mut app), Val::Px(PROMPT_BOTTOM_PX));
    }
}
