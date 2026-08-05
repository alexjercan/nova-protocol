# Retro: menu_scenarios is killed by a signal in the ui smoke, roughly 1 run in 5

- TASK: 20260805-111329
- BRANCH: fix/sync-pipeline-compilation
- REVIEW ROUNDS: 1

## What went well

- The brief was wrong and the investigation said so. The 48 KB truncated
  stderr tail pointed at scenario loading; the full log showed the run
  finishing, and the core dump named the real thread. Reading the artifact
  the brief quoted, rather than trusting the quote, is what turned an
  unfalsifiable flake into a one-line fix.
- Prototype-before-decide. `DECISION.md` was written with 30/30 clean already
  measured, so the decision argued about the design (gate it or not) rather
  than about whether the fix works.
- Two independent readings for a probabilistic claim. A clean 30 against a 10%
  base rate is ~4% likely by luck; the kernel `segfault` count is a second
  reading that a lucky run cannot fake. `## Notes` stated that risk up front
  instead of after the fact.
- Breadth: 3 code lines plus one import. The diff is small because the
  decision rejected four narrower-looking variants that each added a mechanism.

## What went wrong

- The doc sweep updated two occurrences of "window/log/asset setup" and missed
  a third (R1.1, `web/src/wiki/dev/architecture.md:15`) two rows from one it
  did update. The sweep was done by following the concept rather than by
  grepping the literal phrase.
- Churn: none attributable to the plan. One NIT, no rework cycle. The
  from-scratch challenge in `plan` was already answered in `DECISION.md`
  ("built from scratch today the same call holds"), and review agreed.

## What to improve next time

- When a doc edit rewords a stock phrase, grep the OLD phrase across
  `web/src/wiki/` and `README` and fix every hit, instead of editing the
  passages you remember. Cheaper than a review round.
- Cost-gated proofs need their substitute named in advance. The 30-run loop
  costs ~4 min per pass, so neither review pass re-ran it; the reviewer
  improvised a 3-run + kernel-log spot check. A DoD with an expensive `cmd:`
  should say what the reviewer is expected to run instead.
- Context: the flow resumed cleanly from disk after a `/clear` - `sprout ls`
  identified the task root while the main checkout still reported WORKING. No
  compaction pressure observed.

## Action items

- Fix `web/src/wiki/dev/architecture.md:15` to read "window/log/asset/render
  setup" (R1.1, NIT, may ride along with the next docs touch).
- `20260805-114935` already filed: make `tests/examples_smoke.rs` name the
  signal instead of reporting `exited with None`.
- Pending user check: read `NOTES.md` "After-numbers" and confirm the pass
  count, kernel segfault count and median against the recorded baseline.

## Landing message

```
fix(nova_core): compile pipelines synchronously to kill the exit-time SIGSEGV

An async pipeline-compile task still in flight at exit dropped the last
Arc<Device> from an AsyncComputeTaskPool thread while the main thread tore
the same Vulkan device down, segfaulting inside the NVIDIA driver. Only the
self-ending menu_scenarios example hit it, about one ui-smoke run in five,
reported as the uninformative "exited with None".

Set RenderPlugin { synchronous_pipeline_compilation: true } in
AppBuilder::new, so no compile task ever owns a device reference and the race
cannot occur - unconditionally, so the examples exercise the same rendering
configuration the shipped game uses. No-op on wasm, macOS and non-threaded
builds. Measured 0 failures in 60 runs with 0 kernel segfaults against a 2/20
baseline, and no run-time regression.
```
