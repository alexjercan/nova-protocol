//! The objective NOTIFICATION stack: demo 2's top-centre amber objective
//! chip, one per posted objective, read like a notification.
//!
//! Demo 2 (`examples/ui/hud_rework_poc.html`) puts the objective ITSELF on the
//! flight HUD - `<div class="chip obj">&#9670; SALVAGE WRECK ...</div>`, a
//! top-centre bordered amber chip that pops when posted and then breathes.
//! This module is that chip, with two changes the game needs and the mock did
//! not have:
//!
//! - it STACKS: several objectives can be active, so the artifact is a column
//!   of chips, newest on top, not a single element;
//! - it is READ like a notification, not a permanent readout: a chip shows on
//!   posting and leaves when it has been read - EITHER its dwell elapses OR the
//!   player opens the NOVA OS computer (which is where the standing objective
//!   list lives). Once read it does not come back; re-wording an objective
//!   posts it again, unread.
//!
//! That read model is the owner's call: idle cruise has NO objective cue at
//! all, on purpose. The
//! standing answers to "what am I doing" are the world-anchored objective
//! marker chips ([`super::objective_markers`], which own the "go HERE" job and
//! the live range) and the NOVA OS `objectives` command.
//!
//! This REPLACES the top-right status-bar hint: the count-plus-TAB block is
//! gone from the bcs status bar,
//! which is back to fps + version. The `TAB` affordance it owned moved here and
//! rides the stack, leaving with it.
//!
//! The chip IS the posting: it spawns and pops on the
//! frame the objective posts, arriving like a chat message. It used to wait
//! ~3.2 s behind a diegetic cockpit reveal card that tucked into this stack;
//! the owner retired that card, so the stack is the sole presentation of a
//! posting and no handover gate stands between the two.

use bevy::prelude::*;
use bevy_common_systems::prelude::GameObjectives;
use nova_ui::hud::{chip_node, ChipTone};

use super::{emphasis::prelude::*, HudTier, NovaHudAssets, NovaHudSystems};
use crate::prelude::*;

/// Glob-import surface: `use nova_gameplay::hud::objective_stack::prelude::*` re-exports the public API of this module.
pub mod prelude {
    pub use super::{
        objective_stack_hud, ObjectiveNotifications, ObjectiveStackChip, ObjectiveStackHudMarker,
        ObjectiveStackPlugin, OBJECTIVE_DWELL_SECS,
    };
}

/// How long a posted objective stays on screen before it counts as read,
/// counted from the POSTING (the chip is on screen from that frame). Generous:
/// the chip is the only place the objective's text lives while it is up.
/// Tunable at the playtest gate.
pub const OBJECTIVE_DWELL_SECS: f32 = 12.0;

/// Fade-out once a chip is read, matching the comms card's tail so the two
/// notification surfaces leave at the same speed.
const OBJECTIVE_FADE_SECS: f32 = 0.4;

/// The posting POP and the settle-into breath - demo 2's `.obj.emph` (1.16 for
/// 1.2 s) and `@keyframes breathe` (2.4 s, down to 0.72 alpha).
const CHIP_POP_SCALE: f32 = 1.16;
const CHIP_POP_SECS: f32 = 1.2;
const CHIP_BREATH_PERIOD_SECS: f32 = 2.4;
const CHIP_BREATH_MIN_ALPHA: f32 = 0.72;

/// Top offset (px) of the stack. Demo 2 uses `.obj { top: 58px }`, but the mock
/// has nothing else up there: the game's scenario readout strip
/// (`super::readout`) is a top-centre column at `top: 16px` that grows DOWNWARD,
/// and one two-line readout (a time trial's `RELIEF 01:09.7`) already reaches
/// ~65 px - so 58 puts an objective chip on top of the run timer. Measured on
/// the lifeline walk; 96 clears a one-readout strip with margin.
///
/// KNOWN LIMIT: a scenario showing two or more readouts can still reach this
/// far. The durable fix is one shared top-centre COLUMN that both the strip and
/// this stack flow inside, instead of two absolute nodes guessing at each
/// other's height - out of scope here (it restructures a working widget), and
/// worth doing if the playtest hits it.
const STACK_TOP_PX: f32 = 96.0;

/// The diamond that leads every objective chip - demo 2's `.di` glyph. Drawn
/// as a rotated bordered SQUARE, not the `\u{25c6}` character: the shipped
/// Iosevka font has no diamond glyph and renders it as tofu (seen on the
/// lifeline walk). This is the same trick `objective_markers`
/// already uses for the same mark.
const DIAMOND_PX: f32 = 7.0;
const DIAMOND_BORDER_PX: f32 = 1.5;

const CHIP_FONT_PX: f32 = 13.0;
/// The TAB keycap's HEIGHT on the stack's footer, sized like the dock's
/// keycaps: the width follows the cap's own aspect (`KeyCap`), which matters
/// here more than anywhere - Tab is one of the WIDE caps.
const TAB_GLYPH_PX: f32 = 18.0;
const TAB_FONT_PX: f32 = 11.0;

/// Marker for the top-centre stack root; spawned by [`objective_stack_hud`].
#[derive(Component, Debug, Clone, Reflect)]
pub struct ObjectiveStackHudMarker;

/// One rendered objective chip, carrying the id of the objective it shows.
/// Public so rigs and screenshot examples can assert on the stack's contents.
#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct ObjectiveStackChip(pub String);

/// The stack's `TAB` footer - one per stack, shown while any chip is.
#[derive(Component, Debug, Clone, Reflect)]
struct ObjectiveStackTabMarker;

/// The diamond mark leading a chip. Tagged so the breath and the read-fade can
/// drive its BORDER colour alongside the chip's text.
#[derive(Component, Debug, Clone, Reflect)]
struct ObjectiveStackDiamondMarker;

/// One posted objective's notification state.
///
/// ONE clock: the chip is on screen from the posting frame, so `age_secs`
/// seeds the pop AND runs the read dwell. (It used to take two - the chip
/// waited behind a diegetic reveal card and its pop/dwell ran from the card's
/// tuck - until that card was retired.)
#[derive(Clone, Debug, PartialEq)]
struct ObjectiveNotification {
    /// The objective's id, the identity this is tracked by.
    id: String,
    /// The text shown on the chip - re-posting on a change compares this.
    message: String,
    /// Seconds since it was posted (or re-posted).
    age_secs: f32,
    /// Seconds since it was read; `None` while it is still unread.
    read_secs: Option<f32>,
}

impl ObjectiveNotification {
    /// Alpha to render at: full while unread, fading out once read.
    fn alpha(&self) -> f32 {
        match self.read_secs {
            None => 1.0,
            Some(read) => (1.0 - read / OBJECTIVE_FADE_SECS).clamp(0.0, 1.0),
        }
    }

    /// Whether the fade has finished and the chip should leave the stack.
    fn gone(&self) -> bool {
        self.read_secs
            .is_some_and(|read| read >= OBJECTIVE_FADE_SECS)
    }

    /// Mark read (no-op if it already is, so the NOVA OS sweep cannot restart
    /// a fade that is already running).
    fn mark_read(&mut self) {
        if self.read_secs.is_none() {
            self.read_secs = Some(0.0);
        }
    }
}

/// The posted-objective notifications currently on screen, newest LAST (the
/// stack renders newest on top). Read state lives here rather than on the chip
/// entities because the chips are rebuilt from this list every frame.
#[derive(Resource, Clone, Debug, Default)]
pub struct ObjectiveNotifications {
    shown: Vec<ObjectiveNotification>,
    /// Objective ids already posted at least once, so a chip that has been
    /// read is not re-posted every time `GameObjectives` is touched. Cleared
    /// with the feed on scenario teardown.
    seen: Vec<(String, String)>,
}

impl ObjectiveNotifications {
    /// How many chips are on screen (including ones mid-fade).
    pub fn shown(&self) -> usize {
        self.shown.len()
    }

    /// Whether `id` currently has a chip up.
    pub fn contains(&self, id: &str) -> bool {
        self.shown.iter().any(|shown| shown.id == id)
    }

    /// Whether `id`'s chip is still unread (a test seam; a read chip is
    /// fading and about to leave).
    pub fn is_unread(&self, id: &str) -> bool {
        self.shown
            .iter()
            .any(|shown| shown.id == id && shown.read_secs.is_none())
    }
}

/// UI bundle for the stack: a top-centre column the chips are reconciled into.
/// Empty until an objective posts.
pub fn objective_stack_hud() -> impl Bundle {
    (
        Name::new("ObjectiveStackHUD"),
        ObjectiveStackHudMarker,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(STACK_TOP_PX),
            left: Val::Px(0.0),
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(4.0),
            ..default()
        },
        Pickable::IGNORE,
    )
}

/// Drives the objective notification stack: posting detection, the read
/// lifecycle and the rendered chips. Inits [`ObjectiveNotifications`]; runs
/// `post_objective_notifications`, `age_objective_notifications`,
/// `read_on_nova_os`, `sync_objective_chips` and `breathe_objective_chips` in
/// Update within [`NovaHudSystems`].
pub struct ObjectiveStackPlugin;

impl Plugin for ObjectiveStackPlugin {
    fn build(&self, app: &mut App) {
        debug!("ObjectiveStackPlugin: build");

        app.init_resource::<ObjectiveNotifications>();
        app.register_type::<ObjectiveStackHudMarker>();
        app.register_type::<ObjectiveStackChip>();

        app.add_systems(
            Update,
            (
                // Order matters: post FIRST, then age and the read sweeps, and
                // only then render - so a posting's chip is on screen the same
                // frame the objective arrives, not one frame late.
                post_objective_notifications.run_if(resource_changed::<GameObjectives>),
                age_objective_notifications,
                read_on_nova_os,
                sync_objective_chips,
                breathe_objective_chips,
            )
                .chain()
                .in_set(NovaHudSystems),
        );
    }
}

/// Post a chip for every objective that is new or has been RE-WORDED, and drop
/// chips whose objective is gone.
///
/// A removed objective is completed or cleared: its chip leaves rather than
/// re-posting, because the completion cue is `objective_feedback`'s green ghost
/// and a chip for an objective that no longer exists would be stale text.
/// Emptying the feed entirely is scenario teardown, which resets the whole
/// stack (`state-diff-aliases-reset`, like the comms panel).
fn post_objective_notifications(
    objectives: Res<GameObjectives>,
    mut notifications: ResMut<ObjectiveNotifications>,
) {
    if objectives.objectives.is_empty() {
        // Teardown: drop the queue AND the seen list, so the next scenario's
        // objectives post fresh instead of being mistaken for read ones.
        if !notifications.shown.is_empty() || !notifications.seen.is_empty() {
            *notifications = ObjectiveNotifications::default();
        }
        return;
    }

    let live: Vec<(String, String)> = objectives
        .objectives
        .iter()
        .map(|objective| (objective.id.clone(), objective.message.clone()))
        .collect();

    for (id, message) in &live {
        let already = notifications
            .seen
            .iter()
            .any(|(seen_id, seen_message)| seen_id == id && seen_message == message);
        if already {
            continue;
        }
        // New id, or the same id with new words: post it (re-posting resets an
        // existing chip to unread rather than stacking a second copy).
        notifications.seen.retain(|(seen_id, _)| seen_id != id);
        notifications.seen.push((id.clone(), message.clone()));
        notifications.shown.retain(|shown| &shown.id != id);
        notifications.shown.push(ObjectiveNotification {
            id: id.clone(),
            message: message.clone(),
            age_secs: 0.0,
            read_secs: None,
        });
    }

    // An objective that left the list is done: its chip goes with it.
    notifications
        .shown
        .retain(|shown| live.iter().any(|(id, _)| id == &shown.id));
    notifications
        .seen
        .retain(|(id, _)| live.iter().any(|(live_id, _)| live_id == id));
}

/// Age every notification, mark the ones past their dwell read, and drop the
/// ones whose fade has finished.
fn age_objective_notifications(time: Res<Time>, mut notifications: ResMut<ObjectiveNotifications>) {
    if notifications.shown.is_empty() {
        return;
    }
    let delta = time.delta_secs();
    for shown in &mut notifications.shown {
        shown.age_secs += delta;
        // The dwell runs from the POSTING: the chip is on screen from that
        // frame, so age IS how long the player has had it to read.
        match shown.read_secs {
            Some(ref mut read) => *read += delta,
            None if shown.age_secs >= OBJECTIVE_DWELL_SECS => shown.mark_read(),
            None => {}
        }
    }
    notifications.shown.retain(|shown| !shown.gone());
}

/// Opening the NOVA OS computer reads EVERY notification at once: the standing
/// objective list is in there, so once you have opened it they have served
/// their purpose.
fn read_on_nova_os(
    pause: Res<State<crate::PauseStates>>,
    mut notifications: ResMut<ObjectiveNotifications>,
) {
    if !pause.is_changed() || *pause.get() != crate::PauseStates::NovaOs {
        return;
    }
    for shown in &mut notifications.shown {
        shown.mark_read();
    }
}

/// Rebuild the rendered chips from [`ObjectiveNotifications`], newest on top.
///
/// Rebuild-per-frame like the comms stack, for the same reason: the list is
/// small and the alternative is reconciling rows by id. The emphasis on each
/// chip is therefore seeded from its AGE
/// ([`HudEmphasis::popped_at_age`]) - a fresh component would restart its ease
/// every frame and never leave rest.
fn sync_objective_chips(
    notifications: Res<ObjectiveNotifications>,
    assets: Option<Res<NovaHudAssets>>,
    mut commands: Commands,
    q_stack: Query<Entity, With<ObjectiveStackHudMarker>>,
) {
    let Ok(stack) = q_stack.single() else {
        return;
    };
    commands.entity(stack).despawn_related::<Children>();
    if notifications.shown.is_empty() {
        return;
    }

    let tab_cap = assets
        .as_deref()
        .and_then(|assets| assets.key_glyphs.get("Tab"));
    commands.entity(stack).with_children(|stack| {
        // Newest on top: the freshest posting is the one to read first.
        for shown in notifications.shown.iter().rev() {
            stack.spawn(objective_chip(shown));
        }
        // One TAB affordance for the whole stack, riding it: it says "the full
        // list is in the computer", and it leaves when the last chip does.
        stack.spawn(tab_footer(tab_cap.clone()));
    });
}

/// One objective chip: the amber demo-2 chip, diamond + the objective text.
///
/// No `// <range>` suffix (demo 2 mocks one): an [`Objective`] carries no link
/// to the world entity it is about - `ObjectiveMarkerTarget` has a free-form
/// label and no objective id - so there is nothing to measure a distance to.
/// Range stays the world-anchored marker chip's job, which HAS the target.
fn objective_chip(shown: &ObjectiveNotification) -> impl Bundle {
    let alpha = shown.alpha();
    (
        Name::new("ObjectiveStackChip"),
        ObjectiveStackChip(shown.id.clone()),
        // The posting pop, seeded from the chip's age so a rebuilt node keeps
        // playing the same one-shot.
        HudEmphasis::popped_at_age(CHIP_POP_SCALE, CHIP_POP_SECS, shown.age_secs),
        chip_node(),
        // NOT `chip_paint(ChipTone::Amber)`: that bundle already carries a
        // BackgroundColor + BorderColor, and a second pair in the same bundle
        // is a duplicate-component panic at spawn. The fade needs alpha-scaled
        // colours, so the chip paints itself from the same tone.
        BackgroundColor(
            ChipTone::Amber
                .fill()
                .with_alpha(ChipTone::Amber.fill().alpha() * alpha),
        ),
        BorderColor::all(
            ChipTone::Amber
                .border()
                .with_alpha(ChipTone::Amber.border().alpha() * alpha),
        ),
        Pickable::IGNORE,
        children![
            (
                Name::new("ObjectiveStackDiamond"),
                ObjectiveStackDiamondMarker,
                Node {
                    width: Val::Px(DIAMOND_PX),
                    height: Val::Px(DIAMOND_PX),
                    border: UiRect::all(Val::Px(DIAMOND_BORDER_PX)),
                    margin: UiRect::right(Val::Px(3.0)),
                    ..default()
                },
                UiTransform {
                    rotation: Rot2::degrees(45.0),
                    ..default()
                },
                BorderColor::all(chip_alpha(alpha)),
                Pickable::IGNORE,
            ),
            (
                Name::new("ObjectiveStackLabel"),
                Text::new(shown.message.to_uppercase()),
                TextFont::from_font_size(CHIP_FONT_PX),
                TextLayout {
                    linebreak: LineBreak::NoWrap,
                    ..default()
                },
                TextColor(chip_alpha(alpha)),
                Pickable::IGNORE,
            ),
        ],
    )
}

/// The chip's text/mark colour at `factor` of full opacity.
///
/// NOTE: ONE convention for both the read-fade and the breath - a
/// FRACTION of whatever the tone renders at rest, never an absolute alpha. The
/// two paths agreed only because the amber tone happens to be fully opaque
/// today - give it a sub-1.0 alpha and an absolute fade would render a read
/// chip BRIGHTER than an unread one.
fn chip_alpha(factor: f32) -> Color {
    let text = ChipTone::Amber.text();
    text.with_alpha(text.alpha() * factor)
}

/// The stack's TAB footer: the keycap (or the word, on a rig with no glyphs)
/// plus a muted hint that the full list lives in the computer.
fn tab_footer(cap: Option<KeyCap>) -> impl Bundle {
    (
        Name::new("ObjectiveStackTab"),
        ObjectiveStackTabMarker,
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(5.0),
            ..default()
        },
        Pickable::IGNORE,
        children![
            (
                Name::new("ObjectiveStackTabKey"),
                match cap {
                    // One shared sizing path with the dock and the cues: the
                    // cap decides its own box from TAB_GLYPH_PX.
                    Some(cap) => cap.node(TAB_GLYPH_PX),
                    None => (
                        ImageNode::default(),
                        Node {
                            display: Display::None,
                            ..default()
                        },
                    ),
                },
                Pickable::IGNORE,
            ),
            (
                Name::new("ObjectiveStackTabLabel"),
                Text::new("OBJECTIVES"),
                TextFont::from_font_size(TAB_FONT_PX),
                // The AMBER dim tone, not the phosphor the retired hint used:
                // the footer belongs to the objective notification above it, and
                // a green word under an amber chip reads as a second element.
                TextColor(ChipTone::Amber.unit()),
                Pickable::IGNORE,
            ),
        ],
    )
}

/// The slow breath an UNREAD chip settles into once its pop is done - demo 2's
/// `.obj:not(.emph)` animation. A chip mid-pop or mid-fade is left alone: the
/// pop is the emphasis while it plays, and the fade owns the alpha after.
fn breathe_objective_chips(
    time: Res<Time>,
    notifications: Res<ObjectiveNotifications>,
    q_chips: Query<(&ObjectiveStackChip, &HudEmphasis, &Children)>,
    mut q_text: Query<&mut TextColor>,
    mut q_diamond: Query<&mut BorderColor, With<ObjectiveStackDiamondMarker>>,
) {
    if notifications.shown.is_empty() {
        return;
    }
    let phase = time.elapsed_secs() * std::f32::consts::TAU / CHIP_BREATH_PERIOD_SECS;
    let wave = CHIP_BREATH_MIN_ALPHA + (1.0 - CHIP_BREATH_MIN_ALPHA) * (0.5 + 0.5 * phase.sin());

    for (chip, emphasis, children) in &q_chips {
        let Some(shown) = notifications.shown.iter().find(|shown| shown.id == **chip) else {
            continue;
        };
        // Read chips are fading (sync_objective_chips owns their alpha) and a
        // popping chip is already the emphasis; only a settled unread chip
        // breathes.
        let alpha = if shown.read_secs.is_some() || emphasis.popping() {
            continue;
        } else {
            chip_alpha(wave)
        };
        for &child in children {
            if let Ok(mut color) = q_text.get_mut(child) {
                color.0 = alpha;
            }
            if let Ok(mut border) = q_diamond.get_mut(child) {
                *border = BorderColor::all(alpha);
            }
        }
    }
}

/// Spawn the stack with the player ship, like the other HUD widgets.
pub(super) fn setup_objective_stack(
    add: On<Add, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_spaceship: Query<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
) {
    if q_spaceship.get(add.entity).is_err() {
        return;
    }
    commands.spawn((HudTier::Chrome, objective_stack_hud()));
}

/// Despawn the stack with the player ship, and reset the notifications so a
/// respawned HUD does not inherit the last ship's unread chips.
pub(super) fn remove_objective_stack(
    _remove: On<Remove, PlayerSpaceshipMarker>,
    mut commands: Commands,
    mut notifications: ResMut<ObjectiveNotifications>,
    q_stack: Query<Entity, With<ObjectiveStackHudMarker>>,
) {
    for stack in &q_stack {
        commands.entity(stack).despawn();
    }
    *notifications = ObjectiveNotifications::default();
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use bevy::{state::app::StatesPlugin, time::TimeUpdateStrategy};
    use bevy_common_systems::prelude::Objective;

    use super::*;

    /// Virtual time each `app.update()` actually advances in this rig.
    /// MEASURED, not assumed (`manual-time-rig` lesson): the strategy below
    /// asks for 0.5 s but `Time<Virtual>`'s `max_delta` clamps every frame to
    /// 0.25 s, and the first frame advances 0.0. Driving the dwell off the
    /// requested 0.5 would run the loops at half the intended time.
    const FRAME_SECS: f32 = 0.25;

    /// Advance at least `secs` of virtual time.
    fn advance(app: &mut App, secs: f32) {
        for _ in 0..=(secs / FRAME_SECS).ceil() as usize {
            app.update();
        }
    }

    /// A headless app with the stack's own wiring: the notification lifecycle
    /// plus the rendered chips, on a manual clock so the dwell is
    /// deterministic.
    fn stack_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(StatesPlugin);
        app.init_state::<crate::PauseStates>();
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            0.5,
        )));
        app.init_resource::<GameObjectives>();
        app.init_resource::<ObjectiveNotifications>();
        app.add_systems(
            Update,
            (
                post_objective_notifications.run_if(resource_changed::<GameObjectives>),
                age_objective_notifications,
                read_on_nova_os,
                sync_objective_chips,
                breathe_objective_chips,
            )
                .chain(),
        );
        // The shared emphasis driver, in PostUpdate exactly where the real
        // NovaHudPlugin puts it - its absence from this rig is what let a DEAD
        // pop ship. NOTE: it must not go in the Update chain
        // above: the chips come from `sync_objective_chips`'s commands, which
        // are not applied until the schedule's next sync point, so an in-chain
        // driver would never see this frame's chips and every scale would read
        // the default 1.0.
        app.add_systems(PostUpdate, super::super::emphasis::drive_hud_emphasis);
        app.world_mut().spawn(objective_stack_hud());
        app.update();
        app
    }

    fn post(app: &mut App, id: &str, message: &str) {
        app.world_mut()
            .resource_mut::<GameObjectives>()
            .objectives
            .push(Objective::new(id, message));
    }

    fn chip_scale(app: &mut App) -> Option<f32> {
        app.world_mut()
            .query_filtered::<&UiTransform, With<ObjectiveStackChip>>()
            .iter(app.world())
            .map(|transform| transform.scale.x)
            .next()
    }

    fn chip_text_alpha(app: &mut App) -> Option<f32> {
        let mut q = app
            .world_mut()
            .query_filtered::<&Children, With<ObjectiveStackChip>>();
        let children: Vec<Entity> = q.iter(app.world()).next()?.to_vec();
        children
            .into_iter()
            .find_map(|child| Some(app.world().entity(child).get::<TextColor>()?.0.alpha()))
    }

    fn chip_labels(app: &mut App) -> Vec<String> {
        let mut q = app
            .world_mut()
            .query_filtered::<&Children, With<ObjectiveStackChip>>();
        let chips: Vec<Vec<Entity>> = q.iter(app.world()).map(|c| c.to_vec()).collect();
        chips
            .into_iter()
            .filter_map(|children| {
                children
                    .into_iter()
                    .find_map(|child| Some(app.world().entity(child).get::<Text>()?.0.clone()))
            })
            .collect()
    }

    fn chip_ids(app: &mut App) -> Vec<String> {
        app.world_mut()
            .query::<&ObjectiveStackChip>()
            .iter(app.world())
            .map(|chip| (**chip).clone())
            .collect()
    }

    /// A posting puts the objective's own TEXT on screen - the whole point of
    /// the task (the retired hint showed a count) - on the SAME frame, like a
    /// chat notification.
    #[test]
    fn a_posting_shows_the_objective_text_not_a_count() {
        let mut app = stack_app();
        assert!(chip_ids(&mut app).is_empty(), "nothing before a posting");

        post(&mut app, "salvage", "Salvage the wreck");
        app.update();

        assert_eq!(
            chip_labels(&mut app),
            vec!["SALVAGE THE WRECK".to_string()],
            "the chip carries the objective's message"
        );
    }

    /// Several objectives stack as several chips, newest on top.
    #[test]
    fn multiple_objectives_stack_newest_first() {
        let mut app = stack_app();
        post(&mut app, "first", "Scan the relay");
        app.update();
        post(&mut app, "second", "Salvage the wreck");
        app.update();

        assert_eq!(
            chip_ids(&mut app),
            vec!["second".to_string(), "first".to_string()],
            "both are up, newest on top"
        );
    }

    /// The dwell reads the chip on its own and it leaves after the fade -
    /// nothing else has to happen.
    #[test]
    fn a_chip_reads_itself_after_the_dwell_and_leaves() {
        let mut app = stack_app();
        post(&mut app, "salvage", "Salvage the wreck");
        app.update();
        assert!(
            app.world()
                .resource::<ObjectiveNotifications>()
                .is_unread("salvage"),
            "unread while it is being read"
        );

        advance(&mut app, OBJECTIVE_DWELL_SECS);
        assert!(
            !app.world()
                .resource::<ObjectiveNotifications>()
                .is_unread("salvage"),
            "the dwell elapsed, so it is read"
        );

        advance(&mut app, OBJECTIVE_FADE_SECS);
        assert!(
            chip_ids(&mut app).is_empty(),
            "and the faded chip has left the stack"
        );
    }

    /// Opening the NOVA OS computer reads every chip at once - the other half
    /// of the read model.
    #[test]
    fn opening_nova_os_reads_every_chip() {
        let mut app = stack_app();
        post(&mut app, "first", "Scan the relay");
        post(&mut app, "second", "Salvage the wreck");
        app.update();
        assert_eq!(chip_ids(&mut app).len(), 2, "two chips up");

        app.world_mut()
            .resource_mut::<NextState<crate::PauseStates>>()
            .set(crate::PauseStates::NovaOs);
        app.update();
        let notifications = app.world().resource::<ObjectiveNotifications>();
        assert!(
            !notifications.is_unread("first") && !notifications.is_unread("second"),
            "opening the computer reads them all"
        );

        advance(&mut app, OBJECTIVE_FADE_SECS);
        assert!(
            chip_ids(&mut app).is_empty(),
            "and the stack is empty by the time it closes"
        );
    }

    /// A read chip does not come back on its own, but RE-WORDING its objective
    /// posts it again as unread.
    #[test]
    fn a_read_chip_returns_only_when_its_objective_changes() {
        let mut app = stack_app();
        post(&mut app, "salvage", "Salvage the wreck");
        app.update();
        advance(&mut app, OBJECTIVE_DWELL_SECS + OBJECTIVE_FADE_SECS);
        assert!(chip_ids(&mut app).is_empty(), "read and gone");

        // The objective is still active - touching the resource must NOT
        // re-post it.
        app.world_mut()
            .resource_mut::<GameObjectives>()
            .set_changed();
        app.update();
        assert!(
            chip_ids(&mut app).is_empty(),
            "a read notification stays read while its objective is unchanged"
        );

        // Re-wording it is news again.
        app.world_mut().resource_mut::<GameObjectives>().objectives[0].message =
            "Salvage the wreck - two crates left".to_string();
        app.update();
        assert!(
            app.world()
                .resource::<ObjectiveNotifications>()
                .is_unread("salvage"),
            "a re-worded objective posts again, unread"
        );
    }

    /// A completed objective (gone from the list) takes its chip with it
    /// rather than re-posting stale text; emptying the feed is teardown.
    #[test]
    fn a_completed_objective_takes_its_chip_with_it() {
        let mut app = stack_app();
        post(&mut app, "first", "Scan the relay");
        post(&mut app, "second", "Salvage the wreck");
        app.update();

        app.world_mut()
            .resource_mut::<GameObjectives>()
            .objectives
            .retain(|objective| objective.id != "first");
        app.update();
        assert_eq!(
            chip_ids(&mut app),
            vec!["second".to_string()],
            "the completed objective's chip is gone, the other stays"
        );

        app.world_mut()
            .resource_mut::<GameObjectives>()
            .objectives
            .clear();
        app.update();
        assert!(
            chip_ids(&mut app).is_empty(),
            "teardown resets the whole stack"
        );
    }

    /// The POP is the chip's arrival: it appears on the posting frame already
    /// growing, reaches its peak, and settles back on its own.
    ///
    /// This is the test whose absence let a DEAD pop ship: the pop used to be
    /// written straight onto the chip
    /// entity, which `sync_objective_chips` rebuilds every frame, so it was
    /// overwritten before it could play. Assert the rendered SCALE, not the
    /// intent.
    #[test]
    fn the_chip_pops_on_the_posting_and_settles_back() {
        let mut app = stack_app();
        assert_eq!(chip_scale(&mut app), None, "no chip before a posting");

        post(&mut app, "salvage", "Salvage the wreck");
        app.update();
        // One frame past the posting. NB this rig's 0.25 s frame is COARSER
        // than the emphasis's 0.2 s ease, so the ease-in and ease-out each
        // collapse into a single frame here - the eased SHAPE is pinned in
        // `hud::emphasis`'s own tests; what this test pins is that the pop
        // happens at all, and at the posting.
        app.update();
        assert_eq!(
            chip_scale(&mut app),
            Some(CHIP_POP_SCALE),
            "the posted chip arrives at full pop"
        );

        advance(&mut app, CHIP_POP_SECS);
        assert_eq!(
            chip_scale(&mut app),
            Some(1.0),
            "and the pop settles by itself"
        );
    }

    /// Postings that arrive on DIFFERENT frames each keep their own clock: the
    /// older chip is further through its dwell than the younger one. (The two
    /// retired handover tests pinned that each reveal card handed over its own
    /// objective; with the card gone, the surviving edge is that two live
    /// postings do not share one clock.)
    #[test]
    fn each_posting_runs_its_own_dwell() {
        let mut app = stack_app();
        post(&mut app, "first", "Scan the relay");
        app.update();
        advance(&mut app, OBJECTIVE_DWELL_SECS * 0.75);

        post(&mut app, "second", "Salvage the wreck");
        app.update();
        assert_eq!(
            chip_ids(&mut app),
            vec!["second".to_string(), "first".to_string()],
            "both up, newest on top"
        );

        // Enough for "first" to time out, not enough for "second".
        advance(&mut app, OBJECTIVE_DWELL_SECS * 0.3);
        let notifications = app.world().resource::<ObjectiveNotifications>();
        assert!(
            !notifications.is_unread("first"),
            "the older posting's dwell elapsed first"
        );
        assert!(
            notifications.is_unread("second"),
            "the younger posting is still being read"
        );
    }

    /// A settled UNREAD chip breathes; a read one does not (it is fading, and
    /// two systems driving one alpha is how a chip gets stuck mid-breath).
    #[test]
    fn a_settled_chip_breathes_until_it_is_read() {
        let mut app = stack_app();
        post(&mut app, "salvage", "Salvage the wreck");
        app.update();
        advance(&mut app, CHIP_POP_SECS);

        let rest = ChipTone::Amber.text().alpha();
        let (mut dimmed, mut bright) = (false, false);
        for _ in 0..16 {
            app.update();
            let alpha = chip_text_alpha(&mut app).expect("the chip is up");
            assert!(
                alpha <= rest + f32::EPSILON
                    && alpha >= rest * CHIP_BREATH_MIN_ALPHA - f32::EPSILON,
                "the breath stays inside its band (alpha {alpha})"
            );
            dimmed |= alpha < rest * 0.95;
            bright |= alpha > rest * 0.98;
        }
        assert!(dimmed && bright, "an unread chip breathes both ways");

        // Reading it hands the alpha to the FADE. Assert that directly rather
        // than sampling for non-monotonicity: the fade is short (0.4 s) and at
        // this rig's 0.25 s frame only a frame or two are observable, which is
        // not enough to be sure a breath would have been caught - the first cut
        // of this test sampled two frames and passed with the read-guard
        // REMOVED.
        app.world_mut()
            .resource_mut::<NextState<crate::PauseStates>>()
            .set(crate::PauseStates::NovaOs);
        app.update();
        let alpha = chip_text_alpha(&mut app).expect("still fading");
        let expected = app
            .world()
            .resource::<ObjectiveNotifications>()
            .shown
            .first()
            .expect("the notification is fading")
            .alpha()
            * rest;
        assert!(
            (alpha - expected).abs() < 1e-4,
            "a read chip renders the FADE's alpha ({expected}), not a breath sample ({alpha})"
        );
    }

    /// A RE-WORDED objective shows its chip on the same frame too. It used to
    /// be the awkward case - `objective_feedback` spawned it no reveal card (it
    /// is not an "addition"), so it waited out a timed fallback before showing
    /// at all. Every posting is now the same posting.
    #[test]
    fn a_re_worded_objective_shows_its_chip_on_the_same_frame() {
        let mut app = stack_app();
        post(&mut app, "salvage", "Crates: 0/3");
        app.update();
        advance(&mut app, OBJECTIVE_DWELL_SECS + OBJECTIVE_FADE_SECS);
        assert!(chip_ids(&mut app).is_empty(), "read and gone");

        app.world_mut().resource_mut::<GameObjectives>().objectives[0].message =
            "Crates: 1/3".to_string();
        app.update();
        assert_eq!(
            chip_labels(&mut app),
            vec!["CRATES: 1/3".to_string()],
            "the re-worded posting is up immediately, with the new words"
        );
    }
}

/// The footer's TAB keycap, measured on a real layout pass.
///
/// Tab is one of the WIDE caps (112x74 inside a 128x128 canvas), so this is the
/// site where a square box hurt most: the same shared [`KeyCap`] sizing path the
/// dock and the anchored cues use has to reach here too, or the affordance that
/// tells the player where the objective list lives stays unreadable.
#[cfg(test)]
mod tab_footer_sizing_tests {
    use bevy::ecs::system::RunSystemOnce;
    use bevy_common_systems::prelude::Objective;

    use super::*;
    use crate::hud::{
        chip_layout_rig::{chip_layout_app, load_png, measure, settle},
        key_glyphs::{KeyGlyphs, KEY_GLYPH_DIR},
    };

    /// Tab's opaque bounds inside its canvas, measured OUTSIDE this code base
    /// with `magick T_Tab_Key_Alt.png -alpha extract -threshold 0 -format '%@'
    /// info:` -> `112x74+8+32`.
    const MEASURED_TAB_CAP: [f32; 4] = [8.0, 32.0, 120.0, 106.0];

    #[test]
    fn the_footer_keycap_renders_at_tabs_art_aspect() {
        let cap = Rect::new(
            MEASURED_TAB_CAP[0],
            MEASURED_TAB_CAP[1],
            MEASURED_TAB_CAP[2],
            MEASURED_TAB_CAP[3],
        );

        let mut app = chip_layout_app();
        app.init_resource::<GameObjectives>();
        app.init_resource::<ObjectiveNotifications>();

        let tab = load_png(&mut app, &format!("{KEY_GLYPH_DIR}/T_Tab_Key_Alt.png"));
        let mut key_glyphs =
            KeyGlyphs::from_stems(|stem| (stem == "T_Tab_Key_Alt").then(|| tab.clone()));
        key_glyphs.measure_caps(app.world().resource::<Assets<Image>>());
        app.insert_resource(NovaHudAssets {
            key_glyphs,
            ..default()
        });

        app.world_mut().spawn(objective_stack_hud());
        app.world_mut()
            .resource_mut::<GameObjectives>()
            .objectives
            .push(Objective::new("salvage", "Salvage the wreck"));
        // The production path: posting builds the notification, the sync
        // rebuilds the chips and the footer under the stack root.
        app.world_mut()
            .run_system_once(post_objective_notifications)
            .unwrap();
        app.world_mut()
            .run_system_once(sync_objective_chips)
            .unwrap();
        settle(&mut app);

        let footer = app
            .world_mut()
            .query_filtered::<Entity, With<ObjectiveStackTabMarker>>()
            .single(app.world())
            .expect("the stack spawns exactly one TAB footer");
        let children = app
            .world()
            .entity(footer)
            .get::<Children>()
            .unwrap()
            .to_vec();
        let key = children
            .into_iter()
            .find(|child| app.world().entity(*child).contains::<ImageNode>())
            .expect("the footer carries a keycap node");

        let size = measure(&app, key).size;
        let expected_width = TAB_GLYPH_PX * cap.width() / cap.height();
        assert!(
            (size.y - TAB_GLYPH_PX).abs() <= 0.5,
            "the footer cap is pinned to TAB_GLYPH_PX ({TAB_GLYPH_PX}), got {size:?}"
        );
        // Within a pixel: bevy_ui rounds computed nodes to whole physical
        // pixels.
        assert!(
            (size.x - expected_width).abs() <= 1.0,
            "the footer cap measured {size:?} but Tab's art is {cap:?} - at a \
             pinned height of {TAB_GLYPH_PX} that is {expected_width:.1} px \
             wide, so the footer is not on the shared KeyCap sizing path"
        );
        assert_eq!(
            app.world().entity(key).get::<ImageNode>().unwrap().rect,
            Some(cap),
            "the footer draws the cap sub-rect, not the whole canvas"
        );
    }
}
