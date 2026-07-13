# Retro: Controller-provided flight verb flags (family of 3)

- TASKS: 20260712-143832, 20260712-143833, 20260712-143834
- BRANCH: spaceship-controller-verb-flags (one branch for the whole family, per
  the /sprout request; not merged to master - landing is the user's call)
- REVIEW ROUNDS: 1 (out-of-context agent review; no BLOCKER/MAJOR, one MINOR
  fixed and retested, no re-review needed)
- SPIKE: tasks/20260712-143551/SPIKE.md

Process observations only; what/why/evidence live in the three TASK.md files and
the spike fix-record.

## What went well

- **The spike's own open question paid for itself.** The spike explicitly flagged
  the OnStart spawn-vs-action ordering window ("a SetControllerVerb queued at
  OnStart could run before the controller section exists"). Because it was written
  down as a risk, I resolved it *before* wiring - authored the shakedown's initial
  GOTO-off in the player's controller CONFIG instead of an OnStart action - rather
  than discovering it as a flaky "no controller section" warn during a test. A
  named unknown in the spike is cheaper than an emergent bug.
- **Reused three existing seams, minimal new surface.** `flyable` already gated on
  a live controller; the `SetSpeedCap` action was the exact skeleton for
  `SetControllerVerb`; the beat-1 governor-release event was the exact seam for the
  GOTO enable. The feature landed as a thin capability layer, not new machinery.
- **Out-of-context review earned its keep again.** A fresh-context agent caught the
  one real hazard (below) that shared-session eyes glossed over, and independently
  verified hint/execution parity, the pirate-keeps-GOTO claim, and that no new test
  was vacuous. The same-session author had convinced himself the merged query was
  fine.

## What went wrong

- **Review MINOR 1 (fail-closed brick): folded the new REQUIRED `ControllerVerbs`
  fetch into the EXISTING `q_computer` query, which is what `flyable` is computed
  from.** That silently coupled "can this ship fly at all" to "does this controller
  carry the verb-flags component" - a controller with a `PDController` but no
  `ControllerVerbs` would make the whole ship non-flyable, not just default its
  verbs on. It cannot happen in production (the one spawn path always inserts the
  component), but it is an invisible fail-closed trap for any future spawn path.
  **Root cause:** I reused an existing query as the convenient insertion point for
  new data without asking "what does this query already gate, and does adding a
  *required* component narrow that gate?" Adding a required fetch shrinks a query's
  membership set; every consumer of that set inherits the new precondition. Fixed
  by making the component `Optional` in both queries (absent -> all-on default),
  decoupling `flyable` from the flags.
- **`tatr new` same-second collision, AGAIN (now 5th recorded).** Three `tatr new`
  in one `&&` chain during the spike collapsed to a single task (the other two
  overwritten). Recovered per the known recipe: `rm` the survivor, recreate the
  three in separate calls with a clock-tick busy-wait between them. The ledger's
  mechanical-fix promotion is still pending and this keeps costing a recovery.
- **`cargo test -p A f1 -p B f2` errored** with "unexpected argument 'f2'" after a
  full compile - the two-package/two-filter form is the same trap as
  `cargo test a b c`. Had to rerun one package+filter per invocation, paying a
  second compile.

## What to improve next time

- **Before adding a required component to an EXISTING system query, enumerate what
  that query already gates.** If the new data is orthogonal to the query's existing
  purpose (here: verb config vs "is the ship flyable"), fetch it `Optional` or in a
  separate query - do not narrow a shared gate as a side effect. This is
  `reread-after-insert` / `does-the-old-element-survive` applied to query
  membership.
- **One `cargo test` invocation = one `-p` + one filter.** For multiple packages,
  separate runs.
- **One `tatr new` per tool call**, never chained (mechanical fix still pending).

## Action items

- [x] Fixed the fail-closed coupling (b14914c), added a flags-less-controller
      regression test.
- [x] LESSONS.md: new `required-component-in-shared-query`; bumped
      `out-of-context-review-pass`, `tatr-same-second-collision`,
      `one-cargo-test-filter`; positive `spike-open-question-pays-off`.
- [ ] No follow-up code work: the feature is complete. HUD already reflects the
      flags automatically (verb hints darken); no separate HUD task needed.
