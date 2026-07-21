# Retro: Resolve asteroid_field hidden-vs-wiki contradiction

- TASK: 20260721-160842
- BRANCH: fix/asteroid-field-hidden (landed 8c7be318)
- REVIEW ROUNDS: 2

## What went well

- Verify-first paid off immediately: what the spike filed as a docs
  contradiction turned out to be orphaned finished content; blame-then-decide
  changed the fix from a wiki edit to an unhide.
- The out-of-context reviewer did exactly what the mechanism exists for:
  it re-derived the history claim from scratch and caught the implementing
  session's false narrative (R1.1) plus the missing conventional visibility
  pin (R1.2). Both were real, neither was visible from inside the session.
- Sabotage-proving the new pin (master RON -> red, branch RON -> green) took
  two minutes and made R1.2's fix verifiable instead of asserted.

## What went wrong

- R1.1: the first-pass Record stated history that never happened ("shakedown
  chained into asteroid_field"). Root cause: a `git log -S asteroid_field`
  hit on shakedown.rs was read as evidence of a chain, without opening the
  commit - the hit was actually the NEW_GAME_SCENARIO_ID swap. A pickaxe hit
  names a commit that TOUCHED the string, not what it did with it.
- Master carried fmt drift in four files (recent HudReadout/probe commits
  landed unformatted), so this small task's diff absorbed unrelated
  formatting hunks. CI does not gate `cargo fmt --check`, so drift
  accumulates silently until some branch runs the documented fmt ritual.

## What to improve next time

- Before writing any history-evidence sentence into a Record, open the
  named commit's diff (`git show <sha> -- <file>`) and quote what it DID -
  never cite a pickaxe/blame hit alone.
- The fmt drift wants a tool guard, not vigilance (filed below).

## Action items

- [x] LESSONS.md: appended `pickaxe-hit-is-not-a-mechanism` (x1).
- [x] tatr 20260721-163942: add `cargo fmt --check` to CI so master cannot
      accumulate drift (tool > prose).
