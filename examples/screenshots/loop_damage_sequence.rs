//! loop_damage_sequence: the `landing-damage-sequence` webm loop - a corvette
//! takes a broadside, loses a turret, sheds an outer section and drifts off
//! through its own wreckage.
//!
//! The landing page's damage feature row used to borrow the v0.11.0 news loop.
//! That loop is frozen by design - it is the evidence for what one release
//! changed and is never re-cut - so the front page was pinned to what damage
//! looked like in v0.11.0 and could not follow the game. This is the row's own
//! scene: a living loop that re-cuts every capture cycle, staging the four
//! beats the row's copy actually promises.
//!
//! Every beat is the production path. Damage is [`HealthApplyDamage`], the
//! same event a round delivers; the sever is the integrity pipeline
//! (`nova_ship::sections::integrity`), and the script waits on a real
//! [`ShipWreckFragmentMarker`] rather than on a duration, so a run that fails
//! to sever aborts by name instead of recording a loop of a ship sitting
//! there. Nothing is faked for the camera except the send-off nudge, which
//! only sets the pace the separation reads at.
//!
//! Everything the beats touch is on the PORT flank, because the camera is:
//! the cracked plating, the turret that dies and the pod that severs all have
//! to be the ones facing the lens.
//!
//! The same walk also shoots the ships chapter's opening still,
//! `wiki-ships-damage.png`, off the last beat - the hull cracked along the
//! flank, its port turret dead, and the hole where its aft deck tore off. One
//! scene answers both, and the still is taken AFTER the loop closes: a
//! screenshot and a loop frame are the same window capture, and the second one
//! asked for in a frame is dropped.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - the full walk, recording
//!   nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: record and encode the loop into
//!   `NOVA_CAPTURE_DIR/landing-damage-sequence.webm` and shoot the still.
//!
//! Capture:
//! ```text
//! NOVA_CAPTURE_DIR=target/loop-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example loop_damage_sequence --features debug
//! ```

#[path = "shared/kit.rs"]
mod kit;

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "loop_damage_sequence")]
#[command(version = "1.0.0")]
#[command(
    about = "The landing page's damage loop: broadside, turret, sever, drift. Autopilot-only: every actor is scripted or inert",
    long_about = None
)]
struct Cli;

/// The loop this example records - the webm's file stem.
#[cfg(feature = "debug")]
const LOOP_NAME: &str = "landing-damage-sequence";

/// The ships chapter's opening still, shot off the end of the same walk.
#[cfg(feature = "debug")]
const DAMAGE_STILL: &str = "wiki-ships-damage.png";

/// Scenario id of the gunship that takes the beating.
const SUBJECT_ID: &str = "damage_subject";

/// The catalog hull the beating is delivered to: the fleet's warship, which is
/// the only one carrying enough guns for the turret beat to cost it something.
const SUBJECT_HULL: &str = "block_gunship";

/// The plating the broadside walks across, bow to waist, named by BUILD-GRID
/// CELL along the port flank - the coordinate the hull is authored in, where
/// its `plate_N` ids are an artifact of the union order (see
/// [`kit::cell_section`]).
///
/// Three sections rather than one, because the row's claim is that damage
/// SPREADS: a single cracked cell reads as one unlucky hit, and the whole
/// point of the level system is that a hull wears its history across every
/// plate that took something.
#[cfg(feature = "debug")]
const BROADSIDE_CELLS: [Vec3; 3] = [
    Vec3::new(-1.0, 0.0, -2.0),
    Vec3::new(-1.0, 0.0, -1.0),
    Vec3::new(-1.0, 0.0, 0.0),
];

/// How much of a section's maximum the broadside takes off.
///
/// High enough to reach the deep end of the crack ladder, short of the kill
/// that would sever the plate off before the sequence gets to its own severing
/// beat.
#[cfg(feature = "debug")]
const BROADSIDE_FRACTION: f32 = 0.78;

/// The turret that goes quiet. A turret is a leaf on the hull graph, so
/// killing it disables a gun without severing anything - which is the beat:
/// losing a weapon is not the same event as losing structure.
#[cfg(feature = "debug")]
const DISABLED_TURRET: &str = "pdc_forward_port";

/// The cell the final hit goes through: the port aft deck plate.
///
/// It is the seat the aft port mount stands on, and the mount's only link, so
/// cutting the plate frees the gun and the deck it rode on as an independent
/// wreck. A plate with nothing hanging off it frees nothing.
#[cfg(feature = "debug")]
const SEVERED_CELL: Vec3 = Vec3::new(-1.0, 1.0, 1.0);

/// Which way the still's lens faces, in the HULL's own frame - a direction
/// only, so the tumble the sequence ends on cannot decide which face is shot.
///
/// Broadside off the port bow, and not the loop's own bearing: the loop looks
/// from the port QUARTER, which is a fine view of a hull coming apart in
/// motion and the wrong one for a still, because everything the sequence
/// touches is on the flank. Cracked plating runs bow to waist
/// ([`BROADSIDE_CELLS`]), the dead gun is the forward port mount, and the hole
/// is aft at [`SEVERED_CELL`] - one flank-on frame carries all three, an aft
/// quarter carries none of them.
#[cfg(feature = "debug")]
const STILL_EYE: Vec3 = Vec3::new(-1.0, 0.3, -0.35);

/// How far the still's lens stands off the hull, meters.
///
/// The gunship is 85 m stem to stern and the lens is across its beam, so the
/// whole length is in frame: the lens spans 1.47 times its distance, so at this
/// range the frame is about 198 m wide and the hull is a little over two fifths
/// of it, which leaves the silhouette room without shrinking the cracks it is
/// here to show. The pod freed by the last beat is
/// 90 m clear by the time the loop closes and may fall outside; the framing
/// keeps the SHIP, because the hole the pod left is the readable half of the
/// story and a pod at that range is four pixels. The chapter's figure note
/// says so.
#[cfg(feature = "debug")]
const STILL_STANDOFF: f32 = 135.0;

/// The slow tumble the hull carries into the sequence, so the freed structure
/// inherits real motion instead of hanging dead in frame.
#[cfg(feature = "debug")]
const SUBJECT_SPIN: Vec3 = Vec3::new(0.02, 0.12, 0.05);

/// Scripted send-off speed per freed fragment, on top of the inherited spin.
/// Added straight to an avian `LinearVelocity`, so it is an engine world-unit
/// figure (world units per second), not a speed in meters per second.
///
/// Paced against the drift hold and the framing rather than picked: the freed
/// block has to clear the hull it came off while still being IN the shot when
/// the loop closes, and a page that plays the file forever shows an empty
/// frame for however long the wreck leaves early.
#[cfg(feature = "debug")]
const FRAGMENT_DRIFT_SPEED: f32 = 0.9;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::default());
        app.add_plugins(damage_script());
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShips>) {
    commands.trigger(LoadScenario(damage_range(&game_assets, &ships)));
}

/// The set: one gunship three-quarter to the lens with its port flank open,
/// a near rock field for parallax, and the photo rig.
fn damage_range(game_assets: &GameAssets, ships: &GameShips) -> ScenarioConfig {
    let subject = EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: SUBJECT_ID.to_string(),
            name: "Subject".to_string(),
            position: Meters3::ZERO,
            rotation: Quat::from_rotation_y(-0.55),
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::None,
            allegiance: Some(Allegiance::Enemy),
            // The whole shipped gunship, turrets included: the sequence needs
            // a turret to disable and a real mate graph to sever along, and
            // both come from the catalog rather than from a hand-typed copy.
            hull: ShipSource::Inline(kit::catalog_ship(ships, SUBJECT_HULL)),
            ..default()
        }),
    });
    let field = kit::NearField {
        id_prefix: "damage_rock_",
        count: 12,
        seed: 20260831,
        distance: (Meters(550.0), Meters(1_200.0)),
        radius: (Meters(10.0), Meters(25.0)),
        y_spread: Meters(300.0),
    };

    ScenarioConfig {
        description: "A gunship worn down a level at a time, then cut open.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![field.action(game_assets), subject],
                ThreePointRig::around("photo", Meters3::ZERO, 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "loop_damage_sequence".to_string(),
            "Damage Sequence Loop".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The driven walk: frame, spin, then the four beats inside one open loop.
#[cfg(feature = "debug")]
fn damage_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the range")
        .enter(GameStates::Loading)
        .until(subject_present())
        .deadline(60.0)
        .add()
        // One fixed framing for the whole sequence. A cut between beats would
        // read as four clips; the row's claim is that this is one continuous
        // thing happening to one ship, so the camera does not move.
        //
        // The stand-off is the SHIP's, not the crack's. At 76 m this lens was
        // inside its own subject: the loop showed a drive bell and a piece of
        // plating, and the severed deck crossed the lens close enough to fill
        // it. The hull is shot from the port QUARTER, so it presents about
        // 60 m rather than its 85 m length; held at 115 m that silhouette is
        // a little over a third of the frame, which leaves the freed deck room to drift
        // without leaving the shot, and a crack is still a crack.
        .step("frame the port flank")
        .on_enter(|world| {
            hide_hud(world);
            pose_camera(
                world,
                Meters3::new(-88.0, 32.0, 81.0),
                Meters3::new(-10.0, 6.0, 1.0),
            );
        })
        .until(elapsed(0.8))
        .add()
        // Motion first, recording second: the spin is established before the
        // loop opens, so frame one is already alive.
        .step("set the hull adrift")
        .on_enter(spin_subject)
        .until(elapsed(0.7))
        .add()
        .step("open the loop")
        .on_enter(|world| loop_start(world, LOOP_NAME))
        // A beat of the intact hull, so the first hit lands inside the loop
        // rather than on frame one.
        .until(elapsed(0.5))
        .add()
        .step("walk the broadside across the flank")
        .on_enter(spread_cracks)
        .until(elapsed(1.2))
        .add()
        .step("kill the port turret")
        .on_enter(disable_turret)
        .until(elapsed(0.9))
        .add()
        // The cut waits for the production sever - a wreck fragment with its
        // own body - to actually exist, not for a duration.
        .step("cut the port aft deck")
        .on_enter(sever_pod)
        .until(any_entity::<With<ShipWreckFragmentMarker>>())
        .deadline(5.0)
        .add()
        .step("drift through the wreck")
        .on_enter(nudge_fragments)
        .until(elapsed(2.6))
        .add()
        .step("close the loop")
        .on_enter(|world| loop_end(world, LOOP_NAME))
        .until(loop_written(LOOP_NAME))
        .deadline(60.0)
        .add()
        // The still, on the loop's own last state: the hull is already
        // cracked, the turret is already gone and the pod is already clear,
        // which is the whole of what the chapter's figure note promises. It
        // cannot be shot a beat earlier - see the module doc on the shared
        // window capture - and it is not shot on the loop's fixed pose
        // either; see `frame_the_wreck`.
        .step("frame the wreck")
        .on_enter(frame_the_wreck)
        .until(elapsed(0.4))
        .add()
        .step("shoot wiki-ships-damage.png")
        .on_enter(|world| shoot(world, DAMAGE_STILL))
        .until(shot_written(DAMAGE_STILL))
        .deadline(30.0)
        .add()
}

/// Advance once the subject is in the world.
#[cfg(feature = "debug")]
fn subject_present() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        world
            .try_query_filtered::<&EntityId, With<SpaceshipRootMarker>>()
            .is_some_and(|mut query| query.iter(world).any(|id| id.0 == SUBJECT_ID))
    })
}

/// Stop the tumble and put the lens back on the flank the beats hit.
///
/// The facing has to be measured rather than authored: the hull tumbles, so a
/// constant world pose lands on whichever side happens to be turned toward the
/// lens by the last beat - the drive bell, as it happens, with every hit on
/// the far side. Rebuilt from the hull's live rotation, the lens goes back to
/// the PORT flank, which is the only flank any of the beats touched.
///
/// The spin is zeroed first, so the pose the camera is given is the pose the
/// shot is taken on.
#[cfg(feature = "debug")]
fn frame_the_wreck(world: &mut World) {
    let Some(subject) = kit::ship_root(world, SUBJECT_ID) else {
        warn!("damage loop: no subject to frame the still on");
        return;
    };
    if let Some(mut angular) = world
        .entity_mut(subject)
        .get_mut::<avian3d::prelude::AngularVelocity>()
    {
        angular.0 = Vec3::ZERO;
    }
    let Some(hull) = world.get::<GlobalTransform>(subject).copied() else {
        warn!("damage loop: subject carries no global transform");
        return;
    };
    // A Bevy boundary: the hull's transform is world units and the pose the
    // harness takes is meters, so the position crosses over once. The rotation
    // is unitless and turns the meter offset as it is.
    let at = Meters3::from_engine(hull.translation()).get();
    let eye = at + hull.rotation() * STILL_EYE.normalize() * STILL_STANDOFF;
    let meters = |v: Vec3| Meters3::new(v.x, v.y, v.z);
    pose_camera(world, meters(eye), meters(at));
}

/// Give the hull its slow tumble.
#[cfg(feature = "debug")]
fn spin_subject(world: &mut World) {
    let Some(subject) = kit::ship_root(world, SUBJECT_ID) else {
        warn!("damage loop: no subject to spin");
        return;
    };
    if let Some(mut angular) = world
        .entity_mut(subject)
        .get_mut::<avian3d::prelude::AngularVelocity>()
    {
        angular.0 = SUBJECT_SPIN;
    }
}

/// Take most of the plating off three sections along the flank in one volley.
#[cfg(feature = "debug")]
fn spread_cracks(world: &mut World) {
    let ships = world.resource::<GameShips>().clone();
    for cell in BROADSIDE_CELLS {
        let section = kit::cell_section(&ships, SUBJECT_HULL, cell);
        let Some(node) = kit::section_health(world, SUBJECT_ID, &section) else {
            warn!("damage loop: no health node under section '{section}'");
            continue;
        };
        let amount = world
            .get::<Health>(node)
            .map_or(0.0, |health| health.max * BROADSIDE_FRACTION);
        world.trigger(HealthApplyDamage {
            entity: node,
            source: None,
            amount,
        });
        info!("damage loop: cracked '{section}' for {amount:.1}");
    }
}

/// Kill the port turret outright - a gun lost, no structure freed.
#[cfg(feature = "debug")]
fn disable_turret(world: &mut World) {
    kill_section(world, DISABLED_TURRET);
}

/// Kill the port aft deck plate, which frees the mount standing on it.
#[cfg(feature = "debug")]
fn sever_pod(world: &mut World) {
    let ships = world.resource::<GameShips>().clone();
    let section = kit::cell_section(&ships, SUBJECT_HULL, SEVERED_CELL);
    kill_section(world, &section);
}

/// Put one section down through the production damage path.
#[cfg(feature = "debug")]
fn kill_section(world: &mut World, section: &str) {
    let Some(node) = kit::section_health(world, SUBJECT_ID, section) else {
        warn!("damage loop: no health node under section '{section}'");
        return;
    };
    world.trigger(HealthApplyDamage {
        entity: node,
        source: None,
        amount: 1.0e6,
    });
    info!("damage loop: killed '{section}'");
}

/// Send each freed fragment gently away from the hull, on top of whatever
/// motion the sever handed it.
#[cfg(feature = "debug")]
fn nudge_fragments(world: &mut World) {
    let origin = kit::ship_root(world, SUBJECT_ID)
        .and_then(|subject| world.get::<GlobalTransform>(subject))
        .map(|transform| transform.translation())
        .unwrap_or(Vec3::ZERO);
    let fragments: Vec<(Entity, Vec3)> = world
        .query_filtered::<(Entity, &GlobalTransform), With<ShipWreckFragmentMarker>>()
        .iter(world)
        .map(|(entity, transform)| (entity, transform.translation()))
        .collect();
    if fragments.is_empty() {
        warn!("damage loop: no fragments to send adrift");
        return;
    }
    for (fragment, position) in fragments {
        let outward = (position - origin).normalize_or(Vec3::X);
        if let Some(mut velocity) = world
            .entity_mut(fragment)
            .get_mut::<avian3d::prelude::LinearVelocity>()
        {
            velocity.0 += outward * FRAGMENT_DRIFT_SPEED + Vec3::Y * 0.15;
        }
    }
    info!("damage loop: wreck adrift");
}
