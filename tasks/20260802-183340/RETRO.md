# Retro: Port the harness completion protocol into nova_autopilot

- TASK: 20260802-183340
- BRANCH: feat/autopilot-completion
- REVIEW ROUNDS: 2

## What went well

The diff stayed inside the one file the plan named, and both planned deltas
(`NOVA_AUTOPILOT_DEADLINE`, the collector-constant docs) landed as written. The
review's two MAJORs were each pinned by a test that was measured red first
(`1.9999992x` wall time; the empty capture buffer), and R1.2's assertion was
mutation-checked rather than trusted. R1.3's pushback was settled by re-running
rustdoc rather than by argument.

## What went wrong

Both MAJORs were inherited verbatim from
`bevy-common-systems/src/completion.rs`, and one of them the plan actively
protected: a Note told the implementer that the false
"duplicates are harmless because `exited` makes the body idempotent" comment
was load-bearing and to port it verbatim. It reads as sound - `exited` really
does make the EXIT idempotent - but the watcher's `elapsed` accumulation is
not covered by it, so N registrants burned the deadline N times too fast.

The mid-fix `git checkout <path>` used to revert a temporary mutation probe
also discarded the uncommitted fixes in that same file. All the work was
re-applied from context, but the safe move is to stash or copy the file before
mutating it.

## What to improve next time

Churn: the plan-time question that would have caught R1.1 is the cold-reader
rationale test - the Note asserted the comment's claim was true instead of
stating why, so nobody re-derived it. A port plan should say "audit the source
against the new callers" rather than "port verbatim"; this crate adds a third
collector (`nova_probe`'s `capture`) that the upstream never had, which is
exactly the case the bug scales with.

Breadth: no split was missed - the diff is one module, as planned.

Context: no pressure observed. No checkpoint, no compaction warning, no
delegation.

## Action items

- Fix-forward the same two defects upstream in
  `/home/alex/personal/bevy-common-systems/src/completion.rs`: the
  per-registrant `add_systems` and the deadline test that never asserts the
  naming. Not a nova-protocol task; recorded here so the source is not left
  wrong.
- Remaining ports in epic `20260802-120019` should carry an explicit
  "audit against the new callers" step, not "port verbatim".
