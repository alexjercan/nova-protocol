//! Scenario pacing primitives for the mainline scenarios.
//!
//! An objective must never post in the same frame as the conversation that
//! introduces it, or in the same frame as the objective it follows: it posts a
//! beat of scenario time LATER.
//!
//! Mechanism: [`beat_later`] - the introducing handler starts a one-step
//! `Sequence`, and the ENGINE holds the delay. The delay itself belongs to the
//! BEAT: each scenario names its own, beside the line it follows. The chain is
//! owned by the handler that started it, so a beat costs one action and no
//! scenario variable. The clock is engine-owned and pauses behind menus and
//! the outcome overlay, so a gap measures play time, not wall time.

use nova_scenario::prelude::*;

use crate::scenario_helpers::prelude::*;

/// The beat that lands `delay` seconds after the line that introduces it.
///
/// The introducing handler runs this as ONE action: it completes the previous
/// objective, plays its line, and starts a one-step sequence carrying the next
/// objective, its beacon and its markers. The engine holds the delay, and the
/// chain belongs to the handler that started it - so there is no gate variable
/// to seed, no `> 0` guard against an unstamped read, and no act check to prove
/// the beat is still the current one.
///
/// `key` is the sequence key: scenario-local, one per beat.
pub(crate) fn beat_later(
    key: &str,
    delay: f64,
    actions: Vec<EventActionConfig>,
) -> EventActionConfig {
    sequence(key, vec![step(delay, actions)])
}

// --- the outro ---------------------------------------------------------------

/// The sequence key the outro chain runs on. Scenario-local, and a scenario has
/// at most one outro, so a fixed key is safe: the win variants that start it are
/// mutually exclusive, so only one ever holds the cursor.
pub(crate) const OUTRO_SEQUENCE: &str = "outro";

/// The winning blow -> the tease line.
pub(crate) const OUTRO_TEASE_AFTER: f64 = 4.0;
/// The tease line -> the Victory banner. Together these hold the epilogue
/// cadence: a line at +4 s, the banner at +9 s.
pub(crate) const OUTRO_BANNER_AFTER: f64 = 5.0;

/// The two beats between the winning blow and the Victory overlay, as ONE
/// action the winning handler runs.
///
/// The modal overlay must not land on the same frame as the killing hit, or
/// one banner string has to carry both what happened and what comes next,
/// read against a paused world. The win handler posts only the beat it just
/// earned - which is why that line stays variant-specific, per handler - and
/// starts this chain: the tease lands [`OUTRO_TEASE_AFTER`] later while the
/// wreck is still on screen, then the banner and the queued next scenario
/// [`OUTRO_BANNER_AFTER`] after that.
///
/// The chain is owned by the handler that started it, so no beat has to
/// re-check which win opened it.
///
/// `banner_extra` rides the LAST beat. An objective belongs there rather than
/// on either comms beat: the mainline forbids posting one in the same frame as
/// a conversation line, and the banner beat is the only one without a line.
///
/// `next_scenario` is the queued hand-off, riding that same last beat. `None`
/// is a chapter that ENDS - the campaign stops here rather than continuing
/// somewhere - which is what every caller passes today.
pub(crate) fn outro_sequence(
    act_var: &'static str,
    won_act: f64,
    tease_speaker: &str,
    tease: &str,
    banner: &str,
    banner_extra: Vec<EventActionConfig>,
    next_scenario: Option<String>,
) -> EventActionConfig {
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
    sequence(
        OUTRO_SEQUENCE,
        vec![
            step(OUTRO_TEASE_AFTER, vec![comms(tease_speaker, tease)]),
            step(OUTRO_BANNER_AFTER, banner_actions),
        ],
    )
}
