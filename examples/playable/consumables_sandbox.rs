//! consumables_sandbox: fly with repair kits, boost cells and decoy flares
//! aboard, a damaged hull and a torpedo stand that fires at you, and use them
//! on keys 1/2/3 or from the Inventory pane.
//!
//! The player's ship floats in a void with a controller-less torpedo stand
//! 2000 m off its right flank, inside the base flare band of 400 to 3000 m. THE
//! HOLD AND THE HULL DAMAGE ARE DISCLOSED FIXTURE STATE: three of each
//! consumable, and the hold section set 60 HP below its maximum. The stand
//! fires one torpedo at the player every 20 s while none flies.
//!
//! What to do:
//!
//! 1. read the keybind dock: `KIT 3`, `BOOST 3` and `FLARE 3` with their
//!    keys. A chip is lit while a use would pass; `FLARE` stays dark until a
//!    torpedo homes on you inside its band;
//! 2. press 1: the repair kit heals the hold section over 10 s. The status
//!    chip shows `KIT n S`; the `KIT` chip inverts while it runs. A second 1
//!    is refused with the deny cue and keeps the kit;
//! 3. press 2 and hold the main drive: the boost scales thrust by 1.5 for
//!    8 s while you fly by hand;
//! 4. when a torpedo flies, press 3: it drops its lock and fuzes 800 m aside;
//!    the HUD posts `FLARE: 1 DECOYED`;
//! 5. press Tab, open Inventory and click a consumable row: the inspector's
//!    Use button uses one; it is disabled while a use would be refused.
//!
//! The diagnostics layer (F3) is on at start: panel lines with the held
//! consumables, the running effects, the torpedoes and the damaged section's
//! health, and a detail line per other section.
//!
//! ```text
//! nix develop --command cargo run --example consumables_sandbox --features debug
//! ```
//!
//! No deadline ends the run unless `NOVA_AUTOPILOT=1` is set. Then the stand
//! holds fire until the walk asks, and a smoke walk uses a kit on key 1 and a
//! boost cell on key 2 under the main drive, brakes, waits out the repair,
//! fires one torpedo and decoys it on key 3, then damages the hold again and
//! uses a kit from the Inventory pane's Use button. `NOVA_CAPTURE=1` shoots
//! the dock chips at the start, the running effects under the drive, the
//! healed hull, the flare notice and the Use button.

use bevy::prelude::*;
use diagnostics::{DiagnosticsPanel, DiagnosticsSystems};
use nova_protocol::{nova_debug::diagnostics, prelude::*};

/// The player and stand scenario ids.
const PLAYER_ID: &str = "player_spaceship";
const STAND_ID: &str = "torpedo_stand";
/// The section the fixture damages.
const DAMAGED_SECTION: &str = "hold";

/// The base consumable items.
const KIT: &str = "RepairKit";
const CELL: &str = "BoostCell";
const FLARE: &str = "DecoyFlare";

/// The fixture hold: three of each consumable.
const HOLD_EACH: u32 = 3;
/// The fixture damage on the hold section, under one kit's 80 HP.
const HULL_DAMAGE: f32 = 60.0;
/// The stand's range off the player's flank, inside the base flare band.
/// A flank stand stays in the band while the player flies along its nose.
const STAND_RANGE: Meters = Meters(2000.0);
/// Seconds between the stand's torpedoes while none flies.
const STAND_INTERVAL_SECS: f32 = 20.0;

/// The intro line.
const INTRO: &str =
    "1 repair kit, 2 boost cell, 3 decoy flare when a torpedo flies; Tab, Inventory, Use.";

fn main() -> bevy::app::AppExit {
    // NovaMenuPlugin explicitly: the interface's clock hold lives in
    // nova_menu, and `with_game_plugins` turns the menu plugin off. The hold
    // keeps a repair started from the Use button still while Tab is open, as
    // in the game.
    let mut app = AppBuilder::new()
        .with_game_plugins((consumables_plugin, NovaMenuPlugin))
        .build();
    app.add_plugins(diagnostics::DiagnosticsHintPlugin::new(INTRO));
    app.add_systems(
        PostUpdate,
        draw_the_consumables
            .in_set(DiagnosticsSystems)
            .after(TransformSystems::Propagate)
            .run_if(in_state(GameStates::Playing)),
    );

    let autopilot = std::env::var_os("NOVA_AUTOPILOT").is_some();
    // The walk fires the stand when it is ready for the torpedo; a hand run
    // has the stand fire on its own clock.
    app.insert_resource(StandFire {
        armed: !autopilot,
        next_in_secs: STAND_INTERVAL_SECS,
    });
    if autopilot {
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
        (stage_the_damage, fire_the_stand).run_if(in_state(GameStates::Playing)),
    );
}

fn load_the_void(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    sections: Res<GameSections>,
) {
    let placed =
        |id: &str, name: &str, position: Meters3, ship: SpaceshipConfig| ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: id.to_string(),
                name: name.to_string(),
                position,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Spaceship(ship),
        };
    let bay = sections
        .get_section("torpedo_section")
        .expect("consumables_sandbox: the base torpedo bay")
        .clone();
    let stand = SpaceshipConfig {
        controller: SpaceshipController::None,
        design: ShipDesignSource::Inline(ShipDesign {
            sections: vec![SpaceshipSectionConfig {
                id: "bay".to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                source: SectionSource::Inline(bay),
            }],
            ..default()
        }),
        ..default()
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
                vec![
                    EventActionConfig::SpawnScenarioObject(placed(
                        PLAYER_ID,
                        "Player",
                        Meters3::ZERO,
                        player_ship(),
                    )),
                    EventActionConfig::SpawnScenarioObject(placed(
                        STAND_ID,
                        "Torpedo Stand",
                        Meters3(Vec3::new(STAND_RANGE.0, 0.0, 0.0)),
                        stand,
                    )),
                ],
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

/// The player's hull: a controller between two hull blocks (the damaged
/// `hold` below), a port on its nose and a drive on its tail, with the
/// fixture hold.
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
                // A second hull block: one hull's 100 kg hold is short of
                // three of each consumable.
                section("spine", REINFORCED_HULL_SECTION_ID, Vec3::Y),
                section("port", DOCKING_PORT_SECTION_ID, Vec3::NEG_Z),
                section("drive", BASIC_THRUSTER_SECTION_ID, Vec3::Z),
            ],
            ..default()
        }),
        inventory: ShipInventoryStock::new([
            (KIT.into(), HOLD_EACH),
            (CELL.into(), HOLD_EACH),
            (FLARE.into(), HOLD_EACH),
        ]),
        ..default()
    }
}

/// Whether the fixture damage was put on the hull.
#[derive(Resource, Default)]
struct Staging {
    damaged: bool,
}

/// The stand's fire clock. Disarmed, it holds fire.
#[derive(Resource)]
struct StandFire {
    armed: bool,
    next_in_secs: f32,
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

/// Set the hold section [`HULL_DAMAGE`] below its maximum.
fn damage_the_hold(world: &mut World) -> bool {
    let Some(section) = player_section(world, DAMAGED_SECTION) else {
        return false;
    };
    let Some(mut health) = world.get_mut::<Health>(section) else {
        return false;
    };
    health.current = health.max - HULL_DAMAGE;
    true
}

/// Put the fixture damage on the hull once its sections are up.
fn stage_the_damage(world: &mut World) {
    if world.resource::<Staging>().damaged {
        return;
    }
    if damage_the_hold(world) {
        world.resource_mut::<Staging>().damaged = true;
        info!("consumables_sandbox: the hold section is {HULL_DAMAGE} HP down");
    }
}

/// Fire one torpedo from the stand at the player whenever the clock runs
/// out and none flies.
fn fire_the_stand(
    time: Res<Time>,
    mut fire: ResMut<StandFire>,
    mut commands: Commands,
    q_player: Query<Entity, With<PlayerSpaceshipMarker>>,
    q_stand: Query<(Entity, &EntityId), With<SpaceshipRootMarker>>,
    q_bay: Query<(Entity, &ChildOf), (With<TorpedoSectionMarker>, Without<ScriptedTorpedoOrder>)>,
    q_torpedo: Query<(), With<TorpedoProjectileMarker>>,
) {
    if !fire.armed || !q_torpedo.is_empty() {
        return;
    }
    fire.next_in_secs -= time.delta_secs();
    if fire.next_in_secs > 0.0 {
        return;
    }
    let (Ok(target), Some(stand)) = (
        q_player.single(),
        q_stand
            .iter()
            .find(|(_, id)| id.0 == STAND_ID)
            .map(|(stand, _)| stand),
    ) else {
        return;
    };
    if let Some((bay, _)) = q_bay.iter().find(|(_, parent)| parent.parent() == stand) {
        commands.entity(bay).insert(ScriptedTorpedoOrder { target });
        fire.next_in_secs = STAND_INTERVAL_SECS;
        info!("consumables_sandbox: the stand fires a torpedo");
    }
}

/// The panel: the held consumables, the running effects and each section.
fn draw_the_consumables(
    mut panel: ResMut<DiagnosticsPanel>,
    consumables: Res<GameConsumables>,
    fire: Res<StandFire>,
    q_player: Query<
        (&ShipInventory, Option<&ShipConsumableEffects>, &Children),
        With<PlayerSpaceshipMarker>,
    >,
    q_section: Query<(&EntityId, &Health), With<SectionMarker>>,
    q_torpedo: Query<Has<TorpedoTargetEntity>, With<TorpedoProjectileMarker>>,
) {
    let Ok((hold, effects, children)) = q_player.single() else {
        return;
    };
    panel.line(format!(
        "Held: {} kits, {} cells, {} flares",
        hold.count(&KIT.into()),
        hold.count(&CELL.into()),
        hold.count(&FLARE.into())
    ));
    let running: Vec<String> = ConsumableSlotType::ALL
        .into_iter()
        .filter_map(|slot| {
            let left = effects?.remaining_secs(&consumables, slot)?;
            Some(format!("{} {left:.1} s", slot.word()))
        })
        .collect();
    let factor = effects.map_or(1.0, |effects| effects.thrust_factor(&consumables));
    panel.line(format!(
        "Running: {}; thrust x{factor:.1}",
        if running.is_empty() {
            "none".to_string()
        } else {
            running.join(", ")
        }
    ));
    let torpedoes: Vec<&str> = q_torpedo
        .iter()
        .map(|homing| if homing { "homing" } else { "decoyed" })
        .collect();
    panel.line(format!(
        "Torpedoes: {}",
        if torpedoes.is_empty() {
            "none".to_string()
        } else {
            torpedoes.join(", ")
        }
    ));
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
    panel.hint("1 2 3", "kit, boost, flare");
    if fire.armed {
        panel.hint(
            "STAND",
            format!("fires in {:.0} s", fire.next_in_secs.max(0.0)),
        );
    }
}

mod smoke {
    use std::sync::Arc;

    use nova_protocol::{
        nova_debug::harness::{
            press_action, release_action, AutopilotPlugin, Predicate, SHOT_DEADLINE_SECS,
        },
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
    const RUNNING_SHOT: &str = "consumables-sandbox-running.png";
    const HEALED_SHOT: &str = "consumables-sandbox-healed.png";
    const FLARE_SHOT: &str = "consumables-sandbox-flare.png";
    const USE_SHOT: &str = "consumables-sandbox-use.png";

    /// The Inventory row of the player's repair kits.
    const KIT_ROW: &str = "InventoryRowOwnRepairKit";

    fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(check)
    }

    fn player(world: &World) -> Entity {
        player_root(world).expect("consumables_sandbox: a player")
    }

    fn held(world: &World, id: &str) -> u32 {
        world
            .get::<ShipInventory>(player(world))
            .expect("consumables_sandbox: the player carries a hold")
            .count(&ItemDesignId::from(id))
    }

    fn running(world: &World, slot: ConsumableSlotType) -> bool {
        world
            .get::<ShipConsumableEffects>(player(world))
            .and_then(|effects| effects.remaining_secs(world.resource::<GameConsumables>(), slot))
            .is_some()
    }

    /// The player's speed, m/s.
    fn speed(world: &World) -> f32 {
        world
            .get::<avian3d::prelude::LinearVelocity>(player(world))
            .map_or(f32::INFINITY, |velocity| {
                MetersPerSecond3::from_engine(velocity.0).0.length()
            })
    }

    fn hold_health(world: &World) -> Health {
        let section = player_section(world, DAMAGED_SECTION)
            .expect("consumables_sandbox: the hold section is up");
        world
            .get::<Health>(section)
            .expect("consumables_sandbox: a section carries health")
            .clone()
    }

    /// The torpedoes homing on the player, and their distance in meters.
    fn homing(world: &World) -> Vec<Meters> {
        let player = player(world);
        let Some(at) = world.get::<Transform>(player).map(|pose| pose.translation) else {
            return Vec::new();
        };
        world
            .try_query_filtered::<(&Transform, &TorpedoTargetEntity), With<TorpedoProjectileMarker>>()
            .map(|mut torpedoes| {
                torpedoes
                    .iter(world)
                    .filter(|(_, target)| ***target == player)
                    .map(|(pose, _)| Meters::from_engine(pose.translation.distance(at)))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn decoyed(world: &World) -> usize {
        world
            .try_query_filtered::<(), (
                With<TorpedoProjectileMarker>,
                With<TorpedoTargetPosition>,
                Without<TorpedoTargetEntity>,
            )>()
            .map(|mut query| query.iter(world).count())
            .unwrap_or_default()
    }

    /// The dock chip of `verb`'s state, while it is on screen.
    fn chip(world: &World, verb: &str) -> Option<DockChipState> {
        let index = DOCK_VERBS.iter().position(|each| *each == verb)?;
        let mut chips = world.try_query::<(&DockChip, &DockChipState, &Visibility)>()?;
        chips
            .iter(world)
            .find(|(chip, _, visibility)| chip.0 == index && **visibility != Visibility::Hidden)
            .map(|(_, state, _)| *state)
    }

    /// The KIT, BOOST and FLARE chips read `states`.
    fn assert_chips(
        states: [DockChipState; 3],
        beat: &'static str,
    ) -> impl Fn(&mut World) + Send + Sync + 'static {
        move |world: &mut World| {
            let read = ["KIT", "BOOST", "FLARE"].map(|verb| chip(world, verb));
            assert_eq!(
                read,
                states.map(Some),
                "consumables_sandbox: the KIT, BOOST and FLARE chips {beat}"
            );
            info!("consumables_sandbox: PASS the chips {beat}: {read:?}");
        }
    }

    /// The Use button is shown and enabled.
    fn use_enabled(world: &World) -> bool {
        world
            .try_query::<(&Name, &Node, Has<bevy::ui::InteractionDisabled>)>()
            .is_some_and(|mut query| {
                query.iter(world).any(|(name, node, disabled)| {
                    name.as_str() == INVENTORY_USE_BUTTON
                        && node.display != Display::None
                        && !disabled
                })
            })
    }

    fn assert_the_staging(world: &mut World) {
        for item in [KIT, CELL, FLARE] {
            assert_eq!(
                held(world, item),
                HOLD_EACH,
                "consumables_sandbox: {item} aboard"
            );
        }
        let health = hold_health(world);
        assert!(
            (health.max - health.current - HULL_DAMAGE).abs() < 1e-3,
            "consumables_sandbox: the hold section is {HULL_DAMAGE} HP down: {health:?}"
        );
        info!("consumables_sandbox: PASS staged with three of each and a damaged hold");
    }

    fn assert_the_repair(world: &mut World) {
        let health = hold_health(world);
        assert!(
            (health.max - health.current).abs() < 1e-3,
            "consumables_sandbox: the kit healed the hold section: {health:?}"
        );
        assert_eq!(
            held(world, KIT),
            HOLD_EACH - 1,
            "consumables_sandbox: one kit spent"
        );
        info!(
            "consumables_sandbox: PASS key 1 healed the hold section to {}",
            health.current
        );
    }

    fn arm_the_stand(world: &mut World) {
        let mut fire = world.resource_mut::<StandFire>();
        fire.armed = true;
        fire.next_in_secs = 0.0;
    }

    fn disarm_the_stand(world: &mut World) {
        world.resource_mut::<StandFire>().armed = false;
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
            .step("consumables sandbox: check the chips at the start")
            .on_enter(assert_chips(
                [
                    DockChipState::Available,
                    DockChipState::Available,
                    DockChipState::Dim,
                ],
                "at the start",
            ))
            .until(frames(1))
            .add()
            .step("consumables sandbox: shoot the dock chips")
            .on_enter(|world: &mut World| shoot(world, FLIGHT_SHOT))
            .until(shot_written(FLIGHT_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
            .step("consumables sandbox: press 1 for a repair kit")
            .on_enter(press_action(ConsumableSlotType::Repair.action()))
            .until(when(|world| running(world, ConsumableSlotType::Repair)))
            .deadline(BEAT_SECS)
            .add()
            .step("consumables sandbox: let 1 up")
            .on_enter(release_action(ConsumableSlotType::Repair.action()))
            .until(frames(2))
            .add()
            .step("consumables sandbox: press 2 for a boost cell")
            .on_enter(press_action(ConsumableSlotType::ThrustBoost.action()))
            .until(when(|world| {
                running(world, ConsumableSlotType::ThrustBoost)
            }))
            .deadline(BEAT_SECS)
            .add()
            .step("consumables sandbox: let 2 up, hold the main drive")
            .on_enter(|world: &mut World| {
                release_action(ConsumableSlotType::ThrustBoost.action())(world);
                press_action("main_drive")(world);
            })
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("consumables sandbox: check the chips while both run")
            .on_enter(assert_chips(
                [DockChipState::Hot, DockChipState::Hot, DockChipState::Dim],
                "while both run",
            ))
            .until(frames(1))
            .add()
            .step("consumables sandbox: shoot the running effects under the drive")
            .on_enter(|world: &mut World| shoot(world, RUNNING_SHOT))
            .until(shot_written(RUNNING_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
            .step("consumables sandbox: let the drive go, brake")
            .on_enter(|world: &mut World| {
                release_action("main_drive")(world);
                press_action("autopilot_stop")(world);
            })
            .until(when(|world| speed(world) < 5.0))
            .deadline(BEAT_SECS)
            .add()
            // The boost starts a few frames after the repair, and a lavapipe
            // frame can be long, so the boost may outlast the repair.
            .step("consumables sandbox: run the repair and the boost out")
            .on_enter(release_action("autopilot_stop"))
            .until(when(|world| {
                !running(world, ConsumableSlotType::Repair)
                    && !running(world, ConsumableSlotType::ThrustBoost)
            }))
            .deadline(REPAIR_SECS)
            .add()
            .step("consumables sandbox: check the repair")
            .on_enter(assert_the_repair)
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("consumables sandbox: check the chips on a healed hull")
            .on_enter(assert_chips(
                [
                    DockChipState::Dim,
                    DockChipState::Available,
                    DockChipState::Dim,
                ],
                "on a healed hull",
            ))
            .until(frames(1))
            .add()
            .step("consumables sandbox: shoot the healed hull")
            .on_enter(|world: &mut World| shoot(world, HEALED_SHOT))
            .until(shot_written(HEALED_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
            .step("consumables sandbox: the stand fires one torpedo")
            .on_enter(arm_the_stand)
            .until(when(|world| {
                homing(world)
                    .iter()
                    .any(|distance| distance.0 > 400.0 && distance.0 < 3000.0)
            }))
            .deadline(BEAT_SECS)
            .add()
            .step("consumables sandbox: press 3 for a decoy flare")
            .on_enter(|world: &mut World| {
                disarm_the_stand(world);
                press_action(ConsumableSlotType::Flare.action())(world);
            })
            .until(when(|world| {
                decoyed(world) == 1 && homing(world).is_empty() && held(world, FLARE) == 2
            }))
            .deadline(BEAT_SECS)
            .add()
            .step("consumables sandbox: let 3 up")
            .on_enter(release_action(ConsumableSlotType::Flare.action()))
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("consumables sandbox: shoot the flare notice")
            .on_enter(|world: &mut World| {
                info!("consumables_sandbox: PASS key 3 decoyed the torpedo");
                shoot(world, FLARE_SHOT);
            })
            .until(shot_written(FLARE_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
            .step("consumables sandbox: damage the hold again, open the Inventory")
            .on_enter(|world: &mut World| {
                assert!(damage_the_hold(world));
                *world.resource_mut::<InterfacePaneType>() = InterfacePaneType::Inventory;
                world
                    .resource_mut::<NextState<PauseStates>>()
                    .set(PauseStates::Interface);
            })
            .until(ui_node_present(KIT_ROW))
            .deadline(BEAT_SECS)
            .add()
            .click_named(
                "consumables sandbox: pick the repair kit row",
                KIT_ROW,
                when(use_enabled),
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
                when(|world| held(world, KIT) == HOLD_EACH - 2),
                BEAT_SECS,
            )
            .step("consumables sandbox: check the Use")
            .on_enter(|world: &mut World| {
                assert!(
                    running(world, ConsumableSlotType::Repair),
                    "consumables_sandbox: the Use button started a repair"
                );
                info!("consumables_sandbox: PASS the Use button used one kit");
            })
            .until(frames(1))
            .add()
    }
}
