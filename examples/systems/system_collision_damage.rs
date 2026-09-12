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
//! | 5 | `outcome: the contact census is recorded` | RECORD: closing speed, frames spent touching, peak contacts held in one frame, and hit points lost per side, per pair |
//!
//! Claim 5 asserts NOTHING. It is the table `tasks/20260909-213118/FEEDBACK.md`
//! quotes, read against the figures there and never against a threshold.
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

/// One staged contact: two hulls of the same class, nose to nose on the x axis,
/// closing at `closing`.
struct RamPair {
    /// What the log and the census call this contact.
    label: &'static str,
    /// The catalog hull both sides fly.
    hull: &'static str,
    /// Scenario id of the side that starts on -x.
    left: &'static str,
    /// Scenario id of the side that starts on +x.
    right: &'static str,
    /// How fast the two close on each other.
    closing: MetersPerSecond,
    /// Where the pair is staged, so no pair's debris reaches another's.
    lane: Meters,
}

/// The four staged contacts.
///
/// Two under the safe speed and two over it, at both hull sizes, because the
/// rule under test is that the threshold is a SPEED: a capital pair coming
/// alongside at a docking speed and a needle pair nudging each other have to
/// get the same answer.
const PAIRS: [RamPair; 4] = [
    RamPair {
        label: "carrier dock",
        hull: CARRIER,
        left: "dock_carrier_left",
        right: "dock_carrier_right",
        // The task's own case: 0.32 u/s, well inside what a docking clamp
        // takes, and the approach that used to cost both ships their hulls.
        closing: MetersPerSecond(3.2),
        lane: Meters(0.0),
    },
    RamPair {
        label: "skiff touch",
        hull: SKIFF,
        left: "touch_skiff_left",
        right: "touch_skiff_right",
        closing: MetersPerSecond(4.0),
        lane: Meters(6_000.0),
    },
    RamPair {
        label: "skiff ram",
        hull: SKIFF,
        left: "ram_skiff_left",
        right: "ram_skiff_right",
        closing: MetersPerSecond(15.0),
        lane: Meters(12_000.0),
    },
    RamPair {
        label: "skiff hard ram",
        hull: SKIFF,
        left: "hard_skiff_left",
        right: "hard_skiff_right",
        closing: MetersPerSecond(30.0),
        lane: Meters(18_000.0),
    },
];

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
    #[cfg(feature = "debug")]
    app.add_systems(
        Last,
        count_cross_hull_contacts.run_if(resource_exists::<ContactCensus>),
    );
}

fn setup_range(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShips>) {
    commands.trigger(LoadScenario(ram_range(&game_assets, &ships)));
}

/// The range: four pairs of hulls in four lanes of flat space, none of them
/// flown and none of them steered.
fn ram_range(game_assets: &GameAssets, ships: &GameShips) -> ScenarioConfig {
    let hull = |id: &str, name: &str, catalog: &str, at: Meters3| {
        EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: id.to_string(),
                name: name.to_string(),
                position: at,
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

    let mut hulls = Vec::new();
    for pair in &PAIRS {
        hulls.push(hull(
            pair.left,
            pair.label,
            pair.hull,
            Meters3::new(-STAGING_HALF_GAP.get(), 0.0, pair.lane.get()),
        ));
        hulls.push(hull(
            pair.right,
            pair.label,
            pair.hull,
            Meters3::new(STAGING_HALF_GAP.get(), 0.0, pair.lane.get()),
        ));
    }

    ScenarioConfig {
        description: "Four staged contacts, from a docking touch to a head-on ram.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                hulls,
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
        .step("load every staged hull")
        .enter(GameStates::Loading)
        .until(every_hull_weighed())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("let every hull settle")
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

/// Every staged hull is present AND has been weighed: a root avian has not
/// measured yet publishes no envelope, which is the figure the placement reads.
#[cfg(feature = "debug")]
fn every_hull_weighed() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        PAIRS.iter().all(|pair| {
            [pair.left, pair.right].iter().all(|id| {
                staged_hull(world, id).is_some_and(|root| {
                    world.get::<HullEnvelopeRadius>(root).is_some()
                        && world.get::<avian3d::prelude::ComputedMass>(root).is_some()
                })
            })
        })
    })
}

/// Every pair has touched at least once.
///
/// The placement puts all four one second from contact, so this normally holds
/// a second in; it is the guard that a pair which does NOT arrive stalls its
/// own beat by name instead of reading as a contact that cost nothing.
#[cfg(feature = "debug")]
fn every_pair_touching() -> Arc<nova_protocol::nova_debug::harness::Predicate> {
    Arc::new(|world: &World| {
        world.get_resource::<ContactCensus>().is_some_and(|census| {
            census.0.len() == PAIRS.len() && census.0.iter().all(|pair| pair.frames > 0)
        })
    })
}

/// The hull with this scenario id, off a borrowed world.
#[cfg(feature = "debug")]
fn staged_hull(world: &World, id: &str) -> Option<Entity> {
    let mut hulls = world.try_query_filtered::<(Entity, &EntityId), With<SpaceshipRootMarker>>()?;
    hulls
        .iter(world)
        .find(|(_, entity_id)| entity_id.0 == id)
        .map(|(entity, _)| entity)
}

/// The hull with this scenario id, or a named panic.
#[cfg(feature = "debug")]
fn hull_or_panic(world: &World, id: &str) -> Entity {
    staged_hull(world, id)
        .unwrap_or_else(|| panic!("collision damage: staged hull '{id}' is not in the world"))
}

/// What every pair was worth before it touched anything, so a section that is
/// DESTROYED by a ram counts as the whole of itself rather than vanishing from
/// the sum.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct IntactHulls(Vec<(String, f32)>);

/// What one hull is worth right now: live hit points over every health-bearing
/// part under the root.
///
/// Walked by ancestry rather than read off the root, because the root's own
/// `Health` is the ship layer's roll-up over SECTIONS and a hull presents more
/// than its sections to a contact - the outermost thing a shipped skin puts in
/// the way is often a decor patch, with a pool of its own. A ram spends hit
/// points on whatever it actually reaches, and the census has to count that.
#[cfg(feature = "debug")]
fn hull_health(world: &mut World, root: Entity) -> f32 {
    let parts: Vec<(Entity, f32)> = world
        .query::<(Entity, &Health)>()
        .iter(world)
        .map(|(entity, health)| (entity, health.current))
        .collect();
    parts
        .into_iter()
        .filter(|(entity, _)| *entity != root && hull_of(world, *entity) == Some(root))
        .map(|(_, current)| current)
        .sum()
}

/// How far down a hull the census looks for a health pool.
///
/// A section hangs off the root and its skin decor off the section, so three is
/// already one more than the shipped depth; the bound is against a cycle, not
/// against a deep hull.
#[cfg(feature = "debug")]
const PART_DEPTH_LIMIT: usize = 3;

/// The hull root a part hangs off, by walking `ChildOf`. `None` for anything
/// that is not part of a ship.
#[cfg(feature = "debug")]
fn hull_of(world: &World, part: Entity) -> Option<Entity> {
    let mut at = part;
    for _ in 0..PART_DEPTH_LIMIT {
        let parent = world.get::<ChildOf>(at)?.parent();
        if world.get::<SpaceshipRootMarker>(parent).is_some() {
            return Some(parent);
        }
        at = parent;
    }
    None
}

/// How far one hull reaches along the closing axis, from its own origin.
///
/// The hull's own `HullEnvelopeRadius` is the distance to its furthest collider
/// point in ANY direction, which on a long hull is its nose. Two ships placed a
/// gap apart by that figure have their FLANKS several hundred meters apart and
/// a docking approach never arrives, so the placement asks the colliders what
/// the hull measures along the one axis it is closing on.
#[cfg(feature = "debug")]
fn closing_reach(world: &mut World, root: Entity) -> f32 {
    use avian3d::prelude::{ColliderAabb, ColliderOf, Position};

    let origin = world
        .get::<Position>(root)
        .map_or(Vec3::ZERO, |position| position.0);
    world
        .query::<(&ColliderAabb, &ColliderOf)>()
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

    let mut intact = IntactHulls::default();
    let mut census = ContactCensus::default();
    for pair in &PAIRS {
        let left = hull_or_panic(world, pair.left);
        let right = hull_or_panic(world, pair.right);
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

        for (root, id) in [(left, pair.left), (right, pair.right)] {
            intact.0.push((id.to_string(), hull_health(world, root)));
        }
        census.0.push(PairContacts {
            left,
            right,
            frames: 0,
            peak: 0,
        });
        info!(
            "collision damage: {} closing at {:.1} m/s, {:.1} m apart, worth {:.1} / {:.1} hp",
            pair.label,
            pair.closing.get(),
            Meters::from_engine(closing * CONTACT_AFTER_SECS).get(),
            hull_health(world, left),
            hull_health(world, right),
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
    /// Frames in which the two hulls were touching at all.
    frames: usize,
    /// The most contacts they held in one frame - the multiplier the old model
    /// charged its per-contact bite over.
    peak: usize,
}

/// Every pair's contact census, filled in while the pairs press.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct ContactCensus(Vec<PairContacts>);

/// Count what each pair is touching, once a frame, after the solver has run.
///
/// A per-frame census and not a reading taken at the end: two hulls that meet,
/// bite and bounce apart are not touching by the time the measuring beat looks,
/// and the figure the ledger wants is how many contacts they held while they
/// were.
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
            pair.frames += 1;
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
    /// Frames the two hulls spent touching.
    frames: usize,
    /// The most contacts they held in one frame.
    peak: usize,
}

#[cfg(feature = "debug")]
impl RamCost {
    /// Both sides together.
    fn total(self) -> f32 {
        self.left + self.right
    }
}

/// Read one pair against what it was worth intact.
#[cfg(feature = "debug")]
fn read_pair(world: &mut World, index: usize, pair: &RamPair) -> RamCost {
    let left = hull_or_panic(world, pair.left);
    let right = hull_or_panic(world, pair.right);
    let intact_left = intact_health(world, pair.left);
    let intact_right = intact_health(world, pair.right);
    let touching = world
        .get_resource::<ContactCensus>()
        .and_then(|census| census.0.get(index).copied())
        .unwrap_or_else(|| panic!("collision damage: '{}' was never censused", pair.label));

    RamCost {
        left: intact_left - hull_health(world, left),
        right: intact_right - hull_health(world, right),
        frames: touching.frames,
        peak: touching.peak,
    }
}

/// What one staged hull was worth before it touched anything.
#[cfg(feature = "debug")]
fn intact_health(world: &World, id: &str) -> f32 {
    world
        .get_resource::<IntactHulls>()
        .and_then(|hulls| {
            hulls
                .0
                .iter()
                .find(|(staged, _)| staged == id)
                .map(|(_, health)| *health)
        })
        .unwrap_or_else(|| panic!("collision damage: '{id}' was never read intact"))
}

/// The one beat: read all four staged contacts, assert what the safe contact
/// speed promises, and record the table the ledger quotes.
#[cfg(feature = "debug")]
fn measure_every_pair(world: &mut World) {
    let costs: Vec<RamCost> = PAIRS
        .iter()
        .enumerate()
        .map(|(index, pair)| read_pair(world, index, pair))
        .collect();
    for (pair, cost) in PAIRS.iter().zip(&costs) {
        info!(
            "collision damage: {} pair: closing={:.1} m/s touching={} frames peak={} contacts \
             lost={:.2} / {:.2} hp",
            pair.label,
            pair.closing.get(),
            cost.frames,
            cost.peak,
            cost.left,
            cost.right,
        );
    }

    for (pair, cost) in PAIRS.iter().zip(&costs) {
        assert!(
            cost.frames > 0,
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
         safe contact speed - traded {:.2} hit points ({:.2} / {:.2}) over {} touching frames \
         at up to {} contacts each; a docking approach is not a ram at any mass",
        PAIRS[CARRIER_DOCK].closing.get(),
        SAFE_CONTACT_SPEED.get(),
        dock.total(),
        dock.left,
        dock.right,
        dock.frames,
        dock.peak,
    );
    nova_probe::probe_marker(
        world,
        "outcome: two capitals may come alongside for free",
        serde_json::json!({
            "closing_m_s": PAIRS[CARRIER_DOCK].closing.get(),
            "touching_frames": dock.frames,
            "peak_contacts": dock.peak,
            "lost_hp": dock.total(),
        }),
    );

    let touch = costs[SKIFF_TOUCH];
    assert!(
        touch.total() <= 0.0,
        "collision damage: a skiff pair touching at {:.1} m/s - under the same {:.1} m/s - \
         traded {:.2} hit points over {} touching frames; the threshold is a speed, so it has \
         to read the same on the fleet's smallest hull as on its largest",
        PAIRS[SKIFF_TOUCH].closing.get(),
        SAFE_CONTACT_SPEED.get(),
        touch.total(),
        touch.frames,
    );
    nova_probe::probe_marker(
        world,
        "outcome: a touch under the safe speed is free at either hull size",
        serde_json::json!({
            "closing_m_s": PAIRS[SKIFF_TOUCH].closing.get(),
            "touching_frames": touch.frames,
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
            "touching_frames": ram.frames,
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

    nova_probe::probe_marker(
        world,
        "outcome: the contact census is recorded",
        serde_json::json!(PAIRS
            .iter()
            .zip(&costs)
            .map(|(pair, cost)| (
                pair.label.to_string(),
                serde_json::json!({
                    "hull": pair.hull,
                    "closing_m_s": pair.closing.get(),
                    "touching_frames": cost.frames,
                    "peak_contacts": cost.peak,
                    "left_hp": cost.left,
                    "right_hp": cost.right,
                }),
            ))
            .collect::<serde_json::Map<String, serde_json::Value>>()),
    );
    info!("collision damage: all four staged contacts read");
}
