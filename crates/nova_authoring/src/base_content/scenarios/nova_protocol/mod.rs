//! The Nova Protocol campaign chapters and their shared story vocabulary.

mod cast;
mod first_shift;
mod pacing;
mod second_shift;
mod stage;

pub(crate) use first_shift::{first_shift, FIRST_SHIFT_SCENARIO_ID};
pub use first_shift::{first_shift_scene, FirstShiftScene};
pub(crate) use second_shift::{second_shift, SECOND_SHIFT_SCENARIO_ID};

pub(crate) use super::SCENARIO_ELAPSED_VAR;
pub(crate) use crate::base_content::ships;

#[cfg(test)]
mod tests;
