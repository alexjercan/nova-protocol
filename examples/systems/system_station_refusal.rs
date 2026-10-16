//! system_station_refusal: neutralizing a generated hull costs its
//! civilization every living station's dock, and the refusal outlives a
//! save, a Load, and only the price table's window forgives it.
//!
//! Boots the shipped app (via [`editor_app`]), starts New Game on a pinned
//! seed the way the Create World modal does, and lets the window stream in.
//! It scans the window for a civilization with a living, non-Enemy station
//! AND a second living, non-Enemy hull of its own - a separate ship when the
//! seed's window holds one, a second station otherwise - the nearer station
//! is the dock target, the other hull is the one the walk neutralizes. The
//! player docks at the station once to show the request is admitted before
//! anything changes, undocks, then damages the OTHER hull's critical
//! sections - its weapons if it is armed, else a station's flight computer or
//! an unarmed hull's thrusters - with the player's own ship as the damage
//! source, the way `system_job_neutralize` drives a kill. The civilization
//! then refuses the player's dock at the (untouched) station: the HUD's DOCK
//! hint carries the refusal line, and a real berth attempt leaves the player
//! undocked. The walk leaves to the main menu, reads the leave save back off
//! disk through [`open_world`], and loads the world again in this process
//! through the Load screen to show the refusal outlives both. Finally it sets
//! `WorldClock.play_seconds` past the price table's `robbery.refusal_seconds`
//! window (disclosed fixture state: a real player cannot drive the clock,
//! only live through the wait) and lets `forget_served_refusals` prune the
//! record, then berths again to show the station admits once more. The
//! other disclosed fixture state is the `TravelLock` the walk puts on the
//! player before reading the HUD's dock hint: it stands in for the targeting
//! gesture that locks a station, and touches no admission rule.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: a neutralized hull makes its civilization's station refuse docking` | the station admits the player before the neutralize; after it, `NeutralizedCivilizations` is stamped with the hull's civilization, the HUD's dock hint reads `<NAME> REFUSES DOCKING: HULL NEUTRALIZED`, and a real berth attempt leaves the player undocked |
//! | 2 | `outcome: the refusal survives a save and a load` | the leave save's `neutralized` record still refuses the civilization, and after a real Load the record and the HUD hint still refuse the dock |
//! | 3 | `outcome: the refusal ends when the window passes` | once `WorldClock.play_seconds` passes the stamp by the table's `refusal_seconds`, `forget_served_refusals` empties the record and the station admits the player again |
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 NOVA_AUTOPILOT_DEADLINE=3600 \
//!   cargo run --example system_station_refusal --features debug
//! # look for: `station refusal: PASS the station admits the player again`,
//! #           `autopilot: cycle complete, no panic`
//! ```
//! Add `NOVA_CAPTURE=1 NOVA_CAPTURE_DIR=<dir>` to shoot the refusal HUD line
//! as `station-refusal-hud.png`.

use bevy::prelude::*;
use clap::Parser;

#[derive(Parser)]
#[command(name = "system_station_refusal")]
#[command(version = "1.0.0")]
#[command(
    about = "New Game on a pinned seed: neutralize a generated hull with the player as the \
             source, show its civilization's station refuses the dock, check the refusal \
             through a save and a Load, then let the price table's window end it. \
             Autopilot-only correctness range",
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
        // No frame-time claim: the range holds a record, a HUD line and a
        // docked flag.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(walk::RefusalWatchPlugin);
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
    /// spawn's cell. Unverified here: that the window also holds a second
    /// living, non-Enemy hull of that station's civilization - see the
    /// module doc and the report this range shipped with.
    const STATION_SEED: u32 = 10;

    const NEW_GAME_BUTTON: &str = "New Game Button";
    const WORLD_NAME_FIELD: &str = "World Name Field";
    const WORLD_NAME: &str = "Station Refusal World";
    const SEED_FIELD: &str = "World Seed Field";
    const CREATE_WORLD_BUTTON: &str = "Create World Button";
    const WORLD_NEXT_BUTTON: &str = "World Next Button";
    /// The start role the walk picks: the scavenger's salvage hauler, a
    /// compact box, clear of a berthed partner.
    const START_ROLE_BUTTON: &str = "Start Role Button: scavenger";
    const PAUSE_EXIT_BUTTON: &str = "Pause Exit Button";
    const BACK_TO_MENU_BUTTON: &str = "Back To Menu Button";
    const LOAD_BUTTON: &str = "Load Button";
    const LOAD_WORLD_BUTTON: &str = "Load World Button";

    /// The HUD shot the capture path writes.
    const HUD_SHOT: &str = "station-refusal-hud.png";

    /// How far from the station the player parks before the berth.
    const PARK_DISTANCE: Meters = Meters(2_000.0);
    /// The gap the berth leaves between the two port faces, engine units:
    /// half a cell, inside the 10 m capture distance.
    const BERTH_GAP: f32 = 0.5;
    /// Where a port's face sits in its own frame, as the docking verb reads it.
    const PORT_FACE: Vec3 = Vec3::new(0.0, 0.0, -0.5);

    /// Seconds a load, a stream or a beat gets on a software-rendered GPU.
    const SESSION_SECS: f32 = 240.0;
    /// Frames a refused berth attempt gets before the walk calls it settled.
    const SETTLE_FRAMES: u32 = 10;
    /// The virtual clock's clamp on the software rasterizer.
    const WALK_CLOCK_SECS: f32 = 1.0;

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
    struct RefusalWatch {
        /// The civilization the walk neutralizes a hull of.
        civilization: Option<CivilizationId>,
        /// The station the player docks at, before and after.
        dock_id: Option<GeneratedBodyId>,
        /// The hull the walk damages to neutralize.
        neutralize_id: Option<GeneratedBodyId>,
        /// The cell the dock target stands in.
        cell: Option<SectorCoord>,
        /// Where it stood when chosen, for the park point.
        dock_position: Option<Meters3>,
        /// The HUD line the dock hint must read once refused.
        expected_refusal_line: Option<String>,
    }

    pub(super) struct RefusalWatchPlugin;

    impl Plugin for RefusalWatchPlugin {
        fn build(&self, app: &mut App) {
            app.init_resource::<RefusalWatch>();
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
        the_player(world).expect("station refusal: exactly one player ship")
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
            .resource::<RefusalWatch>()
            .dock_id
            .clone()
            .expect("station refusal: the dock target is chosen");
        body_root(world, &id).expect("station refusal: the dock target stands")
    }

    fn neutralize_target(world: &World) -> Entity {
        let id = world
            .resource::<RefusalWatch>()
            .neutralize_id
            .clone()
            .expect("station refusal: the neutralize target is chosen");
        body_root(world, &id).expect("station refusal: the neutralize target stands")
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
                .resource::<RefusalWatch>()
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

    /// The chosen hull carries `NeutralizedMarker`.
    fn the_hull_is_neutralized() -> Arc<Predicate> {
        when(|world| {
            world
                .resource::<RefusalWatch>()
                .neutralize_id
                .as_ref()
                .and_then(|id| body_root(world, id))
                .is_some_and(|target| world.get::<NeutralizedMarker>(target).is_some())
        })
    }

    /// The chosen civilization is stamped in the player's record.
    fn the_civilization_is_recorded() -> Arc<Predicate> {
        when(|world| {
            let civ = world
                .resource::<RefusalWatch>()
                .civilization
                .expect("station refusal: the civilization is chosen");
            world.resource::<NeutralizedCivilizations>().refuses(&civ)
        })
    }

    /// The window's refusal has run its course: the record no longer refuses
    /// the chosen civilization.
    fn the_window_has_passed() -> Arc<Predicate> {
        when(|world| {
            let civ = world
                .resource::<RefusalWatch>()
                .civilization
                .expect("station refusal: the civilization is chosen");
            !world.resource::<NeutralizedCivilizations>().refuses(&civ)
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
        info!("station refusal: the player jumps to {position:?}");
    }

    /// Where the player parks: [`PARK_DISTANCE`] from the dock target toward
    /// the spawn.
    fn park_beside(world: &mut World) {
        let position = world
            .resource::<RefusalWatch>()
            .dock_position
            .expect("station refusal: the dock target is chosen");
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
                .expect("station refusal: a mounted section has a transform");
            position = transform.translation + transform.rotation * position;
            rotation = transform.rotation * rotation;
            at = world
                .get::<ChildOf>(at)
                .expect("station refusal: a section's chain reaches its root")
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
            .unwrap_or_else(|| panic!("station refusal: {root:?} has no free docking port"))
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
            .unwrap_or_else(|| panic!("station refusal: {target:?} has no free docking port"))
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
            .expect("station refusal: the dock target stands")
            .0;
        let rotation = world
            .get::<Rotation>(station)
            .expect("station refusal: the dock target stands")
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
            .unwrap_or_else(|| panic!("station refusal: no field named '{name}'"));
        value.0 = text.to_string();
    }

    /// Every living, non-Enemy, non-derelict, non-neutralized generated hull
    /// in the window, with the civilization and allegiance facts the choice
    /// needs.
    fn candidates(world: &mut World) -> Vec<Candidate> {
        world
            .query_filtered::<(
                &GeneratedBodyId,
                &Position,
                &TraderMarket,
                Has<StationMarker>,
                Has<DerelictShipMarker>,
                Has<NeutralizedMarker>,
                Option<&Allegiance>,
                Option<&Name>,
            ), With<SpaceshipRootMarker>>()
            .iter(world)
            .filter(|(_, _, _, _, derelict, neutralized, allegiance, _)| {
                !*derelict && !*neutralized && *allegiance != Some(&Allegiance::Enemy)
            })
            .map(|(id, position, market, station, _, _, _, name)| Candidate {
                id: id.clone(),
                position: Meters3::from_engine(position.0),
                civilization: market.civilization,
                station,
                name: name.map_or_else(|| format!("{id:?}"), ToString::to_string),
            })
            .collect()
    }

    /// Pick a civilization with a living, non-Enemy station AND a second
    /// living, non-Enemy hull of its own: the nearest such station is the
    /// dock target, the nearest OTHER hull of its civilization is the one the
    /// walk neutralizes. Ties toward the nearest dock target.
    fn choose_the_pair(world: &mut World) {
        let mut groups: BTreeMap<CivilizationId, Vec<Candidate>> = BTreeMap::new();
        for body in candidates(world) {
            groups.entry(body.civilization).or_default().push(body);
        }
        let mut picked: Option<(CivilizationId, Candidate, Candidate)> = None;
        for (civ, members) in groups {
            let Some(dock) = members.iter().find(|m| m.station).cloned() else {
                continue;
            };
            let Some(neutralize) = members
                .iter()
                .filter(|m| m.id != dock.id)
                .min_by(|a, b| {
                    a.position
                        .distance(Meters3::ZERO)
                        .get()
                        .total_cmp(&b.position.distance(Meters3::ZERO).get())
                })
                .cloned()
            else {
                continue;
            };
            let distance = dock.position.distance(Meters3::ZERO).get();
            let better = match &picked {
                None => true,
                Some((_, best_dock, _)) => {
                    distance < best_dock.position.distance(Meters3::ZERO).get()
                }
            };
            if better {
                picked = Some((civ, dock, neutralize));
            }
        }
        let (civ, dock, neutralize) = picked.unwrap_or_else(|| {
            panic!(
                "station refusal: the window on seed {STATION_SEED} holds no civilization with \
                 a living, non-Enemy station and a second living, non-Enemy hull of its own"
            )
        });
        let (edge, _) = world_shape(world);
        let cell = SectorCoord::containing(dock.position, edge);
        let expected_refusal_line = format!("{} REFUSES DOCKING: HULL NEUTRALIZED", dock.name);
        info!(
            "station refusal: chose civilization '{}' ({civ}); station '{}' to dock at, '{}' to \
             neutralize ({})",
            civ.name(),
            dock.name,
            neutralize.name,
            if neutralize.station {
                "a second station"
            } else {
                "a separate hull"
            }
        );
        let mut watch = world.resource_mut::<RefusalWatch>();
        watch.civilization = Some(civ);
        watch.dock_id = Some(dock.id);
        watch.neutralize_id = Some(neutralize.id);
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
                "station refusal: '{target:?}' is armed, zeroing {} weapon section(s)",
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
            "station refusal: the unarmed neutralize target carries no {}",
            if station {
                "controller section"
            } else {
                "thruster section"
            }
        );
        info!(
            "station refusal: '{target:?}' is unarmed, zeroing {} {} section(s)",
            critical.len(),
            if station { "controller" } else { "thruster" }
        );
        for section in critical {
            zero_section(world, player, section);
        }
    }

    /// Claim 1, part one: the record is stamped and the HUD's dock hint reads
    /// the neutralized-hull refusal line.
    fn assert_the_hint(world: &mut World) {
        let watch = world.resource::<RefusalWatch>();
        let civ = watch.civilization.expect("chosen");
        let expected = watch.expected_refusal_line.clone().expect("computed");
        assert!(
            world.resource::<NeutralizedCivilizations>().refuses(&civ),
            "station refusal: '{}' is stamped neutralized",
            civ.name()
        );
        let line = dock_refusal_text(world);
        assert_eq!(
            line.as_deref(),
            Some(expected.as_str()),
            "station refusal: the HUD's dock hint reads the neutralized-hull refusal line"
        );
        info!("station refusal: the HUD reads '{expected}'");
    }

    /// Claim 1, part two: a real berth attempt leaves the player undocked.
    fn assert_the_request_refused(world: &mut World) {
        let watch = world.resource::<RefusalWatch>();
        let civ = watch.civilization.expect("chosen");
        let expected = watch.expected_refusal_line.clone().expect("computed");
        let player = player(world);
        assert!(
            world.get::<DockedShip>(player).is_none(),
            "station refusal: the refused request left the player undocked"
        );
        nova_probe::probe_marker(
            world,
            "outcome: a neutralized hull makes its civilization's station refuse docking",
            serde_json::json!({
                "civilization": civ.name(),
                "line": expected,
            }),
        );
        info!(
            "station refusal: PASS a neutralized hull makes '{}' refuse docking ({expected})",
            civ.name()
        );
    }

    /// Claim 2, part one: the leave save's record still refuses the chosen
    /// civilization.
    fn assert_the_save(world: &mut World) {
        let civ = world
            .resource::<RefusalWatch>()
            .civilization
            .expect("chosen");
        let root = nova_assets::storage::worlds_root()
            .expect("station refusal: the sandboxed CONFIG_ROOT gives a worlds root");
        let slug = world_slug(WORLD_NAME).expect("the world name has a slug");
        let (_folder, _lock, header, state) = open_world(
            &root,
            &slug,
            world.resource::<LoadedSectionPacks>(),
            world.resource::<GameStartRoles>(),
            world.resource::<GameItems>(),
            &RegisteredScenarios::from(world.resource::<GameScenarios>()),
        )
        .unwrap_or_else(|refusal| panic!("station refusal: cannot open the leave save: {refusal}"));
        assert!(
            state.neutralized.refuses(&civ),
            "station refusal: the saved record still refuses '{}'",
            civ.name()
        );
        info!(
            "station refusal: save generation {} (format {}) still refuses '{}'",
            header.generation,
            header.format,
            civ.name()
        );
    }

    /// Claim 2, part two: after a real Load the record and the HUD hint
    /// still refuse the dock.
    fn assert_the_load(world: &mut World) {
        let watch = world.resource::<RefusalWatch>();
        let civ = watch.civilization.expect("chosen");
        let expected = watch.expected_refusal_line.clone().expect("computed");
        assert!(
            world.resource::<NeutralizedCivilizations>().refuses(&civ),
            "station refusal: the loaded record still refuses '{}'",
            civ.name()
        );
        let line = dock_refusal_text(world);
        assert_eq!(
            line.as_deref(),
            Some(expected.as_str()),
            "station refusal: the loaded HUD still reads the neutralized-hull refusal line"
        );
        nova_probe::probe_marker(
            world,
            "outcome: the refusal survives a save and a load",
            serde_json::json!({
                "civilization": civ.name(),
                "line": expected,
            }),
        );
        info!(
            "station refusal: PASS the refusal survives a save and a load ('{}')",
            civ.name()
        );
    }

    /// Set `WorldClock.play_seconds` past the stamp by the table's
    /// `refusal_seconds`. Disclosed fixture state: a real player cannot drive
    /// the clock, only live through the wait; this range cannot assert a
    /// window ending without moving the clock itself.
    fn advance_past_the_window(world: &mut World) {
        let civ = world
            .resource::<RefusalWatch>()
            .civilization
            .expect("chosen");
        let stamp = world
            .resource::<NeutralizedCivilizations>()
            .entries()
            .find(|(id, _)| **id == civ)
            .map(|(_, at)| at)
            .expect("station refusal: the civilization is still stamped");
        let refusal_seconds = world.resource::<GamePriceTable>().0.robbery.refusal_seconds;
        let mut clock = world.resource_mut::<WorldClock>();
        clock.play_seconds = stamp + f64::from(refusal_seconds) + 1.0;
        info!(
            "station refusal: play time set to {:.1} s, past the {:.1} s window stamped at {:.1} s",
            clock.play_seconds, refusal_seconds, stamp
        );
    }

    /// Claim 3: the window passed, the record forgot the civilization, and
    /// the station admits the player again.
    fn assert_the_window_passed(world: &mut World) {
        let civ = world
            .resource::<RefusalWatch>()
            .civilization
            .expect("chosen");
        assert!(
            !world.resource::<NeutralizedCivilizations>().refuses(&civ),
            "station refusal: the record forgot '{}' once the window passed",
            civ.name()
        );
        let player = player(world);
        assert!(
            world.get::<DockedShip>(player).is_some(),
            "station refusal: the player docked once the window passed"
        );
        nova_probe::probe_marker(
            world,
            "outcome: the refusal ends when the window passes",
            serde_json::json!({ "civilization": civ.name() }),
        );
        info!(
            "station refusal: PASS the station admits the player again, the window passed for \
             '{}'",
            civ.name()
        );
    }

    /// The walk. Every beat waits on a world value.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        AutopilotPlugin::<GameStates>::new()
            .step("station refusal: reach the main menu")
            .until(ui_node_present(NEW_GAME_BUTTON))
            .deadline(SESSION_SECS)
            .add()
            .click_named(
                "station refusal: click New Game",
                NEW_GAME_BUTTON,
                ui_node_present(WORLD_NEXT_BUTTON),
                SESSION_SECS,
            )
            .step("station refusal: name the world and enter the seed")
            .on_enter(|world: &mut World| {
                set_field_text(world, WORLD_NAME_FIELD, WORLD_NAME);
                set_field_text(world, SEED_FIELD, &STATION_SEED.to_string());
            })
            .until(frames(2))
            .add()
            .click_named(
                "station refusal: click Next",
                WORLD_NEXT_BUTTON,
                ui_node_present(CREATE_WORLD_BUTTON),
                SESSION_SECS,
            )
            .click_named(
                "station refusal: pick the start role",
                START_ROLE_BUTTON,
                pointer_released(),
                SESSION_SECS,
            )
            .click_named(
                "station refusal: click Create",
                CREATE_WORLD_BUTTON,
                state_is(GameStates::Playing),
                SESSION_SECS,
            )
            .step("station refusal: the window streams in around the spawn")
            .until(window_is(|_| Some(SectorCoord::ORIGIN)))
            .deadline(SESSION_SECS)
            .add()
            .step("station refusal: choose the station and the hull to neutralize")
            .on_enter(choose_the_pair)
            .add()
            .step("station refusal: jump beside the station")
            .on_enter(park_beside)
            .until(and(
                window_is(|world| world.resource::<RefusalWatch>().cell),
                the_dock_target_stands(),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("station refusal: berth and dock before the neutralize")
            .each(|world, _, _| berth_and_dock(world))
            .until(player_docked())
            .deadline(SESSION_SECS)
            .add()
            .step("station refusal: the request was admitted; undock")
            .on_enter(|world: &mut World| {
                info!("station refusal: the player docked before the neutralize");
                let player = player(world);
                world.trigger(DockingReleaseRequest { entity: player });
            })
            .until(player_free())
            .deadline(SESSION_SECS)
            .add()
            .step("station refusal: neutralize the other hull")
            .on_enter(|world: &mut World| {
                let player = player(world);
                let target = neutralize_target(world);
                neutralize_the_hull(world, player, target);
            })
            .until(the_hull_is_neutralized())
            .deadline(SESSION_SECS)
            .add()
            .step("station refusal: the civilization is recorded")
            .until(the_civilization_is_recorded())
            .deadline(SESSION_SECS)
            .add()
            .step("station refusal: lock the station")
            .on_enter(|world: &mut World| {
                let dock = dock_target(world);
                let player = player(world);
                // Stands in for the targeting gesture; disclosed in the doc.
                world.entity_mut(player).insert(TravelLock(Some(dock)));
            })
            .until(frames(1))
            .add()
            .step("station refusal: assert the HUD's refusal hint")
            .on_enter(assert_the_hint)
            .add()
            .step("station refusal: shoot the refusal HUD line")
            .on_enter(|world: &mut World| shoot(world, HUD_SHOT))
            .until(shot_written(HUD_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
            .step("station refusal: a refused berth attempt")
            .on_enter(berth_and_dock)
            .until(frames(SETTLE_FRAMES))
            .deadline(SESSION_SECS)
            .add()
            .step("station refusal: assert the request refused")
            .on_enter(assert_the_request_refused)
            .add()
            .step("station refusal: open the pause menu")
            .on_enter(press_key(KeyCode::Escape))
            .until(ui_node_present(PAUSE_EXIT_BUTTON))
            .deadline(SESSION_SECS)
            .add()
            .step("station refusal: let escape go")
            .on_enter(release_key(KeyCode::Escape))
            .add()
            .click_named(
                "station refusal: click Back to Main Menu",
                BACK_TO_MENU_BUTTON,
                state_is(GameStates::MainMenu),
                SESSION_SECS,
            )
            .step("station refusal: assert the leave save")
            .on_enter(assert_the_save)
            .add()
            .click_named(
                "station refusal: click Load",
                LOAD_BUTTON,
                ui_node_present(format!(
                    "Load World Row: {}",
                    world_slug(WORLD_NAME).expect("the world name has a slug")
                )),
                SESSION_SECS,
            )
            .click_named(
                "station refusal: select the world",
                &format!(
                    "Load World Row: {}",
                    world_slug(WORLD_NAME).expect("the world name has a slug")
                ),
                frames(2),
                SESSION_SECS,
            )
            .click_named(
                "station refusal: click Load World",
                LOAD_WORLD_BUTTON,
                state_is(GameStates::Playing),
                SESSION_SECS,
            )
            .step("station refusal: the loaded window streams the station back")
            .until(and(
                window_is(|world| world.resource::<RefusalWatch>().cell),
                the_dock_target_stands(),
            ))
            .deadline(SESSION_SECS)
            .add()
            .step("station refusal: lock the station again")
            .on_enter(|world: &mut World| {
                let dock = dock_target(world);
                let player = player(world);
                // Stands in for the targeting gesture; disclosed in the doc.
                world.entity_mut(player).insert(TravelLock(Some(dock)));
            })
            .until(frames(1))
            .add()
            .step("station refusal: assert the loaded refusal")
            .on_enter(assert_the_load)
            .add()
            .step("station refusal: advance play time past the refusal window")
            .on_enter(advance_past_the_window)
            .until(the_window_has_passed())
            .deadline(SESSION_SECS)
            .add()
            .step("station refusal: berth and dock once the window passed")
            .each(|world, _, _| berth_and_dock(world))
            .until(player_docked())
            .deadline(SESSION_SECS)
            .add()
            .step("station refusal: assert the window passed")
            .on_enter(assert_the_window_passed)
            .add()
    }
}
