//! Objective change feedback: the objectives panel swaps text silently, so
//! completions and new postings were easy to miss mid-flight. This module
//! diffs [`GameObjectives`] by id each time it changes and answers with one
//! UI sound per change kind - `UiSfx::ObjectiveComplete` for removals,
//! `UiSfx::ObjectiveNew` for additions, both non-positional one-shots.
//!
//! Sound is ALL it owns. Everything the player SEES about an objective is the
//! objective stack's chip at the top of the screen (`objective_stack`): it
//! posts when the objective arrives and it leaves when the objective is done.
//! This module used to draw a second cue of its own as well - a green "ghost"
//! of the finished message, fading down its own column on the right - and it
//! is gone. It repeated the line the player had already read, in a corner they
//! were not looking at, while the chime and the chip leaving said the same
//! thing at the top of the screen.
//!
//! GameObjectives is write-on-diff (nova_scenario's state_to_world), so
//! `resource_changed` here means a REAL change, not the per-frame pulse.

use bevy::prelude::*;
use nova_gameplay::prelude::*;

/// `ObjectiveFeedbackPlugin`.
pub mod prelude {
    pub use super::ObjectiveFeedbackPlugin;
}

/// Feedback tunables, a resource for the inspector and a future settings
/// screen.
#[derive(Resource, Clone, Debug, Reflect)]
#[reflect(Resource)]
pub struct ObjectiveFeedbackSettings {
    /// Seconds between a completion cue and the new-objective cue when
    /// both land in one change - the chime gets its moment before the
    /// posting blip. Pure additions stay immediate.
    pub new_cue_delay_secs: f32,
}

impl Default for ObjectiveFeedbackSettings {
    fn default() -> Self {
        Self {
            new_cue_delay_secs: 1.0,
        }
    }
}

/// The held-back new-objective cue: set when a completion and an addition
/// land in the same change, played by `play_pending_new_cue` when the
/// timer runs out. A further change while pending refreshes it (latest
/// change wins, cues never stack).
#[derive(Resource, Default)]
struct NewCueState {
    /// Some = a new-objective cue is waiting out the post-completion
    /// delay.
    pending: Option<Timer>,
}

/// UI cue volumes: legible over the engine hum, no attenuation (these are
/// panel sounds, not world sounds).
const OBJECTIVE_NEW_VOLUME: f32 = 0.30;
const OBJECTIVE_COMPLETE_VOLUME: f32 = 0.38;

/// Answers each [`GameObjectives`] change with a UI cue per completion and
/// per addition.
///
/// Registers [`ObjectiveFeedbackSettings`], inits it and the pending-cue
/// state, and runs `objective_change_feedback` and `play_pending_new_cue`
/// (chained) in Update within [`super::NovaHudSystems`].
#[derive(Default)]
pub struct ObjectiveFeedbackPlugin;

impl Plugin for ObjectiveFeedbackPlugin {
    fn build(&self, app: &mut App) {
        trace!("ObjectiveFeedbackPlugin: build");

        app.register_type::<ObjectiveFeedbackSettings>();
        app.init_resource::<ObjectiveFeedbackSettings>();
        app.init_resource::<NewCueState>();
        app.add_systems(
            Update,
            (
                objective_change_feedback.run_if(resource_changed::<GameObjectives>),
                play_pending_new_cue,
            )
                .chain()
                .in_set(super::NovaHudSystems),
        );
    }
}

/// Diff the objective ids against the previous frame's list: removals are
/// completions, additions are new postings, and each kind gets its cue.
/// The snapshot starts empty, so a scenario's opening objective plays the
/// "new" cue once on load - correct, it IS new.
fn objective_change_feedback(
    mut commands: Commands,
    objectives: Res<GameObjectives>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    settings: Res<ObjectiveFeedbackSettings>,
    mut new_cue: ResMut<NewCueState>,
    mut snapshot: Local<Vec<Objective>>,
) {
    // A transition to an EMPTY list is scenario teardown (death restart,
    // quit to menu - NovaEventWorld.clear() empties the resource), not a
    // sweep of completions: dying must not play the success chime over the
    // objectives you failed.
    // Mid-scenario the list never empties - a chapter's final handler
    // completes its last beat and posts "done" in one action list.
    if objectives.objectives.is_empty() {
        *snapshot = Vec::new();
        new_cue.pending = None;
        return;
    }

    let completed: Vec<&Objective> = snapshot
        .iter()
        .filter(|old| !objectives.objectives.iter().any(|new| new.id == old.id))
        .collect();
    // Both VISUALS are the objective stack's chip (`objective_stack`) - the
    // posting and the completion alike. This module owns the audio only.
    let added = objectives
        .objectives
        .iter()
        .any(|new| !snapshot.iter().any(|old| old.id == new.id));

    if let Some(bank) = &bank {
        // One cue per change kind per frame: a complete+re-add tally swap
        // plays both once, not per objective.
        if !completed.is_empty() {
            commands.play_sfx(
                bank.get(UiSfx::ObjectiveComplete),
                AudioRoute::Interface,
                OBJECTIVE_COMPLETE_VOLUME,
            );
            // A chime just played: restart any pending blip's clock, or a
            // completion-only change late in the window would land the
            // blip right on this chime's tail - the exact masking this
            // delay exists to prevent.
            if let Some(timer) = new_cue.pending.as_mut() {
                timer.reset();
            }
        }
        if added {
            if completed.is_empty() {
                // Nothing finished in this change: the posting blip plays
                // immediately.
                commands.play_sfx(
                    bank.get(UiSfx::ObjectiveNew),
                    AudioRoute::Interface,
                    OBJECTIVE_NEW_VOLUME,
                );
            } else {
                // The completion chime just played - hold the posting blip
                // back so the two cues do not mask each other. Latest change
                // wins if one was already pending.
                new_cue.pending = Some(Timer::from_seconds(
                    settings.new_cue_delay_secs.max(0.0),
                    TimerMode::Once,
                ));
            }
        }
    }

    *snapshot = objectives.objectives.clone();
}

/// Play the held-back new-objective cue once its post-completion delay
/// runs out (see [`NewCueState`]).
fn play_pending_new_cue(
    time: Res<Time>,
    mut commands: Commands,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut new_cue: ResMut<NewCueState>,
) {
    let Some(timer) = new_cue.pending.as_mut() else {
        return;
    };
    if !timer.tick(time.delta()).is_finished() {
        return;
    }
    new_cue.pending = None;
    if let Some(bank) = &bank {
        commands.play_sfx(
            bank.get(UiSfx::ObjectiveNew),
            AudioRoute::Interface,
            OBJECTIVE_NEW_VOLUME,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feedback_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<GameObjectives>();
        app.init_resource::<ObjectiveFeedbackSettings>();
        app.init_resource::<NewCueState>();
        app.add_systems(
            Update,
            (
                objective_change_feedback.run_if(resource_changed::<GameObjectives>),
                play_pending_new_cue,
            )
                .chain(),
        );
        app
    }

    /// Which cue a PlaySfx trigger carried, resolved by handle identity.
    #[derive(Resource, Default)]
    struct CueCounts {
        complete: usize,
        new: usize,
    }

    /// The feedback rig plus a real SoundBank and a PlaySfx capture, so
    /// tests can assert WHICH cue played and WHEN.
    fn sfx_app() -> App {
        let mut app = feedback_app();
        app.add_plugins(bevy::asset::AssetPlugin::default());
        app.init_asset::<AudioSource>();
        let bank = SoundBank::load(
            app.world().resource::<AssetServer>(),
            nova_gameplay::audio::UI_SFX_FILES,
        );
        app.insert_resource(bank);
        app.init_resource::<CueCounts>();
        app.add_observer(
            |sfx: On<PlaySfx>, bank: Res<SoundBank<UiSfx>>, mut counts: ResMut<CueCounts>| {
                if sfx.handle == bank.get(UiSfx::ObjectiveComplete) {
                    counts.complete += 1;
                } else if sfx.handle == bank.get(UiSfx::ObjectiveNew) {
                    counts.new += 1;
                }
            },
        );
        app
    }

    /// A completion and a posting in ONE change (every shakedown beat
    /// handler does exactly this): the chime plays immediately, the
    /// posting blip waits out the configured delay so the two cues do
    /// not mask each other. Delivery guards: the blip
    /// has NOT played at half the delay, and a pure posting (no
    /// completion) stays immediate.
    #[test]
    fn the_posting_blip_waits_out_the_delay_after_a_chime() {
        use core::time::Duration;

        use bevy::time::TimeUpdateStrategy;

        let mut app = sfx_app();
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            0.2,
        )));

        // A pure posting: immediate blip.
        app.world_mut().resource_mut::<GameObjectives>().objectives =
            vec![Objective::new("b1", "Burn for Beacon 1")];
        app.update();
        app.update();
        let counts = |app: &App| {
            let counts = app.world().resource::<CueCounts>();
            (counts.complete, counts.new)
        };
        assert_eq!(counts(&app), (0, 1), "a pure posting blips immediately");

        // Beat transition: complete b1, post b2 in one change.
        app.world_mut().resource_mut::<GameObjectives>().objectives =
            vec![Objective::new("b2", "Find Beacon 2")];
        app.update();
        assert_eq!(
            counts(&app),
            (1, 1),
            "the chime is immediate; the blip is held back"
        );

        // Half the 1.0s delay (2-3 ticks at 0.2s): still held.
        app.update();
        app.update();
        assert_eq!(
            counts(&app),
            (1, 1),
            "delivery guard: the blip must not fire before the delay"
        );

        // Ride out the rest of the delay: the blip lands.
        for _ in 0..5 {
            app.update();
        }
        assert_eq!(counts(&app), (1, 2), "the blip plays after the delay");
    }

    /// A completion chimes and a still-active objective does not. Delivery
    /// guard: the diff must key on REMOVAL, not on any change to the list.
    #[test]
    fn completing_an_objective_chimes_and_the_others_stay_quiet() {
        let mut app = sfx_app();

        app.world_mut().resource_mut::<GameObjectives>().objectives = vec![
            Objective::new("b1", "Burn for Beacon 1"),
            Objective::new("b2", "Find Beacon 2"),
        ];
        app.update();
        assert_eq!(
            app.world().resource::<CueCounts>().complete,
            0,
            "posting two objectives is not completing one"
        );

        // Complete b1 (remove it), keep b2.
        app.world_mut().resource_mut::<GameObjectives>().objectives =
            vec![Objective::new("b2", "Find Beacon 2")];
        app.update();
        assert_eq!(
            app.world().resource::<CueCounts>().complete,
            1,
            "the completed objective chimes once"
        );
    }

    /// The completion is the objective stack's chip and nothing else. This
    /// module drew a second one for a while - a green ghost of the finished
    /// message, fading down its own column on the right - and it is gone: it
    /// said what the chip already says, somewhere the player is not looking.
    /// Nothing here spawns UI.
    #[test]
    fn a_completion_draws_nothing_of_its_own() {
        let mut app = feedback_app();

        app.world_mut().resource_mut::<GameObjectives>().objectives =
            vec![Objective::new("b1", "Burn for Beacon 1")];
        app.update();
        app.world_mut().resource_mut::<GameObjectives>().objectives =
            vec![Objective::new("b2", "Find Beacon 2")];
        app.update();

        let mut q = app.world_mut().query::<&Text>();
        assert_eq!(
            q.iter(app.world()).count(),
            0,
            "objective feedback spawned text of its own - the chip is the only \
             visual a completion gets"
        );
    }

    /// Scenario teardown empties GameObjectives (death restart, quit to
    /// menu): that transition is a silent reset, NOT a sweep of
    /// completions - no chime for objectives the player failed. Delivery
    /// guard: a real single completion right after the reset still chimes,
    /// proving the snapshot re-armed.
    #[test]
    fn teardown_to_empty_is_a_silent_reset() {
        let mut app = sfx_app();

        app.world_mut().resource_mut::<GameObjectives>().objectives = vec![
            Objective::new("b1", "Burn for Beacon 1"),
            Objective::new("b2", "Find Beacon 2"),
        ];
        app.update();

        // Teardown: the whole list empties at once.
        app.world_mut().resource_mut::<GameObjectives>().objectives = Vec::new();
        app.update();
        assert_eq!(
            app.world().resource::<CueCounts>().complete,
            0,
            "dying must not celebrate the objectives it failed"
        );

        // The restarted run behaves normally: post one, complete it.
        app.world_mut().resource_mut::<GameObjectives>().objectives =
            vec![Objective::new("b1", "Burn for Beacon 1")];
        app.update();
        app.world_mut().resource_mut::<GameObjectives>().objectives =
            vec![Objective::new("b2", "Find Beacon 2")];
        app.update();
        assert_eq!(
            app.world().resource::<CueCounts>().complete,
            1,
            "a real completion after the reset still chimes"
        );
    }

    /// A tally swap (complete + re-add of the SAME id in one change) is
    /// not a completion: same id present before and after means no chime.
    #[test]
    fn a_message_swap_of_the_same_id_is_not_a_completion() {
        let mut app = sfx_app();

        app.world_mut().resource_mut::<GameObjectives>().objectives =
            vec![Objective::new("b3", "Crates: 0/3")];
        app.update();
        app.world_mut().resource_mut::<GameObjectives>().objectives =
            vec![Objective::new("b3", "Crates: 1/3")];
        app.update();

        assert_eq!(
            app.world().resource::<CueCounts>().complete,
            0,
            "same-id message swaps are progress, not completion"
        );
        // The same-id swap is not a fresh posting either way; the chip it
        // re-posts is `objective_stack`'s business (and its own tests').
    }
}
