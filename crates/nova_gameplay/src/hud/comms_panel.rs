//! The comms panel: the HUD surface for SPEAKER-ATTRIBUTED story text (task
//! 20260716-183220) - the story-campaign vocabulary objectives cannot carry.
//!
//! Data path: a scenario's `StoryMessage` action appends to the event world's
//! story log (nova_scenario), whose sync copies it into [`StoryFeed`] here
//! (write-on-diff). Since task 20260721-211526 the panel presents that feed as
//! a bottom-left chat stack: several lines can be visible at once, newest at
//! the bottom, older cards pushed up and fading. Per-line dwell still defaults
//! to [`COMMS_DWELL_SECS`] and clamps to
//! [`COMMS_DWELL_MIN_SECS`]..[`COMMS_DWELL_MAX_SECS`]. Pending overflow drops
//! oldest, but the full transcript stays in [`StoryFeed`] for the NOVA OS log.
//!
//! Scenario teardown clears the event world, the sync writes an empty feed,
//! and the panel resets instantly - queue dropped, fades cancelled, hidden -
//! the same reset class as objectives/emphasis (state-diff-aliases-reset),
//! so a leaked comms line cannot survive into the next scenario or the menu.

use std::collections::VecDeque;

use bevy::prelude::*;
use bevy_common_systems::prelude::{SfxCommandsExt, SoundBank};
use nova_ui::theme;

use super::{HudSelfDrivenVisibility, HudTier};
use crate::{asset_ref::AssetRef, audio::UiSfx};

/// Glob-import surface: `use nova_gameplay::hud::comms_panel::prelude::*` re-exports the public API of this module.
pub mod prelude {
    pub use super::{
        StoryFeed, StoryLine, COMMS_DWELL_MAX_SECS, COMMS_DWELL_MIN_SECS, COMMS_DWELL_SECS,
        COMMS_FADE_OUT_SECS, COMMS_MIN_SECS,
    };
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
    /// Optional speaker icon image. `None` renders the HUD fallback tile.
    pub icon: Option<AssetRef<Image>>,
}

/// The loaded scenario's story-message log, in delivery order. Written by
/// nova_scenario's event-world sync (append-only within a scenario, emptied
/// on teardown); the comms panel displays it through the paced queue. Lives
/// in nova_gameplay because the HUD cannot depend on nova_scenario (the
/// dependency points the other way) - the same split as `GameObjectives`.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct StoryFeed(pub Vec<StoryLine>);

/// Default on-screen hold when nothing waits behind the line. `pub` because the
/// scenario pacing layer (nova_assets `pacing.rs`) derives the beat gap between
/// a conversation line and the objective it introduces from this value - the
/// objective must post as the line finishes, not before it (task
/// 20260722-142341).
pub const COMMS_DWELL_SECS: f32 = 8.0;
/// The floor a showing line holds even with lines waiting: readable, but a
/// burst still flows. `pub` because the scenario pacing layer derives its
/// mid-read INSTRUCTION gap from it - an instructional objective posts once the
/// reader has had this floor with the coaching line (task 20260722-163718).
pub const COMMS_MIN_SECS: f32 = 4.0;
/// Authored per-line dwell clamp (documented author-facing; pub so
/// content_lint warns against the same numbers it clamps to).
pub const COMMS_DWELL_MIN_SECS: f32 = 3.0;
/// Upper clamp on an authored per-line comms dwell, in seconds.
pub const COMMS_DWELL_MAX_SECS: f32 = 30.0;
/// Pending lines beyond this drop OLDEST-first.
const COMMS_QUEUE_CAP: usize = 4;
/// Visible cards in the bottom-left stack.
const COMMS_VISIBLE_CAP: usize = 3;
/// Fade timings (s): quick in, gentler out. `COMMS_FADE_OUT_SECS` is `pub` so
/// the scenario pacing layer can wait out the fade tail as well as the dwell
/// before posting the next objective (task 20260722-142341).
const COMMS_FADE_IN_SECS: f32 = 0.25;
/// Fade-out duration (s) after a line's dwell elapses; the pacing layer adds
/// this to the dwell so the objective posts as the line clears, not mid-fade.
pub const COMMS_FADE_OUT_SECS: f32 = 0.4;
/// Comms blip volume, under the objective cues (0.30/0.38) - chatter, not
/// a milestone.
const COMMS_BLIP_VOLUME: f32 = 0.22;
/// Panel width: wide enough for a spoken line to wrap comfortably, narrow
/// enough to stay a corner element (the objectives column is 280).
const COMMS_PANEL_WIDTH_PX: f32 = 420.0;
/// Square speaker icon size inside a comms card.
const COMMS_ICON_SIZE_PX: f32 = 30.0;
/// Comms line font size (px), matching the objectives' body scale.
const COMMS_FONT_SIZE_PX: f32 = 14.0;

#[derive(Component)]
struct CommsPanelMarker;

#[derive(Component)]
struct CommsTextMarker;

#[derive(Component)]
struct CommsCardMarker;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
struct CommsIconMarker {
    kind: CommsIconKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CommsIconKind {
    Authored,
    Fallback,
}

#[derive(Clone, Debug)]
struct VisibleCommsLine {
    line: StoryLine,
    age_secs: f32,
}

impl VisibleCommsLine {
    fn dwell_secs(&self) -> f32 {
        self.line
            .dwell
            .map(|secs| secs.clamp(COMMS_DWELL_MIN_SECS, COMMS_DWELL_MAX_SECS))
            .unwrap_or(COMMS_DWELL_SECS)
    }

    fn alpha(&self) -> f32 {
        if self.age_secs < COMMS_FADE_IN_SECS {
            return (self.age_secs / COMMS_FADE_IN_SECS).clamp(0.0, 1.0);
        }
        let fade_start = self.dwell_secs();
        if self.age_secs <= fade_start {
            return 1.0;
        }
        (1.0 - (self.age_secs - fade_start) / COMMS_FADE_OUT_SECS).clamp(0.0, 1.0)
    }

    fn expired(&self) -> bool {
        self.age_secs >= self.dwell_secs() + COMMS_FADE_OUT_SECS
    }
}

/// The display queue between [`StoryFeed`] (the log) and the visible stack.
#[derive(Resource, Default)]
struct CommsQueue {
    /// Feed entries consumed so far (the feed is append-only in-scenario).
    seen: usize,
    /// Lines waiting their turn, oldest first.
    pending: VecDeque<StoryLine>,
    /// Lines currently rendered, oldest first.
    visible: VecDeque<VisibleCommsLine>,
}

/// Drives the comms panel: the paced display queue over [`StoryFeed`] that
/// shows speaker-attributed story lines as a bottom-left stack.
pub struct CommsPanelPlugin;

impl Plugin for CommsPanelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StoryFeed>();
        app.init_resource::<CommsQueue>();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.add_systems(Startup, spawn_comms_panel);
        app.add_systems(
            Update,
            (
                enqueue_new_lines.run_if(resource_changed::<StoryFeed>),
                drive_comms_stack,
                sync_comms_cards,
            )
                .chain()
                .in_set(super::NovaHudSystems),
        );
    }
}

/// The panel: bottom-left corner (the objectives own the right column),
/// hidden until a line arrives. `HudSelfDrivenVisibility`: this widget
/// drives its own `Visibility` (queue show/hide), so the HUD-level restore
/// must not stomp it; the tier-off enforcement still hides it with the rest
/// of the Chrome tier.
fn spawn_comms_panel(mut commands: Commands) {
    commands.spawn((
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
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            ..default()
        },
    ));
}

/// Feed changes drive the queue: new entries enqueue (capped drop-oldest);
/// an EMPTIED feed (scenario teardown) resets everything instantly - the
/// leaked-line pin.
fn enqueue_new_lines(
    feed: Res<StoryFeed>,
    mut queue: ResMut<CommsQueue>,
    mut commands: Commands,
    mut panel: Query<(Entity, &mut Visibility), With<CommsPanelMarker>>,
) {
    if feed.0.len() < queue.seen {
        // Teardown (the feed is append-only in-scenario, so shrinking means
        // reset): drop the queue and visible stack, then hide at once.
        queue.seen = 0;
        queue.pending.clear();
        queue.visible.clear();
        if let Ok((entity, mut visibility)) = panel.single_mut() {
            commands.entity(entity).despawn_related::<Children>();
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

/// Tick visible cards, apply controls, and promote pending lines into open
/// visible slots.
fn drive_comms_stack(
    time: Res<Time>,
    mut queue: ResMut<CommsQueue>,
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    panel: Query<Entity, With<CommsPanelMarker>>,
) {
    if panel.single().is_err() {
        return;
    };

    for visible in &mut queue.visible {
        visible.age_secs += time.delta_secs();
    }
    queue.visible.retain(|visible| !visible.expired());

    if keys.just_pressed(KeyCode::KeyV) {
        queue.visible.pop_front();
    }
    if keys.just_pressed(KeyCode::KeyB)
        && !queue.pending.is_empty()
        && queue.visible.len() >= COMMS_VISIBLE_CAP
    {
        queue.visible.pop_front();
    }

    while queue.visible.len() < COMMS_VISIBLE_CAP {
        let Some(line) = queue.pending.pop_front() else {
            break;
        };
        queue.visible.push_back(VisibleCommsLine {
            line,
            age_secs: 0.0,
        });
        if let Some(bank) = &bank {
            commands.play_sfx_volume(bank.get(UiSfx::CommsLine), COMMS_BLIP_VOLUME);
        }
    }
}

fn sync_comms_cards(
    queue: Res<CommsQueue>,
    asset_server: Option<Res<AssetServer>>,
    mut commands: Commands,
    mut panel: Query<(Entity, &mut Visibility), With<CommsPanelMarker>>,
) {
    let Ok((entity, mut visibility)) = panel.single_mut() else {
        return;
    };
    commands.entity(entity).despawn_related::<Children>();
    if queue.visible.is_empty() {
        *visibility = Visibility::Hidden;
        return;
    }
    *visibility = Visibility::Inherited;
    commands.entity(entity).with_children(|parent| {
        for visible in &queue.visible {
            parent.spawn(comms_card(visible, asset_server.as_deref()));
        }
    });
}

fn comms_card(line: &VisibleCommsLine, asset_server: Option<&AssetServer>) -> impl Bundle {
    let alpha = line.alpha();
    (
        CommsCardMarker,
        Node {
            width: Val::Percent(100.0),
            min_height: Val::Px(46.0),
            padding: UiRect::all(Val::Px(8.0)),
            border: UiRect::all(Val::Px(1.0)),
            column_gap: Val::Px(8.0),
            align_items: AlignItems::FlexStart,
            ..default()
        },
        BorderColor::all(theme::PHOSPHOR_MUTED.with_alpha(theme::PHOSPHOR_MUTED.alpha() * alpha)),
        BackgroundColor(theme::SCREEN_0.with_alpha(theme::SCREEN_0.alpha() * alpha)),
        children![
            comms_icon(&line.line, alpha, asset_server),
            (
                Node {
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(2.0),
                    ..default()
                },
                children![(
                    CommsTextMarker,
                    Text::new(format!(
                        "{} > {}",
                        line.line.speaker.to_uppercase(),
                        line.line.text
                    )),
                    TextFont::from_font_size(COMMS_FONT_SIZE_PX),
                    TextColor(theme::SCREEN_TEXT.with_alpha(theme::SCREEN_TEXT.alpha() * alpha)),
                    TextLayout {
                        linebreak: LineBreak::WordBoundary,
                        ..default()
                    },
                )]
            )
        ],
    )
}

fn comms_icon(line: &StoryLine, alpha: f32, asset_server: Option<&AssetServer>) -> impl Bundle {
    let node = Node {
        width: Val::Px(COMMS_ICON_SIZE_PX),
        height: Val::Px(COMMS_ICON_SIZE_PX),
        min_width: Val::Px(COMMS_ICON_SIZE_PX),
        border: UiRect::all(Val::Px(1.0)),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
    };
    match &line.icon {
        Some(icon) => (
            CommsIconMarker {
                kind: CommsIconKind::Authored,
            },
            node,
            ImageNode::new(
                asset_server
                    .map(|server| icon.resolve(server))
                    .unwrap_or_default(),
            )
            .with_color(Color::WHITE.with_alpha(alpha)),
            BorderColor::all(theme::BLUE.with_alpha(alpha)),
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
            children![],
        ),
        None => (
            CommsIconMarker {
                kind: CommsIconKind::Fallback,
            },
            node,
            ImageNode::default(),
            BorderColor::all(theme::PHOSPHOR_MUTED.with_alpha(alpha)),
            BackgroundColor(theme::BLUE.with_alpha(0.18 * alpha)),
            children![],
        ),
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
        // Manual clock so dwell/yield edges are deterministic. MEASURED
        // (manual-time-rig lesson): each update advances virtual time by
        // 0.25s here (max_delta clamp), first frame 0.0.
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            0.5,
        )));
        app.init_resource::<StoryFeed>();
        app.init_resource::<CommsQueue>();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.add_systems(Startup, spawn_comms_panel);
        app.add_systems(
            Update,
            (
                enqueue_new_lines.run_if(resource_changed::<StoryFeed>),
                drive_comms_stack,
                sync_comms_cards,
            )
                .chain(),
        );
        app
    }

    fn push_line(app: &mut App, speaker: &str, text: &str, dwell: Option<f32>) {
        push_line_with_icon(app, speaker, text, dwell, None);
    }

    fn push_line_with_icon(
        app: &mut App,
        speaker: &str,
        text: &str,
        dwell: Option<f32>,
        icon: Option<AssetRef<Image>>,
    ) {
        app.world_mut()
            .resource_mut::<StoryFeed>()
            .0
            .push(StoryLine {
                speaker: speaker.to_string(),
                text: text.to_string(),
                dwell,
                icon,
            });
    }

    fn panel_visibility(app: &mut App) -> Visibility {
        *app.world_mut()
            .query_filtered::<&Visibility, With<CommsPanelMarker>>()
            .single(app.world())
            .expect("the comms panel exists")
    }

    fn visible_texts(app: &mut App) -> Vec<String> {
        app.world_mut()
            .query_filtered::<&Text, With<CommsTextMarker>>()
            .iter(app.world())
            .map(|text| text.0.clone())
            .collect()
    }

    fn press_key(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(key);
    }

    #[test]
    fn a_burst_stacks_visible_lines_newest_at_bottom() {
        let mut app = comms_app();
        app.update();
        push_line(&mut app, "Okono", "First.", None);
        push_line(&mut app, "Vesh", "Second.", None);
        push_line(&mut app, "Relay", "Third.", None);
        app.update();

        assert_eq!(
            visible_texts(&mut app),
            vec![
                "OKONO > First.".to_string(),
                "VESH > Second.".to_string(),
                "RELAY > Third.".to_string(),
            ],
            "child order is top-to-bottom, so newest is the bottom card"
        );
        assert_eq!(panel_visibility(&mut app), Visibility::Inherited);
    }

    #[test]
    fn dismiss_hides_a_visible_line_without_touching_the_log() {
        let mut app = comms_app();
        app.update();
        push_line(&mut app, "Okono", "First.", None);
        push_line(&mut app, "Vesh", "Second.", None);
        app.update();

        press_key(&mut app, KeyCode::KeyV);
        assert_eq!(
            visible_texts(&mut app),
            vec!["VESH > Second.".to_string()],
            "dismiss removes the oldest visible card"
        );
        assert_eq!(
            app.world().resource::<StoryFeed>().0.len(),
            2,
            "dismiss is visual only; the transcript remains complete"
        );
    }

    #[test]
    fn skip_promotes_pending_lines_into_the_stack() {
        let mut app = comms_app();
        app.update();
        for i in 0..6 {
            push_line(&mut app, "Okono", &format!("Line {i}."), None);
        }
        app.update();
        assert_eq!(
            visible_texts(&mut app).len(),
            COMMS_VISIBLE_CAP,
            "the initial burst fills only the visible stack"
        );

        press_key(&mut app, KeyCode::KeyB);
        let texts = visible_texts(&mut app);
        assert_eq!(texts.len(), COMMS_VISIBLE_CAP);
        assert_eq!(
            texts.last().map(String::as_str),
            Some("OKONO > Line 5."),
            "skip advances the next queued card to the bottom immediately"
        );
    }

    #[test]
    fn speaker_icons_use_authored_refs_and_fallback() {
        let mut app = comms_app();
        app.update();
        push_line_with_icon(
            &mut app,
            "Okono",
            "Face.",
            None,
            Some(AssetRef::from("icons/okono.png")),
        );
        push_line(&mut app, "Unknown", "Fallback.", None);
        app.update();

        let icons: Vec<CommsIconKind> = app
            .world_mut()
            .query_filtered::<&CommsIconMarker, With<Node>>()
            .iter(app.world())
            .map(|marker| marker.kind)
            .collect();
        assert_eq!(
            icons,
            vec![CommsIconKind::Authored, CommsIconKind::Fallback],
            "authored icon refs render as images; missing refs render a fallback tile"
        );
    }

    /// The pacing rework's fail-first still holds under the stack: a two-line
    /// burst keeps arrival order instead of latest-wins overwriting the first
    /// line.
    #[test]
    fn a_burst_shows_lines_in_arrival_order() {
        let mut app = comms_app();
        app.update();
        push_line(&mut app, "Okono", "First.", None);
        push_line(&mut app, "Okono", "Second.", None);
        app.update();
        app.update();
        assert_eq!(
            visible_texts(&mut app),
            vec!["OKONO > First.".to_string(), "OKONO > Second.".to_string(),],
            "arrival order: the burst's FIRST line shows first"
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
        // The whole dump enqueues in one frame, the pending cap trims to 4
        // BEFORE visible promotion (lines 0-1, the oldest, drop), then the
        // stack shows lines 2-4 and keeps line 5 pending.
        assert_eq!(
            visible_texts(&mut app),
            vec![
                "OKONO > Line 2.".to_string(),
                "OKONO > Line 3.".to_string(),
                "OKONO > Line 4.".to_string(),
            ],
        );
        let pending: Vec<String> = app
            .world()
            .resource::<CommsQueue>()
            .pending
            .iter()
            .map(|l| l.text.clone())
            .collect();
        assert_eq!(
            pending,
            vec!["Line 5."],
            "drop-oldest keeps the newest lines of a one-frame dump"
        );
    }

    /// An emptied feed (scenario teardown syncs an empty log) resets the
    /// whole pipeline immediately - the leaked-line pin, queue edition.
    #[test]
    fn emptied_feed_resets_the_comms_stack_immediately() {
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
        assert!(
            app.world().resource::<CommsQueue>().visible.is_empty(),
            "teardown drops visible cards too"
        );
    }
}
