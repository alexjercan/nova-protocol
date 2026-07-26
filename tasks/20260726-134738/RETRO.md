# Retro: Match NOVA OS drawer to terminal PoC

- TASK: 20260726-134738
- BRANCH: feature/nova-os-poc-fidelity
- REVIEW ROUNDS: 2

## What went well

- The new task split was right. Keeping UI fidelity separate from command/app
  work made it possible to remove the permanent panes without also inventing
  `log`, `objectives`, `map` or app runtime behavior.
- The widget-tree tests were the right verification level for this pass. They
  pinned topbar, terminal surface, prompt row, footer hints, CRT material and
  command scope without depending on a flaky software-GPU pixel capture.
- Reusing the existing `UiMaterial` pattern from `lock_dwell_ring.rs` kept the
  shader path local and easy to test.

## What went wrong

- R1.1 escaped because I copied the PoC footer hint `Ctrl+C: return from app`
  even though this task deliberately did not implement app runtime. Root cause:
  I treated PoC text as visual chrome without checking whether each hint named a
  currently wired control.
- R1.2 escaped because the first docs update fixed the ship-computer section but
  missed a nearby objective paragraph and the matching changelog line. Root
  cause: the sweep looked for the old phrase after one edit, but did not reread
  the whole live HUD section against the final behavior.

## What to improve next time

- For UI fidelity tasks, classify every copied PoC label as either decorative or
  functional before shipping it. Functional labels need a current input/action
  test or must be deferred with the feature they describe.
- After removing a visible surface, reread the whole live doc section that
  explains the workflow, not just the paragraph touched first. Adjacent
  paragraphs often carry the stale promise.

## Action items

- [x] Bumped `advertised-but-unwired` in `LESSONS.md`.
- [x] Bumped `out-of-context-review-pass` in `LESSONS.md`.
