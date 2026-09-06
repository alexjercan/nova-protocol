//! The Nova Protocol campaign's chapter and the story vocabulary around it.

mod cast;
mod first_shift;
mod pacing;
mod stage;

pub(crate) use first_shift::{first_shift, FIRST_SHIFT_SCENARIO_ID};
pub use first_shift::{first_shift_scene, FirstShiftScene};

pub(crate) use super::SCENARIO_ELAPSED_VAR;
pub(crate) use crate::base_content::{assets::CampaignPortraits, ships};

#[cfg(test)]
mod tests;
