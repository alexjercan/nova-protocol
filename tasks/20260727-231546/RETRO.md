# Retro: Unify NOVA OS commands into one TerminalCommand model

- TASK: 20260727-231546
- BRANCH: refactor/nova-os-terminal-command
- REVIEW ROUNDS: 1 (APPROVE, out-of-context, zero findings)

(What/why/evidence live in TASK.md close-out; this is process only.)

## What went well

- A full consumer-graph grep sweep BEFORE editing (every use of
  TERMINAL_COMMANDS / NovaOsAppCommand / NovaOsAppRegistry / the snapshot
  fields / ResolvedCommand across crates + examples + tests) turned the
  gameplay-side changes into a mechanical checklist. After the nova_os crate
  compiled, `cargo check --workspace --all-targets` passed on the first try - no
  missed silent consumer, which is the usual trap on a cross-crate symbol
  removal.
- Keeping subcommand names as their FULL `&'static` string (`"map view"`, not a
  nested `"view"` needing concatenation) meant the longest-prefix matcher and
  ALL its existing tests stayed behaviorally identical - only the data source
  changed. That single decision shrank the blast radius of a big refactor and
  kept `id: &'static str` valid for `TerminalMode::App` with no ripple.
- The out-of-context reviewer re-ran every check independently and confirmed the
  DoD greps + load-bearing tests, catching nothing - a genuine clean-diff APPROVE
  rather than a rubber stamp, because it built and ran rather than reading.

## What went wrong

- Two self-inflicted unit-test FAILURES on the first `cargo test -p nova_os`,
  both in test FIXTURES I hand-picked by eyeballing rather than computing:
  1. did-you-mean typo `"mep"` was within Levenshtein 2 of `help` (delete `l`,
     sub `h->m`), breaking the "no near core command" assertion. Root cause:
     guessed a typo close to the intended target without checking it was FAR
     from every other command.
  2. the help-listing filter matched `map` before `map view` for the `map view`
     row (both share the `map ` prefix). Root cause: reused the old test's
     first-prefix-match logic without accounting for the new order where the
     app root precedes its subcommand.
  Both were caught by the tests themselves and fixed before review, so they cost
  one extra test run - but they were avoidable.

## What to improve next time

- When writing fixtures for distance-based (Levenshtein/did-you-mean) or
  prefix-based (longest-match/completion) logic, pick the collision/typo cases
  by COMPUTING the distances/prefixes against the whole command set, not by
  eyeballing a plausible-looking string. The whole point of those tests is the
  boundary, so the fixture has to actually sit on the right side of it.

## Action items

- [x] Lesson `test-fixture-distances-computed-not-eyeballed` added to LESSONS.md.
- No follow-up code tasks: the ship-viewer app (20260726-115339) now has the
  unified `TerminalCommand::app(...).with_subcommand(...)` pattern to follow; no
  new task needed, it inherits the seam.
