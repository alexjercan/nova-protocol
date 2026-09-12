//! system_hull_scaling: the two reference hulls of the derived-figure sweep,
//! measured side by side.
//!
//! Task `20260909-213708` asserts that every flight, AI, weapon and destruction
//! figure which is fixed in world units, seconds or counts really depends on
//! the hull, the target or the round. Each of its items is a BALANCE change as
//! well as a fix, so each one is measured on `block_skiff` (the fleet's
//! smallest armed-group hull) and on `block_carrier` (the largest hull the base
//! game ships) before and after. This range is where those measurements are
//! taken, so a "before" figure in the ledger is a reading rather than a
//! recollection.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: the reference hulls span the size the sweep assumes` | the carrier's structural arm and live section count are each an order of magnitude over the skiff's, so a constant that works on one is not evidence about the other |
//! | 2 | `outcome: both hulls publish a live attitude envelope` | each hull publishes both ceilings off its own live geometry, which is the input every derived figure in the sweep reads |
//! | 3 | `outcome: the computer is pinned to the largest shipped hull` | the intact carrier clears its structural ceiling by a small margin and the skiff clears its own by a wide one, so the controller's torque is visible on the fleet's big hull and invisible on its small one |
//! | 4 | `outcome: a bigger hull is seen from further away` | each hull publishes a radar signature derived from its own structure, and the carrier is lockable from several times the distance the skiff is |
//! | 5 | `outcome: the hull inputs are recorded` | RECORD: arm, envelope, cells, mass, inertia, summed computer torque, both ceilings, the live section census and the lock range, per hull |
//!
//! Claim 5 asserts NOTHING. It is the table the ledger quotes, read against the
//! figures in `tasks/20260909-213118/FEEDBACK.md` and never against a
//! threshold.
//!
//! The hulls are parked far apart and given no controller demand and no AI, so
//! nothing here is a statement about flying: the subject is the geometry every
//! other range's constant is supposed to be derived from.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_hull_scaling --features debug
//! # look for: `hull scaling: block_skiff: ...`,
//! #           `hull scaling: block_carrier: ...`,
//! #           `autopilot: cycle complete, no panic`
//! ```

#[path = "../screenshots/shared/kit.rs"]
mod kit;

#[cfg(feature = "debug")]
use std::sync::Arc;

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_hull_scaling")]
#[command(version = "1.0.0")]
#[command(
    about = "The two reference hulls of the derived-figure sweep, measured side by side. Autopilot-only correctness range",
    long_about = None
)]
struct Cli;

/// The small reference hull: the cleanup group's unarmed needle.
const SKIFF: &str = "block_skiff";

/// The large reference hull: the campaign's home, and the largest hull the base
/// game ships.
const CARRIER: &str = "block_carrier";

/// The skiff's scenario id.
const SKIFF_ID: &str = "hull_scaling_skiff";

/// The carrier's scenario id.
const CARRIER_ID: &str = "hull_scaling_carrier";

/// How far apart the two hulls are parked, m. Well over the sum of their
/// envelopes, so neither is inside the other's collider set and the reading of
/// one is not a reading of a collision.
const SEPARATION: Meters = Meters(4_000.0);

/// In-step seconds a beat gets to reach its world condition.
///
/// Over the fleet's usual step deadline for the reason `system_hud_shell`
/// gives: CI runs the correctness pass on a software rasterizer, where one
/// frame of 2 081 sections costs seconds, and the settle beat here waits on a
/// mass solve that needs several of them. A backstop that names a hung beat,
/// not a budget the range is held to.
#[cfg(feature = "debug")]
const STEP_DEADLINE_SECS: f32 = 120.0;

/// In-step seconds both hulls are given to link their colliders and publish
/// their first envelope.
#[cfg(feature = "debug")]
const SETTLE_SECS: f32 = 4.0;

/// The size ratio the sweep's premise needs the two hulls to actually have.
///
/// A DELIVERY GUARD against the catalog, not an authored number: every item in
/// the task argues that a constant tuned on one of these hulls is wrong on the
/// other, and that argument is only worth making while the two really are this
/// far apart. Re-tune the fleet so the carrier is four times the skiff rather
/// than an order of magnitude over it, and this fails and says so.
#[cfg(feature = "debug")]
const ARM_RATIO_FLOOR: f32 = 4.0;

/// The live-section ratio the same premise needs. The carrier is 2 081 sections
/// against the skiff's 21.
#[cfg(feature = "debug")]
const CELL_RATIO_FLOOR: f32 = 20.0;

/// The torque headroom the INTACT carrier must keep over its structural
/// ceiling, as a fraction of that ceiling.
///
/// `DEFAULT_MAX_TORQUE` is pinned here and nowhere else. Below the floor the
/// largest shipped hull is torque-bound with every computer alive, which is the
/// regime the constant used to declare unreachable; above the ceiling the
/// margin is wide enough that losing computers costs the hull nothing, and the
/// number goes back to being invisible on everything that ships. Re-tune the
/// controller, or the carrier, and this says which way it moved.
#[cfg(feature = "debug")]
const CARRIER_HEADROOM_FLOOR: f32 = 0.05;

/// The other side of that band.
#[cfg(feature = "debug")]
const CARRIER_HEADROOM_CEILING: f32 = 0.20;

/// The headroom the skiff must keep, over the same ceiling.
///
/// The same constant has to be invisible on a small hull: a needle is
/// structure-bound on one computer and stays sharp however much of it is shot
/// away, so its margin is a different order of magnitude, not a nearby number.
#[cfg(feature = "debug")]
const SKIFF_HEADROOM_FLOOR: f32 = 10.0;

/// How much further the carrier must be lockable from than the skiff.
///
/// A ship's radar signature is derived from its live structure and the
/// machinery on it, so the fleet's two extremes must not answer a scanner with
/// the same figure: the skiff is a contact a picket finds late and the carrier
/// one it finds from across the volume. The shipped hulls sit near 3x. Flatten
/// the model back toward one range for every ship and this fails and says so.
#[cfg(feature = "debug")]
const LOCK_RANGE_RATIO_FLOOR: f32 = 2.0;

/// The script type, named once so the step list and its helpers agree.
#[cfg(feature = "debug")]
type Script = nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(range_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(scaling_script());
    }

    app.run()
}

fn range_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_range);
}

fn setup_range(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShips>) {
    commands.trigger(LoadScenario(scaling_range(&game_assets, &ships)));
}

/// The range: the two reference hulls parked far apart in flat space, neither
/// flown, neither steered.
fn scaling_range(game_assets: &GameAssets, ships: &GameShips) -> ScenarioConfig {
    let hull = |id: &str, name: &str, catalog: &str, x: Meters| {
        EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: id.to_string(),
                name: name.to_string(),
                position: Meters3::new(x.get(), 0.0, 0.0),
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
                controller: SpaceshipController::None,
                allegiance: None,
                hull: ShipSource::Inline(kit::catalog_ship(ships, catalog)),
                ..default()
            }),
        })
    };

    ScenarioConfig {
        description: "The two reference hulls of the derived-figure sweep, parked and measured."
            .to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![
                    hull(SKIFF_ID, "Skiff", SKIFF, -SEPARATION / 2.0),
                    hull(CARRIER_ID, "Carrier", CARRIER, SEPARATION / 2.0),
                ],
                ThreePointRig::around("hull scaling", Meters3::ZERO, 40.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "hull_scaling_range".to_string(),
            "Hull Scaling Range".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The scripted run: load both hulls, let avian link and weigh them, read them.
#[cfg(feature = "debug")]
fn scaling_script() -> Script {
    Script::new()
        .step("load both reference hulls")
        .enter(GameStates::Loading)
        .until(both_hulls_measured())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("let both hulls settle")
        .until(elapsed(SETTLE_SECS))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("measure both reference hulls")
        .on_enter(measure_both_hulls)
        .add()
}

/// Both hulls are present AND have been weighed: a root avian has not measured
/// yet publishes an infinite torque ceiling, which is the honest answer while
/// its colliders are still being linked and a useless one to record.
#[cfg(feature = "debug")]
fn both_hulls_measured() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        [SKIFF_ID, CARRIER_ID].iter().all(|id| {
            weighed_hull(world, id).is_some_and(|root| {
                world
                    .get::<avian3d::prelude::ComputedAngularInertia>(root)
                    .is_some_and(|inertia| {
                        inertia
                            .principal_angular_inertia_with_local_frame()
                            .0
                            .max_element()
                            > 0.0
                    })
                    && world.get::<HullRadius>(root).is_some()
            })
        })
    })
}

/// The hull with this scenario id, off a borrowed world.
#[cfg(feature = "debug")]
fn weighed_hull(world: &World, id: &str) -> Option<Entity> {
    let mut hulls = world.try_query_filtered::<(Entity, &EntityId), With<SpaceshipRootMarker>>()?;
    hulls
        .iter(world)
        .find(|(_, entity_id)| entity_id.0 == id)
        .map(|(entity, _)| entity)
}

/// Everything about one hull that a derived figure in the sweep reads.
#[cfg(feature = "debug")]
#[derive(Clone, Copy, Debug)]
struct HullInputs {
    /// Live sections.
    cells: usize,
    /// Centre of mass to the furthest live FACE - the arm the attitude ceiling
    /// and the arrival rule read.
    arm: Meters,
    /// Centre of mass to the furthest live collider POINT - the radius the HUD
    /// shells stand outside of.
    envelope: Meters,
    /// What avian weighed the hull at, kg.
    mass: f32,
    /// The largest principal moment of inertia - the axis the attitude ceiling
    /// budgets against.
    inertia: f32,
    /// Summed authored `max_torque` over the live flight computers.
    torque: f32,
    /// Live flight computers.
    controllers: usize,
    /// Live thruster sections.
    thrusters: usize,
    /// Live weapon sections: turrets, torpedo bays and railguns together.
    weapons: usize,
    /// `sum(max_torque) / I`, rad/s2.
    torque_ceiling: f32,
    /// `LOAD_LIMIT / arm`, rad/s2.
    structural_ceiling: f32,
    /// What the hull returns to a scanner, derived from its live structure.
    signature: Meters,
    /// How far that signature is lockable from, under the shipped sensitivity
    /// and before any observer's own cap.
    lock_range: Meters,
}

#[cfg(feature = "debug")]
impl HullInputs {
    /// Which ceiling the hull actually feels.
    fn binds(self) -> &'static str {
        if self.torque_ceiling < self.structural_ceiling {
            "torque"
        } else {
            "structure"
        }
    }

    /// How much torque authority the hull has over its structural ceiling, as a
    /// fraction of that ceiling. Negative on a torque-bound hull.
    fn headroom(self) -> f32 {
        if self.structural_ceiling > 0.0 && self.structural_ceiling.is_finite() {
            self.torque_ceiling / self.structural_ceiling - 1.0
        } else {
            f32::INFINITY
        }
    }
}

#[cfg(feature = "debug")]
fn read_hull(world: &mut World, id: &str) -> HullInputs {
    use avian3d::prelude::{ComputedAngularInertia, ComputedMass};

    let root =
        weighed_hull(world, id).unwrap_or_else(|| panic!("hull scaling: hull '{id}' is present"));
    let arm = world.get::<HullRadius>(root).map_or(0.0, |radius| **radius);
    let envelope = world
        .get::<HullEnvelopeRadius>(root)
        .map_or(0.0, |radius| **radius);
    let mass = world
        .get::<ComputedMass>(root)
        .map_or(0.0, |mass| mass.value());
    let inertia = world
        .get::<ComputedAngularInertia>(root)
        .map_or(0.0, |inertia| {
            inertia
                .principal_angular_inertia_with_local_frame()
                .0
                .max_element()
        });

    let signature = world.get::<LockSignature>(root).map_or(0.0, |sig| **sig);
    let sensitivity = world
        .get_resource::<TargetingSettings>()
        .map_or(0.0, |settings| settings.signature_range_per_unit);

    let cells = world
        .query_filtered::<&ChildOf, (With<SectionMarker>, Without<SectionInactiveMarker>)>()
        .iter(world)
        .filter(|parent| parent.parent() == root)
        .count();
    let torque: f32 = world
        .query_filtered::<(&ControllerSectionTuning, &ChildOf), Without<SectionInactiveMarker>>()
        .iter(world)
        .filter(|(_, parent)| parent.parent() == root)
        .map(|(tuning, _)| tuning.max_torque.max(0.0))
        .sum();
    let controllers = world
        .query_filtered::<&ChildOf, (
            With<ControllerSectionMarker>,
            Without<SectionInactiveMarker>,
        )>()
        .iter(world)
        .filter(|parent| parent.parent() == root)
        .count();
    let thrusters = world
        .query_filtered::<&ChildOf, (With<ThrusterSectionMarker>, Without<SectionInactiveMarker>)>()
        .iter(world)
        .filter(|parent| parent.parent() == root)
        .count();
    let turrets = world
        .query_filtered::<&ChildOf, (With<TurretSectionMarker>, Without<SectionInactiveMarker>)>()
        .iter(world)
        .filter(|parent| parent.parent() == root)
        .count();
    let bays = world
        .query_filtered::<&ChildOf, (With<TorpedoSectionMarker>, Without<SectionInactiveMarker>)>()
        .iter(world)
        .filter(|parent| parent.parent() == root)
        .count();
    let railguns = world
        .query_filtered::<&ChildOf, (With<RailgunSectionMarker>, Without<SectionInactiveMarker>)>()
        .iter(world)
        .filter(|parent| parent.parent() == root)
        .count();

    // Engine boundary: both radii are measured off the sections' avian
    // colliders, so both arrive in world units.
    let envelope_model = AttitudeEnvelope::new(torque, inertia, Meters::from_engine(arm));
    HullInputs {
        cells,
        arm: Meters::from_engine(arm),
        envelope: Meters::from_engine(envelope),
        mass,
        inertia,
        torque,
        controllers,
        thrusters,
        weapons: turrets + bays + railguns,
        torque_ceiling: envelope_model.torque_ceiling,
        structural_ceiling: envelope_model.structural_ceiling,
        signature: Meters::from_engine(signature),
        lock_range: Meters::from_engine(signature * sensitivity),
    }
}

#[cfg(feature = "debug")]
fn log_hull(label: &str, hull: HullInputs) {
    info!(
        "hull scaling: {label}: cells={} arm={:.1} m envelope={:.1} m mass={:.0} kg \
         inertia={:.3e} torque={:.0} controllers={} thrusters={} weapons={} \
         torque_ceiling={:.4} rad/s2 structural_ceiling={:.4} rad/s2 binds={} headroom={:+.1}% \
         signature={:.0} m lock_range={:.1} km",
        hull.cells,
        hull.arm.get(),
        hull.envelope.get(),
        hull.mass,
        hull.inertia,
        hull.torque,
        hull.controllers,
        hull.thrusters,
        hull.weapons,
        hull.torque_ceiling,
        hull.structural_ceiling,
        hull.binds(),
        hull.headroom() * 100.0,
        hull.signature.get(),
        hull.lock_range.get() / 1_000.0,
    );
}

#[cfg(feature = "debug")]
fn hull_payload(hull: HullInputs) -> serde_json::Value {
    serde_json::json!({
        "cells": hull.cells,
        "arm_m": hull.arm.get(),
        "envelope_m": hull.envelope.get(),
        "mass_kg": hull.mass,
        "inertia": hull.inertia,
        "torque": hull.torque,
        "controllers": hull.controllers,
        "thrusters": hull.thrusters,
        "weapons": hull.weapons,
        "torque_ceiling": hull.torque_ceiling,
        "structural_ceiling": hull.structural_ceiling,
        "binds": hull.binds(),
        "headroom": hull.headroom(),
        "signature_m": hull.signature.get(),
        "lock_range_m": hull.lock_range.get(),
    })
}

/// The one beat: read both hulls, assert the premise the sweep rests on, and
/// record the table the ledger quotes.
#[cfg(feature = "debug")]
fn measure_both_hulls(world: &mut World) {
    let skiff = read_hull(world, SKIFF_ID);
    let carrier = read_hull(world, CARRIER_ID);
    log_hull(SKIFF, skiff);
    log_hull(CARRIER, carrier);

    let arm_ratio = carrier.arm.get() / skiff.arm.get().max(f32::EPSILON);
    let cell_ratio = carrier.cells as f32 / skiff.cells.max(1) as f32;
    assert!(
        arm_ratio >= ARM_RATIO_FLOOR && cell_ratio >= CELL_RATIO_FLOOR,
        "hull scaling: the sweep's premise is that a figure tuned on one reference hull is \
         not evidence about the other, but the carrier is only {arm_ratio:.1}x the skiff's \
         arm ({:.1} m against {:.1} m) over {cell_ratio:.1}x its sections ({} against {})",
        carrier.arm.get(),
        skiff.arm.get(),
        carrier.cells,
        skiff.cells,
    );
    nova_probe::probe_marker(
        world,
        "outcome: the reference hulls span the size the sweep assumes",
        serde_json::json!({
            "arm_ratio": arm_ratio,
            "cell_ratio": cell_ratio,
        }),
    );

    for (label, hull) in [(SKIFF, skiff), (CARRIER, carrier)] {
        assert!(
            hull.arm > Meters::ZERO
                && hull.inertia > 0.0
                && hull.structural_ceiling.is_finite()
                && hull.torque_ceiling.is_finite(),
            "hull scaling: {label} publishes no live attitude envelope (arm {:.2} m, \
             inertia {:.3e}, ceilings {:.4} / {:.4} rad/s2); every derived figure in the \
             sweep reads these and would fall back to a constant",
            hull.arm.get(),
            hull.inertia,
            hull.torque_ceiling,
            hull.structural_ceiling,
        );
    }
    nova_probe::probe_marker(
        world,
        "outcome: both hulls publish a live attitude envelope",
        serde_json::json!({
            "skiff_binds": skiff.binds(),
            "carrier_binds": carrier.binds(),
        }),
    );

    assert!(
        carrier.headroom() >= CARRIER_HEADROOM_FLOOR
            && carrier.headroom() <= CARRIER_HEADROOM_CEILING,
        "hull scaling: the shipped flight computer is pinned to this hull - ten of them are \
         supposed to put the intact carrier just over its structural ceiling - but it sits at \
         {:+.1}% ({:.4} rad/s2 of torque against {:.4} rad/s2 of structure, {:.0} of torque \
         over {:.3e} of inertia on a {:.1} m arm)",
        carrier.headroom() * 100.0,
        carrier.torque_ceiling,
        carrier.structural_ceiling,
        carrier.torque,
        carrier.inertia,
        carrier.arm.get(),
    );
    assert!(
        skiff.headroom() >= SKIFF_HEADROOM_FLOOR,
        "hull scaling: the same computer has to be invisible on a small hull, but the skiff \
         keeps only {:+.1}% over its structural ceiling ({:.4} against {:.4} rad/s2) - a \
         needle that can be made blunt by losing torque is a retune that went too far",
        skiff.headroom() * 100.0,
        skiff.torque_ceiling,
        skiff.structural_ceiling,
    );
    nova_probe::probe_marker(
        world,
        "outcome: the computer is pinned to the largest shipped hull",
        serde_json::json!({
            "carrier_headroom": carrier.headroom(),
            "skiff_headroom": skiff.headroom(),
            "carrier_torque": carrier.torque,
            "carrier_controllers": carrier.controllers,
        }),
    );

    let lock_ratio = carrier.lock_range.get() / skiff.lock_range.get().max(f32::EPSILON);
    assert!(
        skiff.signature > Meters::ZERO
            && carrier.signature > Meters::ZERO
            && lock_ratio >= LOCK_RANGE_RATIO_FLOOR,
        "hull scaling: a ship's radar return is supposed to come from its own structure, but \
         the carrier is lockable from only {lock_ratio:.1}x the skiff's range \
         ({:.1} km against {:.1} km, signatures {:.0} m against {:.0} m)",
        carrier.lock_range.get() / 1_000.0,
        skiff.lock_range.get() / 1_000.0,
        carrier.signature.get(),
        skiff.signature.get(),
    );
    nova_probe::probe_marker(
        world,
        "outcome: a bigger hull is seen from further away",
        serde_json::json!({
            "lock_range_ratio": lock_ratio,
            "skiff_lock_range_m": skiff.lock_range.get(),
            "carrier_lock_range_m": carrier.lock_range.get(),
        }),
    );

    nova_probe::probe_marker(
        world,
        "outcome: the hull inputs are recorded",
        serde_json::json!({
            SKIFF: hull_payload(skiff),
            CARRIER: hull_payload(carrier),
        }),
    );
    info!("hull scaling: both reference hulls measured");
}
