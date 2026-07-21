//! Turret lead/intercept pips: one small screen-projected pip per player
//! turret, drawn at the turret's already-computed intercept point
//! (`TurretSectionAimPoint`) so the player can see the lead each turret is
//! taking (task 20260708-165701).
//!
//! A thin consumer of the [`screen_indicator`](mod@super::screen_indicator)
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

/// Hot-shifted pip tint (Q5a of spike 20260713-110039): while the player's
/// weapons are HOT the aim pips go lock-red - raised-manual gunnery has no
/// lock crosshair or inset on screen, so the pip the player is aiming with
/// must carry the state (adversarial F4). Ticks would be noise at 8 px;
/// color-only is the deliberate exception to shape+color here.
const PIP_HOT_COLOR: Color = Color::srgba(1.0, 0.4, 0.3, 0.95);

/// UI bundle for the pip layer. Pips are spawned under it by
/// `sync_turret_pips`, one per player turret.
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

/// Keeps one lead pip per player turret at its published intercept point,
/// hot-shifting them to lock-red while the weapons are hot.
/// Runs `sync_turret_pips`, `drive_pip_anchors` and `drive_pip_hot_tint`
/// (chained) in PostUpdate after `TurretSectionAimSystems` and before
/// `ScreenIndicatorSystems`.
#[derive(Default)]
pub struct TurretLeadPlugin;

impl Plugin for TurretLeadPlugin {
    fn build(&self, app: &mut App) {
        debug!("TurretLeadPlugin: build");

        // The pips consume THIS frame's intercept: after the PostUpdate aim
        // chain publishes it, before the indicator projection places the
        // nodes (task 20260710-231929 - in Update the pip was always one
        // frame behind the solution, jittering against a moving target).
        app.add_systems(
            PostUpdate,
            (sync_turret_pips, drive_pip_anchors, drive_pip_hot_tint)
                .chain()
                .after(TurretSectionAimSystems)
                .before(ScreenIndicatorSystems),
        );
    }
}

/// Shift the pips to the hot tint while the player's weapons are HOT (F4:
/// the manual-gunnery hot cue).
fn drive_pip_hot_tint(
    q_player: Query<&WeaponsHot, With<PlayerSpaceshipMarker>>,
    mut q_pips: Query<&mut BackgroundColor, With<TurretLeadPipMarker>>,
) {
    let hot = q_player.iter().next().is_some_and(|hot| hot.0);
    let color = if hot { PIP_HOT_COLOR } else { PIP_COLOR };
    for mut pip in &mut q_pips {
        if pip.0 != color {
            pip.0 = color;
        }
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

    /// The pip must mark THIS frame's intercept (task 20260710-231929). The
    /// aim chain publishes in PostUpdate; before the fix the pips consumed
    /// it from Update - always one frame behind, so the crosshair jittered
    /// against any moving target by one frame of intercept motion. The rig
    /// uses the REAL TurretLeadPlugin wiring plus the aim system registered
    /// under its production set, drives a target across the sky at 60 u/s,
    /// and demands the pip anchor equal the same frame's freshly published
    /// aim point, every frame.
    #[test]
    fn pip_anchor_carries_the_same_frame_intercept() {
        use core::time::Duration;

        use bevy::time::TimeUpdateStrategy;

        use crate::sections::turret_section::update_turret_aim_point;

        #[derive(Component)]
        struct SkyTarget;

        fn move_target(time: Res<Time>, mut q_target: Query<&mut Transform, With<SkyTarget>>) {
            for mut transform in &mut q_target {
                transform.translation.x += 60.0 * time.delta_secs();
            }
        }

        fn feed_target_input(
            q_target: Query<&Transform, With<SkyTarget>>,
            mut q_turret: Query<&mut TurretSectionTargetInput>,
        ) {
            let Ok(target) = q_target.single() else {
                return;
            };
            for mut input in &mut q_turret {
                **input = Some(target.translation);
            }
        }

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, TransformPlugin, TurretLeadPlugin));
        app.add_plugins(crate::hud::screen_indicator::ScreenIndicatorPlugin);
        // The aim system under its production set (the full section plugin
        // drags render-material plugins into a headless test).
        app.add_systems(
            PostUpdate,
            update_turret_aim_point.in_set(TurretSectionAimSystems),
        );
        app.add_systems(Update, (move_target, feed_target_input).chain());
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            1.0 / 60.0,
        )));

        app.world_mut()
            .spawn(SkyTarget)
            .insert(Transform::from_xyz(0.0, 0.0, -80.0));
        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::IDENTITY,
            ))
            .id();
        let turret = app
            .world_mut()
            .spawn((
                TurretSectionMarker,
                ChildOf(ship),
                Transform::IDENTITY,
                TurretSectionTargetInput(None),
                TurretSectionTargetVelocity(Vec3::X * 60.0),
                TurretSectionConfigHelper(TurretSectionConfig::default()),
                TurretSectionAimPoint(None),
            ))
            .id();
        let muzzle = app
            .world_mut()
            .spawn((
                TurretSectionBarrelMuzzleMarker,
                ChildOf(turret),
                Transform::IDENTITY,
            ))
            .id();
        app.world_mut()
            .entity_mut(turret)
            .insert(TurretSectionMuzzleEntity(muzzle));
        app.world_mut()
            .spawn((TurretLeadHudMarker, Node::default()));

        // Warmup: the reconcile system spawns the pip via commands.
        app.update();
        app.update();

        let mut previous_aim: Option<Vec3> = None;
        for _ in 0..30 {
            app.update();
            let world = app.world_mut();
            let aim: Vec3 = (**world.entity(turret).get::<TurretSectionAimPoint>().unwrap())
                .expect("a fed target yields an aim point");
            let (_, anchor) = world
                .query_filtered::<(&TurretLeadPipMarker, &ScreenIndicatorAnchor), ()>()
                .single(world)
                .expect("the player turret has exactly one pip");
            // Delivery guard: the intercept must MOVE frame to frame, or
            // same-frame and one-frame-stale anchors are indistinguishable.
            if let Some(previous) = previous_aim {
                assert!(
                    (aim - previous).length() > 0.1,
                    "the moving target must move the intercept"
                );
            }
            previous_aim = Some(aim);
            assert_eq!(
                **anchor,
                Some(ScreenIndicatorAnchorKind::Point(aim)),
                "the pip must carry the SAME frame's intercept"
            );
        }
    }
}
