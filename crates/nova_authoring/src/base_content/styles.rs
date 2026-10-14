//! Built-in SKIN STYLE content: the looks a ship's derived cladding can wear.
//!
//! A style is content like a section is content, so this file is a builder and
//! not a table of constants - `content gen` serializes it into
//! `assets/base/styles/base.content.ron` and a mod overlays it by id.
//!
//! Five looks ship: `industrial`, `armoured`, `civilian` and `salvage` for
//! ships, and `station` for the ring station. Each lists its fixtures and the
//! greeble rules that place them. The rules share one link-type set
//! ([`link_types`]): `any` pieces may touch each other, `spaced` pieces touch
//! only empty space, and each line type faces only itself, so a line runs on
//! until a border or an end greeble.

use bevy::prelude::{Color, Vec3};
use nova_ship::prelude::{
    GreebleConfig, GreebleLinkConfig, GreebleLinkPoints, GreebleNearConfig, GreebleRotateType,
    GreebleRules, ShipStyleConfig, StyleFixtureConfig, StylePalette, SurfaceFinish,
};
use nova_world_base::prelude::{
    ARMOURED_STYLE_ID, CIVILIAN_STYLE_ID, INDUSTRIAL_STYLE_ID, SALVAGE_STYLE_ID, STATION_STYLE_ID,
};

use super::assets::BaseContentAssets;

/// Every built-in style, in stable generated-content order.
///
/// Neither the `wfc_ships` producer nor the editor's build view knows a style
/// id - both fall back to the FIRST style the merged content offers - so the
/// head of this list is what an unstyled subject wears.
///
/// `industrial` leads because a FALLBACK must be the least specific look here.
/// It is also the widest kit, so it has the most to place on a hull nobody
/// authored a style for.
pub(crate) fn style_catalog(assets: &BaseContentAssets) -> Vec<ShipStyleConfig> {
    vec![
        industrial_style(assets),
        armoured_style(assets),
        civilian_style(assets),
        salvage_style(assets),
        station_style(),
    ]
}

/// INDUSTRIAL: a working hull with its services outside the plating.
///
/// Three moves carry the look, and each must read from across a frame:
///
/// 1. THE PAINT. Safety yellow on the straight edges of the hull, on the collar
///    of every stack and on every hatch handle. One accent used three ways is a
///    system; one accent used once is a colour.
/// 2. THE SEAM. The wall surface is taken about six times darker than the top,
///    so the drop at every plate boundary reads as a black gap between bolted
///    panels rather than as shading. That is the cheapest "exposed panelling"
///    there is, and it costs no geometry at all.
/// 3. THE KIT. Radiators, conduit, corrugation, louvres, hatches, stacks, a
///    crane, battery racks, plate stock, a winch, floodlights, umbilical
///    points and stencilled part numbers. Fourteen pieces, each a serviceable
///    fitting rather than ornament.
///
/// The greebles: stacks and a crane stand alone, the hazard band and the duct
/// run as lines, radiators and ribbing cluster, and the rest fill between.
fn industrial_style(assets: &BaseContentAssets) -> ShipStyleConfig {
    // Some pieces name their art BY ID rather than through a
    // `BaseContentAssets` field. The namespacing test below pins the
    // id-to-path equality either way.
    let greeble = |id: &str| {
        nova_gameplay::prelude::AssetRef::from(format!("self://gltf/greebles/{id}.glb#Scene0"))
    };
    ShipStyleConfig {
        id: INDUSTRIAL_STYLE_ID.to_string(),
        name: "Industrial".to_string(),
        palette: StylePalette {
            top: SurfaceFinish {
                color: Color::linear_rgb(0.200, 0.190, 0.168),
                roughness: 0.9,
                metallic: 0.15,
            },
            wall: SurfaceFinish {
                color: Color::linear_rgb(0.030, 0.028, 0.024),
                roughness: 0.95,
                metallic: 0.1,
            },
        },
        fixtures: vec![
            StyleFixtureConfig {
                id: "industrial_stack".to_string(),
                model: assets.greeble_industrial_stack.clone(),
                health: 10.0,
                collider: Vec3::new(0.18, 0.28, 0.18),
            },
            StyleFixtureConfig {
                id: "industrial_crane".to_string(),
                model: greeble("industrial_crane"),
                health: 12.0,
                collider: Vec3::new(0.16, 0.45, 0.32),
            },
            StyleFixtureConfig {
                id: "industrial_hazard_band".to_string(),
                model: assets.greeble_industrial_hazard_band.clone(),
                health: 6.0,
                collider: Vec3::new(0.19, 0.03, 0.9),
            },
            StyleFixtureConfig {
                id: "industrial_umbilical".to_string(),
                model: greeble("industrial_umbilical"),
                health: 10.0,
                collider: Vec3::new(0.16, 0.13, 0.44),
            },
            StyleFixtureConfig {
                id: "industrial_plate_rack".to_string(),
                model: greeble("industrial_plate_rack"),
                health: 20.0,
                collider: Vec3::new(0.34, 0.1, 0.43),
            },
            StyleFixtureConfig {
                id: "industrial_cells".to_string(),
                model: greeble("industrial_cells"),
                health: 16.0,
                collider: Vec3::new(0.3, 0.21, 0.46),
            },
            StyleFixtureConfig {
                id: "industrial_radiator".to_string(),
                model: assets.greeble_industrial_radiator.clone(),
                health: 14.0,
                collider: Vec3::new(0.38, 0.27, 0.28),
            },
            StyleFixtureConfig {
                id: "industrial_duct".to_string(),
                model: assets.greeble_industrial_duct.clone(),
                health: 9.0,
                collider: Vec3::new(0.14, 0.16, 0.9),
            },
            StyleFixtureConfig {
                id: "industrial_winch".to_string(),
                model: greeble("industrial_winch"),
                health: 14.0,
                collider: Vec3::new(0.23, 0.25, 0.38),
            },
            StyleFixtureConfig {
                id: "industrial_floodlight".to_string(),
                model: greeble("industrial_floodlight"),
                health: 8.0,
                collider: Vec3::new(0.17, 0.25, 0.12),
            },
            StyleFixtureConfig {
                id: "industrial_louvre".to_string(),
                model: assets.greeble_industrial_louvre.clone(),
                health: 12.0,
                collider: Vec3::new(0.32, 0.07, 0.24),
            },
            StyleFixtureConfig {
                id: "industrial_hatch".to_string(),
                model: assets.greeble_industrial_hatch.clone(),
                health: 18.0,
                collider: Vec3::new(0.32, 0.07, 0.32),
            },
            StyleFixtureConfig {
                id: "industrial_stencil".to_string(),
                model: greeble("industrial_stencil"),
                health: 4.0,
                collider: Vec3::new(0.22, 0.02, 0.34),
            },
            StyleFixtureConfig {
                id: "industrial_ribbing".to_string(),
                model: assets.greeble_industrial_ribbing.clone(),
                health: 15.0,
                collider: Vec3::new(0.46, 0.07, 0.42),
            },
        ],
        greeble_rules: industrial_rules(),
    }
}

/// ARMOURED: a hull that shows a gunner as little as it can.
///
/// The point of view, in one sentence: **an armoured hull's feature is its
/// EDGES**, and everything else stays flush. So the kit spends nearly all of
/// itself on one piece - a belt that runs the length of every straight edge the
/// hull has - and the other three exist to keep that belt from being the only
/// thing on the ship.
///
/// The palette is the other half, and it is worth being exact about what it can
/// and cannot do, because it was measured rather than assumed.
///
/// A `Wall` is NOT the seam between two plates, and this was measured rather
/// than reasoned about: a diagnostic run painted the wall surface magenta and
/// photographed the row. Two plates in a run press their walls against each
/// other and neither is ever seen. What came back magenta was the OUTER RIM of
/// the skin - where the cladding ends against space - plus the exposed side of
/// a plate that climbs past a lower neighbour. Perhaps a twentieth of what a
/// camera sees.
///
/// So a large top-to-wall ratio does not draw a panel seam on every step, which
/// is what this style was first written expecting. It is still worth carrying at
/// 6:1, because what it DOES darken is the ship's own outline, and a hard dark
/// rim is what makes the plate above it read thick. That is a smaller claim than
/// the one it replaced and it is the one the render supports.
///
/// The `Top` is where the palette earns its keep, and mostly through HUE: off
/// the built-in blue slate onto a desaturated gunmetal, matte and
/// metallic-free, so the hull carries no specular highlight.
///
/// The kit: ten pieces, the smallest of the four, all low and flush but the
/// sensor and the mast, which stand alone. The belt runs as a line and the
/// applique tiles cluster.
fn armoured_style(assets: &BaseContentAssets) -> ShipStyleConfig {
    ShipStyleConfig {
        id: ARMOURED_STYLE_ID.to_string(),
        name: "Armoured".to_string(),
        // THE PALETTE, and it is half the look. Linear values, so they read
        // considerably lighter on screen than they look here.
        //
        // - the TOP is gunmetal: barely cool, all but desaturated, metallic
        //   zero, and less than half the albedo of the built-in plate. Matte
        //   armour, which presents no highlight to be seen by. The hue is
        //   MEASURED, not guessed: a neutral 0.155 photographs as warm cream,
        //   because the photo rig's key is warm and a neutral plate takes its
        //   colour;
        // - the WALL is a sixth of the top again. A wall is only ever seen
        //   where the cladding ENDS - see the note below - so this is the dark
        //   underside of an armour belt: it tightens the silhouette and makes
        //   the plate above it read thick.
        //
        // How much a palette can actually do here, measured on the row, because
        // it is less than it looks: the value lever is WEAK and the hue lever is
        // strong. Cutting the top's albedo by 2.2x (0.100 -> 0.045) moved a lit
        // flank from 55% to 46% on screen rather than the 45% the albedo says,
        // because the rig's ambient and the tonemapper put a floor under
        // everything. Going darker still buys almost nothing. What the palette
        // bought outright was the HUE - off the built-in blue slate onto
        // gunmetal - and the SEPARATION between plate and greeble: the kit's
        // pieces are 3x the hull's albedo, so a belt reads as a light rail on a
        // dark hull rather than as a lump of the same stuff.
        palette: StylePalette {
            top: SurfaceFinish {
                color: Color::linear_rgb(0.045, 0.048, 0.055),
                roughness: 0.88,
                metallic: 0.0,
            },
            wall: SurfaceFinish {
                color: Color::linear_rgb(0.007, 0.0075, 0.009),
                roughness: 0.95,
                metallic: 0.0,
            },
        },
        fixtures: vec![
            StyleFixtureConfig {
                id: "armoured_sensor".to_string(),
                model: assets.greeble_armoured_sensor.clone(),
                health: 10.0,
                collider: Vec3::new(0.4, 0.12, 0.4),
            },
            StyleFixtureConfig {
                id: "armoured_mast".to_string(),
                model: assets.greeble_armoured_mast.clone(),
                health: 8.0,
                collider: Vec3::new(0.16, 0.24, 0.16),
            },
            StyleFixtureConfig {
                id: "armoured_cap".to_string(),
                model: assets.greeble_armoured_cap.clone(),
                health: 26.0,
                collider: Vec3::new(0.3, 0.1, 0.3),
            },
            StyleFixtureConfig {
                id: "armoured_strake".to_string(),
                model: assets.greeble_armoured_strake.clone(),
                health: 30.0,
                collider: Vec3::new(0.3, 0.085, 0.95),
            },
            StyleFixtureConfig {
                id: "armoured_intake".to_string(),
                model: assets.greeble_armoured_intake.clone(),
                health: 16.0,
                collider: Vec3::new(0.2, 0.06, 0.44),
            },
            StyleFixtureConfig {
                id: "armoured_chaff".to_string(),
                model: assets.greeble_armoured_chaff.clone(),
                health: 12.0,
                collider: Vec3::new(0.3, 0.13, 0.18),
            },
            StyleFixtureConfig {
                id: "armoured_hatch".to_string(),
                model: assets.greeble_armoured_hatch.clone(),
                health: 14.0,
                collider: Vec3::new(0.3, 0.05, 0.3),
            },
            StyleFixtureConfig {
                id: "armoured_ammo_stripes".to_string(),
                model: assets.greeble_armoured_ammo_stripes.clone(),
                health: 6.0,
                collider: Vec3::new(0.3, 0.02, 0.34),
            },
            StyleFixtureConfig {
                id: "armoured_magazine".to_string(),
                model: assets.greeble_armoured_magazine.clone(),
                health: 22.0,
                collider: Vec3::new(0.34, 0.12, 0.28),
            },
            StyleFixtureConfig {
                id: "armoured_applique".to_string(),
                model: assets.greeble_armoured_applique.clone(),
                health: 24.0,
                collider: Vec3::new(0.42, 0.04, 0.42),
            },
        ],
        greeble_rules: armoured_rules(),
    }
}

/// The CIVILIAN look: painted and faired, with nothing mechanical exposed.
///
/// Two things carry it, and only two. The PAINT: the undressed derivation is a
/// cool dark slate, near enough to bare metal that a "clean" look built on
/// subtle greebling would be indistinguishable from no style at all, so this
/// one repaints the hull pale, satin and non-metallic - an airframe, not plate.
/// And the LIVERY: one continuous cobalt band down every straight edge of the
/// hull.
///
/// Everything else is FINISH: function hidden under styling, where the
/// armoured kit hides it under suppression. The kit carries exactly two
/// accents - COBALT for anything painted and AMBER for anything lit - and
/// every shell among the twelve pieces is hull-coloured, so the kit reads as
/// one livery rather than as twelve props. Detail flows LENGTHWISE: every
/// long piece is authored long on Z and aligned to the run, and nothing here
/// clusters.
///
/// The greebles: window rows and the cobalt stripe run as lines, the dish, the
/// fin and the beacon stand alone, and the registry mark is capped.
fn civilian_style(assets: &BaseContentAssets) -> ShipStyleConfig {
    ShipStyleConfig {
        id: CIVILIAN_STYLE_ID.to_string(),
        name: "Civilian".to_string(),
        // LINEAR values. The top is ~0.68 sRGB against the built-in 0.36: a
        // painted airframe, not a slate. Metallic 0 and roughness 0.35 is
        // satin PAINT - the one material choice that separates a painted hull
        // from one welded out of bare plate.
        palette: StylePalette {
            top: SurfaceFinish {
                color: Color::linear_rgb(0.420, 0.420, 0.400),
                roughness: 0.35,
                metallic: 0.0,
            },
            wall: SurfaceFinish {
                color: Color::linear_rgb(0.058, 0.060, 0.068),
                roughness: 0.50,
                metallic: 0.05,
            },
        },
        fixtures: vec![
            StyleFixtureConfig {
                id: "civilian_windows".to_string(),
                model: assets.greeble_civilian_windows.clone(),
                health: 6.0,
                collider: Vec3::new(0.28, 0.06, 0.7),
            },
            StyleFixtureConfig {
                id: "civilian_dish".to_string(),
                model: assets.greeble_civilian_dish.clone(),
                health: 5.0,
                collider: Vec3::new(0.28, 0.14, 0.28),
            },
            StyleFixtureConfig {
                id: "civilian_door".to_string(),
                model: assets.greeble_civilian_door.clone(),
                health: 8.0,
                collider: Vec3::new(0.26, 0.06, 0.44),
            },
            StyleFixtureConfig {
                id: "civilian_tank".to_string(),
                model: assets.greeble_civilian_tank.clone(),
                health: 12.0,
                collider: Vec3::new(0.24, 0.15, 0.4),
            },
            StyleFixtureConfig {
                id: "civilian_fin".to_string(),
                model: assets.greeble_civilian_fin.clone(),
                health: 5.0,
                collider: Vec3::new(0.1, 0.29, 0.24),
            },
            StyleFixtureConfig {
                id: "civilian_stripe".to_string(),
                model: assets.greeble_civilian_stripe.clone(),
                health: 8.0,
                collider: Vec3::new(0.3, 0.09, 0.6),
            },
            StyleFixtureConfig {
                id: "civilian_livery".to_string(),
                model: assets.greeble_civilian_livery.clone(),
                health: 6.0,
                collider: Vec3::new(0.3, 0.03, 0.46),
            },
            StyleFixtureConfig {
                id: "civilian_skylight".to_string(),
                model: assets.greeble_civilian_skylight.clone(),
                health: 6.0,
                collider: Vec3::new(0.2, 0.05, 0.55),
            },
            StyleFixtureConfig {
                id: "civilian_vent".to_string(),
                model: assets.greeble_civilian_vent.clone(),
                health: 5.0,
                collider: Vec3::new(0.18, 0.09, 0.4),
            },
            StyleFixtureConfig {
                id: "civilian_fairing".to_string(),
                model: assets.greeble_civilian_fairing.clone(),
                health: 14.0,
                collider: Vec3::new(0.4, 0.16, 0.4),
            },
            StyleFixtureConfig {
                id: "civilian_beacon".to_string(),
                model: assets.greeble_civilian_beacon.clone(),
                health: 4.0,
                collider: Vec3::new(0.2, 0.1, 0.2),
            },
            StyleFixtureConfig {
                id: "civilian_registry".to_string(),
                model: assets.greeble_civilian_registry.clone(),
                health: 4.0,
                collider: Vec3::new(0.13, 0.03, 0.6),
            },
        ],
        greeble_rules: civilian_rules(),
    }
}

/// SALVAGE: mismatched plating over repeated repair.
///
/// Mismatch comes from the pieces, not from noise. Three patch materials
/// cluster with each other, so a region reads as repaired twice; the weld
/// seam, the chain and the hose run as lines; the whip and the dish stand
/// alone. Every piece is authored off-centre in its recipe, so a run of one
/// piece does not read as a repeated tile.
fn salvage_style(assets: &BaseContentAssets) -> ShipStyleConfig {
    ShipStyleConfig {
        id: SALVAGE_STYLE_ID.to_string(),
        name: "Salvage".to_string(),
        // WARM and dirty, against the built-in cool slate. This is half the
        // look and it costs nothing: oxidised brown plate is what the scrap
        // steel, the rust and the faded livery green of the patches all have to
        // read AGAINST, and against the stock blue-grey the bare-metal pieces
        // read as uniform rather than mixed. Nearly no metallic and
        // nearly all roughness, so no facet of the hull takes a highlight.
        //
        // The TOP-TO-WALL RATIO is 6:1. That is a SILHOUETTE device and not
        // a panelling one, measured: two plates
        // in a run press their walls together and neither is ever seen, so wall
        // is about a twentieth of what the camera gets - the skin's outer rim,
        // plus the exposed side of a plate climbing past a lower neighbour. The
        // ratio therefore makes the hull's own edge read thick, like plate with
        // depth in it, which suits a ship armoured out of scrap. It does not
        // draw a seam between two plates and nothing here pretends it does.
        //
        // The mismatch itself is carried by HUE, and it has to be. Value barely
        // moves on screen - dropping this colour 6.6x, from 0.132 to 0.020,
        // changed a lit flank by well under half of that, because ambient and
        // the tonemap floor it. So a style cannot vary a plate's colour by its
        // cell anyway (`surfaces` is per ROLE, not per plate) and the three
        // patch materials do the whole job: cool grey steel, orange rust and a
        // faded livery green, all of them a HUE away from this warm brown
        // rather than a brightness away from it.
        palette: StylePalette {
            top: SurfaceFinish {
                color: Color::linear_rgb(0.024, 0.0175, 0.0125),
                roughness: 0.94,
                metallic: 0.04,
            },
            wall: SurfaceFinish {
                color: Color::linear_rgb(0.0038, 0.0028, 0.0020),
                roughness: 0.96,
                metallic: 0.04,
            },
        },
        fixtures: vec![
            StyleFixtureConfig {
                id: "salvage_whip".to_string(),
                model: assets.greeble_salvage_whip.clone(),
                health: 6.0,
                collider: Vec3::new(0.10, 0.42, 0.16),
            },
            StyleFixtureConfig {
                id: "salvage_dish".to_string(),
                model: assets.greeble_salvage_dish.clone(),
                health: 10.0,
                collider: Vec3::new(0.33, 0.24, 0.31),
            },
            StyleFixtureConfig {
                id: "salvage_drum".to_string(),
                model: assets.greeble_salvage_drum.clone(),
                health: 24.0,
                collider: Vec3::new(0.22, 0.23, 0.34),
            },
            StyleFixtureConfig {
                id: "salvage_net".to_string(),
                model: assets.greeble_salvage_net.clone(),
                health: 14.0,
                collider: Vec3::new(0.38, 0.16, 0.57),
            },
            StyleFixtureConfig {
                id: "salvage_hook".to_string(),
                model: assets.greeble_salvage_hook.clone(),
                health: 14.0,
                collider: Vec3::new(0.20, 0.11, 0.14),
            },
            StyleFixtureConfig {
                id: "salvage_chain".to_string(),
                model: assets.greeble_salvage_chain.clone(),
                health: 12.0,
                collider: Vec3::new(0.11, 0.13, 0.88),
            },
            StyleFixtureConfig {
                id: "salvage_weld_seam".to_string(),
                model: assets.greeble_salvage_weld_seam.clone(),
                health: 18.0,
                collider: Vec3::new(0.2, 0.06, 0.92),
            },
            StyleFixtureConfig {
                id: "salvage_patch_strip".to_string(),
                model: assets.greeble_salvage_patch_strip.clone(),
                health: 18.0,
                collider: Vec3::new(0.45, 0.05, 0.84),
            },
            StyleFixtureConfig {
                id: "salvage_hose".to_string(),
                model: assets.greeble_salvage_hose.clone(),
                health: 8.0,
                collider: Vec3::new(0.13, 0.09, 0.87),
            },
            StyleFixtureConfig {
                id: "salvage_grille".to_string(),
                model: assets.greeble_salvage_grille.clone(),
                health: 12.0,
                collider: Vec3::new(0.38, 0.08, 0.3),
            },
            StyleFixtureConfig {
                id: "salvage_cog_patch".to_string(),
                model: assets.greeble_salvage_cog_patch.clone(),
                health: 16.0,
                collider: Vec3::new(0.38, 0.06, 0.38),
            },
            StyleFixtureConfig {
                id: "salvage_patch_plate".to_string(),
                model: assets.greeble_salvage_patch_plate.clone(),
                health: 20.0,
                collider: Vec3::new(0.88, 0.04, 0.68),
            },
            StyleFixtureConfig {
                id: "salvage_kills".to_string(),
                model: assets.greeble_salvage_kills.clone(),
                health: 6.0,
                collider: Vec3::new(0.4, 0.03, 0.3),
            },
            StyleFixtureConfig {
                id: "salvage_patch_scab".to_string(),
                model: assets.greeble_salvage_patch_scab.clone(),
                health: 16.0,
                collider: Vec3::new(0.52, 0.03, 0.44),
            },
        ],
        greeble_rules: salvage_rules(),
    }
}

/// STATION: a pale enamelled habitat with one accent, navigation red.
///
/// One tall piece, the antenna, stands alone; the windows run as lines; the
/// radiators cluster; the red beacons stand alone; hatches and approach
/// chevrons fill between.
fn station_style() -> ShipStyleConfig {
    let greeble = |id: &str| {
        nova_gameplay::prelude::AssetRef::from(format!("self://gltf/greebles/{id}.glb#Scene0"))
    };
    let fixture = |id: &str, health: f32, collider: Vec3| StyleFixtureConfig {
        id: id.to_string(),
        model: greeble(id),
        health,
        collider,
    };
    ShipStyleConfig {
        id: STATION_STYLE_ID.to_string(),
        name: "Station".to_string(),
        palette: StylePalette {
            top: SurfaceFinish {
                color: Color::linear_rgb(0.42, 0.43, 0.45),
                roughness: 0.5,
                metallic: 0.1,
            },
            wall: SurfaceFinish {
                color: Color::linear_rgb(0.06, 0.062, 0.07),
                roughness: 0.8,
                metallic: 0.1,
            },
        },
        fixtures: vec![
            fixture("station_antenna", 8.0, Vec3::new(0.16, 0.38, 0.16)),
            fixture("station_windows", 6.0, Vec3::new(0.14, 0.03, 0.62)),
            fixture("station_radiator", 10.0, Vec3::new(0.4, 0.065, 0.3)),
            fixture("station_beacon", 4.0, Vec3::new(0.12, 0.085, 0.12)),
            fixture("station_hatch", 6.0, Vec3::new(0.26, 0.04, 0.26)),
            fixture("station_marking", 4.0, Vec3::new(0.24, 0.02, 0.32)),
        ],
        greeble_rules: station_rules(),
    }
}

/// The link types every base kit declares, plus one line type per id in
/// `lines`.
///
/// - `any` faces `any` and `empty`: pieces that may touch each other.
/// - `empty` faces `any`, `empty` and `spaced`: the empty greeble's sides.
/// - `spaced` faces only `empty`: a piece that stands alone.
/// - each line type faces only itself, so it forces a line greeble or an end
///   greeble across it.
fn link_types(lines: &[&str]) -> Vec<GreebleLinkConfig> {
    let link = |id: &str, connects: &[&str]| GreebleLinkConfig {
        id: id.into(),
        connects: connects.iter().map(|other| (*other).into()).collect(),
    };
    let mut links = vec![
        link("any", &["any", "empty"]),
        link("empty", &["any", "empty", "spaced"]),
        link("spaced", &["empty"]),
    ];
    links.extend(lines.iter().map(|line| link(line, &[line])));
    links
}

/// A greeble with the link types `[up, right, down, left]`, no range terms and
/// no cap.
fn greeble(
    id: &str,
    fixture: Option<&str>,
    weight: f32,
    rotate: GreebleRotateType,
    [up, right, down, left]: [&str; 4],
) -> GreebleConfig {
    GreebleConfig {
        id: id.into(),
        fixture: fixture.map(str::to_string),
        weight,
        rotate,
        link_points: GreebleLinkPoints {
            up: up.into(),
            right: right.into(),
            down: down.into(),
            left: left.into(),
        },
        near: Vec::new(),
        max_count: None,
    }
}

/// The empty greeble: no fixture, `empty` on every side.
fn empty(weight: f32) -> GreebleConfig {
    greeble("empty", None, weight, GreebleRotateType::None, ["empty"; 4])
}

/// A piece that may touch other pieces. Its greeble id is its fixture id.
fn filler(fixture: &str, weight: f32) -> GreebleConfig {
    greeble(
        fixture,
        Some(fixture),
        weight,
        GreebleRotateType::None,
        ["any"; 4],
    )
}

/// A piece that touches only empty space, at most `max_count` times, with no
/// copy of itself within `radius` cells.
fn alone(fixture: &str, weight: f32, radius: u32, max_count: u32) -> GreebleConfig {
    GreebleConfig {
        near: vec![near(&[fixture], radius, 0.0)],
        max_count: Some(max_count),
        ..greeble(
            fixture,
            Some(fixture),
            weight,
            GreebleRotateType::None,
            ["spaced"; 4],
        )
    }
}

/// The two greebles of a line: the run piece, `line` on `up` and `down`, and
/// the end piece `<fixture>_end`, `line` on `up` only. Both place `fixture`,
/// whose long axis is its `+Z`, so the model lies along the run.
fn line(fixture: &str, line: &str, weight: f32, end_weight: f32) -> [GreebleConfig; 2] {
    [
        greeble(
            fixture,
            Some(fixture),
            weight,
            GreebleRotateType::Half,
            [line, "any", line, "any"],
        ),
        greeble(
            &format!("{fixture}_end"),
            Some(fixture),
            end_weight,
            GreebleRotateType::All,
            [line, "any", "any", "any"],
        ),
    ]
}

/// A range term on `greebles` within `radius` cells.
fn near(greebles: &[&str], radius: u32, factor: f32) -> GreebleNearConfig {
    GreebleNearConfig {
        greebles: greebles.iter().map(|id| (*id).into()).collect(),
        radius,
        factor,
    }
}

/// `greeble` with its weight scaled by `factor` beside a copy of itself, so
/// copies cluster.
fn clustered(mut greeble: GreebleConfig, factor: f32) -> GreebleConfig {
    greeble.near.push(near(&[greeble.id.as_str()], 1, factor));
    greeble
}

/// INDUSTRIAL rules. Two lines: the hazard band (`line`) and the duct
/// (`line_2`).
fn industrial_rules() -> GreebleRules {
    let mut greebles = vec![
        empty(6.0),
        alone("industrial_stack", 0.5, 3, 4),
        alone("industrial_crane", 0.25, 4, 2),
        filler("industrial_umbilical", 0.4),
        filler("industrial_plate_rack", 0.4),
        filler("industrial_cells", 0.3),
        clustered(filler("industrial_radiator", 0.5), 3.0),
        alone("industrial_winch", 0.3, 2, 3),
        alone("industrial_floodlight", 0.3, 2, 4),
        filler("industrial_louvre", 0.5),
        filler("industrial_hatch", 0.5),
        filler("industrial_stencil", 0.4),
        clustered(filler("industrial_ribbing", 0.8), 2.0),
    ];
    greebles.extend(line("industrial_hazard_band", "line", 1.2, 0.4));
    greebles.extend(line("industrial_duct", "line_2", 0.8, 0.3));
    GreebleRules {
        link_types: link_types(&["line", "line_2"]),
        greebles,
    }
}

/// ARMOURED rules. One line: the belt (`line`).
fn armoured_rules() -> GreebleRules {
    let mut greebles = vec![
        empty(6.0),
        alone("armoured_sensor", 0.3, 4, 2),
        alone("armoured_mast", 0.3, 4, 2),
        filler("armoured_cap", 0.4),
        filler("armoured_intake", 0.5),
        filler("armoured_chaff", 0.3),
        filler("armoured_hatch", 0.6),
        filler("armoured_ammo_stripes", 0.3),
        GreebleConfig {
            max_count: Some(4),
            ..filler("armoured_magazine", 0.3)
        },
        clustered(filler("armoured_applique", 1.2), 2.5),
    ];
    greebles.extend(line("armoured_strake", "line", 1.5, 0.4));
    GreebleRules {
        link_types: link_types(&["line"]),
        greebles,
    }
}

/// CIVILIAN rules. Two lines: the window row (`line`) and the stripe
/// (`line_2`).
fn civilian_rules() -> GreebleRules {
    let mut greebles = vec![
        empty(6.0),
        alone("civilian_dish", 0.25, 4, 2),
        GreebleConfig {
            max_count: Some(4),
            ..filler("civilian_door", 0.3)
        },
        filler("civilian_tank", 0.3),
        alone("civilian_fin", 0.3, 3, 3),
        filler("civilian_livery", 0.4),
        filler("civilian_skylight", 0.4),
        filler("civilian_vent", 0.4),
        filler("civilian_fairing", 0.5),
        alone("civilian_beacon", 0.3, 2, 6),
        GreebleConfig {
            near: vec![near(&["civilian_registry"], 3, 0.0)],
            max_count: Some(3),
            ..filler("civilian_registry", 0.2)
        },
    ];
    greebles.extend(line("civilian_windows", "line", 1.0, 0.3));
    greebles.extend(line("civilian_stripe", "line_2", 1.2, 0.3));
    GreebleRules {
        link_types: link_types(&["line", "line_2"]),
        greebles,
    }
}

/// SALVAGE rules. Three lines: the weld seam (`line`), the chain (`line_2`)
/// and the hose (`line_3`). The three patches weight each other up, so they
/// meet in clusters of mixed material.
fn salvage_rules() -> GreebleRules {
    let patches = [
        "salvage_patch_strip",
        "salvage_patch_plate",
        "salvage_patch_scab",
    ];
    let patch = |fixture: &str, weight: f32| GreebleConfig {
        near: vec![near(&patches, 1, 2.0)],
        ..filler(fixture, weight)
    };
    let mut greebles = vec![
        empty(5.0),
        alone("salvage_whip", 0.3, 4, 2),
        alone("salvage_dish", 0.25, 4, 2),
        filler("salvage_drum", 0.3),
        filler("salvage_net", 0.3),
        filler("salvage_hook", 0.3),
        patch("salvage_patch_strip", 0.5),
        filler("salvage_grille", 0.4),
        filler("salvage_cog_patch", 0.3),
        patch("salvage_patch_plate", 0.6),
        GreebleConfig {
            max_count: Some(2),
            ..filler("salvage_kills", 0.2)
        },
        patch("salvage_patch_scab", 0.4),
    ];
    greebles.extend(line("salvage_weld_seam", "line", 1.0, 0.4));
    greebles.extend(line("salvage_chain", "line_2", 0.4, 0.2));
    greebles.extend(line("salvage_hose", "line_3", 0.5, 0.2));
    GreebleRules {
        link_types: link_types(&["line", "line_2", "line_3"]),
        greebles,
    }
}

/// STATION rules. One line: the window row (`line`).
fn station_rules() -> GreebleRules {
    let mut greebles = vec![
        empty(6.0),
        alone("station_antenna", 0.3, 3, 4),
        clustered(filler("station_radiator", 0.6), 3.0),
        alone("station_beacon", 0.3, 2, 6),
        filler("station_hatch", 0.4),
        filler("station_marking", 0.5),
    ];
    greebles.extend(line("station_windows", "line", 1.0, 0.3));
    GreebleRules {
        link_types: link_types(&["line"]),
        greebles,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The least opinionated look leads the catalog.
    ///
    /// Not cosmetic ordering: `wfc_ships` and the editor's build view both
    /// refuse to name a style id and take the first one instead, so this is
    /// what decides what a subject nobody styled is photographed wearing.
    #[test]
    fn the_least_opinionated_look_leads_the_catalog() {
        let ids: Vec<String> = style_catalog(&BaseContentAssets::from_paths())
            .into_iter()
            .map(|style| style.id)
            .collect();
        assert_eq!(ids.first().map(String::as_str), Some(INDUSTRIAL_STYLE_ID));
        for authored in [ARMOURED_STYLE_ID, CIVILIAN_STYLE_ID, SALVAGE_STYLE_ID] {
            assert!(
                ids.iter().any(|id| id == authored),
                "{authored} is not in the catalog",
            );
        }
    }

    /// The kit sizes are an ORDERING: armoured is the smallest because
    /// suppression is its look, salvage the biggest because accumulation is.
    /// Each kit pins its own count; this pins the RELATION between them, so a
    /// new piece cannot invert it. Equality at the top is tolerated; strict
    /// order below it stands.
    #[test]
    fn the_kit_caps_order_the_tone_split() {
        let assets = BaseContentAssets::from_paths();
        let armoured = armoured_style(&assets).fixtures.len();
        let civilian = civilian_style(&assets).fixtures.len();
        let industrial = industrial_style(&assets).fixtures.len();
        let salvage = salvage_style(&assets).fixtures.len();
        assert!(
            armoured < civilian && civilian < industrial && industrial <= salvage,
            "the tone split inverted: armoured {armoured} < civilian \
             {civilian} < industrial {industrial} <= salvage {salvage} must hold",
        );
    }

    /// Ids are the whole overlay contract: content is merged LAST WINS BY ID, so
    /// two base styles sharing one id would silently delete a look, and two
    /// fixtures sharing one id inside a style would make a greeble's `fixture`
    /// ambiguous.
    #[test]
    fn no_two_styles_and_no_two_fixtures_share_an_id() {
        let mut seen = Vec::new();
        for style in style_catalog(&BaseContentAssets::from_paths()) {
            assert!(
                !seen.contains(&style.id),
                "two styles are called {}",
                style.id
            );
            let mut pieces = Vec::new();
            for fixture in &style.fixtures {
                assert!(
                    !pieces.contains(&fixture.id),
                    "{} ships two fixtures called {}",
                    style.id,
                    fixture.id,
                );
                pieces.push(fixture.id.clone());
            }
            seen.push(style.id);
        }
    }

    /// Every model a style names is one of the greebles the base bundle ships,
    /// authored through the scheme like every other base asset ref.
    ///
    /// The runtime gate and the repo lint both check the ref against the
    /// manifest; this catches the same mistake in the BUILDER, where the fix
    /// belongs.
    #[test]
    fn every_fixture_names_a_shipped_greeble() {
        let assets = BaseContentAssets::from_paths();
        for style in style_catalog(&assets) {
            assert!(!style.fixtures.is_empty(), "{} places nothing", style.id);
            for fixture in &style.fixtures {
                let path = fixture.model.path().unwrap_or_default().to_string();
                assert!(
                    path.starts_with("self://gltf/greebles/"),
                    "{} names '{path}', which is not a base greeble",
                    fixture.id,
                );
                assert!(
                    fixture.collider.min_element() > 0.0,
                    "{} carries no collider, so a round would pass through it",
                    fixture.id,
                );
                assert!(fixture.health > 0.0, "{} cannot be shot off", fixture.id);
            }
        }
    }

    /// Every kit's greeble rules compile, and every fixture the kit lists is
    /// placed by at least one greeble. A listed fixture no greeble names is
    /// art the ship never wears.
    #[test]
    fn every_kit_compiles_and_places_every_fixture_it_lists() {
        for style in style_catalog(&BaseContentAssets::from_paths()) {
            let compiled = style
                .compile_greebles()
                .unwrap_or_else(|faults| panic!("{}: {faults:?}", style.id))
                .unwrap_or_else(|| panic!("{} has no greebles", style.id));
            for (index, fixture) in style.fixtures.iter().enumerate() {
                assert!(
                    (0..compiled.rules.greeble_count())
                        .any(|greeble| compiled.fixture(greeble) == Some(index)),
                    "{}: no greeble places {}",
                    style.id,
                    fixture.id,
                );
            }
        }
    }

    /// Every kit is namespaced by its style - fixture ids and the art each one
    /// names - and its size is pinned. Armoured stays the smallest and salvage
    /// the largest (see `the_kit_caps_order_the_tone_split`); a kit that grows
    /// quietly is how this repo once carried forty meshes.
    #[test]
    fn every_kit_is_namespaced_and_pinned_in_size() {
        let assets = BaseContentAssets::from_paths();
        for (style, size) in [
            (industrial_style(&assets), 14),
            (armoured_style(&assets), 10),
            (civilian_style(&assets), 12),
            (salvage_style(&assets), 14),
            (station_style(), 6),
        ] {
            assert_eq!(style.fixtures.len(), size, "{} kit size", style.id);
            let prefix = format!("{}_", style.id);
            for fixture in &style.fixtures {
                assert!(
                    fixture.id.starts_with(&prefix),
                    "'{}' is not namespaced by its style",
                    fixture.id,
                );
                let path = fixture.model.path().unwrap_or_default().to_string();
                assert_eq!(
                    path,
                    format!("self://gltf/greebles/{}.glb#Scene0", fixture.id),
                    "'{}' names another greeble",
                    fixture.id,
                );
            }
        }
    }
}
