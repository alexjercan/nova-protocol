//! ammo_warning_sandbox: fire a PDC until its hold reserve runs low, then dry,
//! and watch the low-ammo warning.
//!
//! The player flies a small gunboat in a void: a bridge, three hull plates,
//! a drive and one Kinetic PDC on the bow plate's roof. THE HOLD IS
//! DISCLOSED FIXTURE STATE:
//!
//! | What | Fixture |
//! | - | - |
//! | PDC magazine | full, 500 rounds |
//! | hold | 600 PDC rounds: one magazine and 100 over the LOW line |
//!
//! The PDC reloads 200 rounds per 3 s off the trigger. Held fire empties the
//! magazine, and the reloads take the hold to 400 (LOW: under one magazine),
//! 200, then 0 (DRY).
//!
//! What to do:
//!
//! 1. raise the weapons (hold RMB) and hold fire (LMB);
//! 2. at 400 rounds in the hold the notice posts `PDC ROUND LOW: 400`, the
//!    ship's ammo gauge sounds once, and the PDC's ring flashes three times
//!    and goes back to normal;
//! 3. at 0 the notice posts `PDC ROUND DRY`; the ring adds nothing and runs
//!    down to a dark track that never refills;
//! 4. restock: `;` opens the shell, `cheats enable`, then
//!    `item give player_spaceship PdcRound 600`. The next drain warns LOW
//!    again, and the ring flashes again.
//!
//! The diagnostics layer (F3) is on at start: the magazine, the hold reserve
//! against the LOW line, and each crossing so far.
//!
//! ```text
//! nix develop --command cargo run --example ammo_warning_sandbox --features debug
//! ```
//!
//! No deadline ends the run unless `NOVA_AUTOPILOT=1` is set. Then a smoke
//! walk raises the weapons, holds the trigger through the reloads, and
//! checks one LOW crossing at 400 with its notice line and one ring flash
//! that ends, then one DRY at 0 with its notice line and no flash; then it
//! puts 600 rounds back in the hold and checks that the next drain warns LOW
//! and flashes again. `NOVA_CAPTURE=1` shoots the ring before the crossing,
//! at a flash peak, after the flash, and empty on a dry hold.

#[path = "shared/open_world_stage.rs"]
mod open_world_stage;
#[path = "shared/station_void.rs"]
mod station_void;

use bevy::prelude::*;
use diagnostics::{DiagnosticsPanel, DiagnosticsSystems};
use nova_protocol::{nova_debug::diagnostics, prelude::*};

/// The player's ship name.
const PLAYER_NAME: &str = "Gunboat";
/// PDC rounds in the hold at the start.
const HOLD_ROUNDS: u32 = 600;
/// The PDC's magazine, from its shipped content.
const MAGAZINE: u32 = 500;

/// The intro line.
const INTRO: &str =
    "600 PDC rounds aboard: hold RMB and LMB, and fire the hold down past LOW and DRY.";

fn main() -> bevy::app::AppExit {
    let mut app = AppBuilder::new()
        .with_game_plugins(ammo_warning_plugin)
        .build();
    app.add_plugins(diagnostics::DiagnosticsHintPlugin::new(INTRO));
    app.add_systems(
        PostUpdate,
        draw_the_reserve
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
    app.init_resource::<smoke::StopAtTheFlashPeak>();
    app.add_systems(
        Update,
        smoke::stop_at_the_flash_peak.run_if(in_state(GameStates::Playing)),
    );
    app.add_plugins(smoke::script());

    app.run()
}

fn ammo_warning_plugin(app: &mut App) {
    app.init_resource::<Crossings>();
    app.init_resource::<HeldTrigger>();
    app.add_observer(record_the_crossing);
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_the_void);
    app.add_systems(
        Update,
        hold_the_trigger.run_if(in_state(GameStates::Playing)),
    );
}

/// One prototype section at a build cell, square with the hull.
fn part(id: &str, prototype: &str, cell: Vec3) -> SpaceshipSectionConfig {
    SpaceshipSectionConfig {
        id: id.to_string(),
        position: cell,
        rotation: Quat::IDENTITY,
        source: SectionSource::prototype(prototype),
    }
}

/// Where a turret stands in the cell above a hull plate: seated down onto the
/// plate's top face, as the shipped block hulls mount it.
const TURRET_SEAT: Vec3 = Vec3::new(0.0, -0.25, 0.0);

/// The gunboat: a bow plate with one PDC on its roof, the bridge, two
/// shoulder plates and a drive. Three plates hold 300 kg, room for 1,500
/// PDC rounds.
fn gunboat() -> ShipDesign {
    ShipDesign {
        sections: vec![
            part("bow", REINFORCED_HULL_SECTION_ID, Vec3::NEG_Z),
            part(
                "pdc",
                PDC_KINETIC_TURRET_SECTION_ID,
                Vec3::new(0.0, 1.0, -1.0) + TURRET_SEAT,
            ),
            part("bridge", BASIC_CONTROLLER_SECTION_ID, Vec3::ZERO),
            part("shoulder_port", LIGHT_HULL_SECTION_ID, Vec3::NEG_X),
            part("shoulder_starboard", LIGHT_HULL_SECTION_ID, Vec3::X),
            part("drive", BASIC_THRUSTER_SECTION_ID, Vec3::Z),
        ],
        presentation: ShipPresentationConfig {
            skin: true,
            style: Some("industrial".to_string()),
            ..ShipPresentationConfig::base_voice()
        },
        ..default()
    }
}

fn load_the_void(mut commands: Commands, game_assets: Res<GameAssets>) {
    let player = station_void::player(
        PLAYER_NAME,
        Meters3::ZERO,
        SpaceshipConfig {
            controller: SpaceshipController::Player(PlayerControllerConfig::default()),
            design: ShipDesignSource::Inline(gunboat()),
            inventory: station_void::stock(&[(ItemRoleType::PdcRound.id(), HOLD_ROUNDS)]),
            ..default()
        },
    );
    commands.trigger(LoadScenario(station_void::void(
        "ammo_warning_sandbox",
        "Ammo Warning Sandbox",
        vec![player],
        &[(Meters3::ZERO, 1.0)],
        &game_assets,
    )));
}

/// Every reserve crossing so far, in order, as the walk and the panel read
/// them.
#[derive(Resource, Default)]
struct Crossings(Vec<(AmmoReserveWarningType, u32)>);

fn record_the_crossing(crossed: On<AmmoReserveCrossed>, mut crossings: ResMut<Crossings>) {
    info!(
        "ammo_warning_sandbox: {:?} at {} rounds of {}",
        crossed.warning, crossed.reserve, crossed.item
    );
    crossings.0.push((crossed.warning, crossed.reserve));
}

/// Whether the smoke walk holds the weapons raised and the trigger down.
#[derive(Resource, Default)]
struct HeldTrigger(bool);

/// While the walk holds the trigger: press the real combat stance every
/// frame, and once the safety reads hot, hold the PDC's trigger. The trigger
/// waits for the safety because a press while cold never latches.
fn hold_the_trigger(world: &mut World) {
    if !world.resource::<HeldTrigger>().0 {
        return;
    }
    drive_action(world, "combat_stance", InputPhase::Press);
    let Some(player) = station_void::player_root(world) else {
        return;
    };
    if !world.get::<WeaponsHot>(player).is_some_and(|hot| hot.0) {
        return;
    }
    let mut turrets = world.query::<(&ChildOf, &mut TurretSectionInput)>();
    for (&ChildOf(ship), mut input) in turrets.iter_mut(world) {
        if ship == player && !input.0 {
            input.0 = true;
        }
    }
}

/// The panel: the magazine, the hold against the LOW line, each crossing.
fn draw_the_reserve(
    mut panel: ResMut<DiagnosticsPanel>,
    crossings: Res<Crossings>,
    q_player: Query<(Entity, &ShipInventory), With<PlayerSpaceshipMarker>>,
    q_turrets: Query<(&SectionAmmo, &ChildOf), With<TurretSectionMarker>>,
) {
    let Ok((player, hold)) = q_player.single() else {
        return;
    };
    for (ammo, _) in q_turrets
        .iter()
        .filter(|(_, child)| child.parent() == player)
    {
        panel.line(format!("PDC magazine: {}/{}", ammo.rounds, ammo.capacity));
    }
    let reserve = hold.count(&ItemDesignId::from(ItemRoleType::PdcRound));
    let warning = ammo_reserve_warning(reserve, MAGAZINE);
    panel.line(format!(
        "Hold: {reserve} PDC rounds, LOW under {MAGAZINE}: {}",
        warning.map_or("clear".to_string(), |warning| format!("{warning:?}"))
    ));
    let seen: Vec<String> = crossings
        .0
        .iter()
        .map(|(warning, reserve)| format!("{warning:?} at {reserve}"))
        .collect();
    panel.line(format!(
        "Crossings: {}",
        if seen.is_empty() {
            "none".to_string()
        } else {
            seen.join(", ")
        }
    ));
    panel.hint("RMB", "raise weapons");
    panel.hint("LMB", "fire");
}

mod smoke {
    use std::sync::Arc;

    use nova_protocol::nova_debug::harness::{AutopilotPlugin, Predicate, SHOT_DEADLINE_SECS};

    use super::*;

    /// Seconds the walk gives the void to load.
    const LOAD_SECS: f32 = 120.0;
    /// Seconds a drain to the next line may take: the magazine, then a 3 s
    /// reload and a 2 s burst per batch, slowed by a software-rendered GPU.
    const DRAIN_SECS: f32 = 150.0;
    /// Seconds the HUD gets to show a crossing.
    const SHOW_SECS: f32 = 30.0;

    /// The frames the capture path writes.
    const BEFORE_SHOT: &str = "ammo-warning-sandbox-before.png";
    const FLASH_SHOT: &str = "ammo-warning-sandbox-flash.png";
    const LOW_SHOT: &str = "ammo-warning-sandbox-low.png";
    const DRY_SHOT: &str = "ammo-warning-sandbox-dry.png";
    /// A flash strength the walk stops on to shoot a peak. Two frames a
    /// quarter pulse apart always reach it: the pulse is a squared sine.
    const FLASH_PEAK: f32 = 0.45;

    const LOW_LINE: &str = "PDC ROUND LOW: 400";
    const DRY_LINE: &str = "PDC ROUND DRY";

    fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(check)
    }

    fn crossings(world: &World) -> Vec<(AmmoReserveWarningType, u32)> {
        world
            .get_resource::<Crossings>()
            .map(|crossings| crossings.0.clone())
            .unwrap_or_default()
    }

    /// The text of the first node carrying `M`, empty when there is none.
    fn text_of<M: Component>(world: &World) -> String {
        world
            .try_query_filtered::<&Text, With<M>>()
            .and_then(|mut query| query.iter(world).next().map(|text| text.0.clone()))
            .unwrap_or_default()
    }

    /// The reserve flash on the PDC's gauge, if one is armed or running.
    fn flash(world: &World) -> Option<AmmoReadoutReserveFlash> {
        world
            .try_query_filtered::<&AmmoReadoutReserveFlash, With<AmmoReadoutMarker>>()
            .and_then(|mut query| query.iter(world).next().copied())
    }

    /// How bright the gauge's flash is this frame, `0.0` with no flash.
    fn glow(world: &World) -> f32 {
        let now = world.resource::<Time<Virtual>>().elapsed_secs();
        flash(world)
            .and_then(|flash| flash.glow(now))
            .unwrap_or(0.0)
    }

    /// The PDC's gauge is on screen.
    fn gauge_drawn(world: &World) -> bool {
        world
            .try_query_filtered::<&Visibility, With<AmmoReadoutMarker>>()
            .is_some_and(|mut query| query.iter(world).any(|v| *v == Visibility::Visible))
    }

    /// While set, the clock stops on the first frame the flash is at least
    /// [`FLASH_PEAK`] bright, so the shot holds that frame. A walk step
    /// cannot do this: its `on_enter` runs a frame after its predicate, a
    /// whole pulse phase later on a software GPU.
    #[derive(Resource, Default)]
    pub(super) struct StopAtTheFlashPeak(bool);

    pub(super) fn stop_at_the_flash_peak(world: &mut World) {
        if world.resource::<StopAtTheFlashPeak>().0 && glow(world) >= FLASH_PEAK {
            world.resource_mut::<StopAtTheFlashPeak>().0 = false;
            pause_clock(world);
        }
    }

    /// The HUD shows `line` in the notice slot.
    fn shows(line: &'static str) -> Arc<Predicate> {
        when(move |world| text_of::<HudNoticeText>(world) == line)
    }

    fn magazine(world: &World) -> u32 {
        world
            .try_query_filtered::<(&SectionAmmo, &ChildOf), With<TurretSectionMarker>>()
            .and_then(|mut query| {
                query
                    .iter(world)
                    .find(|(_, child)| world.get::<PlayerSpaceshipMarker>(child.parent()).is_some())
                    .map(|(ammo, _)| ammo.rounds)
            })
            .unwrap_or_default()
    }

    fn held(world: &mut World) -> u32 {
        let player = station_void::player_root(world).expect("ammo_warning_sandbox: a player");
        world
            .get::<ShipInventory>(player)
            .expect("ammo_warning_sandbox: the player carries a hold")
            .count(&ItemDesignId::from(ItemRoleType::PdcRound))
    }

    fn player_turret(world: &World) -> bool {
        world
            .try_query_filtered::<&ChildOf, (With<TurretSectionMarker>, With<SectionAmmo>)>()
            .is_some_and(|mut query| {
                query
                    .iter(world)
                    .any(|child| world.get::<PlayerSpaceshipMarker>(child.parent()).is_some())
            })
    }

    /// The staging: a full magazine, 600 rounds aboard, no warning yet.
    fn assert_the_staging(world: &mut World) {
        assert_eq!(held(world), HOLD_ROUNDS);
        let player = station_void::player_root(world).expect("ammo_warning_sandbox: a player");
        let warnings = world
            .get::<AmmoReserveWarnings>(player)
            .expect("ammo_warning_sandbox: the player root carries the reserve levels");
        assert_eq!(
            warnings.warning(&ItemDesignId::from(ItemRoleType::PdcRound)),
            None,
            "ammo_warning_sandbox: 600 rounds is over the line"
        );
        assert!(crossings(world).is_empty());
        info!("ammo_warning_sandbox: PASS staged a full magazine and {HOLD_ROUNDS} rounds aboard");
    }

    fn assert_low(world: &mut World) {
        assert_eq!(
            crossings(world),
            [(AmmoReserveWarningType::Low, 400)],
            "ammo_warning_sandbox: one LOW crossing at 400"
        );
        assert_eq!(
            flash(world),
            None,
            "ammo_warning_sandbox: the LOW flash ended on its own"
        );
        info!(
            "ammo_warning_sandbox: PASS one LOW at 400, notice '{LOW_LINE}', one flash that ended"
        );
    }

    fn assert_dry(world: &mut World) {
        assert_eq!(held(world), 0);
        assert_eq!(
            crossings(world),
            [
                (AmmoReserveWarningType::Low, 400),
                (AmmoReserveWarningType::Dry, 0)
            ],
            "ammo_warning_sandbox: one LOW, then one DRY"
        );
        assert_eq!(
            flash(world),
            None,
            "ammo_warning_sandbox: DRY flashes nothing"
        );
        assert_eq!(
            magazine(world),
            0,
            "ammo_warning_sandbox: the magazine ran out"
        );
        info!("ammo_warning_sandbox: PASS one DRY at 0, notice '{DRY_LINE}', no flash, empty magazine");
    }

    fn restock(world: &mut World) {
        let items = world.resource::<GameItems>().clone();
        let player = station_void::player_root(world).expect("ammo_warning_sandbox: a player");
        world
            .get_mut::<ShipInventory>(player)
            .expect("ammo_warning_sandbox: the player carries a hold")
            .add(
                &items,
                &ItemDesignId::from(ItemRoleType::PdcRound),
                HOLD_ROUNDS,
            );
        info!("ammo_warning_sandbox: restocked {HOLD_ROUNDS} rounds");
    }

    fn assert_rearmed(world: &mut World) {
        assert_eq!(
            crossings(world),
            [
                (AmmoReserveWarningType::Low, 400),
                (AmmoReserveWarningType::Dry, 0),
                (AmmoReserveWarningType::Low, 400)
            ],
            "ammo_warning_sandbox: the restock re-armed LOW"
        );
        info!(
            "ammo_warning_sandbox: PASS the restock re-armed: LOW again at 400 and a second flash"
        );
    }

    fn pause_clock(world: &mut World) {
        world.resource_mut::<Time<Virtual>>().pause();
    }

    fn run_clock(world: &mut World) {
        world.resource_mut::<Time<Virtual>>().unpause();
    }

    fn cease_fire(world: &mut World) {
        world.resource_mut::<HeldTrigger>().0 = false;
        drive_action(world, "combat_stance", InputPhase::Release);
    }

    /// The smoke walk. Inert unless `NOVA_AUTOPILOT` is set.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        AutopilotPlugin::<GameStates>::new()
            .step("ammo warning sandbox: load the void")
            .enter(GameStates::Loading)
            .until(when(player_turret))
            .deadline(LOAD_SECS)
            .add()
            .step("ammo warning sandbox: let the scene build")
            .until(scenario_is_built())
            .deadline(LOAD_SECS)
            .add()
            .step("ammo warning sandbox: settle")
            .until(frames(10))
            .add()
            .step("ammo warning sandbox: check the staging")
            .on_enter(assert_the_staging)
            .until(frames(2))
            .add()
            .step("ammo warning sandbox: raise the weapons and hold fire")
            .on_enter(|world: &mut World| world.resource_mut::<HeldTrigger>().0 = true)
            .until(when(|world| {
                gauge_drawn(world) && magazine(world) < MAGAZINE
            }))
            .deadline(DRAIN_SECS)
            .add()
            .step("ammo warning sandbox: shoot the gauge before the crossing")
            .on_enter(|world: &mut World| {
                pause_clock(world);
                shoot(world, BEFORE_SHOT);
            })
            .until(shot_written(BEFORE_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
            .step("ammo warning sandbox: hold fire to LOW")
            .on_enter(run_clock)
            .until(when(|world| !crossings(world).is_empty()))
            .deadline(DRAIN_SECS)
            .add()
            .step("ammo warning sandbox: the notice posts LOW and the gauge flashes")
            .on_enter(|world: &mut World| world.resource_mut::<StopAtTheFlashPeak>().0 = true)
            .until(when(|world| {
                text_of::<HudNoticeText>(world) == LOW_LINE
                    && world.resource::<Time<Virtual>>().is_paused()
            }))
            .deadline(SHOW_SECS)
            .add()
            .step("ammo warning sandbox: shoot the flash")
            .on_enter(|world: &mut World| {
                assert!(
                    glow(world) >= FLASH_PEAK,
                    "ammo_warning_sandbox: the clock stopped on a flash peak"
                );
                info!(
                    "ammo_warning_sandbox: flash glow {:.2} in the shot",
                    glow(world)
                );
                shoot(world, FLASH_SHOT);
            })
            .until(shot_written(FLASH_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
            .step("ammo warning sandbox: the flash ends while LOW holds")
            .on_enter(run_clock)
            .until(when(|world| flash(world).is_none()))
            .deadline(SHOW_SECS)
            .add()
            .step("ammo warning sandbox: check LOW and shoot it")
            .on_enter(|world: &mut World| {
                pause_clock(world);
                assert_low(world);
                shoot(world, LOW_SHOT);
            })
            .until(shot_written(LOW_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
            .step("ammo warning sandbox: hold fire to DRY")
            .on_enter(run_clock)
            .until(when(|world| crossings(world).len() >= 2))
            .deadline(DRAIN_SECS)
            .add()
            .step("ammo warning sandbox: the HUD shows DRY")
            .until(shows(DRY_LINE))
            .deadline(SHOW_SECS)
            .add()
            .step("ammo warning sandbox: fire the magazine out")
            .until(when(|world| magazine(world) == 0))
            .deadline(DRAIN_SECS)
            .add()
            .step("ammo warning sandbox: check DRY and shoot it")
            .on_enter(|world: &mut World| {
                pause_clock(world);
                assert_dry(world);
                shoot(world, DRY_SHOT);
            })
            .until(shot_written(DRY_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
            .step("ammo warning sandbox: restock and drain to LOW again")
            .on_enter(|world: &mut World| {
                run_clock(world);
                restock(world);
            })
            .until(when(|world| crossings(world).len() >= 3))
            .deadline(DRAIN_SECS)
            .add()
            .step("ammo warning sandbox: the gauge flashes again")
            .until(when(|world| glow(world) > 0.0))
            .deadline(SHOW_SECS)
            .add()
            .step("ammo warning sandbox: check the re-arm")
            .on_enter(|world: &mut World| {
                assert_rearmed(world);
                cease_fire(world);
            })
            .until(frames(2))
            .add()
    }
}
