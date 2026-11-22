//! waypoint_goto_sandbox: fly a chart GOTO to a station the window does not
//! hold, watch the waypoint chip, and see the leg hand over to the station
//! when it streams in. Then fly a chart GOTO to a cell.
//!
//! Boots the shipped app and starts New Game on a pinned seed as the bounty
//! hunter, as the Create World modal does but without a save folder. When
//! the start's window has streamed in and the start has docked the player,
//! the player lets go and JUMPS 128 km (four cells) east of the starting
//! station, at rest, bow east. The jump is disclosed fixture state: it puts
//! the starting station out of the window, so only its frozen record says
//! where it is.
//!
//! What to do:
//!
//! 1. press Tab, open the Map and press C for the chart. Click `STN-1`, the
//!    starting station, then GOTO (or G). The panel reads `GOTO STN-1` and
//!    the range under its stock list, and the station's label wears the
//!    `GOTO` tag;
//! 2. close Tab. The nav chip reads `STN-1` and the range. Turn away from it:
//!    it clamps to the screen edge with its chevron. The holo ribbon shows
//!    the predicted path once the forecast lands, and nothing before;
//! 3. fly (or wait). When the station's cell streams in, the leg becomes a
//!    GOTO of the live station with the travel lock on it;
//! 4. in the chart, click a visited cell's coordinate, then GOTO: the leg
//!    flies to the cell's centre. A wreck or a job mark has no GOTO.
//!
//! The diagnostics layer (F3) is on at start: in flight, the waypoint's code,
//! the range to its mark and the order the autopilot flies. It writes
//! nothing while a surface is up, so it covers no part of the Map.
//!
//! ```text
//! nix develop --command cargo run --example waypoint_goto_sandbox --features debug
//! ```
//!
//! No deadline ends the run unless `NOVA_AUTOPILOT=1` is set. Then a smoke
//! walk waits for the staging, checks that the GOTO and Cancel row stands
//! inside the panel and the window under the station's stock list, clicks
//! GOTO with the harness pointer, checks the waypoint, the tag and the chip
//! on and off screen, jumps the
//! ship 20 km short of the mark (disclosed) and checks the hand-off, then
//! clicks a chart GOTO on the cell the ship jumped to and checks its mark.
//! `NOVA_CAPTURE=1` shoots the chart with the tag and the chip on and off
//! screen.

#[path = "shared/open_world_stage.rs"]
mod open_world_stage;
#[path = "shared/station_void.rs"]
mod station_void;

use avian3d::prelude::Position;
use bevy::prelude::*;
use diagnostics::{DiagnosticsPanel, DiagnosticsSystems};
use nova_protocol::{nova_debug::diagnostics, prelude::*};
use open_world_stage::OpenWorldStartPlugin;

/// The world seed and the start role.
const SANDBOX_SEED: u32 = 5;
const SANDBOX_ROLE: &str = "bounty_hunter";
/// How far east of the starting station the ship jumps: four cells, past
/// the two-cell window.
const JUMP_DISTANCE: Meters = Meters(128_000.0);
/// Which way the ship jumps, and its bow after the jump.
const JUMP_AXIS: Vec3 = Vec3::X;

/// The intro line.
const INTRO: &str = "Tab, Map, C: click STN-1, then GOTO. Close Tab and follow the chip.";

fn main() -> bevy::app::AppExit {
    open_world_stage::isolate_the_profile(env!("CARGO_CRATE_NAME"));

    let mut app = nova_protocol::prelude::editor_app(true, None);
    app.add_plugins(OpenWorldStartPlugin {
        seed: SANDBOX_SEED,
        role: SANDBOX_ROLE.to_string(),
    });
    app.init_resource::<Sandbox>();
    app.add_plugins(diagnostics::DiagnosticsHintPlugin::new(INTRO));
    app.add_systems(
        Update,
        drive_the_sandbox.run_if(in_state(GameStates::Playing)),
    );
    app.add_systems(
        PostUpdate,
        draw_the_waypoint
            .in_set(DiagnosticsSystems)
            .after(TransformSystems::Propagate)
            .run_if(in_state(GameStates::Playing)),
    );

    if std::env::var_os("NOVA_AUTOPILOT").is_some() {
        app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
            bevy::ecs::error::panic,
        ));
        app.add_systems(Update, open_world_stage::hold_the_walk_clock);
        app.add_systems(Startup, force_capture_resolution);
    }
    app.add_plugins(smoke::script());

    app.run()
}

/// Where the staging stands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum SandboxPhaseType {
    /// The start's window is streaming in and the start is docking the
    /// player at its starting station.
    #[default]
    Streaming,
    /// The player let go of the starting station.
    Leaving,
    /// The player jumped [`JUMP_DISTANCE`] off; its window is streaming in.
    Away,
    /// The starting station is frozen out of the window. The player has the
    /// run.
    Ready,
}

/// The staging state.
#[derive(Resource, Default, Debug)]
struct Sandbox {
    phase: SandboxPhaseType,
    /// The starting station, once the player is docked to it.
    station: Option<GeneratedBodyId>,
    /// Frames of `Playing` so far.
    frame: u32,
    /// The frame the player was first docked, and the frame the chart first
    /// held the starting station.
    docked_at: Option<u32>,
    charted_at: Option<u32>,
}

/// Advance the staging one phase at a time.
fn drive_the_sandbox(world: &mut World) {
    world.resource_mut::<Sandbox>().frame += 1;
    let phase = world.resource::<Sandbox>().phase;
    let Some(player) = open_world_stage::the_player(world) else {
        return;
    };
    let docked = world.get::<DockedShip>(player).is_some();
    let next = match phase {
        SandboxPhaseType::Ready => return,
        SandboxPhaseType::Streaming => {
            note_the_start_station(world, docked);
            let charted = world.resource::<Sandbox>().charted_at.is_some();
            if !(open_world_stage::window_settled(world) && docked && charted) {
                return;
            }
            world.trigger(DockingReleaseRequest { entity: player });
            SandboxPhaseType::Leaving
        }
        SandboxPhaseType::Leaving => {
            if docked {
                return;
            }
            let Some(station) = world
                .resource::<Sandbox>()
                .station
                .clone()
                .and_then(|id| open_world_stage::body_by_id(world, &id))
            else {
                return;
            };
            let at = Meters3::from_engine(
                world
                    .get::<GlobalTransform>(station)
                    .expect("waypoint_goto_sandbox: the station has a pose")
                    .translation(),
            );
            open_world_stage::jump_facing(
                world,
                player,
                Meters3(at.get() + JUMP_AXIS * JUMP_DISTANCE.get()),
                JUMP_AXIS,
            );
            info!(
                "waypoint_goto_sandbox: jumped {} east of the starting station",
                nova_ui::units::distance(JUMP_DISTANCE)
            );
            SandboxPhaseType::Away
        }
        SandboxPhaseType::Away => {
            let id = world
                .resource::<Sandbox>()
                .station
                .clone()
                .expect("waypoint_goto_sandbox: the start station is recorded");
            if !open_world_stage::window_settled(world)
                || open_world_stage::body_by_id(world, &id).is_some()
                || station_mark(world, &id).is_err()
            {
                return;
            }
            info!("waypoint_goto_sandbox: the starting station '{id}' is frozen out of the window");
            SandboxPhaseType::Ready
        }
    };
    world.resource_mut::<Sandbox>().phase = next;
}

/// Record the starting station on the first docked frame, and the frames
/// the dock and the chart entry landed on.
fn note_the_start_station(world: &mut World, docked: bool) {
    let frame = world.resource::<Sandbox>().frame;
    if docked && world.resource::<Sandbox>().docked_at.is_none() {
        let id = station_void::docked_station(world)
            .and_then(|station| world.get::<GeneratedBodyId>(station).cloned())
            .expect("waypoint_goto_sandbox: the start docks the player at a generated station");
        let mut sandbox = world.resource_mut::<Sandbox>();
        sandbox.docked_at = Some(frame);
        sandbox.station = Some(id);
    }
    let Some(id) = world.resource::<Sandbox>().station.clone() else {
        return;
    };
    if world.resource::<Sandbox>().charted_at.is_none()
        && world.resource::<Chart>().station(&id).is_some()
    {
        let docked_at = world.resource::<Sandbox>().docked_at;
        info!(
            "waypoint_goto_sandbox: the chart holds the starting station '{id}' at Playing \
             frame {frame}; the dock landed at frame {docked_at:?}"
        );
        world.resource_mut::<Sandbox>().charted_at = Some(frame);
    }
}

/// The mark of the station `id`, as the chart GOTO resolves it.
fn station_mark(world: &World, id: &GeneratedBodyId) -> Result<Vec3, WaypointFault> {
    waypoint_mark(
        &WaypointTargetType::Station(id.clone()),
        world.resource::<Chart>(),
        world.resource::<nova_world::prelude::FrozenSectors>(),
        *world.resource::<ChartSectorEdge>(),
    )
}

/// The panel, in flight only: the waypoint's code, the range to its mark
/// and the order the autopilot flies.
fn draw_the_waypoint(world: &mut World) {
    if *world.resource::<State<PauseStates>>().get() != PauseStates::Unpaused {
        return;
    }
    let phase = world.resource::<Sandbox>().phase;
    if phase != SandboxPhaseType::Ready {
        world
            .resource_mut::<DiagnosticsPanel>()
            .line(format!("Staging the jump: {phase:?}"));
        return;
    }
    let Some(player) = open_world_stage::the_player(world) else {
        return;
    };
    let at = world
        .get::<GlobalTransform>(player)
        .map_or(Vec3::ZERO, GlobalTransform::translation);
    let waypoint = world.get::<Waypoint>(player).map(|waypoint| {
        let code = waypoint
            .target
            .chart_code(world.resource::<Chart>())
            .unwrap_or_else(|| "not charted".to_string());
        format!(
            "Waypoint {code}, mark {}",
            nova_ui::units::distance(Meters::from_engine(at.distance(waypoint.mark)))
        )
    });
    let order = match world
        .get::<Autopilot>(player)
        .map(|autopilot| autopilot.action)
    {
        Some(AutopilotAction::GotoPos { .. }) => "GotoPos",
        Some(AutopilotAction::Goto { .. }) => "Goto",
        Some(_) => "other",
        None => "none",
    };
    let mut panel = world.resource_mut::<DiagnosticsPanel>();
    panel.line(waypoint.unwrap_or_else(|| "No waypoint".to_string()));
    panel.line(format!("Autopilot {order}"));
    panel.hint("TAB", "Map, C, click STN-1, GOTO");
}

mod smoke {
    use std::sync::Arc;

    use nova_protocol::{
        nova_debug::harness::{AutopilotPlugin, Predicate, SHOT_DEADLINE_SECS},
        nova_interface::pane::InterfacePaneType,
    };

    use super::*;

    /// Seconds the world gets to start, stream, jump and stream again on a
    /// software GPU.
    const SETUP_SECS: f32 = 1200.0;
    /// Seconds any pane beat may stall before the walk fails it.
    const BEAT_SECS: f32 = 60.0;
    /// Seconds the station's window gets to stream in after the jump back.
    const HAND_OFF_SECS: f32 = 900.0;
    /// Frames a pane, a jump or a camera turn gets to settle.
    const SETTLE_FRAMES: u32 = 10;
    /// How far short of the mark the walk jumps the ship for the hand-off.
    const SHORT_OF_MARK: Meters = Meters(20_000.0);

    /// The frames the capture path writes.
    const CHART_SHOT: &str = "waypoint-goto-sandbox-chart.png";
    const CHIP_SHOT: &str = "waypoint-goto-sandbox-chip.png";
    const CLAMPED_SHOT: &str = "waypoint-goto-sandbox-chip-clamped.png";

    /// What the walk resolved before the GOTO.
    #[derive(Resource, Debug)]
    struct Expected {
        station: GeneratedBodyId,
        code: String,
        mark: Vec3,
        /// The cell the ship jumped to.
        cell: SectorCoord,
    }

    fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(check)
    }

    fn player(world: &World) -> Option<Entity> {
        world
            .try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()
            .and_then(|mut query| query.iter(world).next())
    }

    fn waypoint(world: &World) -> Option<Waypoint> {
        world.get::<Waypoint>(player(world)?).cloned()
    }

    fn action(world: &World) -> Option<AutopilotAction> {
        world
            .get::<Autopilot>(player(world)?)
            .map(|autopilot| autopilot.action)
    }

    fn expected(world: &World) -> &Expected {
        world.resource::<Expected>()
    }

    /// Activate the UI node named `name`, as a click on it would. A chart
    /// label may stand outside the framed view.
    fn activate_named(world: &mut World, name: &str) {
        let target = world
            .query::<(Entity, &Name)>()
            .iter(world)
            .find(|(_, found)| found.as_str() == name)
            .map(|(entity, _)| entity)
            .unwrap_or_else(|| panic!("waypoint_goto_sandbox: no node named '{name}'"));
        world.trigger(bevy::ui_widgets::Activate { entity: target });
    }

    fn named_present(world: &World, name: String) -> bool {
        world
            .try_query::<&Name>()
            .is_some_and(|mut query| query.iter(world).any(|found| found.as_str() == name))
    }

    fn a_text_starts(world: &World, prefix: &str) -> bool {
        world
            .try_query::<&Text>()
            .is_some_and(|mut query| query.iter(world).any(|text| text.0.starts_with(prefix)))
    }

    /// The waypoint chip as drawn: its anchor, whether its chevron shows
    /// (clamped), whether the pill shows, and its label.
    fn chip(world: &World) -> Option<(Option<ScreenIndicatorAnchorKind>, bool, bool, String)> {
        let layer = world
            .try_query_filtered::<&Children, With<WaypointChipHudMarker>>()?
            .iter(world)
            .next()?
            .to_vec();
        let node = *layer.first()?;
        let anchor = **world.get::<ScreenIndicatorAnchor>(node)?;
        let shown = world
            .get::<InheritedVisibility>(node)
            .is_some_and(|visibility| visibility.get());
        let mut clamped = false;
        let mut label = String::new();
        for child in world.get::<Children>(node)?.iter() {
            if world.get::<ScreenIndicatorArrowMarker>(child).is_some() {
                clamped = world.get::<Visibility>(child) != Some(&Visibility::Hidden);
            }
            if world.get::<AnchoredChipLabelMarker>(child).is_some() {
                label = world.get::<Text>(child)?.0.clone();
            }
        }
        Some((anchor, clamped, shown, label))
    }

    /// The starting station is out of the window, charted, and its mark is
    /// its frozen pose.
    fn assert_the_staging(world: &mut World) {
        let station = world
            .resource::<Sandbox>()
            .station
            .clone()
            .expect("waypoint_goto_sandbox: the start station is recorded");
        assert!(
            open_world_stage::body_by_id(world, &station).is_none(),
            "waypoint_goto_sandbox: the starting station is not live"
        );
        let code = world
            .resource::<Chart>()
            .station_code(&station)
            .expect("waypoint_goto_sandbox: the chart holds the starting station");
        let mark = station_mark(world, &station)
            .unwrap_or_else(|fault| panic!("waypoint_goto_sandbox: no mark: {fault}"));
        let player = open_world_stage::the_player(world).expect("waypoint_goto_sandbox: a player");
        let at = world.get::<Position>(player).unwrap().0;
        let cell = open_world_stage::cell_of(world, at).unwrap();
        info!(
            "waypoint_goto_sandbox: PASS {code} ('{station}') is frozen out of the window, its \
             mark {} off; the ship stands in {cell}",
            nova_ui::units::distance(Meters::from_engine(at.distance(mark)))
        );
        world.insert_resource(Expected {
            station,
            code,
            mark,
            cell,
        });
    }

    /// The chart GOTO set the station waypoint at its frozen pose and flies
    /// `GotoPos` there; the panel and the tag show it.
    fn assert_the_chart_goto(world: &mut World) {
        let Expected {
            station,
            code,
            mark,
            ..
        } = expected(world);
        let (station, code, mark) = (station.clone(), code.clone(), *mark);
        assert_eq!(
            waypoint(world),
            Some(Waypoint {
                target: WaypointTargetType::Station(station),
                mark,
            }),
            "waypoint_goto_sandbox: the waypoint names the station at its mark"
        );
        assert_eq!(
            action(world),
            Some(AutopilotAction::GotoPos { position: mark }),
            "waypoint_goto_sandbox: the autopilot flies the mark"
        );
        assert!(
            a_text_starts(world, &format!("GOTO {code}  ")),
            "waypoint_goto_sandbox: the panel reads the chart code"
        );
        let tag = world
            .query::<(&Name, &InheritedVisibility)>()
            .iter(world)
            .find(|(name, _)| name.as_str() == "MapGotoTag")
            .map(|(_, visibility)| visibility.get());
        assert_eq!(
            tag,
            Some(true),
            "waypoint_goto_sandbox: the GOTO tag is up on the chart"
        );
        info!("waypoint_goto_sandbox: PASS chart GOTO {code} flies GotoPos at its frozen pose");
    }

    /// The chip anchors the mark as a point, reads the code and the range,
    /// and is clamped with its chevron (`clamped`) or on screen without it.
    fn assert_the_chip(world: &mut World, clamped: bool) {
        let (code, mark) = (expected(world).code.clone(), expected(world).mark);
        let (anchor, chevron, shown, label) =
            chip(world).expect("waypoint_goto_sandbox: the waypoint chip stands");
        assert_eq!(anchor, Some(ScreenIndicatorAnchorKind::Point(mark)));
        assert!(shown, "waypoint_goto_sandbox: the chip shows");
        assert_eq!(chevron, clamped, "waypoint_goto_sandbox: the chevron");
        assert!(
            label.starts_with(&format!("{code}  ")) && label.ends_with(" km"),
            "waypoint_goto_sandbox: the chip reads '{label}'"
        );
        info!(
            "waypoint_goto_sandbox: PASS the chip reads '{label}', {}",
            if clamped {
                "clamped with its chevron"
            } else {
                "on screen"
            }
        );
    }

    /// The live station took over the leg: a `Goto` of it with the lock on
    /// it, the waypoint kept, the chip on the station.
    fn handed_off(world: &World) -> bool {
        let Some(player) = player(world) else {
            return false;
        };
        let Some(AutopilotAction::Goto { target }) = action(world) else {
            return false;
        };
        world.get::<GeneratedBodyId>(target) == Some(&expected(world).station)
            && world.get::<TravelLock>(player) == Some(&TravelLock(Some(target)))
            && waypoint(world).is_some()
    }

    fn assert_the_hand_off(world: &mut World) {
        let Some(AutopilotAction::Goto { target }) = action(world) else {
            panic!("waypoint_goto_sandbox: the leg is a GOTO");
        };
        assert!(handed_off(world));
        let (anchor, _, _, label) = chip(world).expect("waypoint_goto_sandbox: the chip stands");
        assert_eq!(
            anchor,
            Some(ScreenIndicatorAnchorKind::Entity(target)),
            "waypoint_goto_sandbox: the chip anchors the live station"
        );
        info!(
            "waypoint_goto_sandbox: PASS the live station took the leg: GOTO with the travel \
             lock, chip '{label}'"
        );
    }

    fn sector_goto_set(world: &World) -> bool {
        let cell = expected(world).cell;
        waypoint(world).is_some_and(|waypoint| waypoint.target == WaypointTargetType::Sector(cell))
    }

    fn assert_the_sector_goto(world: &mut World) {
        let cell = expected(world).cell;
        let centre = cell
            .centre(world.resource::<ChartSectorEdge>().0)
            .to_engine();
        assert_eq!(
            waypoint(world),
            Some(Waypoint {
                target: WaypointTargetType::Sector(cell),
                mark: centre,
            })
        );
        assert_eq!(
            action(world),
            Some(AutopilotAction::GotoPos { position: centre })
        );
        assert!(
            a_text_starts(world, &format!("GOTO {cell}  ")),
            "waypoint_goto_sandbox: the panel reads the cell"
        );
        info!("waypoint_goto_sandbox: PASS chart GOTO {cell} flies GotoPos at the cell centre");
    }

    /// Point the camera at `point`, or straight away from it.
    fn look(world: &mut World, toward: bool) {
        let (Some(player), Some(mark)) = (
            player(world),
            world
                .get_resource::<Expected>()
                .map(|expected| expected.mark),
        ) else {
            return;
        };
        let at = world.get::<GlobalTransform>(player).unwrap().translation();
        let point = if toward { mark } else { at - (mark - at) };
        aim_screen_centre_at(world, point);
    }

    fn open_the_chart(
        script: AutopilotPlugin<GameStates>,
        label: &'static str,
        mark: fn(&World) -> String,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("waypoint goto sandbox: open the Map for {label}"))
            .on_enter(|world: &mut World| {
                station_void::open_pane(world, InterfacePaneType::Map);
            })
            .until(and(ui_node_present("MapPanel"), frames(SETTLE_FRAMES)))
            .deadline(BEAT_SECS)
            .add()
            .step(format!("waypoint goto sandbox: press C for {label}"))
            .on_enter(press_action("map_chart"))
            .until(frames(1))
            .add()
            .step(format!("waypoint goto sandbox: let C up for {label}"))
            .on_enter(release_action("map_chart"))
            .until(when(move |world| {
                named_present(world, format!("MapChartMark{}", mark(world)))
            }))
            .deadline(BEAT_SECS)
            .add()
            .step(format!("waypoint goto sandbox: select {label}"))
            .on_enter(move |world: &mut World| {
                let name = format!("MapChartMark{}", mark(world));
                activate_named(world, &name);
            })
            .until(frames(SETTLE_FRAMES))
            .add()
    }

    /// The GOTO and Cancel row stands inside the panel and the window while
    /// the panel's body runs past its height.
    fn assert_the_verbs_in_view(world: &mut World, label: &str) {
        let window = world
            .query_filtered::<&Window, With<bevy::window::PrimaryWindow>>()
            .single(world)
            .map(|window| Rect::new(0.0, 0.0, window.width(), window.height()))
            .expect("waypoint_goto_sandbox: a primary window");
        let panel = ui_node_rect(world, "MapPanel").expect("waypoint_goto_sandbox: the Map panel");
        let overflow = world
            .query::<(&Name, &ComputedNode)>()
            .iter(world)
            .find(|(name, _)| name.as_str() == "MapPanelBody")
            .map(|(_, body)| (body.content_size().y - body.size().y) * body.inverse_scale_factor())
            .expect("waypoint_goto_sandbox: the panel body");
        for verb in ["MapGoto", "MapCancel"] {
            let rect = ui_node_rect(world, verb)
                .unwrap_or_else(|| panic!("waypoint_goto_sandbox: {verb} is not shown"));
            let inside = |outer: Rect| outer.contains(rect.min) && outer.contains(rect.max);
            assert!(
                inside(panel) && inside(window),
                "waypoint_goto_sandbox: {verb} {rect:?} stands outside the panel {panel:?} or \
                 the window {window:?}"
            );
        }
        info!(
            "waypoint_goto_sandbox: PASS GOTO and Cancel stand inside the panel and the {}x{} \
             window on {label}, the body {overflow:.0} px past its height",
            window.width(),
            window.height()
        );
    }

    fn click_goto(
        script: AutopilotPlugin<GameStates>,
        label: &'static str,
        landed: Arc<Predicate>,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!(
                "waypoint goto sandbox: check the GOTO row on {label}"
            ))
            .on_enter(move |world: &mut World| assert_the_verbs_in_view(world, label))
            .until(frames(1))
            .add()
            .click_named(
                &format!("waypoint goto sandbox: click GOTO on {label}"),
                "MapGoto",
                and(landed, frames(SETTLE_FRAMES)),
                BEAT_SECS,
            )
    }

    fn close_the_interface(world: &mut World) {
        world
            .resource_mut::<NextState<PauseStates>>()
            .set(PauseStates::Unpaused);
    }

    /// The smoke walk. Inert unless `NOVA_AUTOPILOT` is set.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let script = AutopilotPlugin::<GameStates>::new()
            .step("waypoint goto sandbox: start, jump and freeze the station")
            .until(when(|world| {
                world
                    .get_resource::<Sandbox>()
                    .is_some_and(|sandbox| sandbox.phase == SandboxPhaseType::Ready)
            }))
            .deadline(SETUP_SECS)
            .add()
            .step("waypoint goto sandbox: check the staging")
            .on_enter(assert_the_staging)
            .until(frames(SETTLE_FRAMES))
            .add();
        let script = open_the_chart(script, "the station", |world| expected(world).code.clone());
        let script = click_goto(
            script,
            "the station",
            when(|world| {
                waypoint(world).is_some_and(|waypoint| {
                    waypoint.target == WaypointTargetType::Station(expected(world).station.clone())
                })
            }),
        );
        let script = script
            .step("waypoint goto sandbox: check the chart GOTO")
            .on_enter(assert_the_chart_goto)
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("waypoint goto sandbox: shoot the chart with the tag")
            .on_enter(|world: &mut World| shoot(world, CHART_SHOT))
            .until(shot_written(CHART_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
            .step("waypoint goto sandbox: close the interface, look away from the mark")
            .on_enter(close_the_interface)
            .each(|world: &mut World, _, _| look(world, false))
            .until(and(
                when(|world| chip(world).is_some_and(|(_, clamped, shown, _)| clamped && shown)),
                frames(SETTLE_FRAMES),
            ))
            .deadline(BEAT_SECS)
            .add()
            .step("waypoint goto sandbox: check the clamped chip")
            .on_enter(|world: &mut World| assert_the_chip(world, true))
            .until(frames(1))
            .add()
            .step("waypoint goto sandbox: shoot the clamped chip")
            .on_enter(|world: &mut World| shoot(world, CLAMPED_SHOT))
            .each(|world: &mut World, _, _| look(world, false))
            .until(shot_written(CLAMPED_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
            .step("waypoint goto sandbox: look at the mark")
            .each(|world: &mut World, _, _| look(world, true))
            .until(and(
                when(|world| chip(world).is_some_and(|(_, clamped, shown, _)| !clamped && shown)),
                frames(SETTLE_FRAMES),
            ))
            .deadline(BEAT_SECS)
            .add()
            .step("waypoint goto sandbox: check the chip on screen")
            .on_enter(|world: &mut World| assert_the_chip(world, false))
            .until(frames(1))
            .add()
            .step("waypoint goto sandbox: shoot the chip on screen")
            .on_enter(|world: &mut World| shoot(world, CHIP_SHOT))
            .each(|world: &mut World, _, _| look(world, true))
            .until(shot_written(CHIP_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
            .step("waypoint goto sandbox: jump short of the mark and wait for the hand-off")
            .on_enter(|world: &mut World| {
                let mark = expected(world).mark;
                let player = player(world).expect("waypoint_goto_sandbox: a player");
                open_world_stage::jump_to(
                    world,
                    player,
                    Meters3::from_engine(mark + JUMP_AXIS * SHORT_OF_MARK.to_engine()),
                );
                info!(
                    "waypoint_goto_sandbox: jumped {} short of the mark",
                    nova_ui::units::distance(SHORT_OF_MARK)
                );
            })
            .until(when(handed_off))
            .deadline(HAND_OFF_SECS)
            .add()
            .step("waypoint goto sandbox: settle the hand-off")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("waypoint goto sandbox: check the hand-off")
            .on_enter(assert_the_hand_off)
            .until(frames(1))
            .add();
        let script = open_the_chart(script, "the cell", |world| expected(world).cell.to_string());
        click_goto(script, "the cell", when(sector_goto_set))
            .step("waypoint goto sandbox: check the sector GOTO")
            .on_enter(assert_the_sector_goto)
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("waypoint goto sandbox: close the interface")
            .on_enter(close_the_interface)
            .until(frames(SETTLE_FRAMES))
            .add()
    }
}
