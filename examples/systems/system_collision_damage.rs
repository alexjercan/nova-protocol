//! system_collision_damage: what a contact costs, from a docking touch to a
//! head-on ram, at both reference hull sizes.
//!
//! Collision damage is the one weapon nobody fires. Two hulls that touch trade
//! hit points through the same health store a turret writes to, so the rule
//! that decides WHEN a touch is a ram has to hold for a needle nudging a crate
//! and for two capitals coming alongside. It used to be a speed floor of
//! 3.16 m/s over damage that was linear in mass, and a carrier pair closing at
//! 3.2 m/s - a docking approach - traded about 49 hit points per contact over
//! hundreds of contacts a frame, which shredded both ships while they were
//! parking. Task `20260909-213708` replaced it with a universal 5 m/s safe
//! contact speed above which only the EXCESS is spent.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: two capitals may come alongside for free` | a `block_carrier` pair closing at a docking speed rests against itself without trading a hit point |
//! | 2 | `outcome: a touch under the safe speed is free at either hull size` | the same holds for a `block_skiff` pair, so the threshold is a speed and not a mass |
//! | 3 | `outcome: a ram spends hit points on both bodies` | a skiff pair over the safe speed damages BOTH sides of the contact |
//! | 4 | `outcome: the bite grows with the closing speed` | twice the closing speed costs more than twice the hit points, because the energy term is quadratic |
//! | 5 | `outcome: a ram on a rock is paid out of both durabilities` | a hull flown into an asteroid spends hit points AND opens the rock's carve field, because the two structures keep their durability in different stores |
//! | 6 | `outcome: a destructive ram leaves a wreck` | a contact hard enough to destroy structure leaves detached wreck bodies in its own lane |
//! | 7 | `outcome: the contact census is recorded` | RECORD: closing speed, fixed steps spent touching, peak contacts held in one step, and what each side paid, per pair |
//!
//! Claim 7 asserts NOTHING. It is the table `tasks/20260909-213118/FEEDBACK.md`
//! quotes, read against the figures there and never against a threshold.
//!
//! # A rock does not keep hit points
//!
//! An asteroid carries no `Health` at all: its durability IS its carve field,
//! the `DamageMarks` list on its collider node, and `apply_damage` writes both
//! stores from the one call a ram makes. So "both structures pay" reads a
//! different ledger on each side of that contact - hit points on the hull,
//! crater volume on the rock - and a census that only summed health would read
//! a rock ram as one-sided.
//!
//! Damage below the shipped safe contact speed is pinned per FORMULA in
//! `nova_gameplay::integrity::core`, including the one case no scene can stage:
//! a hull is ONE rigid body wearing dozens of overlapping health-bearing
//! colliders, and
//! `one_body_wearing_two_overlapping_colliders_never_damages_itself` pins that
//! such a body never grinds itself however hard it is thrown or spun.
//!
//! Every pair is placed nose to nose by the script rather than by the scenario:
//! the gap that matters is between the two hull ENVELOPES, and a carrier's
//! envelope is not known until avian has linked its colliders.
//!
//! What a pair loses is summed over every health pool under its roots and not
//! off the root's own roll-up, because the outermost thing a shipped skin puts
//! in a contact's way is a decor patch with a pool of its own: at these speeds
//! the two skiffs meet scab to scab, and a census that counted sections alone
//! would read a ram as free.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_collision_damage --features debug
//! # look for: `collision damage: block_carrier pair: ...`,
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
#[command(name = "system_collision_damage")]
#[command(version = "1.0.0")]
#[command(
    about = "What a contact costs, from a docking touch to a head-on ram, at both reference hull sizes. Autopilot-only correctness range",
    long_about = None
)]
struct Cli;

/// The safe contact speed the whole range is graded against, mirrored from
/// `nova_gameplay/src/integrity/core.rs`. Below it a touch is free whatever is
/// touching; above it, only the excess is spent.
const SAFE_CONTACT_SPEED: MetersPerSecond = MetersPerSecond(5.0);

/// The large reference hull: the campaign's home, and the largest hull the base
/// game ships.
const CARRIER: &str = "block_carrier";

/// The small reference hull: the cleanup group's unarmed needle.
const SKIFF: &str = "block_skiff";

/// What one side of a staged contact is made of.
#[derive(Clone, Copy)]
enum Side {
    /// A catalog hull, spawned unflown and unsteered. Its durability is the
    /// health pools under its root.
    Hull(&'static str),
    /// A rock of this nominal radius. Its durability is the carve field on its
    /// collider node - an asteroid carries no `Health` at all.
    Rock(Meters),
}

/// Which ledger a staged contact is read out of.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Reading {
    /// Hit points, both sides. The four speed-threshold pairs.
    Hulls,
    /// Hit points on the hull and crater volume on the rock.
    Rock,
    /// Wreckage. The contact is meant to destroy structure, so neither root is
    /// promised to survive long enough to be weighed.
    Wreck,
}

/// One staged contact: two bodies nose to nose on the x axis, closing at
/// `closing`.
struct RamPair {
    /// What the log and the census call this contact.
    label: &'static str,
    /// What starts on -x.
    left: Side,
    /// What starts on +x.
    right: Side,
    /// Scenario id of the side that starts on -x.
    left_id: &'static str,
    /// Scenario id of the side that starts on +x.
    right_id: &'static str,
    /// How fast the two close on each other.
    closing: MetersPerSecond,
    /// Where the pair is staged, so no pair's debris reaches another's.
    lane: Meters,
    /// Which ledger this contact is read out of.
    reads: Reading,
}

/// The six staged contacts.
///
/// The first four are the speed threshold: two under the safe speed and two
/// over it, at both hull sizes, because the rule under test is that the
/// threshold is a SPEED - a capital pair coming alongside at a docking speed
/// and a needle pair nudging each other have to get the same answer.
///
/// The last two are what a ram COSTS once it is one: a rock has to pay out of
/// the only durability it has, and a contact hard enough to destroy structure
/// has to leave that structure behind as wreckage rather than deleting it.
const PAIRS: [RamPair; 6] = [
    RamPair {
        label: "carrier dock",
        left: Side::Hull(CARRIER),
        right: Side::Hull(CARRIER),
        left_id: "dock_carrier_left",
        right_id: "dock_carrier_right",
        // The task's own case: 0.32 u/s, well inside what a docking clamp
        // takes, and the approach that used to cost both ships their hulls.
        closing: MetersPerSecond(3.2),
        lane: Meters(0.0),
        reads: Reading::Hulls,
    },
    RamPair {
        label: "skiff touch",
        left: Side::Hull(SKIFF),
        right: Side::Hull(SKIFF),
        left_id: "touch_skiff_left",
        right_id: "touch_skiff_right",
        closing: MetersPerSecond(4.0),
        lane: Meters(6_000.0),
        reads: Reading::Hulls,
    },
    RamPair {
        label: "skiff ram",
        left: Side::Hull(SKIFF),
        right: Side::Hull(SKIFF),
        left_id: "ram_skiff_left",
        right_id: "ram_skiff_right",
        closing: MetersPerSecond(15.0),
        lane: Meters(12_000.0),
        reads: Reading::Hulls,
    },
    RamPair {
        label: "skiff hard ram",
        left: Side::Hull(SKIFF),
        right: Side::Hull(SKIFF),
        left_id: "hard_skiff_left",
        right_id: "hard_skiff_right",
        closing: MetersPerSecond(30.0),
        lane: Meters(18_000.0),
        reads: Reading::Hulls,
    },
    RamPair {
        label: "skiff into rock",
        left: Side::Hull(SKIFF),
        right: Side::Rock(ROCK_RADIUS),
        left_id: "rock_ram_skiff",
        right_id: "rock_ram_rock",
        closing: MetersPerSecond(30.0),
        lane: Meters(24_000.0),
        reads: Reading::Rock,
    },
    RamPair {
        label: "wrecking ram",
        left: Side::Hull(SKIFF),
        right: Side::Hull(CARRIER),
        left_id: "wreck_ram_skiff",
        right_id: "wreck_ram_carrier",
        closing: WRECKING_SPEED,
        lane: Meters(30_000.0),
        reads: Reading::Wreck,
    },
];

/// The rock the hull is flown into.
///
/// Well under `GravitySettings::min_well_radius` and authored with no mass, so
/// it stays a plain dynamic body: a designated well is put on rails and would
/// pull the lanes either side of it out of shape.
const ROCK_RADIUS: Meters = Meters(40.0);

/// How hard the destructive contact is.
///
/// Well past the point where a section survives, because the cost of a ram is
/// not monotonic in the closing speed and a figure just over the line is not a
/// figure at all. Measured on this pair: 200 m/s spends 79 hit points and
/// 400 m/s spends 58 - faster hulls separate sooner and press shallower, so the
/// middle of the curve dips. This far up, one step
/// of overlap alone settles more than a dozen contacts at once and takes whole
/// sections with it. The claim is that destroyed structure leaves a body
/// behind, not that this exact speed destroys this exact section.
const WRECKING_SPEED: MetersPerSecond = MetersPerSecond(800.0);

/// Index of the carrier pair in [`PAIRS`].
#[cfg(feature = "debug")]
const CARRIER_DOCK: usize = 0;

/// Index of the skiff pair under the safe speed.
#[cfg(feature = "debug")]
const SKIFF_TOUCH: usize = 1;

/// Index of the skiff pair over it.
#[cfg(feature = "debug")]
const SKIFF_RAM: usize = 2;

/// Index of the skiff pair at twice that again.
#[cfg(feature = "debug")]
const SKIFF_HARD_RAM: usize = 3;

/// Index of the hull flown into a rock.
#[cfg(feature = "debug")]
const ROCK_RAM: usize = 4;

/// Index of the contact meant to destroy structure.
#[cfg(feature = "debug")]
const WRECKING_RAM: usize = 5;

/// Where a pair is staged before the script places it, m from the lane centre.
/// Clear of the largest envelope the catalog ships, so no pair starts inside
/// itself while it waits to be weighed.
const STAGING_HALF_GAP: Meters = Meters(600.0);

/// How long every pair is from contact when the script lets it go, s.
///
/// A TIME and not a distance: four pairs closing at four speeds over one gap
/// would meet at four different moments, and the slow one is the pair the
/// range exists for. Each gap is this many seconds of its own closing speed,
/// so all four touch together and every pair gets the same pressing window.
#[cfg(feature = "debug")]
const CONTACT_AFTER_SECS: f32 = 1.0;

/// In-step seconds a beat gets to reach its world condition.
///
/// Over the fleet's usual step deadline for the reason `system_hull_scaling`
/// gives: CI runs the correctness pass on a software rasterizer, where one
/// frame of two carriers' worth of sections costs seconds. A backstop that
/// names a hung beat, not a budget the range is held to.
#[cfg(feature = "debug")]
const STEP_DEADLINE_SECS: f32 = 900.0;

/// In-step seconds every hull is given to link its colliders and be weighed.
#[cfg(feature = "debug")]
const SETTLE_SECS: f32 = 4.0;

/// In-step seconds every pair presses AFTER it has met.
///
/// Contact seconds, which is the subject: the old model's cost was per contact
/// per FRAME, so a rule that is only free for one frame is not free.
#[cfg(feature = "debug")]
const PRESSING_SECS: f32 = 5.0;

/// How much more than twice the hit points twice the closing speed must cost.
///
/// The bite is an impulse term linear in the closing speed plus an energy term
/// quadratic in it, so doubling the speed more than doubles the damage as long
/// as the energy term is doing anything at all. Flatten the model back to a
/// linear one and this fails and says so.
#[cfg(feature = "debug")]
const QUADRATIC_MARGIN: f32 = 2.0;

/// The script type, named once so the step list and its helpers agree.
#[cfg(feature = "debug")]
type Script = nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(range_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(ram_script());
    }

    app.run()
}

fn range_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_range);
    // WITH the damage system and not in `Last`: `deal_contact_impact_damage`
    // runs once per fixed step after the solver, and the fastest staged contact
    // holds for about one step. A census taken once a RENDER frame samples the
    // last step of that frame only, so on a software rasterizer - several steps
    // to a frame - it reads the wrecking ram as a contact that never happened.
    #[cfg(feature = "debug")]
    app.add_systems(
        FixedPostUpdate,
        count_cross_hull_contacts
            .after(avian3d::prelude::PhysicsSystems::Last)
            .run_if(resource_exists::<ContactCensus>),
    );
}

fn setup_range(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShips>) {
    commands.trigger(LoadScenario(ram_range(&game_assets, &ships)));
}

/// The range: four pairs of hulls in four lanes of flat space, none of them
/// flown and none of them steered.
fn ram_range(game_assets: &GameAssets, ships: &GameShips) -> ScenarioConfig {
    let body = |id: &str, name: &str, side: Side, at: Meters3| {
        let base = BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position: at,
            rotation: Quat::IDENTITY,
        };
        let kind = match side {
            Side::Hull(catalog) => ScenarioObjectKind::Spaceship(SpaceshipConfig {
                controller: SpaceshipController::None,
                allegiance: None,
                hull: ShipSource::Inline(kit::catalog_ship(ships, catalog)),
                ..default()
            }),
            Side::Rock(radius) => ScenarioObjectKind::Asteroid(AsteroidConfig {
                radius,
                texture: game_assets.asteroid_texture.clone().into(),
                material: KIND_ROCK.to_string(),
                destroy_sound: None,
                // No well: a designated rock is put on rails and drags every
                // lane inside its sphere of influence along with it.
                mass: None,
                invulnerable: false,
                seed: None,
                lock_signature: None,
            }),
        };
        EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig { base, kind })
    };

    let mut bodies = Vec::new();
    for pair in &PAIRS {
        bodies.push(body(
            pair.left_id,
            pair.label,
            pair.left,
            Meters3::new(-STAGING_HALF_GAP.get(), 0.0, pair.lane.get()),
        ));
        bodies.push(body(
            pair.right_id,
            pair.label,
            pair.right,
            Meters3::new(STAGING_HALF_GAP.get(), 0.0, pair.lane.get()),
        ));
    }

    ScenarioConfig {
        description: "Six staged contacts, from a docking touch to a wrecking ram.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                bodies,
                ThreePointRig::around("collision damage", Meters3::ZERO, 400.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "collision_damage_range".to_string(),
            "Collision Damage Range".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// The scripted run: load every hull, let avian weigh it, put each pair nose to
/// nose, let them meet and press, then read what the contacts cost.
#[cfg(feature = "debug")]
fn ram_script() -> Script {
    Script::new()
        .step("load every staged body")
        .enter(GameStates::Loading)
        .until(every_body_weighed())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("let every body settle")
        .until(elapsed(SETTLE_SECS))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("put every pair nose to nose and close it")
        .on_enter(close_every_pair)
        .add()
        .step("let every pair meet")
        .until(every_pair_touching())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("let every pair press")
        .until(elapsed(PRESSING_SECS))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("read what the contacts cost")
        .on_enter(measure_every_pair)
        .add()
}

/// Every staged body is present AND has been weighed: a root avian has not
/// measured yet reaches nowhere along the closing axis, which is the figure the
/// placement reads.
#[cfg(feature = "debug")]
fn every_body_weighed() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        PAIRS.iter().all(|pair| {
            [pair.left_id, pair.right_id].iter().all(|id| {
                staged_body(world, id).is_some_and(|root| {
                    world.get::<avian3d::prelude::ComputedMass>(root).is_some()
                        && closing_reach(world, root) > 0.0
                })
            })
        })
    })
}

/// Every pair has touched at least once.
///
/// The placement puts all six one second from contact, so this normally holds
/// a second in; it is the guard that a pair which does NOT arrive stalls its
/// own beat by name instead of reading as a contact that cost nothing.
#[cfg(feature = "debug")]
fn every_pair_touching() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        world.get_resource::<ContactCensus>().is_some_and(|census| {
            census.0.len() == PAIRS.len() && census.0.iter().all(|pair| pair.steps > 0)
        })
    })
}

/// The staged body with this scenario id, off a borrowed world.
///
/// Keyed on the scenario id alone and not on `SpaceshipRootMarker`, because
/// half of one staged contact is a rock: the id is the only handle every kind
/// of scenario object hands out.
#[cfg(feature = "debug")]
fn staged_body(world: &World, id: &str) -> Option<Entity> {
    let mut bodies =
        world.try_query_filtered::<(Entity, &EntityId), With<avian3d::prelude::RigidBody>>()?;
    bodies
        .iter(world)
        .find(|(_, entity_id)| entity_id.0 == id)
        .map(|(entity, _)| entity)
}

/// The staged body with this scenario id, or a named panic.
#[cfg(feature = "debug")]
fn body_or_panic(world: &World, id: &str) -> Entity {
    staged_body(world, id)
        .unwrap_or_else(|| panic!("collision damage: staged body '{id}' is not in the world"))
}

/// What one staged body was worth before it touched anything, in whichever
/// store its own durability lives in.
#[cfg(feature = "debug")]
#[derive(Clone, Copy, Default, Debug)]
struct Durability {
    /// Live hit points over every health pool under the body.
    health: f32,
    /// Crater radius over every carve field under the body, in that body's own
    /// units. A rock keeps no hit points at all, so this is the whole of what
    /// a ram can take off it.
    carved: f32,
}

/// What every pair was worth before it touched anything, so a section that is
/// DESTROYED by a ram counts as the whole of itself rather than vanishing from
/// the sum.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct IntactBodies(Vec<(String, Durability)>);

/// What one staged body is worth right now.
///
/// Walked by ancestry rather than read off the root, because the root's own
/// `Health` is the ship layer's roll-up over SECTIONS and a hull presents more
/// than its sections to a contact - the outermost thing a shipped skin puts in
/// the way is often a decor patch, with a pool of its own. A ram spends hit
/// points on whatever it actually reaches, and the census has to count that.
/// The carve field is read the same way: an asteroid keeps it on the collider
/// node under its root, never on the root.
#[cfg(feature = "debug")]
fn durability(world: &World, root: Entity) -> Durability {
    let health = world
        .try_query::<(Entity, &Health)>()
        .map(|mut parts| {
            parts
                .iter(world)
                .filter(|(entity, _)| *entity != root && descends_from(world, *entity, root))
                .map(|(_, health)| health.current)
                .sum()
        })
        .unwrap_or_default();
    let carved = world
        .try_query::<(Entity, &DamageMarks)>()
        .map(|mut fields| {
            fields
                .iter(world)
                .filter(|(entity, _)| descends_from(world, *entity, root))
                .flat_map(|(_, marks)| marks.0.iter())
                .map(|mark| mark.radius)
                .sum()
        })
        .unwrap_or_default();
    Durability { health, carved }
}

/// How far down a body the census looks for a durability store.
///
/// A section hangs off the root and its skin decor off the section, so three is
/// already one more than the shipped depth; the bound is against a cycle, not
/// against a deep hull.
#[cfg(feature = "debug")]
const PART_DEPTH_LIMIT: usize = 3;

/// Whether `part` hangs off `root` by `ChildOf`, within [`PART_DEPTH_LIMIT`].
#[cfg(feature = "debug")]
fn descends_from(world: &World, part: Entity, root: Entity) -> bool {
    let mut at = part;
    for _ in 0..PART_DEPTH_LIMIT {
        let Some(parent) = world.get::<ChildOf>(at).map(ChildOf::parent) else {
            return false;
        };
        if parent == root {
            return true;
        }
        at = parent;
    }
    false
}

/// How far one body reaches along the closing axis, from its own origin.
///
/// A hull's own `HullEnvelopeRadius` is the distance to its furthest collider
/// point in ANY direction, which on a long hull is its nose. Two ships placed a
/// gap apart by that figure have their FLANKS several hundred meters apart and
/// a docking approach never arrives, so the placement asks the colliders what
/// the body measures along the one axis it is closing on - which is also the
/// only reading a rock and a ship both answer.
#[cfg(feature = "debug")]
fn closing_reach(world: &World, root: Entity) -> f32 {
    use avian3d::prelude::{ColliderAabb, ColliderOf, Position};

    let origin = world
        .get::<Position>(root)
        .map_or(Vec3::ZERO, |position| position.0);
    let Some(mut colliders) = world.try_query::<(&ColliderAabb, &ColliderOf)>() else {
        return 0.0;
    };
    colliders
        .iter(world)
        .filter(|(_, collider_of)| collider_of.body == root)
        .map(|(aabb, _)| (aabb.max.x - origin.x).max(origin.x - aabb.min.x))
        .fold(0.0_f32, f32::max)
}

/// Place both sides of every pair one second from contact, record what they are
/// worth intact, and start them closing.
#[cfg(feature = "debug")]
fn close_every_pair(world: &mut World) {
    use avian3d::prelude::{LinearVelocity, Position};

    let mut intact = IntactBodies::default();
    let mut census = ContactCensus::default();
    for pair in &PAIRS {
        let left = body_or_panic(world, pair.left_id);
        let right = body_or_panic(world, pair.right_id);
        // Engine boundary: an envelope is measured off avian colliders, so it
        // arrives in world units and the placement stays in them.
        let lane = pair.lane.to_engine();
        let closing = pair.closing.to_engine();
        let half_gap = closing * CONTACT_AFTER_SECS * 0.5;

        for (root, side) in [(left, -1.0_f32), (right, 1.0_f32)] {
            let offset = side * (closing_reach(world, root) + half_gap);
            world
                .entity_mut(root)
                .insert(Position(Vec3::new(offset, 0.0, lane)))
                .insert(Transform::from_xyz(offset, 0.0, lane))
                .insert(LinearVelocity(Vec3::new(-side * closing * 0.5, 0.0, 0.0)));
        }

        for (root, id) in [(left, pair.left_id), (right, pair.right_id)] {
            intact.0.push((id.to_string(), durability(world, root)));
        }
        census.0.push(PairContacts {
            left,
            right,
            steps: 0,
            peak: 0,
        });
        info!(
            "collision damage: {} closing at {:.1} m/s, {:.1} m apart, worth {:.1} / {:.1} hp",
            pair.label,
            pair.closing.get(),
            Meters::from_engine(closing * CONTACT_AFTER_SECS).get(),
            durability(world, left).health,
            durability(world, right).health,
        );
    }
    world.insert_resource(intact);
    world.insert_resource(census);
}

/// What one pair's two hulls did to each other over the pressing window.
#[cfg(feature = "debug")]
#[derive(Clone, Copy, Debug)]
struct PairContacts {
    /// The -x side's root.
    left: Entity,
    /// The +x side's root.
    right: Entity,
    /// Fixed steps in which the two hulls were touching at all.
    steps: usize,
    /// The most contacts they held in one step - the multiplier the old model
    /// charged its per-contact bite over.
    peak: usize,
}

/// Every pair's contact census, filled in while the pairs press.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct ContactCensus(Vec<PairContacts>);

/// Count what each pair is touching, once a fixed step, after the solver has
/// run.
///
/// A running census and not a reading taken at the end: two hulls that meet,
/// bite and bounce apart are not touching by the time the measuring beat looks,
/// and the figure the ledger wants is how many contacts they held while they
/// were. Sampled per STEP because that is the cadence the damage itself is
/// dealt at, and because a contact can be over inside one step.
#[cfg(feature = "debug")]
fn count_cross_hull_contacts(
    mut census: ResMut<ContactCensus>,
    graph: Res<avian3d::prelude::ContactGraph>,
) {
    for pair in &mut census.0 {
        let (left, right) = (pair.left, pair.right);
        let contacts = graph
            .iter_active_touching()
            .chain(graph.iter_sleeping_touching())
            .filter(|contact| {
                let bodies = (contact.body1, contact.body2);
                bodies == (Some(left), Some(right)) || bodies == (Some(right), Some(left))
            })
            .count();
        if contacts > 0 {
            pair.steps += 1;
            pair.peak = pair.peak.max(contacts);
        }
    }
}

/// What one staged contact cost.
#[cfg(feature = "debug")]
#[derive(Clone, Copy, Debug)]
struct RamCost {
    /// Hit points the -x side lost.
    left: f32,
    /// Hit points the +x side lost.
    right: f32,
    /// Crater radius the -x side gained, in its own units.
    left_carved: f32,
    /// Crater radius the +x side gained, in its own units.
    right_carved: f32,
    /// Fixed steps the two bodies spent touching.
    steps: usize,
    /// The most contacts they held in one step.
    peak: usize,
    /// Detached wreck bodies drifting in this pair's own lane.
    wrecks: usize,
}

#[cfg(feature = "debug")]
impl RamCost {
    /// Both sides together, in hit points.
    fn total(self) -> f32 {
        self.left + self.right
    }
}

/// Read one pair against what it was worth intact.
///
/// A body that is no longer there paid ALL of what it was worth: the
/// destructive contact is allowed to take a root with it, and a reading that
/// panicked on the missing entity would turn the loudest possible evidence into
/// a harness error.
#[cfg(feature = "debug")]
fn read_pair(world: &mut World, index: usize, pair: &RamPair) -> RamCost {
    let touching = world
        .get_resource::<ContactCensus>()
        .and_then(|census| census.0.get(index).copied())
        .unwrap_or_else(|| panic!("collision damage: '{}' was never censused", pair.label));

    let mut spent = [(0.0_f32, 0.0_f32); 2];
    for (slot, id) in [pair.left_id, pair.right_id].into_iter().enumerate() {
        let was = intact_durability(world, id);
        let now =
            staged_body(world, id).map_or_else(Durability::default, |root| durability(world, root));
        spent[slot] = (was.health - now.health, now.carved - was.carved);
    }

    RamCost {
        left: spent[0].0,
        right: spent[1].0,
        left_carved: spent[0].1,
        right_carved: spent[1].1,
        steps: touching.steps,
        peak: touching.peak,
        wrecks: wrecks_in_lane(world, pair.lane),
    }
}

/// What one staged body was worth before it touched anything.
#[cfg(feature = "debug")]
fn intact_durability(world: &World, id: &str) -> Durability {
    world
        .get_resource::<IntactBodies>()
        .and_then(|bodies| {
            bodies
                .0
                .iter()
                .find(|(staged, _)| staged == id)
                .map(|(_, worth)| *worth)
        })
        .unwrap_or_else(|| panic!("collision damage: '{id}' was never read intact"))
}

/// How far off its lane centre a piece still counts as that lane's wreckage.
///
/// Half the lane spacing, so no lane can claim its neighbour's debris and a
/// piece thrown by the kick the detach gives it is still attributed to the
/// contact that made it.
#[cfg(feature = "debug")]
const LANE_HALF_WIDTH: Meters = Meters(3_000.0);

/// Detached wreck bodies drifting in one lane.
///
/// `DetachedPieceMarker` is what a DESTROYED section becomes: the piece stops
/// being part of its hull, keeps the art and the collider it already had, and
/// takes a kick. Counting it is how the range asks whether destroyed structure
/// was left behind rather than deleted.
#[cfg(feature = "debug")]
fn wrecks_in_lane(world: &World, lane: Meters) -> usize {
    let Some(mut pieces) = world.try_query_filtered::<&Transform, With<DetachedPieceMarker>>()
    else {
        return 0;
    };
    pieces
        .iter(world)
        .filter(|piece| {
            Meters::from_engine((piece.translation.z - lane.to_engine()).abs()) < LANE_HALF_WIDTH
        })
        .count()
}

/// The one beat: read all six staged contacts, assert what the safe contact
/// speed promises and what a ram costs once it is one, and record the table the
/// ledger quotes.
#[cfg(feature = "debug")]
fn measure_every_pair(world: &mut World) {
    let costs: Vec<RamCost> = PAIRS
        .iter()
        .enumerate()
        .map(|(index, pair)| read_pair(world, index, pair))
        .collect();
    for (pair, cost) in PAIRS.iter().zip(&costs) {
        info!(
            "collision damage: {} pair: closing={:.1} m/s touching={} steps peak={} contacts \
             lost={:.2} / {:.2} hp carved={:.2} / {:.2} wrecks={}",
            pair.label,
            pair.closing.get(),
            cost.steps,
            cost.peak,
            cost.left,
            cost.right,
            cost.left_carved,
            cost.right_carved,
            cost.wrecks,
        );
    }

    // Delivery guard: the index constants below and the rows they name still
    // agree, so a contact inserted in the middle of PAIRS fails here instead of
    // quietly re-pointing a claim at a different contact.
    assert!(
        PAIRS[ROCK_RAM].reads == Reading::Rock && PAIRS[WRECKING_RAM].reads == Reading::Wreck,
        "collision damage: the staged contacts have moved under the index constants - \
         '{}' is read as {} and '{}' as {}",
        PAIRS[ROCK_RAM].label,
        reads_name(PAIRS[ROCK_RAM].reads),
        PAIRS[WRECKING_RAM].label,
        reads_name(PAIRS[WRECKING_RAM].reads),
    );

    for (pair, cost) in PAIRS.iter().zip(&costs) {
        assert!(
            cost.steps > 0,
            "collision damage: the {} pair never touched at all, so nothing it reads is \
             evidence about a contact; it was released {CONTACT_AFTER_SECS:.1} s from contact \
             at {:.1} m/s",
            pair.label,
            pair.closing.get(),
        );
    }

    let dock = costs[CARRIER_DOCK];
    assert!(
        dock.total() <= 0.0,
        "collision damage: two capitals coming alongside at {:.1} m/s - under the {:.1} m/s \
         safe contact speed - traded {:.2} hit points ({:.2} / {:.2}) over {} fixed steps of contact \
         at up to {} contacts each; a docking approach is not a ram at any mass",
        PAIRS[CARRIER_DOCK].closing.get(),
        SAFE_CONTACT_SPEED.get(),
        dock.total(),
        dock.left,
        dock.right,
        dock.steps,
        dock.peak,
    );
    nova_probe::probe_marker(
        world,
        "outcome: two capitals may come alongside for free",
        serde_json::json!({
            "closing_m_s": PAIRS[CARRIER_DOCK].closing.get(),
            "touching_steps": dock.steps,
            "peak_contacts": dock.peak,
            "lost_hp": dock.total(),
        }),
    );

    let touch = costs[SKIFF_TOUCH];
    assert!(
        touch.total() <= 0.0,
        "collision damage: a skiff pair touching at {:.1} m/s - under the same {:.1} m/s - \
         traded {:.2} hit points over {} fixed steps of contact; the threshold is a speed, so it has \
         to read the same on the fleet's smallest hull as on its largest",
        PAIRS[SKIFF_TOUCH].closing.get(),
        SAFE_CONTACT_SPEED.get(),
        touch.total(),
        touch.steps,
    );
    nova_probe::probe_marker(
        world,
        "outcome: a touch under the safe speed is free at either hull size",
        serde_json::json!({
            "closing_m_s": PAIRS[SKIFF_TOUCH].closing.get(),
            "touching_steps": touch.steps,
            "peak_contacts": touch.peak,
            "lost_hp": touch.total(),
        }),
    );

    let ram = costs[SKIFF_RAM];
    assert!(
        ram.left > 0.0 && ram.right > 0.0,
        "collision damage: a ram at {:.1} m/s cost {:.2} / {:.2} hit points; a contact has \
         two sides and both of them are metal",
        PAIRS[SKIFF_RAM].closing.get(),
        ram.left,
        ram.right,
    );
    nova_probe::probe_marker(
        world,
        "outcome: a ram spends hit points on both bodies",
        serde_json::json!({
            "closing_m_s": PAIRS[SKIFF_RAM].closing.get(),
            "touching_steps": ram.steps,
            "left_hp": ram.left,
            "right_hp": ram.right,
        }),
    );

    let hard = costs[SKIFF_HARD_RAM];
    let speed_ratio = PAIRS[SKIFF_HARD_RAM].closing.get() / PAIRS[SKIFF_RAM].closing.get();
    assert!(
        hard.total() > ram.total() * QUADRATIC_MARGIN,
        "collision damage: {speed_ratio:.1}x the closing speed cost only {:.2} hit points \
         against {:.2} - the bite carries a quadratic energy term, so it has to cost more \
         than {QUADRATIC_MARGIN:.1}x",
        hard.total(),
        ram.total(),
    );
    nova_probe::probe_marker(
        world,
        "outcome: the bite grows with the closing speed",
        serde_json::json!({
            "speed_ratio": speed_ratio,
            "damage_ratio": hard.total() / ram.total().max(f32::EPSILON),
            "ram_hp": ram.total(),
            "hard_ram_hp": hard.total(),
        }),
    );

    let rock = costs[ROCK_RAM];
    assert!(
        rock.left > 0.0 && rock.right_carved > 0.0,
        "collision damage: a hull flown into a rock at {:.1} m/s cost the hull {:.2} hit \
         points and opened {:.3} units of crater on the rock; both structures are in the \
         contact, and a rock keeps no hit points at all - its carve field IS its durability, \
         so a reading that only summed health would call this ram one-sided",
        PAIRS[ROCK_RAM].closing.get(),
        rock.left,
        rock.right_carved,
    );
    nova_probe::probe_marker(
        world,
        "outcome: a ram on a rock is paid out of both durabilities",
        serde_json::json!({
            "closing_m_s": PAIRS[ROCK_RAM].closing.get(),
            "touching_steps": rock.steps,
            "hull_hp": rock.left,
            "rock_carved": rock.right_carved,
            "rock_hp": rock.right,
        }),
    );

    let wrecking = costs[WRECKING_RAM];
    assert!(
        wrecking.wrecks > 0,
        "collision damage: a ram at {:.1} m/s left no wreckage in its own lane; a section \
         destroyed by a contact detaches as a body of its own, so a destructive ram that \
         leaves nothing has deleted structure instead of breaking it off (the contact spent \
         {:.2} / {:.2} hit points over {} fixed steps of contact)",
        PAIRS[WRECKING_RAM].closing.get(),
        wrecking.left,
        wrecking.right,
        wrecking.steps,
    );
    nova_probe::probe_marker(
        world,
        "outcome: a destructive ram leaves a wreck",
        serde_json::json!({
            "closing_m_s": PAIRS[WRECKING_RAM].closing.get(),
            "touching_steps": wrecking.steps,
            "wrecks": wrecking.wrecks,
            "left_hp": wrecking.left,
            "right_hp": wrecking.right,
        }),
    );

    nova_probe::probe_marker(
        world,
        "outcome: the contact census is recorded",
        serde_json::json!(PAIRS
            .iter()
            .zip(&costs)
            .map(|(pair, cost)| (
                pair.label.to_string(),
                serde_json::json!({
                    "left": side_name(pair.left),
                    "right": side_name(pair.right),
                    "reads": reads_name(pair.reads),
                    "closing_m_s": pair.closing.get(),
                    "touching_steps": cost.steps,
                    "peak_contacts": cost.peak,
                    "left_hp": cost.left,
                    "right_hp": cost.right,
                    "left_carved": cost.left_carved,
                    "right_carved": cost.right_carved,
                    "wrecks": cost.wrecks,
                }),
            ))
            .collect::<serde_json::Map<String, serde_json::Value>>()),
    );
    info!("collision damage: all six staged contacts read");
}

/// What the census calls one side of a contact.
#[cfg(feature = "debug")]
fn side_name(side: Side) -> String {
    match side {
        Side::Hull(catalog) => catalog.to_string(),
        Side::Rock(radius) => format!("rock r={:.0} m", radius.get()),
    }
}

/// Which ledger the census says a contact was read out of.
#[cfg(feature = "debug")]
fn reads_name(reads: Reading) -> &'static str {
    match reads {
        Reading::Hulls => "hit points, both sides",
        Reading::Rock => "hit points and crater volume",
        Reading::Wreck => "wreckage",
    }
}
