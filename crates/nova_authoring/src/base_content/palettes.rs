//! Built-in FACTION PALETTE content: the hull colours the world generator deals
//! to its civilizations.
//!
//! `content gen` serializes this into `assets/base/palettes/base.content.ron`
//! and a mod overlays it by id. The generator gives each civilization one
//! palette from the catalog by its lattice node, so eight is the floor: below
//! that two neighbours could share one.
//!
//! A palette is one hue. The role sets the tone, the same for every palette:
//! civilian hulls are light and smooth, industrial hulls matte, scavengers dull
//! and dirty, armored hulls dark and metallic, and stations mid-grey of the
//! hue. So the hue says whose hull it is, and the tone says what it does. A
//! palette may scale every tone's lightness by one factor (a dark maroon, a
//! pale bone); the role order stays.
//!
//! Neighbours never share a palette, but any two palettes can be neighbours
//! after the seeded shuffle, so every pair must read apart. Hues near each
//! other are set apart by saturation or lightness.
//!
//! No hue sits within 20 degrees of the greeble key magenta (300): lint refuses
//! that band, and plum and the reds keep more than 30 degrees from it.

use bevy::prelude::Color;
use nova_ship::prelude::{
    FactionPaletteConfig, GreebleAccentConfig, PaletteRoleType, PaletteRolesConfig, StylePalette,
    SurfaceFinish,
};
use nova_world_base::prelude::{
    ARMOURED_STYLE_ID, CIVILIAN_STYLE_ID, INDUSTRIAL_STYLE_ID, SALVAGE_STYLE_ID,
};

/// One role's tone: sRGB HSL saturation and lightness scales for the top
/// surface, and its roughness and metallic.
struct Tone {
    saturation: f32,
    lightness: f32,
    roughness: f32,
    metallic: f32,
}

/// The tone of `role`. The wall takes the top's hue and saturation at a third
/// of its lightness, so the drop at a plate boundary reads as a panel gap, as
/// in every built-in style.
fn tone(role: PaletteRoleType) -> Tone {
    match role {
        PaletteRoleType::Civilian => Tone {
            saturation: 1.0,
            lightness: 0.55,
            roughness: 0.45,
            metallic: 0.1,
        },
        PaletteRoleType::Industrial => Tone {
            saturation: 1.0,
            lightness: 0.40,
            roughness: 0.9,
            metallic: 0.15,
        },
        PaletteRoleType::Scavenger => Tone {
            saturation: 0.6,
            lightness: 0.30,
            roughness: 0.95,
            metallic: 0.2,
        },
        PaletteRoleType::Armored => Tone {
            saturation: 0.8,
            lightness: 0.24,
            roughness: 0.6,
            metallic: 0.5,
        },
        PaletteRoleType::Station => Tone {
            saturation: 0.7,
            lightness: 0.45,
            roughness: 0.7,
            metallic: 0.3,
        },
    }
}

/// The palette's own colour: sRGB HSL hue in degrees, saturation, and a scale
/// on every role's lightness (1.0 keeps the role tones).
#[derive(Clone, Copy)]
struct Hue {
    hue: f32,
    saturation: f32,
    lightness: f32,
}

/// The finish of `role` in the palette of `hue`.
fn finish(hue: Hue, role: PaletteRoleType) -> StylePalette {
    let tone = tone(role);
    let saturation = hue.saturation * tone.saturation;
    let top = tone.lightness * hue.lightness;
    let colour =
        |lightness: f32| Color::LinearRgba(Color::hsl(hue.hue, saturation, lightness).into());
    StylePalette {
        top: SurfaceFinish {
            color: colour(top),
            roughness: tone.roughness,
            metallic: tone.metallic,
        },
        wall: SurfaceFinish {
            color: colour(top / 3.0),
            roughness: (tone.roughness + 0.05).min(1.0),
            metallic: tone.metallic,
        },
    }
}

/// One accent piece of `style`.
fn accent(style: &str, fixture: &str, weight: f32, max_count: u32) -> GreebleAccentConfig {
    GreebleAccentConfig {
        style: style.to_string(),
        fixture: fixture.to_string(),
        weight,
        max_count,
    }
}

/// The palette `id` of `hue` (sRGB HSL degrees) and `saturation` at the role
/// tones, with its greeble habit.
fn palette(
    id: &str,
    name: &str,
    hue: f32,
    saturation: f32,
    greeble_density: f32,
    accents: Vec<GreebleAccentConfig>,
) -> FactionPaletteConfig {
    shaded_palette(id, name, hue, saturation, 1.0, greeble_density, accents)
}

/// The palette `id` of `hue` and `saturation` with every role tone's lightness
/// scaled by `lightness`, with its greeble habit.
fn shaded_palette(
    id: &str,
    name: &str,
    hue: f32,
    saturation: f32,
    lightness: f32,
    greeble_density: f32,
    accents: Vec<GreebleAccentConfig>,
) -> FactionPaletteConfig {
    let hue = Hue {
        hue,
        saturation,
        lightness,
    };
    FactionPaletteConfig {
        id: id.into(),
        name: name.to_string(),
        roles: PaletteRolesConfig {
            civilian: finish(hue, PaletteRoleType::Civilian),
            industrial: finish(hue, PaletteRoleType::Industrial),
            scavenger: finish(hue, PaletteRoleType::Scavenger),
            armored: finish(hue, PaletteRoleType::Armored),
            station: finish(hue, PaletteRoleType::Station),
        },
        greeble_density,
        accents,
    }
}

/// Every built-in palette, in stable generated-content order: fourteen hues
/// round the wheel from maroon to crimson, and one slate grey with no hue.
///
/// Nearby hues are set apart by saturation or lightness: maroon is a dark red
/// and crimson a bright one, plum a dark violet beside indigo, bone a pale
/// grey-yellow beside ochre. There is no pale violet: between indigo and the
/// key band it reads as indigo under the looks light.
pub(crate) fn palette_catalog() -> Vec<FactionPaletteConfig> {
    vec![
        shaded_palette(
            "maroon",
            "Maroon",
            2.0,
            0.45,
            0.7,
            0.9,
            vec![accent(ARMOURED_STYLE_ID, "armoured_applique", 0.4, 2)],
        ),
        palette(
            "rust",
            "Rust",
            18.0,
            0.55,
            1.7,
            vec![accent(SALVAGE_STYLE_ID, "salvage_weld_seam", 0.5, 4)],
        ),
        palette(
            "ochre",
            "Ochre",
            38.0,
            0.45,
            1.3,
            vec![accent(INDUSTRIAL_STYLE_ID, "industrial_floodlight", 0.5, 3)],
        ),
        shaded_palette("bone", "Bone", 45.0, 0.18, 1.3, 0.55, Vec::new()),
        palette(
            "olive",
            "Olive",
            70.0,
            0.35,
            0.8,
            vec![accent(SALVAGE_STYLE_ID, "salvage_drum", 0.4, 2)],
        ),
        palette(
            "moss",
            "Moss",
            125.0,
            0.3,
            1.0,
            vec![accent(CIVILIAN_STYLE_ID, "civilian_dish", 0.3, 2)],
        ),
        shaded_palette(
            "emerald",
            "Emerald",
            140.0,
            0.55,
            1.15,
            0.75,
            vec![accent(CIVILIAN_STYLE_ID, "civilian_skylight", 0.3, 2)],
        ),
        palette("teal", "Teal", 172.0, 0.4, 0.7, Vec::new()),
        shaded_palette(
            "sky",
            "Sky",
            195.0,
            0.5,
            1.25,
            1.25,
            vec![accent(INDUSTRIAL_STYLE_ID, "industrial_radiator", 0.4, 2)],
        ),
        palette(
            "steel",
            "Steel Blue",
            205.0,
            0.3,
            1.2,
            vec![accent(ARMOURED_STYLE_ID, "armoured_sensor", 0.4, 3)],
        ),
        palette(
            "cobalt",
            "Cobalt",
            226.0,
            0.5,
            1.0,
            vec![accent(CIVILIAN_STYLE_ID, "civilian_beacon", 0.4, 4)],
        ),
        palette(
            "indigo",
            "Indigo",
            255.0,
            0.35,
            1.5,
            vec![
                accent(INDUSTRIAL_STYLE_ID, "industrial_stencil", 0.6, 4),
                accent(ARMOURED_STYLE_ID, "armoured_mast", 0.3, 1),
            ],
        ),
        shaded_palette(
            "plum",
            "Plum",
            267.0,
            0.4,
            0.7,
            1.8,
            vec![
                accent(SALVAGE_STYLE_ID, "salvage_hose", 0.4, 2),
                accent(ARMOURED_STYLE_ID, "armoured_chaff", 0.3, 2),
            ],
        ),
        palette(
            "crimson",
            "Crimson",
            352.0,
            0.6,
            1.4,
            vec![
                accent(ARMOURED_STYLE_ID, "armoured_strake", 0.4, 3),
                accent(INDUSTRIAL_STYLE_ID, "industrial_hazard_band", 0.3, 2),
            ],
        ),
        palette(
            "slate",
            "Slate",
            210.0,
            0.08,
            0.6,
            vec![accent(SALVAGE_STYLE_ID, "salvage_grille", 0.4, 2)],
        ),
    ]
}
