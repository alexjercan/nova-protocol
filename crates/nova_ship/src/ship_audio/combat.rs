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
        railgun_section::{RailgunSectionFireSound, RailgunSectionReloadSound},
        torpedo_section::{TorpedoSectionLaunchSound, TorpedoSectionSpawnerEntity},
        turret_section::{TurretSectionFireSound, TurretSectionPartOf},
    },
};

/// Find the nearest `C` on `entity` or an ancestor. The damage/destroy
/// observers' target is the entity carrying Health - for sections that IS the
/// section entity, but an asteroid keeps its Health on a child node while the
/// snapshots sit on the rock's parent bundle, so the lookup walks up (bounded
/// by the hierarchy, like `hum_source_root`).
fn nearest<'a, C: Component>(
    entity: Entity,
    q: &'a Query<&C>,
    q_child_of: &Query<&ChildOf>,
) -> Option<&'a C> {
    let mut current = entity;
    loop {
        if let Ok(found) = q.get(current) {
            return Some(found);
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
    q_sounds: Query<&DestroySound>,
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
    let Some(sound) = nearest(add.entity, &q_sounds, &q_child_of).and_then(|s| s.0.as_ref()) else {
        return;
    };
    let pos = source.translation();
    if throttle_state.allow(
        ThrottleKey::Explosion(area_cell(pos)),
        time.elapsed_secs(),
        EXPLOSION_MIN_INTERVAL,
    ) {
        let route = route_for(add.entity, &q_child_of, &q_is_root, &q_is_player);
        commands.play_sfx_at(sound.resolve(&asset_server), route, EXPLOSION_VOLUME, pos);
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

/// The hit voice: what the round that just landed sounds like against what it
/// landed on. Throttled per area cell, because a single blast reaches many
/// colliders in one frame.
///
/// Both halves of "what hit what" are read here, and neither is a per-target
/// field any more. The round side is [`SurfaceImpact::kind`]; the target side
/// is the struck body's [`SurfaceMaterial`], found by walking up from the hit
/// (an asteroid keeps its Health on a child node). The pair goes to
/// [`GameImpacts`], which falls back once to the damage type's default row and
/// is otherwise AUTHORED-OR-SILENT like every other voice.
///
/// [`SurfaceImpact`] does not propagate, which is why there is no
/// "am I the original target" guard here: the old cue rode `HealthApplyDamage`
/// up `ChildOf` to the ship root and had to filter the hops back out. It also
/// carries the CONTACT POINT rather than the struck entity's origin, so the cue
/// plays where the round actually bit.
pub(super) fn on_surface_impact_play_sfx(
    impact: On<SurfaceImpact>,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    impacts: Res<GameImpacts>,
    q_material: Query<&SurfaceMaterial>,
    q_child_of: Query<&ChildOf>,
    q_is_root: Query<(), With<SpaceshipRootMarker>>,
    q_is_player: Query<(), With<PlayerSpaceshipMarker>>,
    mut throttle_state: ResMut<SfxThrottle>,
    mut commands: Commands,
) {
    let material = nearest(impact.entity, &q_material, &q_child_of).map(|m| m.0.as_str());
    let Some(sound) = impacts.sound(impact.kind, material) else {
        return;
    };
    let pos = impact.at;
    if throttle_state.allow(
        ThrottleKey::Impact(area_cell(pos)),
        time.elapsed_secs(),
        IMPACT_MIN_INTERVAL,
    ) {
        // Damage landing on YOUR hull is heard through it, not across the gap.
        let route = route_for(impact.entity, &q_child_of, &q_is_root, &q_is_player);
        commands.play_sfx_at(sound.resolve(&asset_server), route, IMPACT_VOLUME, pos);
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
    // No authored sound -> silent, and BEFORE the throttle: an unauthored
    // turret plays nothing, so there is nothing to rate-limit. Only the
    // resolve waits, because it is an `AssetServer::load` and most calls here
    // are about to be thrown away by the throttle.
    let Some(sound) = q_fire_sound.get(part_of.0).ok().and_then(|s| s.0.as_ref()) else {
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
        commands.play_sfx_at(
            sound.resolve(&asset_server),
            route,
            TURRET_FIRE_VOLUME,
            transform.translation,
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
    let Some(sound) = q_launch_sound
        .get(spawner.0)
        .ok()
        .and_then(|s| s.0.as_ref())
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
    commands.play_sfx_at(
        sound.resolve(&asset_server),
        route,
        TORPEDO_LAUNCH_VOLUME,
        source.translation,
    );
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
    fn impact_row(
        id: &str,
        damage: DamageType,
        material: Option<&str>,
        sound: &str,
    ) -> ImpactSoundConfig {
        ImpactSoundConfig {
            id: id.to_string(),
            damage,
            material: material.map(str::to_string),
            sound: AssetRef::from(sound),
        }
    }

    /// App rig for the hit voice: the real observer over a supplied impact
    /// table, counting cues and capturing the last handle. No bank and no audio
    /// device - the cue resolves the table's own refs against the
    /// `AssetServer`.
    fn impact_app(table: Vec<ImpactSoundConfig>) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.init_resource::<SfxThrottle>();
        app.init_resource::<PlayedSfx>();
        app.init_resource::<LastPlayed>();
        app.insert_resource(GameImpacts(table));
        app.add_observer(on_surface_impact_play_sfx);
        app.add_observer(|_: On<PlaySfx>, mut played: ResMut<PlayedSfx>| played.0 += 1);
        app.add_observer(|ev: On<PlaySfx>, mut last: ResMut<LastPlayed>| {
            last.0 = Some(ev.handle.clone());
        });
        app
    }

    fn base_table() -> Vec<ImpactSoundConfig> {
        vec![
            impact_row("kinetic", DamageType::Kinetic, None, "mods/x/thud.wav"),
            impact_row(
                "kinetic_rock",
                DamageType::Kinetic,
                Some(MATERIAL_ROCK),
                "mods/x/gravel.wav",
            ),
            impact_row("pierce", DamageType::Pierce, None, "mods/x/punch.wav"),
        ]
    }

    /// Hit `target` at `at` and flush - the observer plays via `Commands`, so
    /// the queued `PlaySfx` triggers only fire on the next flush.
    fn strike(app: &mut App, target: Entity, kind: DamageType, at: Vec3) {
        app.world_mut().trigger(SurfaceImpact {
            entity: target,
            kind,
            at,
        });
        app.world_mut().flush();
    }

    fn last(app: &App) -> Option<Handle<AudioSource>> {
        app.world().resource::<LastPlayed>().0.clone()
    }

    #[test]
    fn what_a_hit_sounds_like_is_the_round_and_the_material_together() {
        let mut app = impact_app(base_table());
        let server = app.world().resource::<AssetServer>().clone();
        let thud: Handle<AudioSource> = server.load("mods/x/thud.wav");
        let gravel: Handle<AudioSource> = server.load("mods/x/gravel.wav");
        let punch: Handle<AudioSource> = server.load("mods/x/punch.wav");

        let rock = app
            .world_mut()
            .spawn(SurfaceMaterial::new(MATERIAL_ROCK))
            .id();
        let hull = app
            .world_mut()
            .spawn(SurfaceMaterial::new(MATERIAL_HULL))
            .id();

        // Same target, two rounds: the rock has its own kinetic row and no
        // pierce row, so a penetrator into stone takes the PIERCE default -
        // never the kinetic rock voice.
        strike(&mut app, rock, DamageType::Kinetic, Vec3::ZERO);
        assert_eq!(last(&app), Some(gravel));
        strike(
            &mut app,
            rock,
            DamageType::Pierce,
            Vec3::splat(SFX_AREA_CELL * 10.0),
        );
        assert_eq!(last(&app), Some(punch));

        // Same round, two materials: the hull names no row of its own and
        // takes the kinetic default.
        strike(
            &mut app,
            hull,
            DamageType::Kinetic,
            Vec3::splat(SFX_AREA_CELL * 20.0),
        );
        assert_eq!(last(&app), Some(thud));
    }

    #[test]
    fn the_hit_sounds_where_it_landed_and_not_where_the_target_sits() {
        // The old cue rode `HealthApplyDamage` up `ChildOf` and had to filter
        // the hops back out, positioning itself at the struck entity's origin.
        // `SurfaceImpact` does not propagate and carries the contact point, so
        // one hit is one cue and it is keyed at the point, not the body.
        let mut app = impact_app(base_table());
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
                SurfaceMaterial::new(MATERIAL_HULL),
            ))
            .id();

        strike(
            &mut app,
            child,
            DamageType::Kinetic,
            Vec3::new(SFX_AREA_CELL * 4.0, 0.0, 0.0),
        );

        assert_eq!(
            app.world().resource::<PlayedSfx>().0,
            1,
            "one hit must play exactly one impact sound"
        );
        let throttle = app.world().resource::<SfxThrottle>();
        assert!(throttle
            .tracked_keys()
            .eq([ThrottleKey::Impact(area_cell(Vec3::new(
                SFX_AREA_CELL * 4.0,
                0.0,
                0.0
            )))]));
    }

    #[test]
    fn the_material_lookup_walks_up_to_the_asteroid_parent() {
        // The asteroid shape: the colliders that take the hit are CHILD nodes
        // while the material tag sits on the rock's parent bundle.
        let mut app = impact_app(base_table());
        let gravel: Handle<AudioSource> = app
            .world()
            .resource::<AssetServer>()
            .load("mods/x/gravel.wav");
        let rock = app
            .world_mut()
            .spawn(SurfaceMaterial::new(MATERIAL_ROCK))
            .id();
        let node = app.world_mut().spawn(ChildOf(rock)).id();

        strike(&mut app, node, DamageType::Kinetic, Vec3::ZERO);
        assert_eq!(
            last(&app),
            Some(gravel),
            "the hit voice must find the parent's material via the walk"
        );
    }

    #[test]
    fn a_round_the_table_never_names_lands_in_silence() {
        // AUTHORED-OR-SILENT, and the table falls back exactly once - to its
        // own damage type's default row. Explosive has neither, so a blast on
        // a tagged hull makes no hit noise at all.
        let mut app = impact_app(base_table());
        let hull = app
            .world_mut()
            .spawn(SurfaceMaterial::new(MATERIAL_HULL))
            .id();
        strike(&mut app, hull, DamageType::Explosive, Vec3::ZERO);
        assert_eq!(app.world().resource::<PlayedSfx>().0, 0);

        // And an untagged target is not an error: it takes the default row.
        let bare = app.world_mut().spawn_empty().id();
        strike(
            &mut app,
            bare,
            DamageType::Kinetic,
            Vec3::splat(SFX_AREA_CELL * 10.0),
        );
        assert_eq!(app.world().resource::<PlayedSfx>().0, 1);
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
    fn a_destroyed_target_plays_the_voice_it_authored_or_stays_silent() {
        // The destruction voice stayed per-target when the hit voice moved to
        // the table: the destroyed entity's own authored ref plays, and an
        // unauthored target is silent. The authored half is the delivery guard
        // for the silent half.
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.init_resource::<SfxThrottle>();
        app.init_resource::<LastPlayed>();
        app.add_observer(on_destroyed_play_explosion);
        app.add_observer(|ev: On<PlaySfx>, mut last: ResMut<LastPlayed>| {
            last.0 = Some(ev.handle.clone());
        });
        let boom: Handle<AudioSource> = app
            .world()
            .resource::<AssetServer>()
            .load("mods/x/boom.wav");

        let target = app
            .world_mut()
            .spawn((
                GlobalTransform::default(),
                DestroySound(Some(AssetRef::from("mods/x/boom.wav"))),
            ))
            .id();
        app.world_mut()
            .entity_mut(target)
            .insert(IntegrityDestroyMarker);
        app.world_mut().flush();
        assert_eq!(app.world().resource::<LastPlayed>().0, Some(boom));

        // Unauthored target: silent (a different cell, so the throttle is not
        // what is being measured).
        app.world_mut().resource_mut::<LastPlayed>().0 = None;
        let silent = app
            .world_mut()
            .spawn(GlobalTransform::from(Transform::from_translation(
                Vec3::splat(SFX_AREA_CELL * 20.0),
            )))
            .id();
        app.world_mut()
            .entity_mut(silent)
            .insert(IntegrityDestroyMarker);
        app.world_mut().flush();
        assert_eq!(
            app.world().resource::<LastPlayed>().0,
            None,
            "an unauthored target is destroyed in silence"
        );
    }

    #[test]
    fn the_destroy_lookup_walks_up_to_the_asteroid_parent() {
        // The asteroid shape: Health (and the destroy marker) live on a CHILD
        // node while the destruction voice sits on the rock's parent bundle -
        // the observer must find it by walking up.
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
            .spawn(DestroySound(Some(AssetRef::from(
                "base/sounds/explosion.wav",
            ))))
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
                DestroySound(Some(AssetRef::from("base/sounds/explosion.wav"))),
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
