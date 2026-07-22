//! Scenario pacing primitives, shared across the mainline scenarios.
//!
//! Owner playtest (task 20260722-092421, 2026-07-22): objectives were showing
//! in the same frame as the conversation that introduces them, and completing
//! an objective was immediately followed by the next one - no breathing room.
//! The fix is a scenario-clock deadline sitting between a conversation (or an
//! objective completing) and the objective that follows it: the objective
//! posts a beat LATER, never the same frame.
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

use nova_scenario::prelude::*;

use super::shakedown::{eq_num, gt_num, num, set, var};

/// The default breathing beat, in seconds of play time: how long an objective
/// waits after the conversation that introduces it, or after the previous
/// objective completes, before it appears. Owner note: "show the objective,
/// wait a bit, show the message" - and never the swap and the completion in the
/// same instant.
pub(crate) const BEAT_GAP: f64 = 4.0;

/// Stamp `scenario_elapsed + delay` into `key`: a one-shot deadline a later
/// [`clock_past`]/[`gated_once`] waits for. Call it in the handler that opens
/// the beat - the OnStart, or the transition that completes the prior
/// objective. Baking the delay into the stamp (rather than adding it at the
/// gate) means one deadline variable drives one follow-up cleanly.
pub(crate) fn mark_clock(key: &str, delay: f64) -> EventActionConfig {
    set(
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
        VariableConditionNode::new_greater_than(var(SCENARIO_ELAPSED_VAR), var(key)),
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
        eq_num(done_flag, 0.0),
        gt_num(deadline_key, 0.0),
        clock_past(deadline_key),
    ];
    filters.extend(extra_filters);
    let mut all = vec![set(done_flag, num(1.0))];
    all.extend(actions);
    ScenarioEventConfig {
        name: EventConfig::OnUpdate,
        filters,
        actions: all,
    }
}
