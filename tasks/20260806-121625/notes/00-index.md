# Notes index - task 20260806-121625

Knowledge captured during the understanding phase. Evidence from seven parallel
audits, 2026-08-07, against HEAD `4a8b55aa` (clean tree).

**Citations are dated.** Verify any `file:line` against the current tree before
relying on it.

| File | Holds |
| --- | --- |
| `01-decisions.md` | every owner ruling from the Q&A, verbatim where it matters. **Read first.** |
| `02-workspace-map.md` | crate sizes, dependency graph, preludes, feature flags, merge candidates |
| `03-nova-gameplay.md` | module map, the four seams, back-edge evidence, coupling, size outliers |
| `04-nova-probe.md` | the two-programs reality, capabilities, wasm gates, what the owner's sketch got wrong |
| `05-assets-scenario.md` | module maps, the authoring-toolchain split, wasm/IO gates, boundary leaks |
| `06-ui-layer.md` | nova_ui / nova_menu / nova_editor / nova_os, duplication sites, the NOVA OS ownership smear |
| `07-comments-and-docs.md` | the comment measurement that rejected the original premise. **Read before proposing any comment rule.** |
| `08-tests-ci-risk.md` | test inventory, CI gaps, churn, the refactor risk register |

Code review, 2026-08-07 - six parallel reviewers plus a clippy audit. These
record **defects that exist today**, independent of the refactor:

| File | Holds |
| --- | --- |
| `09-clippy-and-lints.md` | clippy at three configurations; the two CI blind spots, measured. **Plain clippy is already clean - `-D warnings` is free today.** |
| `10-review-hud-nova-os.md` | HUD and input. Two player-visible NOVA OS bugs confirmed against `bevy_ui` source |
| `11-review-assets-scenario.md` | **the mod data-loss cluster** plus four unbounded-input paths reachable from an untrusted portal catalog |
| `12-review-ui-layer.md` | two features proved dead (`Tween`, `StatusBarStore`); nova_editor is the weakest crate by defect density |
| `13-review-cross-cutting.md` | workspace-wide pattern sweep with counts. **Read before proposing any lint rule** - most suspected patterns are clean |
| `14-review-flight-sections.md` | flight, physics, camera, sections, audio. The simulation core is sound; the section layer is not |
| `15-review-probe.md` | nova_probe, nova_autopilot, nova_events, examples. **The CI gate is blind in four ways, three failing open.** Read before planning any lane order |
Absorbed into the tree, 2026-08-07:

| File | Holds |
| --- | --- |
| `16-findings-master.md` | **the one ranked list a planner works from.** 86 deduplicated findings, every `file:line` re-verified against the tree, ranked by expected harm. Ends with 17 corrections/withdrawals and a "do not re-audit" list |
| `17-lanes.md` | the 12 lanes, each with dependencies, verification and a **BLOCKS BASELINE / NEUTRAL** marking. Names 14 clusters of findings that are cheaper together than apart |

`00` through `08` were written before the review and now carry dated
amendments where it contradicted them. **The original claim is always still
visible** - marked `Corrected 2026-08-07` or `WITHDRAWN` beside the evidence
that settled it. Amended: `02` (visibility, two dead features), `03` (the
seams now carry known bugs), `04` (probe loader confidence), `05` (wasm rot
withdrawn, defect summary), `06` (scroll clamp count, nova_editor), `07`
(independent corroboration, `#[expect]`), `08` (CI gaps measured, risk
register re-ranked).

Implementation outlines, 2026-08-07:

- `../plan/` - one file per lane (`lane00.md` .. `lane11.md`), each naming the
  structs, function headers, modules and deletions that lane lands.
  `../plan/README.md` records the re-review that closed six gaps in `17`.

Parent records:

- `../NOTES.md` - problem statement, success criteria, constraints, ranked ideas
- `../CONVENTIONS.md` - the Rust house style, 12 rules, **all ruled 2026-08-07**.
  Every rule carries a measured violation count and the lane it lands in
- `../benchmark/README.md` - benchmark protocol
- `../question-set-prompt.md`, `../conventions-prompt.md`, `../review-prompt.md` - handoffs

## The one-paragraph version

~155k LOC across 16 crates. Half of it is `nova_gameplay`; 43% of that crate is
`hud/`, which contains a 14.3k-line terminal runtime that is not a HUD. The
orientation cost is the problem. The comment-noise premise was measured and does
not hold - 83% of comments are why-comments. The real prose problem is volume
and staleness, and `AGENTS.md` itself is measurably wrong. `nova_probe` is two
programs separated by a process boundary that no module name states.

The code review added a second axis. The **simulation core is sound** - flight,
physics, integrity and gravity were audited deeply and came back clean, and no
reachable `unwrap`/indexing panic exists in non-test code anywhere (four
independent confirmations). What is defective is the layer above it: mod
persistence can permanently lose a player's installed mods, mod content is
untrusted input reaching uncapped loops and unbounded recursion, two features
are entirely dead, and four crates repeat the same stale-`Local<T>` and
unguarded-per-frame-write mistakes. Plain clippy is already clean, so `-D
warnings` costs nothing today.

The third axis is **verification**. `nova_probe` is the CI gate, and it is
blind in four ways - three of which fail OPEN, so a run can verdict OK when it
should FAIL. Those fixes come before everything else in the epic, because every
other lane is verified by the gate they repair. See `16-findings-master.md`
for the ranked list and `17-lanes.md` for the order.
