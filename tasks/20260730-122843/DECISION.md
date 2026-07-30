# DECISION: what a hidden dock does with a scenario spotlight

- STATUS: ACCEPTED
- DATE: 2026-07-30
- DECIDER: owner (/flow plan gate)

## Context

The owner asked for the pre-dock rule back: a keybind that is not available now
is not on screen at all, rather than shown dimmed. The dock's own doc argues the
opposite ("a dim chip STAYS on screen ... the constant positions are what make
it glanceable"), so this reverses a shipped design choice from task
20260728-175742 - deliberately, on playtest evidence.

That reversal collides with an existing feature. Scenarios spotlight a verb via
the `HintEmphasisSet` action, and `pulse_emphasized_chips` carries a dedicated
`EMPHASIS_ALPHA_UNAVAILABLE` band precisely so a tutorial can point at a verb
BEFORE it becomes available ("press ORBIT" while the offer is not up yet). The
two wants are mutually exclusive on that chip: a strictly hidden chip cannot
pulse, and a pulsing chip is not hidden. That incompatibility is the decision.

## Options

1. **Spotlight forces the chip visible.** Unavailable chips are hidden, except
   when a scenario emphasizes that verb; then it appears and pulses in the
   existing dim alpha band. Tutorials keep working; the rule needs one
   exception clause and `update_dock` has to read `HintEmphasis`.
2. **Strict hide, no exception.** Unavailable means absent, full stop. Simplest
   rule; a tutorial spotlight on a not-yet-available verb silently shows
   nothing, and the `EMPHASIS_ALPHA_UNAVAILABLE` band becomes dead code.
3. **Keep the dim chips.** No change; reject the playtest feedback.

## Decision

Option 1. The owner picked it at the plan gate.

The visibility rule is one line:

    shown = !hint.key.is_empty() && (state != Dim || emphasis.contains(verb))

`Hot` stays visible even when its hint is unavailable: `chip_state` checks `Hot`
before availability on purpose, because the ORBIT offer retires the instant you
are parked and that is exactly when ORBIT should read as the live maneuver.

## Consequences

- `update_dock` gains `HintEmphasis` as an input, and that resource must join
  its quiet-frame change gate - otherwise an emphasis set on a frame where
  hints/situations/assets did not change will not reveal the chip.
- The dock row is `justify_content: Center`, so it re-centres as verbs come and
  go. Accepted as the price of the rule; the task records the playtest verdict
  on whether the sliding reads badly.
- The dock's module docs asserting the dim-chip rule become wrong and are
  corrected in the same task.
