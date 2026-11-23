//! FACTION PALETTES: the colour and greeble habit a civilization puts on the
//! styles its ships wear.
//!
//! A style ([`ShipStyleConfig`]) holds the plating colour and the greeble kit of
//! one role. A palette is content beside it: one plate finish per role and one
//! for stations, a greeble density and a list of accent pieces. A generated ship
//! names its style AND its civilization's palette ([`ShipStyle::palette`]); the
//! skin then wears the palette's finish for that role in place of the style's
//! own, and solves the style's greebles with the palette's habit applied
//! ([`SkinLook::worn_style`]).
//!
//! No palette is a defined look, not a fallback: the style's own palette and
//! greebles. Start hulls, scenario ships and the editor wear that.
//!
//! The palette is picked by the world generator, not here, and is written into
//! the ship design, so a saved ship keeps it with no field of its own.

use std::{borrow::Cow, fmt, ops::RangeInclusive};

use bevy::{color::Hsla, prelude::*};
use nova_gameplay::prelude::ShipRoleType;

use crate::sections::{
    greeble_solver::{GreebleConfig, GreebleNearConfig, GreebleRotateType},
    shell_shape::ShellSurface,
    skin_style::{GameStyles, ShipStyle, ShipStyleConfig, StylePalette, SurfaceFinish},
};

/// The palette content type, its parts, the resolved look of one ship and the
/// palette lint.
pub mod prelude {
    pub use super::{
        FactionPaletteConfig, FactionPaletteDesignId, FactionPaletteFault, GreebleAccentConfig,
        PaletteRoleType, PaletteRolesConfig, ShipPaletteConfig, SkinLook, DECOR_KEY_COLOR,
        GREEBLE_DENSITY_RANGE, HOSTILE_BAND_DEGREES, MAGENTA_BAND_DEGREES, MIN_FACTION_PALETTES,
    };
}

/// The fewest palettes a merged catalog may hold.
///
/// The world picks a civilization's palette as `order[(x + 2y + 4z) mod N]` on
/// its lattice node. For a neighbour step in `{-1, 0, 1}^3` the index moves by
/// `-7..=7` and never by 0, so with at least eight palettes no two of the 26
/// neighbours of a node share one.
pub const MIN_FACTION_PALETTES: usize = 8;

/// The range a palette's greeble density may take. The weight of every empty
/// greeble of a style is divided by it, so 2 is about twice as busy and 0.5
/// about half. The upper bound keeps the greeble entity count of a large hull
/// (one collider per piece) inside twice the style's own.
pub const GREEBLE_DENSITY_RANGE: RangeInclusive<f32> = 0.5..=2.0;

/// How close, in degrees of hue, a palette colour may come to the HUD hostile
/// red (`nova_ui::theme::semantic::THREAT`) before lint refuses it. A ship's
/// colour must never read as a stance: stance is per player and changes.
pub const HOSTILE_BAND_DEGREES: f32 = 25.0;

/// How close, in degrees of hue, a palette colour may come to the greeble key
/// magenta ([`DECOR_KEY_COLOR`]) before lint refuses it: a missed tint must stay
/// visible against every hull.
pub const MAGENTA_BAND_DEGREES: f32 = 20.0;

/// The HSL saturation (sRGB) under which a colour reads as grey, and has no
/// hue to refuse.
const HUE_SATURATION_FLOOR: f32 = 0.15;

/// The key colour a greeble recipe paints a slot with when the slot must follow
/// the hull. At spawn a tinted piece swaps every material of exactly this base
/// colour for the hull's top finish; a piece that misses its tint shows it.
pub const DECOR_KEY_COLOR: Color = Color::LinearRgba(LinearRgba::new(1.0, 0.0, 1.0, 1.0));

/// The id a palette is named by in the catalog and in a ship design.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct FactionPaletteDesignId(String);

impl FactionPaletteDesignId {
    /// The id as authored.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for FactionPaletteDesignId {
    fn from(id: &str) -> Self {
        Self(id.to_string())
    }
}

impl fmt::Display for FactionPaletteDesignId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Which finish of a palette a hull wears: one per generated ship role, and one
/// for stations.
///
/// Named on the design and not derived from the style, because a station wears
/// the station finish whatever kit its archetype dresses it in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PaletteRoleType {
    /// Unarmed traffic: the lightest tone.
    Civilian,
    /// Miners and haulers.
    Industrial,
    /// Rough fighting ships: dark.
    Scavenger,
    /// Equipped fighting ships: the darkest tone.
    Armored,
    /// Stations, of every archetype.
    Station,
}

impl PaletteRoleType {
    /// Every finish, in the order [`PaletteRolesConfig`] lists them.
    pub const ALL: [Self; 5] = [
        Self::Civilian,
        Self::Industrial,
        Self::Scavenger,
        Self::Armored,
        Self::Station,
    ];
}

impl From<ShipRoleType> for PaletteRoleType {
    fn from(role: ShipRoleType) -> Self {
        match role {
            ShipRoleType::Civilian => Self::Civilian,
            ShipRoleType::Industrial => Self::Industrial,
            ShipRoleType::Scavenger => Self::Scavenger,
            ShipRoleType::Armored => Self::Armored,
        }
    }
}

/// A palette's plate finish for every role and for stations. Every field is
/// required: a palette has no derived or default finish.
#[derive(Clone, Debug, Default, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct PaletteRolesConfig {
    /// Civilian hulls.
    pub civilian: StylePalette,
    /// Industrial hulls.
    pub industrial: StylePalette,
    /// Scavenger hulls.
    pub scavenger: StylePalette,
    /// Armored hulls.
    pub armored: StylePalette,
    /// Stations.
    pub station: StylePalette,
}

impl PaletteRolesConfig {
    /// The finish `role` wears.
    pub fn get(&self, role: PaletteRoleType) -> &StylePalette {
        match role {
            PaletteRoleType::Civilian => &self.civilian,
            PaletteRoleType::Industrial => &self.industrial,
            PaletteRoleType::Scavenger => &self.scavenger,
            PaletteRoleType::Armored => &self.armored,
            PaletteRoleType::Station => &self.station,
        }
    }
}

/// One accent piece a palette adds to every style its civilization's hulls
/// wear: a fixture of any style, placed alone.
///
/// It takes the link points of the wearing style's first empty greeble, so it
/// may stand anywhere an empty plate may, with no copy of itself within two
/// cells. A style with no empty greeble places no accent.
#[derive(Clone, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct GreebleAccentConfig {
    /// The style that authors the piece.
    pub style: String,
    /// The fixture id within that style.
    pub fixture: String,
    /// The greeble weight, against the wearing style's own weights. Above zero.
    pub weight: f32,
    /// The most copies on one hull. At least one.
    pub max_count: u32,
}

/// One civilization look: a plate finish per role and for stations, and a
/// greeble habit. Content, merged by id like a style.
#[derive(Clone, Debug, Default, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct FactionPaletteConfig {
    /// The id a design names this palette by.
    pub id: FactionPaletteDesignId,
    /// The name a picker would show.
    pub name: String,
    /// The plate finish of each role.
    pub roles: PaletteRolesConfig,
    /// How busy the civilization's hulls are, in [`GREEBLE_DENSITY_RANGE`]: the
    /// weight of every empty greeble is divided by it.
    pub greeble_density: f32,
    /// Pieces of other kits the civilization adds to every hull. May be empty.
    pub accents: Vec<GreebleAccentConfig>,
}

/// Why a palette is refused.
#[derive(Clone, Debug, PartialEq)]
pub enum FactionPaletteFault {
    /// A finish colour sits in the HUD hostile-red band.
    HostileHue {
        /// The role whose finish it is.
        role: PaletteRoleType,
        /// `top` or `wall`.
        surface: &'static str,
        /// Its hue, in degrees.
        hue: f32,
    },
    /// A finish colour sits in the greeble key magenta band.
    KeyHue {
        /// The role whose finish it is.
        role: PaletteRoleType,
        /// `top` or `wall`.
        surface: &'static str,
        /// Its hue, in degrees.
        hue: f32,
    },
    /// The greeble density is outside [`GREEBLE_DENSITY_RANGE`].
    Density(f32),
    /// An accent names a style the catalog lacks, or a fixture that style
    /// does not list.
    UnknownAccent {
        /// The style it names.
        style: String,
        /// The fixture it names.
        fixture: String,
    },
    /// An accent names one fixture a second time.
    DuplicateAccent(String),
    /// An accent's weight is not above zero, or its cap is zero.
    AccentWeight(String),
    /// A style's greebles with this palette's habit applied do not compile.
    Habit {
        /// The style.
        style: String,
        /// The compile fault.
        fault: String,
    },
}

impl fmt::Display for FactionPaletteFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HostileHue { role, surface, hue } => write!(
                f,
                "the {role:?} {surface} finish has hue {hue:.0} deg, within \
                 {HOSTILE_BAND_DEGREES} deg of the HUD hostile red"
            ),
            Self::KeyHue { role, surface, hue } => write!(
                f,
                "the {role:?} {surface} finish has hue {hue:.0} deg, within \
                 {MAGENTA_BAND_DEGREES} deg of the greeble key magenta"
            ),
            Self::Density(density) => write!(
                f,
                "greeble_density {density} is outside {}..={}",
                GREEBLE_DENSITY_RANGE.start(),
                GREEBLE_DENSITY_RANGE.end()
            ),
            Self::UnknownAccent { style, fixture } => write!(
                f,
                "accent '{fixture}' of style '{style}': no loaded style lists that fixture"
            ),
            Self::DuplicateAccent(fixture) => write!(f, "accent '{fixture}' is listed twice"),
            Self::AccentWeight(fixture) => write!(
                f,
                "accent '{fixture}' needs a weight above zero and a max_count of at least 1"
            ),
            Self::Habit { style, fault } => {
                write!(f, "style '{style}' with this palette's habit: {fault}")
            }
        }
    }
}

impl std::error::Error for FactionPaletteFault {}

/// The hue distance, in degrees, between `a` and `b` round the colour wheel.
fn hue_distance(a: f32, b: f32) -> f32 {
    let d = (a - b).rem_euclid(360.0);
    d.min(360.0 - d)
}

/// The hue of `color` in degrees, or `None` when it is too grey to have one.
fn hue_of(color: Color) -> Option<f32> {
    let hsla = Hsla::from(color);
    (hsla.saturation >= HUE_SATURATION_FLOOR && hsla.lightness > 0.0 && hsla.lightness < 1.0)
        .then_some(hsla.hue)
}

impl FactionPaletteConfig {
    /// Every fault of this palette against the loaded `styles`: a finish in
    /// the hostile or key band, a density out of range, an unknown, repeated
    /// or unweighted accent, and a role style whose greebles do not compile
    /// with this habit. Lint and the mod merge both call it.
    pub fn faults(&self, styles: &GameStyles) -> Vec<FactionPaletteFault> {
        let mut faults = Vec::new();
        let hostile = Hsla::from(nova_ui::theme::semantic::THREAT).hue;
        let key = Hsla::from(DECOR_KEY_COLOR).hue;
        for role in PaletteRoleType::ALL {
            let finish = self.roles.get(role);
            for (surface, colour) in [("top", finish.top.color), ("wall", finish.wall.color)] {
                let Some(hue) = hue_of(colour) else {
                    continue;
                };
                if hue_distance(hue, hostile) < HOSTILE_BAND_DEGREES {
                    faults.push(FactionPaletteFault::HostileHue { role, surface, hue });
                }
                if hue_distance(hue, key) < MAGENTA_BAND_DEGREES {
                    faults.push(FactionPaletteFault::KeyHue { role, surface, hue });
                }
            }
        }
        if !GREEBLE_DENSITY_RANGE.contains(&self.greeble_density) {
            faults.push(FactionPaletteFault::Density(self.greeble_density));
        }
        let mut seen = std::collections::BTreeSet::new();
        for accent in &self.accents {
            if !seen.insert(accent.fixture.as_str()) {
                faults.push(FactionPaletteFault::DuplicateAccent(accent.fixture.clone()));
            }
            if accent.weight.is_nan() || accent.weight <= 0.0 || accent.max_count == 0 {
                faults.push(FactionPaletteFault::AccentWeight(accent.fixture.clone()));
            }
            if accent_fixture(styles, accent).is_none() {
                faults.push(FactionPaletteFault::UnknownAccent {
                    style: accent.style.clone(),
                    fixture: accent.fixture.clone(),
                });
            }
        }
        if faults.is_empty() {
            for style in styles.iter() {
                let look = SkinLook {
                    catalog: styles,
                    style,
                    palette: Some((self, PaletteRoleType::Civilian)),
                };
                if let Err(errors) = look.worn_style().compile_greebles() {
                    faults.extend(errors.into_iter().map(|error| FactionPaletteFault::Habit {
                        style: style.id.clone(),
                        fault: error.to_string(),
                    }));
                }
            }
        }
        faults
    }
}

/// The fixture `accent` names, out of `styles`.
fn accent_fixture<'a>(
    styles: &'a GameStyles,
    accent: &GreebleAccentConfig,
) -> Option<&'a crate::sections::skin_style::StyleFixtureConfig> {
    styles
        .get_style(&accent.style)?
        .fixtures
        .iter()
        .find(|fixture| fixture.id == accent.fixture)
}

/// The palette a hull wears, as its design and its [`ShipStyle`] name it: the
/// palette id and which of its finishes.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct ShipPaletteConfig {
    /// The palette.
    pub id: FactionPaletteDesignId,
    /// Which of its finishes the hull wears.
    pub role: PaletteRoleType,
}

/// What one ship's skin wears: its style, and the palette over it when the
/// ship names one. Resolved from a [`ShipStyle`] by [`ShipStyle::resolve`].
#[derive(Clone, Copy, Debug)]
pub struct SkinLook<'a> {
    /// The catalog the look resolved against, which also holds every accent's
    /// fixture.
    pub catalog: &'a GameStyles,
    /// The style.
    pub style: &'a ShipStyleConfig,
    /// The palette and the finish of it the hull wears, or `None` for the
    /// style's own palette and greebles.
    pub palette: Option<(&'a FactionPaletteConfig, PaletteRoleType)>,
}

impl<'a> SkinLook<'a> {
    /// The plate finish of `surface`: the palette's finish for the role when
    /// there is a palette, the style's own otherwise; `None` for the cell
    /// floor, which keeps the engine's finish.
    pub fn finish(&self, surface: ShellSurface) -> Option<&'a SurfaceFinish> {
        match self.palette {
            Some((palette, role)) => {
                let finish = palette.roles.get(role);
                match surface {
                    ShellSurface::Top => Some(&finish.top),
                    ShellSurface::Wall => Some(&finish.wall),
                    ShellSurface::Floor => None,
                }
            }
            None => self.style.finish(surface),
        }
    }

    /// The key the plate materials of this look are cached by.
    pub fn key(&self) -> ShipStyle {
        ShipStyle {
            style: Some(self.style.id.clone()),
            palette: self.palette.map(|(palette, role)| ShipPaletteConfig {
                id: palette.id.clone(),
                role,
            }),
        }
    }

    /// The style as this hull wears it: with a palette, the palette's finish
    /// in place of the style's, every empty greeble's weight divided by the
    /// palette's density, and every accent added as a lone greeble. Without
    /// one, the style itself.
    ///
    /// The decoration solve runs on this, so its fixture indices index this
    /// style's fixtures, not the authored style's.
    ///
    /// # Panics
    ///
    /// When an accent names a fixture the catalog lacks: palette lint and the
    /// mod merge refuse such a palette first.
    pub fn worn_style(&self) -> Cow<'a, ShipStyleConfig> {
        let Some((palette, role)) = self.palette else {
            return Cow::Borrowed(self.style);
        };
        let mut worn = self.style.clone();
        worn.palette = palette.roles.get(role).clone();
        if worn.greeble_rules.greebles.is_empty() {
            return Cow::Owned(worn);
        }
        let empty_links = worn
            .greeble_rules
            .greebles
            .iter()
            .find(|greeble| greeble.fixture.is_none())
            .map(|greeble| greeble.link_points.clone());
        for greeble in &mut worn.greeble_rules.greebles {
            if greeble.fixture.is_none() {
                greeble.weight /= palette.greeble_density;
            }
        }
        let Some(links) = empty_links else {
            return Cow::Owned(worn);
        };
        for accent in &palette.accents {
            let fixture = accent_fixture(self.catalog, accent).unwrap_or_else(|| {
                panic!(
                    "SkinLook::worn_style: palette '{}' accent '{}' of style '{}' is not loaded",
                    palette.id, accent.fixture, accent.style
                )
            });
            if !worn.fixtures.iter().any(|known| known.id == fixture.id) {
                worn.fixtures.push(fixture.clone());
            }
            let id = format!("accent_{}", accent.fixture);
            worn.greeble_rules.greebles.push(GreebleConfig {
                id: id.as_str().into(),
                fixture: Some(accent.fixture.clone()),
                weight: accent.weight,
                rotate: GreebleRotateType::All,
                link_points: links.clone(),
                near: vec![GreebleNearConfig {
                    greebles: vec![id.as_str().into()],
                    radius: 2,
                    factor: 0.0,
                }],
                max_count: Some(accent.max_count),
            });
        }
        Cow::Owned(worn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sections::skin_style::block_style;

    fn finish(color: Color) -> StylePalette {
        StylePalette {
            top: SurfaceFinish {
                color,
                roughness: 0.5,
                metallic: 0.0,
            },
            wall: SurfaceFinish {
                color: Color::BLACK,
                roughness: 0.5,
                metallic: 0.0,
            },
        }
    }

    fn palette(color: Color) -> FactionPaletteConfig {
        FactionPaletteConfig {
            id: "test".into(),
            name: "Test".to_string(),
            roles: PaletteRolesConfig {
                civilian: finish(color),
                industrial: finish(color),
                scavenger: finish(color),
                armored: finish(color),
                station: finish(color),
            },
            greeble_density: 1.0,
            accents: Vec::new(),
        }
    }

    fn catalog() -> GameStyles {
        GameStyles::new(vec![block_style(Vec3::splat(0.2))], Vec::new())
    }

    /// A teal palette passes; one whose plating is the HUD hostile red is
    /// refused, so a ship colour never reads as a stance.
    #[test]
    fn a_palette_in_the_hostile_red_band_is_refused() {
        let styles = catalog();
        assert!(palette(Color::hsl(180.0, 0.4, 0.4))
            .faults(&styles)
            .is_empty());
        let faults = palette(Color::hsl(8.0, 0.5, 0.4)).faults(&styles);
        assert!(
            faults
                .iter()
                .any(|fault| matches!(fault, FactionPaletteFault::HostileHue { .. })),
            "{faults:?}"
        );
    }

    /// A palette near the greeble key magenta is refused, so a piece that
    /// missed its tint always shows.
    #[test]
    fn a_palette_in_the_key_magenta_band_is_refused() {
        let faults = palette(Color::hsl(300.0, 0.6, 0.4)).faults(&catalog());
        assert!(
            faults
                .iter()
                .any(|fault| matches!(fault, FactionPaletteFault::KeyHue { .. })),
            "{faults:?}"
        );
    }

    /// An accent naming a fixture no loaded style lists is refused, and a
    /// density outside the stated range is refused.
    #[test]
    fn an_unknown_accent_piece_and_a_density_out_of_range_are_refused() {
        let mut bad = palette(Color::hsl(180.0, 0.4, 0.4));
        bad.accents.push(GreebleAccentConfig {
            style: "test".to_string(),
            fixture: "nowhere".to_string(),
            weight: 1.0,
            max_count: 2,
        });
        bad.greeble_density = 5.0;
        let faults = bad.faults(&catalog());
        assert!(faults.contains(&FactionPaletteFault::UnknownAccent {
            style: "test".to_string(),
            fixture: "nowhere".to_string(),
        }));
        assert!(faults.contains(&FactionPaletteFault::Density(5.0)));
    }

    /// A worn style takes the palette's finish for its role, scales its empty
    /// greeble by the density and places the accents; with no palette it is
    /// the style itself.
    #[test]
    fn a_palette_dresses_the_style_with_its_finish_and_habit() {
        let mut habit = palette(Color::hsl(180.0, 0.4, 0.4));
        habit.roles.armored = finish(Color::hsl(120.0, 0.4, 0.2));
        habit.greeble_density = 2.0;
        habit.accents.push(GreebleAccentConfig {
            style: "test".to_string(),
            fixture: "block".to_string(),
            weight: 0.5,
            max_count: 3,
        });
        let styles = catalog();
        let style = styles.get_style("test").expect("the block style");
        let plain = SkinLook {
            catalog: &styles,
            style,
            palette: None,
        };
        assert!(matches!(plain.worn_style(), Cow::Borrowed(_)));

        let look = SkinLook {
            palette: Some((&habit, PaletteRoleType::Armored)),
            ..plain
        };
        let worn = look.worn_style();
        assert_eq!(
            worn.palette.top.color, habit.roles.armored.top.color,
            "the hull wears the palette's armored finish"
        );
        assert_eq!(
            look.finish(ShellSurface::Top).map(|finish| finish.color),
            Some(habit.roles.armored.top.color)
        );
        let empty = &worn.greeble_rules.greebles[0];
        assert!((empty.weight - 0.0005).abs() < 1e-7, "{}", empty.weight);
        let accent = worn
            .greeble_rules
            .greebles
            .iter()
            .find(|greeble| greeble.id.as_str() == "accent_block")
            .expect("the accent is placed");
        assert_eq!(accent.max_count, Some(3));
        assert!(worn
            .compile_greebles()
            .expect("the habit compiles")
            .is_some());
    }
}
