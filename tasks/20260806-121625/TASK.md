# Refactor nova_* crate for better structure and clarity

- PRIORITY: 40
- TAGS: v0.10.0, refactoring, project
- ACTIVITY: WORKING
- GATES: PLAN
- RESOLUTION: -

PROBLEM: The `nova_probe` crate feels messy and hacked together; I have the
same feeling of other crates, some feel a bit too coupled; There are useless
comment all over the code;

I personally see `nova_timeline`, `nova_invariants` and `nova_frametime` in a
`capabilities` module inside the crate. Then we would have a `trait Capability`
which would define the interface of a capability (mainly collecting evidence ->
TDB exact shape in a prototype during understanding phase). I also noticed
excesive use of `wasm/no-wasm` compiler gates. That makes me think that we can
extract the read/write into a module and abstract it away behind a `Plugin` or
a plain `struct` resource or something (TBD via a prototype). I would also
create an `evaluation` module that does the check runs and verifies the
evidence collected by the capabilities. This would produce a report. The final
step should be converting the report to html. We can probably refactor that
part of the code into a `report` module. Something that use read/write to write
the HTML or whatever we use.

In my mind I have these steps `collect evidence` -> `run evaluation` ->
`generate report`. Each example/bin that includes a Capability collects
evidence. But we also need to add the run evaluation plugin and generate
report one. These are obviously added via NovaProbe Plugin or something like
that.

I think we do not have a "nova probe" plugin so we should add one that manages
all these steps such that it is clear that this binary is being probed.

This is an example of my review of `nova_probe` crate, now it's your job to do
a full multi agent deep dive review of the other crates and the structure of
the project to build a better understanding. The goal of this task is "cleanup"
and "improvement" of the code, which is a hard problem in my opinion because we
need to define what that means; try to use my review of what improving means;
during the understanding phase ASK ME A LOT OF QUESTIONS about any decision you
might consider. Let's try to identify what `improving` the code really means,
because I don't want this to be just shuffling code around but still getting to
a actually good result -> better performance, easier to test, less code,
simpler code, - honestly I wouldn't be able to say that these represent
improvements. It's more about readability and being able to go through the code
structure fast and being able to tell what each module/system does from the
folder structure. Something else is code should be self documenting, keep docs
only for public APIs (make clippy happy). But in code comments should be kept
minimal and only for actually important things "comment why not what". First
step of understanding should be collecting all the context then figuring out
what to do with it.

## Steps

Lane order is the schedule. `L0 -> L2 baseline -> everything else -> final
run`. Behavior-only lanes (L1, L3, L4, L6, L11) run in parallel with owner
review time. Findings are `notes/16-findings-master.md` ids; per-lane detail is
in the linked file. Rationale is in `DECISION.md` and `notes/17-lanes.md` - do
not re-derive it.

WORKFLOW (owner directive, 2026-08-07): one SPROUT per lane, off master, to
keep each unit of work small. A lane LANDS on master as soon as its own steps
and proofs are green - do not hold lanes for each other. Review runs ONCE at
the end, over everything landed, not per lane. The task stays `WORKING` until
the last lane lands.

### Lane00 - "FIX THE MAP, CLOSE THE CI GAPS" - tasks/20260806-121625/plan/lane00.md

Blocks the baseline, lands BEFORE it. First commit of the epic.

- [x] Rewrite the `nova_modding` row in `AGENTS.md` - bundle merge, portal
      client and downloads all live in `nova_assets`; name what `nova_modding`
      actually owns.
- [x] Reword `AGENTS.md:102` so the `nova_events` line reads as the
      scenario/modding vocabulary, not a general no-direct-coupling mandate.
- [x] Add the LOC share to the `AGENTS.md` crate table so `nova_gameplay` being
      half the workspace is visible. Landed as a percentage share (54%) of the
      142,845 `crates/*/src` lines, and the table is now ordered by it.
- [x] Write repo-root `CONVENTIONS.md`: 12 imperative `##` rules, one real
      in-repo snippet and one or two sentences of rationale each, 120-150 lines.
      A rewrite of `CONVENTIONS.md` in this task folder, never a copy.
      **175 lines, not 150.** Twelve rules each carrying a snippet plus
      rationale, and the two mandated closing sections, do not fit 150 without
      dropping required content. Two tightening passes took it from 194.
- [x] Add the `## Tools that would undo these conventions` section
      (`wildcard_imports`, `redundant_pub_crate`, `needless_pass_by_value`,
      pedantic/nursery).
- [x] Add the `## Not yet true` section: rule 3 (80 sites, L5/L7/L8/L9/L10),
      rule 4 (36, with rule 3), rule 10 (84, L9 per seam), rule 1 (28, L5).
- [x] Shrink `AGENTS.md`'s `## Code rules` to a pointer at the new file.
- [x] F79 - add `#[cfg(feature = "debug")]` to the 11 dead items in
      `examples/sections/hull_section.rs:535,547,563`,
      `torpedo_section.rs:69,349`, `controller_section.rs:64`,
      `screenshots/screenshot_combat.rs:128,134`,
      `screenshot_sections.rs:199`, `systems/player_path.rs:55`,
      `sections/many_sections.rs:37`. No body changes.
      Two corrections to the plan, both from the compiler: the last file is
      `examples/stress/many_sections.rs` (an unused IMPORT, not a dead item),
      and `controller_section.rs:64` is the `Layout::B` variant, whose two
      ungated pattern uses and one `unused_mut` had to be gated with it.
- [x] Record in NOTES what `--features debug` actually builds while doing F79 -
      F52 in L5 is the same investigation from the other end.
- [x] F80 - convert the 38 `#[allow(clippy::type_complexity)]` sites to
      `#[expect(clippy::type_complexity, reason = "...")]`, modelled on
      `hints.rs:200` and `keybind_dock.rs:569,737,790`.
      **37 sites, not 38.**
- [x] Delete rather than convert the two known-stale suppressions at
      `ammo_readout.rs:325` and `ammo_readout.rs:510`.
      **Wrong on both counts, corrected on evidence.** All 37 were converted,
      then clippy reported **12** unfulfilled expectations. `:510` is stale;
      `:325` is NOT. The 12 measured-stale sites were deleted, 25 conversions
      survive. Detail in NOTES.
- [x] Gate `crates/nova_probe/src/report.rs` behind
      `#[cfg(not(target_arch = "wasm32"))]` beside its siblings at
      `lib.rs:82-109`. No code inside `report.rs` changes.
- [x] `MOVE tasks/20260806-121625/benchmark/` to `<root>/benchmark/`.
- [x] Replace the hardcoded `^tasks/` filter in `benchmark/sandbox.sh:38-42`
      `repo_files()` with one named exclusion list covering `tasks/` AND
      `benchmark/` - the single chokepoint for both the tar copy and `TREE.txt`.
- [x] `.gitignore benchmark/results/`, keeping `aggregate.json`,
      `aggregate.csv` and `report.html` tracked. The 9 already-committed
      transcript/payload files under `results/smoke/` were `git rm --cached`ed,
      or the new ignore would have been inert on them.
- [x] Run `./sandbox.sh build tree` and inspect `TREE.txt` - zero `benchmark/`
      paths. A wrong exclusion ships `keys/tier1.json` inside `blind`'s image
      and fails silently. 950 lines, 0 `^benchmark/`, 0 `^tasks/`, 0 `keys/`.
- [x] Add `-D warnings` to the clippy step at `.github/workflows/ci.yaml:70`.
      Free today - 0 warnings measured at that configuration.
- [x] Add the default-features CI job (`cargo check --workspace --all-targets`)
      AFTER F79 has landed. `-D warnings` arrives via `RUSTFLAGS`; `cargo
      check` takes no `-- <flags>`.
- [x] Add the wasm CI job (`cargo check --workspace --target
      wasm32-unknown-unknown`) AFTER the `report.rs` gate has landed.
- [x] Re-read `AGENTS.md` against `notes/02-workspace-map.md` and confirm every
      row is true. Two further rows were wrong and were fixed: `nova_ui` has
      zero `nova_*` deps (not just "must not depend on `nova_os`"), and
      `nova_modding` re-exports four named `nova_mod_format` items, not the
      crate wholesale as the note claims.

### Lane01 - "UNBLIND THE PROBE GATE" - tasks/20260806-121625/plan/lane01.md

NEUTRAL. Goes first among the code work; every other lane is verified by the
gate it repairs. No module renamed - that is L8.

- [x] Read `RunArtifacts::load`'s doc comment at `artifacts.rs:41-43` first -
      hard-erroring on a corrupt-but-present artifact is deliberate and must
      survive. Only the failure's SCOPE is wrong.
      The intent survives, moved into the `artifacts_loadable` check. The one
      remaining hard error is `--baseline`, and the doc now says why: it is an
      operator argument, not evidence the run produced.
- [x] F01 - add `pub struct ArtifactFailure { name, reason }` and a
      `failures: Vec<ArtifactFailure>` field to `RunArtifacts`.
- [x] F01 - add the `load_one<T>` helper and route every artifact through it;
      per-artifact parse errors degrade to `None` plus a recorded reason
      instead of `?`-propagating.
      **A `Loader` struct, not a free `load_one`.** It owns `dir` and
      `failures`, so the four threaded arguments collapse to two methods
      (`read` raw, `load` read+parse) - and the globbed cell logs can use it,
      which the free function's `raw: Option<String>` shape could not.
- [x] F01 - add `check_artifacts_loadable` to the checks roster so a present
      unloadable artifact FAILS the run rather than deleting the report.
      Landed as `checks/artifacts_loadable.rs`, LAST in the roster: it grades
      the evidence, so when it fails it is the reason the rows above read the
      way they do.
- [x] F03 - push `web-run.log` into `log_parts` at `artifacts.rs:65-70`; it is
      both chromium's output and the game's (`stats.rs:708` parses `nova perf:`
      out of an `INFO:CONSOLE` line).
- [x] F03 - forbid `log_clean` from SKIPPING on a run whose manifest says a web
      pass happened. A SKIP on a platform that produced a log is the defect.
- [x] F05 - add `stale_cell_logs(out) -> Vec<PathBuf>` beside
      `bin/probe/native/run.rs:29` and have `clean_out_dir` (`run.rs:43`) remove
      those too; the comment at `run.rs:26` already claims this happens.
- [x] F02 - rewrite `build_row` (`sweep.rs:181,187`) so a run with
      `run_error: Some(..)` cannot verdict better than ERROR.
- [x] F02 - add `RunStamp { git_sha, started_unix }` to `checks.json` and reject
      a stamp that is not this run's; `run_identity()` and `sweep` already hold
      both values.
      **Nothing new is written to disk.** Both values were ALREADY in every
      checks.json under `run`, from the manifest; `RunStamp` is a borrowed
      comparison type in `sweep.rs`. `probe report` re-renders history, so it
      passes `started_not_before: 0` and matches on the revision alone.
- [x] F04 - declare `ProbeRecorderSystems::RunEnd` in `nova_probe/src/recorder.rs`
      and `configure_sets(Last, RunEnd.after(AutopilotCompletionSystems))`.
      The edge needed a set on the OTHER side too: `AutopilotCompletionSystems`
      is new in `nova_autopilot/src/completion.rs`. `InvariantsPlugin` joins
      the same set - it drains `AppExit` as well, and either plugin can be
      armed without the other, so both call `order_run_end`.
- [x] F04 - write the test that FAILS if that edge is removed. The current
      behavior is accidentally correct on today's executor.
      `run_end_sees_the_exit_the_completion_watch_writes`: registers the reader
      FIRST and pins a `SingleThreadedExecutor`, so insertion order alone runs
      it before the writer. Fails without `order_run_end`.
- [x] F58 - replace the `if let Ok(..)` swallow at
      `nova_events_macros/src/lib.rs:37` and `:42` with a `compile_error!`, so
      `#[event_name = "x"]` stops compiling to the lowercased ident.
- [x] F63 - guard the empty-slice mean at `run_report/html.rs:217`; copy the
      guarded form already at `capture.rs:499`. It prints NaN today.
- [x] F70 - route the in-app CSV append at `capture.rs:522` through
      `append_frametime_row` (`stats.rs:415-426`) instead of re-implementing it.
- [x] F70 - re-assess AFTER F01 lands; fixing the bigger bug may retire it.
      Record the verdict either way.
      **NOT retired.** F01 downgrades the blast radius; it does not make two
      writers of one file - one of which knows a schema rule the other does not
      - correct. Detail in NOTES.
- [x] F71 - add `NOVA_PERF_CONTRACT` to the `env.retain` filter at
      `bin/probe/native/run.rs:180` so the fps pass stops rewriting
      `probe-contract.json`.
- [x] F76 - add `InheritedVisibility` to `ui_node_rect`'s query
      (`nova_autopilot/src/input.rs:135-151`) and reject hidden nodes; add
      `assert_named_visible`. Fix the harness, not the one example that noticed.
      **It broke 8 existing `nova_autopilot` tests**, all of which hand-spawn
      UI nodes in apps that run no visibility propagation. Fixed in the
      fixtures, never in the query. Four new tests pin the new behavior, which
      nothing covered.
- [x] F76 - re-point `examples/screenshots/screenshot_ui.rs:171` at the new
      assertion; `wiki-settings.png` currently ships as the bare main menu at
      exit 0.
- [x] F77 - extract `release_all_held_keys` and call it from
      `reload_the_run` (`player_path.rs:379`) as well as `replay_the_run`
      (`:537-543`).
      `replay_the_run` was only ever "release, then reload", so once the
      release moved into `reload_the_run` the wrapper held nothing. Deleted,
      and its one caller in `main` re-pointed.
- [x] F78 - gate `tag_gate` (`examples/sections/turret_section.rs:404-406`) on
      the spawner's own marker so the gravity planetoid stops being tagged a
      range gate; `report_status` prints 6 gates for 5.
      Gated on a `RANGE_GATES` roster const rather than a marker: the same
      roster now spawns the gates, so the spawn list and the tag list cannot
      drift apart at all.
- [x] Build the gate fixture suite: truncated `trace.json`, torn
      `timeline.jsonl`, non-UTF-8 `run.log`, `web-run.log`-only run, stale
      `run-<n>.log`, pre-existing `checks.json` plus an errored run. Each
      asserts the verdict the gate SHOULD produce.
      All six, beside the code they grade: 4 in `checks/artifacts_loadable.rs`,
      2 in `checks/log_clean.rs`, 1 in `native/run.rs`, 3 in `native/sweep.rs`.
      `fixtures.rs` grew `write_contract`/`write_manifest`, which three
      modules were each open-coding.
- [x] Byte-compare `probe run --all` verdicts before and after on a healthy
      tree - the fixes must not change a healthy run's answer.
      Before: master `e43d128f` in a throwaway sprout. After: this branch.
      24 rows each, same example set, ZERO verdict differences and zero status
      differences on any shared check. The one difference is the check L1 adds:
      `artifacts_loadable`, PASS on all 24 (measured 5/6 -> 6/7).
      Not clean on the first try - F78 turned `turret_section` FAIL, which is
      the false-invariant finding written up in NOTES. Fixed, then 4/4 repeats
      OK at the pre-F78 runtime before this sweep ran.

### Lane02 - "BUILD AND BASELINE THE BENCHMARK" - tasks/20260806-121625/plan/lane02.md

Not code. This is the gate that makes L5-L10 provable rather than churn.
Depends on L0 only. The owner starts and runs it.

- [x] OWNER - ratify `benchmark/keys/tier1.json` (30 locate questions).
- [x] OWNER - ratify `benchmark/keys/tier2.md` (3 design tasks + rubric).
      The `## Channel scope` table was added AFTER ratification, to fix the
      scoring bug below. It changes what Completeness counts, not a question.
- [x] OWNER - ratify `benchmark/keys/tier3.md` (mod brief + pass criteria).
- [x] Review `aggregate.py` tool-call counting against a REAL stream-json
      transcript; it has only run on synthetic data.
      Recounted `tool_use` blocks independently across all 17 baseline
      transcripts: **17/17 exact, 0 mismatches.**
- [x] Verify a persona can never be handed a paper for a question it was not
      asked - `make-papers.py --check` guards generation, not the wiring.
      Expected id set re-derived from the key per persona and diffed against
      the 5 generated papers: **0 leaked, 0 missing** (30/27/19/27/8).
- [x] Verify `grade.sh` never marks a persona down for a question it never saw.
      The persona-filter rule is implemented twice (`make-papers.py:151`,
      `grade.sh:59-66`) and nothing fails loudly if they drift; make one the
      source of truth.
      Clean in the stated direction. **The inverse defect was real and is the
      more damaging one:** the grader silently dropped `rustdoc`'s `t1-018`,
      and `aggregate.py` derived `asked` from the grades, so the denominator
      shrank to 26 and nothing failed. `asked` now comes from the key,
      `ungraded` is reported per row, and `aggregate.py` prints a loud
      UNGRADED QUESTIONS line. Filter deduplicated into `persona_filter.py`,
      imported by `make-papers.py` and shelled to by `grade.sh`.
- [x] Confirm `[source]` is unreachable from every `rustdoc` page after `src/`
      is stripped, not merely delinked.
      Unreachable: 0 `.rs.html`, 0 `src/` dirs across 5,599 pages. **But the
      hrefs survived**, and `../src/nova_mod_format/lib.rs.html#139` is a
      file:line answer at the grain tier 1 asks for - a readable map of the
      public API left inside the channel. The baseline did not exploit it
      (0 tool calls reference it), so the baseline stands. `stage_rustdoc` now
      rewrites those hrefs away, so the after-run cannot.
- [x] Confirm the `docs` image now picks up the repo-root `CONVENTIONS.md`
      landed in L0; a baseline without it under-measures `docs`.
      Present at 175 lines, with `AGENTS.md`, `README.md` and 32 wiki pages.
- [x] Use Claude on the host once before the batch - the container throws away
      its refreshed OAuth token and the host copy can go stale.
- [x] Smoke run: one persona, one paper, end to end. Nothing has run yet.
      Done; it produced the 9 `results/smoke/` files L0 `git rm --cached`ed.
- [x] OWNER - baseline: `./run.sh baseline all tier1`, then each tier 2 paper,
      then tier 3. 17 containers, exit 0, 0 network hits, $27.14, 59 minutes.
- [x] `./grade.sh baseline all`, `./aggregate.py baseline`,
      `./report.py baseline`; commit `aggregate.json`, `aggregate.csv`,
      `report.html`.
      **The commit half is void**, superseded by the `results/` gitignore
      ruling recorded later in `plan/lane02.md` and implemented at
      `.gitignore:258`. Nothing a run produces is stored; the baseline lives
      only on the owner's disk until `./report.py after baseline` renders both.
- [x] OWNER - run the `owner` persona by hand: the fixed 8-question tier 1
      subset plus one tier 2 task, timed.
      Tier 1: 0.75 (4 full, 4 partial). Tier 2a written and timed at 619s.
      Both transcribed verbatim beside the `.md` they were written in; the
      owner records elapsed time, not tool calls, so `tool_calls` is null
      rather than estimated.
      **This exposed that Cost of arrival was never computable.** It is defined
      as a ratio against the owner's tool-call count, and the owner works in an
      editor, so the denominator does not exist for any task - including the
      one now done. Eleven of twelve baseline graders said so in their own
      citations and improvised anyway, some defaulting to the 0.67 anchor,
      others scoring the respondent's count absolutely. A quarter of every
      tier 2 headline was a number nobody measured. The dimension is now null
      when unanchored and the headline is the mean of what was scored.
- [x] Record the baseline numbers in NOTES. Every structural lane is measured
      against them. `notes/18-benchmark-baseline.md`, indexed in
      `notes/00-index.md`. Findings `B1`-`B6` are what the after-run must move.

### Lane03 - "UNTRUSTED INPUT, DATA LOSS AND PERSISTENCE" - tasks/20260806-121625/plan/lane03.md

NEUTRAL. Depends on L1. Mod content is untrusted input: a reachable panic, OOM
or stack overflow is a defect, not an upheld invariant.

- [x] F06 - replace `read_index_at`'s `Option` (`mod_cache.rs:512`) with
      `enum IndexRead { Absent, Loaded(..), Corrupt(String) }`; `None` currently
      conflates "no index yet" with "corrupt".
- [x] F06 - make `install_local_at` (`mod_cache.rs:582`) refuse on
      `Corrupt`: side-band the file to `installed.mods.ron.bad` and return
      `Err`. Never clobber - it erases every other installed mod and orphans
      their bytes where `remove_mod` can never sweep them.
- [x] F07 - add `write_atomic(path, bytes)` to `nova_assets/src/persist.rs`
      (temp + fsync + rename), modelled on `nova_probe/src/recorder.rs:213` and
      `contract.rs:164`.
- [x] F07 - route the four bare `std::fs::write` sites through it:
      `mod_cache.rs:521`, `persist.rs:91`, `portal/catalog.rs:197`,
      `bin/content.rs:103`.
- [x] F07 - land it as the change that INTRODUCES L10's `Storage::write`
      contract, not a free helper L10 then has to absorb. See lane10.
- [x] F06+F07 - one kill-mid-write-then-install test covering both. They are
      one failure mode in two halves; neither fix alone stops the loss.
- [x] F22 - add a settings flush system in `Last`, ordered before the AppExit
      drain. `settings.rs:247` debounces 15 idle frames with no shutdown flush,
      and `menu_ui.rs:564` writes AppExit immediately.
- [x] F08 - bound `deps.rs:25 visit()` with `MAX_DEP_DEPTH = 64` and a
      `Result` return; it recurses over untrusted `catalog.json`
      (`install.rs:425`) before `validate_entry`'s caps, and a stack overflow
      ABORTS the process uncatchably.
- [x] F08 - add an entry-count cap to `PortalCatalog`; `MAX_FILE_COUNT` bounds
      files per entry, not entries.
- [x] F09 - RULED NOT A DEFECT while implementing, and pinned rather than
      capped. Every production RON decode goes through `ron::de::from_bytes`
      under `Options::default()`, whose `recursion_limit` is `Some(128)`, so
      deep nesting is a parse error and never a stack overflow. A second
      `MAX_EXPR_DEPTH` in `variables.rs`/`filters.rs` would be a redundant
      bound on an already-bounded path, so the ruling is held by a test that
      fails if that limit is ever lifted.
- [x] F12 - cap `ScatterObjectsConfig::count` (`spawn.rs:317`, field at `:244`)
      at `MAX_SCATTER_COUNT = 4096`, add the matching lint rule, and cap the
      `min_separation` rejection sampler's iterations (currently O(count^2)).
- [x] F13 - add `MAX_CATALOG_BYTES = 1 << 20` and bound the body before either
      of `catalog.rs:71`'s two parses, plus `MAX_CATALOG_ENTRIES` (the existing
      `MAX_FILE_COUNT` bounds files PER ENTRY, not entries). NOT at
      `transport.rs:31` as planned: `ehttp` buffers a whole response body
      before it calls back, so the transport never sees a bound-able read.
      `decode_catalog` is the earliest point the client controls, and the code
      records why.
- [x] F10 - apply `.max(f32::EPSILON)` at `turret_section/setup.rs:64`, the
      guarded form already present at `:192`. `fire_rate: 0.0` panics the
      instant the ship spawns.
- [x] F10 - lint `fire_rate` in `lint/ship.rs`, which lints the hinge axis and
      muzzle presence but not this.
- [x] F14 - log the serde failure at `engine.rs:170`; `data: None` reads as
      "does not match" in `filters.rs:71`, so an entity-filtered handler stops
      firing permanently and silently.
- [x] F56 - push undeclared-ref violations for EVERY content kind at
      `merge.rs:214`, not just `Content::Scenario`; the doc at `:145-148`
      already claims this.
- [x] F57 - replace the `HashMap` at `objects/binding_input.rs:83` with a
      `BTreeMap` or sorted-key `serialize_map`; same class at
      `lint_walk.rs:380,532`.
- [x] F57 - regenerate `assets/base/**/*.content.ron` via the builders plus
      `content -- gen` (never a hand-edit) AS ITS OWN COMMIT, so the generated
      churn does not hide a real diff.
- [x] F59 - use `get(index)` at `portal/mod.rs:176`; the `install.files.len()
      != index` guard does not bound `index` against `entry.files.len()`.
- [x] F60 - dedup `ids` before the `order.len() != ids.len()` cycle test at
      `deps.rs:104`, and reject duplicate ids in `mod_set.rs:222`.
- [x] F68 - membership-gate the `self://` rewrite at `mod_refs.rs:75` the same
      way `dep://` is gated. Defense in depth only.
- [x] F69 - key a failed dependency install at `portal/install.rs:459` under
      BOTH the dependency's id and the dependent's, so the UI has a surface.
- [x] F61 - implement an epsilon compare inside `variables.rs:270`'s `Equal`
      node. RULED: not a second `ApproxEqual` node, not a documented sharp edge.
      Pick the epsilon from what the DSL's values are and name it as a constant
      beside the node.
- [x] Build the hostile-RON corpus: malformed bundles, oversized catalogs,
      deeply nested DSL expressions, duplicate ids, degenerate `fire_rate`.

### Lane04 - "RECONCILER DISCIPLINE AND TERMINAL INPUT" - tasks/20260806-121625/plan/lane04.md

NEUTRAL. Depends on L1. MUST precede L9's NOVAOS seam move, or every citation
in `notes/10-review-hud-nova-os.md` has to be re-derived after 14.3k lines
shift. `hud/keybind_dock.rs` is the reference implementation - every fix here
is "make the site look like keybind_dock".

- [x] F19 - add an `Added<NovaOsTerminalRootMarker>` override to
      `rebuild_terminal_ui` (`hud/nova_os/shell.rs:344,363`) so a just-spawned
      terminal treats `last_len` as 0. Its two siblings at `:288` and `:320`
      already carry it; `reset_session` leaves auto-scroll dead for ~190 rows.
      **Keyed on `Added<NovaOsTerminalScrollbackMarker>`, not the root marker.**
      The `Local`s belong to the scrollback list this system writes; the root
      is a different entity and the two are not guaranteed to spawn on the
      same frame.
- [x] F18 - replace the `f32::MAX` sentinel at `shell.rs:379` with a real clamp
      at the point of writing. Bevy never writes the clamped value back, so
      PageUp after a command needs two presses.
      The clamp cannot happen at the write: the new rows are spawned as
      commands and have no `ComputedNode` until layout has run, so the maximum
      is not yet knowable. `SCROLL_TO_BOTTOM` stays the request, and the new
      `normalize_nova_os_scroll` converts it to the measured maximum, ordered
      `.before()` both the keyboard and wheel handlers so they subtract from a
      real number.
- [x] F40 - rebuild scrollback rows only when `scrollback().len()` or the tail
      content changed; caret movement must not reach the row loop
      (`shell.rs:344`, 4,800 entities per 12 keystrokes today).
      Done with a `scrollback_revision` counter on `NovaOsTerminal` rather than
      a len/tail comparison: every mutation funnels through one private method,
      so a rewrite in the middle of the buffer cannot slip past a tail check.
- [x] F40 - bound `scrollback` in `nova_os::Terminal`; nothing trims it today.
      `MAX_SCROLLBACK_ROWS = 500`. The field is now private, so the cap and the
      revision cannot be bypassed by a new caller.
- [x] F40 - route the prompt/hint/ghost writes at `shell.rs:385-400` through
      `set_if_neq`. `Text` is not `PartialEq`, so those four go through a local
      `set_text_if_neq`; the colours use `set_if_neq` directly.
- [x] F39 - guard the `node.width`/`node.height` writes in
      `reconcile_nova_os_target` (`hud/nova_os/crt.rs:219`); its only gate lives
      from ship spawn to despawn, so it runs every frame while flying over a
      subtree of hundreds of `Text` children. The `Visibility` write next to it
      was unguarded too and is now `set_if_neq`.
- [x] F42 - guard `node.left` in `position_nova_os_block_caret`
      (`shell.rs:442`).
- [x] F42 - guard the `TextColor`/`BorderColor`/`BackgroundColor` writes at
      `nova_os_ship/scene.rs:750,772`; the same function already guards its
      `Text` write two lines above.
- [x] F20 - key `play_safety_engaged_cue`'s `Local<bool>`
      (`audio/cues.rs:99`) on the ship `Entity`, or add an `Added<>` override.
      It is process-global today and contradicts its own doc at `:93`.
      Took the `Added<>` arm: the query reads `Ref<WeaponsHot>` and a component
      that `is_added()` seeds `was_hot` from its own state instead of edging
      against the dead ship's. Cheaper than a keyed map for a single player.
- [x] F75 - add `prune_dry_fire_state` for `play_dry_fire_cue`'s
      `Local<HashMap<Entity, bool>>` (`audio/cues.rs:147`), modelled on
      `mixing.rs:195 prune_sfx_throttle`.
      **No separate system.** `prune_sfx_throttle` exists because the throttle
      is keyed on sounds nothing else enumerates; this map is keyed on turrets
      the cue already visits in full every frame, so the loop rebuilds it and
      the live set is exact by construction. A second system would add a
      schedule edge to re-derive what the first one already knows.
- [x] F15 - skip `Key::Character(_) | Key::Space` when Control is held at
      `hud/nova_os/input.rs:267`, matching `handle_nova_os_app_keyboard`
      (`:355,:374`). Ctrl+C/U/W/A/K insert literal characters at the prompt
      today - the finding most likely to be hit by a real player.
- [x] F15 - add a shared `control_held(&KeyboardInput) -> bool`; three handlers
      now ask the same question. Implementing the chords themselves is OPTIONAL
      scope - decide and record.
      Signature is `control_held(&ButtonInput<KeyCode>)`, not `&KeyboardInput`:
      all three callers hold the resource and one of them (`ship_input`) has no
      `KeyboardInput` at all. **The chords are DEFERRED.** Not typing the letter
      is the whole defect; kill-line, kill-word and friends are a readline
      feature with their own design surface (word boundaries, an undo/yank
      buffer) and belong in their own task, not smuggled into a guard fix.
- [x] F34 - route `ship_input` (`nova_os_ship/scene.rs:397`) through the app
      router's Control guard instead of raw `ButtonInput<KeyCode>`; Ctrl+[ both
      exits the app and cycles selection back.
- [x] F33 - give `prompt_completion_ghost` (`nova_os/terminal/view.rs:222`) and
      `refresh_parse` (`edit.rs:338`) one shared accessor for "the prompt as
      parsed"; a leading space greens the prompt with no ghost.
- [x] F73 - collect, sort and dedup in `completion_matches`
      (`edit.rs:293`); Tab-cycle order varies between processes today.
      This changes the authored cycle order of the existing Tab test, which is
      the point: the old expectation was whatever the `HashMap` happened to
      yield.
- [x] F74 - add `MAX_HISTORY = 200` at `edit.rs:109` and skip a submit equal to
      the last entry.
- [x] F16 - change `return` to `continue` at `mesh/explode.rs:130` and `:144`,
      and fix the second `error!` to name `mesh_entity`. One still-loading
      `Mesh3d` currently produces NO fragments and leaves a zero-health wreck
      lingering with its collider live.
- [x] F21 - add `silence_loops_on_scenario_unload` on `OnExit`
      (`audio/loops.rs:188,313`); loop sinks are session-persistent and the
      engine hum roars through the whole scenario load.
      On `OnExit(GameStates::Playing)`. It despawns the sink entities and
      clears the smoothed volume maps - keeping the levels would bring the
      respawned loops back at the volume they died at.
- [x] F23 - home `update_target_position`
      (`torpedo_section/projectile.rs:37`) on `live_structure_anchor` rather
      than the target root's raw `Transform::translation`. `sections/mod.rs:38-43`
      states the rule and every other consumer follows it.
- [x] F41 - guard the unconditional `Text`/`TextColor` writes at
      `nova_ui/src/status_bar.rs:196`. F46 and F51 are deletions in the SAME
      365-line untested file and wait for the baseline - read the file once and
      hold that commit for L5's window.
      Guard landed here; F46/F51 untouched and still L5's.
- [x] Assert change-detection, not just behavior: run two frames with no input
      and assert the component is not marked changed on the second.
      `an_unchanged_status_item_is_not_rewritten` in `status_bar.rs` - the first
      test in that 365-line file. It counts `Changed<Text>`/`Changed<TextColor>`
      hits from a detector system chained after the reconciler; an out-of-schedule
      `is_changed()` on an `EntityRef` compares against the world's stale
      `last_change_tick` and passes either way (verified - the first draft of
      this test passed with the guard removed).
- [x] DECIDE the L7 escape hatch: if L2's ratification drags, land F17's unit
      conversion and F28's shrink clamp in place here (~10 lines, no file
      moved), or defer with reason. Owner's call.
      **DEFERRED to L7.** The trigger is false: L2 ratified, ran and baselined
      in full (`notes/18-benchmark-baseline.md`, findings B1-B6), so the hatch
      it exists to open is not needed. Landing F17 here would also put a unit
      fix on `max_nova_os_scroll_y` in the same commit that changes who calls
      it, for no schedule gain.

### Lane05 - "DELETE THE DEAD AND LYING SURFACE" - tasks/20260806-121625/plan/lane05.md

BLOCKS the baseline, lands AFTER it - deletion count is success criterion #2
and lines deleted before the baseline never enter the ledger. Depends on L2.

- [x] F45 - delete `crates/nova_ui/src/tween.rs` (421 lines, 11 tests), its
      `pub mod tween;` and prelude re-exports in `lib.rs`, and the
      `.add_plugins(TweenPlugin)` at `hud/mod.rs:301`. Zero consumers
      workspace-wide. DO THIS FIRST inside the lane - it makes F55 a two-plugin
      merge and retires `TweenSystems` before L9 counts rule 10's sets.
- [x] F46 - delete `StatusBarStore` (`status_bar.rs:131-136`) and its
      `init_resource` at `:153`. Declared, never read or written.
- [x] F51 - rebuild `insert_status_bar_item` (`status_bar.rs:238`) so the
      caller's entity is the live one; today the observer copies the data into
      a new child and leaves the caller's entity a permanent orphan with no
      `Node`. `nova_core/src/lib.rs:290,297` spawns two.
- [x] F47 - RULED make it real: gate hanabi (`plugin.rs:77`), skybox (`:85`),
      post (`:86`) and the HUD (`:111`) on `NovaGameplayPlugin::render`, so the
      documented headless mode exists.
      `GameObjectives` had to move OUT of the render-gated HUD and into
      `NovaGameplayPlugin` itself: the scenario loader writes it whether or not
      anything draws it, so a headless run panicked on the missing resource.
- [x] F48 - delete `objectives.rs:123 rebuild_lines` and `ObjectivesPlugin`;
      `ObjectivesPanelMarker` appears only inside `objectives.rs` and the live
      objectives HUD is `nova_scenario/src/loader/lifecycle.rs:49-63`.
- [x] F49 - either tag the spawner so `Without<SectionInactiveMarker>`
      (`torpedo_section/bay.rs:112`) is real, or delete it. It reads as a live
      safety gate and does nothing; a disabled bay keeps ticking to ready.
      Made real, not deleted. The marker only ever lands on the SECTION, so the
      liveness reads through `TorpedoSectionPartOf`; `a_disabled_bay_stops_rearming`
      asserts both arms so the disabled case cannot pass on a starved clock.
- [x] F50 - honor the `_skin` parameter in `panel_head`
      (`nova_ui/src/widget/panel.rs:112`); `:121` hardcodes the phosphor band.
      Deleting the parameter is the wrong fix - every call site believes it
      does something.
- [x] F52 - stop `nova_debug/Cargo.toml:18` and root `Cargo.toml:224` forcing
      `nova_gameplay/debug` on every test and example build. Read the L0 notes
      rather than repeating the investigation.
- [x] F52 - delete `nova_info`'s `debug = []` feature outright; zero cfg sites.
- [x] F54 - delete two of the three private `toggle_debug_mode` fns
      (`nova_debug/src/lib.rs:124`, `inspector.rs:180`, `wireframe.rs:66`). A
      fourth sub-plugin silently breaks F11.
- [x] F55 - add `pub struct NovaUiPlugin`, delete the first-caller-wins
      `widget::register` fn and `WidgetObserversRegistered`, and fold
      `StatusBarPlugin` (`status_bar.rs:147`) into it.
      The guard moved from a resource inside the crate to `is_plugin_added` at
      each of the three call sites, which is bevy's own idiom and visible where
      it matters. `StatusBarPluginSystems` became `StatusBarSystems` with the
      plugin it was named for (rule 9, for free).
- [x] Rule 2 - delete the 69 boilerplate `/// Glob-import surface: ...` prelude
      doc lines. Keep the 37 that say something specific
      (`nova_ui/src/lib.rs:24-31` is the model).
      **Replaced, not deleted, and the count is 69/105 not 69/106.** Every crate
      carries `#![warn(missing_docs)]` and L0 put `-D warnings` on CI's clippy,
      so a bare deletion breaks the build. Each of the 69 got the one-line
      "names its contents" form rule 2 itself prescribes; 36 specific docs kept.
- [x] Rule 1 - write the 28 missing module `//!` docs, each with a "touch this
      module when ..." line. COUNT THIS SEPARATELY - it adds lines and nets
      against criterion #2; report two numbers, not one.
      **38 modules, not 28.** The plan's count excluded the 10 `nova_menu`
      test modules and the two NOVA OS app `tests.rs` files; those are modules
      and rule 1 does not exempt them. `nova_info/build.rs` is the one file left
      bare - a build script, not a module in the crate graph.
- [x] Rule 5 - rewrite the 26 docs that cite a task artifact (`DECISION.md`,
      bare task ids) into constraints.
      25 sites in `crates/`. The 26th is `input/reference.rs:10`, which carries
      a live tracker link and is EXEMPT - normalized to the `TODO(<task-id>)`
      spelling the rule names so it reads as exempt. `examples/` holds ~40 more
      of the same shape and is deliberately out of this lane; see NOTES.
- [x] Rule 7 - add one comment per bare hand-written trait impl (6 sites)
      saying why it is not a derive.
      5 written. The 6th is `AssetRef`'s `Clone`/`Debug`/`PartialEq`/`Eq` block,
      already covered by the type-level paragraph CONVENTIONS.md quotes as its
      own model; only its `Default` was uncovered, and now is.
- [x] Rule 9 - rename `HudSituationSensing` -> `HudSituationSensingSystems` and
      `CameraAuthority` -> `CameraAuthoritySystems`.
- [x] Rules 3+4 - add the 19 orphaned module preludes in the crates no
      structural lane opens: `nova_autopilot` 7, `nova_debug` 6, `nova_os` 4,
      `nova_mod_format` 2. One prelude module plus a one-line doc naming its
      contents - never the boilerplate sentence rule 2 deletes.
      19 added, in the repo's inline `pub mod prelude { pub use super::{..}; }`
      idiom rather than the plan's `prelude.rs` file - rule 3's own snippet is
      the inline form. `log_capture` is `#[cfg(test)]` with zero `pub` items and
      gets none. Rule 4: the seven intra-crate deep paths the new preludes fully
      cover were repointed; the rest reach `pub(crate)` helpers no prelude
      should carry, and stay.
      The four crate ROOTS keep their by-name re-exports: each curates a
      narrower surface than a glob would (and `nova_autopilot`'s doc explains
      which names would collide), which rule 3 explicitly allows.
- [x] Run `probe run --all` for F47 - making the headless mode real changes
      what a run builds, so the compiler is not the verification.
- [x] Double-registration check in the menu and editor apps for F55.
      All three adders (`nova_menu:102`, `nova_gameplay/plugin.rs:116`,
      `nova_editor/ui/mod.rs:47`) guard on `is_plugin_added`; bevy panics on a
      duplicate add, so every booting example in the probe run is the check.
- [x] Record the two deletion numbers (removed, added) in NOTES.

### Lane06 - "NOVA_EDITOR" - tasks/20260806-121625/plan/lane06.md

NEUTRAL. Depends on L1. Five defects in 2,378 LOC - worst defect density in the
workspace, and the crate was not on the original list. One lane, one reader.

- [x] F11 - add `required_section(sections, id, kind) -> Option<&SectionConfig>`
      and route the FIVE panic sites at `placement.rs:42,46,100,104,205` through
      it. A mod overlay redefining either seeded id panics "New Hull Ship"
      today; every other catalog lookup in the codebase logs and skips.
- [x] F11 - test that a missing catalog id logs and skips rather than
      panicking.
- [x] F29 - add `capture_binding(input, reserved) -> Option<KeyCode>` at
      `placement.rs:315`: exclude the editor camera keys, take `just_pressed`
      not `pressed`, and sort before picking. Hold W while placing a turret and
      it fires on every burn.
- [x] F30 - add `Pickable::IGNORE` to the keybind chips at `keybind.rs:60`;
      copy the constant `card.rs:24` and `tooltip.rs:22` already use.
- [x] F31 - DECIDE which behavior is intended - reset `PlayerSpaceshipConfig`
      on entry, or rebuild the preview from it - then add
      `rebuild_editor_preview_on_enter` at `lib.rs:110`. The bug is that
      NEITHER happens.
- [x] F32 - add `binding_conflict(config, key)` at `keybind.rs:187`, calling
      `scenario_input_overlaps` directly if it accepts a runtime config. A
      second implementation of "do these bindings overlap" is how the editor
      and the lint drift apart.
- [x] Write one test per finding - 13 existing tests and no in-workspace
      dependents means almost no safety net. Budget for the tests, not just the
      fixes.

### Lane07 - "NOVA_UI::SCREEN EXTRACTION" - tasks/20260806-121625/plan/lane07.md

BLOCKS the baseline, lands AFTER it. Depends on L2. One extraction closes four
defects; fixing the two unit bugs separately means writing the
physical-to-logical conversion twice, which is how they diverged.

- [x] Read the `mods` / `scenarios` / `portal` call sites in `nova_menu`
      together FIRST - the `list_detail_screen` signature falls out of that
      reading, it is not a decision to make in advance.
- [x] Create `crates/nova_ui/src/screen/mod.rs` with `prelude`, `scroll` and
      `list` submodules.
- [x] F17 - add `screen::scroll::max_scroll_y(node)` converting `ComputedNode`
      physical px to the logical px `ScrollPosition` uses, via
      `inverse_scale_factor()` as `shell.rs:440` and `screen_indicator.rs:418`
      already do. On a 2x display the current maximum is twice the real one.
- [x] F17 - add `page_step(node)` in logical px, replacing the physical
      `size.y * 0.8` at `nova_os/input.rs:257` that makes one PageUp jump 1.6
      viewports.
- [x] F28 - add `ScrollViewport`, `scroll_viewports` (wheel, clamped both ends)
      and `clamp_viewports` (re-clamp every frame after layout), ordered AFTER
      `ui_layout_system` or it clamps against last frame's `ComputedNode`.
- [x] Delete `nova_menu/src/widgets.rs:66 max_menu_scroll_y`,
      `:75 scroll_menu_lists` and `ScrollableList`.
- [x] Delete `nova_gameplay/src/hud/nova_os/input.rs:430 max_nova_os_scroll_y`.
- [x] Repoint `nova_os/input.rs:255` (PageUp/PageDown) and `:426` (wheel,
      keeping its `any_hovered` precedence) at the new module.
- [x] Adopt `ScrollViewport` in the unclamped `nova_editor` scroll variant.
- [x] Repoint L4's F18 clamp at `screen::max_scroll_y`. Expected and cheap if
      L4 ran first.
- [x] Add `list_detail_screen` and collapse the `mods`/`scenarios`/`portal`
      triplication onto it.
- [x] Rules 3+4 - add 6 module preludes (`font.rs` and 5 siblings) plus
      `screen/prelude.rs`, and collapse `nova_ui/src/lib.rs:32-51`'s 40-odd
      hand-listed items to `<module>::prelude::*` lines. Whole crate in one pass.
- [x] Add a scale-factor test for F17 - the defect is invisible at scale 1.0.

### Lane08 - "NOVA_PROBE RESTRUCTURE" - tasks/20260806-121625/plan/lane08.md

BLOCKS the baseline, lands AFTER it. Depends on L1 (hard) and L2. No findings -
L1 already fixed the defects; this lane is the structure. Do not let the rename
absorb the fixes, or the fixes drift into a rename.

- [x] Carve `nova_probe_cli` out with module names UNCHANGED, so the commit is
      a pure move and reviewable as one. Commit L8.1.
- [x] Split at the real process boundary: `nova_probe` is the in-game
      collection library (wasm-clean, links into examples); `nova_probe_cli` is
      the host harness (spawns children, reads artifacts, renders reports).
- [x] Keep `contract.rs` and `stats.rs` as the shared wire format in
      `nova_probe`, with `nova_probe_cli` depending on it. No third crate until
      a second consumer exists.
- [x] Rename in a SECOND commit: `capture.rs` -> `capabilities/frametime.rs`,
      `recorder.rs` -> `capabilities/timeline.rs`, `invariants.rs` ->
      `capabilities/invariants.rs`, `profile.rs` -> `capabilities/profile.rs`.
      **`profile.rs` is the one exception, ruled against the plan on the
      code.** It post-processes the chrome trace a finished child WROTE, so it
      reads evidence rather than collecting it; it lives at
      `nova_probe_cli/src/evaluation/profile.rs` with the rest of the grading.
      A capability is a Bevy plugin an example wires; `profile.rs` is neither.
- [x] Rename `run_report/` -> `evaluation/` (artifacts, checks) + `report/`
      (html, manifest); `aggregate.rs` -> `report/aggregate.rs`; `catalog.rs`
      -> `evaluation/catalog.rs`; `report.rs` -> `report/mod.rs`;
      `bin/probe/` -> `main.rs` + `native/`. `manifest.rs` landed in
      `evaluation/`, not `report/`: it is the run's identity that
      `process_exit` and `artifacts_loadable` grade against, and the renderers
      only print it.
- [x] Add `NovaProbePlugin { frametime, timeline, invariants }` on the
      collection side - it BUNDLES the capabilities, it does not replace their
      per-example configuration.
- [x] Delete the ~20 `#[cfg(not(target_arch = "wasm32"))]` attributes at
      `lib.rs:82-163`; the crate boundary now is the cfg. Two survive in
      `lib.rs` (the native-only `fixtures` module) and six in
      `capabilities/mod.rs` - the wasm timeline stub and its native twin, which
      are a real target difference rather than a host/game confusion.
- [x] Rules 3+4 in a THIRD commit - one prelude per module in both crates,
      written at the point each module is created under its new name.
      `nova_probe` has 12 public modules, zero preludes and the workspace's
      worst deep-import count (184). Both crate roots are now prelude globs.
      Test-support modules (`#[cfg(test)] fixtures`) get none: they have no
      public boundary, and `fixtures::*` is already the idiom at every use.
- [x] Evict LAST (the only items that can be argued about): `profile_sandbox.rs`
      beside `supervise`, `fixtures.rs` + `run_report/fixtures.rs` to
      `#[cfg(test)]` or a dev-dependency, `bin/perf_web.rs` as a separate tool.
      Two of three. `profile_sandbox.rs` -> `native/profile_sandbox.rs`;
      `perf_web` -> the new `crates/nova_perf_web`, which also removes
      `nova_probe`'s dependency on the root game package (a crate under
      `crates/` depending on the binary it links into). **`nova_probe`'s
      `fixtures.rs` is NOT evicted:** six examples build ships and asteroids
      with it, so `#[cfg(test)]` is impossible (examples are not tests) and a
      dev-dependency crate would have exactly one consumer. Its `nova_protocol`
      import became `nova_core`. `evaluation/fixtures.rs`, the half the bullet
      could reach, is already `#[cfg(test)]`.
- [x] Move the `Cargo.toml` workspace members, the root dev-dependency and the
      `[[bin]]` entries (`nova_probe/Cargo.toml:18-30`).
- [x] Update every caller of `cargo run -p nova_probe -- run --all` to
      `-p nova_probe_cli` IN THE SAME COMMIT as the split:
      `.github/workflows/ci.yaml`, `AGENTS.md`, any justfile/scripts, every doc
      line quoting it.
- [x] `grep -rn -- '-p nova_probe' .` before declaring the lane done. This is
      the only rename in the epic with a non-Rust consumer.
      Zero live hits. What remains is 100+ lines inside `tasks/` (historical
      records of runs that really were invoked that way) and
      `web/src/news/0.8.0.md`, a published release note - neither is rewritten.
- [x] Byte-compare `probe run --all` verdicts before and after, and re-run L1's
      fixture-driven gate tests.

### Lane09 - "NOVA_GAMEPLAY FOUR-WAY SPLIT" - tasks/20260806-121625/plan/lane09.md

BLOCKS the baseline, lands AFTER it. The bulk of the epic and its highest risk.
Depends on L2, L4, L5, L8. Cut NOVAOS -> HUD -> FLIGHT -> CORE, outermost
first, so each cut is against a base that has not moved.

- [ ] Back-edge 1 - move the helper `camera/framing.rs:200` needs into `math`
      (CORE, already moving).
- [ ] Back-edge 2 - invert the scheduling edge at
      `sections/controller_section.rs:301`; the dependency is on ordering, not
      data.
- [ ] Back-edge 3 - lift `plugin.rs:107,111,115` into the assembly crate. The
      plugin wiring four crates belongs above all four.
- [ ] Confirm all three back-edges are resolved BEFORE any file moves.
- [ ] Seam NOVAOS - cut `hud/nova_os*` (~14.3k lines): a terminal runtime that
      is not a HUD. Densest defect cluster and the biggest navigability win.
- [ ] Seam HUD - cut the rest of `hud/` (43% of the crate minus NOVAOS).
- [ ] Seam FLIGHT - cut `flight/`, `sections/`, `input/`, `camera/`,
      `physics/`.
- [ ] Seam CORE - `math/`, components, shared markers; everything the other
      three import.
- [ ] Rule 10, per seam - declare a `SystemSet` for each of the 68 plugins that
      has none, and give each new crate a `configure_sets` block that proves the
      seam is real and the order intentional.
- [ ] Rule 10 first slice - order the 16 declared-but-unordered sets
      (`DirectionalSphereOrbitSystems`, `HudSituationSensingSystems`,
      `IntegritySystems`, `NovaOsMapSystems`, `NovaOsShipSystems`,
      `ObjectivesPluginSystems`, `PointRotationSystems`,
      `SmoothLookRotationSystems`, `SpaceshipTargetingSystems`,
      `SphereOrbitSystems`, `SphereRandomOrbitSystems`,
      `StatusBarPluginSystems`, `TempEntitySystems`, `TurretSectionAimSystems`,
      `TweenSystems`, `WASDCameraControllerSystems`). Re-count AFTER L5 -
      `TweenSystems`, `StatusBarPluginSystems` and `ObjectivesPluginSystems`
      retire there.
- [ ] F53 - the NOVAOS seam's first `configure_sets` block covers
      `nova_os_ship/mod.rs:166` and `nova_os_map/mod.rs:139`, which are declared
      and never ordered. The measurement shows F53 is not 2 sites, it is 16.
- [ ] F53 follow-through - once the ordering is real, DECIDE whether
      `peek_pending_invocation` (`nova_os_ship/app.rs:195`) is deletable; it
      exists because of the missing edge. That is exactly the deletion criterion
      #2 wants.
- [ ] F81 - add `#[derive(SystemParam)] struct NovaOsAppInput` for the identical
      6-param cluster in `map_input` (`nova_os_map/scene.rs:259`) and
      `ship_input` (`nova_os_ship/scene.rs:336`); removes two
      `too_many_arguments` suppressions. The struct has to sit on one side of
      the seam regardless. Local idiom: `nova_os_ship/sections.rs:223`.
- [ ] Audit the 633 crate-local `pub` items (nova_gameplay holds 358) as each
      seam decides what crosses its boundary. Truly dead items: zero - this is
      "tighten what is public", not "delete what is unused".
- [ ] Rules 3+4 - 26 module preludes, written in the same pass as the
      visibility audit. `math` alone is 5 of the deep-import violations and is
      already moving.
- [ ] Run `probe run --all` PER SEAM, not once at the end.
- [ ] Note as you go which `keys/tier1.json` questions each move invalidates
      (`_coverage` maps ids to areas; `nova_os_hud_seam` is 5 of 30), so L2's
      single re-keying pass is not a reconstruction from memory.

### Lane10 - "NOVA_ASSETS / NOVA_SCENARIO CLEANUP" - tasks/20260806-121625/plan/lane10.md

BLOCKS the baseline, lands AFTER it. Depends on L2 and L3. Independent of L9,
so it can run in parallel with it.

- [ ] Create `nova_authoring` and move `lint_walk.rs`, `balance.rs`,
      `content_report.rs`, `scenario_generation.rs`, `bin/content.rs` (as the
      crate's binary) and `nova_scenario/src/lint/` into it.
- [ ] Verify the test that justifies the move: the game binary does not link
      the linter. Anything in the moved set reachable from a running game did
      not belong in the move.
- [ ] Move `assets/base/**` to sit with the tool that generates it, not the
      runtime crate that reads it.
- [ ] Add `crates/nova_assets/src/storage.rs` with
      `trait Storage { read, write, remove }`, mirroring the existing
      `PortalTransport` pattern.
- [ ] Extend the trait's `write` from L3's F07 contract - atomic on native
      (temp + fsync + rename), a single `set_item` on wasm - rather than
      absorbing a free helper and rewriting the same four call sites.
- [ ] Add `NativeStorage { root }` and `WebStorage`; the two impls already
      exist behind `persist.rs`'s `#[cfg(target_arch = "wasm32")]` split at
      `:75-98`, they are just not behind a trait.
- [ ] Delete the `#[cfg(target_arch = "wasm32")]` gates the trait replaces. Do
      NOT re-argue this from bit-rot - W3 withdrew that; all 14 crates
      type-check clean on wasm32. The case is testability and gate removal.
- [ ] Route the four scenario -> HUD coupling sites through `nova_events`:
      `world.rs:138-144`, `actions/mission.rs:512,534,554`. These are the sites
      `AGENTS.md:102` was actually about - route them because they are
      scenario-observable moments, not because of a blanket rule.
- [ ] Lift `render_scale` out of `nova_scenario` into whichever crate owns the
      render settings; decide by reading its consumers, not in advance.
- [ ] Rules 3+4 - 13 module preludes in `nova_assets` (13 public modules, 1
      prelude today) and 2 in `nova_scenario`, each written at the moved
      module's NEW home.
- [ ] Confirm L3's F57 regeneration landed as its own commit BEFORE the content
      move; otherwise the `content_ron_parity` diff is unreviewable.
- [ ] Verify with `content -- lint`, `content_ron_parity` and the `shakedown`
      scenario walk.

### Lane11 - "PERF AND SMALL CORRECTNESS" - tasks/20260806-121625/plan/lane11.md

NEUTRAL. Depends on L1 - F37 sits directly under the probe's FPS baseline
check, so its evidence is only meaningful once the gate is trustworthy.

- [ ] F37 - add a `DefaultProjectileRender { mesh, material }` resource,
      initialized at plugin startup, and clone two handles in the `None` arm of
      `turret_section/render.rs:126-133`. That arm is the SHIPPED path - every
      stock turret authors no projectile mesh - so a held trigger creates 100
      mesh and 100 material assets per second.
- [ ] F38 - extract `spool_allocated_thrusters` from the byte-identical 16-line
      bodies at `flight/autopilot.rs:877` and `flight/manual.rs:142`, building a
      `HashMap<Entity, usize>` from `allocation` ONCE outside the loop. Do not
      fix the O(ships x thrusters^2)-per-tick bug in place in two files.
- [ ] F38 - preserve the verified invariant that `balance_throttles` always
      returns `engines.len()` entries, so `throttles[i]` cannot panic. Do not
      add a bounds check that hides its loss.
- [ ] F24 - move the AI firing-gate timers (`guns.rs:119`,
      `behavior.rs:292-308`, `torpedo.rs:158`) to `FixedUpdate` or tick them off
      `Time<Fixed>`; the chain is registered in `Update` while the firing
      happens in `FixedUpdate`. Decide which by reading what else the AI chain
      needs from `Update`. The 6-vs-119 ratio is NOT a problem - do not widen.
- [ ] F26 - add the `nova_ui::widget::UiText` marker at `settings.rs:95` and
      `pause.rs:203,286`; they are the only menu files that never import it, so
      those spans render in Bevy's default face. Visible in any screenshot.
- [ ] F27 - clamp `nova_os_bright_detent` / `nova_os_scan_detent` on load at
      `settings.rs:228`, like the volume beside it; `(99+1) % 4 == 0` jumps
      brightest to dimmest.
- [ ] F22 note - whoever opens `settings.rs` carries L3's F22 too. Three
      defects, one file.
- [ ] F25 - commit `button_on_setting` (`nova_ui/src/widget/button.rs:496`) on
      `Activate` like every other button, not `On<Add, Pressed>`; press-drag-off
      currently cannot be cancelled.
- [ ] Skin-divergence pass - read the two paint backends ONCE and fix
      `button.rs:244`, `slider.rs:26`, `slider.rs:78` alongside F25, with one
      skin-comparison screenshot test. F50 is the same investigation and sits
      in L5.
- [ ] F65 - use `try_despawn` at `torpedo_section/projectile.rs:94`, or better,
      add the missing ordering edge between `SpaceshipSectionSystems` and
      `TempEntitySystems::Sync`. Two queued despawns HARD-PANIC under the
      `FallbackErrorHandler(panic)` the autopilot and probe runs install.
- [ ] F66 - RULED INTENDED. Add one comment at `projectile.rs:65` saying a
      no-lock launch is a misfire. Behavior unchanged; without the comment the
      next reviewer re-reports it.
- [ ] F35 - prune `AreaOccupancy` when a body inside a live area despawns
      (`objects/area.rs:53`), not only when the area does, and clear it in
      `teardown_scenario_entities`. A scenario gating on `OnExit` never
      advances today.
- [ ] F36 - exclude `0.0` from the range checks at
      `lint/scenario.rs:291,348`; the message already claims `(0, MAX]` and
      `auto_advance_secs: Some(0.0)` builds a Timer that finishes on tick one.
- [ ] F43 - move the two per-readout-per-frame `String` allocations at
      `hud/readout.rs:207` to the right side of the `existing.0 != text`
      compare that throws them away.
- [ ] F44 - clear the 14 `redundant_clone` sites in per-frame HUD systems
      (`flight_status.rs:204`, `torpedo_target.rs:180`, `turret_lead.rs:222`,
      `damage_tint.rs:473,638`, `nova_os_map/scene.rs:104`,
      `nova_os_ship/scene.rs:213`, ...). Mechanical.
- [ ] F62 - replace `images.get_mut(&config.cubemap).unwrap()`
      (`camera/skybox.rs:118`) with the `let ... else { error!; return }` form
      used one line above.
- [ ] F64 - fall back to `"unknown"` instead of `expect`/`unwrap` at
      `nova_info/build.rs:11-13`; a tarball export with no git fails to build.
- [ ] F67 - DECIDE for `sections/thruster_section.rs:353`: multiply main-drive
      thrust by `dt`, or document why it is a raw impulse. Halving `Time<Fixed>`
      halves every ship's linear acceleration today. Internally consistent, but
      do not leave it undocumented.
- [ ] F72 - add `ScenarioConfig::new(id, name, cubemap)` and delete the
      `Default` impl at `loader/mod.rs:144`, which is invalid by its own doc at
      `:141`. 15 sites, mechanical.
- [ ] F82 - read the four real system params before acting
      (`ai/behavior.rs:909`, `component_lock.rs:403`, `radar.rs:387`,
      `turret_section/aim.rs:510`); a `&mut` that reaches a system signature
      declares a write the scheduler serializes against.
      `chip_layout_rig.rs:278` is verified a test helper - leave it.
- [ ] F85 - fix the two `while_float` loops (`nova_os_map/tests.rs:842`,
      `nova_os_ship/tests.rs:1316`), `iter_with_drain` (`mesh/explode.rs:200`)
      and the case-sensitive extension comparison
      (`run_report/artifacts.rs:81`).
- [ ] F86 - fix or DROP WITH REASON: the unwrapped angle lerp
      (`transform/directional_sphere_orbit.rs:121`), the absolute
      `f32::EPSILON` snap threshold (`math.rs:35`), and `camera/shake.rs:295-296`
      feeding offset and kick the same random sample. None is player-visible.
- [ ] Measure F37 and F38 with `probe run --baseline`. Both should show a
      measurable FPS improvement, and that measurement is the point.

### Close-out - the epic's last commits

- [ ] Re-key `benchmark/keys/tier1.json` ONCE, immediately before the final
      run: question text frozen, only `expect` and `citation` change, and only
      to the new location of the same thing.
- [ ] Record any question whose answer no longer exists as a FINDING, never
      retarget it at its nearest survivor.
- [ ] Bump `_keyed_at` and re-open every touched citation against the tree.
- [ ] OWNER - final benchmark run. Report the `blind` and `tree` deltas
      explicitly; a refactor that raises `docs` but not `blind`/`tree` is the
      failure this benchmark exists to catch.
- [ ] Delete `## Not yet true` from the repo-root `CONVENTIONS.md`. Its
      emptiness is the proof the conventions are real. THIS IS THE LAST COMMIT.
- [ ] Create the separate tatr task for F84 (`proc-macro-error2 v2.0.1`
      future-incompatibility) - transitive, breaks on a rustc bump, `-D warnings`
      does not cover it.

## Definition of Done

- Every lane above is either landed or explicitly deferred with a reason
  recorded in NOTES. (manual: read the Steps checkboxes)
- CI runs clippy with `-D warnings`, a default-features check and a wasm check,
  all green. (cmd: `gh run list --branch master --limit 1`)
- The probe gate cannot pass an unprobed run: L1's fixture suite is red before
  the fixes and green after. (cmd: run the fixture tests on the base branch and
  confirm they fail)
- `probe run --all` verdicts on a healthy tree are byte-identical before and
  after L1 and after each L9 seam.
  (cmd: `nix develop --command cargo run -p nova_probe_cli -- run --all`)
- No caller of `cargo run -p nova_probe -- run --all` survives L8.
  (cmd: `grep -rn -- '-p nova_probe ' . --exclude-dir=target`)
- `TREE.txt` in the `tree` persona image contains no `benchmark/` and no
  `tasks/` path. (cmd: `cd benchmark && ./sandbox.sh build tree && ./sandbox.sh
  inspect tree`)
- Content gates pass after L3 and L10.
  (cmd: `nix develop --command cargo run -p nova_assets --bin content -- lint`)
- The final benchmark run shows the `blind` and `tree` tool-call deltas against
  the L2 baseline, reported as two numbers. (manual: `benchmark/results/*/report.html`)
- Deletion count is reported as TWO numbers, removed and added - rule 1 adds 28
  module docs. (manual: NOTES)
- Repo-root `CONVENTIONS.md` has no `## Not yet true` section.
  (cmd: `grep -c 'Not yet true' CONVENTIONS.md`)
- `AGENTS.md` is true row by row against `notes/02-workspace-map.md`.
  (manual: re-read both)

## Notes

- Assumption: lanes land as separate commits on one branch under this one tatr
  task. Splitting into multiple tasks was rejected by the owner - the lanes
  share one baseline and one findings list.
- Assumption: `probe run --all` and the content gates run locally per lane; the
  full test suite and clippy stay CI's job (`skip-local-tests-and-clippy`).
- Assumption: the L7 escape hatch stays closed unless the owner opens it. F17
  and F28 wait for the baseline by default.
- OWNER GATE: the benchmark runs exactly twice and the owner starts both. A
  lane that needs a benchmark number STOPS AND PROMPTS - it never proceeds on
  an assumed number.
- Risk: L9 is XL and depends on four lanes. If L4 or L5 slips, the NOVAOS seam
  moves 14.3k lines with unfixed defects inside it and every citation in
  `notes/10-review-hud-nova-os.md` has to be re-derived. Hold the seam, do not
  reorder.
- Risk: the benchmark harness has never carried a real run. A bug in
  `aggregate.py` or `grade.sh` silently corrupts the number every structural
  lane is measured against. That is why L2 has a review step and a smoke run
  before the baseline.
- Risk: one final run means a seam that is not paying for itself is invisible
  until the epic is over. Accepted by the owner; `probe run --all` per seam
  still covers correctness continuously.
- Tests are NOT a lane. Owner's explicit instruction - the per-lane
  verification is the evidence each lane needs to land, not a coverage push.
- Never hand-edit `assets/base/**/*.content.ron`; F57 regenerates it via the
  builders plus `content -- gen`, in its own commit.

## Close-out - L0 (2026-08-07)

### What and why

L0 is the epic's first commit and lands before the benchmark baseline because
every item changes what the baseline measures: the docs a persona reads, the
file list a persona is given, and the CI configuration every later lane is
graded by.

Three groups, one commit:

| Group | Change |
| --- | --- |
| The map | `AGENTS.md` crate table re-ordered by LOC share with 5 wrong or misleading rows fixed; the `nova_events` line reworded to say it is the modding vocabulary; `## Code rules` reduced to a pointer |
| The style | new repo-root `CONVENTIONS.md`, a 175-line rewrite of the 655-line evidence record in this task folder |
| The gates | `-D warnings` on clippy, plus default-features and wasm32 `cargo check` jobs, plus the two source fixes (F79, the `report.rs` wasm gate) those jobs would otherwise fail on, plus F80 |

The benchmark moved to `<root>/benchmark/` in the same commit because
`TREE.txt` is a persona's entire information channel and its contents are
decided by `repo_files()`.

### Alternatives considered

- **Copying the task-folder `CONVENTIONS.md` to the root.** Rejected by the
  plan and it was right: the 655-line file is an evidence record (violation
  counts, counter-example file lists, rejected proposals, the owner's
  `RULED 2026-08-07` annotations). None of that is what a contributor needs at
  the root, and the two files now have different jobs.
- **A second clippy pass for default features and wasm** instead of two `check`
  jobs. Rejected on cost: clippy would pay a near-full second build for lint
  coverage of `cfg(not(feature = "debug"))` branches, when what was actually
  missing was rustc's own `dead_code`/`unused_imports` in two never-built
  configurations. `check` under `RUSTFLAGS=-D warnings` buys that for less.
- **Deleting the 37 `#[allow(clippy::type_complexity)]` outright** rather than
  converting them. They suppress a lint the workspace already allows, so
  deletion is behaviour-identical and shorter. Rejected: `#[expect]` re-enables
  the lint at the site, which is what turns each one into a claim that fails
  when it stops being true. The conversion immediately proved its own worth.
- **Reworking `attitude_rig` to stop threading `Layout`** so `Layout::B` could
  be gated cleanly. Out of scope for a lane whose F79 brief says "no body
  changes"; noted in NOTES for whoever next touches the file.

### Difficulties and diagnosis

**The plan's F80 counts were wrong, and the mechanism found it.** The plan said
38 sites with 2 stale by eye. Measurement: 37 sites, and after converting all
37, clippy reported **12** unfulfilled expectations. Of the plan's two named
candidates only one (`ammo_readout.rs:510`) was actually stale; `:325` is live.
Diagnosis was free - convert everything, let `unfulfilled_lint_expectations`
adjudicate, delete what it flags. This is rule 8's argument demonstrated on its
first use.

**Two of F79's eleven sites were mis-specified.** `sections/many_sections.rs:37`
is `stress/many_sections.rs:37` and is an unused *import*, not a dead item.
`controller_section.rs:64` is the `Layout::B` enum variant, which is
constructed only under `debug` but pattern-matched in two ungated places, so
the attribute had to go on the variant, on its `match` arm, and on the `if
matches!` that pushes the off-axis hull. Gating that `if` then made
`ship_sections` non-`mut` without the feature - caught only by running the new
CI job locally, which is the argument for the job.

**`cargo check` rejects `-- -D warnings`.** The plan wrote both new jobs in
clippy's flag form. `RUSTFLAGS` is the working equivalent.

**The `.gitignore` alone would have been inert.** Nine transcript and payload
files under `results/smoke/` were already tracked, and gitignore does not
untrack. They were `git rm --cached`ed; the three rollups stay.

### Evidence

Fail-first, measured on the tree before any edit:

| Configuration | Before | After |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | **11 warnings** (8 const, 2 fn, 1 variant, 1 unused import) | 0, exit 0 under `RUSTFLAGS=-D warnings` |
| `cargo check --workspace --target wasm32-unknown-unknown` | **7 warnings**, all from `report.rs` | 0, exit 0 under `RUSTFLAGS=-D warnings` |
| `cargo clippy --workspace --all-targets --features debug -- -D warnings` | 0 | 0, exit 0 |
| `#[expect(clippy::type_complexity)]` unfulfilled | n/a | **12 found, 12 deleted**; 25 conversions + 4 pre-existing survive |

Also run:

- `cargo fmt --check` - clean.
- `cd benchmark && ./sandbox.sh build tree && ./sandbox.sh inspect tree` -
  `TREE.txt` is 950 lines with **0** `^benchmark/`, **0** `^tasks/` and **0**
  `keys/` paths. The single `benchmark` string in the file is its own header
  sentence. (DoD proof 6.)
- `cd web && npm run ci` - compiled successfully, after updating the now-stale
  CI paragraph in `web/src/wiki/dev/development.md`.
- `probe run controller_section` - the one example where live control flow was
  gated. **OK, 5/6** (fps not armed): `run_completed` at frame 657,
  `reached_playing` at frame 30, 0 invariant violations over 657 frames,
  0 offending log lines.
- `AGENTS.md` re-read row by row against `notes/02-workspace-map.md` and against
  the tree. Every crate row's claim was checked against a real path.
  (DoD proof 11.)

Not run and not applicable to L0: DoD proofs 2 (`gh run list` - needs the push),
3, 4, 5, 7, 8 (later lanes), and 10 (`grep -c 'Not yet true' CONVENTIONS.md`
must be **0** only at the epic's LAST commit; today it is 1, deliberately).

### Reflection

The lane's own thesis - that a refactor silently produces dead code and stale
suppressions, and that CI should report both - was confirmed before the lane
finished. The tree already held 11 dead example items and 12 stale
suppressions, accumulated with no refactor in progress at all. Both classes
were invisible because no CI configuration ever built them.

The transferable lesson is about the shape of the audit, not the counts: the
plan's hand-counted numbers (38 sites, 2 stale) were produced by an agent
reading files, and were wrong in both directions. The numbers that survived
came from making the compiler answer. Where a lane can be phrased as "turn on
the check and delete what it flags", phrase it that way; the by-eye version
costs the same and is wrong.

One thing to carry into L5: `CONVENTIONS.md` overshot its 120-150 line budget
by 17% because twelve rules with a snippet and rationale each have a floor of
about twelve lines apiece. A future line budget for a rules document should be
set from `rules x 12 + sections`, not chosen first.

## Close-out - L3 (2026-08-08)

### What and why

L3 is behavior-only and NEUTRAL on the benchmark. Its frame is that mod content
is untrusted input - it arrives from a remote portal catalog and from files the
player may have edited - so a reachable panic, OOM or permanent data loss is a
defect, not an upheld invariant. Sixteen findings, five commits.

| Commit | Findings |
| --- | --- |
| F06+F07+F22 | the three persistence defects, landed together |
| input caps, portal indexing, epsilon Equal | F08, F13, F59, F60, F68, F69, F61, and the F09 ruling |
| authored-value caps and silent-failure logs | F10, F12, F14, F56 |
| F57 deterministic map serialization | F57 |
| the fire_rate lint | F10's second half |

**F06 and F07 landed as one change on purpose.** F07 produces the corrupt file;
F06 turns it into permanent loss on the next install. `read_index_at` returned
`Option`, conflating "no index yet" with "the index is corrupt", and
`install_local_at` folded both into `Vec::new()` - so one torn write erased
every *other* installed mod from the index and orphaned their bytes on disk
where `remove_mod` could never sweep them. `IndexRead { Absent, Loaded,
Corrupt }` makes the two cases different, a `Corrupt` index side-bands to
`installed.mods.ron.bad` and fails the install loudly, and `write_atomic`
(temp + fsync + rename) stops producing the torn file in the first place.

`write_atomic` is deliberately written as the crate's write *contract* rather
than a free helper, because L10 extracts exactly these four call sites into a
`Storage` trait. Its doc says so; L10 moves it, it does not re-litigate it.

### Alternatives considered

- **A depth cap for F09** (`MAX_EXPR_DEPTH = 32` in `variables.rs` and
  `filters.rs`), as the plan specified. Rejected on measurement: every
  production RON decode goes through `ron::de::from_bytes` under
  `Options::default()`, whose `recursion_limit` is `Some(128)`. Deep nesting is
  already a parse error, never a stack overflow, and the plan's premise - that
  it overflows on the asset-loader task during boot - does not hold. A second
  bound on an already-bounded path is dead code that reads as protection. The
  ruling is pinned by a test that fails if that limit is ever lifted.
- **A second `ApproxEqual` DSL node for F61**, or documenting the sharp edge.
  Both were ruled out by the owner on 2026-08-07; `Equal` compares within
  `EQUAL_EPSILON = 1e-6`, named as a constant beside the node.
- **Bounding the F13 catalog read at `transport.rs:31`**, as the plan
  specified. Not possible as written: `ehttp` buffers a whole response body
  before it invokes the callback, so the transport seam never sees a
  bound-able read. The cap sits at the earliest point the client does control,
  `decode_catalog`, ahead of BOTH parses, and the code records why.
- **Rejecting an over-cap `ScatterObjects` at runtime** rather than clamping.
  Clamping plus a warn keeps a scenario playable; the lint is what fails the
  author early, which is where an absurd count should be caught.

### Difficulties and diagnosis

**F10 is two halves and only one was findable from the runtime.** The clamp at
`turret_section/setup.rs:64` stops the panic, but a clamped `fire_rate: 0.0`
then fires at `1.0 / f32::EPSILON` shots/s - absurd, silent, and now
unreachable by the crash that used to report it. The lint half
(`lint/ship.rs`) is what keeps the fix from trading a loud defect for a quiet
one. It was missing from the first pass and caught by re-reading the step's
clauses against the tree rather than against the diff.

**F22 needed a resource, not a `Local`.** The debounce countdown lived in a
`Local<Option<u32>>`, which no other system can observe, so a flush system had
nothing to flush. It became `PendingSettingsSave`; `flush_settings_on_exit`
runs in `Last` and writes only when a save is actually owed.

**Testing the settings flush would have overwritten the developer's real
settings.** `persist.rs` resolved `dirs::config_dir()` unconditionally. Added
`NOVA_CONFIG_ROOT`, the exact twin of the existing `NOVA_MOD_CACHE_ROOT`
override and for the same reason.

**F57's risk was the generated tree, not the code.** `input_mapping` serialized
straight from a `HashMap` into the GENERATED `assets/base/**/*.content.ron`, so
`content -- gen` could write different bytes each run. Fixed via `BTreeMap` and
landed as its own commit; re-running `content -- gen` reproduces the tree
byte-identically, so no generated churn hides a real diff.

### Evidence

- `cargo test -p nova_assets -p nova_mod_format -p nova_scenario -p nova_events
  -p nova_menu --lib` - 349 passed, 0 failed.
- New tests, all fail-first against the pre-fix code: `a_torn_index_survives_the_next_install`,
  `corrupt_index_reads_corrupt_not_absent`, `missing_index_reads_absent`,
  `a_failed_atomic_write_leaves_the_previous_contents`,
  `the_index_write_leaves_no_temp_file_behind`,
  `a_setting_edited_just_before_quitting_is_still_saved`,
  `a_chain_deeper_than_the_cap_is_refused_not_walked`,
  `decode_catalog_refuses_an_oversized_body_and_an_absurd_entry_count`,
  `duplicate_ids_are_not_a_cycle`, `a_degenerate_fire_rate_does_not_panic_the_spawn`,
  `an_absurd_scatter_count_is_a_lint_error`,
  `a_section_ref_to_an_undeclared_resource_is_a_violation`,
  `equal_compares_numbers_within_an_epsilon`, and
  `a_decode_deeper_than_rons_recursion_limit_is_refused_not_walked` (the F09
  ruling's pin).
- `cargo clippy -p nova_assets -p nova_scenario -p nova_mod_format -p nova_events
  -p nova_menu --all-targets -- -D warnings` - exit 0.
- `cargo fmt --all --check` - exit 0.
- `content -- lint` - 0 errors, 0 warnings, 0 findings, 14 scenarios
  balance-audited, 1 acked. (DoD proof: content gates pass after L3.)
- `content -- gen` - zero diff in `assets/`, so F57's serialization is
  reproducible.
- `probe run --all` at `29e2a8fa` - **24/24 OK**, every example `6/7 measured`
  (`fps_within_baseline` N/A, not claimed). Same 24 rows and the same per-row
  shape as L1's landed run, including `artifacts_loadable` PASS everywhere. The
  lane touches the scenario spawn path, the turret spawn path and four
  persistence sites, and none of them moved a verdict.

### Reflection

The hostile-RON corpus paid for itself the way the lane predicted: every
finding here is "authored data reaches code that assumed it was sensible", so
one fixture idea covered most of the lane. What it did NOT cover is the half of
a fix that lives in the linter, which is a different artifact with a different
test. Two findings in this lane (F10, F12) are runtime-clamp plus lint-rule
pairs, and both were at risk of shipping as clamp-only. For the remaining lanes:
when a fix clamps an authored value, the clamp is half the change - the lint
rule that names the field is the other half, and the step should be read as
two.

The F09 ruling is worth carrying forward too. The plan's finding list was
written from reading the code, and one of its premises was simply false about
the library underneath. Re-deriving the mechanism before implementing a cap
turned an unnecessary bound into a test that documents why none is needed.

## Close-out - L4 (2026-08-08)

### What and why

L4 is behavior-only and NEUTRAL on the benchmark. Twenty-two findings across
three concerns that share one shape: **a system that runs every frame, or holds
state across a respawn, and does not check whether anything actually changed.**
`hud/keybind_dock.rs` was the reference; every site here was made to look like
it. The lane also had to land before L9 moves the NOVAOS seam, or all 22
citations would need re-deriving against 14.3k shifted lines.

| Concern | Findings |
| --- | --- |
| Unguarded per-frame writes | F39, F40, F41, F42 |
| Stale `Local<T>` across a respawn | F19, F20, F75 |
| Missing Control guards | F15, F34 |
| Terminal model | F18, F33, F73, F74 |
| Standalone | F16 (aborted explosion), F21 (loops through load), F23 (torpedo anchor) |

**F40 drove the shape of the whole terminal change.** `rebuild_terminal_ui`
despawned and respawned every scrollback row whenever `NovaOsTerminal` was
marked changed, and every prompt edit goes through `ResMut`, so a caret keypress
- which changes nothing on screen - respawned ~400 entities. The plan proposed
comparing `scrollback().len()` and the tail. That is a heuristic: a rewrite in
the middle of the buffer passes both checks. Instead `scrollback` became a
private field with a `scrollback_revision` counter, and every mutation - push,
extend, replace, the boot reveal, `clear`, `reset_session` - funnels through one
private method that bumps the revision and applies `MAX_SCROLLBACK_ROWS`. The
cap (F40's second half) and the revision are then unbypassable by a future
caller, which a tail comparison in the UI layer would not be.

**F18 could not be fixed where the plan said.** The sentinel is written right
after the new rows are queued as commands, so those rows have no `ComputedNode`
yet and the real maximum is not knowable at the write. `SCROLL_TO_BOTTOM` stays
the request; the new `normalize_nova_os_scroll` runs `.before()` both the
keyboard and the wheel handler and replaces any overshoot with the measured
maximum, so PageUp always subtracts from a real number instead of from
`f32::MAX`.

### Alternatives considered

- **F75 as a separate `prune_dry_fire_state` system** (what the plan asked for,
  modelled on `prune_sfx_throttle`). Rejected. `prune_sfx_throttle` exists
  because its map is keyed on sounds nothing else enumerates. This map is keyed
  on turrets that `play_dry_fire_cue` already visits in full every frame, so
  rebuilding it in the loop makes the live set exact by construction. A second
  system would buy a schedule edge to re-derive what the first already knows.
- **F20 keyed on the ship `Entity`** rather than an `Added<>` override. Rejected:
  there is exactly one player ship, so a map would carry one entry and need its
  own prune - the `Ref<WeaponsHot>::is_added()` arm is the same fix in one line.
- **Implementing the readline chords under F15.** Deferred, recorded in the
  step. Not typing a literal character is the whole defect; Ctrl+U/W/A/K are a
  feature with their own design surface (word boundaries, a yank buffer).
- **Opening the L7 escape hatch.** Declined: L2 ratified and baselined in full,
  so the condition that would open it never occurred.

### Difficulties and diagnosis

**The first change-detection test passed against the unfixed code.** It asserted
`entity.get_ref::<Text>().unwrap().is_changed()` after `app.update()`. Outside a
system that compares the component tick against the world's `last_change_tick`,
which the schedule leaves stale, so it read false whether or not the reconciler
had written. Verified by deleting the guard and watching the test still pass.
The working form is a detector system with `Changed<Text>` / `Changed<TextColor>`
queries chained after the reconciler, counting hits into a resource: system-local
`last_run` ticks are the only reliable frame boundary. It then failed correctly
(2,2 vs 1,1) with the guards removed. **Any future "assert it did not change"
test in this repo should use the detector-system form, not `is_changed()` on an
`EntityRef`.**

**F73 changed an existing test's expectations, correctly.** `completion_matches`
iterated a `HashMap`, so `nova_os_tab_cycles_ambiguous_completions` had pinned
whatever order that yielded on the authoring machine. Sorting made the authored
order wrong and the new order right; the old assertion was never evidence of
anything.

**`git checkout <file>` cost a redo.** Reverting a probe edit on an uncommitted
file discarded the real work in it too. Falsification passes on uncommitted work
need a scratch copy, not a checkout.

### Evidence

- `cargo test -p nova_os --lib` - 24 passed, 0 failed.
- `cargo test -p nova_ui --lib status_bar` - 1 passed. Fails (2,2 vs 1,1) with
  the F41 guards removed.
- `cargo test -p nova_gameplay --lib torpedo` - 76 passed, 0 failed, including
  the new `torpedo_homes_on_the_live_structure_anchor`.
- New tests: `an_unchanged_status_item_is_not_rewritten` (nova_ui - the first
  test in that 365-line file), `torpedo_homes_on_the_live_structure_anchor`,
  `nova_os_completion_matches_are_sorted_and_deduplicated`,
  `nova_os_history_is_bounded_and_skips_repeats`,
  `nova_os_scrollback_is_bounded_and_revisioned`,
  `nova_os_ghost_survives_a_leading_space`.
- `cargo check --workspace --all-targets` - exit 0.
- `cargo fmt --all --check` - exit 0.
- `probe run --all` at `826c8bb4` - **24/24 OK**, every example `6/7 measured`
  (`fps_within_baseline` N/A, not claimed). Same 24 rows and the same per-row
  shape as L1's and L3's landed runs. The lane touches the HUD reconcilers,
  the audio cue/loop systems and the torpedo homing path; `screenshot_nova_os`,
  `screenshot_ui`, `torpedo_section` and `screenshot_combat` all held.
- Full suite and clippy: CI's job (`skip-local-tests-and-clippy`), per the
  lane assumption in NOTES.

### Reflection

The lane's premise - "every fix is make the site look like keybind_dock" - held
for the eight guard sites and broke for the three state sites. A guard is local
and the reference implementation transfers directly. Stale `Local<T>` is not
local: F19 needed a revision counter on a type in a different crate, and F40's
cap had to move into the model to be unbypassable. **For the remaining lanes:
when a plan step names a reference implementation, that is evidence the fix is
local; when the fix needs a new field on a shared type, the reference has
stopped applying and the step deserves re-reading.**

The other carry-forward is that the plan's proof shape ("run two frames and
assert not changed") named the right assertion but not the right mechanism, and
the wrong mechanism was silently green. Behaviour tests fail loudly when wrong;
change-detection tests fail silently. Every one of them should be run once
against the unfixed code before it is trusted - which is what the DoD's
fail-first rule already says, and which is cheap only if it is done at the time.

## Close-out - L5 (2026-08-08)

### What and why

Two jobs in one lane: delete the surface that lies about what it does, then
make the docs say what the code is. They share a constraint - both land after
the L2 baseline so their lines enter the ledger - and nothing else.

The deletions were eleven findings. `tween.rs` went first (F45) because it made
F55 a two-plugin merge instead of three: `NovaUiPlugin` now owns the themed
widget observers and the status bar, and the first-caller-wins
`WidgetObserversRegistered` resource is gone, replaced by `is_plugin_added` at
the three call sites - bevy's own idiom, and visible where the decision is made
rather than buried in the crate. `StatusBarStore` (F46) was declared and never
touched. `ObjectivesPlugin` (F48) rebuilt a panel nothing spawned. Two of the
three `toggle_debug_mode` fns (F54) went, so a fourth debug sub-plugin can no
longer silently invert F11.

Three findings were lies rather than corpses, and got made true instead of
deleted. `NovaGameplayPlugin::render` (F47) documented a headless mode that did
not exist; hanabi, the skybox, post-processing and the HUD are now actually
gated on it. `Without<SectionInactiveMarker>` on the torpedo spawner query
(F49) read as a safety gate and excluded nothing, because the marker lands on
the section; a disabled bay kept rearming. `panel_head`'s `_skin` (F50) was
ignored while every call site believed it did something.

The prose sweep is CONVENTIONS.md's five prose rules made true across
`crates/`: 69 boilerplate prelude docs rewritten to name their contents, 38
module `//!` docs written, 25 task-artifact citations turned into the
constraint they were standing in for, 5 hand-written-impl comments, 2 system
set renames, and 19 module preludes in the four crates no structural lane
opens.

### Alternatives considered

- **Deleting the 69 prelude docs, as written.** Impossible, and the compiler
  says so: `#![warn(missing_docs)]` is on every crate and L0 put `-D warnings`
  on CI's clippy. Rewriting them into the one-line form is what rule 2's own
  text asks for anyway.
- **Deleting `panel_head`'s `_skin` parameter** (F50). Rejected by the finding
  itself and it was right - every call site passes a real skin believing it
  lands somewhere.
- **Globbing the four crate roots onto the new module preludes** (rules 3+4).
  Rejected. Each root curates a narrower surface than its modules export, and
  `nova_autopilot`'s prelude doc spends twelve lines on which names would
  collide with `bevy::prelude` if it did. Rule 3 permits by-name re-export for
  exactly this reason.
- **Sweeping rule 5 through `examples/` too.** Deferred and recorded. Same
  defect, ~40 more sites, and outside every count the lane was planned against.
- **A `prelude.rs` file per module**, as the lane plan spelled it. Used the
  inline `pub mod prelude { ... }` form instead - it is what rule 3's own
  snippet shows and what all 105 existing preludes do.

### Difficulties and diagnosis

**F47 was not a four-line gate.** Gating the HUD on `render` made a headless
run panic: `GameObjectives` was `init_resource`d by the HUD plugin, but the
scenario loader writes it whether or not anything draws it. The resource moved
to `NovaGameplayPlugin`, which is where mission state belongs - the HUD only
ever displayed it. A finding that says "gate these four plugins" is describing
the edit, not the coupling underneath it.

**Four of the six prose counts were wrong**, all in the same direction: the
census under-counted because it was written from a grep that had an implicit
exemption in it. Rule 1 skipped test modules, rule 5 counted an exempt
`TODO`, rule 7 counted a site that was already the convention's own example.
Detail and the corrected numbers are in NOTES. The lesson is not "the plan was
sloppy" - it is that a census whose predicate is not written down cannot be
re-run, and every one of these was recoverable only by re-deriving it.

**Reporting one deletion number would have been a lie.** The lane's diff is
+887/-944, a net -57, which reads as a lane that barely deleted anything. The
halves are -440 (deletions) and +383 (prose, almost all of it rule 1's 38 new
module docs). The step demanded two numbers for this reason and it was right.

### Evidence

- `cargo check --workspace --all-targets` - exit 0, zero warnings. This is the
  real proof for the prose sweep: `missing_docs` is `warn` in all 15 crates, so
  a prelude or module left undocumented is a warning here, and CI fails on it.
- `cargo fmt --all --check` - exit 0.
- `probe run --all` - see the run recorded below; F47 changes what a run
  BUILDS, so the compiler is not the verification for it.
- New test: `a_disabled_bay_stops_rearming` (F49), asserting both arms so the
  disabled case cannot pass on a starved clock.
- Prelude census re-run after the sweep: 0 remaining boilerplate docs, 36
  specific ones kept.
- Module-doc census re-run: 1 file without a `//!`, `nova_info/build.rs`, a
  build script.
- Rule 5 re-grep over `crates/`: 0 hits outside the exempt `TODO(<id>)`.
- Full suite and clippy: CI's job, per the lane assumption in NOTES.

### Reflection

The lane's two halves failed differently and that is the carry-forward. The
deletions were verified by the compiler and by one behaviour test, and the only
surprise was F47's hidden resource ownership - a coupling, which is the failure
mode deletions have. The prose sweep had no compiler check at all until the
`missing_docs` + `-D warnings` pair was noticed, at which point the whole rule
2 step turned out to be un-executable as written. **A prose lane's proof is
whichever lint already fails on the thing it is fixing; find that lint before
writing the first edit, because it also tells you what the edit has to look
like.** Here it turned "delete 69 lines" into "rewrite 69 lines", which is a
different job with a different cost.

The other one is about counts. Four censuses out of six were wrong, and none of
them were wrong by much - which is worse than being wrong by a lot, because a
count that is nearly right does not announce itself. Every future lane that
opens with a number should carry the grep that produced it.

## Close-out - L6 (2026-08-08)

### What and why

Five defects (F11, F29, F30, F31, F32) in the one crate small enough to hold in
one head. The fixes turned out to share a root: `placement.rs` had five
near-identical per-kind spawn blocks, so every rule about a placed section -
which panic guards it, which key it binds, how it is registered in
`PlayerSpaceshipConfig` - was written five times.

| Finding | Change |
| --- | --- |
| F11 | `required_section` logs and returns `None`; the two seed sites add an explicit kind check. No `unwrap`/`panic!` left in the crate |
| F29 | `capture_binding` takes `just_pressed`, filters `EDITOR_CAMERA_KEYS`, and picks the `min` - deterministic under a `HashSet` iteration |
| F30 | `Pickable::IGNORE` on the keybind chip |
| F31 | rebuild, not reset - `rebuild_editor_preview_on_enter` respawns the preview from the surviving config and re-keys both maps onto the new entities |
| F32 | `binding_conflict` rejects a rebind onto a `flight_rig_reserved_sources` source or onto a key another section holds; the section stays armed |

The dedupe that made F31 cheap: `spawn_preview_section` (one `SectionConfig` ->
preview entities) plus `register_preview_section` (the config entry) are now the
only two places that know the shape, shared by click-placement and the rebuild.
`on_click_spaceship_section` went from 272 lines to 87. The file's
implementation half is 517 -> 513 lines overall, because the space the dedupe
freed went into the F31 rebuild and the new helpers' docs; 10 new tests sit on
top of that.

Two invariants got written down rather than re-derived. A preview section's
config `id` IS its preview entity, because `sandbox_objects` keys the
scenario's `input_mapping` by that same entity - the seeds' hand-written
`"initial_hull"` / `"initial_controller"` ids were unreferenced anywhere in the
repo and are gone. And the rebuild is the reason both maps must be re-keyed
together.

### Alternatives considered

- **F31 as a RESET** (clear `PlayerSpaceshipConfig` on entry) instead of a
  rebuild. Rejected: the only route into a second Editor visit is F1 from a
  live sandbox flight, and a player who flies back to the editor wants the ship
  they built, not an empty world. Reset also silently discards work with no
  prompt.
- **`required_section(sections, id, kind)` as the plan wrote it.** `SectionKind`
  carries its per-kind config, so an "expected kind" argument needs either a
  parallel discriminant enum - exactly the drift this lane exists to remove -
  or a closure generic used by two callers. Kept the accessor for the lookup
  (three call sites) and left a two-line `matches!` guard at each seed.
- **Calling `scenario_input_overlaps` for F32.** It is private to `nova_assets`
  and takes a `ScenarioConfig`, not a runtime config. The shared rule under it
  is `nova_gameplay::flight_rig_reserved_sources` + `binding_source`, both
  public; `binding_conflict` calls those, so the editor and the lint read the
  same list.
- **Extending the `editor` autopilot example with a Play -> F1 -> Editor round
  trip** as e2e proof of F31. Deferred: the unit test pins the re-keying, which
  is the part that breaks silently, and the extra beats add scenario load and
  teardown to a run that already covers the placement pipeline.

### Difficulties and diagnosis

**F29's citation named one site; there are three.** `placement.rs:315` is the
turret, but the thruster (`:240`) and torpedo (`:361`) captured input exactly
the same way. Collapsing the five kind arms surfaced them - three copies of the
same `get_pressed().next()` cannot hide once they are one `default_binds_for`.

**The thruster's default bind is `Space`, which is both an editor camera key
and `flight_rig_reserved_sources`' "flight burn".** So placement now refuses to
CAPTURE Space but still falls back to it. Left alone deliberately: a thruster on
the burn key is arguably the intent, and changing a shipped default is not this
lane's call. Worth an owner decision.

**Delete leaked input bindings.** `SectionChoice::Delete` removed the section
from `config.sections` but not from `config.inputs`, so the handed-off
scenario's `input_mapping` kept an entry for a section that no longer existed.
One line, fixed in place while rewriting the branch.

### Evidence

- `cargo test -p nova_editor --lib` - 23 passed, 0 failed (13 before this lane).
- `cargo clippy -p nova_editor --all-targets` - clean.
- `NOVA_AUTOPILOT=1 DISPLAY=:99 cargo run --example editor --features debug` -
  `autopilot: cycle complete, no panic (t=5.7s)`; every beat green, including
  the two real pointer placements and the delete.
- Red-before check: the two F11 tests panic on the base (`unwrap` on an empty
  `GameSections`); the F30 test reads no `Pickable`; the F32 tests see the
  conflicting key accepted; `rebuild_editor_preview_on_enter` did not exist.

### Reflection

The lane's brief said "one lane, one reader" and that was the whole trick. Four
of the five defects were in code that had been copy-pasted five times, and each
finding had been reported against whichever copy the reviewer happened to open.
Reading the crate whole first, then deduping, then fixing, cost less than five
targeted patches would have - and it is why the F31 rebuild is ~30 lines
instead of a sixth copy of the spawn logic.

## Close-out - L7 (2026-08-08)

### What and why

`crates/nova_ui/src/screen/` now owns every scrollable viewport in the game and
the list-beside-details composition the menu builds twice. Four defects close in
one edit, which was the whole argument for making this an extraction rather than
two in-place patches.

| Finding | Change |
| --- | --- |
| F17 | `screen::scroll::max_scroll_y` multiplies the `ComputedNode` overflow by `inverse_scale_factor()`. `ScrollPosition` is logical px, `ComputedNode` is physical, so on a 2x display every maximum in the game was twice the real one |
| F17 | `screen::scroll::page_step` does the same for the keyboard page. The physical `size.y * 0.8` at `nova_os/input.rs:257` made one PageUp jump 1.6 viewports on a 2x display |
| F28 | `clamp_viewports` re-clamps every `ScrollViewport` in `PostUpdate` after `UiSystems::Layout`, so shrinking content pulls the stored offset back instead of leaving the pane blank |
| - | `nova_editor`'s drawer was unclamped at the bottom entirely; adopting `ScrollViewport` gives it both ends |

Three separate wheel handlers (`scroll_menu_lists`, `scroll_editor_panel`, and
`nova_os`'s) had three copies of the line-height constant and two byte-identical
copies of the wrong overflow formula. Two of the three are gone; `nova_os` keeps
its own system because its `any_hovered` precedence is scoped to the drawer, but
it now calls the shared `max_scroll_y`/`page_step`. `NovaUiPlugin` registers the
driver and the clamp once, so no consumer registers a scroll system any more.

`screen::list` collapses the composition: `overlay_root` (3 call sites - the
settings, mods and scenarios panels), `list_detail_screen`, `list_pane`,
`scroll_column`, `scroll_viewport`, `details_pane` and `footer_back_slot`.
`menu_ui.rs` is 637 -> 480 lines with no behaviour change, and the two comments
that were load-bearing - why the list pane pins `flex_shrink: 0`, why the
overlay needs an explicit `GlobalZIndex` - are now written once beside the code
they guard instead of twice as cross-references to each other.

Rules 3+4: `font`, `hud`, `skin` and `widget` gained preludes (`status_bar`
already had one, `screen` ships with one), and `lib.rs`'s 40-odd hand-listed
items became seven `<module>::prelude::*` lines. Publishing a new `nova_ui` name
is now a one-file edit.

### Difficulties and diagnosis

The plan's "mods / scenarios / portal triplication" is a duplication of TWO, not
three: `portal.rs` has no screen of its own - it spawns rows into the mods
panel's list under the Explore tab, reusing `ModRow` and its observer. Reading
the three call sites together (the lane's first step) is what showed that, so
`list_detail_screen` is shaped for the two real callers rather than for a third
that does not exist. The settings panel turned out to be the third caller of
`overlay_root`, which the plan did not anticipate.

`list_pane()` and `scroll_column()` return `Node` rather than a bundle, because
a pane's overflow and its 40% pin are fields of the SAME component - two bundles
cannot each carry a `Node`. Callers spread them (`Node { overflow: ..,
..list_pane() }`), which keeps the per-screen deviation visible at the call site
instead of hiding it behind a boolean parameter.

### Evidence

- `cargo test -p nova_ui --lib` - 28 pass, 5 new in `screen::tests` (the two
  scale-factor contracts, both wheel clamps, the shrink clamp, hover
  precedence). The scale-factor assertions are what the old code fails: it
  answered 200.0 where the display scale demands 100.0.
- `cargo test -p nova_menu --lib` - 77 pass, including the repointed
  `scenarios_list_scrolls_on_wheel_and_clamps`.
- `cargo test -p nova_gameplay --lib nova_os` - 109 pass; `cargo test
  -p nova_editor --lib` - 22 pass.
- `cargo check --workspace --all-targets` - clean, no warnings.
- RUN, not just checked (duplicate-component panics do not surface in `check`):
  `examples/ui/menu_scenarios.rs` and `examples/ui/editor.rs` both reach their
  screens under Xvfb with no panic or error log.

### Reflection

The escape hatch in the lane plan - land the unit conversion in place during L4
and delete the duplicate bodies later - would have cost a second write of the
conversion. It was not needed: L4 landed first, so its F18 clamp already called
the local `max_nova_os_scroll_y`, and repointing it at `screen::max_scroll_y`
was a one-line edit exactly as the plan predicted.

The `nova_editor` local scroll test went away with the system it tested. That is
a net gain in coverage, not a loss - the drawer's bottom clamp was never tested
because it never existed, and the shared tests now cover both ends at two
display scales.
