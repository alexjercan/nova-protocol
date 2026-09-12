//! The load gate: what holds the world still while a scenario is being built,
//! and what happens when the build fails.
//!
//! A scenario used to start the moment its config landed, with its objects
//! arriving over the following frames and its art arriving after that. The
//! player got the helm on a frame where half the scene did not exist yet: rocks
//! popped in around a ship already flying, the clock had already run, and a
//! collision could happen against a body that had not finished spawning.
//!
//! Loading is ATOMIC instead. From the frame a scenario loads until the frame
//! its queued spawns and its required glTF have all settled, the simulation is
//! held ([`FreezeOwner::ScenarioLoad`]) and gameplay input and camera control
//! are gated off. Real time keeps running, so the loading panel animates; the
//! world does not, so nothing behind the panel is a frame the player missed.
//! The release happens on the same frame the panel comes down, and the first
//! frame the player is given is a complete scene at scenario time zero.
//!
//! A load that FAILS never releases. See [`ScenarioLoadGate::Failed`].

use bevy::prelude::*;
use nova_gameplay::prelude::*;

use super::{ScenarioLoaded, UnloadScenario};
use crate::prelude::*;

/// The load gate and its run conditions.
pub mod prelude {
    pub use super::{scenario_is_loading, scenario_play_is_free, ScenarioLoadGate};
}

/// Whether a scenario is still being built, and whether that build failed.
///
/// The gate is what the simulation hold and the input gating both read, so
/// there is one answer to "may the player act" rather than one per surface.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScenarioLoadGate {
    /// Nothing is loading. The scenario, if any, is live and playable.
    #[default]
    Idle,
    /// A scenario is being built: spawns are landing, art is arriving, and the
    /// world is held still behind the loading panel.
    Loading,
    /// The build failed and the run is over. The hold is NEVER released from
    /// here: the alternative is a scene with pieces missing that the player can
    /// fly around in, which is the pop-in this gate exists to remove wearing a
    /// worse face. `FAILED TO START` is on screen and Main Menu is the only way
    /// out, which clears the gate by leaving `GameStates::Playing`.
    Failed,
}

impl ScenarioLoadGate {
    /// Whether the world is held for a load - building or failed.
    pub fn is_held(self) -> bool {
        !matches!(self, Self::Idle)
    }
}

/// Run condition: a scenario is being built or has failed to build.
pub fn scenario_is_loading(gate: Option<Res<ScenarioLoadGate>>) -> bool {
    gate.is_some_and(|gate| gate.is_held())
}

/// Run condition: the player may act on the scenario.
///
/// Optional, so a rig that registers an input or camera set without the loader
/// plugin that owns the gate still runs - an absent gate is an unheld one.
pub fn scenario_play_is_free(gate: Option<Res<ScenarioLoadGate>>) -> bool {
    !scenario_is_loading(gate)
}

/// Take the hold the moment a scenario loads.
///
/// On [`ScenarioLoaded`] rather than `LoadScenario`, exactly as the preload
/// warm-up is: the loader writes `CurrentScenario` and only then triggers this,
/// so a load the content gate REFUSED never takes a hold nothing would release.
pub(super) fn hold_for_scenario_load(
    _: On<ScenarioLoaded>,
    mut gate: ResMut<ScenarioLoadGate>,
    mut clocks: Clocks,
) {
    debug!("hold_for_scenario_load: holding the world while the scenario builds");
    *gate = ScenarioLoadGate::Loading;
    clocks.hold(FreezeOwner::ScenarioLoad);
}

/// Give the world back on the frame the scenario finishes building.
///
/// The release and the loading panel's dismissal read the SAME two conditions
/// (`EventWorld::is_settling` and `ScenarioPreload::is_pending`, which is what
/// [`scenario_has_settled`] is), so the panel cannot come down over a held
/// world and the world cannot start behind a panel.
///
/// The clocks resume here, which means the frame this runs on is the last
/// frozen one: the scenario's first simulated frame is the NEXT one, and its
/// clock starts from zero because nothing advanced it while the hold was on.
pub(super) fn release_when_scenario_is_built(
    settled: In<bool>,
    mut gate: ResMut<ScenarioLoadGate>,
    mut clocks: Clocks,
) {
    if *gate != ScenarioLoadGate::Loading || !*settled {
        return;
    }
    debug!("release_when_scenario_is_built: the scenario is built; the world runs");
    *gate = ScenarioLoadGate::Idle;
    clocks.release(FreezeOwner::ScenarioLoad);
}

/// Fail the load closed: keep the hold, and report through the same
/// `FAILED TO START` overlay the runtime content gate uses.
///
/// Called by the preload tracker when an asset explicitly failed or the load
/// stopped making progress. Nothing continues with placeholder art: a scene
/// missing the art it asked for is a scene the player would be asked to fly
/// through and report as broken.
pub(super) fn fail_scenario_load(
    gate: &mut ScenarioLoadGate,
    failure: Option<&mut ScenarioStartFailure>,
    scenario_name: String,
    messages: Vec<String>,
) {
    if *gate == ScenarioLoadGate::Failed {
        return;
    }
    error!(
        "fail_scenario_load: '{scenario_name}' cannot start ({} asset problem(s)):",
        messages.len()
    );
    for message in &messages {
        error!("  {message}");
    }
    *gate = ScenarioLoadGate::Failed;
    if let Some(failure) = failure {
        failure.0 = Some(ScenarioStartFailureReport {
            scenario_name,
            messages,
        });
    }
}

/// Drop the hold with the scenario that took it.
///
/// Unload is how a failed run is left (Main Menu tears the scenario down), so
/// this is the one path out of [`ScenarioLoadGate::Failed`]. A menu that
/// inherited a held clock would be a menu whose backdrop never moves.
pub(super) fn release_on_unload(
    _: On<UnloadScenario>,
    mut gate: ResMut<ScenarioLoadGate>,
    mut clocks: Clocks,
) {
    if *gate == ScenarioLoadGate::Idle {
        return;
    }
    trace!("release_on_unload: dropping the scenario-load hold");
    *gate = ScenarioLoadGate::Idle;
    clocks.release(FreezeOwner::ScenarioLoad);
}

/// Register the gate, its hold and its release.
pub(super) fn register_scenario_load_gate(app: &mut App) {
    app.init_resource::<ScenarioLoadGate>();
    // The hold ledger belongs to `nova_gameplay`, which a scenario rig need
    // not carry: an example or an editor sandbox registers the loader on its
    // own. Idempotent, so the app that DOES carry it keeps one ledger.
    app.init_resource::<ClockFreeze>();
    app.add_observer(hold_for_scenario_load);
    app.add_observer(release_on_unload);
    // Piped, so the release reads the SAME settle condition the loading panel
    // does rather than a second copy of it that could drift.
    app.add_systems(
        Update,
        scenario_has_settled.pipe(release_when_scenario_is_built),
    );
}

#[cfg(test)]
mod tests;
