//! The comms panel: the HUD surface for SPEAKER-ATTRIBUTED story text (task
//! 20260716-183220) - the story-campaign vocabulary objectives cannot carry.
//!
//! Data path: a scenario's `StoryMessage` action appends to the event world's
//! story log (nova_scenario), whose sync copies it into [`StoryFeed`] here
//! (write-on-diff). This widget renders the LATEST line as `SPEAKER > text`
//! for a dwell, then hides. Scenario teardown clears the event world, the
//! sync writes an empty feed, and the panel hides - the same reset class as
//! objectives/emphasis (state-diff-aliases-reset), so a leaked comms line
//! cannot survive into the next scenario or the menu.

use bevy::prelude::*;
use nova_ui::theme;

use super::{HudSelfDrivenVisibility, HudTier};

pub mod prelude {
    pub use super::{StoryFeed, StoryLine};
}

/// One speaker-attributed story line, as delivered to the HUD.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoryLine {
    /// Who says it (rendered as the line's prefix, upper-cased by the panel).
    pub speaker: String,
    /// The line itself.
    pub text: String,
}

/// The loaded scenario's story-message log, in delivery order. Written by
/// nova_scenario's event-world sync (append-only within a scenario, emptied
/// on teardown); the comms panel renders the last entry. Lives in
/// nova_gameplay because the HUD cannot depend on nova_scenario (the
/// dependency points the other way) - the same split as `GameObjectives`.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct StoryFeed(pub Vec<StoryLine>);

/// How long the latest line stays up before the panel hides. A new line
/// resets the clock (messages never block; the feed keeps the history).
const COMMS_DWELL_SECS: f32 = 8.0;
/// Panel width: wide enough for a spoken line to wrap comfortably, narrow
/// enough to stay a corner element (the objectives column is 280).
const COMMS_PANEL_WIDTH_PX: f32 = 420.0;
/// Comms line font size (px), matching the objectives' body scale.
const COMMS_FONT_SIZE_PX: f32 = 14.0;

#[derive(Component)]
struct CommsPanelMarker;

#[derive(Component)]
struct CommsTextMarker;

/// Time left before the current line hides; `None` = nothing showing.
#[derive(Resource, Default)]
struct CommsDwell(Option<Timer>);

pub struct CommsPanelPlugin;

impl Plugin for CommsPanelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StoryFeed>();
        app.init_resource::<CommsDwell>();
        app.add_systems(Startup, spawn_comms_panel);
        app.add_systems(
            Update,
            (
                show_latest_line.run_if(resource_changed::<StoryFeed>),
                expire_comms_line,
            )
                .chain()
                .in_set(super::NovaHudSystems),
        );
    }
}

/// The panel: bottom-left corner (the objectives own the right column),
/// hidden until a line arrives. `HudSelfDrivenVisibility`: this widget
/// drives its own `Visibility` (dwell show/hide), so the HUD-level restore
/// must not stomp it; the tier-off enforcement still hides it with the rest
/// of the Chrome tier.
fn spawn_comms_panel(mut commands: Commands) {
    commands
        .spawn((
            Name::new("CommsPanelHUD"),
            CommsPanelMarker,
            HudTier::Chrome,
            HudSelfDrivenVisibility,
            Visibility::Hidden,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(16.0),
                bottom: Val::Px(48.0),
                width: Val::Px(COMMS_PANEL_WIDTH_PX),
                padding: UiRect::all(Val::Px(8.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BorderColor::all(theme::BORDER),
            BackgroundColor(theme::PANEL),
        ))
        .with_children(|parent| {
            parent.spawn((
                CommsTextMarker,
                Text::new(String::new()),
                TextFont::from_font_size(COMMS_FONT_SIZE_PX),
                TextColor(theme::TEXT),
            ));
        });
}

/// A feed change shows its LAST line and rewinds the dwell clock; an emptied
/// feed (scenario teardown) hides immediately.
fn show_latest_line(
    feed: Res<StoryFeed>,
    mut dwell: ResMut<CommsDwell>,
    mut panel: Query<&mut Visibility, With<CommsPanelMarker>>,
    mut text: Query<&mut Text, With<CommsTextMarker>>,
) {
    let Ok(mut visibility) = panel.single_mut() else {
        return;
    };
    match feed.0.last() {
        Some(line) => {
            if let Ok(mut text) = text.single_mut() {
                text.0 = format!("{} > {}", line.speaker.to_uppercase(), line.text);
            }
            *visibility = Visibility::Inherited;
            dwell.0 = Some(Timer::from_seconds(COMMS_DWELL_SECS, TimerMode::Once));
        }
        None => {
            *visibility = Visibility::Hidden;
            dwell.0 = None;
        }
    }
}

/// Tick the dwell and hide the panel when it runs out.
fn expire_comms_line(
    time: Res<Time>,
    mut dwell: ResMut<CommsDwell>,
    mut panel: Query<&mut Visibility, With<CommsPanelMarker>>,
) {
    let Some(timer) = dwell.0.as_mut() else {
        return;
    };
    if timer.tick(time.delta()).just_finished() {
        if let Ok(mut visibility) = panel.single_mut() {
            *visibility = Visibility::Hidden;
        }
        dwell.0 = None;
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use bevy::time::TimeUpdateStrategy;

    use super::*;

    fn comms_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        // Manual clock so the dwell expiry is deterministic.
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            0.5,
        )));
        app.init_resource::<StoryFeed>();
        app.init_resource::<CommsDwell>();
        app.add_systems(Startup, spawn_comms_panel);
        app.add_systems(
            Update,
            (
                show_latest_line.run_if(resource_changed::<StoryFeed>),
                expire_comms_line,
            )
                .chain(),
        );
        app
    }

    fn panel_visibility(app: &mut App) -> Visibility {
        *app.world_mut()
            .query_filtered::<&Visibility, With<CommsPanelMarker>>()
            .single(app.world())
            .expect("the comms panel exists")
    }

    fn panel_text(app: &mut App) -> String {
        app.world_mut()
            .query_filtered::<&Text, With<CommsTextMarker>>()
            .single(app.world())
            .expect("the comms text exists")
            .0
            .clone()
    }

    /// A new line shows speaker-prefixed text; the panel starts hidden
    /// (delivery guard for the visibility assertions below).
    #[test]
    fn latest_line_renders_speaker_prefixed() {
        let mut app = comms_app();
        app.update();
        assert_eq!(
            panel_visibility(&mut app),
            Visibility::Hidden,
            "delivery guard: nothing showing before a line arrives"
        );

        app.world_mut()
            .resource_mut::<StoryFeed>()
            .0
            .push(StoryLine {
                speaker: "Foreman Okono".to_string(),
                text: "Strip it clean, Kestrel.".to_string(),
            });
        app.update();
        assert_eq!(panel_visibility(&mut app), Visibility::Inherited);
        assert_eq!(
            panel_text(&mut app),
            "FOREMAN OKONO > Strip it clean, Kestrel."
        );

        // A second line replaces the first (the panel shows the LATEST).
        app.world_mut()
            .resource_mut::<StoryFeed>()
            .0
            .push(StoryLine {
                speaker: "Vesh".to_string(),
                text: "Bring it to me.".to_string(),
            });
        app.update();
        assert_eq!(panel_text(&mut app), "VESH > Bring it to me.");
    }

    /// The dwell expires the line: visible right up to the dwell, hidden
    /// after it - and a fresh line REWINDS the clock.
    #[test]
    fn dwell_expiry_hides_the_panel() {
        let mut app = comms_app();
        app.update();
        app.world_mut()
            .resource_mut::<StoryFeed>()
            .0
            .push(StoryLine {
                speaker: "Okono".to_string(),
                text: "Quota's quota.".to_string(),
            });
        app.update();
        assert_eq!(panel_visibility(&mut app), Visibility::Inherited);

        // MEASURED clock (probe, this task): in this rig each update
        // advances Time by 0.25s regardless of the 0.5s ManualDuration
        // (virtual-time behavior), first frame 0.0. Assert against the
        // measured rate with wide margins: ~4s in, still up; ~14s in, gone.
        for _ in 0..16 {
            app.update();
        }
        assert_eq!(
            panel_visibility(&mut app),
            Visibility::Inherited,
            "still inside the dwell"
        );
        for _ in 0..40 {
            app.update();
        }
        assert_eq!(
            panel_visibility(&mut app),
            Visibility::Hidden,
            "the dwell expired the line"
        );
    }

    /// An emptied feed (scenario teardown syncs an empty log) hides the
    /// panel immediately - the leaked-line reset pin.
    #[test]
    fn emptied_feed_hides_immediately() {
        let mut app = comms_app();
        app.update();
        app.world_mut()
            .resource_mut::<StoryFeed>()
            .0
            .push(StoryLine {
                speaker: "Okono".to_string(),
                text: "Heads up.".to_string(),
            });
        app.update();
        assert_eq!(panel_visibility(&mut app), Visibility::Inherited);

        app.world_mut().resource_mut::<StoryFeed>().0.clear();
        app.update();
        assert_eq!(
            panel_visibility(&mut app),
            Visibility::Hidden,
            "an emptied feed must hide the panel at once"
        );
    }
}
