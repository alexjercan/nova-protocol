//! The comms panel: the HUD surface for SPEAKER-ATTRIBUTED story text - the
//! story-campaign vocabulary objectives cannot carry.
//!
//! Data path: a scenario's `NarrativeCue` action appends to the event world's
//! story log (nova_scenario), whose sync copies it into [`StoryFeed`] here
//! (write-on-diff). The panel presents that feed as a bottom-left chat stack:
//! several lines can be visible at once, newest at the bottom, older cards
//! pushed up and fading. A line holds for [`COMMS_DWELL_SECS`] whatever else
//! is waiting - an authored per-line dwell overrides that and clamps to
//! [`COMMS_DWELL_MIN_SECS`]..[`COMMS_DWELL_MAX_SECS`] - so a burst is paced by
//! how fast the SCENARIO posts, not by the panel taking reading time back. A
//! bounded visible window paces bursts, and a backlog bounded by
//! [`COMMS_PENDING_CAP`] holds
//! what is waiting: past that the OLDEST waiting cue goes, because a line the
//! panel could only reach half a minute of dialogue later is answering a beat
//! the player has already flown past. The story log keeps every line either
//! way.
//!
//! Scenario teardown clears the event world, the sync writes an empty feed,
//! and the panel resets instantly - queue dropped, fades cancelled, hidden -
//! the same reset class as objectives/emphasis (state-diff-aliases-reset),
//! so a leaked comms line cannot survive into the next scenario or the menu.

use std::collections::VecDeque;

use bevy::prelude::*;
use nova_gameplay::{
    asset_ref::AssetRef,
    audio::{AudioRoute, SfxCommandsExt, SoundBank, UiSfx},
};
use nova_ui::hud::{body_colour, CHIP_BORDER_ALPHA, CHIP_FILL};

use super::{HudSelfDrivenVisibility, HudTier};

/// The `StoryFeed` queue, `StoryLine`, and the comms dwell and fade timing
/// constants. The accent a line carries is `nova_gameplay`'s, because both
/// ends of the sync need it.
pub mod prelude {
    pub use super::{
        StoryFeed, StoryLine, COMMS_DWELL_MAX_SECS, COMMS_DWELL_MIN_SECS, COMMS_DWELL_SECS,
        COMMS_FADE_OUT_SECS,
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
    /// The colour this card is drawn in: its border, its speaker line, its
    /// fallback icon, and - lifted toward white - the words the player reads.
    /// Resolved by the scenario sync, so the panel has no catalog to consult
    /// and no unknown-id branch to get wrong.
    pub accent: Color,
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
/// objective must post as the line finishes, not before it.
pub const COMMS_DWELL_SECS: f32 = 8.0;
/// Authored per-line dwell clamp (documented author-facing; pub so
/// content_lint warns against the same numbers it clamps to).
pub const COMMS_DWELL_MIN_SECS: f32 = 3.0;
/// Upper clamp on an authored per-line comms dwell, in seconds.
pub const COMMS_DWELL_MAX_SECS: f32 = 30.0;
/// Visible cards in the bottom-left stack.
const COMMS_VISIBLE_CAP: usize = 3;
/// How many lines may WAIT their turn before the oldest is dropped.
///
/// Derived from what this panel can put on screen, not picked by taste:
///
/// - A slot frees no sooner than `COMMS_DWELL_MIN_SECS + COMMS_FADE_OUT_SECS`
///   = 3.0 + 0.4 = 3.4 s. That is the shortest life a showing line can have -
///   an authored dwell clamped to its floor, plus the fade `expired` waits out
///   - so it is also the fastest a waiting line can be let in.
/// - [`COMMS_VISIBLE_CAP`] = 3 of those slots run at once, so the panel retires
///   at most three lines per 3.4 s.
/// - [`COMMS_DWELL_MAX_SECS`] = 30 s is the longest the panel will hold ONE
///   line, and therefore the longest it claims a line is still worth reading.
///   A cue that has waited longer than that is answering a beat the player has
///   already flown past.
///
/// 3 slots x floor(30.0 / 3.4) = 3 x 8 = 24. The last line of a full backlog
/// comes up after 8 x 3.4 = 27.2 s, inside the window; a twenty-fifth would
/// come up outside it, which is why it is dropped rather than queued.
const COMMS_PENDING_CAP: usize = 24;
/// Fade timings (s): quick in, gentler out. `COMMS_FADE_OUT_SECS` is `pub` so
/// the scenario pacing layer can wait out the fade tail as well as the dwell
/// before posting the next objective.
const COMMS_FADE_IN_SECS: f32 = 0.25;
/// Fade-out duration (s) after a line's dwell elapses; the pacing layer adds
/// this to the dwell so the objective posts as the line clears, not mid-fade.
pub const COMMS_FADE_OUT_SECS: f32 = 0.4;
/// Comms blip volume, under the objective cues (0.30/0.38) - chatter, not
/// a milestone.
const COMMS_BLIP_VOLUME: f32 = 0.22;
/// Screen-relative panel width. The pixel ceiling keeps ultrawide displays
/// from turning a transmission into one long subtitle line.
const COMMS_PANEL_WIDTH_PERCENT: f32 = 48.0;
const COMMS_PANEL_MAX_WIDTH_PX: f32 = 960.0;
/// Square speaker icon size inside a comms card.
const COMMS_ICON_SIZE_PX: f32 = 48.0;
/// How much of the accent washes the FALLBACK icon tile. Faint enough that the
/// tile reads as an empty frame waiting for a portrait, not as a second chip.
const ICON_FILL_ALPHA: f32 = 0.18;
/// Speaker header and message body sizes.
const COMMS_SPEAKER_FONT_SIZE_PX: f32 = 14.0;
const COMMS_BODY_FONT_SIZE_PX: f32 = 20.0;

#[derive(Component)]
struct CommsPanelMarker;

#[derive(Component)]
struct CommsTextMarker;

#[derive(Component)]
struct CommsSpeakerMarker;

#[derive(Component)]
struct CommsCardMarker;

/// Which showing line a card - and every node inside it - draws.
///
/// The card set is reconciled against this id rather than rebuilt, so a line
/// that is still on screen keeps its entities, its taffy nodes and its shaped
/// text for as long as it is showing. Two identical lines are still two ids.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
struct CommsLineId(u64);

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
    /// Identity of this showing line, matched by [`CommsLineId`] on its card.
    id: u64,
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
    /// The next [`CommsLineId`]. Never reused, so a despawned card's id cannot
    /// be mistaken for a live one.
    next_id: u64,
    /// Lines waiting their turn, oldest first. Bounded by
    /// [`COMMS_PENDING_CAP`].
    pending: VecDeque<StoryLine>,
    /// Lines currently rendered, oldest first.
    visible: VecDeque<VisibleCommsLine>,
    /// Whether the overflow episode in progress has already been reported. A
    /// burst that outruns the panel is ONE line in the log, not one per cue it
    /// costs: a scenario that posts faster than the panel can read does it by
    /// the dozen, and the signal worth having is that it happened at all.
    /// Cleared by [`CommsQueue::settle_backlog`] when the backlog is inside its
    /// cap again, so a later burst is reported as the new episode it is.
    warned_overflow: bool,
}

impl CommsQueue {
    /// Drop the oldest waiting lines until the backlog fits
    /// [`COMMS_PENDING_CAP`], reporting the episode once.
    ///
    /// The OLDEST go. What is waiting at the back of an overrun queue is the
    /// scene the player is in; what is at the front was posted half a minute of
    /// dialogue ago and answers a beat that has already ended. Dropping from
    /// the front keeps the panel current at the cost of history the story log
    /// itself still holds.
    fn trim_backlog(&mut self) {
        let Some(over) = self.pending.len().checked_sub(COMMS_PENDING_CAP) else {
            return;
        };
        if over == 0 {
            return;
        }
        self.pending.drain(..over);
        if !self.warned_overflow {
            self.warned_overflow = true;
            warn!(
                "comms backlog is over {COMMS_PENDING_CAP} waiting lines; dropping the oldest.                  A scenario is posting story faster than the panel can show it."
            );
        }
    }

    /// Close an overflow episode once the backlog is inside its cap again, so
    /// the next burst is reported rather than swallowed by the last one's
    /// throttle.
    fn settle_backlog(&mut self) {
        if self.pending.len() < COMMS_PENDING_CAP {
            self.warned_overflow = false;
        }
    }
}

/// Drives the comms panel: the paced display queue over [`StoryFeed`] that
/// shows speaker-attributed story lines as a bottom-left stack.
pub struct CommsPanelPlugin;

impl Plugin for CommsPanelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StoryFeed>();
        app.init_resource::<CommsQueue>();
        app.add_systems(Startup, spawn_comms_panel);
        app.add_systems(
            Update,
            (
                enqueue_new_lines.run_if(resource_changed::<StoryFeed>),
                drive_comms_stack,
                reconcile_comms_cards,
                paint_comms_cards,
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
            width: Val::Percent(COMMS_PANEL_WIDTH_PERCENT),
            max_width: Val::Px(COMMS_PANEL_MAX_WIDTH_PX),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(10.0),
            ..default()
        },
    ));
}

/// Feed changes drive the pending queue, bounded by [`COMMS_PENDING_CAP`]; an
/// EMPTIED feed (scenario teardown) resets everything instantly - the
/// leaked-line pin.
///
/// The QUEUE is the whole reset. The tree follows from it in the same frame,
/// because [`reconcile_comms_cards`] is chained behind this and despawns the
/// card of every line that is no longer visible - so there is exactly one
/// system that edits the card set, and teardown cannot drift from expiry.
fn enqueue_new_lines(feed: Res<StoryFeed>, mut queue: ResMut<CommsQueue>) {
    if feed.0.len() < queue.seen {
        // Teardown (the feed is append-only in-scenario, so shrinking means
        // reset): drop the queue and the visible stack.
        queue.seen = 0;
        queue.pending.clear();
        queue.visible.clear();
        queue.warned_overflow = false;
    }
    let seen = queue.seen;
    for line in feed.0.iter().skip(seen) {
        queue.pending.push_back(line.clone());
    }
    queue.seen = feed.0.len();
    queue.trim_backlog();
}

/// Tick visible cards, apply controls, and promote pending lines into open
/// visible slots.
fn drive_comms_stack(
    time: Res<Time>,
    mut queue: ResMut<CommsQueue>,
    mut commands: Commands,
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

    while queue.visible.len() < COMMS_VISIBLE_CAP {
        let Some(line) = queue.pending.pop_front() else {
            break;
        };
        let id = queue.next_id;
        queue.next_id += 1;
        queue.visible.push_back(VisibleCommsLine {
            id,
            line,
            age_secs: 0.0,
        });
        if let Some(bank) = &bank {
            commands.play_sfx(
                bank.get(UiSfx::CommsLine),
                AudioRoute::Interface,
                COMMS_BLIP_VOLUME,
            );
        }
    }

    queue.settle_backlog();
}

/// Spawn the card of a line that has just appeared and despawn the card of a
/// line that has gone. Nothing else.
///
/// The panel used to tear the whole stack down and rebuild it every frame,
/// idle frames included: with the cap at three that was fifteen despawns and
/// fifteen spawns per frame, fifteen taffy nodes deregistered and
/// re-registered, and six freshly built `Text` components - a full measure and
/// shape pass over every glyph of a wrapped body line, every frame, for as
/// long as a conversation was up. The only thing that actually moved between
/// those frames was the alpha, which [`paint_comms_cards`] writes onto the
/// nodes that are already there.
///
/// New lines are always promoted onto the BACK of `visible`, and spawning
/// appends, so walking the queue in order and spawning the ones that have no
/// card yet keeps the children in stack order without reordering anything.
fn reconcile_comms_cards(
    queue: Res<CommsQueue>,
    asset_server: Option<Res<AssetServer>>,
    mut commands: Commands,
    mut panel: Query<(Entity, &mut Visibility), With<CommsPanelMarker>>,
    cards: Query<(Entity, &CommsLineId), With<CommsCardMarker>>,
) {
    let Ok((panel, mut visibility)) = panel.single_mut() else {
        return;
    };
    let wanted = if queue.visible.is_empty() {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    };
    if *visibility != wanted {
        *visibility = wanted;
    }

    for (card, id) in &cards {
        if !queue.visible.iter().any(|visible| visible.id == id.0) {
            commands.entity(card).despawn();
        }
    }
    commands.entity(panel).with_children(|parent| {
        for visible in &queue.visible {
            if cards.iter().any(|(_, id)| id.0 == visible.id) {
                continue;
            }
            parent.spawn(comms_card(visible, asset_server.as_deref()));
        }
    });
}

/// Fade what is already on screen.
///
/// The alpha is the whole per-frame part of a card - the fade, and nothing
/// else. Written on diff, so the flat middle of a dwell - where nothing is
/// fading - costs nothing at all.
///
/// The TEXT is not here. It is written once, when the card is spawned, because
/// a showing line's words do not change.
#[expect(
    clippy::type_complexity,
    reason = "one query per marked node in a card"
)]
fn paint_comms_cards(
    queue: Res<CommsQueue>,
    mut q_card: Query<
        (&CommsLineId, &mut BorderColor, &mut BackgroundColor),
        With<CommsCardMarker>,
    >,
    mut q_icon: Query<
        (
            &CommsLineId,
            &CommsIconMarker,
            &mut BorderColor,
            &mut BackgroundColor,
            &mut ImageNode,
        ),
        Without<CommsCardMarker>,
    >,
    mut q_speaker: Query<(&CommsLineId, &mut TextColor), With<CommsSpeakerMarker>>,
    mut q_body: Query<
        (&CommsLineId, &mut TextColor),
        (With<CommsTextMarker>, Without<CommsSpeakerMarker>),
    >,
) {
    let showing = |id: &CommsLineId| queue.visible.iter().find(|visible| visible.id == id.0);

    for (id, mut border, mut background) in &mut q_card {
        let Some(visible) = showing(id) else {
            continue;
        };
        let (accent, alpha) = (visible.line.accent, visible.alpha());
        set_border(&mut border, accent.with_alpha(CHIP_BORDER_ALPHA * alpha));
        set_background(
            &mut background,
            CHIP_FILL.with_alpha(CHIP_FILL.alpha() * alpha),
        );
    }
    for (id, icon, mut border, mut background, mut image) in &mut q_icon {
        let Some(visible) = showing(id) else {
            continue;
        };
        let (accent, alpha) = (visible.line.accent, visible.alpha());
        set_border(&mut border, accent.with_alpha(alpha));
        let (fill, tint) = match icon.kind {
            CommsIconKind::Authored => (
                Color::srgba(0.0, 0.0, 0.0, 0.0),
                Color::WHITE.with_alpha(alpha),
            ),
            CommsIconKind::Fallback => (accent.with_alpha(ICON_FILL_ALPHA * alpha), image.color),
        };
        set_background(&mut background, fill);
        if image.color != tint {
            image.color = tint;
        }
    }
    for (id, mut color) in &mut q_speaker {
        let Some(visible) = showing(id) else {
            continue;
        };
        set_text_color(&mut color, visible.line.accent.with_alpha(visible.alpha()));
    }
    for (id, mut color) in &mut q_body {
        let Some(visible) = showing(id) else {
            continue;
        };
        set_text_color(
            &mut color,
            body_colour(visible.line.accent).with_alpha(visible.alpha()),
        );
    }
}

/// The three write-on-diff helpers. A `DerefMut` on any of these marks the
/// component changed, which is what wakes the UI passes downstream, so a card
/// resting at full alpha must not be assigned to.
fn set_border(border: &mut BorderColor, wanted: Color) {
    let wanted = BorderColor::all(wanted);
    if *border != wanted {
        *border = wanted;
    }
}

fn set_background(background: &mut BackgroundColor, wanted: Color) {
    if background.0 != wanted {
        background.0 = wanted;
    }
}

fn set_text_color(color: &mut TextColor, wanted: Color) {
    if color.0 != wanted {
        color.0 = wanted;
    }
}

fn comms_card(line: &VisibleCommsLine, asset_server: Option<&AssetServer>) -> impl Bundle {
    let accent = line.line.accent;
    let alpha = line.alpha();
    let header = line.line.speaker.to_uppercase();
    (
        CommsCardMarker,
        CommsLineId(line.id),
        Node {
            width: Val::Percent(100.0),
            min_height: Val::Px(76.0),
            padding: UiRect::all(Val::Px(14.0)),
            border: UiRect::all(Val::Px(1.0)),
            column_gap: Val::Px(12.0),
            align_items: AlignItems::FlexStart,
            ..default()
        },
        // The card is a member of the HUD chip family - the shared slab, the
        // chip border alpha - drawn in the accent THE CUE names. That is what
        // makes an incoming transmission instantly distinguishable from a
        // flight readout (demo 2 `.comms`), and one voice from another.
        BorderColor::all(accent.with_alpha(CHIP_BORDER_ALPHA * alpha)),
        BackgroundColor(CHIP_FILL.with_alpha(CHIP_FILL.alpha() * alpha)),
        children![
            comms_icon(line.id, &line.line, accent, alpha, asset_server),
            (
                Node {
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(6.0),
                    ..default()
                },
                children![
                    (
                        CommsSpeakerMarker,
                        CommsLineId(line.id),
                        Text::new(header),
                        TextFont::from_font_size(COMMS_SPEAKER_FONT_SIZE_PX),
                        TextColor(accent.with_alpha(alpha)),
                    ),
                    (
                        CommsTextMarker,
                        CommsLineId(line.id),
                        Text::new(line.line.text.clone()),
                        TextFont::from_font_size(COMMS_BODY_FONT_SIZE_PX),
                        TextColor(body_colour(accent).with_alpha(alpha)),
                        TextLayout {
                            linebreak: LineBreak::WordBoundary,
                            ..default()
                        },
                    )
                ]
            )
        ],
    )
}

fn comms_icon(
    id: u64,
    line: &StoryLine,
    accent: Color,
    alpha: f32,
    asset_server: Option<&AssetServer>,
) -> impl Bundle {
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
            CommsLineId(id),
            node,
            ImageNode::new(
                asset_server
                    .map(|server| icon.resolve(server))
                    .unwrap_or_default(),
            )
            .with_color(Color::WHITE.with_alpha(alpha)),
            BorderColor::all(accent.with_alpha(alpha)),
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
            children![],
        ),
        None => (
            CommsIconMarker {
                kind: CommsIconKind::Fallback,
            },
            CommsLineId(id),
            node,
            ImageNode::default(),
            BorderColor::all(accent.with_alpha(alpha)),
            BackgroundColor(accent.with_alpha(ICON_FILL_ALPHA * alpha)),
            children![],
        ),
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use bevy::time::TimeUpdateStrategy;
    use nova_gameplay::narrative_accent::prelude::default_comms_accent;
    use nova_ui::{hud::ChipTone, theme::ActiveUiTheme};

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
        app.add_systems(Startup, spawn_comms_panel);
        app.add_systems(
            Update,
            (
                enqueue_new_lines.run_if(resource_changed::<StoryFeed>),
                drive_comms_stack,
                reconcile_comms_cards,
                paint_comms_cards,
            )
                .chain(),
        );
        // The one system that writes `UiTransform::scale` on a HUD node, in
        // PostUpdate so it sees the cards `reconcile_comms_cards` queued this
        // frame. Without it a card could carry an emphasis and no test here
        // would ever read the scale that emphasis asks for.
        app.add_systems(PostUpdate, crate::emphasis::drive_hud_emphasis);
        // The rewrite counters, in PostUpdate for the same reason: a spawn
        // queued in Update is not in the world until the schedule's sync
        // point. `Changed` can only be read from a SYSTEM - asking an
        // `EntityRef` is silently always false - so the count has to be
        // collected here and read back off the resource.
        app.init_resource::<CardChurn>();
        app.add_systems(PostUpdate, count_card_churn);
        app
    }

    /// How much of the card tree the panel has rebuilt since the app started.
    #[derive(Resource, Default, Debug, PartialEq, Eq)]
    struct CardChurn {
        /// Cards spawned.
        cards: usize,
        /// Body `Text` components written - each one a measure and shape pass
        /// over every glyph of a wrapped line.
        texts: usize,
    }

    fn count_card_churn(
        mut churn: ResMut<CardChurn>,
        cards: Query<(), Added<CommsCardMarker>>,
        texts: Query<(), (Changed<Text>, With<CommsTextMarker>)>,
    ) {
        churn.cards += cards.iter().count();
        churn.texts += texts.iter().count();
    }

    fn churn(app: &App) -> (usize, usize) {
        let churn = app.world().resource::<CardChurn>();
        (churn.cards, churn.texts)
    }

    /// The panel's cards in STACK order - the order the column lays them out,
    /// which is `Children` and not the order a global query happens to walk
    /// its archetypes in.
    fn cards_in_stack_order(app: &mut App) -> Vec<Entity> {
        let world = app.world_mut();
        let Ok(children) = world
            .query_filtered::<&Children, With<CommsPanelMarker>>()
            .single(world)
        else {
            return Vec::new();
        };
        children.iter().collect()
    }

    /// Read `T` off each card in stack order, through `pick`.
    fn per_card<T>(app: &mut App, pick: impl Fn(EntityRef<'_>) -> T) -> Vec<T> {
        cards_in_stack_order(app)
            .into_iter()
            .map(|card| pick(app.world().entity(card)))
            .collect()
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
                accent: default_comms_accent(),
                speaker: speaker.to_string(),
                text: text.to_string(),
                dwell,
                icon,
            });
    }

    /// A cue that names its own colour - the distress orange of the worked
    /// example. Any accent that is not the comms blue would do; this one
    /// is far enough from it that a card drawn in the wrong one is obvious.
    const DISTRESS: Color = Color::srgb(1.0, 0.55, 0.2);

    fn push_accent_line(app: &mut App, accent: Color, speaker: &str) {
        app.world_mut()
            .resource_mut::<StoryFeed>()
            .0
            .push(StoryLine {
                accent,
                speaker: speaker.to_string(),
                text: "Say again.".to_string(),
                dwell: None,
                icon: None,
            });
    }

    /// A colour's channels alone, so an assertion about WHICH colour a node was
    /// painted is not also an assertion about the fade it was caught in.
    fn rgb(color: Color) -> [u8; 3] {
        let [red, green, blue, _] = color.to_srgba().to_u8_array();
        [red, green, blue]
    }

    /// The border alpha of each drawn card, in stack order.
    fn card_border_alphas(app: &mut App) -> Vec<f32> {
        card_border_colours(app)
            .into_iter()
            .map(|border| border.alpha())
            .collect()
    }

    /// The border colour of each drawn card, in stack order.
    fn card_border_colours(app: &mut App) -> Vec<Color> {
        per_card(app, |card| {
            card.get::<BorderColor>()
                .expect("a card draws a border")
                .top
        })
    }

    fn panel_visibility(app: &mut App) -> Visibility {
        *app.world_mut()
            .query_filtered::<&Visibility, With<CommsPanelMarker>>()
            .single(app.world())
            .expect("the comms panel exists")
    }

    fn visible_texts(app: &mut App) -> Vec<String> {
        let cards = cards_in_stack_order(app);
        cards
            .into_iter()
            .filter_map(|card| text_under::<CommsTextMarker>(app.world(), card))
            .collect()
    }

    fn visible_speakers(app: &mut App) -> Vec<String> {
        let cards = cards_in_stack_order(app);
        cards
            .into_iter()
            .filter_map(|card| text_under::<CommsSpeakerMarker>(app.world(), card))
            .collect()
    }

    /// The node inside `card` marked with `M`, if there is one.
    fn node_under<M: Component>(world: &World, card: Entity) -> Option<Entity> {
        fn walk<M: Component>(world: &World, at: Entity) -> Option<Entity> {
            if world.get::<M>(at).is_some() {
                return Some(at);
            }
            world
                .get::<Children>(at)?
                .iter()
                .find_map(|child| walk::<M>(world, child))
        }
        walk::<M>(world, card)
    }

    /// The `Text` of the node inside `card` marked with `M`.
    fn text_under<M: Component>(world: &World, card: Entity) -> Option<String> {
        fn walk<M: Component>(world: &World, at: Entity) -> Option<String> {
            if world.get::<M>(at).is_some() {
                return world.get::<Text>(at).map(|text| text.0.clone());
            }
            world
                .get::<Children>(at)?
                .iter()
                .find_map(|child| walk::<M>(world, child))
        }
        walk::<M>(world, card)
    }

    /// The cap is DERIVED, and this is the arithmetic. If the visible stack
    /// grows, the dwell floor moves or the fade changes, the number the panel
    /// can actually show changes with it and the constant has to be re-derived
    /// rather than left where someone's taste put it.
    #[test]
    fn the_waiting_backlog_is_sized_by_what_the_panel_can_show() {
        let slot_secs = COMMS_DWELL_MIN_SECS + COMMS_FADE_OUT_SECS;
        let per_slot = (COMMS_DWELL_MAX_SECS / slot_secs).floor() as usize;
        assert_eq!(
            COMMS_PENDING_CAP,
            COMMS_VISIBLE_CAP * per_slot,
            "{COMMS_VISIBLE_CAP} slots turning over every {slot_secs}s reach \
             {per_slot} lines each inside the {COMMS_DWELL_MAX_SECS}s a line \
             stays worth reading"
        );
    }

    /// A scenario can post story faster than three cards at a time can read it.
    /// The backlog is bounded, and what goes is the OLDEST waiting line - the
    /// one answering a beat the player has already flown past - never the line
    /// about the fight they are in.
    #[test]
    fn a_backlog_past_its_cap_drops_the_oldest_waiting_line() {
        let mut app = comms_app();
        app.update();
        let burst = COMMS_PENDING_CAP + COMMS_VISIBLE_CAP * 2;
        for index in 0..burst {
            push_line(&mut app, "MERIDIAN", &format!("line {index}"), None);
        }
        app.update();

        let queue = app.world().resource::<CommsQueue>();
        assert_eq!(
            queue.pending.len() + queue.visible.len(),
            COMMS_PENDING_CAP,
            "the backlog is capped, and promoting into the stack comes out of it"
        );
        assert_eq!(
            queue.pending.back().map(|line| line.text.as_str()),
            Some(format!("line {}", burst - 1).as_str()),
            "the newest line is never the one dropped"
        );
        let dropped = burst - COMMS_PENDING_CAP;
        assert_eq!(
            queue
                .visible
                .front()
                .map(|visible| visible.line.text.as_str()),
            Some(format!("line {dropped}").as_str()),
            "the oldest {dropped} lines went, and the stack starts after them"
        );
    }

    /// One line in the log per overflow episode, not one per cue it costs. A
    /// burst that outruns the panel drops lines by the dozen, and a report per
    /// casualty buries the fact that it happened at all.
    #[test]
    fn an_overflow_episode_is_reported_once_and_a_later_one_again() {
        let mut queue = CommsQueue::default();
        let line = StoryLine {
            accent: default_comms_accent(),
            speaker: "MERIDIAN".to_string(),
            text: "overrun".to_string(),
            dwell: None,
            icon: None,
        };
        for _ in 0..COMMS_PENDING_CAP + 4 {
            queue.pending.push_back(line.clone());
        }
        queue.trim_backlog();
        assert_eq!(queue.pending.len(), COMMS_PENDING_CAP);
        assert!(queue.warned_overflow, "the episode is reported once");

        // Still over: the same episode, already said.
        queue.pending.push_back(line.clone());
        queue.trim_backlog();
        queue.settle_backlog();
        assert!(
            queue.warned_overflow,
            "a backlog still at its cap is the same episode"
        );

        // Drained below the cap: the episode is over.
        queue.pending.pop_front();
        queue.settle_backlog();
        assert!(!queue.warned_overflow, "a settled backlog closes it");

        queue.pending.push_back(line.clone());
        queue.pending.push_back(line);
        queue.trim_backlog();
        assert!(queue.warned_overflow, "a later burst is reported again");
    }

    #[test]
    fn the_panel_is_screen_relative_and_the_message_has_two_text_scales() {
        let mut app = comms_app();
        app.update();
        let panel = app
            .world_mut()
            .query_filtered::<&Node, With<CommsPanelMarker>>()
            .single(app.world())
            .unwrap();
        assert_eq!(panel.width, Val::Percent(COMMS_PANEL_WIDTH_PERCENT));
        assert_eq!(panel.max_width, Val::Px(COMMS_PANEL_MAX_WIDTH_PX));

        push_line(&mut app, "Meridian Control", "Return to the plate.", None);
        app.update();
        let speaker_size = app
            .world_mut()
            .query_filtered::<&TextFont, With<CommsSpeakerMarker>>()
            .single(app.world())
            .unwrap()
            .font_size;
        let body_size = app
            .world_mut()
            .query_filtered::<&TextFont, With<CommsTextMarker>>()
            .single(app.world())
            .unwrap()
            .font_size;
        assert_eq!(speaker_size, FontSize::Px(COMMS_SPEAKER_FONT_SIZE_PX));
        assert_eq!(body_size, FontSize::Px(COMMS_BODY_FONT_SIZE_PX));
    }

    #[test]
    fn a_burst_stacks_visible_lines_newest_at_bottom() {
        let mut app = comms_app();
        app.update();
        push_line(&mut app, "Alpha", "First.", None);
        push_line(&mut app, "Bravo", "Second.", None);
        push_line(&mut app, "Relay", "Third.", None);
        app.update();

        assert_eq!(
            visible_texts(&mut app),
            vec![
                "First.".to_string(),
                "Second.".to_string(),
                "Third.".to_string(),
            ],
            "child order is top-to-bottom, so newest is the bottom card"
        );
        assert_eq!(
            visible_speakers(&mut app),
            vec!["ALPHA", "BRAVO", "RELAY"],
            "speaker attribution has its own header instead of prefixing the body"
        );
        assert_eq!(panel_visibility(&mut app), Visibility::Inherited);
    }

    /// The stack is a WINDOW on the queue: a burst larger than the cap shows
    /// the cap's worth and holds the rest back, then lets them through as the
    /// visible cards age out.
    #[test]
    fn a_burst_larger_than_the_cap_drains_as_cards_expire() {
        let mut app = comms_app();
        app.update();
        for i in 0..6 {
            push_line(&mut app, "Alpha", &format!("Line {i}."), None);
        }
        app.update();
        assert_eq!(
            visible_texts(&mut app).len(),
            COMMS_VISIBLE_CAP,
            "the initial burst fills only the visible stack"
        );

        // One card is 8 s of dwell plus a 0.4 s fade, and the manual clock
        // advances 0.25 s an update, so the newest line waits out two cards.
        let mut seen_last_line = false;
        for _ in 0..80 {
            app.update();
            seen_last_line |= visible_texts(&mut app).iter().any(|text| text == "Line 5.");
        }
        assert!(
            seen_last_line,
            "the backlog reaches the stack once the earlier cards expire"
        );
        assert_eq!(
            app.world().resource::<StoryFeed>().0.len(),
            6,
            "the paced window never trims the transcript"
        );
    }

    #[test]
    fn speaker_icons_use_authored_refs_and_fallback() {
        let mut app = comms_app();
        app.update();
        push_line_with_icon(
            &mut app,
            "Alpha",
            "Face.",
            None,
            Some(AssetRef::from("icons/alpha.png")),
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
        push_line(&mut app, "Alpha", "First.", None);
        push_line(&mut app, "Alpha", "Second.", None);
        app.update();
        app.update();
        assert_eq!(
            visible_texts(&mut app),
            vec!["First.".to_string(), "Second.".to_string()],
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
        push_line(&mut app, "Alpha", "Take your time.", None);
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
        push_line(&mut app, "Alpha", "Blink and gone.", Some(0.5));
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

    /// Bursts larger than the visible window are held in the pending queue.
    /// Creator-authored dialogue must not disappear because it arrived in one
    /// frame - only a backlog past [`COMMS_PENDING_CAP`] drops anything, and
    /// what it drops is the oldest.
    #[test]
    fn the_pending_queue_keeps_every_line_past_the_visible_window() {
        let mut app = comms_app();
        app.update();
        for i in 0..6 {
            push_line(&mut app, "Alpha", &format!("Line {i}."), None);
        }
        app.update();
        app.update();
        assert_eq!(
            visible_texts(&mut app),
            vec![
                "Line 0.".to_string(),
                "Line 1.".to_string(),
                "Line 2.".to_string(),
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
            vec!["Line 3.", "Line 4.", "Line 5."],
            "every line outside the visible window remains pending"
        );
    }

    /// An emptied feed (scenario teardown syncs an empty log) resets the
    /// whole pipeline immediately - the leaked-line pin, queue edition.
    #[test]
    fn emptied_feed_resets_the_comms_stack_immediately() {
        let mut app = comms_app();
        app.update();
        push_line(&mut app, "Alpha", "Heads up.", None);
        push_line(&mut app, "Alpha", "Backlog.", None);
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

    /// The claim the rewrite is for, stated as COUNTS rather than
    /// milliseconds: a card that is still showing is not rebuilt.
    ///
    /// The panel used to despawn and respawn its whole stack every frame. With
    /// the cap at three that was 15 despawns and 15 spawns per frame and six
    /// `Text` components built from scratch - a full measure and shape pass
    /// over every glyph of a wrapped body line - for as long as a conversation
    /// was up. Here three lines hold for six more frames and the counters do
    /// not move: three cards, three body texts, once each.
    #[test]
    fn a_showing_card_is_not_rebuilt_while_it_holds() {
        let mut app = comms_app();
        app.update();
        push_line(&mut app, "ALPHA", "First line.", None);
        push_line(&mut app, "BRAVO", "Second line.", None);
        push_line(&mut app, "CHARLIE", "Third line.", None);
        app.update();

        let settled = cards_in_stack_order(&mut app);
        assert_eq!(settled.len(), 3, "the burst filled the visible stack");
        assert_eq!(
            churn(&app),
            (3, 3),
            "three cards and three body texts, built once"
        );

        for _ in 0..6 {
            app.update();
        }
        assert_eq!(
            cards_in_stack_order(&mut app),
            settled,
            "a line that is still showing keeps its card - the same entities, \
             the same taffy nodes, the same shaped text"
        );
        assert_eq!(
            churn(&app),
            (3, 3),
            "and nothing was spawned or re-shaped to hold it there"
        );
    }

    /// The other half: the card set still FOLLOWS the queue. A line that
    /// expires takes its own card and leaves its neighbours alone, and a new
    /// line arrives at the bottom of the stack.
    #[test]
    fn expiry_takes_one_card_and_arrival_appends_one() {
        let mut app = comms_app();
        app.update();
        // A short line that expires first, then one that outlives it.
        push_line(&mut app, "ALPHA", "Brief.", Some(COMMS_DWELL_MIN_SECS));
        push_line(&mut app, "BRAVO", "Longer.", None);
        app.update();
        let before = cards_in_stack_order(&mut app);
        assert_eq!(before.len(), 2);
        let (spawned, _) = churn(&app);

        // The clamped 3.0 s dwell plus its 0.4 s fade, at 0.25 s an update -
        // and nowhere near the other line's 8.0 s.
        for _ in 0..14 {
            app.update();
        }
        let after = cards_in_stack_order(&mut app);
        assert_eq!(
            after,
            vec![before[1]],
            "the expired line's card goes and the one still showing is the \
             SAME entity, not a rebuild of it"
        );
        assert_eq!(visible_speakers(&mut app), vec!["BRAVO"]);
        assert_eq!(churn(&app).0, spawned, "nothing respawns to close the gap");

        push_line(&mut app, "CHARLIE", "Arriving.", None);
        app.update();
        let arrived = cards_in_stack_order(&mut app);
        assert_eq!(arrived.len(), 2);
        assert_eq!(arrived[0], before[1], "the held card keeps its slot");
        assert_eq!(
            visible_speakers(&mut app),
            vec!["BRAVO", "CHARLIE"],
            "and the new line lands at the bottom of the stack"
        );
        assert_eq!(churn(&app).0, spawned + 1, "one card for one line");
    }

    /// A card that is not rebuilt still FADES: the alpha is the per-frame part,
    /// written onto the nodes that are already there. Without this the test
    /// above would pass just as well on a panel that had stopped drawing.
    #[test]
    fn a_held_card_still_fades_on_its_own_entities() {
        let mut app = comms_app();
        app.update();
        push_line(&mut app, "ALPHA", "Fading.", Some(COMMS_DWELL_MIN_SECS));
        app.update();
        let card = cards_in_stack_order(&mut app);
        assert_eq!(card.len(), 1);
        // The card is spawned at age zero, which is the bottom of its fade-in.
        let arriving = card_border_alphas(&mut app);
        assert_eq!(arriving, vec![0.0], "it arrives transparent");

        // One 0.25 s update is exactly the fade-in, so the card is fully up.
        app.update();
        let up = card_border_alphas(&mut app);
        assert_eq!(
            cards_in_stack_order(&mut app),
            card,
            "the same entity is still the one on screen"
        );
        assert!(
            up[0] > arriving[0],
            "and the fade is written onto it rather than respawned: {} then {}",
            arriving[0],
            up[0]
        );
        assert_eq!(churn(&app).0, 1, "one card, faded in place");

        // Out the far side: the dwell and the fade tail, and the card goes.
        for _ in 0..14 {
            app.update();
        }
        assert!(
            cards_in_stack_order(&mut app).is_empty(),
            "and it is taken down when its line expires"
        );
    }

    /// Arrival must not transform either the new card or the existing log.
    /// Scaling a flex child does not reserve the scaled space, so it crosses
    /// the screen edge and overlaps the previous card even when anchored.
    #[test]
    fn comms_arrival_keeps_every_card_at_layout_size() {
        let mut app = comms_app();
        push_line(&mut app, "ALPHA", "First line.", None);
        app.update();
        push_line(&mut app, "BRAVO", "Second line.", None);
        app.update();

        let world = app.world_mut();
        let mut cards = world.query_filtered::<
            (Option<&UiTransform>, Option<&crate::emphasis::HudEmphasis>),
            With<CommsCardMarker>,
        >();
        let found: Vec<(Vec2, bool)> = cards
            .iter(world)
            .map(|(transform, emphasis)| {
                (
                    transform.map_or(Vec2::ONE, |transform| transform.scale),
                    emphasis.is_some(),
                )
            })
            .collect();
        // Both halves matter: an emphasis is what would scale a card, and the
        // scale is what would push it over the screen edge. Asserting only the
        // scale passes on a panel that has no emphasis driver at all.
        assert_eq!(found, vec![(Vec2::ONE, false), (Vec2::ONE, false)]);
    }

    /// A cue that names no accent is ordinary comms traffic, and comms traffic
    /// is blue. With channels gone this default is the panel's whole fallback
    /// behaviour: no catalog to consult, no unknown id to get wrong, one
    /// colour - and it has to stay the blue the HUD already speaks in.
    #[test]
    fn a_cue_that_names_no_accent_is_drawn_in_the_comms_blue() {
        assert_eq!(
            default_comms_accent(),
            ChipTone::Comms.text(&ActiveUiTheme::default()),
            "the cue default IS the HUD's comms tone, not a second blue beside it"
        );

        let mut app = comms_app();
        app.update();
        push_line(&mut app, "Meridian Control", "Say again.", None);
        // Past the fade-in, where the card is at its steady strength.
        for _ in 0..3 {
            app.update();
        }

        let borders = card_border_colours(&mut app);
        assert_eq!(borders.len(), 1);
        assert_eq!(
            rgb(borders[0]),
            rgb(default_comms_accent()),
            "a default cue is framed in the comms blue"
        );
    }

    /// One authored colour drives the whole card. A cue that sets an accent and
    /// gets a blue speaker over an orange frame is a card wearing two voices at
    /// once, which is exactly what the channel lookup used to allow.
    ///
    /// The body is the one part that is NOT the accent itself: it is the accent
    /// lifted toward white, so the words stay the brightest thing on the card
    /// while still reading as the same voice. The slab underneath belongs to
    /// the HUD, not to the cue.
    #[test]
    fn a_custom_accent_draws_the_border_speaker_icon_and_body() {
        let mut app = comms_app();
        app.update();
        push_accent_line(&mut app, DISTRESS, "Meridian Control");
        for _ in 0..3 {
            app.update();
        }

        let card = *cards_in_stack_order(&mut app)
            .first()
            .expect("the accented line is showing");
        let border = app
            .world()
            .entity(card)
            .get::<BorderColor>()
            .expect("a card draws a border")
            .top;
        assert_eq!(rgb(border), rgb(DISTRESS), "the frame is the accent");
        // Every other node shares this card's fade, so read it off the frame
        // rather than assuming the frame the cards were measured on.
        let fade = border.alpha() / CHIP_BORDER_ALPHA;

        let fill = app
            .world()
            .entity(card)
            .get::<BackgroundColor>()
            .expect("a card draws a slab")
            .0;
        assert_eq!(
            rgb(fill),
            rgb(CHIP_FILL),
            "the slab stays the shared HUD dark, whatever the cue is accented"
        );

        let speaker = node_under::<CommsSpeakerMarker>(app.world(), card)
            .and_then(|node| {
                app.world()
                    .entity(node)
                    .get::<TextColor>()
                    .map(|text| text.0)
            })
            .expect("the card names its speaker");
        assert_eq!(
            speaker,
            DISTRESS.with_alpha(fade),
            "the speaker is the accent"
        );

        let icon =
            node_under::<CommsIconMarker>(app.world(), card).expect("the card draws an icon");
        let icon_border = app
            .world()
            .entity(icon)
            .get::<BorderColor>()
            .expect("the icon tile is framed")
            .top;
        let icon_fill = app
            .world()
            .entity(icon)
            .get::<BackgroundColor>()
            .expect("the icon tile is washed")
            .0;
        assert_eq!(icon_border, DISTRESS.with_alpha(fade));
        assert_eq!(
            icon_fill,
            DISTRESS.with_alpha(ICON_FILL_ALPHA * fade),
            "the fallback tile is a faint wash of the same accent"
        );

        let body = node_under::<CommsTextMarker>(app.world(), card)
            .and_then(|node| {
                app.world()
                    .entity(node)
                    .get::<TextColor>()
                    .map(|text| text.0)
            })
            .expect("the card carries its line");
        assert_eq!(body, body_colour(DISTRESS).with_alpha(fade));
        assert_ne!(
            rgb(body),
            rgb(DISTRESS),
            "the words are the accent LIFTED, not the accent itself"
        );
    }
}
