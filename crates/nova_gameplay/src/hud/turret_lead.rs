//! Turret lead/intercept pips: one small screen-projected pip per player
//! turret, drawn at the turret's already-computed intercept point
//! (`TurretSectionAimPoint`) so the player can see the lead each turret is
//! taking (task 20260708-165701).
//!
//! A thin consumer of the [`screen_indicator`](super::screen_indicator)
//! widget with `Point` anchors: a reconcile system keeps one pip per turret
//! child of the player ship (turrets die mid-fight when their section is
//! destroyed), and a driver copies each turret's aim point into its pip's
//! anchor every frame. The layer itself is spawned/despawned with the player
//! ship by the hud/mod.rs observers, like the other HUD overlays.

use bevy::prelude::*;

use crate::prelude::*;

/// On-screen size (px) of a lead pip. Small on purpose: it marks a computed
/// point, not a silhouette, and must not read as a lock reticle.
const PIP_PX: f32 = 8.0;

/// Pip tint. Warm amber, distinct from the nav-cyan destination marker and
/// the untinted lock reticle.
const PIP_COLOR: Color = Color::srgba(1.0, 0.75, 0.2, 0.9);

pub mod prelude {
    pub use super::{
        turret_lead_hud, TurretLeadHudMarker, TurretLeadPipMarker, TurretLeadPipTurret,
        TurretLeadPlugin,
    };
}

/// Marker for the full-screen pip layer (the root the HUD setup spawns).
#[derive(Component, Debug, Clone, Reflect)]
pub struct TurretLeadHudMarker;

/// Marker for one lead pip node.
#[derive(Component, Debug, Clone, Reflect)]
pub struct TurretLeadPipMarker;

/// The turret section entity this pip renders the aim point of.
#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct TurretLeadPipTurret(pub Entity);

/// UI bundle for the pip layer. Pips are spawned under it by
/// [`sync_turret_pips`], one per player turret.
pub fn turret_lead_hud() -> impl Bundle {
    (
        Name::new("TurretLeadHUD"),
        TurretLeadHudMarker,
        screen_indicator_layer(),
    )
}

/// Bundle for a single pip: a fixed-size tinted square indicator anchored to
/// a world point the driver rewrites each frame.
fn turret_lead_pip(turret: Entity) -> impl Bundle {
    (
        Name::new("TurretLeadPip"),
        TurretLeadPipMarker,
        TurretLeadPipTurret(turret),
        screen_indicator(ScreenIndicatorConfig {
            anchor: None,
            size: ScreenIndicatorSize::Fixed(Vec2::splat(PIP_PX)),
            offset: Vec2::ZERO,
            offscreen: ScreenIndicatorOffscreen::Hide,
        }),
        BackgroundColor(PIP_COLOR),
    )
}

#[derive(Default)]
pub struct TurretLeadPlugin;

impl Plugin for TurretLeadPlugin {
    fn build(&self, app: &mut App) {
        debug!("TurretLeadPlugin: build");

        app.add_systems(
            Update,
            (sync_turret_pips, drive_pip_anchors)
                .chain()
                .in_set(super::NovaHudSystems),
        );
    }
}

/// Keep exactly one pip per turret child of the player ship. A reconcile
/// system rather than add/remove observers: turret sections can be destroyed
/// mid-fight and ships can gain their sections after the player marker, and
/// one idempotent pass covers every ordering without observer choreography.
#[allow(clippy::type_complexity)]
fn sync_turret_pips(
    mut commands: Commands,
    q_layer: Query<Entity, With<TurretLeadHudMarker>>,
    q_turrets: Query<(Entity, &ChildOf), With<TurretSectionMarker>>,
    q_pips: Query<(Entity, &TurretLeadPipTurret), With<TurretLeadPipMarker>>,
    q_player: Query<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
) {
    let Ok(layer) = q_layer.single() else {
        // No layer means no player HUD; the layer's despawn already removed
        // the pips with it.
        return;
    };
    let Ok(player) = q_player.single() else {
        // Player ship gone but the HUD teardown has not run yet; the layer
        // (and its pips) despawn with the removal observer.
        return;
    };

    // Despawn pips whose turret died or left the player ship.
    for (pip, turret) in &q_pips {
        let alive = q_turrets
            .get(**turret)
            .is_ok_and(|(_, ChildOf(parent))| *parent == player);
        if !alive {
            commands.entity(pip).despawn();
        }
    }

    // Spawn pips for player turrets that have none yet.
    for (turret, ChildOf(parent)) in &q_turrets {
        if *parent != player {
            continue;
        }
        let has_pip = q_pips.iter().any(|(_, pip_turret)| **pip_turret == turret);
        if !has_pip {
            commands.entity(layer).with_child(turret_lead_pip(turret));
        }
    }
}

/// Copy each turret's intercept point into its pip's anchor. An inactive
/// (disabled) turret clears the anchor explicitly: `update_turret_aim_point`
/// keeps computing aim points for inactive turrets, so the stale-looking
/// value must not be drawn.
fn drive_pip_anchors(
    mut q_pips: Query<
        (&TurretLeadPipTurret, &mut ScreenIndicatorAnchor),
        With<TurretLeadPipMarker>,
    >,
    q_turrets: Query<
        (&TurretSectionAimPoint, Has<SectionInactiveMarker>),
        With<TurretSectionMarker>,
    >,
) {
    for (turret, mut anchor) in &mut q_pips {
        let aim_point = match q_turrets.get(**turret) {
            Ok((_, true)) | Err(_) => None,
            Ok((aim_point, false)) => **aim_point,
        };
        **anchor = aim_point.map(ScreenIndicatorAnchorKind::Point);
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    fn spawn_player_with_turrets(world: &mut World, turret_count: usize) -> (Entity, Vec<Entity>) {
        let player = world
            .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker))
            .id();
        let turrets = (0..turret_count)
            .map(|_| {
                world
                    .spawn((TurretSectionMarker, TurretSectionAimPoint(None)))
                    .insert(ChildOf(player))
                    .id()
            })
            .collect();
        (player, turrets)
    }

    #[test]
    fn sync_spawns_one_pip_per_player_turret() {
        let mut world = World::new();
        world.spawn(turret_lead_hud());
        let (_, turrets) = spawn_player_with_turrets(&mut world, 2);

        world.run_system_once(sync_turret_pips).unwrap();

        let mut pip_turrets: Vec<Entity> = world
            .query_filtered::<&TurretLeadPipTurret, With<TurretLeadPipMarker>>()
            .iter(&world)
            .map(|pip_turret| **pip_turret)
            .collect();
        pip_turrets.sort();
        let mut expected = turrets.clone();
        expected.sort();
        assert_eq!(pip_turrets, expected);

        // Idempotent: a second pass adds nothing.
        world.run_system_once(sync_turret_pips).unwrap();
        assert_eq!(
            world
                .query_filtered::<(), With<TurretLeadPipMarker>>()
                .iter(&world)
                .count(),
            2
        );
    }

    #[test]
    fn sync_despawns_the_pip_of_a_dead_turret() {
        let mut world = World::new();
        world.spawn(turret_lead_hud());
        let (_, turrets) = spawn_player_with_turrets(&mut world, 2);
        world.run_system_once(sync_turret_pips).unwrap();

        world.despawn(turrets[0]);
        world.run_system_once(sync_turret_pips).unwrap();

        let pip_turrets: Vec<Entity> = world
            .query_filtered::<&TurretLeadPipTurret, With<TurretLeadPipMarker>>()
            .iter(&world)
            .map(|pip_turret| **pip_turret)
            .collect();
        assert_eq!(pip_turrets, vec![turrets[1]]);
    }

    #[test]
    fn sync_ignores_turrets_of_other_ships() {
        let mut world = World::new();
        world.spawn(turret_lead_hud());
        spawn_player_with_turrets(&mut world, 1);
        let enemy = world.spawn(SpaceshipRootMarker).id();
        world
            .spawn((TurretSectionMarker, TurretSectionAimPoint(None)))
            .insert(ChildOf(enemy));

        world.run_system_once(sync_turret_pips).unwrap();

        assert_eq!(
            world
                .query_filtered::<(), With<TurretLeadPipMarker>>()
                .iter(&world)
                .count(),
            1,
            "only the player turret gets a pip"
        );
    }

    #[test]
    fn driver_copies_aim_point_and_clears_for_inactive_turrets() {
        let mut world = World::new();
        world.spawn(turret_lead_hud());
        let (_, turrets) = spawn_player_with_turrets(&mut world, 1);
        let turret = turrets[0];
        world.run_system_once(sync_turret_pips).unwrap();
        let pip = world
            .query_filtered::<Entity, With<TurretLeadPipMarker>>()
            .iter(&world)
            .next()
            .expect("pip spawned");

        // Tracking: the aim point becomes the pip's Point anchor.
        let aim = Vec3::new(10.0, 5.0, -80.0);
        **world
            .entity_mut(turret)
            .get_mut::<TurretSectionAimPoint>()
            .unwrap() = Some(aim);
        world.run_system_once(drive_pip_anchors).unwrap();
        assert_eq!(
            **world.entity(pip).get::<ScreenIndicatorAnchor>().unwrap(),
            Some(ScreenIndicatorAnchorKind::Point(aim))
        );

        // Disabled turret: the (still computed) aim point must not be drawn.
        world.entity_mut(turret).insert(SectionInactiveMarker);
        world.run_system_once(drive_pip_anchors).unwrap();
        assert_eq!(
            **world.entity(pip).get::<ScreenIndicatorAnchor>().unwrap(),
            None
        );

        // No target: aim point None clears the anchor too.
        world.entity_mut(turret).remove::<SectionInactiveMarker>();
        **world
            .entity_mut(turret)
            .get_mut::<TurretSectionAimPoint>()
            .unwrap() = None;
        world.run_system_once(drive_pip_anchors).unwrap();
        assert_eq!(
            **world.entity(pip).get::<ScreenIndicatorAnchor>().unwrap(),
            None
        );
    }
}
