//! The screenshot photo kit: the three pieces every screenshot scene shares.
//!
//! Included by each `examples/screenshots/*.rs` producer with
//! `#[path = "shared/kit.rs"] mod kit;`. It lives one level down on purpose -
//! `catalog_matches_disk` (`tests/examples_smoke.rs`) treats every `.rs`
//! DIRECTLY under a category dir as a cataloged example, so a sibling
//! `kit.rs` would fail the catalog check.
//!
//! What it holds, and nothing else:
//!
//! - [`photo_rig`]: a three-point light rig that replaces the scenario's single
//!   top-down key light.
//! - [`kenney_hull`]: the section list for a whole Kenney ship, rebuilt from the
//!   `*_cube_*` prototypes in the catalog.
//! - [`NearField`]: near-field asteroid dressing, close enough to the subject to
//!   actually be in frame.
//!
//! Scene layout (where the planetoid sits, where the ships are posed, how the
//! camera is framed) stays with each producer - this is the kit, not the set.

// Each producer includes the whole kit and uses the part its scene needs; the
// unused half is not dead code, it is another scene's tool.
#![allow(dead_code)]

use bevy::prelude::*;
use nova_protocol::prelude::*;

/// Marks a light this kit spawned, so [`replace_key_light`] leaves the rig
/// alone and only fires on the scenario's own light.
#[derive(Component)]
pub struct PhotoRigLight;

/// Replace the scenario's one straight-down key light with a three-point rig.
///
/// Every scenario spawns exactly one `DirectionalLight` (10000 lux, pointing
/// straight down - `crates/nova_scenario/src/loader/lifecycle.rs`), which is
/// what makes the current shots read flat: no rim, no separation from the
/// skybox, no shape on a hull. There is no authorable lighting in the scenario
/// RON yet (that is task `20260805-111534`), so the rig is example-side: an
/// observer swaps the light out the moment the scenario spawns it.
///
/// The rig keeps the scenario's own scoping - the lights carry
/// [`ScenarioScopedMarker`], so a scenario swap tears them down with everything
/// else and the next scenario's light gets replaced in turn.
pub fn photo_rig() -> PhotoRigPlugin {
    PhotoRigPlugin
}

/// Plugin returned by [`photo_rig`].
pub struct PhotoRigPlugin;

impl Plugin for PhotoRigPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(replace_key_light);
    }
}

/// Swap any non-rig directional light for the three-point rig. Spawning the rig
/// re-enters this observer once per light, which is why the rig lights carry
/// [`PhotoRigLight`] - the marker is the recursion guard.
fn replace_key_light(
    add: On<Add, DirectionalLight>,
    q_rig: Query<(), With<PhotoRigLight>>,
    mut commands: Commands,
) {
    if q_rig.contains(add.entity) {
        return;
    }
    // INFO, not debug: the example's own target sits at the default level, and
    // this one line is how a headless run proves the swap happened.
    info!("photo rig: replacing the scenario key light with the three-point rig");
    commands.entity(add.entity).despawn();

    // A light shines along its own -Z, so each transform is placed at the
    // direction the light comes FROM and aimed back at the subject.
    let light = |name: &str, from: Vec3, illuminance: f32, color: Color, shadows: bool| {
        (
            ScenarioScopedMarker,
            PhotoRigLight,
            Name::new(name.to_string()),
            DirectionalLight {
                illuminance,
                color,
                shadow_maps_enabled: shadows,
                ..default()
            },
            Transform::from_translation(from).looking_at(Vec3::ZERO, Vec3::Y),
        )
    };

    // Key: warm, high and to the camera's left, carrying the subject's main
    // form. The only light casting shadows - a second shadow caster on a
    // blocky hull reads as dirt rather than depth.
    commands.spawn(light(
        "Photo Rig Key",
        Vec3::new(-6.0, 5.0, 6.0),
        11000.0,
        Color::srgb(1.0, 0.96, 0.90),
        true,
    ));
    // Rim: the shot's whole reason for existing - a cold, hard edge from behind
    // that separates a dark hull from a dark skybox.
    commands.spawn(light(
        "Photo Rig Rim",
        Vec3::new(3.0, 4.0, -8.0),
        16000.0,
        Color::srgb(0.72, 0.86, 1.0),
        false,
    ));
    // Fill: cool and dim, from the shadow side, so the unlit half keeps some
    // detail without flattening the key.
    commands.spawn(light(
        "Photo Rig Fill",
        Vec3::new(7.0, -2.0, 4.0),
        2600.0,
        Color::srgb(0.62, 0.72, 0.95),
        false,
    ));
}

/// The cell a Kenney hull's core controller fills, for the cargo hulls whose
/// grid leaves it hollow (`cargoa_core_controller`, `cargob_core_controller`).
/// Matches every shipped assembly (`menu_waystation`, `lifeline`,
/// `final_tally`).
const CORE_CONTROLLER_CELL: Vec3 = Vec3::new(0.0, 1.0, 0.0);

/// Rebuild a whole Kenney ship's section list from the catalog: every
/// `{hull}_cube_*` prototype, placed at the grid cell its id names, plus the
/// `{hull}_core_controller` when the hull has one.
///
/// `hull` is the kit name without a trailing underscore: `"racer"`, `"cargoa"`,
/// `"cargob"`. The prefix match is `{hull}_cube_`, so `"racer"` picks up the
/// racer and never the separate `racer_light_cube_*` set.
///
/// The ids are the whole layout - `racer_cube_im1_j0_km2` sits at
/// `(-1, 0, -2)` - which is why this is generated rather than copied: the
/// shipped assemblies (`menu_scrapyard`, `broadside`, `lifeline`, ...) are
/// hundreds of lines of RON saying exactly that, and every one of them agrees
/// with the ids.
///
/// Panics if the hull is not in the catalog or an id does not parse - a scene
/// missing its ship is a broken capture, not something to render around.
pub fn kenney_hull(sections: &GameSections, hull: &str) -> Vec<SpaceshipSectionConfig> {
    let cube_prefix = format!("{hull}_cube_");
    let mut out: Vec<SpaceshipSectionConfig> = sections
        .iter()
        .filter(|section| section.base.id.starts_with(&cube_prefix))
        .map(|section| {
            let cell = grid_cell(&section.base.id[cube_prefix.len()..]).unwrap_or_else(|| {
                panic!(
                    "kenney_hull: section '{}' does not name a grid cell",
                    section.base.id
                )
            });
            mounted(&section.base.id, cell, mount_rotation(section, cell))
        })
        .collect();

    let core = format!("{hull}_core_controller");
    if sections.get_section(&core).is_some() {
        out.push(mounted(&core, CORE_CONTROLLER_CELL, Quat::IDENTITY));
    }

    assert!(
        !out.is_empty(),
        "kenney_hull: no '{cube_prefix}*' prototypes in the section catalog"
    );
    // The catalog's order is the content file's order; sorting makes the built
    // ship the same list whatever the merge produced.
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

/// One hull cube, mounted at `position` and referencing its catalog prototype.
fn mounted(id: &str, position: Vec3, rotation: Quat) -> SpaceshipSectionConfig {
    SpaceshipSectionConfig {
        id: id.to_string(),
        position,
        rotation,
        source: SectionSource::Prototype(id.to_string()),
        modifications: vec![],
    }
}

/// Parse the grid-cell suffix of a cube prototype id (`i1_j0_km2`) into its
/// cell. `m` is the minus sign the ids use, so `km2` is `z = -2`.
fn grid_cell(suffix: &str) -> Option<Vec3> {
    let mut parts = suffix.split('_');
    let x = axis(parts.next()?, 'i')?;
    let y = axis(parts.next()?, 'j')?;
    let z = axis(parts.next()?, 'k')?;
    if parts.next().is_some() {
        return None;
    }
    Some(Vec3::new(x, y, z))
}

/// One axis of a grid-cell suffix: the axis letter, an optional `m` for minus,
/// then the magnitude.
fn axis(part: &str, letter: char) -> Option<f32> {
    let rest = part.strip_prefix(letter)?;
    match rest.strip_prefix('m') {
        Some(magnitude) => Some(-magnitude.parse::<f32>().ok()?),
        None => rest.parse::<f32>().ok(),
    }
}

/// How a cube mounts on the hull: hulls, thrusters and controllers sit as cut
/// (identity), and a turret is rolled 90 degrees about Z so its barrel points
/// away from the ship - away being the side of the hull it sits on.
///
/// This is the rule every shipped assembly follows: the racer's two wing-tip
/// turrets and the cargo-B's two dorsal turrets are the only rotated pieces in
/// `broadside`, `lifeline`, `shakedown_run` and `final_tally`, and all of them
/// carry exactly this rotation.
fn mount_rotation(section: &SectionConfig, cell: Vec3) -> Quat {
    if !matches!(section.kind, SectionKind::Turret(_)) || cell.x == 0.0 {
        return Quat::IDENTITY;
    }
    Quat::from_rotation_z(-cell.x.signum() * std::f32::consts::FRAC_PI_2)
}

/// Near-field asteroid dressing: a ring of rocks close enough to the subject to
/// be IN the shot.
///
/// The old reel scattered its field 90-180 units out, where it reads as
/// background noise or nothing at all. The defaults here start at 25 units with
/// real radius variance, so a wide shot has something with parallax in it -
/// close enough to be in frame, far enough that a hero at the origin is not
/// buried in rock at a 15-unit camera. Scenes tune the fields for their own
/// framing; the subject is assumed to sit at the origin, which is where
/// [`ScatterRegion::Ring`] centres.
pub struct NearField {
    /// Id prefix each rock gets (`"{id_prefix}{i}"`).
    pub id_prefix: &'static str,
    /// How many rocks.
    pub count: u32,
    /// Layout seed - fixed, so every run of a capture frames the same field.
    pub seed: u64,
    /// Ring radii (world units) the rocks land between.
    pub distance: (f32, f32),
    /// Per-rock radius range.
    pub radius: (f32, f32),
    /// Vertical spread above and below the subject's plane.
    pub y_spread: f32,
}

impl Default for NearField {
    fn default() -> Self {
        Self {
            id_prefix: "near_rock_",
            count: 30,
            seed: 20260805,
            distance: (25.0, 90.0),
            radius: (1.2, 5.0),
            y_spread: 18.0,
        }
    }
}

impl NearField {
    /// The scatter action to put in a scenario's `OnStart`.
    pub fn action(&self, game_assets: &GameAssets) -> EventActionConfig {
        EventActionConfig::ScatterObjects(ScatterObjectsConfig {
            id_prefix: self.id_prefix.to_string(),
            count: self.count,
            seed: self.seed,
            region: ScatterRegion::Ring {
                inner: self.distance.0,
                outer: self.distance.1,
                y_min: -self.y_spread,
                y_max: self.y_spread,
            },
            template: ScenarioObjectConfig {
                base: BaseScenarioObjectConfig {
                    id: self.id_prefix.to_string(),
                    name: "Rock".to_string(),
                    position: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                },
                kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                    radius: self.radius.0,
                    texture: game_assets.asteroid_texture.clone().into(),
                    health: 100.0,
                    impact_sound: None,
                    destroy_sound: None,
                    // No wells in the dressing: a near-field rock strong enough
                    // to pull the posed subject would drift it out of frame
                    // over a capture run.
                    surface_gravity: None,
                    invulnerable: false,
                    lock_signature: None,
                }),
            },
            asteroid_radius: Some(self.radius),
        })
    }
}
