//! The engine's one playback path.
//!
//! Every sound in the game is a voice entity, and this file is the ONLY place
//! an `AudioPlayer` is constructed - pinned by
//! `the_engine_is_the_only_place_an_audio_player_is_built`, which reads every
//! Rust source in the workspace. A category sound is a [`SoundVoice`] the
//! engine spawns from a [`PlaySound`](super::PlaySound) or a
//! [`SoundLoop`](super::SoundLoop); a legacy caller-levelled sound is an
//! [`SfxVoice`].
//!
//! What the engine owns, so no caller re-implements it: the file's loudness
//! gain, the category level, the bus gain, the distance fade, the stereo
//! placement, the freeze behind a paused sim, the silence when a scenario
//! unloads, and the cap on how many positional loops may sound at once.

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
    bus::{AudioBus, AudioRoute, Mixer},
    category::SoundBus,
    listener::{resolve_hearing, Hearing, SoundListenerAnchor},
    loudness::SoundLoudness,
    mixing::{distance_attenuation, SfxListenerMarker, SFX_AUDIBLE_THRESHOLD},
    play::{SoundLoop, SoundVoice, VoiceGroup, VoicePlace},
    spatial::{listener_ears, pan_compensation, pan_gains},
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

/// Marks every engine voice, of either kind. Required by [`SfxVoice`] and
/// [`SoundVoice`], so the passes can keep their pose reads off the voices
/// they move.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct VoiceMarker;

/// Where a legacy voice is heard from. Read only for [`AudioRoute::Exterior`] -
/// interface and hull voices are non-positional by definition.
#[derive(Clone, Copy, PartialEq, Debug, Default, Reflect)]
pub enum SfxSource {
    /// No position: the voice plays unattenuated and unpanned.
    #[default]
    Unplaced,
    /// A fixed point in the world, for a cue that happened somewhere and is
    /// over.
    At(Vec3),
    /// An entity's world pose, re-read every frame so a moving source keeps its
    /// bearing. A voice whose entity is gone holds silence until its owner
    /// despawns it.
    Follow(Entity),
}

/// One legacy sound the engine is playing: what it is, which track scales it,
/// how loud its owner wants it, and where it is heard from.
///
/// Spawn one to start a sound. For a loop, keep the entity and write
/// [`volume`](Self::volume) each frame; the engine reads it and does the rest.
/// For a one-shot, prefer [`PlaySfx`](super::PlaySfx), which spawns and retires
/// the voice for you.
#[derive(Component, Clone, Debug)]
#[require(VoiceMarker)]
pub struct SfxVoice {
    /// The sound to play.
    pub handle: Handle<AudioSource>,
    /// Which track scales it, and whether it is placed in the world.
    pub route: AudioRoute,
    /// The owner's linear level, before the bus gain, the rolloff and the pan.
    pub volume: f32,
    /// Playback speed, which also shifts pitch (1.0 is normal).
    ///
    /// LIVE, like the volume: `drive_sfx_voices` pushes it to the sink every
    /// frame.
    pub speed: f32,
    /// Where the sound is coming from.
    pub source: SfxSource,
    /// Whether the clip repeats forever. A one-shot retires itself; a loop
    /// plays until its owner despawns the entity.
    pub looping: bool,
}

impl SfxVoice {
    /// A one-shot on `route`, at full level and normal speed.
    pub fn one_shot(handle: Handle<AudioSource>, route: AudioRoute) -> Self {
        Self {
            handle,
            route,
            volume: 1.0,
            speed: 1.0,
            source: SfxSource::Unplaced,
            looping: false,
        }
    }

    /// A loop on `route`, starting silent. Raise [`volume`](Self::volume) to
    /// bring it in.
    pub fn looping(handle: Handle<AudioSource>, route: AudioRoute) -> Self {
        Self {
            handle,
            route,
            volume: 0.0,
            speed: 1.0,
            source: SfxSource::Unplaced,
            looping: true,
        }
    }

    /// Set the owner's linear level.
    pub fn with_volume(mut self, volume: f32) -> Self {
        self.volume = volume;
        self
    }

    /// Set the playback speed (and pitch).
    pub fn with_speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }

    /// Hear the voice from a fixed world point.
    pub fn at(mut self, position: Vec3) -> Self {
        self.source = SfxSource::At(position);
        self
    }

    /// Hear the voice from `entity`, following it as it moves.
    pub fn following(mut self, entity: Entity) -> Self {
        self.source = SfxSource::Follow(entity);
        self
    }
}

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
    /// Resolved spatial emitter, or `None` for a flat voice.
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
            emitter: Some(hearing.emitter(bearing)),
        }
    }
}

/// Mix one legacy voice: bus gain, rolloff, pan.
///
/// A positional legacy voice with no listener (early startup, the editor) or
/// no place to be heard from falls back to non-positional at full range.
fn place_legacy(
    voice: &SfxVoice,
    bus_gain: f32,
    hearing: Option<&Hearing>,
    point: &VoicePoint,
) -> VoicePlacement {
    if matches!(point, VoicePoint::Lost) {
        return VoicePlacement::flat(0.0);
    }
    let base = voice.volume.max(0.0) * bus_gain;
    let (VoicePoint::At(source), true, Some(hearing)) =
        (point, voice.route.is_positional(), hearing)
    else {
        return VoicePlacement::flat(base);
    };
    let level = base * distance_attenuation(hearing.distance(*source));
    VoicePlacement::placed(level, hearing, *source)
}

/// Mix one category voice: the clip's loudness gain, the category level and
/// loop intensity, the bus gain, the distance fade and the pan.
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
        spec.intensity_gain(voice.intensity)
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
/// doc.
fn voice_player(
    handle: &Handle<AudioSource>,
    looping: bool,
    speed: f32,
    spatial: bool,
    gain: f32,
) -> impl Bundle {
    (
        AudioPlayer(handle.clone()),
        PlaybackSettings {
            mode: if looping {
                PlaybackMode::Loop
            } else {
                PlaybackMode::Despawn
            },
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

/// One voice, of either kind, as the passes see it.
struct VoiceView<'a> {
    handle: &'a Handle<AudioSource>,
    looping: bool,
    speed: f32,
    spatial: bool,
    /// On the world track: frozen with the sim, retired when play ends.
    world: bool,
}

impl<'a> VoiceView<'a> {
    fn of(legacy: Option<&'a SfxVoice>, sound: Option<&'a SoundVoice>) -> Option<Self> {
        if let Some(voice) = sound {
            let effect = voice.category.bus() == SoundBus::Effect;
            return Some(Self {
                handle: &voice.sound,
                looping: voice.looping,
                speed: voice.rate,
                spatial: effect,
                world: effect,
            });
        }
        legacy.map(|voice| Self {
            handle: &voice.handle,
            looping: voice.looping,
            speed: voice.speed,
            spatial: voice.route.is_positional(),
            world: voice.route.bus() == AudioBus::World,
        })
    }
}

/// The pose reads every pass shares: the listener and the voices' sources.
type PoseQuery<'w, 's> = Query<'w, 's, &'static GlobalTransform, Without<VoiceMarker>>;
/// The marked camera.
type EarsQuery<'w, 's> =
    Query<'w, 's, &'static GlobalTransform, (With<SfxListenerMarker>, Without<VoiceMarker>)>;
/// The player's ship.
type AnchorQuery<'w, 's> =
    Query<'w, 's, &'static GlobalTransform, (With<SoundListenerAnchor>, Without<VoiceMarker>)>;

fn hearing(q_ears: &EarsQuery, q_anchor: &AnchorQuery) -> Option<Hearing> {
    resolve_hearing(q_ears.iter().next(), q_anchor.iter().next())
}

/// Mix one voice of either kind. `None` for a category voice whose clip is
/// not measured yet: it waits for its loudness gain.
fn place(
    legacy: Option<&SfxVoice>,
    sound: Option<&SoundVoice>,
    mixer: &Mixer,
    loudness: Option<&SoundLoudness>,
    hearing: Option<&Hearing>,
    q_pose: &PoseQuery,
) -> Option<VoicePlacement> {
    if let Some(voice) = sound {
        let clip = loudness?.get(&voice.sound)?;
        let point = match voice.place {
            VoicePlace::Ui => VoicePoint::Unplaced,
            VoicePlace::At(point) => VoicePoint::At(point),
            VoicePlace::Follow(entity) => follow(entity, q_pose),
        };
        let bus_gain = mixer.sound_bus_gain(voice.category.bus());
        return Some(place_sound(voice, clip.gain(), bus_gain, hearing, &point));
    }
    let voice = legacy?;
    let point = match voice.source {
        SfxSource::Unplaced => VoicePoint::Unplaced,
        SfxSource::At(point) => VoicePoint::At(point),
        SfxSource::Follow(entity) => follow(entity, q_pose),
    };
    Some(place_legacy(
        voice,
        mixer.bus_gain(voice.route),
        hearing,
        &point,
    ))
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

/// Give the listener camera its ears the moment it is marked, so nothing has to
/// remember to spawn a [`SpatialListener`] beside the marker.
pub(super) fn on_add_listener(add: On<Add<SfxListenerMarker>>, mut commands: Commands) {
    commands.entity(add.entity).insert(listener_ears());
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
    time: Res<Time<Real>>,
    q_new: Query<
        (Entity, Option<&SfxVoice>, Option<&SoundVoice>),
        (With<VoiceMarker>, Without<AudioPlayer>),
    >,
    q_ears: EarsQuery,
    q_anchor: AnchorQuery,
    q_pose: PoseQuery,
) {
    let hearing = hearing(&q_ears, &q_anchor);
    for (entity, legacy, sound) in &q_new {
        let Some(view) = VoiceView::of(legacy, sound) else {
            continue;
        };
        if sound.is_some() && view.world && !view.looping && hearing.is_none() {
            debug!("nova audio: dropped a world one-shot: no listener");
            commands.entity(entity).try_despawn();
            continue;
        }
        let Some(placement) = place(
            legacy,
            sound,
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
        let mut voice_entity = commands.entity(entity);
        voice_entity.try_insert((
            voice_player(
                view.handle,
                view.looping,
                view.speed,
                view.spatial,
                placement.gain,
            ),
            GlobalTransform::from_translation(placement.emitter.unwrap_or_default()),
        ));
        if !view.looping && legacy.is_some() {
            voice_entity.try_insert(AwaitingSink(time.elapsed()));
        }
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
    q_ears: EarsQuery,
    q_anchor: AnchorQuery,
    q_pose: PoseQuery,
    mut q_voices: Query<
        (
            Entity,
            Option<&SfxVoice>,
            Option<&SoundVoice>,
            &mut GlobalTransform,
            Option<&mut AudioSink>,
            Option<&mut SpatialAudioSink>,
            Option<&mut VoiceMix>,
        ),
        With<VoiceMarker>,
    >,
) {
    let hearing = hearing(&q_ears, &q_anchor);
    let mut placements: HashMap<Entity, VoicePlacement> = HashMap::new();
    let mut grouped: HashMap<(AssetId<AudioSource>, VoiceGroup), (u8, Vec<LoopRank>)> =
        HashMap::new();
    let mut loose: Vec<LoopRank> = Vec::new();
    // A loop its owner has turned down to nothing holds its sink instead of
    // mixing silence, and takes no slot under the cap.
    let mut silenced = HashSet::new();
    for (entity, legacy, sound, ..) in &q_voices {
        let Some(placement) = place(
            legacy,
            sound,
            &mixer,
            loudness.as_deref(),
            hearing.as_ref(),
            &q_pose,
        ) else {
            continue;
        };
        let Some(view) = VoiceView::of(legacy, sound) else {
            continue;
        };
        if view.looping && placement.level < SFX_AUDIBLE_THRESHOLD {
            silenced.insert(entity);
        } else if view.looping && view.spatial {
            let priority = sound.map_or(0, |voice| voice.category.spec().priority);
            let rank = LoopRank {
                priority,
                level: placement.level,
                entity,
            };
            match sound {
                Some(voice) => grouped
                    .entry((voice.sound.id(), voice.group))
                    .or_insert_with(|| (voice.category.spec().max_voices, Vec::new()))
                    .1
                    .push(rank),
                None => loose.push(rank),
            }
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

    for (entity, legacy, sound, mut pose, sink, spatial_sink, voice_mix) in &mut q_voices {
        let Some(placement) = placements.get(&entity) else {
            continue;
        };
        let Some(view) = VoiceView::of(legacy, sound) else {
            continue;
        };
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
        Option<&'static SfxVoice>,
        Option<&'static SoundVoice>,
        Option<&'static AudioSink>,
        Option<&'static SpatialAudioSink>,
    ),
    With<VoiceMarker>,
>;

/// Freeze the WORLD voices while the sim is frozen (the pause overlay or the
/// Tab ship computer). Audio sinks do not follow `Time<Virtual>`, so without
/// this a loop keeps roaring at its last volume behind a stopped world.
///
/// Interface voices play THROUGH the freeze, by bus rather than by a guard:
/// the interface is what the player is looking at while the world is stopped.
pub fn pause_world_voices(mut freeze: ResMut<SoundFreeze>, q_voices: WorldSinkQuery) {
    freeze.set_if_neq(SoundFreeze(true));
    for (legacy, sound, sink, spatial_sink) in &q_voices {
        if !VoiceView::of(legacy, sound).is_some_and(|view| view.world) {
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
    for (legacy, sound, sink, spatial_sink) in &q_voices {
        if !VoiceView::of(legacy, sound).is_some_and(|view| view.world) {
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

/// Retire every world voice when gameplay ends, and silence every
/// [`SoundLoop`].
///
/// A loop's level is only driven inside a scenario-gated set, so between one
/// scenario ending and the next becoming live the last engine hum would keep
/// roaring at its final level through the whole load. Its voice comes back
/// silent and waits for its owner to raise it again. Interface voices are left
/// alone: they belong to the UI that spawned them and are torn down with it.
pub fn stop_world_voices(
    mut commands: Commands,
    q_voices: Query<(Entity, Option<&SfxVoice>, Option<&SoundVoice>), With<VoiceMarker>>,
    mut q_loops: Query<&mut SoundLoop>,
) {
    for (entity, legacy, sound) in &q_voices {
        if VoiceView::of(legacy, sound).is_some_and(|view| view.world) {
            // try_despawn: this runs on the way out of a scenario, which is
            // exactly when the scoped sweep is despawning the same voices.
            commands.entity(entity).try_despawn();
        }
    }
    for mut sound_loop in &mut q_loops {
        sound_loop.intensity = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::{ClipLoudness, SoundCategoryType, LOUDNESS_TARGET_LUFS};

    fn voice_at(route: AudioRoute, volume: f32, point: Vec3) -> SfxVoice {
        SfxVoice::one_shot(Handle::default(), route)
            .with_volume(volume)
            .at(point)
    }

    fn listener_at(position: Vec3) -> Hearing {
        Hearing::new(
            GlobalTransform::from(Transform::from_translation(position)),
            None,
        )
    }

    #[test]
    fn a_hull_voice_is_never_attenuated_however_far_the_camera_pulls_back() {
        // The routing fact that replaces the old "if this is the player, skip
        // attenuation" special case: the survey dolly stretches the camera to
        // 250 world units (2.5 km) and your own engines must not fade for it.
        let listener = listener_at(Vec3::new(0.0, 0.0, 250.0));
        let voice = voice_at(AudioRoute::Hull, 0.3, Vec3::ZERO);
        let placement = place_legacy(&voice, 1.0, Some(&listener), &VoicePoint::At(Vec3::ZERO));
        assert_eq!(placement.level, 0.3);
        assert!(
            placement.emitter.is_none(),
            "a hull voice is not placed in the world at all"
        );
    }

    #[test]
    fn an_exterior_voice_keeps_the_tuned_rolloff_as_its_amplitude_law() {
        // The pan must not smuggle in rodio's own 1/d law: the LEVEL is exactly
        // the engine's curve, and the pan rides on top as a ratio.
        let listener = listener_at(Vec3::ZERO);
        let source = Vec3::new(170.0, 0.0, 0.0);
        let voice = voice_at(AudioRoute::Exterior, 0.4, source);
        let placement = place_legacy(&voice, 1.0, Some(&listener), &VoicePoint::At(source));
        let expected = 0.4 * distance_attenuation(170.0);
        assert!(
            (placement.level - expected).abs() < 1e-6,
            "expected the tuned rolloff {expected}, got {}",
            placement.level
        );
        assert!(expected > 0.0 && expected < 0.4, "the rolloff must bite");
        let emitter = placement.emitter.expect("an exterior voice is placed");
        assert!(
            (emitter.length() - super::super::spatial::SPATIAL_EMITTER_RADIUS).abs() < 1e-4,
            "and parked on the fixed-radius sphere, not at 170 u"
        );
    }

    #[test]
    fn an_exterior_voice_beyond_the_far_distance_is_silent() {
        let listener = listener_at(Vec3::ZERO);
        let source = Vec3::new(1000.0, 0.0, 0.0);
        let voice = voice_at(AudioRoute::Exterior, 1.0, source);
        let placement = place_legacy(&voice, 1.0, Some(&listener), &VoicePoint::At(source));
        assert_eq!(placement.level, 0.0);
        assert!(placement.level < SFX_AUDIBLE_THRESHOLD);
    }

    #[test]
    fn a_voice_following_a_despawned_entity_holds_silence() {
        // The alternative - falling back to non-positional - would make a dead
        // ship's engine jump to full volume in the player's ear.
        let listener = listener_at(Vec3::ZERO);
        let voice = SfxVoice::looping(Handle::default(), AudioRoute::Exterior).with_volume(0.5);
        let placement = place_legacy(&voice, 1.0, Some(&listener), &VoicePoint::Lost);
        assert_eq!(placement.level, 0.0);
        assert_eq!(placement.gain, 0.0);
    }

    #[test]
    fn a_positional_voice_without_a_listener_degrades_to_full_volume() {
        let voice = voice_at(AudioRoute::Exterior, 0.5, Vec3::new(500.0, 0.0, 0.0));
        let placement = place_legacy(
            &voice,
            1.0,
            None,
            &VoicePoint::At(Vec3::new(500.0, 0.0, 0.0)),
        );
        assert_eq!(placement.level, 0.5, "no listener yet: play, do not vanish");
        assert!(placement.emitter.is_none());
    }

    #[test]
    fn the_bus_gain_scales_the_level() {
        let listener = listener_at(Vec3::ZERO);
        let voice = voice_at(AudioRoute::Exterior, 0.8, Vec3::ZERO);
        let half = place_legacy(&voice, 0.5, Some(&listener), &VoicePoint::At(Vec3::ZERO));
        assert!((half.level - 0.4).abs() < 1e-6);
        let silenced = place_legacy(&voice, 0.0, Some(&listener), &VoicePoint::At(Vec3::ZERO));
        assert_eq!(silenced.level, 0.0, "a track at zero plays nothing");
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
        let voice = app
            .world_mut()
            .spawn(SfxVoice::looping(Handle::default(), AudioRoute::Exterior).with_volume(0.5))
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
        app.world_mut()
            .spawn((SfxListenerMarker, GlobalTransform::IDENTITY));
        let exterior = |point: Vec3| {
            SfxVoice::looping(Handle::default(), AudioRoute::Exterior)
                .with_volume(1.0)
                .with_speed(0.5)
                .at(point)
        };
        let near = app.world_mut().spawn(exterior(Vec3::X * 60.0)).id();
        let far = app.world_mut().spawn(exterior(Vec3::X * 200.0)).id();
        let source = app
            .world_mut()
            .spawn(GlobalTransform::from_translation(Vec3::X * 60.0))
            .id();
        let leaving = app
            .world_mut()
            .spawn(
                SfxVoice::looping(Handle::default(), AudioRoute::Exterior)
                    .with_volume(1.0)
                    .following(source),
            )
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
        assert!(mix(far).gain > 0.0, "far is still inside the rolloff");
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

    /// A frozen sim pauses World-bus sinks, so capture must hear them paused
    /// too; interface voices keep playing behind the overlay.
    #[test]
    fn a_frozen_sim_publishes_world_voices_as_paused() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.insert_resource(SoundFreeze(true));
        app.add_systems(Update, (start_sfx_voices, drive_sfx_voices).chain());
        let hull = app
            .world_mut()
            .spawn(SfxVoice::looping(Handle::default(), AudioRoute::Hull).with_volume(1.0))
            .id();
        let interface = app
            .world_mut()
            .spawn(SfxVoice::looping(Handle::default(), AudioRoute::Interface).with_volume(1.0))
            .id();

        app.update();

        let paused = |entity| app.world().get::<VoiceMix>(entity).unwrap().paused;
        assert!(paused(hull), "a World-bus voice is held by the freeze");
        assert!(!paused(interface), "an interface voice plays through it");
    }

    /// Capture silences a voice only on the engine's word: past the cap, the
    /// quietest exterior loop publishes `capped` and the rest do not.
    #[test]
    fn the_quietest_loop_past_the_cap_publishes_as_capped() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.add_systems(Update, (start_sfx_voices, drive_sfx_voices).chain());
        app.world_mut()
            .spawn((SfxListenerMarker, GlobalTransform::IDENTITY));
        let voices: Vec<Entity> = (0..=MAX_EFFECT_LOOP_VOICES)
            .map(|i| {
                let point = Vec3::X * (40.0 + 10.0 * i as f32);
                app.world_mut()
                    .spawn(
                        SfxVoice::looping(Handle::default(), AudioRoute::Exterior)
                            .with_volume(1.0)
                            .at(point),
                    )
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
        }
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
                SfxVoice::one_shot(Handle::default(), AudioRoute::Interface),
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
        let voice = app
            .world_mut()
            .spawn(SfxVoice::one_shot(Handle::default(), AudioRoute::Interface))
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

        // One voice per retiring path. The first is silent, so the start pass
        // drops it instead of opening a player; the second has been waiting
        // out its grace with no sink; the third is a world loop, which is what
        // the end of a scenario silences.
        let inaudible = app
            .world_mut()
            .spawn(voice_at(AudioRoute::Exterior, 0.0, Vec3::ZERO))
            .id();
        let stranded = app
            .world_mut()
            .spawn((
                SfxVoice::one_shot(Handle::default(), AudioRoute::Interface),
                AwaitingSink(Duration::ZERO),
            ))
            .id();
        let world_loop = app
            .world_mut()
            .spawn(SfxVoice::looping(Handle::default(), AudioRoute::Hull))
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
    /// unrouted, unattenuated, unpannable, and deaf to every setting.
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
                let Ok(source) = std::fs::read_to_string(path) else {
                    return;
                };
                for (line_number, line) in source.lines().enumerate() {
                    // Strip line comments: the doc comments in this module name
                    // the constructor on purpose.
                    let code = line.split("//").next().unwrap_or_default();
                    // Both ways bevy lets you build one - the tuple struct and
                    // the convenience constructor.
                    if code.contains("AudioPlayer(") || code.contains("AudioPlayer::new") {
                        offenders.push(format!("{}:{}", path.display(), line_number + 1));
                    }
                }
            });
        }
        assert!(
            offenders.is_empty(),
            "audio must go through the engine, not straight to bevy. \
             Trigger PlaySound or add a SoundLoop instead of building an AudioPlayer at:\n  {}",
            offenders.join("\n  ")
        );
    }

    /// The one file allowed to name bevy's playback component, as a path
    /// suffix so the check reads the same from any checkout.
    const THE_ONE_PLACE: &str = "nova_sound/src/voice.rs";

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
