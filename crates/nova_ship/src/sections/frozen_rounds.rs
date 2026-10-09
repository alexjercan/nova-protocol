//! Freeze/thaw of a flying round, pairing nova_gameplay's pure flight record
//! with the turret or railgun art a live round wore.
//!
//! [`nova_gameplay::rounds::freeze_round_flight`] and `thaw_round_flight`
//! know nothing about turrets, railguns, or render meshes - they are the
//! gun-agnostic flight (pose, velocity, damage, owner, rake). This module is
//! the one gun-aware layer above them: it tells a turret bullet from a
//! railgun slug, carries the bullet's render mesh so a thaw looks the same
//! as what was saved, and re-adds the marker the gameplay pair never touches.

use bevy::prelude::*;
use nova_gameplay::prelude::{
    freeze_round_flight, thaw_round_flight, AssetRef, FrozenRoundFlight,
    RailgunSlugProjectileMarker, RoundRake, TransientFreezeFault, TurretBulletProjectileMarker,
};

use super::turret_section::BulletProjectileRenderMesh;

/// `FrozenRound`, `RoundSourceType`, `freeze_round` and `thaw_round`.
pub mod prelude {
    pub use super::{freeze_round, thaw_round, FrozenRound, RoundSourceType};
}

/// A flying round as a save keeps it: the gun-agnostic flight record plus
/// which gun fired it.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FrozenRound {
    /// The round's pose, velocity, damage, owner and rake.
    pub flight: FrozenRoundFlight,
    /// Which gun fired it, and the art a thaw must re-add beside it.
    pub source: RoundSourceType,
}

/// Which gun fired a round, and the one piece of its render state that does
/// not live on [`FrozenRoundFlight`].
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RoundSourceType {
    /// A turret bullet. Carries the `BulletProjectileRenderMesh` the live
    /// round wore: `insert_projectile_render` requires one beside the
    /// marker, and without it a thaw would log that observer's `error!`.
    Turret {
        /// The authored projectile mesh, or `None` for the default art by
        /// damage type.
        render_mesh: Option<AssetRef<WorldAsset>>,
    },
    /// A railgun slug. No per-round render state to carry.
    Railgun,
}

/// The round `entity` is riding, as a value a save can keep.
///
/// # Errors
///
/// Whatever [`freeze_round_flight`] returns.
///
/// # Panics
///
/// If `entity` carries neither [`TurretBulletProjectileMarker`] nor
/// [`RailgunSlugProjectileMarker`] - every `GunRoundMarker` entity nova spawns
/// is one or the other, so this is a bug in the caller, not a save-time
/// condition. If a turret round's render mesh is a code-built
/// `AssetRef::Handle` with no authorable path: that value has no RON form
/// (`AssetRef`'s `Serialize` impl errors on it), so this fails loudly, naming
/// the round, instead of failing inside a later RON write with no entity to
/// point at.
pub fn freeze_round(world: &World, entity: Entity) -> Result<FrozenRound, TransientFreezeFault> {
    let flight = freeze_round_flight(world, entity)?;
    let source = if world.get::<TurretBulletProjectileMarker>(entity).is_some() {
        let render_mesh = world
            .get::<BulletProjectileRenderMesh>(entity)
            .unwrap_or_else(|| {
                panic!("freeze_round: turret round {entity} carries no BulletProjectileRenderMesh")
            })
            .0
            .clone();
        if render_mesh
            .as_ref()
            .is_some_and(|mesh| mesh.path().is_none())
        {
            panic!(
                "freeze_round: turret round {entity} carries a code-built render mesh handle \
                 with no asset path, which cannot be saved"
            );
        }
        RoundSourceType::Turret { render_mesh }
    } else if world.get::<RailgunSlugProjectileMarker>(entity).is_some() {
        RoundSourceType::Railgun
    } else {
        panic!("freeze_round: round {entity} is neither a turret bullet nor a railgun slug");
    };
    Ok(FrozenRound { flight, source })
}

/// Spawns the thawed round and returns it.
///
/// Carries `Allegiance` and a fresh [`RoundRake`] when the flight had them
/// (see [`thaw_round_flight`]'s doc for why those are inserted here rather
/// than in its own bundle), and the turret or railgun marker and art -
/// without `TurretSectionPartOf`, `TurretSectionMuzzleEntity` or a wake. Those
/// feed only the launch cues and the wake effect, and a resumed round was not
/// launched.
pub fn thaw_round(commands: &mut Commands, round: &FrozenRound, owner: Entity) -> Entity {
    let mut entity = commands.spawn(thaw_round_flight(&round.flight, owner));
    if let Some(allegiance) = round.flight.allegiance {
        entity.insert(allegiance);
    }
    if let Some(radius) = round.flight.rake_radius {
        entity.insert(RoundRake::new(radius));
    }
    match &round.source {
        RoundSourceType::Turret { render_mesh } => {
            entity.insert((
                Name::new("Turret Projectile"),
                TurretBulletProjectileMarker,
                BulletProjectileRenderMesh(render_mesh.clone()),
            ));
        }
        RoundSourceType::Railgun => {
            entity.insert((Name::new("Railgun Slug"), RailgunSlugProjectileMarker));
        }
    }
    entity.id()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use avian3d::prelude::*;
    use bevy::{
        asset::AssetPlugin,
        log::tracing_subscriber::{self, util::SubscriberInitExt},
        time::TimeUpdateStrategy,
    };
    use bevy_hanabi::prelude::*;
    use nova_gameplay::{prelude::*, test_log::CapturedLog};

    use super::*;
    use crate::{
        prelude::*,
        sections::{
            torpedo_section::TorpedoSectionSpawnerEntity,
            turret_section::{turret_section, TurretSectionPlugin},
        },
    };

    /// Count of `PlaySfx` triggers observed, local to this test - the
    /// ship_audio crate's own `PlayedSfx`/`LastPlayed` are `pub(super)` to
    /// `ship_audio` and unreachable from here.
    #[derive(Resource, Default)]
    struct PlaySfxCount(u32);

    /// A ship + turret rig driven by the REAL production chain: the full
    /// `TurretSectionPlugin` (render on, so the real `on_projectile_marker_effect`
    /// is registered) and `ShipAudioPlugin` (so the real `on_turret_fire_play_sfx`
    /// is registered). `shoot_spawn_projectile`, both cue observers and
    /// `insert_projectile_render` are `pub(super)` to their own modules and
    /// unreachable from `sections::frozen_rounds` (a sibling module), so the
    /// only way to exercise the REAL functions is through the public plugins
    /// that register them - not a hand-rolled trigger.
    fn real_fire_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            TransformPlugin,
            SmoothLookRotationPlugin,
        ));
        // Asset types `insert_turret_barrel_muzzle_effect`, `insert_turret_joint_render`
        // and `insert_projectile_render` read/write, normally initialised by
        // `HanabiPlugin`/`AssetPlugin`'s own asset-type registration. No
        // `HanabiPlugin`: it needs a render world this test does not build.
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
        app.init_asset::<WorldAsset>();
        app.init_asset::<EffectAsset>();
        app.init_asset::<Image>();
        app.init_asset::<AudioSource>();

        app.add_plugins(TurretSectionPlugin { render: true });
        // `ShipAudioPlugin` also wires `play_lock_cues` (an unrelated cockpit
        // cue in the same `Update` set as the fire cue), which reads four
        // messages normally registered by `nova_ship::input::targeting`'s own
        // plugin - not added here, so they are registered directly.
        app.add_message::<RadarLockAcquired>();
        app.add_message::<RadarRetargeted>();
        app.add_message::<LockClearedToast>();
        app.add_message::<RadarDenied>();
        app.add_plugins(ShipAudioPlugin);

        app.init_resource::<PlaySfxCount>();
        app.add_observer(|_: On<PlaySfx>, mut count: ResMut<PlaySfxCount>| count.0 += 1);
        app
    }

    /// A ship at rest with one turret (one round of ammo, an authored fire
    /// sound, no aim point - fail-open, fires freely per
    /// `shoot_spawn_projectile`'s own "a turret with no aim point ... fires
    /// freely" rule) as a child. No `SectionAnimations`, so
    /// `insert_turret_stow` never matches this turret and it is always read
    /// as deployed (`stow.is_none_or(TurretStow::is_deployed)` with `stow:
    /// None`) - the same fixture shape `firing.rs`'s own
    /// `spawn_firing_turret` test rig uses.
    fn spawn_ship_with_turret(app: &mut App) -> (Entity, Entity) {
        let fire_sound = AssetRef::from("base/sounds/turret_fire.wav");
        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                Transform::IDENTITY,
                Position(Vec3::ZERO),
                Rotation::default(),
                LinearVelocity(Vec3::ZERO),
                AngularVelocity(Vec3::ZERO),
                ComputedCenterOfMass(Vec3::ZERO),
            ))
            .id();
        let turret = app
            .world_mut()
            .spawn((
                turret_section(TurretSectionConfig {
                    fire_sound: Some(fire_sound),
                    ..default()
                }),
                Transform::IDENTITY,
                ChildOf(ship),
            ))
            .id();
        app.world_mut().flush();
        app.world_mut()
            .entity_mut(turret)
            .insert((TurretSectionInput(true), SectionAmmo::new(1)));
        app.world_mut().flush();
        (ship, turret)
    }

    /// The turret's muzzle-flash child, as the real `insert_turret_barrel_muzzle_effect`
    /// observer built it (a `ParticleEffect` + `EffectProperties` sibling of
    /// the projectile mesh, parented to the muzzle). Seeds it with the
    /// `EffectSpawner` that `bevy_hanabi::tick_spawners` would otherwise add
    /// once the GPU pipeline compiled the effect asset - a render-world step
    /// this headless test does not run - built from that SAME asset's own
    /// `SpawnerSettings`, so the "once, no emit-on-start" shape
    /// `on_projectile_marker_effect` resets is the real one, not a stand-in.
    fn seed_muzzle_effect_spawner(app: &mut App, muzzle: Entity) -> Entity {
        let effect_entity = {
            let mut query = app
                .world_mut()
                .query::<(Entity, &ChildOf, &ParticleEffect)>();
            query
                .iter(app.world())
                .find(|(_, child_of, _)| child_of.0 == muzzle)
                .map(|(entity, _, effect)| (entity, effect.handle.clone()))
        };
        let Some((effect_entity, handle)) = effect_entity else {
            panic!("seed_muzzle_effect_spawner: muzzle {muzzle:?} has no ParticleEffect child");
        };
        let settings = app
            .world()
            .resource::<Assets<EffectAsset>>()
            .get(&handle)
            .expect("insert_turret_barrel_muzzle_effect built the asset synchronously")
            .spawner;
        app.world_mut()
            .entity_mut(effect_entity)
            .insert(EffectSpawner::new(&settings));
        effect_entity
    }

    /// Fires a one-round magazine through several fixed ticks and returns the
    /// muzzle-flash child entity. The ammo cap, not the tick count, is what
    /// makes exactly one round leave - `shoot_spawn_projectile`'s own
    /// `a_turret_with_ammo_fires_exactly_its_magazine_then_stops` proves the
    /// same cap.
    fn fire_one_real_round(app: &mut App, ship: Entity, turret: Entity) -> Entity {
        let muzzle = **app
            .world()
            .get::<TurretSectionMuzzleEntity>(turret)
            .expect("insert_turret_section built the muzzle");
        let effect_entity = seed_muzzle_effect_spawner(app, muzzle);

        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            1.0 / 30.0,
        )));
        for _ in 0..8 {
            app.update();
        }

        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<TurretBulletProjectileMarker>>()
                .iter(app.world())
                .count(),
            1,
            "a one-round magazine must fire exactly one round at rest {ship:?}/{turret:?}"
        );
        effect_entity
    }

    /// The real fire path (one shot) gives exactly one muzzle-flash cue and
    /// one fire sound. A `thaw_round` of a saved round gives neither and logs
    /// no error: the cue observers listen for `RoundFired`, which only a fire
    /// triggers, and the render observer finds the `BulletProjectileRenderMesh`
    /// the thaw carries.
    #[test]
    fn a_resumed_projectile_plays_no_launch_cue() {
        let mut app = real_fire_app();
        let (ship, turret) = spawn_ship_with_turret(&mut app);

        let effect_entity = fire_one_real_round(&mut app, ship, turret);
        assert!(
            !app.world()
                .get::<EffectSpawner>(effect_entity)
                .expect("seeded above")
                .has_completed(),
            "a real fire must reset the muzzle spawner (SpawnerSettings::once \
             reads as completed until on_projectile_marker_effect calls reset())"
        );
        assert_eq!(
            app.world().resource::<PlaySfxCount>().0,
            1,
            "a real fire with an authored fire_sound must play exactly one sound"
        );

        // Mark the flash "spent" again - a fresh `EffectSpawner` built from the
        // SAME once/no-emit-on-start settings reads as completed, exactly as it
        // did right after `insert_turret_barrel_muzzle_effect` built it and
        // before the real fire above reset it - so a thaw that wrongly
        // re-fired the cue would flip `has_completed()` to `false` below
        // exactly as the real fire did above.
        let settings = app
            .world()
            .get::<EffectSpawner>(effect_entity)
            .unwrap()
            .settings;
        app.world_mut()
            .entity_mut(effect_entity)
            .insert(EffectSpawner::new(&settings));
        assert!(
            app.world()
                .get::<EffectSpawner>(effect_entity)
                .unwrap()
                .has_completed(),
            "the spawner must read spent again before the thaw, or a false pass \
             would prove nothing"
        );

        let frozen_round = FrozenRound {
            flight: FrozenRoundFlight {
                translation: Vec3::new(0.0, 0.0, -10.0),
                rotation: Quat::IDENTITY,
                velocity: Vec3::new(0.0, 0.0, -100.0),
                damage: ProjectileDamage::new(4.0, DamageType::Kinetic),
                allegiance: None,
                owner: SavedOwner::Gone,
                rake_radius: None,
            },
            source: RoundSourceType::Turret { render_mesh: None },
        };
        // The thaw's observers run in this flush, on this thread, so the
        // thread's own log sink sees every line they write.
        let log = CapturedLog::default();
        let writer = log.clone();
        let guard = tracing_subscriber::fmt()
            .with_writer(move || writer.clone())
            .set_default();
        let thawed = {
            let mut commands = app.world_mut().commands();
            thaw_round(&mut commands, &frozen_round, ship)
        };
        app.world_mut().flush();
        drop(guard);
        assert!(
            !log.contents().contains("ERROR"),
            "a thaw logs no error:\n{}",
            log.contents()
        );

        assert!(
            app.world()
                .get::<EffectSpawner>(effect_entity)
                .unwrap()
                .has_completed(),
            "thaw_round must not reset the muzzle flash: it triggers no \
             RoundFired, so on_projectile_marker_effect never runs for it"
        );
        assert_eq!(
            app.world().resource::<PlaySfxCount>().0,
            1,
            "thaw_round must not play a fire sound: on_turret_fire_play_sfx \
             never runs for it either"
        );

        // `insert_projectile_render` still runs on `Add` of the marker, so the
        // thawed round wears the same art as a fired one.
        let render_child = app
            .world()
            .get::<Children>(thawed)
            .and_then(|children| children.first().copied());
        assert!(
            render_child.is_some(),
            "insert_projectile_render must have run its happy path (not its \
             error branch) on the thawed round, which carries \
             BulletProjectileRenderMesh"
        );

        // The torpedo half of the same contrast, added to this SAME app: a
        // real launch gives exactly one more launch effect + one more launch
        // sound (on top of the turret's own one above), and `thaw_torpedo`
        // gives neither and logs no error either - `TorpedoLaunched` is the
        // one event both cue observers answer, and only
        // `shoot_spawn_projectile` ever triggers it.
        app.add_plugins(TorpedoSectionPlugin { render: true });

        let torpedo_ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                Transform::IDENTITY,
                Position(Vec3::ZERO),
                Rotation::default(),
                LinearVelocity(Vec3::ZERO),
                AngularVelocity(Vec3::ZERO),
                ComputedCenterOfMass(Vec3::ZERO),
            ))
            .id();
        let torpedo_bay = app
            .world_mut()
            .spawn((
                torpedo_section(TorpedoSectionConfig {
                    fire_rate: 100.0,
                    ammunition: AmmoCapacity::Limited(1),
                    launch_sound: Some(AssetRef::from("base/sounds/torpedo_launch.wav")),
                    ..default()
                }),
                Transform::IDENTITY,
                ChildOf(torpedo_ship),
            ))
            .id();
        app.world_mut().flush();
        app.world_mut()
            .entity_mut(torpedo_bay)
            .insert(TorpedoSectionInput(true));

        let spawner = **app
            .world()
            .get::<TorpedoSectionSpawnerEntity>(torpedo_bay)
            .expect("insert_torpedo_section built the spawner");
        let torpedo_effect_entity = seed_muzzle_effect_spawner(&mut app, spawner);

        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            1.0 / 60.0,
        )));
        for _ in 0..4 {
            app.update();
        }

        let torpedo = app
            .world_mut()
            .query_filtered::<Entity, With<TorpedoProjectileMarker>>()
            .iter(app.world())
            .next()
            .expect("the one-round bay launched its torpedo");
        assert!(
            !app.world()
                .get::<EffectSpawner>(torpedo_effect_entity)
                .expect("seeded above")
                .has_completed(),
            "a real launch must reset the spawner's launch-burst effect"
        );
        assert_eq!(
            app.world().resource::<PlaySfxCount>().0,
            2,
            "a real launch with an authored launch_sound must play exactly \
             one more sound, on top of the turret's own one above"
        );

        // Mark the launch-burst effect "spent" again, same reasoning as the
        // muzzle flash above: a thaw that wrongly re-fired the cue would flip
        // this back to `false`.
        let settings = app
            .world()
            .get::<EffectSpawner>(torpedo_effect_entity)
            .unwrap()
            .settings;
        app.world_mut()
            .entity_mut(torpedo_effect_entity)
            .insert(EffectSpawner::new(&settings));
        assert!(
            app.world()
                .get::<EffectSpawner>(torpedo_effect_entity)
                .unwrap()
                .has_completed(),
            "the spawner must read spent again before the thaw, or a false \
             pass would prove nothing"
        );
        app.world_mut().entity_mut(torpedo).despawn();

        let frozen_torpedo = FrozenTorpedo {
            config: TorpedoSectionConfig::default(),
            owner: SavedOwner::Gone,
            section: None,
            translation: Vec3::new(0.0, 0.0, -10.0),
            rotation: Quat::IDENTITY,
            linear: Vec3::new(0.0, 0.0, -50.0),
            angular: Vec3::ZERO,
            allegiance: None,
            arming: TorpedoArming::new(0.5, 50.0, Vec3::ZERO, 0.0),
            cold: None,
            target: SavedTorpedoTarget::DumbFire,
            steering: Vec3::NEG_Z,
            weave: TorpedoWeave::new(0.0, 0.0, Vec3::NEG_Z, 0.0),
            ignited: true,
            controller_health: 10.0,
            thruster_health: 10.0,
        };
        let log = CapturedLog::default();
        let writer = log.clone();
        let guard = tracing_subscriber::fmt()
            .with_writer(move || writer.clone())
            .set_default();
        let thawed_torpedo = {
            let mut commands = app.world_mut().commands();
            thaw_torpedo(&mut commands, &frozen_torpedo, torpedo_ship, None)
        };
        app.world_mut().flush();
        drop(guard);
        assert!(
            !log.contents().contains("ERROR"),
            "a torpedo thaw logs no error:\n{}",
            log.contents()
        );
        assert!(
            app.world().entities().contains(thawed_torpedo),
            "thaw_torpedo must still spawn the projectile"
        );

        assert!(
            app.world()
                .get::<EffectSpawner>(torpedo_effect_entity)
                .unwrap()
                .has_completed(),
            "thaw_torpedo must not reset the launch-burst effect: it \
             triggers no TorpedoLaunched, so on_torpedo_launch_effect never \
             runs for it"
        );
        assert_eq!(
            app.world().resource::<PlaySfxCount>().0,
            2,
            "thaw_torpedo must not play a launch sound: \
             on_torpedo_launch_play_sfx never runs for it either"
        );
    }
}
