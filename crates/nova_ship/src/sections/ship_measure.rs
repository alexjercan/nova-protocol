//! What a ship's sections add up to: mass, drive thrust and flight computers,
//! and the two floors a built ship must clear.
//!
//! ONE copy, two callers: the open-world generator grows a hull's drive bank
//! and flight computers until they clear the floors and checks every layout
//! against them, and a section swap or add at a station is refused when it
//! would drop a ship under a floor it cleared (`refit_build_fault`).

use crate::sections::base_section::prelude::{SectionCollider, SectionConfig, SectionKind};

/// The measure, its floors and the shortfall a ship can show.
pub mod prelude {
    pub use super::{
        section_mass, ShipMeasure, ShipMeasureShortfallType, CELLS_PER_CONTROLLER,
        MIN_THRUST_PER_CELL,
    };
}

/// The least drive thrust a ship carries per cell of its mass.
pub const MIN_THRUST_PER_CELL: f32 = 0.06;

/// The most cells of mass one flight computer turns.
pub const CELLS_PER_CONTROLLER: f32 = 50.0;

/// A section's mass: its collider's volume, at the density 1 every section
/// spawns with.
pub fn section_mass(config: &SectionConfig) -> f32 {
    use std::f32::consts::PI;
    match config.base.collider.unwrap_or_default() {
        SectionCollider::Cuboid { size } => size.x * size.y * size.z,
        SectionCollider::Sphere { radius } => 4.0 / 3.0 * PI * radius.powi(3),
        SectionCollider::Capsule { radius, length } => {
            PI * radius * radius * (length + 4.0 / 3.0 * radius)
        }
        SectionCollider::Cylinder { radius, height } => PI * radius * radius * height,
    }
}

/// What a ship's sections add up to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShipMeasure {
    /// Mass, in cells.
    pub mass: f32,
    /// Summed drive thrust.
    pub thrust: f32,
    /// Flight computers.
    pub controllers: usize,
}

/// The floor a ship falls short of, see [`ShipMeasure::shortfall`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ShipMeasureShortfallType {
    /// The drives push less than the ship's mass needs.
    Underpowered {
        /// The drives' summed thrust.
        thrust: f32,
        /// The thrust the ship's mass needs.
        needed: f32,
    },
    /// The ship carries fewer flight computers than its mass needs.
    Undercontrolled {
        /// The flight computers it carries.
        controllers: usize,
        /// How many its mass needs.
        needed: usize,
    },
}

impl ShipMeasure {
    /// The measure of a ship made of `configs`.
    pub fn of<'c>(configs: impl IntoIterator<Item = &'c SectionConfig>) -> Self {
        let mut measure = Self {
            mass: 0.0,
            thrust: 0.0,
            controllers: 0,
        };
        for config in configs {
            measure.mass += section_mass(config);
            match &config.kind {
                SectionKind::Thruster(thruster) => measure.thrust += thruster.magnitude,
                SectionKind::Controller(_) => measure.controllers += 1,
                _ => {}
            }
        }
        measure
    }

    /// The thrust the ship's mass needs: [`MIN_THRUST_PER_CELL`] per cell.
    pub fn needed_thrust(self) -> f32 {
        MIN_THRUST_PER_CELL * self.mass
    }

    /// The flight computers the ship's mass needs: one per
    /// [`CELLS_PER_CONTROLLER`] cells, at least one.
    pub fn needed_controllers(self) -> usize {
        ((self.mass / CELLS_PER_CONTROLLER).ceil() as usize).max(1)
    }

    /// The first floor the ship falls short of: thrust, then flight
    /// computers.
    pub fn shortfall(self) -> Option<ShipMeasureShortfallType> {
        if self.thrust < self.needed_thrust() {
            return Some(ShipMeasureShortfallType::Underpowered {
                thrust: self.thrust,
                needed: self.needed_thrust(),
            });
        }
        (self.controllers < self.needed_controllers()).then(|| {
            ShipMeasureShortfallType::Undercontrolled {
                controllers: self.controllers,
                needed: self.needed_controllers(),
            }
        })
    }
}
