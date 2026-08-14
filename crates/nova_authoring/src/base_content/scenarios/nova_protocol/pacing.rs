//! Scenario pacing primitives, shared across the mainline scenarios.
//!
//! Owner playtest (2026-07-22): objectives were showing in the same frame as
//! the conversation that introduces them, and completing an objective was
//! immediately followed by the next one - no breathing room. The fix is a
//! scenario-clock deadline sitting between a conversation (or an objective
//! completing) and the objective that follows it: the objective posts a beat
//! LATER, never the same frame.
//!
//! Mechanism:
//! - [`mark_clock`] stamps `scenario_elapsed + delay` into a variable at the
//!   moment a beat opens (OnStart, or the handler that completes the previous
//!   objective).
//! - [`clock_past`] is the filter that passes once the clock reaches that
//!   deadline.
//! - [`gated_once`] is the canonical "post the next objective / next line one
//!   beat later, exactly once" handler built on the two.
//!
//! Before this module each mainline scenario grew its own copy of the same
//! idea (shakedown's old `stamp_gate`/`past_gate` pair, final_tally's own
//! `mark_clock`/`clock_past`); they now share this one definition site
//! (shakedown keeps a thin `stamp_gate` alias for its breathers). The
//! clock (`scenario_elapsed`) is engine-owned and pauses behind menus/outcome,
//! so a deadline measures play time, not wall time.

use nova_hud::prelude::{COMMS_DWELL_SECS, COMMS_FADE_OUT_SECS, COMMS_MIN_SECS};
use nova_scenario::prelude::*;

use super::SCENARIO_ELAPSED_VAR;
use crate::scenario_helpers::prelude::*;

// The beat gap - how long an objective waits after the conversation line that
// introduces it - is a FEEL call, and the right value depends on the line's
// RELATIONSHIP to the objective (out-of-context pacing review). A single global
// gap (the old BEAT_GAP) conflated two: a coaching line the objective echoes
// wants the objective mid-read, while a threat/situation reveal wants the line
// to fully land first. So there are three named categories below, all derived
// from the comms panel's own constants so they cannot drift from the dwell.
// THESE ARE TUNABLE: they are authored timings, not physics - nudge them after
// playtest.

/// Reveal gap: the line fully lands and fades, THEN the objective posts. For a
/// threat or situation reveal the player should absorb before acting (the
/// scavenger telegraph, the corvette ambush, the flagship cast-off). This is
/// the previous uniform gap ([`COMMS_DWELL_SECS`] + [`COMMS_FADE_OUT_SECS`],
/// 8.4s).
pub(crate) const REVEAL_GAP: f64 = (COMMS_DWELL_SECS + COMMS_FADE_OUT_SECS) as f64;

/// Instruction gap: the objective posts MID-READ, while the coaching line is
/// still on screen (the line still holds its full dwell - nothing is queued
/// behind it - only the objective posts early). For a line the objective
/// echoes in real time: "Now hand her to the computer" -> "Press [G]" lands as
/// the player reads to the keypress. Tied to [`COMMS_MIN_SECS`], the panel's
/// yield floor - "the reader has had a beat with the line" (4s).
pub(crate) const INSTRUCTION_GAP: f64 = COMMS_MIN_SECS as f64;

/// Mid gap: halfway between instruction and reveal (~6s), for a line that
/// reveals then instructs ("that's the planetoid's pull - ease off the drive").
/// Let the reveal register, land the task as the player reaches the coaching
/// half.
pub(crate) const MID_GAP: f64 = ((COMMS_DWELL_SECS + COMMS_MIN_SECS) / 2.0) as f64;

/// Stamp an ABSOLUTE deadline of `delay` seconds into `key`, for a gate that
/// opens at `OnStart`. The engine clock `scenario_elapsed` is UNDEFINED at
/// OnStart (its first tick has not run), and the content evaluator errors on an
/// undefined read - so [`mark_clock`] (which reads `scenario_elapsed`) must NOT
/// be used at OnStart. The opening beat is at `t ~= 0`, so an absolute `delay`
/// deadline is exactly what the relative one would be. Every gate the scenario
/// uses must ALSO be seeded at OnStart (this seeds the opening gate; seed the
/// rest with `set_variable(gate, number(0.0))`), so a `gated_once` filter never reads an
/// undefined gate before its transition stamps it.
pub(crate) fn open_gate(key: &str, delay: f64) -> EventActionConfig {
    set_variable(key, number(delay))
}

/// Stamp `scenario_elapsed + delay` into `key`: a one-shot deadline a later
/// [`clock_past`]/[`gated_once`] waits for. Call it MID-SCENARIO - the
/// transition that completes the prior objective - where the clock is live.
/// NOT at OnStart: `scenario_elapsed` is undefined there (use [`open_gate`]).
/// Baking the delay into the stamp (rather than adding it at the gate) means
/// one deadline variable drives one follow-up cleanly.
pub(crate) fn mark_clock(key: &str, delay: f64) -> EventActionConfig {
    set_variable(
        key,
        VariableExpressionNode::new_add(
            VariableTermNode::Factor(VariableFactorNode::new_name(SCENARIO_ELAPSED_VAR)),
            VariableExpressionNode::new_term(VariableTermNode::Factor(
                VariableFactorNode::Literal(VariableLiteral::Number(delay)),
            )),
        ),
    )
}

/// Filter: the scenario clock has passed the deadline stamped in `key` by
/// [`mark_clock`].
pub(crate) fn clock_past(key: &str) -> EventFilterConfig {
    EventFilterConfig::Expression(ExpressionFilterConfig(
        VariableConditionNode::new_greater_than(variable(SCENARIO_ELAPSED_VAR), variable(key)),
    ))
}

/// A one-shot OnUpdate handler that fires `actions` once, a beat after the
/// deadline in `deadline_key` (set by [`mark_clock`]). `done_flag` latches it so
/// it never re-fires. The `deadline_key > 0` guard keeps it from firing before
/// the deadline is ever stamped: an unread scenario variable reads 0 and the
/// clock starts at 0, so without the guard an unstamped deadline would look
/// "already passed" and the objective would post on frame 0 - exactly the bug
/// this module removes. `extra_filters` add beat/act preconditions (e.g. only
/// while the ambush is live).
///
/// This is how an objective posts the beat AFTER its conversation, or after the
/// previous objective's completion, instead of the same frame.
pub(crate) fn gated_once(
    done_flag: &str,
    deadline_key: &str,
    extra_filters: Vec<EventFilterConfig>,
    actions: Vec<EventActionConfig>,
) -> ScenarioEventConfig {
    let mut filters = vec![
        number_equals(done_flag, 0.0),
        number_greater_than(deadline_key, 0.0),
        clock_past(deadline_key),
    ];
    filters.extend(extra_filters);
    let mut all = vec![set_variable(done_flag, number(1.0))];
    all.extend(actions);
    ScenarioEventConfig {
        name: EventConfig::OnUpdate,
        filters,
        actions: all,
    }
}

// --- the outro ---------------------------------------------------------------

/// Timer keys the outro chain runs on. Scenario-local, and a scenario has at
/// most one outro, so fixed keys are safe.
pub(crate) const OUTRO_TEASE_TIMER: &str = "outro_tease";
pub(crate) const OUTRO_BANNER_TIMER: &str = "outro_banner";

/// The winning blow -> the tease line.
pub(crate) const OUTRO_TEASE_AFTER: f64 = 4.0;
/// The tease line -> the Victory banner. Together these reproduce the finale's
/// playtested epilogue cadence (a line at +4s, the banner at +9s).
pub(crate) const OUTRO_BANNER_AFTER: f64 = 5.0;

/// The actions a winning handler runs to CLOSE the fight and open the outro:
/// move to the epilogue act and start the tease timer.
///
/// `epilogue_act` must sit outside every defeat gate (the mainline gates read
/// `act < 2` or `act == 1`), so the win is locked the instant it lands and a
/// death during the outro declares nothing.
pub(crate) fn open_outro(
    act_var: &str,
    epilogue_act: f64,
    mut actions: Vec<EventActionConfig>,
) -> Vec<EventActionConfig> {
    let mut all = vec![
        set_variable(act_var, number(epilogue_act)),
        start_timer(OUTRO_TEASE_TIMER, OUTRO_TEASE_AFTER),
    ];
    all.append(&mut actions);
    all
}

/// The two beats between the winning blow and the Victory overlay.
///
/// A win used to fire its modal overlay on the same frame as the killing hit,
/// so everything the moment had to carry - what just happened AND what it
/// means for the next chapter - was crammed into one banner string, read
/// against a paused world. The win handler now posts only the beat it just
/// earned (which is why that line stays variant-specific, per handler) and
/// opens this shared chain: the tease lands [`OUTRO_TEASE_AFTER`] later while
/// the wreck is still on screen, then the banner and the queued next scenario
/// [`OUTRO_BANNER_AFTER`] after that.
///
/// Both beats gate on `epilogue_act` alone, so variants that differ only in
/// their kill line (destroyed vs neutralized, the yacht's fate) share one
/// copy of the tail instead of duplicating it per handler.
///
/// `banner_extra` rides the LAST beat. An objective belongs there rather than
/// on either comms beat: the mainline forbids posting one in the same frame as
/// a conversation line, and the banner beat is the only one without a line.
pub(crate) fn outro_beats(
    act_var: &'static str,
    epilogue_act: f64,
    won_act: f64,
    tease_speaker: &str,
    tease: &str,
    banner: &str,
    banner_extra: Vec<EventActionConfig>,
    next_scenario: Option<String>,
) -> Vec<ScenarioEventConfig> {
    let mut banner_actions = vec![set_variable(act_var, number(won_act))];
    banner_actions.extend(banner_extra);
    banner_actions.push(EventActionConfig::Outcome(OutcomeActionConfig::new(
        ScenarioOutcomeKind::Victory,
        banner,
    )));
    if let Some(scenario_id) = next_scenario {
        banner_actions.push(EventActionConfig::NextScenario(NextScenarioActionConfig {
            scenario_id,
            linger: true,
            delay: None,
        }));
    }
    vec![
        ScenarioEventConfig {
            name: EventConfig::OnTimerEnd,
            filters: vec![
                timer(OUTRO_TEASE_TIMER),
                number_equals(act_var, epilogue_act),
            ],
            actions: vec![
                story_message(tease_speaker, tease),
                start_timer(OUTRO_BANNER_TIMER, OUTRO_BANNER_AFTER),
            ],
        },
        ScenarioEventConfig {
            name: EventConfig::OnTimerEnd,
            filters: vec![
                timer(OUTRO_BANNER_TIMER),
                number_equals(act_var, epilogue_act),
            ],
            actions: banner_actions,
        },
    ]
}
