# Review: Mod loading + load-order overlay + a demo mod

- TASK: 20260714-134127
- BRANCH: modding/mods-overlay

## Round 1

- VERDICT: APPROVE

Reviewed the mods diff (nova_modding `ModList`/`ModListLoader`; nova_assets
`GameAssets.mod_list` + `register_bundles` mod-append + the pure `merge_bundles`
core; the demo mod under `assets/mods/demo/`). Both a self pass and an
independent out-of-context adversarial pass. Independently re-derived the
load-bearing claims rather than trusting the summary:

- Load gating: `count_loaded_handles` in bevy_asset_loader uses
  `is_loaded_with_dependencies` (RECURSIVE) for done-counting, and `ModList` /
  `BundleAsset` both visit their dependency handles - so the whole
  enable-list -> mod bundles -> content tree is loaded before Processing runs.
  `register_bundles` sees fully-loaded mods. Verified from source.
- Full production path exercised: `register_bundles_applies_enabled_mods` drives
  the real system with a populated `ModList` (not just `merge_bundles`), and a
  LIVE run with the demo mod ENABLED in `enabled.mods.ron` loaded clean (0 loader
  errors, 0 conflicts, no panic) in `12_menu_newgame`; reverted to empty after.
- Untyped-load stem rule pinned for the enable-list too
  (`mods_enable_list_untyped_load_resolves_the_loader`), mirroring the 163342 fix.

The out-of-context reviewer verified: cross-bundle-overlay vs intra-bundle-conflict
distinction (per-bundle seen-sets), section in-place Vec replace preserving palette
order, scenario map insert, `register_bundles` ordering + error-skip + closure
lifetimes, ModListLoader asset-root-relative path resolution, the demo mod RON
(exact base id `reinforced_hull_section`, genuinely-new `demo_mod_arena`,
deserializable), and the empty-enable-list base parity. No BLOCKER/MAJOR/MINOR
defects in the code.

- [x] R1.1 (MINOR) [informational - accepted as intentional]
  crates/nova_assets/src/lib.rs (merge_bundles) - the intra-bundle conflict guard
  also applies to the BASE bundle (its content files flatten into one bundle), so a
  hypothetical duplicate id within base now keeps the FIRST occurrence + logs an
  `error!`, where master silently last-wins-overlaid. The base has NO duplicate
  section or scenario ids today (confirmed), so there is no observable behavior
  change on the empty-enable-list path. Kept intentionally: a duplicate id inside a
  single bundle is an authoring bug, and surfacing it loudly + deterministically
  (first-kept) beats a silent overlay. Documented in the `merge_bundles` doc.

Tests: nova_assets 20 unit (incl. 2 new `merge_bundles`) + demo_scenario 5
integration (base typed, `merge_bundles` overlay, full `register_bundles` path,
2 untyped guards); nova_modding 3 (incl. `mod_list_manifest_ron_decodes`);
`cargo test --workspace --no-run` green; fmt clean; parity green;
`12_menu_newgame` + `09_editor` behavior-identical with the empty enable-list.

Correct. Ships.
