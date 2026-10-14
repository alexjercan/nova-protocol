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
//!    unfocused.
//!
//! ```text
//! nix develop --command cargo run --example start_role_sandbox --features debug -- --role pirate
//! ```
//!
//! The diagnostics panel (`F3`) shows the role, the hull, the kit, the live
//! balance and hold, the hull's value at ask and its trade-in by health, the
//! cell and the distance from the origin, and the generated hulls in the
//! window. The gizmos ring each generated hull at its containment radius,
//! labelled with its span.
//!
//! Under `NOVA_AUTOPILOT=1` (debug only) a smoke walk waits for the window,
//! checks the player is the role's hull with the role's kit, and shoots
//! `start-role-<role>.png` with `NOVA_CAPTURE=1`. Unset, no walk and no
//! deadline run.

#[path = "shared/diagnostics.rs"]
mod diagnostics;
#[path = "shared/open_world_stage.rs"]
mod open_world_stage;

use avian3d::prelude::Position;
use bevy::prelude::*;
use clap::Parser;
use diagnostics::{palette, DiagnosticsGizmos, DiagnosticsPanel, DiagnosticsSystems};
use nova_protocol::prelude::*;
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
    #[cfg(feature = "debug")]
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
    app.add_plugins(diagnostics::PlayableDiagnosticsPlugin::new(
        "fly the role's hull   [TAB] interface, [M] next tab, [C] chart on Map   [:] shell: \
         cheats enable, credits give <ship-id> 1000",
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

    #[cfg(feature = "debug")]
    {
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
    }

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

#[cfg(feature = "debug")]
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
        let cell = world
            .get::<Position>(player)
            .and_then(|position| cell_of(world, position.0));
        info!(
            "start role sandbox: PASS '{id}' started as '{}' with {credits} cr and {held:?} in \
             cell {cell:?}",
            role.hull
        );
    }

    /// The smoke walk: the world streams in, the start is the role's, and a
    /// frame of it with the diagnostics up is shot.
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
