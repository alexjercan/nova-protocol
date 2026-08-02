# Build the v0.10.0 showcase scenario suite

- PRIORITY: 80
- TAGS: v0.10.0, content, scenario, examples, testing
- KIND: STORY
- ACTIVITY: -
- GATES: -
- RESOLUTION: -
- PARENT: 20260802-115955
- DEPENDS ON: 20260802-120025

## Story

Define a curated `showcase` run group that demonstrates Nova's strongest
player-visible systems while remaining useful as regression and performance
evidence. Promote existing production-path examples before adding new content.

## Steps

- [ ] Audit the example catalog against six beats: flight/radar/autopilot,
      campaign combat/outcomes, destruction, gravity/orbit, editor/menu, and
      NOVA OS/HUD.
- [ ] Add `[package.metadata.nova_probe] showcase = [...]` as the single ordered
      suite definition and teach probe spec resolution to load it.
- [ ] Select at least six stable examples. Add or extend a scenario/example only
      when no existing run shows a required beat; avoid duplicate thin wrappers.
- [ ] Convert selected scripts to checkpoint-driven automation. Give each run
      meaningful probe markers/invariants, declared capture checkpoints, and an
      explicit FPS policy or exemption reason.
- [ ] Add catalog/metadata tests that fail on missing, duplicated, unprobed, or
      uncapturable showcase members.
- [ ] Document what each run proves and which shipped images or benchmarks it
      produces.

## Definition of Done

- `showcase` resolves to an ordered, representative set with at least six real
  player/UI paths. (test: `showcase_group_resolves_declared_examples_in_order`)
- Every member has correctness evidence and an honest profiling policy.
  (test: `showcase_members_declare_probe_and_fps_coverage`)
- Every declared capture checkpoint belongs to a cataloged showcase member.
  (test: `showcase_capture_producers_are_cataloged`)
- The suite completes headlessly without manual input.
  (cmd: `nix develop --command cargo run -p nova_probe -- run showcase`)

## Notes

- Strong initial candidates: `playable`, `broadside`, `lifeline`,
  `screenshot_orbit`, `editor`, `menu_scenarios`, and `screenshot_nova_os`.
- Final membership follows measured stability and narrative coverage, not a
  target count beyond the minimum six.
