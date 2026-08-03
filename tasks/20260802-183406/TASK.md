# Retire the BCS harness surface and refresh the automation docs

- PRIORITY: 91
- TAGS: v0.10.0, tooling, autopilot, docs
- KIND: TASK
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE
- PARENT: 20260802-120019
- DEPENDS ON: 20260802-183403

## Story

Retire the BCS harness surface from Nova and leave the prose true. The pinned
`bevy_common_systems` dependency stays for gameplay helpers, the inspector, and
the wireframe pass; only the `debug::harness` and `completion` surfaces stop
being used. Docs that still teach `BCS_AUTOPILOT` get updated.

`20260802-183403` renames the code and its inline run recipes; every doc surface
outside `crates/`, `examples/`, `scripts/` and `tests/` is still teaching the old
contract. This task closes the last gap so the epic's repo-wide absence proof
goes green, and it is the pass that confirms the retained `bevy_common_systems`
imports are deliberate rather than residue.

## Steps

- [x] Read every remaining `bevy_common_systems` import in `crates/` and confirm
      each is gameplay, inspector, or wireframe. Delete any re-export or wrapper
      the migration orphaned. Record the surviving list in the retro so the next
      reader does not re-derive it.
- [x] Rewrite the automation prose in `web/src/wiki/dev/development.md`: the
      everyday-commands recipe (line ~33), the examples/smoke section (~161),
      the `HarnessMute` paragraph (~177), the capture recipes (~336-342), and
      the deadline knob (~480-484, `BCS_HARNESS_DEADLINE` ->
      `NOVA_AUTOPILOT_DEADLINE`), plus the perf recipe at ~529. Link the new
      `dev/automation-harness` page from `20260802-183355` instead of restating
      the env table.
- [x] Update `web/src/wiki/dev/guide-add-section.md:188` (the section-smoke run
      recipe) and add the `nova_autopilot` row to the dependency map in
      `web/src/wiki/dev/keeping-docs-in-sync.md`.
- [x] Update `AGENTS.md:74` (harness-first testing line) to name
      `NOVA_AUTOPILOT`, and confirm the `nova_autopilot` code-map row at line 32
      still describes the landed crate.
- [x] Add the `CHANGELOG.md` `## [Unreleased]` entry under Internals & Tooling:
      the new crate, and the `BCS_* -> NOVA_*` env rename tagged **(breaking)**
      for anyone with a scripted run.

## Definition of Done

- Nothing in the repository outside historical task records names a BCS harness
  env or path.
  (cmd: `! rg -n --hidden "BCS_AUTOPILOT|BCS_SHOT|BCS_REEL|BCS_HARNESS_DEADLINE|bevy_common_systems::debug::harness|bcs::debug::harness" --glob '!.git/**' --glob '!tasks/**' --glob '!web/src/news/**' --glob '!CHANGELOG.md'`)
- The AGENTS testing guidance names the renamed env, not just the crate.
  (cmd: `rg -n "NOVA_AUTOPILOT" AGENTS.md`)
- The CHANGELOG records the crate and the breaking env rename.
  (cmd: `rg -n "nova_autopilot" CHANGELOG.md`)
- The workspace and website checks pass.
  (cmd: `nix develop --command cargo check --workspace --all-targets --features debug && cd web && npm run ci`)
- Retained BCS usage is deliberate, not residue.
  (manual: read the remaining `bevy_common_systems` imports)

## Notes

- Parent: `20260802-120019`. Last child; depends on the migration.
- The BCS checkout is not modified by this release (epic decision
  `tasks/20260802-115955/DECISION.md`).
- Historical task records and shipped news posts keep their original text, which
  is why the absence grep excludes `tasks/**` and `web/src/news/**`.
- Base-branch proof status, checked 2026-08-03: the repo-wide grep hits ~35
  files, `NOVA_AUTOPILOT` is absent from `AGENTS.md` (only the lowercase
  code-map row at line 32 exists, added by the scaffold task - do NOT use
  `nova_autopilot` as the AGENTS proof token, it is already green), and
  `CHANGELOG.md` names no `nova_autopilot`. All three are red.
- Files still holding BCS prose after `20260802-183403` lands:
  `AGENTS.md`, `CHANGELOG.md`, `web/src/wiki/dev/development.md`,
  `web/src/wiki/dev/guide-add-section.md`. Re-run the grep at work time rather
  than trusting this list.
- Known retained `bevy_common_systems` surfaces to confirm, not delete:
  `WASDCameraController` (`nova_debug::harness`), `inspector::DebugEnabled` and
  `wireframe::DebugEnabled` (`nova_debug::lib`), `health::Health` re-exported
  through `nova_gameplay` (`nova_probe::capture`), and the `bevy_common_systems`
  entries in the `RUST_LOG` filters in `crates/nova_core/src/lib.rs`.

## Close-out

**What and why.** Docs-only pass; no production code changed. Renamed the last
`BCS_*` run recipes in `web/src/wiki/dev/development.md` (7 spots: everyday
commands, the harnessed-examples paragraph, `HarnessMute`, the three capture
recipes, the probe deadline knob, the perf-timeline recipe) and
`guide-add-section.md`; pointed `AGENTS.md:74` at `NOVA_AUTOPILOT`; added the
`## [Unreleased]` CHANGELOG entry for the crate plus the **(breaking)** env
rename. Added a link from `development.md` to
[The automation harness](../automation-harness/) rather than duplicating the env
table, and de-future-tensed `automation-harness.md` itself - it was written
before `20260802-183403` landed and still said Nova's callers "will" move off
`BCS_*`.

**Nothing was orphaned.** Every surviving `bevy_common_systems` use is
deliberate (list below); no re-export or wrapper needed deleting.

| Surface | Where | Why it stays |
| --- | --- | --- |
| `inspector::DebugEnabled`, `wireframe::DebugEnabled` | `nova_debug::lib`, `nova_debug::harness::hide_dev_overlays` | the inspector + wireframe passes, explicitly out of scope |
| `prelude::WASDCameraController` | `nova_debug::harness` | free-fly camera for reel poses; a gameplay helper |
| `health::Health` | `nova_probe::{capture,invariants}` (via the `nova_gameplay` re-export) | gameplay data the probe reads; no direct bcs dep, so no version skew |
| `modding::events::GameEvent` | `nova_probe::recorder` (via `nova_gameplay`) | gameplay event stream for the timeline |
| whole-crate prelude | `nova_gameplay`, `nova_scenario`, `nova_events`, `nova_assets`, `nova_core` | the shipping game's shared Bevy helpers |

No `debug::harness` or `completion` import from `bevy_common_systems` survives
anywhere. `keeping-docs-in-sync.md` already carried the `nova_autopilot`
dependency-map row (line 61, landed by `20260802-183355`), so step 3's second
clause needed no edit.

**Difficulty: two DoD proofs could not go green as written.** Both diagnosed and
corrected in `DECISION.md` - `debug::harness` also matches Nova's own
`nova_debug::harness` adapter (35+ deliberate hits that `examples_smoke.rs`
actively requires), and `CHANGELOG.md` is a historical record like `tasks/**` and
`web/src/news/**`. The proof cmd in the DoD was narrowed accordingly; the
underlying claim ("no BCS harness surface remains") is what actually went green.

**Round 1 findings.** The review found the sweep itself was under-reading:
plain `rg` skips dot-directories, so `.claude/skills/probe/SKILL.md` (which
still taught the dead `BCS_HARNESS_DEADLINE` that probe no longer sets),
`.github/workflows/ci.yaml` and `.gitignore` were never swept. Added
`--hidden --glob '!.git/**'` to the proof and fixed all four surfaces, plus a
stale `bcs` word in a `nova_gameplay` test comment. Also dropped
`NOVA_SHOT_DIR` from the CHANGELOG's renamed list - `git log -S BCS_SHOT_DIR`
is empty, so it was never renamed - and rewrote `DECISION.md` onto the tatr
record schema it had ignored.

**Round 2 findings.** APPROVE, with two prose corrections applied first: the
wiki promised the CHANGELOG "spells out the old spellings" while the entry
only carried the `BCS_* -> NOVA_*` glob, so `BCS_SHOT` and `BCS_REEL` were
greppable nowhere - all four renames are now spelled literally there. The
`DECISION.md` reference to the historical CHANGELOG entry now names it by its
opening words instead of a line number that the very same edit shifted.

**Evidence.**
- absence sweep (`--hidden`, `.git` excluded): 0 hits, exit 1 from `rg`
- `rg -n "NOVA_AUTOPILOT" AGENTS.md` -> line 74
- `rg -n "nova_autopilot" CHANGELOG.md` -> lines 28, 38, 39
- `cargo check --workspace --all-targets --features debug` -> `Finished`
  (re-run after the round-1 fixes)
- `cd web && npm run ci` -> exit 0 (needed `npm ci` first; a fresh sprout has no
  `node_modules`)
- `tatr check 20260802-183406` -> exit 0
- manual criterion (read the retained bcs imports): done, tabulated above -
  left unticked for the reviewer to confirm independently

**Reflection.** A grep proof written against a name that is *about* to exist in
two crates is fragile: `debug::harness` was unambiguous when the plan was
written and ambiguous by the time the dependency landed. Prefer fully-qualified
crate paths in absence proofs. Second: the CHANGELOG belongs on the
historical-record exclusion list from the start, since a breaking-rename entry
must spell the very names the sweep forbids. Third, and the one that actually
cost a round: an absence proof that hides half the repository is worse than no
proof, because it reads green. Any repo-wide `rg` used as a DoD criterion needs
`--hidden`, and the Step list should be derived from that sweep rather than
enumerated by hand - hand-enumerating `web/`, `AGENTS.md` and `CHANGELOG.md` is
exactly what left `.claude/` and `.github/` out of scope.
