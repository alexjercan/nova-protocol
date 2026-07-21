//! Salvage crate scenario object (task 20260712-093044, spike
//! docs/spikes/20260712-092926-starter-scenario.md): a minimal proximity
//! pickup. The crate is a small tumbling prop that doubles as its own
//! trigger area - flying through it fires `OnEnter` under the crate's
//! scenario id, and the scenario script pairs that with
//! `DespawnScenarioObject` plus whatever counting it wants. No inventory:
//! "collected" is scenario state, which keeps pickup consequences in
//! scenario data instead of a hardcoded item system.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;

use crate::prelude::*;

pub mod prelude {
    pub use super::{
        salvage_crate_scenario_object, SalvageCrateConfig, SalvageCratePlugin,
        SALVAGE_CRATE_TYPE_NAME,
    };
}

pub const SALVAGE_CRATE_TYPE_NAME: &str = "salvage_crate";

/// Tumble rate (radians/second) of the crate's render child.
const CRATE_TUMBLE_RAD_PER_SEC: f32 = 0.6;

/// Crate body color: bright against grey rock (layer-0 conveyance - the
/// prop advertises itself; spike, "Conveying objectives").
const CRATE_COLOR: Color = Color::srgb(1.0, 0.75, 0.15);

/// Self-glow band the highlight pulse sweeps (task 20260712-093831): the
/// old static 2.0 becomes a sine between these - visible motion against
/// static debris, still far dimmer than a beacon (8..60, a landmark).
/// The period is the shared item-highlight clock
/// (ITEM_HIGHLIGHT_PULSE_PERIOD_SECS, nova_gameplay), so the mesh glow and
/// the HUD bracket breathe together.
const CRATE_EMISSIVE_MIN: f32 = 3.0;
const CRATE_EMISSIVE_MAX: f32 = 6.0;

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SalvageCrateConfig {
    /// Edge length of the crate's visible box (world units).
    pub size: f32,
    /// Pickup radius: the sensor sphere that counts as "collected".
    pub area_radius: f32,
    /// The pickup "ding" this crate plays, an authorable
    /// [`AssetRef<AudioSource>`] like any other content sound (task
    /// 20260717-101659, spike 20260717-101524 - the LAST world sound to move
    /// onto content; the transitional WorldSfx bank is gone). AUTHORED-OR-
    /// SILENT: an omitted sound picks up quietly; base crates author
    /// `self://sounds/salvage_pickup.wav`.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub pickup_sound: Option<AssetRef<AudioSource>>,
}

/// The crate's authored pickup ding, snapshotted UNRESOLVED from
/// [`SalvageCrateConfig::pickup_sound`] by the bundle; the pickup cue resolves
/// it.
#[derive(Component, Clone, Debug)]
struct SalvageCratePickupSound(Option<AssetRef<AudioSource>>);

pub fn salvage_crate_scenario_object(config: SalvageCrateConfig) -> impl Bundle {
    debug!("salvage_crate_scenario_object: config {:?}", config);

    (
        SalvageCrateMarker,
        EntityTypeName::new(SALVAGE_CRATE_TYPE_NAME),
        SalvageCrateSize(config.size),
        SalvageCratePickupSound(config.pickup_sound.clone()),
        // Every pickup advertises itself (task 20260712-093831): the HUD's
        // item-highlights observer grows a bracket sized to the crate's
        // VISIBLE half-diagonal (authored, not collider-derived - the only
        // collider here is the sensor sphere, review R1.1). Intrinsic, not
        // scenario data - a silent pickup is a bug.
        ItemHighlight::new(config.size * 3f32.sqrt() / 2.0),
        // The pickup volume: the crate IS its own trigger area, so OnEnter
        // fires under its scenario id via the area plugin.
        ScenarioAreaMarker,
        Collider::sphere(config.area_radius),
        Sensor,
        // On rails: a sensor-only collider contributes no mass, so a
        // Dynamic crate would be an avian zero-mass warning; the visual
        // tumble is a render-child animation, not physics.
        RigidBody::Static,
    )
}

/// Marks a salvage crate root.
#[derive(Component, Clone, Debug, Reflect)]
pub struct SalvageCrateMarker;

/// Render input: the crate's visible edge length.
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub struct SalvageCrateSize(pub f32);

/// The tumble driver on the crate's render child: axis and phase seeded
/// from the entity index so a cluster of crates does not spin in lockstep.
#[derive(Component, Clone, Debug, Reflect)]
pub struct CrateTumble {
    pub axis: Vec3,
}

/// The glow driver on the crate's render child: the material whose
/// emissive the highlight pulse sweeps. Unlike beacons, NO per-entity
/// phase: the crates and their HUD brackets share one clock, so the whole
/// highlight system moves as one.
#[derive(Component, Clone, Debug, Reflect)]
pub struct CrateGlow {
    pub material: Handle<StandardMaterial>,
}

/// The salvage crate scenario object: a static proximity pickup that fires
/// `OnEnter` under its own scenario id. `render` gates the visible box, tumble
/// and glow; the pickup ding is audio and registered regardless.
/// Adds the pickup-cue and despawn-cleanup observers unconditionally, and (when
/// `render`) the crate-render observer plus the tumble and glow `Update`
/// systems.
pub struct SalvageCratePlugin {
    pub render: bool,
}

impl Plugin for SalvageCratePlugin {
    fn build(&self, app: &mut App) {
        debug!("SalvageCratePlugin: build");

        // The per-crate pickup cue (task 20260714-090002) is audio, not render:
        // register it regardless of the render flag. It no-ops without a
        // SoundBank (editor, headless), so it is safe to add unconditionally.
        app.init_resource::<DingedCrates>();
        app.add_observer(on_crate_pickup_play_sfx);
        app.add_observer(forget_despawned_crate);

        if self.render {
            app.add_observer(insert_crate_render);
            app.add_systems(Update, (tumble_crates, pulse_crate_glow));
        }
    }
}

/// Crate entities that have already sounded their pickup ding. A player ship is
/// a compound of many section colliders on ONE rigid body, so avian fires a
/// separate `CollisionStart` per section that enters the crate sensor
/// (empirically 3+ for the shakedown ship). This set collapses that burst to a
/// single ding per crate, so a pickup is one ding regardless of collider count
/// or how the scenario's despawn happens to interleave with physics steps.
/// Pruned by [`forget_despawned_crate`] when a crate leaves the world.
#[derive(Resource, Default)]
struct DingedCrates(bevy::platform::collections::HashSet<Entity>);

/// Play the light pickup "ding" when the PLAYER flies into a salvage crate's
/// sensor (task 20260714-090002). It lives here, in the crate's own plugin,
/// because `SalvageCrateMarker` is a `nova_scenario` type and `nova_gameplay`'s
/// audio module - which owns every other cue - cannot see it.
///
/// Gated to the player on purpose: the scenario pickup handler filters on the
/// player entering, so an AI ship brushing a crate does NOT collect it and must
/// not ding either. Non-positional, like the objective and lock UI cues: the
/// pickup always happens at the player's own ship, so distance attenuation would
/// be a no-op. The cue is a graceful no-op until the [`SoundBank`] exists.
///
/// This observes the same `CollisionStart` the area plugin turns into the
/// scenario OnEnter - the truest pickup signal, firing on contact and never on
/// scenario teardown (unlike an `On<Remove>` on the marker) - but dedups per
/// crate via [`DingedCrates`] so one pickup is one ding even though a ship's
/// many section colliders each fire the event.
fn on_crate_pickup_play_sfx(
    collision: On<CollisionStart>,
    asset_server: Res<AssetServer>,
    q_crate: Query<&SalvageCratePickupSound, With<SalvageCrateMarker>>,
    q_player: Query<(), With<PlayerSpaceshipMarker>>,
    mut dinged: ResMut<DingedCrates>,
    mut commands: Commands,
) {
    let (Some(a), Some(b)) = (collision.body1, collision.body2) else {
        return;
    };
    // Identify which body is the crate; avian does not guarantee the ordering.
    let (crate_entity, pickup_sound) = if let Ok(sound) = q_crate.get(a) {
        (a, sound)
    } else if let Ok(sound) = q_crate.get(b) {
        (b, sound)
    } else {
        return;
    };
    let other = if crate_entity == a { b } else { a };
    // Only the player collects a crate (the scenario handler filters on it).
    if !q_player.contains(other) {
        return;
    }
    // `insert` returns true only the first time this crate is seen, collapsing
    // the per-section-collider burst to a single ding. AUTHORED-OR-SILENT
    // (spike 20260717-101524): the ding is the crate's own authored ref.
    if dinged.0.insert(crate_entity) {
        if let Some(handle) = pickup_sound.0.as_ref().map(|r| r.resolve(&asset_server)) {
            commands.play_sfx_volume(handle, SALVAGE_PICKUP_VOLUME);
        }
    }
}

/// Drop a crate from the ding-dedup set when it leaves the world (picked up or
/// torn down), keeping the set bounded and correct if an entity index is later
/// reused.
fn forget_despawned_crate(
    remove: On<Remove, SalvageCrateMarker>,
    mut dinged: ResMut<DingedCrates>,
) {
    dinged.0.remove(&remove.entity);
}

/// The visible crate: a bright box child, tumbling for life. A child (not
/// the root) so the pickup collider on the root keeps its own radius and
/// the tumble never rotates the sensor.
fn insert_crate_render(
    add: On<Add, SalvageCrateMarker>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    q_crate: Query<&SalvageCrateSize, With<SalvageCrateMarker>>,
) {
    let entity = add.entity;
    let Ok(size) = q_crate.get(entity) else {
        error!(
            "insert_crate_render: entity {:?} not found in q_crate",
            entity
        );
        return;
    };

    let material = materials.add(StandardMaterial {
        base_color: CRATE_COLOR,
        emissive: CRATE_COLOR.to_linear() * CRATE_EMISSIVE_MAX,
        ..default()
    });

    // A per-crate tumble axis from the entity index - decorrelated without
    // rand (scenario spawn order is stable, aesthetics only).
    let seed = entity.index_u32() as f32;
    let axis = Vec3::new((seed * 0.7).sin(), (seed * 1.3).cos(), (seed * 2.1).sin())
        .try_normalize()
        .unwrap_or(Vec3::Y);

    commands.entity(entity).insert(children![(
        Name::new("SalvageCrateBox"),
        Transform::default(),
        Mesh3d(meshes.add(Cuboid::from_length(**size))),
        MeshMaterial3d(material.clone()),
        CrateTumble { axis },
        CrateGlow { material },
    )]);
}

/// Sweep each crate's emissive between the glow band's floor and ceiling
/// on the shared item-highlight clock.
fn pulse_crate_glow(
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    q_glow: Query<&CrateGlow>,
) {
    for glow in &q_glow {
        let Some(mut material) = materials.get_mut(&glow.material) else {
            continue;
        };
        let t = time.elapsed_secs() * std::f32::consts::TAU / ITEM_HIGHLIGHT_PULSE_PERIOD_SECS;
        let wave = 0.5 + 0.5 * t.sin();
        let luminance = CRATE_EMISSIVE_MIN + (CRATE_EMISSIVE_MAX - CRATE_EMISSIVE_MIN) * wave;
        material.emissive = material.base_color.to_linear() * luminance;
    }
}

/// Spin each crate's render child around its tumble axis.
fn tumble_crates(time: Res<Time>, mut q_tumble: Query<(&CrateTumble, &mut Transform)>) {
    for (tumble, mut transform) in &mut q_tumble {
        transform.rotate(Quat::from_axis_angle(
            tumble.axis,
            CRATE_TUMBLE_RAD_PER_SEC * time.delta_secs(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The crate's contract: a static sensor trigger (the pickup volume,
    /// firing OnEnter under its scenario id) that never collides physically
    /// and never moves - the ScenarioAreaMarker + Sensor + Static trio.
    #[test]
    fn crate_is_a_static_sensor_trigger() {
        let mut world = World::new();
        let entity = world
            .spawn(salvage_crate_scenario_object(SalvageCrateConfig {
                size: 1.5,
                area_radius: 6.0,
                pickup_sound: None,
            }))
            .id();

        assert!(world.get::<SalvageCrateMarker>(entity).is_some());
        let highlight = world
            .get::<ItemHighlight>(entity)
            .expect("a pickup advertises itself: the HUD bracket hangs off this tag");
        // The bracket radius is the crate box's half-diagonal - the VISIBLE
        // extent, decoupled from the (much larger) sensor sphere (R1.1).
        let expected = 1.5 * 3f32.sqrt() / 2.0;
        assert!(
            (highlight.world_radius - expected).abs() < 1e-5,
            "highlight radius {} is the visible half-diagonal {expected}, not the 6.0 sensor",
            highlight.world_radius
        );
        assert!(world.get::<ScenarioAreaMarker>(entity).is_some());
        assert!(world.get::<Sensor>(entity).is_some());
        assert!(world.get::<Collider>(entity).is_some());
        assert!(matches!(
            world.get::<RigidBody>(entity),
            Some(RigidBody::Static)
        ));
    }

    /// The pickup path end to end through real physics AND the real event
    /// pipeline: a moving body entering the crate's sensor fires OnEnter
    /// under the CRATE's scenario id, an EventHandler filtered on
    /// (crate id, other id) matches it, and its VariableSet action lands in
    /// NovaEventWorld - the exact chain a scenario's salvage beat runs on.
    /// Delivery guard: the variable starts false and only the handler can
    /// flip it, so a quiet sensor fails the test.
    #[test]
    fn entering_the_crate_sensor_drives_the_scenario_event_pipeline() {
        use core::time::Duration;

        use bevy::time::TimeUpdateStrategy;
        use bevy_common_systems::prelude::{EventHandler, GameEventsPlugin, GameObjectives};

        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            TransformPlugin,
            AssetPlugin::default(),
            bevy::mesh::MeshPlugin,
            PhysicsPlugins::default(),
        ));
        app.insert_resource(Gravity(Vec3::ZERO));
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            0.02,
        )));
        app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.add_plugins(super::super::area::ScenarioAreaPlugin);
        app.finish();

        // The handler a scenario would register: OnEnter(crate_1, player) ->
        // picked_up = true.
        let mut handler = EventHandler::<NovaEventWorld>::from(crate::events::EventConfig::OnEnter);
        handler.add_filter(EventFilterConfig::Entity(EntityFilterConfig {
            id: Some("crate_1".to_string()),
            other_id: Some("player_spaceship".to_string()),
            ..default()
        }));
        handler.add_action(EventActionConfig::VariableSet(VariableSetActionConfig {
            key: "picked_up".to_string(),
            expression: VariableExpressionNode::new_term(VariableTermNode::new_factor(
                VariableFactorNode::new_literal(VariableLiteral::Boolean(true)),
            )),
        }));
        app.world_mut().spawn(handler);

        app.world_mut().spawn((
            salvage_crate_scenario_object(SalvageCrateConfig {
                size: 1.5,
                area_radius: 6.0,
                pickup_sound: None,
            }),
            EntityId::new("crate_1".to_string()),
            Transform::from_translation(Vec3::ZERO),
        ));
        // The mover: a player-ship stand-in flying straight through the
        // pickup volume.
        app.world_mut().spawn((
            EntityId::new("player_spaceship".to_string()),
            EntityTypeName::new("spaceship".to_string()),
            RigidBody::Dynamic,
            Collider::sphere(0.5),
            ColliderDensity(1.0),
            Transform::from_translation(Vec3::new(0.0, 0.0, 12.0)),
            LinearVelocity(Vec3::new(0.0, 0.0, -60.0)),
        ));

        assert!(
            app.world()
                .resource::<NovaEventWorld>()
                .get_variable("picked_up")
                .is_none(),
            "delivery guard: the variable does not exist until the handler runs"
        );

        // 12u at 60 u/s: inside the 6u sensor within ~0.15s; run half a
        // second of fixed ticks so collision, event queue and handler all
        // get their turns.
        for _ in 0..25 {
            app.update();
        }

        assert_eq!(
            app.world()
                .resource::<NovaEventWorld>()
                .get_variable("picked_up"),
            Some(&VariableLiteral::Boolean(true)),
            "the crate's OnEnter drove the filtered handler's action"
        );
    }

    /// Count of pickup dings observed, standing in for "sounds played". This
    /// rig wires no other cue source, so every `PlaySfx` is a crate ding.
    #[derive(Resource, Default)]
    struct PickupDings(usize);

    /// Count of `CollisionStart`s that touched a crate - the delivery guard for
    /// the negative test: it proves the mover really entered the sensor.
    #[derive(Resource, Default)]
    struct CrateCollisions(usize);

    /// The proven salvage-pipeline physics rig (zero gravity, manual fixed
    /// steps) with the pickup observer and a `PlaySfx` counter wired on. The
    /// crate carries `CollisionEventsEnabled` directly - production attaches it
    /// via the area plugin's `On<Add, ScenarioAreaMarker>` observer, but this
    /// test exercises only the audio seam, so it adds the one flag that seam
    /// needs rather than the whole ScenarioAreaPlugin.
    fn pickup_audio_app() -> App {
        use core::time::Duration;

        use bevy::time::TimeUpdateStrategy;

        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            TransformPlugin,
            AssetPlugin::default(),
            bevy::mesh::MeshPlugin,
            PhysicsPlugins::default(),
        ));
        app.init_asset::<AudioSource>();
        app.insert_resource(Gravity(Vec3::ZERO));
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            0.02,
        )));
        app.init_resource::<PickupDings>();
        app.init_resource::<DingedCrates>();
        app.add_observer(on_crate_pickup_play_sfx);
        app.add_observer(forget_despawned_crate);
        app.add_observer(|_: On<PlaySfx>, mut dings: ResMut<PickupDings>| dings.0 += 1);
        app.finish();
        app
    }

    /// Spawn a crate at the origin with the production bundle plus the
    /// collision-events flag; return its entity.
    fn spawn_pickup_crate(app: &mut App) -> Entity {
        app.world_mut()
            .spawn((
                salvage_crate_scenario_object(SalvageCrateConfig {
                    size: 1.5,
                    area_radius: 6.0,
                    pickup_sound: Some(AssetRef::from("base/sounds/salvage_pickup.wav")),
                }),
                CollisionEventsEnabled,
                Transform::from_translation(Vec3::ZERO),
            ))
            .id()
    }

    /// Spawn a body flying straight through the crate at the origin. `mover`
    /// gets whatever extra markers the caller bundles (the player marker, or
    /// nothing for an AI stand-in).
    fn spawn_mover(app: &mut App, extra: impl Bundle) {
        app.world_mut().spawn((
            RigidBody::Dynamic,
            Collider::sphere(0.5),
            ColliderDensity(1.0),
            Transform::from_translation(Vec3::new(0.0, 0.0, 12.0)),
            LinearVelocity(Vec3::new(0.0, 0.0, -60.0)),
            extra,
        ));
    }

    /// The pickup cue's happy path through real physics: a PLAYER body flying
    /// into a crate's sensor plays exactly one pickup ding. Delivery guard: the
    /// counter starts at zero, so a quiet sensor fails the test.
    #[test]
    fn a_player_flying_into_a_crate_dings_once() {
        let mut app = pickup_audio_app();
        spawn_pickup_crate(&mut app);
        spawn_mover(&mut app, PlayerSpaceshipMarker);

        assert_eq!(
            app.world().resource::<PickupDings>().0,
            0,
            "delivery guard: silent before the pass"
        );

        // 12u at 60 u/s: inside the 6u sensor within ~0.15s; a half second of
        // fixed ticks covers collision detection and the observer's flush.
        for _ in 0..25 {
            app.update();
        }

        assert_eq!(
            app.world().resource::<PickupDings>().0,
            1,
            "one crate, one player pickup ding"
        );
    }

    #[test]
    fn an_unauthored_crate_picks_up_silently() {
        // AUTHORED-OR-SILENT (task 20260717-101659, the last world sound to
        // move onto content): a crate whose config omits pickup_sound plays
        // nothing. `a_player_flying_into_a_crate_dings_once` is the delivery
        // guard - same rig, authored crate, dings.
        let mut app = pickup_audio_app();
        app.world_mut().spawn((
            salvage_crate_scenario_object(SalvageCrateConfig {
                size: 1.5,
                area_radius: 6.0,
                pickup_sound: None,
            }),
            CollisionEventsEnabled,
            Transform::from_translation(Vec3::ZERO),
        ));
        spawn_mover(&mut app, PlayerSpaceshipMarker);
        for _ in 0..25 {
            app.update();
        }
        assert_eq!(
            app.world().resource::<PickupDings>().0,
            0,
            "an unauthored crate must pick up silently"
        );
    }

    /// The dedup: a real ship is one RigidBody with many section colliders, so
    /// avian fires a `CollisionStart` per section entering the sensor (this rig
    /// reproduces the empirical 3x). The pickup must still be exactly ONE ding.
    /// The `== 1` assertion is self-guarding: without the collision it would be
    /// 0, and without the dedup it would be 3 (both fail the test).
    #[test]
    fn a_multi_collider_player_dings_once_per_crate() {
        let mut app = pickup_audio_app();
        spawn_pickup_crate(&mut app);
        // One player body, three section colliders strung along the flight axis
        // (like controller/hull/thruster on the shakedown ship).
        let root = app
            .world_mut()
            .spawn((
                RigidBody::Dynamic,
                PlayerSpaceshipMarker,
                Transform::from_translation(Vec3::new(0.0, 0.0, 12.0)),
                LinearVelocity(Vec3::new(0.0, 0.0, -60.0)),
            ))
            .id();
        for dz in [0.0f32, 1.0, 2.0] {
            app.world_mut().spawn((
                Collider::cuboid(1.0, 1.0, 1.0),
                ColliderDensity(1.0),
                Transform::from_translation(Vec3::new(0.0, 0.0, dz)),
                ChildOf(root),
            ));
        }

        for _ in 0..25 {
            app.update();
        }

        assert_eq!(
            app.world().resource::<PickupDings>().0,
            1,
            "three section colliders entering one sensor must be one pickup ding"
        );
    }

    /// The gate: an AI (non-player) body sweeping the same crate collects
    /// nothing - the scenario handler ignores it - so it must not ding. The
    /// `CrateCollisions` guard proves the stimulus fired: the mover really
    /// entered the sensor, yet the player gate kept it silent (a bare
    /// zero-assertion could pass on a rig the collision never reached).
    #[test]
    fn a_non_player_body_through_a_crate_stays_silent() {
        let mut app = pickup_audio_app();
        app.init_resource::<CrateCollisions>();
        app.add_observer(
            |collision: On<CollisionStart>,
             q_crate: Query<(), With<SalvageCrateMarker>>,
             mut hits: ResMut<CrateCollisions>| {
                let touched = collision.body1.is_some_and(|e| q_crate.contains(e))
                    || collision.body2.is_some_and(|e| q_crate.contains(e));
                if touched {
                    hits.0 += 1;
                }
            },
        );
        spawn_pickup_crate(&mut app);
        // No PlayerSpaceshipMarker: an AI stand-in.
        spawn_mover(&mut app, ());

        for _ in 0..25 {
            app.update();
        }

        assert!(
            app.world().resource::<CrateCollisions>().0 > 0,
            "delivery guard: the non-player mover must actually enter the sensor"
        );
        assert_eq!(
            app.world().resource::<PickupDings>().0,
            0,
            "a non-player body collects nothing, so it must not ding"
        );
    }

    /// The glow pulse actually moves the emissive: after a nonzero step
    /// off the wave's crest the luminance has left its spawn value, and it
    /// stays inside the authored band. Real observer + system, real
    /// material asset, deterministic manual clock (review R1.6).
    #[test]
    fn crate_glow_pulses_inside_its_band() {
        use core::time::Duration;

        use bevy::time::TimeUpdateStrategy;

        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            TransformPlugin,
            AssetPlugin::default(),
            bevy::mesh::MeshPlugin,
        ));
        // An eighth of the pulse period per frame - well under
        // Time<Virtual>'s 0.25s max-delta clamp.
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            ITEM_HIGHLIGHT_PULSE_PERIOD_SECS / 8.0,
        )));
        app.init_asset::<StandardMaterial>();
        app.add_observer(insert_crate_render);
        app.add_systems(Update, pulse_crate_glow);

        let root = app
            .world_mut()
            .spawn(salvage_crate_scenario_object(SalvageCrateConfig {
                size: 1.5,
                area_radius: 6.0,
                pickup_sound: None,
            }))
            .id();

        app.update();
        let emissive_of = |app: &App| {
            let children = app.world().get::<Children>(root).expect("render child");
            let child = children.iter().next().expect("one child");
            let glow = app.world().get::<CrateGlow>(child).unwrap();
            app.world()
                .resource::<Assets<StandardMaterial>>()
                .get(&glow.material)
                .unwrap()
                .emissive
        };
        let spawn_emissive = emissive_of(&app);

        // An eighth period into the wave the luminance must have moved.
        app.update();
        let later = emissive_of(&app);
        assert_ne!(
            spawn_emissive, later,
            "the pulse moves the emissive off its spawn value"
        );

        // And the sweep stays inside the authored band (checked on the
        // luminance factor recovered against base color red = 1.0).
        let factor = later.red;
        assert!(
            (CRATE_EMISSIVE_MIN..=CRATE_EMISSIVE_MAX).contains(&factor),
            "emissive factor {factor} escaped [{CRATE_EMISSIVE_MIN}, {CRATE_EMISSIVE_MAX}]"
        );
    }

    /// The tumble animates the RENDER CHILD, not the trigger root: the
    /// sensor sphere must not inherit a spin (and the root must not need a
    /// mass). Runs the real observer + system on a rendered crate.
    #[test]
    fn tumble_rotates_the_render_child_only() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            TransformPlugin,
            AssetPlugin::default(),
            bevy::mesh::MeshPlugin,
        ));
        app.init_asset::<StandardMaterial>();
        app.add_observer(insert_crate_render);
        app.add_systems(Update, tumble_crates);

        let root = app
            .world_mut()
            .spawn(salvage_crate_scenario_object(SalvageCrateConfig {
                size: 1.5,
                area_radius: 6.0,
                pickup_sound: None,
            }))
            .id();

        // Two updates: one to flush the observer's child spawn, one to tick
        // the tumble with a nonzero delta.
        app.update();
        std::thread::sleep(std::time::Duration::from_millis(5));
        app.update();

        let children = app.world().get::<Children>(root).expect("render child");
        let child = children.iter().next().expect("one child");
        let child_rotation = app.world().get::<Transform>(child).unwrap().rotation;
        assert!(
            child_rotation.angle_between(Quat::IDENTITY) > 0.0,
            "the render child tumbles"
        );
        let root_rotation = app.world().get::<Transform>(root);
        assert!(
            root_rotation.is_none() || root_rotation.unwrap().rotation == Quat::IDENTITY,
            "the trigger root does not rotate"
        );
    }
}
