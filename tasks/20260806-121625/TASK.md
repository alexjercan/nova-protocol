# Refactor nova_* crate for better structure and clarity

- STATUS: IN_PROGRESS
- PRIORITY: 40
- TAGS: v0.10.0, refactoring, project

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

CHECKPOINT (L9.5, commit 67c3715a). ALL THREE SEAMS ARE CUT and the CORE seam
is settled as "no fourth crate", so the split itself is DONE:
`nova_os_ui` (L9.2), `nova_hud` (L9.3), `nova_ship` (L9.5), over a
`nova_gameplay` base that is now 7% of the workspace. The branch is clean, the
five CI commands are green, and `probe run --all` graded OK 24/24 at
`31cdf5dd`.
CHECKPOINT (L9.7, commit cf090439). THE VISIBILITY AND PRELUDE PASS IS DONE
and L9 has ONE step left, which is BLOCKED ON THE OWNER, not on work. The
branch is clean, the five CI commands are green, `cargo doc` carries no
`nova_ship`/`nova_hud`/`nova_os_ui`/`nova_gameplay` warning, and `probe run
--all` at `7af7fb3d` graded 23/24 with the one FAIL diagnosed as a wall-clock
harness watchdog (see that step).
  1. "Rule 10, per seam" - was the only open step, and it was a RULING, not a
     task. RULED 2026-08-08 and landed in L9.8: the rewrite is adopted, the 16
     leaf plugins close with no set, and the 6 handle-with-no-holder sets are
     deleted. See that step.
  2. "Rule 10 first slice" - DONE at L9.6, 7 sets not 16. See that step.
  3. The `pub` audit and the prelude count - DONE at L9.7 for `nova_gameplay`,
     `nova_hud` and `nova_os_ui`, plus 14 `nova_ship` leaf modules made
     private. `unreachable_pub` is zero across all four.
  4. `probe run --all` and the tier1 list - DONE at L9.7. The tier1 list is now
     compiler-derived, and it found five dead citations belonging to L8 and
     L10 that no lane had noted.

CHECKPOINT (L9.8). **L9 IS COMPLETE AND READY TO LAND.** Every step is ticked.
The rule-10 ruling arrived, `CONVENTIONS.md` rule 10 is rewritten to its
two-clause form, and its `## Not yet true` row is deleted - that table is now
down to rules 3, 4 and 1, all owned by L7 and L10.
Rule 10's close was not free: the rewrite is LOOSER on the 16 leaf plugins and
STRICTER on the 6 orphan sets, which had to be deleted rather than left. It
also invalidated one downstream citation, L11's F65, corrected in place at
that step.
NEXT UNIT OF WORK IS L10, then L11. Neither depends on anything in L9 beyond
what lands with it.

- [x] Back-edge 1 - move the helper `camera/framing.rs:200` needs into `math`
      (CORE, already moving). Landed in L9.1 with `FORWARD_ALIGNMENT_COS`, the
      unit test, and `math`'s prelude (rules 3+4); the five deep
      `crate::math::*` imports route through it.
- [x] Back-edge 2 - invert the scheduling edge at
      `sections/controller_section.rs:301`; the dependency is on ordering, not
      data. Landed in L9.1 as `ControllerSectionSystems::SyncRotationInput`
      (rule 10 for that plugin), with `NovaFlightPlugin` declaring
      `.before(SyncRotationInput)`. Pinned by
      `rotation_command_pipeline_runs_flight_then_sync_then_pd`, which reads
      `["sync", "flight", "pd"]` without the inverted edge.
- [x] Back-edge 3 - lift `plugin.rs:107,111,115` into the assembly crate. The
      plugin wiring four crates belongs above all four. DEFERRED to the first
      seam cut ON PURPOSE: the destination is `nova_core`, and moving
      `NovaGameplayPlugin` there before any seam exists is a public break with
      nothing to show for it. Blast radius is one consumer
      (`nova_core/src/lib.rs:144`). Landed WITH the NOVAOS seam as the three
      `add_plugins` calls leaving `hud/mod.rs` for `nova_os_ui::NovaOsUiPlugin`,
      which `nova_core::AppBuilder` adds render-gated.
- [x] Confirm all three back-edges are resolved BEFORE any file moves.
- [x] Seam NOVAOS - cut `hud/nova_os*` (~14.3k lines): a terminal runtime that
      is not a HUD. Densest defect cluster and the biggest navigability win.
      Survey done, do not redo it: destination crate `nova_os_ui`
      (the name `nova_os` is taken by the existing pure-logic terminal/shell
      crate, which `nova_os_ui` depends on). Moving:
      `hud/nova_os/`, `hud/nova_os_map/`, `hud/nova_os_ship/`,
      `hud/nova_os_pointer_rig.rs`. Its whole crate-internal dependency
      surface is `audio`, `settings`, `objectives`, `GameStates`/`PauseStates`,
      `hud::NovaHudAssets` and `prelude::*` - so `nova_os_ui` depends on
      `nova_gameplay`, not the reverse. The only HUD->NOVAOS edges to break are
      in `hud/mod.rs`: the three `add_plugins` calls (lift with back-edge 3),
      the `nova_os::DRAWER_EXEMPT_Z` read at `:478`, and the
      `nova_os::NovaOsMonitorSettings` prelude re-export at `:64`.
      `nova_menu` also reads `NovaOsMonitorSettings`.
- [x] Seam HUD - cut the rest of `hud/` (19,015 lines) into `nova_hud`.
      Landed in L9.3. Survey done, do not redo it: only TWO edges pointed into
      `hud/` from the rest of `nova_gameplay` - `lib.rs`'s `hud::prelude::*`
      re-export and `plugin.rs:126`'s `add_plugins`. The other direction is
      `prelude`, `camera`, `sections`, `input`, `flight`, `gravity`, `mesh`,
      `transform`, `objectives`, `audio`, `asset_ref` - all staying in
      `nova_gameplay`, so `nova_hud -> nova_gameplay` and never the reverse.
      `nova_core::AppBuilder` adds `NovaHudPlugin` render-gated, before
      `NovaOsUiPlugin`. Consumers repointed: `nova_os_ui`, `nova_scenario`,
      `nova_assets`, `nova_menu`, `nova_debug`, `nova_core`.
- [x] Back-edge 4 - `juice.rs -> camera::shake`. Landed in L9.4a: `shake.rs`
      left `camera/` for the crate root, and the two edges it declared against
      `ChaseCameraSystems::Sync` were DELETED, not moved. `CameraAuthorityPlugin`
      already folds every camera-`Transform` writer into one total order, so
      those edges were a redundant weaker duplicate (they silently dropped
      whenever the chase plugin was absent). Pinned by
      `the_shake_brackets_the_chase_base_writer`, sabotage-verified.
      The other three L9.3 "back-edges" were NOT edges: `transform/mod.rs`,
      `mesh/mod.rs` and `damage.rs` only carry intra-doc links into `camera` /
      `sections`. They cost a reworded docstring at cut time, nothing more.
- [x] Seam FLIGHT - cut `flight/`, `sections/`, `input/`, `camera/`,
      `physics/` into a new crate. LANDED in L9.5 as `nova_ship`: 32,524 lines
      out of `nova_gameplay` across six directories, `NovaShipPlugin` owning
      the `SpaceshipSystems` brackets in both schedules, and `nova_core::
      AppBuilder` adding it after `NovaGameplayPlugin`. Every consumer
      (`nova_hud`, `nova_os_ui`, `nova_menu`, `nova_scenario`, `nova_assets`,
      `nova_debug`, `nova_editor`, `nova_core`) gained a manifest entry and a
      `use nova_ship::prelude::*`. `nova_gameplay` is 7% of the workspace,
      `nova_ship` 23%.
      THE MOVE, as it ran: SIX directories, not the five this step names -
      `flight/`, `sections/`, `input/`, `camera/`, `physics/` AND the new
      `ship_audio/` (L9.4e put the ship's soundtrack there for exactly this).
      `nova_ship`'s dev-dependency on `nova_gameplay` carries
      `features = ["test-support"]` (L9.4f). `NovaShipPlugin` takes over the
      `add_plugins` calls `plugin.rs` makes for those modules, including
      `ShipAudioPlugin`; `nova_core::AppBuilder` adds it after
      `NovaGameplayPlugin`.
      Back-edge 4 landed in L9.4f: `integrity/test_support.rs` moved to the
      CRATE ROOT as `nova_gameplay::test_support`, `pub`, behind a
      `test-support` cargo feature (`#[cfg(any(test, feature =
      "test-support"))]`). The crate root, not `integrity`, because 24 of its
      call sites are in `flight/`, `sections/`, `input/`, `camera/`, `damage`
      and `gravity` - it is the crate's avian app, not an integrity detail.
      The feature has NO consumer until the cut adds `nova_ship`'s
      dev-dependency, so it is proved for now by
      `cargo check -p nova_gameplay --features test-support`; workspace CI
      covers it the moment the cut lands. Do not add a CI job for it before
      then.
      Back-edge 2 landed as `crates/nova_gameplay/src/ship_audio/` -
      `combat.rs`, `cues.rs`, `loops.rs`, their per-cue volumes and throttle
      intervals, and a new `levels.rs` for the two loop curves
      (`engine_volume`, `rcs_volume`), behind a `ShipAudioPlugin` that
      `NovaGameplayPlugin` adds after `NovaAudioPlugin`. `audio/` keeps the
      generic engine (`sfx`, `registry`, `mixing`) plus `UiSfx` and the
      UI/NOVA-OS cue volumes. The seam is compiler-proved, not asserted:
      `audio/` now has ZERO `crate::` code references - `mixing.rs` dropped
      `use crate::prelude::*` for `use super::sfx::SfxCommandsExt`. The split
      of `mixing.rs` was NOT the plan's "mixing stays whole": its
      `ENGINE_MAX_VOLUME`/`RCS_MAX_VOLUME` and the three `*_MIN_INTERVAL`
      constants are ship tuning, so they went up with the cues and only the
      rolloff/throttle/listener stayed. `SfxThrottle::last` stayed private
      behind a new `tracked_keys()` accessor rather than being widened to
      `pub` for one cross-module test assertion.
      Back-edge 1 landed as a `nova_gameplay::markers` module holding TEN
      markers, not eleven: `ControllerSectionMarker` joined its three
      section-kind siblings rather than being left behind for one `audio/loops`
      read. `AINonCombatant` did NOT move down as planned - once
      `integrity::neutralize` stopped reading the AI, its only remaining
      consumer was the AI itself, so it goes UP with `nova_ship` and the survey's
      "eleven move, one inverts" is really "ten move, TWO invert".
      The neutralize inversion is `input::ai`'s `on_neutralized_stand_down`
      observing `On<Add, NeutralizedMarker>`; the gravity inversion moved
      `insert_gravity_affected_on_ai_ship` beside its marker. Each moved test
      moved with its observer.
      Back-edge 3 landed WITHOUT a test, on evidence - see the L9.4d commit
      message. It also MEASURED a new finding for the rule-10 step below: with
      `ambiguity_detection: Error` on FixedUpdate, there are **8 conflicting
      unordered pairs inside `SpaceshipSectionSystems` itself**. That set is not
      internally ordered, so no ordering test through it can fail.
      **The L9.3 survey was WRONG and its numbers must not be reused.** It
      counted only DEEP `crate::<module>` paths and so missed every reference
      that arrives through `crate::prelude::*` - which is most of them. The cut
      was attempted at L9.4a against that survey and reverted; the measurement
      below replaces it and was taken from the compiler, not from grep
      (`cargo check -p nova_gameplay` with the five directories removed).
      SIZES (unchanged, these were right): `input` 12,314, `sections` 10,126,
      `flight` 5,903, `camera` 3,552, `physics` 629 = **32,524 lines**, leaving
      11,635 behind.
      NAME: `nova_ship`, not `nova_flight`. The crate is the ship and how it is
      flown - `sections` (10k) and `input` (12k) are two thirds of it and
      neither is "flight". `nova_flight` would repeat the `hud/` mis-naming.
      The composition root is then `NovaShipPlugin` with no collision against
      the existing `flight::NovaFlightPlugin`.
      REAL BACK-EDGES, 129 compiler errors in four places:
      1. **The ship-structure markers** - `SpaceshipRootMarker`, `SectionMarker`,
         `SectionInactiveMarker`, `PlayerSpaceshipMarker`, `AISpaceshipMarker`,
         `AINonCombatant`, `TurretSectionMarker`, `ThrusterSectionMarker`,
         `TorpedoSectionMarker`, `ControllerSectionMarker`,
         `TorpedoProjectileMarker`, `TurretBulletProjectileMarker`. Defined
         across 8 files in `sections/` and `input/`, read by `integrity/`,
         `gravity.rs` and `audio/`. Eleven are plain unit structs and move down
         cheaply into a `nova_gameplay::markers` module. `AISpaceshipMarker` is
         the exception - it `#[require]`s `AIFireCadence`/`AIThreat`/`AIEvade`,
         which are AI behaviour and cannot move - so its two consumers
         (`gravity.rs`'s body classification, `integrity/neutralize.rs`) need an
         inversion, not a move.
      2. **`audio/` is two things**, exactly like `hud/` was. `mixing.rs`,
         `sfx.rs`, `registry.rs` and `mod.rs`'s plugin are a generic SFX engine
         that `nova_menu` and `nova_os_ui` also use. `combat.rs` (543),
         `cues.rs` (445) and `loops.rs` (748) are the SHIP's soundtrack: they
         read `SectionAmmo`, `TurretSectionInput`, `ThrusterSectionInput`,
         `FlightVerb`, `RcsIntent`, `WeaponsHot`, `WithheldVerbs`,
         `ImpactDestroySounds` and the radar-lock events. Those 1,736 lines go
         UP into `nova_ship`; the engine stays.
      3. **`gravity.rs:240` orders itself on `SpaceshipSectionSystems`** - a
         scheduling back-edge. Invert it the way L9.1 inverted back-edge 2 and
         L9.3 inverted the HUD seam: the ship declares the edge.
      4. **`integrity::test_support` is `#[cfg(test)] pub(crate)`** and is used
         by 22 test sites in `sections/`, `flight/`, `input/` and `camera/`. A
         `#[cfg(test)]` module is invisible across a crate boundary, so the cut
         needs it behind a `test-support` FEATURE on `nova_gameplay` that
         `nova_ship` dev-depends on. Do not just make it `pub` - that compiles
         the avian test harness into every release build.
      CONSUMERS: only 19 deep paths outside `nova_gameplay` name these modules
      (`nova_hud` holds 12), but every consumer reaches them through
      `nova_gameplay::prelude::*`, so each one gains a
      `use nova_ship::prelude::*;` and a manifest entry. `nova_core` is the only
      crate that adds `NovaGameplayPlugin`; it adds `NovaShipPlugin` after it.
      `nova_ship`'s prelude must NOT re-export `nova_gameplay`'s - `nova_hud`
      set the precedent of importing both at the use site.
      FOUR CORRECTIONS the cut earned, none of them in the survey:
      (a) `ship_audio` is PRIVATE, not `pub`. The survey assumed six public
      modules; the soundtrack has zero consumers outside the crate, so
      `mod ship_audio;` is the honest surface and it retires the rule-3 prelude
      question for that module rather than manufacturing an unused one.
      (b) `nova_core` re-exports every sub-crate BY NAME beside the prelude
      re-export (`pub use nova_gameplay;` and friends), and `nova_ship` was
      added to the prelude but not to that list. Nothing caught it until
      `--features debug`: `examples/sections/thruster_section.rs` reaches
      `nova_protocol::nova_ship::sections::...` only in a debug-gated import.
      The plain `cargo check --workspace --all-targets` is green either way -
      the default-features CI job would NOT have caught this, only the clippy
      job's `--features debug`.
      (c) The DOC surface, not just the code, carries the seam. `cargo doc
      --workspace --no-deps` went from 9 warnings on master to 23 on the
      branch: 14 intra-doc links the three seam cuts broke, spread over
      `nova_gameplay` (6), `nova_ship` (5) and `nova_os_ui` (3). A
      `[`crate::camera`]` link is silently dead the moment `camera/` leaves the
      crate, and nothing in CI reads it. Back to exactly 9 now. Cross-crate
      links that point DOWN the graph got a real path
      (`nova_gameplay::prelude::PlayerSpaceshipMarker`); links pointing UP the
      graph cannot resolve at all and became plain code spans.
      (d) Two of the broken links were not the seam's fault but its exposure:
      `play_positional` has not existed for some time and was referenced from
      two docstrings, and `SFX_ROLLOFF_FLOOR` is private. They only surfaced
      because L9.4e's `audio/` split made `distance_attenuation` public.
      PROOFS: `cargo check --workspace --all-targets`, `cargo clippy
      --workspace --all-targets --features debug -- -D warnings`, `cargo fmt
      --all --check`, `RUSTFLAGS=-D warnings cargo check --workspace --exclude
      nova_probe_cli --target wasm32-unknown-unknown` - all clean. `cargo test
      -p nova_ship --lib` 411 passed, `-p nova_gameplay --lib` 134 passed.
- [x] Seam CORE - SETTLED, and the answer is that there is no fourth crate.
      Two facts decide it. First, `nova_core` is TAKEN: it is the existing
      598-line app-assembly crate, so the planned name was never available.
      Second, the 11,635-line remainder is not "core primitives" - it is
      `audio` (2,770), `integrity` (2,392), `gravity` (1,127), `juice` (1,050),
      `mesh` (1,009), `transform` (867), `damage` (560), `settings` (552) and a
      tail of small files, and `math.rs` is 179 lines, not the `math/` directory
      this step assumed. That remainder is the shared gameplay layer and it
      keeps the name `nova_gameplay`. Splitting it again to manufacture a
      "CORE" would put `math.rs` (179) and `cooldown.rs` (182) in a ceremonial
      crate, which is the shuffling-without-improving outcome the task problem
      statement rules out.
      What the step was RIGHT about is "shared markers": the FLIGHT survey found
      them, and they land as a `markers` module inside `nova_gameplay` rather
      than as a crate. The four-way split is therefore a THREE-way one -
      `nova_os_ui`, `nova_hud`, `nova_ship` - over a `nova_gameplay` base.
- [x] Rule 10, per seam - declare a `SystemSet` for each of the 68 plugins that
      has none, and give each new crate a `configure_sets` block that proves the
      seam is real and the order intentional.
      MEASURED at L9.6, and the second half of this step is BLOCKED ON AN OWNER
      RULING - do not just add 16 sets. The `configure_sets` half is already
      done: each new crate got its block with its seam. The "68 plugins" half
      re-measures to **16**, and inspecting all 16 says none of them wants a
      set:
      `AsteroidPlugin`, `BeaconPlugin`, `SalvageCratePlugin`, `RenderScalePlugin`
      (nova_scenario), `GameAssetsPlugin`, `PortalPlugin` (nova_assets),
      `NovaAudioPlugin`, `NovaJuicePlugin`, `NovaSettingsPlugin` (nova_gameplay),
      `SectionDamageTintPlugin` (nova_ship), `LoadingScreenPlugin` (nova_core),
      `FrameTimePlugin` (nova_probe), and four dev-only `nova_debug` plugins
      (`InspectorDebugPlugin`, `ScenarioLoadedAssertPlugin`,
      `ScreenshotHotkeyPlugin`, `WireframeDebugPlugin`).
      Every one is a LEAF: one system, or an internally `.chain()`ed pair, with
      no other plugin needing a handle to order against it. Where a real
      constraint exists they already state it directly on the system -
      `NovaJuicePlugin` is the model, with `draw_juice_flashes.after(
      TransformSystems::Propagate)` and no set at all. Wrapping these 16 in
      `<Name>Systems::Sync` enums nothing references produces 16 more of the
      six sets L9.6 just declined to invent, and it is precisely the "shuffling
      code around but still getting no actually good result" the PROBLEM
      statement rules out.
      **THE RULING WANTED** (same question as L9.6's, and these two are the
      whole remaining rule-10 balance): rule 10 currently reads "every subsystem
      plugin declares a `SystemSet` and orders it", but its own justification is
      "a set nothing is ordered against records nothing". The codebase's good
      examples - `CameraAuthoritySystems`, `SpaceshipSectionSystems`,
      `NovaHudSystems`, and now `TurretSectionAimSystems` - are all sets that
      exist because ANOTHER plugin needs to order against them. Proposed
      rewrite: *declare the edge wherever a real constraint exists, and declare
      a `SystemSet` when another plugin needs a handle for it.* Under that
      wording rule 10's remaining open sites drop to zero and the CONVENTIONS.md
      "Not yet true" row for rule 10 can be deleted. Under the current wording
      22 ceremonial sets get written. Owner picks; L9 does not.
      **RULED 2026-08-08: the rewrite is adopted.** Landed in L9.8. Rule 10 is
      now "state every ordering constraint; declare a `SystemSet` when another
      plugin needs the handle", written as two greppable violations rather than
      one count of plugins. The 16 leaf plugins close as-is - no set, and the
      constraints they do have already stated on the system.
      The rewrite is STRICTER on the other six, and that half was real work,
      not bookkeeping: a set with no outside orderer is now a violation, so
      `DirectionalSphereOrbitSystems`, `PointRotationSystems`,
      `SphereOrbitSystems`, `SphereRandomOrbitSystems`, `TempEntitySystems` and
      `StatusBarSystems` were DELETED with their `in_set` calls and prelude
      exports, joining `WASDCameraControllerSystems` from L9.6. Behaviour is
      unchanged - an unconstrained set imposes no ordering, which is the same
      fact that made them worthless. `cargo check --workspace --all-targets` is
      clean and no reference survives outside this task folder.
      One downstream citation was invalidated and has been corrected in place:
      L11's F65 offered "add the edge between `SpaceshipSectionSystems` and
      `TempEntitySystems::Sync`" as its better fix. If L11 takes that route it
      re-declares the set, which the new rule permits precisely because the
      edge gives it a holder.
- [x] Rule 10 first slice - order the 16 declared-but-unordered sets
      (`DirectionalSphereOrbitSystems`, `HudSituationSensingSystems`,
      `IntegritySystems`, `NovaOsMapSystems`, `NovaOsShipSystems`,
      `ObjectivesPluginSystems`, `PointRotationSystems`,
      `SmoothLookRotationSystems`, `SpaceshipTargetingSystems`,
      `SphereOrbitSystems`, `SphereRandomOrbitSystems`,
      `StatusBarPluginSystems`, `TempEntitySystems`, `TurretSectionAimSystems`,
      `TweenSystems`, `WASDCameraControllerSystems`). Re-count AFTER L5 -
      `TweenSystems`, `StatusBarPluginSystems` and `ObjectivesPluginSystems`
      retire there.
      RE-MEASURED at L9.6 (do not reuse the 16): 32 sets exist, 25 carry a real
      ordering edge, **7 do not**. The list shrank for three reasons - L5
      retired `TweenSystems`, `StatusBarPluginSystems` and
      `ObjectivesPluginSystems`; the seams ordered `NovaOsMapSystems` and
      `NovaOsShipSystems` (F53); and the original count only looked at
      `configure_sets`, so it wrongly flagged `HudSituationSensingSystems`,
      `IntegritySystems` and `SpaceshipTargetingSystems`, which are ordered
      with `.before`/`.after` on `add_systems` instead.
      **Ordering is not always the answer**, so the remaining 7 are
      dispositioned rather than mechanically ordered:
      - `WASDCameraControllerSystems` - DELETED. The plugin adds zero systems
        (it is all observers), so the set could never contain anything; it was
        declared, exported through the prelude, and never used in an `in_set`.
      - `SmoothLookRotationSystems` / `TurretSectionAimSystems` - the one REAL
        missing edge, and it is cross-crate. Both run in `PostUpdate` and both
        touch `SmoothLookRotationTarget`/`Output`: the aim chain reads the
        output and writes the target, `Sync` does the reverse. Landed as
        `TurretSectionAimSystems.before(SmoothLookRotationSystems::Sync)` in
        `TurretSectionPlugin` - the driver's crate declares it, because
        `nova_gameplay` owns a generic rig that names no driver (same shape as
        `CameraShakePlugin` vs `CameraAuthorityPlugin`).
      - `DirectionalSphereOrbitSystems`, `PointRotationSystems`,
        `SphereOrbitSystems`, `SphereRandomOrbitSystems`, `TempEntitySystems`,
        `StatusBarSystems` - NO edge available and none invented. Every one is
        a single-`Sync` enum over one chain whose consumers sit in a DIFFERENT
        schedule (the rigs write `*Output` in `PostUpdate`; the camera, HUD and
        intent systems that read it run in `Update`, so the one-frame lag is
        the design, not a race). An edge inside one schedule cannot express
        that, and a set nothing orders against is what rule 10 already calls
        worthless. They stay as public ordering handles for downstream apps.
        **OWNER CALL WANTED**: rule 10 as written ("every subsystem plugin
        declares a `SystemSet` and orders it") cannot be satisfied by these
        six; either the rule gains a cross-schedule exemption or the six sets
        get deleted like the WASD one.
      Proof: `the_aim_chain_is_ordered_against_the_rig_it_steers` in
      `turret_section/mod.rs`. It asserts through `ambiguity_detection: Error`
      on a two-plugin app, NOT through an observed order - the first draft read
      the order back and passed WITHOUT the edge, because bevy's tie-break
      happens to supply it today. That vacuous-green is exactly how the missing
      edge stayed hidden, and it is the trap any remaining rule-10 test has to
      dodge.
- [x] F53 - the NOVAOS seam's first `configure_sets` block covers
      `nova_os_ship/mod.rs:166` and `nova_os_map/mod.rs:139`, which are declared
      and never ordered. The measurement shows F53 is not 2 sites, it is 16.
      Landed: `terminal/mod.rs:157,167`, `ship/mod.rs:116`, `map/mod.rs:125`.
- [x] F53 follow-through - once the ordering is real, DECIDE whether
      `peek_pending_invocation` (`nova_os_ship/app.rs:195`) is deletable; it
      exists because of the missing edge. That is exactly the deletion criterion
      #2 wants. DECISION: KEEP. Re-read with the ordering in place, the peek is
      not an ordering workaround. `ship ...` and `map ...` share ONE pending
      invocation slot, so whichever handler runs first - in ANY total order -
      would swallow the other family's verb. The peek is ownership dispatch on a
      shared slot, and the only thing that deletes it is a per-app queue, which
      is a bigger change than the workaround it removes. `nova_os`'s docstring on
      `peek_pending_invocation` already states exactly this reason.
- [x] F81 - add `#[derive(SystemParam)] struct NovaOsAppInput` for the identical
      6-param cluster in `map_input` (`nova_os_map/scene.rs:259`) and
      `ship_input` (`nova_os_ship/scene.rs:336`); removes two
      `too_many_arguments` suppressions. The struct has to sit on one side of
      the seam regardless. Local idiom: `nova_os_ship/sections.rs:223`. Landed at
      `terminal/input.rs:467`, used by both `scene.rs` input systems.
- [x] Audit the 633 crate-local `pub` items (nova_gameplay holds 358) as each
      seam decides what crosses its boundary. Truly dead items: zero - this is
      "tighten what is public", not "delete what is unused".
      FLIGHT seam's share is decided: `nova_ship` publishes `camera`, `flight`,
      `input`, `physics` and `sections`, and keeps `ship_audio` PRIVATE - it
      has no consumer outside the crate, so nothing about the soundtrack is
      API. Two `pub use` re-exports that only fed the old crate-internal
      reach were deleted with it.
      LANDED at L9.7. **`rustc -W unreachable_pub` over all four crates reports
      ZERO** - every `pub` item is reachable from outside, so the audit is not
      about reachability and mass-demoting by usage would be wrong: these crates
      ARE the game's API and `nova_hud` alone has 128 of 169 boundary names with
      no in-workspace consumer yet. What the audit CAN prove is composition, and
      that is what shipped:
      - `nova_os_ui` publishes `NovaOsUiPlugin`, `NovaOsMonitorSettings`,
        `MapContactCode` and `SectionCode` - nothing else. `NovaOsPlugin`,
        `NovaOsMapPlugin`, `NovaOsShipPlugin` and `MonitorFrame` are added BY
        `NovaOsUiPlugin` and never by a consumer, so all four are `pub(crate)`
        and off every prelude. Adding them as `pub(crate) use` re-exports first
        produced three `unused_imports` warnings, which is the compiler saying
        an internal plugin belongs on no prelude at all - the composition root
        names its own submodules directly.
      - FOURTEEN `nova_ship` leaf modules are now private (`input::ai::{
        acquisition, behavior, guns, threat, torpedo}`,
        `input::player::{hints, weapons}`, `input::reference`,
        `input::targeting::{component_lock, contacts, gesture, state}`,
        `physics::{pd_controller, rigid_body}`). Their parents' preludes
        already re-export every item they define and NOTHING outside
        `nova_ship/src` names any of them, so they were `pub` announcing a
        boundary nobody crossed. This also closes 14 rule-3 sites by DELETION
        rather than by writing 14 more preludes - see the rule-3 note added to
        `CONVENTIONS.md`.
      Cost: three intra-doc links in `nova_ship` broke the moment the modules
      went private (`physics/mod.rs` x2, `input/mod.rs` x1) and now point at
      the public item each file supplies, via `prelude::` - a module-root link
      does not resolve for a name that only exists in the prelude.
- [x] Rules 3+4 - 26 module preludes, written in the same pass as the
      visibility audit. `math` alone is 5 of the deep-import violations and is
      already moving.
      `nova_ship` is DONE: a prelude on every public module (`camera`,
      `flight`, `input`, `input/ai`, `input/player`, `input/targeting`,
      `physics`, `sections`, `sections/turret_section`,
      `sections/torpedo_section`) plus the crate one, which re-exports them by
      name and deliberately does NOT glob `nova_gameplay`'s. The 8 remaining
      deep `crate::a::b::` imports are all inside `#[cfg(test)] mod tests`
      blocks reaching sibling `test_support` - rule 4 is about production
      imports, so they stay.
      LANDED at L9.7 for the other three crates. RULE 3: `nova_hud` and
      `nova_os_ui` were already at zero; `nova_gameplay` was missing three
      (`audio`, `plugin`, `settings`) and the crate prelude listed their items
      inline instead - now `audio::prelude::*`, `plugin::prelude::*`,
      `settings::prelude::*`, which is rule 3's whole stated payoff (a new
      public item is a one-line edit inside its own module). `test_support` and
      `test_log` stay prelude-less on purpose: they are `#[cfg(any(test,
      feature = "test-support"))]` rigs, not API.
      RULE 4: the epic measures it as `use crate::a::b::` (TWO segments), and by
      that measure `nova_gameplay`, `nova_hud` and `nova_os_ui` are all at zero.
      The one-segment reaches were fixed anyway where they bypassed a real
      prelude: `integrity/core.rs -> damage::prelude`, `settings.rs ->
      juice::prelude`, `keybind_dock.rs -> key_glyphs::prelude` (x2, which
      wanted `KEY_GLYPH_FILES` on that prelude beside its sibling
      `KEY_GLYPH_DIR`), and five `nova_os_ui` terminal files that reached
      `nova_gameplay::{audio,objectives,settings}::` across the crate boundary.
      That last one moved the TEN `NOVA_OS_*` cue volumes onto `audio::prelude`
      - they are boundary items by the module's own docstring ("`pub` because
      the cues are fired from `nova_os_ui`"). Also `nova_probe ->
      nova_ship::flight::prelude` and `nova_scenario`'s skybox e2e test ->
      `camera::skybox::prelude`.
      ONE REVERT, and it is the interesting one: `HudReadoutFormat` was added to
      `nova_hud::readout::prelude` and rustc rejected the workspace with
      `ambiguous glob re-exports` - `nova_scenario` exports an AUTHORING twin of
      the same name and `nova_core`'s prelude globs both crates. It is back off
      the boundary with a comment saying why, and `nova_scenario/src/world.rs`
      keeps the fully-qualified path on both sides at the one site that converts
      between them. A prelude is not free: it is a name claim in every
      downstream glob.
      `CONVENTIONS.md`'s `## Not yet true` table is re-measured against the
      tree: rule 3 is 80 -> **15**, rule 4 is 36 -> **25**, rule 1 is 28 -> **1**,
      rule 10 is 84 -> **22 and blocked on the owner ruling**. The table now
      names the owning crate for every remaining site, so L7 and L10 inherit a
      work list instead of a number. It does NOT empty here - rules 3 and 4
      still hold `nova_assets`, `nova_scenario`, `nova_ui`, `nova_probe_cli` and
      `nova_editor` sites, which are L7's and L10's.
- [x] Run `probe run --all` PER SEAM, not once at the end.
      FLIGHT seam: `cargo run -p nova_probe_cli -- run --all` at `31cdf5dd`,
      aggregate **OK**, 24/24 rows OK, each `measured 6/7`
      (`probe-runs/31cdf5dd/index.html`). The one unmeasured check across the
      fleet is `fps_within_baseline` - no `--fps` pass in a clean run.
      VISIBILITY PASS (L9.7), at `7af7fb3d` + this working tree: 23/24 OK, each
      `measured 6/7`, and the aggregate read **FAIL** on ONE row -
      `screenshot_combat` (`probe-runs/7af7fb3d/index.html`).
      DIAGNOSED, and it is NOT this pass's change - `probe run
      screenshot_combat` on the same tree is **OK**, 7/7 clean, `run_end at
      frame 911`. The fleet run's only offending line was
      `nova_autopilot: step \`track the torpedoes in\` stalled after 12.0s (run
      39.0s)`; that watchdog is WALL-CLOCK, and the failing run had pushed to
      1044 frames against a box running the other 23 examples. Under load the
      step misses a 12s deadline it clears comfortably alone.
      That is a real finding about the harness, not about the game: a
      wall-clock step watchdog inside a whole-fleet run measures the host, so
      `run --all` will FAIL intermittently on a loaded machine and the row that
      fails will move. It is L11's or the probe lane's to fix (a frame-budget
      deadline, or serialising the autopilot-heavy examples); do not re-key it
      as a combat regression. Evidence for the call is one isolated re-run, not
      a repeat count.
- [x] Note as you go which `keys/tier1.json` questions each move invalidates
      (`_coverage` maps ids to areas; `nova_os_hud_seam` is 5 of 30), so L2's
      single re-keying pass is not a reconstruction from memory.
      NOVAOS seam, 7 questions to re-key (running list, do not re-derive):
      `t1-001` expect+citation -> `crates/nova_os_ui/src/terminal/`; its DOCS CUE
      note also changes, CONVENTIONS.md rule 4's bad-import example is now
      `crate::terminal::shell::*` and no longer discloses a `hud/` path.
      `t1-007` -> `nova_os_ui/src/terminal/components.rs`; the `nova_menu`
      half is unchanged. `t1-008` both numbers move: `hud/` is now 19,015 lines
      and the NOVA OS subtree is out of it entirely (15,200 in `nova_os_ui`),
      which is the whole point of the question. `t1-010` ->
      `nova_os_ui/src/{map,ship}/scene.rs`. `t1-022` citation ->
      `nova_os_ui/src/{map/contacts.rs,ship/sections.rs}`. `t1-024` ->
      `nova_os_ui/src/terminal/input.rs`. `t1-030` implementor citation ->
      `nova_os_ui/src/{map,ship}/app.rs`; its ANSWER (none) still holds.
      `t1-009` is unaffected - `nova_os` still owns the model and still draws
      none of it.
      HUD seam, 2 more (running list continues): `t1-005` citation ->
      `crates/nova_hud/src/ammo_readout.rs:485`; its ANSWER (`nova_ui`) is a
      CONTROL and still holds, but the "HUD reaches it by deep path" note now
      describes a CROSS-CRATE deep path, which is a different (and weaker)
      finding than the same-crate one the question was keyed on. `t1-008` is
      now OBSOLETE, not merely re-cited: the folder it asks about
      (`crates/nova_gameplay/src/hud/`) does not exist, and the lie it probed -
      a HUD folder that is 43% NOVA OS runtime - is exactly what L9.2 and L9.3
      deleted. L2's re-keying pass has to REPLACE it with a question that
      probes the new structure, or the tier-1 set silently loses a slot.
      FLIGHT seam, 4 more (running list continues). `t1-006` (plugin add
      order) needs BOTH halves re-cited and is arguably a BETTER question now:
      the order is decided in three files, not two -
      `nova_ship/src/lib.rs` (`NovaShipPlugin::build`, the leaf adds and the
      two `configure_sets` blocks), `nova_gameplay/src/plugin.rs` (what is left
      of it) and `nova_core/src/lib.rs` `AppBuilder`. Its `notes` scoring rubric
      is keyed to TWO files and must be rewritten for three.
      `t1-014` citation only: `nova_assets/src/sections.rs` now opens with BOTH
      `use nova_gameplay::prelude::*` and `use nova_ship::prelude::*`, and the
      `why_this_question` collision it describes is now
      `nova_ship/src/sections/` vs `nova_assets/src/sections.rs` - the same
      trap, one crate over.
      `t1-026` is OBSOLETE, like `t1-008`. Its whole answer was that
      `NovaGameplayPlugin::render` gates exactly one plugin while claiming
      three. After the cut it gates the `nova_ui` wiring and nothing else it
      does not claim, `SpaceshipSectionPlugin` moved to `NovaShipPlugin`'s own
      `render` field, and the HUD is render-gated by `AppBuilder`. The lying
      surface it probed is gone; L2 must replace it, not re-cite it.
      `t1-027` was ALREADY stale before this seam and needs re-measuring, not
      re-citing: its cited site (`nova_debug/Cargo.toml:18` hard-forcing
      `features = ["debug"]` on `nova_gameplay`) no longer exists - the
      manifest now lists plain `nova_gameplay` and `nova_ship` deps. Whichever
      earlier lane dropped that line did not note it here. Its expected answer
      ("all builds") may now be false, which changes the question, not its
      citation.
      VERIFIED at L9.7 against the tree, not from memory: every `crates/...`
      path in `benchmark/keys/tier1.json` was resolved with the filesystem, and
      **13 questions carry at least one citation that no longer exists**. The
      running list above covers L9's eight (`t1-001`, `t1-005`, `t1-007`,
      `t1-008`, `t1-010`, `t1-014`, `t1-024`, `t1-030`) and nothing in it is
      stale - `t1-006`, `t1-022`, `t1-026` and `t1-027` need re-keying for
      CONTENT, which a path check cannot see, so the prose list stays load
      bearing.
      FIVE OF THE THIRTEEN ARE NOT L9's, and no lane had noted them:
      `t1-003` (`nova_probe/src/run_report/`), `t1-004`
      (`nova_probe/src/capture.rs`), `t1-011`
      (`nova_probe/src/bin/probe/native/{env,supervise}.rs`) and `t1-012`
      (`nova_probe/src/recorder.rs`) were all invalidated by L8's `nova_probe`
      restructure; `t1-016` (`nova_assets/src/bin/content.rs`) by an earlier
      lane. That is four of `_coverage`'s five `nova_probe` questions dead
      without a note. L2's re-keying pass must run this path check FIRST - it is
      one script and it finds what a lane forgets to write down.

### Lane10 - "NOVA_ASSETS / NOVA_SCENARIO CLEANUP" - tasks/20260806-121625/plan/lane10.md

BLOCKS the baseline, lands AFTER it. Depends on L2 and L3. Independent of L9,
so it can run in parallel with it.

LANE COMPLETE 2026-08-08, sprout `refactor/l10-assets-scenario-cleanup`, five
commits: `04813d60` L10.1 nova_authoring, `577a453c` L10.2 the Storage trait,
`1d828e00` L10.3 the two HudReadoutFormat halves, `06819ae3` L10.4
render_scale's crate doc, `6fbbb4a6` L10.5 the preludes. Every step ticked;
gates in the last step.

REBASED OVER L9 AND READY TO LAND (2026-08-08). The lane was cut at `e0b374e0`
and L9 (`54ebcc2a`) landed the `nova_gameplay` four-way split underneath it,
so the SHAs above are the REPLAYED ones - the pre-rebase five (`72f86261`,
`a52f990d`, `7b5a6991`, `0f24b066`, `8a41de9e`) are preserved on
`backup/l10-prerebase` and are what the proofs in the steps below were first
measured against. Every gate was re-run on the replay; see "The rebase" below.

- [x] Create `nova_authoring` and move `lint_walk.rs`, `balance.rs`,
      `content_report.rs`, `scenario_generation.rs`, `bin/content.rs` (as the
      crate's binary) and `nova_scenario/src/lint/` into it.
      CORRECTED against the code: `nova_scenario/src/lint/` STAYS. The next
      step's test excludes it - `nova_assets/src/merge.rs:285` calls
      `lint_scenario` in the runtime merge sweep, so the shipping game does
      link it, by design. The `scenario/` builders and `sections.rs` moved too;
      they are `scenario_generation`'s inputs and nothing else reads them.
- [x] Verify the test that justifies the move: the game binary does not link
      the linter. Anything in the moved set reachable from a running game did
      not belong in the move.
- [x] Move `assets/base/**` to sit with the tool that generates it, not the
      runtime crate that reads it.
      NOT MOVED, premise falsified. `assets/base/**` does not live with
      `nova_assets` - it is under the repo-root `assets/`, which IS the bevy
      asset root `nova_core::assets_plugin` hands `AssetPlugin`. The shipping
      game loads it from there at runtime, as do the `mods://` source, the
      wasm bundle and the deploy. Moving it under `crates/nova_authoring/`
      would break loading to satisfy a filing preference. The generator's own
      path (`bin/content/native.rs:62`, `../../assets`) still resolves after
      the crate move - `nova_authoring` sits at the same depth.
- [x] Add `crates/nova_assets/src/storage.rs` with
      `trait Storage { read, write, remove }`, mirroring the existing
      `PortalTransport` pattern.
      `remove` DROPPED on YAGNI: nothing in the workspace deletes a persisted
      value, and an unused trait method is a contract no impl is held to. The
      trait is `read` + `write`.
- [x] Extend the trait's `write` from L3's F07 contract - atomic on native
      (temp + fsync + rename), a single `set_item` on wasm - rather than
      absorbing a free helper and rewriting the same four call sites.
      `write_atomic` moved `persist.rs` -> `storage.rs` (it is the native
      backend's primitive, not the codec's) and IS `NativeStorage::write`. Its
      three path-owning callers - the mod cache index, the portal catalog, the
      content generator - were repointed, not rewritten.
- [x] Add `NativeStorage { root }` and `WebStorage`; the two impls already
      exist behind `persist.rs`'s `#[cfg(target_arch = "wasm32")]` split at
      `:75-98`, they are just not behind a trait.
      Selected once by `storage::platform() -> Option<PlatformStorage>`.
      `WebStorage::key` compiles on native too, so the localStorage key
      derivation keeps the test it had - a typo there orphans every web
      player's save and no wasm test runs in CI.
- [x] Delete the `#[cfg(target_arch = "wasm32")]` gates the trait replaces. Do
      NOT re-argue this from bit-rot - W3 withdrew that; all 14 crates
      type-check clean on wasm32. The case is testability and gate removal.
      HONEST COUNT: the gates did not disappear, they CONCENTRATED.
      `persist.rs` went 7 -> 1 (the surviving one gates its test module) and
      `mod_prefs` / `settings_store` are gate-free; `storage.rs` now carries
      12, all of them impl selection in the one module that owns the split.
      The win is that no caller branches on the target and every store is
      testable through `NativeStorage::at(tmpdir)` - which is what the two
      `nova_menu` test seams now use instead of path-explicit helpers.
- [x] Route the four scenario -> HUD coupling sites through `nova_events`:
      `world.rs:138-144`, `actions/mission.rs:512,534,554`. These are the sites
      `AGENTS.md:102` was actually about - route them because they are
      scenario-observable moments, not because of a blanket rule.
      **PREMISE FALSIFIED; NOT routed through `nova_events`.** The four cited
      sites are one production write plus three test reads of the SAME thing:
      the `HudReadouts` mirror in `state_to_world_system`. That mirror is the
      deliberate, documented pattern the same function already uses for
      `GameObjectives` and `StoryFeed` (write-on-diff, resource-guarded); a
      `nova_events` kind is the wrong shape for it twice over - the readout
      value is rebuilt EVERY frame off a live variable, so an event stream
      would be one event per readout per frame, and a `nova_events` kind is
      something scenarios FILTER and DISPATCH on, which a presentation mirror
      is not. The authored moment - the `HudReadout` action - already runs
      through the event engine.
      The real defect the finding saw was the FULL-PATH reach
      (`nova_gameplay::hud::readout::HudReadoutFormat`), and
      `readout.rs`'s own prelude doc already recorded its cause: a name
      collision with nova_scenario's authoring enum, which nova_core globs
      alongside it. Fixed at the cause - the scenario-side enum is now
      `HudReadoutFormatConfig` (matching its `*ActionConfig` siblings and the
      `StoryMessageActionConfig -> StoryLine` split), `HudReadoutFormat` joins
      `hud::readout::prelude`, and the sync's 10-line inline match is a
      `From<HudReadoutFormatConfig>` impl beside the enum it mirrors. Zero
      full paths remain in `world.rs` and the three mission tests. The variant
      names are what RON serializes, so no content changed -
      `content_ron_parity` passes without regeneration.
- [x] Lift `render_scale` out of `nova_scenario` into whichever crate owns the
      render settings; decide by reading its consumers, not in advance.
      **NOT LIFTED - the consumers forbid it, which is what reading them was
      for.** Both `reconcile_render_scale` and its teardown query
      `With<ScenarioCameraMarker>`, a `nova_scenario` type this crate spawns
      (`loader/lifecycle.rs:644`). The crate that owns the render settings is
      `nova_gameplay` (`settings.rs:180`, `GraphicsBudget::render_scale`), and
      its Cargo.toml deps are `nova_events`, `nova_info`, `nova_os`, `nova_ui` -
      no `nova_scenario`, by the same rule that keeps the HUD off it. The move
      would invert the crate dependency to relocate a file.
      The split is already correct and was mis-read as a violation: the tier
      picks the fraction (nova_gameplay), the scenario view applies it
      (nova_scenario). What WAS wrong is that `lib.rs:1-10` introduced the
      module list as "the vocabulary a scenario is built from" and then listed
      `render_scale` inside it - the one module that is not vocabulary. The
      crate doc now names it as the exception and says why it lives here.
- [x] Rules 3+4 - 13 module preludes in `nova_assets` (13 public modules, 1
      prelude today) and 2 in `nova_scenario`, each written at the moved
      module's NEW home.
      **9 in `nova_assets`, not 13** - and the count was right when it was
      written. L10.1 moved four modules out to `nova_authoring`, so the crate
      is 6 public modules (`mod_cache`, `mod_prefs`, `mod_refs`, `persist`,
      `portal`, `storage`) plus 4 private ones (`collections`, `merge`,
      `mod_set`, `plugin`). `storage` already had its prelude from L10.2, so
      this step wrote the other 9. The private four get one too: rule 3 is
      about every module that EXPORTS items, and their export path is the
      crate root, which is now four `<module>::prelude::*` lines instead of a
      hand-maintained item list that had drifted from the modules twice.
      The three cfg-split modules carry their gates INSIDE the prelude
      (`mod_set`, `mod_cache`). `mod_cache` is the one that matters: both
      platforms export the same five file-bytes names with different
      signatures (native `io::Result`, wasm `async`), so an ungated prelude
      would not compile on either target. Verified on both.
      `nova_scenario` was exactly 2 as planned (`world`, `render_scale`) - the
      other seven modules already had theirs - and its crate prelude is now
      nine uniform `::prelude::*` lines with no item list left in it.
- [x] Confirm L3's F57 regeneration landed as its own commit BEFORE the content
      move; otherwise the `content_ron_parity` diff is unreviewable.
      Confirmed, and the hazard never existed. `git log -- assets/base/` shows
      NO commit from this epic: the F57 regeneration was a byte-for-byte
      NO-OP, so there was no generated churn for a content move to hide behind.
      `content_ron_parity` passing on the unchanged committed tree is the
      proof that the builders and `assets/base/**` still agree. The content
      move itself was falsified three steps up, so the ordering constraint is
      moot on both ends.
- [x] Verify with `content -- lint`, `content_ron_parity` and the `shakedown`
      scenario walk.
      `content -- lint`: 0 errors, 0 warnings, 0 findings, 14 scenarios
      balance-audited, 1 acked. `content_ron_parity`: 2/2. Both now run under
      `-p nova_authoring`, not `-p nova_assets` - L10.1 moved the binary and
      the test with the toolchain, so DoD proof 7's `-p nova_assets --bin
      content` no longer resolves and needs the same correction at review.
      The scenario walk ran as the three nova_assets scenario e2e suites that
      exercise the merge pipeline and the readout path this lane touched -
      `example_scenario` 14/14, `gauntlet_course` 12/12, `lifeline_convoy`
      8/8. The windowed `probe run --all` walk is the epic's own final-run
      step, not repeated per lane.
      Gates across the whole lane: `cargo check --workspace --all-targets`
      green, `cargo check -p nova_assets -p nova_menu --target
      wasm32-unknown-unknown` green, `cargo clippy --workspace --all-targets
      -- -D warnings` clean at the configuration L0 added to CI.

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
      **`TempEntitySystems` NO LONGER EXISTS** - L9.8 deleted it as one of the
      six handle-with-no-holder sets. That does NOT rule out the edge fix: if
      you take it, re-declare the set in `lifetime.rs`, because then it HAS a
      holder and the rewritten rule 10 is satisfied by the very edge you are
      adding. Re-read the race first - the double despawn is an idempotency
      bug as much as an ordering one, and `try_despawn` closes it with no new
      public type.
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

## Close-out - L9.2, the NOVAOS seam (2026-08-08)

### What and why

`crates/nova_os_ui` now owns the NOVA OS cockpit monitor: 15,200 lines that were
sitting in `nova_gameplay/src/hud/` under a folder name that lied. The monitor is
a terminal runtime with a screen; the HUD is instruments drawn over the world.
They share exactly two things - the `PauseStates::NovaOs` axis and the
`HudNovaOsExempt` tag - which is what made the cut possible without a new
abstraction.

| Was | Is |
| --- | --- |
| `hud/nova_os/` | `nova_os_ui/src/terminal/` |
| `hud/nova_os_map/` | `nova_os_ui/src/map/` |
| `hud/nova_os_ship/` | `nova_os_ui/src/ship/` |
| `hud/nova_os_pointer_rig.rs` | `nova_os_ui/src/pointer_rig.rs` |

`hud/` drops from 33,774 to 19,015 lines and `nova_gameplay` from 54% of the
workspace to 43%. Direction of the seam: `nova_os_ui -> nova_gameplay`, never the
reverse. The whole crate-internal surface the moved code needed was `audio`,
`settings`, `objectives`, `GameStates`/`PauseStates`, `NovaHudAssets` and the
prelude - all of which the monitor CONSUMES.

Back-edge 3 landed with the seam, exactly as the plan deferred it. The three
`add_plugins` calls left `hud/mod.rs` for `NovaOsUiPlugin`, which
`nova_core::AppBuilder` adds render-gated. `nova_menu` picked up a direct
`nova_os_ui` dependency for `NovaOsMonitorSettings` instead of reading it through
`nova_gameplay`'s prelude. Nine audio cue volumes went `pub(crate)` -> `pub` with
a docstring each; they stay in `audio/` so every cue volume in the game is still
declared in one file.

The two HUD->NOVAOS edges the survey found both resolved by moving, not by
adding an interface: `lift_exempt_chrome_over_nova_os` went with
`DRAWER_EXEMPT_Z` into `terminal/shell.rs` (it is a NOVA OS behaviour that
happens to write HUD entities), and the `NovaOsMonitorSettings` re-export left
`hud::prelude`.

F53 and F81 both closed here, because both are questions the seam forces.
F81 is `NovaOsAppInput` in `terminal/input.rs:467`, one `SystemParam` replacing
an identical 6-param cluster in `map/scene.rs` and `ship/scene.rs` and two
`too_many_arguments` suppressions.

F53 is the monitor frame: `NovaOsSystems::{Toggle, Input, Simulate, Paint}` plus
`NovaOsMapSystems` and `NovaOsShipSystems`, ordered
`Toggle -> Input -> map -> ship -> Paint`, with Input/Simulate/Paint inside
`NovaHudSystems`. Before this, both app sets were declared and never passed to
`configure_sets`, so whether a `ship repair` result row reached the screen this
frame or the next was bevy's topological tie-break.

### Alternatives considered

**Where the set ordering lives.** First written as three `configure_sets` blocks,
one per plugin - which is what rule 10 literally asks for. Moved to a
`MonitorFrame` plugin at the crate root, because every edge in it crosses between
the three plugins: `map` before `ship` is a statement about a slot they share,
and it cannot be owned by either one. The split also makes the contract testable:
`NovaOsPlugin` pulls in `UiMaterialPlugin<NovaOsCrtMaterial>` and panics without
the render stack, so a test that adds `NovaOsUiPlugin` cannot run headless while
one that adds `MonitorFrame` can.

**Deleting `peek_pending_invocation` (F53 follow-through).** The lane predicted
the peek was a workaround for the missing ordering and would become deletable
once the ordering was real. Re-read with the ordering in place: it is not. The
`ship ...` and `map ...` handlers share ONE pending invocation slot, so in ANY
total order the first handler to run reaches a slot that may hold the other
family's verb. The peek is ownership dispatch, and only a per-app queue removes
it - a bigger change than the workaround it deletes. KEPT, and the reason was
already written on `nova_os`'s docstring.

**A `nova_os_ui` -> `nova_gameplay` interface trait.** Rejected: the dependency
is one-directional and shallow. A trait would exist only to make the arrow look
symmetric.

### Difficulties and diagnosis

The crate name. `nova_os` was taken by the existing pure-logic terminal/shell
crate, which this one depends on, so the UI half could not simply be `nova_os`.
`nova_os_ui` says which half it is, and the two crate docs now point at each
other.

The move invalidated `nova_os`'s own docs in a way `cargo check` cannot see: 15
doc references to `nova_gameplay` as "the crate that draws this" were all stale
the moment the files moved. Found by grepping the CONSUMER's name out of the
dependency's docs, which is worth doing after any extraction.

`benchmark/keys/tier1.json` is measurement infrastructure that this seam
partially invalidates - 7 of 30 tier-1 questions cite paths or line counts that
just changed. Recorded in the lane's step list rather than re-keyed, because L2
owns a single re-keying pass and two passes would disagree.

### Evidence

- `cargo test -p nova_os_ui --lib` - 106 pass, all 105 moved tests plus
  `the_monitor_frame_runs_input_then_map_then_ship_then_paint`.
- SABOTAGE: dropping `.before(NovaOsShipSystems)` from `MonitorFrame` fails that
  test. The ordering assertion is load-bearing, not incidentally green.
- `cargo test -p nova_gameplay --lib` - 750 pass, 1 ignored.
- `cargo check --workspace --all-targets` - clean, zero warnings.
- `cargo fmt --all --check` - clean.
- `probe run --all` - 24/24 examples OK, including `screenshot_nova_os`.
  `screenshot_nova_os` re-run OK after the `MonitorFrame` extraction.
- Doc sweep: `AGENTS.md` code map (share table re-measured against 146,008
  lines), `README.md`, `web/src/wiki/dev/project-tour.md` and
  `web/src/wiki/dev/architecture.md` (crate table, mermaid graph, boundary
  policy) all name the new crate. `nova_os`'s 15 stale `nova_gameplay`
  references retargeted. `CONVENTIONS.md` rule 4's bad-import example no longer
  cites a path that does not exist.

### Reflection

The plan's instruction to survey the seam BEFORE cutting, and then not redo the
survey, is what kept this to one commit. The survey's list of crate-internal
dependencies was exactly right, so the cut was mechanical and every surprise was
in the docs rather than the code.

The F53 follow-through is the more useful lesson: the lane predicted a deletion
and the prediction was wrong, but only reading the code with the fix in place
could show that. Recording the DECISION and its reason costs a paragraph and
stops the next reader from re-opening it.

Three seams remain - HUD, FLIGHT, CORE - plus the rule-10 sweep over the plugins
those seams move.

## Close-out - L9.3, the HUD seam (2026-08-08)

### What and why

`crates/nova_hud` now owns the flight HUD: 19,087 lines that were the remainder
of `nova_gameplay/src/hud/` once the NOVA OS monitor left in L9.2. One module per
widget - instruments, reticles, readouts, markers, the comms panel, the keybind
dock and the `screen_indicator` projection they all share.

The survey found only TWO edges pointing INTO `hud/` from the rest of
`nova_gameplay`: `lib.rs`'s `hud::prelude::*` re-export and `plugin.rs:126`'s
render-gated `add_plugins`. Both deleted. Everything the HUD reads - `prelude`,
`camera`, `sections`, `input`, `flight`, `gravity`, `mesh`, `transform`,
`objectives`, `audio`, `asset_ref` - stays in `nova_gameplay`, so the arrow runs
`nova_hud -> nova_gameplay` and never the reverse. That asymmetry is the whole
justification for the cut: the HUD *reads* gameplay state and never drives it.

`nova_gameplay` drops from 54% of the workspace at the start of the lane to 30%
(44,159 of 146,146 lines). It is still the biggest crate, but no longer by a
multiple - which is the navigability claim the epic was after.

Consumers repointed: `nova_os_ui`, `nova_scenario`, `nova_assets`, `nova_menu`,
`nova_debug`, `nova_core`. `nova_core::AppBuilder` adds `NovaHudPlugin`
render-gated, BEFORE `NovaOsUiPlugin` - the monitor orders its own sets against
`NovaHudSystems`, so the HUD has to be in the app first.

The seam edge itself moved with the code. `nova_gameplay`'s two `configure_sets`
chains no longer name `NovaHudSystems`; `nova_hud::configure_hud_seam` declares
`NovaHudSystems.after(SpaceshipSectionSystems).before(NovaCameraSystems)` in both
schedules instead. The HUD sits after the sections that produce what it reads
(ammo, locks, integrity) and before the camera that consumes the screen-space
anchors it writes. Declaring that edge from the HUD side IS the seam: the crate
that owns the systems owns their placement.

### Alternatives considered

**Leaving the ordering in `nova_gameplay`.** `nova_gameplay` could have kept
naming `NovaHudSystems` in its chain by depending on `nova_hud` for the set type
alone. Rejected: that is a circular dependency bought for one identifier, and it
would have made the seam a fiction - the crates would still have been one unit
wearing two names.

**Keeping the two widened items `pub(crate)`.** Two items had to cross the seam
because tests in `nova_hud` use the real production systems rather than
re-implementing them: `input::player::hints::keyboard_label` (the key-glyph
coverage test labels the real bindings with it) and
`sections::turret_section::update_turret_aim_point` (the turret-lead pip
regression registers the real aim system with its production set constraints,
because the full `TurretSectionPlugin` drags render-material plugins into
headless tests). The alternative - duplicating both in the test rig - was
rejected under "reuse production helpers in test rigs": a copied label formatter
that drifts from the real one turns a coverage test into a test of itself. Both
widenings carry the reason in a comment at the definition.

**A `nova_hud` -> `nova_gameplay` interface trait.** Rejected for the same reason
L9.2 rejected it for the monitor: the dependency is one-directional and shallow,
and a trait would exist only to make the arrow look symmetric.

### Difficulties and diagnosis

The move was mechanically large (19k lines, 25 widget modules) but structurally
cheap, precisely because L9.2 went first. Cutting the monitor out established
that `hud/` was two things wearing one folder name; once the 15,200-line terminal
runtime was gone, what remained had a single coherent job and a single direction
of dependency. Ordering the seams outermost-first was the load-bearing decision,
and it paid here.

The one non-mechanical part was the schedule edge. `cargo check` cannot see a
missing `configure_sets` call - dropping the edge entirely still compiles and
still mostly works, because bevy's topological tie-break happens to order the
systems correctly on today's executor. That is the same accidental-correctness
trap L1's F04 hit. It is pinned by an explicit test rather than left to luck.

### Evidence

- `cargo test -p nova_hud --lib` - 207 pass, including
  `the_hud_set_runs_between_sections_and_camera_in_both_schedules`, which
  exercises `configure_hud_seam` (the production wiring, factored out for exactly
  that reason) rather than a hand-built copy of it.
- SABOTAGE: deleting the `FixedUpdate` half of `configure_hud_seam` fails that
  test (`lib.rs:1298`). The "in both schedules" half of its name is a real
  assertion, not decoration - the `Update` edge alone still passes everything
  else.
- `cargo test -p nova_gameplay --lib` - 544 pass, 1 ignored. 544 + 207 = 751
  against 750 before the cut: every moved test survived, plus the new seam test.
- `cargo test -p nova_os_ui --lib` - 106 pass. `-p nova_menu --lib` - 77 pass.
  `-p nova_scenario --lib` - 154 pass. The four repointed consumers with tests.
- All three CI check configurations green, each at `RUSTFLAGS=-D warnings`:
  `cargo check --workspace --all-targets`, `cargo check --workspace --exclude
  nova_probe_cli --target wasm32-unknown-unknown`, and `cargo fmt --all --check`.

### Reflection

The survey-before-cut discipline is what made this lane's second seam cheap. Both
NOVAOS and HUD were surveyed to the point of naming every edge that crossed the
boundary BEFORE a file moved, and both surveys were written into the step list so
the cut itself was bookkeeping. The steps that took real thought - back-edge 2's
inverted scheduling edge, F53's monitor frame, this seam's `configure_hud_seam` -
were all *ordering* questions, not moving questions. Worth carrying into FLIGHT
and CORE: the risk in a split is never the files, it is the schedule.

Two seams remain - FLIGHT and CORE - plus the rule-10 sweep, the visibility audit
and the module preludes over what those seams move.

## Close-out - L9.4a, the shake back-edge and the corrected FLIGHT survey (2026-08-08)

### What and why

Two things landed, and the second is the larger one.

**The code change.** `camera/shake.rs` is now `shake.rs` at the crate root, and
the two ordering edges it declared against `ChaseCameraSystems::Sync` are gone
rather than relocated. This clears the one real back-edge of the four L9.3
listed for the FLIGHT seam: `juice.rs` (combat feedback, damage-driven) fed
`CameraShake`, so a lower module reached into `camera`. The shake is a generic
drift-free trauma rig over any `Transform` - its own docstring says so - and its
only in-repo feeder is `juice`. It belongs next to `juice`, not inside the
ship's camera folder. The edges it named were a redundant, weaker duplicate of
what `CameraAuthorityPlugin` already declares: authority folds
`ChaseCameraSystems::Sync` into `CameraAuthoritySystems::Solve` and chains
`Restore -> Solve -> Additive`, while shake's own edges silently dropped to
nothing whenever the chase plugin was absent. Deleting them removes a
duplicated contract instead of moving it.

**The survey.** The step this pass was meant to execute - cut `flight/`,
`sections/`, `input/`, `camera/`, `physics/` - was attempted and reverted,
because the survey it inherited was wrong. L9.3 counted deep `crate::<module>`
paths and found 17 references and 4 back-edges. The actual number, taken by
removing the five directories and reading the compiler, is 129 errors across
`audio/` (103), `gravity.rs` (16), `integrity/` (5) and the crate root. The gap
is `crate::prelude::*`: a glob import makes a cross-seam reference invisible to
the grep that was used, and most of this crate's references arrive that way.
The corrected survey is written into the step, with the four real edge families
named and a fix chosen for each.

Also settled, because the step demanded it before the cut: there is no CORE
crate. `nova_core` is already the app-assembly crate, and the remainder is the
shared gameplay layer, not primitives. The split is three-way over a
`nova_gameplay` base.

### Alternatives considered

**Moving `juice.rs` up into the ship crate instead of moving `shake` down.**
Zero code changes - the shake would have stayed local to `camera/`. Rejected:
`juice` is damage-driven combat feedback and belongs beside `damage`,
`integrity` and `audio`. Putting hit-flash rings in the ship crate to avoid one
import is placing code by convenience, which is the habit this epic exists to
break.

**Keeping shake's `.after(ChaseCameraSystems::Sync)` edges and re-pointing them
across the seam.** Rejected: it would have made the shared layer name a type
from the crate above it, for an edge authority already owns.

**Pushing on and finishing the cut in this pass.** Rejected on scope. Once the
real survey was in hand, the remaining work is a `markers` module extracted
from 8 files, an inversion for `AISpaceshipMarker`, an `audio/` split, a
scheduling inversion in `gravity.rs`, a `test-support` feature, and then
repointing 12 consumer crates - materially more than "move five directories",
and more than one context can carry. A broken tree cannot be checkpointed, so
the cut was reverted and the survey kept. The next pass cuts once, against a
measurement that is right.

**Reverting everything, including the shake move.** Rejected: the shake fix is
independently correct, independently green, and is precisely the "clear the
back-edges BEFORE any file moves" discipline this lane already used in L9.1.

### Difficulties and diagnosis

The failure mode worth remembering is the survey method, not the code. Three of
the four "back-edges" L9.3 recorded (`transform/mod.rs -> camera`,
`mesh/mod.rs -> camera`, `damage.rs -> sections::prelude`) turned out to be
intra-doc links - `[`Foo`](crate::camera::Foo)` in a docstring - which grep
cannot distinguish from a real import. Meanwhile the 103 genuine references in
`audio/` were invisible because they come through `crate::prelude::*`. So the
inherited survey was wrong in both directions at once: it reported edges that
were not edges and missed the ones that were. Deleting the directories and
reading the compiler took one command and produced the truth.

The `audio/` finding is the same shape as L9.2's: a folder that is two things
wearing one name. `hud/` was 43% terminal runtime; `audio/` is 63% ship
soundtrack over a 37% generic SFX engine that `nova_menu` and `nova_os_ui` also
consume. Finding it before the cut is worth more than the cut would have been.

### Evidence

- `cargo test -p nova_gameplay --lib` - 545 pass, 1 ignored. 544 before, plus
  `the_shake_brackets_the_chase_base_writer`.
- SABOTAGE: dropping `ChaseCameraSystems::Sync` from authority's `Solve` fold
  fails that test and nothing else. Without it the guarantee moved out of
  `shake.rs` would have been unpinned - `cargo check` cannot see a missing
  `configure_sets`, and bevy's topological tie-break supplies the right order by
  accident on today's executor. Same trap as L1's F04 and L9.3's seam edge.
- `cargo test -p nova_hud --lib` - 207 pass.
- All three CI check configurations green at `RUSTFLAGS=-D warnings`:
  `cargo check --workspace --all-targets`, `cargo check --workspace --exclude
  nova_probe_cli --target wasm32-unknown-unknown`, `cargo fmt --all --check`.
- The corrected FLIGHT survey is reproducible: remove the five directories from
  `nova_gameplay/src`, drop them from `lib.rs`, and run
  `cargo check -p nova_gameplay --all-targets --message-format short`.

### Reflection

L9.3's reflection said the risk in a split is never the files, it is the
schedule. This pass adds a second one: the risk is also the *survey*, and a
survey done with grep over a crate that globs its own prelude is not a survey.
The two seams that went smoothly were surveyed when `hud/` still had explicit
edges to count. `nova_gameplay`'s interior does not, so the only trustworthy
instrument is the compiler. Cut the directories, read the errors, revert, then
plan - it costs one command and it is the difference between a cut that is
bookkeeping and a cut that is discovery halfway through.

Worth carrying: `probe run --all` has NOT run this pass. Nothing behavioural
changed - one ordering contract moved between two plugins that are always added
together, and it is pinned - but the per-seam probe run the lane requires
belongs with the FLIGHT cut itself, not with this checkpoint.

## Close-out - L9.6, rule 10's first slice (2026-08-08)

WHAT. Re-measured rule 10 against the post-seam tree, then acted on the seven
sets the measurement left standing: deleted `WASDCameraControllerSystems`,
pinned `TurretSectionAimSystems.before(SmoothLookRotationSystems::Sync)` in
`TurretSectionPlugin`, and left six alone with the reason written down.

WHY the number moved so far. The plan said 16 unordered sets and 68 setless
plugins; the truth is 7 and 18. Three separate causes, and only one of them is
"work already landed": L5 retired three sets and the seams ordered two more,
but the rest is a MEASUREMENT BUG in the original survey - it looked only at
`configure_sets` and never at `.before`/`.after` on `add_systems`, so three
sets that were correctly ordered all along were on the list. Any lane that
re-derives a rule-10 count has to count both spellings.

ALTERNATIVES. The mechanical reading of rule 10 - give all seven an edge - was
rejected. Six of them have no counterparty in their own schedule: the transform
rigs publish `*Output` in `PostUpdate` and every consumer runs in `Update`, so
the one-frame lag is the design. Manufacturing an intra-schedule edge would
record a constraint nobody has, which is the same "records nothing" failure the
rule was written against. Recorded as an owner call instead of silently
deciding the rule means something narrower than it says.

DIFFICULTY, and the one worth remembering. The first version of the ordering
test read the execution order back out of an `Order` resource - the idiom
`camera/authority.rs` already uses - and it PASSED WITHOUT THE FIX. Bevy's
topological tie-break happens to run the aim chain first today, so the test was
green on a codebase with the bug in it. Rewriting it around
`ambiguity_detection: LogLevel::Error` on a two-plugin app made it fail for the
right reason (exactly 1 conflicting pair, named in the panic) and pass on the
edge. An order-observing test only proves anything when the wrong order is
reachable in that app; for a missing edge it is a coin flip you are asserting
on.

EVIDENCE. `cargo check --workspace --all-targets` clean, `cargo fmt --check`
clean, `cargo test -p nova_ship --lib turret` 42/42 including the new
`the_aim_chain_is_ordered_against_the_rig_it_steers`. The aim-convergence test's
comment claiming production ordering "is not required" is now false and was
corrected: production carries the edge, the test app repeats it only because it
wires systems directly instead of through the plugin.

REFLECTION. Two of the three lane steps here were reported as counts, and both
counts were wrong in the same direction - the survey overstated the work by
counting declarations instead of behaviour. The seam lanes measured sites;
this one had to measure MEANING (is this set ordered? can it be?), and that
does not survive being cached as an integer in a plan.

Worth carrying: `probe run --all` has NOT run this pass either. Nothing
behavioural changed that a run would show - the one new edge is between two
plugins always added together - but the lane still owes its per-seam run.

## CHECKPOINT - L9.6 done, rule 10 needs an owner ruling (2026-08-08)

The scheduling half of the post-seam pass is finished as far as it can go
without the owner. Branch clean at `73e08d4f`, `cargo check --workspace
--all-targets` and `cargo fmt --check` green.

DONE: rule 10's first slice. One real cross-crate edge landed with an
ambiguity-detection test, one dead set deleted, and both rule-10 counts
re-measured from the post-seam tree (7 unordered sets, not 16; 16 setless
plugins, not 68).

BLOCKED, and it is the only thing blocking the rest of rule 10: 22 sites - six
unordered sets and sixteen setless plugins - would all be ceremonial sets under
rule 10's literal wording and zero sites under its stated rationale. The
proposed rewrite is in the "Rule 10, per seam" step. This is a CONVENTIONS.md
edit, so it is the owner's, and it decides whether that file's "Not yet true"
row for rule 10 closes at zero or at 22 new sets.

NEXT UNIT (independent of the ruling, wants a fresh context):
  1. The `pub` audit for the crates the seams did not settle - 633 crate-local
     `pub` items originally, `nova_ship`'s share already decided.
  2. Rules 3+4, the module preludes, in the same pass. `nova_ship` is done.
  3. `probe run --all` for the lane, then append to the tier1 re-keying list.
Both counts in 1 and 2 predate L5 and the seams. RE-MEASURE THEM FIRST - that
warning has now been right three times in a row, and twice the error was not
stale work but a survey that counted declarations instead of behaviour.

## Close-out - L10, the rebase over L9 (2026-08-08)

### What and why

L10 was finished at `8a41de9e` over a base (`e0b374e0`) that L9 then moved: the
`nova_gameplay` four-way split (`54ebcc2a`) landed while the lane ran. The lane
record's only open item was replaying it over that split. It is replayed; the
five lane commits keep their shapes and messages, at new SHAs.

The replay is `git rebase --onto master e0b374e0` plus one deliberate rewrite:
the Cargo.toml correction below belongs INSIDE L10.1, not on top of it, so
every commit on the branch builds. Rather than an interactive rebase the branch
was rebuilt by cherry-picking the seven rebased commits onto master in order and
amending the first. The rebuilt tree is byte-identical to the plain rebase's
apart from those two files - verified with `git diff` between the two tips
before the branch ref moved. The pre-rebase branch survives as
`backup/l10-prerebase`.

### Difficulties and diagnosis

**Four content conflicts, each a name L9 moved out from under L10.**

| Site | Resolution |
| --- | --- |
| `nova_assets/src/merge.rs` | test imports: `nova_gameplay::prelude::{BaseSectionConfig, HullSectionConfig, SectionKind}` is `nova_ship::prelude::*` now; kept L10's added `ScenarioConfig` import beside it |
| `nova_menu/src/settings_store.rs` | same shape - `NovaOsMonitorSettings` moved to `nova_os_ui`; kept L10's `storage::NativeStorage` import |
| `nova_hud/src/readout.rs` | git followed the rename itself; took L10.3's prelude |
| `nova_scenario/src/{world,actions/mission}.rs` | took L10.3's `From` impl and bare `HudReadouts` over master's re-pointed full paths |

**The `nova_hud/readout.rs` conflict is the one that mattered, and L10.3 WINS
it on the merits.** Master's side carries a doc comment and a code comment
explaining why `HudReadoutFormat` is deliberately kept OFF `readout::prelude`:
nova_scenario exported an authoring twin of the same name and nova_core globs
both preludes, so exporting from both is an ambiguous glob re-export. L10.3
removed the CAUSE - the scenario-side enum is `HudReadoutFormatConfig` now - so
the exclusion and both comments describe a collision that no longer exists.
Taking master's side would have re-introduced the full-path reach that L10.3
was written to delete. The prelude exports it and the two comments are gone.

**`nova_authoring` did not build after the replay, and the rebase could not
have known.** The crate is CREATED by L10.1 out of files that lived in
`nova_assets`, so its Cargo.toml is a NEW file - no merge base, no conflict, and
git had nothing to reconcile. L9 had meanwhile repointed those files' imports at
`nova_ship`, which auto-merged cleanly into the moved sources while the new
manifest still listed only `nova_gameplay`. 99 errors, all one missing
dependency; `nova_hud` was the same story at two sites. Both added to L10.1.
The lesson is narrow and worth keeping: a rebase verifies nothing about a crate
the rebased commits INVENT, because a new file has no other side to conflict
with.

**One test was red on master before this lane touched it.**
`the_bundle_ships_the_raid_and_bumps_the_version` asserts the-ledger bundle ships
`version: "1.15.0"`; `7eacb14c` republished it at `1.17.0`. It fails identically
on master (run there to confirm, not inferred) and is inside CI's `cargo test
--workspace --features debug`. Out of L10's lane, so it is its own commit
(`52f1d425`), not folded into a lane commit.

### Evidence

Every gate re-run on the replayed branch, not carried over from `8a41de9e`:

- `cargo check --workspace --all-targets` - clean.
- `cargo clippy --workspace --all-targets -- -D warnings` - clean.
- `cargo clippy --workspace --all-targets --features debug -- -D warnings` -
  clean (the exact CI line, `ci.yaml:75`).
- `cargo check -p nova_assets -p nova_menu --target wasm32-unknown-unknown` -
  clean.
- `cargo fmt --all -- --check` - clean.
- `content -- lint` (now `-p nova_authoring`) - 0 errors, 0 warnings, 0
  findings, 14 scenarios balance-audited, 1 acked.
- `content_ron_parity` 2/2. This is the load-bearing one: it proves L9's split
  changed no serialized name, so `assets/base/**` still matches the builders
  across both lanes with no regeneration.
- Lib tests: nova_scenario 154, nova_hud 207, nova_menu 77, nova_assets 60,
  nova_authoring 44 - all pass.
- Integration tests, all 20 targets across `nova_assets` and `nova_authoring` -
  all pass, including the three scenario e2e walks the lane's last step names
  (`example_scenario` 14, `gauntlet_course` 12, `lifeline_convoy` 8) and
  `ledger_ch5_raid` 13 after the fix above.

DoD proof 7's `-p nova_assets --bin content` still needs the correction the
lane's last step flagged: the binary is `-p nova_authoring --bin content` now.

### Reflection

The lane record's own warning - "do not merge it un-rebased: this branch's
TASK.md predates L9's ticks and would revert them" - turned out to cost nothing,
because L9 and L10 wrote to disjoint sections of the file and the three-way
merge took both without a conflict. The warning was still right to write: the
failure it describes is silent, and confirming it did not happen took one grep
of the tick counts.

What the abandoned first attempt got wrong was aborting on the first conflict.
All four conflicts are the same mechanical edit - a name L9 moved - and the
whole set took less time than the note explaining why it had been deferred. The
real work was not the conflicts at all; it was the 99 errors in the crate that
had no conflicts to resolve.
