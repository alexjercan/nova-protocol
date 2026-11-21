//! The engine's one playback path.
//!
//! Every sound in the game is a voice entity, and this file is the ONLY place
//! an `AudioPlayer` is constructed - pinned by
//! `the_engine_is_the_only_place_an_audio_player_is_built`, which reads every
//! Rust source in the workspace. A category sound is a [`SoundVoice`] the
//! engine spawns from a [`PlaySound`](super::PlaySound) or a
//! [`SoundLoop`](super::SoundLoop); a music track is a
//! [`MusicVoice`](super::MusicVoice) the [`MusicDeck`](super::MusicDeck)
//! spawns, and its player is built here too.
//!
//! What the engine owns, so no caller re-implements it: the file's loudness
//! gain, the category level, the bus gain, the distance fade, the stereo
//! placement, a loop's release once its source idles, the freeze behind a
//! paused sim, the silence when a scenario unloads, and the cap on how many
//! positional loops may sound at once.

use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

use bevy::{
    audio::{AudioSinkPlayback, PlaybackMode, SpatialAudioSink, Volume},
    prelude::*,
};
use nova_events::prelude::Meters;

use super::{
    bus::Mixer,
    category::SoundBus,
    listener::{resolve_hearing, Hearing, SoundListenerAnchor},
    loudness::SoundLoudness,
    mixing::{SfxListenerMarker, SFX_AUDIBLE_THRESHOLD},
    play::{SoundLoop, SoundVoice, VoiceGroup, VoicePlace},
    spatial::{emitter_point, pan_compensation, pan_gains},
};

/// How many positional LOOPS may sound at once.
///
/// Loops are per-source (every burning ship has its own), so a busy scene
/// could otherwise open a sink for every thruster in it. The highest-priority,
/// loudest voices keep playing and the rest are held at silence - they stay
/// alive and are re-ranked every frame, so a ship that closes the range fades
/// back in rather than restarting.
pub const MAX_EFFECT_LOOP_VOICES: usize = 8;

/// How much louder a silent loop must be than the one it would displace before
/// the cap trades them.
///
/// The cap is a RANK, so two ships drifting past each other at the boundary
/// swap places on any wobble in the range. A capped voice STOPS its sink, so a
/// swap is a transport change, and the boundary needs a band. A sounding voice
/// is ranked as if it were this much louder, which is what keeps a tie from
/// chattering.
const VOICE_CAP_HYSTERESIS: f32 = 1.15;

/// How long a loop at the top of its category's dB range takes to fall silent
/// once its owner reports zero intensity, in seconds.
///
/// The ramp is one slope for every loop: the whole range over this time. A
/// quieter loop is silent sooner, so a tapped source never rings longer than a
/// held one. The time is short so the sound ends with the key that held it,
/// and not zero because a loop cut from full level on one volume write
/// clicks.
pub const LOOP_RELEASE_SECONDS: f32 = 0.12;

/// A loop voice's release: the ramp to silence after its owner's intensity
/// falls to zero.
///
/// The gains here are the loop's intensity gain
/// ([`SoundCategory::intensity_gain`](super::SoundCategory::intensity_gain)),
/// so the distance fade, the bus and the master still apply live to a
/// releasing voice. The default is no release: the voice sounds at its
/// owner's intensity.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct LoopRelease {
    /// The gain the loop sounded at on the last frame: where a release
    /// starts.
    sounded: f32,
    /// What is left of the ramp. Zero when no release runs.
    tail: f32,
}

impl LoopRelease {
    /// The state one frame of `dt` seconds on, for a loop whose intensity
    /// maps to the gain `target` this frame.
    fn step(self, target: f32, dt: f32) -> Self {
        // An idle source starts the ramp from the gain the loop last sounded
        // at. A working source lets an older ramp run out under it.
        let from = if target > 0.0 {
            self.tail
        } else {
            self.tail.max(self.sounded)
        };
        let tail = (from - dt / LOOP_RELEASE_SECONDS).max(0.0);
        Self {
            sounded: target.max(tail),
            tail,
        }
    }

    /// The gain the loop sounds at this frame: its intensity's `target`, or
    /// the ramp while that is louder. A source that relights inside its own
    /// release therefore never steps down.
    fn gain(self, target: f32) -> f32 {
        target.max(self.tail)
    }
}

/// Marks every engine voice. Required by [`SoundVoice`] and
/// [`MusicVoice`](super::MusicVoice), so the passes can keep their pose reads
/// off the voices they move.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct VoiceMarker;

/// What a voice's place resolves to this frame.
enum VoicePoint {
    /// Not placed in the world.
    Unplaced,
    /// A world point.
    At(Vec3),
    /// The followed entity is gone.
    Lost,
}

/// The engine's resolved mix for one voice in the current frame.
///
/// Capture tooling reads this instead of reproducing the engine's private
/// placement and voice-cap policy. [`gain`](Self::gain) and
/// [`channel_gains`](Self::channel_gains) exclude master volume so a muted
/// capture run still records the authored mix; apply
/// [`capped`](Self::capped) and [`paused`](Self::paused) as silence with the
/// playhead held.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct VoiceMix {
    /// The level before pan compensation: loudness gain, category level, bus
    /// gain and distance fade.
    pub level: f32,
    /// Pre-master sink gain, including pan compensation.
    pub gain: f32,
    /// Pre-master `[left, right]` amplitude that reaches each output channel.
    ///
    /// A flat voice has [`gain`](Self::gain) on both channels, applied to the
    /// sample's own channels. A positional voice has `gain` times the
    /// [`pan_gains`] of its bearing, and its spatial sink applies them to the
    /// sum of the sample's channels.
    pub channel_gains: [f32; 2],
    /// Where the spatial emitter is parked: the bearing at
    /// [`SPATIAL_EMITTER_RADIUS`](super::SPATIAL_EMITTER_RADIUS) around the
    /// engine's fixed ears at the origin. `None` for a flat voice.
    pub emitter: Option<Vec3>,
    /// Whether the voice plays through a spatial sink, which sums the
    /// sample's channels before it pans.
    pub spatial: bool,
    /// Whether the voice loops.
    pub looping: bool,
    /// Positive playback speed after the engine's clamp.
    pub speed: f32,
    /// Whether the loop cap is holding this voice silent.
    pub capped: bool,
    /// Whether a frozen sim holds this world voice's sink paused.
    pub paused: bool,
}

/// The mix decision for one voice this frame.
struct VoicePlacement {
    /// The intended amplitude BEFORE the pan compensation. This is what "how
    /// loud is this" means - the audible-threshold gate and the voice-cap
    /// ranking both read it.
    level: f32,
    /// The sink gain: [`Self::level`] with the pan compensation folded in.
    gain: f32,
    /// [`Self::gain`] times the spatial sink's per-ear factors.
    channel_gains: [f32; 2],
    /// Where to park the spatial emitter, or `None` for a non-positional voice.
    emitter: Option<Vec3>,
}

impl VoicePlacement {
    /// A non-positional placement at `level`.
    fn flat(level: f32) -> Self {
        Self {
            level,
            gain: level,
            channel_gains: [level; 2],
            emitter: None,
        }
    }

    /// A positional placement at `level` from `source`.
    fn placed(level: f32, hearing: &Hearing, source: Vec3) -> Self {
        let bearing = hearing.bearing(source);
        let gain = level * pan_compensation(bearing);
        let (left, right) = pan_gains(bearing);
        Self {
            level,
            gain,
            channel_gains: [gain * left, gain * right],
            emitter: Some(emitter_point(bearing)),
        }
    }
}

/// Mix one category voice: the clip's loudness gain, the category level, the
/// loop intensity and its release, the bus gain, the distance fade and the
/// pan.
///
/// An [`SoundBus::Effect`] voice with no listener or no place is silent: there
/// is no flat fallback for a sound that happens in the world.
fn place_sound(
    voice: &SoundVoice,
    clip_gain: f32,
    bus_gain: f32,
    hearing: Option<&Hearing>,
    point: &VoicePoint,
) -> VoicePlacement {
    let spec = voice.category.spec();
    let intensity = if voice.looping {
        voice.release.gain(spec.intensity_gain(voice.intensity))
    } else {
        1.0
    };
    let base = clip_gain * spec.level() * intensity * bus_gain;
    if spec.bus == SoundBus::Ui {
        return VoicePlacement::flat(base);
    }
    let (VoicePoint::At(source), Some(hearing)) = (point, hearing) else {
        return VoicePlacement::flat(0.0);
    };
    let level = base * spec.distance_fade(Meters::from_engine(hearing.distance(*source)));
    VoicePlacement::placed(level, hearing, *source)
}

/// The `AudioPlayer` bundle for one voice. THE constructor - see the module
/// doc. A category loop plays in `Loop` and a one-shot in `Despawn`; a music
/// track plays in `Once`, so its sink outlives the clip and tells the deck
/// the track ended.
pub(crate) fn voice_player(
    handle: &Handle<AudioSource>,
    mode: PlaybackMode,
    speed: f32,
    spatial: bool,
    gain: f32,
) -> impl Bundle {
    (
        AudioPlayer(handle.clone()),
        PlaybackSettings {
            mode,
            // WITHOUT the master: bevy's `audio_output` multiplies
            // `GlobalVolume` in once at sink creation. Every later write goes
            // through `Mixer::master_gain`, which folds it back in by hand.
            volume: Volume::Linear(gain.max(0.0)),
            // Rodio does not accept a non-positive playback rate.
            speed: speed.max(f32::MIN_POSITIVE),
            spatial,
            ..default()
        },
    )
}

/// One voice as the passes see it.
struct VoiceView<'a> {
    handle: &'a Handle<AudioSource>,
    looping: bool,
    speed: f32,
    spatial: bool,
    /// On the world track: frozen with the sim, retired when play ends.
    world: bool,
}

impl<'a> VoiceView<'a> {
    fn of(voice: &'a SoundVoice) -> Self {
        let effect = voice.category.bus() == SoundBus::Effect;
        Self {
            handle: &voice.sound,
            looping: voice.looping,
            speed: voice.rate,
            spatial: effect,
            world: effect,
        }
    }
}

/// The pose reads every pass shares: the listener and the voices' sources.
type PoseQuery<'w, 's> = Query<'w, 's, &'static GlobalTransform, Without<VoiceMarker>>;
/// The marked camera.
type CameraQuery<'w, 's> =
    Query<'w, 's, &'static GlobalTransform, (With<SfxListenerMarker>, Without<VoiceMarker>)>;
/// The player's ship.
type AnchorQuery<'w, 's> =
    Query<'w, 's, &'static GlobalTransform, (With<SoundListenerAnchor>, Without<VoiceMarker>)>;

fn hearing(q_camera: &CameraQuery, q_anchor: &AnchorQuery) -> Option<Hearing> {
    resolve_hearing(q_camera.iter().next(), q_anchor.iter().next())
}

/// Mix one voice. `None` while its clip is not measured yet: it waits for
/// its loudness gain.
fn place(
    voice: &SoundVoice,
    mixer: &Mixer,
    loudness: Option<&SoundLoudness>,
    hearing: Option<&Hearing>,
    q_pose: &PoseQuery,
) -> Option<VoicePlacement> {
    let clip = loudness?.get(&voice.sound)?;
    let point = match voice.place {
        VoicePlace::Ui => VoicePoint::Unplaced,
        VoicePlace::At(point) => VoicePoint::At(point),
        VoicePlace::Follow(entity) => follow(entity, q_pose),
    };
    let bus_gain = mixer.bus_gain(voice.category.bus());
    Some(place_sound(voice, clip.gain(), bus_gain, hearing, &point))
}

fn follow(entity: Entity, q_pose: &PoseQuery) -> VoicePoint {
    match q_pose.get(entity) {
        Ok(pose) => VoicePoint::At(pose.translation()),
        Err(_) => VoicePoint::Lost,
    }
}

/// How long a one-shot may wait for a sink before the engine gives up on it.
///
/// Bevy only opens a sink when there IS an audio device and the clip has
/// loaded; with neither - a headless probe run, a CI box, a clip that failed to
/// load - it leaves the entity queued and `PlaybackMode::Despawn` never fires.
/// A category one-shot also waits for its clip's loudness measure. Nothing else
/// would ever retire that voice. Loops are exempt: their owner stops them.
const ONE_SHOT_SINK_GRACE: Duration = Duration::from_secs(2);

/// When a one-shot began waiting for its sink. See [`ONE_SHOT_SINK_GRACE`].
///
/// Real time, not virtual: a cue fired into a paused world is still waiting.
#[derive(Component)]
pub(super) struct AwaitingSink(pub(super) Duration);

/// Run every loop's release ramp one frame. See [`LoopRelease`].
///
/// On the real clock: a sink plays on it, and an interface loop plays through
/// a frozen sim, where the virtual clock stands still.
pub(super) fn ramp_loop_releases(time: Res<Time<Real>>, mut q_voices: Query<&mut SoundVoice>) {
    let dt = time.delta_secs();
    for mut voice in &mut q_voices {
        if !voice.looping {
            continue;
        }
        let target = voice.category.spec().intensity_gain(voice.intensity);
        voice.release = voice.release.step(target, dt);
    }
}

/// Start every voice that has no player yet.
///
/// A ONE-SHOT that is already inaudible is despawned instead: opening a sink
/// for a cue nobody can hear is the churn the audible threshold exists to
/// avoid. So is a world one-shot with no listener to hear it. A silent LOOP is
/// kept - it is waiting to be raised. A category voice whose clip is not
/// measured yet waits.
///
/// The writes are the try_ variants because the engine is not the only owner
/// of a voice: a one-shot is scenario-scoped, so the teardown behind a Retry
/// can despawn it in the same command flush this pass is opening its player
/// in. A voice that died before its sink opened wanted no sink.
pub(super) fn start_sfx_voices(
    mut commands: Commands,
    mixer: Mixer,
    loudness: Option<Res<SoundLoudness>>,
    q_new: Query<(Entity, &SoundVoice), Without<AudioPlayer>>,
    q_camera: CameraQuery,
    q_anchor: AnchorQuery,
    q_pose: PoseQuery,
) {
    let hearing = hearing(&q_camera, &q_anchor);
    for (entity, voice) in &q_new {
        let view = VoiceView::of(voice);
        if view.world && !view.looping && hearing.is_none() {
            debug!("nova audio: dropped a world one-shot: no listener");
            commands.entity(entity).try_despawn();
            continue;
        }
        let Some(placement) = place(
            voice,
            &mixer,
            loudness.as_deref(),
            hearing.as_ref(),
            &q_pose,
        ) else {
            continue;
        };
        if !view.looping && placement.level < SFX_AUDIBLE_THRESHOLD {
            commands.entity(entity).try_despawn();
            continue;
        }
        commands.entity(entity).try_insert((
            voice_player(
                view.handle,
                if view.looping {
                    PlaybackMode::Loop
                } else {
                    PlaybackMode::Despawn
                },
                view.speed,
                view.spatial,
                placement.gain,
            ),
            GlobalTransform::from_translation(placement.emitter.unwrap_or_default()),
        ));
    }
}

/// Retire a one-shot that never got a sink.
///
/// The counterpart to `PlaybackMode::Despawn`, which only fires for a voice
/// that actually played. See [`ONE_SHOT_SINK_GRACE`] for what leaves one
/// stranded. A voice that DID get its sink leaves this query the moment bevy
/// inserts it, so the grace period never truncates a playing cue.
pub(super) fn retire_unplayable_one_shots(
    mut commands: Commands,
    time: Res<Time<Real>>,
    q_waiting: Query<(Entity, &AwaitingSink), (Without<AudioSink>, Without<SpatialAudioSink>)>,
) {
    for (entity, waiting) in &q_waiting {
        if time.elapsed().saturating_sub(waiting.0) >= ONE_SHOT_SINK_GRACE {
            // try_despawn, like every other write to a voice here: the entity
            // is scenario-scoped and the teardown behind a Retry can despawn it
            // in the same flush this sweep retires it in.
            commands.entity(entity).try_despawn();
        }
    }
}

/// One positional loop, as the cap ranks it.
#[derive(Clone, Copy, Debug)]
struct LoopRank {
    priority: u8,
    level: f32,
    entity: Entity,
}

/// Re-mix every playing voice: its gains, its distance, its bearing, its
/// playback rate, and whether its sink runs at all.
///
/// Runs every frame rather than at spawn because a voice's mix is a function of
/// two things that both move - the source and the listener - so a cue that only
/// panned where it started would swing wrong the moment the camera turned.
///
/// A voice the cap silences is PAUSED, not merely turned down: an open sink
/// mixes its silence into every audio callback, and ~200 of those is the whole
/// of the underrun noise a crowded scene generates. It resumes where it
/// stopped rather than where it would have got to, which is why the loops this
/// applies to are hums and hisses and not tonal material.
pub(super) fn drive_sfx_voices(
    mut commands: Commands,
    mixer: Mixer,
    loudness: Option<Res<SoundLoudness>>,
    freeze: Option<Res<SoundFreeze>>,
    mut sounding: Local<HashSet<Entity>>,
    q_camera: CameraQuery,
    q_anchor: AnchorQuery,
    q_pose: PoseQuery,
    mut q_voices: Query<
        (
            Entity,
            &SoundVoice,
            &mut GlobalTransform,
            Option<&mut AudioSink>,
            Option<&mut SpatialAudioSink>,
            Option<&mut VoiceMix>,
        ),
        With<VoiceMarker>,
    >,
) {
    let hearing = hearing(&q_camera, &q_anchor);
    let mut placements: HashMap<Entity, VoicePlacement> = HashMap::new();
    let mut grouped: HashMap<(AssetId<AudioSource>, VoiceGroup), (u8, Vec<LoopRank>)> =
        HashMap::new();
    let mut loose: Vec<LoopRank> = Vec::new();
    // A loop its owner has turned down to nothing, once its release has run
    // out, holds its sink instead of mixing silence, and takes no slot under
    // the cap.
    let mut silenced = HashSet::new();
    for (entity, voice, ..) in &q_voices {
        let Some(placement) = place(
            voice,
            &mixer,
            loudness.as_deref(),
            hearing.as_ref(),
            &q_pose,
        ) else {
            continue;
        };
        let view = VoiceView::of(voice);
        if view.looping && placement.level < SFX_AUDIBLE_THRESHOLD {
            silenced.insert(entity);
        } else if view.looping && view.spatial {
            let spec = voice.category.spec();
            grouped
                .entry((voice.sound.id(), voice.group))
                .or_insert_with(|| (spec.max_voices, Vec::new()))
                .1
                .push(LoopRank {
                    priority: spec.priority,
                    level: placement.level,
                    entity,
                });
        }
        placements.insert(entity, placement);
    }
    for (max_voices, mut ranks) in grouped.into_values() {
        let over = loops_over_the_cap(&mut ranks, &sounding, usize::from(max_voices));
        loose.extend(
            ranks
                .into_iter()
                .filter(|rank| !over.contains(&rank.entity)),
        );
        silenced.extend(over);
    }
    silenced.extend(loops_over_the_cap(
        &mut loose,
        &sounding,
        MAX_EFFECT_LOOP_VOICES,
    ));
    *sounding = loose
        .iter()
        .map(|rank| rank.entity)
        .filter(|entity| !silenced.contains(entity))
        .collect();
    // While the sim is frozen `pause_world_voices` owns every world sink, and
    // the cap must not argue with it: a capped voice is already stopped, and
    // resuming one behind a pause overlay would be a loop playing through a
    // stopped world. The RANKING still runs, so the frame the overlay closes
    // resumes exactly the voices that should sound.
    let frozen = freeze.is_some_and(|freeze| freeze.0);
    let master = mixer.master_gain();

    for (entity, voice, mut pose, sink, spatial_sink, voice_mix) in &mut q_voices {
        let Some(placement) = placements.get(&entity) else {
            continue;
        };
        let view = VoiceView::of(voice);
        let capped = silenced.contains(&entity);
        // Rodio does not accept a non-positive playback rate, and the owner of
        // a loop writes this field every frame.
        let speed = view.speed.max(f32::MIN_POSITIVE);
        let resolved = VoiceMix {
            level: placement.level,
            gain: placement.gain,
            channel_gains: placement.channel_gains,
            emitter: placement.emitter,
            spatial: view.spatial,
            looping: view.looping,
            speed,
            capped,
            paused: frozen && view.world,
        };
        if let Some(mut voice_mix) = voice_mix {
            voice_mix.set_if_neq(resolved);
        } else {
            commands.entity(entity).try_insert(resolved);
        }
        let gain = if capped { 0.0 } else { placement.gain * master };
        if let Some(emitter) = placement.emitter {
            *pose = GlobalTransform::from_translation(emitter);
        }
        if let Some(mut sink) = sink {
            sink.set_volume(Volume::Linear(gain));
            sink.set_speed(speed);
            if !frozen {
                hold_sink(&*sink, capped);
            }
        }
        if let Some(mut sink) = spatial_sink {
            sink.set_volume(Volume::Linear(gain));
            sink.set_speed(speed);
            if !frozen {
                hold_sink(&*sink, capped);
            }
        }
    }
}

/// Stop a capped sink, or let it run again. Guarded both ways: this is read
/// once per voice per frame, and the flag is cheaper than the call.
fn hold_sink(sink: &impl AudioSinkPlayback, held: bool) {
    if held == sink.is_paused() {
        return;
    }
    if held {
        sink.pause();
    } else {
        sink.play();
    }
}

/// Which loops must hold silence this frame, given a cap of `cap` and which of
/// them are `sounding` already.
///
/// A RANK, not a level threshold. A formation of identical ships at identical
/// range ties on level, and a threshold lets the whole tie through - losing the
/// cap in precisely the crowded scene it exists for. Priority ranks first,
/// then level; the entity breaks the tie, so the same voices keep sounding
/// frame to frame instead of trading places.
///
/// A sounding voice ranks with [`VOICE_CAP_HYSTERESIS`] applied, so the
/// boundary pair has to actually separate before their sinks trade transport
/// state. The cap itself is exact either way: the count that sounds is the
/// same, only which voices they are is sticky.
fn loops_over_the_cap(
    loops: &mut [LoopRank],
    sounding: &HashSet<Entity>,
    cap: usize,
) -> HashSet<Entity> {
    if loops.len() <= cap {
        return HashSet::new();
    }
    let level = |rank: &LoopRank| {
        if sounding.contains(&rank.entity) {
            rank.level * VOICE_CAP_HYSTERESIS
        } else {
            rank.level
        }
    };
    loops.sort_by(|a, b| {
        b.priority
            .cmp(&a.priority)
            .then(level(b).total_cmp(&level(a)))
            .then(a.entity.cmp(&b.entity))
    });
    loops[cap..].iter().map(|rank| rank.entity).collect()
}

/// Whether the sim is frozen, so every WORLD voice is held paused.
///
/// The engine's one hook into game state. The engine does not know the game's
/// states: whoever freezes the world runs [`pause_world_voices`] on the way in
/// and [`resume_world_voices`] on the way out (`nova_gameplay` does, on every
/// frozen `PauseStates` variant), and those two set this. The per-frame pass
/// reads it so the cap never resumes a voice behind the freeze and the capture
/// hears world voices as [`VoiceMix::paused`].
#[derive(Resource, Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct SoundFreeze(pub bool);

/// The world-voice sinks, for the freeze and the stop.
type WorldSinkQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static SoundVoice,
        Option<&'static AudioSink>,
        Option<&'static SpatialAudioSink>,
    ),
>;

/// Freeze the WORLD voices while the sim is frozen (the pause overlay or the
/// Tab ship computer). Audio sinks do not follow `Time<Virtual>`, so without
/// this a loop keeps roaring at its last volume behind a stopped world.
///
/// Interface voices play THROUGH the freeze, by bus rather than by a guard:
/// the interface is what the player is looking at while the world is stopped.
pub fn pause_world_voices(mut freeze: ResMut<SoundFreeze>, q_voices: WorldSinkQuery) {
    freeze.set_if_neq(SoundFreeze(true));
    for (voice, sink, spatial_sink) in &q_voices {
        if !VoiceView::of(voice).world {
            continue;
        }
        if let Some(sink) = sink {
            sink.pause();
        }
        if let Some(sink) = spatial_sink {
            sink.pause();
        }
    }
}

/// Unfreeze what [`pause_world_voices`] stopped.
pub fn resume_world_voices(mut freeze: ResMut<SoundFreeze>, q_voices: WorldSinkQuery) {
    freeze.set_if_neq(SoundFreeze(false));
    for (voice, sink, spatial_sink) in &q_voices {
        if !VoiceView::of(voice).world {
            continue;
        }
        if let Some(sink) = sink {
            sink.play();
        }
        if let Some(sink) = spatial_sink {
            sink.play();
        }
    }
}

/// Retire every world voice when gameplay ends, and silence every world
/// [`SoundLoop`].
///
/// A loop's level is only driven inside a scenario-gated set, so between one
/// scenario ending and the next becoming live the last engine hum would keep
/// roaring at its final level through the whole load. Its voice comes back
/// silent and waits for its owner to raise it again. Interface voices and
/// loops are left alone: they belong to the UI that spawned them and are torn
/// down with it.
pub fn stop_world_voices(
    mut commands: Commands,
    q_voices: Query<(Entity, &SoundVoice)>,
    mut q_loops: Query<&mut SoundLoop>,
) {
    for (entity, voice) in &q_voices {
        if VoiceView::of(voice).world {
            // try_despawn: this runs on the way out of a scenario, which is
            // exactly when the scoped sweep is despawning the same voices.
            commands.entity(entity).try_despawn();
        }
    }
    for mut sound_loop in &mut q_loops {
        if sound_loop.category.bus() == SoundBus::Effect {
            sound_loop.intensity = 0.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::{ClipLoudness, SoundCategoryType, LOUDNESS_TARGET_LUFS};

    fn listener_at(position: Vec3) -> Hearing {
        Hearing::new(
            GlobalTransform::from(Transform::from_translation(position)),
            None,
        )
    }

    #[test]
    fn a_voice_following_a_despawned_entity_holds_silence() {
        // The alternative - falling back to non-positional - would make a dead
        // ship's engine jump to full volume in the player's ear.
        let listener = listener_at(Vec3::ZERO);
        let voice = SoundVoice {
            looping: true,
            ..sound_voice(
                SoundCategoryType::Drive,
                VoicePlace::Follow(Entity::PLACEHOLDER),
            )
        };
        let placement = place_sound(&voice, 1.0, 1.0, Some(&listener), &VoicePoint::Lost);
        assert_eq!(placement.level, 0.0);
        assert_eq!(placement.gain, 0.0);
    }

    fn loops(levels: impl IntoIterator<Item = f32>) -> Vec<LoopRank> {
        levels
            .into_iter()
            .enumerate()
            .map(|(index, level)| LoopRank {
                priority: 0,
                level,
                entity: Entity::from_raw_u32(index as u32 + 1).expect("a valid test entity"),
            })
            .collect()
    }

    /// The cold start every ranking test but the hysteresis one wants: no
    /// voice has a slot yet, so the rank is the level.
    fn nothing_sounding() -> HashSet<Entity> {
        HashSet::new()
    }

    #[test]
    fn the_loop_cap_keeps_the_loudest_voices() {
        // Under the cap nothing is cut.
        let mut few = loops((0..MAX_EFFECT_LOOP_VOICES).map(|i| i as f32));
        assert!(
            loops_over_the_cap(&mut few, &nothing_sounding(), MAX_EFFECT_LOOP_VOICES).is_empty()
        );

        // Over it, exactly the overflow is silenced, and it is the quiet end.
        let mut many = loops((0..MAX_EFFECT_LOOP_VOICES + 5).map(|i| 0.1 * i as f32));
        let silenced = loops_over_the_cap(&mut many, &nothing_sounding(), MAX_EFFECT_LOOP_VOICES);
        assert_eq!(silenced.len(), 5);
        let loudest_silenced = many
            .iter()
            .filter(|rank| silenced.contains(&rank.entity))
            .map(|rank| rank.level)
            .fold(f32::MIN, f32::max);
        let quietest_kept = many
            .iter()
            .filter(|rank| !silenced.contains(&rank.entity))
            .map(|rank| rank.level)
            .fold(f32::MAX, f32::min);
        assert!(
            loudest_silenced < quietest_kept,
            "the cut is at the quiet end"
        );
    }

    #[test]
    fn a_formation_at_one_range_is_still_capped() {
        // The reason the cap ranks instead of thresholding: identical ships at
        // identical range tie on level, and a level floor would pass the whole
        // tie through and open a sink for every one of them.
        let mut tied = loops(std::iter::repeat_n(0.5, MAX_EFFECT_LOOP_VOICES + 6));
        let silenced = loops_over_the_cap(&mut tied, &nothing_sounding(), MAX_EFFECT_LOOP_VOICES);
        assert_eq!(
            tied.len() - silenced.len(),
            MAX_EFFECT_LOOP_VOICES,
            "a tie must not defeat the cap"
        );
    }

    #[test]
    fn the_cap_keeps_the_same_voices_across_frames() {
        // Held at silence, not despawned - so a voice that flips in and out on
        // an arbitrary tie-break would chatter. The entity fixes the order.
        let mut frame = loops(std::iter::repeat_n(0.5, MAX_EFFECT_LOOP_VOICES + 3));
        let first = loops_over_the_cap(
            &mut frame.clone(),
            &nothing_sounding(),
            MAX_EFFECT_LOOP_VOICES,
        );
        let mut shuffled: Vec<_> = frame.drain(..).rev().collect();
        let second = loops_over_the_cap(&mut shuffled, &nothing_sounding(), MAX_EFFECT_LOOP_VOICES);
        assert_eq!(first, second, "iteration order must not move the cut");
    }

    #[test]
    fn a_sounding_voice_keeps_its_slot_until_a_rival_clearly_beats_it() {
        // The boundary pair, a hair apart. A capped voice now STOPS its sink,
        // so trading the slot on a wobble in the range is a restart every
        // frame - the hysteresis band is what makes the swap mean something.
        // One slot short: the loudest seven are never in doubt, and the last
        // slot is the one the boundary pair are arguing over.
        const SETTLED: usize = MAX_EFFECT_LOOP_VOICES - 1;
        let mut frame = loops((0..SETTLED).map(|_| 1.0).chain([0.50, 0.49]));
        let incumbent = frame[SETTLED].entity;
        let challenger = frame[SETTLED + 1].entity;
        let sounding: HashSet<Entity> = frame[..=SETTLED].iter().map(|rank| rank.entity).collect();

        // The challenger edges ahead on the raw level and still loses: it has
        // not beaten the band.
        frame[SETTLED + 1].level = 0.52;
        let silenced = loops_over_the_cap(&mut frame.clone(), &sounding, MAX_EFFECT_LOOP_VOICES);
        assert!(
            silenced.contains(&challenger) && !silenced.contains(&incumbent),
            "a hair of extra level must not trade a sounding voice's slot"
        );

        // Well past the band, it takes the slot - the cap is still a rank.
        frame[SETTLED + 1].level = 0.80;
        let silenced = loops_over_the_cap(&mut frame, &sounding, MAX_EFFECT_LOOP_VOICES);
        assert!(
            silenced.contains(&incumbent) && !silenced.contains(&challenger),
            "a voice that is genuinely louder must still win the slot"
        );
    }

    #[test]
    fn a_voice_entity_carries_a_world_pose_and_no_local_one() {
        // `drive_sfx_voices` writes the spatial emitter STRAIGHT into the
        // voice's `GlobalTransform`, and that is only sound because nothing
        // propagates a voice: it has a world pose and no `Transform`. Give it
        // one - directly, or through a bevy `#[require]` on some component
        // added here later - and `TransformSystems::Propagate` silently
        // overwrites the placement every frame, collapsing every positional
        // cue onto the origin.
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.add_systems(Update, start_sfx_voices);
        app.insert_resource(measured());
        app.world_mut()
            .spawn((SfxListenerMarker, GlobalTransform::IDENTITY));
        let voice = app
            .world_mut()
            .spawn(drive_loop(Vec3::X * 60.0, IVec3::ZERO))
            .id();

        app.update();

        let voice = app.world().entity(voice);
        assert!(
            voice.contains::<GlobalTransform>(),
            "the engine parks the spatial emitter on the voice's world pose"
        );
        assert!(
            !voice.contains::<Transform>(),
            "a voice with a local transform is a voice Propagate owns, and the \
             engine would be writing a pose something else overwrites"
        );
    }

    /// Capture reads the mix the engine resolved, not a copy of its maths: a
    /// nearer voice publishes more gain than a farther one, a positional voice
    /// publishes its per-ear pan, and a voice whose source moves away
    /// publishes a falling gain.
    #[test]
    fn every_voice_publishes_the_mix_the_engine_resolved_this_frame() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.add_systems(Update, (start_sfx_voices, drive_sfx_voices).chain());
        app.insert_resource(measured());
        app.world_mut()
            .spawn((SfxListenerMarker, GlobalTransform::IDENTITY));
        let slow = |point: Vec3, cell: IVec3| SoundVoice {
            rate: 0.5,
            ..drive_loop(point, cell)
        };
        let near = app.world_mut().spawn(slow(Vec3::X * 60.0, IVec3::X)).id();
        let far = app.world_mut().spawn(slow(Vec3::X * 200.0, IVec3::Y)).id();
        let source = app
            .world_mut()
            .spawn(GlobalTransform::from_translation(Vec3::X * 60.0))
            .id();
        let leaving = app
            .world_mut()
            .spawn(SoundVoice {
                place: VoicePlace::Follow(source),
                ..drive_loop(Vec3::ZERO, IVec3::Z)
            })
            .id();

        let mut leaving_gains = Vec::new();
        for distance in [60.0, 120.0, 180.0] {
            *app.world_mut()
                .get_mut::<GlobalTransform>(source)
                .expect("the source has a pose") =
                GlobalTransform::from_translation(Vec3::X * distance);
            app.update();
            let mix = *app
                .world()
                .get::<VoiceMix>(leaving)
                .expect("every voice publishes its mix");
            leaving_gains.push(mix.gain);
        }

        let mix = |entity| *app.world().get::<VoiceMix>(entity).unwrap();
        assert!(mix(near).gain > mix(far).gain, "near is louder than far");
        assert!(
            mix(far).gain > 0.0,
            "far is still inside the category's reach"
        );
        let (left, right) = pan_gains(Vec3::X);
        assert_eq!(
            mix(near).channel_gains,
            [mix(near).gain * left, mix(near).gain * right],
            "a voice to starboard publishes the spatial sink's per-ear factors"
        );
        assert!(left < right, "starboard leans right: L {left} R {right}");
        assert_eq!(mix(near).speed, 0.5, "the owner's speed is published");
        assert!(!mix(near).capped, "two loops are under the cap");
        assert!(!mix(near).paused, "the sim is not frozen");
        assert!(
            leaving_gains.windows(2).all(|pair| pair[1] < pair[0]),
            "a source moving away fades: {leaving_gains:?}"
        );
    }

    /// A frozen sim pauses Effect-bus sinks, so capture must hear them paused
    /// too; interface voices keep playing behind the overlay.
    #[test]
    fn a_frozen_sim_publishes_world_voices_as_paused() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.insert_resource(SoundFreeze(true));
        app.insert_resource(measured());
        app.add_systems(Update, (start_sfx_voices, drive_sfx_voices).chain());
        app.world_mut()
            .spawn((SfxListenerMarker, GlobalTransform::IDENTITY));
        let world = app.world_mut().spawn(drive_loop(Vec3::X, IVec3::ZERO)).id();
        let interface = app
            .world_mut()
            .spawn(SoundVoice {
                looping: true,
                ..sound_voice(SoundCategoryType::TerminalBed, VoicePlace::Ui)
            })
            .id();

        app.update();

        let paused = |entity| app.world().get::<VoiceMix>(entity).unwrap().paused;
        assert!(paused(world), "an Effect-bus voice is held by the freeze");
        assert!(!paused(interface), "an interface voice plays through it");
    }

    /// Capture silences a voice only on the engine's word: past the cap, the
    /// quietest world loop publishes `capped` and the rest do not.
    #[test]
    fn the_quietest_loop_past_the_cap_publishes_as_capped() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.add_systems(Update, (start_sfx_voices, drive_sfx_voices).chain());
        app.insert_resource(measured());
        app.world_mut()
            .spawn((SfxListenerMarker, GlobalTransform::IDENTITY));
        let voices: Vec<Entity> = (0..=MAX_EFFECT_LOOP_VOICES)
            .map(|i| {
                let point = Vec3::X * (40.0 + 10.0 * i as f32);
                app.world_mut()
                    .spawn(drive_loop(point, IVec3::new(i as i32, 0, 0)))
                    .id()
            })
            .collect();

        app.update();

        let mix = |entity| *app.world().get::<VoiceMix>(entity).unwrap();
        let capped: Vec<Entity> = voices
            .iter()
            .copied()
            .filter(|&entity| mix(entity).capped)
            .collect();
        let quietest = *voices.last().unwrap();
        assert_eq!(capped, [quietest], "only the farthest loop is capped");
        assert!(
            voices
                .windows(2)
                .all(|pair| mix(pair[1]).level < mix(pair[0]).level),
            "every loop sits at a distinct level"
        );
    }

    /// One frame of the release tests. The first update of an app runs at
    /// dt 0, so every rig below warms up before it measures.
    const RELEASE_FRAME: Duration = Duration::from_millis(16);

    /// Hold a loop at full intensity for `held` frames, let it go, and return
    /// the level it sounded at on the last held frame and on each frame after.
    fn release_levels(held: usize) -> (f32, Vec<f32>) {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            RELEASE_FRAME,
        ));
        app.init_asset::<AudioSource>();
        app.add_plugins(crate::NovaSoundPlugin);
        app.insert_resource(measured());
        app.world_mut()
            .spawn((SfxListenerMarker, GlobalTransform::IDENTITY));
        app.update();

        let owner = app
            .world_mut()
            .spawn((
                GlobalTransform::IDENTITY,
                SoundLoop {
                    intensity: 1.0,
                    ..SoundLoop::new(Handle::default(), SoundCategoryType::Drive)
                },
            ))
            .id();
        let level = |app: &mut App| {
            let mut voices = app.world_mut().query::<(&SoundVoice, &VoiceMix)>();
            voices
                .iter(app.world())
                .find(|(voice, _)| voice.place == VoicePlace::Follow(owner))
                .map_or(0.0, |(_, mix)| if mix.capped { 0.0 } else { mix.level })
        };
        for _ in 0..held {
            app.update();
        }
        let sounded = level(&mut app);

        app.world_mut()
            .get_mut::<SoundLoop>(owner)
            .expect("the owner keeps its loop")
            .intensity = 0.0;
        let after = (0..32)
            .map(|_| {
                app.update();
                level(&mut app)
            })
            .collect();
        (sounded, after)
    }

    /// The release rule every loop gets: a source that idles is silent within
    /// [`LOOP_RELEASE_SECONDS`], after one frame of sound as after a long
    /// hold, and it gets there by a ramp and not on one volume write.
    #[test]
    fn a_tapped_loop_is_silent_within_the_release_time() {
        let frame = RELEASE_FRAME.as_secs_f32();
        let release_frames = (LOOP_RELEASE_SECONDS / frame).ceil() as usize;
        for held in [1, 96] {
            let (sounded, after) = release_levels(held);
            assert!(sounded > 0.0, "a loop held {held} frames must sound");
            assert!(
                after[0] > 0.0 && after[0] < sounded,
                "held {held}: the release ramps, it does not cut: {sounded} then {after:?}"
            );
            assert!(
                after.windows(2).all(|pair| pair[1] <= pair[0]),
                "held {held}: a release only falls: {after:?}"
            );
            assert!(
                after[release_frames..].iter().all(|&level| level == 0.0),
                "held {held}: silent within {LOOP_RELEASE_SECONDS} s of the source idling: \
                 {after:?}"
            );
        }
    }

    /// A source that relights inside its own release picks up from the ramp:
    /// the relight's low intensity gain must not step the level down first.
    #[test]
    fn a_loop_relit_inside_its_release_does_not_step_down() {
        let dt = RELEASE_FRAME.as_secs_f32();
        let released = LoopRelease::default().step(1.0, dt).step(0.0, dt);
        let ramp = released.gain(0.0);
        assert!(ramp > 0.5, "one frame into a release from full: {ramp}");

        let relit = released.step(0.2, dt);
        let level = relit.gain(0.2);
        assert!(
            level > 0.2 && level < ramp,
            "the ramp carries the relight down to its own gain: {level}"
        );
        let mut state = relit;
        for _ in 0..16 {
            state = state.step(0.2, dt);
        }
        assert_eq!(state.gain(0.2), 0.2, "then the source's gain holds");
    }

    fn sound_voice(category: SoundCategoryType, place: VoicePlace) -> SoundVoice {
        SoundVoice {
            sound: Handle::default(),
            category,
            place,
            group: VoiceGroup::Ui,
            looping: false,
            intensity: 1.0,
            rate: 1.0,
            started: Duration::ZERO,
            release: LoopRelease::default(),
        }
    }

    /// A full drive loop at `point`, alone in the world cell `cell`, so the
    /// per-group voice count never silences it.
    fn drive_loop(point: Vec3, cell: IVec3) -> SoundVoice {
        SoundVoice {
            looping: true,
            group: VoiceGroup::Cell(cell),
            ..sound_voice(SoundCategoryType::Drive, VoicePlace::At(point))
        }
    }

    /// The per-ear gains rodio's spatial source applies to an emitter at
    /// `emitter`, for the ears bevy feeds it: those of the first
    /// `SpatialListener`, read after propagation as bevy's `EarPositions`
    /// reads them. A mirror of `rodio::source::Spatial::set_positions` for
    /// ears anywhere, where [`pan_gains`] assumes the engine's own geometry.
    fn rodio_ear_gains(world: &mut World, emitter: Vec3) -> [f32; 2] {
        let mut q_ears = world.query::<(&GlobalTransform, &SpatialListener)>();
        let (pose, ears) = q_ears.iter(world).next().expect("the app has ears");
        let left = pose.transform_point(ears.left_ear_offset);
        let right = pose.transform_point(ears.right_ear_offset);
        let (left_sq, right_sq) = (
            left.distance_squared(emitter),
            right.distance_squared(emitter),
        );
        let lean = (left_sq.sqrt() - right_sq.sqrt()) / left.distance(right);
        [
            ((lean + 1.0) / 4.0 + 0.5).min(1.0) * (1.0 / left_sq).min(1.0),
            ((-lean + 1.0) / 4.0 + 0.5).min(1.0) * (1.0 / right_sq).min(1.0),
        ]
    }

    /// The chase camera's Normal-stance pose around `ship` at `yaw`: 20.6 u
    /// behind and 5 u over the hull, looking 20 u past it.
    fn chase_pose(ship: Vec3, yaw: f32) -> Transform {
        let turn = Quat::from_rotation_y(yaw);
        Transform::from_translation(ship + turn * Vec3::new(0.0, 5.0, 20.6))
            .looking_at(ship + turn * Vec3::new(0.0, 0.0, -20.0), Vec3::Y)
    }

    /// A fast, uneven mouse swing: degrees of yaw per 16 ms frame.
    const SWING_DEGREES: [f32; 8] = [6.0, 4.0, 8.0, 5.0, 7.0, 3.0, 9.0, 6.0];

    #[derive(Resource, Default)]
    struct Swing {
        yaw: f32,
        frame: usize,
    }

    /// Turn the camera like the chase rig does: its `Transform`, in
    /// `PostUpdate`, before propagation.
    fn swing_camera(
        mut swing: ResMut<Swing>,
        mut q_camera: Query<&mut Transform, With<SfxListenerMarker>>,
    ) {
        swing.yaw += SWING_DEGREES[swing.frame % SWING_DEGREES.len()].to_radians();
        swing.frame += 1;
        for mut camera in &mut q_camera {
            *camera = chase_pose(SWING_SHIP, swing.yaw);
        }
    }

    /// Far from the origin, where a hull flies.
    const SWING_SHIP: Vec3 = Vec3::new(2500.0, 0.0, -800.0);

    /// The crackle in a fast camera swing: bevy places the ears from the
    /// camera's pose AFTER propagation, so ears on the camera stood a frame
    /// away from the sphere the engine parked every emitter on, and each
    /// voice's per-ear gain strayed up to 20 dB from its pan. Whatever the
    /// camera does, rodio must apply the per-ear gains the engine published.
    #[test]
    fn rodio_hears_the_published_pan_however_fast_the_camera_swings() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default(), TransformPlugin));
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            RELEASE_FRAME,
        ));
        app.init_asset::<AudioSource>();
        app.add_plugins(crate::NovaSoundPlugin);
        app.insert_resource(measured());
        app.init_resource::<Swing>();
        app.add_systems(PostUpdate, swing_camera.before(crate::SoundSystems));
        app.world_mut()
            .spawn((SfxListenerMarker, chase_pose(SWING_SHIP, 0.0)));
        let ship = app
            .world_mut()
            .spawn((SoundListenerAnchor, Transform::from_translation(SWING_SHIP)))
            .id();
        let thruster = app
            .world_mut()
            .spawn(Transform::from_translation(SWING_SHIP + Vec3::Z * 3.0))
            .id();
        for (owner, category) in [
            (ship, SoundCategoryType::Rcs),
            (thruster, SoundCategoryType::Drive),
        ] {
            app.world_mut().entity_mut(owner).insert(SoundLoop {
                intensity: 1.0,
                ..SoundLoop::new(Handle::default(), category)
            });
        }

        let db = |gain: f32| 20.0 * gain.max(1e-9).log10();
        let mut worst: f32 = 0.0;
        for _ in 0..(3 * SWING_DEGREES.len()) {
            app.update();
            let mut q_voices = app.world_mut().query::<(&GlobalTransform, &VoiceMix)>();
            let voices: Vec<_> = q_voices
                .iter(app.world())
                .map(|(pose, mix)| (pose.translation(), *mix))
                .collect();
            assert_eq!(voices.len(), 2, "both loops sound");
            for (emitter, mix) in voices {
                let heard = rodio_ear_gains(app.world_mut(), emitter);
                for ear in 0..2 {
                    let error = (db(mix.gain * heard[ear]) - db(mix.channel_gains[ear])).abs();
                    worst = worst.max(error);
                }
            }
        }
        assert!(
            worst < 0.01,
            "rodio's per-ear gains strayed {worst:.2} dB from the published pan"
        );
    }

    /// A table with the default handle's clip measured at the target, so its
    /// loudness gain is unity.
    fn measured() -> SoundLoudness {
        let mut loudness = SoundLoudness::default();
        loudness.insert(
            Handle::<AudioSource>::default().id(),
            ClipLoudness {
                lufs: LOUDNESS_TARGET_LUFS,
            },
        );
        loudness
    }

    fn start_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.add_systems(Update, (start_sfx_voices, drive_sfx_voices).chain());
        app
    }

    /// No flat fallback: a sound that happens in the world needs ears to
    /// place it, and without them it is dropped rather than played in the
    /// player's face.
    #[test]
    fn a_world_one_shot_without_a_listener_is_dropped_not_played_flat() {
        let mut app = start_app();
        app.insert_resource(measured());
        let voice = app
            .world_mut()
            .spawn(sound_voice(
                SoundCategoryType::Explosion,
                VoicePlace::At(Vec3::ZERO),
            ))
            .id();
        app.update();
        assert!(app.world().get_entity(voice).is_err());
    }

    /// Playback applies the clip's loudness gain, so a voice cannot start
    /// before its clip is measured.
    #[test]
    fn a_category_voice_waits_for_its_clip_to_be_measured() {
        let mut app = start_app();
        let voice = app
            .world_mut()
            .spawn(sound_voice(SoundCategoryType::Cockpit, VoicePlace::Ui))
            .id();
        app.update();
        assert!(!app.world().entity(voice).contains::<AudioPlayer>());

        app.insert_resource(measured());
        app.update();
        assert!(app.world().entity(voice).contains::<AudioPlayer>());
        let mix = *app.world().get::<VoiceMix>(voice).unwrap();
        let level = SoundCategoryType::Cockpit.spec().level();
        assert!(
            (mix.level - level).abs() < 1e-5,
            "the category sets the level"
        );
        assert!(!mix.spatial, "a cockpit tone is flat");
    }

    /// The player's own ship plays in the world: the listener stands on it,
    /// so a sound beside it is near and placed however far the camera pulls
    /// back.
    #[test]
    fn the_own_ship_is_heard_from_the_anchor_not_from_the_camera() {
        let mut app = start_app();
        app.insert_resource(measured());
        app.world_mut().spawn((
            SfxListenerMarker,
            GlobalTransform::from_translation(Vec3::Z * 250.0),
        ));
        app.world_mut()
            .spawn((SoundListenerAnchor, GlobalTransform::IDENTITY));
        let voice = app
            .world_mut()
            .spawn(sound_voice(
                SoundCategoryType::Machinery,
                VoicePlace::At(Vec3::X),
            ))
            .id();
        app.update();
        let mix = *app.world().get::<VoiceMix>(voice).unwrap();
        let level = SoundCategoryType::Machinery.spec().level();
        assert!(
            (mix.level - level).abs() < 1e-5,
            "10 m from the ship is inside the reference distance"
        );
        assert!(mix.spatial && mix.emitter.is_some(), "and it is placed");
    }

    #[test]
    fn a_one_shot_that_never_gets_a_sink_is_retired() {
        // The headless case: no audio device, so bevy never opens a sink and
        // `PlaybackMode::Despawn` never fires. Nothing else retires the voice.
        let mut app = App::new();
        app.init_resource::<Time<Real>>();
        app.add_systems(Update, retire_unplayable_one_shots);
        let voice = app
            .world_mut()
            .spawn((
                sound_voice(SoundCategoryType::UiClick, VoicePlace::Ui),
                AwaitingSink(Duration::ZERO),
            ))
            .id();

        app.update();
        assert!(
            app.world().get_entity(voice).is_ok(),
            "inside the grace the clip may still be loading"
        );

        app.world_mut()
            .resource_mut::<Time<Real>>()
            .advance_by(ONE_SHOT_SINK_GRACE);
        app.update();
        assert!(
            app.world().get_entity(voice).is_err(),
            "a cue that can never play must not outlive the session"
        );
    }

    /// The scenario teardown owns the same entities the engine does: an audio
    /// one-shot carries `ScenarioScopedMarker`, so a Retry despawns it wherever
    /// it happens to be in its life. That despawn and the pass that opens the
    /// voice's player land in the SAME command flush, the teardown first, so
    /// the pass writes onto an entity that is already gone.
    ///
    /// Seen live as a crash on Retry in `system_outcomes`: "Entity despawned"
    /// on the `AudioPlayer` insert, then a panic applying the buffers.
    #[test]
    fn a_voice_torn_down_in_the_same_flush_does_not_take_the_pass_with_it() {
        fn tear_down_every_voice(
            mut commands: Commands,
            q_voices: Query<Entity, With<VoiceMarker>>,
        ) {
            for entity in &q_voices {
                commands.entity(entity).despawn();
            }
        }

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        // The race, pinned rather than hoped for: ordering the teardown first
        // with the schedule's automatic sync points OFF leaves its despawn
        // pending while the pass reads the voice, so both buffers apply in one
        // flush, despawn first. Left to the scheduler this is whichever way
        // the sort happens to fall.
        app.edit_schedule(Update, |schedule| {
            schedule.set_build_settings(bevy::ecs::schedule::ScheduleBuildSettings {
                auto_insert_apply_deferred: false,
                ..default()
            });
        });
        app.add_systems(Update, (tear_down_every_voice, start_sfx_voices).chain());
        app.insert_resource(measured());
        let voice = app
            .world_mut()
            .spawn(sound_voice(SoundCategoryType::UiClick, VoicePlace::Ui))
            .id();

        app.update();

        assert!(
            app.world().get_entity(voice).is_err(),
            "the teardown is the owner that wins: the voice is gone"
        );
    }

    /// Every RETIRING write in this module is a `try_` for the same reason the
    /// player-opening write beside it is: the voices are scenario-scoped, and
    /// the teardown behind a Retry despawns them in the same command flush
    /// these passes are retiring them in.
    ///
    /// All three retiring paths at once, because they lose the same race: the
    /// inaudible one-shot the start pass drops, the one-shot that never got a
    /// sink, and the world voice the end of a scenario silences.
    ///
    /// Asserted on the LOG, not on a panic: `EntityCommands::despawn` bakes in
    /// the WARN handler at queue time, so a
    /// stale one is a warn line rather than a crash - and the probe's clean
    /// pass fails a run that logs it, which is the same reason
    /// `teardown_scenario_entities` reaches for `try_despawn`.
    #[test]
    fn a_voice_the_teardown_already_took_is_retired_without_a_stale_command() {
        use std::sync::{Arc, Mutex};

        use bevy::log::tracing_subscriber::{self, util::SubscriberInitExt};

        /// Cloneable in-memory log sink; every clone shares the same buffer.
        #[derive(Clone, Default)]
        struct CapturedLog(Arc<Mutex<Vec<u8>>>);

        impl CapturedLog {
            fn contents(&self) -> String {
                String::from_utf8_lossy(&self.0.lock().unwrap()).into_owned()
            }
        }

        impl std::io::Write for CapturedLog {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(buf);
                Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        fn tear_down_every_voice(
            mut commands: Commands,
            q_voices: Query<Entity, With<VoiceMarker>>,
        ) {
            for entity in &q_voices {
                commands.entity(entity).try_despawn();
            }
        }

        let log = CapturedLog::default();
        let writer = log.clone();
        let _guard = tracing_subscriber::fmt()
            .with_writer(move || writer.clone())
            .set_default();

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        // The same pinning the flush test above uses: the teardown runs first
        // with the automatic sync points off, so its despawn is still pending
        // while each retiring pass reads the voice it is about to drop.
        app.edit_schedule(Update, |schedule| {
            schedule.set_build_settings(bevy::ecs::schedule::ScheduleBuildSettings {
                auto_insert_apply_deferred: false,
                ..default()
            });
        });
        app.add_systems(
            Update,
            (
                tear_down_every_voice,
                start_sfx_voices,
                retire_unplayable_one_shots,
                stop_world_voices,
            )
                .chain(),
        );
        app.world_mut()
            .resource_mut::<Time<Real>>()
            .advance_by(ONE_SHOT_SINK_GRACE);

        // One voice per retiring path. The first is silent under a muted
        // Interface slider, so the start pass drops it instead of opening a
        // player; the second's clip is never measured, so it has been waiting
        // out its grace with no sink; the third is a world loop, which is what
        // the end of a scenario silences.
        app.insert_resource(measured());
        app.insert_resource(crate::InterfaceVolume(0.0));
        let unmeasured = app
            .world_mut()
            .resource_mut::<Assets<AudioSource>>()
            .reserve_handle();
        let inaudible = app
            .world_mut()
            .spawn(sound_voice(SoundCategoryType::UiClick, VoicePlace::Ui))
            .id();
        let stranded = app
            .world_mut()
            .spawn((
                SoundVoice {
                    sound: unmeasured,
                    ..sound_voice(SoundCategoryType::UiClick, VoicePlace::Ui)
                },
                AwaitingSink(Duration::ZERO),
            ))
            .id();
        let world_loop = app
            .world_mut()
            .spawn(drive_loop(Vec3::ZERO, IVec3::ZERO))
            .id();

        app.update();

        for voice in [inaudible, stranded, world_loop] {
            assert!(
                app.world().get_entity(voice).is_err(),
                "the teardown is the owner that wins: {voice} is gone"
            );
        }
        assert!(
            !log.contents().contains("despawn"),
            "no retiring pass may leave a stale command behind; got: {}",
            log.contents()
        );
    }

    /// The convention the whole module exists to enforce: no random sound
    /// spawns in game code. `AudioPlayer` is bevy's playback component, so
    /// building one anywhere else is a voice the engine does not know about -
    /// unrouted, unattenuated, unpannable, and deaf to every setting. A
    /// [`SoundVoice`] built outside this crate skips the repeat interval and
    /// the voice count [`PlaySound`](super::PlaySound) and
    /// [`SoundLoop`](super::SoundLoop) hold.
    ///
    /// A source scan is the honest way to pin this: the rule is about WHERE
    /// code is written, which no type system in this repo can express.
    #[test]
    fn the_engine_is_the_only_place_an_audio_player_is_built() {
        let root = workspace_root();
        let mut offenders = Vec::new();
        for dir in SCANNED_ROOTS {
            visit_rust_sources(&root.join(dir), &mut |path| {
                if path.ends_with(THE_ONE_PLACE) {
                    return;
                }
                let in_engine = path.to_string_lossy().contains(THE_ENGINE);
                let Ok(source) = std::fs::read_to_string(path) else {
                    return;
                };
                for (line_number, line) in source.lines().enumerate() {
                    // Strip line comments: the doc comments in this module name
                    // the constructor on purpose.
                    let code = line.split("//").next().unwrap_or_default();
                    // Both ways bevy lets you build one - the tuple struct and
                    // the convenience constructor - and the engine's own voice.
                    let player = code.contains("AudioPlayer(") || code.contains("AudioPlayer::new");
                    let voice = !in_engine && code.contains("SoundVoice {");
                    if player || voice {
                        offenders.push(format!("{}:{}", path.display(), line_number + 1));
                    }
                }
            });
        }
        assert!(
            offenders.is_empty(),
            "audio must go through the engine, not straight to bevy. \
             Trigger PlaySound or add a SoundLoop instead of building a voice at:\n  {}",
            offenders.join("\n  ")
        );
    }

    /// The one file allowed to name bevy's playback component, as a path
    /// suffix so the check reads the same from any checkout.
    const THE_ONE_PLACE: &str = "nova_sound/src/voice.rs";

    /// The crate that may build a [`SoundVoice`]: the engine's front door.
    const THE_ENGINE: &str = "nova_sound/src/";

    /// Every hand-written Rust root in the workspace. `examples/` matters as
    /// much as `crates/`: a playable example that spawns its own player is the
    /// same unrouted voice, and it is the easier place to write one.
    const SCANNED_ROOTS: [&str; 5] = ["crates", "examples", "src", "tests", "tools"];

    fn workspace_root() -> PathBuf {
        // CARGO_MANIFEST_DIR is `<root>/crates/nova_sound`.
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("the crate sits two levels under the workspace root")
            .to_path_buf()
    }

    fn visit_rust_sources(dir: &Path, visit: &mut impl FnMut(&Path)) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                visit_rust_sources(&path, visit);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                visit(&path);
            }
        }
    }
}
