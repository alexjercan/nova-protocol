//! Combat one-shots: explosions, impacts, turret fire and torpedo
//! launches, each resolving the target's authored sound or staying
//! silent.

use bevy::prelude::*;
use nova_gameplay::{
    audio::{
        area_cell, listener_position, play_positional_handle, SfxListenerMarker, SfxThrottle,
        ThrottleKey,
    },
    prelude::*,
};

use super::{
    EXPLOSION_MIN_INTERVAL, EXPLOSION_VOLUME, IMPACT_MIN_INTERVAL, IMPACT_VOLUME,
    TORPEDO_LAUNCH_VOLUME, TURRET_FIRE_MIN_INTERVAL, TURRET_FIRE_VOLUME,
};
use crate::{
    prelude::*,
    sections::{
        torpedo_section::{TorpedoSectionLaunchSound, TorpedoSectionSpawnerEntity},
        turret_section::{TurretSectionFireSound, TurretSectionPartOf},
    },
};

/// Find the nearest [`ImpactDestroySounds`] on `entity` or an ancestor. The
/// damage/destroy observers' target is the entity carrying Health - for
/// sections that IS the section entity, but an asteroid keeps its Health on a
/// child node while the sounds snapshot sits on the rock's parent bundle, so
/// the lookup walks up (bounded by the hierarchy, like `hum_source_root`).
fn impact_destroy_sounds<'a>(
    entity: Entity,
    q_sounds: &'a Query<&ImpactDestroySounds>,
    q_child_of: &Query<&ChildOf>,
) -> Option<&'a ImpactDestroySounds> {
    let mut current = entity;
    loop {
        if let Ok(sounds) = q_sounds.get(current) {
            return Some(sounds);
        }
        match q_child_of.get(current) {
            Ok(&ChildOf(parent)) => current = parent,
            Err(_) => return None,
        }
    }
}

/// Explosion cue on any destruction (section, asteroid, or torpedo detonation,
/// which all funnel through `IntegrityDestroyMarker`).
pub(super) fn on_destroyed_play_explosion(
    add: On<Add, IntegrityDestroyMarker>,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    q_transform: Query<&GlobalTransform>,
    q_sounds: Query<&ImpactDestroySounds>,
    q_child_of: Query<&ChildOf>,
    q_camera: Query<&GlobalTransform, With<SfxListenerMarker>>,
    mut throttle_state: ResMut<SfxThrottle>,
    mut commands: Commands,
) {
    // The destroyed entity has existed for frames, so its GlobalTransform is
    // valid world-space.
    let Ok(source) = q_transform.get(add.entity) else {
        return;
    };
    // AUTHORED-OR-SILENT: the destruction voice is the TARGET's authored
    // destroy_sound (per-target = per-material), found on the entity or an
    // ancestor (asteroid node shape) and resolved here.
    let Some(handle) = impact_destroy_sounds(add.entity, &q_sounds, &q_child_of)
        .and_then(|s| s.destroy.as_ref())
        .map(|r| r.resolve(&asset_server))
    else {
        return;
    };
    let pos = source.translation();
    if throttle_state.allow(
        ThrottleKey::Explosion(area_cell(pos)),
        time.elapsed_secs(),
        EXPLOSION_MIN_INTERVAL,
    ) {
        play_positional_handle(
            &mut commands,
            handle,
            EXPLOSION_VOLUME,
            pos,
            listener_position(&q_camera),
        );
    }
}

/// Impact cue whenever damage is applied. Throttled because a single blast
/// deals damage to many colliders in one frame.
///
/// Propagation caveat: `HealthApplyDamage` auto-propagates up `ChildOf`
/// (section -> ship root), and ship death depends on that bubbling, so it must
/// not be stopped here - but a global observer fires once per hop, which would
/// double the cue whenever the section and root land in different area cells.
/// Reacting only to the original target keeps one hit = one cue, and the
/// original target is also the better cue position: the actual hit location,
/// not the ship root's origin. Any future damage-cue observer needs this same
/// guard.
pub(super) fn on_damage_play_impact(
    damage: On<HealthApplyDamage>,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    q_transform: Query<&GlobalTransform>,
    q_sounds: Query<&ImpactDestroySounds>,
    q_child_of: Query<&ChildOf>,
    q_camera: Query<&GlobalTransform, With<SfxListenerMarker>>,
    mut throttle_state: ResMut<SfxThrottle>,
    mut commands: Commands,
) {
    if damage.entity != damage.original_event_target() {
        return;
    }
    let Ok(source) = q_transform.get(damage.entity) else {
        return;
    };
    // AUTHORED-OR-SILENT: the hit voice is the TARGET's authored impact_sound
    // (per-target = per-material), found on the entity or an ancestor.
    let Some(handle) = impact_destroy_sounds(damage.entity, &q_sounds, &q_child_of)
        .and_then(|s| s.impact.as_ref())
        .map(|r| r.resolve(&asset_server))
    else {
        return;
    };
    let pos = source.translation();
    if throttle_state.allow(
        ThrottleKey::Impact(area_cell(pos)),
        time.elapsed_secs(),
        IMPACT_MIN_INTERVAL,
    ) {
        play_positional_handle(
            &mut commands,
            handle,
            IMPACT_VOLUME,
            pos,
            listener_position(&q_camera),
        );
    }
}

/// Turret-fire cue when a round spawns. Throttled hard because the PDC fires at
/// a high rate.
///
/// AUTHORED-OR-SILENT: the sound is the firing turret's
/// [`TurretSectionConfig::fire_sound`], snapshotted at spawn as
/// [`TurretSectionFireSound`] and resolved here - content owns the sound, and a
/// turret that authors none fires silently (every base turret authors it via
/// gen_content, so the shipped game is unchanged; the old global bank fallback
/// is gone with its `WorldSfx::TurretFire` key). Everything else (per-turret
/// throttle key, distance attenuation, positioning) is unchanged.
pub(super) fn on_turret_fire_play_sfx(
    add: On<Add, TurretBulletProjectileMarker>,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    q_projectile: Query<(&Transform, &TurretSectionPartOf)>,
    q_fire_sound: Query<&TurretSectionFireSound>,
    q_camera: Query<&GlobalTransform, With<SfxListenerMarker>>,
    mut throttle_state: ResMut<SfxThrottle>,
    mut commands: Commands,
) {
    // The projectile is a freshly-spawned ROOT entity, so its GlobalTransform
    // is still identity this frame; its local Transform is already world-space.
    // `TurretSectionPartOf` names the firing turret, so each gun throttles on
    // its own key - the fix for "only one of several guns is audible".
    let Ok((transform, part_of)) = q_projectile.get(add.entity) else {
        return;
    };
    // No authored sound -> silent (still stamp the throttle key? No: an
    // unauthored turret plays nothing, so there is nothing to rate-limit).
    let Some(handle) = q_fire_sound
        .get(part_of.0)
        .ok()
        .and_then(|s| s.0.as_ref())
        .map(|r| r.resolve(&asset_server))
    else {
        return;
    };
    if throttle_state.allow(
        ThrottleKey::TurretFire(part_of.0),
        time.elapsed_secs(),
        TURRET_FIRE_MIN_INTERVAL,
    ) {
        play_positional_handle(
            &mut commands,
            handle,
            TURRET_FIRE_VOLUME,
            transform.translation,
            listener_position(&q_camera),
        );
    }
}

/// Launch cue when a torpedo projectile spawns.
///
/// AUTHORED-OR-SILENT: the sound is the firing bay's
/// [`TorpedoSectionConfig::launch_sound`], snapshotted onto the bay's spawner
/// as [`TorpedoSectionLaunchSound`] and reached from the projectile via its
/// [`TorpedoSectionSpawnerEntity`] back-ref (the same path the launch flash
/// effect takes). A bay that authors none launches silently; base bays author
/// it via gen_content, so the shipped game is unchanged.
pub(super) fn on_torpedo_launch_play_sfx(
    add: On<Add, TorpedoProjectileMarker>,
    asset_server: Res<AssetServer>,
    q_projectile: Query<(&Transform, &TorpedoSectionSpawnerEntity)>,
    q_launch_sound: Query<&TorpedoSectionLaunchSound>,
    q_camera: Query<&GlobalTransform, With<SfxListenerMarker>>,
    mut commands: Commands,
) {
    // Freshly-spawned root entity: use local Transform (== world) this frame.
    let Ok((source, spawner)) = q_projectile.get(add.entity) else {
        return;
    };
    let Some(handle) = q_launch_sound
        .get(spawner.0)
        .ok()
        .and_then(|s| s.0.as_ref())
        .map(|r| r.resolve(&asset_server))
    else {
        return;
    };
    play_positional_handle(
        &mut commands,
        handle,
        TORPEDO_LAUNCH_VOLUME,
        source.translation,
        listener_position(&q_camera),
    );
}

#[cfg(test)]
mod tests {
    use nova_gameplay::audio::SFX_AREA_CELL;

    use super::*;
    use crate::ship_audio::test_support::{LastPlayed, PlayedSfx};
    #[test]
    fn a_propagated_hit_on_a_straddling_hierarchy_plays_one_impact() {
        // `HealthApplyDamage` auto-propagates child -> parent, and with the
        // parent one area cell away the per-cell throttle cannot collapse the
        // hops, so one hit played two impact sounds. The original-target guard
        // must keep it at exactly one.
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.init_resource::<SfxThrottle>();
        app.init_resource::<PlayedSfx>();
        app.add_observer(on_damage_play_impact);
        app.add_observer(|_: On<PlaySfx>, mut played: ResMut<PlayedSfx>| played.0 += 1);

        let parent = app
            .world_mut()
            .spawn(GlobalTransform::from(Transform::from_translation(
                Vec3::new(SFX_AREA_CELL * 4.0, 0.0, 0.0),
            )))
            .id();
        let child = app
            .world_mut()
            .spawn((
                GlobalTransform::default(),
                ChildOf(parent),
                // Authored-or-silent: the rig's target must author its impact
                // voice for the cue to fire at all.
                ImpactDestroySounds {
                    impact: Some(AssetRef::from("base/sounds/impact.wav")),
                    destroy: None,
                },
            ))
            .id();

        app.world_mut().trigger(HealthApplyDamage {
            entity: child,
            source: None,
            amount: 10.0,
        });
        // The observer plays via `Commands`, so the queued `PlaySfx` triggers
        // only fire on the next flush.
        app.world_mut().flush();

        assert_eq!(
            app.world().resource::<PlayedSfx>().0,
            1,
            "one hit must play exactly one impact sound"
        );
        // The cue is keyed (and positioned) at the hit location's cell, not the
        // parent's.
        let throttle = app.world().resource::<SfxThrottle>();
        assert!(throttle
            .tracked_keys()
            .eq([ThrottleKey::Impact(area_cell(Vec3::ZERO))]));
    }

    /// App rig for the turret-fire cue: the real `on_turret_fire_play_sfx`
    /// observer, capturing the played handle. No audio device needed (nothing
    /// constructs an `AudioSink`), and no bank: the cue is authored-or-silent,
    /// resolving the turret's own snapshot against the `AssetServer`.
    fn turret_fire_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.init_resource::<SfxThrottle>();
        app.init_resource::<LastPlayed>();
        app.add_observer(on_turret_fire_play_sfx);
        app.add_observer(|ev: On<PlaySfx>, mut last: ResMut<LastPlayed>| {
            last.0 = Some(ev.handle.clone());
        });
        app
    }

    /// Spawn a turret round parented (by `TurretSectionPartOf`) to `turret`,
    /// firing the `On<Add, TurretBulletProjectileMarker>` cue observer.
    fn fire_round(app: &mut App, turret: Entity) {
        app.world_mut().spawn((
            TurretBulletProjectileMarker,
            Transform::default(),
            TurretSectionPartOf(turret),
        ));
        app.world_mut().flush();
    }

    #[test]
    fn a_turret_with_a_declared_fire_sound_plays_that_handle() {
        // The section-authored audio path: a turret carrying a
        // `TurretSectionFireSound(Some(AssetRef))` must have the cue RESOLVE
        // that ref and play its handle - a mod turret sounds like its own gun.
        // Delivery guard for the silent test below.
        let mut app = turret_fire_app();
        let mod_sound: Handle<AudioSource> = app
            .world()
            .resource::<AssetServer>()
            .load("mods/x/sounds/railgun.wav");

        let turret = app
            .world_mut()
            .spawn(TurretSectionFireSound(Some(AssetRef::from(
                "mods/x/sounds/railgun.wav",
            ))))
            .id();
        fire_round(&mut app, turret);

        assert_eq!(
            app.world().resource::<LastPlayed>().0,
            Some(mod_sound),
            "a turret with a declared fire_sound must resolve + play its own handle"
        );
    }

    #[test]
    fn a_turret_without_a_declared_fire_sound_fires_silently() {
        // Authored-or-silent: no snapshot (or a `None` one) means NO sound -
        // the old global bank fallback is gone with the `WorldSfx::TurretFire`
        // key. The authored test above is the delivery guard proving this rig's
        // cue path plays when a sound exists, so this silence is the gate at
        // work, not a dead rig.
        let mut app = turret_fire_app();

        let bare = app.world_mut().spawn_empty().id();
        fire_round(&mut app, bare);
        assert_eq!(
            app.world().resource::<LastPlayed>().0,
            None,
            "no snapshot -> silent"
        );

        let unauthored = app.world_mut().spawn(TurretSectionFireSound(None)).id();
        fire_round(&mut app, unauthored);
        assert_eq!(
            app.world().resource::<LastPlayed>().0,
            None,
            "a None snapshot (config left fire_sound unset) -> silent"
        );
    }

    #[test]
    fn a_torpedo_bay_with_a_declared_launch_sound_plays_it_and_silent_without() {
        // Same authored-or-silent seam for the torpedo bay: the projectile
        // reaches the bay's spawner via its `TorpedoSectionSpawnerEntity`
        // back-ref; an authored `TorpedoSectionLaunchSound` plays, an
        // unauthored bay is silent. The authored half doubles as the delivery
        // guard.
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.init_resource::<LastPlayed>();
        app.add_observer(on_torpedo_launch_play_sfx);
        app.add_observer(|ev: On<PlaySfx>, mut last: ResMut<LastPlayed>| {
            last.0 = Some(ev.handle.clone());
        });
        let expected: Handle<AudioSource> = app
            .world()
            .resource::<AssetServer>()
            .load("base/sounds/torpedo_launch.wav");

        let authored = app
            .world_mut()
            .spawn(TorpedoSectionLaunchSound(Some(AssetRef::from(
                "base/sounds/torpedo_launch.wav",
            ))))
            .id();
        app.world_mut().spawn((
            TorpedoProjectileMarker,
            Transform::default(),
            TorpedoSectionSpawnerEntity(authored),
        ));
        app.world_mut().flush();
        assert_eq!(
            app.world().resource::<LastPlayed>().0,
            Some(expected),
            "an authored launch_sound must resolve + play"
        );

        app.world_mut().resource_mut::<LastPlayed>().0 = None;
        let silent = app.world_mut().spawn(TorpedoSectionLaunchSound(None)).id();
        app.world_mut().spawn((
            TorpedoProjectileMarker,
            Transform::default(),
            TorpedoSectionSpawnerEntity(silent),
        ));
        app.world_mut().flush();
        assert_eq!(
            app.world().resource::<LastPlayed>().0,
            None,
            "an unauthored bay launches silently"
        );
    }

    #[test]
    fn impact_and_destroy_play_the_targets_authored_sounds_or_stay_silent() {
        // Per-target voices: the hit/destroyed entity's own authored refs play;
        // an unauthored target is silent. The authored half is the delivery
        // guard for the silent half.
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.init_resource::<SfxThrottle>();
        app.init_resource::<LastPlayed>();
        app.add_observer(on_damage_play_impact);
        app.add_observer(on_destroyed_play_explosion);
        app.add_observer(|ev: On<PlaySfx>, mut last: ResMut<LastPlayed>| {
            last.0 = Some(ev.handle.clone());
        });
        let thud: Handle<AudioSource> = app
            .world()
            .resource::<AssetServer>()
            .load("mods/x/thud.wav");
        let boom: Handle<AudioSource> = app
            .world()
            .resource::<AssetServer>()
            .load("mods/x/boom.wav");

        // Authored target: impact plays ITS thud.
        let target = app
            .world_mut()
            .spawn((
                GlobalTransform::default(),
                ImpactDestroySounds {
                    impact: Some(AssetRef::from("mods/x/thud.wav")),
                    destroy: Some(AssetRef::from("mods/x/boom.wav")),
                },
            ))
            .id();
        app.world_mut().trigger(HealthApplyDamage {
            entity: target,
            source: None,
            amount: 1.0,
        });
        app.world_mut().flush();
        assert_eq!(app.world().resource::<LastPlayed>().0, Some(thud));

        // Destruction plays ITS boom (different cell so the throttle is clean).
        app.world_mut()
            .entity_mut(target)
            .insert(GlobalTransform::from(Transform::from_translation(
                Vec3::splat(SFX_AREA_CELL * 10.0),
            )));
        app.world_mut()
            .entity_mut(target)
            .insert(IntegrityDestroyMarker);
        app.world_mut().flush();
        assert_eq!(app.world().resource::<LastPlayed>().0, Some(boom));

        // Unauthored target: both cues silent.
        app.world_mut().resource_mut::<LastPlayed>().0 = None;
        let silent = app
            .world_mut()
            .spawn(GlobalTransform::from(Transform::from_translation(
                Vec3::splat(SFX_AREA_CELL * 20.0),
            )))
            .id();
        app.world_mut().trigger(HealthApplyDamage {
            entity: silent,
            source: None,
            amount: 1.0,
        });
        app.world_mut()
            .entity_mut(silent)
            .insert(IntegrityDestroyMarker);
        app.world_mut().flush();
        assert_eq!(
            app.world().resource::<LastPlayed>().0,
            None,
            "an unauthored target is silent for both cues"
        );
    }

    #[test]
    fn the_sound_lookup_walks_up_to_the_asteroid_parent() {
        // The asteroid shape: Health (and the destroy marker) live on a CHILD
        // node while ImpactDestroySounds sits on the rock's parent bundle - the
        // observers must find it by walking up.
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.init_resource::<SfxThrottle>();
        app.init_resource::<LastPlayed>();
        app.add_observer(on_destroyed_play_explosion);
        app.add_observer(|ev: On<PlaySfx>, mut last: ResMut<LastPlayed>| {
            last.0 = Some(ev.handle.clone());
        });
        let crack: Handle<AudioSource> = app
            .world()
            .resource::<AssetServer>()
            .load("base/sounds/explosion.wav");

        let rock = app
            .world_mut()
            .spawn(ImpactDestroySounds {
                impact: None,
                destroy: Some(AssetRef::from("base/sounds/explosion.wav")),
            })
            .id();
        let node = app
            .world_mut()
            .spawn((GlobalTransform::default(), ChildOf(rock)))
            .id();
        app.world_mut()
            .entity_mut(node)
            .insert(IntegrityDestroyMarker);
        app.world_mut().flush();
        assert_eq!(
            app.world().resource::<LastPlayed>().0,
            Some(crack),
            "the destroy cue must find the parent's authored sound via the walk"
        );
    }
}
