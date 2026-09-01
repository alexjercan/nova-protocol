//! Combat one-shots: explosions, impacts, turret fire, torpedo launches and
//! the railgun's report, each resolving the target's authored sound or staying
//! silent.
//!
//! Every cue here is ROUTED before it is played: `route_for` walks to the ship
//! the noise belongs to, and the player's own ship is
//! [`AudioRoute::Hull`] while everything else is [`AudioRoute::Exterior`]. The
//! rolloff and the pan follow from that, so no observer here looks up the
//! listener.

use bevy::prelude::*;
use nova_gameplay::{
    audio::{area_cell, SfxThrottle, ThrottleKey},
    prelude::*,
};

use super::{
    routing::{owning_root, route_for, route_from},
    DESTROY_SHIP_VOLUME, EXPLOSION_MIN_INTERVAL, EXPLOSION_VOLUME, IMPACT_MIN_INTERVAL,
    IMPACT_VOLUME, RAILGUN_FIRE_VOLUME, RAILGUN_RELOAD_VOLUME, TORPEDO_LAUNCH_MIN_INTERVAL,
    TORPEDO_LAUNCH_VOLUME, TURRET_FIRE_MIN_INTERVAL, TURRET_FIRE_VOLUME,
};
use crate::{
    prelude::*,
    sections::{
        railgun_section::{RailgunFired, RailgunSectionFireSound, RailgunSectionReloadSound},
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
    q_is_root: Query<(), With<SpaceshipRootMarker>>,
    q_is_player: Query<(), With<PlayerSpaceshipMarker>>,
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
        let route = route_for(add.entity, &q_child_of, &q_is_root, &q_is_player);
        commands.play_sfx_at(handle, route, EXPLOSION_VOLUME, pos);
    }
}

/// The lance's breech cycle, when its magazine comes back to capacity.
///
/// Placed at the GUN and routed by whose hull it is on, like the shot: a
/// raider's lance chambering across the arena is a thing worth hearing, and it
/// is exactly the tell that says the next one is coming.
///
/// AUTHORED-OR-SILENT, and the reason the report is section-generic while this
/// is not: every weapon with a magazine reports, and only the ones that author
/// a breech have one. A PDC's reload is a trickle with no moment in it.
pub(super) fn on_reload_complete_play_sfx(
    complete: On<SectionReloadComplete>,
    asset_server: Res<AssetServer>,
    q_sound: Query<&RailgunSectionReloadSound>,
    q_where: Query<&GlobalTransform>,
    q_child_of: Query<&ChildOf>,
    q_is_root: Query<(), With<SpaceshipRootMarker>>,
    q_is_player: Query<(), With<PlayerSpaceshipMarker>>,
    mut commands: Commands,
) {
    let gun = complete.entity;
    let Some(handle) = q_sound
        .get(gun)
        .ok()
        .and_then(|sound| sound.0.as_ref())
        .map(|r| r.resolve(&asset_server))
    else {
        return;
    };
    let Ok(at) = q_where.get(gun) else {
        return;
    };
    let route = route_for(gun, &q_child_of, &q_is_root, &q_is_player);
    commands.play_sfx_at(handle, route, RAILGUN_RELOAD_VOLUME, at.translation());
}

/// The hull-loss cue, on the frame a ship COLLAPSES.
///
/// The collapse edge, not the root's death: a ship stops being a ship the
/// moment it falls under its structural threshold, and what follows is several
/// frames of its sections peeling away one at a time
/// (`cascade_structural_collapse`). Hanging the cue on the root's eventual
/// despawn would put it after the wreck had already come apart on screen. It
/// also gives the sound the length it was written for - two and a half seconds
/// of debris, running OVER the frames the sections actually leave.
///
/// Deliberately NOT throttled against the section explosions it overlaps.
/// Those share a cell key and collapse into one cue between them; this is a
/// different event at a different scale, and letting the cell swallow it would
/// mean a hull dying sounded exactly like one more piece coming off.
///
/// AUTHORED-OR-SILENT on the hull's own [`ShipCollapseSound`]: a ship that
/// names none still comes apart, to the sound of its sections.
pub(super) fn on_collapse_play_hull_loss(
    add: On<Add, StructuralCollapseMarker>,
    asset_server: Res<AssetServer>,
    q_transform: Query<&GlobalTransform>,
    q_sound: Query<&ShipCollapseSound>,
    q_child_of: Query<&ChildOf>,
    q_is_root: Query<(), With<SpaceshipRootMarker>>,
    q_is_player: Query<(), With<PlayerSpaceshipMarker>>,
    mut commands: Commands,
) {
    let ship = add.entity;
    let Some(handle) = q_sound
        .get(ship)
        .ok()
        .and_then(|sound| sound.0.as_ref())
        .map(|r| r.resolve(&asset_server))
    else {
        return;
    };
    let Ok(at) = q_transform.get(ship) else {
        return;
    };
    let route = route_for(ship, &q_child_of, &q_is_root, &q_is_player);
    commands.play_sfx_at(handle, route, DESTROY_SHIP_VOLUME, at.translation());
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
    q_is_root: Query<(), With<SpaceshipRootMarker>>,
    q_is_player: Query<(), With<PlayerSpaceshipMarker>>,
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
        // Damage landing on YOUR hull is heard through it, not across the gap.
        let route = route_for(damage.entity, &q_child_of, &q_is_root, &q_is_player);
        commands.play_sfx_at(handle, route, IMPACT_VOLUME, pos);
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
    q_child_of: Query<&ChildOf>,
    q_is_root: Query<(), With<SpaceshipRootMarker>>,
    q_is_player: Query<(), With<PlayerSpaceshipMarker>>,
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
        // Routed off the FIRING TURRET, not the round: the shell is a fresh
        // root with no ship above it, and whose gun it left is the question.
        let route = route_for(part_of.0, &q_child_of, &q_is_root, &q_is_player);
        commands.play_sfx_at(handle, route, TURRET_FIRE_VOLUME, transform.translation);
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
///
/// THROTTLED PER SHIP, not per bay: a hull's tubes share a trigger, so they all
/// launch on the same frame and eight thumps sum into one clipped one. The key
/// is the firing ship, so a salvo is a single report while two ships firing at
/// once are still two.
pub(super) fn on_torpedo_launch_play_sfx(
    add: On<Add, TorpedoProjectileMarker>,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    mut throttle: ResMut<SfxThrottle>,
    q_projectile: Query<(&Transform, &TorpedoSectionSpawnerEntity)>,
    q_launch_sound: Query<&TorpedoSectionLaunchSound>,
    q_child_of: Query<&ChildOf>,
    q_is_root: Query<(), With<SpaceshipRootMarker>>,
    q_is_player: Query<(), With<PlayerSpaceshipMarker>>,
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
    // Routed off the firing BAY, for the same reason the turret is - and the
    // same walk gives the throttle the ship to collapse the salvo onto.
    let root = owning_root(spawner.0, &q_child_of, &q_is_root);
    if !throttle.allow(
        ThrottleKey::TorpedoLaunch(root),
        time.elapsed_secs(),
        TORPEDO_LAUNCH_MIN_INTERVAL,
    ) {
        return;
    }
    let route = route_from(root, &q_is_player);
    commands.play_sfx_at(handle, route, TORPEDO_LAUNCH_VOLUME, source.translation);
}

/// The lance's report, on the tick the shot leaves.
///
/// AUTHORED-OR-SILENT like the turret and the bay, and UNTHROTTLED unlike the
/// turret: one lance fires once per reload cycle, so there is no stream here
/// to rate-limit and a throttle could only ever swallow the one shot.
///
/// Answers [`RailgunFired`] rather than the slug's spawn: the shot is the
/// event, the slug is one consequence of it, and a lance that somehow spends
/// its charge without a shell still has a bore that discharged.
pub(super) fn on_railgun_fire_play_sfx(
    fired: On<RailgunFired>,
    asset_server: Res<AssetServer>,
    q_fire_sound: Query<&RailgunSectionFireSound>,
    q_child_of: Query<&ChildOf>,
    q_is_root: Query<(), With<SpaceshipRootMarker>>,
    q_is_player: Query<(), With<PlayerSpaceshipMarker>>,
    mut commands: Commands,
) {
    let Some(handle) = q_fire_sound
        .get(fired.entity)
        .ok()
        .and_then(|sound| sound.0.as_ref())
        .map(|asset_ref| asset_ref.resolve(&asset_server))
    else {
        return;
    };
    let route = route_for(fired.entity, &q_child_of, &q_is_root, &q_is_player);
    commands.play_sfx_at(handle, route, RAILGUN_FIRE_VOLUME, fired.muzzle);
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
        app.init_resource::<SfxThrottle>();
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
    fn a_ships_whole_salvo_is_one_report_and_two_ships_are_two() {
        // The fix for "a lot of torpedo bays is really loud on launch": every
        // tube on a hull fires on the same frame, so the throttle keys on the
        // SHIP and the salvo collapses to one thump. Two hulls firing together
        // are still two, which is what a cell or a global key would have lost.
        #[derive(Resource, Default)]
        struct Reports(usize);

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.init_resource::<SfxThrottle>();
        app.init_resource::<Reports>();
        app.add_observer(on_torpedo_launch_play_sfx);
        app.add_observer(|_: On<PlaySfx>, mut reports: ResMut<Reports>| reports.0 += 1);

        // Two hulls, four bays each, every tube launching on the same frame.
        for _ in 0..2 {
            let ship = app.world_mut().spawn(SpaceshipRootMarker).id();
            for _ in 0..4 {
                let bay = app
                    .world_mut()
                    .spawn((
                        TorpedoSectionLaunchSound(Some(AssetRef::from(
                            "base/sounds/torpedo_launch.wav",
                        ))),
                        ChildOf(ship),
                    ))
                    .id();
                app.world_mut().spawn((
                    TorpedoProjectileMarker,
                    Transform::default(),
                    TorpedoSectionSpawnerEntity(bay),
                ));
            }
        }
        app.world_mut().flush();

        assert_eq!(
            app.world().resource::<Reports>().0,
            2,
            "eight tubes on two hulls must report twice, once per salvo"
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

    #[test]
    fn a_filled_magazine_clunks_with_the_breech_its_own_gun_authored() {
        // The report is section-generic - every magazine that fills reports -
        // and only a gun that authored a breech has one. A PDC's trickle
        // reload must stay silent under the same event.
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.init_resource::<LastPlayed>();
        app.add_observer(on_reload_complete_play_sfx);
        app.add_observer(|ev: On<PlaySfx>, mut last: ResMut<LastPlayed>| {
            last.0 = Some(ev.handle.clone());
        });
        let breech: Handle<AudioSource> = app
            .world()
            .resource::<AssetServer>()
            .load("base/sounds/railgun_reload.wav");

        let lance = app
            .world_mut()
            .spawn((
                RailgunSectionReloadSound(Some(AssetRef::from("base/sounds/railgun_reload.wav"))),
                GlobalTransform::default(),
            ))
            .id();
        app.world_mut()
            .trigger(SectionReloadComplete { entity: lance });
        app.world_mut().flush();
        assert_eq!(app.world().resource::<LastPlayed>().0, Some(breech));

        app.world_mut().resource_mut::<LastPlayed>().0 = None;
        let pdc = app.world_mut().spawn(GlobalTransform::default()).id();
        app.world_mut()
            .trigger(SectionReloadComplete { entity: pdc });
        app.world_mut().flush();
        assert_eq!(
            app.world().resource::<LastPlayed>().0,
            None,
            "a gun with no authored breech reloads in silence"
        );
    }

    #[test]
    fn a_collapsing_hull_sounds_at_its_own_scale_and_bypasses_the_section_throttle() {
        // The cue rides OVER the section explosions of the peel that follows,
        // and it must not share their per-cell key: a hull dying is a different
        // event from one more piece coming off, and letting the cell swallow it
        // would make the two sound identical.
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.init_resource::<SfxThrottle>();
        app.init_resource::<LastPlayed>();
        app.add_observer(on_collapse_play_hull_loss);
        app.add_observer(on_destroyed_play_explosion);
        app.add_observer(|ev: On<PlaySfx>, mut last: ResMut<LastPlayed>| {
            last.0 = Some(ev.handle.clone());
        });
        let debris: Handle<AudioSource> = app
            .world()
            .resource::<AssetServer>()
            .load("base/sounds/destroy_ship.wav");

        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                GlobalTransform::default(),
                ShipCollapseSound(Some(AssetRef::from("base/sounds/destroy_ship.wav"))),
                // A section blowing up in the same cell, first: its cue takes
                // the Destroy key for this area.
                ImpactDestroySounds {
                    impact: None,
                    destroy: Some(AssetRef::from("base/sounds/explosion.wav")),
                },
            ))
            .id();
        app.world_mut()
            .entity_mut(ship)
            .insert(IntegrityDestroyMarker);
        app.world_mut().flush();

        app.world_mut()
            .entity_mut(ship)
            .insert(StructuralCollapseMarker::default());
        app.world_mut().flush();
        assert_eq!(
            app.world().resource::<LastPlayed>().0,
            Some(debris),
            "the hull-loss cue plays even with the area's Destroy key already spent"
        );
    }

    #[test]
    fn a_hull_that_names_no_collapse_sound_comes_apart_quietly() {
        // Authored-or-silent on the hull, and the guard against a fallback:
        // the sections' own explosions are what a silent hull dies to.
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.init_resource::<LastPlayed>();
        app.add_observer(on_collapse_play_hull_loss);
        app.add_observer(|ev: On<PlaySfx>, mut last: ResMut<LastPlayed>| {
            last.0 = Some(ev.handle.clone());
        });
        let ship = app
            .world_mut()
            .spawn((SpaceshipRootMarker, GlobalTransform::default()))
            .id();
        app.world_mut()
            .entity_mut(ship)
            .insert(StructuralCollapseMarker::default());
        app.world_mut().flush();
        assert_eq!(app.world().resource::<LastPlayed>().0, None);
    }
}
