//! A ship's STYLE: the look of its derived skin, authored as content.
//!
//! A style is DATA, not constants. It has an id, a material per surface role,
//! and a set of decoration fixtures with the rule that says where each may
//! stand. A ship names one by id and the skin wears it, which is the same
//! resolution a section gets - so a mod can ship a style, a scenario can put a
//! raider style on its enemies and a clean one on the civilians, and none of
//! that is a code change.
//!
//! The three parts, and why each is here:
//!
//! - [`StyleSurfaceConfig`] dresses the plates themselves. The plate MESH is
//!   derived and cannot be authored - it is a function of the hull - so a style
//!   can only change what it is made of, which is exactly the seam that keeps
//!   the derivation and the look apart.
//! - [`StyleFixtureConfig`] is one piece of decoration: a model, what it is
//!   made of as a destructible fixture, and its scatter rule. The model is an
//!   [`AssetRef`] like a section's `render_mesh`, so a mod ships its own `.glb`
//!   and names it the same way.
//! - [`ScatterRule`] is where the piece may stand, in the vocabulary
//!   [`PlateReading`] exposes. Nothing in a rule names a cell, a seed or a
//!   count: it is a filter over the neighbourhood plus a share, and the scatter
//!   is a pure function of the structure.

use bevy::prelude::*;
use nova_gameplay::prelude::AssetRef;

use crate::sections::{
    shell_shape::ShellSurface,
    skin_reading::{PlateFacing, PlateReading, PlateRelief},
};

/// The style content type, its parts, the loaded catalog and the ship-root
/// component that names one.
pub mod prelude {
    pub use super::{
        GameStyles, ScatterAlign, ScatterRule, ShipStyle, ShipStyleConfig, StyleFixtureConfig,
        StyleSurfaceConfig,
    };
}

/// Which way a piece is turned on the plate it stands on.
///
/// Quarter turns about the plate's own outward axis and nothing finer: the
/// plate lattice is square, both axes below are cardinals, and a piece snapped
/// to the grid is the whole point. There is no jitter and no free yaw,
/// deliberately - see the module note on alignment.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ScatterAlign {
    /// Not turned. The authored default, and right for anything with no long
    /// axis - a blister, a stud, a hatch.
    #[default]
    Free,
    /// The piece's own `+Z` lies DOWN THE RUN, along
    /// [`PlateReading::along`](super::skin_reading::PlateReading::along): a rib
    /// strip follows the spine it is on, a row of vents lines up with itself.
    Run,
    /// The piece's own `+Z` points OFF THE SHIP, down
    /// [`PlateReading::fall`](super::skin_reading::PlateReading::fall): a
    /// fairing leans out over the edge it stands on rather than lying along it.
    ///
    /// Square to [`Run`](ScatterAlign::Run), and the reason the fall is read at
    /// all. A plate that does not fall one way is left unturned, so this is a
    /// rule for the falling plate - `Brink` above all.
    Outward,
}

impl ScatterAlign {
    /// Whether this turns nothing - the `skip_serializing_if` a style keeps the
    /// field out of its RON with.
    pub fn is_free(&self) -> bool {
        *self == Self::Free
    }
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
    /// The material each surface role of a plate is made of. A role left out
    /// keeps the built-in colour, so a style that only wants to restate the
    /// decoration says nothing here.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub surfaces: Vec<StyleSurfaceConfig>,
    /// The decoration this style scatters, in PRIORITY order: a plate takes at
    /// most one piece, and the first rule that claims it wins.
    ///
    /// That ordering is the whole of the conflict resolution, and it is what
    /// lets a style put a rare, striking piece first and a common filler last
    /// without either having to know about the other.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub fixtures: Vec<StyleFixtureConfig>,
}

impl ShipStyleConfig {
    /// The material this style asks for on `surface`, if it names one.
    pub fn surface(&self, surface: ShellSurface) -> Option<&StyleSurfaceConfig> {
        self.surfaces.iter().find(|dress| dress.surface == surface)
    }
}

/// What one surface role of a plate is made of.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StyleSurfaceConfig {
    /// The role this dresses: the cell floor, the walls, or the top.
    pub surface: ShellSurface,
    /// Base colour.
    pub color: Color,
    /// PBR roughness, 0 (mirror) to 1 (matte).
    pub roughness: f32,
    /// PBR metallic, 0 (dielectric) to 1 (metal).
    pub metallic: f32,
}

/// One piece of decoration a style scatters over a skin.
///
/// A decoration is a fixture in the full sense - health, mass, a collider,
/// shot off like a plate - because the alternative is art a round passes
/// through. It is NOT a section: shooting an antenna off costs the ship no
/// capability, and that is the line.
#[derive(Clone, Debug, Default, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StyleFixtureConfig {
    /// Names the piece within its style. Also the SALT the scatter hashes with,
    /// so two pieces sharing one rule do not claim the same plates.
    pub id: String,
    /// The model, authored in the plate's own frame: `+Y` out of the plate,
    /// `y = 0` the mounting face. See `assets/base/gltf/greebles/README.md`.
    #[reflect(ignore)]
    pub model: AssetRef<WorldAsset>,
    /// What the piece takes before it comes off. Well under a plate's, since
    /// decoration is the first thing a burst strips.
    pub health: f32,
    /// Density, handed to avian, which derives real mass from it and the
    /// collider's volume. See the skin's own `SKIN_DENSITY`.
    pub density: f32,
    /// The box a round stops on, in cells, standing ON the mount face. Not the
    /// model: a greeble is a handful of small solids and a hull of them would
    /// cost more than it is worth.
    pub collider: Vec3,
    /// Where the piece may stand.
    #[cfg_attr(feature = "serde", serde(default))]
    pub scatter: ScatterRule,
}

/// Where one decoration may stand, as a filter over the plate vocabulary.
///
/// Every field is a FILTER except the last four. A rule with nothing set
/// matches every plate, which is why a piece with no rule covers a hull.
///
/// The order the four are applied in is fixed and load-bearing: the FILTER
/// says what kind of place the piece belongs, the LATTICE says which cells of
/// it are on the grid, the SHARE thins that out, and the PATCH puts one back
/// wherever the thinning left a block of hull bare.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ScatterRule {
    /// The reliefs the piece may stand on. Empty means any.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub relief: Vec<PlateRelief>,
    /// Which way the plate must face, in the ship's own frame.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "PlateFacing::is_any")
    )]
    pub facing: PlateFacing,
    /// The shortest run of LIKE plate the piece will stand on. A long piece
    /// wants a long run under it, whether that is a panel or the edge of a hull.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "is_zero"))]
    pub min_run: u8,
    /// How much of its cell the plate must fill, in quarter cells. Keeps a tall
    /// piece off a sliver of plate that is nearly all floor.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "is_zero"))]
    pub min_height: u8,
    /// How far in from the edge of its flat patch the plate must be. `0` allows
    /// the edge itself.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "is_zero"))]
    pub min_border: u8,
    /// How far in from that edge the plate may be at most. `Some(0)` is TRIM:
    /// the piece only ever lands on the border of a panel, which is where the
    /// decoration research says a greeble belongs.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub max_border: Option<u8>,
    /// How much structure must stand under the plate. Keeps a heavy piece off
    /// the skin over a one-cell spar.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "is_zero"))]
    pub min_depth: u8,
    /// How many of the eight cells around the plate must carry the surface on.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "is_zero"))]
    pub min_enclosure: u8,
    /// How close the mouth of a fitting must be, across the surface. `Some(1)`
    /// is "right beside a nozzle or a gun well"; `None` does not care.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub near_fitting: Option<u8>,
    /// The LATTICE the piece claims cells on: `1` is every eligible plate, `2`
    /// is every other cell on both in-plane axes.
    ///
    /// This is the grid-occupancy half of the scatter, and it is what the
    /// decoration research says to use INSTEAD of blue noise: Poisson sampling
    /// deliberately destroys alignment, and alignment is the whole difference
    /// between greebles that read as bolted on and greebles that read as
    /// confetti.
    #[cfg_attr(feature = "serde", serde(default = "one"))]
    pub stride: u8,
    /// The share of the plates that pass everything above which actually take
    /// the piece, 0 to 1. Decided by hashing the CELL, never by an RNG.
    #[cfg_attr(feature = "serde", serde(default = "all"))]
    pub chance: f32,
    /// The block of hull, in cells, this rule is guaranteed a piece in: `0` is
    /// off, and any other value says "at least one piece per `patch` cubed
    /// cells of ship that this rule can stand on at all".
    ///
    /// The DENSITY NORMALISATION, and the reason a rule needs one: every other
    /// knob here is per plate, and they multiply. A stride of 2 is a quarter of
    /// the surface, a share of 0.5 is half of that, and a relief filter is
    /// another fraction again - which reads as a field of pieces on a
    /// 150-plate generated hull and as three pieces on a 20-plate hand-built
    /// one. Measured: a rule tuned on the row put ONE visible piece on an
    /// editor build.
    ///
    /// A FLOOR and never a cap. Where the share has already put a piece of this
    /// rule in a block, nothing happens; where it has not, the block's lowest
    /// hashing eligible plate takes one. It never displaces another rule's
    /// piece, so priority still means what it says.
    ///
    /// The floor drops the SHARE only - the neighbourhood filter and the
    /// lattice still hold, so a floor piece lands on the same grid the rest of
    /// the rule does and a row does not acquire a stray off-lattice member.
    /// Keep `patch` at or above [`stride`](ScatterRule::stride), or a block can
    /// hold no lattice cell for the floor to pick.
    ///
    /// It is still a pure function of the structure, but of the BLOCK rather
    /// than of the cell: a hull that grows by one cell keeps every piece
    /// outside the block that cell lands in, and inside it can move one piece -
    /// only the floor's own, and only if the newcomer hashes lower.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "is_zero"))]
    pub patch: u8,
    /// Which way the piece is turned on its plate.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "ScatterAlign::is_free")
    )]
    pub align: ScatterAlign,
}

impl Default for ScatterRule {
    fn default() -> Self {
        Self {
            relief: Vec::new(),
            facing: PlateFacing::Any,
            min_run: 0,
            min_height: 0,
            min_border: 0,
            max_border: None,
            min_depth: 0,
            min_enclosure: 0,
            near_fitting: None,
            stride: 1,
            chance: 1.0,
            patch: 0,
            align: ScatterAlign::Free,
        }
    }
}

impl ScatterRule {
    /// Whether a plate reading `reading` is the KIND of place this rule wants.
    ///
    /// The neighbourhood half only. The lattice and the share are decided by
    /// the scatter, which is the only thing that knows the cell.
    pub fn accepts(&self, reading: &PlateReading) -> bool {
        if !self.relief.is_empty() && !self.relief.contains(&reading.relief) {
            return false;
        }
        if !self.facing.accepts(reading.out) {
            return false;
        }
        if reading.run < self.min_run
            || reading.height < self.min_height
            || reading.border < self.min_border
            || reading.depth < self.min_depth
            || reading.enclosure < self.min_enclosure
        {
            return false;
        }
        if self.max_border.is_some_and(|most| reading.border > most) {
            return false;
        }
        if self.near_fitting.is_some_and(|near| reading.pocket > near) {
            return false;
        }
        true
    }
}

/// `serde` default for [`ScatterRule::stride`]: every eligible plate.
#[cfg(feature = "serde")]
fn one() -> u8 {
    1
}

/// `serde` default for [`ScatterRule::chance`]: every eligible plate.
#[cfg(feature = "serde")]
fn all() -> f32 {
    1.0
}

/// `skip_serializing_if` for a count that defaults to none.
#[cfg(feature = "serde")]
fn is_zero(count: &u8) -> bool {
    *count == 0
}

/// The loaded catalog of authored styles, filled by the mod merge exactly as
/// the section catalog is. Look one up by id with
/// [`get_style`](GameStyles::get_style).
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
/// On the ship ROOT, beside [`ShipSkin`](super::shell_skin::ShipSkin), because
/// a style is a property of the ship and not of any one plate. `None` is the
/// undressed derivation: built-in plate colours and no decoration at all.
#[derive(Component, Clone, Default, Debug, PartialEq, Eq, Reflect)]
#[reflect(Component)]
pub struct ShipStyle(pub Option<String>);

impl ShipStyle {
    /// The style this ship wears, out of `styles`, or `None` when the ship
    /// names none. A named style that nothing authored is a MISS, not a
    /// fallback: quietly wearing another mod's look would be worse than
    /// wearing none.
    pub fn resolve<'a>(&self, styles: &'a GameStyles) -> Option<&'a ShipStyleConfig> {
        styles.get_style(self.0.as_deref()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reading(relief: PlateRelief) -> PlateReading {
        PlateReading {
            out: IVec3::Y,
            along: IVec3::X,
            fall: IVec3::Z,
            relief,
            enclosure: 8,
            run: 4,
            border: 2,
            height: 2,
            depth: 3,
            pocket: 3,
        }
    }

    /// An empty rule takes any plate at all, which is what makes a fixture with
    /// nothing authored cover a hull rather than land nowhere.
    #[test]
    fn a_rule_with_nothing_set_accepts_every_plate() {
        let rule = ScatterRule::default();
        for relief in [
            PlateRelief::Flat,
            PlateRelief::Step,
            PlateRelief::Ridge,
            PlateRelief::Peak,
            PlateRelief::Bevel,
            PlateRelief::Brink,
            PlateRelief::Spur,
        ] {
            assert!(rule.accepts(&reading(relief)), "{relief:?} was refused");
        }
    }

    /// The falling plate is THREE reliefs a rule can name apart, which is the
    /// whole point of the split: a rim strip on the straight edge of a hull is
    /// not the same piece as a cap on the tip of a spar.
    #[test]
    fn a_rule_can_name_one_kind_of_falling_plate_without_the_others() {
        let edges = ScatterRule {
            relief: vec![PlateRelief::Brink],
            ..default()
        };
        assert!(edges.accepts(&reading(PlateRelief::Brink)));
        assert!(!edges.accepts(&reading(PlateRelief::Spur)));
        assert!(!edges.accepts(&reading(PlateRelief::Bevel)));
    }

    /// The filters that make a look: relief, facing, run and the two borders.
    #[test]
    fn a_rule_filters_on_the_neighbourhood_it_names() {
        let ridges = ScatterRule {
            relief: vec![PlateRelief::Ridge],
            ..default()
        };
        assert!(ridges.accepts(&reading(PlateRelief::Ridge)));
        assert!(!ridges.accepts(&reading(PlateRelief::Flat)));

        let roof = ScatterRule {
            facing: PlateFacing::Down,
            ..default()
        };
        assert!(
            !roof.accepts(&reading(PlateRelief::Flat)),
            "this plate faces up"
        );

        let long = ScatterRule {
            min_run: 6,
            ..default()
        };
        assert!(!long.accepts(&reading(PlateRelief::Flat)), "the run is 4");

        // TRIM: only ever on the border of a panel.
        let trim = ScatterRule {
            max_border: Some(0),
            ..default()
        };
        assert!(!trim.accepts(&reading(PlateRelief::Flat)), "border is 2");
    }

    /// A fitting filter passes only where the mouth of one is close.
    #[test]
    fn a_fitting_filter_reads_the_pocket_distance() {
        let beside = ScatterRule {
            near_fitting: Some(1),
            ..default()
        };
        let mut near = reading(PlateRelief::Flat);
        near.pocket = 1;
        assert!(beside.accepts(&near));
        assert!(
            !beside.accepts(&reading(PlateRelief::Flat)),
            "a plate with no fitting near it took a piece that wanted one",
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
