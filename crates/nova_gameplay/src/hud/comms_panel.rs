//! The comms panel: the HUD surface for SPEAKER-ATTRIBUTED story text (task
//! 20260716-183220) - the story-campaign vocabulary objectives cannot carry.
//!
//! Data path: a scenario's `StoryMessage` action appends to the event world's
//! story log (nova_scenario), whose sync copies it into [`StoryFeed`] here
//! (write-on-diff). Since the pacing rework (task 20260717-163033) the panel
//! runs a display QUEUE over that feed instead of latest-wins: lines show in
//! ARRIVAL order with a fade, each holds the screen for its dwell
//! (`COMMS_DWELL_SECS` default, per-line override clamped to
//! [`COMMS_DWELL_MIN_SECS`]..[`COMMS_DWELL_MAX_SECS`]) but yields early to a
//! waiting line after `COMMS_MIN_SECS` - so a two-line beat reads as two
//! beats and a mid-fight line can no longer destroy an unread one. The
//! pending queue is capped at `COMMS_QUEUE_CAP` (drop-oldest): a stale
//! backlog must not narrate the previous fight; the full log stays in
//! [`StoryFeed`] regardless. Each line SHOWS with a comms blip
//! (`UiSfx::CommsLine`).
//!
//! Scenario teardown clears the event world, the sync writes an empty feed,
//! and the panel resets instantly - queue dropped, fades cancelled, hidden -
//! the same reset class as objectives/emphasis (state-diff-aliases-reset),
//! so a leaked comms line cannot survive into the next scenario or the menu.

use std::collections::VecDeque;

use bevy::prelude::*;
use bevy_common_systems::prelude::{SfxCommandsExt, SoundBank, Tween, TweenOnComplete};
use nova_ui::theme;

use super::{HudSelfDrivenVisibility, HudTier};
use crate::audio::UiSfx;

pub mod prelude {
    pub use super::{StoryFeed, StoryLine, COMMS_DWELL_MAX_SECS, COMMS_DWELL_MIN_SECS};
}

/// One speaker-attributed story line, as delivered to the HUD.
#[derive(Clone, Debug, PartialEq)]
pub struct StoryLine {
    /// Who says it (rendered as the line's prefix, upper-cased by the panel).
    pub speaker: String,
    /// The line itself.
    pub text: String,
    /// Authored on-screen hold override (seconds); `None` = the default
    /// dwell. Clamped by the panel to the documented range at use.
    pub dwell: Option<f32>,
}

/// The loaded scenario's story-message log, in delivery order. Written by
/// nova_scenario's event-world sync (append-only within a scenario, emptied
/// on teardown); the comms panel displays it through the paced queue. Lives
/// in nova_gameplay because the HUD cannot depend on nova_scenario (the
/// dependency points the other way) - the same split as `GameObjectives`.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct StoryFeed(pub Vec<StoryLine>);

/// Default on-screen hold when nothing waits behind the line.
const COMMS_DWELL_SECS: f32 = 8.0;
/// The floor a showing line holds even with lines waiting: readable, but a
/// burst still flows.
const COMMS_MIN_SECS: f32 = 4.0;
/// Authored per-line dwell clamp (documented author-facing; pub so
/// content_lint warns against the same numbers it clamps to).
pub const COMMS_DWELL_MIN_SECS: f32 = 3.0;
pub const COMMS_DWELL_MAX_SECS: f32 = 30.0;
/// Pending lines beyond this drop OLDEST-first.
const COMMS_QUEUE_CAP: usize = 4;
/// Fade timings (s): quick in, gentler out.
const COMMS_FADE_IN_SECS: f32 = 0.25;
const COMMS_FADE_OUT_SECS: f32 = 0.4;
/// Comms blip volume, under the objective cues (0.30/0.38) - chatter, not
/// a milestone.
const COMMS_BLIP_VOLUME: f32 = 0.22;
/// Panel width: wide enough for a spoken line to wrap comfortably, narrow
/// enough to stay a corner element (the objectives column is 280).
const COMMS_PANEL_WIDTH_PX: f32 = 420.0;
/// Comms line font size (px), matching the objectives' body scale.
const COMMS_FONT_SIZE_PX: f32 = 14.0;

#[derive(Component)]
struct CommsPanelMarker;

#[derive(Component)]
struct CommsTextMarker;

/// The paced display queue between [`StoryFeed`] (the log) and the panel
/// (one line at a time).
#[derive(Resource, Default)]
struct CommsQueue {
    /// Feed entries consumed so far (the feed is append-only in-scenario).
    seen: usize,
    /// Lines waiting their turn, oldest first.
    pending: VecDeque<StoryLine>,
}

/// What the panel is doing right now.
#[derive(Resource, Default)]
enum CommsDisplay {
    #[default]
    Idle,
    /// A line is up (fade-in runs visually underneath); the timer is its
    /// clamped dwell.
    Showing { dwell: Timer },
    /// Fading out; when the tween finishes (removes itself) the next line
    /// shows or the panel hides.
    FadingOut,
}

pub struct CommsPanelPlugin;

impl Plugin for CommsPanelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StoryFeed>();
        app.init_resource::<CommsQueue>();
        app.init_resource::<CommsDisplay>();
        app.add_systems(Startup, spawn_comms_panel);
        app.add_systems(
            Update,
            (
                enqueue_new_lines.run_if(resource_changed::<StoryFeed>),
                advance_comms_display,
            )
                .chain()
                .in_set(super::NovaHudSystems),
        );
        // The fade maps the panel's tween value onto its colors; ordered
        // after the tween advances (bcs TweenPlugin is registered by the
        // gameplay plugin).
        app.add_systems(
            Update,
            apply_comms_fade.after(bevy_common_systems::prelude::TweenSystems::Advance),
        );
    }
}

/// The panel: bottom-left corner (the objectives own the right column),
/// hidden until a line arrives. `HudSelfDrivenVisibility`: this widget
/// drives its own `Visibility` (queue show/hide), so the HUD-level restore
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

/// Feed changes drive the queue: new entries enqueue (capped drop-oldest);
/// an EMPTIED feed (scenario teardown) resets everything instantly - the
/// leaked-line pin.
fn enqueue_new_lines(
    feed: Res<StoryFeed>,
    mut queue: ResMut<CommsQueue>,
    mut display: ResMut<CommsDisplay>,
    mut commands: Commands,
    mut panel: Query<(Entity, &mut Visibility), With<CommsPanelMarker>>,
) {
    if feed.0.len() < queue.seen {
        // Teardown (the feed is append-only in-scenario, so shrinking means
        // reset): drop the queue, cancel any fade, hide at once.
        queue.seen = 0;
        queue.pending.clear();
        *display = CommsDisplay::Idle;
        if let Ok((entity, mut visibility)) = panel.single_mut() {
            commands.entity(entity).remove::<Tween<f32>>();
            *visibility = Visibility::Hidden;
        }
    }
    let seen = queue.seen;
    for line in feed.0.iter().skip(seen) {
        queue.pending.push_back(line.clone());
    }
    queue.seen = feed.0.len();
    while queue.pending.len() > COMMS_QUEUE_CAP {
        // Oldest pending drops first: better to lose stale backlog than the
        // line that just fired (the log keeps everything).
        queue.pending.pop_front();
    }
}

/// The display state machine: show the next pending line (blip + fade-in +
/// dwell), yield a held line early when something waits, fade out, repeat.
fn advance_comms_display(
    time: Res<Time>,
    mut queue: ResMut<CommsQueue>,
    mut display: ResMut<CommsDisplay>,
    mut commands: Commands,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut panel: Query<
        (
            Entity,
            &mut Visibility,
            Option<&Tween<f32>>,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        With<CommsPanelMarker>,
    >,
    mut text: Query<(&mut Text, &mut TextColor), With<CommsTextMarker>>,
) {
    let Ok((entity, mut visibility, tween, mut background, mut border)) = panel.single_mut() else {
        return;
    };
    match &mut *display {
        CommsDisplay::Idle => {
            let Some(line) = queue.pending.pop_front() else {
                return;
            };
            if let Ok((mut text, mut color)) = text.single_mut() {
                text.0 = format!("{} > {}", line.speaker.to_uppercase(), line.text);
                // Start the frame INVISIBLE: visibility flips now but the
                // command-inserted tween only advances next frame, and a
                // one-frame full-alpha flash reads as a flicker (R1.4).
                color.0 = theme::TEXT.with_alpha(0.0);
            }
            background.0 = theme::PANEL.with_alpha(0.0);
            *border = BorderColor::all(theme::BORDER.with_alpha(0.0));
            *visibility = Visibility::Inherited;
            // Keep, not Remove: the completed fade-in stays applied at
            // exactly 1.0 (Remove flushes before the apply system ever
            // sees the end value - R1.4); the fade-out's insert overwrites
            // it, and its ABSENCE after Remove is FadingOut's edge.
            commands.entity(entity).insert(
                Tween::<f32>::new(0.0, 1.0, COMMS_FADE_IN_SECS, EaseFunction::QuadraticOut)
                    .with_on_complete(TweenOnComplete::Keep),
            );
            if let Some(bank) = &bank {
                commands.play_sfx_volume(bank.get(UiSfx::CommsLine), COMMS_BLIP_VOLUME);
            }
            let dwell = line
                .dwell
                .map(|secs| secs.clamp(COMMS_DWELL_MIN_SECS, COMMS_DWELL_MAX_SECS))
                .unwrap_or(COMMS_DWELL_SECS);
            *display = CommsDisplay::Showing {
                dwell: Timer::from_seconds(dwell, TimerMode::Once),
            };
        }
        CommsDisplay::Showing { dwell } => {
            dwell.tick(time.delta());
            let yields = !queue.pending.is_empty() && dwell.elapsed_secs() >= COMMS_MIN_SECS;
            if dwell.is_finished() || yields {
                commands.entity(entity).insert(
                    Tween::<f32>::new(1.0, 0.0, COMMS_FADE_OUT_SECS, EaseFunction::QuadraticIn)
                        .with_on_complete(TweenOnComplete::Remove),
                );
                *display = CommsDisplay::FadingOut;
            }
        }
        CommsDisplay::FadingOut => {
            // The fade tween removes itself on completion; its absence is
            // the transition edge.
            if tween.is_none() {
                if queue.pending.is_empty() {
                    *visibility = Visibility::Hidden;
                }
                *display = CommsDisplay::Idle;
            }
        }
    }
}

/// Map the panel's active fade tween onto its colors (text, border,
/// background) each frame. Base colors come from the theme so the fade
/// composes with an already-translucent panel.
fn apply_comms_fade(
    panel: Query<(&Tween<f32>, &Children), With<CommsPanelMarker>>,
    mut writers: ParamSet<(
        Query<(&mut BackgroundColor, &mut BorderColor), With<CommsPanelMarker>>,
        Query<&mut TextColor, With<CommsTextMarker>>,
    )>,
) {
    let Ok((tween, _children)) = panel.single() else {
        return;
    };
    let alpha = tween.value().clamp(0.0, 1.0);
    if let Ok((mut background, mut border)) = writers.p0().single_mut() {
        background.0 = theme::PANEL.with_alpha(theme::PANEL.alpha() * alpha);
        *border = BorderColor::all(theme::BORDER.with_alpha(theme::BORDER.alpha() * alpha));
    }
    if let Ok(mut text) = writers.p1().single_mut() {
        text.0 = theme::TEXT.with_alpha(theme::TEXT.alpha() * alpha);
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use bevy::time::TimeUpdateStrategy;
    use bevy_common_systems::prelude::TweenPlugin;

    use super::*;

    fn comms_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(TweenPlugin);
        // Manual clock so dwell/yield edges are deterministic. MEASURED
        // (manual-time-rig lesson): each update advances virtual time by
        // 0.25s here (max_delta clamp), first frame 0.0.
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            0.5,
        )));
        app.init_resource::<StoryFeed>();
        app.init_resource::<CommsQueue>();
        app.init_resource::<CommsDisplay>();
        app.add_systems(Startup, spawn_comms_panel);
        app.add_systems(
            Update,
            (
                enqueue_new_lines.run_if(resource_changed::<StoryFeed>),
                advance_comms_display,
            )
                .chain(),
        );
        app
    }

    fn push_line(app: &mut App, speaker: &str, text: &str, dwell: Option<f32>) {
        app.world_mut()
            .resource_mut::<StoryFeed>()
            .0
            .push(StoryLine {
                speaker: speaker.to_string(),
                text: text.to_string(),
                dwell,
            });
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

    /// The pacing rework's fail-first: a two-line burst shows the FIRST
    /// line first (the old latest-wins panel showed the second and the
    /// first was never visible), then flows to the second after the yield
    /// floor. 0.25s/update measured clock: 4s = 16 updates.
    #[test]
    fn a_burst_shows_lines_in_arrival_order() {
        let mut app = comms_app();
        app.update();
        push_line(&mut app, "Okono", "First.", None);
        push_line(&mut app, "Okono", "Second.", None);
        app.update();
        app.update();
        assert_eq!(
            panel_text(&mut app),
            "OKONO > First.",
            "arrival order: the burst's FIRST line shows first"
        );

        // Past the 4s yield floor (+ fade), the second line takes over.
        for _ in 0..24 {
            app.update();
        }
        assert_eq!(
            panel_text(&mut app),
            "OKONO > Second.",
            "the held line yields to the pending one after the floor"
        );
        assert_eq!(panel_visibility(&mut app), Visibility::Inherited);
    }

    /// A solo line holds the FULL default dwell (no early yield with an
    /// empty queue): still up past the yield floor, gone after the dwell
    /// plus fade.
    #[test]
    fn a_solo_line_holds_the_full_dwell() {
        let mut app = comms_app();
        app.update();
        push_line(&mut app, "Okono", "Take your time.", None);
        app.update();
        app.update();
        assert_eq!(panel_visibility(&mut app), Visibility::Inherited);

        // ~5s in (20 updates at 0.25s): past the yield floor, inside the
        // 8s dwell.
        for _ in 0..20 {
            app.update();
        }
        assert_eq!(
            panel_visibility(&mut app),
            Visibility::Inherited,
            "no pending line: the yield floor must not hide a solo line"
        );
        // ~14s total: dwell + fade long gone.
        for _ in 0..36 {
            app.update();
        }
        assert_eq!(panel_visibility(&mut app), Visibility::Hidden);
    }

    /// The authored per-line dwell is respected and clamped: a 3s-clamped
    /// line (authored 0.5) expires well before the default would.
    #[test]
    fn per_line_dwell_is_clamped_and_respected() {
        let mut app = comms_app();
        app.update();
        push_line(&mut app, "Okono", "Blink and gone.", Some(0.5));
        app.update();
        app.update();
        assert_eq!(panel_visibility(&mut app), Visibility::Inherited);

        // Authored 0.5 clamps to 3.0; by ~5s (fade included) it is gone -
        // while the default dwell would still be showing.
        for _ in 0..20 {
            app.update();
        }
        assert_eq!(
            panel_visibility(&mut app),
            Visibility::Hidden,
            "the clamped short dwell expired the line early"
        );
    }

    /// Pending lines beyond the cap drop OLDEST first: after a 6-line
    /// dump, the first displayed line is the one showing, and the queue
    /// kept only the newest four of the rest.
    #[test]
    fn the_pending_queue_drops_oldest_past_the_cap() {
        let mut app = comms_app();
        app.update();
        for i in 0..6 {
            push_line(&mut app, "Okono", &format!("Line {i}."), None);
        }
        app.update();
        app.update();
        // The whole dump enqueues in one frame, the cap trims to 4 BEFORE
        // the first pop (lines 0-1, the oldest, drop), then line 2 shows.
        assert_eq!(panel_text(&mut app), "OKONO > Line 2.");
        let pending: Vec<String> = app
            .world()
            .resource::<CommsQueue>()
            .pending
            .iter()
            .map(|l| l.text.clone())
            .collect();
        assert_eq!(
            pending,
            vec!["Line 3.", "Line 4.", "Line 5."],
            "drop-oldest keeps the newest lines of a one-frame dump"
        );
    }

    /// An emptied feed (scenario teardown syncs an empty log) resets the
    /// whole pipeline immediately - the leaked-line pin, queue edition.
    #[test]
    fn emptied_feed_resets_immediately() {
        let mut app = comms_app();
        app.update();
        push_line(&mut app, "Okono", "Heads up.", None);
        push_line(&mut app, "Okono", "Backlog.", None);
        app.update();
        assert_eq!(panel_visibility(&mut app), Visibility::Inherited);

        app.world_mut().resource_mut::<StoryFeed>().0.clear();
        app.update();
        assert_eq!(
            panel_visibility(&mut app),
            Visibility::Hidden,
            "an emptied feed must hide the panel at once"
        );
        assert!(
            app.world().resource::<CommsQueue>().pending.is_empty(),
            "teardown drops the pending backlog too"
        );
    }
}
