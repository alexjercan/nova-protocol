//! Listen to one mono cue at six bearings around a stationary camera.
//!
//! Run `cargo run --example spatial_audio_directions`. Use headphones for the
//! left/right comparison. Forward, back, up, and down have centered stereo
//! balance; their labels indicate placement, not an audible height or depth cue.

use bevy::{asset::LoadState, prelude::*};
use nova_protocol::prelude::*;

const SOUND: &str = "base/sounds/radar_deny.wav";
const RADIUS: f32 = 2.5;
const DWELL_SECS: f64 = 3.0;
const READY_DEADLINE_SECS: f64 = 15.0;
const VOICE_DEADLINE_SECS: f64 = 2.0;
const DIRECTIONS: [(&str, Vec3); 6] = [
    ("LEFT  <", Vec3::NEG_X),
    ("RIGHT  >", Vec3::X),
    ("FORWARD  ^  (-Z)", Vec3::NEG_Z),
    ("BACK  v  (+Z)", Vec3::Z),
    ("UP  ^  (+Y)", Vec3::Y),
    ("DOWN  v  (-Y)", Vec3::NEG_Y),
];

#[derive(Resource)]
struct ListeningCycle {
    clip: Handle<AudioSource>,
    phase: usize,
    since: f64,
    fired_at: Option<f64>,
    voice: Option<Entity>,
    source: Vec3,
    last_mix: Option<[f32; 2]>,
    sink_seen: bool,
    error: Option<String>,
}

#[derive(Component)]
struct ListeningReadout;

fn main() -> AppExit {
    AppBuilder::new()
        .with_game_plugins(spatial_audio_plugin)
        .build()
        .run()
}

fn spatial_audio_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameStates::Playing), setup_listening);
    // The engine mixes in PostUpdate; Update reads the preceding frame's mix.
    app.add_systems(
        Update,
        advance_listening.run_if(in_state(GameStates::Playing)),
    );
}

fn setup_listening(mut commands: Commands, assets: Res<AssetServer>, time: Res<Time<Real>>) {
    commands.spawn((
        Name::new("Spatial listening camera"),
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb(0.02, 0.04, 0.08)),
            ..default()
        },
        Transform::default(),
        SfxListenerMarker,
    ));
    commands.spawn((
        ListeningReadout,
        Text::new("Loading spatial audio..."),
        TextFont {
            font_size: FontSize::Px(30.0),
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(35.0),
            left: Val::Percent(12.0),
            ..default()
        },
    ));
    commands.insert_resource(ListeningCycle {
        clip: assets.load(SOUND),
        phase: 0,
        since: time.elapsed_secs_f64(),
        fired_at: None,
        voice: None,
        source: Vec3::ZERO,
        last_mix: None,
        sink_seen: false,
        error: None,
    });
}

fn advance_listening(
    mut commands: Commands,
    time: Res<Time<Real>>,
    assets: Res<AssetServer>,
    mut cycle: ResMut<ListeningCycle>,
    listener: Query<(&GlobalTransform, &SpatialListener), With<SfxListenerMarker>>,
    voices: Query<(Entity, &SfxVoice, &VoiceMix, Option<&SpatialAudioSink>)>,
    mut readout: Single<&mut Text, With<ListeningReadout>>,
) {
    let now = time.elapsed_secs_f64();
    if cycle.error.is_none() {
        match assets.get_load_state(cycle.clip.id()) {
            Some(LoadState::Failed(error)) => {
                cycle.error = Some(format!("Sound load failed: {error}"))
            }
            Some(LoadState::Loaded) => {}
            _ if now - cycle.since > READY_DEADLINE_SECS => {
                cycle.error = Some(format!(
                    "Sound did not load within {READY_DEADLINE_SECS}s: {SOUND}"
                ));
            }
            _ => {
                readout.0 = format!("Loading mono cue: {SOUND}");
                return;
            }
        }
    }
    if let Some(error) = &cycle.error {
        readout.0 = format!("SPATIAL AUDIO ERROR\n{error}");
        return;
    }
    let Ok((pose, _ears)) = listener.single() else {
        if now - cycle.since > READY_DEADLINE_SECS {
            cycle.error = Some("Expected one camera with spatial listener ears".to_string());
            readout.0 = format!(
                "SPATIAL AUDIO ERROR\n{}",
                cycle.error.as_deref().unwrap_or_default()
            );
        } else {
            readout.0 = "Waiting for camera and spatial listener ears...".to_string();
        }
        return;
    };
    if cycle.fired_at.is_none() {
        let (label, direction) = DIRECTIONS[cycle.phase];
        let source = pose.transform_point(direction * RADIUS);
        commands.trigger(PlaySfx::new(cycle.clip.clone(), AudioRoute::Exterior).at(source));
        cycle.fired_at = Some(now);
        cycle.voice = None;
        cycle.source = source;
        cycle.last_mix = None;
        cycle.sink_seen = false;
        info!("spatial audio: {label} source={source:?}");
    }
    if let Some((entity, _, mix, sink)) = voices.iter().find(|(entity, voice, _, _)| {
        voice.handle.id() == cycle.clip.id()
            && voice.route == AudioRoute::Exterior
            && voice.source == SfxSource::At(cycle.source)
            && (cycle.voice.is_none() || cycle.voice == Some(*entity))
    }) {
        cycle.voice = Some(entity);
        if cycle.last_mix.is_none() {
            info!(
                "spatial audio: {} VoiceMix left={:.3} right={:.3}",
                DIRECTIONS[cycle.phase].0, mix.channel_gains[0], mix.channel_gains[1]
            );
        }
        cycle.last_mix = Some(mix.channel_gains);
        cycle.sink_seen |= sink.is_some();
    }
    if now - cycle.fired_at.unwrap_or(now) > VOICE_DEADLINE_SECS {
        if cycle.last_mix.is_none() {
            cycle.error =
                Some("No mixed exterior voice: check listener, route, and volume".to_string());
        } else if !cycle.sink_seen {
            cycle.error = Some("No spatial audio sink: check the audio device and cue".to_string());
        }
    }
    if let Some(error) = &cycle.error {
        error!("spatial audio: {error}");
        readout.0 = format!("SPATIAL AUDIO ERROR\n{error}");
        return;
    }
    let (label, _) = DIRECTIONS[cycle.phase];
    let evidence = cycle.last_mix.map_or_else(
        || "Waiting for engine voice mix...".to_string(),
        |[left, right]| {
            format!(
                "VoiceMix L {left:.3} / R {right:.3} (pre-master)\nSpatial sink: {}",
                if cycle.sink_seen { "opened" } else { "waiting" }
            )
        },
    );
    readout.0 = format!(
        "SIX-DIRECTION LISTENING\n\n{label}\n\n{evidence}\n\nMono cue at {RADIUS} engine units\nLeft/right pan only. Centered directions\ndo not encode height or depth.\nPhase {}/6",
        cycle.phase + 1
    );
    if cycle.last_mix.is_some() && now - cycle.fired_at.unwrap_or(now) >= DWELL_SECS {
        cycle.phase = (cycle.phase + 1) % DIRECTIONS.len();
        cycle.fired_at = None;
    }
}
