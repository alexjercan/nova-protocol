//! Background music: the [`MusicDeck`] and the voices it plays.
//!
//! The deck plays one track at a time on [`SoundBus::Music`], crossfades to
//! the next, ducks and stops. It knows tracks and fades, not game states:
//! whoever picks the music calls [`MusicDeck::play`], [`MusicDeck::stop`] and
//! [`MusicDeck::duck`], and reads [`MusicDeck::status`] to know when a track
//! ended.
//!
//! A music voice is a [`MusicVoice`], not a [`SoundVoice`](super::SoundVoice),
//! because the one-shot rules do not fit a track:
//!
//! - it is never dropped as inaudible: a track started with the slider at 0
//!   plays on at gain 0, so raising the slider brings it in mid-track;
//! - it is never retired for want of a sink: with no audio device the deck
//!   reports [`MusicDeckStatusType::NoOutput`] and holds, so a headless run
//!   does not see a track end every two seconds;
//! - no repeat interval or voice count applies: the deck holds at most two
//!   voices, the outgoing and the incoming track of a crossfade.
//!
//! It plays through a frozen sim and through the end of a scenario: the
//! freeze and the stop hooks touch world voices only. Its ramps run on the
//! real clock for the same reason. Like every voice it publishes its
//! [`VoiceMix`], and its player is built by the voice module's one
//! constructor.
//!
//! ```text
//! gain = clip loudness gain x Music row level x fade x duck
//!        x Music slider x master
//! ```

use std::{f32::consts::FRAC_PI_2, time::Duration};

use bevy::{
    audio::{AudioSinkPlayback, PlaybackMode, Volume},
    prelude::*,
};

use super::{
    bus::Mixer,
    category::{db_to_gain, SoundBus, SoundCategoryType},
    loudness::SoundLoudness,
    voice::{voice_player, VoiceMarker, VoiceMix},
};

/// How long a music voice may wait for its sink, after its player is built,
/// before the deck reports [`MusicDeckStatusType::NoOutput`].
///
/// Bevy opens a sink in the frame after the player is built when there is an
/// audio device, and never when there is none.
const MUSIC_SINK_GRACE: Duration = Duration::from_secs(2);

/// What the deck's current track is doing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
pub enum MusicDeckStatusType {
    /// No track: nothing was played yet, or the deck was stopped. An
    /// outgoing track may still be fading out.
    #[default]
    Idle,
    /// The track waits for its clip's loudness measure or for its sink.
    Starting,
    /// The track plays.
    Playing,
    /// The track played to its end. The deck holds silent until the next
    /// [`MusicDeck::play`] or [`MusicDeck::stop`].
    Ended,
    /// The track has no sink: there is no audio device. The track holds and
    /// never ends; its mix is still published.
    NoOutput,
}

/// A request the deck's systems apply in the next mixing pass.
#[derive(Clone, Debug)]
enum MusicRequest {
    Play {
        track: Handle<AudioSource>,
        fade: Duration,
    },
    Stop {
        fade: Duration,
    },
}

/// The duck: an attenuation on every music voice, ramped in dB.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct MusicDuck {
    /// The attenuation now, in dB. Zero or below.
    db: f32,
    /// Where the ramp is heading, in dB.
    target: f32,
    /// The ramp speed, in dB per second.
    rate: f32,
}

impl MusicDuck {
    fn step(&mut self, dt: f32) {
        self.db = ramp(self.db, self.target, self.rate, dt);
    }

    fn gain(self) -> f32 {
        db_to_gain(self.db)
    }
}

/// The background-music player.
///
/// Calls take effect in the next [`SoundSystems`](super::SoundSystems) pass.
/// When several calls land in one frame, the last [`Self::play`] or
/// [`Self::stop`] wins.
#[derive(Resource, Debug, Default)]
pub struct MusicDeck {
    request: Option<MusicRequest>,
    duck: MusicDuck,
    status: MusicDeckStatusType,
    /// Whether the missing output was logged in this run.
    no_output_logged: bool,
}

impl MusicDeck {
    /// Crossfade to `track` over `fade`: the current track fades out while
    /// `track` fades in. A track that is already the current one plays on.
    /// A track still fading out from an earlier crossfade is cut at once, so
    /// at most two voices sound.
    pub fn play(&mut self, track: Handle<AudioSource>, fade: Duration) {
        self.request = Some(MusicRequest::Play { track, fade });
        self.status = MusicDeckStatusType::Starting;
    }

    /// Fade every track out over `fade`.
    pub fn stop(&mut self, fade: Duration) {
        self.request = Some(MusicRequest::Stop { fade });
        self.status = MusicDeckStatusType::Idle;
    }

    /// Ramp the music to `db` below its level over `fade`. `0.0` lifts the
    /// duck; a positive value is read as `0.0`.
    pub fn duck(&mut self, db: f32, fade: Duration) {
        let target = db.min(0.0);
        let distance = (target - self.duck.db).abs();
        self.duck.target = target;
        self.duck.rate = rate_over(distance, fade);
    }

    /// What the current track is doing.
    pub fn status(&self) -> MusicDeckStatusType {
        self.status
    }
}

/// One track the deck is playing. Spawned and retired by the deck; read it,
/// never spawn it.
#[derive(Component, Clone, Debug)]
#[require(VoiceMarker)]
pub struct MusicVoice {
    sound: Handle<AudioSource>,
    /// The fade's progress, `0.0..=1.0`, shaped by [`equal_power`].
    fade: f32,
    /// Where the fade is heading: `1.0` for the current track, `0.0` for an
    /// outgoing one.
    target: f32,
    /// The fade's speed, in progress per second.
    rate: f32,
    /// When it was spawned, in real time: the newest outgoing voice is the
    /// one a crossfade keeps.
    started: Duration,
    /// When its player was built, in real time.
    player_built: Option<Duration>,
}

impl MusicVoice {
    /// The track.
    pub fn sound(&self) -> &Handle<AudioSource> {
        &self.sound
    }

    /// Whether this is the deck's current track and not an outgoing one.
    pub fn is_current(&self) -> bool {
        self.target > 0.0
    }
}

/// `value` moved toward `target` by `rate` per second over `dt`. An infinite
/// rate is a cut.
fn ramp(value: f32, target: f32, rate: f32, dt: f32) -> f32 {
    if rate.is_infinite() {
        return target;
    }
    let step = rate * dt;
    if value < target {
        (value + step).min(target)
    } else {
        (value - step).max(target)
    }
}

/// The speed that covers `distance` in `fade`. A zero fade is a cut.
fn rate_over(distance: f32, fade: Duration) -> f32 {
    let seconds = fade.as_secs_f32();
    if seconds > 0.0 {
        distance / seconds
    } else {
        f32::INFINITY
    }
}

/// The amplitude of a fade at `progress`. A quarter sine, so the outgoing and
/// the incoming track of a crossfade sum to constant power: two unrelated
/// tracks on a linear ramp dip 3 dB in the middle.
fn equal_power(progress: f32) -> f32 {
    (progress.clamp(0.0, 1.0) * FRAC_PI_2).sin()
}

/// A music voice's pre-master gain.
fn music_gain(voice: &MusicVoice, clip_gain: f32, duck: f32, bus_gain: f32) -> f32 {
    clip_gain * SoundCategoryType::Music.spec().level() * equal_power(voice.fade) * duck * bus_gain
}

/// Apply the deck's pending [`MusicDeck::play`] or [`MusicDeck::stop`].
pub(crate) fn apply_music_requests(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut deck: ResMut<MusicDeck>,
    mut q_voices: Query<(Entity, &mut MusicVoice)>,
) {
    let Some(request) = deck.request.take() else {
        return;
    };
    match request {
        MusicRequest::Play { track, fade } => {
            if q_voices
                .iter()
                .any(|(_, voice)| voice.is_current() && voice.sound == track)
            {
                return;
            }
            let rate = rate_over(1.0, fade);
            let mut outgoing: Vec<(Duration, Entity)> = Vec::new();
            for (entity, mut voice) in &mut q_voices {
                voice.target = 0.0;
                voice.rate = rate;
                outgoing.push((voice.started, entity));
            }
            outgoing.sort_unstable();
            if let Some((_, older)) = outgoing.split_last() {
                for (_, entity) in older {
                    commands.entity(*entity).try_despawn();
                }
            }
            commands.spawn((
                Name::new("Music"),
                MusicVoice {
                    sound: track,
                    fade: 0.0,
                    target: 1.0,
                    rate,
                    started: time.elapsed(),
                    player_built: None,
                },
            ));
        }
        MusicRequest::Stop { fade } => {
            let rate = rate_over(1.0, fade);
            for (_, mut voice) in &mut q_voices {
                voice.target = 0.0;
                voice.rate = rate;
            }
        }
    }
}

/// Build the player of every music voice whose clip is measured. A voice
/// whose clip is not measured yet waits: playback applies the loudness gain.
pub(crate) fn start_music_voices(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mixer: Mixer,
    loudness: Option<Res<SoundLoudness>>,
    deck: Res<MusicDeck>,
    mut q_new: Query<(Entity, &mut MusicVoice), Without<AudioPlayer>>,
) {
    let bus_gain = mixer.bus_gain(SoundBus::Music);
    for (entity, mut voice) in &mut q_new {
        let Some(clip) = loudness
            .as_deref()
            .and_then(|table| table.get(&voice.sound))
        else {
            continue;
        };
        let gain = music_gain(&voice, clip.gain(), deck.duck.gain(), bus_gain);
        commands.entity(entity).try_insert(voice_player(
            &voice.sound,
            PlaybackMode::Once,
            1.0,
            false,
            gain,
        ));
        voice.player_built = Some(time.elapsed());
    }
}

/// Run every music voice's fade and the duck, write the sink gains, retire
/// what has faded out or played to its end, and set the deck's status.
pub(crate) fn drive_music_voices(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mixer: Mixer,
    loudness: Option<Res<SoundLoudness>>,
    mut deck: ResMut<MusicDeck>,
    mut q_voices: Query<(
        Entity,
        &mut MusicVoice,
        Option<&mut AudioSink>,
        Option<&mut VoiceMix>,
    )>,
) {
    let dt = time.delta_secs();
    deck.duck.step(dt);
    let duck = deck.duck.gain();
    let bus_gain = mixer.bus_gain(SoundBus::Music);
    let master = mixer.master_gain();
    let mut current = None;
    for (entity, mut voice, sink, voice_mix) in &mut q_voices {
        voice.fade = ramp(voice.fade, voice.target, voice.rate, dt);
        let faded_out = !voice.is_current() && voice.fade <= 0.0;
        let ended = sink.as_ref().is_some_and(|sink| sink.empty());
        if faded_out || ended {
            commands.entity(entity).try_despawn();
            if ended && voice.is_current() {
                current = Some(MusicDeckStatusType::Ended);
            }
            continue;
        }
        if voice.is_current() {
            current = Some(if sink.is_some() {
                MusicDeckStatusType::Playing
            } else if voice
                .player_built
                .is_some_and(|built| time.elapsed().saturating_sub(built) >= MUSIC_SINK_GRACE)
            {
                MusicDeckStatusType::NoOutput
            } else {
                MusicDeckStatusType::Starting
            });
        }
        let Some(clip) = loudness
            .as_deref()
            .and_then(|table| table.get(&voice.sound))
        else {
            continue;
        };
        let gain = music_gain(&voice, clip.gain(), duck, bus_gain);
        let resolved = VoiceMix {
            level: gain,
            gain,
            channel_gains: [gain; 2],
            emitter: None,
            spatial: false,
            looping: false,
            speed: 1.0,
            capped: false,
            paused: false,
        };
        if let Some(mut voice_mix) = voice_mix {
            voice_mix.set_if_neq(resolved);
        } else {
            commands.entity(entity).try_insert(resolved);
        }
        if let Some(mut sink) = sink {
            sink.set_volume(Volume::Linear(gain * master));
        }
    }
    let status = match current {
        Some(status) => status,
        None if deck.status == MusicDeckStatusType::Ended => MusicDeckStatusType::Ended,
        None => MusicDeckStatusType::Idle,
    };
    if status == MusicDeckStatusType::NoOutput && !deck.no_output_logged {
        info!("nova audio: music has no output device; the deck holds its track");
        deck.no_output_logged = true;
    }
    deck.status = status;
}

#[cfg(test)]
mod tests {
    use bevy::time::TimeUpdateStrategy;

    use super::*;
    use crate::{ClipLoudness, MusicVolume, LOUDNESS_TARGET_LUFS};

    /// One frame of real time in these rigs.
    const FRAME: Duration = Duration::from_millis(100);

    /// A rig with the deck's systems, no audio device, and `tracks` clips
    /// measured at the loudness target, so a full voice sounds at the Music
    /// row level.
    fn deck_app(tracks: usize) -> (App, Vec<Handle<AudioSource>>) {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.insert_resource(TimeUpdateStrategy::ManualDuration(FRAME));
        app.init_resource::<MusicDeck>();
        app.add_systems(
            Update,
            (apply_music_requests, start_music_voices, drive_music_voices).chain(),
        );
        let mut loudness = SoundLoudness::default();
        let handles: Vec<Handle<AudioSource>> = (0..tracks)
            .map(|_| {
                let handle = app
                    .world_mut()
                    .resource_mut::<Assets<AudioSource>>()
                    .reserve_handle();
                loudness.insert(
                    handle.id(),
                    ClipLoudness {
                        lufs: LOUDNESS_TARGET_LUFS,
                    },
                );
                handle
            })
            .collect();
        app.insert_resource(loudness);
        // The first update of a manual clock advances nothing.
        app.update();
        (app, handles)
    }

    fn run(app: &mut App, frames: usize) {
        for _ in 0..frames {
            app.update();
        }
    }

    fn deck(app: &mut App) -> Mut<'_, MusicDeck> {
        app.world_mut().resource_mut::<MusicDeck>()
    }

    /// Every live music voice's track and pre-master gain.
    fn voices(app: &mut App) -> Vec<(Handle<AudioSource>, f32)> {
        let mut query = app.world_mut().query::<(&MusicVoice, Option<&VoiceMix>)>();
        query
            .iter(app.world())
            .map(|(voice, mix)| (voice.sound.clone(), mix.map_or(f32::NAN, |mix| mix.gain)))
            .collect()
    }

    fn gain_of(voices: &[(Handle<AudioSource>, f32)], track: &Handle<AudioSource>) -> f32 {
        voices
            .iter()
            .find(|(sound, _)| sound == track)
            .map(|(_, gain)| *gain)
            .expect("the track has a voice")
    }

    #[test]
    fn a_music_voice_at_slider_zero_plays_silent_and_stays_alive() {
        let (mut app, tracks) = deck_app(1);
        app.insert_resource(MusicVolume(0.0));
        deck(&mut app).play(tracks[0].clone(), Duration::ZERO);
        run(&mut app, 5);

        let mut query = app
            .world_mut()
            .query_filtered::<&VoiceMix, (With<MusicVoice>, With<AudioPlayer>)>();
        let mix = *query.single(app.world()).expect("the voice has its player");
        assert_eq!(mix.gain, 0.0, "the slider at zero is silent");

        app.insert_resource(MusicVolume(1.0));
        run(&mut app, 1);
        let full = SoundCategoryType::Music.spec().level();
        let gain = gain_of(&voices(&mut app), &tracks[0]);
        assert!(
            (gain - full).abs() < 1e-5,
            "the same voice comes in when the slider rises: {gain}"
        );
    }

    #[test]
    fn a_crossfade_ramps_both_tracks_and_retires_the_old_one() {
        let (mut app, tracks) = deck_app(2);
        let (old, new) = (&tracks[0], &tracks[1]);
        let full = SoundCategoryType::Music.spec().level();
        deck(&mut app).play(old.clone(), Duration::ZERO);
        run(&mut app, 2);
        assert!((gain_of(&voices(&mut app), old) - full).abs() < 1e-5);

        deck(&mut app).play(new.clone(), Duration::from_secs(1));
        run(&mut app, 5);
        let mid = voices(&mut app);
        assert_eq!(mid.len(), 2, "both tracks sound in the crossfade");
        let (out_gain, in_gain) = (gain_of(&mid, old), gain_of(&mid, new));
        assert!(
            out_gain > 0.0 && out_gain < full && in_gain > 0.0 && in_gain < full,
            "half way, both are between silence and full: out {out_gain}, in {in_gain}"
        );
        let power = (out_gain / full).powi(2) + (in_gain / full).powi(2);
        assert!(
            (power - 1.0).abs() < 1e-3,
            "the crossfade holds constant power: {power}"
        );

        run(&mut app, 6);
        let after = voices(&mut app);
        assert_eq!(after.len(), 1, "the old track is retired");
        assert!((gain_of(&after, new) - full).abs() < 1e-5);
    }

    #[test]
    fn a_track_with_no_sink_holds_and_reports_no_output() {
        let (mut app, tracks) = deck_app(1);
        deck(&mut app).play(tracks[0].clone(), Duration::ZERO);
        run(&mut app, 1);
        assert_eq!(deck(&mut app).status(), MusicDeckStatusType::Starting);
        let voice = {
            let mut query = app.world_mut().query_filtered::<Entity, With<MusicVoice>>();
            query.single(app.world()).expect("one music voice")
        };

        let grace_frames = (MUSIC_SINK_GRACE.as_millis() / FRAME.as_millis()) as usize;
        for _ in 0..grace_frames * 10 {
            app.update();
            let status = deck(&mut app).status();
            assert_ne!(
                status,
                MusicDeckStatusType::Ended,
                "a track never ends unheard"
            );
        }
        assert_eq!(deck(&mut app).status(), MusicDeckStatusType::NoOutput);
        assert!(
            app.world().get_entity(voice).is_ok(),
            "the deck holds the same voice"
        );
    }
}
