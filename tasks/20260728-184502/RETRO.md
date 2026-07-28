# Retro: NOVA OS shell-like help + wrong-command usage messages

- TASK: 20260728-184502
- BRANCH: feat/nova-os-shell-help
- REVIEW ROUNDS: 1 (APPROVE)

## What went well

- The one load-bearing fork (name the argument vs keep a generic `<arg>`) was
  surfaced at the plan gate and recorded in DECISION.md before any code, so the
  build had no mid-flight "the model can't express this" surprise. The rich
  authoring form already carried `summary`; hanging `arg_hint` next to it was a
  natural, low-friction extension.
- Renderers are pure functions over scrollback, so all six DoD proofs are fast
  pure-terminal tests in `nova_os` - no ECS app spin-up for the core behavior.
- The out-of-context reviewer did its job: it found a real coverage gap the
  implementing session was blind to (below), and otherwise confirmed a clean
  diff with a short round.

## What went wrong

- R1.1: the `<section>`/`<label>` arg hints are set at four `nova_gameplay`
  registration sites but the only usage-line tests lived in `nova_os` and used
  hand-built synthetic specs. Nothing proved the real `ship_command_tree()` /
  map tree actually wires the hint through to the renderer. Root cause: tested
  at the pure-helper altitude and trusted the registration edits by eyeball -
  the exact shape of `pin-each-caller-not-just-shared-core`. Fixed by adding an
  end-to-end `ship_verb_help_names_the_section_argument` test that submits
  `ship <verb> help` against the registered tree.
- DECISION.md STATUS was written as `ACCEPTED (owner approved ...)`, which
  `tatr check` rejects (`bad-decision-status` - the field is a strict enum).
  Cheap fix: moved the note to the DATE line. tatr check is the guard here, so
  no ledger entry needed.

## What to improve next time

- When a change adds a data field consumed by a shared renderer but SET at N
  registration sites, add one end-to-end test that drives a real registration
  site through the renderer in the SAME pass - do not stop at a pure-helper test
  with a synthetic fixture.
- Keep DECISION.md/TASK.md status fields to their bare enum tokens; put dates
  and notes on separate lines.

## Action items

- [x] Bumped `pin-each-caller-not-just-shared-core` to x3 in LESSONS.md and
      parked it under Pending promotions (target: work verify step + review
      Tests dimension).
- No follow-up code tasks. The HTML PoC (R1.2) stays a frozen reference; a PoC
  refresh belongs to the UI-rework epic (20260728-175719) if it revisits it.
