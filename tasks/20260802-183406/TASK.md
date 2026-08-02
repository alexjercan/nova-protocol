# Retire the BCS harness surface and refresh the automation docs

- PRIORITY: 91
- TAGS: v0.10.0, tooling, autopilot, docs
- KIND: TASK
- ACTIVITY: -
- GATES: -
- RESOLUTION: -
- PARENT: 20260802-120019
- DEPENDS ON: 20260802-183403

## Story

Retire the BCS harness surface from Nova and leave the prose true. The pinned
`bevy_common_systems` dependency stays for gameplay helpers, the inspector, and
the wireframe pass; only the `debug::harness` and `completion` surfaces stop
being used. Docs that still teach `BCS_AUTOPILOT` get updated.

## Steps

- [ ] Delete the now-dead re-exports and wrappers left behind by the migration
      and confirm the retained `bevy_common_systems` use is gameplay,
      inspector, and wireframe only.
- [ ] Sweep the prose: `AGENTS.md`, `web/src/wiki/dev/*`, the example catalog
      comments, and `CHANGELOG.md`; add the crate row to the AGENTS code map.

## Definition of Done

- Nothing in the repository outside historical task records names a BCS harness
  env or path.
  (cmd: `! rg -n "BCS_AUTOPILOT|BCS_SHOT|BCS_REEL|BCS_HARNESS_DEADLINE|debug::harness" --glob '!tasks/**' --glob '!web/src/news/**'`)
- `nova_autopilot` appears in the AGENTS code map and the testing section names
  the renamed env.
  (cmd: `rg -n "nova_autopilot|NOVA_AUTOPILOT" AGENTS.md`)
- The workspace and website checks pass.
  (cmd: `nix develop --command cargo check --workspace --all-targets --features debug && cd web && npm run ci`)
- Retained BCS usage is deliberate, not residue.
  (manual: read the remaining `bevy_common_systems` imports)

## Notes

- Parent: `20260802-120019`. Last child; depends on the migration.
- The BCS checkout is not modified by this release (epic decision
  `tasks/20260802-115955/DECISION.md`).
- Historical task records and shipped news posts keep their original text.
