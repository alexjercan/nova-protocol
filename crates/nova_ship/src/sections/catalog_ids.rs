//! [`GameSections`] keys for the base-game section prototypes that Rust code
//! outside the authoring crate has to name.
//!
//! Add an id here ONLY when a crate that cannot reach `nova_authoring` must
//! name it; everything else stays a `const` beside the builder that authors it.

/// The base-game prototype ids engine-side code names.
pub mod prelude {
    pub use super::{
        BASIC_CONTROLLER_SECTION_ID, BASIC_THRUSTER_SECTION_ID, LIGHT_HULL_SECTION_ID,
        PDC_KINETIC_TURRET_SECTION_ID, RAILGUN_LANCE_SECTION_ID, REINFORCED_HULL_SECTION_ID,
    };
}

/// The generic flight computer: the part that makes a hull flyable.
pub const BASIC_CONTROLLER_SECTION_ID: &str = "basic_controller_section";
/// The generic drive.
pub const BASIC_THRUSTER_SECTION_ID: &str = "basic_thruster_section";
/// The thin-walled hull plate; scavenger grade.
pub const LIGHT_HULL_SECTION_ID: &str = "light_hull_section";
/// The one turret every shipped craft mounts.
pub const PDC_KINETIC_TURRET_SECTION_ID: &str = "pdc_kinetic_turret_section";
/// The spinal lance: the one gun the HULL aims, named here so the editor
/// sandbox can put one on the ship it hands a builder.
pub const RAILGUN_LANCE_SECTION_ID: &str = "railgun_lance_section";
/// The standard hull plate, and the seed the editor starts a new ship from.

pub const REINFORCED_HULL_SECTION_ID: &str = "reinforced_hull_section";
