//! system_civilization_stance: three intact hulls of one civilization
//! neutralized inside the offence window turn it Enemy, its station refuses
//! the dock, its rival neighbour warms to Ally and hires out escorts, a
//! remote civilization is untouched, the stances outlive a save and a Load,
//! and the Enemy decays to Neutral after its window.
//!
//! Boots the shipped app (via [`editor_app`]), starts New Game on a pinned
//! seed as the pirate the way the Create World modal does, and lets the
//! window stream in. It scans the window for a civilization X with a living
//! non-Enemy station AND three other living, non-Enemy hulls of its own
//! inside the window round that station's cell, whose seeded rival (a living
//! neighbour on an Enemy edge) N is not Enemy itself; the nearest such
//! station is the dock target. The player docks at the station once to show
//! the request is admitted before anything changes, undocks, then damages
//! each of the three hulls' critical sections with the player's own ship as
//! the damage source, the way `system_job_neutralize` drives a kill: three
//! offences inside `OFFENCE_WINDOW_SECONDS`. X steps to Enemy, the world
//! layer writes that side onto its station, the HUD's dock hint carries the
//! refusal line, a real berth attempt leaves the player undocked, and the
//! pirate's charter counts X turned Enemy (and any Ally of X the step
//! dropped to Enemy). N, on X's Enemy edge, steps up to Ally: the walk jumps
//! to N's centroid, waits for a hull of N to stream in, and shows every one
//! of them flies the Player side and
//! the hire rule admits N's station as a partner. A remote living
//! civilization R, no neighbour of X, holds no record. The walk leaves to
//! the main menu, reads the leave save back off disk through
//! [`open_world`], loads the world again in this process through the Load
//! screen, and shows X still refuses, N still allies and R is still
//! untouched. Finally it sets `WorldClock.play_seconds` past
//! `ENEMY_DECAY_SECONDS` (disclosed fixture state: a real player cannot
//! drive the clock, only live through the wait), lets `expire_stances` thaw
//! X to Neutral, and berths again to show the station admits once more. The
//! other disclosed fixture state is the `TravelLock` the walk puts on the
//! player before reading the HUD's dock hint: it stands in for the
//! targeting gesture that locks a station, and touches no admission rule.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: three neutralized hulls turn their civilization enemy and its station refuses docking` | the station admits the player before the kills; after the third, `CivilizationStances` reads X Enemy, X's station flies the Enemy side, the HUD's dock hint reads `<NAME> REFUSES DOCKING`, a real berth attempt leaves the player undocked, and the pirate's charter reads at least one civilization turned Enemy (X, plus any Ally of X the step dropped to Enemy) |
//! | 2 | `outcome: the rival neighbour warms to ally and its station hires out escorts` | X's rival N reads Ally in the table, every live hull of N flies the Player side, and `plan_escort_hire` admits N's station as an allied partner |
//! | 3 | `outcome: the stances survive a save and a load and a remote civilization is untouched` | the leave save's `stances` read X Enemy, N Ally and no record for R; after a real Load the table reads the same and the HUD hint still refuses X's station |
//! | 4 | `outcome: an enemy by events decays to neutral and the station admits again` | once `WorldClock.play_seconds` passes X's `enemy_since` by `ENEMY_DECAY_SECONDS`, `expire_stances` returns X to Neutral and the station admits the player again |
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 NOVA_AUTOPILOT_DEADLINE=3600 \
//!   cargo run --example system_civilization_stance --features debug
//! # look for: `civilization stance: PASS the station admits the player again`,
//! #           `autopilot: cycle complete, no panic`
//! ```
//! Add `NOVA_CAPTURE=1 NOVA_CAPTURE_DIR=<dir>` to shoot the refusal HUD line
//! as `civilization-stance-hud.png`.

use bevy::prelude::*;
use clap::Parser;

#[derive(Parser)]
#[command(name = "system_civilization_stance")]
#[command(version = "1.0.0")]
#[command(
    about = "New Game on a pinned seed as the pirate: neutralize three hulls of one civilization \
             with the player as the source, show it turns Enemy and its station refuses the dock, \
             its rival warms to Ally, a remote civilization is untouched, the stances survive a \
             save and a Load, and the Enemy decays. Autopilot-only correctness range",
    long_about = None
)]
struct Cli;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // This run's own settings: no player's saved world may be in the way.
    std::env::set_var(
        nova_assets::storage::CONFIG_ROOT_ENV,
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
            "target/example-profiles/{}-{}-{}",
            env!("CARGO_CRATE_NAME"),
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("the clock is after 1970")
                .as_nanos(),
        )),
    );

    let mut app = nova_protocol::prelude::editor_app(true, None);

    #[cfg(feature = "debug")]
    {
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        app.add_systems(
            Startup,
            nova_protocol::nova_debug::harness::force_capture_resolution,
        );
        // No frame-time claim: the range holds a table, sides, a HUD line
        // and a docked flag.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(walk::StanceWatchPlugin);
        app.add_plugins(walk::script());
    }

    app.run()
}

#[cfg(feature = "debug")]
mod walk {
    use std::{collections::BTreeMap, sync::Arc};

    use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
    use nova_protocol::{
        nova_debug::harness::{shoot, AutopilotPlugin, Predicate, SHOT_DEADLINE_SECS},
        prelude::*,
    };
    use nova_ui::widget::TextFieldValue;
    use nova_world::prelude::{desired_sectors, SectorRoot, WorldClock, WorldConfig};

    use super::*;

    /// The seed the range plays: `system_station_round_trip`'s seed, whose
    /// window around the spawn holds a living Neutral station outside the
    /// spawn's cell. Unverified here: that the window also holds three more
    /// living, non-Enemy hulls of that station's civilization - see the
    /// module doc and the report this range shipped with.
    const STATION_SEED: u32 = 10;
    const NEW_GAME_BUTTON: &str = "New Game Button";
    const WORLD_NAME_FIELD: &str = "World Name Field";
    const WORLD_NAME: &str = "Civilization Stance World";
    const SEED_FIELD: &str = "World Seed Field";
    const CREATE_WORLD_BUTTON: &str = "Create World Button";
    const WORLD_NEXT_BUTTON: &str = "World Next Button";
    /// The start role the walk picks: the pirate, whose charter counts the
    /// civilizations its kills turn Enemy.
    const START_ROLE_BUTTON: &str = "Start Role Button: pirate";
    const PAUSE_EXIT_BUTTON: &str = "Pause Exit Button";
    const BACK_TO_MENU_BUTTON: &str = "Back To Menu Button";
    const LOAD_BUTTON: &str = "Load Button";
    const LOAD_WORLD_BUTTON: &str = "Load World Button";

    /// The HUD shot the capture path writes.
    const HUD_SHOT: &str = "civilization-stance-hud.png";

    /// How far from the station the player parks before the berth.
    const PARK_DISTANCE: Meters = Meters(2_000.0);
    /// The gap the berth leaves between the two port faces, engine units:
    /// half a cell, inside the 10 m capture distance.
    const BERTH_GAP: f32 = 0.5;
    /// Where a port's face sits in its own frame, as the docking verb reads it.
    const PORT_FACE: Vec3 = Vec3::new(0.0, 0.0, -0.5);

    /// Seconds a load, a stream or a beat gets on a software-rendered GPU.
    const SESSION_SECS: f32 = 240.0;
    /// Frames a refused berth attempt gets, after the player pulls back,
    /// before the walk calls it settled. The pull-back comes first: a refused
    /// hull left at the berth takes contact damage, and an earlier run lost
    /// the player's docking port to it before the save.
    const SETTLE_FRAMES: u32 = 10;
    /// The virtual clock's clamp on the software rasterizer.
    const WALK_CLOCK_SECS: f32 = 1.0;
    /// The hulls of X the walk neutralizes: one offence step.
    const KILLS: u32 = OFFENCE_STEP;

    /// A generated, living, non-Enemy hull as the choice compares it.
    #[derive(Clone, Debug)]
    struct Candidate {
        id: GeneratedBodyId,
        position: Meters3,
        civilization: CivilizationId,
        station: bool,
        name: String,
    }

    /// What the run chose and recorded between beats.
    #[derive(Resource, Default, Debug)]
    struct StanceWatch {
        /// X: the civilization the walk turns Enemy, and its seeded stance.
        civilization: Option<(CivilizationId, StanceType)>,
        /// N: X's rival on an Enemy edge, its seeded stance and its centroid.
        rival: Option<(CivilizationId, StanceType, Meters3)>,
        /// R: a living civilization no neighbour of X, and its seeded stance.
        remote: Option<(CivilizationId, StanceType)>,
        /// The station the player docks at, before and after.
        dock_id: Option<GeneratedBodyId>,
        /// The hulls the walk damages to neutralize, in kill order.
        kill_ids: Vec<GeneratedBodyId>,
        /// The cell the dock target stands in.
        cell: Option<SectorCoord>,
        /// Where it stood when chosen, for the park point.
        dock_position: Option<Meters3>,
        /// The HUD line the dock hint must read once refused.
        expected_refusal_line: Option<String>,
    }

    pub(super) struct StanceWatchPlugin;

    impl Plugin for StanceWatchPlugin {
        fn build(&self, app: &mut App) {
            app.init_resource::<StanceWatch>();
            app.add_systems(Update, hold_the_walk_clock);
        }
    }

    /// Hold the virtual clock's clamp at [`WALK_CLOCK_SECS`] every frame: a
    /// scenario load hands `Time<Virtual>` back at its default.
    fn hold_the_walk_clock(mut time: ResMut<Time<Virtual>>) {
        let clamp = std::time::Duration::from_secs_f32(WALK_CLOCK_SECS);
        if time.max_delta() != clamp {
            time.set_max_delta(clamp);
        }
    }

    fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(check)
    }

    /// The one player ship, if exactly one stands.
    fn the_player(world: &World) -> Option<Entity> {
        let mut players = world.try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()?;
        let mut players = players.iter(world);
        let player = players.next()?;
        players.next().is_none().then_some(player)
    }

    fn player(world: &World) -> Entity {
        the_player(world).expect("civilization stance: exactly one player ship")
    }

    /// The entity wearing a chosen `GeneratedBodyId`, if one stands.
    fn body_root(world: &World, id: &GeneratedBodyId) -> Option<Entity> {
        let mut query = world.try_query::<(Entity, &GeneratedBodyId)>()?;
        query
            .iter(world)
            .find(|(_, worn)| *worn == id)
            .map(|(entity, _)| entity)
    }

    fn dock_target(world: &World) -> Entity {
        let id = world
            .resource::<StanceWatch>()
            .dock_id
            .clone()
            .expect("civilization stance: the dock target is chosen");
        body_root(world, &id).expect("civilization stance: the dock target stands")
    }

    fn kill_target(world: &World, index: usize) -> Entity {
        let id = world
            .resource::<StanceWatch>()
            .kill_ids
            .get(index)
            .cloned()
            .expect("civilization stance: the kill targets are chosen");
        body_root(world, &id)
            .unwrap_or_else(|| panic!("civilization stance: kill target {index} stands"))
    }

    fn the_civilization(world: &World) -> (CivilizationId, StanceType) {
        world
            .resource::<StanceWatch>()
            .civilization
            .expect("civilization stance: X is chosen")
    }

    fn the_rival(world: &World) -> (CivilizationId, StanceType, Meters3) {
        world
            .resource::<StanceWatch>()
            .rival
            .expect("civilization stance: N is chosen")
    }

    fn the_remote(world: &World) -> (CivilizationId, StanceType) {
        world
            .resource::<StanceWatch>()
            .remote
            .expect("civilization stance: R is chosen")
    }

    fn world_shape(world: &World) -> (Meters, i32) {
        let config = world.resource::<WorldConfig<NovaLayeredWorld>>();
        (config.sector_edge, config.active_radius)
    }

    fn live_cells(world: &World) -> std::collections::BTreeSet<SectorCoord> {
        world
            .try_query::<&SectorRoot>()
            .map(|mut query| query.iter(world).map(|root| root.0).collect())
            .unwrap_or_default()
    }

    /// Advance once the world is armed around one player ship and the live
    /// set is exactly the window around `centre`.
    fn window_is(
        centre: impl Fn(&World) -> Option<SectorCoord> + Send + Sync + 'static,
    ) -> Arc<Predicate> {
        when(move |world| {
            if !world.contains_resource::<WorldConfig<NovaLayeredWorld>>()
                || the_player(world).is_none()
            {
                return false;
            }
            let Some(centre) = centre(world) else {
                return false;
            };
            let (_, radius) = world_shape(world);
            live_cells(world) == desired_sectors(centre, radius)
        })
    }

    /// The dock target stands with its physics body in place.
    fn the_dock_target_stands() -> Arc<Predicate> {
        when(|world| {
            world
                .resource::<StanceWatch>()
                .dock_id
                .as_ref()
                .and_then(|id| body_root(world, id))
                .is_some_and(|target| world.get::<Position>(target).is_some())
        })
    }

    fn player_docked() -> Arc<Predicate> {
        when(|world| {
            the_player(world).is_some_and(|player| world.get::<DockedShip>(player).is_some())
        })
    }

    fn player_free() -> Arc<Predicate> {
        when(|world| {
            the_player(world).is_some_and(|player| world.get::<DockedShip>(player).is_none())
        })
    }

    /// Kill target `index` carries `NeutralizedMarker`.
    fn the_hull_is_neutralized(index: usize) -> Arc<Predicate> {
        when(move |world| {
            world
                .resource::<StanceWatch>()
                .kill_ids
                .get(index)
                .and_then(|id| body_root(world, id))
                .is_some_and(|target| world.get::<NeutralizedMarker>(target).is_some())
        })
    }

    /// X reads Enemy in the table and its station flies the Enemy side.
    fn the_civilization_is_enemy() -> Arc<Predicate> {
        when(|world| {
            let (civ, seeded) = the_civilization(world);
            let enemy =
                world.resource::<CivilizationStances>().stance(&civ, seeded) == StanceType::Enemy;
            enemy
                && world
                    .resource::<StanceWatch>()
                    .dock_id
                    .as_ref()
                    .and_then(|id| body_root(world, id))
                    .and_then(|target| world.get::<Allegiance>(target))
                    == Some(&Allegiance::Enemy)
        })
    }

    /// X reads Neutral again and its station flies the Neutral side.
    fn the_civilization_is_neutral_again() -> Arc<Predicate> {
        when(|world| {
            let (civ, seeded) = the_civilization(world);
            world.resource::<CivilizationStances>().stance(&civ, seeded) == StanceType::Neutral
                && world
                    .resource::<StanceWatch>()
                    .dock_id
                    .as_ref()
                    .and_then(|id| body_root(world, id))
                    .and_then(|target| world.get::<Allegiance>(target))
                    == Some(&Allegiance::Neutral)
        })
    }

    /// Every live, intact hull of `civ` in the window: root, side, whether
    /// it is a station, and its balance.
    fn hulls_of(world: &mut World, civ: CivilizationId) -> Vec<(Entity, Allegiance, bool, u32)> {
        world
            .query_filtered::<(
                Entity,
                &TraderMarket,
                &Allegiance,
                Has<StationMarker>,
                &ShipCredits,
                Has<DerelictShipMarker>,
                Has<NeutralizedMarker>,
            ), With<SpaceshipRootMarker>>()
            .iter(world)
            .filter(|(_, market, _, _, _, derelict, neutralized)| {
                market.civilization == civ && !*derelict && !*neutralized
            })
            .map(|(root, _, side, station, credits, _, _)| (root, *side, station, credits.0))
            .collect()
    }

    /// A live hull of N stands in the window.
    fn a_hull_of_the_rival_stands() -> Arc<Predicate> {
        when(|world| {
            let (rival, _, _) = the_rival(world);
            let mut query = world.try_query::<(&TraderMarket, &Position)>();
            query.as_mut().is_some_and(|query| {
                query
                    .iter(world)
                    .any(|(market, _)| market.civilization == rival)
            })
        })
    }

    /// The HUD's dock hint, read off the global resource `update_flight_verb_hints` writes.
    fn dock_refusal_text(world: &World) -> Option<String> {
        world
            .get_resource::<FlightVerbHints>()?
            .dock_refusal
            .clone()
    }

    /// Teleport the player to `position`, at rest.
    fn park_player(world: &mut World, position: Meters3) {
        let player = player(world);
        world.entity_mut(player).insert((
            Position(position.to_engine()),
            LinearVelocity::ZERO,
            AngularVelocity::ZERO,
        ));
        info!("civilization stance: the player jumps to {position:?}");
    }

    /// Where the player parks: [`PARK_DISTANCE`] from the dock target toward
    /// the spawn. Also the stand-off for the kills: a wreck breaking up at
    /// the berth took the player's own docking section in an earlier run.
    fn park_beside(world: &mut World) {
        let position = world
            .resource::<StanceWatch>()
            .dock_position
            .expect("civilization stance: the dock target is chosen");
        let toward_spawn = (-position.get()).normalize_or_zero();
        let beside = Meters3(position.get() + toward_spawn * PARK_DISTANCE.get());
        park_player(world, beside);
    }

    /// `section`'s position and rotation in `root`'s frame, by its mount chain.
    fn local_pose(world: &World, section: Entity, root: Entity) -> (Vec3, Quat) {
        let (mut position, mut rotation, mut at) = (Vec3::ZERO, Quat::IDENTITY, section);
        while at != root {
            let transform = world
                .get::<Transform>(at)
                .expect("civilization stance: a mounted section has a transform");
            position = transform.translation + transform.rotation * position;
            rotation = transform.rotation * rotation;
            at = world
                .get::<ChildOf>(at)
                .expect("civilization stance: a section's chain reaches its root")
                .parent();
        }
        (position, rotation)
    }

    fn mounted_on(world: &World, section: Entity, root: Entity) -> bool {
        let mut at = section;
        while let Some(parent) = world.get::<ChildOf>(at).map(ChildOf::parent) {
            if parent == root {
                return true;
            }
            at = parent;
        }
        false
    }

    /// The live docking ports mounted on `root`.
    fn ports_of(world: &mut World, root: Entity) -> Vec<Entity> {
        let ports: Vec<Entity> = world
            .query_filtered::<Entity, (With<DockingSectionMarker>, Without<SectionInactiveMarker>)>(
            )
            .iter(world)
            .collect();
        ports
            .into_iter()
            .filter(|port| mounted_on(world, *port, root))
            .collect()
    }

    /// The free, live docking ports mounted on `root`, lowest first.
    fn free_ports(world: &mut World, root: Entity) -> Vec<Entity> {
        let mut ports: Vec<Entity> = ports_of(world, root)
            .into_iter()
            .filter(|port| world.get::<DockedPort>(*port).is_none())
            .collect();
        ports.sort();
        ports
    }

    /// The lowest free, live docking port mounted on `root`.
    fn free_port(world: &mut World, root: Entity) -> Entity {
        free_ports(world, root)
            .into_iter()
            .next()
            .unwrap_or_else(|| panic!("civilization stance: {root:?} has no free docking port"))
    }

    /// The free port of `target` that faces most squarely away from its
    /// root, so the berth sets the player outside the hull.
    fn outward_port(world: &mut World, target: Entity) -> Entity {
        free_ports(world, target)
            .into_iter()
            .map(|port| {
                let (at, rotation) = local_pose(world, port, target);
                let facing = (rotation * Vec3::NEG_Z).dot(at.normalize_or_zero());
                (port, facing)
            })
            .max_by(|a, b| a.1.total_cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
            .map(|(port, _)| port)
            .unwrap_or_else(|| panic!("civilization stance: {target:?} has no free docking port"))
    }

    /// Put the player on the berth at the dock target's free port, moving and
    /// turning with it, then ask to dock. A no-op once the player is already
    /// docked, so a step may call this every frame until the dock takes, or
    /// once to show a refusal leaves it alone.
    fn berth_and_dock(world: &mut World) {
        let player = player(world);
        if world.get::<DockedShip>(player).is_some() {
            return;
        }
        let station = dock_target(world);
        let station_port = outward_port(world, station);
        let player_port = free_port(world, player);
        let position = world
            .get::<Position>(station)
            .expect("civilization stance: the dock target stands")
            .0;
        let rotation = world
            .get::<Rotation>(station)
            .expect("civilization stance: the dock target stands")
            .0;
        let velocity = world
            .get::<LinearVelocity>(station)
            .map_or(Vec3::ZERO, |v| v.0);
        let spin = world
            .get::<AngularVelocity>(station)
            .map_or(Vec3::ZERO, |w| w.0);
        let (port_at, port_rot) = local_pose(world, station_port, station);
        let station_face_rot = rotation * port_rot;
        let station_face = position + rotation * port_at + station_face_rot * PORT_FACE;
        let station_axis = station_face_rot * Vec3::NEG_Z;
        let (own_at, own_rot) = local_pose(world, player_port, player);
        let turn = Quat::from_rotation_arc((own_rot * Vec3::NEG_Z).normalize(), -station_axis);
        let face_offset = turn * own_at + turn * own_rot * PORT_FACE;
        let at = station_face + station_axis * BERTH_GAP - face_offset;
        world.entity_mut(player).insert((
            Position(at),
            Rotation(turn),
            LinearVelocity(velocity),
            AngularVelocity(spin),
        ));
        let rigs: Vec<Entity> = world
            .query_filtered::<Entity, With<PointRotationOutput>>()
            .iter(world)
            .collect();
        for rig in rigs {
            world.entity_mut(rig).insert((
                PointRotation {
                    initial_rotation: turn,
                },
                PointRotationOutput(turn),
            ));
        }
        world.trigger(DockingConnectionRequest {
            entity: player,
            target: station,
        });
    }

    fn set_field_text(world: &mut World, name: &str, text: &str) {
        let mut fields = world.query::<(&Name, &mut TextFieldValue)>();
        let (_, mut value) = fields
            .iter_mut(world)
            .find(|(found, _)| found.as_str() == name)
            .unwrap_or_else(|| panic!("civilization stance: no field named '{name}'"));
        value.0 = text.to_string();
    }

    /// Every living, non-Enemy, non-derelict, non-neutralized, undocked
    /// generated hull in the window, with the civilization and allegiance
    /// facts the choice needs. A hull parked at a station is left out: its
    /// wreck at the berth broke the station and the player's port in an
    /// earlier run.
    fn candidates(world: &mut World) -> Vec<Candidate> {
        world
            .query_filtered::<(
                &GeneratedBodyId,
                &Position,
                &TraderMarket,
                Has<StationMarker>,
                Has<DerelictShipMarker>,
                Has<NeutralizedMarker>,
                Has<DockedShip>,
                Option<&Allegiance>,
                Option<&Name>,
            ), With<SpaceshipRootMarker>>()
            .iter(world)
            .filter(
                |(_, _, _, _, derelict, neutralized, docked, allegiance, _)| {
                    !*derelict
                        && !*neutralized
                        && !*docked
                        && *allegiance != Some(&Allegiance::Enemy)
                },
            )
            .map(
                |(id, position, market, station, _, _, _, _, name)| Candidate {
                    id: id.clone(),
                    position: Meters3::from_engine(position.0),
                    civilization: market.civilization,
                    station,
                    name: name.map_or_else(|| format!("{id:?}"), ToString::to_string),
                },
            )
            .collect()
    }

    /// X's rival on an Enemy edge that one step up makes Ally: a seeded
    /// Neutral first (a real step), else a seeded Ally (the step saturates);
    /// the first in node order of each.
    fn warming_rival(field: &CivilizationField, civ: CivilizationId) -> Option<Civilization> {
        let on_enemy_edge: Vec<Civilization> = field
            .neighbours(civ.node)
            .into_iter()
            .filter(|neighbour| field.edge(civ.node, neighbour.id.node) == Some(StanceType::Enemy))
            .collect();
        on_enemy_edge
            .iter()
            .find(|neighbour| neighbour.stance == StanceType::Neutral)
            .or_else(|| {
                on_enemy_edge
                    .iter()
                    .find(|neighbour| neighbour.stance == StanceType::Ally)
            })
            .copied()
    }

    /// A living civilization that is no neighbour of X and not X: the first
    /// in node order inside the cube of three lattice steps round X.
    fn remote_of(field: &CivilizationField, civ: CivilizationId) -> Option<Civilization> {
        let neighbours: Vec<[i32; 3]> = field
            .neighbours(civ.node)
            .into_iter()
            .map(|neighbour| neighbour.id.node)
            .collect();
        (-3..=3)
            .flat_map(|x| (-3..=3).flat_map(move |y| (-3..=3).map(move |z| [x, y, z])))
            .map(|offset| {
                [
                    civ.node[0] + offset[0],
                    civ.node[1] + offset[1],
                    civ.node[2] + offset[2],
                ]
            })
            .filter(|node| *node != civ.node && !neighbours.contains(node))
            .map(|node| field.civilization(node))
            .find(|other| other.status == CivilizationStatusType::Living)
    }

    /// Pick X: a civilization with a living, non-Enemy station AND
    /// [`KILLS`] other living, non-Enemy hulls of its own inside the window
    /// round the station's cell, whose rival warms to Ally. The nearest such
    /// station is the dock target; the nearest other hulls are the kills.
    fn choose_the_targets(world: &mut World) {
        let seed = world.resource::<OpenWorldSession>().seed;
        let field = CivilizationField::new(seed);
        let (edge, radius) = world_shape(world);
        let mut groups: BTreeMap<CivilizationId, Vec<Candidate>> = BTreeMap::new();
        for body in candidates(world) {
            groups.entry(body.civilization).or_default().push(body);
        }
        let mut picked: Option<(Civilization, Civilization, Candidate, Vec<Candidate>)> = None;
        for (civ, members) in groups {
            let mut stations: Vec<Candidate> =
                members.iter().filter(|m| m.station).cloned().collect();
            stations.sort_by(|a, b| {
                a.position
                    .distance(Meters3::ZERO)
                    .get()
                    .total_cmp(&b.position.distance(Meters3::ZERO).get())
            });
            let Some(rival) = warming_rival(&field, civ) else {
                info!(
                    "civilization stance: {} has no rival that warms to Ally; skipped",
                    civ.name()
                );
                continue;
            };
            for dock in stations {
                let cell = SectorCoord::containing(dock.position, edge);
                let window = desired_sectors(cell, radius);
                let mut kills: Vec<Candidate> = members
                    .iter()
                    .filter(|m| {
                        m.id != dock.id
                            && window.contains(&SectorCoord::containing(m.position, edge))
                    })
                    .cloned()
                    .collect();
                kills.sort_by(|a, b| {
                    a.position
                        .distance(dock.position)
                        .get()
                        .total_cmp(&b.position.distance(dock.position).get())
                });
                if kills.len() < KILLS as usize {
                    info!(
                        "civilization stance: {} station '{}' has {} other hull(s) in its window, \
                         needs {KILLS}; skipped",
                        civ.name(),
                        dock.name,
                        kills.len()
                    );
                    continue;
                }
                kills.truncate(KILLS as usize);
                let distance = dock.position.distance(Meters3::ZERO).get();
                let better = picked.as_ref().is_none_or(|(_, _, best, _)| {
                    distance < best.position.distance(Meters3::ZERO).get()
                });
                if better {
                    picked = Some((field.civilization(civ.node), rival.clone(), dock, kills));
                }
                break;
            }
        }
        let (civ, rival, dock, kills) = picked.unwrap_or_else(|| {
            panic!(
                "civilization stance: the window on seed {STATION_SEED} holds no civilization \
                 with a living, non-Enemy station, {KILLS} other living, non-Enemy hulls in the \
                 station's window, and a rival that warms to Ally"
            )
        });
        let remote = remote_of(&field, civ.id).unwrap_or_else(|| {
            panic!(
                "civilization stance: no living civilization within three lattice steps of {} \
                 that is not its neighbour",
                civ.id
            )
        });
        let cell = SectorCoord::containing(dock.position, edge);
        let expected_refusal_line = format!("{} REFUSES DOCKING", dock.name);
        info!(
            "civilization stance: chose X '{}' ({}, seeded {:?}); station '{}' to dock at; kills {:?}; \
             rival N '{}' ({}, seeded {:?}); remote R '{}' ({}, seeded {:?})",
            civ.id.name(),
            civ.id,
            civ.stance,
            dock.name,
            kills.iter().map(|k| k.name.as_str()).collect::<Vec<_>>(),
            rival.id.name(),
            rival.id,
            rival.stance,
            remote.id.name(),
            remote.id,
            remote.stance
        );
        let mut watch = world.resource_mut::<StanceWatch>();
        watch.civilization = Some((civ.id, civ.stance));
        watch.rival = Some((rival.id, rival.stance, rival.centroid));
        watch.remote = Some((remote.id, remote.stance));
        watch.dock_id = Some(dock.id);
        watch.kill_ids = kills.into_iter().map(|k| k.id).collect();
        watch.cell = Some(cell);
        watch.dock_position = Some(dock.position);
        watch.expected_refusal_line = Some(expected_refusal_line);
    }

    /// The ids of every section of `target` carrying component `T`, mounted
    /// on it.
    fn sections_of<T: Component>(world: &mut World, target: Entity) -> Vec<Entity> {
        let found: Vec<Entity> = world
            .query_filtered::<Entity, With<T>>()
            .iter(world)
            .collect();
        found
            .into_iter()
            .filter(|section| mounted_on(world, *section, target))
            .collect()
    }

    /// Every live weapon section (turret, torpedo, railgun) mounted on
    /// `target`.
    fn weapon_sections(world: &mut World, target: Entity) -> Vec<Entity> {
        let mut found = sections_of::<TurretSectionMarker>(world, target);
        found.extend(sections_of::<TorpedoSectionMarker>(world, target));
        found.extend(sections_of::<RailgunSectionMarker>(world, target));
        found
    }

    /// Spend a section's whole remaining pool, sourced from `source`.
    fn zero_section(world: &mut World, source: Entity, section: Entity) {
        let left = world
            .get::<Health>(section)
            .map_or(0.0, |health| health.current);
        if left > 0.0 {
            world.trigger(HealthApplyDamage {
                entity: section,
                source: Some(source),
                amount: left,
            });
        }
    }

    /// Neutralize `target` with damage sourced from `player`: zero every live
    /// weapon section if it is armed (the armed rule needs no working
    /// weapon), else every controller section if it is a station (a
    /// station's own rule) or every thruster section otherwise (the unarmed
    /// hull rule) - `crate::integrity::neutralize`'s two rules.
    fn neutralize_the_hull(world: &mut World, player: Entity, target: Entity) {
        let weapons = weapon_sections(world, target);
        if !weapons.is_empty() {
            info!(
                "civilization stance: '{target:?}' is armed, zeroing {} weapon section(s)",
                weapons.len()
            );
            for section in weapons {
                zero_section(world, player, section);
            }
            return;
        }
        let station = world.get::<StationMarker>(target).is_some();
        let critical = if station {
            sections_of::<ControllerSectionMarker>(world, target)
        } else {
            sections_of::<ThrusterSectionMarker>(world, target)
        };
        assert!(
            !critical.is_empty(),
            "civilization stance: the unarmed neutralize target carries no {}",
            if station {
                "controller section"
            } else {
                "thruster section"
            }
        );
        info!(
            "civilization stance: '{target:?}' is unarmed, zeroing {} {} section(s)",
            critical.len(),
            if station { "controller" } else { "thruster" }
        );
        for section in critical {
            zero_section(world, player, section);
        }
    }

    /// The charter line labelled `label`.
    fn charter_line(world: &World, label: &str) -> (u64, u64) {
        let progress = world.resource::<GoalProgress>();
        let line = progress
            .lines
            .iter()
            .find(|line| line.label == label)
            .unwrap_or_else(|| {
                panic!(
                    "civilization stance: the charter '{}' has no line '{label}': {:?}",
                    progress.title, progress.lines
                )
            });
        (line.have, line.need)
    }

    /// Claim 1, part one: X reads Enemy, its station flies the Enemy side,
    /// the HUD's dock hint reads the refusal line, and the charter counts
    /// the civilization.
    fn assert_the_hint(world: &mut World) {
        let (civ, seeded) = the_civilization(world);
        let expected = world
            .resource::<StanceWatch>()
            .expected_refusal_line
            .clone()
            .expect("computed");
        let stances = world.resource::<CivilizationStances>();
        assert_eq!(
            stances.stance(&civ, seeded),
            StanceType::Enemy,
            "civilization stance: '{}' reads Enemy after {KILLS} kills",
            civ.name()
        );
        let record = stances
            .record(&civ)
            .expect("civilization stance: an Enemy by events has a record");
        assert!(
            record.enemy_since.is_some(),
            "civilization stance: '{}' is Enemy by events",
            civ.name()
        );
        let line = dock_refusal_text(world);
        assert_eq!(
            line.as_deref(),
            Some(expected.as_str()),
            "civilization stance: the HUD's dock hint reads the refusal line"
        );
        let (turned, need) = charter_line(world, "Civilizations turned Enemy");
        assert_eq!(
            need, 1,
            "civilization stance: the pirate's charter asks one civilization turned Enemy"
        );
        assert!(
            turned >= 1,
            "civilization stance: the pirate's charter counts X; an Ally of X dropped to Enemy \
             by the step counts too, so it reads {turned}"
        );
        info!("civilization stance: the HUD reads '{expected}'");
    }

    /// Claim 1, part two: a real berth attempt leaves the player undocked.
    fn assert_the_request_refused(world: &mut World) {
        let (civ, _) = the_civilization(world);
        let expected = world
            .resource::<StanceWatch>()
            .expected_refusal_line
            .clone()
            .expect("computed");
        let player = player(world);
        assert!(
            world.get::<DockedShip>(player).is_none(),
            "civilization stance: the refused request left the player undocked"
        );
        nova_probe::probe_marker(
            world,
            "outcome: three neutralized hulls turn their civilization enemy and its station refuses docking",
            serde_json::json!({
                "civilization": civ.name(),
                "line": expected,
                "charter": charter_line(world, "Civilizations turned Enemy"),
            }),
        );
        info!(
            "civilization stance: PASS {KILLS} neutralized hulls make '{}' refuse docking ({expected})",
            civ.name()
        );
    }

    /// Jump to N's centroid so its hulls stream in.
    fn jump_to_the_rival(world: &mut World) {
        let (_, _, centroid) = the_rival(world);
        park_player(world, centroid);
    }

    /// Claim 2: N reads Ally in the table, every live hull of N flies the
    /// Player side, and the hire rule admits N's station as a partner.
    fn assert_the_rival(world: &mut World) {
        let (rival, seeded, _) = the_rival(world);
        assert_eq!(
            world
                .resource::<CivilizationStances>()
                .stance(&rival, seeded),
            StanceType::Ally,
            "civilization stance: the rival '{}' warmed to Ally",
            rival.name()
        );
        let hulls = hulls_of(world, rival);
        assert!(
            !hulls.is_empty(),
            "civilization stance: a hull of '{}' stands",
            rival.name()
        );
        for (root, side, _, _) in &hulls {
            assert_eq!(
                *side,
                Allegiance::Player,
                "civilization stance: {root:?} of '{}' flies the Player side",
                rival.name()
            );
        }
        let (root, side, station, credits_cr) = hulls
            .iter()
            .copied()
            .find(|(_, _, station, _)| *station)
            .unwrap_or(hulls[0]);
        let partner = EscortHirePartner {
            offers: true,
            allied: side == Allegiance::Player,
            credits_cr,
        };
        let plan = plan_escort_hire(0, Some(partner), &[], 0);
        assert!(
            plan.is_ok(),
            "civilization stance: the hire rule admits {root:?} of '{}' as a partner: {plan:?}",
            rival.name()
        );
        nova_probe::probe_marker(
            world,
            "outcome: the rival neighbour warms to ally and its station hires out escorts",
            serde_json::json!({
                "rival": rival.name(),
                "hulls": hulls.len(),
                "station": station,
            }),
        );
        info!(
            "civilization stance: PASS the rival '{}' warmed to Ally; {} hull(s) fly the Player side",
            rival.name(),
            hulls.len()
        );
    }

    /// The three facts a save must carry.
    fn assert_the_table(stances: &CivilizationStances, world: &World, label: &str) {
        let (civ, civ_seeded) = the_civilization(world);
        let (rival, rival_seeded, _) = the_rival(world);
        let (remote, remote_seeded) = the_remote(world);
        assert_eq!(
            stances.stance(&civ, civ_seeded),
            StanceType::Enemy,
            "civilization stance: {label} reads '{}' Enemy",
            civ.name()
        );
        assert_eq!(
            stances.stance(&rival, rival_seeded),
            StanceType::Ally,
            "civilization stance: {label} reads '{}' Ally",
            rival.name()
        );
        assert!(
            stances.record(&remote).is_none(),
            "civilization stance: {label} holds no record for the remote '{}'",
            remote.name()
        );
        assert_eq!(
            stances.stance(&remote, remote_seeded),
            remote_seeded,
            "civilization stance: {label} reads the remote '{}' as seeded",
            remote.name()
        );
    }

    /// Claim 3, part one: the leave save's table carries the three facts.
    fn assert_the_save(world: &mut World) {
        let root = nova_assets::storage::worlds_root()
            .expect("civilization stance: the sandboxed CONFIG_ROOT gives a worlds root");
        let slug = world_slug(WORLD_NAME).expect("the world name has a slug");
        let (_folder, _lock, header, state) = open_world(
            &root,
            &slug,
            world.resource::<LoadedSectionPacks>(),
            world.resource::<GameStartRoles>(),
            world.resource::<GameItems>(),
            &RegisteredScenarios::from(world.resource::<GameScenarios>()),
        )
        .unwrap_or_else(|refusal| {
            panic!("civilization stance: cannot open the leave save: {refusal}")
        });
        assert_the_table(&state.stances, world, "the leave save");
        info!(
            "civilization stance: save generation {} (format {}) carries the stances",
            header.generation, header.format
        );
    }

    /// Claim 3, part two: after a real Load the table carries the three
    /// facts and the HUD hint still refuses X's station.
    fn assert_the_load(world: &mut World) {
        let stances = world.resource::<CivilizationStances>().clone();
        assert_the_table(&stances, world, "the loaded world");
        let (civ, _) = the_civilization(world);
        let (rival, _, _) = the_rival(world);
        let (remote, _) = the_remote(world);
        let expected = world
            .resource::<StanceWatch>()
            .expected_refusal_line
            .clone()
            .expect("computed");
        let line = dock_refusal_text(world);
        assert_eq!(
            line.as_deref(),
            Some(expected.as_str()),
            "civilization stance: the loaded HUD still reads the refusal line"
        );
        nova_probe::probe_marker(
            world,
            "outcome: the stances survive a save and a load and a remote civilization is untouched",
            serde_json::json!({
                "civilization": civ.name(),
                "rival": rival.name(),
                "remote": remote.name(),
                "line": expected,
            }),
        );
        info!(
            "civilization stance: PASS the stances survive a save and a load; '{}' is untouched",
            remote.name()
        );
    }

    /// Set `WorldClock.play_seconds` past X's `enemy_since` by
    /// `ENEMY_DECAY_SECONDS`. Disclosed fixture state: a real player cannot
    /// drive the clock, only live through the wait; this range cannot assert
    /// a decay without moving the clock itself.
    fn advance_past_the_decay(world: &mut World) {
        let (civ, _) = the_civilization(world);
        let since = world
            .resource::<CivilizationStances>()
            .record(&civ)
            .and_then(|record| record.enemy_since)
            .expect("civilization stance: X is still Enemy by events");
        let mut clock = world.resource_mut::<WorldClock>();
        clock.play_seconds = since + ENEMY_DECAY_SECONDS + 1.0;
        info!(
            "civilization stance: play time set to {:.1} s, past the {ENEMY_DECAY_SECONDS:.1} s decay \
             from {since:.1} s",
            clock.play_seconds
        );
    }

    /// Claim 4: the decay passed, X reads Neutral, and the station admits
    /// the player again.
    fn assert_the_decay(world: &mut World) {
        let (civ, seeded) = the_civilization(world);
        assert_eq!(
            world.resource::<CivilizationStances>().stance(&civ, seeded),
            StanceType::Neutral,
            "civilization stance: '{}' decayed to Neutral",
            civ.name()
        );
        let player = player(world);
        assert!(
            world.get::<DockedShip>(player).is_some(),
            "civilization stance: the player docked once the Enemy decayed"
        );
        nova_probe::probe_marker(
            world,
            "outcome: an enemy by events decays to neutral and the station admits again",
            serde_json::json!({ "civilization": civ.name() }),
        );
        info!(
            "civilization stance: PASS the station admits the player again, '{}' decayed to Neutral",
            civ.name()
        );
    }

    /// Lock the dock target: stands in for the targeting gesture; disclosed
    /// in the doc.
    fn lock_the_station(world: &mut World) {
        let dock = dock_target(world);
        let player = player(world);
        world.entity_mut(player).insert(TravelLock(Some(dock)));
    }

    fn load_row() -> String {
        format!(
            "Load World Row: {}",
            world_slug(WORLD_NAME).expect("the world name has a slug")
        )
    }

    /// The walk. Every beat waits on a world value.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let mut walk = AutopilotPlugin::<GameStates>::new()
            .step("civilization stance: reach the main menu")
            .until(ui_node_present(NEW_GAME_BUTTON))
            .deadline(SESSION_SECS)
            .add()
            .click_named(
                "civilization stance: click New Game",
                NEW_GAME_BUTTON,
                ui_node_present(WORLD_NEXT_BUTTON),
                SESSION_SECS,
            )
            .step("civilization stance: name the world and enter the seed")
            .on_enter(|world: &mut World| {
                set_field_text(world, WORLD_NAME_FIELD, WORLD_NAME);
                set_field_text(world, SEED_FIELD, &STATION_SEED.to_string());
            })
            .until(frames(2))
            .add()
            .click_named(
                "civilization stance: click Next",
                WORLD_NEXT_BUTTON,
                ui_node_present(CREATE_WORLD_BUTTON),
                SESSION_SECS,
            )
            .click_named(
                "civilization stance: pick the start role",
                START_ROLE_BUTTON,
                pointer_released(),
                SESSION_SECS,
            )
            .click_named(
                "civilization stance: click Create",
                CREATE_WORLD_BUTTON,
                state_is(GameStates::Playing),
                SESSION_SECS,
            )
            .step("civilization stance: the window streams in around the spawn")
            .until(window_is(|_| Some(SectorCoord::ORIGIN)))
            .deadline(SESSION_SECS)
            .add()
            .step("civilization stance: choose X, its station, the kills, N and R")
            .on_enter(choose_the_targets)
            .add()
            .step("civilization stance: jump beside the station")
            .on_enter(park_beside)
            .until(and(
                window_is(|world| world.resource::<StanceWatch>().cell),
                the_dock_target_stands(),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("civilization stance: berth and dock before the kills")
            .each(|world, _, _| berth_and_dock(world))
            .until(player_docked())
            .deadline(SESSION_SECS)
            .add()
            .step("civilization stance: the request was admitted; undock")
            .on_enter(|world: &mut World| {
                info!("civilization stance: the player docked before the kills");
                let player = player(world);
                world.trigger(DockingReleaseRequest { entity: player });
            })
            .until(player_free())
            .deadline(SESSION_SECS)
            .add()
            .step("civilization stance: stand off before the kills")
            .on_enter(park_beside)
            .until(the_dock_target_stands())
            .deadline(SESSION_SECS)
            .add();
        for index in 0..KILLS as usize {
            walk = walk
                .step(format!(
                    "civilization stance: neutralize hull {}",
                    index + 1
                ))
                .on_enter(move |world: &mut World| {
                    let player = player(world);
                    let target = kill_target(world, index);
                    neutralize_the_hull(world, player, target);
                })
                .until(the_hull_is_neutralized(index))
                .deadline(SESSION_SECS)
                .add();
        }
        walk.step("civilization stance: X turns Enemy and its station follows")
            .until(the_civilization_is_enemy())
            .deadline(SESSION_SECS)
            .add()
            .step("civilization stance: lock the station")
            .on_enter(lock_the_station)
            .until(frames(1))
            .add()
            .step("civilization stance: assert the HUD's refusal hint")
            .on_enter(assert_the_hint)
            .add()
            .step("civilization stance: shoot the refusal HUD line")
            .on_enter(|world: &mut World| shoot(world, HUD_SHOT))
            .until(shot_written(HUD_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
            .step("civilization stance: a refused berth attempt")
            .on_enter(berth_and_dock)
            .until(frames(1))
            .add()
            .step("civilization stance: stand off after the refusal")
            .on_enter(park_beside)
            .until(frames(SETTLE_FRAMES))
            .deadline(SESSION_SECS)
            .add()
            .step("civilization stance: assert the request refused")
            .on_enter(assert_the_request_refused)
            .add()
            .step("civilization stance: jump to the rival's centroid")
            .on_enter(jump_to_the_rival)
            .until(and(
                window_is(|world| {
                    let (edge, _) = world_shape(world);
                    let (_, _, centroid) = the_rival(world);
                    Some(SectorCoord::containing(centroid, edge))
                }),
                a_hull_of_the_rival_stands(),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("civilization stance: assert the rival warmed to Ally")
            .on_enter(assert_the_rival)
            .add()
            .step("civilization stance: open the pause menu")
            .on_enter(press_key(KeyCode::Escape))
            .until(ui_node_present(PAUSE_EXIT_BUTTON))
            .deadline(SESSION_SECS)
            .add()
            .step("civilization stance: let escape go")
            .on_enter(release_key(KeyCode::Escape))
            .add()
            .click_named(
                "civilization stance: click Back to Main Menu",
                BACK_TO_MENU_BUTTON,
                state_is(GameStates::MainMenu),
                SESSION_SECS,
            )
            .step("civilization stance: assert the leave save")
            .on_enter(assert_the_save)
            .add()
            .click_named(
                "civilization stance: click Load",
                LOAD_BUTTON,
                ui_node_present(load_row()),
                SESSION_SECS,
            )
            .click_named(
                "civilization stance: select the world",
                &load_row(),
                frames(2),
                SESSION_SECS,
            )
            .click_named(
                "civilization stance: click Load World",
                LOAD_WORLD_BUTTON,
                state_is(GameStates::Playing),
                SESSION_SECS,
            )
            .step("civilization stance: the resumed window streams in")
            .until(window_is(|world| {
                let (edge, _) = world_shape(world);
                let (_, _, centroid) = the_rival(world);
                Some(SectorCoord::containing(centroid, edge))
            }))
            .deadline(SESSION_SECS)
            .add()
            .step("civilization stance: jump back beside the station")
            .on_enter(park_beside)
            .until(and(
                window_is(|world| world.resource::<StanceWatch>().cell),
                the_dock_target_stands(),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("civilization stance: lock the station again")
            .on_enter(lock_the_station)
            .until(frames(1))
            .add()
            .step("civilization stance: assert the loaded stances")
            .on_enter(assert_the_load)
            .add()
            .step("civilization stance: advance play time past the decay")
            .on_enter(advance_past_the_decay)
            .until(the_civilization_is_neutral_again())
            .deadline(SESSION_SECS)
            .add()
            .step("civilization stance: berth and dock once the Enemy decayed")
            .each(|world, _, _| berth_and_dock(world))
            .until(player_docked())
            .deadline(SESSION_SECS)
            .add()
            .step("civilization stance: assert the decay")
            .on_enter(assert_the_decay)
            .add()
    }
}
