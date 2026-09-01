//! The engine's one playback path.
//!
//! Every sound in the game is an [`SfxVoice`] entity, and this file is the ONLY
//! place an `AudioPlayer` is constructed - pinned by
//! `the_engine_is_the_only_place_an_audio_player_is_built`, which reads the
//! crates' sources. A one-shot is a voice the engine spawns and retires for
//! you (see [`PlaySfx`](super::PlaySfx)); a loop is a voice its owner spawns,
//! keeps, and moves [`volume`](SfxVoice::volume) on each frame.
//!
//! What the engine owns, so no caller re-implements it: the bus gain, the
//! distance rolloff, the stereo placement, the freeze behind a paused sim, the
//! silence when a scenario unloads, and the cap on how many exterior loops may
//! sound at once.

use std::collections::HashMap;

use bevy::{
    audio::{PlaybackMode, SpatialAudioSink, Volume},
    prelude::*,
};

use super::{
    bus::{AudioRoute, Mixer},
    mixing::{distance_attenuation, SfxListenerMarker, SFX_AUDIBLE_THRESHOLD},
    spatial::{emitter_point, listener_ears, local_bearing, pan_compensation},
};

/// How many [`AudioRoute::Exterior`] LOOPS may sound at once.
///
/// Loops are per-source now (every burning ship has its own), so a busy scene
/// could otherwise open a sink for every thruster in it. The loudest voices
/// keep playing and the rest are held at silence - they stay alive and are
/// re-ranked every frame, so a ship that closes the range fades back in rather
/// than restarting. [`AudioRoute::Hull`] is never capped: the player's own ship
/// is a fixed, small handful of voices and is the one thing that must always be
/// audible.
pub const MAX_EXTERIOR_LOOP_VOICES: usize = 8;

/// Where a voice is heard from. Read only for [`AudioRoute::Exterior`] -
/// interface, hull and music voices are non-positional by definition.
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

/// One sound the engine is playing: what it is, which track scales it, how loud
/// its owner wants it, and where it is heard from.
///
/// Spawn one to start a sound. For a loop, keep the entity and write
/// [`volume`](Self::volume) each frame; the engine reads it and does the rest.
/// For a one-shot, prefer [`PlaySfx`](super::PlaySfx), which spawns and retires
/// the voice for you.
#[derive(Component, Clone, Debug)]
pub struct SfxVoice {
    /// The sound to play.
    pub handle: Handle<AudioSource>,
    /// Which track scales it, and whether it is placed in the world.
    pub route: AudioRoute,
    /// The owner's linear level, before the bus gain, the rolloff and the pan.
    pub volume: f32,
    /// Playback speed, which also shifts pitch (1.0 is normal).
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

/// What an [`SfxSource`] resolves to this frame.
enum VoicePoint {
    /// Not placed in the world.
    Unplaced,
    /// A world point.
    At(Vec3),
    /// The followed entity is gone.
    Lost,
}

/// The mix decision for one voice this frame.
struct VoicePlacement {
    /// The intended amplitude: the owner's level through its bus and the
    /// distance rolloff, BEFORE the pan compensation. This is what "how loud is
    /// this" means - the audible-threshold gate and the voice-cap ranking both
    /// read it.
    level: f32,
    /// The sink gain: [`Self::level`] with the pan compensation folded in.
    gain: f32,
    /// Where to park the spatial emitter, or `None` for a non-positional voice.
    emitter: Option<Vec3>,
}

/// Mix one voice: bus gain, rolloff, pan.
///
/// A positional voice with no listener (early startup, the editor) or no place
/// to be heard from falls back to non-positional at full range, mirroring how
/// the rest of the engine degrades rather than going silent.
fn place_voice(
    voice: &SfxVoice,
    bus_gain: f32,
    listener: Option<&GlobalTransform>,
    point: &VoicePoint,
) -> VoicePlacement {
    let flat = |level: f32| VoicePlacement {
        level,
        gain: level,
        emitter: None,
    };
    if matches!(point, VoicePoint::Lost) {
        return flat(0.0);
    }
    let base = voice.volume.max(0.0) * bus_gain;
    let (VoicePoint::At(source), true, Some(listener)) =
        (point, voice.route.is_positional(), listener)
    else {
        return flat(base);
    };

    let level = base * distance_attenuation(listener.translation().distance(*source));
    let bearing = local_bearing(listener, *source);
    VoicePlacement {
        level,
        gain: level * pan_compensation(bearing),
        emitter: Some(emitter_point(listener, bearing)),
    }
}

/// The `AudioPlayer` bundle for one voice. THE constructor - see the module
/// doc.
fn voice_player(voice: &SfxVoice, gain: f32) -> impl Bundle {
    (
        AudioPlayer(voice.handle.clone()),
        PlaybackSettings {
            mode: if voice.looping {
                PlaybackMode::Loop
            } else {
                PlaybackMode::Despawn
            },
            // WITHOUT the master: bevy's `audio_output` multiplies
            // `GlobalVolume` in once at sink creation. Every later write goes
            // through `Mixer::output_gain`, which folds it back in by hand.
            volume: Volume::Linear(gain.max(0.0)),
            // Rodio does not accept a non-positive playback rate.
            speed: voice.speed.max(f32::MIN_POSITIVE),
            spatial: voice.route.is_positional(),
            ..default()
        },
    )
}

/// Give the listener camera its ears the moment it is marked, so nothing has to
/// remember to spawn a [`SpatialListener`] beside the marker.
pub(super) fn on_add_listener(add: On<Add, SfxListenerMarker>, mut commands: Commands) {
    commands.entity(add.entity).insert(listener_ears());
}

/// Start every voice that has no player yet.
///
/// A ONE-SHOT that is already inaudible is despawned instead: opening a sink
/// for a cue nobody can hear is the churn the audible threshold exists to
/// avoid. A silent LOOP is kept - it is waiting to be raised.
pub(super) fn start_sfx_voices(
    mut commands: Commands,
    mixer: Mixer,
    q_new: Query<(Entity, &SfxVoice), Without<AudioPlayer>>,
    q_listener: Query<&GlobalTransform, (With<SfxListenerMarker>, Without<SfxVoice>)>,
    q_pose: Query<&GlobalTransform, Without<SfxVoice>>,
) {
    let listener = q_listener.iter().next();
    for (entity, voice) in &q_new {
        let point = resolve_point(voice.source, &q_pose);
        let placement = place_voice(voice, mixer.bus_gain(voice.route), listener, &point);
        if !voice.looping && placement.level < SFX_AUDIBLE_THRESHOLD {
            commands.entity(entity).despawn();
            continue;
        }
        commands.entity(entity).insert((
            voice_player(voice, placement.gain),
            GlobalTransform::from_translation(placement.emitter.unwrap_or_default()),
        ));
    }
}

/// Re-mix every playing voice: its bus gain, its distance, its bearing.
///
/// Runs every frame rather than at spawn because a voice's mix is a function of
/// two things that both move - the source and the listener - so a cue that only
/// panned where it started would swing wrong the moment the camera turned.
pub(super) fn drive_sfx_voices(
    mixer: Mixer,
    q_listener: Query<&GlobalTransform, (With<SfxListenerMarker>, Without<SfxVoice>)>,
    q_pose: Query<&GlobalTransform, Without<SfxVoice>>,
    mut q_voices: Query<(
        Entity,
        &SfxVoice,
        &mut GlobalTransform,
        Option<&mut AudioSink>,
        Option<&mut SpatialAudioSink>,
    )>,
) {
    let listener = q_listener.iter().next();
    let mut placements: HashMap<Entity, VoicePlacement> = HashMap::new();
    let mut exterior_loops: Vec<f32> = Vec::new();
    for (entity, voice, ..) in &q_voices {
        let point = resolve_point(voice.source, &q_pose);
        let placement = place_voice(voice, mixer.bus_gain(voice.route), listener, &point);
        if voice.looping && voice.route.is_positional() {
            exterior_loops.push(placement.level);
        }
        placements.insert(entity, placement);
    }
    let loop_floor = exterior_loop_floor(&mut exterior_loops);
    let master = mixer.master_gain();

    for (entity, voice, mut pose, sink, spatial_sink) in &mut q_voices {
        let Some(placement) = placements.get(&entity) else {
            continue;
        };
        let capped = voice.looping && voice.route.is_positional() && placement.level < loop_floor;
        let gain = if capped { 0.0 } else { placement.gain * master };
        if let Some(emitter) = placement.emitter {
            *pose = GlobalTransform::from_translation(emitter);
        }
        if let Some(mut sink) = sink {
            sink.set_volume(Volume::Linear(gain));
        }
        if let Some(mut sink) = spatial_sink {
            sink.set_volume(Volume::Linear(gain));
        }
    }
}

/// The quietest level an exterior loop may hold and still sound, given
/// [`MAX_EXTERIOR_LOOP_VOICES`]. Below the cap every voice sounds, so the floor
/// is zero.
fn exterior_loop_floor(levels: &mut Vec<f32>) -> f32 {
    if levels.len() <= MAX_EXTERIOR_LOOP_VOICES {
        return 0.0;
    }
    levels.sort_by(|a, b| b.total_cmp(a));
    levels[MAX_EXTERIOR_LOOP_VOICES - 1]
}

fn resolve_point(
    source: SfxSource,
    q_pose: &Query<&GlobalTransform, Without<SfxVoice>>,
) -> VoicePoint {
    match source {
        SfxSource::Unplaced => VoicePoint::Unplaced,
        SfxSource::At(point) => VoicePoint::At(point),
        SfxSource::Follow(entity) => match q_pose.get(entity) {
            Ok(pose) => VoicePoint::At(pose.translation()),
            Err(_) => VoicePoint::Lost,
        },
    }
}

/// Freeze the WORLD voices while the sim is frozen (the pause overlay or the
/// Tab ship computer). Audio sinks do not follow `Time<Virtual>`, so without
/// this a loop keeps roaring at its last volume behind a stopped world.
///
/// Interface voices play THROUGH the freeze, by routing rather than by a guard:
/// the interface is what the player is looking at while the world is stopped,
/// and its own hum is not part of the world that stopped.
pub(super) fn pause_world_voices(
    q_voices: Query<(&SfxVoice, Option<&AudioSink>, Option<&SpatialAudioSink>)>,
) {
    for (voice, sink, spatial_sink) in &q_voices {
        if voice.route.bus() != super::bus::AudioBus::World {
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
pub(super) fn resume_world_voices(
    q_voices: Query<(&SfxVoice, Option<&AudioSink>, Option<&SpatialAudioSink>)>,
) {
    for (voice, sink, spatial_sink) in &q_voices {
        if voice.route.bus() != super::bus::AudioBus::World {
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

/// Retire every world voice when gameplay ends.
///
/// Loop entities outlive the ship that raised them while their levels are only
/// driven inside a scenario-gated set, so between one scenario ending and the
/// next becoming live the last engine hum would keep roaring at its final
/// volume through the whole load. Interface voices are left alone: they belong
/// to the UI that spawned them and are torn down with it.
pub(super) fn stop_world_voices(mut commands: Commands, q_voices: Query<(Entity, &SfxVoice)>) {
    for (entity, voice) in &q_voices {
        if voice.route.bus() == super::bus::AudioBus::World {
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;

    fn voice_at(route: AudioRoute, volume: f32, point: Vec3) -> SfxVoice {
        SfxVoice::one_shot(Handle::default(), route)
            .with_volume(volume)
            .at(point)
    }

    fn listener_at(position: Vec3) -> GlobalTransform {
        GlobalTransform::from(Transform::from_translation(position))
    }

    #[test]
    fn a_hull_voice_is_never_attenuated_however_far_the_camera_pulls_back() {
        // The routing fact that replaces the old "if this is the player, skip
        // attenuation" special case: the survey dolly stretches the camera to
        // 250 u and your own engines must not fade for it.
        let listener = listener_at(Vec3::new(0.0, 0.0, 250.0));
        let voice = voice_at(AudioRoute::Hull, 0.3, Vec3::ZERO);
        let placement = place_voice(&voice, 1.0, Some(&listener), &VoicePoint::At(Vec3::ZERO));
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
        let placement = place_voice(&voice, 1.0, Some(&listener), &VoicePoint::At(source));
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
        let placement = place_voice(&voice, 1.0, Some(&listener), &VoicePoint::At(source));
        assert_eq!(placement.level, 0.0);
        assert!(placement.level < SFX_AUDIBLE_THRESHOLD);
    }

    #[test]
    fn a_voice_following_a_despawned_entity_holds_silence() {
        // The alternative - falling back to non-positional - would make a dead
        // ship's engine jump to full volume in the player's ear.
        let listener = listener_at(Vec3::ZERO);
        let voice = SfxVoice::looping(Handle::default(), AudioRoute::Exterior).with_volume(0.5);
        let placement = place_voice(&voice, 1.0, Some(&listener), &VoicePoint::Lost);
        assert_eq!(placement.level, 0.0);
        assert_eq!(placement.gain, 0.0);
    }

    #[test]
    fn a_positional_voice_without_a_listener_degrades_to_full_volume() {
        let voice = voice_at(AudioRoute::Exterior, 0.5, Vec3::new(500.0, 0.0, 0.0));
        let placement = place_voice(
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
        let half = place_voice(&voice, 0.5, Some(&listener), &VoicePoint::At(Vec3::ZERO));
        assert!((half.level - 0.4).abs() < 1e-6);
        let silenced = place_voice(&voice, 0.0, Some(&listener), &VoicePoint::At(Vec3::ZERO));
        assert_eq!(silenced.level, 0.0, "a track at zero plays nothing");
    }

    #[test]
    fn the_exterior_loop_cap_keeps_the_loudest_voices() {
        // Under the cap nothing is cut.
        let mut few: Vec<f32> = (0..MAX_EXTERIOR_LOOP_VOICES).map(|i| i as f32).collect();
        assert_eq!(exterior_loop_floor(&mut few), 0.0);

        // Over it, the floor is the quietest voice that still sounds, so
        // exactly `MAX_EXTERIOR_LOOP_VOICES` are at or above it.
        let mut many: Vec<f32> = (0..MAX_EXTERIOR_LOOP_VOICES + 5)
            .map(|i| 0.1 * i as f32)
            .collect();
        let floor = exterior_loop_floor(&mut many);
        let kept = many.iter().filter(|level| **level >= floor).count();
        assert_eq!(kept, MAX_EXTERIOR_LOOP_VOICES, "floor was {floor}");
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
        let crates = workspace_root().join("crates");
        let mut offenders = Vec::new();
        visit_rust_sources(&crates, &mut |path| {
            if path.ends_with(THE_ONE_PLACE) {
                return;
            }
            let Ok(source) = std::fs::read_to_string(path) else {
                return;
            };
            for (line_number, line) in source.lines().enumerate() {
                // Strip line comments: the doc comments in this module name the
                // constructor on purpose.
                let code = line.split("//").next().unwrap_or_default();
                if code.contains("AudioPlayer(") {
                    offenders.push(format!("{}:{}", path.display(), line_number + 1));
                }
            }
        });
        assert!(
            offenders.is_empty(),
            "audio must go through the engine, not straight to bevy. \
             Spawn an SfxVoice (or trigger PlaySfx) instead of an AudioPlayer at:\n  {}",
            offenders.join("\n  ")
        );
    }

    /// The one file allowed to name bevy's playback component, as a path
    /// suffix so the check reads the same from any checkout.
    const THE_ONE_PLACE: &str = "nova_gameplay/src/audio/voice.rs";

    fn workspace_root() -> PathBuf {
        // CARGO_MANIFEST_DIR is `<root>/crates/nova_gameplay`.
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
