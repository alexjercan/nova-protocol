//! greeble_solve: one shipped hull in its own style, with the greeble solve
//! drawn over it.
//!
//! The skin of a ship is read as a grid of cells, one per plate, each with four
//! edges. The style's `greeble_rules` put one greeble on every cell, and the
//! link types on two facing edges must connect. This example spawns one
//! catalog hull, clad and dressed by the game's own spawn path, and solves the
//! same rules again for the diagnostics, so the drawing shows what placed the
//! pieces you see.
//!
//! What to try:
//!
//! 1. `N` loads the next hull of the base fleet, `--ship <id>` starts on one;
//! 2. fly round the hull (WASD, mouse) and match each fixture to its cell;
//! 3. follow a line greeble (a stripe, a strake, a weld seam) from end piece
//!    to end piece along its link points.
//!
//! ```text
//! nix develop --command cargo run --example greeble_solve --features debug
//! nix develop --command cargo run --example greeble_solve --features debug -- --ship raider
//! ```
//!
//! The diagnostics (`F3`) draw every cell of the solve: a green sphere where
//! the greeble places a fixture and a grey one where it is empty space, each
//! fixture labelled with its greeble id. Each cell draws its four link points
//! as ticks toward its edges, coloured by the link type the placed greeble
//! shows there. A line joins two linked edges, green where the two link types
//! connect and red where they do not, labelled "no fit"; a red tick is an
//! edge with no neighbour. The panel names the hull and style, and the links
//! that do not connect or the fallback when there are any; `N` is hinted.
//! Detail (`Shift+F3`) counts cells, links and border edges, reports restarts,
//! and lists the cells each greeble took.
//!
//! Under `NOVA_AUTOPILOT=1` a smoke walk waits for the hull, checks that every
//! fixture cell of the solve has one spawned piece, and shoots
//! `greeble-solve.png` with `NOVA_CAPTURE=1`. Unset, no walk and no deadline
//! run.

#[path = "../screenshots/shared/kit.rs"]
mod kit;

use bevy::prelude::*;
use clap::Parser;
use diagnostics::{palette, DiagnosticsGizmos, DiagnosticsPanel, DiagnosticsSystems};
use nova_protocol::{nova_debug::diagnostics, prelude::*};

#[derive(Parser)]
#[command(name = "greeble_solve")]
#[command(version = "1.0.0")]
#[command(
    about = "One shipped hull in its style, with the greeble solve drawn over it",
    long_about = None
)]
struct Cli {
    /// The catalog ship to start on.
    #[arg(long, default_value = "light_freighter")]
    ship: String,
}

/// The base fleet `N` steps through, in catalog order.
const FLEET: [&str; 9] = [
    "mining_tug",
    "salvage_hauler",
    "light_freighter",
    "interceptor",
    "raider",
    "patrol_picket",
    "line_warship",
    "freighter_derelict",
    "debris_plate",
];

/// The eye's offset from the hull: above the port bow, as in
/// `screenshot_base_fleet`.
const EYE: Meters3 = Meters3::new(-110.0, 55.0, -95.0);

/// The radius of a cell's marker, in cells.
const CELL_MARK: f32 = 0.06;

/// How far from a cell's middle its link point tick reaches, in cells. Short
/// of the edge, so two ticks that meet at one edge stay apart.
const TICK_REACH: f32 = 0.38;

/// How far off the plate the drawing stands, in cells, so the plate does not
/// hide it.
const LIFT: f32 = 0.08;

/// The colour of each link type, by its index in the style's rules.
const LINK_COLOURS: [Color; 5] = [
    palette::VELOCITY,
    palette::TARGET,
    palette::ZONE,
    palette::PUSH,
    palette::MUTED,
];

fn main() -> bevy::app::AppExit {
    let cli = Cli::parse();
    let start = FLEET
        .iter()
        .position(|id| *id == cli.ship)
        .unwrap_or_else(|| panic!("greeble_solve: '{}' is not a base fleet ship", cli.ship));

    let mut app = AppBuilder::new()
        .with_game_plugins(move |app: &mut App| {
            app.insert_resource(Focus(start));
            app.init_resource::<Solved>();
            app.add_systems(OnEnter(GameAssetsStates::Loaded), load_hull);
            app.add_systems(
                Update,
                (
                    next_hull_on_key.run_if(in_state(GameStates::Playing)),
                    frame_new_camera,
                    solve_the_hull.run_if(in_state(GameStates::Playing)),
                ),
            );
        })
        .build();
    app.add_plugins(diagnostics::DiagnosticsHintPlugin::new(
        "one base hull with its greeble solve drawn over it",
    ));
    app.add_systems(
        PostUpdate,
        draw_the_solve
            .in_set(DiagnosticsSystems)
            .after(TransformSystems::Propagate)
            .run_if(in_state(GameStates::Playing)),
    );
    // Nothing flies the hull; a still hull keeps the drawing on its plates.
    app.add_systems(Update, freeze_bodies);
    if std::env::var_os("NOVA_AUTOPILOT").is_some() {
        app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
            bevy::ecs::error::panic,
        ));
        app.add_systems(Startup, force_capture_resolution);
    }
    app.add_plugins(smoke::script());

    app.run()
}

/// The index into [`FLEET`] of the hull on stage.
#[derive(Resource, Clone, Copy)]
struct Focus(usize);

/// The solve of the hull on stage, in the ship's own frame.
#[derive(Resource, Default)]
struct Solved {
    /// The ship root the solve is for, and the sections it read.
    source: Option<(Entity, usize)>,
    /// The style the hull wears.
    style: String,
    /// One row per cell, in plate order.
    cells: Vec<SolvedCell>,
    /// Per cell and slot, the neighbour cell and its slot.
    links: Vec<[Option<(usize, usize)>; 4]>,
    /// Per link type, the link types it connects.
    connects: Vec<Vec<bool>>,
    /// How many times the solve started again.
    restarts: u32,
    /// Whether the solve ran without the forcing greebles.
    fallback: bool,
    /// Per greeble id, the cells it took, in rule order.
    taken: Vec<(String, usize)>,
}

/// One cell of the solve.
struct SolvedCell {
    /// The middle of the plate, in the ship's frame.
    centre: Vec3,
    /// The face the plate shows to space.
    out: IVec3,
    /// The placed greeble's id, or `None` where the cell contradicted.
    greeble: Option<String>,
    /// Whether the greeble places a fixture.
    fixture: bool,
    /// The link type index on each slot, after the turn.
    sides: Option<[usize; 4]>,
}

/// Load the focused hull once the assets are in.
fn load_hull(
    commands: Commands,
    focus: Res<Focus>,
    game_assets: Res<GameAssets>,
    ships: Res<GameShipDesigns>,
) {
    load_scenario(commands, FLEET[focus.0], &game_assets, &ships);
}

/// Load hull `id`, alone, under the photo rig.
fn load_scenario(
    mut commands: Commands,
    id: &str,
    game_assets: &GameAssets,
    ships: &GameShipDesigns,
) {
    let hull = EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: format!("hull_{id}"),
            name: id.to_string(),
            position: Meters3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            controller: SpaceshipController::None,
            allegiance: Some(Allegiance::Neutral),
            design: ShipDesignSource::Inline(kit::catalog_ship(ships, id)),
            ..default()
        }),
    });
    let rig = ThreePointRig::around("photo", Meters3::ZERO, 1.0).actions();
    commands.trigger(LoadScenario(ScenarioConfig {
        description: "One base hull with its greeble solve drawn over it.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: std::iter::once(hull).chain(rig).collect(),
        }],
        ..ScenarioConfig::new(
            "greeble_solve".to_string(),
            "Greeble Solve".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }));
}

/// `N` loads the next hull of the fleet.
fn next_hull_on_key(
    keys: Res<ButtonInput<KeyCode>>,
    mut focus: ResMut<Focus>,
    mut solved: ResMut<Solved>,
    commands: Commands,
    game_assets: Res<GameAssets>,
    ships: Res<GameShipDesigns>,
) {
    if !keys.just_pressed(KeyCode::KeyN) {
        return;
    }
    focus.0 = (focus.0 + 1) % FLEET.len();
    *solved = Solved::default();
    load_scenario(commands, FLEET[focus.0], &game_assets, &ships);
}

/// Stand every new scenario camera off the hull's port bow.
///
/// The free-fly rig reads its pose from the `Transform` when its component is
/// inserted and writes the `Transform` from then on, so the rig component is
/// inserted again after the pose is set.
fn frame_new_camera(
    mut commands: Commands,
    focus: Res<Focus>,
    mut q_camera: Query<
        (Entity, &mut Transform, &WASDCamera),
        (With<ScenarioCameraMarker>, Added<ScenarioCameraMarker>),
    >,
) {
    let scale = match FLEET[focus.0] {
        "line_warship" => 2.0,
        "debris_plate" => 0.6,
        _ => 1.0,
    };
    for (camera, mut transform, rig) in &mut q_camera {
        *transform =
            Transform::from_translation((EYE * scale).to_engine()).looking_at(Vec3::ZERO, Vec3::Y);
        commands.entity(camera).insert(*rig);
    }
}

/// Solve the hull's style over its skin, as `spawn_ship_skin` does, whenever
/// the hull or its section count changes.
fn solve_the_hull(
    mut solved: ResMut<Solved>,
    styles: Option<Res<GameStyles>>,
    q_root: Query<(Entity, &ShipSkin, &ShipStyle, &Children), With<SpaceshipRootMarker>>,
    q_sections: Query<
        (
            &Transform,
            &SectionLinkPoints,
            Option<&SectionExit>,
            Option<&SectionFootprint>,
        ),
        With<SectionMarker>,
    >,
) {
    let Some(styles) = styles else { return };
    let Some((root, skin, worn, children)) = q_root.iter().next() else {
        return;
    };
    if !skin.0 {
        return;
    }
    let sections: Vec<_> = children
        .iter()
        .filter_map(|child| q_sections.get(child).ok())
        .collect();
    if solved.source == Some((root, sections.len())) || sections.is_empty() {
        return;
    }
    let Some(style) = worn.resolve(&styles) else {
        return;
    };
    let Ok(Some(greebles)) = style.compile_greebles() else {
        return;
    };
    let placed: Vec<PlacedPart> = sections
        .iter()
        .map(|(pose, sockets, exit, footprint)| PlacedPart {
            position: pose.translation,
            rotation: pose.rotation,
            link_points: sockets.0.as_slice(),
            footprint: footprint.map_or(UVec3::ONE, |footprint| footprint.0),
            exit: exit.map(|exit| exit.0),
            port: false,
        })
        .collect();
    let (structure, phase, _) = read_structure(&placed);
    let plates = derive_skin(&structure);
    let surface = skin_surface(&structure, &plates);
    // The spawn path's own solve: a piece only where its plate can seat it.
    let solution = solve_seated_greebles(&plates, &surface, style, &greebles);

    let rules = &style.greeble_rules;
    let link_index = |id: &GreebleLinkDesignId| {
        rules
            .link_types
            .iter()
            .position(|link| &link.id == id)
            .expect("a compiled rule set names only declared link types")
    };
    let connects: Vec<Vec<bool>> = rules
        .link_types
        .iter()
        .map(|link| {
            rules
                .link_types
                .iter()
                .map(|other| link.connects.contains(&other.id))
                .collect()
        })
        .collect();
    let mut taken: Vec<(String, usize)> = (0..greebles.rules.greeble_count())
        .map(|index| (greebles.rules.greeble_id(index).as_str().to_string(), 0))
        .collect();
    let cells = plates
        .iter()
        .zip(&solution.cells)
        .map(|(plate, placement)| {
            let placed = placement.map(|placement| {
                taken[placement.greeble].1 += 1;
                let id = greebles.rules.greeble_id(placement.greeble);
                let config = rules
                    .greebles
                    .iter()
                    .find(|greeble| &greeble.id == id)
                    .expect("a compiled greeble is in its rule set");
                let points = &config.link_points;
                let own = [&points.up, &points.right, &points.down, &points.left];
                // Turn `t` puts the greeble's `up` link point on slot `t`.
                let sides: [usize; 4] = std::array::from_fn(|slot| {
                    link_index(own[(slot + 4 - usize::from(placement.turns)) % 4])
                });
                (
                    id.as_str().to_string(),
                    greebles.rules.fixture(placement.greeble).is_some(),
                    sides,
                )
            });
            SolvedCell {
                centre: plate.cell.as_vec3() + phase,
                out: plate.cell - plate.anchor,
                greeble: placed.as_ref().map(|(id, ..)| id.clone()),
                fixture: placed.as_ref().is_some_and(|(_, fixture, _)| *fixture),
                sides: placed.map(|(.., sides)| sides),
            }
        })
        .collect();
    let links = surface
        .cells()
        .iter()
        .map(|cell| {
            cell.links
                .map(|link| link.map(|link| (link.cell, usize::from(link.slot))))
        })
        .collect();
    *solved = Solved {
        source: Some((root, sections.len())),
        style: style.id.clone(),
        cells,
        links,
        connects,
        restarts: solution.restarts,
        fallback: solution.fallback,
        taken,
    };
}

/// Draw the cells, their link points and the links, and fill the panel.
fn draw_the_solve(
    focus: Res<Focus>,
    solved: Res<Solved>,
    q_root: Query<&GlobalTransform, With<SpaceshipRootMarker>>,
    mut gizmos: Gizmos<DiagnosticsGizmos>,
    mut panel: ResMut<DiagnosticsPanel>,
) {
    panel.hint("N", "next hull");
    let Some((root, _)) = solved.source else {
        panel.line(format!("hull {}: waiting for it", FLEET[focus.0]));
        return;
    };
    let Ok(ship) = q_root.get(root) else {
        return;
    };
    let world = |local: Vec3| ship.transform_point(local);
    // The point of `cell`'s link point on `slot`, lifted off the plate.
    let tick = |cell: &SolvedCell, slot: usize| {
        cell.centre
            + slot_direction(cell.out, slot).as_vec3() * TICK_REACH
            + cell.out.as_vec3() * LIFT
    };

    let mut borders = 0;
    let mut links = 0;
    let mut clashes = 0;
    for (index, cell) in solved.cells.iter().enumerate() {
        let middle = cell.centre + cell.out.as_vec3() * LIFT;
        let colour = match (&cell.greeble, cell.fixture) {
            (None, _) => palette::ALERT,
            (Some(_), true) => palette::OK,
            (Some(_), false) => palette::MUTED,
        };
        diagnostics::sphere(&mut gizmos, world(middle), CELL_MARK, colour);
        if cell.fixture {
            if let Some(id) = &cell.greeble {
                panel.label(world(middle), id.clone());
            }
        }
        for slot in 0..4 {
            let end = tick(cell, slot);
            let link = solved.links[index][slot];
            let colour = match (cell.sides, link) {
                (_, None) => palette::ALERT,
                (Some(sides), Some(_)) => LINK_COLOURS[sides[slot] % LINK_COLOURS.len()],
                (None, Some(_)) => palette::MUTED,
            };
            gizmos.line(world(middle), world(end), colour);
            let Some((other, back)) = link else {
                borders += 1;
                continue;
            };
            // Each link is drawn once, from its lower cell.
            if other < index {
                continue;
            }
            links += 1;
            let far = &solved.cells[other];
            let fits = match (cell.sides, far.sides) {
                (Some(near), Some(far)) => solved.connects[near[slot]][far[back]],
                _ => false,
            };
            if !fits {
                clashes += 1;
            }
            let colour = if fits { palette::OK } else { palette::ALERT };
            gizmos.line(world(end), world(tick(far, back)), colour);
            if !fits {
                panel.label_tinted(
                    world(end.midpoint(tick(far, back))),
                    "no fit",
                    palette::ALERT,
                );
            }
        }
    }

    panel.line(format!("hull {}   style {}", FLEET[focus.0], solved.style));
    if clashes > 0 {
        panel.line(format!("{clashes} links do not connect"));
    }
    if solved.fallback {
        panel.line("the solve fell back");
    }
    panel.detail(format!(
        "{} cells, {links} links, {borders} border edges, {clashes} links that do not connect",
        solved.cells.len()
    ));
    panel.detail(format!(
        "restarts {}, fallback {}",
        solved.restarts,
        if solved.fallback { "yes" } else { "no" }
    ));
    for (id, count) in &solved.taken {
        panel.detail(format!("  {id}: {count}"));
    }
}

mod smoke {
    use std::sync::Arc;

    use nova_protocol::nova_debug::harness::{shoot, AutopilotPlugin};

    use super::*;

    /// Seconds the hull gets to load, clad and solve on a software GPU.
    const SETUP_SECS: f32 = 300.0;
    /// Seconds a shot gets to land.
    const SHOT_SECS: f32 = 120.0;
    /// Frames the view gets before the shot.
    const SETTLE_FRAMES: u32 = 20;
    /// The PNG of the hull with the solve drawn over it.
    const SHOT: &str = "greeble-solve.png";

    fn solved(world: &World) -> bool {
        world
            .get_resource::<Solved>()
            .is_some_and(|solved| !solved.cells.is_empty())
    }

    /// Every fixture cell of the solve has one spawned piece, and no link of
    /// the solve fails to connect.
    fn assert_the_solve(world: &mut World) {
        let solved = world.resource::<Solved>();
        let fixtures = solved.cells.iter().filter(|cell| cell.fixture).count();
        let links_fit = solved.cells.iter().enumerate().all(|(index, cell)| {
            (0..4).all(|slot| match (solved.links[index][slot], cell.sides) {
                (Some((other, back)), Some(near)) => solved.cells[other]
                    .sides
                    .is_some_and(|far| solved.connects[near[slot]][far[back]]),
                _ => true,
            })
        });
        let fallback = solved.fallback;
        let cells = solved.cells.len();
        let pieces = world
            .query_filtered::<(), With<ShipDecorMarker>>()
            .iter(world)
            .count();
        assert!(fixtures > 0, "greeble solve: the solve places fixtures");
        assert_eq!(
            fixtures, pieces,
            "greeble solve: one spawned piece per fixture cell of the solve"
        );
        assert!(!fallback, "greeble solve: the solve needs no fallback");
        assert!(links_fit, "greeble solve: every link connects");
        info!("greeble solve: PASS {cells} cells, {pieces} pieces match the solve");
    }

    /// The smoke walk: the hull loads and solves, the solve matches the
    /// spawned pieces, and a frame with the diagnostics up is shot.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        AutopilotPlugin::<GameStates>::new()
            .step("greeble solve: load and solve the hull")
            .until(Arc::new(solved))
            .deadline(SETUP_SECS)
            .add()
            .step("greeble solve: check the solve")
            .on_enter(assert_the_solve)
            .add()
            .step("greeble solve: settle the view")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("greeble solve: shoot the hull")
            .on_enter(|world: &mut World| shoot(world, SHOT))
            .until(shot_written(SHOT))
            .deadline(SHOT_SECS)
            .add()
    }
}
