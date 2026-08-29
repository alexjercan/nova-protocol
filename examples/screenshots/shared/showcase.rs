//! The section showcase: one ship carrying every section type, plus the
//! turntable that brings each one round to the camera for its closeup.
//!
//! The ship is the ENGINE's own section prototypes, not a Kenney hull: these
//! shots document what each section type IS, so a reader has to be able to tell
//! the parts apart. A pretty hull would hide exactly the thing being shown.
//!
//! Framing is a turntable, not a camera fly-around. The photo rig
//! (`shared/kit.rs`) is fixed in WORLD space, so orbiting the camera would light
//! each closeup differently - one crisp, one flat, one backlit. The camera
//! instead stays on one bearing inside the rig's good wedge and the SHIP yaws to
//! bring each section round to it, so every closeup gets the same key, the same
//! rim and the same read.
//!
//! Included by each closeup producer with
//! `#[path = "shared/showcase.rs"] mod showcase;`. It lives one level down on
//! purpose - `catalog_matches_disk`
//! (`crates/nova_probe_cli/tests/catalog_drift.rs`) treats every `.rs` DIRECTLY
//! under a category dir as a cataloged example, so a sibling `showcase.rs` would
//! fail the catalog check.

// Each producer includes the whole showcase and frames the sections its page
// needs; the ones it skips are not dead code, they are another producer's shot.
#![allow(
    dead_code,
    reason = "one source, many example targets: what one producer leaves unused another needs, so no single build can fulfil an expectation"
)]

use bevy::prelude::*;
use nova_protocol::prelude::*;

/// A single ship carrying every section type, laid out along its axis so each
/// sits at a known spot the script's camera can frame:
/// torpedo(-2) turret(-1) controller(0) hull(+1) thruster(+2).
pub fn section_ship(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let section = |id: &str| {
        sections
            .get_section(id)
            .unwrap_or_else(|| panic!("section '{id}' not found"))
            .clone()
    };
    let at = |id: &str, kind: &str, position: Vec3, rotation: Quat| SpaceshipSectionConfig {
        id: id.to_string(),
        position,
        rotation,
        source: SectionSource::Inline(section(kind)),
        modifications: vec![],
    };
    let upright = Quat::IDENTITY;
    // The turret stands on the right flank: one quarter turn about Z puts the
    // mount's base plate against that face and the gun out of it.
    let turret_rot = Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2);

    // A real ship shape: a spine (front hull -> controller -> rear hull ->
    // thruster) along -/+Z, with the turret and the torpedo bay mounted on the
    // left/right flanks rather than stacked in front.
    let ship = SpaceshipConfig {
        allegiance: None,
        controller: SpaceshipController::None,
        hull: ShipSource::Inline(ShipHull {
            sections: vec![
                at(
                    "controller",
                    "basic_controller_section",
                    Vec3::new(0.0, 0.0, 0.0),
                    Quat::IDENTITY,
                ),
                at(
                    "hull_front",
                    "reinforced_hull_section",
                    Vec3::new(0.0, 0.0, -1.0),
                    Quat::IDENTITY,
                ),
                at(
                    "hull_rear",
                    "reinforced_hull_section",
                    Vec3::new(0.0, 0.0, 1.0),
                    Quat::IDENTITY,
                ),
                at(
                    "thruster",
                    "basic_thruster_section",
                    Vec3::new(0.0, 0.0, 2.0),
                    Quat::IDENTITY,
                ),
                // Turret on the right flank, torpedo bay on the left - both upright.
                at(
                    "turret",
                    "pdc_kinetic_turret_section",
                    Vec3::new(0.75, 0.0, 0.0),
                    turret_rot,
                ),
                at(
                    "torpedo",
                    "torpedo_section",
                    Vec3::new(-1.0, 0.0, 0.0),
                    upright,
                ),
            ],
            ..default()
        }),
        ..default()
    };

    ScenarioConfig {
        description: "A ship carrying every section type for the wiki shots.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            // The scene lights itself: the engine spawns no light, so a
            // scenario that authors none renders black.
            actions: [
                vec![EventActionConfig::SpawnScenarioObject(
                    ScenarioObjectConfig {
                        base: BaseScenarioObjectConfig {
                            id: "showcase_ship".to_string(),
                            name: "Showcase Ship".to_string(),
                            position: Vec3::ZERO,
                            rotation: Quat::IDENTITY,
                        },
                        kind: ScenarioObjectKind::Spaceship(ship),
                    },
                )],
                ThreePointRig::around("showcase", Vec3::ZERO, 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "section_showcase".to_string(),
            "Section Showcase".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// Where the camera stands, as a direction from the section it is framing.
///
/// Picked off the rig in `shared/kit.rs`, not off the ship: the key comes from
/// `(-6, 5, 6)` and the rim from `(3, 4, -8)`, so a camera on the far side of
/// the key from the rim gets the key raking ACROSS the subject (form, not a
/// flat front-lit face) while the rim draws the far edge against the skybox.
/// Standing on the key's own bearing would light every closeup flat.
#[cfg(feature = "debug")]
pub const CAMERA_BEARING: Vec3 = Vec3::new(0.78, 0.36, 0.51);

/// Seconds a step may sit before it is called a stall. Sized with headroom for
/// a slow software-rendered CI GPU (llvmpipe). An expiry is an error exit
/// naming the step, so a run that never loads the showcase fails loudly instead
/// of producing an unframed shot.
#[cfg(feature = "debug")]
pub const STEP_DEADLINE_SECS: f32 = 30.0;

/// One closeup: which section, which way it has to be turned, how close.
#[cfg(feature = "debug")]
pub struct SectionShot {
    /// The section's spot on the ship, in the ship's own space - the same
    /// coordinates [`section_ship`] mounts it at.
    pub mount: Vec3,
    /// The direction, in the ship's own space, this section's identifying face
    /// points. The turntable yaws the ship until this points at the camera:
    /// the thruster nozzle, the turret barrel and the bay doors are each on a
    /// different face, and each is the reason its shot exists.
    pub faces: Vec3,
    /// Camera distance. Small enough that the section fills the frame, large
    /// enough that its neighbours still place it on a ship.
    pub distance: f32,
    pub path: &'static str,
}

/// Turn the ship so `shot.faces` points at the camera, then frame the section.
#[cfg(feature = "debug")]
pub fn present_section(world: &mut World, shot: &SectionShot) {
    let yaw = presenting_yaw(shot.faces);
    yaw_ship(world, yaw);
    // The mount rides round with the hull, so the framed point is the yawed one
    // - not the authored coordinate.
    let subject = yaw * shot.mount;
    let eye = subject + CAMERA_BEARING.normalize() * shot.distance;
    pose_camera(world, eye, subject);
}

/// The yaw that brings `faces` round to the camera. Both vectors are flattened
/// onto the ground plane first: the rig's height is what puts the camera above
/// the ship, and tilting the hull to meet it would read as a ship adrift rather
/// than a part on a bench.
#[cfg(feature = "debug")]
pub fn presenting_yaw(faces: Vec3) -> Quat {
    let flatten = |v: Vec3| Vec3::new(v.x, 0.0, v.z).normalize_or_zero();
    let (from, to) = (flatten(faces), flatten(CAMERA_BEARING));
    if from == Vec3::ZERO || to == Vec3::ZERO {
        return Quat::IDENTITY;
    }
    Quat::from_rotation_arc(from, to)
}

/// Set the showcase ship's rotation. The scene is frozen on a capture run, so
/// this sticks for the step's settle frames and nothing drifts it back.
#[cfg(feature = "debug")]
pub fn yaw_ship(world: &mut World, yaw: Quat) {
    let mut ships = world.query_filtered::<&mut Transform, With<SpaceshipRootMarker>>();
    for mut transform in ships.iter_mut(world) {
        transform.rotation = yaw;
    }
}
