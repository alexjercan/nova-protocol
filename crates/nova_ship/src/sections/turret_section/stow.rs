//! The retractable mount: a turret that sinks into its housing when the
//! fight is over and rises before the next one.
//!
//! Sequencing lives HERE, not in track data. The section's authored
//! [`SectionAnimations`] tracks own only what the aim stack does not - the
//! [`SectionAnimationCue::StowLift`] elevator and the
//! [`SectionAnimationCue::StowDoors`] lids - while the barrel-up attitude is
//! a commanded aim through the existing look controllers
//! ([`SmoothLookRotation`] REPLACES a joint's rotation every tick, so a
//! rotation track composed onto an aimed joint would fight it). The state
//! machine below steers the cues in order and advances phases by reading
//! `cue_progress`, the same trick as the bay's door-gated ejection.

use bevy::prelude::*;

use super::*;

/// How long a deployed mount must sit with weapons cold and nothing tracked
/// before it stows. Deploy is fast and stow is lazy on purpose: a lull in a
/// fight must not fold the battery mid-engagement.
const STOW_SETTLE_SECONDS: f32 = 4.0;

/// How close (radians) every hinge must be to its stow angle before the
/// assembly may sink - about a degree, the visible end of the fold-up.
const STOW_AIM_SETTLE_RAD: f32 = 0.02;

/// Where a turret is in its stow cycle. Only [`Self::Deployed`] tracks and
/// fires; the travel through the other three phases is the deploy delay the
/// design prices as a real combat cost.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub enum TurretStowPhase {
    /// Up, free to track and fire.
    Deployed,
    /// Folding: barrel up, then sink, then shut the lids.
    Stowing,
    /// Down behind shut lids. The rest state of a quiet ship.
    Stowed,
    /// Rising: part the lids, then raise, then hand the joints back to the
    /// aim solver.
    Deploying,
}

/// The housing lids were just commanded to move. The seam the audio half hangs
/// the servo cue on, so the state machine itself stays headless - the same
/// shape [`RailgunFired`](crate::sections::railgun_section::RailgunFired) uses.
///
/// Fired when the LIDS move, not when the phase flips: the fold-away spends
/// most of a second folding and sinking before the doors do anything, so a cue
/// at the phase edge would play to a still housing.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct TurretStowDoorsMoved {
    /// The turret whose housing it is.
    pub entity: Entity,
    /// True when the lids are parting (deploying), false when they are shutting.
    pub opening: bool,
}

/// The stow state machine of one retractable turret. Inserted by
/// [`insert_turret_stow`] on every LIVE turret whose section authors a
/// [`SectionAnimationCue::StowLift`] track (editor previews carry no
/// [`TurretSectionInput`] and keep showing the deployed gun); a turret
/// without the track never stows and behaves exactly as before.
#[derive(Component, Clone, Copy, Debug, Reflect)]
pub struct TurretStow {
    phase: TurretStowPhase,
    /// Seconds the deployed mount has been quiet (cold and untracked).
    quiet_secs: f32,
    /// Whether this cycle has already reported its lids moving, so the report
    /// is an EDGE and not a per-frame repeat while they travel.
    doors_reported: bool,
}

impl TurretStow {
    /// A machine resting in `phase`. `pub(super)` for the armer and the
    /// sibling gate tests; live phases only ever come from the driver.
    pub(super) fn new(phase: TurretStowPhase) -> Self {
        Self {
            phase,
            quiet_secs: 0.0,
            doors_reported: false,
        }
    }

    /// Where the mount is in its stow cycle.
    pub fn phase(&self) -> TurretStowPhase {
        self.phase
    }

    /// True when the mount may track and fire. The single gate the aim and
    /// fire paths read: anything short of fully deployed holds both.
    pub fn is_deployed(&self) -> bool {
        self.phase == TurretStowPhase::Deployed
    }
}

/// Arm the stow machine on every live retractable turret, STOWED. Scenes
/// start stowed by SNAP - cues landed at 1 and hinges landed on the stow
/// attitude - so a cold start shows a shut housing and no fold-down wiggle;
/// the rig writes the landed pose the moment its scenes resolve. Rest pose
/// in the art stays deployed, which is what animation-less apps show.
pub(super) fn insert_turret_stow(
    mut commands: Commands,
    mut q_turret: Query<
        (Entity, &mut SectionAnimations),
        (
            With<TurretSectionMarker>,
            With<TurretSectionInput>,
            Without<TurretStow>,
            Without<SectionInactiveMarker>,
        ),
    >,
    mut q_joint: Query<(
        &TurretSectionPartOf,
        &SmoothLookRotation,
        &mut SmoothLookRotationTarget,
        &mut SmoothLookRotationOutput,
    )>,
) {
    for (turret, mut animations) in &mut q_turret {
        if !animations.has_cue(SectionAnimationCue::StowLift) {
            continue;
        }
        animations.snap_cue(SectionAnimationCue::StowLift, 1.0);
        animations.snap_cue(SectionAnimationCue::StowDoors, 1.0);
        for (part_of, look, mut target, mut output) in &mut q_joint {
            if **part_of != turret {
                continue;
            }
            let angle = stow_angle(look);
            **target = angle;
            **output = angle;
        }
        // The lift joint is code-built, so no scene ready ever marks it: the
        // armer queues the rig resolve that catches it (and any lid nodes
        // whose scenes readied before the machine armed).
        commands.entity(turret).insert((
            TurretStow::new(TurretStowPhase::Stowed),
            SectionAnimationRigDirty,
        ));
    }
}

/// A hinge's stow angle: a clamped hinge rides its TOP stop (the shipped
/// pitch's straight-up), a free hinge returns home. That is the "barrel up"
/// attitude for every shipped mount, derived rather than authored so the
/// twin and the gatling need no extra data.
fn stow_angle(look: &SmoothLookRotation) -> f32 {
    look.max.unwrap_or(look.initial)
}

/// Drive every stow machine one step: read the deploy demand, steer the
/// hinges and the two cues in sequence, and advance the phase on what the
/// tracks report back.
///
/// Deploy demand is weapons hot OR a live tracked body OR a Flight Computer
/// point-defense assignment - the mount comes up autonomously against an
/// inbound, before the safety exemption in the fire path needs it. An
/// UNMANAGED ship (no [`WeaponsHot`]) reads as hot, the same fail-open as
/// the fire path, so bare rigs and example ranges deploy at spawn and never
/// stow. Stow demand is the opposite of all three, held for
/// [`STOW_SETTLE_SECONDS`].
pub(super) fn drive_turret_stow(
    mut commands: Commands,
    time: Res<Time>,
    mut q_turret: Query<
        (
            Entity,
            &mut TurretStow,
            &mut SectionAnimations,
            Option<&ChildOf>,
            Option<&TurretSectionTargetEntity>,
            Option<&TurretDefenseTarget>,
        ),
        (With<TurretSectionMarker>, Without<SectionInactiveMarker>),
    >,
    q_hot: Query<&WeaponsHot>,
    mut q_joint: Query<(
        &TurretSectionPartOf,
        &SmoothLookRotation,
        &mut SmoothLookRotationTarget,
        &SmoothLookRotationOutput,
    )>,
) {
    let dt = time.delta_secs();
    for (turret, mut stow, mut animations, ship, tracked, assignment) in &mut q_turret {
        // A parentless section is unmanaged the same way a ship with no
        // WeaponsHot is: it reads as hot, deploys and never stows, instead of
        // arming a machine nothing can ever drive back up.
        let hot = ship.is_none_or(|child_of| q_hot.get(child_of.0).map_or(true, |hot| hot.0));
        let tracked = tracked.is_some_and(|tracked| tracked.is_some());
        let assigned = assignment.is_some_and(|assignment| assignment.is_some());
        let wants_deployed = hot || tracked || assigned;

        // An absent cue reads as already satisfied: a mount that authors only
        // one of the two stow tracks cycles on the track it has, instead of
        // jamming forever in a phase whose gate no track can ever report.
        let lift = animations.cue_progress(SectionAnimationCue::StowLift);
        let doors = animations.cue_progress(SectionAnimationCue::StowDoors);
        match stow.phase {
            TurretStowPhase::Deployed => {
                if wants_deployed {
                    stow.quiet_secs = 0.0;
                } else {
                    stow.quiet_secs += dt;
                    if stow.quiet_secs >= STOW_SETTLE_SECONDS {
                        stow.phase = TurretStowPhase::Stowing;
                        stow.doors_reported = false;
                    }
                }
            }
            TurretStowPhase::Stowing => {
                if wants_deployed {
                    stow.phase = TurretStowPhase::Deploying;
                    stow.doors_reported = false;
                    continue;
                }
                // Fold up, sink, then shut - and hold the lids OPEN until
                // the assembly is fully down, so an interrupted deploy never
                // sinks the gun through half-shut lids.
                let settled = command_stow_attitude(turret, &mut q_joint);
                if settled {
                    animations.set_cue(SectionAnimationCue::StowLift, 1.0);
                }
                let sunk = settled && lift.is_none_or(|lift| lift == 1.0);
                animations.set_cue(SectionAnimationCue::StowDoors, if sunk { 1.0 } else { 0.0 });
                if sunk && !stow.doors_reported {
                    stow.doors_reported = true;
                    commands.trigger(TurretStowDoorsMoved {
                        entity: turret,
                        opening: false,
                    });
                }
                if sunk && doors.is_none_or(|doors| doors == 1.0) {
                    stow.phase = TurretStowPhase::Stowed;
                    stow.quiet_secs = 0.0;
                }
            }
            TurretStowPhase::Stowed => {
                command_stow_attitude(turret, &mut q_joint);
                if wants_deployed {
                    stow.phase = TurretStowPhase::Deploying;
                    stow.doors_reported = false;
                }
            }
            TurretStowPhase::Deploying => {
                if !wants_deployed {
                    stow.phase = TurretStowPhase::Stowing;
                    stow.doors_reported = false;
                    continue;
                }
                // Part the lids, then raise. The column stays on the stow
                // attitude the whole way up; the aim solver takes the
                // hinges back only once the phase flips to Deployed.
                command_stow_attitude(turret, &mut q_joint);
                animations.set_cue(SectionAnimationCue::StowDoors, 0.0);
                if !stow.doors_reported {
                    stow.doors_reported = true;
                    commands.trigger(TurretStowDoorsMoved {
                        entity: turret,
                        opening: true,
                    });
                }
                if doors.is_none_or(|doors| doors == 0.0) {
                    animations.set_cue(SectionAnimationCue::StowLift, 0.0);
                    if lift.is_none_or(|lift| lift == 0.0) {
                        stow.phase = TurretStowPhase::Deployed;
                        stow.quiet_secs = 0.0;
                        command_rest_attitude(turret, &mut q_joint);
                    }
                }
            }
        }
    }
}

/// Steer every hinge of `turret` onto its stow angle; true once all are
/// within [`STOW_AIM_SETTLE_RAD`] of it.
fn command_stow_attitude(
    turret: Entity,
    q_joint: &mut Query<(
        &TurretSectionPartOf,
        &SmoothLookRotation,
        &mut SmoothLookRotationTarget,
        &SmoothLookRotationOutput,
    )>,
) -> bool {
    let mut settled = true;
    for (part_of, look, mut target, output) in q_joint {
        if **part_of != turret {
            continue;
        }
        let angle = stow_angle(look);
        if **target != angle {
            **target = angle;
        }
        if (**output - angle).abs() > STOW_AIM_SETTLE_RAD {
            settled = false;
        }
    }
    settled
}

/// Return every hinge of `turret` to its authored rest angle, so an idle
/// deployed mount reads exactly as an idle mount did before the stow
/// existed. The aim solver overwrites these targets the moment the mount
/// has something to look at.
fn command_rest_attitude(
    turret: Entity,
    q_joint: &mut Query<(
        &TurretSectionPartOf,
        &SmoothLookRotation,
        &mut SmoothLookRotationTarget,
        &SmoothLookRotationOutput,
    )>,
) {
    for (part_of, look, mut target, _) in q_joint {
        if **part_of != turret {
            continue;
        }
        if **target != look.initial {
            **target = look.initial;
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::time::TimeUpdateStrategy;
    use nova_gameplay::transform::prelude::SmoothLookRotationPlugin;

    use super::*;

    /// The production stow rig on a manual clock: the real joint tree, the
    /// real look controllers and the real animation driver, with the stow
    /// systems in the production order.
    fn stow_app(dt: f32) -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            SmoothLookRotationPlugin,
            SectionAnimationPlugin,
        ));
        app.add_observer(insert_turret_section);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f32(dt),
        ));
        app.add_systems(
            FixedUpdate,
            (insert_turret_stow, drive_turret_stow)
                .chain()
                .before(SmoothLookRotationSystems::Sync),
        );
        app
    }

    /// The authored stow tracks, fast, for the machine tests.
    fn stow_tracks(travel: f32) -> SectionAnimations {
        SectionAnimations::new(vec![
            SectionAnimation {
                cue: SectionAnimationCue::StowLift,
                node_prefix: "stow_lift".to_string(),
                motion: SectionAnimationMotion::Translate {
                    offset: Vec3::new(0.0, -0.8, 0.0),
                },
                open_seconds: travel,
                close_seconds: travel,
            },
            SectionAnimation {
                cue: SectionAnimationCue::StowDoors,
                node_prefix: "stow_lid_".to_string(),
                motion: SectionAnimationMotion::Translate {
                    offset: Vec3::new(-0.24, 0.0, 0.0),
                },
                open_seconds: travel,
                close_seconds: travel,
            },
        ])
    }

    /// A managed ship (weapons cold) carrying one retractable turret.
    fn spawn_stowable_turret(app: &mut App, hot: bool, travel: f32) -> (Entity, Entity) {
        let ship = app.world_mut().spawn(WeaponsHot(hot)).id();
        let turret = app
            .world_mut()
            .spawn(turret_section(TurretSectionConfig::default()))
            .id();
        app.world_mut().entity_mut(turret).insert((
            ChildOf(ship),
            Transform::default(),
            stow_tracks(travel),
        ));
        app.world_mut().flush();
        (ship, turret)
    }

    fn phase(app: &App, turret: Entity) -> TurretStowPhase {
        app.world().get::<TurretStow>(turret).unwrap().phase()
    }

    fn cue(app: &App, turret: Entity, cue: SectionAnimationCue) -> f32 {
        app.world()
            .get::<SectionAnimations>(turret)
            .unwrap()
            .cue_progress(cue)
            .unwrap()
    }

    /// The pitch joint's controller output, in radians.
    fn pitch_output(app: &App, turret: Entity) -> f32 {
        let world = app.world();
        world
            .iter_entities()
            .find_map(|e| {
                let part_of = world.get::<TurretSectionPartOf>(e.id())?;
                if **part_of != turret {
                    return None;
                }
                let look = world.get::<SmoothLookRotation>(e.id())?;
                (look.axis == Vec3::X)
                    .then(|| **world.get::<SmoothLookRotationOutput>(e.id()).unwrap())
            })
            .expect("the tree has a pitch hinge")
    }

    /// The pitch joint's authored rest angle, in radians.
    fn pitch_initial(app: &App, turret: Entity) -> f32 {
        let world = app.world();
        world
            .iter_entities()
            .find_map(|e| {
                let part_of = world.get::<TurretSectionPartOf>(e.id())?;
                if **part_of != turret {
                    return None;
                }
                let look = world.get::<SmoothLookRotation>(e.id())?;
                (look.axis == Vec3::X).then_some(look.initial)
            })
            .expect("the tree has a pitch hinge")
    }

    #[test]
    fn a_cold_scene_starts_stowed_by_snap_with_the_barrel_already_up() {
        let mut app = stow_app(0.05);
        let (_, turret) = spawn_stowable_turret(&mut app, false, 0.2);

        // Two updates: the manual clock's dt-0 baseline, then one fixed
        // step for the insert system.
        app.update();
        app.update();

        assert_eq!(phase(&app, turret), TurretStowPhase::Stowed);
        assert_eq!(cue(&app, turret, SectionAnimationCue::StowLift), 1.0);
        assert_eq!(cue(&app, turret, SectionAnimationCue::StowDoors), 1.0);
        // Snapped, not travelled: the pitch is ALREADY at its top stop.
        assert!(
            (pitch_output(&app, turret) - std::f32::consts::FRAC_PI_2).abs() < 1e-5,
            "no fold-down wiggle on a cold start"
        );
    }

    #[test]
    fn weapons_hot_deploys_through_doors_then_lift() {
        let mut app = stow_app(0.05);
        let (ship, turret) = spawn_stowable_turret(&mut app, false, 0.2);
        app.update();
        app.update();
        assert_eq!(phase(&app, turret), TurretStowPhase::Stowed);

        app.world_mut().entity_mut(ship).insert(WeaponsHot(true));
        app.update();
        app.update();
        assert_eq!(phase(&app, turret), TurretStowPhase::Deploying);
        // The lids part FIRST; the assembly stays down until they are open.
        let mid_doors = cue(&app, turret, SectionAnimationCue::StowDoors);
        assert!(mid_doors < 1.0, "the lids are parting");
        assert_eq!(
            cue(&app, turret, SectionAnimationCue::StowLift),
            1.0,
            "the gun must not rise through shut lids"
        );

        for _ in 0..20 {
            app.update();
        }
        assert_eq!(phase(&app, turret), TurretStowPhase::Deployed);
        assert_eq!(cue(&app, turret, SectionAnimationCue::StowLift), 0.0);
        assert_eq!(cue(&app, turret, SectionAnimationCue::StowDoors), 0.0);

        // With nothing to aim at, the barrel folds back down to its rest
        // angle: an idle deployed mount must not sit saluting the sky.
        for _ in 0..20 {
            app.update();
        }
        assert!(
            (pitch_output(&app, turret) - pitch_initial(&app, turret)).abs() < STOW_AIM_SETTLE_RAD,
            "the idle mount returns to its rest attitude"
        );
    }

    #[test]
    fn a_quiet_cold_mount_waits_the_settle_then_stows_lift_before_doors() {
        let mut app = stow_app(0.05);
        let (ship, turret) = spawn_stowable_turret(&mut app, true, 0.2);
        app.update();
        app.update();
        // Deployed by the hot ship the moment the machine arms... after the
        // snap-to-stow start, the deploy travel runs first.
        for _ in 0..20 {
            app.update();
        }
        assert_eq!(phase(&app, turret), TurretStowPhase::Deployed);

        // Weapons cold: the mount holds deployed through the settle window.
        app.world_mut().entity_mut(ship).insert(WeaponsHot(false));
        for _ in 0..40 {
            app.update();
        }
        assert_eq!(
            phase(&app, turret),
            TurretStowPhase::Deployed,
            "stow is lazy: two quiet seconds are not enough"
        );

        // Just past the settle (4.1 s quiet): the fold has begun, the lift
        // is mid-travel, and the lids MUST still be open.
        for _ in 0..42 {
            app.update();
        }
        assert_eq!(phase(&app, turret), TurretStowPhase::Stowing);
        assert!(cue(&app, turret, SectionAnimationCue::StowLift) < 1.0);
        assert_eq!(
            cue(&app, turret, SectionAnimationCue::StowDoors),
            0.0,
            "the lids stay open until the assembly is fully down"
        );
        for _ in 0..40 {
            app.update();
        }
        assert_eq!(phase(&app, turret), TurretStowPhase::Stowed);
        assert_eq!(cue(&app, turret, SectionAnimationCue::StowDoors), 1.0);
    }

    #[test]
    fn the_doors_report_once_per_cycle_and_only_when_the_lids_actually_move() {
        // The audio seam. Two things it must get right: the report is an EDGE,
        // not a per-frame repeat while the lids travel; and the SHUT report
        // waits for the assembly to be down, because the fold spends most of a
        // second folding and sinking before the lids do anything.
        #[derive(Resource, Default)]
        struct Reports(Vec<bool>);

        let mut app = stow_app(0.05);
        app.init_resource::<Reports>();
        app.add_observer(|ev: On<TurretStowDoorsMoved>, mut log: ResMut<Reports>| {
            log.0.push(ev.opening);
        });
        let (ship, turret) = spawn_stowable_turret(&mut app, true, 0.2);
        for _ in 0..22 {
            app.update();
        }
        assert_eq!(phase(&app, turret), TurretStowPhase::Deployed);
        assert_eq!(
            app.world().resource::<Reports>().0,
            vec![true],
            "one open report for the whole rise, not one per frame of it"
        );

        // Cold: the fold begins after the settle, but the lids do not move
        // until the assembly is down - and neither does the report.
        app.world_mut().entity_mut(ship).insert(WeaponsHot(false));
        for _ in 0..82 {
            app.update();
        }
        assert_eq!(phase(&app, turret), TurretStowPhase::Stowing);
        assert_eq!(
            app.world().resource::<Reports>().0,
            vec![true],
            "the fold has begun but the lids have not moved, so nothing sounds"
        );

        for _ in 0..40 {
            app.update();
        }
        assert_eq!(phase(&app, turret), TurretStowPhase::Stowed);
        assert_eq!(
            app.world().resource::<Reports>().0,
            vec![true, false],
            "the shut report lands once, when the lids are told to close"
        );
    }

    #[test]
    fn a_live_tracked_target_deploys_a_cold_mount() {
        // Point defense comes up autonomously: no weapons hot, just a body
        // being tracked - the AI feed and the Flight Computer assignment
        // both land here as a Some target entity.
        let mut app = stow_app(0.05);
        let (_, turret) = spawn_stowable_turret(&mut app, false, 0.2);
        app.update();
        app.update();
        assert_eq!(phase(&app, turret), TurretStowPhase::Stowed);

        let torpedo = app.world_mut().spawn_empty().id();
        app.world_mut()
            .entity_mut(turret)
            .insert(TurretSectionTargetEntity(Some(torpedo)));
        app.update();
        app.update();
        assert_eq!(phase(&app, turret), TurretStowPhase::Deploying);
    }

    #[test]
    fn a_point_defense_assignment_deploys_a_cold_mount() {
        // The Flight Computer deploys a mount by assigning it: a cold player
        // battery's only deploy demand is `TurretDefenseTarget`, so the
        // assignment alone must bring the mount up.
        let mut app = stow_app(0.05);
        let (_, turret) = spawn_stowable_turret(&mut app, false, 0.2);
        app.update();
        app.update();
        assert_eq!(phase(&app, turret), TurretStowPhase::Stowed);

        let torpedo = app.world_mut().spawn_empty().id();
        app.world_mut()
            .entity_mut(turret)
            .insert(TurretDefenseTarget(Some(torpedo)));
        app.update();
        app.update();
        assert_eq!(phase(&app, turret), TurretStowPhase::Deploying);
    }

    #[test]
    fn an_unmanaged_turret_deploys_and_never_stows() {
        // The fire path's fail-open, mirrored: a ship with no WeaponsHot is
        // unmanaged, so its mounts deploy at spawn and hold - bare rigs and
        // example ranges keep working with no weapons plumbing.
        let mut app = stow_app(0.05);
        let ship = app.world_mut().spawn_empty().id();
        let turret = app
            .world_mut()
            .spawn(turret_section(TurretSectionConfig::default()))
            .id();
        app.world_mut().entity_mut(turret).insert((
            ChildOf(ship),
            Transform::default(),
            stow_tracks(0.2),
        ));
        app.world_mut().flush();

        for _ in 0..30 {
            app.update();
        }
        assert_eq!(phase(&app, turret), TurretStowPhase::Deployed);

        // A long quiet spell changes nothing: unmanaged means always hot.
        for _ in 0..120 {
            app.update();
        }
        assert_eq!(phase(&app, turret), TurretStowPhase::Deployed);
    }

    #[test]
    fn a_turret_without_stow_tracks_is_untouched() {
        let mut app = stow_app(0.05);
        let ship = app.world_mut().spawn(WeaponsHot(false)).id();
        let turret = app
            .world_mut()
            .spawn(turret_section(TurretSectionConfig::default()))
            .id();
        app.world_mut().entity_mut(turret).insert((
            ChildOf(ship),
            Transform::default(),
            SectionAnimations::default(),
        ));
        app.world_mut().flush();

        for _ in 0..10 {
            app.update();
        }
        assert!(
            app.world().get::<TurretStow>(turret).is_none(),
            "only a section authoring a StowLift track gets the machine"
        );
    }

    #[test]
    fn a_mount_authoring_only_the_lift_track_cycles_without_doors() {
        // The armer admits a section on StowLift alone, so the driver must
        // let it through: an absent doors cue reads as satisfied, and the
        // mount sinks and rises on the lift instead of jamming in a phase
        // whose doors gate no track can ever report.
        let mut app = stow_app(0.05);
        let ship = app.world_mut().spawn(WeaponsHot(false)).id();
        let turret = app
            .world_mut()
            .spawn(turret_section(TurretSectionConfig::default()))
            .id();
        app.world_mut().entity_mut(turret).insert((
            ChildOf(ship),
            Transform::default(),
            SectionAnimations::new(vec![SectionAnimation {
                cue: SectionAnimationCue::StowLift,
                node_prefix: "stow_lift".to_string(),
                motion: SectionAnimationMotion::Translate {
                    offset: Vec3::new(0.0, -0.8, 0.0),
                },
                open_seconds: 0.2,
                close_seconds: 0.2,
            }]),
        ));
        app.world_mut().flush();
        app.update();
        app.update();
        assert_eq!(phase(&app, turret), TurretStowPhase::Stowed);

        app.world_mut().entity_mut(ship).insert(WeaponsHot(true));
        for _ in 0..20 {
            app.update();
        }
        assert_eq!(phase(&app, turret), TurretStowPhase::Deployed);
        assert_eq!(cue(&app, turret, SectionAnimationCue::StowLift), 0.0);

        // And back down: the full stow completes on the lift alone.
        app.world_mut().entity_mut(ship).insert(WeaponsHot(false));
        for _ in 0..160 {
            app.update();
        }
        assert_eq!(phase(&app, turret), TurretStowPhase::Stowed);
    }

    #[test]
    fn a_parentless_turret_reads_as_unmanaged_and_deploys() {
        // No ChildOf at all: the machine still arms, and the driver treats
        // the missing parent as unmanaged-hot, so the mount deploys instead
        // of parking behind lids nothing can ever reopen.
        let mut app = stow_app(0.05);
        let turret = app
            .world_mut()
            .spawn(turret_section(TurretSectionConfig::default()))
            .id();
        app.world_mut()
            .entity_mut(turret)
            .insert((Transform::default(), stow_tracks(0.2)));
        app.world_mut().flush();

        for _ in 0..30 {
            app.update();
        }
        assert_eq!(phase(&app, turret), TurretStowPhase::Deployed);
    }
}
