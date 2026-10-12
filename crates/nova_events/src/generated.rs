//! The world grid cell and the id of a body a generator placed in one.
//!
//! Both live here, below `nova_scenario` and `nova_world`, because the two
//! crates sit the wrong way round for the id to live with its minter:
//! `nova_world` MINTS a [`GeneratedBodyId`] and depends on `nova_scenario`,
//! while `nova_scenario` RESOLVES one for an objective marker and lints an
//! authored literal that spells one. This is the deepest crate both reach,
//! the same reason [`EntityId`](crate::EntityId) and
//! [`ScenarioAddressableMarker`](crate::ScenarioAddressableMarker) are here.

use std::{fmt, str::FromStr};

use bevy::prelude::*;

use crate::units::prelude::{Meters, Meters3};

/// An integer sector coordinate: which cell of the world grid, never where in
/// meters.
///
/// `Ord` is derived so a desired set and a live set are both ordered
/// collections. Generation must not depend on iteration order, but a REPORT
/// that lists sectors in hash order is unreadable.
#[derive(
    Component,
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(deny_unknown_fields)]
pub struct SectorCoord {
    /// Cell index along +X.
    pub x: i32,
    /// Cell index along +Y.
    pub y: i32,
    /// Cell index along +Z.
    pub z: i32,
}

impl SectorCoord {
    /// The cell the world origin falls in.
    pub const ORIGIN: Self = Self { x: 0, y: 0, z: 0 };

    /// The cell at these indices.
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    /// The cell `position` falls in.
    ///
    /// A cell is CENTRED on its coordinate - cell `n` spans
    /// `[n * edge - edge/2, n * edge + edge/2)` - rather than cornered at it.
    /// That keeps cell (0, 0, 0) around the world origin, which is where the
    /// scenario loader parks a cameraless scene's free-fly camera: a cornered
    /// grid would open every run looking out of the field instead of into it.
    ///
    /// `floor` after the half-cell shift rather than `round`: `f32::round`
    /// goes half AWAY FROM ZERO, so it put `-edge/2` in cell -1 while the span
    /// above says cell 0 owns it. That broke the interval on the negative face
    /// only, and the face a position lands on decides which cell owns what
    /// stands there.
    pub fn containing(position: Meters3, edge: Meters) -> Self {
        let cell = |meters: Meters| (meters.get() / edge.get() + 0.5).floor() as i32;
        Self {
            x: cell(position.x()),
            y: cell(position.y()),
            z: cell(position.z()),
        }
    }

    /// The cell's centre, in meters.
    pub fn centre(self, edge: Meters) -> Meters3 {
        let axis = |index: i32| index as f32 * edge.get();
        Meters3::new(axis(self.x), axis(self.y), axis(self.z))
    }

    /// The cell this many cells away.
    pub const fn offset(self, x: i32, y: i32, z: i32) -> Self {
        Self {
            x: self.x + x,
            y: self.y + y,
            z: self.z + z,
        }
    }

    /// The cell's stable id text, and the prefix every body it owns is named
    /// from. `n` reads as minus, because a `-` in a scenario id is not a
    /// snake_case slug.
    pub fn slug(self) -> String {
        format!(
            "{SECTOR_SLUG_PREFIX}{}_{}_{}",
            index_slug(self.x),
            index_slug(self.y),
            index_slug(self.z)
        )
    }
}

impl fmt::Display for SectorCoord {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "({}, {}, {})", self.x, self.y, self.z)
    }
}

/// What every cell slug, and so every generated id, starts with.
const SECTOR_SLUG_PREFIX: &str = "sector_";

/// One signed cell index as a snake_case-safe slug. `n` reads as minus,
/// because a `-` in a scenario id is not a slug.
fn index_slug(index: i32) -> String {
    if index < 0 {
        format!("n{}", index.unsigned_abs())
    } else {
        index.to_string()
    }
}

/// The inverse of [`index_slug`]: only the spelling it writes, so `-0`, a
/// leading zero or a sign other than `n` is not a cell index.
fn parse_index_slug(slug: &str) -> Option<i32> {
    let (negative, digits) = match slug.strip_prefix('n') {
        Some(digits) => (true, digits),
        None => (false, slug),
    };
    let magnitude = parse_canonical_u32(digits)?;
    if negative {
        if magnitude == 0 {
            return None;
        }
        i32::try_from(magnitude).ok().map(|value| -value)
    } else {
        i32::try_from(magnitude).ok()
    }
}

/// A `u32` written the one way `Display` writes it: digits only, no leading
/// zero except for zero itself.
fn parse_canonical_u32(digits: &str) -> Option<u32> {
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    if digits.len() > 1 && digits.starts_with('0') {
        return None;
    }
    digits.parse().ok()
}

/// The id of one body a generator placed in a cell: the cell it was generated
/// in, the generator's stem for what it placed, and the body's index under
/// that stem. `Display` writes it as the scenario id the body wears,
/// `<cell slug>_<stem>_<index>`, and [`FromStr`] reads exactly that spelling
/// back, so a payload string that names a generated body round-trips to the
/// typed id and nothing an author writes by hand does by accident.
///
/// The `origin` is the cell that GENERATED the body, fixed for its life: a
/// ship that drifts into another cell is adopted by that cell's root and
/// frozen in its record, but keeps the id it was minted with. One cell never
/// mints one id twice; the manifest validation refuses the repeat.
///
/// Worn as a component beside the body's [`EntityId`](crate::EntityId) on
/// every streamed body, at spawn and at thaw. It is the lookup key a scenario
/// action resolves a generated body by: such a body is never
/// [`ScenarioAddressableMarker`](crate::ScenarioAddressableMarker), because
/// `nova_world` reads that marker's absence as "mine to freeze".
#[derive(Component, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GeneratedBodyId {
    /// The cell the generator placed the body in.
    pub origin: SectorCoord,
    /// The generator's name for what it placed, a snake_case word or words:
    /// `background_rock`, `cluster_1_0_n1_hull`.
    pub stem: String,
    /// Which body of that stem in that cell.
    pub index: u32,
}

impl GeneratedBodyId {
    /// The id of body `index` of `stem` in `origin`.
    ///
    /// # Panics
    ///
    /// When `stem` is empty or is not snake_case (`a-z`, `0-9`, `_`): the id
    /// would not read back from its own spelling. A generator's stems are
    /// fixed text, so this is an assembly fault, not a runtime condition.
    pub fn new(origin: SectorCoord, stem: impl Into<String>, index: u32) -> Self {
        let stem = stem.into();
        assert!(
            is_snake_case(&stem),
            "nova_events: a generated body stem must be snake_case, not '{stem}'"
        );
        Self {
            origin,
            stem,
            index,
        }
    }
}

/// Whether `text` is one or more `a-z`, `0-9` or `_` characters.
fn is_snake_case(text: &str) -> bool {
    !text.is_empty()
        && text
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

impl fmt::Display for GeneratedBodyId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}_{}_{}",
            self.origin.slug(),
            self.stem,
            self.index
        )
    }
}

impl From<GeneratedBodyId> for crate::EntityId {
    fn from(id: GeneratedBodyId) -> Self {
        crate::EntityId(id.to_string())
    }
}

/// Why a string is not a [`GeneratedBodyId`]. Carries no detail on purpose:
/// every caller asks "is this a generated id" and treats a `No` alike.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NotAGeneratedBodyId;

impl fmt::Display for NotAGeneratedBodyId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("not a generated body id")
    }
}

impl std::error::Error for NotAGeneratedBodyId {}

impl FromStr for GeneratedBodyId {
    type Err = NotAGeneratedBodyId;

    /// Read `<cell slug>_<stem>_<index>` back. The cell is the first three
    /// `_`-separated fields after `sector_`, the index is the last field, and
    /// the stem is everything between, so a stem may itself hold `_`.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let rest = text
            .strip_prefix(SECTOR_SLUG_PREFIX)
            .ok_or(NotAGeneratedBodyId)?;
        let mut fields = rest.splitn(4, '_');
        let mut axis = || {
            fields
                .next()
                .and_then(parse_index_slug)
                .ok_or(NotAGeneratedBodyId)
        };
        let origin = SectorCoord::new(axis()?, axis()?, axis()?);
        let body = fields.next().ok_or(NotAGeneratedBodyId)?;
        let (stem, index) = body.rsplit_once('_').ok_or(NotAGeneratedBodyId)?;
        if !is_snake_case(stem) {
            return Err(NotAGeneratedBodyId);
        }
        let index = parse_canonical_u32(index).ok_or(NotAGeneratedBodyId)?;
        Ok(Self {
            origin,
            stem: stem.to_string(),
            index,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every stem the base generator mints, with the cell spellings that
    /// exercise both signs: a cluster body under its lattice-node stem, an
    /// escort under the hull it escorts, and a background rock.
    const MINTED_STEMS: [&str; 5] = [
        "cluster_1_0_n1_parent",
        "cluster_1_0_n1_rock",
        "cluster_1_0_n1_hull",
        "cluster_1_0_n1_hull_0_escort",
        "background_rock",
    ];

    #[test]
    fn every_minted_stem_round_trips_through_its_string() {
        for stem in MINTED_STEMS {
            for origin in [
                SectorCoord::ORIGIN,
                SectorCoord::new(-2, 0, 0),
                SectorCoord::new(3, -17, 2_000_000),
            ] {
                for index in [0, 1, 42] {
                    let id = GeneratedBodyId::new(origin, stem, index);
                    let text = id.to_string();
                    assert_eq!(text, format!("{}_{stem}_{index}", origin.slug()));
                    assert_eq!(text.parse::<GeneratedBodyId>(), Ok(id), "{text}");
                }
            }
        }
    }

    #[test]
    fn todays_spelling_is_kept() {
        assert_eq!(
            GeneratedBodyId::new(SectorCoord::new(-2, 0, 1), "background_rock", 3).to_string(),
            "sector_n2_0_1_background_rock_3"
        );
    }

    #[test]
    fn an_authored_or_cell_id_is_not_a_generated_body_id() {
        for text in [
            "",
            "beacon_1",
            "player_spaceship",
            "sector_0_0_0",
            "sector_0_0_0_rock",
            "sector_0_0_0__3",
            "sector_0_0_0_rock_",
            "sector_0_0_0_rock_03",
            "sector_0_0_0_rock_-1",
            "sector_n0_0_0_rock_1",
            "sector_00_0_0_rock_1",
            "sector_a_0_0_rock_1",
            "sector_0_0_rock_1",
            "sector_0_0_0_Rock_1",
            "sector_0_0_0_rock_1/2",
            "sectors_0_0_0_rock_1",
        ] {
            assert_eq!(
                text.parse::<GeneratedBodyId>(),
                Err(NotAGeneratedBodyId),
                "'{text}' must not read as a generated id"
            );
        }
    }

    #[test]
    fn a_stem_that_cannot_round_trip_is_refused_at_mint() {
        let result = std::panic::catch_unwind(|| {
            GeneratedBodyId::new(SectorCoord::ORIGIN, "Hull-0", 0);
        });
        assert!(
            result.is_err(),
            "a stem outside snake_case is an assembly fault"
        );
    }
}
