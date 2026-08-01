# Epic: KISS pass over the crates - split oversized files, cut comment fluff

- STATUS: IN_PROGRESS
- PRIORITY: 45
- TAGS: v0.9.0, epic, refactor, chore
- KIND: EPIC
- FLOW STEP: WORKING
- PLAN STATUS: APPROVED

## Epic

Maintainer-facing KISS pass over every crate. Two axes, one sweep per area:

1. **Structure**: split oversized files so a single file fits one agent
   context. 139k lines of Rust across 14 crates; 20 files exceed 1500 lines,
   topped by `hud/nova_os.rs` (8274) and `nova_menu/src/lib.rs` (7705).
   Extract by cohesion (module per concern), not by line count.
2. **Comments**: 27136 comment lines, 18171 of them doc comments, leaving
   ~8965 non-doc comments; 1197 of those name a task HUID. Delete the fluff,
   compact the rest.

Behavior must not change. This is moves, renames, and deletions only.

## Comment rubric (applies to every child)

| Comment | Action |
| --- | --- |
| `///` / `//!` rustdoc on public API | Keep. Improve if wrong. |
| Narration of what the code plainly does | Delete. |
| "task 20260724-102309 asked for ..." | Delete the provenance clause; keep a `NOTE:` only if the constraint still binds. |
| Guards a value ("do not raise, causes X") | Keep; make it `NOTE:`. |
| Known defect or deferred work | Keep as `TODO:`/`FIXME:`/`BUG:` with the tatr ID if one exists; open a backlog task if none does. |
| Non-obvious workaround (Bevy ordering, wasm quirk) | Keep as `NOTE:`. |
| Commented-out code | Delete. |

Only `NOTE`, `TODO`, `FIXME`, `BUG` survive as non-doc markers. Bare prose
comments are fluff by default; the burden is on keeping them.

## Structure rubric

- Split a file when it holds more than one clear concern, not merely when it
  is long. A 900-line file with one concern stays.
- Extraction is `mod`-level: new sibling file or a folder module with `mod.rs`
  re-exporting. Public paths must not change; crate preludes keep their exports.
- No new abstractions, traits, or config knobs. Moving code only.
- Tests move with the code they cover.

## Done Means

- `cmd: nix develop --command cargo check --workspace --all-targets` clean.
- `cmd: nix develop --command cargo fmt --check` clean.
- `cmd: grep -rnE '//.*[0-9]{8}-[0-9]{6}' crates src --include=*.rs` returns
  only lines that are deliberate `NOTE:`/`TODO:`/`FIXME:`/`BUG:` references.
- Every child landed with no public-path breakage: crate preludes unchanged
  or additively updated.
- `manual:` maintainer confirms the largest remaining file per crate is
  defensible.

## Decisions

- Rubric lives here, not duplicated per child. Children read this index.
- Scope excludes `bevy-common-systems` (separate repo, separate flow).
- Excludes `tools/` and `web/` (not Rust crates under `crates/`).

## Fog

- Whether `nova_gameplay/src/hud/nova_os.rs` splits cleanly along app
  boundaries or needs a shared-state module first.
- Whether `nova_menu/src/lib.rs` (7705 lines, 2 files) has enough internal
  seams to split without touching behavior.

## Out of Scope

- Behavior changes, bug fixes, perf work. Any defect found becomes a backlog
  task, not a fix in the same commit.
- Renaming public items or reorganizing crate boundaries.
- Doc-comment authoring campaigns; only fix rustdoc that is wrong or moved.

## Child Tasks

Derive with `tatr frontier <epic-id>`.

## Manual Acceptance

Open `manual:` items inherited from landed children, for the owner to confirm
before the epic closes.

- [ ] 20260731-170322 - skim the NOVA OS HUD split diff and agree no behavior
      changed. Supporting evidence in that task's REVIEW.md: item multisets
      identical to the base, all three plugin `build` bodies byte-identical,
      and the same 102 tests passing by name.
- [ ] 20260731-170329 - skim the combat-readout comment diff and agree no
      behavior changed. Supporting evidence in that task's NOTES.md: item
      multiset identical at 485, `cargo doc` warnings unchanged at 14, and the
      entire non-comment diff is 3 test assertion strings, so every plugin
      `build` body is byte-identical.
- [ ] 20260731-170329 - the test-name-list proof was NOT obtained; running the
      suite locally exhausts this box's RAM (see 20260731-210651). CI runs it
      on the PR. Confirm that substitution is acceptable, or run it somewhere
      with headroom.
- [ ] 20260731-170359 - skim the nova_menu split diff and agree no behavior
      changed. Supporting evidence in that task's NOTES.md: `lib.rs` 7705 -> 219
      lines with the `Plugin::build` body byte-identical apart from comments,
      the 76 test names an identical multiset before and after (and all 76
      passing locally - `cargo test -p nova_menu --lib` fits in RAM), and the
      non-comment source text a pure move with 28 base-only lines, all import
      fragments or rustfmt re-wraps.
- [ ] 20260731-170340 - skim the gameplay input-layer split diff and agree no
      behavior changed. Supporting evidence in that task's NOTES.md: all three
      oversized files split into folder modules (5427/3666/2727 -> 21 files,
      largest 1076), every `Plugin::build` body multiset-identical to the base,
      the 179 `#[test]` fns conserved exactly with 180 passing locally
      (`cargo test -p nova_gameplay --lib input::` fits in RAM), and a
      non-comment line multiset whose every difference is a visibility keyword
      or a moved import. Parent paths were re-proved with a throwaway import
      probe after review round 1 found them broken.
- [ ] 20260731-170345 - skim the flight/camera/audio split diff and agree no
      behavior changed. Supporting evidence in that task's NOTES.md: the three
      oversized files split into folder modules (5812/2264/1752 -> 26 files,
      largest 939), the 119 `#[test]` fns conserved exactly with all 119
      passing locally (`cargo test -p nova_gameplay --lib` per module fits in
      RAM), and an executable-line multiset whose only differences are
      rustfmt re-wraps of signatures lengthened by a visibility keyword or a
      changed import path. Outside the three splits the entire non-comment
      diff is one wiki table cell.
- [ ] 20260731-170351 - skim the sections/integrity split diff and agree no
      behavior changed. Supporting evidence in that task's REVIEW.md:
      `turret_section.rs` 3668 -> 7 files and `torpedo_section/mod.rs` 1820 ->
      2, largest file now 1377; the 126 `#[test]` fns over the whole scope
      conserved exactly with 100 + 21 passing locally (`cargo test -p
      nova_gameplay --lib sections::` / `integrity::` fit in RAM); every
      removed comment line accounted for by a base-vs-new comment multiset
      diff, with all four value-guarding comments found again as `NOTE:`; and
      `cargo doc -p nova_gameplay --no-deps` clean under the touched scope, so
      no intra-doc link broke when items moved between modules.
- [ ] 20260731-170409 - skim the nova_assets split diff and agree no behavior
      changed. Supporting evidence in that task's REVIEW.md: `lib.rs` 2683 ->
      84, `portal.rs` 1773 -> 5 files and `scenario/shakedown.rs` 2843 -> 4,
      largest file now 1221; and a comment-stripped line multiset over the
      whole crate whose entire residue is `mod`/`use`/`pub use` lines, the
      listed visibility widenings and rustfmt re-wraps, so no statement,
      literal or signature changed. Tests were NOT run to completion locally
      (this box's RAM, see 20260731-210651): 24/24 integration binaries and
      95/96 lib tests passed, the one failure reproduced on master before any
      edit and filed as 20260801-122138.
- [ ] 20260731-170409 - two MINOR review findings were left unfixed rather than
      re-opening the branch: a dead `#[allow(missing_docs)]` at
      `portal/mod.rs:107` and `fn entry(...)` duplicated into
      `portal/catalog.rs` and `portal/install.rs`. Confirm they should fold
      into the epic's next crate pass, or ask for them now.
- [ ] 20260731-170427 - skim the nova_scenario split diff and agree no behavior
      changed. Supporting evidence in that task's REVIEW.md: `actions.rs` 2908,
      `loader.rs` 2849 and `lint.rs` 2124 split into folder modules, largest
      file now 1070; a comment-stripped line multiset over the three split
      files whose entire residue is `mod`/`use`/`pub use` lines and visibility
      keywords, so no statement, literal or signature changed; the 90 `#[test]`
      names identical as sorted lists; and every module prelude byte-identical
      to master. Tests DID run to completion here: `cargo test -p nova_scenario
      --lib` 145 passed, `--test skybox_swap_e2e` 1 passed.
- [ ] 20260731-170427 - `loader::OrbitHold` and `loader::LockEcho` were `pub`
      under `pub mod loader` and are now `pub(super)` inside the private
      `loader::trackers`. Neither is in any prelude and nothing outside the
      crate names them, so the narrowing looks right - confirm, or ask for them
      back on the public path.
- [ ] 20260731-170432 - skim the nova_probe split diff and agree no behavior
      changed. Supporting evidence in that task's NOTES.md and REVIEW.md:
      `bin/probe.rs` 2460 and `run_report.rs` 1590 split into folder modules,
      largest file now 913; a comment-stripped line multiset over the crate
      whose entire residue is `mod`/`use`/`pub use` lines, visibility keywords
      and rustfmt re-wrapping; the `#[test]` names an identical sorted list
      before and after; every `pub` item of the old `run_report.rs`
      re-exported by `run_report/mod.rs`; and the relocated bin target smoke-
      run on both `Cmd` arms. Tests ran here: `cargo test -p nova_probe --lib
      --bins` 71 + 26 passed.
- [ ] 20260731-170437 - skim the nova_ui / nova_os split + comment diff and
      agree no behavior changed. Supporting evidence in that task's REVIEW.md:
      `widget.rs` 2265 and `terminal.rs` 1579 split into folder modules,
      largest file now 863; a comment-stripped line multiset over both splits
      whose entire residue is imports, `pub(super)` keywords, `mod`/`pub use`
      lines and rustfmt re-wraps, with one deliberate deletion (the dead test
      helper `only_button`, recorded in NOTES.md); the `#[test]` names an
      identical sorted list before and after; all 56 old public paths compiled
      from an external crate by a throwaway import probe; and the whole
      `nova_editor` + `nova_debug` diff comment-only. Tests ran here: `cargo
      test -p nova_ui -p nova_os -p nova_editor -p nova_debug --lib` 65 passed.
