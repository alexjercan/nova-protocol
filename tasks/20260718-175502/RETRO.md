# Retro: SHIFT keybind hint + disable RCS in the mainline campaign

- TASK: 20260718-175502
- BRANCH: feat/rcs-keybind-disable (landed as master 0d4e53f4)
- REVIEW ROUNDS: 1 (APPROVE, 2 NITs)

Process only.

## What went well

- The keybind hint dropped straight onto the proven `radar` verb-hint pattern
  (fixed label + `verb_granted`), and the cluster's existing "contextual rows"
  behavior gave "show SHIFT only when RCS enabled" for free - a one-field +
  one-row change.
- Checked the exact regression risk before landing: the shakedown controller-verb
  test uses `.any(|m| DisableVerb == verb)`, NOT an exact set, so adding
  `DisableVerb(Rcs)` to its gate is safe. Verifying the assertion SHAPE (not just
  running the test) is what made this confident.

## What went wrong

- Burned a whole pass editing the four scenario `.content.ron` files DIRECTLY to
  add `DisableVerb(Rcs)`, then discovered they are GENERATED from Rust builders
  and guarded by `content_ron_parity`. Reverted and edited the builders +
  regenerated. Root cause: didn't check for a generator before hand-editing
  content. New lesson `edit-the-builder-not-the-generated-ron`.
- A heuristic script (match `source` containing "controller") mis-identified the
  player controller in a racer scenario (prototype names vary:
  `basic_controller_section` vs `racer_cube_i0_j1_k0`); had to read each builder's
  player controller precisely instead of scripting blindly.
- Verification was crippled by external machine load (average ~48-50): the
  `content -- gen` and each test compile were starved for 20-40 min. Not a code
  issue, but it made the last mile very slow.

## What to improve next time

- Before hand-editing any `assets/**/*.content.ron`, grep for a builder /
  `content -- gen` and a parity test; edit the builder if one exists.
- When targeting "the player" in scenario data, find it structurally (the section
  under `controller: Player`), not by a prototype-name heuristic that varies by
  ship type.

## Action items

- [x] Ledger: added `edit-the-builder-not-the-generated-ron`.
- [ ] Parent RCS task 20260717-105406 to be CLOSED now that the family is
  delivered (core, input, HUD, autopilot terminal, keybind + mainline-disable);
  remaining follow-ups are seeded: cap ring (20260718-144939), ORBIT
  error-relative RCS + terminal-creep (20260718-151102).
