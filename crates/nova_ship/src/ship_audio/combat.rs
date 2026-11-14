//! Combat one-shots: explosions, impacts, turret fire, torpedo launches and
//! deaths, the railgun's report and the kill stinger, each resolving the
//! target's authored sound or staying silent.
//!
//! Every cue names a category and where it happens; `nova_sound` owns the
//! level, the reach, the pan and the repeat budget. A cue on a section or a
//! hull plays at that entity and groups with its ship, so one hull's salvo or
//! one hull shedding its sections is one report. A cue with no body - a
//! torpedo's drive, its blast, its death - plays at a point and groups with
//! its world cell.

use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_sound::prelude::*;

use crate::{
    prelude::*,
    sections::{
        railgun_section::{RailgunSectionFireSound, RailgunSlugImpactSound},
        torpedo_section::{
            TorpedoLaunchConfig, TorpedoSectionLaunchSound, TorpedoSectionSpawnerEntity,
        },
        turret_section::{TurretSectionFireSound, TurretSectionPartOf},
    },
};

/// Find the nearest `C` on `entity` or an ancestor. The damage/destroy
/// observers' target is the entity carrying Health - for sections that IS the
/// section entity, but an asteroid keeps its Health on a child node while the
/// snapshots sit on the rock's parent bundle, so the lookup walks up.
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

/// Trigger one world cue.
fn play(
    commands: &mut Commands,
    sound: Handle<AudioSource>,
    category: SoundCategoryType,
    at: SoundAt,
) {
    commands.trigger(PlaySound {
        sound,
        category,
        at,
    });
}

/// Explosion cue on any destruction that funnels through
/// `IntegrityDestroyMarker`: a section or an asteroid. A torpedo section is
/// skipped (see [`QuietDestroyMarker`]).
///
/// Played at the destroyed entity, so a dying hull's sections share its
/// group: one structure coming apart is one explosion.
pub(super) fn on_destroyed_play_explosion(
    add: On<Add<IntegrityDestroyMarker>>,
    asset_server: Res<AssetServer>,
    q_sounds: Query<&DestroySound>,
    q_child_of: Query<&ChildOf>,
    q_quiet: Query<(), With<QuietDestroyMarker>>,
    mut commands: Commands,
) {
    // A torpedo section's death is a dud or an intercept, voiced on
    // `TorpedoKilled`; walking up from it would find the warhead's sound.
    if q_quiet.contains(add.entity) {
        return;
    }
    // AUTHORED-OR-SILENT: the destruction voice is the TARGET's authored
    // destroy_sound (per-target = per-material), found on the entity or an
    // ancestor (asteroid node shape) and resolved here.
    let Some(sound) = nearest(add.entity, &q_sounds, &q_child_of).and_then(|s| s.0.as_ref()) else {
        return;
    };
    play(
        &mut commands,
        sound.resolve(&asset_server),
        SoundCategoryType::Explosion,
        SoundAt::Entity(add.entity),
    );
}

/// The reload cue, each time a batch lands in a magazine: the lance's breech,
/// the torpedo bay's rack. Played at the SECTION: a raider's lance chambering
/// across the arena is the tell that says the next one is coming.
///
/// AUTHORED-OR-SILENT on the section's [`SectionReloadSound`]. Every weapon
/// with a magazine reports a batch, and only the ones that author a reload
/// sound have one: a PDC's reload is a trickle with no moment in it, and no
/// turret carries the component.
pub(super) fn on_reload_landed_play_sfx(
    landed: On<SectionReloadLanded>,
    asset_server: Res<AssetServer>,
    q_sound: Query<&SectionReloadSound>,
    mut commands: Commands,
) {
    let section = landed.entity;
    let Some(handle) = q_sound
        .get(section)
        .ok()
        .and_then(|sound| sound.0.as_ref())
        .map(|r| r.resolve(&asset_server))
    else {
        return;
    };
    play(
        &mut commands,
        handle,
        SoundCategoryType::Reload,
        SoundAt::Entity(section),
    );
}

/// The hull-loss cue, on the frame a ship COLLAPSES.
///
/// The collapse edge, not the root's death: a ship stops being a ship the
/// moment it falls under its structural threshold, and what follows is several
/// frames of its sections peeling away one at a time
/// (`cascade_structural_collapse`). Hanging the cue on the root's eventual
/// despawn would put it after the wreck had already come apart on screen.
///
/// Its own category, so the section explosions it overlaps never swallow it.
///
/// AUTHORED-OR-SILENT on the hull's own [`ShipCollapseSound`]: a ship that
/// names none still comes apart, to the sound of its sections.
pub(super) fn on_collapse_play_hull_loss(
    add: On<Add<StructuralCollapseMarker>>,
    asset_server: Res<AssetServer>,
    q_sound: Query<&ShipCollapseSound>,
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
    play(
        &mut commands,
        handle,
        SoundCategoryType::Collapse,
        SoundAt::Entity(ship),
    );
}

/// The hit voice: what the round that just landed sounds like against what it
/// landed on.
///
/// Both halves of "what hit what" are read here, and neither is authored. The
/// round side is [`SurfaceImpact::kind`]; the target side is the struck body's
/// [`ImpactSurface`], found by walking up from the hit (an asteroid keeps its
/// Health on a child node). Both are closed enums, so [`ImpactSounds`] answers
/// the pair with a `match` - every hit has a voice, and a body that carries no
/// tag at all is plate, which is what a nameless collider in this game is.
///
/// One round overrides the table: a railgun slug whose lance authored an
/// `impact_sound` carries it as [`RailgunSlugImpactSound`], and that plays in
/// the table's place as a [`SoundCategoryType::HeavyImpact`].
///
/// Played at the struck entity, so a blast raking a dozen of one hull's
/// colliders groups as one hull.
pub(super) fn on_surface_impact_play_sfx(
    impact: On<SurfaceImpact>,
    asset_server: Res<AssetServer>,
    impacts: Res<ImpactSounds>,
    q_slug_sound: Query<&RailgunSlugImpactSound>,
    q_surface: Query<&ImpactSurface>,
    q_child_of: Query<&ChildOf>,
    mut commands: Commands,
) {
    let authored = impact
        .source
        .and_then(|source| q_slug_sound.get(source).ok())
        .map(|sound| sound.0.resolve(&asset_server));
    let (sound, category) = match authored {
        Some(sound) => (sound, SoundCategoryType::HeavyImpact),
        None => {
            let surface = nearest(impact.entity, &q_surface, &q_child_of)
                .copied()
                .unwrap_or_default();
            (
                impacts.sound(impact.kind, surface).clone(),
                SoundCategoryType::Impact,
            )
        }
    };
    play(
        &mut commands,
        sound,
        category,
        SoundAt::Entity(impact.entity),
    );
}

/// Turret-fire cue when a round spawns, at the firing turret.
///
/// AUTHORED-OR-SILENT: the sound is the firing turret's
/// [`TurretSectionConfig::fire_sound`], snapshotted at spawn as
/// [`TurretSectionFireSound`] and resolved here - content owns the sound, and a
/// turret that authors none fires silently.
pub(super) fn on_turret_fire_play_sfx(
    fired: On<RoundFired>,
    asset_server: Res<AssetServer>,
    q_projectile: Query<&TurretSectionPartOf>,
    q_fire_sound: Query<&TurretSectionFireSound>,
    mut commands: Commands,
) {
    // `TurretSectionPartOf` names the firing turret: the shell is a fresh
    // root with no ship above it, and whose gun it left is the question.
    let Ok(part_of) = q_projectile.get(fired.round) else {
        return;
    };
    let Some(sound) = q_fire_sound.get(part_of.0).ok().and_then(|s| s.0.as_ref()) else {
        return;
    };
    play(
        &mut commands,
        sound.resolve(&asset_server),
        SoundCategoryType::Weapon,
        SoundAt::Entity(part_of.0),
    );
}

/// Launch cue when a torpedo projectile spawns, at the firing bay.
///
/// AUTHORED-OR-SILENT: the sound is the firing bay's
/// [`TorpedoSectionConfig::launch_sound`], snapshotted onto the bay's spawner
/// as [`TorpedoSectionLaunchSound`] and reached from the projectile via its
/// [`TorpedoSectionSpawnerEntity`] back-ref (the same path the launch flash
/// effect takes).
///
/// Every bay on a hull fires on one frame, and the bays share the hull's
/// group, so a salvo is one report while two ships firing at once are two.
pub(super) fn on_torpedo_launch_play_sfx(
    launched: On<TorpedoLaunched>,
    asset_server: Res<AssetServer>,
    q_projectile: Query<&TorpedoSectionSpawnerEntity>,
    q_launch_sound: Query<&TorpedoSectionLaunchSound>,
    mut commands: Commands,
) {
    let Ok(spawner) = q_projectile.get(launched.torpedo) else {
        return;
    };
    let Some(sound) = q_launch_sound
        .get(spawner.0)
        .ok()
        .and_then(|s| s.0.as_ref())
    else {
        return;
    };
    play(
        &mut commands,
        sound.resolve(&asset_server),
        SoundCategoryType::Launch,
        SoundAt::Entity(spawner.0),
    );
}

/// The drive lighting, when a torpedo's cold coast ends.
///
/// AUTHORED-OR-SILENT: the bay's `ignition_sound`, read off the torpedo's own
/// [`TorpedoLaunchConfig`] snapshot, so a torpedo thawed mid-coast still has
/// it. A torpedo has no ship above it, so the cue plays at the point where the
/// drive lit; a salvo's drives light in one cell on one frame, and that is one
/// ignition.
pub(super) fn on_torpedo_ignited_play_sfx(
    ignited: On<TorpedoIgnited>,
    asset_server: Res<AssetServer>,
    q_torpedo: Query<&TorpedoLaunchConfig>,
    mut commands: Commands,
) {
    let Ok(config) = q_torpedo.get(ignited.torpedo) else {
        return;
    };
    let Some(sound) = config.ignition_sound.as_ref() else {
        return;
    };
    play(
        &mut commands,
        sound.resolve(&asset_server),
        SoundCategoryType::Ignition,
        SoundAt::Point(ignited.at),
    );
}

/// The warhead, as a torpedo's blast lands.
///
/// AUTHORED-OR-SILENT: the bay's `detonation_sound`, which the fuze moves from
/// the torpedo onto the blast it spawns ([`DestroySound`]), because the
/// torpedo is despawned in the same flush. A dud or an intercept spawns no
/// blast, so neither ever plays it.
///
/// The blast has no ship above it: it plays at its point, and a salvo
/// bursting in one cell is one boom.
pub(super) fn on_torpedo_blast_play_detonation(
    add: On<Add<NovaBlast>>,
    asset_server: Res<AssetServer>,
    q_blast: Query<(&Transform, &DestroySound), With<TorpedoSectionPartOf>>,
    mut commands: Commands,
) {
    let Ok((transform, sound)) = q_blast.get(add.entity) else {
        return;
    };
    let Some(sound) = sound.0.as_ref() else {
        return;
    };
    // Spawned this flush and unparented: the Transform is the world pose.
    play(
        &mut commands,
        sound.resolve(&asset_server),
        SoundCategoryType::Explosion,
        SoundAt::Point(transform.translation),
    );
}

/// The dud or the intercept, when a torpedo dies with its warhead unfired.
///
/// AUTHORED-OR-SILENT: the bay's `dud_sound` or `intercept_sound`, read off
/// the torpedo's own [`TorpedoLaunchConfig`] snapshot, so a torpedo whose bay
/// is gone still has its voice. Every torpedo, the player's and a hostile's.
///
/// At the point it died: point defense breaks up a salvo in one patch of sky,
/// and that cell is one burst.
pub(super) fn on_torpedo_killed_play_sfx(
    killed: On<TorpedoKilled>,
    asset_server: Res<AssetServer>,
    q_config: Query<&TorpedoLaunchConfig>,
    mut commands: Commands,
) {
    let Ok(config) = q_config.get(killed.torpedo) else {
        return;
    };
    let sound = match killed.death {
        TorpedoDeathType::Contact => &config.dud_sound,
        TorpedoDeathType::Damage => &config.intercept_sound,
    };
    let Some(sound) = sound.as_ref() else {
        return;
    };
    play(
        &mut commands,
        sound.resolve(&asset_server),
        SoundCategoryType::TorpedoDeath,
        SoundAt::Point(killed.at),
    );
}

/// The kill stinger, once per hull the player's own fire got: a round, a
/// slug, a blast or a ram, whichever landed inside the credit window.
///
/// A cockpit cue, on [`SoundBus::Ui`]: the computer confirming the kill, not
/// the kill itself, which has its own explosion out in the world.
/// AUTHORED-OR-SILENT on the player ship's [`ShipFeedbackSounds::kill_confirm`].
pub(super) fn on_player_kill_play_stinger(
    _confirmed: On<PlayerKillConfirmed>,
    asset_server: Res<AssetServer>,
    q_player: Query<&ShipFeedbackSounds, With<PlayerSpaceshipMarker>>,
    mut commands: Commands,
) {
    let Some(handle) = q_player
        .iter()
        .next()
        .and_then(|sounds| sounds.kill_confirm.as_ref())
        .map(|sound| sound.resolve(&asset_server))
    else {
        return;
    };
    play(
        &mut commands,
        handle,
        SoundCategoryType::Cockpit,
        SoundAt::Ui,
    );
}

/// The lance's report, on the tick the shot leaves, at the lance.
///
/// AUTHORED-OR-SILENT like the turret and the bay. Answers [`RailgunFired`]
/// rather than the slug's spawn: the shot is the event, the slug is one
/// consequence of it.
pub(super) fn on_railgun_fire_play_sfx(
    fired: On<RailgunFired>,
    asset_server: Res<AssetServer>,
    q_fire_sound: Query<&RailgunSectionFireSound>,
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
    play(
        &mut commands,
        handle,
        SoundCategoryType::HeavyWeapon,
        SoundAt::Entity(fired.entity),
    );
}

#[cfg(test)]
mod tests {
    use nova_sound::SFX_AREA_CELL;

    use super::*;
    use crate::ship_audio::test_support::{LastPlayed, PlayedSfx};
    /// App rig for the hit voice: the real observer over the engine's own
    /// sample bank, counting cues and capturing the last handle. No audio
    /// device - the bank loads four paths against the `AssetServer`.
    fn impact_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.init_resource::<PlayedSfx>();
        app.init_resource::<LastPlayed>();
        app.init_resource::<ImpactSounds>();
        app.add_observer(on_surface_impact_play_sfx);
        app.add_observer(|_: On<PlaySound>, mut played: ResMut<PlayedSfx>| played.0 += 1);
        app.add_observer(|ev: On<PlaySound>, mut last: ResMut<LastPlayed>| {
            last.0 = Some(ev.sound.clone());
        });
        app
    }

    /// The bank the rig loaded, to compare a played handle against.
    fn bank(app: &App) -> ImpactSounds {
        app.world().resource::<ImpactSounds>().clone()
    }

    /// Hit `target` at `at` and flush - the observer plays via `Commands`, so
    /// the queued `PlaySfx` triggers only fire on the next flush.
    fn strike(app: &mut App, target: Entity, kind: DamageType, at: Vec3) {
        strike_from(app, target, None, kind, at);
    }

    /// Hit `target` with `source` - the round the sweep names - and flush.
    fn strike_from(
        app: &mut App,
        target: Entity,
        source: Option<Entity>,
        kind: DamageType,
        at: Vec3,
    ) {
        app.world_mut().trigger(SurfaceImpact {
            entity: target,
            source,
            kind,
            at,
        });
        app.world_mut().flush();
    }

    fn last(app: &App) -> Option<Handle<AudioSource>> {
        app.world().resource::<LastPlayed>().0.clone()
    }

    #[test]
    fn what_a_hit_sounds_like_is_the_round_and_the_surface_together() {
        let mut app = impact_app();
        let bank = bank(&app);

        let rock = app.world_mut().spawn(ImpactSurface::Rock).id();
        let hull = app.world_mut().spawn(ImpactSurface::Hull).id();

        // Same target, two rounds: a slug into stone is gravel, a penetrator
        // into the same stone is the penetration - the round wins when the
        // round is what the sample is OF.
        strike(&mut app, rock, DamageType::Kinetic, Vec3::ZERO);
        assert_eq!(last(&app), Some(bank.kinetic_rock.clone()));
        strike(
            &mut app,
            rock,
            DamageType::Pierce,
            Vec3::splat(SFX_AREA_CELL * 10.0),
        );
        assert_eq!(last(&app), Some(bank.pierce.clone()));

        // Same round, two surfaces: the slug is the one that hears what it hit.
        strike(
            &mut app,
            hull,
            DamageType::Kinetic,
            Vec3::splat(SFX_AREA_CELL * 20.0),
        );
        assert_eq!(last(&app), Some(bank.kinetic_hull.clone()));

        // A lance round that carries its own voice plays THAT, whatever it
        // hit; a lance round that carries none is the table's Pierce row.
        let authored = AssetRef::from("sounds/railgun_impact.wav".to_string());
        let expected = authored.resolve(app.world().resource::<AssetServer>());
        let voiced = app
            .world_mut()
            .spawn((
                RailgunSlugProjectileMarker,
                RailgunSlugImpactSound(authored),
            ))
            .id();
        let mute = app.world_mut().spawn(RailgunSlugProjectileMarker).id();
        strike_from(
            &mut app,
            rock,
            Some(voiced),
            DamageType::Pierce,
            Vec3::splat(SFX_AREA_CELL * 30.0),
        );
        assert_eq!(last(&app), Some(expected), "the authored impact wins");
        strike_from(
            &mut app,
            rock,
            Some(mute),
            DamageType::Pierce,
            Vec3::splat(SFX_AREA_CELL * 40.0),
        );
        assert_eq!(
            last(&app),
            Some(bank.pierce.clone()),
            "no authored impact: the bank"
        );
    }

    #[test]
    fn one_hit_is_one_cue_on_the_struck_collider() {
        // The old cue rode `HealthApplyDamage` up `ChildOf` and had to filter
        // the hops back out. `SurfaceImpact` does not propagate, so one hit is
        // one cue, placed on the collider the round bit: its ship is the
        // group.
        #[derive(Resource, Default)]
        struct Heard(Vec<SoundAt>);
        let mut app = impact_app();
        app.init_resource::<Heard>();
        app.add_observer(|ev: On<PlaySound>, mut heard: ResMut<Heard>| heard.0.push(ev.at));
        let parent = app.world_mut().spawn(GlobalTransform::default()).id();
        let child = app
            .world_mut()
            .spawn((
                GlobalTransform::default(),
                ChildOf(parent),
                ImpactSurface::Hull,
            ))
            .id();

        strike(&mut app, child, DamageType::Kinetic, Vec3::X);

        assert_eq!(
            app.world().resource::<Heard>().0,
            vec![SoundAt::Entity(child)],
            "one hit must play exactly one impact sound, on the struck collider"
        );
    }

    #[test]
    fn the_surface_lookup_walks_up_to_the_asteroid_parent() {
        // The asteroid shape: the colliders that take the hit are CHILD nodes
        // while the surface tag sits on the rock's parent bundle.
        let mut app = impact_app();
        let bank = bank(&app);
        let rock = app.world_mut().spawn(ImpactSurface::Rock).id();
        let node = app.world_mut().spawn(ChildOf(rock)).id();

        strike(&mut app, node, DamageType::Kinetic, Vec3::ZERO);
        assert_eq!(
            last(&app),
            Some(bank.kinetic_rock),
            "the hit voice must find the parent's surface via the walk"
        );
    }

    #[test]
    fn every_round_has_a_voice_and_an_untagged_body_is_plate() {
        // The whole point of closing both axes: there is no silent pair left.
        // A blast used to need an authored row and made no noise without one.
        let mut app = impact_app();
        let bank = bank(&app);
        let hull = app.world_mut().spawn(ImpactSurface::Hull).id();
        strike(&mut app, hull, DamageType::Explosive, Vec3::ZERO);
        assert_eq!(last(&app), Some(bank.explosive));

        // And an untagged target is not an error: it is plate.
        let bare = app.world_mut().spawn_empty().id();
        strike(
            &mut app,
            bare,
            DamageType::Kinetic,
            Vec3::splat(SFX_AREA_CELL * 10.0),
        );
        assert_eq!(last(&app), Some(bank.kinetic_hull));
        assert_eq!(app.world().resource::<PlayedSfx>().0, 2);
    }

    /// App rig for the turret-fire cue: the real `on_turret_fire_play_sfx`
    /// observer, capturing the played handle. No audio device needed (nothing
    /// constructs an `AudioSink`), and no bank: the cue is authored-or-silent,
    /// resolving the turret's own snapshot against the `AssetServer`.
    fn turret_fire_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.init_resource::<LastPlayed>();
        app.add_observer(on_turret_fire_play_sfx);
        app.add_observer(|ev: On<PlaySound>, mut last: ResMut<LastPlayed>| {
            last.0 = Some(ev.sound.clone());
        });
        app
    }

    /// Spawn a turret round parented (by `TurretSectionPartOf`) to `turret`,
    /// then trigger `RoundFired` to fire the cue observer.
    fn fire_round(app: &mut App, turret: Entity) {
        let round = app
            .world_mut()
            .spawn((
                TurretBulletProjectileMarker,
                Transform::default(),
                TurretSectionPartOf(turret),
            ))
            .id();
        app.world_mut().flush();
        app.world_mut().trigger(RoundFired { round });
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
        app.add_observer(|ev: On<PlaySound>, mut last: ResMut<LastPlayed>| {
            last.0 = Some(ev.sound.clone());
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
        let torpedo = app
            .world_mut()
            .spawn((
                TorpedoProjectileMarker,
                Transform::default(),
                TorpedoSectionSpawnerEntity(authored),
            ))
            .id();
        app.world_mut().flush();
        app.world_mut().trigger(TorpedoLaunched { torpedo });
        app.world_mut().flush();
        assert_eq!(
            app.world().resource::<LastPlayed>().0,
            Some(expected),
            "an authored launch_sound must resolve + play"
        );

        app.world_mut().resource_mut::<LastPlayed>().0 = None;
        let silent = app.world_mut().spawn(TorpedoSectionLaunchSound(None)).id();
        let torpedo = app
            .world_mut()
            .spawn((
                TorpedoProjectileMarker,
                Transform::default(),
                TorpedoSectionSpawnerEntity(silent),
            ))
            .id();
        app.world_mut().flush();
        app.world_mut().trigger(TorpedoLaunched { torpedo });
        app.world_mut().flush();
        assert_eq!(
            app.world().resource::<LastPlayed>().0,
            None,
            "an unauthored bay launches silently"
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
        app.init_resource::<LastPlayed>();
        app.add_observer(on_destroyed_play_explosion);
        app.add_observer(|ev: On<PlaySound>, mut last: ResMut<LastPlayed>| {
            last.0 = Some(ev.sound.clone());
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
        app.init_resource::<LastPlayed>();
        app.add_observer(on_destroyed_play_explosion);
        app.add_observer(|ev: On<PlaySound>, mut last: ResMut<LastPlayed>| {
            last.0 = Some(ev.sound.clone());
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
    fn a_landed_batch_clunks_with_the_reload_sound_its_own_section_authored() {
        // The report is section-generic - every batch that lands reports -
        // and only a section that authored a reload sound has one. A PDC's
        // trickle reload must stay silent under the same event.
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.init_resource::<LastPlayed>();
        app.add_observer(on_reload_landed_play_sfx);
        app.add_observer(|ev: On<PlaySound>, mut last: ResMut<LastPlayed>| {
            last.0 = Some(ev.sound.clone());
        });
        let breech: Handle<AudioSource> = app
            .world()
            .resource::<AssetServer>()
            .load("base/sounds/railgun_reload.wav");

        let lance = app
            .world_mut()
            .spawn((
                SectionReloadSound(Some(AssetRef::from("base/sounds/railgun_reload.wav"))),
                GlobalTransform::default(),
            ))
            .id();
        app.world_mut().trigger(SectionReloadLanded {
            entity: lance,
            rounds: 1,
        });
        app.world_mut().flush();
        assert_eq!(app.world().resource::<LastPlayed>().0, Some(breech));

        app.world_mut().resource_mut::<LastPlayed>().0 = None;
        let pdc = app.world_mut().spawn(GlobalTransform::default()).id();
        app.world_mut().trigger(SectionReloadLanded {
            entity: pdc,
            rounds: 1,
        });
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
        app.init_resource::<LastPlayed>();
        app.add_observer(on_collapse_play_hull_loss);
        app.add_observer(on_destroyed_play_explosion);
        app.add_observer(|ev: On<PlaySound>, mut last: ResMut<LastPlayed>| {
            last.0 = Some(ev.sound.clone());
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
        app.add_observer(|ev: On<PlaySound>, mut last: ResMut<LastPlayed>| {
            last.0 = Some(ev.sound.clone());
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

    #[test]
    fn a_fuzed_torpedo_voices_its_warhead_as_the_blast_lands() {
        // The real fuze, not a hand-built blast: the warhead's voice rides from
        // the torpedo onto the blast it spawns, and the blast observer has to
        // find it there on Add. Bevy runs that observer inside the spawn
        // command, so a voice inserted by a later command is never heard.
        use avian3d::collider_tree::ColliderTrees;

        use crate::sections::torpedo_section::torpedo_detonate_system;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        // An empty broad phase: no skin to find, so the fuze arms on the
        // frozen aim point under the torpedo.
        app.init_resource::<ColliderTrees>();
        app.init_resource::<LastPlayed>();
        app.add_systems(Update, torpedo_detonate_system);
        app.add_observer(on_torpedo_blast_play_detonation);
        app.add_observer(|ev: On<PlaySound>, mut last: ResMut<LastPlayed>| {
            last.0 = Some(ev.sound.clone());
        });
        let warhead: Handle<AudioSource> = app
            .world()
            .resource::<AssetServer>()
            .load("base/sounds/torpedo_detonate.wav");

        let bay = app.world_mut().spawn_empty().id();
        let mut arming = TorpedoArming::new(0.5, 5.0, Vec3::ZERO, 0.0);
        arming.tick(1.0, Vec3::ZERO, None);
        app.world_mut().spawn((
            TorpedoProjectileMarker,
            Transform::default(),
            TorpedoTargetPosition(Vec3::ZERO),
            arming,
            TorpedoBlast {
                radius: 30.0,
                damage: 100.0,
            },
            TorpedoSectionPartOf(bay),
            DestroySound(Some(AssetRef::from("base/sounds/torpedo_detonate.wav"))),
        ));
        app.update();

        let mut q_blast = app.world_mut().query_filtered::<(), With<NovaBlast>>();
        assert_eq!(
            q_blast.iter(app.world()).count(),
            1,
            "the armed torpedo on its aim point fuzed"
        );
        assert_eq!(
            app.world().resource::<LastPlayed>().0,
            Some(warhead),
            "the blast must play the warhead the torpedo carried"
        );
    }
}
