//! A ship's STYLE: the look of its derived skin, authored as content.
//!
//! A style is DATA, not constants. It has an id, a palette for the two surfaces
//! a player ever sees, and a set of decoration fixtures with the placement that
//! says where each belongs. A ship names one by id and the skin wears it, which
//! is the same resolution a section gets - so a mod can ship a style, a scenario
//! can put a raider style on its enemies and a clean one on the civilians, and
//! none of that is a code change.
//!
//! The three parts, and why each is here:
//!
//! - [`StylePalette`] dresses the plates themselves. The plate MESH is derived
//!   and cannot be authored - it is a function of the hull - so a style can only
//!   change what it is made of, which is exactly the seam that keeps the
//!   derivation and the look apart.
//! - [`StyleFixtureConfig`] is one piece of decoration: a model, what it is made
//!   of as a destructible fixture, and where it belongs. The model is an
//!   [`AssetRef`] like a section's `render_mesh`, so a mod ships its own `.glb`
//!   and names it the same way.
//! - [`FixturePlacement`] is that "where", in three words: a region of the hull,
//!   how much of it to cover, and which way to turn. Nothing in a placement
//!   names a cell, a seed or a count.
//!
//! # Intent, not plate-reading
//!
//! A placement says what a piece is FOR. It expands - privately, in
//! [`ScatterRule`] below - into the neighbourhood filter, the lattice, the share
//! and the patch floor that the scatter actually runs. That expansion is engine
//! policy: it is tuned against real hulls, it changes when the derivation
//! changes, and a style that had spelled it out would go wrong silently when it
//! did. An author picks the high ground; the engine decides what the high ground
//! measures on this hull.
//!
//! Two things ARE read off the piece rather than authored, both out of its
//! collider, because the piece's own size already answers them and a second
//! authored word saying the same thing is a word that can disagree with it:
//! how long a run of like plate it needs under it
//! ([`min_run`](FixturePlacement::min_run)), and whether it needs an unbroken
//! seat at all ([`needs_seat`](FixturePlacement::needs_seat)) - a decal does
//! not, a stack does.

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
        FixtureDensity, FixtureOrientation, FixturePlacement, FixtureRegion, GameStyles, ShipStyle,
        ShipStyleConfig, StyleFixtureConfig, StylePalette, SurfaceFinish, DECOR_DENSITY,
        TRIM_HEIGHT,
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

/// The part of a hull one piece of decoration belongs on.
///
/// VISUAL INTENT, in the words a style author thinks in - not the plate reading
/// it expands to. The expansion is [`FixtureRegion::rule`], and it is engine
/// policy: measured against real hulls, and free to change when the derivation
/// does.
///
/// Ordered as a style reads: the flat places first, then the edges, then the
/// high ground, then the two that ignore relief entirely.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FixtureRegion {
    /// Unbroken panel: the flat face of a hull, and the panel with one corner
    /// taken off that still reads as one. Where a sign, a hatch or a decal goes.
    Panel,
    /// Working deck: flat plate with real ship under it. The laydown area - a
    /// crate, a rack, a tank, anything that would need a floor to stand on.
    Deck,
    /// The ship's SIDES: deck plate that faces out rather than up. Windows,
    /// doors and anything read from another ship.
    Flank,
    /// The straight edge of a hull, where the skin falls away along one whole
    /// side. Rim strips, hazard paint, fairings - the pieces that draw a
    /// silhouette.
    Edge,
    /// The pointiest thing on the hull: crests, spar tips and studs. Masts,
    /// stacks and aerials, and the one region that accepts a creased top -
    /// refusing it would strip a ship of its silhouette to fix a bedding defect.
    HighGround,
    /// Right beside the mouth of a fitting - a nozzle, a gun well, a bay. The
    /// service kit: grilles, lighting, machinery.
    NearFitting,
    /// Anywhere with a seat on it. The FILLER, and the default: a style's last
    /// rule, which takes whatever the rules above it left.
    #[default]
    Anywhere,
}

/// How much of the region it names a fixture actually covers.
///
/// A ladder rather than a number, because the three dials underneath it -
/// lattice, share and patch floor - interact, and an author setting them apart
/// gets a hull carpeted at one size and bare at another. Each rung is one
/// tuned (stride, chance, patch) triple; see [`FixtureDensity::rule`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FixtureDensity {
    /// The thinnest rung there is: a piece or two on a whole ship, with the
    /// floor under it so a big hull does not lose them both. Never a field.
    Rare,
    /// Scattered thinly, with a floor so a big hull does not clump it.
    Sparse,
    /// The workhorse rung: a piece every few cells of the region.
    #[default]
    Regular,
    /// Heavy cover, still on the lattice.
    Dense,
    /// Every plate the region admits, off the lattice entirely. For the pieces
    /// that ARE the surface - a rim strip down a whole edge, cladding.
    Every,
}

/// Which way a piece is turned on the plate it stands on.
///
/// Quarter turns about the plate's own outward axis and nothing finer: the plate
/// lattice is square, both axes below are cardinals, and a piece snapped to the
/// grid is the whole point. There is no jitter and no free yaw, deliberately.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FixtureOrientation {
    /// Not turned. Right for anything with no long axis - a blister, a stud, a
    /// hatch.
    #[default]
    Free,
    /// The piece's own `+Z` lies DOWN THE RUN, along
    /// [`PlateReading::along`]: a rib strip follows the spine it is on, a row of
    /// vents lines up with itself.
    Along,
    /// The piece's own `+Z` points OFF THE SHIP, down [`PlateReading::fall`]: a
    /// fairing leans out over the edge it stands on rather than lying along it.
    ///
    /// Square to [`Along`](FixtureOrientation::Along). A plate that does not
    /// fall one way is left unturned, so this is a rule for the falling plate -
    /// [`Edge`](FixtureRegion::Edge) above all.
    Outward,
}

/// Where one piece of decoration belongs, in three authored words.
#[derive(Clone, Copy, Debug, Default, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FixturePlacement {
    /// The part of the hull the piece is for.
    #[cfg_attr(feature = "serde", serde(default))]
    pub region: FixtureRegion,
    /// How much of that region it covers.
    #[cfg_attr(feature = "serde", serde(default))]
    pub density: FixtureDensity,
    /// Which way it is turned on the plate it lands on.
    #[cfg_attr(feature = "serde", serde(default))]
    pub orientation: FixtureOrientation,
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
    /// The decoration this style scatters, in PRIORITY order: a plate takes at
    /// most one piece, and the first fixture that claims it wins.
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

/// One piece of decoration a style scatters over a skin.
///
/// A decoration is a fixture in the full sense - health, a collider, shot off
/// like a plate - because the alternative is art a round passes through. It is
/// NOT a section: shooting an antenna off costs the ship no capability, and that
/// is the line.
#[derive(Clone, Debug, Default, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StyleFixtureConfig {
    /// Names the piece within its style. Also the SALT the scatter hashes with,
    /// so two pieces sharing one placement do not claim the same plates.
    pub id: String,
    /// The model, authored in the plate's own frame: `+Y` out of the plate,
    /// `y = 0` the mounting face. See `assets/base/gltf/greebles/README.md`.
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
    /// could otherwise be read from. It is also what
    /// [`FixturePlacement::min_run`] reads, so it is load-bearing for the LOOK
    /// as well as for the physics.
    pub collider: Vec3,
    /// Where the piece belongs.
    #[cfg_attr(feature = "serde", serde(default))]
    pub placement: FixturePlacement,
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
}

impl StyleFixtureConfig {
    /// The private rule the scatter runs for this piece: its placement expanded,
    /// with the run gate read off its own collider.
    pub(crate) fn scatter(&self) -> ScatterRule {
        let region = self.placement.region.rule();
        let (stride, chance, patch) = self.placement.density.rule();
        ScatterRule {
            min_run: FixturePlacement::min_run(self.collider),
            // Two ways to be free of the seat gate, and a piece needs only one:
            // the region says the high ground is all cones, or the piece is
            // trim and does not care.
            whole_seat: region.whole_seat && FixturePlacement::needs_seat(self.collider),
            stride,
            chance,
            patch,
            ..region
        }
    }
}

impl FixturePlacement {
    /// The shortest run of like plate a piece of this size will stand on.
    ///
    /// One cell of run per half-cell the piece spans across the plate, so a
    /// strip most of a cell long asks for a neighbour either side and a stud
    /// asks for nothing. Read off the collider rather than authored: the
    /// collider is already the piece's size in cells, and a second authored
    /// number saying the same thing is a number that can disagree with it.
    ///
    /// The two IN-PLANE axes only. `y` stands off the plate and says nothing
    /// about how much surface the piece needs under it.
    pub fn min_run(collider: Vec3) -> u8 {
        let span = collider.x.max(collider.z).max(0.0);
        ((span * RUN_PER_CELL).ceil() as u8).clamp(0, MAX_DERIVED_RUN)
    }

    /// Whether a piece of this size needs an unbroken seat under it.
    ///
    /// TRIM does not: a stencil, a livery patch or a hazard stripe stands a
    /// hand's breadth off the plate, and a crease under it is a fold in a decal
    /// rather than a gap you can see. Anything that stands PROUD does - a stack
    /// or a winch on a crease touches along one line and floats at both ends.
    ///
    /// Read off the collider for the reason [`min_run`](Self::min_run) is: the
    /// piece's own size already answers this, and a second authored word saying
    /// it is a word that can disagree.
    ///
    /// This is also what keeps a HAND-BUILT hull dressed. Such a hull is one
    /// cell thick nearly everywhere, so no plate on it is coplanar at all and
    /// every seated region reaches exactly nothing; its decoration is the trim
    /// and the high ground, and this is half of that.
    pub fn needs_seat(collider: Vec3) -> bool {
        collider.y >= TRIM_HEIGHT
    }
}

/// Cells of run demanded per cell of piece. See [`FixturePlacement::min_run`].
const RUN_PER_CELL: f32 = 2.0;

/// How far off the plate a piece may stand and still count as TRIM: one metre,
/// at ten metres to the cell. See [`FixturePlacement::needs_seat`].
///
/// `pub` because it is an AUTHORING fact, not an implementation one: a style
/// author sizing a decal is deciding which side of this it lands on, and the
/// base-content tests check their own kits against it.
pub const TRIM_HEIGHT: f32 = 0.1;

/// The longest run a derived gate will ask for, however big a mod's collider is.
///
///
/// A hull's longest like-plate run saturates at
/// [`RUN_REACH`](super::skin_reading::RUN_REACH), and a gate above what any hull
/// can offer is a piece that lands nowhere with nothing on screen to say why.
const MAX_DERIVED_RUN: u8 = 3;

impl FixtureRegion {
    /// The neighbourhood half of this region's rule.
    ///
    /// ENGINE POLICY, and tuned against measured hulls rather than derived:
    /// generated ships come out four fifths falling plate and a seventh
    /// [`Step`](PlateRelief::Step), with [`Flat`](PlateRelief::Flat) under a
    /// seventh, so a region written for flat panels alone lands on almost
    /// nothing. The relief lists below are what make each word mean a real part
    /// of a real hull.
    fn rule(self) -> ScatterRule {
        let base = ScatterRule::ANY;
        match self {
            // Bevel rides along with Flat: it is a panel with one corner taken
            // off, a piece sized for Flat very nearly fits, and the seat gate
            // below refuses the ones that actually crease.
            Self::Panel => ScatterRule {
                relief: &[PlateRelief::Flat, PlateRelief::Bevel],
                min_height: 1,
                ..base
            },
            // Depth 2: a laydown deck is plate with ship under it, not skin
            // stretched over a one-cell spar.
            Self::Deck => ScatterRule {
                relief: &[PlateRelief::Flat, PlateRelief::Step],
                min_height: 1,
                min_depth: 2,
                ..base
            },
            Self::Flank => ScatterRule {
                relief: &[PlateRelief::Flat, PlateRelief::Step],
                facing: PlateFacing::Side,
                min_height: 1,
                min_depth: 2,
                ..base
            },
            Self::Edge => ScatterRule {
                relief: &[PlateRelief::Brink],
                ..base
            },
            // The one region that drops the seat gate, and it has to: a crest,
            // a spar tip and a stud are CONES every time, so a whole-seat gate
            // would refuse every plate this region is for. `min_height` stays 0
            // for the same reason - a spur fills so little of its cell that it
            // measures height 0 - and `min_depth` 1 is what keeps a mast off
            // skin with nothing behind it.
            Self::HighGround => ScatterRule {
                relief: &[
                    PlateRelief::Ridge,
                    PlateRelief::Peak,
                    PlateRelief::Spur,
                    PlateRelief::Step,
                ],
                whole_seat: false,
                facing: PlateFacing::Up,
                min_depth: 1,
                ..base
            },
            // Distance 1 is "the plate is touching the mouth", which is the only
            // reading that puts a grille ON a nozzle rather than near one.
            Self::NearFitting => ScatterRule {
                near_fitting: Some(1),
                ..base
            },
            Self::Anywhere => base,
        }
    }
}

impl FixtureDensity {
    /// This rung as `(stride, chance, patch)`.
    ///
    /// The lattice is 2 on every rung but [`Every`](FixtureDensity::Every),
    /// because the decoration research says alignment - not blue noise - is what
    /// makes a greeble read as bolted on, and every other cell is the coarsest
    /// lattice that still reads as one. The share is what separates the rungs,
    /// and the patch floor rises as the share falls: a thin share on a small
    /// hull otherwise lands nothing at all.
    ///
    /// [`Rare`](FixtureDensity::Rare) keeps a share rather than leaning on the
    /// floor alone, and that was MEASURED: at share zero it lands nothing at
    /// all wherever another rule shares its region, because the share pass -
    /// the one pass that reads priority plate by plate - is the pass it sat
    /// out. `armoured_sensor` is first in its kit and took NOTHING on any bench
    /// seed. A tenth of a lattice that is already every other cell is one or
    /// two pieces on a generated hull, which is what the rung means.
    fn rule(self) -> (u8, f32, u8) {
        match self {
            Self::Rare => (2, 0.1, 6),
            Self::Sparse => (2, 0.15, 6),
            Self::Regular => (2, 0.3, 5),
            Self::Dense => (2, 0.5, 4),
            Self::Every => (1, 1.0, 0),
        }
    }
}

/// Where one decoration may stand, as a filter over the plate vocabulary.
///
/// PRIVATE, and the reason the authored vocabulary above exists: every field
/// here is a fact about how a skin is derived, so a style that had spelled one
/// out would be a style that went quietly wrong the next time the derivation
/// changed. It is built by [`StyleFixtureConfig::scatter`] and never authored.
///
/// The order the four halves are applied in is fixed and load-bearing: the
/// FILTER says what kind of place the piece belongs, the LATTICE says which
/// cells of it are on the grid, the SHARE thins that out, and the PATCH puts one
/// back wherever the thinning left a block of hull bare.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ScatterRule {
    /// The reliefs the piece may stand on. Empty means any.
    pub(crate) relief: &'static [PlateRelief],
    /// Whether the plate's top must be ONE unbroken surface.
    ///
    /// On for every region but the high ground. Tilted counts - a ramp is one
    /// flat surface and a piece is bedded onto it by
    /// [`decor_pose`](super::skin_decor::decor_pose); what this refuses is the
    /// CREASE, where a flat-bottomed model can only ever touch along one line.
    pub(crate) whole_seat: bool,
    /// Which way the plate must face, in the ship's own frame.
    pub(crate) facing: PlateFacing,
    /// The shortest run of LIKE plate the piece will stand on, off its collider.
    pub(crate) min_run: u8,
    /// How much of its cell the plate must fill, in quarter cells. Keeps a piece
    /// off a sliver of plate that is nearly all floor.
    pub(crate) min_height: u8,
    /// How much structure must stand under the plate. Keeps a heavy piece off
    /// the skin over a one-cell spar.
    pub(crate) min_depth: u8,
    /// How close the mouth of a fitting must be, across the surface.
    pub(crate) near_fitting: Option<u8>,
    /// The LATTICE the piece claims cells on: `1` is every eligible plate, `2`
    /// is every other cell on both in-plane axes.
    pub(crate) stride: u8,
    /// The share of the plates that pass everything above which actually take
    /// the piece, 0 to 1. Decided by hashing the CELL, never by an RNG.
    pub(crate) chance: f32,
    /// The block of hull, in cells, this rule is guaranteed a piece in: `0` is
    /// off, and any other value says "at least one piece per `patch` cubed cells
    /// of ship that this rule can stand on at all".
    ///
    /// The DENSITY NORMALISATION. Every other knob here is per plate and they
    /// multiply, which reads as a field of pieces on a 150-plate generated hull
    /// and as three pieces on a 20-plate hand-built one. A FLOOR and never a
    /// cap: where the share has already put a piece of this rule in a block,
    /// nothing happens; where it has not, the block's lowest hashing eligible
    /// plate takes one. It never displaces another rule's piece, so priority
    /// still means what it says.
    pub(crate) patch: u8,
}

impl ScatterRule {
    /// Every plate with a whole seat on it, at every plate: what a region starts
    /// from before it names anything.
    const ANY: Self = Self {
        relief: &[],
        whole_seat: true,
        facing: PlateFacing::Any,
        min_run: 0,
        min_height: 0,
        min_depth: 0,
        near_fitting: None,
        stride: 1,
        chance: 1.0,
        patch: 0,
    };

    /// Whether a plate reading `reading` is the KIND of place this rule wants.
    ///
    /// The neighbourhood half only. The lattice and the share are decided by the
    /// scatter, which is the only thing that knows the cell.
    pub(crate) fn accepts(&self, reading: &PlateReading) -> bool {
        if self.whole_seat && !reading.coplanar {
            return false;
        }
        if !self.relief.is_empty() && !self.relief.contains(&reading.relief) {
            return false;
        }
        if !self.facing.accepts(reading.out) {
            return false;
        }
        if reading.run < self.min_run
            || reading.height < self.min_height
            || reading.depth < self.min_depth
        {
            return false;
        }
        if self.near_fitting.is_some_and(|near| reading.fitting > near) {
            return false;
        }
        true
    }
}

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
pub struct ShipStyle(pub Option<String>);

impl ShipStyle {
    /// The style this ship wears, out of `styles`, or `None` when the ship names
    /// none. A named style that nothing authored is a MISS, not a fallback:
    /// quietly wearing another mod's look would be worse than wearing none.
    pub fn resolve<'a>(&self, styles: &'a GameStyles) -> Option<&'a ShipStyleConfig> {
        styles.get_style(self.0.as_deref()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every region, in the order the enum declares them.
    const REGIONS: [FixtureRegion; 7] = [
        FixtureRegion::Panel,
        FixtureRegion::Deck,
        FixtureRegion::Flank,
        FixtureRegion::Edge,
        FixtureRegion::HighGround,
        FixtureRegion::NearFitting,
        FixtureRegion::Anywhere,
    ];

    /// Every rung, thinnest first.
    const DENSITIES: [FixtureDensity; 5] = [
        FixtureDensity::Rare,
        FixtureDensity::Sparse,
        FixtureDensity::Regular,
        FixtureDensity::Dense,
        FixtureDensity::Every,
    ];

    fn reading(relief: PlateRelief) -> PlateReading {
        PlateReading {
            out: IVec3::Y,
            along: IVec3::X,
            fall: IVec3::Z,
            relief,
            coplanar: true,
            enclosure: 8,
            run: 4,
            border: 2,
            height: 2,
            depth: 3,
            fitting: 3,
        }
    }

    fn piece(region: FixtureRegion, collider: Vec3) -> StyleFixtureConfig {
        StyleFixtureConfig {
            id: "piece".to_string(),
            collider,
            placement: FixturePlacement {
                region,
                ..default()
            },
            ..default()
        }
    }

    /// Each region admits the relief it is NAMED for, and the seat gate is on
    /// everywhere but the high ground.
    ///
    /// The one table worth pinning by hand: a region is a promise about where a
    /// piece lands, and the expansion is the only thing that keeps it.
    #[test]
    fn every_region_admits_the_part_of_a_hull_it_names() {
        let admits =
            |region: FixtureRegion, relief: PlateRelief| region.rule().accepts(&reading(relief));

        assert!(admits(FixtureRegion::Panel, PlateRelief::Flat));
        assert!(admits(FixtureRegion::Panel, PlateRelief::Bevel));
        assert!(!admits(FixtureRegion::Panel, PlateRelief::Brink));

        assert!(admits(FixtureRegion::Deck, PlateRelief::Step));
        assert!(!admits(FixtureRegion::Deck, PlateRelief::Spur));

        assert!(admits(FixtureRegion::Edge, PlateRelief::Brink));
        assert!(!admits(FixtureRegion::Edge, PlateRelief::Flat));

        assert!(admits(FixtureRegion::HighGround, PlateRelief::Peak));
        assert!(admits(FixtureRegion::HighGround, PlateRelief::Spur));
        assert!(!admits(FixtureRegion::HighGround, PlateRelief::Flat));

        // The filler takes any relief at all, which is what makes it the filler.
        for relief in crate::sections::skin_reading::RELIEFS {
            assert!(
                admits(FixtureRegion::Anywhere, relief),
                "the filler refused {relief:?}",
            );
        }
    }

    /// The high ground is the ONE region that drops the seat gate, and it has
    /// to: a crest, a spar tip and a stud are cones every time, so a whole-seat
    /// gate would refuse every plate the region exists for.
    ///
    /// It must also ask nothing of the plate's HEIGHT for the same reason - a
    /// spur fills so little of its cell that it measures 0 - and a regression
    /// here empties the silhouette of a ship without emptying any other region.
    #[test]
    fn only_the_high_ground_takes_a_creased_top_and_it_measures_no_height() {
        let creased = PlateReading {
            coplanar: false,
            height: 0,
            ..reading(PlateRelief::Spur)
        };

        assert!(
            FixtureRegion::HighGround.rule().accepts(&creased),
            "the high ground refused the pointiest plate on a hull",
        );
        for region in REGIONS {
            if region == FixtureRegion::HighGround {
                continue;
            }
            assert!(
                !region.rule().accepts(&creased),
                "{region:?} seated a flat-bottomed piece on a crease",
            );
        }
    }

    /// No seat-gated region is made ENTIRELY of cones.
    ///
    /// The relief list says which ZONE of a hull a piece belongs in; the seat
    /// says whether there is a surface to lie on. `Bevel`, `Ridge`, `Peak` and
    /// `Spur` are creased every time, so a seat-gated region naming only those
    /// would refuse every plate it admits - a piece that lands NOWHERE while
    /// looking perfectly well authored, which is how these lists were written
    /// before the seat existed.
    ///
    /// `Panel` naming `Bevel` is the deliberate case and not an exception: the
    /// seat gate is read off the PIECE, so a decal reaches a bevelled panel and
    /// a stack does not, which is exactly the split that field is for.
    #[test]
    fn no_seat_gated_region_is_made_entirely_of_creased_plate() {
        let seats = |relief: &PlateRelief| {
            matches!(
                relief,
                PlateRelief::Flat | PlateRelief::Brink | PlateRelief::Step
            )
        };
        for region in REGIONS {
            let rule = region.rule();
            if !rule.whole_seat || rule.relief.is_empty() {
                continue;
            }
            assert!(
                rule.relief.iter().any(seats),
                "{region:?} gates the seat and names only cones, so it can never                  fire for a piece that stands proud",
            );
        }
    }

    /// A region that wants real ship under it says so, and the thin half of a
    /// hull is what that refuses.
    #[test]
    fn the_laydown_regions_refuse_skin_over_a_one_cell_spar() {
        let thin = PlateReading {
            depth: 1,
            ..reading(PlateRelief::Flat)
        };
        assert!(!FixtureRegion::Deck.rule().accepts(&thin));
        assert!(FixtureRegion::Panel.rule().accepts(&thin));
        assert!(FixtureRegion::Anywhere.rule().accepts(&thin));
    }

    /// A flank is deck plate turned to face OUT, which is what a window is read
    /// through and a roof hatch is not.
    #[test]
    fn a_flank_is_the_plate_that_faces_out_rather_than_up() {
        let roof = reading(PlateRelief::Flat);
        let side = PlateReading {
            out: IVec3::X,
            ..roof
        };
        assert!(!FixtureRegion::Flank.rule().accepts(&roof));
        assert!(FixtureRegion::Flank.rule().accepts(&side));
        // ...and the high ground is the opposite claim: the roof, never a flank.
        let peak = PlateReading {
            out: IVec3::X,
            coplanar: false,
            ..reading(PlateRelief::Peak)
        };
        assert!(!FixtureRegion::HighGround.rule().accepts(&peak));
    }

    /// The service region is the one that measures a DISTANCE rather than a
    /// shape: touching the mouth of a fitting, not merely near one.
    #[test]
    fn the_fitting_region_wants_the_plate_against_the_mouth() {
        let rule = FixtureRegion::NearFitting.rule();
        let mut beside = reading(PlateRelief::Flat);
        beside.fitting = 1;
        assert!(rule.accepts(&beside));
        assert!(
            !rule.accepts(&reading(PlateRelief::Flat)),
            "a plate three cells from any fitting took the service kit",
        );
    }

    /// The ladder is ordered on the dial that actually separates the rungs, and
    /// the floor rises as the share falls - a thin share on a small hull
    /// otherwise lands nothing at all.
    #[test]
    fn the_density_ladder_thins_the_share_and_raises_the_floor() {
        let rules: Vec<(u8, f32, u8)> = DENSITIES.iter().map(|rung| rung.rule()).collect();

        for pair in rules.windows(2) {
            assert!(
                pair[0].1 <= pair[1].1,
                "the share goes backwards up the ladder: {rules:?}",
            );
        }
        assert!(
            (0.0..rules[1].1).contains(&rules[0].1),
            "`Rare` is a real share, and thinner than `Sparse`: {rules:?}",
        );
        assert_eq!(rules[4], (1, 1.0, 0), "`Every` is every plate, off lattice");

        // A block smaller than the lattice can hold no cell for the floor to
        // pick, so a rung whose patch fell under its own stride would silently
        // stop flooring anything.
        for (stride, _, patch) in rules {
            assert!(
                patch == 0 || patch >= stride,
                "a patch of {patch} cannot hold a lattice of {stride}",
            );
        }
    }

    /// TRIM is free of the seat gate whatever region it is in, and that is what
    /// keeps a HAND-BUILT hull dressed: such a hull is one cell thick nearly
    /// everywhere, so nothing on it is coplanar and every seated region reaches
    /// exactly nothing.
    ///
    /// The measured case this exists for: on the owner's own L, no plate at all
    /// has a seat, so a decal that needed one would land on a ship NOWHERE while
    /// looking perfectly well authored.
    #[test]
    fn trim_lies_on_a_crease_and_anything_standing_proud_does_not() {
        let creased = PlateReading {
            coplanar: false,
            ..reading(PlateRelief::Flat)
        };

        // A livery patch: three centimetres of sticker on the plate.
        let decal = piece(FixtureRegion::Anywhere, Vec3::new(0.3, 0.03, 0.46));
        assert!(!decal.scatter().whole_seat);
        assert!(
            decal.scatter().accepts(&creased),
            "a decal was kept off a hand-built hull's only kind of plate",
        );

        // A stack: it would touch a crease along one line and float at both ends.
        let stack = piece(FixtureRegion::Anywhere, Vec3::new(0.18, 0.28, 0.18));
        assert!(stack.scatter().whole_seat);
        assert!(!stack.scatter().accepts(&creased));

        assert!(!FixturePlacement::needs_seat(Vec3::new(0.3, 0.0, 0.3)));
        assert!(FixturePlacement::needs_seat(Vec3::new(
            0.3,
            TRIM_HEIGHT,
            0.3
        )));
    }

    /// The run gate is read off the piece and nothing else: a strip most of a
    /// cell long asks for neighbours, a stud asks for nothing.
    #[test]
    fn a_long_piece_asks_for_a_long_run_and_a_stud_asks_for_none() {
        // The two in-plane axes only - `y` stands off the plate.
        assert_eq!(FixturePlacement::min_run(Vec3::new(0.2, 0.9, 0.2)), 1);
        assert_eq!(FixturePlacement::min_run(Vec3::new(0.19, 0.03, 0.9)), 2);
        assert_eq!(FixturePlacement::min_run(Vec3::new(0.9, 0.03, 0.19)), 2);
        assert_eq!(FixturePlacement::min_run(Vec3::new(0.18, 0.28, 0.18)), 1);

        // However big a mod's collider, the gate stays inside what a hull can
        // offer: a run saturates at RUN_REACH, and a gate above it is a piece
        // that lands nowhere with nothing on screen to say why.
        assert_eq!(
            FixturePlacement::min_run(Vec3::splat(40.0)),
            MAX_DERIVED_RUN
        );

        // And it reaches the rule the scatter actually runs.
        let band = piece(FixtureRegion::Edge, Vec3::new(0.19, 0.03, 0.9));
        assert_eq!(band.scatter().min_run, 2);
        let short = PlateReading {
            run: 1,
            ..reading(PlateRelief::Brink)
        };
        assert!(
            !band.scatter().accepts(&short),
            "a strip took a one-cell stub of edge",
        );
    }

    /// A fixture's scatter is its region, its rung and its collider, with
    /// nothing else reaching in - and the orientation reaches none of it.
    ///
    /// The turn is deliberately NOT part of the rule: it decides how a piece is
    /// posed on the plate it won, never which plate it wins, so a region and a
    /// rung must expand the same way whichever way the piece is turned. That
    /// the turn then reaches the pose is `skin_decor`'s two alignment tests.
    #[test]
    fn a_placement_expands_to_its_region_its_rung_and_its_turn() {
        for region in REGIONS {
            for density in DENSITIES {
                for orientation in [
                    FixtureOrientation::Free,
                    FixtureOrientation::Along,
                    FixtureOrientation::Outward,
                ] {
                    let fixture = StyleFixtureConfig {
                        placement: FixturePlacement {
                            region,
                            density,
                            orientation,
                        },
                        collider: Vec3::splat(0.2),
                        ..default()
                    };
                    let rule = fixture.scatter();
                    let (stride, chance, patch) = density.rule();
                    let placed = region.rule();

                    assert_eq!(
                        (rule.stride, rule.chance, rule.patch),
                        (stride, chance, patch)
                    );
                    assert_eq!(rule.relief, placed.relief, "{region:?} lost its relief");
                    assert_eq!(rule.whole_seat, placed.whole_seat);
                    assert_eq!(rule.facing, placed.facing);
                    assert_eq!(rule.min_depth, placed.min_depth);
                    assert_eq!(rule.min_height, placed.min_height);
                }
            }
        }
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
