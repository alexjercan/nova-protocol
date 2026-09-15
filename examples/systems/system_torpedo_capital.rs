//! system_torpedo_capital: one shipped warhead delivered onto the two ends of
//! the hull scale.
//!
//! Task 20260909-213623. `system_torpedo_launch` already owns the launch chain:
//! clearance, arming, guidance, cruise and detonation against asteroid gates and
//! one crossing target. None of it asks what the ordnance does when the thing it
//! is closing on is a SHIP - a compound body whose skin is two thousand separate
//! colliders - or when it is the smallest ship there is. This range asks only
//! that, and it asks it as a matched pair fired from one bay.
//!
//! One controller-less battery sits at the origin with a single `torpedo_section`
//! and fires twice, under a scripted order rather than a trigger, so the range
//! says which hull each shot was committed to instead of letting a targeting
//! sweep choose:
//!
//! - shot one at `block_carrier`, 2 081 sections and ~420 m across;
//! - shot two at a one-section drone, the minimum a ship can be.
//!
//! Four claims, all measured against the run's own matched pair:
//!
//! - The fuze stands the warhead off the SKIN at both sizes. The standoff is the
//!   same small number on a 2 081-section capital as on a one-section drone, and
//!   neither torpedo ever touched the hull it was closing on. A centre-based
//!   fuze would put the capital's burst 190 m out in vacuum.
//! - The warhead stops short of the point it was AIMING at by the hull's own
//!   depth. A torpedo homes on `live_structure_anchor` - the target's live centre
//!   of mass - so on a capital the aim point sits deep inside the hull and the
//!   skin arrives first; on the minimum hull the two are the same place.
//! - The capital pays LOCALLY. A 300 m warhead on a 420 m hull spends itself on
//!   the sections it reached and leaves the rest of the ship whole, and the
//!   damage it did deal sits between the burst and the aim point rather than
//!   scattered over the hull.
//! - A one-section hull is fuzed, not overflown: the burst lands on the approach
//!   side of the drone and the drone's one section pays for it.
//!
//! What the shipped pair reads (2026-09-14, `block_carrier` at 2 081 sections):
//!
//! | | capital | one-section drone |
//! |---|---|---|
//! | standoff at the burst | 29.6 m | 26.8 m |
//! | distance to the aim point | 198.7 m | 33.0 m |
//! | aim point behind the face | 169.1 m | 6.2 m |
//! | containment radius | 194.3 m | 8.7 m |
//! | sections inside the 300 m sphere | 1 774 | 1 |
//! | sections that paid | 152 | 1 |
//!
//! So a warhead that geometrically swallows 1 774 of a carrier's sections is
//! paid for by 152 of them: the rest are behind something the pressure could not
//! get through. The damage runs 215 m into a hull the sphere reaches 299 m into.
//!
//! Why the standoff is a fixed number here: `torpedo_detonate_system` runs in
//! `FixedPostUpdate`, so its swept term is the fixed timestep and not the frame
//! rate (see the schedule comment in `torpedo_section/mod.rs`). At a Serpent's
//! 320 m/s against a parked hull the swept term is a third of a unit, so the
//! reach is the [`CONTACT_FUZE`] floor on every box this runs on. The range
//! measures that term anyway and asserts the floor governed, so a reach that
//! ever stopped being the floor shows up as a failure and not as a silent
//! widening of the window the other claims are read against.
//!
//! Controls: none. The shots are scripted; fly and look around freely.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_torpedo_capital --features debug
//! # look for: `torpedo capital: 'capital' fuzed ...`,
//! #           `torpedo capital: 'drone' fuzed ...`,
//! #           `torpedo capital: the capital paid on ... of its 2081 sections`,
//! #           `autopilot: cycle complete, no panic`
//! ```

#[cfg(feature = "debug")]
use avian3d::prelude::*;
use bevy::prelude::*;
use clap::Parser;
use nova_probe::fixtures::{self, prelude::*};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_torpedo_capital")]
#[command(version = "1.0.0")]
#[command(about = "A test range for one torpedo delivered onto a capital hull and onto the minimum hull. Autopilot-only correctness range", long_about = None)]
struct Cli;

/// The scenario the range loads under.
const SCENARIO_ID: &str = "torpedo_capital";

/// The battery that fires both shots, and the one bay on it.
const BATTERY_ID: &str = "battery";
const BAY_SECTION: &str = "bay";

/// The capital target and the shipped prototype it is built from.
const CAPITAL_ID: &str = "capital";
const CAPITAL_HULL: &str = "block_carrier";

/// The minimum hull: one section, nothing else.
const DRONE_ID: &str = "drone";
const DRONE_SECTION: &str = "drone_core";

/// Where the two targets are parked.
///
/// Same range and opposite bearings, so one bay reaches both without turning
/// the battery and neither shot's 300 m warhead can reach the other target: the
/// two hulls stand 1 km apart, and the capital's own 420 m of hull is inside
/// that gap. Far enough out that both shots arm and settle onto a guided run-in
/// long before the fuze window matters.
const CAPITAL_AT: Meters3 = Meters3::new(-500.0, 0.0, -1_300.0);
const DRONE_AT: Meters3 = Meters3::new(500.0, 0.0, -1_300.0);

/// How close the torpedo root gets to its target's skin before the warhead
/// fires, world units.
///
/// `CONTACT_FUZE` in `nova_ship/src/sections/torpedo_section/projectile.rs`,
/// which is private. Restated here because it is the number every standoff in
/// this range is read against: the live reach is
/// `CONTACT_FUZE.max(closing_speed * step)`, and the range asserts the swept
/// term stayed under the floor rather than assuming it.
#[cfg(feature = "debug")]
const CONTACT_FUZE: f32 = 3.0;

/// How far apart (world units) the two hull sizes' standoffs may sit.
///
/// The claim is that the standoff does NOT grow with the hull, so the band is
/// the whole fuze window: two bursts inside one [`CONTACT_FUZE`] of their own
/// skins are the same standoff however many sections the skin is made of.
#[cfg(feature = "debug")]
const STANDOFF_BAND: f32 = CONTACT_FUZE;

/// How many times further behind the face the capital's aim point sits than the
/// drone's.
///
/// A floor, not a measurement: measured on the shipped pair the capital's aim
/// point stands 165 m behind the face the warhead reached and the drone's 7 m,
/// which is 23x. Stated at 5 so the claim is about the ORDER of the difference
/// and not about `block_carrier`'s current section count.
#[cfg(feature = "debug")]
const AIM_DEPTH_RATIO: f32 = 5.0;

/// In-step seconds a beat gets to reach its world condition.
///
/// Well over the fleet's usual. A shipped capital renders and simulates at a
/// crawl on a software rasterizer, and each shot flies 1.4 km before it fuzes;
/// this is a backstop that NAMES a hung beat, not a budget the range is held to.
#[cfg(feature = "debug")]
const STEP_DEADLINE_SECS: f32 = 120.0;

/// Frames a burst gets to finish resolving.
///
/// The blast is a sensor sphere: it is spawned in `FixedPostUpdate`, its
/// overlaps are collected on the next physics tick and the pressure resolves
/// after that, so a reading taken on the frame the sphere appeared is a reading
/// of nothing.
#[cfg(feature = "debug")]
const SETTLE_FRAMES: u32 = 12;

/// The script type, named once so the step list and its helpers agree.
#[cfg(feature = "debug")]
type Script = nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(range_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.init_resource::<Shot>();
        app.init_resource::<LastSample>();
        app.init_resource::<Deliveries>();
        app.init_resource::<Paid>();
        app.init_resource::<Contacts>();
        app.add_observer(record_delivery);
        app.add_observer(record_paid);
        app.add_observer(record_contact);
        // Between physics finishing and the fuze reading it: the sample is the
        // same pose, the same velocity and the same step the fuze is about to
        // test, so the reach this range reports is the reach production used.
        app.add_systems(
            FixedPostUpdate,
            sample_the_torpedo
                .after(PhysicsSystems::Last)
                .before(SpaceshipSectionSystems),
        );
        // No frame-time pass: a 2 081-section capital plus a live warhead is not
        // a frame budget anyone should read, and this range claims nothing about
        // one.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(assert_scenario_loaded(SCENARIO_ID));
        app.add_plugins(nova_screenshot(capital_script()));
    }

    app.run()
}

fn range_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_range);
}

fn setup_range(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(capital_range(&game_assets, &sections)));
}

/// The scene: one battery at the origin, a capital off one bow and the minimum
/// hull off the other.
fn capital_range(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    // Controller-less on purpose. A player or AI hull rewrites its own bay
    // trigger and picks its own target every frame, and this range has to say
    // WHICH hull each shot was committed to. `shoot_spawn_projectile` lets an
    // unmanaged ship fire freely, so the scripted order is the whole brain.
    let battery = fixtures::ship(
        sections,
        SpaceshipController::None,
        &[
            SectionSpec::new("controller", "basic_controller_section", Vec3::ZERO),
            SectionSpec::new("hull", "reinforced_hull_section", Vec3::new(0.0, 0.0, 1.0)),
            // The bay is 1x1x2: centred at -1.5 its two cells sit on the grid
            // and its aft socket mates the controller. Unlimited, so the second
            // shot does not wait out the bay's ten-second reload.
            SectionSpec::new(BAY_SECTION, "torpedo_section", Vec3::new(0.0, 0.0, -1.5)).unlimited(),
        ],
    );

    let drone = SpaceshipConfig {
        controller: SpaceshipController::None,
        design: ShipDesignSource::Inline(ShipDesign {
            sections: vec![SpaceshipSectionConfig {
                id: DRONE_SECTION.to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                source: SectionSource::Inline(
                    sections
                        .get_section("basic_controller_section")
                        .expect("section 'basic_controller_section' not found")
                        .clone(),
                ),
            }],
            ..default()
        }),
        ..default()
    };

    let capital = SpaceshipConfig {
        controller: SpaceshipController::None,
        design: ShipDesignSource::prototype(CAPITAL_HULL),
        ..default()
    };

    let spawn = |id: &str, name: &str, position: Meters3, ship: SpaceshipConfig| {
        EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: id.to_string(),
                name: name.to_string(),
                position,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Spaceship(ship),
        })
    };

    let events = vec![ScenarioEventConfig {
        label: None,
        name: EventConfig::OnStart,
        once: false,
        filters: vec![],
        actions: [
            vec![
                spawn(BATTERY_ID, "Torpedo Battery", Meters3::ZERO, battery),
                spawn(CAPITAL_ID, "Fleet Carrier", CAPITAL_AT, capital),
                spawn(DRONE_ID, "Survey Drone", DRONE_AT, drone),
            ],
            // The range lights itself: the engine spawns no light, and an unlit
            // scene shoots black.
            ThreePointRig::around("capital", Meters3::ZERO, 90.0).actions(),
        ]
        .concat(),
    }];

    ScenarioConfig {
        description: "A test range for one torpedo delivered onto a capital hull and onto the \
                      minimum hull."
            .to_string(),
        events,
        ..ScenarioConfig::new(
            SCENARIO_ID.to_string(),
            "Torpedo Capital Range".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

// --- What the range records --------------------------------------------------

/// The hull the live shot is committed to.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct Shot {
    target: Option<Entity>,
    id: &'static str,
}

/// The last look physics' own clock got at the torpedo in flight: the velocity
/// and the step the fuze is about to test against.
#[cfg(feature = "debug")]
#[derive(Clone, Copy, Debug, Default)]
struct TorpedoSample {
    velocity: Vec3,
    closing: f32,
    step: f32,
}

#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct LastSample(Option<TorpedoSample>);

/// One warhead's delivery, measured on the tick the blast appeared and before
/// its pressure resolved - the near face is about to stop existing.
#[cfg(feature = "debug")]
#[derive(Clone, Debug)]
struct Delivery {
    /// Where the warhead went off.
    at: Vec3,
    /// The point the torpedo was homing on: the target's live structure anchor.
    aim: Vec3,
    /// The blast's own radius, world units.
    radius: f32,
    /// The direction the torpedo was travelling when it fuzed.
    approach: Vec3,
    /// Distance from the burst to the nearest box of the target's own colliders
    /// - the set the fuze itself walks.
    skin_gap: f32,
    /// Distance from the burst to the point the torpedo was homing on.
    aim_gap: f32,
    /// The target's containment radius, world units.
    envelope: f32,
    /// How many sections the target still had.
    sections: usize,
    /// How many of the target's sections had their collider centre inside the
    /// blast sphere.
    inside: usize,
    /// How deep the furthest of those sections stood, measured along the line
    /// the torpedo was flying. The reach of the sphere INTO the hull, as
    /// opposed to the reach of the pressure.
    inside_depth: f32,
    /// The swept term of the live fuze reach: closing speed times the step.
    swept: f32,
}

#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct Deliveries(std::collections::BTreeMap<&'static str, Delivery>);

/// What the live shot's target paid, section by section: the section, its
/// collider centre and the total it took.
///
/// Cleared when a shot is ordered and filtered to that shot's target, so a
/// capital still shedding wreckage cannot land in the drone's bucket.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct Paid(std::collections::BTreeMap<Entity, (Vec3, f32)>);

/// How many times a torpedo body touched the hull it was ordered onto. A
/// contact dud is the failure the fuze claim exists to catch, and it is not
/// visible in the blast geometry: a torpedo that touches first dies without one.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct Contacts(usize);

// --- Instrumentation ---------------------------------------------------------

/// Sample the one torpedo in flight on the clock the fuze reads.
#[cfg(feature = "debug")]
fn sample_the_torpedo(
    time: Res<Time>,
    q_torpedo: Query<
        (Option<&LinearVelocity>, Option<&TorpedoTargetEntity>),
        With<TorpedoProjectileMarker>,
    >,
    q_motion: Query<&LinearVelocity>,
    mut sample: ResMut<LastSample>,
) {
    let Some((velocity, target)) = q_torpedo.iter().next() else {
        return;
    };
    let velocity = velocity.map_or(Vec3::ZERO, |velocity| velocity.0);
    let target_velocity = target
        .and_then(|target| q_motion.get(**target).ok())
        .map_or(Vec3::ZERO, |velocity| velocity.0);
    sample.0 = Some(TorpedoSample {
        velocity,
        closing: (velocity - target_velocity).length(),
        step: time.delta_secs(),
    });
}

/// Measure the delivery the instant the warhead goes off.
///
/// The blast is spawned by `torpedo_detonate_system` and resolves its pressure
/// on a later tick, so this observer is the only place the near face still
/// exists to be measured. First burst per ordered shot only: a dying hull can
/// put more explosives in the sky, and the claim is about the warhead.
#[cfg(feature = "debug")]
fn record_delivery(
    blast: On<Add, NovaBlast>,
    q_blast: Query<(&Transform, &NovaBlast)>,
    q_anchor: Query<(
        &Transform,
        Option<&ComputedCenterOfMass>,
        Option<&HullEnvelopeRadius>,
    )>,
    q_sections: Query<(&ChildOf, &ColliderAabb), With<SectionMarker>>,
    q_body_colliders: Query<&RigidBodyColliders>,
    q_aabb: Query<&ColliderAabb>,
    shot: Res<Shot>,
    sample: Res<LastSample>,
    mut deliveries: ResMut<Deliveries>,
) {
    let Some(target) = shot.target else {
        return;
    };
    if deliveries.0.contains_key(shot.id) {
        return;
    }
    let Ok((transform, area)) = q_blast.get(blast.entity) else {
        return;
    };
    let Ok((target_transform, com, envelope)) = q_anchor.get(target) else {
        return;
    };
    let at = transform.translation;
    let aim = live_structure_anchor(target_transform, com);
    let sample = sample.0.unwrap_or_default();
    let approach = sample.velocity.normalize_or_zero();

    // The skin comes off the body's OWN collider list, which is the set the
    // fuze walks (`distance_to_skin`). Reading it off the root's section
    // children instead misses whatever else is bolted to the hull, and on
    // `block_carrier` that difference is a fifth of a unit - enough to put the
    // burst outside a window it is actually inside.
    //
    // Box distance, not the shape's: an AABB encloses its collider, so this can
    // only read SHORTER than the fuze's own reading, which is the safe
    // direction for a claim that the burst is near the skin.
    let mut skin_gap = f32::INFINITY;
    for collider in q_body_colliders.get(target).into_iter().flatten() {
        let Ok(aabb) = q_aabb.get(collider) else {
            continue;
        };
        let outside = (aabb.min - at).max(at - aabb.max).max(Vec3::ZERO);
        skin_gap = skin_gap.min(outside.length());
    }

    let mut sections = 0usize;
    let mut inside = 0usize;
    let mut inside_depth = 0.0f32;
    for (ChildOf(parent), aabb) in &q_sections {
        if *parent != target {
            continue;
        }
        sections += 1;
        let centre = (aabb.min + aabb.max) / 2.0;
        if centre.distance(at) < area.radius {
            inside += 1;
            inside_depth = inside_depth.max((centre - at).dot(approach));
        }
    }

    deliveries.0.insert(
        shot.id,
        Delivery {
            at,
            aim,
            radius: area.radius,
            approach,
            skin_gap,
            aim_gap: at.distance(aim),
            envelope: envelope.map_or(0.0, |envelope| **envelope),
            sections,
            inside,
            inside_depth,
            swept: sample.closing * sample.step,
        },
    );
}

/// Record what the live shot's target paid.
#[cfg(feature = "debug")]
fn record_paid(
    damage: On<HealthApplyDamage>,
    q_section: Query<(&ChildOf, &ColliderAabb), With<SectionMarker>>,
    shot: Res<Shot>,
    mut paid: ResMut<Paid>,
) {
    // The event propagates up `ChildOf`; only the entity it landed on paid.
    if damage.entity != damage.original_event_target() {
        return;
    }
    let Ok((ChildOf(parent), aabb)) = q_section.get(damage.entity) else {
        return;
    };
    if shot.target != Some(*parent) {
        return;
    }
    let entry = paid
        .0
        .entry(damage.entity)
        .or_insert(((aabb.min + aabb.max) / 2.0, 0.0));
    entry.1 += damage.amount;
}

/// Count a torpedo body touching the hull it was ordered onto.
#[cfg(feature = "debug")]
fn record_contact(
    collision: On<CollisionStart>,
    q_torpedo: Query<(), With<TorpedoProjectileMarker>>,
    shot: Res<Shot>,
    mut contacts: ResMut<Contacts>,
) {
    let Some((first, second)) = collision.body1.zip(collision.body2) else {
        return;
    };
    let touched = (q_torpedo.contains(first) && shot.target == Some(second))
        || (q_torpedo.contains(second) && shot.target == Some(first));
    if touched {
        contacts.0 += 1;
    }
}

// --- The script --------------------------------------------------------------

/// The run: one shot at the capital, one at the minimum hull, then the pair read
/// against each other.
#[cfg(feature = "debug")]
fn capital_script() -> Script {
    Script::new()
        .step("load the range")
        .enter(GameStates::Loading)
        .until(and(
            ship_present(BATTERY_ID),
            and(ship_present(CAPITAL_ID), ship_present(DRONE_ID)),
        ))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("order the shot at the capital")
        .on_enter(order_the_shot(CAPITAL_ID))
        .until(the_warhead_landed(CAPITAL_ID))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("let the capital's burst resolve")
        .until(frames(SETTLE_FRAMES))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("read the capital")
        .on_enter(assert_the_capital_pays_locally)
        .until(frames(1))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("order the shot at the minimum hull")
        .on_enter(order_the_shot(DRONE_ID))
        .until(the_warhead_landed(DRONE_ID))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("let the drone's burst resolve")
        .until(frames(SETTLE_FRAMES))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("read the minimum hull")
        .on_enter(assert_the_minimum_hull_is_not_overflown)
        .until(frames(1))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("read the pair")
        .on_enter(assert_the_fuze_reads_the_skin)
        .until(frames(1))
        .deadline(STEP_DEADLINE_SECS)
        .add()
}

// --- Staging -----------------------------------------------------------------

/// Commit the bay's next launch to `id`.
#[cfg(feature = "debug")]
fn order_the_shot(id: &'static str) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| {
        let target = object_by_id(world, id)
            .unwrap_or_else(|| panic!("torpedo capital: '{id}' must be in the sky to be shot at"));
        let bay = bay_section(world).expect("torpedo capital: the battery has no bay");
        world
            .entity_mut(bay)
            .insert(ScriptedTorpedoOrder { target });
        world.resource_mut::<Paid>().0.clear();
        world.resource_mut::<Contacts>().0 = 0;
        *world.resource_mut::<Shot>() = Shot {
            target: Some(target),
            id,
        };
        info!("torpedo capital: the bay is ordered onto '{id}'");
    }
}

// --- Claims ------------------------------------------------------------------

/// The capital's reading: the hit is local, and the damage sits between the
/// burst and the point the torpedo was aiming at.
#[cfg(feature = "debug")]
fn assert_the_capital_pays_locally(world: &mut World) {
    let paid: Vec<(Vec3, f32)> = world.resource::<Paid>().0.values().copied().collect();
    let delivery = delivery(world, CAPITAL_ID);

    assert!(
        !paid.is_empty(),
        "torpedo capital: the capital took nothing from a {:.0} m warhead {:.0} m off its skin",
        metres(delivery.radius),
        metres(delivery.skin_gap)
    );
    let deepest = paid
        .iter()
        .map(|(centre, _)| centre.distance(delivery.at))
        .fold(0.0f32, f32::max);
    assert!(
        deepest <= delivery.radius,
        "torpedo capital: a section {:.0} m out paid for a {:.0} m blast",
        metres(deepest),
        metres(delivery.radius)
    );
    // A capital is not a target one warhead consumes. The claim is that the hit
    // is LOCAL: the sections that paid are the ones the sphere reached, and the
    // rest of the ship is untouched.
    assert!(
        paid.len() < delivery.sections,
        "torpedo capital: every one of the capital's {} sections paid for one warhead",
        delivery.sections
    );
    // And local INSIDE the sphere too, which is the part a radius alone cannot
    // give you: the sphere swallows most of the forward hull, and the pressure
    // stops at the first section it cannot destroy. Most of what the warhead
    // reached geometrically is shielded by what stood in front of it, so a
    // strict majority of the sections inside the sphere pay nothing at all.
    assert!(
        paid.len() * 2 < delivery.inside,
        "torpedo capital: {} of the {} sections inside the sphere paid - the pressure is not \
         stopping on the hull it is going through",
        paid.len(),
        delivery.inside
    );

    // Where the damage sits: forward of the burst along the line the torpedo
    // was flying, and short of the aim point. A torpedo homes on the live
    // structure anchor, so on a hull this deep the aim point is the far side of
    // the material that stopped the warhead.
    let centroid = paid
        .iter()
        .fold(Vec3::ZERO, |sum, (centre, _)| sum + *centre)
        / paid.len() as f32;
    let depth = (centroid - delivery.at).dot(delivery.approach);
    let aim_depth = (delivery.aim - delivery.at).dot(delivery.approach);
    let paid_depth = paid
        .iter()
        .map(|(centre, _)| (*centre - delivery.at).dot(delivery.approach))
        .fold(f32::NEG_INFINITY, f32::max);
    assert!(
        depth > 0.0,
        "torpedo capital: the damage centroid sits {:.0} m BEHIND the burst",
        metres(-depth)
    );
    assert!(
        depth < aim_depth,
        "torpedo capital: the damage centroid sits {:.0} m in, past the {:.0} m aim point",
        metres(depth),
        metres(aim_depth)
    );
    // The far edge of the damage against the far edge of the sphere: the
    // pressure died short of where the geometry reached, which is the layer
    // rule seen from the outside.
    assert!(
        paid_depth < delivery.inside_depth,
        "torpedo capital: the damage runs {:.0} m in and the sphere {:.0} m - the blast went as \
         deep as its own radius",
        metres(paid_depth),
        metres(delivery.inside_depth)
    );

    let spared = delivery.inside.saturating_sub(paid.len());
    info!(
        "torpedo capital: the capital paid on {} of its {} sections; {spared} of the {} inside \
         the {:.0} m sphere were spared; the damage runs {:.0} m into a hull the sphere reaches \
         {:.0} m into, centroid {:.0} m in and {:.0} m short of the aim point",
        paid.len(),
        delivery.sections,
        delivery.inside,
        metres(delivery.radius),
        metres(paid_depth),
        metres(delivery.inside_depth),
        metres(depth),
        metres(aim_depth - depth),
    );
    nova_probe::probe_marker(
        world,
        "outcome: the capital pays on the face the warhead reached",
        serde_json::json!({
            "sections": delivery.sections,
            "inside_the_sphere": delivery.inside,
            "paid": paid.len(),
            "spared_inside": spared,
            "centroid_depth_m": metres(depth),
            "damage_depth_m": metres(paid_depth),
            "sphere_depth_m": metres(delivery.inside_depth),
            "aim_depth_m": metres(aim_depth),
        }),
    );
}

/// The minimum hull's reading: the burst lands short of it along the line the
/// torpedo was flying, and its one section pays.
#[cfg(feature = "debug")]
fn assert_the_minimum_hull_is_not_overflown(world: &mut World) {
    let paid: Vec<(Vec3, f32)> = world.resource::<Paid>().0.values().copied().collect();
    let delivery = delivery(world, DRONE_ID);

    assert!(
        delivery.approach.length_squared() > 0.0,
        "torpedo capital: the drone's torpedo was never sampled in flight"
    );
    // Short of the hull along the line it was flying. A warhead that stepped
    // over its own window goes off PAST the target, with the hull behind it,
    // which is what "overflown" means for a body one cell across.
    let past = (delivery.at - delivery.aim).dot(delivery.approach);
    assert!(
        past < 0.0,
        "torpedo capital: the drone's burst went off {:.0} m past its aim point",
        metres(past)
    );
    assert!(
        !paid.is_empty(),
        "torpedo capital: the drone took nothing from a warhead {:.0} m off its skin",
        metres(delivery.skin_gap)
    );

    info!(
        "torpedo capital: the drone was fuzed {:.0} m off its skin, {:.0} m short of its aim \
         point, and its {} section(s) paid {:.0} points",
        metres(delivery.skin_gap),
        metres(-past),
        paid.len(),
        paid.iter().map(|(_, amount)| amount).sum::<f32>(),
    );
    nova_probe::probe_marker(
        world,
        "outcome: a one-section hull is fuzed, not overflown",
        serde_json::json!({
            "skin_gap_m": metres(delivery.skin_gap),
            "short_by_m": metres(-past),
            "sections_paid": paid.len(),
        }),
    );
}

/// The pair, read against each other: the standoff belongs to the skin, and the
/// depth of the aim point belongs to the hull.
#[cfg(feature = "debug")]
fn assert_the_fuze_reads_the_skin(world: &mut World) {
    let contacts = world.resource::<Contacts>().0;
    let capital = delivery(world, CAPITAL_ID);
    let drone = delivery(world, DRONE_ID);

    // The reach the fuze actually used. On the fixed clock the swept term is a
    // constant, and at a Serpent's cruise it is a fraction of the floor - so
    // the standoffs below are read against the floor, and this says so out loud
    // rather than assuming it.
    for (id, delivery) in [(CAPITAL_ID, &capital), (DRONE_ID, &drone)] {
        assert!(
            delivery.swept < CONTACT_FUZE,
            "torpedo capital: '{id}' swept {:.0} m in one step, so the fuze window was not the \
             {:.0} m floor these standoffs are read against",
            metres(delivery.swept),
            metres(CONTACT_FUZE)
        );
        assert!(
            delivery.skin_gap < CONTACT_FUZE,
            "torpedo capital: '{id}' fuzed {:.0} m off its skin, outside the {:.0} m window",
            metres(delivery.skin_gap),
            metres(CONTACT_FUZE)
        );
        info!(
            "torpedo capital: '{id}' fuzed {:.0} m off a {} section hull, {:.0} m from the point \
             it was aiming at; that hull's own containment radius is {:.0} m, and the fuze swept \
             {:.1} m in its last step against a {:.0} m floor",
            metres(delivery.skin_gap),
            delivery.sections,
            metres(delivery.aim_gap),
            metres(delivery.envelope),
            metres(delivery.swept),
            metres(CONTACT_FUZE),
        );
    }
    assert_eq!(
        contacts, 0,
        "torpedo capital: a torpedo touched the hull it was closing on before its warhead fired"
    );
    // The point of the pair: the standoff belongs to the SKIN, so it does not
    // grow with the hull behind it.
    let spread = (capital.skin_gap - drone.skin_gap).abs();
    assert!(
        spread <= STANDOFF_BAND,
        "torpedo capital: the capital stood off {:.0} m and the minimum hull {:.0} m - {:.0} m \
         apart, wider than the {:.0} m window",
        metres(capital.skin_gap),
        metres(drone.skin_gap),
        metres(spread),
        metres(STANDOFF_BAND)
    );
    info!(
        "torpedo capital: {} sections and {} sections, and the two standoffs are {:.0} m apart",
        capital.sections,
        drone.sections,
        metres(spread),
    );
    nova_probe::probe_marker(
        world,
        "outcome: the fuze stands off the skin of both hull sizes",
        serde_json::json!({
            "capital_sections": capital.sections,
            "capital_skin_gap_m": metres(capital.skin_gap),
            "drone_sections": drone.sections,
            "drone_skin_gap_m": metres(drone.skin_gap),
            "spread_m": metres(spread),
            "contacts": contacts,
        }),
    );

    // And the aim point belongs to the HULL. A torpedo homes on the target's
    // live structure anchor, which on a capital sits most of a hull radius
    // behind the face the warhead reached and on the minimum hull sits half a
    // cell behind it.
    let capital_depth = capital.aim_gap - capital.skin_gap;
    let drone_depth = drone.aim_gap - drone.skin_gap;
    assert!(
        capital_depth > drone_depth * AIM_DEPTH_RATIO,
        "torpedo capital: the capital's aim point sat {:.0} m behind the face the warhead reached \
         and the drone's {:.0} m - the warhead is not stopping on the hull in front of it",
        metres(capital_depth),
        metres(drone_depth)
    );
    assert!(
        capital.aim_gap > capital.envelope * 0.25,
        "torpedo capital: the capital's burst stood {:.0} m from its aim point inside a hull of \
         {:.0} m containment radius",
        metres(capital.aim_gap),
        metres(capital.envelope)
    );
    info!(
        "torpedo capital: the capital's aim point sat {:.0} m past the face the warhead reached, \
         the drone's {:.0} m",
        metres(capital_depth),
        metres(drone_depth),
    );
    nova_probe::probe_marker(
        world,
        "outcome: the warhead stops on the face, not on the aim point",
        serde_json::json!({
            "capital_aim_gap_m": metres(capital.aim_gap),
            "capital_envelope_m": metres(capital.envelope),
            "capital_depth_behind_face_m": metres(capital_depth),
            "drone_aim_gap_m": metres(drone.aim_gap),
            "drone_depth_behind_face_m": metres(drone_depth),
        }),
    );
}

// --- Predicates --------------------------------------------------------------

/// A scenario object with `id` is in the sky.
#[cfg(feature = "debug")]
fn ship_present(id: &'static str) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| object_by_id(world, id).is_some())
}

/// The warhead ordered onto `id` has gone off and been measured.
#[cfg(feature = "debug")]
fn the_warhead_landed(
    id: &'static str,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        world
            .get_resource::<Deliveries>()
            .is_some_and(|deliveries| deliveries.0.contains_key(id))
    })
}

// --- Readings ----------------------------------------------------------------

/// What one shot delivered, lifted out of the world so the claim can log and
/// mark against it.
#[cfg(feature = "debug")]
fn delivery(world: &mut World, id: &str) -> Delivery {
    world
        .resource::<Deliveries>()
        .0
        .get(id)
        .unwrap_or_else(|| panic!("torpedo capital: '{id}' has no delivery - it was never fuzed"))
        .clone()
}

/// World units as the meters a player reads.
#[cfg(feature = "debug")]
fn metres(units: f32) -> f32 {
    Meters::from_engine(units).get()
}

/// The scenario object with `id`.
#[cfg(feature = "debug")]
fn object_by_id(world: &World, id: &str) -> Option<Entity> {
    world
        .try_query::<(Entity, &EntityId)>()
        .and_then(|mut query| {
            query
                .iter(world)
                .find(|(_, entity_id)| ***entity_id == *id)
                .map(|(entity, _)| entity)
        })
}

/// The battery's one torpedo bay.
#[cfg(feature = "debug")]
fn bay_section(world: &mut World) -> Option<Entity> {
    world
        .query_filtered::<(Entity, &EntityId), With<TorpedoSectionInput>>()
        .iter(world)
        .find(|(_, entity_id)| ***entity_id == *BAY_SECTION)
        .map(|(entity, _)| entity)
}
