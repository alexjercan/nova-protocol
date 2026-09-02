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
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - the full walk, recording
//!   nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: record and encode the loop into
//!   `NOVA_CAPTURE_DIR/landing-damage-sequence.webm`.
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

/// Scenario id of the corvette that takes the beating.
const SUBJECT_ID: &str = "damage_subject";

/// The plating the broadside walks across, bow to waist.
///
/// Three sections rather than one, because the row's claim is that damage
/// SPREADS: a single cracked cell reads as one unlucky hit, and the whole
/// point of the level system is that a hull wears its history across every
/// plate that took something.
#[cfg(feature = "debug")]
const BROADSIDE_SECTIONS: [&str; 3] = ["nose", "fuselage", "pod_port"];

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
const DISABLED_TURRET: &str = "turret_port";

/// The section the final hit goes through.
///
/// The port pod is the spine joint of the port flank: the engine block hangs
/// off the hull through it, so cutting the pod frees the engine as an
/// independent wreck. A tail cut frees nothing - the tail is a leaf.
#[cfg(feature = "debug")]
const SEVERED_SECTION: &str = "pod_port";

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

/// The set: one corvette three-quarter to the lens with its port flank open,
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
            // The whole shipped corvette, turrets included: the sequence needs
            // a turret to disable and a real mate graph to sever along, and
            // both come from the catalog rather than from a hand-typed copy.
            hull: ShipSource::Inline(ShipHull {
                sections: kit::kenney_hull(ships, "cargoa"),
                ..default()
            }),
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
        description: "A corvette worn down a level at a time, then cut open.".to_string(),
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
        // thing happening to one ship, so the camera does not move. Close
        // enough that a crack is a crack rather than a smudge, and no closer:
        // the frame also has to hold the freed engine block for the whole
        // drift, and that block travels away from the hull.
        .step("frame the port flank")
        .on_enter(|world| {
            hide_hud(world);
            pose_camera(
                world,
                Meters3::new(-62.0, 23.0, 54.0),
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
        .step("cut the port pod")
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
    for section in BROADSIDE_SECTIONS {
        let Some(node) = kit::section_health(world, SUBJECT_ID, section) else {
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

/// Kill the port pod, which frees the engine block behind it.
#[cfg(feature = "debug")]
fn sever_pod(world: &mut World) {
    kill_section(world, SEVERED_SECTION);
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
