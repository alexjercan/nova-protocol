# Retro: NOVA OS chin controls 3D pass

- TASK: 20260727-135208
- BRANCH: feature/nova-os-chin-3d
- REVIEW ROUNDS: 1 (APPROVE, one NIT)

## What went well

- Reused the file's established 3D vocabulary (`BackgroundGradient` +
  `LinearGradient`/`RadialGradient` + `ColorStop`, already used by the casing,
  screws and glass sheen) instead of reaching for a new primitive. No new
  rendering path, and the reviewer flagged zero design/consistency issues.
- Matched the PoC constants exactly (dial `#4a555d/#232a30/#0d1114`, button
  `#333c44/#1a2026`), so "does it match the web version" is answerable by a
  colour-by-colour read, not a judgement call.
- Strengthened the one existing test the behaviour change touched (the SND
  label-swap assertion) into a bulb-colour + fixed-label assertion, rather than
  deleting it. The change locked in the new behaviour instead of hiding it.
- Out-of-context review was cheap and decisive (1 round, APPROVE), and it
  re-confirmed the load-bearing claim (LED reset paths) I had also checked.

## What went wrong

- The DoD proof command copied from the sibling casing task
  (`cargo test -p nova_gameplay drawer`) matched ZERO tests - the nova_os tests
  live under `hud::nova_os::tests::*`, no "drawer" in the path. The first run
  reported `test result: ok. 0 passed ... 690 filtered out`: a green that proved
  nothing. Root cause: inherited a DoD `cmd:` from a look-alike task without
  checking the filter selected the new tests' module path. Caught only because I
  read the test-COUNT line rather than trusting the "ok".

## What to improve next time

- When a DoD proof is a test filter, verify at verify-time that it runs a
  NON-ZERO count of the INTENDED tests (read "N passed", not just "ok") before
  trusting the green. Same discipline as checking arity/flags.

## Action items

- [x] Bumped ledger `validate-proof-command-shape-at-plan-time` to x2, sharpened
  to cover the wrong-target/0-tests false-green case (not just malformed arity).
- [x] Corrected this task's DoD filter in TASK.md to the real chin-control test
  names.
- Documented (TASK.md close-out) the one look choice deviating from the literal
  feedback: SND label "SND" vs the owner's suggested "SND ON/OFF" - a one-line
  change if the owner prefers the literal. Left for the manual acceptance check.
