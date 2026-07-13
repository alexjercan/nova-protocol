# Retro: Disabled-in-place controller still torques toward its frozen command

- TASK: 20260709-155922
- BRANCH: fix/disabled-controller-torque (squash-merged as 10ea607)
- REVIEW ROUNDS: 1 (APPROVE, one NIT raised and addressed)

What shipped is in the task's Resolution and
`tasks/20260709-155922/NOTES.md`. A one-line
`Without<SectionInactiveMarker>` filter; the cycle's value was in proving it was
correct, complete, and non-vacuously tested. A smooth cycle - short retro.

## What went well

- **Last retro's lessons applied on purpose, and they paid.** I used a
  tolerance-based physics assertion from the first draft and chose the spin axis
  so the torque-free motion is genuinely constant (symmetric top about z) - so
  the float-precision trap that bit the last three cycles did not recur. And I
  kept all file edits (test, docs, TASK.md steps) on the branch. The compounding
  is visibly working.
- **The regression test was proven, not assumed.** In the review I reverted the
  filter and confirmed `a_disabled_controller_leaves_the_spin_untouched` FAILS
  without the fix, then restored it. Pairing it with a live-controller control
  case means the test can neither pass vacuously nor be a lucky tolerance. This
  revert-and-confirm step is cheap and worth making standard for regression
  tests.
- **Claims were verified, not narrated.** "nova is the only consumer of the PD
  output" was checked by grepping every `PDControllerOutput` use across nova and
  bcs; "both disable paths are covered" was reasoned through (non-leaf -> marker
  -> filtered; leaf -> despawned -> no output). The prior COM retro's "verify a
  comment like code" lesson generalized to verifying the design claims.

## What went wrong

- **Forgot to flip STATUS to CLOSED on the branch before squashing.** Last cycle
  I did it on the branch; this cycle I only ticked the step checkboxes, so the
  squash landed an OPEN task and I had to `tatr edit` + `git commit --amend` on
  master. Root cause: "close the task" is not yet part of my fixed pre-squash
  checklist, so it depends on memory.
- **Wasted a build passing two positional filters to `cargo test`.** `cargo test
  a b` treats the second as an unexpected arg and errors after the (slow)
  compile. Root cause: `cargo test` takes one substring filter, not a list.

## What to improve next time

- **Pre-squash checklist, on the branch:** (1) all file edits done, (2) STATUS
  flipped to CLOSED, (3) REVIEW.md at APPROVE, (4) checks green. Then land. Items
  (1) and (2) are both "do it on the branch, not on master after the fact".
- **One `cargo test` filter at a time** - pick a substring that matches the
  intended set (here "controller" covered both the new tests and the module), or
  run separate commands.

## Action items

- [ ] AGENTS.md "Testing and examples": still-pending proposal from the
      20260709-144906 retro (prefer tolerance over exact-equality for
      physics/float values). This cycle is positive evidence it works when
      applied preemptively - worth landing the rule. Awaiting user OK to edit the
      global file.
