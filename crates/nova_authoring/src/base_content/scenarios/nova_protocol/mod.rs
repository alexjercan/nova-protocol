//! The Nova Protocol campaign chapters and their shared story vocabulary.

mod broadside;
mod cast;
mod final_tally;
mod lifeline;
mod pacing;
mod shakedown;

pub(crate) use broadside::{
    broadside, broadside_gunship, BROADSIDE_GUNSHIP_SCENARIO_ID, BROADSIDE_SCENARIO_ID,
};
pub(crate) use final_tally::{final_tally, FINAL_TALLY_SCENARIO_ID};
pub(crate) use lifeline::{lifeline, LIFELINE_SCENARIO_ID};
pub(crate) use shakedown::{shakedown_run as shakedown, SHAKEDOWN_SCENARIO_ID};

pub(crate) use super::{SCATTER_SEED, SCENARIO_ELAPSED_VAR};
pub(crate) use crate::base_content::ships;

#[cfg(test)]
mod tests;
