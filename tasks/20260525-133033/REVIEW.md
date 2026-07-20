# Review: rustdoc umbrella - crate-level docs + convention + enforcement decision

- TASK: 20260525-133033
- BRANCH: docs/rustdoc-pass

## Round 1

- VERDICT: APPROVE

The umbrella's three deliverables landed; a strict `cargo doc` run turned up a
real finding that the task now routes correctly.

- **Crate-level docs are accurate, not invented.** Verified every referenced
  type against code before writing: `AppBuilder` (nova_core), `NovaGameplayPlugin`
  + `GameStates` (nova_gameplay), `NovaScenarioPlugin` (nova_scenario), and the
  module lists match each crate's actual `pub mod`s. Distilled from the AGENTS.md
  table + architecture wiki rather than duplicating them. 7 crates written/
  expanded (the 4 with no header + 3 too thin); the 8 already-adequate headers
  left alone.
- **Convention recorded** in AGENTS.md "## Conventions" - crate-level `//!`;
  `///` what-and-why on public items; intra-doc links for reachable types + wiki
  links for concepts; runnable examples not required per item; missing_docs
  per-crate-as-clean; keep `cargo doc` warning-free.
- **Enforcement decision made AND proven.** `#![warn(missing_docs)]` per crate
  as it comes clean (not workspace-wide - a blanket turn-on is the 133032 push).
  `nova_info` wired as the exemplar and independently verified clean:
  `cargo check -p nova_info` and `RUSTDOCFLAGS="-D warnings" cargo doc -p
  nova_info` both emit zero warnings, so the lint genuinely passes (not just
  present).
- **Honest re-scope of the "clean cargo doc" DoD.** The strict run surfaced 108
  PRE-EXISTING per-item broken intra-doc links (nova_gameplay 78, others 30) in
  existing `///` docs - not from this task. Fixing them is per-ITEM work that
  belongs to the per-crate passes (nova_gameplay -> 133030, rest -> 133032), so
  the umbrella records the count/breakdown and routes it rather than pre-empting
  those tasks. `cargo doc` BUILDS (exit 0); full warning-free is the strand's
  end state, and nova_info is the first crate there. This is the umbrella doing
  its "sequence the per-crate work" job, not a corner cut.

- [ ] R1.1 (NIT) The 108-link finding is recorded in THIS task; the child tasks
  133030/133032 on master do not yet reference it. A one-line pointer into each
  (nova_gameplay's 78 -> 133030, the 30 others -> 133032) at land time would
  make the routing actionable from those tasks. Cheap; do at land or as the next
  cycle's first act.
