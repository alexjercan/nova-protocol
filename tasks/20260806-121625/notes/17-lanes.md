# Lanes

The owner has decided: **one tatr task, all findings, split into lanes.** That
decision is settled and is not re-opened here. This file defines the lanes so
the work is landable in sequence.

Findings are referenced by their `16-findings-master.md` ids.

## The scheduling fact everything else hangs off

The benchmark baseline (`../benchmark/README.md`) measures **cold-read
navigability of the current tree**. So every lane falls on one of two sides of
a line, and the plan depends on this more than on any severity ranking:

| Marking | Meaning |
| --- | --- |
| **BLOCKS BASELINE** | The lane moves a file, renames a symbol, or edits a doc. It changes what the benchmark measures, so it must be **explicitly ordered** relative to the baseline run - never allowed to land in between |
| **NEUTRAL** | Behavior-only bug fixes. No file moved, no symbol renamed, no doc changed. Can land in any order, in parallel with the baseline work, without disturbing it |

Two consequences that are easy to get backwards:

1. **L0 lands BEFORE the baseline.** The AGENTS.md correction and `-D warnings`
   have to be in place first, or the baseline measures a tree CI does not check
   and docs that lie - and the resulting delta is unattributable.
2. **L5 (the deletion sweep) lands AFTER the baseline.** Deletion count is
   success criterion #2. Lines deleted before the baseline is taken are lines
   that never enter the ledger. This is the opposite of L0 and the two are
   easily confused because both are "cheap doc-ish work".

Behavior-only lanes (L1, L3, L4, L6, L11) are deliberately unconstrained by the
baseline. That is what makes them safe to run in parallel with the slowest part
of the epic - owner review of the question set.

## Lane summary

| Lane | Name | Baseline | Depends on | Size | CONVENTIONS |
| --- | --- | --- | --- | --- | --- |
| L0 | Fix the map, close the CI gaps | **BLOCKS** (before) | - | S | 8, 11, 12 |
| L1 | Unblind the probe gate | NEUTRAL | - | M | - |
| L2 | Build and baseline the benchmark | (is the gate) | L0 | M + owner time | - |
| L3 | Untrusted input, data loss, persistence | NEUTRAL | L1 | L | - |
| L4 | Reconciler discipline and terminal input | NEUTRAL | L1 | M | - |
| L5 | Delete the dead and lying surface | **BLOCKS** (after) | L2 | M | 1, 2, 5, 7, 9 + 19 preludes |
| L6 | nova_editor | NEUTRAL | L1 | S | - |
| L7 | `nova_ui::screen` extraction | **BLOCKS** (after) | L2 | M | 3, 4 (6 preludes) |
| L8 | nova_probe restructure | **BLOCKS** (after) | L1, L2 | L | 3, 4 (13 preludes) |
| L9 | nova_gameplay four-way split | **BLOCKS** (after) | L2, L4, L5, L8 | XL | **10**, 3, 4 (26 preludes) |
| L10 | nova_assets / nova_scenario cleanup | **BLOCKS** (after) | L2, L3 | L | 3, 4 (15 preludes) |
| L11 | Perf and small correctness | NEUTRAL | L1 | M | - |

Sizes are relative: S = hours, M = a day or two, L = several days, XL = the
bulk of the epic.

## Where CONVENTIONS.md lands

All 12 rules were ruled on 2026-08-07 (`../CONVENTIONS.md`), so every violation
count in that document is now scheduled work. **It is not a lane.** Each rule is
placed by the same question this file uses everywhere - does it move a file,
rename a symbol, or edit a doc? - and then by which lane is already reading the
affected code:

| Rule | Work | Sites | Lands in |
| --- | --- | --- | --- |
| 8 | `#[allow]` -> `#[expect(reason)]` | 38 | L0 (= F80) |
| 12 | note the nightly-only rustfmt keys | 0 | L0 |
| 11 | documents current practice, no edits | 0 | L0 |
| 2 | delete the prelude boilerplate doc | 69 | L5 |
| 1 | write the missing module docs | 28 | L5 |
| 5 | rewrite docs citing task artifacts | 26 | L5 |
| 7 | say why a hand-written impl is not a derive | 6 | L5 |
| 9 | rename 2 `SystemSet` types | 2 | L5 |
| 3 | add the missing module preludes | 80 | L5/L7/L8/L9/L10 |
| 4 | route deep imports through the prelude | 36 (+69 via rule 3) | with rule 3 |
| **10** | **68 new `SystemSet`s, 16 unordered sets** | **84** | **L9, per seam** |
| 6 | the test rules 1/2/5 are judged by | - | not a task |

Two of these are large enough to change a lane rather than ride along, and both
were ruled **against** the direction the extraction proposed:

1. **Rule 10 is not a chore.** As ruled it is 68 new sets plus 68 ordering
   decisions. Made before L9 they are all re-made after it, because the seam
   *is* the ordering question. It goes inside L9, per seam.
2. **Rules 3 and 4 are one edit, and it is the edit the structural lanes are
   already doing.** Deciding what goes in a module's prelude is deciding what
   crosses its boundary - the same audit L9 already absorbs. Do not schedule a
   workspace-wide prelude pass; let each lane pay for its own crates, and give
   L5 the 19 that belong to crates no structural lane opens.

---

## L0 - Fix the map, close the CI gaps

**Baseline: BLOCKS - must land before it.** Edits docs and CI config.

Contents:

- Correct `AGENTS.md`: the `nova_modding` row (wrong on 3 of 4 items - bundle
  merge, portal client and downloads are all in `nova_assets`), the
  `nova_events` line at `AGENTS.md:102` ("Cross-subsystem communication through
  `nova_events`, not direct coupling" reads as a general architecture mandate
  and has already misled one audit into flagging 46 healthy files), and the
  absent signal that `nova_gameplay` is half the workspace.
- Add `-D warnings` to the CI clippy step. **Free today: the tree produces 0
  warnings at that configuration**, measured.
- **F79** - `#[cfg(feature = "debug")]` the 11 dead example items, then add the
  default-features job. In that order; the job fails on those 11 otherwise.
- Gate `nova_probe/src/report.rs` behind `cfg(not(target_arch = "wasm32"))`
  like its siblings in `lib.rs:82-109`, then add the wasm `cargo check` job.
  Same ordering logic - the gate clears all 7 wasm warnings.
- **F80** - convert the `#[allow(clippy::type_complexity)]` attributes to
  `#[expect(..., reason = "...")]`. **Amended 2026-08-07:** the count is **38**,
  not 37, and all 38 are currently **dead** - `Cargo.toml:314-316` sets
  `type_complexity = "allow"` workspace-wide and all 17 manifests carry
  `[lints] workspace = true`, so they suppress a lint that cannot fire. The
  conversion still works and is still the right change: `#[expect]` overrides
  the workspace `allow` at the site, proven by the 4 existing
  `#[expect(clippy::type_complexity, ...)]` sites coexisting with it while
  clippy sits at 0 warnings. See CONVENTIONS.md rule 8.
- Write the repo-root `CONVENTIONS.md` and shrink `AGENTS.md`'s `## Code rules`
  to a pointer. **This is a rewrite, not a move** - see below.

**Depends on:** nothing.

**Verified by:** CI itself - the new jobs are the verification. Plus a
re-read of AGENTS.md against `02-workspace-map.md`.

### The repo-root CONVENTIONS.md

**Lands in L0, before the baseline.** Two reasons, and the second is the one
that matters: the benchmark measures cold-read navigability, and a house-style
document at the repo root is exactly the thing a cold reader opens - if it lands
after the baseline its effect is silently credited to the structural work
instead. And L1 through L11 all need one place to look up the house style while
they are writing code.

**It is a rewrite of `../CONVENTIONS.md`, not a copy.** That file is 648 lines
because it is the *evidence* record - violation counts, file lists, the
rulings, the rejected proposals, the lane placement. None of that belongs at the
repo root, and all of it stays in the task folder. `~/personal/scufris/CONVENTIONS.md`
is the shape and the length to match: 63 lines, one `##` per rule.

| Keep | Drop (stays in the task record) |
| --- | --- |
| The rule, as an imperative heading | Every violation count |
| One real snippet per rule | The counter-example file lists |
| One or two sentences of rationale | The `RULED 2026-08-07` annotations and rejected originals |
| The tool-trap table (`wildcard_imports`, `redundant_pub_crate`, `needless_pass_by_value`, pedantic/nursery) | `## What the rulings cost` and the lane table |
| The prelude-doc form settled under rule 3 | The per-crate prelude gap table |

Target: 120-150 lines.

**The one thing that must not be dropped: a `## Not yet true` section.**
Four rules will be normative on 2026-08-07 and violated by the tree until late
in the epic:

| Rule | Open sites | Closed by |
| --- | --- | --- |
| 3 - every module carries a prelude | 80 | L5, L7, L8, L9, L10 |
| 4 - import through the prelude | 36 | with rule 3 |
| 10 - every plugin declares and orders a `SystemSet` | 84 | L9, per seam |
| 1 - every module opens with a `//!` | 28 | L5 |

Without that section the document is `AGENTS.md`'s failure mode again - a
confident file the tree contradicts - and worse, every agent reading it during
L1-L11 will "helpfully" fix preludes inside unrelated diffs. With it, the
section is a checklist that empties as the lanes land, and **deleting it is the
epic's last commit.** Its emptiness is the proof the conventions are real.

Rules 6, 7, 11 and 12 are true today. Rule 8 becomes true inside this lane, with
F80. Rules 2, 5 and 9 become true in L5.

**Cluster note.** These five items look unrelated and are not. Each is
individually a one-line change, and together they turn the two things a large
refactor silently produces - **unused imports / dead code** (caught by
`-D warnings`) and **stale suppressions** (caught by
`unfulfilled_lint_expectations` once F80 lands) - from invisible into
CI-reported. Doing `-D warnings` without F80 leaves the second class
unaudited; doing F80 without `-D warnings` leaves it reported but not
blocking. The ordering constraints inside the lane (F79 before the
default-features job, the `report.rs` gate before the wasm job) are the only
sequencing that matters.

---

## L1 - Unblind the probe gate

**Baseline: NEUTRAL.** Behavior-only. No module renamed - that is L8.

**This lane goes first among the code work, and it is the one lane whose
absence invalidates every other lane's verification.** `cargo run -p nova_probe
-- run --all` blocks CI. Right now it can pass a run that was never probed
(F02), skip the log check on an entire platform (F03), or discard all evidence
because one artifact was truncated (F01). A green sweep after a large refactor
currently means less than it appears to.

Contents: **F01, F02, F03, F04, F05** (the blind-gate cluster), **F76, F77,
F78** (the example-harness defects), **F58** (the proc macro that accepts a
typo and silently changes behavior), **F63, F70, F71** (report-writer
robustness).

**Depends on:** nothing.

**Verified by:** this is the hard part, because the thing being fixed is the
thing that normally does the verifying. It needs its own harness:

- A fixture-driven test per gate defect - a truncated `trace.json`, a torn
  `timeline.jsonl`, a non-UTF-8 `run.log`, a `web-run.log`-only web run, a
  stale `run-<n>.log`, a pre-existing `checks.json` with an errored row. Each
  asserts the verdict the gate *should* produce.
- `probe run --all` before and after, byte-comparing the verdicts on a tree
  known to be healthy - the fixes must not change a healthy run's answer.
- F04 specifically wants an explicit `.before()`/`.after()` edge plus a test
  that fails if the edge is removed, because the current behavior is
  "accidentally correct on today's executor".

**Cluster note 1.** F01 and F03 share one root cause - the loader in front of
a good pipeline has no per-artifact error isolation. **One change** (degrade a
failed parse to `None` for that artifact and let its own check report the
failure) fixes F01 and, in spirit, F03. Do not fix them as two edits.

**Cluster note 2.** F70 (the missing CSV schema-version guard) is only
catastrophic *because* of F01: a schema mismatch currently destroys the whole
report instead of rejecting one row. Fix F01 first and re-assess whether F70 is
still worth its own change. This is the one place in the epic where fixing the
bigger bug may retire the smaller one.

**Cluster note 3.** F76-F78 belong here, not in a separate examples lane. The
examples **are** the harness. A lane that fixes the loader but leaves an
assertion that cannot fail (F76 - `wiki-settings.png` ships as a shot of the
bare main menu, exit 0) has not finished the job it claimed to do.

---

## L2 - Build and baseline the benchmark

**This is the gate that makes L5-L10 provable rather than churn.**

Contents: the `benchmark/` question set and answer key (`question-set-prompt.md`
workstream), owner ratification, then the baseline run by out-of-context agents
and by the owner.

**State 2026-08-07:** questions, keys and the Docker harness are built and
unit-verified; **no agent has run a paper**. Remaining: ratification, a review
of the harness code, the placement decision (the benchmark is a rerunnable tool
sitting in a dated task folder), then the run. Detail: `../plan/lane02.md`.

**Depends on:** L0 only (so the delta is attributable to structure rather than
to a stale table). **Not** blocked by L1 - the probe gate and the benchmark are
independent, and L1 should run in parallel with the slowest part of this lane,
which is owner review time.

**Verified by:** owner ratification of the question set, plus a green smoke
run. The transcript audit that used to verify this lane is obsolete: each
persona now runs in a Docker image holding only its own channel, so isolation
is a property of the image rather than something checked after the fact.

**Owner ruling 2026-08-07 - two runs, owner-driven.** The benchmark runs
exactly twice: this baseline, and once at the end of the epic. Not per seam.
**The owner starts and runs both** - no lane and no agent runs it. A lane that
needs a benchmark number stops and prompts the owner. Re-key once, immediately
before the final run: question text frozen, only `expect` and `citation`
change, and a question whose answer no longer exists is a finding rather than a
re-key. Detail: `../plan/lane02.md`.

---

## L3 - Untrusted input, data loss and persistence

**Baseline: NEUTRAL.** Behavior-only.

Contents: **F06, F07** (the mod data-loss pair), **F08, F09, F12, F13** (the
input caps), **F10** (`fire_rate` panic), **F14** (the silent-`None` dispatch
break), **F22** (settings lost on quit), **F56** (the ref-violation gate that
only covers scenarios), **F57** (hash-ordered generated content), **F59, F60,
F68, F69**, and **F61** once the owner has ruled on it.

**Depends on:** L1, for the reason every lane does - so a green probe run means
something.

**Verified by:** a hostile-RON corpus. This lane has the best test story in the
epic: every finding is "authored data reaches code that assumes it was
sensible", so one fixture set of malformed bundles, oversized catalogs, deeply
nested DSL expressions, duplicate ids and degenerate `fire_rate` values covers
most of it. Plus a kill-mid-write test for F07.

**Cluster note 1 (already identified).** F07's atomic-write helper touches
`persist.rs:91`, `mod_cache.rs:521`, `portal/catalog.rs:197` and
`bin/content.rs:103` - **the same four files as the `Storage`-trait extraction
in L10**. "Write atomically" belongs in the trait as a contract rather than
repeated as a convention at four call sites. If L10 is going to happen, F07
should be the change that introduces the trait's write method rather than a
free helper that L10 then has to absorb.

**Cluster note 2.** F06 and F07 are not two bugs, they are one failure mode.
F07 produces the corrupt file; F06 turns it into permanent loss on the next
install. Fixing either alone leaves a player who can still lose mods - F07
alone still loses them to a manually-corrupted file, F06 alone still loses the
*write* to a mid-serialize kill. Land them together with one test that kills
mid-write and then installs.

**Cluster note 3.** F08, F09, F12 and F13 are one slice of work with one owner
and one test strategy, and they are orthogonal to every structural move. F10
looks like a gameplay bug and belongs here rather than with the section code,
because its root cause is the same (an unvalidated authored `f32`) and its fix
is one function away in the same file (`setup.rs:192` already guards it).

**Cluster note 4 (new).** F57's fix (`BTreeMap` or a sorted-key
`serialize_map`) regenerates `assets/base/**/*.content.ron`. Per the
`base-content-ron-is-generated` rule those files are never hand-edited - the
change is to the builders plus a `content -- gen` run. Schedule F57 as its own
commit so the generated-file churn does not hide a real diff in review.

---

## L4 - Reconciler discipline and terminal input

**Baseline: NEUTRAL.** Behavior-only. **This lane must precede L9's NOVAOS
seam move**, or the same defects get fixed after 14.3k lines have shifted and
every citation in `10-review-hud-nova-os.md` has to be re-derived.

Contents: **F15, F34** (missing Control guards), **F18** (the `f32::MAX`
sentinel), **F19, F20, F75** (stale `Local<T>`), **F39, F40, F41, F42** (the
unguarded per-frame writes), **F16** (the aborted explosion), **F21** (audio
loops through scenario load), **F23** (torpedo anchor), **F33, F73, F74**
(terminal).

**Depends on:** L1.

**Verified by:** `keybind_dock.rs` is the in-repo reference implementation for
this whole lane - `set_if_neq` throughout, guarded `Node` writes,
`Added<DockChip>` overrides, real `.after()` edges. Behavioral tests can be
written against its shape. For the per-frame-write findings specifically, the
assertion is a change-detection one: run two frames with no input and assert
the component is not marked changed on the second.

**Cluster note 1 (new).** F18, F19, F39, F40 and F42 are five defects living in
three files - `hud/nova_os/shell.rs`, `input.rs` and `crt.rs`. Fixed
separately, that is three files read five times. Fixed as one pass it is three
files read once, against one reference implementation, with one test rig. This
is the largest single-lane saving in the epic after F38.

**Cluster note 2 (new).** F19, F20 and F75 are the same pattern at three sites,
and fixing them together is what makes the CONVENTIONS.md rule defensible - the
rule can then cite both the violation count **and** the fix, in-repo. Fixing
one and writing the rule from it is weaker evidence.

**Cluster note 3 (new).** F15 and F34 are both "a keyboard handler that
bypasses the Control guard its sibling applies". Same reading, same fix shape,
adjacent code.

---

## L5 - Delete the dead and lying surface

**Baseline: BLOCKS - must land AFTER it.** Deletion count is success criterion
#2; lines deleted before the baseline never enter the ledger.

Contents: **F45** (the whole `Tween` subsystem, 421 lines + 11 tests + a plugin
registration, zero consumers), **F46** (`StatusBarStore`), **F47**
(`render: bool`), **F48** (`objectives.rs rebuild_lines`), **F49**
(`bay.rs:112`'s inert `Without<>`), **F50** (`panel_head`'s discarded `skin`),
**F51** (the never-rendered status-bar entity), **F52** (the `nova_debug`
feature leak and `nova_info`'s dead flag), **F54** (three `toggle_debug_mode`
fns), **F55** (fold `widget::register` into a real plugin), plus the stale
narrative.

**Added 2026-08-07 - the CONVENTIONS.md prose sweep.** Five accepted rules are
pure prose-and-rename work with no behavioral risk, and they share this lane's
constraint exactly: all block the baseline, all land after it. 131 sites:

| Rule | Work | Sites |
| --- | --- | --- |
| 2 | delete the prelude boilerplate doc line | **69** |
| 1 | write the 28 missing module docs (`//!` + a "touch this module when ..." line) | 28 |
| 5 | rewrite the docs that cite a task artifact (`DECISION.md`, bare task ids) | 26 |
| 7 | one comment per bare hand-written trait impl, saying why it is not a derive | 6 |
| 9 | rename `HudSituationSensing` and `CameraAuthority` to `*Systems` | 2 |

Rule 2's count **corrects the 91 figure** this lane previously carried: of 106
prelude docs, **69** are the exact boilerplate sentence and **37** say something
specific (`nova_ui/src/lib.rs:24-31` is the model). Delete 69, keep 37.

**Count rule 1 separately from the deletions.** It *adds* 28 module docs.
Deletion count is success criterion #2 and rule 1 nets against it; report the
two numbers rather than one.

**Also here: the orphaned share of rules 3 and 4.** The 80 missing module
preludes are paid for by whichever structural lane already opens the crate -
`nova_gameplay` 26 in L9, `nova_assets` 13 + `nova_scenario` 2 in L10,
`nova_probe` 13 in L8, `nova_ui` 6 in L7. That leaves **19 in four crates no
structural lane touches**: `nova_autopilot` 7, `nova_debug` 6, `nova_os` 4,
`nova_mod_format` 2. They are additive, block the baseline, and carry no
behavioral risk, so they belong here. Each is one prelude module plus a one-line
doc naming its contents - never the boilerplate sentence rule 2 deletes.

**Depends on:** L2.

**Verified by:** the compiler for the deletions, `probe run --all` for F47
(making the headless mode real changes what a run builds), and a
double-registration check in the menu and editor apps for F55.

**Cluster note 1 (new).** F45 makes F55 cheaper. Deleting `TweenPlugin`
outright turns "fold three entry points into one `NovaUiPlugin`" into a
two-plugin merge, and removes the need to decide where tween scheduling sits in
the new plugin's set ordering. Do F45 first, inside the same lane.

**Cluster note 2 (new).** F41, F46 and F51 are three defects in one 365-line
untested vendored file (`nova_ui/src/status_bar.rs`). F41 is behavior-only and
sits in L4; F46 and F51 are deletions and sit here. **That split is a cost** -
someone reads the same untested file twice. If the lanes are run by the same
person, do the whole file at once and let the F41 commit land in L4's window
while F46/F51 wait for the baseline. Flag this explicitly in the plan; it is
the one place where the baseline line cuts through a file rather than between
files.

**Cluster note 3 (new).** F52 and F79 (in L0) are the same investigation - what
`--features debug` actually builds and what is orphaned without it. F79 has to
land in L0 to unblock the default-features CI job; F52 is a deletion and waits
for the baseline. Whoever does F79 should write down what they learned for F52,
or the investigation happens twice.

---

## L6 - nova_editor

**Baseline: NEUTRAL.** Behavior-only.

Contents: **F11, F29, F30, F31, F32**.

**Depends on:** L1.

**Verified by:** the crate has 13 tests and no in-workspace dependents, so
nothing else pins it and nothing else breaks if it changes. That cuts both
ways: the lane is low-risk to land and has almost no existing safety net. F11
in particular wants a test that a missing catalog id logs and skips rather than
panicking.

**Cluster note (new).** Five defects in 2,378 LOC - the worst defect density in
the workspace, and the crate was **not on the epic's list at all**. It is small
enough to read whole in one sitting, which no other lane's scope is. Whoever
fixes one of these is already holding the entire crate in their head; splitting
this across lanes or people wastes that. Keep it as one lane and one reader.

---

## L7 - `nova_ui::screen` extraction

**Baseline: BLOCKS - after.** Creates a module and moves code.

Contents: the list+details+scroll composition module proposed in
`06-ui-layer.md`, collapsing the `mods` / `scenarios` / `portal` triplication
inside `nova_menu`, plus **F17** and **F28**.

**Depends on:** L2.

**Added 2026-08-07 - this lane holds two player-visible bugs behind owner
time.** F17 and F28 are behavior defects a player hits today, and they wait for
the baseline only because the *fix* is an extraction. If L2's owner review
stretches, split them: land the unit conversion and the shrink clamp in place
during L4's window (NEUTRAL, ~10 lines, no file moved), and let L7 delete the
duplicated bodies afterwards. The cost is writing the conversion twice; the
benefit is not shipping a known scroll bug for the length of a review cycle.
This is the owner's call, not the plan's.

**Verified by:** the existing `nova_menu` test suite (2,800 LOC, 35% of the
crate) - this is the best-covered structural change in the epic. F17 needs a
scale-factor test specifically, since the defect is invisible at 1.0.

**Added 2026-08-07 - CONVENTIONS.md rules 3 and 4.** `nova_ui` is the crate that
made rule 3 a question: its root prelude names 40-odd items by hand
(`lib.rs:32-51`) while `font.rs` and 5 siblings have no prelude, so every new
public item is a two-file edit. It also has **zero** `use crate::prelude`
anywhere - its own prelude is exercised only by downstream crates. Under the
ruling it converts to the glob form: **6** new module preludes, and the root
prelude collapses to a list of `<module>::prelude::*` lines. Creating a
`screen` module in this lane means creating its prelude too, so do the whole
crate in one pass.

**Cluster note (already identified, now sharper).** This extraction stopped
being a duplication argument. It fixes **four** defects in one edit: the unit
bug in `max_nova_os_scroll_y`, the same unit bug in `max_menu_scroll_y`, the
shrink-clamp gap in `scroll_menu_lists`, and the unclamped
`scroll_editor_panel` variant which gets a correct implementation to adopt.
Fixing the two unit bugs separately means writing the physical-to-logical
conversion twice, which is how they diverged in the first place.

---

## L8 - nova_probe restructure

**Baseline: BLOCKS - after.** Splits a crate and renames every module.

Contents: split into `nova_probe` (in-game collection library) and
`nova_probe_cli` (host harness) at the real process boundary; rename to
`capabilities/` `evaluation/` `report/`; a collection-side bundle plugin
preserving per-example config; evict `fixtures.rs`, `profile_sandbox.rs` and
`bin/perf_web.rs`; add a prelude (184 deep-path imports, worst in the
workspace).

**Added 2026-08-07 - the split renames the gate's own invocation.** `cargo run
-p nova_probe -- run --all` becomes `-p nova_probe_cli`. Every caller moves in
the same commit as the split, or CI breaks on a crate that no longer has a
binary: `.github/workflows/ci.yaml`, `AGENTS.md`, `justfile`/scripts if any, and
every doc line that quotes the command. Grep for `-p nova_probe` before
declaring the lane done - this is the one rename with a consumer outside the
Rust source.

**Amended 2026-08-07 - CONVENTIONS.md rules 3 and 4.** "Add a prelude" is now
"add **13** preludes": `nova_probe` has 12 public modules and **zero** preludes,
the only crate in the workspace with public modules and no prelude at all. That
is the largest single-crate share of rule 3's missing 80, and it is why this
crate's deep-import count is the workspace record. The restructure already
renames every module (`capabilities/` `evaluation/` `report/`), so the prelude
for each new module is written once, at the point the module is created, rather
than added to the old names and then moved.

**Depends on:** **L1 - hard.** Restructuring a gate that is currently blind
means the restructure's own verification is unreliable. Also L2.

**Verified by:** `probe run --all` before and after, plus the fixture-driven
gate tests L1 introduced - which is precisely why L1 must precede it.

**Note.** `04-nova-probe.md`'s "rename, do not rebuild" verdict on
`run_report/` stands for the **structure**. It was amended on **confidence**:
the loader in front of that pipeline is what L1 fixes. Do not let the rename
absorb the fixes, and do not let the fixes drift into a rename.

---

## L9 - nova_gameplay four-way split

**Baseline: BLOCKS - after.** The bulk of the epic and its highest risk.

Contents: CORE <- FLIGHT <- HUD <- NOVAOS, one seam at a time, in the order
NOVAOS -> HUD -> FLIGHT -> CORE. Resolve the three back-edge sites first
(`camera/framing.rs:200` moves to `math`;
`sections/controller_section.rs:301`'s scheduling edge inverts;
`plugin.rs:107,111,115` lift into the assembly crate). Plus **F53** and
**F81**.

**Depends on:** L2, **L4** (fix the NOVAOS defects before the lines move),
**L5** (added 2026-08-07 - the rule-10 set count is 16 only after `TweenSystems`
and `StatusBarPluginSystems` die with F45/F46), and L8 (the gate must be
trustworthy first).

**Verified by:** `probe run --all` **per seam**, not once at the end.

**The benchmark does NOT rerun per seam - owner ruling 2026-08-07.** Two runs
in the whole epic: the L2 baseline and one final run, both started and run by
the owner. Re-key once, immediately before the final run. This lane invalidates
the key, so note as you go which questions your moves break.

**Cluster note (new).** F53 and F81 both live in `nova_os_map` and
`nova_os_ship` and are both questions the NOVAOS split has to answer anyway.
F53 - `NovaOsShipSystems` / `NovaOsMapSystems` declared and never passed to
`configure_sets` - is exactly the "what crosses this seam and in what order"
decision the split forces. F81 - `map_input` and `ship_input` taking an
identical 6-param cluster - removes two `too_many_arguments` suppressions and a
duplication with one `#[derive(SystemParam)]` struct, and that struct has to be
placed on one side of the seam regardless. Doing either before the split means
doing it twice.

**Also folded in for free:** the 633 crate-local `pub` items
(`02-workspace-map.md`). Splitting four ways forces each seam to decide what
crosses its boundary, so the visibility audit is work the split does anyway
rather than a separate pass.

**Added 2026-08-07 - CONVENTIONS.md rule 10, and it is not free.** The owner
ruled that **every** subsystem plugin declares a `SystemSet` and orders it:
*"let's use all system sets just to have a predictable order."* Measured, that
is 98 plugins against 30 sets, 21 `configure_sets` calls, and only **14 sets
ever passed to one** - so 68 plugins need a new set and 16 existing sets need an
ordering they have never had.

**This lands here, per seam, and nowhere earlier.** Sixty-eight ordering
decisions made across `nova_gameplay` as it stands today are sixty-eight
decisions re-made the moment the crate is cut four ways, because "what runs
before what, across this boundary" is precisely the question the seam forces.
Doing it before the split means doing it twice. Done *as* the split, the
`configure_sets` block for each new crate is the artifact that proves the seam
is real and the order is intentional.

The 16 declared-but-unordered sets are the natural first slice and the cheapest
evidence the rule is workable:

`DirectionalSphereOrbitSystems`, `HudSituationSensing`, `IntegritySystems`,
`NovaOsMapSystems`, `NovaOsShipSystems`, `ObjectivesPluginSystems`,
`PointRotationSystems`, `SmoothLookRotationSystems`, `SpaceshipTargetingSystems`,
`SphereOrbitSystems`, `SphereRandomOrbitSystems`, `StatusBarPluginSystems`,
`TempEntitySystems`, `TurretSectionAimSystems`, `TweenSystems`,
`WASDCameraControllerSystems`.

This **subsumes F53**, which named `NovaOsShipSystems` and `NovaOsMapSystems`.
The measurement shows F53 is not two sites; it is 16.

Two of these retire for free elsewhere: `TweenSystems` dies with the `Tween`
subsystem (F45, L5) and `StatusBarPluginSystems` is in the same file as F46/F51.
Do the L5 deletions before counting the remaining set work.

**Added 2026-08-07 - CONVENTIONS.md rules 3 and 4, `nova_gameplay`'s share.**
Every module that exports items carries a prelude, and consumers import through
it. `nova_gameplay` is missing **26** module preludes of the workspace's 80.
This is the same edit as the visibility audit above - deciding what goes in a
module's prelude *is* deciding what crosses its boundary - so it costs nothing
extra if done in the same pass, and costs a second full read of the crate if
not. `math` alone accounts for 5 of the deep-import violations and is already
moving in this lane (`camera/framing.rs:200`).

---

## L10 - nova_assets / nova_scenario cleanup

**Baseline: BLOCKS - after.** Extracts a crate and moves content.

Contents: extract the authoring toolchain (`lint_walk`, `balance`,
`content_report`, `scenario_generation`, `bin/content`, plus
`nova_scenario/src/lint`) into one crate; move base content out; the `Storage`
trait mirroring the existing `PortalTransport` pattern; route scenario -> HUD
through `nova_events` (`world.rs:138-144`, `actions/mission.rs:512,534,554`);
lift `render_scale` out of `nova_scenario`.

**Depends on:** L2, and **L3** for the `Storage` trait specifically - see
below.

**Verified by:** the `content -- lint` and `content_ron_parity` gates, plus the
`shakedown` scenario walk. Independent of L9, so it can run in parallel.

**Cluster note.** The `Storage` trait and F07's atomic-write helper touch the
same four files. Sequence F07 (in L3) as the change that **introduces** the
trait's write contract, then L10 extends the trait to cover the read side and
the wasm backends. The alternative - a free helper in L3 that L10 then
absorbs - means writing the same four call sites twice.

**Added 2026-08-07 - CONVENTIONS.md rules 3 and 4.** `nova_assets` is missing
**13** module preludes (13 public modules, 1 prelude) and `nova_scenario` is
missing 2. Same argument as L8 and L9: this lane already extracts a crate and
moves modules, so each moved module gets its prelude written at its new home
once, instead of being retrofitted at the old one and then relocated.

Note also that the wasm-rot argument for this lane was **withdrawn** (W3 in
`16-findings-master.md`): all 14 crates type-check clean on wasm32. The trait
is justified by testability and gate removal, which is a narrower but still
sufficient case. Do not re-argue it from bit-rot.

---

## L11 - Perf and small correctness

**Baseline: NEUTRAL.** Behavior-only.

Contents: **F37** (the per-bullet Mesh + StandardMaterial allocation), **F38**
(the engine-spool duplicate and its complexity bug), **F24** (framerate-
dependent AI DPS), **F25, F26, F27** (widget and skin divergence), **F35, F36**
(scenario lifecycle and lint), **F43, F44** (per-frame allocations), **F62,
F64, F65, F66, F67, F72**, **F82, F85, F86**.

**Depends on:** L1 - F37 in particular sits directly under the probe's FPS
baseline check, so its before/after evidence is only meaningful once the gate
is trustworthy.

**Verified by:** `probe run` with `--baseline` for F37 and F38 - both should
show a measurable FPS improvement, and that measurement is the point. The rest
are unit-testable.

**Cluster note 1 (already identified).** F38 is the best cost/benefit ratio in
the review: **one extraction kills a 16-line byte-identical duplicate and both
copies of the workspace's only real per-tick complexity bug** (O(ships x
thrusters x thrusters_on_this_ship), every FixedUpdate tick). Do not fix the
complexity bug in place in two files.

**Cluster note 2 (new).** F26 and F27 are both in `nova_menu/src/settings.rs`,
as is F22 (which sits in L3 because it is data loss). Three defects, one file,
and `settings.rs` plus `pause.rs` are the **only** menu files that never import
`UiText`. Whoever opens `settings.rs` should carry the F22 fix with them
regardless of which lane the commit lands in.

**Cluster note 3 (new).** F25, F50 (in L5) and the three paint nits at
`button.rs:244`, `slider.rs:26` and `slider.rs:78` are one investigation:
**where the phosphor and hardware skins diverge**. Five sites, one reading of
the two paint backends, one skin-comparison screenshot test. The findings are
split across two lanes by the baseline line (F50 is a deletion), but the
reading should happen once.

**Cluster note 4 (new).** F65 and F66 are both in
`torpedo_section/projectile.rs`, alongside F23 in L4. Three findings, one
90-line file. Same argument as cluster note 2 - read once.

## What is deliberately NOT a lane

- **Tests.** Owner's explicit instruction: "tests as you said should be a
  separate task, I will see to it **do not create it**." The per-lane
  verification described above is the evidence each lane needs to land, not a
  coverage push.
- **A `clippy::pedantic` CI job.** 66% of its output here is
  `needless_pass_by_value` and `redundant_pub_crate`, both of which are wrong
  for a Bevy codebase. See `09-clippy-and-lints.md`.
- **A `cast_*` cleanup pass.** Sampled from two directions, measured clean
  (W17).
- **F84** (`proc-macro-error2 v2.0.1` future-incompatibility). A transitive
  dependency, not this code, and `-D warnings` does not cover it. It breaks on
  a rustc bump, so it needs **its own tracking task** - but not a lane here.
  Added 2026-08-07; it was the one finding with no lane and no disposition.
- **Splitting this into multiple tatr tasks.** Owner decision, settled.
