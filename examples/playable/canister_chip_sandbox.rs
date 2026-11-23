//! canister_chip_sandbox: fourteen cargo canisters in the reach of the
//! player's intake, and the content chips that label them.
//!
//! The player flies the industrial start hull, spawned as New Game spawns a
//! start role. Fourteen canisters stand in front of its intake door: a tight
//! cluster of eight (seven kinds of item) on the left, a cluster of three of
//! one item on the right, a pair and one canister alone nearer the ship. Each
//! canister is in reach, so the intake pulls it in at 8 m/s.
//!
//! What to try:
//!
//! 1. look at the two clusters: each draws one list chip, one row per item
//!    kind with the summed count, at most five rows;
//! 2. the pair and the single canister keep a chip each;
//! 3. let the intake pull the canisters in: a cluster that falls under three
//!    canisters splits back into single chips;
//! 4. press `1` to put all fourteen back.
//!
//! ```text
//! nix develop --command cargo run --example canister_chip_sandbox --features debug
//! ```
//!
//! The diagnostics (`F3`) show how many canisters are in reach, how many
//! canister chips show and the screen area they take.
//!
//! Under `NOVA_AUTOPILOT=1` a smoke walk waits for the fourteen canisters to
//! stand in reach, holds the clock, logs the canister chips' screen area and
//! checks the grouping: two list chips and three single chips. With
//! `NOVA_CAPTURE=1` it shoots `canister-chips-{phosphor,hardware}.png`. Unset,
//! no walk and no deadline run.

use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;
use clap::Parser;
use diagnostics::DiagnosticsPanel;
use nova_protocol::{nova_debug::diagnostics, prelude::*};

#[derive(Parser)]
#[command(name = "canister_chip_sandbox")]
#[command(version = "1.0.0")]
#[command(
    about = "Fourteen canisters in the intake's reach: clustered canisters share one list chip",
    long_about = None
)]
struct Cli;

/// Scenario id of the player ship.
const PLAYER_ID: &str = "canister_chip_player";
/// The start role the player flies.
const PLAYER_ROLE: &str = "industrial";

/// One canister: where it stands off the intake door, which item kind it
/// holds (an index into the sandbox's kinds) and how many.
///
/// The offset is in meters along the door's frame: `x` to the ship's right,
/// `y` out along the door normal, `z` toward the ship's nose.
struct Drop {
    at: Meters3,
    kind: usize,
    count: u32,
}

const fn drop(x: f32, y: f32, z: f32, kind: usize, count: u32) -> Drop {
    Drop {
        at: Meters3::new(x, y, z),
        kind,
        count,
    }
}

/// The item kinds the canisters hold.
const KINDS: usize = 7;

/// The fourteen canisters: eight of seven kinds on the left, three of one
/// kind on the right, a pair and a single nearer the ship. Every one is in
/// front of the door and inside the 120 m pull range.
const DROPS: [Drop; 14] = [
    drop(-61.0, 25.0, 80.0, 0, 2),
    drop(-58.0, 25.0, 80.0, 1, 1),
    drop(-55.0, 25.0, 80.0, 2, 3),
    drop(-52.0, 25.0, 80.0, 3, 1),
    drop(-61.0, 28.0, 80.0, 4, 2),
    drop(-58.0, 28.0, 80.0, 5, 1),
    drop(-55.0, 28.0, 80.0, 6, 2),
    drop(-52.0, 28.0, 80.0, 0, 1),
    drop(52.0, 25.0, 80.0, 1, 1),
    drop(55.0, 25.0, 80.0, 1, 2),
    drop(58.0, 25.0, 80.0, 1, 3),
    drop(-3.0, 4.0, 20.0, 2, 1),
    drop(3.0, 4.0, 20.0, 3, 1),
    drop(0.0, 60.0, 60.0, 4, 1),
];

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(sandbox_plugin).build();
    app.add_plugins(diagnostics::DiagnosticsHintPlugin::new(
        "clustered canisters share one list chip; press 1 to put the canisters back",
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
    app.init_resource::<Scatter>();
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_the_sandbox);
    app.add_systems(
        Update,
        (read_the_keys, scatter_the_canisters, draw_the_chips)
            .chain()
            .run_if(in_state(GameStates::Playing)),
    );
}

fn load_the_sandbox(world: &mut World) {
    let scenario = sandbox(world);
    world.trigger(LoadScenario(scenario));
}

/// Whether the canisters still have to be put in front of the intake.
#[derive(Resource, Debug)]
struct Scatter {
    pending: bool,
}

impl Default for Scatter {
    fn default() -> Self {
        Self { pending: true }
    }
}

/// A start role's ship, as `SpawnStartRole` builds it: the catalog hull, the
/// bindings a role hull spawns with, and the role's kit.
///
/// # Panics
///
/// On a role or a hull the loaded content does not hold.
fn role_ship(world: &World, role: &str) -> (String, SpaceshipConfig) {
    let role = world
        .resource::<GameStartRoles>()
        .get_role(&role.into())
        .unwrap_or_else(|| panic!("canister_chip_sandbox: no start role '{role}'"))
        .clone();
    let designs = world.resource::<GameShipDesigns>();
    let sections = world.resource::<GameSections>();
    let name = designs
        .get_design(&role.hull)
        .unwrap_or_else(|| panic!("canister_chip_sandbox: no hull '{}'", role.hull))
        .name
        .clone();
    let design = ShipDesignSource::prototype(role.hull.clone());
    let (hull, _) = resolve_ship_design(&design, designs, sections);
    let ship = SpaceshipConfig {
        design,
        controller: SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: start_role_bindings(&hull),
        }),
        capabilities: default(),
        initial_velocity: MetersPerSecond3::ZERO,
        allegiance: None,
        inventory: role.kit.stock.clone(),
        lootable: false,
        credits: role.kit.credits,
    };
    (name, ship)
}

fn sandbox(world: &World) -> ScenarioConfig {
    let (player_name, player) = role_ship(world, PLAYER_ROLE);
    ScenarioConfig {
        description: "A start-role player with fourteen canisters at its intake".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            position: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![EventActionConfig::SpawnScenarioObject(
                    ScenarioObjectConfig {
                        base: BaseScenarioObjectConfig {
                            id: PLAYER_ID.to_string(),
                            name: player_name,
                            position: Meters3::ZERO,
                            rotation: Quat::IDENTITY,
                        },
                        kind: ScenarioObjectKind::Spaceship(player),
                    },
                )],
                ThreePointRig::around("canister chips", Meters3::ZERO, 20.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "canister_chip_sandbox".to_string(),
            "Canister Chip Sandbox".to_string(),
            world.resource::<GameAssets>().cubemap.clone().into(),
        )
    }
}

/// `1` puts every canister back in front of the intake.
fn read_the_keys(keys: Res<ButtonInput<KeyCode>>, mut scatter: ResMut<Scatter>) {
    if keys.just_pressed(KeyCode::Digit1) {
        scatter.pending = true;
    }
}

/// The item kinds the canisters hold: the first [`KINDS`] catalog items, in
/// id order.
fn kinds(items: &GameItems) -> Vec<ItemDesignId> {
    let mut ids: Vec<ItemDesignId> = items.iter().map(|item| item.id.clone()).collect();
    ids.sort();
    assert!(
        ids.len() >= KINDS,
        "canister_chip_sandbox: the catalog holds {} items, the sandbox needs {KINDS}",
        ids.len()
    );
    ids.truncate(KINDS);
    ids
}

/// Clear every canister and stand the fourteen in front of the player's
/// intake, at rest with the ship.
fn scatter_the_canisters(world: &mut World) {
    if !world.resource::<Scatter>().pending {
        return;
    }
    let Some(ship) = world
        .query_filtered::<Entity, With<PlayerSpaceshipMarker>>()
        .iter(world)
        .next()
    else {
        return;
    };
    let mut intakes = world.query_filtered::<(
        &GlobalTransform,
        &SectionCollider,
        &CargoIntakeSectionConfigHelper,
        &ChildOf,
    ), With<CargoIntakeSectionMarker>>();
    let Some((transform, collider, config)) = intakes
        .iter(world)
        .find(|(.., ChildOf(parent))| *parent == ship)
        .map(|(transform, collider, config, _)| (*transform, *collider, (**config).clone()))
    else {
        return;
    };
    let Some(ship_pose) = world.get::<GlobalTransform>(ship).copied() else {
        return;
    };
    world.resource_mut::<Scatter>().pending = false;

    let old: Vec<Entity> = world
        .query_filtered::<Entity, With<CargoCanister>>()
        .iter(world)
        .collect();
    for canister in old {
        world.entity_mut(canister).despawn();
    }

    let (_, rotation, translation) = transform.to_scale_rotation_translation();
    let (face, normal) = cargo_intake_face(translation, rotation, collider);
    let nose = ship_pose.forward().as_vec3();
    let forward = (nose - normal * nose.dot(normal)).normalize();
    let right = forward.cross(normal).normalize();
    let ship_velocity = world
        .get::<LinearVelocity>(ship)
        .map(|velocity| velocity.0)
        .unwrap_or_default();
    let items = world.resource::<GameItems>().clone();
    let kinds = kinds(&items);
    for drop in &DROPS {
        let at = drop.at.to_engine();
        let position = face + right * at.x + normal * at.y + forward * at.z;
        let id = world.resource_mut::<CargoCanisterIdAllocator>().mint();
        world.spawn((
            cargo_canister(
                CargoCanister::new(&items, &kinds[drop.kind], drop.count),
                Transform::from_translation(position).with_rotation(rotation),
                ship_velocity,
                config.canister_mesh.clone(),
            ),
            id,
        ));
    }
    info!(
        "canister chip sandbox: {} canisters in front of the intake",
        DROPS.len()
    );
}

/// The canister chips on screen: each shown chip's size in UI px.
fn shown_canister_chips(world: &mut World) -> Vec<Vec2> {
    world
        .query::<(&ScreenLabelType, &ComputedNode, &Visibility)>()
        .iter(world)
        .filter(|(label, _, visibility)| {
            **label == ScreenLabelType::Canister && **visibility != Visibility::Hidden
        })
        .map(|(_, computed, _)| computed.size() * computed.inverse_scale_factor())
        .collect()
}

/// How many canisters stand in the reach of the player's intake.
fn canisters_in_reach(world: &mut World) -> usize {
    let Some(ship) = world
        .query_filtered::<Entity, With<PlayerSpaceshipMarker>>()
        .iter(world)
        .next()
    else {
        return 0;
    };
    world
        .resource::<CargoPickupReadiness>()
        .pairs
        .iter()
        .filter(|pair| pair.ship == ship && pair.in_reach)
        .count()
}

/// The panel: canisters in reach, canister chips shown and their area.
fn draw_the_chips(world: &mut World) {
    let in_reach = canisters_in_reach(world);
    let chips = shown_canister_chips(world);
    let area: f32 = chips.iter().map(|size| size.x * size.y).sum();
    let mut panel = world.resource_mut::<DiagnosticsPanel>();
    panel.hint("1", "put the canisters back");
    panel.line(format!("canisters in reach: {in_reach}"));
    panel.line(format!("canister chips: {}, {area:.0} px2", chips.len()));
}

mod smoke {
    use std::sync::Arc;

    use bevy::prelude::*;
    use nova_protocol::{
        nova_debug::harness::{shoot, AutopilotPlugin, Predicate},
        prelude::*,
    };
    use nova_ui::theme::{ActiveUiTheme, SelectedUiTheme};

    use super::{canisters_in_reach, shown_canister_chips, DROPS, PLAYER_ID};

    /// Real seconds the load may take on lavapipe.
    const LOAD_DEADLINE: f32 = 600.0;
    /// Real seconds a beat or a shot may take on lavapipe.
    const BEAT_DEADLINE: f32 = 300.0;
    /// Frames the chips get to lay out and fit after the clock holds.
    const SETTLE_FRAMES: u32 = 6;
    /// Frames a theme switch gets to re-tint.
    const THEME_FRAMES: u32 = 4;
    /// The themes every frame is shot on, by id and file tag.
    const THEMES: [(&str, &str); 2] = [
        (nova_ui::theme::PHOSPHOR_THEME_ID, "phosphor"),
        (nova_ui::theme::HARDWARE_THEME_ID, "hardware"),
    ];

    fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(check)
    }

    fn player_staged(world: &World) -> bool {
        world
            .try_query_filtered::<&EntityId, Without<ChildOf>>()
            .is_some_and(|mut query| query.iter(world).any(|id| id.0 == PLAYER_ID))
    }

    fn all_in_reach(world: &World) -> bool {
        let Some(mut ships) = world.try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()
        else {
            return false;
        };
        let Some(ship) = ships.iter(world).next() else {
            return false;
        };
        world
            .get_resource::<CargoPickupReadiness>()
            .is_some_and(|readiness| {
                readiness
                    .pairs
                    .iter()
                    .filter(|pair| pair.ship == ship && pair.in_reach)
                    .count()
                    == DROPS.len()
            })
    }

    fn set_virtual_paused(paused: bool) -> impl Fn(&mut World) + Send + Sync + 'static {
        move |world: &mut World| {
            let mut time = world.resource_mut::<Time<Virtual>>();
            if paused {
                time.pause();
            } else {
                time.unpause();
            }
        }
    }

    /// Log every shown canister chip and the screen area they take.
    fn log_the_area(world: &mut World) {
        let in_reach = canisters_in_reach(world);
        let chips = shown_canister_chips(world);
        let area: f32 = chips.iter().map(|size| size.x * size.y).sum();
        for size in &chips {
            info!(
                "canister chip sandbox: chip {:.1} x {:.1} px",
                size.x, size.y
            );
        }
        info!(
            "canister chip sandbox: {in_reach} canisters in reach, {} chips, {area:.0} px2",
            chips.len()
        );
        nova_probe::probe_marker(
            world,
            "outcome: canister chip area",
            serde_json::json!({ "chips": chips.len(), "area_px2": area }),
        );
    }

    /// The two clusters each draw one list chip and the pair and the single
    /// keep a chip each: five canister chips on screen.
    fn assert_the_grouping(world: &mut World) {
        let mut lists: Vec<usize> = world
            .query::<&CargoCanisterListMembers>()
            .iter(world)
            .map(|members| members.len())
            .collect();
        lists.sort();
        assert_eq!(
            lists,
            vec![3, 8],
            "canister_chip_sandbox: the clusters of three and eight each share one list"
        );
        let shown = shown_canister_chips(world).len();
        assert_eq!(
            shown, 5,
            "canister_chip_sandbox: two lists, the pair and the single show {shown} chips, not 5"
        );
        nova_probe::probe_marker(
            world,
            "outcome: two clusters share a list chip each, the pair and the single keep theirs",
            serde_json::json!({ "lists": format!("{lists:?}"), "chips": shown }),
        );
        info!("canister chip sandbox: PASS two lists of 3 and 8, five chips on screen");
    }

    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let mut script = AutopilotPlugin::<GameStates>::new()
            .step("canister chip sandbox: load the cast")
            .enter(GameStates::Loading)
            .until(and(
                and(state_is(GameStates::Playing), player_ship_present()),
                when(player_staged),
            ))
            .deadline(LOAD_DEADLINE)
            .add()
            .step("canister chip sandbox: every canister in reach")
            .until(when(all_in_reach))
            .deadline(BEAT_DEADLINE)
            .add()
            .step("canister chip sandbox: hold the clock")
            .on_enter(set_virtual_paused(true))
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("canister chip sandbox: log the chip area")
            .on_enter(log_the_area)
            .add();
        for (theme, tag) in THEMES {
            let shot = format!("canister-chips-{tag}.png");
            let written = shot.clone();
            script = script
                .step(format!("canister chip sandbox: switch to {tag}"))
                .on_enter(move |world: &mut World| {
                    world.resource_mut::<SelectedUiTheme>().0 = theme.to_string();
                })
                .until(when(move |world| {
                    world.resource::<ActiveUiTheme>().id() == theme
                }))
                .deadline(BEAT_DEADLINE)
                .add()
                .step(format!("canister chip sandbox: settle {tag}"))
                .until(frames(THEME_FRAMES))
                .add()
                .step(format!("canister chip sandbox: shoot {shot}"))
                .on_enter(move |world: &mut World| shoot(world, &shot))
                .until(shot_written(written))
                .deadline(BEAT_DEADLINE)
                .add();
        }
        script
            .step("canister chip sandbox: back to the default theme")
            .on_enter(|world: &mut World| {
                *world.resource_mut::<SelectedUiTheme>() = SelectedUiTheme::default();
            })
            .until(frames(THEME_FRAMES))
            .add()
            .step("canister chip sandbox: assert the grouping")
            .on_enter(assert_the_grouping)
            .add()
    }
}
