//! The HUD notice slot: one line of text the game posts once, holds for a
//! moment and fades, with one interface cue.
//!
//! A STATUS holds while its state holds and stays where it is: the mode chip,
//! the speed chip, the keybind dock. A NOTICE says that something just
//! happened, so a status surface never swaps its own text for it. A station's
//! dock refusal ([`FlightVerbHints::dock_refusal`]) posts here when it starts
//! or names another station; the DOCK chip only goes dark. A held job whose
//! work is done ([`JobDone`]) posts `JOB-n done: hand in at STN-m`, naming
//! no credits: payment stays at the hand-in.
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
use nova_ship::input::prelude::FlightVerbHints;
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
const NOTICE_BOTTOM_PX: f32 = 72.0;

/// The widest the box grows, as a share of the window. From the midline that
/// leaves a 5 % right margin; a longer line clips instead of wrapping.
const NOTICE_MAX_WIDTH_PERCENT: f32 = 45.0;

/// Notice text size (px).
const NOTICE_FONT_PX: f32 = 13.0;

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

/// Registers [`HudNotice`], posts a station's dock refusal and a held job's
/// done to it, and runs `post_dock_refusal_notice`, `show_hud_notice` and
/// `paint_hud_notice` chained in Update within [`NovaHudSystems`]. The box
/// itself is a player-scoped widget spawned by `NovaHudPlugin`.
#[derive(Default)]
pub struct HudNoticePlugin;

impl Plugin for HudNoticePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<HudNotice>();
        app.init_resource::<ShownNotice>();
        app.register_type::<HudNoticeMarker>();
        app.register_type::<HudNoticeText>();
        app.add_observer(post_job_done_notice);
        app.add_systems(
            Update,
            (post_dock_refusal_notice, show_hud_notice, paint_hud_notice)
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
        .mark_code(&done.id)
        .expect("a done job is held: the world marks a held job done");
    let job = jobs
        .job(&done.id)
        .expect("a done job is held: the world marks a held job done");
    let station = chart
        .station_code(&job.target)
        .expect("a held job's poster is charted: the accept docked there");
    notices.write(HudNotice(format!("{code} done: hand in at {station}")));
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
        let id = JobId {
            station: station.clone(),
            slot: 0,
            posting: 0,
        };
        let mut jobs = Jobs::default();
        jobs.accept(Job {
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
        });
        app.insert_resource(jobs);
        let mut chart = Chart::default();
        chart.find_station(&station, false, false, None, 0.0);
        app.insert_resource(chart);

        app.world_mut().resource_mut::<Jobs>().mark_done(&id);
        app.world_mut().trigger(JobDone { id: id.clone() });
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
