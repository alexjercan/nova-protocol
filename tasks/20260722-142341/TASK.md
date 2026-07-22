# Objective posts before its intro dialogue finishes: match the beat gap to the comms dwell (mainline scenarios)

- STATUS: CLOSED
- PRIORITY: 84
- TAGS: v0.8.0, content, scenario, pacing, playtest

Owner playtest follow-up to the pacing pass (task 20260722-092421). The
gated_once mechanism is in place across all four mainline scenarios, but the
gap is still too short: an objective posts while the conversation that
introduces it is STILL on screen.

## Root cause

`pacing::BEAT_GAP = 4.0s`, but a comms line holds the screen for
`COMMS_DWELL_SECS = 8.0s` (nova_gameplay comms_panel) when nothing is queued
behind it. Every `open_gate`/`mark_clock` call across the scenarios follows a
`story()` line in the same action block, then posts the objective at t+4s -
four seconds before the dialogue line finishes and fades. So "wait for the
dialogue to finish before the next objective appears" is not actually
happening.

## Owner ask

- Wait AT LEAST for the dialogue to finish before adding the next objective.
- Add the timeout / change the objective beat only after the dialogue is done.

## Fix

The post-dialogue gap must equal the comms dwell, not a fixed 4s.
`nova_assets` depends on `nova_gameplay`, so `pacing.rs` can reference the
comms dwell constant as the single source of truth.

1. In `crates/nova_gameplay/src/hud/comms_panel.rs`: make `COMMS_DWELL_SECS`
   (and the fade-out tail `COMMS_FADE_OUT_SECS`) `pub`, exported via the
   module prelude - the same way `COMMS_DWELL_MIN_SECS`/`MAX` already are.
2. In `crates/nova_assets/src/scenario/pacing.rs`: derive the beat gap from
   `COMMS_DWELL_SECS + COMMS_FADE_OUT_SECS` so an objective posts as its
   introducing line finishes and fades, never before. Document the coupling.
   (A `line_gap(dwell: Option<f32>)` helper for authored per-line dwell
   overrides was considered but DEFERRED: no scenario line sets a dwell
   override today, so the helper would be dead code - the BEAT_GAP doc-comment
   notes where it would slot in. Revisit when a beat authors a longer intro
   line before a gate.)
3. Sweep the four scenarios (shakedown, lifeline, broadside, final_tally):
   every gate that follows a `story()` line uses the dialogue-length gap. A
   pure objective-complete breather (no line) may keep a short gap if any
   exist - audit each site.

## Verify

- `cargo fmt` + `cargo check` clean.
- A `/probe` run on a mainline scenario (shakedown or lifeline) confirming the
  objective now appears as the intro line fades, not mid-line.

Related: [[pacing.rs]] module, task 20260722-092421 (pacing pass),
20260722-114541 (OnStart gate undefined-read bug).
