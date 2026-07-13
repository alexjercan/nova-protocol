# Retro: Investigate Entity-despawned command error on menu to game transition

- TASK: 20260713-175352
- BRANCH: fix/menu-despawn-command-warn (landed as c6cfde7)
- REVIEW ROUNDS: 1 (one MINOR, fixed in-round)

## What went well

- diagnostic-first held under temptation: the static audit surfaced juicy
  suspects (menu ambience teardown, editor preview DespawnOnExit), and none
  of them got "fixed" - 10 harnessed runs showed the warn simply does not
  fire natively, so no speculative queue_handled landed anywhere.
- The null result was converted into durable value instead of a shrug: the
  probe rig became examples/13_menu_newgame, CI now covers the shipped New
  Game boot flow (previously uncovered), and the panic-on-command-error
  handler makes the pinned non-behavior falsifiable.
- The pin itself was A/B proven (injected stale-entity command -> exit 134
  with the web log's exact error shape) before being trusted.

## What went wrong

- The editorplay probe phases were drafted before reading 09_editor's
  existing autopilot machinery, which already had the button-driving
  pattern to copy. Root cause: eagerness to write the rig before the
  read-first step; cost was small but real.
- R1.1: docs/development.md maintains an enumerated example list and the
  new example was not added to it until review. Root cause: additive
  changes have no sweep habit - the delete/move sweep lesson exists, but
  nothing prompts "what doc index enumerates artifacts of this kind?" on
  an ADD.

## What to improve next time

- On adding an artifact of an enumerated kind (example, crate, workflow),
  grep docs/ for the list that enumerates its kind before committing.
- When an investigation heads toward "cannot reproduce", decide early what
  permanent pin the rig should become - it changes how you build the rig
  (env knobs, error-handler swap) while the cost is still marginal.

## Action items

- [x] LESSONS.md: add `additions-join-doc-indexes` (x1)
- [x] LESSONS.md: add `null-result-becomes-a-pin` (positive, x1)
- [ ] User: after deploying the fixed build, re-check the web console for
      the despawn warn (task record has the follow-up routing)
