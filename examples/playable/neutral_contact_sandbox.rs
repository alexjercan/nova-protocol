//! neutral_contact_sandbox: bump, ram and shoot a Neutral ship and a Neutral
//! station, and see when a Neutral answers.
//!
//! The player flies the bounty hunter's interceptor with its kit, spawned as
//! New Game spawns a start role: the catalog hull, the role's bindings and
//! the role's credits and stock. A Neutral raider, armed and stocked with the
//! pirate kit, holds station about 160 m ahead. A Neutral ring station, built
//! by the station generator at mid advancement, stands about 1.5 km off.
//! Nothing ends the run.
//!
//! What to try:
//!
//! 1. drift into the raider under 10 m/s: the contact is free, no damage;
//! 2. ram it faster: only the approach over 10 m/s bites, and the hit adds
//!    to its tally of what you did to it;
//! 3. fire a short PDC burst (left mouse): a few rounds stay under the 30 HP
//!    threshold and drain away at 10 HP/s; a longer burst crosses it, and the
//!    raider turns on you while its side stays Neutral;
//! 4. fly to the station and do the same to it.
//!
//! ```text
//! nix develop --command cargo run --example neutral_contact_sandbox --features debug
//! ```
//!
//! The diagnostics (`F3`) put your tally against
//! [`NEUTRAL_RETALIATION_THRESHOLD`], drained to now, above each Neutral,
//! and your closing speed at the tip of the closing-velocity arrow (green
//! under the 10 m/s floor, red over it) while you are within 1 km. A ring
//! round each Neutral turns red while it answers someone, with a line to the
//! ship it answers. The panel keeps the last contact's peak approach speed,
//! and `LMB` is hinted near a Neutral. Detail (`Shift+F3`) lists every
//! attacker's tally, whom each Neutral answers, and the damage each hull
//! took.
//!
//! Under `NOVA_AUTOPILOT=1` a smoke walk waits for the cast,
//! checks the Neutral sides and the clean tallies, lands one staged 25 HP hit
//! from the player on the raider, checks that the tally holds it and the
//! raider does not answer, and shoots `neutral-contact.png` with
//! `NOVA_CAPTURE=1`. Unset, no walk and no deadline run.

use std::collections::HashMap;

use avian3d::prelude::{Collisions, LinearVelocity, PhysicsSystems, Sensor};
use bevy::{prelude::*, reflect::GetPath};
use clap::Parser;
use diagnostics::{palette, DiagnosticsGizmos, DiagnosticsPanel, DiagnosticsSystems};
use generated_station::generated_station_design;
use nova_protocol::{nova_debug::diagnostics, prelude::*};

#[path = "../shared/generated_station.rs"]
mod generated_station;

#[derive(Parser)]
#[command(name = "neutral_contact_sandbox")]
#[command(version = "1.0.0")]
#[command(
    about = "Bump, ram and shoot a Neutral ship and a Neutral station, and see when a Neutral \
             answers",
    long_about = None
)]
struct Cli;

/// Scenario id of the player ship.
const PLAYER_ID: &str = "neutral_contact_player";
/// Scenario id of the Neutral raider.
const RAIDER_ID: &str = "neutral_contact_raider";
/// Scenario id of the Neutral station.
const STATION_ID: &str = "neutral_contact_station";
/// The start role the player flies.
const PLAYER_ROLE: &str = "bounty_hunter";
/// The start role whose hull and kit the Neutral raider carries: armed, and
/// with rounds to answer.
const RAIDER_ROLE: &str = "pirate";
/// Where the raider holds station: 150 m ahead of the player, a little left
/// and up, so the chase camera sees it past the player's hull.
const RAIDER_POSITION: Meters3 = Meters3::new(-50.0, 15.0, -150.0);
/// Where the station stands: about 1.5 km off, ahead and to the right.
const STATION_POSITION: Meters3 = Meters3::new(500.0, 0.0, -1_400.0);
/// The approach speed under which a contact is free, m/s. Mirrors
/// nova_gameplay's private `SAFE_CONTACT_SPEED`; the overlay reads it,
/// nothing else does.
const CONTACT_FLOOR_MPS: f32 = 10.0;
/// Hit points per second each tally drains. Mirrors nova_ship's
/// `pub(crate)` `NEUTRAL_PROVOCATION_DRAIN`.
const PROVOCATION_DRAIN: f32 = 10.0;
/// Distance (m) inside which the closing speed is labelled and the gun
/// hinted.
const NEAR_M: f32 = 1_000.0;
/// Seconds of closing velocity the arrow draws.
const VELOCITY_ARROW_SECS: f32 = 2.0;
/// Characters in a tally bar.
const BAR_WIDTH: usize = 10;
/// The seed of the generated ring station.
const STATION_SEED: u32 = 7;
/// The advancement the ring station is generated at.
const STATION_ADVANCEMENT: f32 = 0.5;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(sandbox_plugin).build();
    app.add_plugins(diagnostics::DiagnosticsHintPlugin::new(
        "drift into a Neutral under 10 m/s, ram it faster, or shoot it",
    ));

    if std::env::var_os("NOVA_AUTOPILOT").is_some() {
        app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
            bevy::ecs::error::panic,
        ));
        app.add_systems(
            Startup,
            nova_protocol::nova_debug::harness::force_capture_resolution,
        );
    }
    app.add_plugins(smoke::script());

    app.run()
}

fn sandbox_plugin(app: &mut App) {
    app.init_resource::<ContactRead>();
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_the_sandbox);
    app.add_observer(record_damage);
    app.add_systems(
        FixedPostUpdate,
        read_the_contacts.after(PhysicsSystems::Last),
    );
    app.add_systems(
        PostUpdate,
        draw_the_neutrals
            .in_set(DiagnosticsSystems)
            .after(TransformSystems::Propagate)
            .run_if(in_state(GameStates::Playing)),
    );
}

fn load_the_sandbox(world: &mut World) {
    let scenario = sandbox(world);
    world.trigger(LoadScenario(scenario));
}

/// A start role's ship, as `SpawnStartRole` builds it: the catalog hull, the
/// bindings a role hull spawns with, and the role's kit.
///
/// # Panics
///
/// On a role or a hull the loaded content does not hold.
fn role_ship(
    world: &World,
    role: &str,
    controller: impl FnOnce(&ResolvedShipDesign) -> SpaceshipController,
) -> (String, SpaceshipConfig) {
    let role = world
        .resource::<GameStartRoles>()
        .get_role(&role.into())
        .unwrap_or_else(|| panic!("neutral_contact_sandbox: no start role '{role}'"))
        .clone();
    let designs = world.resource::<GameShipDesigns>();
    let sections = world.resource::<GameSections>();
    let name = designs
        .get_design(&role.hull)
        .unwrap_or_else(|| panic!("neutral_contact_sandbox: no hull '{}'", role.hull))
        .name
        .clone();
    let design = ShipDesignSource::prototype(role.hull.clone());
    let (hull, _) = resolve_ship_design(&design, designs, sections);
    let ship = SpaceshipConfig {
        design,
        controller: controller(&hull),
        capabilities: default(),
        initial_velocity: MetersPerSecond3::ZERO,
        allegiance: None,
        inventory: role.kit.stock.clone(),
        lootable: false,
        credits: role.kit.credits,
    };
    (name, ship)
}

fn object(
    id: &str,
    name: String,
    position: Meters3,
    kind: ScenarioObjectKind,
) -> EventActionConfig {
    EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name,
            position,
            rotation: Quat::IDENTITY,
        },
        kind,
    })
}

fn sandbox(world: &World) -> ScenarioConfig {
    let (player_name, player) = role_ship(world, PLAYER_ROLE, |hull| {
        SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: start_role_bindings(hull),
        })
    });
    let (raider_name, raider) = role_ship(world, RAIDER_ROLE, |_| {
        SpaceshipController::AI(AIControllerConfig::default())
    });
    let raider = SpaceshipConfig {
        allegiance: Some(Allegiance::Neutral),
        ..raider
    };
    let station = StationConfig {
        design: StationDesignSource::Inline(
            generated_station_design(
                world.resource::<LoadedSectionPacks>(),
                world.resource::<GameItems>(),
                StationArchetypeType::Ring,
                STATION_ADVANCEMENT,
                STATION_SEED,
            )
            .1,
        ),
        allegiance: Allegiance::Neutral,
        derelict: false,
        initial_velocity: MetersPerSecond3::ZERO,
    };

    ScenarioConfig {
        description: "A start-role player, a Neutral raider and a Neutral station".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![
                    object(
                        PLAYER_ID,
                        player_name,
                        Meters3::ZERO,
                        ScenarioObjectKind::Spaceship(player),
                    ),
                    object(
                        RAIDER_ID,
                        format!("Neutral {raider_name}"),
                        RAIDER_POSITION,
                        ScenarioObjectKind::Spaceship(raider),
                    ),
                    object(
                        STATION_ID,
                        "Neutral Ring Station".to_string(),
                        STATION_POSITION,
                        ScenarioObjectKind::Station(station),
                    ),
                ],
                ThreePointRig::around("neutral contact", Meters3::ZERO, 20.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "neutral_contact_sandbox".to_string(),
            "Neutral Contact Sandbox".to_string(),
            world.resource::<GameAssets>().cubemap.clone().into(),
        )
    }
}

/// The staged root with scenario id `id`.
fn staged(world: &World, id: &str) -> Option<Entity> {
    world
        .try_query_filtered::<(Entity, &EntityId), Without<ChildOf>>()?
        .iter(world)
        .find_map(|(entity, entity_id)| (entity_id.0 == id).then_some(entity))
}

/// The topmost `ChildOf` ancestor of `entity`.
fn root_of(entity: Entity, q_parent: &Query<&ChildOf>) -> Entity {
    q_parent.root_ancestor(entity)
}

/// What contacts and damage the overlay reports.
#[derive(Resource, Default, Debug)]
struct ContactRead {
    /// The peak approach speed (m/s) of the contact under way, and the hull
    /// the player is touching.
    touching: Option<(f32, Entity)>,
    /// The peak approach speed (m/s) of the last contact that ended, and the
    /// hull it was with.
    last: Option<(f32, Entity)>,
    /// Per ship root: the damage that reached it, total and the last hit.
    damage: HashMap<Entity, (f32, f32)>,
}

/// Note every hit that reaches a ship root. The event propagates up
/// `ChildOf`, so a root sees each hit on any of its sections once.
fn record_damage(
    damage: On<HealthApplyDamage>,
    q_roots: Query<(), (With<EntityId>, Without<ChildOf>)>,
    mut read: ResMut<ContactRead>,
) {
    if damage.amount <= 0.0 || !q_roots.contains(damage.entity) {
        return;
    }
    let entry = read.damage.entry(damage.entity).or_default();
    entry.0 += damage.amount;
    entry.1 = damage.amount;
}

/// Read the solver's contacts between the player and any other ship root
/// after the physics step: the peak approach speed along the contact normal,
/// which is the speed the ram rule judges. A pair with a `Sensor` side is
/// skipped: no solver impulse crosses it, so the ram rule never judges it.
/// A cargo intake's trigger is one, a detached body of its own beside the
/// ship.
fn read_the_contacts(
    collisions: Collisions,
    q_sensors: Query<(), With<Sensor>>,
    q_player: Query<Entity, With<PlayerSpaceshipMarker>>,
    q_parent: Query<&ChildOf>,
    mut read: ResMut<ContactRead>,
) {
    let Ok(player) = q_player.single() else {
        return;
    };
    let mut now: Option<(f32, Entity)> = None;
    for pair in collisions.iter() {
        let (Some(body1), Some(body2)) = (pair.body1, pair.body2) else {
            continue;
        };
        if q_sensors.contains(pair.collider1) || q_sensors.contains(pair.collider2) {
            continue;
        }
        let (root1, root2) = (root_of(body1, &q_parent), root_of(body2, &q_parent));
        let other = match (root1 == player, root2 == player) {
            (true, false) => root2,
            (false, true) => root1,
            _ => continue,
        };
        let approach = pair
            .manifolds
            .iter()
            .flat_map(|manifold| manifold.points.iter())
            .map(|point| -point.normal_speed)
            .fold(None, |peak: Option<f32>, speed| {
                Some(peak.map_or(speed, |peak| peak.max(speed)))
            });
        let Some(approach) = approach else {
            continue;
        };
        let speed = Meters::from_engine(approach.max(0.0)).0;
        if now.is_none_or(|(peak, _)| speed > peak) {
            now = Some((speed, other));
        }
    }
    read.touching = match (read.touching, now) {
        (Some((peak, other)), Some((speed, _))) => Some((peak.max(speed), other)),
        (None, now) => now,
        (Some(ended), None) => {
            read.last = Some(ended);
            None
        }
    };
}

/// One attacker's tally as the Neutral holds it: who, the undrained damage,
/// and the virtual time it was last updated.
struct Tally {
    attacker: Entity,
    damage: f32,
    at: f32,
}

/// The tallies an [`AIProvocation`] holds. Its fields are private to
/// nova_ship, so this reads them through reflection.
fn tallies(provocation: &AIProvocation) -> Vec<Tally> {
    (0..)
        .map_while(|index| {
            let attacker = *provocation
                .path::<Entity>(format!("tallies[{index}].attacker").as_str())
                .ok()?;
            let damage = *provocation
                .path::<f32>(format!("tallies[{index}].damage").as_str())
                .ok()?;
            let at = *provocation
                .path::<f32>(format!("tallies[{index}].at").as_str())
                .ok()?;
            Some(Tally {
                attacker,
                damage,
                at,
            })
        })
        .collect()
}

/// A tally drained to `now`, as the next hit will drain it.
fn drained(tally: &Tally, now: f32) -> f32 {
    (tally.damage - PROVOCATION_DRAIN * (now - tally.at).max(0.0)).max(0.0)
}

/// `value` of `whole` as a text bar.
fn bar(value: f32, whole: f32) -> String {
    let filled = ((value / whole).clamp(0.0, 1.0) * BAR_WIDTH as f32).round() as usize;
    format!("[{}{}]", "#".repeat(filled), "-".repeat(BAR_WIDTH - filled))
}

/// `speed` against the contact floor.
fn against_floor(speed: f32) -> &'static str {
    if speed <= CONTACT_FLOOR_MPS {
        "free"
    } else {
        "ram"
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "one overlay over the player, the Neutrals, their tallies and the contacts"
)]
fn draw_the_neutrals(
    mut gizmos: Gizmos<DiagnosticsGizmos>,
    mut panel: ResMut<DiagnosticsPanel>,
    time: Res<Time<Virtual>>,
    read: Res<ContactRead>,
    q_player: Query<(Entity, &GlobalTransform, &LinearVelocity), With<PlayerSpaceshipMarker>>,
    q_neutrals: Query<(
        Entity,
        &Name,
        &GlobalTransform,
        &Allegiance,
        &AIProvocation,
        Option<&RetaliationTarget>,
        Option<&HullRadius>,
        Option<&LinearVelocity>,
    )>,
    q_names: Query<&Name>,
) {
    let name_of = |entity: Entity, player: Option<Entity>| -> String {
        if Some(entity) == player {
            "you".to_string()
        } else {
            q_names
                .get(entity)
                .map_or_else(|_| format!("{entity}"), |name| name.as_str().to_string())
        }
    };
    let player = q_player.single().ok();
    let player_entity = player.map(|(entity, ..)| entity);
    let now = time.elapsed_secs();

    match &read.touching {
        Some((peak, other)) => panel.detail(format!(
            "contact: touching {} now, peak approach {peak:.1} m/s -> {} (floor {CONTACT_FLOOR_MPS} m/s)",
            name_of(*other, player_entity),
            against_floor(*peak),
        )),
        None => panel.detail("contact: none now"),
    }
    match &read.last {
        Some((peak, other)) => panel.line(format!(
            "last contact: {} at {peak:.1} m/s, {}",
            name_of(*other, player_entity),
            against_floor(*peak),
        )),
        None => {}
    }

    for (entity, name, transform, allegiance, provocation, retaliation, radius, velocity) in
        &q_neutrals
    {
        if *allegiance != Allegiance::Neutral {
            continue;
        }
        let centre = transform.translation();
        let answering = retaliation.and_then(|target| target.0);

        if let Some((_, player_transform, player_velocity)) = player {
            let offset = centre - player_transform.translation();
            let relative = player_velocity.0 - velocity.map_or(Vec3::ZERO, |velocity| velocity.0);
            let closing = Meters::from_engine(relative.dot(offset.normalize_or_zero())).0;
            let colour = if closing > CONTACT_FLOOR_MPS {
                palette::ALERT
            } else {
                palette::OK
            };
            diagnostics::vector(
                &mut gizmos,
                player_transform.translation(),
                relative,
                VELOCITY_ARROW_SECS,
                colour,
            );
            let off = Meters::from_engine(offset.length()).0;
            if off < NEAR_M {
                panel.label_tinted(
                    player_transform.translation() + relative * VELOCITY_ARROW_SECS,
                    format!("{closing:+.1} m/s {}", against_floor(closing)),
                    colour,
                );
                panel.hint("LMB", "fire the PDC");
            }
            panel.detail(format!(
                "{name}: {:.0} m off, you close at {closing:+.1} m/s ({})",
                Meters::from_engine(offset.length()).0,
                if closing > CONTACT_FLOOR_MPS {
                    "a ram, over the floor"
                } else {
                    "free, under the floor"
                },
            ));
        }

        let ring = radius.map_or(5.0, |radius| radius.0);
        let colour = if answering.is_some() {
            palette::ALERT
        } else {
            palette::MUTED
        };
        diagnostics::ring(&mut gizmos, centre, Vec3::Y, ring, colour);
        if let Some(target) = answering {
            if let Some((_, target_transform, _)) = player.filter(|(player, ..)| *player == target)
            {
                gizmos.line(centre, target_transform.translation(), palette::ALERT);
            }
        }

        let list = tallies(provocation);
        let mine = list
            .iter()
            .find(|tally| Some(tally.attacker) == player_entity)
            .map_or(0.0, |tally| drained(tally, now));
        panel.label_tinted(
            centre + Vec3::Y * ring,
            format!(
                "{} {mine:.0}/{NEUTRAL_RETALIATION_THRESHOLD:.0} HP{}",
                bar(mine, NEUTRAL_RETALIATION_THRESHOLD),
                if answering.is_some() {
                    " ANSWERING"
                } else {
                    ""
                }
            ),
            if answering.is_some() {
                palette::ALERT
            } else if mine > 0.0 {
                palette::TARGET
            } else {
                palette::MUTED
            },
        );
        panel.detail(format!(
            "    answers: {}",
            answering.map_or("nobody".to_string(), |target| name_of(
                target,
                player_entity
            )),
        ));
        if list.is_empty() {
            panel.detail(format!(
                "    tally: none (threshold {NEUTRAL_RETALIATION_THRESHOLD:.0} HP, drains {PROVOCATION_DRAIN:.0} HP/s)"
            ));
        }
        for tally in &list {
            let left = drained(tally, now);
            panel.detail(format!(
                "    tally from {}: {left:.1} / {NEUTRAL_RETALIATION_THRESHOLD:.0} HP {} ({:.1} at the last hit, drains {PROVOCATION_DRAIN:.0} HP/s)",
                name_of(tally.attacker, player_entity),
                bar(left, NEUTRAL_RETALIATION_THRESHOLD),
                tally.damage,
            ));
        }
        let (total, last) = read.damage.get(&entity).copied().unwrap_or_default();
        panel.detail(format!(
            "    damage taken: {total:.1} HP, last hit {last:.1} HP"
        ));
    }

    if let Some(player) = player_entity {
        let (total, last) = read.damage.get(&player).copied().unwrap_or_default();
        panel.detail(format!(
            "you: damage taken {total:.1} HP, last hit {last:.1} HP"
        ));
    }
}

mod smoke {
    use std::sync::Arc;

    use bevy::prelude::*;
    use nova_protocol::{
        nova_debug::harness::{shoot, AutopilotPlugin},
        prelude::*,
    };

    use super::{staged, tallies, PLAYER_ID, RAIDER_ID, STATION_ID};

    /// The staged hit, HP: under the threshold, so the raider must hold it
    /// and not answer, and large enough that the shot still shows it after a
    /// second of drain.
    const STAGED_HIT: f32 = 25.0;
    /// Real seconds the load may take on lavapipe.
    const LOAD_DEADLINE: f32 = 600.0;
    /// The frame the walk writes.
    const SHOT: &str = "neutral-contact.png";

    fn cast_present(world: &World) -> bool {
        [PLAYER_ID, RAIDER_ID, STATION_ID]
            .iter()
            .all(|id| staged(world, id).is_some())
    }

    /// Both Neutrals carry a clean tally and answer nobody.
    fn check_the_cast(world: &mut World) {
        for id in [RAIDER_ID, STATION_ID] {
            let entity = staged(world, id)
                .unwrap_or_else(|| panic!("neutral_contact_sandbox: '{id}' is not staged"));
            assert_eq!(
                world.get::<Allegiance>(entity),
                Some(&Allegiance::Neutral),
                "neutral_contact_sandbox: '{id}' must be Neutral"
            );
            let provocation = world
                .get::<AIProvocation>(entity)
                .unwrap_or_else(|| panic!("neutral_contact_sandbox: '{id}' has no AIProvocation"));
            assert!(
                tallies(provocation).is_empty(),
                "neutral_contact_sandbox: '{id}' starts with a tally"
            );
            assert!(
                world
                    .get::<RetaliationTarget>(entity)
                    .is_none_or(|target| target.0.is_none()),
                "neutral_contact_sandbox: '{id}' starts answering someone"
            );
        }
        info!("neutral contact sandbox: PASS the raider and the station start Neutral and calm");
    }

    /// One hit from the player under the threshold.
    fn hit_the_raider(world: &mut World) {
        let raider = staged(world, RAIDER_ID).expect("neutral_contact_sandbox: no raider");
        let player = staged(world, PLAYER_ID).expect("neutral_contact_sandbox: no player");
        world.trigger(HealthApplyDamage {
            entity: raider,
            source: Some(player),
            amount: STAGED_HIT,
        });
    }

    /// The raider holds the hit in the player's tally and answers nobody.
    fn check_the_tally(world: &mut World) {
        let raider = staged(world, RAIDER_ID).expect("neutral_contact_sandbox: no raider");
        let player = staged(world, PLAYER_ID).expect("neutral_contact_sandbox: no player");
        let list = tallies(
            world
                .get::<AIProvocation>(raider)
                .expect("neutral_contact_sandbox: the raider has no AIProvocation"),
        );
        let mine = list
            .iter()
            .find(|tally| tally.attacker == player)
            .unwrap_or_else(|| panic!("neutral_contact_sandbox: no player tally on the raider"));
        assert!(
            (mine.damage - STAGED_HIT).abs() < 0.01,
            "neutral_contact_sandbox: the raider tallied {} HP, not {STAGED_HIT}",
            mine.damage
        );
        assert!(
            world
                .get::<RetaliationTarget>(raider)
                .is_none_or(|target| target.0.is_none()),
            "neutral_contact_sandbox: a {STAGED_HIT} HP hit must not provoke the raider"
        );
        info!(
            "neutral contact sandbox: PASS a {STAGED_HIT} HP hit sits in the tally and the raider \
             does not answer"
        );
    }

    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        AutopilotPlugin::<GameStates>::new()
            .step("neutral contact sandbox: load the cast")
            .enter(GameStates::Loading)
            .until(and(
                and(state_is(GameStates::Playing), player_ship_present()),
                Arc::new(cast_present),
            ))
            .deadline(LOAD_DEADLINE)
            .add()
            .step("neutral contact sandbox: check the cast")
            .on_enter(check_the_cast)
            .until(frames(10))
            .add()
            .step("neutral contact sandbox: land a staged hit")
            .on_enter(hit_the_raider)
            .until(frames(2))
            .add()
            .step("neutral contact sandbox: check the tally and shoot the overlay")
            .on_enter(check_the_tally)
            .on_enter(|world: &mut World| shoot(world, SHOT))
            .until(shot_written(SHOT))
            .deadline(LOAD_DEADLINE)
            .add()
    }
}
