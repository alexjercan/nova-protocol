# Retro: rustdoc umbrella (crate-level docs + convention + enforcement)

- TASK: 20260525-133033
- BRANCH: docs/rustdoc-pass
- REVIEW ROUNDS: 1 (APPROVE, one NIT - superseded by the dedicated cleanup task)

## What went well

- **Surveyed before writing.** A one-shot grep of each crate's `//!` line count
  found exactly which crates needed work (4 with none, 3 too thin) and which
  were already fine (8), so the pass touched only what needed it instead of
  rewriting good headers.
- **Verified every referenced type against code.** `AppBuilder`, `GameStates`,
  `NovaGameplayPlugin`, `NovaScenarioPlugin` and the module lists were confirmed
  in the source before being named in prose - no invented API names in a doc
  that is supposed to be the API's source of truth.
- **Proved the enforcement path, did not just assert it.** Instead of only
  writing "missing_docs per crate as clean", wired `nova_info` with the lint and
  independently verified it emits zero warnings (`cargo check` + strict
  `cargo doc`), so the exemplar genuinely holds.
- **The strict `cargo doc` run earned its keep.** `RUSTDOCFLAGS="-D warnings"`
  turned a vague "is it clean?" into a hard count: 108 pre-existing broken
  intra-doc links with a per-crate breakdown - which became actionable scope
  instead of a lurking unknown.

## What went wrong

- The task's DoD ("cargo doc warning-free") assumed a mostly-clean surface; it
  wasn't (108 warnings). Not a failure of this cycle - the strict run is exactly
  what exposed it - but it meant the umbrella could not honestly claim the full
  DoD without pre-empting the per-crate tasks. Re-scoped transparently and
  surfaced to the user, who chose a dedicated cleanup sweep.

## What to improve next time

- For a "make cargo doc clean" DoD, run the strict `RUSTDOCFLAGS="-D warnings"`
  measurement FIRST (before writing any docs), so the true size of the existing
  debt is known up front and the task can be scoped against it rather than
  discovered mid-flight.

## Action items

- [x] The 108-link finding is recorded with a per-crate breakdown; the user
  opted for a dedicated cleanup sweep (filed + flowed next), which supersedes
  NIT R1.1's "point the child tasks at it".
- No new ledger slug: the "measure the existing debt before scoping a clean-it
  DoD" note is a specific instance of reproduce/measure-first; recorded here.
