# Dev book audit (`docs/` -> `/dev/`)

Audited every chapter against the tree at `8bc576f9`. No cargo was run: source
was read with `rg` and Read, `mdbook build` compiles no Rust. `web/src/` was
READ to run the surface check below, and never written.

Two passes: a correctness pass (does every path, symbol and claim resolve) and
a surface pass against `CONVENTIONS.md` Documentation 4 as it now stands -
**can this page's reader reach the thing it describes?**

## Per-chapter verdict

| Chapter | Verdict | Evidence |
| --- | --- | --- |
| `SUMMARY.md` | fixed | added the `performance.md` entry; every other chapter already listed |
| `README.md` | fixed | dropped the "old ephemeral-docs model is retired" history, which was also duplicated verbatim in `keeping-docs-in-sync.md` |
| `introduction.md` | fixed | reading order now points at the performance chapter; everything else re-derived and correct |
| `concept-index.md` | fixed | probe row split into three (contract/report, frame cost + census, world snapshot); `DAMAGE_PER_UNIT_VOLUME` 8 hp/unit^3 confirmed against `carve.rs:83` |
| `project-tour.md` | fixed | `nova_probe` line named three capabilities, tree has six; crate map and change-X table otherwise resolve |
| `keeping-docs-in-sync.md` | fixed | dropped two task citations and the retired-docs-model history; probe row split; new row for frame-cost work; "check means re-derive" kept, de-anecdotalised |
| `development.md` | rewritten (split) | 1,098 -> 862 lines. Measurement material moved to `performance.md`; six stale claims fixed (see below); three `systems/` ranges were missing from a list that claims to be "what is on disk today" |
| `performance.md` | NEW | the measurement chapter, composed by MOVING landed material out of `development.md` plus the fixed-step amplifier derivation |
| `architecture.md` | fixed | six defects: capability list, `HealthDisplay`, the `nova_authoring` builder paths, `mesh/explode.rs`, the pause mermaid missing `NovaOs`, three task citations |
| `sections.md` | holed -> filled | the `explode.rs` bullet described a mechanism that no longer exists and carried an explicit HOLE. Rewritten as a new "How a body comes apart" section. `FragmentMaterial` fixed. Everything the brief said was current (attitude envelope, `SECTION_CRACK_BUCKETS`) spot-checked and IS current |
| `scenario-system.md` | rewritten (surface) | 718 -> 546 lines. Four sections were re-stating the authored contract `/create/` owns; see the surface pass. Also: wrong test name, two task citations. The carve section is pure mechanism, re-derived against `asteroid_carve.rs`, and is current |
| `automation-harness.md` | fixed | "the table above is the whole contract" was false; three history passages removed; Xvfb pointer added. The env table itself, the snapshot section and the scripting rules are current |
| `guide-add-section.md` | fixed | step 6 named a function that no longer exists and a match that moved crates; step 8 now states the `catalog_ids` rule |
| `guide-extend-scenarios.md` | current | every path, symbol and recipe re-derived against `nova_events`, `events.rs`, `filters.rs`, `actions/`, `objects/`. No changes |
| `ship-layout-sense.md` | fixed | dropped the dead branch name and two task-file citations. `Part::aim`, `AFT`, `seed_stern`, `SPIKE_SUPPORT`, `erode_studs`, `VACUUM_BOW_TAPER` all still present; items 1 and 2 are still DONE and 3 still not landed |

## Stale references found (defects)

Line numbers are against the pre-fix files.

| Location | Was | Should be |
| --- | --- | --- |
| `sections.md:325-338` | "`explode.rs` - debris, mesh fragments"; geometry walk in `mesh/explode.rs`; `ExplodeFragments`; and an explicit HOLE saying the mechanism "is being replaced" | `explode.rs` DETACHES the body whole (`detach_destroyed_body`, `DetachedPieceMarker`). `mesh/explode.rs` and `ExplodeFragments` do not exist. The hole is filled |
| `sections.md:791` | a section keeps a `FragmentMaterial` | `SectionCracks.source`, a strong `Handle<StandardMaterial>` (`damage_cracks.rs:168`) |
| `architecture.md:117` | the generic `HealthDisplay` bar | gone from the tree; `nova_ui::status_bar` is the generic readout |
| `architecture.md:322-323` | builders in `nova_authoring` (`sections.rs`, `scenario.rs`, `scenario/`) | `src/base_content/` -> `sections/`, `ships/`, `scenarios/`, `styles.rs`, `campaigns.rs`, `assets.rs`. All three named paths are dead |
| `architecture.md:18` | `mesh` (mesh slicing) | `mesh/` is `builder.rs` + `field.rs`; nothing there takes a finished mesh apart |
| `architecture.md:33` | `nova_probe` capabilities: frametime, timeline, invariants | six: + `snapshot`, `census`, `framecost` |
| `architecture.md:107` | generic helpers include "mesh explode" | the mesh builder and the signed field |
| `architecture.md:191-212` | pause mermaid shows only `Unpaused`/`Paused` | `PauseStates` has a third variant `NovaOs` (`nova_gameplay/src/lib.rs:134`), which the prose eight lines above already documented |
| `development.md:275-278` | every example wires `nova_timeline()` + `nova_invariants()` | every example adds `NovaProbePlugin` |
| `development.md:710-717` | the frame-time claim is opt-IN; "the four `stress_*` ranges are what wire it today" | opt-OUT: `NovaProbePlugin::default()` wires it, `without_frametime()` declines. 5 stress ranges exist, and the wirers are most of `systems/` plus `carve_asteroids` and `wfc_arena`. The same page contradicted itself 120 lines later ("the whole fleet now carries the capture capability") |
| `development.md:826-830` | "Measuring a named SHIPPED scenario is UNDOCUMENTED until a `probe` subcommand loads one from its `.ron`" | `probe scenario <id\|file.ron>` exists and is documented on the same page. A hole that had been filled |
| `development.md:1036-1042` | wasm CI job is `cargo check ... under RUSTFLAGS=-D warnings` | `cargo clippy --target wasm32-unknown-unknown -- -D warnings` with `CLIPPY_CONF_DIR=ci/wasm-clippy` - the panicking-std ban list |
| `development.md:177-218` | the `systems/` roster, "what is on disk today" | missing `bug_carve_apply`, `system_blast_penetration`, `system_section_severing` |
| `development.md:981` | one post per feature release "(`v0.1.0` ... `v0.9.0`)" | enumerating shipped versions in a rule; now `v0.X.0` |
| `scenario-system.md:491` | test `wrecking_after_the_win_declares_nothing` | `a_wreck_after_the_finish_declares_nothing` (`crates/nova_assets/tests/scenario_gate_course.rs:399`) |
| `guide-add-section.md:126` | editor arm `placement_rotation` (rotation from the surface normal) | gone. The pose comes from `snap_placement` mating link points; the kind arm moved to `insert_preview_section` in `preview.rs` |
| `automation-harness.md:71` | "The table above is the whole contract" | false: it lists 7 of the ~20 `NOVA_*` variables the workspace reads. Now scoped to the driver contract, with the `NOVA_PERF_*` table routed to `nova_probe` rustdoc |

Removed as HISTORY (Documentation 1 and 3, Comments 3 and 4) rather than as
factual errors: the examples-category dissolution narrative, the retired
`sweep|web|profile` probe aliases, the shared-helpers env rename, the deleted
`ScreenshotReelPlugin`, the retired `com_range` example (kept as an
anti-pattern, lost its migration framing), the vendored-helpers repo, the
retired ephemeral-docs model (twice), and all eleven `tasks/<id>` citations.

Checks that came back CLEAN, so they are not defects:

- Every `crates/`, `examples/`, `scripts/` path in the book resolves.
- 613 backticked symbols swept against the tree; the only misses are a Linux
  sysctl, a Bevy renderer internal, a deliberate `Shield` placeholder and a
  test filename `rg` cannot match by content.
- Every `NOVA_*` variable named in the book exists in the tree, including
  `NOVA_STRESS_PD_BAYS`, which is built at runtime from a format string.
- Every internal link and heading anchor resolves (146 anchors, 0 broken).
- `docs/` is pure ASCII.

## Surface pass: can this page's reader reach it?

The dev book's reader is a contributor with the repo, so the
`/wiki/`-shaped failure (documenting something the reader cannot run) barely
applies here - swept for it and found NOTHING. No `<kbd>`, no
"press X", no player-experience prose in any chapter.

The MIRROR failure is the one this book had: restating the authored contract
`/create/` must hold exact. All of it in `scenario-system.md`, and it is
substantial - the chapter's own opening line already promised `/create/` owned
the catalog, and then carried the catalog anyway.

| Section | What it duplicated | Fixed by |
| --- | --- | --- |
| `## Events` (a 12-row firing table) | `/create/events.md`, one `##` per event | the identity/pair-shape/one-shot-edge mechanism, plus a routing table |
| `## Filters` (4 variants with their fields) | `/create/filters.md`, one `##` per filter | one paragraph: read-only, `&NovaEventWorld` + `GameEventInfo` -> bool, all must pass |
| `## Actions` (24 of 25 actions with RON) | `/create/actions.md`, one `###` per action | "What an action does that its RON cannot show" - the four with engine behaviour behind them (`Outcome` pausing, `SetCamera` authority, `SetSkybox` deferred install, `NextScenario` lingering) |
| `### Transition pacing (the three gears)` | `/create/actions.md#nextscenario`, same three names | "Two clocks pace a transition" - the delay on `Time<Virtual>`, `auto_advance_secs` on the wall clock BECAUSE the overlay stops virtual time |
| `### Story pacing` (RON, `dwell` clamp, card budget) | `/create/actions.md#storymessage`, same fields | "Story pacing is a QUEUE" - the HUD edge and the scenario scoping |
| `### Typed queries and watched variables` (RON, beat and wave shapes) | `/create/expressions.md`, same snippets | three mechanism facts: the watch owns the name, the clock exists whether or not it is exposed, unavailability fails closed |
| `## Scenario objects` (7 kinds with authored fields) | `/create/objects.md`, one `##` per kind | what the modules share (`base_scenario_object` carries no body) plus three facts the configs do not show |

**The drift the rule predicts had already happened.** `/create/actions.md`
documents `NextScenario`'s `delay` field and `Outcome`'s `auto_advance_secs`
cap of 300 s; the dev-book copy of the same three gears had neither the cap nor
the field table, and its `StoryMessage` copy was missing both lint warnings
`/create/` carries. Both copies still read as true - the defect was that
nothing said which one to believe.

Kept deliberately, because they pass reachability and have no other home:
the gate-counter, act-gating and Gauntlet PATTERNS. They compose the authored
vocabulary rather than restating it, they reference `webmods/gauntlet` and
`crates/nova_assets/tests/scenario_gate_course.rs`, and neither `/create/` nor
`/wiki/` carries them.

A verbatim line-level diff of `docs/` against every `/create/` and `/wiki/`
page now returns ZERO shared lines. Worth saying that it returned zero BEFORE
these fixes too - every duplication above was reworded, not pasted. A line
diff does not find this class; comparing the two surfaces' HEADINGS does.

### The WFC arena material offered from the wiki: DROPPED

The NOVA-OS-freezes-the-arena content deleted from `/wiki/nova-os/` passes the
dev book's reachability test - a contributor can run the example - but no
chapter wants it. `development.md` gives each of the ten `playable/` examples
one clause; three sentences on one example's pause semantics would be out of
proportion. It also already has a home in the source that a contributor reads
first: `examples/playable/wfc_arena.rs:85-86` states the Escape/NOVA OS/rebind
behaviour, and `examples/playable/wfc_arena/result.rs` pins it with
`nova_os_owns_the_cursor_and_match_clocks` and
`leaving_nova_os_resumes_match_clocks`. Homeless on the wiki is not a reason to
file it here.

## The performance chapter: YES, and why

A new chapter, `docs/performance.md`. Two things decided it:

1. **The distinctive material has now landed.** When the audit started, census
   and ablation were on a throwaway branch and rule 1 said do not document
   them. `1dc74718` landed `capabilities/census.rs` and `framecost.rs` and the
   `NOVA_PERF_RENDER_DIAG` gate. Together with the repeat set and its validity
   band, the fixed-step pin and the Xvfb constant, that is a methodology and
   not a knob list.
2. **`development.md` was carrying it badly.** At 1,098 lines it was by far the
   largest chapter, and its measurement half had drifted into contradicting
   itself within 120 lines. The split is by READER TASK: `development.md` keeps
   the gate you run before landing (verbs, run dir, profile sandbox, timeline,
   the report), `performance.md` takes the instrument (capture, repeats, fixed
   steps, Xvfb, frame cost and census, presets, the profiled pass).

It is a MOVE, not a rewrite: the prose came across as written, with stale
claims fixed on the way. One thing was added that was not in either place - the
fixed-step amplifier derivation, `F = B / (1 - s/T)`, which explains why
`NOVA_PERF_MAX_DELTA` exists. That mechanism is entirely in the tree; only the
derivation was missing, and `20260820-003401` asked for it to live in the
harness docs rather than one lane's notes.

The Xvfb constant is documented ONCE, in `performance.md`. `development.md` and
`automation-harness.md` each carry a one-line pointer at the place a reader
first meets a headless run - not a paste of the numbers. `dccf0d01` landed the
other half of that advice and it is documented beside it: an armed run wears
`nova_core::MEASURE_WINDOW_CLASS`, so a real-display measurement can be placed
off the working desk by a window-manager rule instead of being pushed onto a
software X server.

## Deliberately NOT documented

- **The ablation switches** (`ABL_NOGATE`, `ABL_PEACE`, the typo-must-fail
  rule). Task `20260820-003401` is still OPEN and they are not in the tree.
  Documenting a switch nobody can set is rule 1.
- **The interleaved paired protocol** (fresh reference capture before every
  arm, ratio of medians, a spread straddling 1.00 has measured nothing).
  Real methodology, but no tool implements it: it was hand-run on
  `arena-ablation`. When it lands as a probe capability it belongs in
  `performance.md` beside the repeat set. Left in
  `tasks/20260819-173219/NOTES.md` and `notes-ablation.md` until then.
- **Every measured number from this epic** - 10.37 ms/ship, the 16.74 ms floor,
  the 4v4 and 1v1 budgets, D1-D11. These are a moving investigation, several
  have already been retracted (`197fc1bd` retracted the floor), and a book that
  quotes them goes stale the next time anyone measures. The book documents the
  INSTRUMENT; the numbers live in the task.
- **A `NOVA_OS_*` env table.** There are ~90 of them, all CRT tuning knobs.
  They belong in `nova_os_ui` rustdoc, not in a chapter.

## Belongs on another surface - NOT fixed here

- **`CHANGELOG.md`.** The `[Unreleased]` Internals & Tooling section documents
  `probe run --repeat`, the fixed-step ceiling and the `screenshots/` naming
  rule, but has NO entry for: the world-state snapshot capability
  (`NOVA_PERF_SNAPSHOT`), the scene census, the frame-cost breakdown, or
  `NOVA_PERF_RENDER_DIAG`. Four contributor-visible tools with no line. The
  Xvfb finding needs no entry - it is a measurement fact, not a change.
- **`CHANGELOG.md`, second item.** The `[Unreleased]` block has grown past 300
  entries across this cycle. Changelog rule 4 ("re-read the whole
  `[Unreleased]` block as ONE document") has not been applied in a while; at
  least the examples-category churn and the probe-verb churn look like
  candidates for rule 2 collapsing.
- **`/wiki/`.** Nothing found. The book carries no player-facing prose.
- **`/create/`, one gap the surface pass turned up.** `/create/objects.md`
  documents the `Asteroid` fields, but the dev book was the only place stating
  that a rock has NO `health` field on purpose and that `mass` is the body's
  `mu`, setting the SOI by `mu = soi_cutoff_accel * soi^2`. The dev book now
  keeps it as mechanism; whether `/create/objects/` gives an author the same
  sizing formula is worth one check by whoever owns that surface. Same question
  for the `Light`-or-black rule, which a creator hits on their first scenario.
- **`/create/sections/` cross-check.** `sections.md` documents `damage_effects`
  as authored CONTENT serialized into `base.content.ron`, vocabulary
  `Cracks`/`Sparks`/`Plume`, default `[Cracks]`. `/create/sections.md` has a
  "Damage effects" section; I did not diff their contents (not my surface to
  fix), so it is worth one comparison.

## Verification

- `nix develop --command mdbook build` - exit 0, no warnings beyond
  mdbook-mermaid's version note.
- Rendered HTML opened and read for `performance.html`, `development.html`,
  `architecture.html`, `sections.html`, `automation-harness.html`,
  `scenario-system.html`. Every mermaid block renders as
  `<pre class="mermaid">`; none fell back to a code block.
- Link and anchor sweep: 0 broken across every internal link and heading
  anchor.
- 574 backticked symbols swept against the tree; 4 known non-repo names left
  (a Linux sysctl, a Bevy renderer internal, the `Shield` placeholder, and a
  test filename `rg` cannot match by content).
- `docs/` is pure ASCII; no `tasks/<id>` citation remains.
- No cargo invocation of any kind.
