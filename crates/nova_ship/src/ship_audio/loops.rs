//! The continuous loops: the drive hum on every thruster, the RCS hiss on the
//! ship root, and the railgun's capacitor bank on the lance.
//!
//! Each pass only reports how hard its source is working, as a
//! [`SoundLoop`]'s intensity. Everything after that belongs to `nova_sound`:
//! the category's level and dB range, the release once the source idles, the
//! distance fade, the pan, the freeze behind a paused sim, and the voice caps.
//! A ship's thrusters share one drive voice through the category's per-ship
//! voice count.

use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_sound::prelude::*;

use super::RAILGUN_CHARGE_TOP_SPEED;
use crate::{
    prelude::*,
    sections::{
        railgun_section::RailgunSectionChargeSound, thruster_section::ThrusterSectionLoopSound,
    },
};

/// How fast a hum's intensity chases the level of a working source, in units
/// per second. Framerate-independent, and slow enough that a throttle change
/// fades rather than clicks. An idle source is not eased: see [`ease_hum`].
const LOOP_SMOOTHING_RATE: f32 = 8.0;

/// Under this, an easing intensity has arrived. Without it an ease would
/// write a float a hair nearer its target every frame for ever.
const LOOP_SETTLE: f32 = 1e-4;

/// Under this a hum's source is idle.
///
/// Any intensity above zero plays at least the bottom of the category's loop
/// dB range, so a source that only approaches zero would hold its hum at that
/// level for ever.
const HUM_IDLE: f32 = 0.01;

/// `current` eased toward `target` by `alpha` of the gap.
fn ease(current: f32, target: f32, alpha: f32) -> f32 {
    let next = current + (target - current) * alpha;
    if (target - next).abs() < LOOP_SETTLE {
        target
    } else {
        next
    }
}

/// Ease `owner`'s hum toward `target`, or start it once the source first works.
///
/// An idle source reports zero on the same frame, and the engine's release
/// ramps the voice out. An intensity eased to zero would hold the loop at the
/// bottom of its dB range for the whole ease, about a second.
///
/// AUTHORED-OR-SILENT: an owner with no authored loop never gets one. An idle
/// source never opens a voice at all: a torpedo coasting at zero headroom is
/// exactly this case, and there are many of them in a salvo.
#[expect(
    clippy::too_many_arguments,
    reason = "one body for both hums: the owner, its authored loop, its category, its live loop and the step"
)]
fn ease_hum(
    commands: &mut Commands,
    asset_server: &AssetServer,
    owner: Entity,
    authored: Option<&AssetRef<AudioSource>>,
    category: SoundCategoryType,
    sound_loop: Option<Mut<SoundLoop>>,
    target: f32,
    alpha: f32,
) {
    let target = if target < HUM_IDLE { 0.0 } else { target };
    match sound_loop {
        Some(mut sound_loop) => {
            let next = if target > 0.0 {
                ease(sound_loop.intensity, target, alpha)
            } else {
                0.0
            };
            if next != sound_loop.intensity {
                sound_loop.intensity = next;
            }
        }
        None if target > 0.0 => {
            let Some(authored) = authored else {
                return;
            };
            commands.entity(owner).try_insert(SoundLoop {
                intensity: target * alpha,
                ..SoundLoop::new(authored.resolve(asset_server), category)
            });
        }
        None => {}
    }
}

/// Drive each thruster's hum from its own throttle.
///
/// A drive hums while its plume is lit. The throttle spools down on an
/// exponential (`FlightSettings::spool_down_rate`), still pushing the hull,
/// and the plume follows it to its dark step ([`plume_bucket`] 0); the hum
/// idles on that same step, so it never outlives the flame the player sees.
///
/// A ship's thrusters share one voice: the drive category allows one per
/// sound per ship, and the loudest thruster holds it.
pub(super) fn drive_thruster_loops(
    mut commands: Commands,
    time: Res<Time>,
    asset_server: Res<AssetServer>,
    mut q_thrusters: Query<
        (
            Entity,
            &ThrusterSectionInput,
            &ThrusterSectionLoopSound,
            Has<SectionInactiveMarker>,
            Option<&mut SoundLoop>,
        ),
        With<ThrusterSectionMarker>,
    >,
) {
    let alpha = (time.delta_secs() * LOOP_SMOOTHING_RATE).clamp(0.0, 1.0);
    for (thruster, input, authored, inactive, sound_loop) in &mut q_thrusters {
        let target = if inactive || plume_bucket(input.0) == 0 {
            0.0
        } else {
            input.0.min(1.0)
        };
        ease_hum(
            &mut commands,
            &asset_server,
            thruster,
            authored.0.as_ref(),
            SoundCategoryType::Drive,
            sound_loop,
            target,
            alpha,
        );
    }
}

/// Drive the RCS hiss from how hard each ship is fine-adjusting.
///
/// ROOT-based and DRIVER-agnostic: it reads the thrust `rcs_burn_system`
/// delivered on the last fixed tick ([`RcsBudget::applied`]), whether the
/// player's modal or the autopilot wrote the command. A command the burn
/// refused - no capability, a suppressed partner, an empty magazine - pushed
/// nothing, so it hisses nothing.
pub(super) fn drive_rcs_loops(
    mut commands: Commands,
    time: Res<Time>,
    asset_server: Res<AssetServer>,
    mut q_ships: Query<
        (
            Entity,
            &ShipFeedbackSounds,
            &RcsBudget,
            Option<&mut SoundLoop>,
        ),
        With<SpaceshipRootMarker>,
    >,
) {
    let alpha = (time.delta_secs() * LOOP_SMOOTHING_RATE).clamp(0.0, 1.0);
    for (root, sounds, budget, sound_loop) in &mut q_ships {
        ease_hum(
            &mut commands,
            &asset_server,
            root,
            sounds.rcs_loop.as_ref(),
            SoundCategoryType::Rcs,
            sound_loop,
            budget.applied.clamp(0.0, 1.0),
            alpha,
        );
    }
}

/// Drive the lance's charge loop from where each gun is in its cycle.
///
/// The hums EASE; this loop must not. A commit has to be audible on the frame
/// it happens - it is the tell the whole weapon is balanced around - and the
/// shot has to cut the bank dead, because a capacitor tail smeared under the
/// report is just mud. So the loop opens at the bottom of its category's range
/// and is removed outright the frame the gun leaves
/// [`RailgunCharge::Charging`].
///
/// The intensity and the playback RATE rise together with progress, to
/// [`RAILGUN_CHARGE_TOP_SPEED`]. The rate is what makes it read as approaching
/// something rather than holding a note, and the loop is authored on even
/// partials so its seam stays silent at any rate in between.
///
/// AUTHORED-OR-SILENT: a lance with no `charge_sound` charges quietly.
pub(super) fn drive_railgun_charge_loops(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut q_railgun: Query<
        (
            Entity,
            &RailgunSectionConfigHelper,
            &RailgunCharge,
            &RailgunSectionChargeSound,
            Has<SectionInactiveMarker>,
            Option<&mut SoundLoop>,
        ),
        With<RailgunSectionMarker>,
    >,
) {
    for (section, config, charge, authored, inactive, sound_loop) in &mut q_railgun {
        if inactive || !matches!(charge, RailgunCharge::Charging { .. }) {
            if sound_loop.is_some() {
                commands.entity(section).try_remove::<SoundLoop>();
            }
            continue;
        }
        let progress = charge.progress(config.charge_seconds).clamp(0.0, 1.0);
        // Never zero while charging: zero is silence, and the bank must be
        // heard from the frame the charge commits.
        let intensity = progress.max(f32::MIN_POSITIVE);
        let rate = 1.0 + (RAILGUN_CHARGE_TOP_SPEED - 1.0) * progress;
        match sound_loop {
            Some(mut sound_loop) => {
                sound_loop.intensity = intensity;
                sound_loop.rate = rate;
            }
            None => {
                let Some(authored) = authored.0.as_ref() else {
                    continue;
                };
                commands.entity(section).try_insert(SoundLoop {
                    intensity,
                    rate,
                    ..SoundLoop::new(authored.resolve(&asset_server), SoundCategoryType::Charge)
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use nova_sound::LOOP_RELEASE_SECONDS;

    use super::*;

    /// One test frame, long enough that the smoothing moves visibly and short
    /// enough that it still takes several frames to arrive.
    const TEST_FRAME: std::time::Duration = std::time::Duration::from_millis(16);

    /// App rig for the hum passes: the real systems, no audio device. Time is
    /// stepped by hand so the smoothing is deterministic; the first update
    /// still runs at dt 0, so every assertion below takes at least two.
    fn loop_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(TEST_FRAME));
        app.init_asset::<AudioSource>();
        app.add_systems(Update, (drive_thruster_loops, drive_rcs_loops));
        app
    }

    /// The standard authored hum for rig thrusters (the base default's path).
    const RIG_HUM: &str = "base/sounds/thruster_loop.wav";
    /// The base RCS loop path.
    const RIG_RCS: &str = "base/sounds/rcs_loop.wav";

    /// `owner`'s loop, if it has one.
    fn loop_of(app: &App, owner: Entity) -> Option<SoundLoop> {
        app.world().get::<SoundLoop>(owner).cloned()
    }

    /// A one-thruster ship; returns the thruster.
    fn spawn_burning_thruster(app: &mut App, throttle: f32) -> Entity {
        let root = app
            .world_mut()
            .spawn((SpaceshipRootMarker, GlobalTransform::default()))
            .id();
        app.world_mut()
            .spawn((
                ThrusterSectionMarker,
                ThrusterSectionInput(throttle),
                ThrusterSectionLoopSound(Some(AssetRef::from(RIG_HUM))),
                ChildOf(root),
            ))
            .id()
    }

    /// Run until the smoothing has caught up, so a test can assert on the level
    /// rather than on the ramp.
    fn settle(app: &mut App) {
        for _ in 0..96 {
            app.update();
        }
    }

    #[test]
    fn a_thruster_hums_at_its_own_throttle_on_the_drive_category() {
        let mut app = loop_app();
        let thruster = spawn_burning_thruster(&mut app, 0.5);
        settle(&mut app);

        let hum = loop_of(&app, thruster).expect("a burning thruster hums");
        let server = app.world().resource::<AssetServer>().clone();
        assert_eq!(hum.sound, server.load(RIG_HUM));
        assert_eq!(hum.category, SoundCategoryType::Drive);
        assert!((hum.intensity - 0.5).abs() < 1e-3, "got {}", hum.intensity);
    }

    #[test]
    fn the_hum_eases_toward_its_target_instead_of_snapping() {
        let mut app = loop_app();
        let thruster = spawn_burning_thruster(&mut app, 1.0);
        app.update(); // first frame: dt = 0

        let mut last = 0.0;
        for _ in 0..5 {
            app.update();
            let level = loop_of(&app, thruster).map_or(0.0, |hum| hum.intensity);
            assert!(
                level >= last && level < 1.0,
                "the intensity must rise toward the target, got {level} after {last}"
            );
            last = level;
        }
        assert!(
            last > 0.0,
            "the intensity must have started chasing the target"
        );
    }

    #[test]
    fn a_throttle_cut_eases_down_and_a_closed_throttle_reports_idle_at_once() {
        let mut app = loop_app();
        let thruster = spawn_burning_thruster(&mut app, 1.0);
        settle(&mut app);

        app.world_mut()
            .get_mut::<ThrusterSectionInput>(thruster)
            .unwrap()
            .0 = 0.4;
        app.update();
        let mid = loop_of(&app, thruster).map(|hum| hum.intensity);
        assert!(
            mid.is_some_and(|level| level > 0.4 && level < 1.0),
            "a working drive's intensity must ease down, not cut, got {mid:?}"
        );

        app.world_mut()
            .get_mut::<ThrusterSectionInput>(thruster)
            .unwrap()
            .0 = 0.0;
        app.update();
        assert_eq!(
            loop_of(&app, thruster).map(|hum| hum.intensity),
            Some(0.0),
            "an idle drive reports zero the same frame: the release is the engine's"
        );
    }

    /// The spool-down tail: a closed throttle leaves the input a small
    /// positive number under the plume's first lit step, and the hum must
    /// reach zero with the plume.
    #[test]
    fn a_thruster_spooled_under_its_dark_plume_step_falls_silent() {
        let mut app = loop_app();
        let thruster = spawn_burning_thruster(&mut app, 1.0);
        settle(&mut app);

        app.world_mut()
            .get_mut::<ThrusterSectionInput>(thruster)
            .unwrap()
            .0 = 0.03;
        assert_eq!(plume_bucket(0.03), 0, "0.03 draws the dark plume step");
        app.update();
        assert_eq!(loop_of(&app, thruster).map(|hum| hum.intensity), Some(0.0));
    }

    #[test]
    fn an_idle_thruster_never_opens_a_voice_at_all() {
        let mut app = loop_app();
        let thruster = spawn_burning_thruster(&mut app, 0.0);
        settle(&mut app);
        assert!(loop_of(&app, thruster).is_none());
    }

    #[test]
    fn an_unauthored_thruster_makes_no_voice() {
        let mut app = loop_app();
        let thruster = spawn_burning_thruster(&mut app, 1.0);
        app.world_mut()
            .entity_mut(thruster)
            .insert(ThrusterSectionLoopSound(None));
        settle(&mut app);
        assert!(loop_of(&app, thruster).is_none());
    }

    /// A ship carrying its RCS voice on the ROOT, delivering `applied` of a
    /// full command.
    fn spawn_rcs_ship(app: &mut App, applied: f32) -> Entity {
        let sounds = ShipFeedbackSounds {
            rcs_loop: Some(AssetRef::from(RIG_RCS)),
            ..Default::default()
        };
        app.world_mut()
            .spawn((
                SpaceshipRootMarker,
                GlobalTransform::default(),
                RcsIntent(Vec3::X),
                RcsBudget {
                    applied,
                    ..default()
                },
                sounds,
            ))
            .id()
    }

    #[test]
    fn the_rcs_loop_follows_the_delivered_burn_and_fades_at_rest() {
        let mut app = loop_app();
        let ship = spawn_rcs_ship(&mut app, 1.0);
        settle(&mut app);
        let hiss = loop_of(&app, ship).expect("a burning ship hisses");
        assert_eq!(hiss.category, SoundCategoryType::Rcs);
        assert!((hiss.intensity - 1.0).abs() < 1e-3);

        app.world_mut().get_mut::<RcsBudget>(ship).unwrap().applied = 0.0;
        settle(&mut app);
        assert_eq!(loop_of(&app, ship).map(|hiss| hiss.intensity), Some(0.0));
    }

    #[test]
    fn the_rcs_loop_is_silent_while_the_burn_delivers_nothing() {
        let mut app = loop_app();
        let ship = spawn_rcs_ship(&mut app, 0.0);
        settle(&mut app);
        assert!(loop_of(&app, ship).is_none());
    }

    /// A 16-bit mono WAV of a quarter second of 1 kHz tone: a clip the engine
    /// can measure, so a voice gets a level.
    fn tone_wav() -> Vec<u8> {
        const SAMPLE_RATE: u32 = 48_000;
        let samples = SAMPLE_RATE / 4;
        let data_len = samples * 2;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
        bytes.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&16u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_len.to_le_bytes());
        for n in 0..samples {
            let phase = std::f32::consts::TAU * 1000.0 * n as f32 / SAMPLE_RATE as f32;
            let value = (0.25 * phase.sin() * f32::from(i16::MAX)) as i16;
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    /// App rig for what the player HEARS: the hum passes over the whole
    /// engine, a listener at the origin, and a measured clip to author the
    /// loops with. Returned warmed up: the first update runs at dt 0, and the
    /// engine measures a clip the frame after it is added.
    fn heard_app() -> (App, Handle<AudioSource>) {
        let mut app = loop_app();
        app.add_plugins(NovaSoundPlugin);
        app.world_mut()
            .spawn((SfxListenerMarker, GlobalTransform::IDENTITY));
        let clip = app
            .world_mut()
            .resource_mut::<Assets<AudioSource>>()
            .add(AudioSource {
                bytes: tone_wav().into(),
            });
        for _ in 0..3 {
            app.update();
        }
        (app, clip)
    }

    /// The level the engine mixes `owner`'s loop at this frame. Zero with no
    /// voice, and zero for a voice the engine holds silent.
    fn heard(app: &mut App, owner: Entity) -> f32 {
        let mut voices = app.world_mut().query::<(&SoundVoice, &VoiceMix)>();
        voices
            .iter(app.world())
            .find(|(voice, _)| voice.place == VoicePlace::Follow(owner))
            .map_or(0.0, |(_, mix)| if mix.capped { 0.0 } else { mix.level })
    }

    /// Run frames after a source was let go and return how many passed before
    /// the loop fell silent for good. `frame` advances the source one tick.
    fn frames_until_silent(app: &mut App, owner: Entity, mut frame: impl FnMut(&mut App)) -> usize {
        let mut silent_after = 0;
        for index in 1..=188 {
            frame(app);
            if heard(app, owner) > 0.0 {
                silent_after = index;
            }
        }
        silent_after
    }

    /// One flight tick and one frame: spool `thruster` toward `throttle` at
    /// the flight rates, as the burn does, then mix.
    fn burn_frame(app: &mut App, thruster: Entity, throttle: f32) {
        let settings = FlightSettings::default();
        let mut input = app
            .world_mut()
            .get_mut::<ThrusterSectionInput>(thruster)
            .unwrap();
        input.0 = crate::flight::spool(
            input.0,
            throttle,
            settings.spool_up_rate,
            settings.spool_down_rate,
            TEST_FRAME.as_secs_f32(),
        );
        app.update();
    }

    /// Hold the burn key for `held` frames and let go. Returns the seconds
    /// after the release at which the plume went dark and at which the hum
    /// fell silent.
    fn thruster_release(held: usize) -> (f32, f32) {
        let (mut app, clip) = heard_app();
        let root = app
            .world_mut()
            .spawn((SpaceshipRootMarker, GlobalTransform::default()))
            .id();
        let thruster = app
            .world_mut()
            .spawn((
                ThrusterSectionMarker,
                ThrusterSectionInput(0.0),
                ThrusterSectionLoopSound(Some(AssetRef::from(clip))),
                GlobalTransform::default(),
                ChildOf(root),
            ))
            .id();
        let mut loudest = 0.0f32;
        for _ in 0..held {
            burn_frame(&mut app, thruster, 1.0);
            loudest = loudest.max(heard(&mut app, thruster));
        }
        assert!(loudest > 0.0, "the burn itself must be heard");

        let mut dark_after = None;
        let mut index = 0;
        let silent_after = frames_until_silent(&mut app, thruster, |app| {
            burn_frame(app, thruster, 0.0);
            index += 1;
            let input = app.world().get::<ThrusterSectionInput>(thruster).unwrap().0;
            if dark_after.is_none() && plume_bucket(input) == 0 {
                dark_after = Some(index);
            }
        });
        let seconds = |frames: usize| frames as f32 * TEST_FRAME.as_secs_f32();
        (
            seconds(dark_after.expect("a closed throttle darkens the plume")),
            seconds(silent_after + 1),
        )
    }

    /// A tap on the burn key must not leave the drive humming. The drive
    /// spools down by design and the hum follows the plume; past a dark plume
    /// the engine's release is all that is left.
    #[test]
    fn a_tapped_thruster_is_silent_within_the_release_time_of_its_plume_going_dark() {
        let frame = TEST_FRAME.as_secs_f32();
        let (dark, silent) = thruster_release(3);
        assert!(
            silent <= dark + LOOP_RELEASE_SECONDS + frame,
            "a tapped drive rang on: plume dark {dark:.3} s after the key, hum silent after \
             {silent:.3} s"
        );
        let (held_dark, held_silent) = thruster_release(96);
        assert!(
            held_silent <= held_dark + LOOP_RELEASE_SECONDS + frame,
            "a held drive rang on: plume dark {held_dark:.3} s after the key, hum silent after \
             {held_silent:.3} s"
        );
        assert!(
            silent <= held_silent,
            "a tap must not ring longer than a full burn: {silent:.3} s against {held_silent:.3} s"
        );
    }

    /// The RCS burn has no spool: the hiss is silent within the release time
    /// of the last delivered push, after a two-frame puff as after a long one.
    #[test]
    fn a_tapped_rcs_burn_is_silent_within_the_release_time() {
        let frame = TEST_FRAME.as_secs_f32();
        for held in [2, 96] {
            let (mut app, clip) = heard_app();
            let sounds = ShipFeedbackSounds {
                rcs_loop: Some(AssetRef::from(clip)),
                ..Default::default()
            };
            let ship = app
                .world_mut()
                .spawn((
                    SpaceshipRootMarker,
                    GlobalTransform::default(),
                    RcsBudget {
                        applied: 1.0,
                        ..default()
                    },
                    sounds,
                ))
                .id();
            let mut loudest = 0.0f32;
            for _ in 0..held {
                app.update();
                loudest = loudest.max(heard(&mut app, ship));
            }
            assert!(loudest > 0.0, "the puff itself must be heard");

            app.world_mut().get_mut::<RcsBudget>(ship).unwrap().applied = 0.0;
            let silent = frames_until_silent(&mut app, ship, App::update) as f32 * frame + frame;
            assert!(
                silent <= LOOP_RELEASE_SECONDS + frame,
                "an RCS burn held {held} frames rang on for {silent:.3} s"
            );
        }
    }

    /// The base lance's charge sound path.
    const RIG_CHARGE: &str = "base/sounds/railgun_charge.wav";

    /// A lance on its own ship, charged to `progress` of a two-second charge.
    /// `sound` is the authored file, or `None` for a gun that voices nothing.
    fn spawn_charging_lance(app: &mut App, sound: Option<&str>, progress: f32) -> Entity {
        let root = app
            .world_mut()
            .spawn((SpaceshipRootMarker, GlobalTransform::default()))
            .id();
        app.world_mut()
            .spawn((
                // The production bundle, so the test reads the same authored
                // snapshot the spawn path writes.
                preview_railgun_section(RailgunSectionConfig {
                    charge_seconds: 2.0,
                    charge_sound: sound.map(AssetRef::from),
                    ..default()
                }),
                RailgunCharge::Charging {
                    elapsed: 2.0 * progress,
                },
                ChildOf(root),
            ))
            .id()
    }

    /// App rig for the capacitor pass. Separate from [`loop_app`] because the
    /// charge loop is deliberately not smoothed.
    fn charge_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.add_systems(Update, drive_railgun_charge_loops);
        app
    }

    #[test]
    fn the_capacitor_opens_audible_and_arrives_full_and_fast() {
        // The one loop that must NOT ease: a bank winding up is heard on the
        // FIRST frame of the charge, and the rise is the charge itself.
        let mut app = charge_app();
        let gun = spawn_charging_lance(&mut app, Some(RIG_CHARGE), 0.0);
        app.update();
        let opened = loop_of(&app, gun).expect("the bank opens a voice");
        assert_eq!(opened.category, SoundCategoryType::Charge);
        assert!(opened.intensity > 0.0, "an opening bank is not silent");
        assert!((opened.rate - 1.0).abs() < 1e-5);

        app.world_mut()
            .entity_mut(gun)
            .insert(RailgunCharge::Charging { elapsed: 2.0 });
        app.update();
        let full = loop_of(&app, gun).expect("the voice stays open");
        assert!((full.intensity - 1.0).abs() < 1e-5);
        assert!((full.rate - RAILGUN_CHARGE_TOP_SPEED).abs() < 1e-5);
    }

    #[test]
    fn the_bank_goes_silent_the_frame_the_charge_ends() {
        let mut app = charge_app();
        let gun = spawn_charging_lance(&mut app, Some(RIG_CHARGE), 0.5);
        app.update();
        assert!(loop_of(&app, gun).is_some());

        app.world_mut().entity_mut(gun).insert(RailgunCharge::Ready);
        app.update();
        assert!(
            loop_of(&app, gun).is_none(),
            "leaving the charge removes the loop outright, with no tail"
        );
    }

    #[test]
    fn a_lance_that_authors_no_charge_sound_winds_up_quietly() {
        let mut app = charge_app();
        let gun = spawn_charging_lance(&mut app, None, 0.5);
        app.update();
        assert!(loop_of(&app, gun).is_none());
    }
}
