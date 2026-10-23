//! The base game's BLOCK fleet: hand-authored ships assembled from shipped
//! cube prototypes on a cell grid, wearing a derived skin.
//!
//! A block ship is a set of cells plus a handful of placed specials. It uses no
//! modelled part, so a hull is reproducible from this source alone. A cell
//! takes the hull's plate unless a zone names another hull prototype for it:
//! the freight cubes and tanks of the derelict freighter.
//!
//! The six start hulls are built from parts at advancement 0 only: the
//! starting station builds every one of those, so any section a starter
//! loses can be rebuilt there. The start-role lint refuses a part above 0.
//!
//! Every hull names the style its derived skin is clad with. See
//! `base_content::styles`.
//!
//! Cells are BUILD-GRID cells, the one authored coordinate that is not metric:
//! one cell is one world unit is 10 m. `-Z` is the bow, `+Z` the stern, `-X`
//! the port flank.
//!
//! Every crewed hull carries a docking port, `port_collar`, standing off its
//! port flank: every station service needs a dock. The patrol picket and the
//! line warship carry a second, `starboard_collar`, so losing one flank in a
//! fight still leaves a hatch to dock by.

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use nova_scenario::prelude::{SectionSource, SpaceshipSectionConfig};
use nova_ship::prelude::{
    BASIC_CONTROLLER_SECTION_ID, BASIC_THRUSTER_SECTION_ID, DOCKING_PORT_SECTION_ID,
    LIGHT_HULL_SECTION_ID, PDC_KINETIC_TURRET_SECTION_ID, RAILGUN_LANCE_SECTION_ID,
    REINFORCED_HULL_SECTION_ID, TORPEDO_SECTION_ID,
};
use nova_world_base::prelude::{
    ARMOURED_STYLE_ID, CIVILIAN_STYLE_ID, INDUSTRIAL_STYLE_ID, SALVAGE_STYLE_ID,
};

use crate::base_content::sections::{
    CARGO_HULL_SECTION_ID, CARGO_INTAKE_SECTION_ID, MINING_BEAM_SECTION_ID, SOLAR_PANEL_SECTION_ID,
    TANK_HULL_SECTION_ID, VECTOR_THRUSTER_SECTION_ID,
};

/// The drive section ids the block fleet repeats: a pair on the beam, one on
/// the centreline, one vectoring drive on a transom, or the centre bell of a
/// square of nine. Content that patches a drive names these.
const DRIVE_PORT_SECTION_ID: &str = "drive_port";
const DRIVE_STARBOARD_SECTION_ID: &str = "drive_starboard";
const DRIVE_CENTRE_SECTION_ID: &str = "drive_centre";
pub(crate) const MAIN_DRIVE_SECTION_ID: &str = "main_drive";

/// The second flight computer a hull heavier than one computer turns carries.
const CONTROL_AFT_SECTION_ID: &str = "control_aft";

/// How far a turret drops into its own cell to put its socket on the plate
/// below: the mount's one link point sits a quarter cell under its centre.
const TURRET_SEAT: Vec3 = Vec3::new(0.0, -0.25, 0.0);

/// The section id every block ship's main flight computer carries, so content
/// can harden or disable one bridge without knowing which hull it is on.
pub const BLOCK_BRIDGE_SECTION_ID: &str = "bridge";

/// The waystation's second flight computer, in the far corner of its lower
/// deck: a warhead from above or off the collar flank spends the station's
/// pool down to its floor before it reaches this one, so one torpedo cannot
/// take both computers and turn the station derelict.
const WAYSTATION_LOWER_BRIDGE_SECTION_ID: &str = "lower_bridge";

/// The port-flank docking collar every crewed block ship and the waystation
/// carry. Content that hardens, reads or shoots the hatch names this rather
/// than the hull it is bolted to.
pub const BLOCK_PORT_COLLAR_SECTION_ID: &str = "port_collar";
/// The starboard-flank docking collar the patrol picket and the line warship
/// carry opposite their port collar.
pub(crate) const BLOCK_STARBOARD_COLLAR_SECTION_ID: &str = "starboard_collar";

/// The point-defense mount the patrol picket and the training picket carry on
/// the nose, so content that arms or disarms it names a section rather than a
/// hull.
pub(crate) const PICKET_GUN_SECTION_ID: &str = "pdc";

/// The line warship's point-defense mounts: dorsal fore, dorsal aft, then the
/// two ventral mounts. Content that sets a magazine per turret walks this.
pub const LINE_WARSHIP_PDC_IDS: [&str; 6] = [
    "pdc_forward_port",
    "pdc_forward_starboard",
    "pdc_aft_port",
    "pdc_aft_starboard",
    "pdc_ventral_port",
    "pdc_ventral_starboard",
];

/// The mining tug's emitter, at the tip of its bow boom.
pub const MINING_TUG_EMITTER_ID: &str = "mining_beam";

/// The hull side a flank collar opens on.
#[derive(Clone, Copy)]
enum FlankType {
    /// World `-X`.
    Port,
    /// World `+X`.
    Starboard,
}

/// One placed part that is not a plain hull cell. A special whose position
/// lands exactly on a cell REPLACES that cell; one placed off the grid (a
/// turret seated on a face, a drive standing off the transom) is added beside
/// it.
#[derive(Clone, Copy)]
pub(super) struct Special {
    id: &'static str,
    prototype: &'static str,
    position: Vec3,
    rotation: Quat,
}

/// A block hull: its cells, its specials, the plate every unclaimed cell is
/// built from, the zones that build some cells from another hull prototype,
/// and the style its skin wears.
pub(super) struct BlockShip {
    pub(super) cells: Vec<IVec3>,
    pub(super) specials: Vec<Special>,
    /// The prototype every plain cell outside a zone takes.
    pub(super) plate: &'static str,
    /// Hull prototypes for listed cells, in place of `plate`. A zone cell
    /// that is not in `cells` is an authoring error the tests refuse.
    pub(super) zones: Vec<(&'static str, Vec<IVec3>)>,
    pub(super) style: &'static str,
}

/// The industrial start: a low work hull of light plate with a two-cell boom
/// off the bow and a mining emitter at its tip, a cargo intake on the aft
/// deck, a cab forward, and three bell drives.
///
/// The boom puts the emitter a hull length ahead of the cab, so the beam lane
/// and the rock it cuts sit clear of the plating.
pub(super) fn mining_tug() -> BlockShip {
    BlockShip {
        cells: union(vec![
            block(IVec3::new(-1, 0, -3), IVec3::new(3, 1, 7)),
            vec![IVec3::new(0, 1, -2), IVec3::new(0, 1, -1)],
            vec![IVec3::new(0, 0, -4), IVec3::new(0, 0, -5)],
        ]),
        specials: vec![
            cell_part(
                BLOCK_BRIDGE_SECTION_ID,
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(0, 1, -2),
            ),
            cell_part(
                MINING_TUG_EMITTER_ID,
                MINING_BEAM_SECTION_ID,
                IVec3::new(0, 0, -6),
            ),
            door_up_intake("cargo_intake", Vec3::new(0.0, 1.0, 1.5)),
            cell_part(
                DRIVE_PORT_SECTION_ID,
                BASIC_THRUSTER_SECTION_ID,
                IVec3::new(-1, 0, 3),
            ),
            cell_part(
                DRIVE_CENTRE_SECTION_ID,
                BASIC_THRUSTER_SECTION_ID,
                IVec3::new(0, 0, 3),
            ),
            cell_part(
                DRIVE_STARBOARD_SECTION_ID,
                BASIC_THRUSTER_SECTION_ID,
                IVec3::new(1, 0, 3),
            ),
            flank_collar(
                BLOCK_PORT_COLLAR_SECTION_ID,
                FlankType::Port,
                IVec3::new(-2, 0, -1),
            ),
        ],
        plate: LIGHT_HULL_SECTION_ID,
        zones: vec![],
        style: INDUSTRIAL_STYLE_ID,
    }
}

/// The scavenger start: a deep three-by-three hold of light plate, a cab on
/// the nose with a mining emitter ahead of it, a cargo intake on the dorsal
/// deck, one point-defense mount, and eight bell drives round the second
/// computer on the square transom.
///
/// The biggest hold of the start hulls: salvage is bulky and the hauler has
/// to bring it home.
pub(super) fn salvage_hauler() -> BlockShip {
    BlockShip {
        cells: union(vec![
            block(IVec3::new(-1, -1, -3), IVec3::new(3, 3, 7)),
            block(IVec3::new(0, 0, -4), IVec3::new(1, 2, 1)),
        ]),
        specials: vec![
            cell_part(
                BLOCK_BRIDGE_SECTION_ID,
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(0, 1, -4),
            ),
            cell_part(
                CONTROL_AFT_SECTION_ID,
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(0, 0, 3),
            ),
            cell_part("mining_beam", MINING_BEAM_SECTION_ID, IVec3::new(0, 0, -5)),
            door_up_intake("cargo_intake", Vec3::new(0.0, 2.0, 0.5)),
            turret("pdc_dorsal", IVec3::new(0, 2, -2)),
            cell_part(
                DRIVE_PORT_SECTION_ID,
                BASIC_THRUSTER_SECTION_ID,
                IVec3::new(-1, 0, 3),
            ),
            cell_part(
                DRIVE_STARBOARD_SECTION_ID,
                BASIC_THRUSTER_SECTION_ID,
                IVec3::new(1, 0, 3),
            ),
            cell_part(
                "drive_dorsal",
                BASIC_THRUSTER_SECTION_ID,
                IVec3::new(0, 1, 3),
            ),
            cell_part(
                "drive_ventral",
                BASIC_THRUSTER_SECTION_ID,
                IVec3::new(0, -1, 3),
            ),
            cell_part(
                "drive_port_dorsal",
                BASIC_THRUSTER_SECTION_ID,
                IVec3::new(-1, 1, 3),
            ),
            cell_part(
                "drive_starboard_dorsal",
                BASIC_THRUSTER_SECTION_ID,
                IVec3::new(1, 1, 3),
            ),
            cell_part(
                "drive_port_ventral",
                BASIC_THRUSTER_SECTION_ID,
                IVec3::new(-1, -1, 3),
            ),
            cell_part(
                "drive_starboard_ventral",
                BASIC_THRUSTER_SECTION_ID,
                IVec3::new(1, -1, 3),
            ),
            flank_collar(
                BLOCK_PORT_COLLAR_SECTION_ID,
                FlankType::Port,
                IVec3::new(-2, 0, -2),
            ),
        ],
        plate: LIGHT_HULL_SECTION_ID,
        zones: vec![],
        style: SALVAGE_STYLE_ID,
    }
}

/// The cells forward of the light freighter's stern: a flat spine, a stack
/// amidships, a cab, and a pod on the starboard flank. Shared with the
/// derelict, so its collar stands where the freighter's does.
fn freighter_bow() -> Vec<IVec3> {
    union(vec![
        block(IVec3::new(-1, 0, -3), IVec3::new(3, 1, 6)),
        block(IVec3::new(-1, 1, -2), IVec3::new(3, 1, 4)),
        vec![IVec3::new(0, 1, -3)],
        block(IVec3::new(2, 0, 0), IVec3::new(1, 1, 2)),
    ])
}

/// The derelict's freight cubes and tanks: the yard fit of a freighter past
/// ring 0, where the stack and the pod are built from parts above
/// advancement 0.
fn derelict_zones() -> Vec<(&'static str, Vec<IVec3>)> {
    vec![
        (
            CARGO_HULL_SECTION_ID,
            block(IVec3::new(-1, 1, -2), IVec3::new(3, 1, 4)),
        ),
        (
            TANK_HULL_SECTION_ID,
            block(IVec3::new(2, 0, 0), IVec3::new(1, 1, 3)),
        ),
    ]
}

/// The light freighter's forward specials: the cab, the second computer and
/// the collar.
fn freighter_fittings() -> Vec<Special> {
    vec![
        cell_part(
            BLOCK_BRIDGE_SECTION_ID,
            BASIC_CONTROLLER_SECTION_ID,
            IVec3::new(0, 1, -3),
        ),
        cell_part(
            CONTROL_AFT_SECTION_ID,
            BASIC_CONTROLLER_SECTION_ID,
            IVec3::new(0, 0, 1),
        ),
        flank_collar(
            BLOCK_PORT_COLLAR_SECTION_ID,
            FlankType::Port,
            IVec3::new(-2, 0, -2),
        ),
    ]
}

/// The light freighter's drives: one bell on each cell of the transom's aft
/// face. Nine bells give the thrust of one vectoring drive from the part
/// every station builds. The centre bell is `main_drive`, the one the
/// derelict's mayday says the raid took.
fn freighter_drives() -> Vec<Special> {
    vec![
        cell_part(
            MAIN_DRIVE_SECTION_ID,
            BASIC_THRUSTER_SECTION_ID,
            IVec3::new(0, 0, 4),
        ),
        cell_part(
            DRIVE_PORT_SECTION_ID,
            BASIC_THRUSTER_SECTION_ID,
            IVec3::new(-1, 0, 4),
        ),
        cell_part(
            DRIVE_STARBOARD_SECTION_ID,
            BASIC_THRUSTER_SECTION_ID,
            IVec3::new(1, 0, 4),
        ),
        cell_part(
            "drive_dorsal",
            BASIC_THRUSTER_SECTION_ID,
            IVec3::new(0, 1, 4),
        ),
        cell_part(
            "drive_ventral",
            BASIC_THRUSTER_SECTION_ID,
            IVec3::new(0, -1, 4),
        ),
        cell_part(
            "drive_port_dorsal",
            BASIC_THRUSTER_SECTION_ID,
            IVec3::new(-1, 1, 4),
        ),
        cell_part(
            "drive_starboard_dorsal",
            BASIC_THRUSTER_SECTION_ID,
            IVec3::new(1, 1, 4),
        ),
        cell_part(
            "drive_port_ventral",
            BASIC_THRUSTER_SECTION_ID,
            IVec3::new(-1, -1, 4),
        ),
        cell_part(
            "drive_starboard_ventral",
            BASIC_THRUSTER_SECTION_ID,
            IVec3::new(1, -1, 4),
        ),
    ]
}

/// The transport start: a stack of light plate on a flat spine, a cargo
/// intake on top of the stack, a pod on the starboard flank, and nine bell
/// drives on a square transom. Unarmed.
pub(super) fn light_freighter() -> BlockShip {
    let mut specials = freighter_fittings();
    specials.push(door_up_intake("cargo_intake", Vec3::new(0.0, 2.0, 0.5)));
    specials.extend(freighter_drives());
    BlockShip {
        cells: union(vec![
            freighter_bow(),
            vec![IVec3::new(2, 0, 2)],
            // The transom: the three-by-three face the drives mate.
            block(IVec3::new(-1, -1, 3), IVec3::new(3, 3, 1)),
        ]),
        specials,
        plate: LIGHT_HULL_SECTION_ID,
        zones: vec![],
        style: CIVILIAN_STYLE_ID,
    }
}

/// A yard-fitted freighter after a raid took its stern: reinforced plate,
/// freight cubes, a tank pod and a solar wing either side of the stack, with
/// no drive, no transom, the aft tank cube and the starboard wing's outer
/// panel gone. Everything forward of the tear matches [`light_freighter`]
/// cell for cell, so the cab and the collar stand where a docking approach
/// expects them.
pub(super) fn freighter_derelict() -> BlockShip {
    let mut specials = freighter_fittings();
    specials.extend([
        solar("solar_port_inner", IVec3::new(-2, 1, -1)),
        solar("solar_port_outer", IVec3::new(-3, 1, -1)),
        solar("solar_starboard_inner", IVec3::new(2, 1, -1)),
    ]);
    BlockShip {
        cells: carve(
            union(vec![
                freighter_bow(),
                // A ragged tear, a column at a time: the derived skin needs an
                // uneven edge to clad a break.
                vec![IVec3::new(-1, 0, 3), IVec3::new(-1, -1, 3)],
            ]),
            &[IVec3::new(1, 1, 1), IVec3::new(1, 0, 2)],
        ),
        specials,
        plate: REINFORCED_HULL_SECTION_ID,
        zones: derelict_zones(),
        style: CIVILIAN_STYLE_ID,
    }
}

/// The bounty hunter start: a compact two-deck body of light plate with stub
/// wings, two point-defense mounts on the top deck, a cargo intake aft of
/// them and three bell drives.
pub(super) fn interceptor() -> BlockShip {
    BlockShip {
        cells: union(vec![
            block(IVec3::new(-1, 0, -2), IVec3::new(3, 2, 5)),
            block(IVec3::new(-2, 0, 1), IVec3::new(1, 1, 2)),
            block(IVec3::new(2, 0, 1), IVec3::new(1, 1, 2)),
        ]),
        specials: vec![
            cell_part(
                BLOCK_BRIDGE_SECTION_ID,
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(0, 1, -1),
            ),
            turret("pdc_port", IVec3::new(-1, 2, -1)),
            turret("pdc_starboard", IVec3::new(1, 2, -1)),
            door_up_intake("cargo_intake", Vec3::new(0.0, 2.0, 1.5)),
            cell_part(
                DRIVE_PORT_SECTION_ID,
                BASIC_THRUSTER_SECTION_ID,
                IVec3::new(-1, 0, 2),
            ),
            cell_part(
                DRIVE_CENTRE_SECTION_ID,
                BASIC_THRUSTER_SECTION_ID,
                IVec3::new(0, 0, 2),
            ),
            cell_part(
                DRIVE_STARBOARD_SECTION_ID,
                BASIC_THRUSTER_SECTION_ID,
                IVec3::new(1, 0, 2),
            ),
            flank_collar(
                BLOCK_PORT_COLLAR_SECTION_ID,
                FlankType::Port,
                IVec3::new(-2, 0, 0),
            ),
        ],
        plate: LIGHT_HULL_SECTION_ID,
        zones: vec![],
        style: ARMOURED_STYLE_ID,
    }
}

/// The pirate start: a short hull of light plate with a nose stub, an
/// outrigger and a gun boom on the starboard side, a dorsal mount, a cargo
/// intake on the aft deck, and four bell drives. Its lopsided silhouette
/// reads as a refit, not a yard design.
pub(super) fn raider() -> BlockShip {
    BlockShip {
        cells: union(vec![
            block(IVec3::new(-1, 0, -2), IVec3::new(3, 1, 6)),
            block(IVec3::new(-1, 1, -2), IVec3::new(3, 1, 3)),
            block(IVec3::new(2, 0, -1), IVec3::new(1, 1, 3)),
            block(IVec3::new(2, 1, -1), IVec3::new(1, 1, 2)),
            vec![IVec3::new(0, 0, -3)],
        ]),
        specials: vec![
            cell_part(
                BLOCK_BRIDGE_SECTION_ID,
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(0, 1, -2),
            ),
            turret("pdc_dorsal", IVec3::new(0, 2, -1)),
            turret("pdc_boom", IVec3::new(2, 2, 0)),
            door_up_intake("cargo_intake", Vec3::new(0.0, 1.0, 1.5)),
            cell_part(
                DRIVE_PORT_SECTION_ID,
                BASIC_THRUSTER_SECTION_ID,
                IVec3::new(-1, 0, 3),
            ),
            cell_part(
                DRIVE_CENTRE_SECTION_ID,
                BASIC_THRUSTER_SECTION_ID,
                IVec3::new(0, 0, 3),
            ),
            cell_part(
                DRIVE_STARBOARD_SECTION_ID,
                BASIC_THRUSTER_SECTION_ID,
                IVec3::new(1, 0, 3),
            ),
            cell_part(
                "drive_outrigger",
                BASIC_THRUSTER_SECTION_ID,
                IVec3::new(2, 0, 2),
            ),
            flank_collar(
                BLOCK_PORT_COLLAR_SECTION_ID,
                FlankType::Port,
                IVec3::new(-2, 0, 0),
            ),
        ],
        plate: LIGHT_HULL_SECTION_ID,
        zones: vec![],
        style: SALVAGE_STYLE_ID,
    }
}

/// The military start: a slim dart of light plate with one
/// point-defense mount on the nose face, where the hull cannot mask its
/// forward arc, a second mount on the dorsal spine, a torpedo bay under the
/// nose, a cargo intake on the aft deck and two bell drives.
pub(super) fn patrol_picket() -> BlockShip {
    BlockShip {
        cells: union(vec![
            block(IVec3::new(-1, 0, -2), IVec3::new(3, 1, 6)),
            block(IVec3::new(0, 1, -2), IVec3::new(1, 1, 2)),
            vec![IVec3::new(0, 0, -3)],
        ]),
        specials: vec![
            cell_part(
                BLOCK_BRIDGE_SECTION_ID,
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(0, 1, -1),
            ),
            // Rotate the mount's -Y base onto the nose's -Z face. The
            // half-cell mount then centres just outside it.
            part(
                PICKET_GUN_SECTION_ID,
                PDC_KINETIC_TURRET_SECTION_ID,
                Vec3::new(0.0, 0.0, -3.75),
                Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
            ),
            turret("pdc_dorsal", IVec3::new(0, 2, -2)),
            // Hung under the nose by its top sockets: a bay beside the nose
            // would mask the nose mount's arc on that flank.
            part(
                "torpedo_keel",
                TORPEDO_SECTION_ID,
                Vec3::new(0.0, -1.0, -2.5),
                Quat::IDENTITY,
            ),
            door_up_intake("cargo_intake", Vec3::new(0.0, 1.0, 1.5)),
            cell_part(
                DRIVE_PORT_SECTION_ID,
                BASIC_THRUSTER_SECTION_ID,
                IVec3::new(-1, 0, 3),
            ),
            cell_part(
                DRIVE_STARBOARD_SECTION_ID,
                BASIC_THRUSTER_SECTION_ID,
                IVec3::new(1, 0, 3),
            ),
            flank_collar(
                BLOCK_PORT_COLLAR_SECTION_ID,
                FlankType::Port,
                IVec3::new(-2, 0, 0),
            ),
            flank_collar(
                BLOCK_STARBOARD_COLLAR_SECTION_ID,
                FlankType::Starboard,
                IVec3::new(2, 0, 0),
            ),
        ],
        plate: LIGHT_HULL_SECTION_ID,
        zones: vec![],
        style: ARMOURED_STYLE_ID,
    }
}

/// The tutorial range's hull, flown by the trainer and the drones: a slim
/// dart of light plate with one point-defense mount on the nose face, where
/// the hull cannot mask its forward arc, and a dorsal spine over two bell
/// drives. Scenario-only, so it carries no cargo intake.
pub(super) fn training_picket() -> BlockShip {
    BlockShip {
        cells: union(vec![
            block(IVec3::new(-1, 0, -2), IVec3::new(3, 1, 5)),
            block(IVec3::new(0, 1, -1), IVec3::new(1, 1, 3)),
            vec![IVec3::new(0, 0, -3)],
        ]),
        specials: vec![
            cell_part(
                BLOCK_BRIDGE_SECTION_ID,
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(0, 1, -1),
            ),
            // Rotate the mount's -Y base onto the nose's -Z face. The
            // half-cell mount then centres just outside it.
            part(
                PICKET_GUN_SECTION_ID,
                PDC_KINETIC_TURRET_SECTION_ID,
                Vec3::new(0.0, 0.0, -3.75),
                Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
            ),
            cell_part(
                DRIVE_PORT_SECTION_ID,
                BASIC_THRUSTER_SECTION_ID,
                IVec3::new(-1, 0, 2),
            ),
            cell_part(
                DRIVE_STARBOARD_SECTION_ID,
                BASIC_THRUSTER_SECTION_ID,
                IVec3::new(1, 0, 2),
            ),
            flank_collar(
                BLOCK_PORT_COLLAR_SECTION_ID,
                FlankType::Port,
                IVec3::new(-2, 0, 0),
            ),
        ],
        plate: LIGHT_HULL_SECTION_ID,
        zones: vec![],
        style: ARMOURED_STYLE_ID,
    }
}

/// The Line Warship class, the military goal: a long armoured body over a
/// keel, two shoulders carrying a torpedo bay forward and a bell drive aft,
/// a railgun on the bow, six point-defense mounts over both hemispheres, a
/// dorsal bridge tower, a cargo intake on the aft deck, a docking collar off
/// each shoulder and one vectoring drive on the transom.
pub(super) fn line_warship() -> BlockShip {
    BlockShip {
        cells: union(vec![
            block(IVec3::new(-1, 0, -6), IVec3::new(3, 2, 14)),
            // A narrow nose for the lance: a full-width bow would clad the
            // torpedo bays' lanes shut.
            block(IVec3::new(0, 0, -7), IVec3::new(1, 2, 1)),
            // The keel squares the transom off to the drive's 3x3 face and
            // gives the ventral mounts a plate to hang from.
            block(IVec3::new(-1, -1, -1), IVec3::new(3, 1, 9)),
            block(IVec3::new(-2, 0, -4), IVec3::new(1, 1, 12)),
            block(IVec3::new(2, 0, -4), IVec3::new(1, 1, 12)),
            block(IVec3::new(0, 2, -3), IVec3::new(1, 1, 3)),
        ]),
        specials: vec![
            cell_part(
                BLOCK_BRIDGE_SECTION_ID,
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(0, 2, -2),
            ),
            cell_part(
                "control_fore",
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(0, 0, -5),
            ),
            cell_part(
                "control_mid",
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(0, 0, 0),
            ),
            cell_part(
                CONTROL_AFT_SECTION_ID,
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(0, 0, 5),
            ),
            part(
                MAIN_DRIVE_SECTION_ID,
                VECTOR_THRUSTER_SECTION_ID,
                Vec3::new(0.0, 0.0, 8.5),
                Quat::IDENTITY,
            ),
            cell_part(
                DRIVE_PORT_SECTION_ID,
                BASIC_THRUSTER_SECTION_ID,
                IVec3::new(-2, 0, 7),
            ),
            cell_part(
                DRIVE_STARBOARD_SECTION_ID,
                BASIC_THRUSTER_SECTION_ID,
                IVec3::new(2, 0, 7),
            ),
            part(
                "railgun_spinal",
                RAILGUN_LANCE_SECTION_ID,
                Vec3::new(0.0, 0.0, -9.0),
                Quat::IDENTITY,
            ),
            part(
                "torpedo_port",
                TORPEDO_SECTION_ID,
                Vec3::new(-2.0, 0.0, -5.5),
                Quat::IDENTITY,
            ),
            part(
                "torpedo_starboard",
                TORPEDO_SECTION_ID,
                Vec3::new(2.0, 0.0, -5.5),
                Quat::IDENTITY,
            ),
            turret(LINE_WARSHIP_PDC_IDS[0], IVec3::new(-1, 2, -5)),
            turret(LINE_WARSHIP_PDC_IDS[1], IVec3::new(1, 2, -5)),
            turret(LINE_WARSHIP_PDC_IDS[2], IVec3::new(-1, 2, 3)),
            turret(LINE_WARSHIP_PDC_IDS[3], IVec3::new(1, 2, 3)),
            under_turret(LINE_WARSHIP_PDC_IDS[4], IVec3::new(-1, -2, 2)),
            under_turret(LINE_WARSHIP_PDC_IDS[5], IVec3::new(1, -2, 2)),
            flank_collar(
                BLOCK_PORT_COLLAR_SECTION_ID,
                FlankType::Port,
                IVec3::new(-3, 0, 0),
            ),
            flank_collar(
                BLOCK_STARBOARD_COLLAR_SECTION_ID,
                FlankType::Starboard,
                IVec3::new(3, 0, 0),
            ),
            door_up_intake("cargo_intake", Vec3::new(0.0, 2.0, 5.5)),
        ],
        plate: REINFORCED_HULL_SECTION_ID,
        zones: vec![],
        style: ARMOURED_STYLE_ID,
    }
}

/// Loose plating: the small piece a debris field is mostly built from.
pub(super) fn debris_plate() -> BlockShip {
    wreck(union(vec![
        block(IVec3::new(-1, 0, -1), IVec3::new(3, 1, 3)),
        vec![IVec3::new(2, 0, 0), IVec3::new(0, 1, 1)],
    ]))
}

/// The waystation: a two-deck plated block with the bridge set into the top
/// deck, a second flight computer in the far corner of the lower deck and a
/// docking collar standing off the port flank. No drive: a station is
/// parked, and its computers only hold the attitude it was placed at.
pub(super) fn waystation() -> BlockShip {
    BlockShip {
        cells: block(IVec3::new(-1, 0, -1), IVec3::new(3, 2, 3)),
        specials: vec![
            cell_part(
                BLOCK_BRIDGE_SECTION_ID,
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(0, 1, 0),
            ),
            cell_part(
                WAYSTATION_LOWER_BRIDGE_SECTION_ID,
                BASIC_CONTROLLER_SECTION_ID,
                IVec3::new(1, 0, 1),
            ),
            flank_collar(
                BLOCK_PORT_COLLAR_SECTION_ID,
                FlankType::Port,
                IVec3::new(-2, 0, 0),
            ),
        ],
        plate: REINFORCED_HULL_SECTION_ID,
        zones: vec![],
        style: INDUSTRIAL_STYLE_ID,
    }
}

/// One debris piece: industrial plating with no computer, drive or gun, so it
/// never reports itself neutralized.
fn wreck(cells: Vec<IVec3>) -> BlockShip {
    BlockShip {
        cells,
        specials: vec![],
        plate: REINFORCED_HULL_SECTION_ID,
        zones: vec![],
        style: INDUSTRIAL_STYLE_ID,
    }
}

/// A special placed at an arbitrary pose: a drive standing off a transom, a
/// gun seated on a nose face, a bay sunk into a flank.
fn part(id: &'static str, prototype: &'static str, position: Vec3, rotation: Quat) -> Special {
    Special {
        id,
        prototype,
        position,
        rotation,
    }
}

/// A turret standing in `cell`, seated down onto the top face of the hull cell
/// below it. The mount's socket is a quarter cell off its centre, so the
/// turret sits that far into its own cell rather than floating in the middle
/// of it.
fn turret(id: &'static str, cell: IVec3) -> Special {
    Special {
        id,
        prototype: PDC_KINETIC_TURRET_SECTION_ID,
        position: cell.as_vec3() + TURRET_SEAT,
        rotation: Quat::IDENTITY,
    }
}

/// The same turret hung under a hull cell, rolled over so its socket still
/// faces the plate it stands on.
fn under_turret(id: &'static str, cell: IVec3) -> Special {
    Special {
        id,
        prototype: PDC_KINETIC_TURRET_SECTION_ID,
        position: cell.as_vec3() - TURRET_SEAT,
        rotation: Quat::from_rotation_z(std::f32::consts::PI),
    }
}

/// A docking collar on a hull's `flank`, hatch outboard.
///
/// The port's one blind face is its own `-Z`, so a quarter turn about Y puts
/// the hatch on the flank's side and leaves the `+Z` socket facing the plate
/// inboard of it. `cell` is the collar's own cell, one step outboard of the
/// hull cell it mates to: a collar sunk into the skin would be a hatch that
/// opens into structure.
fn flank_collar(id: &'static str, flank: FlankType, cell: IVec3) -> Special {
    let turn = match flank {
        FlankType::Port => std::f32::consts::FRAC_PI_2,
        FlankType::Starboard => -std::f32::consts::FRAC_PI_2,
    };
    Special {
        id,
        prototype: DOCKING_PORT_SECTION_ID,
        position: cell.as_vec3(),
        rotation: Quat::from_rotation_y(turn),
    }
}

/// A special that occupies one whole cell, replacing the plate that would
/// otherwise be there.
fn cell_part(id: &'static str, prototype: &'static str, cell: IVec3) -> Special {
    Special {
        id,
        prototype,
        position: cell.as_vec3(),
        rotation: Quat::IDENTITY,
    }
}

/// A cargo intake lying on a deck with its door up: local -Z (the door) turns
/// to world +Y and local +Y to world +Z, so it covers three cells across and
/// two along at `centre`'s height. It is off-grid, so its cells are not hull.
fn door_up_intake(id: &'static str, centre: Vec3) -> Special {
    part(
        id,
        CARGO_INTAKE_SECTION_ID,
        centre,
        Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
    )
}

/// One solar panel standing in `cell`. It mates only side to side, so a
/// wing is a row of panels off a hull flank.
fn solar(id: &'static str, cell: IVec3) -> Special {
    cell_part(id, SOLAR_PANEL_SECTION_ID, cell)
}

impl BlockShip {
    /// The section list a catalog entry is built from: one plate per cell no
    /// special claimed, built from its zone's prototype or the hull plate,
    /// then the specials themselves.
    pub(super) fn sections(self) -> Vec<SpaceshipSectionConfig> {
        let plate = self.plate;
        let zoned: HashMap<IVec3, &'static str> = self
            .zones
            .iter()
            .flat_map(|(prototype, cells)| cells.iter().map(move |cell| (*cell, *prototype)))
            .collect();
        let claimed: HashSet<IVec3> = self
            .specials
            .iter()
            .filter_map(|part| {
                let rounded = part.position.round();
                part.position
                    .abs_diff_eq(rounded, 1e-5)
                    .then_some(rounded.as_ivec3())
            })
            .collect();

        let mut sections: Vec<_> = self
            .cells
            .into_iter()
            .filter(|cell| !claimed.contains(cell))
            .enumerate()
            .map(|(index, cell)| SpaceshipSectionConfig {
                id: format!("plate_{index}"),
                position: cell.as_vec3(),
                rotation: Quat::IDENTITY,
                source: SectionSource::prototype(zoned.get(&cell).copied().unwrap_or(plate)),
            })
            .collect();
        sections.extend(self.specials.iter().map(|part| SpaceshipSectionConfig {
            id: part.id.to_string(),
            position: part.position,
            rotation: part.rotation,
            source: SectionSource::prototype(part.prototype),
        }));
        sections
    }
}

/// A solid box of cells: `size` cells from `origin`, inclusive of the origin.
fn block(origin: IVec3, size: IVec3) -> Vec<IVec3> {
    (0..size.x)
        .flat_map(|x| {
            (0..size.y).flat_map(move |y| (0..size.z).map(move |z| origin + IVec3::new(x, y, z)))
        })
        .collect()
}

/// `cells` without the `hole`: the cells a ruin tore away.
fn carve(cells: Vec<IVec3>, hole: &[IVec3]) -> Vec<IVec3> {
    cells
        .into_iter()
        .filter(|cell| !hole.contains(cell))
        .collect()
}

/// Every cell of every part, each one once and in first-seen order, so a hull
/// built from overlapping boxes stays a stable section list.
fn union(parts: Vec<Vec<IVec3>>) -> Vec<IVec3> {
    let mut seen = HashSet::new();
    parts
        .into_iter()
        .flatten()
        .filter(|cell| seen.insert(*cell))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    /// Every hand-authored block hull, so a fleet-wide test cannot miss one.
    fn fleet() -> Vec<(&'static str, BlockShip)> {
        vec![
            ("mining tug", mining_tug()),
            ("salvage hauler", salvage_hauler()),
            ("light freighter", light_freighter()),
            ("freighter derelict", freighter_derelict()),
            ("interceptor", interceptor()),
            ("raider", raider()),
            ("patrol picket", patrol_picket()),
            ("line warship", line_warship()),
            ("debris plate", debris_plate()),
            ("waystation", waystation()),
        ]
    }

    /// The hulls that carry a bridge and a collar: everything but the debris.
    fn crewed() -> Vec<(&'static str, BlockShip)> {
        fleet()
            .into_iter()
            .filter(|(name, _)| *name != "debris plate")
            .collect()
    }

    /// Every id in a block hull is unique. Content addresses a section by id,
    /// so a duplicate would resolve to whichever section came first.
    #[test]
    fn every_block_ship_names_each_section_once() {
        for (name, ship) in fleet() {
            let sections = ship.sections();
            let ids: HashSet<_> = sections.iter().map(|section| section.id.as_str()).collect();
            assert_eq!(
                ids.len(),
                sections.len(),
                "'{name}' repeats a section id across {} sections",
                sections.len()
            );
        }
    }

    /// No two sections stand in the same cell: a special REPLACES the plate
    /// it lands on rather than sitting inside it.
    #[test]
    fn no_two_block_sections_share_a_cell() {
        for (name, ship) in fleet() {
            let mut seats: HashMap<IVec3, &str> = HashMap::new();
            for section in &ship.sections() {
                let rounded = section.position.round();
                if !section.position.abs_diff_eq(rounded, 1e-5) {
                    continue;
                }
                if let Some(other) = seats.insert(rounded.as_ivec3(), &section.id) {
                    panic!("'{name}' seats '{}' on top of '{other}'", section.id);
                }
            }
        }
    }

    /// A zone names cells of its own hull: a zone cell outside the hull would
    /// silently build nothing.
    #[test]
    fn every_zone_cell_is_a_hull_cell() {
        for (name, ship) in fleet() {
            let cells: HashSet<IVec3> = ship.cells.iter().copied().collect();
            let ruined = name == "freighter derelict";
            for (prototype, zone) in &ship.zones {
                for cell in zone {
                    assert!(
                        cells.contains(cell) || ruined,
                        "'{name}' zones {cell} as '{prototype}' but has no cell there"
                    );
                }
            }
        }
    }

    /// Both pickets carry the nose gun id content names.
    #[test]
    fn the_picket_carries_the_mount_content_names() {
        for picket in [patrol_picket(), training_picket()] {
            assert!(
                picket
                    .sections()
                    .iter()
                    .any(|section| section.id == PICKET_GUN_SECTION_ID),
                "a picket has no '{PICKET_GUN_SECTION_ID}' mount"
            );
        }
    }

    /// The line warship carries exactly the turrets content walks by name.
    #[test]
    fn the_line_warship_carries_every_turret_content_addresses() {
        let sections = line_warship().sections();
        for turret in LINE_WARSHIP_PDC_IDS {
            assert!(
                sections.iter().any(|section| section.id == turret
                    && section.source.prototype_id() == PDC_KINETIC_TURRET_SECTION_ID),
                "the line warship is missing turret '{turret}'"
            );
        }
    }

    /// Every collar stands outboard of a hull cell and opens away from it
    /// with nothing in front of the hatch. The link graph accepts a reversed
    /// hatch, so this checks the pose.
    #[test]
    fn every_collar_opens_outboard_from_a_hull_cell() {
        for (name, ship) in crewed() {
            let cells: HashSet<IVec3> = ship.cells.iter().copied().collect();
            let sections = ship.sections();
            let collars: Vec<_> = sections
                .iter()
                .filter(|section| section.source.prototype_id() == DOCKING_PORT_SECTION_ID)
                .collect();
            assert!(
                collars
                    .iter()
                    .any(|collar| collar.id == BLOCK_PORT_COLLAR_SECTION_ID),
                "'{name}' has no '{BLOCK_PORT_COLLAR_SECTION_ID}'"
            );
            for collar in collars {
                let hatch = (collar.rotation * Vec3::NEG_Z).round().as_ivec3();
                let cell = collar.position.round().as_ivec3();
                assert!(
                    cells.contains(&(cell - hatch)),
                    "'{name}' '{}' has no hull cell inboard of it",
                    collar.id
                );
                for step in 1..=3 {
                    let ahead = (cell + hatch * step).as_vec3();
                    if let Some(blocker) = sections
                        .iter()
                        .find(|section| section.position.distance(ahead) < 1.0)
                    {
                        panic!(
                            "'{name}': '{}' stands in front of '{}'",
                            blocker.id, collar.id
                        );
                    }
                }
            }
        }
    }

    /// Debris carries plain plating only: no computer, no drive and no gun,
    /// so it never flies, shoots or reports itself neutralized.
    #[test]
    fn debris_carries_nothing_that_works() {
        for section in debris_plate().sections() {
            assert_eq!(
                section.source.prototype_id(),
                REINFORCED_HULL_SECTION_ID,
                "debris section '{}' is not plain plating",
                section.id
            );
        }
    }

    /// Every crewed block ship carries the one bridge id content addresses.
    #[test]
    fn every_block_ship_carries_a_bridge() {
        for (name, ship) in crewed() {
            assert!(
                ship.sections()
                    .iter()
                    .any(|section| section.id == BLOCK_BRIDGE_SECTION_ID),
                "'{name}' has no '{BLOCK_BRIDGE_SECTION_ID}' section"
            );
        }
    }
}
