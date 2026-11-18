//! consumables_sandbox: fly with repair kits aboard and a damaged hull, and
//! use a kit from the Inventory pane.
//!
//! The player's ship floats in a void. THE HOLD AND THE HULL DAMAGE ARE
//! DISCLOSED FIXTURE STATE: three repair kits, and the hold section set 60 HP
//! below its maximum.
//!
//! What to do:
//!
//! 1. press Tab, open Inventory and click the repair kit row: the inspector
//!    shows Use. It is enabled while a use would pass;
//! 2. click Use: one kit leaves the hold. Close the interface; the interface
//!    holds the clock, so the repair runs once it closes;
//! 3. read the status chip: `KIT n S` while the kit heals the hold section
//!    over 10 s;
//! 4. open the Inventory again while the kit runs, or once the hull is whole:
//!    Use is greyed, and a press plays the deny cue.
//!
//! The diagnostics layer (F3) is on at start: panel lines with the held kits,
//! the running effect and the damaged section's health, and a detail line per
//! other section.
//!
//! ```text
//! nix develop --command cargo run --example consumables_sandbox --features debug
//! ```
//!
//! No deadline ends the run unless `NOVA_AUTOPILOT=1` is set. Then a smoke
//! walk opens the Inventory, picks the kit row, clicks Use, closes the
//! interface, waits out the repair and opens the Inventory again to find Use
//! greyed. `NOVA_CAPTURE=1` shoots the dock at the start, the Use button, the
//! `KIT` status chip, the healed hull and the greyed Use button.

use bevy::prelude::*;
use diagnostics::{DiagnosticsPanel, DiagnosticsSystems};
use nova_protocol::{nova_debug::diagnostics, prelude::*};

/// The player scenario id.
const PLAYER_ID: &str = "player_spaceship";
/// The section the fixture damages.
const DAMAGED_SECTION: &str = "hold";

/// The base repair kit item.
const KIT: &str = "RepairKit";

/// The fixture hold: three repair kits.
const HOLD_KITS: u32 = 3;
/// The fixture damage on the hold section, under one kit's 80 HP.
const HULL_DAMAGE: f32 = 60.0;

/// The intro line.
const INTRO: &str = "Tab, Inventory, pick the repair kit, Use; close the interface to run it.";

fn main() -> bevy::app::AppExit {
    let mut app = AppBuilder::new()
        .with_game_plugins(consumables_plugin)
        .build();
    app.add_plugins(diagnostics::DiagnosticsHintPlugin::new(INTRO));
    app.add_systems(
        PostUpdate,
        draw_the_consumables
            .in_set(DiagnosticsSystems)
            .after(TransformSystems::Propagate)
            .run_if(in_state(GameStates::Playing)),
    );

    if std::env::var_os("NOVA_AUTOPILOT").is_some() {
        app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
            bevy::ecs::error::panic,
        ));
        app.add_systems(Startup, force_capture_resolution);
    }
    app.add_plugins(smoke::script());

    app.run()
}

fn consumables_plugin(app: &mut App) {
    app.init_resource::<Staging>();
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_the_void);
    app.add_systems(
        Update,
        stage_the_damage.run_if(in_state(GameStates::Playing)),
    );
}

fn load_the_void(mut commands: Commands, game_assets: Res<GameAssets>) {
    let player = ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: PLAYER_ID.to_string(),
            name: "Player".to_string(),
            position: Meters3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(player_ship()),
    };
    commands.trigger(LoadScenario(ScenarioConfig {
        description: "Consumables Sandbox".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            position: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![EventActionConfig::SpawnScenarioObject(player)],
                ThreePointRig::around("rig", Meters3::ZERO, 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "consumables_sandbox".to_string(),
            "Consumables Sandbox".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }));
}

/// The player's hull: a controller over a hull block (the damaged `hold`), a
/// port on its nose and a drive on its tail, with the fixture hold.
fn player_ship() -> SpaceshipConfig {
    let section = |id: &str, prototype: &str, position: Vec3| SpaceshipSectionConfig {
        id: id.to_string(),
        position,
        rotation: Quat::IDENTITY,
        source: SectionSource::prototype(prototype),
    };
    SpaceshipConfig {
        controller: SpaceshipController::Player(PlayerControllerConfig::default()),
        design: ShipDesignSource::Inline(ShipDesign {
            sections: vec![
                section("controller", BASIC_CONTROLLER_SECTION_ID, Vec3::ZERO),
                section(DAMAGED_SECTION, REINFORCED_HULL_SECTION_ID, Vec3::NEG_Y),
                section("port", DOCKING_PORT_SECTION_ID, Vec3::NEG_Z),
                section("drive", BASIC_THRUSTER_SECTION_ID, Vec3::Z),
            ],
            ..default()
        }),
        inventory: ShipInventoryStock::new([(KIT.into(), HOLD_KITS)]),
        ..default()
    }
}

/// Whether the fixture damage was put on the hull.
#[derive(Resource, Default)]
struct Staging {
    damaged: bool,
}

/// The player root, once it has spawned.
fn player_root(world: &World) -> Option<Entity> {
    let mut players = world.try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()?;
    players.iter(world).next()
}

/// The player's section with scenario id `id`.
fn player_section(world: &World, id: &str) -> Option<Entity> {
    let children = world.get::<Children>(player_root(world)?)?;
    children.iter().find(|child| {
        world
            .get::<EntityId>(*child)
            .is_some_and(|each| each.0 == id)
    })
}

/// Put the fixture damage on the hull once its sections are up: the hold
/// section [`HULL_DAMAGE`] below its maximum.
fn stage_the_damage(world: &mut World) {
    if world.resource::<Staging>().damaged {
        return;
    }
    let Some(section) = player_section(world, DAMAGED_SECTION) else {
        return;
    };
    let Some(mut health) = world.get_mut::<Health>(section) else {
        return;
    };
    health.current = health.max - HULL_DAMAGE;
    world.resource_mut::<Staging>().damaged = true;
    info!("consumables_sandbox: the hold section is {HULL_DAMAGE} HP down");
}

/// The panel: the held kits, the running effect and each section.
fn draw_the_consumables(
    mut panel: ResMut<DiagnosticsPanel>,
    consumables: Res<GameConsumables>,
    q_player: Query<
        (&ShipInventory, Option<&ShipConsumableEffect>, &Children),
        With<PlayerSpaceshipMarker>,
    >,
    q_section: Query<(&EntityId, &Health), With<SectionMarker>>,
) {
    let Ok((hold, effect, children)) = q_player.single() else {
        return;
    };
    panel.line(format!("Held: {} kits", hold.count(&KIT.into())));
    panel.line(match effect {
        Some(effect) => format!(
            "Running: {} {:.1} s",
            effect.item,
            effect.remaining_secs(&consumables)
        ),
        None => "Running: none".to_string(),
    });
    for (id, health) in children
        .iter()
        .filter_map(|child| q_section.get(child).ok())
    {
        let line = format!(
            "Section {}: {:.0} / {:.0} HP",
            id.0, health.current, health.max
        );
        // The damaged section on the panel itself, the rest under detail.
        if id.0 == DAMAGED_SECTION {
            panel.line(line);
        } else {
            panel.detail(line);
        }
    }
    panel.hint("TAB", "Inventory, repair kit, Use");
}

mod smoke {
    use std::sync::Arc;

    use nova_protocol::{
        nova_debug::harness::{AutopilotPlugin, Predicate, SHOT_DEADLINE_SECS},
        nova_interface::{inventory::INVENTORY_USE_BUTTON, pane::InterfacePaneType},
    };

    use super::*;

    /// Seconds the walk gives the void to load.
    const LOAD_SECS: f32 = 120.0;
    /// Seconds any later beat may stall before the walk fails it.
    const BEAT_SECS: f32 = 60.0;
    /// Real seconds the 10 s repair may take under a software renderer.
    const REPAIR_SECS: f32 = 300.0;
    /// Frames the HUD and the panes get to settle before a shot.
    const SETTLE_FRAMES: u32 = 10;

    /// The frames the capture path writes.
    const FLIGHT_SHOT: &str = "consumables-sandbox-flight.png";
    const USE_SHOT: &str = "consumables-sandbox-use.png";
    const RUNNING_SHOT: &str = "consumables-sandbox-running.png";
    const HEALED_SHOT: &str = "consumables-sandbox-healed.png";
    const REFUSED_SHOT: &str = "consumables-sandbox-refused.png";

    /// The Inventory row of the player's repair kits.
    const KIT_ROW: &str = "InventoryRowOwnRepairKit";

    fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(check)
    }

    fn player(world: &World) -> Entity {
        player_root(world).expect("consumables_sandbox: a player")
    }

    fn kits(world: &World) -> u32 {
        world
            .get::<ShipInventory>(player(world))
            .expect("consumables_sandbox: the player carries a hold")
            .count(&ItemDesignId::from(KIT))
    }

    fn running(world: &World) -> bool {
        world.get::<ShipConsumableEffect>(player(world)).is_some()
    }

    fn hold_health(world: &World) -> Health {
        let section = player_section(world, DAMAGED_SECTION)
            .expect("consumables_sandbox: the hold section is up");
        world
            .get::<Health>(section)
            .expect("consumables_sandbox: a section carries health")
            .clone()
    }

    fn pause_is(state: PauseStates) -> Arc<Predicate> {
        when(move |world| {
            world
                .get_resource::<State<PauseStates>>()
                .is_some_and(|pause| *pause.get() == state)
        })
    }

    /// The Use button is shown, and whether it is enabled.
    fn use_button(world: &World) -> Option<bool> {
        let mut query = world.try_query::<(&Name, &Node, Has<bevy::ui::InteractionDisabled>)>()?;
        query
            .iter(world)
            .find(|(name, node, _)| {
                name.as_str() == INVENTORY_USE_BUTTON && node.display != Display::None
            })
            .map(|(_, _, disabled)| !disabled)
    }

    fn open_the_inventory(world: &mut World) {
        *world.resource_mut::<InterfacePaneType>() = InterfacePaneType::Inventory;
        world
            .resource_mut::<NextState<PauseStates>>()
            .set(PauseStates::Interface);
    }

    fn close_the_interface(world: &mut World) {
        world
            .resource_mut::<NextState<PauseStates>>()
            .set(PauseStates::Unpaused);
    }

    fn assert_the_staging(world: &mut World) {
        assert_eq!(kits(world), HOLD_KITS, "consumables_sandbox: kits aboard");
        let health = hold_health(world);
        assert!(
            (health.max - health.current - HULL_DAMAGE).abs() < 1e-3,
            "consumables_sandbox: the hold section is {HULL_DAMAGE} HP down: {health:?}"
        );
        assert!(
            !running(world),
            "consumables_sandbox: nothing runs at the start"
        );
        info!("consumables_sandbox: PASS staged with three kits and a damaged hold");
    }

    fn assert_the_use(world: &mut World) {
        assert!(
            running(world),
            "consumables_sandbox: the Use button started a repair"
        );
        assert_eq!(
            use_button(world),
            Some(false),
            "consumables_sandbox: Use is greyed while the kit runs"
        );
        info!("consumables_sandbox: PASS the Use button used one kit");
    }

    fn assert_the_repair(world: &mut World) {
        let health = hold_health(world);
        assert!(
            (health.max - health.current).abs() < 1e-3,
            "consumables_sandbox: the kit healed the hold section: {health:?}"
        );
        assert_eq!(
            kits(world),
            HOLD_KITS - 1,
            "consumables_sandbox: one kit spent"
        );
        info!(
            "consumables_sandbox: PASS the kit healed the hold section to {}",
            health.current
        );
    }

    /// The smoke walk. Inert unless `NOVA_AUTOPILOT` is set.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        AutopilotPlugin::<GameStates>::new()
            .step("consumables sandbox: load the void")
            .enter(GameStates::Loading)
            .until(when(|world| {
                world
                    .get_resource::<Staging>()
                    .is_some_and(|staging| staging.damaged)
            }))
            .deadline(LOAD_SECS)
            .add()
            .step("consumables sandbox: settle the HUD")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("consumables sandbox: check the staging")
            .on_enter(assert_the_staging)
            .until(frames(1))
            .add()
            .step("consumables sandbox: shoot the dock")
            .on_enter(|world: &mut World| shoot(world, FLIGHT_SHOT))
            .until(shot_written(FLIGHT_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
            .step("consumables sandbox: open the Inventory")
            .on_enter(open_the_inventory)
            .until(ui_node_present(KIT_ROW))
            .deadline(BEAT_SECS)
            .add()
            .click_named(
                "consumables sandbox: pick the repair kit row",
                KIT_ROW,
                when(|world| use_button(world) == Some(true)),
                BEAT_SECS,
            )
            .step("consumables sandbox: settle the inspector")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("consumables sandbox: shoot the Use button")
            .on_enter(|world: &mut World| shoot(world, USE_SHOT))
            .until(shot_written(USE_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
            .click_named(
                "consumables sandbox: use a kit",
                INVENTORY_USE_BUTTON,
                when(|world| kits(world) == HOLD_KITS - 1 && use_button(world) == Some(false)),
                BEAT_SECS,
            )
            .step("consumables sandbox: check the Use")
            .on_enter(assert_the_use)
            .until(frames(1))
            .add()
            .step("consumables sandbox: close the interface")
            .on_enter(close_the_interface)
            .until(pause_is(PauseStates::Unpaused))
            .deadline(BEAT_SECS)
            .add()
            .step("consumables sandbox: settle the status chip")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("consumables sandbox: shoot the KIT status chip")
            .on_enter(|world: &mut World| {
                assert!(
                    running(world),
                    "consumables_sandbox: the kit runs in flight"
                );
                shoot(world, RUNNING_SHOT);
            })
            .until(shot_written(RUNNING_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
            .step("consumables sandbox: run the repair out")
            .until(when(|world| !running(world)))
            .deadline(REPAIR_SECS)
            .add()
            .step("consumables sandbox: check the repair")
            .on_enter(assert_the_repair)
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("consumables sandbox: shoot the healed hull")
            .on_enter(|world: &mut World| shoot(world, HEALED_SHOT))
            .until(shot_written(HEALED_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
            // The inspector keeps the kit row picked while the interface is
            // closed, so Use shows again at once.
            .step("consumables sandbox: open the Inventory on a whole hull")
            .on_enter(open_the_inventory)
            .until(when(|world| use_button(world) == Some(false)))
            .deadline(BEAT_SECS)
            .add()
            .step("consumables sandbox: settle the greyed Use")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("consumables sandbox: shoot the greyed Use")
            .on_enter(|world: &mut World| {
                assert_eq!(
                    kits(world),
                    HOLD_KITS - 1,
                    "consumables_sandbox: no kit spent"
                );
                info!("consumables_sandbox: PASS Use is greyed with nothing to repair");
                shoot(world, REFUSED_SHOT);
            })
            .until(shot_written(REFUSED_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
    }
}
