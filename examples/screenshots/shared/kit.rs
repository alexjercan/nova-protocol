//! The screenshot photo kit: the pieces every screenshot scene shares.
//!
//! The three-point photo rig used to live here as an observer that swapped out
//! the engine's hardcoded key light. Lighting is authored scenario content now
//! ([`ThreePointRig`], task `20260805-111534`), so each producer spawns the
//! same rig from its own `ScenarioConfig` and the kit keeps only geometry.
//!
//! Included by each `examples/screenshots/*.rs` producer with
//! `#[path = "shared/kit.rs"] mod kit;`. It lives one level down on purpose -
//! `catalog_matches_disk` (`crates/nova_probe/tests/catalog_drift.rs`) treats
//! every `.rs` DIRECTLY under a category dir as a cataloged example, so a sibling
//! `kit.rs` would fail the catalog check.
//!
//! What it holds, and nothing else:
//!
//! - [`kenney_hull`]: the section list for a whole semantic-parts Kenney ship.
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

/// Rebuild the shipped semantic-parts assembly used by screenshot fixtures.
///
/// The catalog remains the authority for prototype existence. Positions mirror
/// the generated base ship builders so captures exercise the production parts.
pub fn kenney_hull(sections: &GameSections, hull: &str) -> Vec<SpaceshipSectionConfig> {
    let identity = Quat::IDENTITY;
    let port = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
    let starboard = Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2);
    let specs: &[(&str, Vec3, Quat)] = match hull {
        "racer" => &[
            (
                "engine_starboard",
                Vec3::new(0.655, 0.570945, 1.512835),
                identity,
            ),
            (
                "engine_port",
                Vec3::new(-0.655, 0.570945, 1.512835),
                identity,
            ),
            ("wing_starboard", Vec3::new(0.805, 0.5, 0.117835), identity),
            ("wing_port", Vec3::new(-0.805, 0.5, 0.117835), identity),
            ("nose", Vec3::new(0.0, 0.611325, -1.512835), identity),
            ("tail", Vec3::new(0.0, 0.85, 1.612835), identity),
            ("fuselage", Vec3::new(0.0, 0.7, 0.1), identity),
        ],
        "cargob" => &[
            ("engine_starboard", Vec3::new(1.005, 0.7, 2.0), identity),
            ("engine_port", Vec3::new(-1.005, 0.7, 2.0), identity),
            ("pod_starboard", Vec3::new(1.055, 0.7, -0.5), identity),
            ("pod_port", Vec3::new(-1.055, 0.7, -0.5), identity),
            ("nose", Vec3::new(0.0, 1.0, -1.75), identity),
            ("tail", Vec3::new(0.0, 0.65, 2.0), identity),
            ("fuselage", Vec3::new(0.0, 0.9, 0.25), identity),
            ("turret_starboard", Vec3::new(1.55, 1.2, 0.0), starboard),
            ("turret_port", Vec3::new(-1.55, 1.2, 0.0), port),
        ],
        "cargoa" => &[
            ("engine_starboard", Vec3::new(1.205, 0.6, 1.975), identity),
            ("engine_port", Vec3::new(-1.205, 0.6, 1.975), identity),
            ("pod_starboard", Vec3::new(1.205, 0.7, 0.475), identity),
            ("pod_port", Vec3::new(-1.205, 0.7, 0.475), identity),
            ("nose", Vec3::new(0.0, 0.8, -1.8), identity),
            ("tail", Vec3::new(0.0, 0.5875, 1.975), identity),
            ("fuselage", Vec3::new(0.0, 0.8, 0.175), identity),
            ("turret_starboard", Vec3::new(0.85, 0.8, -1.8), starboard),
            ("turret_port", Vec3::new(-0.85, 0.8, -1.8), port),
        ],
        _ => panic!("kenney_hull: unknown semantic ship '{hull}'"),
    };

    specs
        .iter()
        .map(|(id, position, rotation)| {
            let prototype = format!("{hull}_{id}");
            assert!(
                sections.get_section(&prototype).is_some(),
                "kenney_hull: missing prototype '{prototype}'"
            );
            SpaceshipSectionConfig {
                id: (*id).to_string(),
                position: *position,
                rotation: *rotation,
                source: SectionSource::Prototype(prototype),
                modifications: vec![],
            }
        })
        .collect()
}

/// Near-field asteroid dressing: a ring of rocks close enough to the subject to
/// be IN the shot.
///
/// The old reel scattered its field 90-180 units out, where it reads as
/// background noise or nothing at all. The defaults here start at 25 units with
/// real radius variance, so a wide shot has something with parallax in it -
/// close enough to be in frame, far enough that a hero at the origin is not
/// buried in rock at a 15-unit camera. Scenes tune the fields for their own
/// framing; the subject is assumed to sit at the origin, so the field's
/// [`ScatterRegion::Ring`] is centred there.
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
                center: Vec3::ZERO,
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
                    mass: None,
                    invulnerable: false,
                    seed: None,
                    lock_signature: None,
                }),
            },
            asteroid_radius: Some(self.radius),
            min_separation: None,
        })
    }
}
