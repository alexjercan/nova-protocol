//! Built-in SKIN STYLE content: the looks a ship's derived cladding can wear.
//!
//! A style is content like a section is content, so this file is a builder and
//! not a table of constants - `content gen` serializes it into
//! `assets/base/styles/base.content.ron` and a mod overlays it by id.
//!
//! Four AUTHORED looks ship (task `20260815-225748`, Phase B) and one piece of
//! SCAFFOLDING trails them. `industrial`, `armoured`, `civilian` and `salvage`
//! are four points of view about what a hull is, and they share one plate
//! vocabulary and one greeble generator - so grafting a rule from one onto
//! another is a content edit rather than a rewrite. `placeholder` wires the
//! four magenta placeholder greebles to four rules chosen to exercise the
//! whole vocabulary rather than to look like anything.
//!
//! # The SEAT, and which pieces are free of it
//!
//! Every region but the high ground asks for a plate whose top is ONE SURFACE.
//! That is what a relief list used to try to say and got wrong: measured over
//! 526 generated plates, the flat panel and the straight edge are surfaces
//! every time, the corner-cut panel, the crest, the stud and the outer corner
//! are cones every time, and the shoulder splits about in half. So a region
//! names a ZONE of the hull, and the seat gate decides per plate whether a
//! piece can actually lie on it.
//!
//! A piece escapes that gate in one of two ways, and neither is authored here.
//! [`FixtureRegion::HighGround`] drops it outright, because a crest and a spar
//! tip are cones by definition and a blanket "surfaces only" would take a
//! ship's silhouette off to fix a bedding defect - the stack, the whip, the
//! fin, the two masts and the corner boss all ride that. Everything else
//! escapes by being TRIM: a collider shorter than
//! [`TRIM_HEIGHT`](nova_ship::prelude::TRIM_HEIGHT) stands a hand's breadth off
//! the plate, so a crease under it is a fold in a decal rather than a gap you
//! can see.
//!
//! The trim route is what keeps a HAND-BUILT hull dressed. Such a hull is one
//! cell thick nearly everywhere and derives as cones from end to end - zero
//! coplanar plates on half the bench roster - so a kit whose every filler
//! stands proud places NOTHING on the shapes players actually build. Each style
//! therefore carries at least one region that asks nothing about relief and at
//! least one piece flat enough to lie on a crease: the industrial stencil,
//! hatch and ribbing, the armoured applique, the civilian registry, and the
//! salvage scab, tally, grille, cog and doubler plate.

use bevy::prelude::{Color, Vec3};
use nova_ship::prelude::{
    FixtureDensity, FixtureOrientation, FixturePlacement, FixtureRegion, ShipStyleConfig,
    StyleFixtureConfig, StylePalette, SurfaceFinish,
};

use super::assets::BaseContentAssets;

/// The id the placeholder style is named by. The editor and the generator
/// reference it, so a rename is one constant.
pub const PLACEHOLDER_STYLE_ID: &str = "placeholder";

/// The id the industrial look is named by.
pub const INDUSTRIAL_STYLE_ID: &str = "industrial";

/// The id the armoured look is named by.
pub const ARMOURED_STYLE_ID: &str = "armoured";

/// The id the civilian look is named by.
pub const CIVILIAN_STYLE_ID: &str = "civilian";

/// The id the salvage look is named by.
pub const SALVAGE_STYLE_ID: &str = "salvage";

/// Every built-in style, in stable generated-content order.
///
/// AUTHORED looks first, scaffolding last. Neither the `wfc_ships` producer nor
/// the editor's build view knows a style id - both fall back to the FIRST style
/// the merged content offers - so the head of this list is what an unstyled
/// subject wears, and a placeholder there would photograph the test pattern.
///
/// `industrial` leads because a FALLBACK should be the least opinionated thing
/// here. The other three are cast parts - the warship, the yacht, the raider -
/// and a ship that never named a style is none of them; a working hull with its
/// services on the outside is what an arbitrary generated hull reads as. It is
/// also the widest kit (seven pieces over six zones of the vocabulary), so it
/// has the most to say about a hull nobody authored for it.
pub(crate) fn style_catalog(assets: &BaseContentAssets) -> Vec<ShipStyleConfig> {
    vec![
        industrial_style(assets),
        armoured_style(assets),
        civilian_style(assets),
        salvage_style(assets),
        placeholder_style(assets),
    ]
}

/// INDUSTRIAL: a working hull, built to be maintained rather than admired.
///
/// The thesis is that nothing on this ship is hidden. Services run OUTSIDE the
/// plating where a fitter can get a spanner on them, every panel that opens is
/// visibly a panel that opens, and the edges a crew would bark a shin on are
/// painted. Three moves carry it, and they are meant to be legible from across
/// a frame:
///
/// 1. THE PAINT. Safety yellow on the straight edges of the hull, on the collar
///    of every stack and on every hatch handle. One accent used three ways is a
///    system; one accent used once is a colour.
/// 2. THE SEAM. The wall surface is taken far darker than the top - about six
///    times, against the placeholder's one and a half - so the drop at every
///    plate boundary reads as a black gap between bolted panels rather than as
///    shading. That is the cheapest "exposed panelling" there is, and it costs
///    no geometry at all.
/// 3. THE KIT. Radiators, conduit, corrugation, louvres, hatches, stacks -
///    and, since the builders batch (task 20260816-222639), a crane, battery
///    racks, plate stock, a winch, floodlights, umbilical points and
///    stencilled part numbers. Fourteen pieces, no ornament among them -
///    every one is something a fitter would unbolt. The batch sharpened the
///    fiction to THE BUILDERS: a working shipyard that never left the ship.
///
/// The rules below are a ZONE ALLOCATION rather than a set of overlapping
/// filters: each piece owns a part of the hull that the vocabulary can name, so
/// priority mostly resolves ties rather than starving anything. Reading down:
/// the high ground, the hoist and then the painted line down the long edges,
/// the flanks, the laydown decks (plate stock, then cells), the flat panel
/// (radiator, then corrugation), the three pocket fittings (winch,
/// floodlight, louvre), and last the three that lie on anything - hatches,
/// stencilled part numbers, and the ribbing that takes whatever is left.
fn industrial_style(assets: &BaseContentAssets) -> ShipStyleConfig {
    // The builders-batch pieces name their art BY ID rather than through a
    // `BaseContentAssets` field: four vocabulary batches land in parallel and
    // the assets struct is the one file every lane would collide in. The
    // namespacing test below pins the id-to-path equality either way.
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
                // THE HIGH GROUND, and the only rule allowed to break the
                // silhouette. A crest and a spar tip are CONES, and the high
                // ground is the one region that drops the seat gate for exactly
                // that reason: a stack standing up the tip of a spar is the
                // picture, and a stack banished to the flat deck is not this
                // rule.
                //
                // It is also half of the kit's reach onto a HAND-BUILT hull.
                // Such a hull is one cell thick nearly everywhere, so it derives
                // as cones end to end and every seated region reaches nothing;
                // the high ground and the flat trim below are what is left. The
                // rung's patch floor makes that a guarantee rather than a coin
                // toss: one stack per block of high ground, whatever the share
                // does.
                placement: FixturePlacement {
                    region: FixtureRegion::HighGround,
                    density: FixtureDensity::Regular,
                    orientation: FixtureOrientation::Free,
                },
            },
            StyleFixtureConfig {
                id: "industrial_crane".to_string(),
                model: greeble("industrial_crane"),
                health: 12.0,
                collider: Vec3::new(0.16, 0.45, 0.32),
                // THE HOIST, and deliberately ABOVE the band, which was
                // MEASURED the hard way: drafted below it the crane took
                // NOTHING on any of the three bench seeds. The band is `Every`,
                // so it claims the whole edge, and the stubs the crane was
                // drafted for - the one-cell edge at a deck corner - are not
                // there on a generated hull. A rule under an `Every` rule in
                // its own region is starved by design.
                //
                // Above it, and at the thinnest rung there is, the cost is the
                // one thing the old order was protecting: two or three cells of
                // painted line lost to a hoist. That reads as a thing the paint
                // was worked around, where a crane that is never on any ship
                // reads as a crane nobody modelled. `Outward` leans the jib
                // overboard, off the side it loads from.
                placement: FixturePlacement {
                    region: FixtureRegion::Edge,
                    density: FixtureDensity::Rare,
                    orientation: FixtureOrientation::Outward,
                },
            },
            StyleFixtureConfig {
                id: "industrial_hazard_band".to_string(),
                model: assets.greeble_industrial_hazard_band.clone(),
                health: 6.0,
                collider: Vec3::new(0.19, 0.03, 0.9),
                // THE SIGNATURE. `Edge` is the straight side of a hull, where
                // the skin falls away along one whole flank - 45-55% of the
                // falling plate on a generated ship. Painting it draws the
                // ship's own silhouette in yellow.
                //
                // `Every` on purpose, which is the opposite of every other rule
                // here. A stripe is a LINE: thinned to a share it becomes
                // dashes, and dashes are the confetti the whole alignment
                // machinery exists to avoid. What keeps it honest is the piece's
                // own length - nine tenths of a cell across the plate asks for
                // two cells of edge under it - so a one-cell corner is not a run
                // and is not painted.
                placement: FixturePlacement {
                    region: FixtureRegion::Edge,
                    density: FixtureDensity::Every,
                    orientation: FixtureOrientation::Along,
                },
            },
            StyleFixtureConfig {
                id: "industrial_umbilical".to_string(),
                model: greeble("industrial_umbilical"),
                health: 10.0,
                collider: Vec3::new(0.16, 0.13, 0.44),
                // THE FLANKS: ships get built plugged in, and the capped
                // shore connections stay where the gantry stood - down the
                // SIDES, the zone civilian spends on windows, which is the
                // tone split doing its job. `Flank` is this kit's only
                // side-facing rule, and `Flank` is a SUBSET of `Deck` - the
                // same plate, facing out instead of up - so every deck rule
                // above it eats its cells. Measured: below the rack it took
                // zero on two of three bench seeds and one on the third, hence
                // this rule now rides above the deck accents as well as above
                // the radiator and the duct. The region keeps sockets on hull
                // body where trunking plausibly runs behind them; `Along` rows
                // them down the flank; the rung keeps the row punctuation
                // rather than cladding.
                placement: FixturePlacement {
                    region: FixtureRegion::Flank,
                    density: FixtureDensity::Regular,
                    orientation: FixtureOrientation::Along,
                },
            },
            StyleFixtureConfig {
                id: "industrial_plate_rack".to_string(),
                model: greeble("industrial_plate_rack"),
                health: 20.0,
                collider: Vec3::new(0.34, 0.1, 0.43),
                // THE LAYDOWN YARD: the builders carry their materials, so
                // spare plate stock lies lashed to the decks. ABOVE the
                // radiator and the duct, and that ordering was MEASURED, not
                // reasoned: drafted below them, the radiator's half share
                // plus the conduit's full-share lines left this rule ZERO
                // pieces on six of the eight bench blocks - the starved-
                // signature defect the vocabulary spike names. A sparse
                // chunky accent samples before the broad claimants, the same
                // law the hatch already pins against the corrugation. The
                // cost runs the other way and is cheap: a conduit run loses
                // at most the odd cell, and a stack of plate parked mid-run
                // reads as a thing the pipe was routed around, where a hole
                // in a painted line reads as a mistake. `Deck` already asks
                // for two cells of ship under the plate, so the stock never
                // lies on skin over a spar; the rung keeps the yard from
                // tipping into scrapyard (the salvage drum's measured
                // lesson), and its patch floor still guarantees a big deck
                // its stock pile.
                placement: FixturePlacement {
                    region: FixtureRegion::Deck,
                    density: FixtureDensity::Regular,
                    orientation: FixtureOrientation::Along,
                },
            },
            StyleFixtureConfig {
                id: "industrial_cells".to_string(),
                model: greeble("industrial_cells"),
                health: 16.0,
                collider: Vec3::new(0.3, 0.21, 0.46),
                // THE POWER CELLS, racked in the open where a fitter can swap
                // one - the shared matrix's power-cell class in its
                // industrial voice. Above the radiator and the duct for the
                // plate rack's measured reason, and behind the rack because
                // of the two the rack is the bigger silhouette. `Sparse` is
                // the thinnest rung any deck piece in the kit takes, and its
                // patch floor is what turns that into "one rack per block of
                // hull" instead of a battery farm. Same lattice as the other
                // deck accents, so racks and hatches read as one bolt grid,
                // and `Deck` keeps them on real ship. The yellow bus bar across
                // the terminals is the accent's COLLAR use - the discipline
                // (edges, collars, handles) is why this piece may carry
                // paint at all.
                placement: FixturePlacement {
                    region: FixtureRegion::Deck,
                    density: FixtureDensity::Sparse,
                    orientation: FixtureOrientation::Along,
                },
            },
            StyleFixtureConfig {
                id: "industrial_radiator".to_string(),
                model: assets.greeble_industrial_radiator.clone(),
                health: 14.0,
                collider: Vec3::new(0.38, 0.27, 0.28),
                // FLAT PANEL, which is where the one piece with real height and
                // a long axis fits.
                //
                // `Dense` rather than a thinner rung, and that was MEASURED on
                // the hand-built bench: panel plate is 0-8 of a small subject's
                // plates and mostly reads one cell of run, so a thinner rung
                // reached two plates on one subject and nothing anywhere else.
                // On a generated hull's 6-22 panels this is the same 3-11
                // radiators; on a hand-built deck it is finally not zero.
                //
                // `Panel` carries the corner-off plate beside the flat one, and
                // the seat gate is what sorts them: this piece stands proud, so
                // the ones that actually crease refuse it.
                placement: FixturePlacement {
                    region: FixtureRegion::Panel,
                    density: FixtureDensity::Dense,
                    orientation: FixtureOrientation::Along,
                },
            },
            StyleFixtureConfig {
                id: "industrial_duct".to_string(),
                model: assets.greeble_industrial_duct.clone(),
                health: 9.0,
                collider: Vec3::new(0.14, 0.16, 0.9),
                // THE SERVICES RUN, on the panel face rather than on the edge.
                // Not `Edge`, though a pipe would lie on one perfectly well:
                // measured, a rule that took the edge reached 11-14 plates and
                // TOOK 1-4, because the band above had already painted every
                // long one. A reach that is mostly another rule's plates is a
                // number that cannot be tuned against.
                //
                // `Every`, unlike almost everything else here. A conduit is a
                // LINE like the band is, and the piece spans nine tenths of its
                // cell precisely so that consecutive plates of one run join up;
                // a lattice would dash it. That same length asks for two cells
                // of like panel underneath, so a lone flat cell takes no pipe.
                placement: FixturePlacement {
                    region: FixtureRegion::Panel,
                    density: FixtureDensity::Every,
                    orientation: FixtureOrientation::Along,
                },
            },
            StyleFixtureConfig {
                id: "industrial_winch".to_string(),
                model: greeble("industrial_winch"),
                health: 14.0,
                collider: Vec3::new(0.23, 0.25, 0.38),
                // DECK MACHINERY AT THE FITTINGS - the wheels-and-cogs class,
                // which the shared matrix gives ONLY to the working styles: a
                // warship armours its gearing over and a sold ship fairs it
                // in, so an exposed cog is half the builders' voice on its
                // own. AHEAD of the louvre, and measured: the pocket is 2-7
                // plates on a generated hull, so a `Dense` grille sampled first
                // left the two machines below it nothing at all. The specific
                // piece goes above the cladding-weight one, which is the same
                // law the rack already pins against the conduit. The crank knob
                // is the accent's HANDLE use, the hatch handle's paint.
                placement: FixturePlacement {
                    region: FixtureRegion::NearFitting,
                    density: FixtureDensity::Regular,
                    orientation: FixtureOrientation::Along,
                },
            },
            StyleFixtureConfig {
                id: "industrial_floodlight".to_string(),
                model: greeble("industrial_floodlight"),
                health: 8.0,
                collider: Vec3::new(0.17, 0.25, 0.12),
                // WORK LIGHTING OVER THE WORK: the second of the three pocket
                // rules, and on a punctuation rung rather than a heavy one
                // because three pieces crowding every fitting is confetti with
                // a theme. The scatter cannot aim a piece AT its
                // fitting, so the recipe tilts the lamp heads down and
                // `Outward` turns the pair off the hull - rigged yard lighting
                // over the work face rather than lamps combed with the panel
                // grain. The region asks nothing about the ship under the
                // plate, which is right for a lamp: it bolts to anything,
                // including the thin structure a boom-mounted gun rides on.
                // Above the louvre for the winch's measured reason, and behind
                // the winch because of the two the winch is the bigger
                // silhouette.
                placement: FixturePlacement {
                    region: FixtureRegion::NearFitting,
                    density: FixtureDensity::Regular,
                    orientation: FixtureOrientation::Outward,
                },
            },
            StyleFixtureConfig {
                id: "industrial_louvre".to_string(),
                model: assets.greeble_industrial_louvre.clone(),
                health: 12.0,
                collider: Vec3::new(0.32, 0.07, 0.24),
                // THE FITTINGS. `NearFitting` is the documented trap - it reads
                // as narrow and admits a third of a hull - so it is LAST of
                // the pocket rules and eleventh here rather than first, on a
                // rung that keeps the lattice and a share. What it is worth is
                // the read: grilles clustered where the drives and the gun
                // wells are says the machinery is under that panel. Seven
                // centimetres proud, so it is TRIM and lies on a crease
                // honestly.
                placement: FixturePlacement {
                    region: FixtureRegion::NearFitting,
                    density: FixtureDensity::Dense,
                    orientation: FixtureOrientation::Along,
                },
            },
            StyleFixtureConfig {
                id: "industrial_hatch".to_string(),
                model: assets.greeble_industrial_hatch.clone(),
                health: 18.0,
                collider: Vec3::new(0.32, 0.07, 0.32),
                // Free-turning: a round hatch has no long axis, so there is
                // nothing for an orientation to point down.
                //
                // ABOVE the corrugation, and it has to be. Below it the two
                // shared one lattice, the corrugation's reach covered nearly all
                // of it, and the hatch measured `x0 of 12` - a third of the
                // kit's yellow gone, on a hull where nothing else was competing
                // for those plates. A sparse accent has to be sampled BEFORE the
                // filler, never out of the filler's leavings.
                placement: FixturePlacement {
                    region: FixtureRegion::Anywhere,
                    density: FixtureDensity::Regular,
                    orientation: FixtureOrientation::Free,
                },
            },
            StyleFixtureConfig {
                id: "industrial_stencil".to_string(),
                model: greeble("industrial_stencil"),
                health: 4.0,
                collider: Vec3::new(0.22, 0.02, 0.34),
                // THE MARKINGS, and the kit's flat-detail carrier (finding
                // 1.5.3 of the vocabulary spike). `Anywhere` asks nothing
                // about relief, and two centimetres of stand-off make the
                // piece TRIM, which drops the seat gate as well: a hand-built
                // hull one cell thick derives as cones end to end, and a
                // placard is the one piece that genuinely lies on any of them
                // - which is why the marking class carries the carrier duty
                // (flat detail may sit anywhere, PG 3.6). This
                // far down the list every zone rule has picked first, so on a
                // thick hull it numbers the gaps; on a thin frame it is most
                // of what fires, and a bare frame stencilled with part
                // numbers IS the builders' read. A middling rung - a serial
                // is punctuation - whose patch floor keeps any block of hull
                // from going entirely unnumbered. Above the ribbing because
                // even a decal is an accent, and accents are never sampled
                // from a filler's leavings.
                placement: FixturePlacement {
                    region: FixtureRegion::Anywhere,
                    density: FixtureDensity::Regular,
                    orientation: FixtureOrientation::Along,
                },
            },
            StyleFixtureConfig {
                id: "industrial_ribbing".to_string(),
                model: assets.greeble_industrial_ribbing.clone(),
                health: 15.0,
                collider: Vec3::new(0.46, 0.07, 0.42),
                // FILLER, and the piece that decides whether the hull reads as
                // busy. LAST, because it stands anywhere and would otherwise eat
                // the plates every rule above it wants.
                //
                // `Dense` - the heaviest rung still on the lattice. A rung below
                // it the filler measured `x3 of 27` on the smallest ship and the
                // hull photographed as bare grey between the painted lines. A
                // panel this size does not need a sparser grid to look
                // deliberate: it very nearly fills its own plate, so a field of
                // them reads as plating rather than as specks.
                placement: FixturePlacement {
                    region: FixtureRegion::Anywhere,
                    density: FixtureDensity::Dense,
                    orientation: FixtureOrientation::Along,
                },
            },
        ],
    }
}

/// ARMOURED: a warship that shows a gunner as little as it can.
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
/// the built-in blue slate onto a desaturated gunmetal, matte and metallic-free,
/// which is the in-fiction answer as much as the graphic one - a warship does
/// not want a specular highlight.
///
/// The kit, in PRIORITY order, and what each reads. Ten pieces after the
/// vocabulary batch (task 20260816-222644), and still the smallest kit of the
/// four BY DOCTRINE: everything sealed, everything numbered, every new piece
/// low, flush and bolted, colour budget gunmetal plus ONE stencil white -
/// nothing lit, nothing specular:
///
/// 1. the sensor blister takes the PANELS on the rarest rung - the thinnest
///    share there is, with a patch floor under it - so a hull wears a handful
///    of them wherever it is broad enough to carry one. The rarest piece and
///    the only one that stands proud;
/// 2. the mast is the silhouette's one spike, shortest of the four styles' tall
///    pieces, and it is sampled before the cap sweeps the same high ground;
/// 3. the corner boss reads the HIGH GROUND - the tips and outer corners - which
///    is the armour a ship is hit on;
/// 4. the belt reads the EDGE, the straight side of a hull and its largest
///    single bucket, turned down the RUN. It is the whole look;
/// 5. the intake is a vent CLOSED - a shuttered slit down the panel runs;
/// 6. the chaff tube is the sparse accent, sampled before the fillers;
/// 7. the hatch is the flush filler, giving a bare panel a scale without giving
///    a gunner a target;
/// 8. the ammo stripes read the gun pockets, pinned below every zone rule
///    because `NearFitting` is the documented carpet, and TRIM because the
///    plates beside a boom-mounted gun are cones (bench-proven on
///    `asym_gunship`);
/// 9. the magazine is the chunky rarity, and it stands ANYWHERE, so it waits
///    until the zone rules have taken the plates they can name;
/// 10. the applique tile grid is the trim filler, LAST - the kit's reach onto a
///     hull with no seated plate on it at all.
fn armoured_style(assets: &BaseContentAssets) -> ShipStyleConfig {
    ShipStyleConfig {
        id: ARMOURED_STYLE_ID.to_string(),
        name: "Armoured".to_string(),
        // THE PALETTE, and it is half the look. Linear values, so they read
        // considerably lighter on screen than they look here.
        //
        // - the TOP is gunmetal: barely cool, all but desaturated, metallic
        //   zero, and less than half the albedo of the built-in plate. Matte
        //   armour, because a warship presents no highlight to be seen by. The
        //   hue is MEASURED, not guessed - the first cut ran a neutral 0.155
        //   and the row came back a warm cream liner, because the photo rig's
        //   key is warm and a neutral plate takes its colour;
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
                // The PANELS, on the thinnest rung there is: a tenth share
                // with a patch floor under it, which comes out as a blister or
                // two per block of ship that can carry one. Anything heavier
                // puts four on one flank, and the piece is rare enough that
                // where it lands is visible.
                //
                // FIRST, and it has to be. The two `Dense` panel rules below it
                // would otherwise take every plate this one is eligible for,
                // and a rung this thin has almost nothing left to hash against.
                // Priority alone is still not enough on a small hull - the
                // floor has to BORROW a plate back off a denser rule for the
                // rung to land at all - which is the engine rule this kit is
                // the worked example of.
                //
                // `Rare` rather than a hand-written filter is also what carries
                // the piece onto a small hull. Measured, the filter this rule
                // used to spell out by hand described a generated hull's broad
                // fields and reached ZERO plates on the entire hand-built
                // bench. The region plus the seat gate now do the gating - the
                // piece stays on coplanar panel.
                placement: FixturePlacement {
                    region: FixtureRegion::Panel,
                    density: FixtureDensity::Rare,
                    orientation: FixtureOrientation::Free,
                },
            },
            StyleFixtureConfig {
                id: "armoured_mast".to_string(),
                model: assets.greeble_armoured_mast.clone(),
                health: 8.0,
                collider: Vec3::new(0.16, 0.24, 0.16),
                // The HIGH GROUND, read the way every mast in this file reads
                // it. What makes it armoured is the RATION - a warship hides
                // its silhouette, so this is a thinner rung than the
                // industrial stack's and the piece itself is the shortest tall
                // piece of the four styles. The rung's patch floor keeps the
                // ration honest rather than absent: one spike per block of high
                // ground, so a hull reads manned without reading bristling.
                //
                // BEFORE the cap, and it has to be: the cap sweeps the same
                // high ground at full cover, and a sparse accent sampled out of
                // a carpet's leavings logs `x0` - the measured starvation trap
                // the industrial hatch documents.
                placement: FixturePlacement {
                    region: FixtureRegion::HighGround,
                    density: FixtureDensity::Sparse,
                    orientation: FixtureOrientation::Outward,
                },
            },
            StyleFixtureConfig {
                id: "armoured_cap".to_string(),
                model: assets.greeble_armoured_cap.clone(),
                health: 26.0,
                collider: Vec3::new(0.3, 0.1, 0.3),
                // The high ground is a third of a hull, and `Every` is the
                // point: every outer corner takes a boss, all the way round.
                // Reinforcing only the corners on top was the first cut and it
                // is a worse sentence - a corner is a corner from whatever side
                // it is shot at - and it left the flanks bare.
                //
                // Everything this rule used to spell out came off after the
                // reach was logged. A depth floor and a height floor together
                // with a facing took it to `x0 of 0` and `x2 of 2`: a spur is
                // the TIP of something, so there is rarely two cells of ship
                // under it and it fills almost none of its own cell. Three
                // filters that each read as mild, multiplied, made an
                // impossible rule out of a bucket 42 plates deep. The region
                // now carries the one depth floor that is safe, and nothing
                // else.
                //
                // The crest of a one-cell-wide run and the top of a lone stud
                // are the same pointy armour the boss is for, and they are what
                // a hand-built L is MADE of - which is why the high ground takes
                // them all. Measured before it did, the owner's own build wore
                // zero armoured pieces: no other armoured rule can touch a
                // creased plate at all.
                placement: FixturePlacement {
                    region: FixtureRegion::HighGround,
                    density: FixtureDensity::Every,
                    orientation: FixtureOrientation::Free,
                },
            },
            StyleFixtureConfig {
                id: "armoured_strake".to_string(),
                model: assets.greeble_armoured_strake.clone(),
                health: 30.0,
                collider: Vec3::new(0.3, 0.085, 0.95),
                // THE LOOK. Every edge plate in a run of two or more - the
                // piece is 0.95 cells long, which is what asks for the run -
                // turned down that run, and the model very nearly fills its
                // plate, so neighbours meet and a run comes out as ONE unbroken
                // strake rather than as a dashed line. That is the "decoration
                // cannot span cells" ceiling paid off with the only currency
                // available: a piece that fills its own cell.
                //
                // `Every`, which for any other rule would be a carpet. Here it
                // is the point: an armour belt with gaps in it is not an armour
                // belt, and the pieces are all one line rather than 40 objects.
                placement: FixturePlacement {
                    region: FixtureRegion::Edge,
                    density: FixtureDensity::Every,
                    orientation: FixtureOrientation::Along,
                },
            },
            StyleFixtureConfig {
                id: "armoured_intake".to_string(),
                model: assets.greeble_armoured_intake.clone(),
                health: 16.0,
                collider: Vec3::new(0.2, 0.06, 0.44),
                // The VENT class in this style's voice: shuttered, flush, laid
                // down the panel runs. `Panel` and not `Edge`, because the belt
                // owns every straight edge at full cover and a slit punched into
                // the belt would hole the one continuous line the look is built
                // on. Six hundredths of a cell proud, so it is TRIM and the thin
                // subjects get it too. The rung keeps it a scatter of shutters
                // rather than a grille field, and its patch floor keeps a small
                // hull from losing the class entirely.
                placement: FixturePlacement {
                    region: FixtureRegion::Panel,
                    density: FixtureDensity::Dense,
                    orientation: FixtureOrientation::Along,
                },
            },
            StyleFixtureConfig {
                id: "armoured_chaff".to_string(),
                model: assets.greeble_armoured_chaff.clone(),
                health: 12.0,
                collider: Vec3::new(0.3, 0.13, 0.18),
                // The one piece that hints at the ship's combat life, so it is
                // rationed like the mast and sampled BEFORE the fillers for
                // the same starvation reason. `Outward` turns the tube's long
                // axis off the ship where the plate falls away - a decoy
                // launches overboard - and a plate with no fall leaves it
                // square, which is the rest of the hull.
                placement: FixturePlacement {
                    region: FixtureRegion::Panel,
                    density: FixtureDensity::Sparse,
                    orientation: FixtureOrientation::Outward,
                },
            },
            StyleFixtureConfig {
                id: "armoured_hatch".to_string(),
                model: assets.greeble_armoured_hatch.clone(),
                health: 14.0,
                collider: Vec3::new(0.3, 0.05, 0.3),
                // The broadest panel rule in the kit, because a flush hatch is
                // the one piece that cannot spoil a silhouette - five hundredths
                // of a cell tall, which is also what makes it TRIM and lets it
                // lie on a panel that creases.
                placement: FixturePlacement {
                    region: FixtureRegion::Panel,
                    density: FixtureDensity::Dense,
                    orientation: FixtureOrientation::Free,
                },
            },
            StyleFixtureConfig {
                id: "armoured_ammo_stripes".to_string(),
                model: assets.greeble_armoured_ammo_stripes.clone(),
                health: 6.0,
                collider: Vec3::new(0.3, 0.02, 0.34),
                // The MARKING class, and the kit's one job for stencil white:
                // a rounds-count tally beside every gun well. BELOW every rule
                // that names a zone - `NearFitting` is the measured carpet, and
                // no kit lets it pick before a rule that can say where it goes.
                //
                // `NearFitting` and `Every` are both the HARD CONSTRAINT from
                // the blocks bench (task 20260816-203837 closure), and the
                // collider is the third: the plates beside a boom-mounted gun
                // are CONES, so the tally has to be TRIM - two centimetres of
                // paint, which is free of the seat gate - or it never fires on
                // `asym_gunship` at all. `NearFitting` asks nothing of depth
                // (a boom is one cell thin) or of how much of its cell the
                // plate fills (the cone beside a gun is a spur, and a spur
                // measures 0). `Every` is the rung with NO lattice, because the
                // carrier deck's tucked drive proved lattice parity can zero a
                // strided pocket rule outright - and with no share either, so
                // nothing can hash its way out of a tally.
                placement: FixturePlacement {
                    region: FixtureRegion::NearFitting,
                    density: FixtureDensity::Every,
                    orientation: FixtureOrientation::Along,
                },
            },
            StyleFixtureConfig {
                id: "armoured_magazine".to_string(),
                model: assets.greeble_armoured_magazine.clone(),
                health: 22.0,
                collider: Vec3::new(0.34, 0.12, 0.28),
                // The POWER CELL class. A LOW rung because the drum's measured
                // lesson (a third of a share gave a hull 13-14 drums) says
                // external stores at any real share read as a depot, and its
                // patch floor turns that into one low box per block of hull -
                // the doctrine statement: this ship carries rounds, and it does
                // not show you how many. The stripes do the counting.
                //
                // `Anywhere` names no zone, so it waits until the rules that
                // can name one have picked; above the applique because a box is
                // an accent and the tile grid is a filler.
                placement: FixturePlacement {
                    region: FixtureRegion::Anywhere,
                    density: FixtureDensity::Sparse,
                    orientation: FixtureOrientation::Along,
                },
            },
            StyleFixtureConfig {
                id: "armoured_applique".to_string(),
                model: assets.greeble_armoured_applique.clone(),
                health: 24.0,
                collider: Vec3::new(0.42, 0.04, 0.42),
                // The kit's TRIM filler (finding 1.5.3 of the greeble spike):
                // `Anywhere` asks nothing about relief, and reactive tiles are
                // 0.04 cells proud, which puts them under the trim height and
                // drops the seat gate with it. That pair is what reaches a
                // hand-built hull - one cell thick, cones end to end - where a
                // kit whose every filler is seated leaves exactly the shapes
                // players build bare. AFTER the hatch: on thick hulls the panel
                // filler keeps first refusal and the grid takes the leavings
                // plus the whole thin regime the hatch cannot touch.
                placement: FixturePlacement {
                    region: FixtureRegion::Anywhere,
                    density: FixtureDensity::Dense,
                    orientation: FixtureOrientation::Free,
                },
            },
        ],
    }
}

/// The CIVILIAN look: a private yacht's, and a ship built to be SOLD.
///
/// Two things carry it, and only two. The PAINT: the undressed derivation is a
/// cool dark slate, near enough to bare metal that a "clean" look built on
/// subtle greebling would be indistinguishable from no style at all, so this
/// one repaints the hull pale, satin and non-metallic - an airframe, not plate.
/// And the LIVERY: one continuous cobalt band down every straight edge of the
/// hull, which is the only piece a customer would ever have asked for.
///
/// Everything else is restraint, which the vocabulary batch (task
/// 20260816-222651) names FINISH: function hidden under styling, where the
/// armoured kit hides it under suppression. The kit carries exactly two
/// accents - COBALT for anything painted and AMBER for anything lit - and
/// every shell among the twelve pieces is hull-coloured, so the kit reads as
/// one livery rather than as twelve props. Detail flows LENGTHWISE: every
/// long piece is authored long on Z and aligned to the run, the aero grammar,
/// and nothing here clusters - clustered detail is machinery showing, which a
/// sold hull never allows.
///
/// In priority order, and the reason for each place in it:
///
/// 1. the WINDOW row is the most specific piece and the one that says what
///    the ship is FOR, so it gets first refusal on the flanks;
/// 2. the DISH is the kit's rarest share, and a rare rule sampled after a
///    broad one is starved into `x0` - so the faired radome looks at the panel
///    before anything broad does;
/// 3. the DOOR takes what is left of the flanks: cabins outrank a way in on a
///    short side, but a hull with no door at all reads as a drone;
/// 4. the TANK is the other rare share, faired tankage on the thick body;
/// 5. the FIN is the only piece that breaks the silhouette, on the high
///    ground and raked outboard - the high ground is a small bucket, so a
///    heavy rung there still comes out as a few blades;
/// 6. the STRIPE is the look. See its own note - it is the one rule here that
///    had to be measured into existence rather than written. It is the kit's
///    only claim on the EDGE, so the band is never punctured;
/// 7. the LIVERY panel is commerce, the civilian voice, and the first of the
///    four panel rules because an advert is the one a customer paid for;
/// 8. the SKYLIGHT is the window row's claim made to a viewer looking DOWN -
///    the attitude every bench render and chase camera actually holds;
/// 9. the VENT is the service read, faired, because on this hull even the
///    machinery access is a styling exercise;
/// 10. the FAIRING is the filler on panelling, hull-coloured on purpose, and
///     last of the panel rules;
/// 11. the BEACON reads the pocket, below every rule that names a zone -
///     `NearFitting` is the measured carpeting trap, and high in a list it
///     starves everything under it;
/// 12. the REGISTRY mark is the kit's TRIM carrier and mostly a per-block
///     floor - every hull that left a yard is registered SOMEWHERE, and
///     exactly once per neighbourhood reads as paperwork rather than as
///     pattern. LAST, because paint lies on anything and would otherwise take
///     plate a named rule wanted.
fn civilian_style(assets: &BaseContentAssets) -> ShipStyleConfig {
    ShipStyleConfig {
        id: CIVILIAN_STYLE_ID.to_string(),
        name: "Civilian".to_string(),
        // LINEAR values. The top is ~0.68 sRGB against the built-in 0.36: a
        // painted airframe, not a slate. Metallic 0 and roughness 0.35 is
        // satin PAINT - the one material choice that separates a hull sold to
        // a customer from one welded out of plate.
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
                // The FLANKS - deck plate turned out rather than up, which is
                // where a liner draws its window line, and the one region that
                // keeps the rule off the deck edges the stripe owns end to end.
                //
                // Six hundredths of a cell proud, so the row is TRIM and the
                // seat gate lets it lie on a flank that creases. That matters
                // more here than anywhere: measured, a hand-built flank never
                // grows a coplanar interior - a two-cell-tall side is edge row
                // on edge row - so a seated window row was eligible NOWHERE on
                // the whole bench and the style's most characterful piece never
                // shipped. The piece's own length asks for two cells of like
                // flank, which is what keeps a row a row.
                placement: FixturePlacement {
                    region: FixtureRegion::Flank,
                    density: FixtureDensity::Dense,
                    orientation: FixtureOrientation::Along,
                },
            },
            StyleFixtureConfig {
                id: "civilian_dish".to_string(),
                model: assets.greeble_civilian_dish.clone(),
                health: 5.0,
                collider: Vec3::new(0.28, 0.14, 0.28),
                // A faired SATCOM radome, the antenna as a product. The
                // thinnest rung in the kit, so it is sampled right after the
                // windows: a sparse rule under a broad one logs `x0` while
                // looking starved rather than absent. Panel face only - a dome
                // on a flank reads as a defect - and the rung's patch floor is
                // what actually places it: one radome per block of panel is an
                // appliance, a row of them is an array farm.
                placement: FixturePlacement {
                    region: FixtureRegion::Panel,
                    density: FixtureDensity::Sparse,
                    orientation: FixtureOrientation::Free,
                },
            },
            StyleFixtureConfig {
                id: "civilian_door".to_string(),
                model: assets.greeble_civilian_door.clone(),
                health: 8.0,
                collider: Vec3::new(0.26, 0.06, 0.44),
                // The access class as the customer sees it: a flush leaf with
                // a painted outline and a courtesy light, on the FLANKS where
                // a boarding party would actually stand. Under the window row
                // so cabins outrank doors on a short flank, but ABOVE every
                // panel rule and that was measured: drafted down among the
                // fillers it took nothing on any bench seed, because the window
                // row is `Dense` and a flank is a small bucket. The rung's
                // patch floor is the rest of it - a hull with no way in reads
                // as a drone: every ship gets a door, few get two.
                placement: FixturePlacement {
                    region: FixtureRegion::Flank,
                    density: FixtureDensity::Regular,
                    orientation: FixtureOrientation::Along,
                },
            },
            StyleFixtureConfig {
                id: "civilian_tank".to_string(),
                model: assets.greeble_civilian_tank.clone(),
                health: 12.0,
                collider: Vec3::new(0.24, 0.15, 0.4),
                // The power-cell class in the civilian voice: tankage exists,
                // but only its FAIRING shows. Chunky, so it takes the drum's
                // measured lesson whole: `Deck` keeps it on the thick body, and
                // the rung is thin because a third of a share put 13-14 drums on
                // one bench hull - a scrapyard, not a sponson line. Aligned to
                // the run so a row of blisters reads as one faired sponson down
                // the hull.
                placement: FixturePlacement {
                    region: FixtureRegion::Deck,
                    density: FixtureDensity::Sparse,
                    orientation: FixtureOrientation::Along,
                },
            },
            StyleFixtureConfig {
                id: "civilian_fin".to_string(),
                model: assets.greeble_civilian_fin.clone(),
                health: 5.0,
                collider: Vec3::new(0.1, 0.29, 0.24),
                // The high ground, turned OUTBOARD so the blade rakes off the
                // ship. The region asks nothing about how much of its cell the
                // plate fills and drops the seat gate, and both are the same
                // argument: a crest fills an eighth of its cell and a spur tip
                // less, every one of them is a crease, and a fin belongs on
                // exactly them.
                placement: FixturePlacement {
                    region: FixtureRegion::HighGround,
                    density: FixtureDensity::Dense,
                    orientation: FixtureOrientation::Outward,
                },
            },
            StyleFixtureConfig {
                id: "civilian_stripe".to_string(),
                model: assets.greeble_civilian_stripe.clone(),
                health: 8.0,
                collider: Vec3::new(0.3, 0.09, 0.6),
                // THE LOOK. A continuous band, and every part of this rule is
                // there to keep it continuous:
                //
                // - `Edge` is the straight side of a hull, and the only region
                //   whose run is a LINE rather than a patch;
                // - the piece is 0.6 cells long, which asks for two cells of
                //   edge under it. A one-cell corner is a stub, not a livery;
                // - `Every` - the one rung with no lattice and no share. Both
                //   turn a line back into dashes, and one missing cell in a band
                //   reads as a mistake rather than as sparsity;
                // - `Along` lays each piece down the edge, and the model fills
                //   its whole cell along that axis, so neighbours butt together
                //   into one unbroken rail.
                //
                // It was authored the other way first, as a rounded terminal
                // plus a body rule keyed on how far in from the end of a run a
                // plate sits, and the diagnostic killed it in one run:
                // `civilian_stripe x0 of 0`. A hull edge always has open space
                // on one side and unlike plate on the other, so it reads as the
                // end of its own run ALWAYS. The terminal rule therefore took
                // the whole edge and the band never landed - a row of pills with
                // a gap between each, which photographs as a dashed line and is
                // why the piece it used to need is not in this kit.
                //
                // The collider is deliberately SHORTER than the model. Every
                // other piece here is smaller than its cell, but this one fills
                // it, and two full-length boxes on adjacent plates would be two
                // bodies born overlapping.
                placement: FixturePlacement {
                    region: FixtureRegion::Edge,
                    density: FixtureDensity::Every,
                    orientation: FixtureOrientation::Along,
                },
            },
            StyleFixtureConfig {
                id: "civilian_livery".to_string(),
                model: assets.greeble_civilian_livery.clone(),
                health: 6.0,
                collider: Vec3::new(0.3, 0.03, 0.46),
                // COMMERCE, the voice no other kit has: a sold hull sells its
                // own panelling. Three hundredths of a cell proud, so it is
                // TRIM and lies on panel that creases - and it takes the PANEL
                // rather than the edge on purpose, so an advert never lands on
                // a cell of the band. Aligned to the run like every long piece
                // here: adverts read down the hull, not across it.
                //
                // FIRST of the four panel rules, and measured: below them it
                // took nothing on any bench seed, because a `Dense` fairing and
                // two more mid rules empty the bucket. Of the four this is the
                // one the customer paid for, so it picks first.
                placement: FixturePlacement {
                    region: FixtureRegion::Panel,
                    density: FixtureDensity::Regular,
                    orientation: FixtureOrientation::Along,
                },
            },
            StyleFixtureConfig {
                id: "civilian_skylight".to_string(),
                model: assets.greeble_civilian_skylight.clone(),
                health: 6.0,
                collider: Vec3::new(0.2, 0.05, 0.55),
                // The passenger read for a viewer looking DOWN, which is the
                // attitude the bench and the chase camera both hold - the window
                // row faces sideways, so the strongest civilian tell was
                // invisible from above. The piece is 0.55 cells long and asks
                // for two cells of like panel, because a cabin line is a LINE:
                // one lit strip alone reads as a hatch, two butted down a run
                // read as a deck of paying passengers. Five hundredths proud, so
                // it is TRIM and a narrow deck that creases still glazes.
                //
                // `Panel` and not `Edge`: the band owns every long edge at full
                // cover, and the one continuous line the look rests on is never
                // punctured from above.
                placement: FixturePlacement {
                    region: FixtureRegion::Panel,
                    density: FixtureDensity::Regular,
                    orientation: FixtureOrientation::Along,
                },
            },
            StyleFixtureConfig {
                id: "civilian_vent".to_string(),
                model: assets.greeble_civilian_vent.clone(),
                health: 5.0,
                collider: Vec3::new(0.18, 0.09, 0.4),
                // The vent class with the machinery hidden: a faired scoop
                // whose one dark surface is the mouth itself, amber-lit. NOT
                // `NearFitting` - the beacon owns the pocket in this kit and
                // two rules on one halo carpet it (the measured trap). A scoop
                // rides the open panel runs instead, aligned down them, so
                // intakes flow along the hull the way the whole kit does.
                placement: FixturePlacement {
                    region: FixtureRegion::Panel,
                    density: FixtureDensity::Regular,
                    orientation: FixtureOrientation::Along,
                },
            },
            StyleFixtureConfig {
                id: "civilian_fairing".to_string(),
                model: assets.greeble_civilian_fairing.clone(),
                health: 14.0,
                collider: Vec3::new(0.4, 0.16, 0.4),
                // The filler, and the only rule with no accent on it: a
                // hull-coloured shell over whatever a working ship would have
                // left exposed. `Panel` rather than the window row's flank - a
                // fairing belongs on the roof as much as the side - and a heavy
                // rung, because on this kit the filler is the paint's own
                // texture. The rung's patch floor carries it onto a small build,
                // where the share alone would leave nothing.
                //
                // It stands too proud to be trim, so a creased hull refuses it.
                // The kit's reach onto a one-cell-thick build is the fin and the
                // registry mark instead.
                placement: FixturePlacement {
                    region: FixtureRegion::Panel,
                    density: FixtureDensity::Dense,
                    orientation: FixtureOrientation::Free,
                },
            },
            StyleFixtureConfig {
                id: "civilian_beacon".to_string(),
                model: assets.greeble_civilian_beacon.clone(),
                health: 4.0,
                collider: Vec3::new(0.2, 0.1, 0.2),
                // BELOW every rule that names a zone: "near a fitting" reads as
                // narrow and is not, and high in a priority list it carpeted
                // 45% of a measured hull. Down here it lights the drive bays
                // and the gun wells with whatever plate the ten rules above did
                // not want.
                placement: FixturePlacement {
                    region: FixtureRegion::NearFitting,
                    density: FixtureDensity::Dense,
                    orientation: FixtureOrientation::Free,
                },
            },
            StyleFixtureConfig {
                id: "civilian_registry".to_string(),
                model: assets.greeble_civilian_registry.clone(),
                health: 4.0,
                collider: Vec3::new(0.13, 0.03, 0.6),
                // The marking class, and the kit's TRIM carrier (task
                // 20260816-222651, the batch's cone brief): `Anywhere` asks
                // nothing about relief or depth, and three hundredths of a cell
                // of stand-off drops the seat gate, because paint lies on any
                // facet a one-cell-thick build derives - the fairing alone was
                // one coin toss per thin hull. The share is nearly off and the
                // patch floor does the placing: a registry mark is PAPERWORK,
                // one per neighbourhood, and a hull sprayed with them reads as
                // a pattern rather than as insurance.
                //
                // LAST, and that is what a trim carrier on `Anywhere` is for:
                // paint lies on any facet, so sampled higher it would take
                // plate a rule that can name its zone wanted.
                placement: FixturePlacement {
                    region: FixtureRegion::Anywhere,
                    density: FixtureDensity::Sparse,
                    orientation: FixtureOrientation::Along,
                },
            },
        ],
    }
}

/// SALVAGE: a hull assembled out of whatever was to hand and repaired more than
/// once. The raiders' look.
///
/// # Making a deterministic scatter read as haphazard
///
/// Everything here is a pure function of the structure, hashed off the cell -
/// and what is wanted is a hull that reads as if nobody planned it. Those are
/// not in conflict, because MISMATCH IS A PATTERN TOO. Four devices carry it,
/// and none of them is jitter:
///
/// 1. **Three patches, three materials, one territory each.** A hash is
///    spatially INCOHERENT, so splitting one rule three ways by share would
///    give a hull evenly speckled in three colours - confetti. The territories
///    are structural instead, and structure is coherent: the livery-green strip
///    takes the fittings so it clusters round the drives and wells a ship gets
///    shot at, the bare-steel plate takes the open body at a middling rung, and
///    the rusted scab is thinner still and mostly its own patch floor over
///    everything else. So a REGION comes out mostly one material with the others
///    cutting
///    into it at its edges, which is what a hull repaired twice looks like.
/// 2. **The long axes CROSS.** `salvage_patch_plate` is authored long on X and
///    `salvage_patch_strip` long on Z, and both align to the run. So where
///    their territories meet, two neighbouring repairs lie at right angles to
///    each other while every piece stays square to the grid. This is the one
///    place the look breaks the alignment rule the research settled on, and it
///    breaks it by a quarter turn rather than by a random angle - deliberate,
///    not noisy.
/// 3. **Every piece is authored OFF-CENTRE.** The scatter puts a fixture at the
///    middle of its plate and offers no jitter, deliberately, so the asymmetry
///    is in the RECIPES: each patch overhangs one side of its cell, the drum's
///    straps are not symmetric about its own axis, and the cleat's two posts
///    are different heights. A run of the same piece therefore does not read as
///    a repeated tile.
/// 4. **The weld bead is hand-run.** `salvage_weld_seam` is four lumps of
///    different size wandering either side of a line, not the `ribs` primitive:
///    even ribbing reads as a machined surface, which is the opposite claim.
/// 5. **Two pieces are nearly a cell across, and stop being dashes.** The two
///    patches and the bead raise their own footprint budget to 0.86-0.94, so
///    consecutive plates of one run JOIN. The default half cell guards against
///    a piece spilling over a seam when it is scattered off the plate centre,
///    and the scatter has no such jitter - a fixture always stands at the
///    middle of its plate.
/// 6. **The seam is thinned by the RUN, not by a share.** It takes every edge
///    plate it can reach, and its own length is what asks for a run under it, so
///    a long hull edge is welded down its whole length and a short one is left
///    bare. Which edges carry a seam is then a property of the hull, and a share
///    would only have dotted every edge equally - the confetti failure again.
///
/// One thing here is a KNOWN COST kept on purpose. A piece is lifted to its
/// plate's MEAN height, so a sheet does not sit flush on a plate that falls
/// away: the surface's height varies ALONG the fall, and a piece with a long
/// extent in that direction stands proud at one end. `salvage_patch_plate` is
/// long on X and aligned to the run, which puts its long axis square to the run
/// and therefore down the fall, so it is the piece this happens to most. On any
/// other look that is a defect to design out. Here it is kept: a repair that
/// does not quite lie down is what a repair looks like, and a perfectly bedded
/// one reads as factory trim. It is a small effect - a fraction of a cell - and
/// the render is the honest measure of it, not this comment.
///
/// # The identity is spread across regions on purpose
///
/// No one piece carries the look. The whip reads the high ground, the drum the
/// broad decks, the seam the straight edges, the two big patches the body and
/// the fittings, and the scab whatever is left. That is partly what mismatch
/// means, and partly insurance: a look concentrated on ONE region is hostage to
/// which way that part of a hull happens to face, and a hull whose edges all
/// point away from the camera then shows no accent at all. Every piece here
/// lands on all three subject ships.
///
/// # Priority order
///
/// Narrow and striking first, broad filler last, as a plate takes one piece.
/// The sparse accents - whip, drum, cleat - are all sampled BEFORE any filler,
/// because a rare rule placed after a broad one is starved out of its leavings
/// and logs `x0` while looking identical on screen to a rule that matched
/// nothing. The two rules the traps live in are placed accordingly:
/// `NearFitting` is fifth and eighth rather than first, because it reads as
/// narrow and is not.
///
/// # The batch pieces (task 20260816-222658) slot into the same allocation
///
/// Seven pieces doubled the kit without adding a rule SHAPE it did not have:
/// the dish is a second rationed silhouette breaker (flanks, as the whip is the
/// high ground), the net shares the drum's decks, the chain rigs a scattering
/// of the edges the seam would otherwise bead end to end, the hose is the duct
/// class at a
/// share instead of full cover, the grille and the cog sit on the body ahead of
/// the broad plate, and the kill tally is a second trim carrier ahead of the
/// scab. The ham budget from the art direction (GREEBLES.md section 2) is
/// enforced by the rungs here: the one new silhouette breaker (the dish) is
/// rationed under the whip, every other new rule is thinned at least as hard as
/// the piece it sits beside, and the one new hue in the whole batch rides the
/// dish recipe alone.
fn salvage_style(assets: &BaseContentAssets) -> ShipStyleConfig {
    ShipStyleConfig {
        id: SALVAGE_STYLE_ID.to_string(),
        name: "Salvage".to_string(),
        // WARM and dirty, against the built-in cool slate. This is half the
        // look and it costs nothing: oxidised brown plate is what the scrap
        // steel, the rust and the faded livery green of the patches all have to
        // read AGAINST, and against the stock blue-grey the bare-metal pieces
        // read as clean rather than as scavenged. Nearly no metallic and nearly
        // all roughness - a hull nobody has polished since they stole it.
        //
        // The TOP-TO-WALL RATIO is 6:1, against the placeholder's 1.5:1. That
        // is a SILHOUETTE device and not a panelling one, measured: two plates
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
                // The HIGH GROUND, and the same reading the placeholder mast
                // takes. The region asks nothing about how much of its cell the
                // plate fills and drops the seat gate, which is what a mast
                // needs: a crest fills an eighth of its cell, a spur tip less,
                // and every one of them is a crease.
                //
                // `Outward` is what makes it a raider's aerial: the whip is
                // modelled leaning down its own `+Z`, so the fall turns it out
                // over the edge it stands on rather than upright.
                placement: FixturePlacement {
                    region: FixtureRegion::HighGround,
                    density: FixtureDensity::Sparse,
                    orientation: FixtureOrientation::Outward,
                },
            },
            StyleFixtureConfig {
                id: "salvage_dish".to_string(),
                model: assets.greeble_salvage_dish.clone(),
                health: 10.0,
                collider: Vec3::new(0.33, 0.24, 0.31),
                // Someone else's comms dish, bolted to a FLANK where it can
                // see - the second silhouette breaker in the kit, and rationed
                // at the same thin rung as the whip, for the same reason the
                // whip is rationed at all: a hull bristling with breakers reads
                // as a porcupine, not a raider. The rung keeps its lattice, so
                // two stolen dishes never stand shoulder to shoulder, which no
                // crew would rig.
                //
                // This is the piece the kit's ONE foreign hue rides on - the
                // faded cobalt lives in the recipe, nowhere else - so the rule
                // has to keep it rare enough that the accent stays an anecdote
                // ("that came off a liner") rather than a fourth patch colour.
                placement: FixturePlacement {
                    region: FixtureRegion::Flank,
                    density: FixtureDensity::Sparse,
                    orientation: FixtureOrientation::Free,
                },
            },
            StyleFixtureConfig {
                id: "salvage_drum".to_string(),
                model: assets.greeble_salvage_drum.clone(),
                health: 24.0,
                collider: Vec3::new(0.22, 0.23, 0.34),
                // External tankage needs somewhere broad and solid to be
                // strapped to, which is what `Deck` means: flat plate with real
                // ship under it, plus the shoulder where one deck climbs past
                // the next - a corner to wedge a drum into, and on a small hull
                // the only one there is.
                placement: FixturePlacement {
                    region: FixtureRegion::Deck,
                    density: FixtureDensity::Sparse,
                    orientation: FixtureOrientation::Along,
                },
            },
            StyleFixtureConfig {
                id: "salvage_net".to_string(),
                model: assets.greeble_salvage_net.clone(),
                health: 14.0,
                collider: Vec3::new(0.38, 0.16, 0.57),
                // Lashed loot shares the drum's zone - the broad, solid decks
                // are where a crew straps things down - and is sampled right
                // AFTER it, so the two chunky pieces interleave on the same
                // ground instead of one carpeting it: the drum takes its share
                // first and the net takes the same share of what is left. Same
                // region and the same rung on purpose; a shared zone with a
                // shared dominant material (green tarp beside green patches) is
                // accumulation, two zones of one piece each would be
                // allocation.
                placement: FixturePlacement {
                    region: FixtureRegion::Deck,
                    density: FixtureDensity::Sparse,
                    orientation: FixtureOrientation::Along,
                },
            },
            StyleFixtureConfig {
                id: "salvage_hook".to_string(),
                model: assets.greeble_salvage_hook.clone(),
                health: 14.0,
                collider: Vec3::new(0.20, 0.11, 0.14),
                // The tight ring round a fitting: the four cells beside a
                // nozzle or a gun well. Fifth and not first, and on a rung that
                // keeps its lattice and a share, because this rule is the
                // documented carpet - first in the list it took 45% of a ship.
                placement: FixturePlacement {
                    region: FixtureRegion::NearFitting,
                    density: FixtureDensity::Dense,
                    orientation: FixtureOrientation::Free,
                },
            },
            StyleFixtureConfig {
                id: "salvage_chain".to_string(),
                model: assets.greeble_salvage_chain.clone(),
                health: 12.0,
                collider: Vec3::new(0.11, 0.13, 0.88),
                // THE RIGGING on the edges. Sampled BEFORE the seam, and that
                // was measured: the seam is `Every`, so under it this rule took
                // nothing on any of the three bench seeds. Two rules on one
                // region cannot split it by priority alone - the thin one has to
                // pick first or it never picks at all.
                //
                // What keeps the seam whole is the chain's own RUNG, not its
                // place: a small share on a lattice takes a scattering of edge
                // plates and leaves the rest of every run to the bead. That is
                // the read wanted either way - a crew laces the edges they tow
                // from, and every edge wearing chain would be the uniform
                // machined finish this kit exists to refuse.
                placement: FixturePlacement {
                    region: FixtureRegion::Edge,
                    density: FixtureDensity::Sparse,
                    orientation: FixtureOrientation::Along,
                },
            },
            StyleFixtureConfig {
                id: "salvage_weld_seam".to_string(),
                model: assets.greeble_salvage_weld_seam.clone(),
                health: 18.0,
                collider: Vec3::new(0.2, 0.06, 0.92),
                // The STRAIGHT EDGE of a hull and nothing else, which is the
                // one region with a long line to run a bead down.
                //
                // `Every`, and that is the whole point of it: the bead is 0.92
                // of a cell long, so consecutive plates of one run JOIN into an
                // unbroken seam. A share here would dot the line instead, and a
                // dotted line is the confetti failure wearing a different hat.
                //
                // The bead's own length is then the only thing thinning it, and
                // it is what stops the piece reading as machined trim: an edge
                // with a real length to it is welded down its whole length and a
                // short one is left bare, so the seams are a property of the
                // hull rather than a uniform finish on it. A longer gate was
                // tried and was too tight - the third subject ship has no edge
                // run that long anywhere and logged `x0 of 0`, and a rule that
                // lands nothing on one hull in three is not a sparse accent, it
                // is a broken filter.
                placement: FixturePlacement {
                    region: FixtureRegion::Edge,
                    density: FixtureDensity::Every,
                    orientation: FixtureOrientation::Along,
                },
            },
            StyleFixtureConfig {
                id: "salvage_patch_strip".to_string(),
                model: assets.greeble_salvage_patch_strip.clone(),
                health: 18.0,
                collider: Vec3::new(0.45, 0.05, 0.84),
                // The livery-green offcut, round the FITTINGS: a drive bay
                // comes out with cleats right against the mouth and green
                // plating over the cells beside it, which is the damage pattern
                // the look is about - a ship shot where its fittings are and
                // patched there.
                //
                // Measured, and the fitting trap caught this rule too: a halo
                // two cells out reads as narrow and admitted 80-92 plates of a
                // 132-162 plate hull, better than half the ship. `NearFitting`
                // is the four cells beside a mouth and means what it says.
                placement: FixturePlacement {
                    region: FixtureRegion::NearFitting,
                    density: FixtureDensity::Regular,
                    orientation: FixtureOrientation::Along,
                },
            },
            StyleFixtureConfig {
                id: "salvage_hose".to_string(),
                model: assets.greeble_salvage_hose.clone(),
                health: 8.0,
                collider: Vec3::new(0.13, 0.09, 0.87),
                // The services run, in this kit's voice: where the industrial
                // duct takes every panel cell it can reach, the hose is rigged
                // where somebody NEEDED a line - a middling rung against the
                // duct's full cover, and sampled after the fitting rules so the
                // cleats and green plating keep the ring the pocket rules own.
                // The piece affords the thinning: a sag between two clamps is
                // complete on one cell, so a broken run of hose reads as a
                // bundle picked up and dropped again, where a dashed duct would
                // read as a broken machine. Its own length still wants a
                // shoulder with a run in it - a hose needs somewhere to be
                // going.
                placement: FixturePlacement {
                    region: FixtureRegion::Panel,
                    density: FixtureDensity::Regular,
                    orientation: FixtureOrientation::Along,
                },
            },
            StyleFixtureConfig {
                id: "salvage_grille".to_string(),
                model: assets.greeble_salvage_grille.clone(),
                health: 12.0,
                collider: Vec3::new(0.38, 0.08, 0.3),
                // A vent cut wherever there was room to cut one, which is why
                // it takes `Anywhere`: any surface at all can hide a duct
                // mouth, and the region's silence about depth is what reaches
                // the one-cell-thick shapes players build. Eight hundredths of
                // a cell proud, so it is TRIM and a creased plate still vents.
                // The rung's lattice and share keep grates from clustering: two
                // vents side by side read as a designed bank, and nothing on
                // this ship was designed together.
                placement: FixturePlacement {
                    region: FixtureRegion::Anywhere,
                    density: FixtureDensity::Regular,
                    orientation: FixtureOrientation::Free,
                },
            },
            StyleFixtureConfig {
                id: "salvage_cog_patch".to_string(),
                model: assets.greeble_salvage_cog_patch.clone(),
                health: 16.0,
                collider: Vec3::new(0.38, 0.06, 0.38),
                // A cog used as armour, on the same ground the steel patch
                // plates: anywhere at all, thin builds included. Sampled just
                // BEFORE the broad plate so the gear is not starved into the
                // plate's leavings, but on a thinner rung - the gear is the
                // kit's most literal piece of junk, and one per neighbourhood
                // is a story where three make a machine shop.
                placement: FixturePlacement {
                    region: FixtureRegion::Anywhere,
                    density: FixtureDensity::Sparse,
                    orientation: FixtureOrientation::Free,
                },
            },
            StyleFixtureConfig {
                id: "salvage_patch_plate".to_string(),
                model: assets.greeble_salvage_patch_plate.clone(),
                health: 20.0,
                collider: Vec3::new(0.88, 0.04, 0.68),
                // The BODY of the ship, and the broadest rule in the kit, so it
                // sits under every rule that can name a zone and above only the
                // tally and the filler. Four hundredths of a cell proud, which
                // makes a doubler plate TRIM: it lies on a crease the way sheet
                // steel bent over one does, and a hull with no coplanar plate
                // anywhere still gets plated. The tips and crests are still the
                // whip's, because that rule samples first.
                placement: FixturePlacement {
                    region: FixtureRegion::Anywhere,
                    density: FixtureDensity::Regular,
                    orientation: FixtureOrientation::Along,
                },
            },
            StyleFixtureConfig {
                id: "salvage_kills".to_string(),
                model: assets.greeble_salvage_kills.clone(),
                health: 6.0,
                collider: Vec3::new(0.4, 0.03, 0.3),
                // The kill tally, and the kit's second TRIM carrier (the scab
                // is the first): three hundredths of a cell of paint lies on
                // anything, so the seat gate is off, and `Anywhere` asks nothing
                // about depth - the two filters that blank a one-cell-thick
                // build.
                //
                // After the plate, before the scab: the tally is painted ON the
                // hull the patches left showing, so it takes what the plate
                // declined, and the scab's own patch floor still guarantees the
                // true filler lands last. The rung keeps its lattice because two
                // tallies in adjacent cells read as one crew bragging twice -
                // the marks are the ship talking, and a ship says a thing once.
                placement: FixturePlacement {
                    region: FixtureRegion::Anywhere,
                    density: FixtureDensity::Regular,
                    orientation: FixtureOrientation::Free,
                },
            },
            StyleFixtureConfig {
                id: "salvage_patch_scab".to_string(),
                model: assets.greeble_salvage_patch_scab.clone(),
                health: 16.0,
                collider: Vec3::new(0.52, 0.03, 0.44),
                // The FILLER, and the piece that has to carry a hull nothing
                // else fits - a spar, a tip, a hand-built ship with no flat
                // plate on it at all. A small share on a rung with a patch
                // floor, so it reads as a scatter of old repairs on a big hull
                // and still puts something on a small one.
                //
                // Three hundredths of a cell proud, so the seat gate is off:
                // this is the kit's trim carrier, and a one-cell-thick build has
                // no coplanar plate anywhere - seated, the filler that exists
                // for "a hull nothing else fits" was refusing exactly that hull,
                // and the thin half of the bench photographed as bare rock.
                placement: FixturePlacement {
                    region: FixtureRegion::Anywhere,
                    density: FixtureDensity::Sparse,
                    orientation: FixtureOrientation::Free,
                },
            },
        ],
    }
}

/// The scaffolding style: four placeholder greebles on four rules, one per
/// question the plate vocabulary can answer.
///
/// Each rule is here to DEMONSTRATE a reading, and they are in priority order
/// because a plate takes at most one piece:
///
/// 1. the mast reads the HIGH GROUND - the crests, tips and studs of a ship's
///    upper surface, which is the one region that takes a creased plate;
/// 2. the vent reads the PANEL and aligns down it, on the lattice, which is the
///    grid-occupancy claim the research settled on instead of blue noise. Its
///    rung carries a density FLOOR, so a hand-built hull the share would have
///    thinned to nothing still wears a row;
/// 3. the block reads ANYWHERE at the rarest rung - one per block of hull and no
///    more - and is turned OUTBOARD, which is the only reading a region with no
///    relief in it can still express;
/// 4. the blister reads the POCKET distance - beside the mouth of a fitting,
///    which is the "weight decoration toward link points" finding. It goes LAST
///    because "near a fitting" is broad even with the distance counted in face
///    steps: first in the order it carpets 45% of every ship and the other three
///    rules never get a plate.
fn placeholder_style(assets: &BaseContentAssets) -> ShipStyleConfig {
    ShipStyleConfig {
        id: PLACEHOLDER_STYLE_ID.to_string(),
        name: "Placeholder".to_string(),
        // A restatement of the built-in plate colours with the top lifted and
        // warmed a little: enough to prove a style really does dress the
        // derived plates, and not so much that the greebles stop reading
        // against them.
        palette: StylePalette {
            top: SurfaceFinish {
                color: Color::linear_rgb(0.125, 0.135, 0.160),
                roughness: 0.6,
                metallic: 0.2,
            },
            wall: SurfaceFinish {
                color: Color::linear_rgb(0.070, 0.082, 0.110),
                roughness: 0.7,
                metallic: 0.15,
            },
        },
        // In PRIORITY order, most specific first - a plate takes one piece.
        // Tuned against what a real hull actually offers, measured on the wfc
        // row: about four fifths of every ship FALLS AWAY somewhere, a seventh
        // is a STEP and under a seventh is flat, with ridges rare and studs
        // absent. A rule written for flat panels alone lands on almost nothing,
        // which is why the trim below reads the border of any relief and the
        // fairing reads the falling plate rather than the flat.
        fixtures: vec![
            StyleFixtureConfig {
                id: "placeholder_mast".to_string(),
                model: assets.greeble_mast.clone(),
                health: 8.0,
                collider: Vec3::new(0.12, 0.38, 0.12),
                // The HIGH GROUND: the crests, tips and outer corners a hull
                // ends at. One word, and it carries every reading a mast needs -
                // the pointy reliefs, the upper surface, a cell of ship to bolt
                // to, and no seat gate. Spelled out by hand it was three filters
                // that each read as mild and, multiplied, landed on NOTHING: a
                // crest fills an eighth of its cell and a spur tip less, so any
                // height floor asks for something no pointy plate is.
                placement: FixturePlacement {
                    region: FixtureRegion::HighGround,
                    density: FixtureDensity::Regular,
                    orientation: FixtureOrientation::Free,
                },
            },
            StyleFixtureConfig {
                id: "placeholder_vent".to_string(),
                model: assets.greeble_vent.clone(),
                health: 12.0,
                collider: Vec3::new(0.32, 0.04, 0.2),
                // FLAT PANEL. The region takes the corner-cut panel with the
                // flat one - there are more of them on a generated hull than
                // there are flat plates - and the seat gate is what keeps a vent
                // off the ones that actually crease. The rung's patch floor is
                // the floor under its lattice and share: without one this rule
                // is a field of vents on a big hull and nothing at all on a
                // small one.
                placement: FixturePlacement {
                    region: FixtureRegion::Panel,
                    density: FixtureDensity::Dense,
                    orientation: FixtureOrientation::Along,
                },
            },
            StyleFixtureConfig {
                id: "placeholder_block".to_string(),
                model: assets.greeble_block.clone(),
                health: 20.0,
                collider: Vec3::new(0.22, 0.1, 0.22),
                // Turned OUTBOARD rather than down the run: where a hull turns
                // a corner, the fall is the only reading that says which side of
                // it is off the ship. A plate that does not fall one way is left
                // square, which is the rest of the hull.
                //
                // `Rare` is the pure per-block form: no share at all, and one
                // piece per block of hull. A share tuned on the generated row
                // put ONE piece on a 20-plate ship; stated as a density instead,
                // it reads the same at both sizes.
                placement: FixturePlacement {
                    region: FixtureRegion::Anywhere,
                    density: FixtureDensity::Rare,
                    orientation: FixtureOrientation::Outward,
                },
            },
            StyleFixtureConfig {
                id: "placeholder_blister".to_string(),
                model: assets.greeble_blister.clone(),
                health: 16.0,
                collider: Vec3::new(0.18, 0.07, 0.18),
                placement: FixturePlacement {
                    region: FixtureRegion::NearFitting,
                    density: FixtureDensity::Dense,
                    orientation: FixtureOrientation::Free,
                },
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The authored looks lead the catalog and the scaffolding trails them.
    ///
    /// Not cosmetic ordering: `wfc_ships` and the editor's build view both
    /// refuse to name a style id and take the first one instead, so this is
    /// what decides whether a subject is photographed wearing a look or wearing
    /// four magenta primitives.
    #[test]
    fn the_authored_looks_lead_the_catalog() {
        let ids: Vec<String> = style_catalog(&BaseContentAssets::from_paths())
            .into_iter()
            .map(|style| style.id)
            .collect();
        assert_eq!(ids.first().map(String::as_str), Some(INDUSTRIAL_STYLE_ID));
        assert_eq!(ids.last().map(String::as_str), Some(PLACEHOLDER_STYLE_ID));
        for authored in [ARMOURED_STYLE_ID, CIVILIAN_STYLE_ID, SALVAGE_STYLE_ID] {
            assert!(
                ids.iter().any(|id| id == authored),
                "{authored} is not in the catalog",
            );
        }
    }

    /// Every style ships something a ONE-CELL-THICK hull can wear.
    ///
    /// Such a hull is all cones and has no coplanar plate anywhere on it, so a
    /// style made entirely of pieces that need a seat lands NOTHING on the
    /// owner's own builds while looking perfectly well authored - the placement
    /// defect task 20260816-203812 opened, and the one this pins shut.
    ///
    /// Two things reach a cone, and a style needs at least one: the
    /// [`FixtureRegion::HighGround`], which is cones by definition, and TRIM -
    /// a decal thin enough that a crease under it is a fold rather than a gap.
    /// Which pieces those are is no longer authored, so this pins the PROPERTY
    /// rather than a list of ids.
    #[test]
    fn every_style_ships_a_piece_a_one_cell_hull_can_wear() {
        for style in style_catalog(&BaseContentAssets::from_paths()) {
            let carriers: Vec<&str> = style
                .fixtures
                .iter()
                .filter(|fixture| {
                    fixture.placement.region == FixtureRegion::HighGround
                        || !FixturePlacement::needs_seat(fixture.collider)
                })
                .map(|fixture| fixture.id.as_str())
                .collect();
            assert!(
                !carriers.is_empty(),
                "{} has nothing a one-cell-thick hull can wear, so the owner's \
                 own builds come out bare",
                style.id,
            );
        }
    }

    /// The pieces that stand PROUD are the ones a seat gate can refuse, and a
    /// style built only of those is the defect above. Stated as a ratio so a
    /// kit cannot drift into it one piece at a time.
    #[test]
    fn no_style_is_made_entirely_of_pieces_that_stand_proud() {
        for style in style_catalog(&BaseContentAssets::from_paths()) {
            let trim = style
                .fixtures
                .iter()
                .filter(|fixture| !FixturePlacement::needs_seat(fixture.collider))
                .count();
            assert!(
                trim * 4 >= style.fixtures.len(),
                "{} is {trim} trim of {} pieces; a kit that thin on decals \
                 dresses a hand-built hull with almost nothing",
                style.id,
                style.fixtures.len(),
            );
        }
    }

    /// The kit sizes are an ORDERING, and the ordering IS the tone split
    /// (task 20260816-194637): armoured is the smallest because suppression
    /// is its look, salvage the biggest because accumulation is. The four
    /// vocabulary batches (tasks 20260816-2226xx) each pinned their own cap;
    /// this pin holds the RELATION so a future piece cannot quietly invert
    /// the doctrine. Equality at the top is tolerated until the tuning pass
    /// (GREEBLES.md follow-up 6) re-ranks the styles; strict order below it
    /// stands.
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
    /// fixtures sharing one id inside a style would share a scatter salt and
    /// claim the same plates.
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
            assert!(!style.fixtures.is_empty(), "{} scatters nothing", style.id);
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

    /// Every piece of an authored kit is namespaced by its style, so four
    /// candidate looks can ship recipes, meshes and fixture ids into the same
    /// three directories without colliding.
    #[test]
    fn the_industrial_kit_namespaces_everything_it_ships() {
        let style = industrial_style(&BaseContentAssets::from_paths());
        assert_eq!(
            style.fixtures.len(),
            14,
            "the industrial cap was raised 7 -> 14 by the builders batch \
             (task 20260816-222639) DELIBERATELY: seven new classes with one \
             piece each, not variants, so the doctrine - mismatch comes from \
             placement and material, never from count - still holds"
        );
        for fixture in &style.fixtures {
            let prefix = format!("{INDUSTRIAL_STYLE_ID}_");
            assert!(
                fixture.id.starts_with(&prefix),
                "'{}' is not namespaced by its style",
                fixture.id,
            );
            let path = fixture.model.path().unwrap_or_default().to_string();
            assert!(
                path == format!("self://gltf/greebles/{}.glb#Scene0", fixture.id),
                "'{}' names '{path}' rather than its own greeble",
                fixture.id,
            );
        }
    }

    /// The hazard band is a LINE, and the two ways to break a line are both
    /// mistakes the rest of this file warns about: thinning it with a share
    /// turns it into dashes, and dropping `min_run` lets every one-cell corner
    /// of a broken hull count as an edge worth painting.
    #[test]
    fn the_hazard_band_paints_runs_rather_than_specks() {
        let style = industrial_style(&BaseContentAssets::from_paths());
        let band = style
            .fixtures
            .iter()
            .find(|fixture| fixture.id == "industrial_hazard_band")
            .expect("the kit ships a hazard band");
        assert_eq!(
            band.placement.density,
            FixtureDensity::Every,
            "every other rung thins a line into dashes",
        );
        assert_eq!(
            band.placement.region,
            FixtureRegion::Edge,
            "the band paints the straight edge of a hull and nothing else",
        );
        assert_eq!(
            band.placement.orientation,
            FixtureOrientation::Along,
            "a stripe that does not lie down its own edge is a smear",
        );
        // The run gate is READ OFF the band's own length, so a one-cell corner
        // is not an edge worth painting without anything being authored to say
        // so - and a band shortened by an artist relaxes its own gate with it.
        assert!(
            FixturePlacement::min_run(band.collider) >= 2,
            "a one-cell corner is not an edge",
        );
    }

    /// `NearFitting` reads as narrow and is not, so the one rule that uses it
    /// must never be able to claim a hull before the rest of the kit has had a
    /// look. Pinned rather than commented, because the order is the only thing
    /// holding it.
    #[test]
    fn the_broad_rules_of_the_industrial_kit_come_last() {
        let style = industrial_style(&BaseContentAssets::from_paths());
        let pocket = style
            .fixtures
            .iter()
            .position(|fixture| fixture.placement.region == FixtureRegion::NearFitting)
            .expect("the kit dresses the fittings");
        let shaped = style
            .fixtures
            .iter()
            .position(|fixture| {
                matches!(
                    fixture.placement.region,
                    FixtureRegion::Panel
                        | FixtureRegion::Deck
                        | FixtureRegion::Flank
                        | FixtureRegion::Edge
                        | FixtureRegion::HighGround
                )
            })
            .expect("the kit names a part of the hull");
        assert!(
            shaped < pocket,
            "a `NearFitting` piece above a shaped one carpets the ship",
        );
    }

    /// Every piece of a kit is prefixed with the style that owns it - the ids
    /// salt the scatter and name the `.glb`, and four looks are authored into
    /// one pair of folders.
    #[test]
    fn the_armoured_kit_is_namespaced() {
        let style = armoured_style(&BaseContentAssets::from_paths());
        // Raised 4 -> 10 by the vocabulary batch (task 20260816-222644):
        // mast, intake, magazine, chaff, applique and ammo stripes joined the
        // kit. Pinned exactly, because a kit that grows quietly is the defect
        // that once put forty meshes in this repo - and armoured staying the
        // SMALLEST of the four is the art direction (restraint is the
        // identity); the cross-style ordering is the coordinator's pin, not
        // this one.
        assert_eq!(
            style.fixtures.len(),
            10,
            "the armoured kit is pinned at 10 pieces",
        );
        for fixture in &style.fixtures {
            assert!(
                fixture.id.starts_with("armoured_"),
                "'{}' is not namespaced to its style",
                fixture.id,
            );
            let path = fixture.model.path().unwrap_or_default().to_string();
            assert!(
                path.starts_with("self://gltf/greebles/armoured_"),
                "'{}' names '{path}', which is another kit's art",
                fixture.id,
            );
        }
    }

    /// The belt is the whole look, so the two things that make it one
    /// continuous line rather than a row of blocks are pinned: the run it lies
    /// on, and the alignment that turns it down that run.
    #[test]
    fn the_armoured_belt_runs_down_the_straight_edge_of_a_hull() {
        let style = armoured_style(&BaseContentAssets::from_paths());
        let belt = style
            .fixtures
            .iter()
            .find(|fixture| fixture.id == "armoured_strake")
            .expect("the armoured kit has lost its belt");
        assert_eq!(belt.placement.region, FixtureRegion::Edge);
        assert_eq!(belt.placement.orientation, FixtureOrientation::Along);
        assert!(
            FixturePlacement::min_run(belt.collider) >= 2,
            "a one-cell run is not a run",
        );
        assert_eq!(
            belt.placement.density,
            FixtureDensity::Every,
            "a belt with holes thinned into it is not a belt",
        );
    }

    /// The civilian kit is namespaced like the others, and its size is pinned
    /// DELIBERATELY at twelve: the vocabulary batch (task 20260816-222651)
    /// raised the cap from five for seven new CLASSES - vent, door, tank,
    /// registry, livery, skylight, dish - one piece each, no variants. The
    /// doctrine stands: mismatch comes from placement and material, never
    /// from count, and a thirteenth piece needs a class this kit lacks. The
    /// cross-style ordering (armoured smallest, salvage biggest) is re-pinned
    /// by the coordinator after all four batches land, not here.
    #[test]
    fn the_civilian_kit_is_namespaced_and_capped() {
        let style = civilian_style(&BaseContentAssets::from_paths());
        assert_eq!(
            style.fixtures.len(),
            12,
            "the civilian kit is pinned at 12 pieces - one per approved class",
        );
        for fixture in &style.fixtures {
            let prefix = format!("{CIVILIAN_STYLE_ID}_");
            assert!(
                fixture.id.starts_with(&prefix),
                "'{}' is not namespaced by its style",
                fixture.id,
            );
            let path = fixture.model.path().unwrap_or_default().to_string();
            assert!(
                path == format!("self://gltf/greebles/{}.glb#Scene0", fixture.id),
                "'{}' names '{path}' rather than its own greeble",
                fixture.id,
            );
        }
    }

    /// Every piece below the stripe that lists `Brink` must STAY below it:
    /// the band is continuous only because nothing above it can claim a cell
    /// of a straight edge, and the livery panel is the one piece allowed on
    /// edges at all.
    #[test]
    fn nothing_above_the_civilian_stripe_touches_a_brink() {
        let style = civilian_style(&BaseContentAssets::from_paths());
        let stripe = style
            .fixtures
            .iter()
            .position(|fixture| fixture.id == "civilian_stripe")
            .expect("the civilian kit has no band");
        for fixture in &style.fixtures[..stripe] {
            // `Edge` is the one region that names the straight edge of a hull,
            // so keeping it off the pieces above the band is the whole of it -
            // and `Anywhere` claims any plate at all, edges included.
            assert!(
                !matches!(
                    fixture.placement.region,
                    FixtureRegion::Edge | FixtureRegion::Anywhere
                ),
                "'{}' sits above the stripe and can claim an edge, which \
                 punctures the band",
                fixture.id,
            );
        }
    }

    /// The ammo stripes are the armoured pocket rule, and every clause here is
    /// the HARD CONSTRAINT the blocks bench proved (task 20260816-203837
    /// closure): the plates beside a boom-mounted gun are CONES, so a seat
    /// gate, a depth floor, a height floor or a stride each silently zero the
    /// rule on exactly the gun it exists to mark - `asym_gunship` measured 0
    /// for every seat-gated pocket rule, and the carrier deck's tucked
    /// drive zeroed the strided ones via lattice parity.
    #[test]
    fn the_armoured_ammo_stripes_read_the_gun_pocket_off_any_seat() {
        let style = armoured_style(&BaseContentAssets::from_paths());
        let pocket: Vec<usize> = style
            .fixtures
            .iter()
            .enumerate()
            .filter(|(_, fixture)| fixture.placement.region == FixtureRegion::NearFitting)
            .map(|(index, _)| index)
            .collect();
        // `NearFitting` is the measured carpet, so it picks only after every
        // rule that can NAME the part of the hull it wants. Below it there is
        // nothing left but `Anywhere`, which is broader still.
        assert_eq!(pocket.len(), 1, "the kit dresses the pocket exactly once");
        for (index, fixture) in style.fixtures.iter().enumerate() {
            assert!(
                (index > pocket[0]) == (fixture.placement.region == FixtureRegion::Anywhere),
                "'{}' sits on the wrong side of the pocket rule",
                fixture.id,
            );
        }
        let stripes = &style.fixtures[pocket[0]];
        assert_eq!(stripes.id, "armoured_ammo_stripes");
        assert!(
            !FixturePlacement::needs_seat(stripes.collider),
            "a tally that stands proud never fires beside a boom-mounted gun, \
             where every plate is a cone",
        );
        assert_eq!(
            stripes.placement.density,
            FixtureDensity::Every,
            "`Every` is the one rung with no lattice and no share: parity can \
             zero a strided pocket rule, and a share lets a well hash its way \
             out of its own tally",
        );
    }

    /// The livery band is the civilian look, and it is a LINE: the model fills
    /// its whole cell so neighbours in a run butt together, and any rule that
    /// skips a cell turns it back into a dashed row.
    ///
    /// Every clause below is one way the band was broken while it was authored,
    /// so this is a regression test and not a restatement of the code.
    #[test]
    fn the_civilian_band_is_continuous() {
        let style = civilian_style(&BaseContentAssets::from_paths());
        let stripe = style
            .fixtures
            .iter()
            .find(|fixture| fixture.id == "civilian_stripe")
            .expect("the civilian kit has no band");

        assert_eq!(
            stripe.placement.density,
            FixtureDensity::Every,
            "a strided or shared band is a dashed line",
        );
        assert_eq!(
            stripe.placement.orientation,
            FixtureOrientation::Along,
            "a band not laid down its run cannot join its neighbours",
        );
        assert_eq!(
            stripe.placement.region,
            FixtureRegion::Edge,
            "the band follows the straight edge of a hull and nothing else",
        );
        assert!(
            FixturePlacement::min_run(stripe.collider) >= 2,
            "a band that will stand on a one-cell stub is a speck",
        );
    }

    /// `NearFitting` is the measured carpeting trap: it reads as narrow and
    /// admits most of a hull. The civilian kit uses it exactly once, and that
    /// rule must sit below every rule that can NAME its part of the hull, or
    /// it starves them. Only `Anywhere`, which is broader still, goes under it.
    #[test]
    fn the_civilian_pocket_rule_picks_after_every_named_zone() {
        let style = civilian_style(&BaseContentAssets::from_paths());
        let pocket: Vec<usize> = style
            .fixtures
            .iter()
            .enumerate()
            .filter(|(_, fixture)| fixture.placement.region == FixtureRegion::NearFitting)
            .map(|(index, _)| index)
            .collect();
        assert_eq!(pocket.len(), 1, "the kit dresses the pocket exactly once");
        for (index, fixture) in style.fixtures.iter().enumerate() {
            assert!(
                (index > pocket[0]) == (fixture.placement.region == FixtureRegion::Anywhere),
                "'{}' sits on the wrong side of the pocket rule",
                fixture.id,
            );
        }
    }

    /// The candidate looks are merged into one catalog by hand, so every id a
    /// kit introduces carries its own prefix - a fixture id is the SALT of its
    /// own scatter, and two kits sharing one id would silently decorate the
    /// same plates.
    ///
    /// The count is pinned for a different reason: forty generated meshes were
    /// deleted from this repo once, because a kit that grows is a kit that has
    /// stopped being a small fixed set of committed assets. Mismatch comes from
    /// placement and material, not from piece count.
    #[test]
    fn the_salvage_kit_is_prefixed_and_stays_small() {
        let style = salvage_style(&BaseContentAssets::from_paths());
        // Raised 7 -> 14 by the approved vocabulary batch (task
        // 20260816-222658): seven CLASSES with one piece each, not variants.
        // The doctrine survives the raise - mismatch still comes from
        // placement and material - and the pin still exists for the same
        // reason: a kit that grows past its batch has stopped being a small
        // fixed set of committed assets.
        assert_eq!(
            style.fixtures.len(),
            14,
            "the salvage kit is pinned at 14 pieces (7 original + batch \
             20260816-222658); grow it through an approved batch or not at all",
        );
        for fixture in &style.fixtures {
            assert!(
                fixture.id.starts_with("salvage_"),
                "'{}' is not prefixed, so it can collide with another kit",
                fixture.id,
            );
            let path = fixture.model.path().unwrap_or_default().to_string();
            assert!(
                path.starts_with("self://gltf/greebles/salvage_"),
                "'{}' names '{path}', which is not a salvage greeble",
                fixture.id,
            );
        }
    }

    /// No salvage rule matches every plate.
    ///
    /// A `ScatterRule` with nothing set takes the whole hull, and first in a
    /// priority list it starves every rule under it. That is the documented
    /// trap, and a look built on SEVEN rules competing for one hull is exactly
    /// where it would do the most damage - so it is pinned rather than
    /// remembered.
    #[test]
    fn every_salvage_rule_narrows_the_hull_somehow() {
        for fixture in salvage_style(&BaseContentAssets::from_paths()).fixtures {
            let placement = fixture.placement;
            // `Anywhere` names no part of the hull and `Every` thins nothing,
            // so the two together are the carpet: first in a priority list it
            // starves every piece under it, and from a screenshot that is
            // indistinguishable from a piece whose region was simply too tight.
            assert!(
                placement.region != FixtureRegion::Anywhere
                    || placement.density != FixtureDensity::Every,
                "'{}' names no region and thins nothing, so it claims every \
                 plate under it",
                fixture.id,
            );
            // A LINE is the one thing allowed to take every plate it is
            // eligible for: a share would only dot the piece it is joining up.
            assert!(
                placement.density != FixtureDensity::Every
                    || placement.region == FixtureRegion::Edge,
                "'{}' takes every plate it is eligible for without being a \
                 seam, which is a carpet",
                fixture.id,
            );
        }
    }

    /// The two long patches CROSS: both align to the run, and their models are
    /// authored long on opposite in-plane axes, so where their territories meet
    /// two neighbouring repairs sit at right angles.
    ///
    /// This is the whole trick the look rests on, and it is invisible in the
    /// rules alone - the crossing lives half in the RECIPES. What can be pinned
    /// here is the half that lives in the style: if either of these stops
    /// aligning to the run, both lie whichever way the plate frame happens to
    /// point and the mismatch becomes noise.
    #[test]
    fn the_salvage_patches_align_to_the_run_so_they_can_cross() {
        let style = salvage_style(&BaseContentAssets::from_paths());
        let aligned = |id: &str| {
            style
                .fixtures
                .iter()
                .find(|fixture| fixture.id == id)
                .unwrap_or_else(|| panic!("the salvage kit lost '{id}'"))
                .placement
                .orientation
        };
        assert_eq!(aligned("salvage_patch_plate"), FixtureOrientation::Along);
        assert_eq!(aligned("salvage_patch_strip"), FixtureOrientation::Along);
        assert_eq!(
            aligned("salvage_patch_scab"),
            FixtureOrientation::Free,
            "the filler has no long axis to turn, and turning it would line the \
             fillers up with each other",
        );
    }

    /// The placeholder style covers the whole authored vocabulary, which is the
    /// only reason it exists: the checkpoint render has to be able to show
    /// whether the placement words can carry a look at all.
    ///
    /// Pinned on the words rather than on what they expand to. The expansion is
    /// engine policy and `nova_ship` tests it; what a STYLE can get wrong is
    /// authoring four pieces that all say the same thing.
    #[test]
    fn the_placeholder_style_exercises_the_whole_vocabulary() {
        let style = placeholder_style(&BaseContentAssets::from_paths());
        let places: Vec<FixturePlacement> = style
            .fixtures
            .iter()
            .map(|fixture| fixture.placement)
            .collect();

        let regions: Vec<FixtureRegion> = places.iter().map(|place| place.region).collect();
        assert!(
            regions.contains(&FixtureRegion::HighGround),
            "nothing stands on the pointiest thing on a hull",
        );
        assert!(
            regions.contains(&FixtureRegion::NearFitting),
            "nothing reads the pocket distance",
        );
        assert!(
            regions.contains(&FixtureRegion::Panel),
            "nothing stands on flat panel",
        );
        assert!(
            regions.contains(&FixtureRegion::Anywhere),
            "nothing fills in what the rest left",
        );

        for orientation in [
            FixtureOrientation::Free,
            FixtureOrientation::Along,
            FixtureOrientation::Outward,
        ] {
            assert!(
                places.iter().any(|place| place.orientation == orientation),
                "nothing is turned {orientation:?}, so the checkpoint cannot \
                 show whether that turn works",
            );
        }

        // Three rungs at least, and one of them the thinnest: a rule whose
        // share alone would land almost nothing, and which therefore stands or
        // falls on its patch floor, is the case a screenshot cannot tell from
        // a well-tuned one, so the checkpoint has to contain it.
        let rungs: Vec<FixtureDensity> = places.iter().map(|place| place.density).collect();
        assert!(
            rungs.contains(&FixtureDensity::Rare),
            "nothing leans on the patch floor",
        );
        let mut spread = rungs.clone();
        spread.sort_by_key(|rung| format!("{rung:?}"));
        spread.dedup();
        assert!(
            spread.len() >= 3,
            "the placeholder style uses {} rungs of the ladder; it is the \
             subject that has to show the ladder working",
            spread.len(),
        );
    }
}
