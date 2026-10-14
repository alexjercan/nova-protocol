//! start_role_sandbox: start the open world as any start role, and fly it.
//!
//! Boots the shipped app and starts New Game on a pinned seed as the picked
//! start role, as the Create World modal does after the role page, but
//! without a save folder: nothing this run does is saved. The role comes from
//! `--role <id>`, else the `NOVA_ROLE` variable, else `industrial`; an id no
//! loaded mod authors stops the run and names the known ones. The base roles
//! are `industrial`, `scavenger`, `transport`, `bounty_hunter`, `military`
//! and `pirate`.
//!
//! `--start-km <km>` (or `NOVA_START_KM`) jumps the player that far out
//! along +X once the spawn's window is in, to fly where the advancement curve
//! fields larger hulls: small hulls only in the inner rings, medium ones near
//! 2,000 km, the largest past 3,000 km. The panel lists the generated hulls
//! in the window with their spans.
//!
//! What to try:
//!
//! 1. fly the role's hull with its bound kit: the trigger fires every
//!    turret, `R` the railgun, `F` the torpedo bay, `V` the mining beam;
//! 2. TAB opens the interface: the Ship tab shows the hull, `M` steps to the
//!    Map, and `C` there draws the chart of the cells entered;
//! 3. `:` opens the shell: `cheats enable`, then `credits give <ship-id>
//!    1000` or `credits set <ship-id> 0`, with the ship id from the panel;
//! 4. Esc opens the pause menu; Settings > Graphics has Pause when
//!    unfocused;
//! 5. jettison and take back: TAB, Inventory, click a hold row, Jettison,
//!    pick the quantity, Confirm; the canister leaves the intake door. Then
//!    fly the intake door (top or bottom face, per the panel) onto the
//!    canister: the box in front of the door is the pickup area.
//!
//! ```text
//! nix develop --command cargo run --example start_role_sandbox --features debug -- --role pirate
//! ```
//!
//! The diagnostics panel (`F3`) shows the role, the hull, the kit, the live
//! balance and hold, the hull's value at ask and its trade-in by health, the
//! cell and the distance from the origin, and the generated hulls in the
//! window, whether the player hull has an intake and which face it is on,
//! the pickups ready now, and the last jettison and pickup (item, count).
//! The gizmos ring each generated hull at its containment radius, labelled
//! with its span; draw each ship's intake face, its pickup area (the
//! trigger box), its detection range and its normal; and ring each drifting
//! canister, labelled with its contents.
//!
//! Under `NOVA_AUTOPILOT=1` a smoke walk waits for the window,
//! checks the player is the role's hull with the role's kit and an intake,
//! jettisons one unit of the kit's first item as the Inventory pane's
//! Jettison applies its plan, waits for the panel to log it, and shoots
//! `start-role-<role>.png` with `NOVA_CAPTURE=1`. Unset, no walk and no
//! deadline run.

#[path = "shared/open_world_stage.rs"]
mod open_world_stage;

use avian3d::prelude::Position;
use bevy::prelude::*;
use clap::Parser;
use diagnostics::{palette, DiagnosticsGizmos, DiagnosticsPanel, DiagnosticsSystems};
use nova_protocol::{nova_debug::diagnostics, prelude::*};
use open_world_stage::{cell_of, the_player, window_settled, OpenWorldStartPlugin};

#[derive(Parser)]
#[command(name = "start_role_sandbox")]
#[command(version = "1.0.0")]
#[command(
    about = "Start the open world as a picked start role, with no save folder, and fly it",
    long_about = None
)]
struct Cli {
    /// The start role id; else `NOVA_ROLE`, else `industrial`.
    #[arg(long)]
    role: Option<String>,
    /// The world seed.
    #[arg(long, default_value_t = SANDBOX_SEED)]
    seed: u32,
    /// Jump this many kilometres out along +X once the spawn's window is in;
    /// else `NOVA_START_KM`, else stay at the spawn.
    #[arg(long)]
    start_km: Option<f32>,
}

/// The default world seed: the one the station sandboxes and
/// `system_start_roles` use.
const SANDBOX_SEED: u32 = 10;
/// The role a run with no `--role` and no `NOVA_ROLE` starts as.
const DEFAULT_ROLE: &str = "industrial";

fn main() -> bevy::app::AppExit {
    let cli = Cli::parse();
    let role = cli
        .role
        .or_else(|| std::env::var("NOVA_ROLE").ok())
        .unwrap_or_else(|| DEFAULT_ROLE.to_string());
    let start_km = cli.start_km.or_else(|| {
        std::env::var("NOVA_START_KM").ok().map(|km| {
            km.parse()
                .unwrap_or_else(|_| panic!("start role sandbox: NOVA_START_KM '{km}' is no number"))
        })
    });
    open_world_stage::isolate_the_profile(env!("CARGO_CRATE_NAME"));
    let shot = format!("start-role-{role}.png");

    let mut app = nova_protocol::prelude::editor_app(true, None);
    app.add_plugins(OpenWorldStartPlugin {
        seed: cli.seed,
        role: role.clone(),
    });
    app.insert_resource(Sandbox {
        role,
        start_km,
        phase: SandboxPhaseType::Streaming,
    });
    app.add_plugins(diagnostics::DiagnosticsHintPlugin::new(
        "fly the role's hull   [TAB] interface, [M] next tab, [C] chart on Map   TAB Inventory: \
         Jettison, then fly the intake box onto the canister   [:] shell: cheats enable",
    ));
    app.add_systems(
        Update,
        drive_the_sandbox.run_if(in_state(GameStates::Playing)),
    );
    app.add_systems(
        PostUpdate,
        draw_the_start
            .in_set(DiagnosticsSystems)
            .after(TransformSystems::Propagate)
            .run_if(in_state(GameStates::Playing)),
    );
    app.init_resource::<IntakeLog>();
    app.add_observer(note_a_canister_born);
    app.add_observer(note_a_canister_gone);
    app.add_observer(note_a_jettison);
    app.add_observer(note_a_pickup);
    app.add_systems(
        PostUpdate,
        draw_the_intakes
            .in_set(DiagnosticsSystems)
            .after(draw_the_start)
            .after(TransformSystems::Propagate)
            .run_if(in_state(GameStates::Playing)),
    );

    if std::env::var_os("NOVA_AUTOPILOT").is_some() {
        app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
            bevy::ecs::error::panic,
        ));
        app.add_systems(Update, open_world_stage::hold_the_walk_clock);
        app.add_systems(
            Startup,
            nova_protocol::nova_debug::harness::force_capture_resolution,
        );
    }
    app.add_plugins(smoke::script(&shot));

    app.run()
}

/// Where the sandbox's setup stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SandboxPhaseType {
    /// The window around the spawn is streaming in.
    Streaming,
    /// The player jumped out; the new window is streaming in.
    Jumped,
    /// The player has the run.
    Ready,
}

/// The sandbox's setup.
#[derive(Resource, Debug)]
struct Sandbox {
    role: String,
    start_km: Option<f32>,
    phase: SandboxPhaseType,
}

/// Jump the player out once the spawn's window is in, if asked, and hand it
/// the run once its window is in.
fn drive_the_sandbox(world: &mut World) {
    let phase = world.resource::<Sandbox>().phase;
    match phase {
        SandboxPhaseType::Streaming => {
            if !window_settled(world) {
                return;
            }
            let Some(km) = world.resource::<Sandbox>().start_km else {
                world.resource_mut::<Sandbox>().phase = SandboxPhaseType::Ready;
                info!("start role sandbox: ready at the spawn");
                return;
            };
            let Some(player) = the_player(world) else {
                return;
            };
            let at = Meters3(Vec3::X * km * 1_000.0);
            open_world_stage::jump_to(world, player, at);
            world.resource_mut::<Sandbox>().phase = SandboxPhaseType::Jumped;
            info!("start role sandbox: jumped {km} km out along +X");
        }
        SandboxPhaseType::Jumped => {
            if window_settled(world) {
                world.resource_mut::<Sandbox>().phase = SandboxPhaseType::Ready;
                info!("start role sandbox: ready");
            }
        }
        SandboxPhaseType::Ready => {}
    }
}

/// The role's hull's value at ask, and what a station pays for it now: each
/// live section's recipe at its health share.
fn hull_values(
    items: &GameItems,
    design: &ShipDesignSections,
    live: &[(String, f32)],
) -> (u64, u64) {
    let price = hull_price_cr(
        items,
        design.0.iter().map(|section| &section.config.base.build),
    );
    let trade_in = hull_trade_in_cr(
        items,
        live.iter().filter_map(|(id, share)| {
            design
                .0
                .iter()
                .find(|section| section.id == *id)
                .map(|section| (&section.config.base.build, *share))
        }),
    );
    (price, trade_in)
}

/// The role, the hull, the kit, the balance, the hold and the hull's value,
/// where the player stands, and every generated hull in the window ringed
/// and labelled with its span.
#[expect(
    clippy::too_many_arguments,
    reason = "one panel reading the role, the player, its sections and the world's hulls"
)]
fn draw_the_start(
    mut gizmos: Gizmos<DiagnosticsGizmos>,
    mut panel: ResMut<DiagnosticsPanel>,
    sandbox: Res<Sandbox>,
    roles: Res<GameStartRoles>,
    items: Option<Res<GameItems>>,
    q_player: Query<
        (
            &Position,
            &EntityId,
            &ShipDesignSections,
            &ShipCredits,
            &ShipInventory,
            &Children,
        ),
        With<PlayerSpaceshipMarker>,
    >,
    q_sections: Query<(&EntityId, &Health), With<SectionMarker>>,
    q_hulls: Query<
        (&GlobalTransform, &HullRadius, Has<StationMarker>),
        (With<GeneratedBodyId>, With<SpaceshipRootMarker>),
    >,
    config: Option<Res<nova_world::prelude::WorldConfig<NovaLayeredWorld>>>,
) {
    let role = roles.get_role(&StartRoleDesignId::from(sandbox.role.as_str()));
    match role {
        Some(role) => {
            panel.line(format!("role: {} ({})", role.name, role.id));
            let kit: Vec<String> = role
                .kit
                .stock
                .stacks()
                .map(|(item, count)| format!("{count} {item}"))
                .collect();
            panel.line(format!(
                "hull: {}   kit: {} cr, {}",
                role.hull,
                role.kit.credits,
                kit.join(", ")
            ));
        }
        None => panel.line(format!("role: '{}' is not loaded", sandbox.role)),
    }
    panel.line(format!("setup: {:?}", sandbox.phase));
    if let (Ok((position, id, design, credits, hold, children)), Some(items)) =
        (q_player.single(), items.as_deref())
    {
        let live: Vec<(String, f32)> = children
            .iter()
            .filter_map(|child| q_sections.get(child).ok())
            .map(|(id, health)| (id.0.clone(), health.current / health.max))
            .collect();
        let (price, trade_in) = hull_values(items, design, &live);
        panel.line(format!(
            "ship id: {}   credits: {} cr   hold: {:.0}/{:.0} kg",
            id.0,
            credits.0,
            hold.used_g(items) as f32 / 1_000.0,
            (u64::from(hold.used_g(items)) + u64::from(hold.free_g(items))) as f32 / 1_000.0,
        ));
        panel.line(format!(
            "hull value: {price} cr at ask, trade-in {trade_in} cr by health, {}/{} sections live",
            live.len(),
            design.0.len()
        ));
        let at = Meters3::from_engine(position.0);
        let cell = config
            .as_deref()
            .map(|config| SectorCoord::containing(at, config.sector_edge).to_string());
        panel.line(format!(
            "cell {}   {:.1} km from the origin",
            cell.unwrap_or_else(|| "-".to_string()),
            at.distance(Meters3::ZERO).get() / 1_000.0
        ));
    }
    let mut spans: Vec<f32> = Vec::new();
    let mut stations = 0;
    for (transform, radius, station) in &q_hulls {
        let span = Meters::from_engine(radius.0 * 2.0).get();
        let centre = transform.translation();
        if station {
            stations += 1;
            diagnostics::sphere(&mut gizmos, centre, radius.0, palette::ZONE);
            panel.label(centre, format!("station {span:.0} m"));
        } else {
            spans.push(span);
            diagnostics::sphere(&mut gizmos, centre, radius.0, palette::TARGET);
            panel.label(centre, format!("{span:.0} m"));
        }
    }
    spans.sort_by(|a, b| b.total_cmp(a));
    panel.line(format!(
        "generated hulls in the window: {} ships, {stations} stations; longest spans {:?} m",
        spans.len(),
        spans
            .iter()
            .take(5)
            .map(|span| span.round())
            .collect::<Vec<_>>()
    ));
}

/// One canister's contents as `(item, count)` pairs.
type Contents = Vec<(String, u32)>;

/// The player's last jettison and pickup. The intake triggers its event
/// after it spawns or despawns the canister, and the event names only the
/// intake, so the canister's contents wait in `moved` until the event says
/// which intake moved it.
#[derive(Resource, Default, Debug)]
struct IntakeLog {
    moved: Contents,
    last_jettison: Option<Contents>,
    last_pickup: Option<Contents>,
}

fn contents_of(canister: &CargoCanister) -> Contents {
    canister
        .stacks()
        .map(|(item, count)| (item.to_string(), count))
        .collect()
}

fn show_contents(contents: Option<&Contents>) -> String {
    contents.map_or_else(
        || "-".to_string(),
        |contents| {
            contents
                .iter()
                .map(|(item, count)| format!("{count} {item}"))
                .collect::<Vec<_>>()
                .join(", ")
        },
    )
}

fn note_a_canister_born(
    add: On<Add, CargoCanister>,
    q_canisters: Query<&CargoCanister>,
    mut log: ResMut<IntakeLog>,
) {
    if let Ok(canister) = q_canisters.get(add.entity) {
        log.moved = contents_of(canister);
    }
}

fn note_a_canister_gone(
    remove: On<Remove, CargoCanister>,
    q_canisters: Query<&CargoCanister>,
    mut log: ResMut<IntakeLog>,
) {
    if let Ok(canister) = q_canisters.get(remove.entity) {
        log.moved = contents_of(canister);
    }
}

/// True when `intake` is a section of the player ship.
fn on_the_player(
    intake: Entity,
    q_parents: &Query<&ChildOf>,
    q_player: &Query<(), With<PlayerSpaceshipMarker>>,
) -> bool {
    q_parents
        .get(intake)
        .is_ok_and(|parent| q_player.contains(parent.parent()))
}

fn note_a_jettison(
    ejected: On<CargoCanisterEjected>,
    q_parents: Query<&ChildOf>,
    q_player: Query<(), With<PlayerSpaceshipMarker>>,
    mut log: ResMut<IntakeLog>,
) {
    if on_the_player(ejected.entity, &q_parents, &q_player) {
        log.last_jettison = Some(std::mem::take(&mut log.moved));
    }
}

fn note_a_pickup(
    taken: On<CargoCanisterTaken>,
    q_parents: Query<&ChildOf>,
    q_player: Query<(), With<PlayerSpaceshipMarker>>,
    mut log: ResMut<IntakeLog>,
) {
    if on_the_player(taken.entity, &q_parents, &q_player) {
        log.last_pickup = Some(std::mem::take(&mut log.moved));
    }
}

/// Each live intake's face, pickup area, detection range and normal, each
/// drifting canister with its contents, and the player's intake traffic.
fn draw_the_intakes(
    mut gizmos: Gizmos<DiagnosticsGizmos>,
    mut panel: ResMut<DiagnosticsPanel>,
    log: Res<IntakeLog>,
    readiness: Option<Res<CargoPickupReadiness>>,
    q_player: Query<(Entity, &GlobalTransform), With<PlayerSpaceshipMarker>>,
    q_intakes: Query<
        (
            Entity,
            &GlobalTransform,
            &CargoIntakeSectionConfigHelper,
            &SectionCollider,
            &ChildOf,
        ),
        (With<CargoIntakeSectionMarker>, With<SectionMarker>),
    >,
    q_canisters: Query<(&GlobalTransform, &CargoCanister)>,
) {
    let player = q_player.single().ok();
    let ready = |intake: Entity| {
        readiness.as_deref().is_some_and(|readiness| {
            readiness
                .pairs
                .iter()
                .any(|pair| pair.intake == intake && pair.ready)
        })
    };
    let mut faces: Vec<&str> = Vec::new();
    for (intake, transform, config, collider, parent) in &q_intakes {
        let rotation = transform.rotation();
        let (face, normal) = cargo_intake_face(transform.translation(), rotation, *collider);
        let gap = config.capture_gap.to_engine();
        let mine = player.is_some_and(|(ship, _)| ship == parent.parent());
        let color = if ready(intake) {
            palette::PUSH
        } else if mine {
            palette::OK
        } else {
            palette::MUTED
        };
        gizmos.cube(
            Transform::from_translation(face + normal * gap * 0.5)
                .with_rotation(rotation)
                .with_scale(Vec3::new(
                    config.aperture_width.to_engine(),
                    config.aperture_height.to_engine(),
                    gap,
                )),
            color,
        );
        let range = config.detection_range.to_engine();
        diagnostics::ring(&mut gizmos, face, normal, range, color);
        diagnostics::vector(&mut gizmos, face, normal, range, color);
        if let Some((_, ship)) = player.filter(|_| mine) {
            let up = ship.rotation().inverse() * normal;
            let side = if up.y > 0.5 {
                "top"
            } else if up.y < -0.5 {
                "bottom"
            } else {
                "side"
            };
            faces.push(side);
            panel.label(face + normal * range, format!("intake ({side})"));
        }
    }
    let size = CARGO_CANISTER_SIZE.length() * 0.5;
    let mut drifting = 0;
    for (transform, canister) in &q_canisters {
        drifting += 1;
        let centre = transform.translation();
        diagnostics::sphere(&mut gizmos, centre, size, palette::TARGET);
        panel.label(centre, show_contents(Some(&contents_of(canister))));
    }
    let ready_now = player.map_or(0, |(ship, _)| {
        readiness.as_deref().map_or(0, |readiness| {
            readiness
                .pairs
                .iter()
                .filter(|pair| pair.ship == ship && pair.ready)
                .count()
        })
    });
    let intake = if faces.is_empty() {
        "no".to_string()
    } else {
        format!("yes ({})", faces.join(", "))
    };
    panel.line(format!(
        "intake: {intake}   canisters drifting: {drifting}   pickups ready: {ready_now}"
    ));
    panel.line(format!(
        "last jettison: {}   last pickup: {}",
        show_contents(log.last_jettison.as_ref()),
        show_contents(log.last_pickup.as_ref())
    ));
}

mod smoke {
    use std::{collections::BTreeMap, sync::Arc};

    use nova_protocol::nova_debug::harness::{shoot, AutopilotPlugin};

    use super::*;

    /// Seconds the world gets to start and stream on a software GPU.
    const SETUP_SECS: f32 = 600.0;
    /// Seconds a shot gets to land.
    const SHOT_SECS: f32 = 120.0;
    /// Frames the scene gets to settle before the shot.
    const SETTLE_FRAMES: u32 = 10;
    /// Seconds the intake gets to open its door and drop the canister.
    const JETTISON_SECS: f32 = 120.0;

    fn ready(world: &World) -> bool {
        world
            .get_resource::<Sandbox>()
            .is_some_and(|sandbox| sandbox.phase == SandboxPhaseType::Ready)
    }

    /// The player is the role's hull with the role's kit.
    fn assert_the_start(world: &mut World) {
        let id = world.resource::<Sandbox>().role.clone();
        let role = world
            .resource::<GameStartRoles>()
            .get_role(&StartRoleDesignId::from(id.as_str()))
            .cloned()
            .unwrap_or_else(|| panic!("start role sandbox: no role '{id}'"));
        let hull: Vec<String> = world
            .resource::<GameShipDesigns>()
            .get_design(&role.hull)
            .unwrap_or_else(|| panic!("start role sandbox: '{id}' names a missing hull"))
            .design
            .sections
            .iter()
            .map(|section| section.id.clone())
            .collect();
        let player = the_player(world).expect("start role sandbox: one player ship");
        let spawned: Vec<String> = world
            .get::<ShipDesignSections>(player)
            .expect("start role sandbox: the player carries its design")
            .0
            .iter()
            .map(|section| section.id.clone())
            .collect();
        assert_eq!(spawned, hull, "start role sandbox: '{id}' hull sections");
        let credits = world
            .get::<ShipCredits>(player)
            .expect("start role sandbox: the player carries a balance")
            .0;
        assert_eq!(credits, role.kit.credits, "start role sandbox: kit credits");
        let held: BTreeMap<String, u32> = world
            .get::<ShipInventory>(player)
            .expect("start role sandbox: the player carries a hold")
            .stacks()
            .map(|(item, count)| (item.to_string(), count))
            .collect();
        let kit: BTreeMap<String, u32> = role
            .kit
            .stock
            .stacks()
            .map(|(item, count)| (item.to_string(), count))
            .collect();
        assert_eq!(held, kit, "start role sandbox: kit stock");
        let mut q_intakes = world.query_filtered::<&ChildOf, With<CargoIntakeSectionMarker>>();
        assert!(
            q_intakes
                .iter(world)
                .any(|parent| parent.parent() == player),
            "start role sandbox: '{id}' hull has no intake"
        );
        let cell = world
            .get::<Position>(player)
            .and_then(|position| cell_of(world, position.0));
        info!(
            "start role sandbox: PASS '{id}' started as '{}' with {credits} cr and {held:?} in \
             cell {cell:?}",
            role.hull
        );
    }

    /// The kit's first item: the one the smoke jettisons.
    fn first_kit_item(world: &World) -> ItemDesignId {
        let id = world.resource::<Sandbox>().role.clone();
        world
            .resource::<GameStartRoles>()
            .get_role(&StartRoleDesignId::from(id.as_str()))
            .and_then(|role| role.kit.stock.stacks().next().map(|(item, _)| item.clone()))
            .unwrap_or_else(|| panic!("start role sandbox: '{id}' starts with an empty hold"))
    }

    /// Jettison one unit of the kit's first item through the player's first
    /// intake, as the Inventory pane's Jettison applies its plan.
    fn jettison_one_item(world: &mut World) {
        let player = the_player(world).expect("start role sandbox: one player ship");
        let mut q_intakes =
            world.query_filtered::<(Entity, &ChildOf), With<CargoIntakeSectionMarker>>();
        let intake = q_intakes
            .iter(world)
            .find(|(_, parent)| parent.parent() == player)
            .map(|(intake, _)| intake)
            .expect("start role sandbox: the player hull has an intake");
        let item = first_kit_item(world);
        let own = world
            .get::<ShipInventory>(player)
            .expect("start role sandbox: the player carries a hold")
            .clone();
        let plan = plan_item_jettison(
            world.resource::<GameItems>(),
            world.get::<DockedShip>(player).is_some(),
            true,
            None,
            &item,
            Some(1),
            &own,
        )
        .unwrap_or_else(|refusal| panic!("start role sandbox: jettison refused: {refusal:?}"));
        world
            .get_mut::<ShipInventory>(player)
            .expect("start role sandbox: the player carries a hold")
            .remove(&item, plan.count);
        world
            .entity_mut(intake)
            .insert(CargoIntakeEjectionQueue(plan.canisters.into()));
    }

    /// The panel logged the jettison: one unit of the kit's first item.
    fn jettison_logged(world: &World) -> bool {
        let item = first_kit_item(world).to_string();
        world
            .resource::<IntakeLog>()
            .last_jettison
            .as_ref()
            .is_some_and(|contents| *contents == [(item.clone(), 1)])
    }

    /// The smoke walk: the world streams in, the start is the role's, one
    /// item leaves the intake and the panel logs it, and a frame of it with
    /// the diagnostics up is shot.
    pub(super) fn script(shot: &str) -> AutopilotPlugin<GameStates> {
        let name = shot.to_string();
        AutopilotPlugin::<GameStates>::new()
            .step("start role sandbox: the window streams in")
            .until(Arc::new(ready))
            .diagnose(|world| format!("{:?}", world.get_resource::<Sandbox>()))
            .deadline(SETUP_SECS)
            .add()
            .step("start role sandbox: check the start")
            .on_enter(assert_the_start)
            .add()
            .step("start role sandbox: jettison one kit item")
            .on_enter(jettison_one_item)
            .until(Arc::new(jettison_logged))
            .diagnose(|world| format!("{:?}", world.get_resource::<IntakeLog>()))
            .deadline(JETTISON_SECS)
            .add()
            .step("start role sandbox: settle")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("start role sandbox: shoot the start")
            .on_enter(move |world: &mut World| shoot(world, &name))
            .until(shot_written(shot))
            .deadline(SHOT_SECS)
            .add()
    }
}
