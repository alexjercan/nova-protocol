# Review: KISS pass on nova_assets - split the multi-concern files, cut comment fluff

- TASK: 20260731-170409
- BRANCH: refactor/kiss-nova-assets

Worktree: `/home/alex/.cache/sprouts/nova-protocol/refactor/kiss-nova-assets`
Head `c0c2431e`, base `master` @ `e038c34e`.

## Round 1

- REVIEWER: primary, in session
- VERDICT: APPROVE

Exception to the out-of-context default: this session runs under a standing
"no subagent dispatch" directive, so no independent reviewer was available.
Compensated mechanically - the no-behavior-change claim was re-derived from the
source rather than read off the diff (see Method).

### Method

The task's central claim is "moves, renames, deletions only; no behavior
change" over a 8.3k-line diff, which reading the diff cannot settle. Instead
both trees were reduced to a comment-stripped, whitespace-normalized multiset
of source lines over `crates/nova_assets/**/*.rs` and differenced:

- Whole crate, `master` vs `c0c2431e`: the residue is exclusively `mod`
  declarations, `use` lines, `pub use` re-exports, the visibility widenings
  listed below, and rustfmt re-wrapping caused by changed indent depth. No
  statement, expression, literal or signature is added or removed.
- `scenario/shakedown.rs` vs `shakedown/{mod,tests/*}.rs` on its own: residue
  is 3 rustfmt re-joins plus `mod pins/walk/tests` and `use super::*`.

Visibility changes, all narrowing-safe (private -> `pub(crate)` or `pub(super)`
for the new module boundary; nothing reaches the public surface): the four
`collections.rs` systems, and 18 `portal` items. The `pub` surface is
re-exported verbatim from `portal/mod.rs` and `lib.rs`.

### Findings

**MINOR - dead lint suppression added.** `crates/nova_assets/src/portal/mod.rs:107`
`#[allow(missing_docs)]` on `PortalChannel`. Not present on master (the struct
was private there) and not needed now - `pub(super)` is not publicly reachable.
Verified by deleting the line and running `cargo check -p nova_assets`: no
missing-docs warning. A pass whose mandate is deletions should not leave a
suppression behind, and a stale `allow` silences the lint if the struct is ever
promoted. Change: delete the line.

**MINOR - test fixture duplicated by the split.**
`crates/nova_assets/src/portal/catalog.rs:302` and
`crates/nova_assets/src/portal/install.rs:622` now both define `fn entry(id,
version, bundle, paths) -> PortalEntry` with identical bodies; master had one.
The shakedown split solved the same problem correctly by hoisting shared
helpers into `shakedown/tests/mod.rs`. Change: hoist one copy into a
`#[cfg(test)] pub(super) mod` under `portal/mod.rs` and import it from both, or
record in NOTES.md why portal was handled differently.

**MINOR - record undercounts the DoD grep.** `tasks/20260731-170409/NOTES.md:61`
and TASK.md's close-out both say the grep returns 6 hits. It returns 7 -
`collections.rs:236` is in NOTES.md's marker table but excluded from the count.
Every hit is a deliberate `TODO(...)`, so DoD 3 holds; only the number is
wrong. Change: say 7.

### Proofs re-run

| Proof | Result |
| --- | --- |
| DoD 1 `cargo check --workspace --all-targets` | green (4 pre-existing `nova_gameplay` ambiguous-import warnings, untouched by this diff) |
| DoD 2 `cargo fmt --check` | clean |
| DoD 3 HUID grep | 7 hits, all `TODO(<id>)` |
| DoD 4 `wc -l` | max 1221 (`shakedown/mod.rs`); nothing over 1500, so the NOTES.md exception clause is unused |
| DoD 5 existing tests | claim accepted: the shakedown test bodies are line-identical to master (see Method), so `an_early_derelict_kill_skips_to_the_fight` cannot have been introduced here. Correctly filed as 20260801-122138 rather than fixed. |

### Verdict

VERDICT: APPROVE. No BLOCKER or MAJOR. The three MINORs are non-blocking; the
first two are worth folding into the epic's next crate pass if not fixed here.

Pending user check: DoD 6 - owner skims the diff and agrees no behavior
changed. The Method section above is the evidence offered for it.
