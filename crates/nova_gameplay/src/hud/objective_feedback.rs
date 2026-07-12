//! Objective change feedback (task 20260712-125342, playtest round 3
//! finding 2): the objectives panel swaps text silently, so completions
//! and new postings were easy to miss mid-flight. This module diffs
//! [`GameObjectives`] by id each time it changes and answers with:
//!
//! - a UI sound per change (NovaSfx::ObjectiveComplete for removals,
//!   NovaSfx::ObjectiveNew for additions; non-positional one-shots), and
//! - a transient "ghost" line for each completed objective: the finished
//!   message in done-green, fading out over a couple of seconds. The
//!   ghost is NOT a child of the bcs panel - rebuild_lines replaces the
//!   panel's whole child set on every change and would despawn a ghost
//!   mid-fade - so ghosts stack in their own absolute node beside it.
//!
//! GameObjectives is write-on-diff (nova_scenario's state_to_world since
//! review R1.1 of 20260711-180506), so `resource_changed` here means a
//! REAL change, not the per-frame pulse.

use bevy::prelude::*;

use super::{HudTier, OBJECTIVES_PANEL_WIDTH_PX};
use crate::prelude::*;

pub mod prelude {
    pub use super::{ObjectiveFeedbackPlugin, ObjectiveGhostLineMarker, ObjectiveGhostsHudMarker};
}

/// Feedback tunables, a resource for the inspector and a future settings
/// screen (playtest round 4: "can be configured maybe").
#[derive(Resource, Clone, Debug, Reflect)]
#[reflect(Resource)]
pub struct ObjectiveFeedbackSettings {
    /// Seconds between a completion cue and the new-objective cue when
    /// both land in one change - the chime gets its moment before the
    /// posting blip (playtest round 4). Pure additions stay immediate.
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

/// How long a completed objective's ghost line lingers (seconds), fading
/// linearly to zero alpha.
const GHOST_FADE_SECS: f32 = 2.5;

/// Done-green for the ghost line text.
const GHOST_COLOR: Color = Color::srgba(0.4, 0.95, 0.5, 1.0);

const GHOST_FONT_PX: f32 = 13.0;

/// Marker for the ghost stack container (one, spawned with the plugin).
#[derive(Component, Debug, Clone, Reflect)]
pub struct ObjectiveGhostsHudMarker;

/// One fading completed-objective line; `age` drives the fade.
#[derive(Component, Debug, Clone, Reflect)]
pub struct ObjectiveGhostLineMarker {
    pub age: f32,
}

#[derive(Default)]
pub struct ObjectiveFeedbackPlugin;

impl Plugin for ObjectiveFeedbackPlugin {
    fn build(&self, app: &mut App) {
        debug!("ObjectiveFeedbackPlugin: build");

        app.register_type::<ObjectiveGhostLineMarker>();
        app.register_type::<ObjectiveFeedbackSettings>();
        app.init_resource::<ObjectiveFeedbackSettings>();
        app.init_resource::<NewCueState>();
        app.add_systems(Startup, spawn_ghost_stack);
        app.add_systems(
            Update,
            (
                objective_change_feedback.run_if(resource_changed::<GameObjectives>),
                play_pending_new_cue,
                fade_ghost_lines,
            )
                .chain()
                .in_set(super::NovaHudSystems),
        );
    }
}

/// The ghost stack: an absolute column just below the objectives panel's
/// anchor, independent of the bcs panel entity (whose children are
/// replaced wholesale on every rebuild).
fn spawn_ghost_stack(mut commands: Commands) {
    commands.spawn((
        Name::new("ObjectiveGhostsHUD"),
        ObjectiveGhostsHudMarker,
        HudTier::Chrome,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(58.0),
            right: Val::Px(8.0),
            width: Val::Px(OBJECTIVES_PANEL_WIDTH_PX),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(2.0),
            ..default()
        },
    ));
}

/// Diff the objective ids against the previous frame's list: removals are
/// completions (sound + ghost line), additions are new postings (sound).
/// The snapshot starts empty, so a scenario's opening objective plays the
/// "new" cue once on load - correct, it IS new.
fn objective_change_feedback(
    mut commands: Commands,
    objectives: Res<GameObjectives>,
    bank: Option<Res<SoundBank<NovaSfx>>>,
    settings: Res<ObjectiveFeedbackSettings>,
    mut new_cue: ResMut<NewCueState>,
    q_stack: Query<Entity, With<ObjectiveGhostsHudMarker>>,
    mut snapshot: Local<Vec<Objective>>,
) {
    // A transition to an EMPTY list is scenario teardown (death restart,
    // quit to menu - NovaEventWorld.clear() empties the resource), not a
    // sweep of completions: dying must not play the success chime over
    // green ghosts of the objectives you failed (review R1.1 MAJOR).
    // Mid-scenario the list never empties - shakedown's final handler
    // completes b5 and posts "done" in one action list.
    if objectives.objectives.is_empty() {
        *snapshot = Vec::new();
        new_cue.pending = None;
        return;
    }

    let completed: Vec<&Objective> = snapshot
        .iter()
        .filter(|old| !objectives.objectives.iter().any(|new| new.id == old.id))
        .collect();
    let added = objectives
        .objectives
        .iter()
        .any(|new| !snapshot.iter().any(|old| old.id == new.id));

    if let Some(bank) = &bank {
        // One cue per change kind per frame: a complete+re-add tally swap
        // plays both once, not per objective.
        if !completed.is_empty() {
            commands.play_sfx_volume(
                bank.get(NovaSfx::ObjectiveComplete),
                OBJECTIVE_COMPLETE_VOLUME,
            );
            // A chime just played: restart any pending blip's clock, or a
            // completion-only change late in the window would land the
            // blip right on this chime's tail - the exact masking this
            // delay exists to prevent (review R1.2).
            if let Some(timer) = new_cue.pending.as_mut() {
                timer.reset();
            }
        }
        if added {
            if completed.is_empty() {
                // Nothing finished in this change: the posting blip plays
                // immediately.
                commands.play_sfx_volume(bank.get(NovaSfx::ObjectiveNew), OBJECTIVE_NEW_VOLUME);
            } else {
                // The completion chime just played - hold the posting blip
                // back so the two cues do not mask each other (playtest
                // round 4). Latest change wins if one was already pending.
                new_cue.pending = Some(Timer::from_seconds(
                    settings.new_cue_delay_secs.max(0.0),
                    TimerMode::Once,
                ));
            }
        }
    }

    if let Ok(stack) = q_stack.single() {
        for objective in &completed {
            commands.entity(stack).with_children(|parent| {
                parent.spawn((
                    Name::new(format!("ObjectiveGhost {}", objective.id)),
                    ObjectiveGhostLineMarker { age: 0.0 },
                    Text::new(objective.message.clone()),
                    TextFont::from_font_size(GHOST_FONT_PX),
                    TextLayout {
                        justify: Justify::Left,
                        linebreak: LineBreak::WordBoundary,
                    },
                    TextColor(GHOST_COLOR),
                    Pickable::IGNORE,
                ));
            });
        }
    }

    *snapshot = objectives.objectives.clone();
}

/// Play the held-back new-objective cue once its post-completion delay
/// runs out (see [`NewCueState`]).
fn play_pending_new_cue(
    time: Res<Time>,
    mut commands: Commands,
    bank: Option<Res<SoundBank<NovaSfx>>>,
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
        commands.play_sfx_volume(bank.get(NovaSfx::ObjectiveNew), OBJECTIVE_NEW_VOLUME);
    }
}

/// Fade each ghost line's alpha with age and despawn it when spent.
fn fade_ghost_lines(
    time: Res<Time>,
    mut commands: Commands,
    mut q_ghosts: Query<(Entity, &mut ObjectiveGhostLineMarker, &mut TextColor)>,
) {
    for (ghost, mut marker, mut color) in &mut q_ghosts {
        marker.age += time.delta_secs();
        if marker.age >= GHOST_FADE_SECS {
            commands.entity(ghost).try_despawn();
            continue;
        }
        let alpha = 1.0 - marker.age / GHOST_FADE_SECS;
        color.0 = GHOST_COLOR.with_alpha(alpha);
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
        app.add_systems(Startup, spawn_ghost_stack);
        app.add_systems(
            Update,
            (
                objective_change_feedback.run_if(resource_changed::<GameObjectives>),
                play_pending_new_cue,
                fade_ghost_lines,
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
            crate::audio::NOVA_SFX_FILES,
        );
        app.insert_resource(bank);
        app.init_resource::<CueCounts>();
        app.add_observer(
            |sfx: On<PlaySfx>, bank: Res<SoundBank<NovaSfx>>, mut counts: ResMut<CueCounts>| {
                if sfx.handle == bank.get(NovaSfx::ObjectiveComplete) {
                    counts.complete += 1;
                } else if sfx.handle == bank.get(NovaSfx::ObjectiveNew) {
                    counts.new += 1;
                }
            },
        );
        app
    }

    /// A completion and a posting in ONE change (every shakedown beat
    /// handler does exactly this): the chime plays immediately, the
    /// posting blip waits out the configured delay so the two cues do
    /// not mask each other (playtest round 4). Delivery guards: the blip
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

    /// A completed objective leaves a fading ghost of its message; the
    /// fade despawns it. The still-active objective leaves no ghost
    /// (delivery guard: the diff must key on REMOVAL, not any change).
    #[test]
    fn completing_an_objective_spawns_a_fading_ghost() {
        use core::time::Duration;

        use bevy::time::TimeUpdateStrategy;

        let mut app = feedback_app();
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            0.2,
        )));

        app.world_mut().resource_mut::<GameObjectives>().objectives = vec![
            Objective::new("b1", "Burn for Beacon 1"),
            Objective::new("b2", "Find Beacon 2"),
        ];
        app.update();
        app.update();

        let ghost_count = |app: &mut App| -> usize {
            let mut q = app
                .world_mut()
                .query_filtered::<(), With<ObjectiveGhostLineMarker>>();
            q.iter(app.world()).count()
        };
        assert_eq!(ghost_count(&mut app), 0, "no completions yet, no ghosts");

        // Complete b1 (remove it), keep b2.
        app.world_mut().resource_mut::<GameObjectives>().objectives =
            vec![Objective::new("b2", "Find Beacon 2")];
        app.update();

        assert_eq!(ghost_count(&mut app), 1, "the completed objective ghosts");
        let mut q = app
            .world_mut()
            .query_filtered::<&Text, With<ObjectiveGhostLineMarker>>();
        let text = q.single(app.world()).unwrap();
        assert_eq!(
            text.0, "Burn for Beacon 1",
            "the ghost shows the DONE message"
        );

        // Ride out the fade: the ghost despawns.
        for _ in 0..20 {
            app.update();
        }
        assert_eq!(ghost_count(&mut app), 0, "the ghost fades out and despawns");
    }

    /// Scenario teardown empties GameObjectives (death restart, quit to
    /// menu): that transition is a silent reset, NOT a sweep of
    /// completions - no ghosts (and no chime) for objectives the player
    /// failed. Delivery guard: a real single completion right after the
    /// reset still ghosts, proving the snapshot re-armed.
    #[test]
    fn teardown_to_empty_is_a_silent_reset() {
        let mut app = feedback_app();

        app.world_mut().resource_mut::<GameObjectives>().objectives = vec![
            Objective::new("b1", "Burn for Beacon 1"),
            Objective::new("b2", "Find Beacon 2"),
        ];
        app.update();

        // Teardown: the whole list empties at once.
        app.world_mut().resource_mut::<GameObjectives>().objectives = Vec::new();
        app.update();

        let ghost_count = |app: &mut App| -> usize {
            let mut q = app
                .world_mut()
                .query_filtered::<(), With<ObjectiveGhostLineMarker>>();
            q.iter(app.world()).count()
        };
        assert_eq!(
            ghost_count(&mut app),
            0,
            "dying must not celebrate the failed objectives"
        );

        // The restarted run behaves normally: post one, complete it, ghost.
        app.world_mut().resource_mut::<GameObjectives>().objectives =
            vec![Objective::new("b1", "Burn for Beacon 1")];
        app.update();
        app.world_mut().resource_mut::<GameObjectives>().objectives =
            vec![Objective::new("b2", "Find Beacon 2")];
        app.update();
        assert_eq!(
            ghost_count(&mut app),
            1,
            "a real completion after the reset still ghosts"
        );
    }

    /// A tally swap (complete + re-add of the SAME id in one change) is
    /// not a completion: same id present before and after means no ghost.
    #[test]
    fn a_message_swap_of_the_same_id_leaves_no_ghost() {
        let mut app = feedback_app();

        app.world_mut().resource_mut::<GameObjectives>().objectives =
            vec![Objective::new("b3", "Crates: 0/3")];
        app.update();
        app.world_mut().resource_mut::<GameObjectives>().objectives =
            vec![Objective::new("b3", "Crates: 1/3")];
        app.update();

        let mut q = app
            .world_mut()
            .query_filtered::<(), With<ObjectiveGhostLineMarker>>();
        assert_eq!(
            q.iter(app.world()).count(),
            0,
            "same-id message swaps are progress, not completion"
        );
    }
}
