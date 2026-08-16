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
//! # The SEAT, and which rules opt out of it
//!
//! A rule that says nothing about its seat stands its piece on a plate whose
//! top is ONE SURFACE ([`ScatterSeat::Whole`], the default). That is what a
//! relief list used to try to say and got wrong: measured over 526 generated
//! plates, `Flat` and `Brink` are surfaces every time, `Bevel`, `Ridge`, `Peak`
//! and `Spur` are cones every time, and `Step` splits about in half. So the
//! lists here name a ZONE of the hull and nothing more, and no seated rule
//! lists a relief that can never carry a seat.
//!
//! Seven rules opt out with [`ScatterSeat::Any`]. Five are pieces that WANT
//! the high ground - the stack, the whip, the fin, the mast and the corner
//! boss. The pointiest plate on a hull is a cone every time, so a blanket
//! "surfaces only" would take a ship's silhouette off to fix a bedding
//! defect. A piece on a cone stands up its own cell rather than lying down
//! one of its facets, which is what a mast on a spar tip wants anyway.
//!
//! The other two are THIN-SHAPE CARRIERS (task 20260816-203812): the civilian
//! fairing and the salvage scab. A hand-built hull one cell thick derives as
//! cones from end to end - zero coplanar plates on half the bench roster - so
//! a style whose every filler is seated places NOTHING on the shapes players
//! actually build. One cone-friendly filler per style is the floor under the
//! look; the armoured cap and the industrial stack already were one.

use bevy::prelude::{Color, Vec3};
use nova_ship::prelude::{
    PlateFacing, PlateRelief, ScatterAlign, ScatterRule, ScatterSeat, ShellSurface,
    ShipStyleConfig, StyleFixtureConfig, StyleSurfaceConfig,
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
/// here. The other three are cast parts - the warship, the racer, the raider -
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
/// 3. THE KIT. Radiators, conduit, corrugation, louvres, hatches and stacks.
///    Seven pieces, no ornament among them - every one is something that would
///    have a part number.
///
/// The rules below are a ZONE ALLOCATION rather than a set of overlapping
/// filters: each piece owns a part of the hull that the vocabulary can name, so
/// priority mostly resolves ties rather than starving anything. Reading down:
/// the high ground, the straight edges, the flat panel, the long runs, the
/// fittings, and then two fillers on one lattice for everything left.
fn industrial_style(assets: &BaseContentAssets) -> ShipStyleConfig {
    ShipStyleConfig {
        id: INDUSTRIAL_STYLE_ID.to_string(),
        name: "Industrial".to_string(),
        surfaces: vec![
            // Mid-grey primer, warm and matte. Deliberately much lighter than
            // the built-in slate: a hull that reads as PAINTED STEEL is what
            // makes a dark greeble a silhouette against it, and a working ship
            // is not a stealth ship.
            StyleSurfaceConfig {
                surface: ShellSurface::Top,
                color: Color::linear_rgb(0.200, 0.190, 0.168),
                roughness: 0.9,
                metallic: 0.15,
            },
            // The gap between panels, and the one move here that does the most
            // work per byte. See point 2 above.
            StyleSurfaceConfig {
                surface: ShellSurface::Wall,
                color: Color::linear_rgb(0.030, 0.028, 0.024),
                roughness: 0.95,
                metallic: 0.1,
            },
        ],
        fixtures: vec![
            StyleFixtureConfig {
                id: "industrial_stack".to_string(),
                model: assets.greeble_industrial_stack.clone(),
                health: 10.0,
                density: 0.08,
                collider: Vec3::new(0.18, 0.28, 0.18),
                // THE HIGH GROUND, and the only rule allowed to break the
                // silhouette. `seat: Any` because a crest and a spar tip are
                // CONES, so the seat gate would have refused every plate this
                // rule is for. A stack standing up the tip of a spar is the
                // picture; a stack banished to the flat deck is not this rule
                // any more.
                //
                // This is also the kit's THIN-SHAPE CARRIER (task
                // 20260816-203812): a hand-built hull one cell thick is all
                // cones, so the one seat-free rule has to be the one that
                // reaches it. `min_depth` was 2 and `min_height` was 1, and
                // together they zeroed the whole thin half of the bench - a
                // spar has one cell of ship under its tip, and a spur fills so
                // little of its cell it MEASURES height 0. The patch floor is
                // what makes the reach a guarantee rather than a coin toss:
                // one stack per block of high ground, whatever the share says.
                scatter: ScatterRule {
                    relief: vec![
                        PlateRelief::Ridge,
                        PlateRelief::Peak,
                        PlateRelief::Spur,
                        PlateRelief::Step,
                    ],
                    seat: ScatterSeat::Any,
                    facing: PlateFacing::Up,
                    min_depth: 1,
                    chance: 0.35,
                    patch: 4,
                    ..Default::default()
                },
            },
            StyleFixtureConfig {
                id: "industrial_hazard_band".to_string(),
                model: assets.greeble_industrial_hazard_band.clone(),
                health: 6.0,
                density: 0.05,
                collider: Vec3::new(0.19, 0.03, 0.9),
                // THE SIGNATURE. `Brink` is the one relief with a straight edge
                // and an outward direction - the flat side of a hull falling
                // away along one whole side - and it is 45-55% of the falling
                // plate on a generated ship. Painting it draws the ship's own
                // silhouette in yellow.
                //
                // `chance: 1.0` on purpose, which is the opposite of every other
                // rule here. A stripe is a LINE: thinned to a share it becomes
                // dashes, and dashes are the confetti the whole alignment
                // machinery exists to avoid. `min_run: 3` is what keeps it
                // honest - an edge has to be three cells of edge before it is
                // worth painting, and without it the trap in `modding/styles.md`
                // applies and a one-cell corner counts as a run.
                scatter: ScatterRule {
                    relief: vec![PlateRelief::Brink],
                    min_run: 3,
                    min_height: 1,
                    align: ScatterAlign::Run,
                    ..Default::default()
                },
            },
            StyleFixtureConfig {
                id: "industrial_radiator".to_string(),
                model: assets.greeble_industrial_radiator.clone(),
                health: 14.0,
                density: 0.1,
                collider: Vec3::new(0.38, 0.27, 0.28),
                // FLAT PANEL, which is where the one piece with real height and
                // a long axis fits.
                //
                // The rule used to ask for `min_run: 2` on a stride of 2 at
                // full share - a description of GENERATED flats, which come in
                // fields. Measured on the hand-built bench (task
                // 20260816-203812), `Flat` is 0-8 plates a subject and mostly
                // reads run 1, so the old filter reached 2 plates on one
                // subject and nothing anywhere else. Now every flat is in
                // reach and the SHARE does the thinning the lattice used to:
                // on a generated hull's 6-22 flats that is the same 3-11
                // radiators, and on a hand-built deck it is finally not zero.
                //
                // `Bevel` came off the list when the seat gate went in: a panel
                // with a corner taken off is a CONE, so it never carried this
                // piece flat in the first place and the gate refuses it now.
                scatter: ScatterRule {
                    relief: vec![PlateRelief::Flat],
                    chance: 0.5,
                    patch: 3,
                    align: ScatterAlign::Run,
                    ..Default::default()
                },
            },
            StyleFixtureConfig {
                id: "industrial_duct".to_string(),
                model: assets.greeble_industrial_duct.clone(),
                health: 9.0,
                density: 0.07,
                collider: Vec3::new(0.14, 0.16, 0.9),
                // THE SHOULDER: a `Step` is where one deck stands proud of the
                // next, and that is where a services run belongs. The seat gate
                // sorts the two kinds of step out for it - square-on to a raise
                // a step is a clean ramp and takes the pipe, on the diagonal it
                // is a cone and does not.
                //
                // No `Brink` in the list, though a pipe would lie on one
                // perfectly well. Measured: with `Brink` in it the rule reached
                // 11-14 plates and TOOK 1-4, because the band above had already
                // painted every brink of three cells or more. A reach that is
                // mostly other rules' plates is a number that cannot be tuned
                // against, so the list says what the rule can actually have.
                //
                // Stride 1, unlike everything else here. A conduit is a LINE
                // like the band is, and the piece spans nine tenths of its cell
                // precisely so that consecutive plates of one run join up.
                scatter: ScatterRule {
                    relief: vec![PlateRelief::Step, PlateRelief::Flat],
                    min_run: 3,
                    patch: 4,
                    align: ScatterAlign::Run,
                    ..Default::default()
                },
            },
            StyleFixtureConfig {
                id: "industrial_louvre".to_string(),
                model: assets.greeble_industrial_louvre.clone(),
                health: 12.0,
                density: 0.12,
                collider: Vec3::new(0.32, 0.07, 0.24),
                // THE FITTINGS. `near_fitting: Some(1)` is the documented trap -
                // it reads as narrow and admits a third of a hull - so it is
                // FIFTH here rather than first, on a stride, with a share. What
                // it is worth is the read: grilles clustered where the drives
                // and the gun wells are says the machinery is under that panel.
                scatter: ScatterRule {
                    near_fitting: Some(1),
                    stride: 2,
                    chance: 0.55,
                    align: ScatterAlign::Run,
                    ..Default::default()
                },
            },
            StyleFixtureConfig {
                id: "industrial_hatch".to_string(),
                model: assets.greeble_industrial_hatch.clone(),
                health: 18.0,
                density: 0.18,
                collider: Vec3::new(0.32, 0.07, 0.32),
                // Free-turning: a round hatch has no long axis, so there is
                // nothing for an alignment to point down.
                //
                // ABOVE the corrugation, and it has to be. Below it the two
                // shared one lattice, the corrugation's reach covered nearly all
                // of it, and the hatch measured `x0 of 12` - a third of the
                // kit's yellow gone, on a hull where nothing else was competing
                // for those plates. A sparse accent has to be sampled BEFORE the
                // filler, never out of the filler's leavings.
                scatter: ScatterRule {
                    min_depth: 2,
                    min_height: 1,
                    stride: 2,
                    chance: 0.3,
                    patch: 5,
                    ..Default::default()
                },
            },
            StyleFixtureConfig {
                id: "industrial_ribbing".to_string(),
                model: assets.greeble_industrial_ribbing.clone(),
                health: 15.0,
                density: 0.15,
                collider: Vec3::new(0.46, 0.07, 0.42),
                // FILLER, and the piece that decides whether the hull reads as
                // busy. LAST, because it takes any relief and would otherwise
                // eat the plates every rule above it wants. `min_depth: 2` keeps
                // it off the skin over a one-cell spar, where a panel this broad
                // would be wider than the ship under it.
                //
                // NO LATTICE, which is the one rule here that wants none.
                // Everything above claims on a stride of 2, so a filler on the
                // same stride is fighting five rules for one quarter of the hull
                // and measured `x3 of 27` on the smallest ship - a hull that
                // photographed as bare grey between the painted lines. Off the
                // lattice it competes for every plate instead of a quarter of
                // them, and a panel this size does not need a grid to look
                // deliberate: it very nearly fills its own plate, so a field of
                // them reads as plating rather than as specks.
                scatter: ScatterRule {
                    min_depth: 2,
                    chance: 0.45,
                    patch: 3,
                    align: ScatterAlign::Run,
                    ..Default::default()
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
/// The kit, in PRIORITY order, and what each reads:
///
/// 1. the sensor blister takes the FLAT panels, on a density floor rather than
///    a share, so a hull wears a handful of them wherever it is broad enough to
///    carry one. The rarest piece and the only one that stands proud;
/// 2. the corner boss reads the SPUR - the tips and outer corners - and only on
///    the upper surface, which is the armour a ship is hit on;
/// 3. the belt reads the BRINK, the straight edge of a hull and its largest
///    single bucket, and is turned down the RUN. It is the whole look;
/// 4. the hatch is the filler, on whatever flat and stepped plate the sensor
///    left, giving a bare panel a scale without giving a gunner a target.
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
        surfaces: vec![
            StyleSurfaceConfig {
                surface: ShellSurface::Top,
                color: Color::linear_rgb(0.045, 0.048, 0.055),
                roughness: 0.88,
                metallic: 0.0,
            },
            StyleSurfaceConfig {
                surface: ShellSurface::Wall,
                color: Color::linear_rgb(0.007, 0.0075, 0.009),
                roughness: 0.95,
                metallic: 0.0,
            },
        ],
        fixtures: vec![
            StyleFixtureConfig {
                id: "armoured_sensor".to_string(),
                model: assets.greeble_armoured_sensor.clone(),
                health: 10.0,
                density: 0.1,
                collider: Vec3::new(0.4, 0.12, 0.4),
                // The PANELS, and the pure per-patch form on top: no share at
                // all, one blister per block of ship that can carry one. A
                // share here would have put four on one flank and none on the
                // other, and the piece is rare enough that where it lands is
                // visible. (`Bevel` was in this list and is out: a corner-cut
                // panel is a cone, so a hood never lay flat on one.)
                //
                // `Step` is in (task 20260816-203812): the seat gate already
                // keeps only the clean ramps of it, and a hood on the panel
                // beside a raise is still a hood on a panel. It has to be,
                // measured: the old `Flat + min_run 2 + min_depth 2` filter
                // described a generated hull's broad fields and reached ZERO
                // plates on the entire hand-built bench - flats there are
                // run-1 singletons, and half the subjects are one cell thick.
                // The seat plus the patch floor now do all the gating: the
                // piece stays on coplanar panel and stays rare.
                //
                // The block is 6 cells and not 4, measured: at 4 a hull wore
                // 7-12 blisters, which is a rash of the one piece that is
                // supposed to be rare.
                scatter: ScatterRule {
                    relief: vec![PlateRelief::Flat, PlateRelief::Step],
                    chance: 0.0,
                    patch: 6,
                    ..Default::default()
                },
            },
            StyleFixtureConfig {
                id: "armoured_cap".to_string(),
                model: assets.greeble_armoured_cap.clone(),
                health: 26.0,
                density: 0.25,
                collider: Vec3::new(0.3, 0.1, 0.3),
                // The SPUR is a third of a hull, and the LATTICE is the whole of
                // the thinning: every other outer corner takes a boss, all the
                // way round. Reinforcing only the corners on top was the first
                // cut and it is a worse sentence - a corner is a corner from
                // whatever side it is shot at - and it left the flanks bare.
                //
                // Everything else this rule used to ask for came off after the
                // reach was logged. `min_depth: 2` and `min_height: 1` together
                // with the facing took it to `x0 of 0` and `x2 of 2`: a spur is
                // the TIP of something, so there is rarely two cells of ship
                // under it and it fills almost none of its own cell. Three
                // filters that each read as mild, multiplied, made an
                // impossible rule out of a bucket 42 plates deep.
                //
                // `seat: Any` is the fourth filter that would have done it
                // again: a spur is a CONE by definition - it falls two ways or
                // more - so a seated rule for spurs is a rule for nothing. A
                // boss on a corner is the exception the gate is written to
                // allow, and it stands up its own cell rather than lying down
                // one facet of it.
                //
                // `Ridge` and `Peak` joined `Spur` (task 20260816-203812):
                // the crest of a one-cell-wide run and the top of a lone stud
                // are the same pointy armour the boss is for, and they are
                // what a hand-built L is MADE of. Measured before the widening
                // the owner's own build wore zero armoured pieces - its twelve
                // spurs all sat off this rule's lattice, a parity accident of
                // where the hull happens to stand, and no other armoured rule
                // can touch a creased plate at all.
                scatter: ScatterRule {
                    relief: vec![PlateRelief::Spur, PlateRelief::Ridge, PlateRelief::Peak],
                    seat: ScatterSeat::Any,
                    stride: 2,
                    ..Default::default()
                },
            },
            StyleFixtureConfig {
                id: "armoured_strake".to_string(),
                model: assets.greeble_armoured_strake.clone(),
                health: 30.0,
                density: 0.25,
                collider: Vec3::new(0.3, 0.085, 0.95),
                // THE LOOK. Every `Brink` in a run of two or more, at full
                // share, turned down the run - and the model is 0.98 cells long
                // against a plate of 1.0, so neighbours meet and a run comes out
                // as ONE unbroken strake rather than as a dashed line. That is
                // the "decoration cannot span cells" ceiling paid off with the
                // only currency available: a piece that fills its own cell.
                //
                // No share and no stride, which for any other rule would be a
                // carpet. Here it is the point: an armour belt with gaps in it
                // is not an armour belt, and the pieces are all one line rather
                // than 40 objects.
                scatter: ScatterRule {
                    relief: vec![PlateRelief::Brink],
                    min_run: 2,
                    align: ScatterAlign::Run,
                    ..Default::default()
                },
            },
            StyleFixtureConfig {
                id: "armoured_hatch".to_string(),
                model: assets.greeble_armoured_hatch.clone(),
                health: 14.0,
                density: 0.15,
                collider: Vec3::new(0.3, 0.05, 0.3),
                // LAST, and the only rule allowed to be broad, because a flush
                // hatch is the one piece that cannot spoil a silhouette - it is
                // four hundredths of a cell tall. `Step` is in the list where it
                // is out of the sensor's: a step is a hard edge rather than a
                // panel, which is wrong for a hood and right for a hatch.
                scatter: ScatterRule {
                    relief: vec![PlateRelief::Flat, PlateRelief::Step],
                    min_height: 1,
                    stride: 2,
                    chance: 0.6,
                    ..Default::default()
                },
            },
        ],
    }
}

/// The CIVILIAN look: the racer's, and a ship built to be SOLD.
///
/// Two things carry it, and only two. The PAINT: the undressed derivation is a
/// cool dark slate, near enough to bare metal that a "clean" look built on
/// subtle greebling would be indistinguishable from no style at all, so this
/// one repaints the hull pale, satin and non-metallic - an airframe, not plate.
/// And the LIVERY: one continuous cobalt band down every straight edge of the
/// hull, which is the only piece a customer would ever have asked for.
///
/// Everything else is restraint. The kit carries exactly two accents - COBALT
/// for anything painted and AMBER for anything lit - and its other four pieces
/// are hull-coloured, so five pieces read as one livery rather than as five
/// props.
///
/// In priority order, and the reason for each place in it:
///
/// 1. the WINDOW row is the rarest and most specific piece and the only one
///    that says what the ship is FOR, so it gets first refusal on the flanks;
/// 2. the FIN is the only piece that breaks the silhouette, on the high ground
///    and raked outboard, and it is thinned because one fin is a detail and
///    twelve are a radar farm;
/// 3. the STRIPE is the look. See its own note - it is the one rule here that
///    had to be measured into existence rather than written;
/// 4. the FAIRING is the filler on panelling, hull-coloured on purpose;
/// 5. the BEACON reads the pocket and therefore goes LAST - `near_fitting` is
///    the measured carpeting trap, and first in a list it starves everything
///    under it.
fn civilian_style(assets: &BaseContentAssets) -> ShipStyleConfig {
    ShipStyleConfig {
        id: CIVILIAN_STYLE_ID.to_string(),
        name: "Civilian".to_string(),
        // LINEAR values. The top is ~0.68 sRGB against the built-in 0.36: a
        // painted airframe, not a slate. Metallic 0 and roughness 0.35 is
        // satin PAINT - the one material choice that separates a hull sold to
        // a customer from one welded out of plate.
        surfaces: vec![
            StyleSurfaceConfig {
                surface: ShellSurface::Top,
                color: Color::linear_rgb(0.420, 0.420, 0.400),
                roughness: 0.35,
                metallic: 0.0,
            },
            // Seven times darker than the top, and worth far less than that
            // ratio suggests. MEASURED, by painting this surface pure red and
            // shooting the row: a derived skin hides nearly all of its own
            // walls behind neighbouring plates, and what survives is a thin
            // wedge at the OUTER SILHOUETTE and the lip around a fitting. So
            // this is not the panel-line trick it looks like - it is an outline
            // that keeps a pale hull from bleeding into space.
            StyleSurfaceConfig {
                surface: ShellSurface::Wall,
                color: Color::linear_rgb(0.058, 0.060, 0.068),
                roughness: 0.50,
                metallic: 0.05,
            },
        ],
        fixtures: vec![
            StyleFixtureConfig {
                id: "civilian_windows".to_string(),
                model: assets.greeble_civilian_windows.clone(),
                health: 6.0,
                density: 0.05,
                collider: Vec3::new(0.28, 0.06, 0.7),
                // The FLANKS. `Flat` alone would land almost nowhere (6-22
                // plates a ship, and none on a hand-built hull), so a step - a
                // panel climbing structure beside it - counts as panel too,
                // and the seat gate keeps only the clean ramps of them.
                //
                // `Brink` is in, and the run and depth gates are off (task
                // 20260816-203812). Measured: a hand-built flank NEVER grows a
                // flat interior - a two-cell-tall side is edge row on edge row
                // - so the old `Flat/Step + min_run 2 + min_depth 2` filter
                // was eligible NOWHERE on the whole bench and the style's most
                // characterful piece never shipped. The straight edge of a
                // flank is exactly where a liner draws its window line, and
                // the facing is what keeps the rule off the deck edges the
                // stripe owns end to end. Strided and shared down hard: a
                // window row is precious, and a hull covered in them is a
                // cruise liner.
                scatter: ScatterRule {
                    relief: vec![PlateRelief::Flat, PlateRelief::Step, PlateRelief::Brink],
                    facing: PlateFacing::Side,
                    stride: 2,
                    chance: 0.8,
                    patch: 4,
                    align: ScatterAlign::Run,
                    ..Default::default()
                },
            },
            StyleFixtureConfig {
                id: "civilian_fin".to_string(),
                model: assets.greeble_civilian_fin.clone(),
                health: 5.0,
                density: 0.05,
                collider: Vec3::new(0.1, 0.29, 0.24),
                // The high ground, turned OUTBOARD so the blade rakes off the
                // ship. `min_height` is 1 and not 2 for the reason the
                // placeholder mast records: a ridge fills an eighth of its cell
                // and a spur less, so asking for half a cell of plate under a
                // tall piece asks for something no pointy plate is. `seat: Any`
                // is the same argument about the crease: all three of these
                // reliefs are cones, and a fin belongs on exactly them.
                scatter: ScatterRule {
                    relief: vec![PlateRelief::Spur, PlateRelief::Ridge, PlateRelief::Peak],
                    seat: ScatterSeat::Any,
                    facing: PlateFacing::Up,
                    min_depth: 1,
                    min_height: 1,
                    chance: 0.5,
                    align: ScatterAlign::Outward,
                    ..Default::default()
                },
            },
            StyleFixtureConfig {
                id: "civilian_stripe".to_string(),
                model: assets.greeble_civilian_stripe.clone(),
                health: 8.0,
                density: 0.1,
                collider: Vec3::new(0.3, 0.09, 0.6),
                // THE LOOK. A continuous band, and every part of this rule is
                // there to keep it continuous:
                //
                // - `Brink` is the straight edge of a hull, and the only relief
                //   whose run is a LINE rather than a patch;
                // - `min_run: 3` so it only fires where there is a run to
                //   follow. A two-cell band is a stub, not a livery;
                // - NO stride and NO share. Both turn a line back into dashes,
                //   and one missing cell in a band reads as a mistake rather
                //   than as sparsity;
                // - `align: Run` lays each piece down the edge, and the model
                //   fills its whole cell along that axis, so neighbours butt
                //   together into one unbroken rail.
                //
                // It was authored the other way first - a rounded terminal on
                // `max_border: Some(0)` and this band on `min_border: 1` - and
                // the diagnostic killed it in one run: `civilian_stripe x0 of
                // 0`. `border` is the minimum over ALL FOUR in-plane walks, and
                // a hull edge always has open space on one side and unlike
                // plate on the other, so a `Brink` reads border 0 ALWAYS. Only
                // the middle of a wide flat field can read more. The terminal
                // rule therefore took the whole edge and the band never landed
                // - a row of 0.58-cell pills with a 0.42-cell gap between each,
                // which photographs as a dashed line and is why the piece it
                // used to need is not in this kit.
                //
                // The collider is deliberately SHORTER than the model. Every
                // other piece here is smaller than its cell, but this one fills
                // it, and two full-length boxes on adjacent plates would be two
                // bodies born overlapping.
                scatter: ScatterRule {
                    relief: vec![PlateRelief::Brink],
                    min_run: 3,
                    align: ScatterAlign::Run,
                    ..Default::default()
                },
            },
            StyleFixtureConfig {
                id: "civilian_fairing".to_string(),
                model: assets.greeble_civilian_fairing.clone(),
                health: 14.0,
                density: 0.15,
                collider: Vec3::new(0.4, 0.16, 0.4),
                // The filler, and the only rule with no accent on it: a
                // hull-coloured shell over whatever a working ship would have
                // left exposed. Panelled reliefs like the windows, minus the
                // facing - a fairing belongs on the roof as much as the flank.
                // `patch` carries it onto a small build, where the share and the
                // stride between them would otherwise leave nothing.
                //
                // Also the kit's THIN-SHAPE CARRIER (task 20260816-203812),
                // on the armoured cap's filter shape: `Spur` in the relief,
                // the seat gate off, no height floor - a spur measures height
                // 0. A one-cell-thick build is all cones, so without one
                // cone-friendly filler the style's only reach there is the
                // fin's coin toss, and the bench's two-cell run wore NOTHING.
                // A fairing is the piece that can afford this: a smooth shell
                // over a spar tip is what fairing structure MEANS.
                scatter: ScatterRule {
                    relief: vec![PlateRelief::Flat, PlateRelief::Step, PlateRelief::Spur],
                    seat: ScatterSeat::Any,
                    stride: 2,
                    chance: 0.8,
                    patch: 3,
                    ..Default::default()
                },
            },
            StyleFixtureConfig {
                id: "civilian_beacon".to_string(),
                model: assets.greeble_civilian_beacon.clone(),
                health: 4.0,
                density: 0.05,
                collider: Vec3::new(0.2, 0.1, 0.2),
                // LAST, and only ever last: "near a fitting" reads as narrow
                // and is not, and first in a priority list it carpeted 45% of a
                // measured hull. Down here it lights the drive bays and the gun
                // wells with whatever plate the four rules above did not want.
                scatter: ScatterRule {
                    near_fitting: Some(1),
                    stride: 2,
                    chance: 0.6,
                    ..Default::default()
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
///    is gated on the pocket distance so it clusters round the drives and wells
///    a ship gets shot at, the bare-steel plate on `min_depth` so it covers the
///    thick body, and the rusted scab is a per-patch floor over everything
///    else. So a REGION comes out mostly one material with the others cutting
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
/// 6. **The seam is thinned by the RUN, not by a share.** It stands at
///    `chance: 1.0` and is gated on `min_run: 4`, so a long hull edge is welded
///    down its whole length and a short one is left bare. Which edges carry a
///    seam is then a property of the hull, and a share would only have dotted
///    every edge equally - the confetti failure again.
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
/// # The identity is spread across reliefs on purpose
///
/// No one piece carries the look. The whip reads the crests and tips, the drum
/// the broad decks, the seam the straight edges, the two big patches the body
/// and the fittings, and the scab whatever is left. That is partly what
/// mismatch means, and partly insurance: a look concentrated on ONE relief is
/// hostage to which way that relief happens to face, and a hull whose edges all
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
/// `near_fitting` is third and fifth rather than first (it reads as narrow and
/// is not), and the broad patches carry `min_height` so they stay off the
/// slivers.
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
        surfaces: vec![
            StyleSurfaceConfig {
                surface: ShellSurface::Top,
                color: Color::linear_rgb(0.024, 0.0175, 0.0125),
                roughness: 0.94,
                metallic: 0.04,
            },
            StyleSurfaceConfig {
                surface: ShellSurface::Wall,
                color: Color::linear_rgb(0.0038, 0.0028, 0.0020),
                roughness: 0.96,
                metallic: 0.04,
            },
        ],
        fixtures: vec![
            StyleFixtureConfig {
                id: "salvage_whip".to_string(),
                model: assets.greeble_salvage_whip.clone(),
                health: 6.0,
                density: 0.04,
                collider: Vec3::new(0.10, 0.42, 0.16),
                // The HIGH GROUND, and the same reading the placeholder mast
                // took, for the same measured reason: `min_height` is 1 and not
                // 2 because a ridge fills an eighth of its cell and a spur
                // less, so a mast asking for half a cell lands nowhere.
                //
                // `Outward` is what makes it a raider's aerial: the whip is
                // modelled leaning down its own `+Z`, so the fall turns it out
                // over the edge it stands on rather than upright.
                //
                // `seat: Any` because the crests and tips it wants are cones,
                // every one of them - see the module note on the seat.
                scatter: ScatterRule {
                    relief: vec![
                        PlateRelief::Ridge,
                        PlateRelief::Peak,
                        PlateRelief::Spur,
                        PlateRelief::Step,
                    ],
                    seat: ScatterSeat::Any,
                    facing: PlateFacing::Up,
                    min_depth: 2,
                    min_height: 1,
                    chance: 0.18,
                    align: ScatterAlign::Outward,
                    ..Default::default()
                },
            },
            StyleFixtureConfig {
                id: "salvage_drum".to_string(),
                model: assets.greeble_salvage_drum.clone(),
                health: 24.0,
                density: 0.18,
                collider: Vec3::new(0.22, 0.23, 0.34),
                // External tankage needs somewhere broad and solid to be
                // strapped to, which is the one thing a generated hull has few
                // of - `Flat` is 6-22 plates a ship. `Step` is in the list to
                // widen it: a plate climbing structure beside it is a corner to
                // wedge a drum into, and a hand-built hull has nothing else.
                scatter: ScatterRule {
                    relief: vec![PlateRelief::Flat, PlateRelief::Step],
                    min_run: 2,
                    min_height: 2,
                    min_depth: 2,
                    // Measured: at 0.28 a hull carried 13-14 drums, which is a
                    // scrapyard rather than a raider. The share is now barely
                    // more than the FLOOR does on its own - one tank per block
                    // of five cells, which is a density stated in ship rather
                    // than in plates.
                    chance: 0.10,
                    patch: 6,
                    align: ScatterAlign::Run,
                    ..Default::default()
                },
            },
            StyleFixtureConfig {
                id: "salvage_hook".to_string(),
                model: assets.greeble_salvage_hook.clone(),
                health: 14.0,
                density: 0.15,
                collider: Vec3::new(0.20, 0.11, 0.14),
                // The tight ring round a fitting: `Some(1)` is the four cells
                // beside a nozzle or a gun well. Third and not first, and on a
                // lattice with a share on top, because this rule is the
                // documented carpet - first in the list it took 45% of a ship.
                scatter: ScatterRule {
                    near_fitting: Some(1),
                    stride: 2,
                    chance: 0.5,
                    ..Default::default()
                },
            },
            StyleFixtureConfig {
                id: "salvage_weld_seam".to_string(),
                model: assets.greeble_salvage_weld_seam.clone(),
                health: 18.0,
                density: 0.12,
                collider: Vec3::new(0.2, 0.06, 0.92),
                // The STRAIGHT EDGE of a hull and nothing else, which is the
                // one relief with a long line to run a bead down.
                //
                // The only rule in the kit at `chance: 1.0`, and that is the
                // whole point of it: the bead is 0.92 of a cell long, so
                // consecutive plates of one run JOIN into an unbroken seam. A
                // share here would dot the line instead, and a dotted line is
                // the confetti failure wearing a different hat.
                //
                // `min_run: 3` is then the only thing thinning it, and it is
                // what stops the bead reading as machined trim: an edge with a
                // real length to it is welded down its whole length and a short
                // one is left bare, so the seams are a property of the hull
                // rather than a uniform finish on it.
                //
                // Measured at 4, and it was too tight: the third subject ship
                // has no brink run that long anywhere and logged `x0 of 0`. A
                // rule that lands nothing on one hull in three is not a sparse
                // accent, it is a broken filter.
                //
                // It was meant to be `min_border: 1` - stop the bead one cell
                // short of each end of the run, a repair that ran out where the
                // salvaged plate did. That is IMPOSSIBLE on a `Brink` and the
                // log said so instantly (`x0 of 0`): `border` is the MINIMUM of
                // the four in-plane like-runs, and a plate that FALLS AWAY has
                // vacuum on one side and unlike plate on the other by
                // definition, so every `Brink`, `Bevel` and `Spur` on every hull
                // reads border 0. `min_border` is a rule for the interior of a
                // patch of like plate, and only `Flat` has one.
                scatter: ScatterRule {
                    relief: vec![PlateRelief::Brink],
                    min_run: 3,
                    align: ScatterAlign::Run,
                    ..Default::default()
                },
            },
            StyleFixtureConfig {
                id: "salvage_patch_strip".to_string(),
                model: assets.greeble_salvage_patch_strip.clone(),
                health: 18.0,
                density: 0.14,
                collider: Vec3::new(0.45, 0.05, 0.84),
                // The livery-green offcut, round the FITTINGS: a drive bay
                // comes out with cleats right against the mouth and green
                // plating over the cells beside it, which is the damage pattern
                // the look is about - a ship shot where its fittings are and
                // patched there.
                //
                // Measured, and the `near_fitting` trap caught this rule too:
                // `Some(2)` reads as "two cells out" and admitted 80-92 plates
                // of a 132-162 plate hull, better than half the ship. `Some(1)`
                // is the four cells beside a mouth and means what it says.
                scatter: ScatterRule {
                    near_fitting: Some(1),
                    min_height: 1,
                    chance: 0.35,
                    align: ScatterAlign::Run,
                    ..Default::default()
                },
            },
            StyleFixtureConfig {
                id: "salvage_patch_plate".to_string(),
                model: assets.greeble_salvage_patch_plate.clone(),
                health: 20.0,
                density: 0.15,
                collider: Vec3::new(0.88, 0.04, 0.68),
                // The BODY of the ship, and the broadest rule in the kit, so
                // it is next to last. `min_depth` 2 keeps bare steel off the
                // skin over a one-cell spar, where a doubler plate this size
                // would be wider than the thing it is bolted to.
                //
                // This rule used to carry a relief list - `Flat`, `Bevel`,
                // `Brink` and `Step` - and it was the predicate spelled badly:
                // an enumeration of "the reliefs with a broad top", written
                // before there was a way to ask. It has come off. The SEAT says
                // what it was reaching for and says it exactly, and it says it
                // NARROWER: 308 of 526 measured plates against the 380 the list
                // admitted, because half the steps in it were cones and the
                // bevels all were. `min_depth` still leaves the one-cell spars
                // alone, and the tips and crests still go to the whip and the
                // scab.
                scatter: ScatterRule {
                    min_depth: 2,
                    min_height: 1,
                    chance: 0.4,
                    align: ScatterAlign::Run,
                    ..Default::default()
                },
            },
            StyleFixtureConfig {
                id: "salvage_patch_scab".to_string(),
                model: assets.greeble_salvage_patch_scab.clone(),
                health: 16.0,
                density: 0.15,
                collider: Vec3::new(0.52, 0.03, 0.44),
                // The FILLER, and the piece that has to carry a hull nothing
                // else fits - a spar, a tip, a hand-built ship with no flat
                // plate on it at all. A small share plus a per-patch floor, so
                // it reads as a scatter of old repairs on a big hull and still
                // puts something on a small one.
                //
                // The seat gate is OFF (task 20260816-203812): this is the
                // kit's thin-shape carrier, and a one-cell-thick build has no
                // coplanar plate anywhere - seated, the filler that exists for
                // "a hull nothing else fits" was refusing exactly that hull,
                // and the thin half of the bench photographed as bare rock.
                // `min_height` stays: a scab on a crest or a stud reads as a
                // repair, a scab hovering over a height-0 spur tip does not -
                // the tips stay the whip's.
                scatter: ScatterRule {
                    seat: ScatterSeat::Any,
                    min_height: 1,
                    chance: 0.05,
                    patch: 10,
                    ..Default::default()
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
/// 1. the mast reads RELIEF, FACING, DEPTH and HEIGHT together - only on the
///    high ground of a ship's upper surface, and only where there is ship under
///    it to bolt to;
/// 2. the vent reads the RUN and aligns to it, on a stride, which is the
///    grid-occupancy claim the research settled on instead of blue noise. It
///    also carries the density FLOOR, so a hand-built hull that the stride and
///    the share between them would have thinned to nothing still wears a row;
/// 3. the block reads the BORDER and the FALL - trim at the end of a run, and
///    on the straight edge of a hull it is turned OUTBOARD, which the run alone
///    could not say;
/// 4. the blister reads the POCKET distance - beside the mouth of a fitting,
///    which is the "weight decoration toward link points" finding. It goes LAST
///    because "near a fitting" is broad even now that the distance is counted in
///    face steps: first in the order, on the ring it used to be measured over,
///    it carpeted 45% of every ship and the other three rules never got a plate.
fn placeholder_style(assets: &BaseContentAssets) -> ShipStyleConfig {
    ShipStyleConfig {
        id: PLACEHOLDER_STYLE_ID.to_string(),
        name: "Placeholder".to_string(),
        // A restatement of the built-in plate colours with the top lifted and
        // warmed a little: enough to prove a style really does dress the
        // derived plates, and not so much that the greebles stop reading
        // against them.
        surfaces: vec![
            StyleSurfaceConfig {
                surface: ShellSurface::Top,
                color: Color::linear_rgb(0.125, 0.135, 0.160),
                roughness: 0.6,
                metallic: 0.2,
            },
            StyleSurfaceConfig {
                surface: ShellSurface::Wall,
                color: Color::linear_rgb(0.070, 0.082, 0.110),
                roughness: 0.7,
                metallic: 0.15,
            },
        ],
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
                density: 0.05,
                collider: Vec3::new(0.12, 0.38, 0.12),
                // The HIGH GROUND, which now includes the SPUR - the tips and
                // outer corners a hull ends at. Before the falling plate split
                // three ways there was no way to say that: a tip and the middle
                // of a flank were one relief.
                //
                // `min_height` is 1 and not 2, measured: a ridge fills an eighth
                // of its cell and a spur less, so a mast asking for half a cell
                // of plate under it was asking for something no pointy plate
                // is - the rule read as narrow and landed on NOTHING. `seat` is
                // the same trap in the same rule: the high ground is a cone, so
                // a mast has to ask for one.
                scatter: ScatterRule {
                    relief: vec![
                        PlateRelief::Ridge,
                        PlateRelief::Peak,
                        PlateRelief::Step,
                        PlateRelief::Spur,
                    ],
                    seat: ScatterSeat::Any,
                    facing: PlateFacing::Up,
                    min_depth: 2,
                    min_height: 1,
                    chance: 0.25,
                    ..Default::default()
                },
            },
            StyleFixtureConfig {
                id: "placeholder_vent".to_string(),
                model: assets.greeble_vent.clone(),
                health: 12.0,
                density: 0.15,
                collider: Vec3::new(0.32, 0.04, 0.2),
                // FLAT PANEL. `Bevel` was in this list to widen it - a panel
                // with one corner taken off, and there are more of them on a
                // generated hull than there are flat plates - and it came off
                // when the seat gate went in: a bevel is a CONE, so a vent on
                // one never lay flat. `patch` is the floor under the stride and
                // the share: without it this rule is a field of vents on a big
                // hull and nothing at all on a small one.
                scatter: ScatterRule {
                    relief: vec![PlateRelief::Flat],
                    min_run: 2,
                    stride: 2,
                    chance: 0.8,
                    patch: 3,
                    align: ScatterAlign::Run,
                    ..Default::default()
                },
            },
            StyleFixtureConfig {
                id: "placeholder_block".to_string(),
                model: assets.greeble_block.clone(),
                health: 20.0,
                density: 0.2,
                collider: Vec3::new(0.22, 0.1, 0.22),
                // No stride. A lattice is for making a ROW, which is what the
                // vent above wants; the END of a run is already sparse and
                // already structured, and striding it as well left a small
                // build with a single greeble on it.
                //
                // Turned OUTBOARD rather than down the run: at the end of a run
                // is exactly where a hull turns a corner, and the fall is the
                // only reading that says which side of it is off the ship. A
                // plate that does not fall one way is left square, which is the
                // rest of the hull.
                //
                // The PURE per-patch form: no share at all, and one piece per
                // block of hull. This is the rule that has to carry a small
                // build, because it is the only one a hand-built hull passes -
                // the vent wants flat plate and there is none - and a share
                // tuned on the row put ONE piece on a 20-plate ship. Stated as a
                // density instead, it reads the same at both sizes.
                scatter: ScatterRule {
                    // At the end of a RUN, and a run is two cells or more. The
                    // border alone reads as specific and is not: on a hull as
                    // broken up as the generator draws, nearly every plate is at
                    // the end of its own one-cell run, so the rule admitted 126
                    // of 132 plates - the `near_fitting` trap wearing another
                    // field's name.
                    min_run: 2,
                    max_border: Some(0),
                    // A quarter cell, not half. Measured on a hand-built hull:
                    // it is nearly all ridges and spurs, which fill an eighth
                    // of their cell, so a trim piece asking for half a cell of
                    // plate is the vent's mistake again in another rule.
                    min_height: 1,
                    chance: 0.0,
                    patch: 3,
                    align: ScatterAlign::Outward,
                    ..Default::default()
                },
            },
            StyleFixtureConfig {
                id: "placeholder_blister".to_string(),
                model: assets.greeble_blister.clone(),
                health: 16.0,
                density: 0.2,
                collider: Vec3::new(0.18, 0.07, 0.18),
                scatter: ScatterRule {
                    near_fitting: Some(1),
                    stride: 2,
                    chance: 0.5,
                    ..Default::default()
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

    /// The SEAT gate is on everywhere but the high ground and the thin-shape
    /// carriers, and every piece that opts out is named here.
    ///
    /// Pinned as a LIST rather than as a count: `seat: Any` is the one field
    /// that puts a piece back on a crease, and a rule that acquires it quietly
    /// is the placement defect coming back. Five of these stand on the
    /// pointiest thing on the hull; the fairing and the scab are the carriers
    /// task 20260816-203812 opened - one cone-friendly filler per style, so a
    /// one-cell-thick build (all cones, zero coplanar plates) is not bare.
    #[test]
    fn only_the_high_ground_and_the_carriers_opt_out_of_the_seat() {
        let opted: Vec<String> = style_catalog(&BaseContentAssets::from_paths())
            .into_iter()
            .flat_map(|style| style.fixtures)
            .filter(|fixture| fixture.scatter.seat == ScatterSeat::Any)
            .map(|fixture| fixture.id)
            .collect();
        assert_eq!(
            opted,
            vec![
                "industrial_stack",
                "armoured_cap",
                "civilian_fin",
                "civilian_fairing",
                "salvage_whip",
                "salvage_patch_scab",
                "placeholder_mast",
            ],
        );
    }

    /// No SEATED rule names a relief that can never carry a seat.
    ///
    /// The relief list says which ZONE of a hull a piece belongs in; the seat
    /// says whether there is a surface to lie on. `Bevel`, `Ridge`, `Peak` and
    /// `Spur` are cones every time, so listing one in a seated rule is a filter
    /// that can never fire - and it is exactly how these lists were written
    /// before the seat existed, as an enumeration of "the reliefs with a broad
    /// top" that was wrong about a fifth of a hull.
    #[test]
    fn no_seated_rule_names_a_relief_that_is_always_a_cone() {
        for style in style_catalog(&BaseContentAssets::from_paths()) {
            for fixture in style.fixtures {
                if fixture.scatter.seat != ScatterSeat::Whole {
                    continue;
                }
                for relief in &fixture.scatter.relief {
                    assert!(
                        matches!(
                            relief,
                            PlateRelief::Flat | PlateRelief::Brink | PlateRelief::Step
                        ),
                        "'{}' is seated and asks for {relief:?}, which is a cone \
                         every time - the rule can never fire",
                        fixture.id,
                    );
                }
            }
        }
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
        assert_eq!(style.fixtures.len(), 7, "the kit is capped at 6-8 pieces");
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
        assert!(band.scatter.chance >= 1.0, "a dashed stripe is confetti");
        assert_eq!(band.scatter.stride, 1, "a lattice would break the line");
        assert!(
            band.scatter.min_run >= 2,
            "a one-cell corner is not an edge"
        );
        assert_eq!(
            band.scatter.align,
            ScatterAlign::Run,
            "a stripe that does not lie down its own edge is a smear",
        );
    }

    /// `near_fitting` reads as narrow and is not, so the one rule that uses it
    /// must never be able to claim a hull before the rest of the kit has had a
    /// look. Pinned rather than commented, because the order is the only thing
    /// holding it.
    #[test]
    fn the_broad_rules_of_the_industrial_kit_come_last() {
        let style = industrial_style(&BaseContentAssets::from_paths());
        let pocket = style
            .fixtures
            .iter()
            .position(|fixture| fixture.scatter.near_fitting.is_some())
            .expect("the kit reads the pocket distance");
        let shaped = style
            .fixtures
            .iter()
            .position(|fixture| !fixture.scatter.relief.is_empty())
            .expect("the kit reads the relief");
        assert!(
            shaped < pocket,
            "a `near_fitting` rule above a shaped one carpets the ship",
        );
    }

    /// Every piece of a kit is prefixed with the style that owns it - the ids
    /// salt the scatter and name the `.glb`, and four looks are authored into
    /// one pair of folders.
    #[test]
    fn the_armoured_kit_is_namespaced() {
        let style = armoured_style(&BaseContentAssets::from_paths());
        assert!(!style.fixtures.is_empty());
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
        assert_eq!(belt.scatter.relief, vec![PlateRelief::Brink]);
        assert_eq!(belt.scatter.align, ScatterAlign::Run);
        assert!(
            belt.scatter.min_run >= 2,
            "a one-cell run is not a run - see the `max_border` trap",
        );
        assert!(
            belt.scatter.stride <= 1 && belt.scatter.chance >= 1.0,
            "a belt with holes thinned into it is not a belt",
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
            .map(|fixture| &fixture.scatter)
            .expect("the civilian kit has no band");

        assert_eq!(stripe.stride, 1, "a strided band is a dashed line");
        assert_eq!(stripe.chance, 1.0, "a shared band is a dashed line");
        assert_eq!(
            stripe.align,
            ScatterAlign::Run,
            "a band not laid down its run cannot join its neighbours",
        );
        assert_eq!(
            stripe.relief,
            vec![PlateRelief::Brink],
            "the band follows the straight edge of a hull and nothing else",
        );
        assert!(
            stripe.min_run >= 3,
            "a band shorter than three cells is a stub",
        );
        // The measured trap: `border` is the minimum over all four in-plane
        // walks, and a hull edge has open space on one side, so a `Brink` reads
        // border 0 always. Asking a band for the INTERIOR of its run is asking
        // for a plate no generated hull has - `x0 of 0` in the spawn log.
        assert_eq!(
            stripe.min_border, 0,
            "a Brink never reads a border above 0, so this band can never land",
        );
    }

    /// `near_fitting` is the measured carpeting trap: it reads as narrow and
    /// admits most of a hull. The civilian kit uses it exactly once, and that
    /// rule must be the LAST in the priority list or it starves the five above.
    #[test]
    fn the_civilian_pocket_rule_is_last() {
        let style = civilian_style(&BaseContentAssets::from_paths());
        let pocket: Vec<usize> = style
            .fixtures
            .iter()
            .enumerate()
            .filter(|(_, fixture)| fixture.scatter.near_fitting.is_some())
            .map(|(index, _)| index)
            .collect();
        assert_eq!(
            pocket,
            vec![style.fixtures.len() - 1],
            "the pocket rule is not the one and only last rule",
        );
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
        assert!(
            (6..=8).contains(&style.fixtures.len()),
            "the salvage kit is {} pieces, and the cap is 6 to 8",
            style.fixtures.len(),
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
            let rule = &fixture.scatter;
            let narrowed = !rule.relief.is_empty()
                || !rule.facing.is_any()
                || rule.min_run > 0
                || rule.min_height > 0
                || rule.min_depth > 0
                || rule.min_enclosure > 0
                || rule.min_border > 0
                || rule.max_border.is_some()
                || rule.near_fitting.is_some()
                || rule.stride > 1;
            assert!(
                narrowed,
                "'{}' filters nothing, so it claims every plate under it",
                fixture.id,
            );
            // A rule at full share takes every plate its filter admits, which
            // is a carpet unless the filter is a LINE - the continuity case,
            // where a share would only dot the piece it is trying to join up.
            assert!(
                rule.chance < 1.0
                    || rule.stride > 1
                    || (!rule.relief.is_empty() && rule.min_run >= 3),
                "'{}' takes every plate it is eligible for without naming a \
                 relief and a run, which is a carpet rather than a seam",
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
                .scatter
                .align
        };
        assert_eq!(aligned("salvage_patch_plate"), ScatterAlign::Run);
        assert_eq!(aligned("salvage_patch_strip"), ScatterAlign::Run);
        assert_eq!(
            aligned("salvage_patch_scab"),
            ScatterAlign::Free,
            "the filler has no long axis to turn, and turning it would line the \
             fillers up with each other",
        );
    }

    /// The placeholder style's rules cover the whole vocabulary, which is the
    /// only reason it exists: the checkpoint render has to be able to show
    /// whether the readings can carry a look.
    #[test]
    fn the_placeholder_style_exercises_every_reading() {
        let style = placeholder_style(&BaseContentAssets::from_paths());
        let rules: Vec<&ScatterRule> = style.fixtures.iter().map(|f| &f.scatter).collect();
        assert!(
            rules.iter().any(|rule| rule.near_fitting.is_some()),
            "nothing reads the pocket distance",
        );
        assert!(
            rules.iter().any(|rule| !rule.relief.is_empty()),
            "nothing reads the relief",
        );
        assert!(
            rules.iter().any(|rule| !rule.facing.is_any()),
            "nothing reads the facing",
        );
        assert!(
            rules.iter().any(|rule| rule.min_depth > 0),
            "nothing reads the support depth",
        );
        assert!(
            rules.iter().any(|rule| rule.min_height > 0),
            "nothing reads how much of its cell a plate fills",
        );
        assert!(
            rules
                .iter()
                .any(|rule| rule.min_run > 0 && rule.align == ScatterAlign::Run),
            "nothing reads the run or aligns to it",
        );
        assert!(
            rules.iter().any(|rule| rule.align == ScatterAlign::Outward),
            "nothing turns a piece off the ship, so nothing reads the fall",
        );
        assert!(
            rules
                .iter()
                .any(|rule| rule.relief.iter().any(|relief| matches!(
                    relief,
                    PlateRelief::Bevel | PlateRelief::Brink | PlateRelief::Spur
                ))),
            "nothing stands on the falling plate, which is four fifths of a hull",
        );
        assert!(
            rules.iter().any(|rule| rule.max_border == Some(0)),
            "nothing reads the border, so nothing is trim",
        );
        assert!(
            rules.iter().any(|rule| rule.stride > 1),
            "nothing claims a lattice, which is what keeps a look off confetti",
        );
        assert!(
            rules.iter().any(|rule| rule.patch > 0),
            "nothing normalises its density, so a small build wears almost none",
        );
    }
}
