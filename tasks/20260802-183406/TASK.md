# Retire the BCS harness surface and refresh the automation docs

- PRIORITY: 91
- TAGS: v0.10.0, tooling, autopilot, docs
- KIND: TASK
- ACTIVITY: PLANNING
- GATES: PLAN
- RESOLUTION: -
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

- [ ] Read every remaining `bevy_common_systems` import in `crates/` and confirm
      each is gameplay, inspector, or wireframe. Delete any re-export or wrapper
      the migration orphaned. Record the surviving list in the retro so the next
      reader does not re-derive it.
- [ ] Rewrite the automation prose in `web/src/wiki/dev/development.md`: the
      everyday-commands recipe (line ~33), the examples/smoke section (~161),
      the `HarnessMute` paragraph (~177), the capture recipes (~336-342), and
      the deadline knob (~480-484, `BCS_HARNESS_DEADLINE` ->
      `NOVA_AUTOPILOT_DEADLINE`), plus the perf recipe at ~529. Link the new
      `dev/automation-harness` page from `20260802-183355` instead of restating
      the env table.
- [ ] Update `web/src/wiki/dev/guide-add-section.md:188` (the section-smoke run
      recipe) and add the `nova_autopilot` row to the dependency map in
      `web/src/wiki/dev/keeping-docs-in-sync.md`.
- [ ] Update `AGENTS.md:74` (harness-first testing line) to name
      `NOVA_AUTOPILOT`, and confirm the `nova_autopilot` code-map row at line 32
      still describes the landed crate.
- [ ] Add the `CHANGELOG.md` `## [Unreleased]` entry under Internals & Tooling:
      the new crate, and the `BCS_* -> NOVA_*` env rename tagged **(breaking)**
      for anyone with a scripted run.

## Definition of Done

- Nothing in the repository outside historical task records names a BCS harness
  env or path.
  (cmd: `! rg -n "BCS_AUTOPILOT|BCS_SHOT|BCS_REEL|BCS_HARNESS_DEADLINE|debug::harness" --glob '!tasks/**' --glob '!web/src/news/**'`)
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
