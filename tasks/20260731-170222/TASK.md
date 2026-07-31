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
