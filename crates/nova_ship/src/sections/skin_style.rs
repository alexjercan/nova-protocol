//! A ship's STYLE: the look of its derived skin, authored as content.
//!
//! A style is DATA, not constants. It has an id, a palette for the two surfaces
//! a player ever sees, a list of decoration fixtures, and the greeble rules that
//! place them. A ship names one by id and the skin wears it, which is the same
//! resolution a section gets - so a mod can ship a style, a scenario can put a
//! raider style on its enemies and a clean one on the civilians, and none of
//! that is a code change.
//!
//! The parts, and why each is here:
//!
//! - [`StylePalette`] dresses the plates themselves. The plate MESH is derived
//!   and cannot be authored - it is a function of the hull - so a style can only
//!   change what it is made of, which is exactly the seam that keeps the
//!   derivation and the look apart.
//! - [`StyleFixtureConfig`] is one piece of decoration: a model, and what it is
//!   made of as a destructible fixture. The model is an [`AssetRef`] like a
//!   section's `render_mesh`, so a mod ships its own `.glb` and names it the
//!   same way.
//! - [`GreebleRules`] say where the pieces go. They are solver configuration:
//!   link types, greebles with four link points, `near` terms and caps. The
//!   solve reads no plate geometry. See
//!   [`greeble_solver`](super::greeble_solver).

use bevy::prelude::*;
use nova_gameplay::prelude::AssetRef;

use crate::sections::{
    greeble_solver::{CompiledGreebleRules, GreebleDesignId, GreebleRules, GreebleRulesError},
    shell_shape::ShellSurface,
};

/// The style content type, its parts, the loaded catalog and the ship-root
/// component that names one.
pub mod prelude {
    pub use super::{
        GameStyles, ShipStyle, ShipStyleConfig, StyleFixtureConfig, StyleGreebleError,
        StyleGreebles, StylePalette, SurfaceFinish, DECOR_DENSITY,
    };
}

/// Physics density every decoration stands at, and never authored.
///
/// A quarter, the same as the skin's own `SKIN_DENSITY`: a greeble is cladding
/// rather than solid ship, and avian derives the real mass from this and the
/// authored collider's volume. One number rather than a field, because nothing
/// a style says about its look should be able to make one ship heavier than
/// another wearing a different one.
pub const DECOR_DENSITY: f32 = 0.25;

/// What one surface of a plate is made of.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SurfaceFinish {
    /// Base colour.
    pub color: Color,
    /// PBR roughness, 0 (mirror) to 1 (matte).
    pub roughness: f32,
    /// PBR metallic, 0 (dielectric) to 1 (metal).
    pub metallic: f32,
}

impl Default for SurfaceFinish {
    fn default() -> Self {
        Self {
            color: Color::BLACK,
            roughness: 1.0,
            metallic: 0.0,
        }
    }
}

/// The two surfaces a style dresses.
///
/// Both are required, because the relationship BETWEEN them is most of a look:
/// the drop from top to wall at every plate boundary is what reads as a panel
/// gap, and a style that set only one would be a style whose main move was
/// decided by whichever default it happened to keep.
///
/// The third surface - the cell floor, against the section it clads - is not
/// here. It is never in shot, so authoring it is authoring something nobody can
/// see; it keeps the engine's own finish.
#[derive(Clone, Debug, Default, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StylePalette {
    /// The shell face exposed to space, and normally the only one lit.
    pub top: SurfaceFinish,
    /// The shell side exposed at slopes, gaps and edges.
    pub wall: SurfaceFinish,
}

/// One authored look for a derived skin.
///
/// Resolved by [`id`](ShipStyleConfig::id) out of [`GameStyles`], which the mod
/// merge fills exactly as it fills the section catalog - so a mod's style with
/// the id of a base one REPLACES it, and a new id is a new look.
#[derive(Clone, Debug, Default, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ShipStyleConfig {
    /// The id a ship names this style by.
    pub id: String,
    /// The name a picker would show. Not used by the skin itself.
    pub name: String,
    /// What the plates are made of.
    pub palette: StylePalette,
    /// The decoration pieces this style can place. A greeble names one by id
    /// in its `fixture` field.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub fixtures: Vec<StyleFixtureConfig>,
    /// The rules that place the fixtures. Required: a style with no greebles
    /// writes `greeble_rules: (link_types: [], greebles: [])` and places no
    /// decoration.
    #[reflect(ignore)]
    pub greeble_rules: GreebleRules,
}

/// One piece of decoration a style can place on a skin.
///
/// A decoration is a fixture in the full sense - health, a collider, shot off
/// like a plate - because the alternative is art a round passes through. It is
/// NOT a section: shooting an antenna off costs the ship no capability, and that
/// is the line.
#[derive(Clone, Debug, Default, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StyleFixtureConfig {
    /// Names the piece within its style. A greeble's `fixture` names this id.
    pub id: String,
    /// The model, authored in the plate's own frame: `+Y` out of the plate,
    /// `y = 0` the mounting face, `+Z` toward the greeble's `up` link point.
    /// See `assets/base/gltf/greebles/README.md`.
    #[reflect(ignore)]
    pub model: AssetRef<WorldAsset>,
    /// What the piece takes before it comes off. Well under a plate's, since
    /// decoration is the first thing a burst strips.
    ///
    /// Authored, and not derived from the collider: this is the number that
    /// decides when a piece is shot OFF, and two greebles of a size are not
    /// equally worth shooting off.
    pub health: f32,
    /// The box a round stops on, in cells, standing ON the mount face. Not the
    /// model: a greeble is a handful of small solids and a hull of them would
    /// cost more than it is worth.
    ///
    /// Authored because a headless simulation never loads the model these bounds
    /// could otherwise be read from.
    pub collider: Vec3,
}

impl ShipStyleConfig {
    /// The finish this style asks for on `surface`, or `None` for the cell floor
    /// - which is never seen and keeps the engine's own.
    pub fn finish(&self, surface: ShellSurface) -> Option<&SurfaceFinish> {
        match surface {
            ShellSurface::Top => Some(&self.palette.top),
            ShellSurface::Wall => Some(&self.palette.wall),
            ShellSurface::Floor => None,
        }
    }

    /// The style's greeble rules compiled, with every greeble's `fixture`
    /// resolved against [`fixtures`](Self::fixtures).
    ///
    /// `Ok(None)` when the rules hold no greebles: the style places no
    /// decoration. Content lint and the mod merge call this, so an invalid rule
    /// set fails before a ship wears it. Every fault is returned, not only the
    /// first.
    pub fn compile_greebles(&self) -> Result<Option<StyleGreebles>, Vec<StyleGreebleError>> {
        if self.greeble_rules.greebles.is_empty() {
            return Ok(None);
        }
        let mut faults: Vec<StyleGreebleError> = Vec::new();
        let mut fixtures: Vec<Option<usize>> = Vec::new();
        for greeble in &self.greeble_rules.greebles {
            let Some(id) = greeble.fixture.as_deref() else {
                fixtures.push(None);
                continue;
            };
            match self.fixtures.iter().position(|fixture| fixture.id == id) {
                Some(index) => {
                    let collider = self.fixtures[index].collider;
                    if collider.x.max(collider.z) > 1.0 {
                        faults.push(StyleGreebleError::OversizeFixture {
                            greeble: greeble.id.clone(),
                            fixture: id.to_string(),
                            collider,
                        });
                    }
                    fixtures.push(Some(index));
                }
                None => faults.push(StyleGreebleError::UnknownFixture {
                    greeble: greeble.id.clone(),
                    fixture: id.to_string(),
                }),
            }
        }
        let rules = self.greeble_rules.compile().map_err(|errors| {
            errors
                .into_iter()
                .map(StyleGreebleError::Rules)
                .collect::<Vec<_>>()
        });
        match rules {
            Ok(rules) if faults.is_empty() => Ok(Some(StyleGreebles { rules, fixtures })),
            Ok(_) => Err(faults),
            Err(mut errors) => {
                errors.append(&mut faults);
                Err(errors)
            }
        }
    }
}

/// A style's greeble rules after [`ShipStyleConfig::compile_greebles`]: the
/// compiled rules and, per greeble, the index of the fixture it places.
#[derive(Clone, Debug)]
pub struct StyleGreebles {
    /// The compiled rules the solve runs.
    pub rules: CompiledGreebleRules,
    /// Per greeble, in rule order: the index into
    /// [`ShipStyleConfig::fixtures`], or `None` for a greeble with no fixture.
    fixtures: Vec<Option<usize>>,
}

impl StyleGreebles {
    /// The fixture index greeble `greeble` places, or `None` when it places
    /// nothing.
    pub fn fixture(&self, greeble: usize) -> Option<usize> {
        self.fixtures[greeble]
    }
}

/// Why a style's greeble rules do not compile.
#[derive(Clone, Debug, PartialEq)]
pub enum StyleGreebleError {
    /// The rules themselves are invalid.
    Rules(GreebleRulesError),
    /// A greeble names a fixture the style does not list.
    UnknownFixture {
        /// The greeble.
        greeble: GreebleDesignId,
        /// The fixture id it names.
        fixture: String,
    },
    /// A greeble places a fixture whose collider is wider than one cell
    /// across the plate, so no plate can seat it and the solve could never
    /// place it.
    OversizeFixture {
        /// The greeble.
        greeble: GreebleDesignId,
        /// The fixture id it names.
        fixture: String,
        /// The fixture's collider, in cells.
        collider: Vec3,
    },
}

impl std::fmt::Display for StyleGreebleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rules(error) => error.fmt(f),
            Self::UnknownFixture { greeble, fixture } => write!(
                f,
                "greeble '{greeble}' names fixture '{fixture}', which the style does not list"
            ),
            Self::OversizeFixture {
                greeble,
                fixture,
                collider,
            } => write!(
                f,
                "greeble '{greeble}' places fixture '{fixture}', whose collider is {} x {} cells \
                 across the plate; no plate is wider than 1 cell, so none can seat it",
                collider.x, collider.z
            ),
        }
    }
}

impl std::error::Error for StyleGreebleError {}

/// The loaded catalog of authored styles, filled by the mod merge exactly as the
/// section catalog is. Look one up by id with [`get_style`](GameStyles::get_style).
#[derive(Resource, Clone, Debug, Deref, DerefMut, Default)]
pub struct GameStyles(pub Vec<ShipStyleConfig>);

impl GameStyles {
    /// The style with this id, or `None` if nothing authored it.
    pub fn get_style(&self, id: &str) -> Option<&ShipStyleConfig> {
        self.0.iter().find(|style| style.id == id)
    }
}

/// The style a ship's skin wears, by id.
///
/// On the ship ROOT, beside [`ShipSkin`](super::shell_skin::ShipSkin), because a
/// style is a property of the ship and not of any one plate. `None` is the
/// undressed derivation: built-in plate colours and no decoration at all.
#[derive(Component, Clone, Default, Debug, PartialEq, Eq, Reflect)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct ShipStyle(pub Option<String>);

impl ShipStyle {
    /// The style this ship wears, out of `styles`, or `None` when the ship names
    /// none. A named style that nothing authored is a MISS, not a fallback:
    /// quietly wearing another mod's look would be worse than wearing none.
    pub fn resolve<'a>(&self, styles: &'a GameStyles) -> Option<&'a ShipStyleConfig> {
        styles.get_style(self.0.as_deref()?)
    }
}

/// A test style with one fixture, `block`, and rules that place it on nearly
/// every plate: one link type that faces itself, and an empty greeble at a
/// thousandth of the block's weight.
#[cfg(test)]
pub(crate) fn block_style(collider: Vec3) -> ShipStyleConfig {
    use crate::sections::greeble_solver::{
        GreebleConfig, GreebleLinkConfig, GreebleLinkPoints, GreebleRotateType,
    };
    let sides = || GreebleLinkPoints {
        up: "any".into(),
        right: "any".into(),
        down: "any".into(),
        left: "any".into(),
    };
    let greeble = |id: &str, fixture: Option<&str>, weight: f32| GreebleConfig {
        id: id.into(),
        fixture: fixture.map(str::to_string),
        weight,
        rotate: GreebleRotateType::None,
        link_points: sides(),
        near: Vec::new(),
        max_count: None,
    };
    ShipStyleConfig {
        id: "test".to_string(),
        name: "Test".to_string(),
        palette: default(),
        fixtures: vec![StyleFixtureConfig {
            id: "block".to_string(),
            model: AssetRef::from("self://gltf/greebles/placeholder_block.glb#Scene0".to_string()),
            health: 10.0,
            collider,
        }],
        greeble_rules: GreebleRules {
            link_types: vec![GreebleLinkConfig {
                id: "any".into(),
                connects: vec!["any".into()],
            }],
            greebles: vec![
                greeble("empty", None, 0.001),
                greeble("block", Some("block"), 1.0),
            ],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A greeble that names a fixture the style does not list is refused, and
    /// the refusal names both ids.
    #[test]
    fn a_greeble_naming_an_unlisted_fixture_does_not_compile() {
        let mut style = block_style(Vec3::splat(0.2));
        style.fixtures.clear();
        let faults = style.compile_greebles().expect_err("an unknown fixture");
        assert_eq!(
            faults,
            [StyleGreebleError::UnknownFixture {
                greeble: "block".into(),
                fixture: "block".to_string(),
            }]
        );
    }

    /// A style with no greebles compiles to no rules, so it places nothing; a
    /// style with greebles resolves each one's fixture by id.
    #[test]
    fn greebles_resolve_their_fixture_by_id() {
        assert!(ShipStyleConfig::default()
            .compile_greebles()
            .expect("empty rules compile")
            .is_none());
        let compiled = block_style(Vec3::splat(0.2))
            .compile_greebles()
            .expect("the block style compiles")
            .expect("the block style has greebles");
        assert_eq!(compiled.fixture(0), None);
        assert_eq!(compiled.fixture(1), Some(0));
    }

    /// The floor is the one surface a style cannot dress, because it is the one
    /// nobody can see.
    #[test]
    fn a_palette_dresses_the_two_surfaces_that_are_in_shot() {
        let style = ShipStyleConfig {
            palette: StylePalette {
                top: SurfaceFinish {
                    color: Color::WHITE,
                    roughness: 0.2,
                    metallic: 0.9,
                },
                wall: SurfaceFinish::default(),
            },
            ..default()
        };
        assert_eq!(
            style.finish(ShellSurface::Top).map(|f| f.metallic),
            Some(0.9)
        );
        assert!(style.finish(ShellSurface::Wall).is_some());
        assert!(
            style.finish(ShellSurface::Floor).is_none(),
            "a style dressed the face against the section it clads",
        );
    }

    /// A style resolves by id, and a name nothing authored resolves to nothing
    /// rather than to whatever happened to be first.
    #[test]
    fn a_ship_resolves_its_style_by_id_or_not_at_all() {
        let styles = GameStyles(vec![
            ShipStyleConfig {
                id: "raider".to_string(),
                ..default()
            },
            ShipStyleConfig {
                id: "clean".to_string(),
                ..default()
            },
        ]);

        let worn = ShipStyle(Some("clean".to_string()));
        assert_eq!(
            worn.resolve(&styles).map(|style| style.id.as_str()),
            Some("clean")
        );
        assert!(ShipStyle(None).resolve(&styles).is_none());
        assert!(
            ShipStyle(Some("nothing".to_string()))
                .resolve(&styles)
                .is_none(),
            "an unknown style must not fall back to another mod's look",
        );
    }
}
