# Retro: ch3 overspeed - sustained ~3.5s window on the second strike

- TASK: 20260723-182850
- BRANCH: feature/ch3-overspeed-window
- REVIEW ROUNDS: 1 (APPROVE, out-of-context, zero findings)

See TASK.md Outcome for what changed and why; this is process only.

## What went well

- TDD landed the behaviour change cleanly: renaming the trip test to the
  held-breach path and adding the cancel test FIRST, then watching the renamed
  test go red against the still-instant-trip RON for exactly the right reason
  (`speed_warned` stuck at 2, never reaching the new countdown state 3), gave a
  real red->green. The `include_str!` coupling meant I had to hold off editing
  the RON until the red run finished - worth remembering that a content rig
  compiles the data in, so the red check and the fix cannot overlap.
- Grounding the new mechanic in an existing idiom (the `beat_gate`
  `Add(Factor(Name("scenario_elapsed")), ...)` deadline stamp) meant zero new
  vocabulary for the reviewer and no engine change - the whole feature was three
  RON handlers + one seeded variable.
- Followed `seed-helper-drifts-from-source` deliberately: `overspeed_deadline`
  went into OnStart, the rig's `armed_app` seed list, AND the `on_start_seeds_*`
  key pin in the same change; the out-of-context reviewer confirmed no drift.
- Designed for order-independence: CANCEL (`<7`) and TRIP (`>8`) share the
  counting state but are mutually exclusive on speed, so correctness does not
  depend on handler evaluation order - the reviewer re-derived the same and had
  nothing to flag.

## What went wrong

- The DoD's lint command was wrong: I copied `content -- lint webmods/the-ledger`
  from the sibling task 20260723-143603 without checking it, but the CLI takes
  `--target webmods/the-ledger` (the positional form errors). Root cause:
  trusted an inherited command string from a prior task's DoD instead of
  verifying against `--help`. Caught immediately on first run, fixed the DoD
  text, no real cost - but it would have been a stale-command finding if the
  reviewer had hit it first.

## What to improve next time

- When a DoD or Step copies a CLI invocation from a prior task, run it (or
  `--help`) once before trusting it - inherited command strings drift as the
  tool evolves.

## Action items

- [x] Fixed the DoD lint command to the `--target` form in TASK.md.
- [x] Lessons ledger: added `inherited-cli-string-drifts` (x1) - a CLI string
  copied from a prior task's DoD can be stale against the tool's current flags.
