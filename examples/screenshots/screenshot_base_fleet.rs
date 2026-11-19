//! screenshot_base_fleet: every shipped catalog ship, one framed still each.
//!
//! Spawns the base fleet in a row, inert and frozen, under the photo rig, and
//! frames each hull three-quarters from above its port bow, the flank every
//! crewed hull carries its docking collar on. One still per ship:
//! `fleet-<ship id>.png`. The shots are for reviewing a hull, so no manifest
//! image ships from here.
//!
//! - `NOVA_AUTOPILOT=1` alone: walk the framings and exit clean, capturing
//!   nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also capture each still (staged under
//!   `NOVA_CAPTURE_DIR`).
//! - `--looks`: shoot the looks set in place of the row. The six start hulls,
//!   four views each (`<ship id>-bow.png`, `-stern.png`, `-profile.png`,
//!   `-top.png`), on a flat grey backdrop, sized from the hull's own box. The
//!   hull turns and the camera stands still, so every view of every hull
//!   takes the rig's light from the same side.
//!
//! ```text
//! NOVA_CAPTURE_DIR=target/shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example screenshot_base_fleet --features debug
//! NOVA_CAPTURE_DIR=target/looks NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example screenshot_base_fleet --features debug -- --looks
//! ```

#[path = "shared/kit.rs"]
mod kit;

use bevy::{core_pipeline::Skybox, prelude::*};
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "screenshot_base_fleet")]
#[command(version = "1.0.0")]
#[command(about = "Every shipped catalog ship, framed one at a time for review. Autopilot-only: a posed row behind a scripted camera", long_about = None)]
struct Cli {
    /// Shoot the looks set: the six start hulls, four views each, on grey.
    #[arg(long)]
    looks: bool,
}

/// Which set this run shoots.
#[derive(Resource, Clone, Copy)]
struct Looks(bool);

/// The fleet, in catalog order, with the eye distance each hull needs: the
/// start hulls are about a hundred meters long, the line warship twice that.
const FLEET: [(&str, f32); 9] = [
    ("mining_tug", 1.0),
    ("salvage_hauler", 1.0),
    ("light_freighter", 1.0),
    ("interceptor", 0.9),
    ("raider", 0.9),
    ("patrol_picket", 0.8),
    ("line_warship", 2.0),
    ("freighter_derelict", 1.0),
    ("debris_plate", 0.6),
];

/// Space between two hulls in the row: wider than the longest hull, so no
/// framing catches a neighbour.
const SPACING: Meters = Meters(600.0);

/// The eye's offset from a hull at distance factor 1: above the port bow.
#[cfg(feature = "debug")]
const EYE: Meters3 = Meters3::new(-110.0, 55.0, -95.0);

fn slot(index: usize) -> Meters3 {
    Meters3::new(SPACING.0 * index as f32, 0.0, 0.0)
}

/// The looks set's hulls: the first six of [`FLEET`], the start hulls.
const LOOKS_HULLS: usize = 6;

/// One view of the looks set: the file suffix, where the viewer stands in the
/// hull's own frame (-Z is the bow, -X is port) and the hull axis that points
/// up the frame. The profile is the port flank with the bow to the left; the
/// plan keeps the bow to the left too.
const LOOKS_VIEWS: [(&str, Vec3, Vec3); 4] = [
    ("bow", Vec3::new(-1.0, 0.62, -0.9), Vec3::Y),
    ("stern", Vec3::new(1.0, 0.62, 0.9), Vec3::Y),
    ("profile", Vec3::new(-1.0, 0.0, 0.0), Vec3::Y),
    ("top", Vec3::new(0.0, 1.0, 0.0), Vec3::X),
];

/// Where the looks camera stands from every hull. Picked off the rig: the key
/// comes from `(-6, 5, 6)`, so from here it falls from the upper left, about
/// sixty degrees off the lens, the fill comes from the right and the rim from
/// behind the hull.
const LOOKS_BEARING: Vec3 = Vec3::new(0.25, 0.15, 1.0);

/// How far the cladding and its greebles stand out past the cell box, in
/// cells: the plate over a covered cell.
#[cfg(feature = "debug")]
const LOOKS_MARGIN: f32 = 0.4;

/// The share of the frame's width and of its height the padded hull box may
/// take. The corner that needs the most room sets the distance.
#[cfg(feature = "debug")]
const LOOKS_FILL: Vec2 = Vec2::new(0.94, 0.94);

/// The looks camera's vertical field, radians: 20 degrees. The game camera's
/// 45 degree lens stands so near a 50 m hull that the near corner sets the
/// frame and the hull fills under half of it; the long lens also keeps the
/// bow and the stern at one scale.
const LOOKS_FOV: f32 = 0.349;

/// The looks backdrop, linear: a neutral light grey, so a dark hull reads as
/// a silhouette. The skybox it replaces is near black.
const LOOKS_BACKDROP: Color = Color::linear_rgb(0.55, 0.55, 0.55);

/// The looks rig's ambient floor: a face no rig light reaches keeps its plate
/// lines. The game's scenes carry no ambient light.
const LOOKS_AMBIENT: f32 = 250.0;

/// The looks camera's axes in the world: right, up and back.
fn looks_axes() -> (Vec3, Vec3, Vec3) {
    let back = LOOKS_BEARING.normalize();
    let up = (Vec3::Y - back * back.y).normalize();
    (up.cross(back), up, back)
}

/// The rotation that shows a hull from `viewer`, with `up` up the frame, to
/// the camera standing on [`LOOKS_BEARING`].
fn looks_rotation(viewer: Vec3, up: Vec3) -> Quat {
    let back = viewer.normalize();
    let up = (up - back * up.dot(back)).normalize();
    let hull = Mat3::from_cols(up.cross(back), up, back);
    let (right, lift, bearing) = looks_axes();
    Quat::from_mat3(&(Mat3::from_cols(right, lift, bearing) * hull.transpose()))
}

/// The looks set's slot for view `view` of hull `hull`.
fn looks_slot(hull: usize, view: usize) -> usize {
    hull * LOOKS_VIEWS.len() + view
}

/// The camera's eye and aim for one looks view: on the bearing, aimed at the
/// middle of the turned hull, at the nearest distance that holds every
/// section's padded cell inside [`LOOKS_FILL`] of a 16:9 frame at
/// [`LOOKS_FOV`].
///
/// The fit reads the design's own cells, not a fixed offset or the hull's
/// outer box, so a reshaped hull is framed the same way and an empty box
/// corner asks for no room.
#[cfg(feature = "debug")]
fn looks_pose(world: &World, id: &str, at: Meters3, rotation: Quat) -> (Meters3, Meters3) {
    let design = kit::catalog_ship(world.resource::<GameShipDesigns>(), id);
    let (right, lift, bearing) = looks_axes();
    let pad = 0.5 + LOOKS_MARGIN;
    // Every padded cell corner on the camera's axes: right, up and toward
    // the lens.
    let corners: Vec<Vec3> = design
        .sections
        .iter()
        .flat_map(|section| {
            (0..8).map(move |corner| {
                let reach = Vec3::new(
                    if corner & 1 == 0 { -pad } else { pad },
                    if corner & 2 == 0 { -pad } else { pad },
                    if corner & 4 == 0 { -pad } else { pad },
                );
                let turned = rotation * (section.position + reach);
                Vec3::new(turned.dot(right), turned.dot(lift), turned.dot(bearing))
            })
        })
        .collect();
    let (min, max) = corners.iter().fold(
        (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
        |(min, max), corner| (min.min(*corner), max.max(*corner)),
    );
    let middle = (min + max) / 2.0;
    let half_tan = (LOOKS_FOV / 2.0).tan();
    let room = Vec2::new(
        LOOKS_FILL.x * half_tan * 16.0 / 9.0,
        LOOKS_FILL.y * half_tan,
    );
    // A corner nearer the lens draws larger, so each corner asks for its own
    // distance and the greediest one sets it.
    let distance = corners.iter().fold(0.0_f32, |distance, corner| {
        let from = *corner - middle;
        distance.max(from.z + (from.x.abs() / room.x).max(from.y.abs() / room.y))
    });
    // Engine boundary: the cells are build-grid cells, which are engine
    // units, so the fit is solved there and crosses to meters at the pose.
    let aim = at.to_engine() + right * middle.x + lift * middle.y + bearing * middle.z;
    let eye = aim + bearing * distance;
    (Meters3::from_engine(eye), Meters3::from_engine(aim))
}

/// Looks rig only: the long lens, a flat grey backdrop in place of the
/// skybox, and an ambient floor. Runs each frame, since the loader hangs the
/// skybox on the scenario camera some frames after the camera is up.
fn looks_rig(
    mut commands: Commands,
    mut cameras: Query<
        (Entity, &mut Camera, &mut Projection, Has<Skybox>),
        With<ScenarioCameraMarker>,
    >,
) {
    for (entity, mut camera, mut projection, skybox) in &mut cameras {
        if let Projection::Perspective(perspective) = projection.as_mut() {
            perspective.fov = LOOKS_FOV;
        }
        if skybox {
            camera.clear_color = ClearColorConfig::Custom(LOOKS_BACKDROP);
            commands
                .entity(entity)
                .remove::<Skybox>()
                .insert(AmbientLight {
                    brightness: LOOKS_AMBIENT,
                    ..default()
                });
        }
    }
}

fn main() -> bevy::app::AppExit {
    let looks = Cli::parse().looks;
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();
    app.insert_resource(Looks(looks));
    if looks {
        app.add_systems(Update, looks_rig);
    }

    #[cfg(feature = "debug")]
    {
        app.add_systems(
            Startup,
            (force_capture_resolution, hide_dev_overlays, hide_hud),
        );
        app.add_systems(Update, freeze_bodies);
        let mut script = nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
            .step("wait for the fleet row")
            .enter(GameStates::Loading)
            .until(and(
                state_is(GameStates::Playing),
                scenario_camera_present(),
            ))
            .deadline(STEP_DEADLINE_SECS)
            .add();
        let row = FLEET.into_iter().enumerate().map(|(index, (id, scale))| {
            let centre = slot(index);
            let frame: Box<dyn Fn(&mut World) + Send + Sync> = Box::new(move |world| {
                pose_camera(world, centre + EYE * scale, centre);
            });
            (format!("fleet-{id}.png"), frame)
        });
        let views = FLEET
            .into_iter()
            .take(LOOKS_HULLS)
            .enumerate()
            .flat_map(|(hull, (id, _))| {
                LOOKS_VIEWS
                    .into_iter()
                    .enumerate()
                    .map(move |(view, (name, viewer, up))| {
                        let at = slot(looks_slot(hull, view));
                        let rotation = looks_rotation(viewer, up);
                        let frame: Box<dyn Fn(&mut World) + Send + Sync> = Box::new(move |world| {
                            let (eye, aim) = looks_pose(world, id, at, rotation);
                            pose_camera(world, eye, aim);
                        });
                        (format!("{id}-{name}.png"), frame)
                    })
            });
        let shots: Vec<_> = if looks {
            views.collect()
        } else {
            row.collect()
        };
        for (shot, frame) in shots {
            script = script
                .step(format!("frame {shot}"))
                .on_enter(frame)
                .until(frames(SETTLE_FRAMES))
                .add();
            let name = shot.clone();
            script = script
                .step(format!("shoot {shot}"))
                .on_enter(move |world: &mut World| shoot(world, &name))
                .until(shot_written(&shot))
                .deadline(SHOT_DEADLINE_SECS)
                .add();
        }
        app.add_plugins(script);
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    ships: Res<GameShipDesigns>,
    looks: Res<Looks>,
) {
    // One stand per hull: object id, ship id, slot and rotation. The looks
    // set stands one copy of a start hull per view, each turned to its view.
    let stands: Vec<(String, &str, usize, Quat)> = if looks.0 {
        FLEET
            .iter()
            .take(LOOKS_HULLS)
            .enumerate()
            .flat_map(|(hull, (id, _))| {
                LOOKS_VIEWS
                    .iter()
                    .enumerate()
                    .map(move |(view, (name, viewer, up))| {
                        (
                            format!("looks_{id}_{name}"),
                            *id,
                            looks_slot(hull, view),
                            looks_rotation(*viewer, *up),
                        )
                    })
            })
            .collect()
    } else {
        FLEET
            .iter()
            .enumerate()
            .map(|(index, (id, _))| (format!("fleet_{id}"), *id, index, Quat::IDENTITY))
            .collect()
    };
    let hulls = stands.into_iter().map(|(object, id, index, rotation)| {
        EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: object,
                name: id.to_string(),
                position: slot(index),
                rotation,
            },
            kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
                controller: SpaceshipController::None,
                allegiance: Some(Allegiance::Neutral),
                design: ShipDesignSource::Inline(kit::catalog_ship(&ships, id)),
                ..default()
            }),
        })
    });
    // One rig: its lights are directional, so a rig per hull would stack.
    let rigs = ThreePointRig::around("photo", Meters3::ZERO, 1.0).actions();
    commands.trigger(LoadScenario(ScenarioConfig {
        description: "The base fleet in a row, for review.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            position: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: hulls.chain(rigs).collect(),
        }],
        ..ScenarioConfig::new(
            "base_fleet".to_string(),
            "Base Fleet".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }));
}
