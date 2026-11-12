//! The HUD notice slot: one line of text the game posts once, holds for a
//! moment and fades, with one interface cue.
//!
//! A STATUS holds while its state holds and stays where it is: the mode chip,
//! the speed chip, the keybind dock. A NOTICE says that something just
//! happened, so a status surface never swaps its own text for it. A station's
//! dock refusal ([`FlightVerbHints::dock_refusal`]) posts here when it starts
//! or names another station; the DOCK chip only goes dark. A held job whose
//! work is done ([`JobDone`]) posts `JOB-n done: hand in at STN-m`, naming
//! no credits: payment stays at the hand-in. A dock with a held distress
//! call's caller that rescues nobody ([`DistressRefused`]) posts why, once
//! per dock; it is raised after the clamp closes, so it replaces that dock's
//! `DOCKED <NAME>`. A MATCH refused for its target,
//! RCS or magazine, or cut short ([`MatchRefusal`]), posts why. The gates the
//! chord shares with STOP (no STOP capability, docked without the helm, the
//! docked partner) stay silent, as STOP does. A `DOCK` press that did nothing
//! ([`DockPressRefusal`]) posts why, once per press; a target that refuses
//! writes no press line, so a station's line is never said twice. The
//! player's clamp closing ([`DockClamped`]) posts `DOCKED <NAME>` and letting
//! go on a request ([`DockReleased`]) posts `UNDOCKED`, on whichever end of
//! the dock the player's hull is; a dock a Load rebuilds and a dock damage
//! takes apart raise neither.
//!
//! One slot, no queue: a new text replaces the shown one and plays the cue; the
//! same text again while it shows restarts the dwell and stays quiet. The dwell
//! runs on virtual time, so a paused game holds the notice.
//!
//! The box's left edge sits on the screen midline, not centred on it: the comms
//! panel spans the bottom-left from 16 px to 48 % of the width (`comms_panel`),
//! so a centred box at this height would print through a comms card on any
//! window narrower than 2 x the notice width. A box that starts at 50 % clears
//! it on every window wider than 800 px.

use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_ship::{
    flight::prelude::{MatchRefusal, MatchRefusalType},
    input::prelude::FlightVerbHints,
    sections::prelude::{DockClamped, DockPressRefusal, DockPressRefusalType, DockReleased},
};
use nova_ui::{
    hud::{chip_node, ChipTone},
    theme::ActiveUiTheme,
};

use super::NovaHudSystems;

/// `HudNotice`, the notice spawner and its markers, and `HudNoticePlugin`.
pub mod prelude {
    pub use super::{hud_notice_hud, HudNotice, HudNoticeMarker, HudNoticePlugin, HudNoticeText};
}

/// How long a notice holds at full strength (virtual seconds).
const NOTICE_DWELL_SECS: f32 = 4.0;

/// The fade after the dwell, matching the objective chip's and the comms
/// card's tail.
const NOTICE_FADE_SECS: f32 = 0.4;

/// How far off the bottom of the screen the notice box sits: clear above the
/// keybind dock row.
pub(crate) const NOTICE_BOTTOM_PX: f32 = 72.0;

/// The widest the box grows, as a share of the window. From the midline that
/// leaves a 5 % right margin; a longer line clips instead of wrapping.
const NOTICE_MAX_WIDTH_PERCENT: f32 = 45.0;

/// Notice text size (px).
const NOTICE_FONT_PX: f32 = 13.0;

/// The box height (px): `LineBreak::NoWrap` holds the text to one line of
/// [`NOTICE_FONT_PX`], and `chip_node` adds 10 px of padding and border.
pub(crate) const NOTICE_MAX_HEIGHT_PX: f32 = 26.0;

/// Notice cue volume: over the comms blip (0.22), under the objective posting
/// blip (0.30).
const HUD_NOTICE_VOLUME: f32 = 0.26;

/// A line to post to the HUD notice slot.
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct HudNotice(pub String);

/// The notice on screen and how long it has been there, or `None` when the
/// slot is empty.
#[derive(Resource, Debug, Default)]
struct ShownNotice(Option<Notice>);

#[derive(Debug)]
struct Notice {
    text: String,
    age_secs: f32,
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
        BackgroundColor(Color::NONE),
        BorderColor::all(Color::NONE),
        Pickable::IGNORE,
        children![(
            HudNoticeText,
            Text::new(""),
            TextFont::from_font_size(NOTICE_FONT_PX),
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
/// The box's `left` and `bottom` are fixed, so only its size needs layout. On
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
        let left = view.x / 2.0;
        let bottom = view.y - NOTICE_BOTTOM_PX;
        Some(Rect::new(left, bottom - size.y, left + size.x, bottom))
    }
}

/// Registers [`HudNotice`], posts a station's dock refusal, a MATCH refusal,
/// a refused `DOCK` press, the player's clamp closing and letting go, a
/// held job's done and a refused distress rescue to it, and runs
/// `post_dock_refusal_notice`, `post_match_refusal_notice`,
/// `post_dock_press_notice`, `show_hud_notice` and `paint_hud_notice` chained
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
        app.init_resource::<ShownNotice>();
        app.register_type::<HudNoticeMarker>();
        app.register_type::<HudNoticeText>();
        app.add_observer(post_job_done_notice);
        app.add_observer(post_distress_refused_notice);
        app.add_observer(post_dock_clamped_notice);
        app.add_observer(post_dock_released_notice);
        app.add_systems(
            Update,
            (
                post_dock_refusal_notice,
                post_match_refusal_notice,
                post_dock_press_notice,
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
        notices.write(HudNotice(line.clone()));
    }
    *last = hints.dock_refusal.clone();
}

/// Post why a MATCH was refused or ended short of the match.
fn post_match_refusal_notice(
    mut refusals: MessageReader<MatchRefusal>,
    mut notices: MessageWriter<HudNotice>,
) {
    for MatchRefusal(refusal) in refusals.read() {
        notices.write(HudNotice(match_refusal_line(*refusal).to_string()));
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
        notices.write(HudNotice(dock_press_refusal_line(*refusal).to_string()));
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
    notices.write(HudNotice(line));
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
    notices.write(HudNotice("UNDOCKED".to_string()));
}

/// Post `JOB-n done: hand in at STN-m` when a held job's work is done. The
/// poster is charted: the accept docked there.
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
    let code = jobs
        .mark_code(&done.job.id)
        .expect("a done job is held: the world marks a held job done");
    let station = chart
        .station_code(&done.job.target)
        .expect("a held job's poster is charted: the accept docked there");
    notices.write(HudNotice(format!("{code} done: hand in at {station}")));
}

/// Post why a dock with a held distress call's caller rescued nobody.
fn post_distress_refused_notice(
    refused: On<DistressRefused>,
    mut notices: MessageWriter<HudNotice>,
) {
    let line = distress_refusal_line(refused.refusal);
    debug!("hud_notice: {line} ('{}')", refused.id);
    notices.write(HudNotice(line.to_string()));
}

/// The notice line for a refused distress rescue.
fn distress_refusal_line(refusal: DistressRefusalType) -> &'static str {
    match refusal {
        DistressRefusalType::NoMedkit => "NO MEDKIT ABOARD",
    }
}

/// Age the shown notice and take the latest posting into the slot. A new text
/// plays the cue once on the interface route, so the master, interface and
/// harness mutes apply; the same text again only restarts the dwell.
///
/// The slot empties while no notice box exists: the box is player-scoped, and
/// a notice left over from a ship that despawned must not show on the next
/// one's box.
fn show_hud_notice(
    time: Res<Time<Virtual>>,
    mut notices: MessageReader<HudNotice>,
    mut shown: ResMut<ShownNotice>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    q_box: Query<(), With<HudNoticeMarker>>,
    mut commands: Commands,
) {
    if q_box.is_empty() {
        notices.clear();
        if shown.0.is_some() {
            shown.0 = None;
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
    let Some(HudNotice(text)) = notices.read().last() else {
        return;
    };
    let repeat = shown.0.as_ref().is_some_and(|notice| notice.text == *text);
    shown.0 = Some(Notice {
        text: text.clone(),
        age_secs: 0.0,
    });
    if !repeat {
        if let Some(bank) = bank {
            commands.play_sfx(
                bank.get(UiSfx::HudNotice),
                AudioRoute::Interface,
                HUD_NOTICE_VOLUME,
            );
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

    /// Which cues reached `PlaySfx`, by handle identity and route.
    #[derive(Resource, Default)]
    struct NoticeCues {
        interface: usize,
        other: usize,
    }

    /// The notice wiring on a manual clock, with a real UI sound bank and a
    /// `PlaySfx` capture.
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
            nova_gameplay::audio::UI_SFX_FILES,
        );
        app.insert_resource(bank);
        app.init_resource::<NoticeCues>();
        app.add_observer(
            |sfx: On<PlaySfx>, bank: Res<SoundBank<UiSfx>>, mut cues: ResMut<NoticeCues>| {
                if sfx.handle == bank.get(UiSfx::HudNotice) && sfx.route == AudioRoute::Interface {
                    cues.interface += 1;
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
            },
            target: station.clone(),
            reward_cr: 100,
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

    /// A refused distress rescue posts `NO MEDKIT ABOARD` with one cue, and
    /// posts over the `DOCKED <NAME>` line of the same dock.
    #[test]
    fn a_refused_distress_rescue_posts_no_medkit_aboard_over_the_docked_line() {
        use nova_events::prelude::{GeneratedBodyId, SectorCoord};

        let mut app = notice_app();
        let (_, player_port) = hull(&mut app, "Kestrel", true);
        let (caller, _) = hull(&mut app, "Vell caller", false);
        let station = GeneratedBodyId::new(SectorCoord::ORIGIN, "cluster_0_0_0_station", 0);
        let id = JobId::slot(station, 0, 0);

        app.world_mut().trigger(DockClamped {
            entity: player_port,
            partner: caller,
        });
        app.world_mut().trigger(DistressRefused {
            id: id.clone(),
            refusal: DistressRefusalType::NoMedkit,
        });
        app.update();
        assert_eq!(rendered(&mut app).0, "NO MEDKIT ABOARD");
        assert_eq!(app.world().resource::<NoticeCues>().interface, 1);

        advance(&mut app, NOTICE_DWELL_SECS + NOTICE_FADE_SECS + 1.0);
        app.world_mut().trigger(DistressRefused {
            id,
            refusal: DistressRefusalType::NoMedkit,
        });
        app.update();
        assert_eq!(
            rendered(&mut app).0,
            "NO MEDKIT ABOARD",
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
}
