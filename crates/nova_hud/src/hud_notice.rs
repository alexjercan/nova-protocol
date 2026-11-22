//! The HUD notice slot: one line of text the game posts once, holds for a
//! moment and fades, with one interface cue.
//!
//! A STATUS holds while its state holds and stays where it is: the mode chip,
//! the speed chip, the keybind dock. A NOTICE says that something just
//! happened, so a status surface never swaps its own text for it. A station's
//! dock refusal ([`FlightVerbHints::dock_refusal`]) posts here when it starts
//! or names another station; the DOCK chip only goes dark. A held job whose
//! work is done ([`JobDone`]) posts `JOB-n done: hand in at STN-m`, naming
//! no credits: the hand-in pays. A paid hand-in ([`JobHandedIn`]) posts
//! `JOB-n PAID 1,460 CR` without the cue ([`HudNotice::silent`]): the Jobs
//! pane plays the payout sound, and the pane hides the HUD and holds its
//! clock, so the line shows for its whole dwell once the pane shuts. A hail
//! job has no station hand-in: its done posts nothing, and the dock with
//! its caller that pays it, or the pane's hand-in while that dock holds,
//! posts the same `PAID` line with the payout sound
//! ([`HudNotice::payout`]): the pane plays none for it. The role goal's
//! current step turning met ([`GoalStepDone`]) posts `GOAL-n done: hand in`
//! with the cue, and its hand-in ([`GoalStepHandedIn`]) posts `GOAL-n PAID
//! 3,000 CR` silent, as a job's do; both wait behind the shown line
//! ([`HudNotice::waiting`]). A hull that hails the
//! player ([`HailRaised`]) posts `HAIL <CALLER>`. A dock
//! with a held supply job's
//! caller that delivers nothing ([`SupplyRefused`]) posts what the hold
//! lacks, `JOB-n: SHORT 3 MEDKIT`, once per dock; it is raised after the
//! clamp closes, so it replaces that dock's `DOCKED <NAME>`. A MATCH refused
//! for its target,
//! RCS or magazine, or cut short ([`MatchRefusal`]), posts why. The gates the
//! chord shares with STOP (no STOP capability, docked without the helm, the
//! docked partner) stay silent, as STOP does. A `DOCK` press that did nothing
//! ([`DockPressRefusal`]) posts why, once per press; a target that refuses
//! writes no press line, so a station's line is never said twice. The
//! player's clamp closing ([`DockClamped`]) posts `DOCKED <NAME>` and letting
//! go on a request ([`DockReleased`]) posts `UNDOCKED`, on whichever end of
//! the dock the player's hull is; a dock a Load rebuilds and a dock damage
//! takes apart raise neither. A player repair job that ends on its own
//! ([`RepairJobEnded`]) posts how: `HULL-3 REPAIRED: 2 PLATES`,
//! `DOCK-1 REBUILT` or `HULL-3 REPAIR STOPPED: NO PLATES`. A player rescue
//! call that ends on its own ([`RescueCallEnded`]) posts how:
//! `SOS ANSWERED: PAID 1,400 CR` when the answer takes the bill,
//! `SOS FAILED: NO ANSWER` when the station refuses and nothing is taken,
//! `SOS ENDED: DOCKED` when the ship docks before the answer, and
//! `NO FREE BERTH: SHIP FIXED` when the paid berth gives up.
//! A player craft job that ends on its own ([`CraftJobEnded`]) posts how:
//! `CRAFTED 500 PDC ROUND`, `CRAFT STOPPED: NO STONE ORE, 300 MADE` or
//! `CRAFT STOPPED: UNDOCKED`. A cancel posts nothing: the player pressed it
//! in the Ship pane or the Craft view, where its row goes. The player's hold reserve of an ammunition item crossing into
//! LOW or DRY ([`AmmoReserveCrossed`]) posts `PDC ROUND LOW: 400` or
//! `TORPEDO DRY`, once per crossing.
//!
//! One slot: a new text replaces the shown one and plays the cue, unless it
//! was posted silent; the same text again while it shows restarts the dwell
//! and stays quiet. A waiting line is the one exception: posted while a line
//! that does not wait shows, it holds one place behind it and shows when
//! that line ends, and a later waiting line takes its place. So the step
//! that the third bounty's hand-in meets shows `JOB-3 PAID` first, then
//! `GOAL-1 done: hand in`, or `GOAL-1 PAID` if the step was handed in in
//! the same pane visit. The dwell runs on virtual time, so a paused game,
//! and an open interface, holds the notice and the line behind it.
//!
//! The box is centred on the screen midline over the key row, so its centre
//! stays on the dock's centre for any line length. Centred, it reaches left
//! under the comms panel's right end, so the comms stack stands over the
//! box's tallest top on the column floor (`column_floor_px`).

use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_ship::{
    flight::prelude::{MatchRefusal, MatchRefusalType},
    input::prelude::FlightVerbHints,
    sections::prelude::{
        AmmoReserveCrossed, AmmoReserveWarningType, CraftJobEndType, CraftJobEnded, DockClamped,
        DockPressRefusal, DockPressRefusalType, DockReleased, RepairJobEndType, RepairJobEnded,
        RescueCallEnded, RescueEndType,
    },
};
use nova_sound::prelude::*;
use nova_ui::{
    hud::{chip_node, ChipTone},
    theme::ActiveUiTheme,
    widget::{ThemedRadius, UiText},
};

use super::NovaHudSystems;

/// `HudNotice`, the notice spawner and its markers, and `HudNoticePlugin`.
pub mod prelude {
    pub use super::{
        hud_notice_hud, HudNotice, HudNoticeCueType, HudNoticeMarker, HudNoticePlugin,
        HudNoticeText,
    };
}

/// How long a notice holds at full strength (virtual seconds).
const NOTICE_DWELL_SECS: f32 = 4.0;

/// The fade after the dwell, matching the objective chip's and the comms
/// card's tail.
const NOTICE_FADE_SECS: f32 = 0.4;

/// How far off the bottom of the screen the notice box sits: clear above the
/// keybind dock row.
pub(crate) const NOTICE_BOTTOM_PX: f32 = 72.0;

/// The widest the box grows, as a share of the window. Centred, that leaves
/// 27.5 % at each side; a longer line clips instead of wrapping.
const NOTICE_MAX_WIDTH_PERCENT: f32 = 45.0;

/// Notice text size (px).
const NOTICE_FONT_PX: f32 = 13.0;

/// The box height (px): `LineBreak::NoWrap` holds the text to one line of
/// [`NOTICE_FONT_PX`], and `chip_node` adds 10 px of padding and border.
pub(crate) const NOTICE_MAX_HEIGHT_PX: f32 = 26.0;

/// The sound a new notice text plays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HudNoticeCueType {
    /// The notice cue.
    Notice,
    /// The payout sound: credits just came in and no pane is open to play
    /// it.
    Payout,
    /// None: the line's event plays a sound of its own.
    Silent,
}

/// A line to post to the HUD notice slot.
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct HudNotice {
    /// The line.
    pub text: String,
    /// The sound a new text plays.
    pub cue: HudNoticeCueType,
    /// It waits behind a shown line that does not wait
    /// ([`waiting`](Self::waiting)).
    pub waits: bool,
}

impl HudNotice {
    /// A line that plays the notice cue when it shows.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            cue: HudNoticeCueType::Notice,
            waits: false,
        }
    }

    /// A line that shows without a cue: its event has its own sound.
    pub fn silent(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            cue: HudNoticeCueType::Silent,
            waits: false,
        }
    }

    /// A line that plays the payout sound when it shows.
    pub fn payout(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            cue: HudNoticeCueType::Payout,
            waits: false,
        }
    }

    /// This line, waiting: posted while a line that does not wait shows, it
    /// shows, with its cue, when that line ends, and a later waiting line
    /// takes its place. A role goal step's line waits, because the hand-in
    /// that meets a step posts its own `PAID` line the same frame.
    pub fn waiting(self) -> Self {
        Self {
            waits: true,
            ..self
        }
    }
}

/// The notice on screen and how long it has been there, or `None` when the
/// slot is empty.
#[derive(Resource, Debug, Default)]
struct ShownNotice(Option<Notice>);

/// The waiting line behind the shown one, or `None`.
#[derive(Resource, Debug, Default)]
struct QueuedNotice(Option<HudNotice>);

#[derive(Debug)]
struct Notice {
    text: String,
    age_secs: f32,
    /// It was posted waiting: a later waiting line replaces it at once.
    waits: bool,
}

impl Notice {
    /// Full strength through the dwell, then a linear fade to zero.
    fn alpha(&self) -> f32 {
        (1.0 - (self.age_secs - NOTICE_DWELL_SECS) / NOTICE_FADE_SECS).clamp(0.0, 1.0)
    }

    fn gone(&self) -> bool {
        self.age_secs >= NOTICE_DWELL_SECS + NOTICE_FADE_SECS
    }
}

/// Marker for the notice box; spawned by [`hud_notice_hud`].
#[derive(Component, Debug, Clone, Reflect)]
pub struct HudNoticeMarker;

/// Marker for the notice text. Public so probes can read the line the player
/// sees.
#[derive(Component, Debug, Clone, Reflect)]
pub struct HudNoticeText;

/// UI bundle for the notice box: empty and fully transparent until a notice
/// shows. It hides by alpha, never `Display::None`, so showing a notice never
/// relayouts anything else.
pub fn hud_notice_hud() -> impl Bundle {
    let mut node = chip_node();
    node.position_type = PositionType::Absolute;
    node.left = Val::Percent(50.0);
    node.bottom = Val::Px(NOTICE_BOTTOM_PX);
    node.max_width = Val::Percent(NOTICE_MAX_WIDTH_PERCENT);
    node.overflow = Overflow::clip();
    (
        Name::new("HudNotice"),
        HudNoticeMarker,
        node,
        // Back by half the box's own width, so the midline `left` names its
        // centre for a line of any length.
        UiTransform::from_translation(Val2::percent(-50.0, 0.0)),
        ThemedRadius::control(),
        BackgroundColor(Color::NONE),
        BorderColor::all(Color::NONE),
        Pickable::IGNORE,
        children![(
            HudNoticeText,
            Text::new(""),
            TextFont::from_font_size(NOTICE_FONT_PX),
            UiText,
            TextLayout {
                linebreak: LineBreak::NoWrap,
                ..default()
            },
            TextColor(Color::NONE),
            Pickable::IGNORE,
        )],
    )
}

/// The shown notice box as a rect other HUD pieces keep clear of.
///
/// The box's centre and `bottom` are fixed, so only its size needs layout. On
/// the frame the text changes, layout has not measured the new line yet, so
/// the rect takes the widest and tallest box the node allows: a pass before
/// layout would otherwise clear last frame's line.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct HudNoticeKeepOut<'w, 's> {
    shown: Res<'w, ShownNotice>,
    q_box: Query<'w, 's, &'static ComputedNode, With<HudNoticeMarker>>,
    q_text: Query<'w, 's, Ref<'static, Text>, With<HudNoticeText>>,
}

impl HudNoticeKeepOut<'_, '_> {
    /// The box's rect in UI px on a `view` of that size, or `None` while the
    /// slot is empty.
    pub(crate) fn rect(&self, view: Vec2) -> Option<Rect> {
        self.shown.0.as_ref()?;
        let computed = self.q_box.iter().next()?;
        let text = self.q_text.iter().next()?;
        let size = if text.is_changed() {
            Vec2::new(
                view.x * NOTICE_MAX_WIDTH_PERCENT / 100.0,
                NOTICE_MAX_HEIGHT_PX,
            )
        } else {
            computed.size() * computed.inverse_scale_factor()
        };
        let left = (view.x - size.x) / 2.0;
        let bottom = view.y - NOTICE_BOTTOM_PX;
        Some(Rect::new(left, bottom - size.y, left + size.x, bottom))
    }
}

/// Registers [`HudNotice`], posts a station's dock refusal, a MATCH refusal,
/// a refused `DOCK` press, the player's clamp closing and letting go, a
/// held job's done, a paid hand-in, a hail, a refused supply dock and a player repair
/// job's end to it, and runs `post_dock_refusal_notice`, `post_match_refusal_notice`,
/// `post_dock_press_notice`, `post_repair_job_notice`, `post_rescue_notice`,
/// `post_craft_job_notice`, `show_hud_notice` and
/// `paint_hud_notice` chained
/// in Update within [`NovaHudSystems`]. The box itself is a player-scoped
/// widget spawned by `NovaHudPlugin`.
#[derive(Default)]
pub struct HudNoticePlugin;

impl Plugin for HudNoticePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<HudNotice>();
        // Owned by `nova_ship`'s flight layer; registered here as well so the
        // notice reader stands alone. A second registration is a no-op.
        app.add_message::<MatchRefusal>();
        app.add_message::<DockPressRefusal>();
        app.add_message::<RepairJobEnded>();
        app.add_message::<RescueCallEnded>();
        app.add_message::<CraftJobEnded>();
        app.init_resource::<ShownNotice>();
        app.init_resource::<QueuedNotice>();
        app.register_type::<HudNoticeMarker>();
        app.register_type::<HudNoticeText>();
        app.add_observer(post_job_done_notice);
        app.add_observer(post_job_paid_notice);
        app.add_observer(post_goal_step_done_notice);
        app.add_observer(post_goal_step_paid_notice);
        app.add_observer(post_hail_notice);
        app.add_observer(post_supply_refused_notice);
        app.add_observer(post_dock_clamped_notice);
        app.add_observer(post_dock_released_notice);
        app.add_observer(post_ammo_reserve_notice);
        app.add_systems(
            Update,
            (
                post_dock_refusal_notice,
                post_match_refusal_notice,
                post_dock_press_notice,
                post_repair_job_notice,
                post_rescue_notice,
                post_craft_job_notice,
                show_hud_notice,
                paint_hud_notice,
            )
                .chain()
                .in_set(NovaHudSystems),
        );
    }
}

/// Post the refusal line when it starts or names another station. The hints
/// carry it every frame the lock holds, so only the edge posts.
fn post_dock_refusal_notice(
    hints: Res<FlightVerbHints>,
    mut last: Local<Option<String>>,
    mut notices: MessageWriter<HudNotice>,
) {
    if hints.dock_refusal == *last {
        return;
    }
    if let Some(line) = &hints.dock_refusal {
        notices.write(HudNotice::new(line.clone()));
    }
    *last = hints.dock_refusal.clone();
}

/// Post why a MATCH was refused or ended short of the match.
fn post_match_refusal_notice(
    mut refusals: MessageReader<MatchRefusal>,
    mut notices: MessageWriter<HudNotice>,
) {
    for MatchRefusal(refusal) in refusals.read() {
        notices.write(HudNotice::new(match_refusal_line(*refusal)));
    }
}

/// The notice line for a MATCH refusal.
fn match_refusal_line(refusal: MatchRefusalType) -> &'static str {
    match refusal {
        MatchRefusalType::NoTarget => "MATCH: NO TARGET",
        MatchRefusalType::RcsDisabled => "MATCH: NO RCS",
        MatchRefusalType::RcsEmpty => "MATCH: RCS EMPTY",
        MatchRefusalType::TargetLost => "MATCH: TARGET LOST",
        MatchRefusalType::NoFlightComputer => "MATCH: NO FLIGHT COMPUTER",
    }
}

/// Post why a `DOCK` press did nothing.
fn post_dock_press_notice(
    mut refusals: MessageReader<DockPressRefusal>,
    mut notices: MessageWriter<HudNotice>,
) {
    for DockPressRefusal(refusal) in refusals.read() {
        notices.write(HudNotice::new(dock_press_refusal_line(*refusal)));
    }
}

/// The notice line for a refused `DOCK` press.
fn dock_press_refusal_line(refusal: DockPressRefusalType) -> &'static str {
    match refusal {
        DockPressRefusalType::NoLock => "DOCK: NO LOCK",
        DockPressRefusalType::NoFreePort => "DOCK: NO FREE PORT",
        DockPressRefusalType::TooFar => "DOCK: TOO FAR",
        DockPressRefusalType::NotFacing => "DOCK: NOT FACING",
        DockPressRefusalType::TooFast => "DOCK: TOO FAST",
    }
}

/// Post how a player ship's repair job ended, unless the player cancelled it.
fn post_repair_job_notice(
    mut ended: MessageReader<RepairJobEnded>,
    q_player: Query<(), With<PlayerSpaceshipMarker>>,
    mut notices: MessageWriter<HudNotice>,
) {
    for ended in ended.read() {
        if !q_player.contains(ended.ship) {
            continue;
        }
        if let Some(line) = repair_job_end_line(&ended.code, &ended.end) {
            debug!("hud_notice: {line}");
            notices.write(HudNotice::new(line));
        }
    }
}

/// Post how a player ship's rescue call ended, unless the player cancelled
/// it.
fn post_rescue_notice(
    mut ended: MessageReader<RescueCallEnded>,
    q_player: Query<(), With<PlayerSpaceshipMarker>>,
    mut notices: MessageWriter<HudNotice>,
) {
    for ended in ended.read() {
        if !q_player.contains(ended.ship) {
            continue;
        }
        if let Some(line) = rescue_end_line(&ended.end) {
            debug!("hud_notice: {line}");
            notices.write(HudNotice::new(line));
        }
    }
}

/// Post how a player ship's craft job ended, unless the player cancelled
/// it. A job runs only once the item and recipe catalogs load.
fn post_craft_job_notice(
    mut ended: MessageReader<CraftJobEnded>,
    q_player: Query<(), With<PlayerSpaceshipMarker>>,
    items: Option<Res<GameItems>>,
    recipes: Option<Res<GameRecipes>>,
    mut notices: MessageWriter<HudNotice>,
) {
    for ended in ended.read() {
        if !q_player.contains(ended.ship) {
            continue;
        }
        let (Some(items), Some(recipes)) = (items.as_deref(), recipes.as_deref()) else {
            continue;
        };
        if let Some(line) = craft_job_end_line(items, recipes.design(&ended.recipe), &ended.end) {
            debug!("hud_notice: {line}");
            notices.write(HudNotice::new(line));
        }
    }
}

/// The notice line for a craft job of `recipe` that ended on its own;
/// `None` for a cancel, whose pane row goes. A stop names what it made
/// before it, when it made any.
fn craft_job_end_line(
    items: &GameItems,
    recipe: &RecipeDesign,
    end: &CraftJobEndType,
) -> Option<String> {
    let made = |batches: u32| u64::from(batches) * u64::from(recipe.output_count());
    let output = items.design(recipe.output_item()).name.to_uppercase();
    let (batches, why) = match end {
        CraftJobEndType::Crafted { batches } => {
            return Some(format!("CRAFTED {} {output}", made(*batches)));
        }
        CraftJobEndType::Cancelled { .. } => return None,
        CraftJobEndType::Stopped { batches, reason } => (
            *batches,
            match reason {
                CraftRefusalType::ShortInput { item, .. } => {
                    format!("NO {}", items.design(item).name.to_uppercase())
                }
                CraftRefusalType::ShortCredits { .. } => "NO CREDITS".to_string(),
                CraftRefusalType::NotAtCraftStation => "UNDOCKED".to_string(),
                CraftRefusalType::CreditOverflow => "STATION FULL".to_string(),
                CraftRefusalType::Running | CraftRefusalType::BadBatches => "REFUSED".to_string(),
            },
        ),
    };
    Some(if batches == 0 {
        format!("CRAFT STOPPED: {why}")
    } else {
        format!("CRAFT STOPPED: {why}, {} MADE", made(batches))
    })
}

/// The notice line for a rescue call that ended on its own; `None` for a
/// cancel, whose pane row goes.
///
/// A refused call took nothing and leaves the ship stranded, so the pane
/// offers SOS again. A berth that gives up after a paid answer leaves the
/// ship fixed beside the station, to dock by itself.
fn rescue_end_line(end: &RescueEndType) -> Option<String> {
    Some(match end {
        RescueEndType::Answered { paid_cr } => {
            format!("SOS ANSWERED: PAID {}", cr_text(*paid_cr).to_uppercase())
        }
        RescueEndType::Docked => "SOS ENDED: DOCKED".to_string(),
        RescueEndType::Refused => "SOS FAILED: NO ANSWER".to_string(),
        RescueEndType::NoBerth => "NO FREE BERTH: SHIP FIXED".to_string(),
        RescueEndType::Cancelled => return None,
    })
}

/// The notice line for a repair job that ended on its own; `None` for a
/// cancel.
fn repair_job_end_line(code: &str, end: &RepairJobEndType) -> Option<String> {
    let plates = |count: u32| {
        if count == 1 {
            "1 PLATE".to_string()
        } else {
            format!("{count} PLATES")
        }
    };
    Some(match end {
        RepairJobEndType::Repaired { plates: count } => {
            format!("{code} REPAIRED: {}", plates(*count))
        }
        RepairJobEndType::Rebuilt => format!("{code} REBUILT"),
        RepairJobEndType::RepairStopped { reason, .. } => {
            let why = match reason {
                PlateRepairRefusalType::NoPlates | PlateRepairRefusalType::InsufficientPlates => {
                    "NO PLATES"
                }
                PlateRepairRefusalType::Destroyed | PlateRepairRefusalType::NoIntegrity => {
                    "SECTION LOST"
                }
                PlateRepairRefusalType::Full | PlateRepairRefusalType::ExcessPlates => "FULL",
            };
            format!("{code} REPAIR STOPPED: {why}")
        }
        RepairJobEndType::RebuildStopped(refusal) => {
            let why = match refusal {
                SectionRebuildRefusalType::Payment(_) => "PARTS MISSING",
                SectionRebuildRefusalType::Unanchored => "NO MOUNT",
                SectionRebuildRefusalType::SectionNotLost => "NOT LOST",
                SectionRebuildRefusalType::NoRebuildService
                | SectionRebuildRefusalType::NotBuilt => "REFUSED",
            };
            format!("{code} REBUILD STOPPED: {why}")
        }
        RepairJobEndType::Cancelled { .. } => return None,
    })
}

/// The hull the player is docked with, when the player's hull is one end of
/// the dock `port` reported against `partner`: the partner when the port is
/// the player's, the port's own hull when the partner is the player. `None`
/// for a dock between two other hulls.
fn player_dock_partner(
    port: Entity,
    partner: Entity,
    q_child_of: &Query<&ChildOf>,
    q_player: &Query<(), With<PlayerSpaceshipMarker>>,
) -> Option<Entity> {
    let hull = q_child_of.get(port).ok().map(ChildOf::parent)?;
    if q_player.contains(hull) {
        Some(partner)
    } else if q_player.contains(partner) {
        Some(hull)
    } else {
        None
    }
}

/// Post `DOCKED <NAME>` when the player's clamp closes, naming the other hull
/// in the refusal lines' register. A hull with no name posts `DOCKED`.
fn post_dock_clamped_notice(
    clamped: On<DockClamped>,
    q_child_of: Query<&ChildOf>,
    q_player: Query<(), With<PlayerSpaceshipMarker>>,
    q_names: Query<&Name>,
    mut notices: MessageWriter<HudNotice>,
) {
    let Some(partner) =
        player_dock_partner(clamped.entity, clamped.partner, &q_child_of, &q_player)
    else {
        return;
    };
    let line = match q_names.get(partner) {
        Ok(name) => format!("DOCKED {}", name.as_str().to_uppercase()),
        Err(_) => "DOCKED".to_string(),
    };
    debug!("hud_notice: {line}");
    notices.write(HudNotice::new(line));
}

/// Post `UNDOCKED` when the player's dock lets go on a request, from either
/// end.
fn post_dock_released_notice(
    released: On<DockReleased>,
    q_child_of: Query<&ChildOf>,
    q_player: Query<(), With<PlayerSpaceshipMarker>>,
    mut notices: MessageWriter<HudNotice>,
) {
    if player_dock_partner(released.entity, released.partner, &q_child_of, &q_player).is_none() {
        return;
    }
    debug!("hud_notice: UNDOCKED");
    notices.write(HudNotice::new("UNDOCKED"));
}

/// Post `JOB-n done: hand in at STN-m` when a held job's work is done. The
/// poster is charted: the accept docked there. A hail job posts nothing
/// here: it has no hand-in, and the dock that does it pays it.
///
/// # Panics
///
/// When the done job is not held or its poster is not charted: the world
/// raises [`JobDone`] for a held job it just marked, and the accept charted
/// the poster.
fn post_job_done_notice(
    done: On<JobDone>,
    jobs: Res<Jobs>,
    chart: Res<Chart>,
    mut notices: MessageWriter<HudNotice>,
) {
    if done.job.id.is_hail() {
        return;
    }
    let code = jobs
        .mark_code(&done.job.id)
        .expect("a done job is held: the world marks a held job done");
    let station = chart
        .station_code(&done.job.target)
        .expect("a held job's poster is charted: the accept docked there");
    notices.write(HudNotice::new(format!("{code} done: hand in at {station}")));
}

/// Post `JOB-n PAID <reward> CR` when a held job is paid. A station hand-in
/// posts without a cue: the Jobs pane plays the payout sound. A hail job is
/// paid by its caller's dock, mostly with no pane open, so its line plays
/// the payout sound and the pane plays none.
fn post_job_paid_notice(handed_in: On<JobHandedIn>, mut notices: MessageWriter<HudNotice>) {
    let line = job_paid_line(&handed_in.code, handed_in.job.reward.total_cr());
    debug!("hud_notice: {line} ('{}')", handed_in.job.id);
    notices.write(if handed_in.job.id.is_hail() {
        HudNotice::payout(line)
    } else {
        HudNotice::silent(line)
    });
}

/// Post `GOAL-n done: hand in` with the notice cue when the role goal's
/// current step reads met. It waits behind the shown line: the step a job
/// hand-in meets is met while that hand-in's `PAID` line shows.
fn post_goal_step_done_notice(done: On<GoalStepDone>, mut notices: MessageWriter<HudNotice>) {
    let line = format!("GOAL-{} done: hand in", done.step + 1);
    debug!("hud_notice: {line}");
    notices.write(HudNotice::new(line).waiting());
}

/// Post `GOAL-n PAID <reward> CR` when a goal step is handed in, without a
/// cue: the Jobs pane plays the payout sound. It waits behind the shown
/// line and takes the place of a waiting `done` line for the step.
fn post_goal_step_paid_notice(
    handed_in: On<GoalStepHandedIn>,
    mut notices: MessageWriter<HudNotice>,
) {
    let line = job_paid_line(&format!("GOAL-{}", handed_in.step + 1), handed_in.reward_cr);
    debug!("hud_notice: {line}");
    notices.write(HudNotice::silent(line).waiting());
}

/// Post `HAIL <CALLER>` when a hull hails the player with a job.
fn post_hail_notice(raised: On<HailRaised>, mut notices: MessageWriter<HudNotice>) {
    let line = format!("HAIL {}", raised.caller.to_uppercase());
    debug!("hud_notice: {line} ('{}')", raised.job.id);
    notices.write(HudNotice::new(line));
}

/// The notice line of the held job `code` for a hand-in that paid
/// `reward_cr`.
fn job_paid_line(code: &str, reward_cr: u32) -> String {
    format!("{code} PAID {}", cr_text(reward_cr).to_uppercase())
}

/// Post what the hold lacks when a dock with a held supply job's caller
/// delivered nothing: `JOB-n: SHORT <missing> <ITEM>`. An item the catalog
/// lacks reads by its id.
///
/// # Panics
///
/// When the refused job is not held: the world refuses a held job's dock.
fn post_supply_refused_notice(
    refused: On<SupplyRefused>,
    jobs: Res<Jobs>,
    items: Res<GameItems>,
    mut notices: MessageWriter<HudNotice>,
) {
    let code = jobs
        .mark_code(&refused.id)
        .expect("a refused supply job is held: the world refuses a held job's dock");
    let line = supply_refusal_line(&code, &refused.refusal, &items);
    debug!("hud_notice: {line} ('{}')", refused.id);
    notices.write(HudNotice::new(line));
}

/// The notice line of the held job `code` for a refused supply dock.
fn supply_refusal_line(code: &str, refusal: &SupplyRefusalType, items: &GameItems) -> String {
    match refusal {
        SupplyRefusalType::ShortItems { item, need, have } => {
            let name = items
                .get(item)
                .map_or_else(|| item.to_string(), |design| design.name.clone());
            format!("{code}: SHORT {} {}", need - have, name.to_uppercase())
        }
    }
}

/// Post `<ITEM> LOW: <reserve>` or `<ITEM> DRY` when the player's hold
/// reserve of an ammunition item crosses into that level. An item the
/// catalog lacks reads by its id.
fn post_ammo_reserve_notice(
    crossed: On<AmmoReserveCrossed>,
    items: Res<GameItems>,
    q_player: Query<(), With<PlayerSpaceshipMarker>>,
    mut notices: MessageWriter<HudNotice>,
) {
    if !q_player.contains(crossed.ship) {
        return;
    }
    let line = ammo_reserve_line(&crossed, &items);
    debug!("hud_notice: {line}");
    notices.write(HudNotice::new(line));
}

/// The notice line for a reserve crossing.
fn ammo_reserve_line(crossed: &AmmoReserveCrossed, items: &GameItems) -> String {
    let name = items
        .get(&crossed.item)
        .map_or_else(|| crossed.item.to_string(), |design| design.name.clone())
        .to_uppercase();
    match crossed.warning {
        AmmoReserveWarningType::Low => format!("{name} LOW: {}", crossed.reserve),
        AmmoReserveWarningType::Dry => format!("{name} DRY"),
    }
}

/// Age the shown notice and drop it once faded, then take this frame's
/// posts: the last line that does not wait replaces the shown one, and the
/// last waiting line waits behind a shown line that does not wait, or
/// replaces the shown one. A waiting line shows once the slot empties. A
/// new text plays its cue once on the interface route, so the master,
/// interface and harness mutes apply, unless it is silent; the same text
/// again restarts the dwell quietly.
///
/// The slot and the line behind it empty while no notice box exists: the
/// box is player-scoped, and a notice left over from a ship that despawned
/// must not show on the next one's box.
#[expect(
    clippy::too_many_arguments,
    reason = "one system reading the clock, the posts, the slot, the line behind it, the bank and the box"
)]
fn show_hud_notice(
    time: Res<Time<Virtual>>,
    mut notices: MessageReader<HudNotice>,
    mut shown: ResMut<ShownNotice>,
    mut queued: ResMut<QueuedNotice>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    q_box: Query<(), With<HudNoticeMarker>>,
    mut commands: Commands,
) {
    if q_box.is_empty() {
        notices.clear();
        if shown.0.is_some() {
            shown.0 = None;
        }
        if queued.0.is_some() {
            queued.0 = None;
        }
        return;
    }
    // Checked through `Deref` first: an empty slot must not mark the resource
    // changed, or the painter would repaint every frame.
    if shown.0.is_some() {
        let gone = shown.0.as_mut().is_some_and(|notice| {
            notice.age_secs += time.delta_secs();
            notice.gone()
        });
        if gone {
            shown.0 = None;
        }
    }
    let posted: Vec<HudNotice> = notices.read().cloned().collect();
    let mut show = |notice: HudNotice, shown: &mut ShownNotice| {
        let repeat = shown.0.as_ref().is_some_and(|old| old.text == notice.text);
        shown.0 = Some(Notice {
            text: notice.text,
            age_secs: 0.0,
            waits: notice.waits,
        });
        let sound = match notice.cue {
            HudNoticeCueType::Notice => (UiSfx::HudNotice, SoundCategoryType::Notice),
            // The category the Jobs pane plays a station hand-in's payout on.
            HudNoticeCueType::Payout => (UiSfx::Payout, SoundCategoryType::UiConfirm),
            HudNoticeCueType::Silent => return,
        };
        if !repeat {
            if let Some(bank) = bank.as_deref() {
                commands.trigger(PlaySound::ui(bank.get(sound.0), sound.1));
            }
        }
    };
    if let Some(plain) = posted.iter().rev().find(|notice| !notice.waits) {
        show(plain.clone(), &mut shown);
    }
    if let Some(waiting) = posted.iter().rev().find(|notice| notice.waits) {
        if shown.0.as_ref().is_some_and(|notice| !notice.waits) {
            queued.0 = Some(waiting.clone());
        } else {
            queued.0 = None;
            show(waiting.clone(), &mut shown);
        }
    }
    if shown.0.is_none() {
        if let Some(waiting) = queued.0.take() {
            show(waiting, &mut shown);
        }
    }
}

/// Paint the box and its text from the shown notice: amber at the notice's
/// alpha, transparent while the slot is empty.
fn paint_hud_notice(
    shown: Res<ShownNotice>,
    theme: Res<ActiveUiTheme>,
    mut q_box: Query<(&mut BackgroundColor, &mut BorderColor), With<HudNoticeMarker>>,
    mut q_text: Query<(&mut Text, &mut TextColor), With<HudNoticeText>>,
    q_added: Query<(), Added<HudNoticeMarker>>,
) {
    if !shown.is_changed() && !theme.is_changed() && q_added.is_empty() {
        return;
    }
    let (text, alpha) = shown
        .0
        .as_ref()
        .map_or(("", 0.0), |notice| (notice.text.as_str(), notice.alpha()));
    let tone = ChipTone::Amber;
    let scaled = |color: Color| color.with_alpha(color.alpha() * alpha);
    for (mut fill, mut border) in &mut q_box {
        fill.set_if_neq(BackgroundColor(scaled(tone.fill())));
        border.set_if_neq(BorderColor::all(scaled(tone.border(&theme))));
    }
    for (mut line, mut color) in &mut q_text {
        if line.0 != text {
            line.0 = text.to_string();
        }
        color.set_if_neq(TextColor(scaled(tone.text(&theme))));
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use bevy::{state::app::StatesPlugin, time::TimeUpdateStrategy};

    use super::*;

    /// Virtual time each `app.update()` advances in this rig, measured: the
    /// strategy asks for 0.5 s and `Time<Virtual>`'s `max_delta` clamps every
    /// frame to 0.25 s (the first frame advances 0.0).
    const FRAME_SECS: f32 = 0.25;

    /// Which cues reached `PlaySound`, by handle identity and category.
    #[derive(Resource, Default)]
    struct NoticeCues {
        interface: usize,
        payout: usize,
        other: usize,
    }

    /// The notice wiring on a manual clock, with a real UI sound bank and a
    /// `PlaySound` capture.
    fn notice_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(StatesPlugin);
        app.add_plugins(bevy::asset::AssetPlugin::default());
        app.init_asset::<AudioSource>();
        app.init_resource::<ActiveUiTheme>();
        app.init_resource::<FlightVerbHints>();
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            0.5,
        )));
        let bank = SoundBank::load(
            app.world().resource::<AssetServer>(),
            nova_sound::UI_SFX_FILES,
        );
        app.insert_resource(bank);
        app.init_resource::<NoticeCues>();
        app.add_observer(
            |sfx: On<PlaySound>, bank: Res<SoundBank<UiSfx>>, mut cues: ResMut<NoticeCues>| {
                if sfx.sound == bank.get(UiSfx::HudNotice)
                    && sfx.category == SoundCategoryType::Notice
                    && sfx.at == SoundAt::Ui
                {
                    cues.interface += 1;
                } else if sfx.sound == bank.get(UiSfx::Payout)
                    && sfx.category == SoundCategoryType::UiConfirm
                    && sfx.at == SoundAt::Ui
                {
                    cues.payout += 1;
                } else {
                    cues.other += 1;
                }
            },
        );
        app.add_plugins(HudNoticePlugin);
        app.world_mut().spawn(hud_notice_hud());
        app.update();
        app
    }

    fn refuse(app: &mut App, line: Option<&str>) {
        app.world_mut()
            .resource_mut::<FlightVerbHints>()
            .dock_refusal = line.map(str::to_string);
    }

    fn advance(app: &mut App, secs: f32) {
        for _ in 0..(secs / FRAME_SECS).ceil() as usize {
            app.update();
        }
    }

    /// The rendered line and its text alpha.
    fn rendered(app: &mut App) -> (String, f32) {
        let mut q = app
            .world_mut()
            .query_filtered::<(&Text, &TextColor), With<HudNoticeText>>();
        let (text, color) = q.single(app.world()).expect("one notice text");
        (text.0.clone(), color.0.alpha())
    }

    #[test]
    fn a_refusal_posts_one_notice_and_one_cue_then_fades() {
        let mut app = notice_app();
        refuse(&mut app, Some("ENEMY POST REFUSES DOCKING"));
        app.update();

        let (text, alpha) = rendered(&mut app);
        assert_eq!(text, "ENEMY POST REFUSES DOCKING");
        assert_eq!(alpha, 1.0, "a fresh notice shows at full strength");
        assert_eq!(app.world().resource::<NoticeCues>().interface, 1);

        // Still full strength just before the dwell ends.
        advance(&mut app, NOTICE_DWELL_SECS - 2.0 * FRAME_SECS);
        assert_eq!(
            rendered(&mut app).1,
            1.0,
            "the notice holds through its dwell"
        );

        // Fading partway through the fade, gone after it.
        advance(&mut app, 2.0 * FRAME_SECS + FRAME_SECS);
        let (_, fading) = rendered(&mut app);
        assert!(
            fading > 0.0 && fading < 1.0,
            "the notice fades after its dwell, read {fading}"
        );
        advance(&mut app, NOTICE_FADE_SECS);
        assert_eq!(
            rendered(&mut app),
            (String::new(), 0.0),
            "the slot empties after the fade"
        );

        let cues = app.world().resource::<NoticeCues>();
        assert_eq!(cues.interface, 1, "one notice, one cue");
        assert_eq!(cues.other, 0, "the cue plays only on the interface route");
    }

    /// A held job marked done posts `JOB-n done: hand in at STN-m` once, with
    /// one cue and no credits; a `Jobs` restored done, as a Load does, posts
    /// nothing.
    #[test]
    fn a_done_job_posts_one_notice_naming_its_hand_in_station() {
        use nova_events::prelude::{GeneratedBodyId, SectorCoord};

        let mut app = notice_app();
        let station = GeneratedBodyId::new(SectorCoord::ORIGIN, "cluster_0_0_0_station", 0);
        let id = JobId::slot(station.clone(), 0, 0);
        let job = Job {
            id: id.clone(),
            kind: JobType::Escort {
                hauler: GeneratedBodyId::new(
                    SectorCoord::ORIGIN,
                    "cluster_0_0_0_station_job_0_0_hauler",
                    0,
                ),
                cargo_cr: 1_500,
                raiders: 1,
            },
            target: station.clone(),
            leg_m: 0,
            threat: 0,
            reward: JobReward::flat(100),
        };
        let mut jobs = Jobs::default();
        jobs.accept(job.clone());
        app.insert_resource(jobs);
        let mut chart = Chart::default();
        chart.find_station(&station, false, false, None, 0.0);
        app.insert_resource(chart);

        app.world_mut().resource_mut::<Jobs>().mark_done(&id);
        app.world_mut().trigger(JobDone { job });
        app.update();
        assert_eq!(rendered(&mut app).0, "JOB-1 done: hand in at STN-1");
        assert_eq!(app.world().resource::<NoticeCues>().interface, 1);

        advance(&mut app, NOTICE_DWELL_SECS + NOTICE_FADE_SECS + 1.0);
        assert_eq!(rendered(&mut app).0, "", "the notice fades out");

        // A Load restores a done job whole and raises no `JobDone`.
        let restored = app.world().resource::<Jobs>().clone();
        *app.world_mut().resource_mut::<Jobs>() = restored;
        app.update();
        assert_eq!(
            rendered(&mut app).0,
            "",
            "a restored done job posts nothing"
        );
        assert_eq!(
            app.world().resource::<NoticeCues>().interface,
            1,
            "one done, one cue"
        );
    }

    /// A paid hand-in posts `JOB-n PAID <reward> CR`, the terms summed, and
    /// plays no cue: the Jobs pane plays the payout sound. The next notice
    /// plays its cue as before.
    #[test]
    fn a_hand_in_posts_what_it_paid_and_plays_no_notice_cue() {
        use nova_events::prelude::{GeneratedBodyId, SectorCoord};

        let mut app = notice_app();
        let station = GeneratedBodyId::new(SectorCoord::ORIGIN, "cluster_0_0_0_station", 0);
        let job = Job {
            id: JobId::slot(station.clone(), 0, 0),
            kind: JobType::Escort {
                hauler: GeneratedBodyId::new(
                    SectorCoord::ORIGIN,
                    "cluster_0_0_0_station_job_0_0_hauler",
                    0,
                ),
                cargo_cr: 1_600,
                raiders: 1,
            },
            target: station,
            leg_m: 8_000,
            threat: 1,
            reward: JobReward {
                base_cr: 520,
                goods_cr: 400,
                range_cr: 240,
                threat_cr: 300,
            },
        };

        app.world_mut().trigger(JobHandedIn {
            job,
            code: "JOB-2".to_string(),
        });
        app.update();
        assert_eq!(rendered(&mut app), ("JOB-2 PAID 1,460 CR".to_string(), 1.0));
        let cues = app.world().resource::<NoticeCues>();
        assert_eq!(
            (cues.interface, cues.other),
            (0, 0),
            "the paid notice plays no cue"
        );

        refuse(&mut app, Some("RELAY DOCK OCCUPIED"));
        app.update();
        assert_eq!(rendered(&mut app).0, "RELAY DOCK OCCUPIED");
        assert_eq!(app.world().resource::<NoticeCues>().interface, 1);
    }

    /// A hail posts `HAIL <CALLER>` with the notice cue. Its job's done posts
    /// nothing, and its pay posts the `PAID` line with the payout sound once:
    /// no pane is open in space to play it.
    #[test]
    fn a_hail_posts_its_caller_and_its_paid_job_plays_the_payout_sound() {
        use nova_events::prelude::{GeneratedBodyId, SectorCoord};

        let mut app = notice_app();
        let caller = GeneratedBodyId::new(SectorCoord::ORIGIN, "encounter_0_caller", 0);
        let job = Job {
            id: JobId::hail(caller.clone(), 0),
            kind: JobType::Distress {
                caller: caller.clone(),
            },
            target: caller,
            leg_m: 6_000,
            threat: 0,
            reward: JobReward::flat(300),
        };
        app.init_resource::<Chart>();
        app.world_mut().trigger(HailRaised {
            caller: "Vell civilian, adrift".to_string(),
            job: job.clone(),
        });
        app.update();
        assert_eq!(rendered(&mut app).0, "HAIL VELL CIVILIAN, ADRIFT");
        assert_eq!(app.world().resource::<NoticeCues>().interface, 1);

        let mut jobs = Jobs::default();
        jobs.accept(job.clone());
        jobs.mark_done(&job.id);
        app.insert_resource(jobs);
        app.world_mut().trigger(JobDone { job: job.clone() });
        app.update();
        assert_eq!(
            rendered(&mut app).0,
            "HAIL VELL CIVILIAN, ADRIFT",
            "a done hail job names no hand-in"
        );

        app.world_mut().trigger(JobHandedIn {
            job,
            code: "JOB-1".to_string(),
        });
        app.update();
        assert_eq!(rendered(&mut app), ("JOB-1 PAID 300 CR".to_string(), 1.0));
        app.update();
        let cues = app.world().resource::<NoticeCues>();
        assert_eq!(
            (cues.interface, cues.payout, cues.other),
            (1, 1, 0),
            "the paid line plays the payout sound once"
        );
    }

    /// A refused supply dock posts `JOB-n: SHORT <missing> <ITEM>` with one
    /// cue, over the `DOCKED <NAME>` line of the same dock, and again on the
    /// next refused dock.
    #[test]
    fn a_refused_supply_dock_posts_what_the_hold_lacks_over_the_docked_line() {
        use nova_events::prelude::{GeneratedBodyId, SectorCoord};

        let mut app = notice_app();
        app.insert_resource(nova_gameplay::test_support::test_items());
        let (_, player_port) = hull(&mut app, "Kestrel", true);
        let (caller, _) = hull(&mut app, "Vell caller", false);
        let station = GeneratedBodyId::new(SectorCoord::ORIGIN, "cluster_0_0_0_station", 0);
        let id = JobId::slot(station.clone(), 0, 0);
        let mut jobs = Jobs::default();
        jobs.accept(Job {
            id: id.clone(),
            kind: JobType::Supply {
                caller: GeneratedBodyId::new(
                    SectorCoord::ORIGIN,
                    "cluster_0_0_0_station_job_0_0_caller",
                    0,
                ),
                item: ItemRoleType::Medkit.into(),
                count: 4,
            },
            target: station,
            leg_m: 0,
            threat: 0,
            reward: JobReward::flat(700),
        });
        app.insert_resource(jobs);
        let short = || SupplyRefused {
            id: id.clone(),
            refusal: SupplyRefusalType::ShortItems {
                item: ItemRoleType::Medkit.into(),
                need: 4,
                have: 1,
            },
        };

        app.world_mut().trigger(DockClamped {
            entity: player_port,
            partner: caller,
        });
        app.world_mut().trigger(short());
        app.update();
        assert_eq!(rendered(&mut app).0, "JOB-1: SHORT 3 MEDKIT");
        assert_eq!(app.world().resource::<NoticeCues>().interface, 1);

        advance(&mut app, NOTICE_DWELL_SECS + NOTICE_FADE_SECS + 1.0);
        app.world_mut().trigger(short());
        app.update();
        assert_eq!(
            rendered(&mut app).0,
            "JOB-1: SHORT 3 MEDKIT",
            "the next refused dock posts again"
        );
        assert_eq!(app.world().resource::<NoticeCues>().interface, 2);
    }

    /// A hull with a port on it, named.
    fn hull(app: &mut App, name: &str, player: bool) -> (Entity, Entity) {
        let mut root = app.world_mut().spawn(Name::new(name.to_string()));
        if player {
            root.insert(PlayerSpaceshipMarker);
        }
        let root = root.id();
        let port = app.world_mut().spawn(ChildOf(root)).id();
        (root, port)
    }

    /// The player's clamp closing posts `DOCKED <NAME>` once and letting go
    /// posts `UNDOCKED` once, from either end of the dock; a dock between two
    /// other hulls posts nothing.
    #[test]
    fn the_players_dock_posts_docked_and_undocked_once_and_another_pairs_dock_posts_nothing() {
        let mut app = notice_app();
        let (player, player_port) = hull(&mut app, "Tender", true);
        let (station, station_port) = hull(&mut app, "Waystation", false);
        let (raider, raider_port) = hull(&mut app, "Raider", false);

        app.world_mut().trigger(DockClamped {
            entity: player_port,
            partner: station,
        });
        app.update();
        assert_eq!(rendered(&mut app).0, "DOCKED WAYSTATION");
        assert_eq!(app.world().resource::<NoticeCues>().interface, 1);

        // The partner lets go: the player's hull is the other end.
        app.world_mut().trigger(DockReleased {
            entity: station_port,
            partner: player,
        });
        app.update();
        assert_eq!(rendered(&mut app).0, "UNDOCKED");
        assert_eq!(app.world().resource::<NoticeCues>().interface, 2);

        advance(&mut app, NOTICE_DWELL_SECS + NOTICE_FADE_SECS + 1.0);
        app.world_mut().trigger(DockClamped {
            entity: raider_port,
            partner: station,
        });
        app.world_mut().trigger(DockReleased {
            entity: station_port,
            partner: raider,
        });
        app.update();
        assert_eq!(rendered(&mut app).0, "", "another pair's dock is not news");
        assert_eq!(app.world().resource::<NoticeCues>().interface, 2);
    }

    /// A player reserve crossing posts its line with one cue; another ship's
    /// crossing posts nothing.
    #[test]
    fn a_reserve_crossing_posts_the_item_and_what_is_left() {
        let mut app = notice_app();
        app.insert_resource(nova_gameplay::test_support::test_items());
        let (player, _) = hull(&mut app, "Kestrel", true);
        let (raider, _) = hull(&mut app, "Raider", false);
        let crossed = |ship, item: ItemRoleType, warning, reserve| AmmoReserveCrossed {
            ship,
            item: item.into(),
            warning,
            reserve,
        };

        app.world_mut().trigger(crossed(
            raider,
            ItemRoleType::PdcRound,
            AmmoReserveWarningType::Low,
            400,
        ));
        app.update();
        assert_eq!(rendered(&mut app).0, "", "another ship's hold is not news");

        app.world_mut().trigger(crossed(
            player,
            ItemRoleType::PdcRound,
            AmmoReserveWarningType::Low,
            400,
        ));
        app.update();
        assert_eq!(rendered(&mut app).0, "PDC ROUND LOW: 400");
        assert_eq!(app.world().resource::<NoticeCues>().interface, 1);

        app.world_mut().trigger(crossed(
            player,
            ItemRoleType::Torpedo,
            AmmoReserveWarningType::Dry,
            0,
        ));
        app.update();
        assert_eq!(rendered(&mut app).0, "TORPEDO DRY");
        assert_eq!(app.world().resource::<NoticeCues>().interface, 2);
    }

    #[test]
    fn a_held_refusal_does_not_repeat_the_cue() {
        let mut app = notice_app();
        refuse(&mut app, Some("ENEMY POST REFUSES DOCKING"));
        // The hints carry the line every frame the lock holds, past the dwell
        // and the fade.
        advance(&mut app, NOTICE_DWELL_SECS + NOTICE_FADE_SECS + 1.0);
        assert_eq!(app.world().resource::<NoticeCues>().interface, 1);
        assert_eq!(
            rendered(&mut app).0,
            "",
            "a held refusal posts once and leaves"
        );

        // Another station's line is a new notice.
        refuse(&mut app, Some("RELAY DOCK OCCUPIED"));
        app.update();
        assert_eq!(rendered(&mut app).0, "RELAY DOCK OCCUPIED");
        assert_eq!(app.world().resource::<NoticeCues>().interface, 2);
    }

    /// A craft job's end names the output it made, what stopped it, and
    /// what it made before the stop; a cancel posts nothing.
    #[test]
    fn a_craft_job_end_names_its_output_and_why_it_stopped() {
        let items = nova_gameplay::test_support::test_items();
        let recipe = RecipeDesign {
            id: RecipeDesignId::from("base/pdc_round"),
            inputs: vec![
                (ItemRoleType::IronOre.into(), 1),
                (ItemRoleType::StoneOre.into(), 1),
            ],
            output: (ItemRoleType::PdcRound.into(), 100),
            duration_secs: 8.0,
            site: CraftSiteType::Ship,
        };
        let line = |end| craft_job_end_line(&items, &recipe, &end);
        assert_eq!(
            line(CraftJobEndType::Crafted { batches: 5 }).as_deref(),
            Some("CRAFTED 500 PDC ROUND")
        );
        assert_eq!(
            line(CraftJobEndType::Stopped {
                batches: 3,
                reason: CraftRefusalType::ShortInput {
                    item: ItemRoleType::StoneOre.into(),
                    need: 1,
                    have: 0,
                },
            })
            .as_deref(),
            Some("CRAFT STOPPED: NO STONE ORE, 300 MADE")
        );
        assert_eq!(
            line(CraftJobEndType::Stopped {
                batches: 0,
                reason: CraftRefusalType::NotAtCraftStation,
            })
            .as_deref(),
            Some("CRAFT STOPPED: UNDOCKED")
        );
        assert_eq!(line(CraftJobEndType::Cancelled { batches: 2 }), None);
    }

    /// A bounty hand-in that the station paid as `code`.
    fn bounty_paid(code: &str) -> JobHandedIn {
        use nova_events::prelude::{GeneratedBodyId, SectorCoord};

        let station = GeneratedBodyId::new(SectorCoord::ORIGIN, "cluster_0_0_0_station", 0);
        JobHandedIn {
            job: Job {
                id: JobId::slot(station.clone(), 0, 0),
                kind: JobType::Neutralize {
                    hull: GeneratedBodyId::new(SectorCoord::ORIGIN, "cluster_hull", 0),
                    hull_name: "Raider".to_string(),
                },
                target: station,
                leg_m: 0,
                threat: 0,
                reward: JobReward::flat(400),
            },
            code: code.to_string(),
        }
    }

    /// A step turning met posts `GOAL-n done: hand in` with the notice cue,
    /// and its hand-in posts `GOAL-n PAID <reward> CR` in its place at once,
    /// silent: the Jobs pane plays the payout sound.
    #[test]
    fn a_done_step_posts_goal_done_and_a_paid_step_posts_goal_paid() {
        let mut app = notice_app();
        app.world_mut().trigger(GoalStepDone {
            step: 0,
            reward_cr: 3_000,
        });
        app.update();
        assert_eq!(
            rendered(&mut app),
            ("GOAL-1 done: hand in".to_string(), 1.0)
        );
        assert_eq!(app.world().resource::<NoticeCues>().interface, 1);

        app.world_mut().trigger(GoalStepHandedIn {
            step: 0,
            reward_cr: 3_000,
        });
        app.update();
        assert_eq!(
            rendered(&mut app),
            ("GOAL-1 PAID 3,000 CR".to_string(), 1.0)
        );
        let cues = app.world().resource::<NoticeCues>();
        assert_eq!(
            (cues.interface, cues.payout, cues.other),
            (1, 0, 0),
            "the paid line plays no cue"
        );
    }

    /// The third bounty's hand-in meets step 1 while its `JOB-3 PAID` line
    /// shows: the goal's line waits behind it and shows, with its cue, once
    /// the paid line ends, so the player sees both. A step handed in before
    /// then takes the waiting line's place: `GOAL-1 PAID` follows `JOB-3
    /// PAID`, and the done line never shows. A plain notice in between
    /// replaces the shown line and keeps the waiting one.
    #[test]
    fn a_goal_line_waits_behind_the_job_paid_line_and_shows_after_it() {
        let mut app = notice_app();
        app.world_mut().trigger(bounty_paid("JOB-3"));
        app.update();
        app.world_mut().trigger(GoalStepDone {
            step: 0,
            reward_cr: 3_000,
        });
        app.update();
        assert_eq!(
            rendered(&mut app).0,
            "JOB-3 PAID 400 CR",
            "the paid line holds the slot"
        );
        assert_eq!(app.world().resource::<NoticeCues>().interface, 0);

        refuse(&mut app, Some("RELAY DOCK OCCUPIED"));
        app.update();
        assert_eq!(rendered(&mut app).0, "RELAY DOCK OCCUPIED");
        refuse(&mut app, None);

        advance(&mut app, NOTICE_DWELL_SECS + NOTICE_FADE_SECS + FRAME_SECS);
        assert_eq!(
            rendered(&mut app),
            ("GOAL-1 done: hand in".to_string(), 1.0),
            "the goal's line shows once the line before it ends"
        );
        assert_eq!(
            app.world().resource::<NoticeCues>().interface,
            2,
            "the refusal's cue, then the goal line's"
        );

        let mut app = notice_app();
        app.world_mut().trigger(bounty_paid("JOB-3"));
        app.world_mut().trigger(GoalStepDone {
            step: 0,
            reward_cr: 3_000,
        });
        app.update();
        app.world_mut().trigger(GoalStepHandedIn {
            step: 0,
            reward_cr: 3_000,
        });
        app.update();
        assert_eq!(rendered(&mut app).0, "JOB-3 PAID 400 CR");
        advance(&mut app, NOTICE_DWELL_SECS + NOTICE_FADE_SECS + FRAME_SECS);
        assert_eq!(rendered(&mut app).0, "GOAL-1 PAID 3,000 CR");
        assert_eq!(
            app.world().resource::<NoticeCues>().interface,
            0,
            "the done line never showed, so its cue never played"
        );
        advance(&mut app, NOTICE_DWELL_SECS + NOTICE_FADE_SECS + FRAME_SECS);
        assert_eq!(
            rendered(&mut app).0,
            "",
            "nothing waits after the goal's line"
        );
    }

    /// The notice box stands centred on the screen midline for a short and a
    /// long line, on a 1280 and a 1024 px wide window, the same as the key
    /// row's centred chips under it.
    #[test]
    fn a_notice_box_of_any_length_is_centred_on_the_midline() {
        use crate::chip_layout_rig::{chip_layout_app, measure, set_rig_width, settle};

        let mut app = chip_layout_app();
        let notice = app.world_mut().spawn(hud_notice_hud()).id();
        let mut widths = Vec::new();
        for view_width in [1280_u32, 1024] {
            set_rig_width(&mut app, view_width);
            for line in ["UNDOCKED", "HULL-3 REPAIR STOPPED: NO PLATES"] {
                let mut q_text = app
                    .world_mut()
                    .query_filtered::<&mut Text, With<HudNoticeText>>();
                q_text
                    .single_mut(app.world_mut())
                    .expect("one notice text")
                    .0 = line.to_string();
                settle(&mut app);
                let rect = measure(&app, notice).rect;
                let midline = view_width as f32 / 2.0;
                assert!(
                    (rect.center().x - midline).abs() <= 1.0,
                    "`{line}` at {view_width} px: box {rect:?} centres at {} not {midline}",
                    rect.center().x
                );
                widths.push(rect.width());
            }
        }
        assert!(
            widths[1] > 2.0 * widths[0],
            "delivery guard: the long line draws a much wider box: {widths:?}"
        );
    }
}
