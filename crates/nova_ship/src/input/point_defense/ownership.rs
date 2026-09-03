//! Who holds each of the player's mounts: the ownership precedence, the aim
//! and trigger the Flight Computer drives a mount it holds with, and the thin
//! line that says so on screen.
//!
//! # The precedence, per mount
//!
//! 1. [`MountAuthority::PlayerLock`] - a combat lock (the CTRL targeting flow)
//!    is on a target, and every mount follows it.
//! 2. [`MountAuthority::PlayerManual`] - the weapons are RAISED (RMB) and the
//!    mounts follow the crosshair. THE SEAM: a future RMB combat mode is a
//!    refinement of this tier, not a fourth one.
//! 3. [`MountAuthority::FlightComputer`] - nobody is using the battery, so the
//!    computer works the idle mounts against inbound ordnance.
//! 4. [`MountAuthority::Cold`] - the computer is not allowed to (the verb is
//!    withheld) or not allowed YET (the release grace is still running).
//!
//! It is resolved per MOUNT even though today's two player tiers are ship-wide
//! signals, because that is the shape the seam needs: a mode that hands the
//! player one mount and leaves the rest to the computer changes
//! [`mount_authority`] and nothing else.
//!
//! # Why the computer may fire a COLD ship
//!
//! The weapons safety is the player's trigger discipline: SAFE unless raised or
//! locked. An idle battery is exactly the state this feature exists for, so a
//! mount the computer holds is exempt from it in the section fire path. The
//! exemption is narrow by construction - it needs both an assignment and the
//! computer tier, and the assignment can only ever be an inbound HOSTILE
//! torpedo.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_gameplay::prelude::*;

use super::{assignment::TurretDefenseTarget, mount_may_shoot};
use crate::prelude::*;

/// How long a mount stays the player's after the player stops using it, before
/// the computer may take it back.
///
/// The owner called it "like a debounce", and that is exactly what it is for.
/// The player's own gestures have GAPS in them: a tap-clear is a sub-
/// [`RADAR_TAP_SECS`] press on the way to the next lock, and RMB comes up
/// between bursts. Without a grace the battery would swing away from the
/// crosshair and back inside one gesture, which is the swinging-and-hitting-
/// nothing that the assignment's own dwell exists to prevent - and it would do
/// it while the player was mid-aim, so it would read as the ship fighting the
/// helm.
///
/// Two tap windows. Long enough to cover a release-and-re-press; short against
/// the thing being defended against, which crosses ~3% of the 150 u point-
/// defence envelope in that time at torpedo speeds.
pub const POINT_DEFENSE_REGRASP_SECS: f32 = 2.0 * RADAR_TAP_SECS;

/// Which authority holds one mount this frame, highest first.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
pub enum MountAuthority {
    /// A combat lock holds it: the CTRL targeting flow chose the target and
    /// every mount follows it, raised or not (the LOCK-WINS routing).
    PlayerLock,
    /// Manual gunnery holds it: the weapons are raised and the mount follows
    /// the crosshair.
    ///
    /// THE SEAM. A future RMB combat mode - per-mount manual control, a
    /// gunnery sub-mode, anything the player drives a single mount with -
    /// plugs in HERE, as a refinement of this tier inside [`mount_authority`].
    /// It does not need a new tier, because "the player is working this mount
    /// by hand" is already what this one means.
    PlayerManual,
    /// The Flight Computer holds it: the battery is idle and the computer's
    /// point defence may assign and fire it.
    FlightComputer,
    /// Nobody holds it. The default, because a mount that has never been
    /// resolved must not read as claimed.
    #[default]
    Cold,
}

impl MountAuthority {
    /// Whether the PLAYER holds the mount (either of the two top tiers).
    pub fn is_player(self) -> bool {
        matches!(self, Self::PlayerLock | Self::PlayerManual)
    }
}

/// Per-mount point-defence ownership on the PLAYER's hull. A turret without one
/// belongs to whoever flies its ship outright (an AI hull's whole battery).
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[reflect(Component)]
pub struct PointDefenseMount {
    /// Who held the mount at the last resolve.
    pub authority: MountAuthority,
    /// Seconds left of [`POINT_DEFENSE_REGRASP_SECS`] before the computer may
    /// take the mount back. Reset to the full grace every frame the player
    /// holds it, so the clock starts at the RELEASE.
    pub regrasp: f32,
    /// Whether the COMPUTER is the one holding this mount's trigger down.
    ///
    /// The fire inputs are latched bools, so without this the computer's held
    /// trigger would survive into the player's hands the instant the player
    /// raised - a mount firing that nobody pressed. It is the same
    /// fresh-press discipline the weapons-safety trigger-interrupt enforces.
    pub firing: bool,
}

impl PointDefenseMount {
    /// Whether the computer is allowed to work this mount.
    pub fn computer_owns(&self) -> bool {
        self.authority == MountAuthority::FlightComputer
    }
}

/// Whether the computer is ACTUALLY working this mount: it owns it and has
/// something for it to shoot.
///
/// The pair, not the tier alone. An owned mount with nothing inbound is left to
/// the player's own aim feed, so an idle battery keeps tracking the crosshair
/// exactly as it always has, and the change is invisible until a torpedo turns
/// up.
pub fn flight_computer_works(
    mount: Option<&PointDefenseMount>,
    assignment: Option<&TurretDefenseTarget>,
) -> bool {
    mount.is_some_and(PointDefenseMount::computer_owns) && assignment.is_some_and(|a| a.is_some())
}

/// Whether a live controller section on `ship` grants point defence.
///
/// FAIL-OPEN on a hull with no controller section at all, unlike
/// `ship_grants_lock`: those are the bare rigs the examples and the section
/// ranges fly, and a range whose guns silently stood down would be a worse
/// failure than one that defends itself. Only an explicit `DisableVerb` /
/// `SetControllerVerb` takes the capability away.
fn ship_grants_point_defense(
    ship: Entity,
    q_controllers: &Query<
        (&ChildOf, Option<&WithheldVerbs>),
        (
            With<ControllerSectionMarker>,
            Without<SectionInactiveMarker>,
        ),
    >,
) -> bool {
    let mut computers = 0usize;
    for (ChildOf(parent), withheld) in q_controllers {
        if *parent != ship {
            continue;
        }
        computers += 1;
        if withheld.is_none_or(|w| w.granted(FlightVerb::PointDefense)) {
            return true;
        }
    }
    computers == 0
}

/// The precedence itself, as one pure function - the whole rule in one place,
/// and the one place a future combat mode edits.
fn mount_authority(locked: bool, raised: bool, granted: bool, regrasp: f32) -> MountAuthority {
    if locked {
        MountAuthority::PlayerLock
    } else if raised {
        MountAuthority::PlayerManual
    } else if granted && regrasp <= 0.0 {
        MountAuthority::FlightComputer
    } else {
        MountAuthority::Cold
    }
}

/// Give every turret on the player's hull its ownership slot.
///
/// Same reasoning as the defense slot beside it: a turret's ship is only
/// knowable once it is parented, so this is a system rather than a `#[require]`.
pub(super) fn insert_point_defense_mount(
    mut commands: Commands,
    q_turret: Query<(Entity, &ChildOf), (With<TurretSectionMarker>, Without<PointDefenseMount>)>,
    q_ship: Query<(), (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
) {
    for (turret, ChildOf(ship)) in &q_turret {
        if q_ship.contains(*ship) {
            commands.entity(turret).insert(PointDefenseMount::default());
        }
    }
}

/// Resolve the precedence on every one of the player's mounts, and run the
/// release grace.
///
/// Runs BEFORE the assignment pass: a mount the player just took must be out of
/// the computer's pool the same frame, or the claim it still holds keeps a
/// torpedo away from a mount that could work it.
#[expect(
    clippy::type_complexity,
    reason = "one query term per precedence input"
)]
pub(super) fn update_point_defense_ownership(
    time: Res<Time>,
    q_ship: Query<
        (Entity, &CombatLock, Option<&WeaponsRaised>),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    q_controllers: Query<
        (&ChildOf, Option<&WithheldVerbs>),
        (
            With<ControllerSectionMarker>,
            Without<SectionInactiveMarker>,
        ),
    >,
    mut q_turret: Query<(Entity, &ChildOf, &mut PointDefenseMount), With<TurretSectionMarker>>,
) {
    let delta = time.delta_secs();
    for (turret, ChildOf(ship), mut mount) in &mut q_turret {
        let Ok((ship, lock, raised)) = q_ship.get(*ship) else {
            continue;
        };
        let locked = lock.0.is_some();
        let raised = raised.is_some_and(|raised| raised.0);

        // The grace clock: held by the player = full, otherwise counting down.
        // Reading it from the PLAYER's two tiers rather than from the resolved
        // authority is what makes it a release timer - a mount sitting in
        // `Cold` because the verb is withheld is not serving a grace it would
        // have to serve again later.
        let next = if locked || raised {
            mount.regrasp = POINT_DEFENSE_REGRASP_SECS;
            mount_authority(locked, raised, false, mount.regrasp)
        } else {
            mount.regrasp = (mount.regrasp - delta).max(0.0);
            let granted = ship_grants_point_defense(ship, &q_controllers);
            mount_authority(false, false, granted, mount.regrasp)
        };

        if mount.authority != next {
            debug!("update_point_defense_ownership: mount {turret:?} -> {next:?}");
        }
        mount.authority = next;
    }
}

/// Aim every mount the computer is working at the torpedo it was assigned.
///
/// The player's own turret feed skips these mounts, so this is the only writer
/// while the computer holds one; both write the same pair of section inputs, so
/// the lead solve, the hinge pass and the fire gate downstream cannot tell the
/// difference between a computer-aimed mount and a player-aimed one.
pub(super) fn update_point_defense_aim(
    mut q_turret: Query<(
        &PointDefenseMount,
        &TurretDefenseTarget,
        &mut TurretSectionTargetInput,
        &mut TurretSectionTargetVelocity,
        &mut TurretSectionTargetEntity,
    )>,
    q_torpedo: Query<(&Transform, Option<&LinearVelocity>)>,
) {
    for (mount, assignment, mut target, mut velocity, mut tracked) in &mut q_turret {
        if !flight_computer_works(Some(mount), Some(assignment)) {
            continue;
        }
        // `flight_computer_works` proved the assignment is Some; a torpedo that
        // died between the assignment pass and here simply leaves the mount on
        // its last aim for one frame, and the next pass clears the claim.
        let Some(torpedo) = **assignment else {
            continue;
        };
        let Ok((transform, torpedo_velocity)) = q_torpedo.get(torpedo) else {
            continue;
        };
        **target = Some(transform.translation);
        **velocity = torpedo_velocity.map(|v| **v).unwrap_or(Vec3::ZERO);
        // Names the body the velocity belongs to, so the mount's track starts
        // over when the assignment moves to a different torpedo.
        **tracked = Some(torpedo);
    }
}

/// Hold the trigger of every mount the computer is working, once its barrel
/// bears.
///
/// No burst cadence and no line-of-fire ray, matching the AI's own point-
/// defence arm: bursts are a discipline for shooting at ships, and inbound
/// ordnance is the one case where a wasted round beats a held trigger. The two
/// gates it DOES apply are [`mount_may_shoot`](super::mount_may_shoot) - the
/// same range gate and the same 0.92 deg bearing cone the AI trigger uses, not
/// a second looser number.
pub(super) fn update_point_defense_trigger(
    mut q_turret: Query<(
        &mut PointDefenseMount,
        &TurretDefenseTarget,
        &TurretSectionMuzzleEntity,
        &TurretSectionAimPoint,
        &TurretEngineFigures,
        &mut TurretSectionInput,
    )>,
    q_muzzle: Query<&GlobalTransform, With<TurretSectionBarrelMuzzleMarker>>,
    q_torpedo: Query<(&Transform, Option<&ComputedCenterOfMass>)>,
) {
    for (mut mount, assignment, muzzle, aim_point, figures, mut input) in &mut q_turret {
        // Not ours: release only what WE are holding down. Stomping the input
        // unconditionally would fight the player's own trigger every frame.
        if !flight_computer_works(Some(&mount), Some(assignment)) {
            if mount.firing {
                **input = false;
                mount.firing = false;
            }
            continue;
        }

        let anchor = (**assignment)
            .and_then(|torpedo| q_torpedo.get(torpedo).ok())
            .map(|(transform, com)| live_structure_anchor(transform, com));
        let muzzle_transform = q_muzzle.get(**muzzle).ok();
        let shoot = match (anchor, muzzle_transform) {
            // Align against the LEADED aim point the turret actually steers to,
            // falling back to the raw anchor before the lead resolves - a mount
            // correctly leading a crosser never bears on the anchor itself.
            (Some(anchor), Some(muzzle_transform)) => mount_may_shoot(
                muzzle_transform,
                figures,
                anchor,
                aim_point.unwrap_or(anchor),
            ),
            _ => false,
        };

        if **input != shoot {
            **input = shoot;
        }
        mount.firing = shoot;
    }
}

/// The gizmo group the point-defence lines draw in: their own, so the width
/// below is theirs alone and does not thin the juice flashes.
#[derive(Clone, Default, Reflect, GizmoConfigGroup)]
pub struct PointDefenseGizmos;

/// Line width (px) of a point-defence line. Deliberately under the 2.0 px gizmo
/// default: this is an ambient readout of what the ship is doing on its own,
/// not a HUD element, and it is drawn once per engaged mount - a full battery
/// answering a salvo must still read as a fight rather than as a diagram.
const POINT_DEFENSE_LINE_WIDTH: f32 = 0.75;

/// Colour of a point-defence line: the HUD's amber gunnery family (the turret
/// lead pip's `PIP_COLOR`), at the alpha that keeps it ambient.
const POINT_DEFENSE_LINE_COLOR: Color = Color::srgba(1.0, 0.72, 0.28, 0.35);

/// Draw one thin line from each mount the COMPUTER is working to the torpedo it
/// is working, so an autonomous battery is legible: which mounts the ship took,
/// and what each one picked.
///
/// Only those mounts. A player-held mount is the player's aim and already has a
/// crosshair, a lead pip and an inset; drawing a second line to it would say
/// the ship was doing something it is not.
pub(super) fn draw_point_defense_lines(
    mut gizmos: Gizmos<PointDefenseGizmos>,
    q_turret: Query<(
        &PointDefenseMount,
        &TurretDefenseTarget,
        &TurretSectionMuzzleEntity,
    )>,
    q_muzzle: Query<&GlobalTransform, With<TurretSectionBarrelMuzzleMarker>>,
    q_torpedo: Query<&GlobalTransform, With<TorpedoProjectileMarker>>,
) {
    for (mount, assignment, muzzle) in &q_turret {
        if !flight_computer_works(Some(mount), Some(assignment)) {
            continue;
        }
        let (Some(torpedo), Ok(muzzle)) = (**assignment, q_muzzle.get(**muzzle)) else {
            continue;
        };
        let Ok(target) = q_torpedo.get(torpedo) else {
            continue;
        };
        gizmos.line(
            muzzle.translation(),
            target.translation(),
            POINT_DEFENSE_LINE_COLOR,
        );
    }
}

/// The gizmo group's config: the thin line the owner asked for.
pub(super) fn point_defense_gizmo_config() -> GizmoConfig {
    GizmoConfig {
        line: GizmoLineConfig {
            width: POINT_DEFENSE_LINE_WIDTH,
            ..default()
        },
        ..default()
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use bevy::{ecs::system::RunSystemOnce, time::TimeUpdateStrategy};

    use super::*;

    /// A player hull with a live flight computer and one turret. Every
    /// component production inserts on the path under test is here: the
    /// targeting state the precedence reads, the ownership slot, and the
    /// defense slot the assignment writes.
    fn player_hull(app: &mut App) -> (Entity, Entity, Entity) {
        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::default(),
                CombatLock(None),
                WeaponsRaised(false),
            ))
            .id();
        let computer = app
            .world_mut()
            .spawn((ControllerSectionMarker, ChildOf(ship)))
            .id();
        let turret = app
            .world_mut()
            .spawn((
                TurretSectionMarker,
                ChildOf(ship),
                PointDefenseMount::default(),
                TurretDefenseTarget::default(),
            ))
            .id();
        (ship, computer, turret)
    }

    /// A one-system app on a fixed manual clock. `Time` is what the release
    /// grace runs on, so the harness has to be a real app rather than a bare
    /// world.
    fn ownership_app(step: f32) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            step,
        )));
        app.add_systems(Update, update_point_defense_ownership);
        app
    }

    fn authority(app: &App, turret: Entity) -> MountAuthority {
        app.world()
            .get::<PointDefenseMount>(turret)
            .expect("the mount keeps its ownership slot")
            .authority
    }

    #[test]
    fn an_idle_battery_is_claimed_by_the_computer() {
        let mut app = ownership_app(1.0 / 60.0);
        let (_, _, turret) = player_hull(&mut app);

        // The first update lands a zero delta; the second advances a frame.
        app.update();
        app.update();

        assert_eq!(
            authority(&app, turret),
            MountAuthority::FlightComputer,
            "no lock and no raise is the state the computer exists for"
        );
    }

    #[test]
    fn a_withheld_verb_means_no_claim() {
        let mut app = ownership_app(1.0 / 60.0);
        let (_, computer, turret) = player_hull(&mut app);
        app.world_mut()
            .entity_mut(computer)
            .insert(WithheldVerbs::default());

        app.update();
        app.update();
        assert_eq!(
            authority(&app, turret),
            MountAuthority::FlightComputer,
            "an empty withheld set grants every verb"
        );

        // The scenario lever, mid-run: SetControllerVerb writes exactly this.
        app.world_mut()
            .get_mut::<WithheldVerbs>(computer)
            .unwrap()
            .withhold(FlightVerb::PointDefense);
        app.update();
        assert_eq!(
            authority(&app, turret),
            MountAuthority::Cold,
            "a computer that does not grant point defence claims nothing"
        );

        // And back again, without a respawn: the flip works both ways.
        app.world_mut()
            .get_mut::<WithheldVerbs>(computer)
            .unwrap()
            .grant(FlightVerb::PointDefense);
        app.update();
        assert_eq!(authority(&app, turret), MountAuthority::FlightComputer);
    }

    #[test]
    fn a_dead_computer_does_not_take_point_defence_with_it() {
        // Fail-open: only an explicit withhold takes the capability away, so
        // the bare rigs the section ranges fly still defend themselves.
        let mut app = ownership_app(1.0 / 60.0);
        let (_, computer, turret) = player_hull(&mut app);
        app.world_mut().despawn(computer);

        app.update();
        app.update();

        assert_eq!(authority(&app, turret), MountAuthority::FlightComputer);
    }

    #[test]
    fn a_player_lock_takes_the_mount_instantly_and_the_grace_gives_it_back() {
        let mut app = ownership_app(1.0 / 60.0);
        let (ship, _, turret) = player_hull(&mut app);
        let prey = app.world_mut().spawn_empty().id();

        app.update();
        app.update();
        assert_eq!(authority(&app, turret), MountAuthority::FlightComputer);

        // The CTRL flow lands a combat lock: one frame, no grace.
        app.world_mut().get_mut::<CombatLock>(ship).unwrap().0 = Some(prey);
        app.update();
        assert_eq!(authority(&app, turret), MountAuthority::PlayerLock);

        // Tap-clear. The mount is the player's for the whole grace...
        app.world_mut().get_mut::<CombatLock>(ship).unwrap().0 = None;
        app.update();
        assert_eq!(
            authority(&app, turret),
            MountAuthority::Cold,
            "the release starts the debounce, it does not end it"
        );

        // ...and only then the computer's again.
        let frames = (POINT_DEFENSE_REGRASP_SECS * 60.0).ceil() as usize;
        for _ in 0..frames {
            app.update();
        }
        assert_eq!(authority(&app, turret), MountAuthority::FlightComputer);
    }

    #[test]
    fn the_raised_stance_takes_the_mount_too() {
        // The manual tier, and the seam a future combat mode refines.
        let mut app = ownership_app(1.0 / 60.0);
        let (ship, _, turret) = player_hull(&mut app);

        app.update();
        app.update();
        assert_eq!(authority(&app, turret), MountAuthority::FlightComputer);

        app.world_mut().get_mut::<WeaponsRaised>(ship).unwrap().0 = true;
        app.update();
        assert_eq!(authority(&app, turret), MountAuthority::PlayerManual);

        // A lock while RAISED is still the lock's mount: the tiers are ordered,
        // not exclusive (LOCK-WINS, the same routing the aim feed follows).
        let prey = app.world_mut().spawn_empty().id();
        app.world_mut().get_mut::<CombatLock>(ship).unwrap().0 = Some(prey);
        app.update();
        assert_eq!(authority(&app, turret), MountAuthority::PlayerLock);
    }

    #[test]
    fn the_grace_does_not_run_while_the_player_still_holds_the_mount() {
        // The clock starts at the RELEASE. A grace that ticked down under a
        // held lock would hand the battery back mid-engagement.
        let mut app = ownership_app(POINT_DEFENSE_REGRASP_SECS);
        let (ship, _, turret) = player_hull(&mut app);
        let prey = app.world_mut().spawn_empty().id();

        app.update();
        app.world_mut().get_mut::<CombatLock>(ship).unwrap().0 = Some(prey);
        for _ in 0..4 {
            app.update();
        }
        assert_eq!(authority(&app, turret), MountAuthority::PlayerLock);

        app.world_mut().get_mut::<CombatLock>(ship).unwrap().0 = None;
        app.update();
        assert_eq!(
            authority(&app, turret),
            MountAuthority::Cold,
            "the frame of the release still belongs to the player"
        );
        app.update();
        assert_eq!(authority(&app, turret), MountAuthority::FlightComputer);
    }

    #[test]
    fn a_released_mount_drops_the_trigger_the_computer_was_holding() {
        // The latched-bool hazard: the fire inputs are held bools, so a
        // computer trigger left down would fire in the player's hands the
        // instant they raised - a shot nobody pressed.
        let mut world = World::new();
        let turret = world
            .spawn((
                PointDefenseMount {
                    authority: MountAuthority::PlayerManual,
                    regrasp: POINT_DEFENSE_REGRASP_SECS,
                    firing: true,
                },
                TurretDefenseTarget::default(),
                TurretSectionMuzzleEntity(Entity::PLACEHOLDER),
                TurretSectionAimPoint(None),
                TurretSectionConfigHelper(TurretSectionConfig::default()),
                TurretSectionInput(true),
            ))
            .id();

        world.run_system_once(update_point_defense_trigger).unwrap();

        assert!(
            !**world.get::<TurretSectionInput>(turret).unwrap(),
            "the computer releases what it was holding when it loses the mount"
        );
        assert!(!world.get::<PointDefenseMount>(turret).unwrap().firing);
    }

    #[test]
    fn the_computer_never_stomps_the_players_own_trigger() {
        // The other side of it: a mount the player is firing must not be
        // released by a pass that does not own it.
        let mut world = World::new();
        let turret = world
            .spawn((
                PointDefenseMount {
                    authority: MountAuthority::PlayerLock,
                    regrasp: POINT_DEFENSE_REGRASP_SECS,
                    firing: false,
                },
                TurretDefenseTarget::default(),
                TurretSectionMuzzleEntity(Entity::PLACEHOLDER),
                TurretSectionAimPoint(None),
                TurretSectionConfigHelper(TurretSectionConfig::default()),
                TurretSectionInput(true),
            ))
            .id();

        world.run_system_once(update_point_defense_trigger).unwrap();

        assert!(**world.get::<TurretSectionInput>(turret).unwrap());
    }

    #[test]
    fn a_held_mount_is_aimed_at_its_torpedo_with_its_lead_velocity() {
        let mut world = World::new();
        let velocity = Vec3::new(0.0, 0.0, 30.0);
        let torpedo = world
            .spawn((
                TorpedoProjectileMarker,
                Transform::from_translation(Vec3::new(0.0, 10.0, -60.0)),
                LinearVelocity(velocity),
            ))
            .id();
        let turret = world
            .spawn((
                PointDefenseMount {
                    authority: MountAuthority::FlightComputer,
                    ..default()
                },
                TurretDefenseTarget(Some(torpedo)),
                TurretSectionTargetInput(None),
                TurretSectionTargetVelocity(Vec3::ZERO),
            ))
            .id();

        world.run_system_once(update_point_defense_aim).unwrap();

        assert_eq!(
            **world.get::<TurretSectionTargetInput>(turret).unwrap(),
            Some(Vec3::new(0.0, 10.0, -60.0))
        );
        assert_eq!(
            **world.get::<TurretSectionTargetVelocity>(turret).unwrap(),
            velocity,
            "the lead solve needs the torpedo's velocity, like the AI feed"
        );
        assert_eq!(
            **world.get::<TurretSectionTargetEntity>(turret).unwrap(),
            Some(torpedo),
            "and the body it belongs to, so the mount's track restarts when \
             the assignment moves"
        );
    }

    #[test]
    fn an_owned_mount_with_nothing_inbound_is_left_to_the_player() {
        // The pair rule: ownership alone is not enough. An idle battery with
        // no threat keeps following the crosshair, so the feature is invisible
        // until a torpedo turns up.
        let mount = PointDefenseMount {
            authority: MountAuthority::FlightComputer,
            ..default()
        };
        assert!(!flight_computer_works(
            Some(&mount),
            Some(&TurretDefenseTarget(None))
        ));
        assert!(flight_computer_works(
            Some(&mount),
            Some(&TurretDefenseTarget(Some(Entity::PLACEHOLDER)))
        ));
    }
}
