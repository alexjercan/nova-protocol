# Review: Bundle meta block - mod metadata moves into bundle.ron, catalogs become thin pointers

- TASK: 20260715-142849
- BRANCH: feature/bundle-meta

## Round 1

- VERDICT: APPROVE (two NITs, both addressed before landing)

Out-of-context review pass (fresh-context agent over the full 11-file diff vs
master; close-out claims re-verified independently). Verified: the loader
really carries meta onto `BundleAsset`; the integration test is genuinely
falsifying (the thin catalog contains none of the asserted strings, so a
dropped meta falls back to id and fails the exact-string asserts);
`build_mod_catalog` degrades decl-only on a missing bundle, no panic path, and
the `OnEnter(Processing)` schedule makes that fallback defensive rather than
racy; catalog order preserved; `seed_enabled_mods`/`register_bundles` are pure
renames with identical keyed fields; the authored bundle metas moved the old
catalog strings VERBATIM, so the menu renders identically; the meta-less
back-compat pin is real; no missed `ModEntry`/`.name`/`.description` consumers
(workspace grep + `cargo check --workspace --all-targets` clean); docs and
close-out match behavior. Test re-runs: nova_modding 3, demo_scenario 10,
nova_menu 13, all passing; fmt clean.

- [x] R1.1 (NIT) docs/modding-ron-format.md (meta bullet) - `icon` is an
  `Option` decoded with strict RON, so `icon: "icon.png"` fails to parse; the
  doc did not warn that it must be `icon: Some("icon.png")`.
  - Response: fixed on-branch - the doc bullet now spells out the
    `Some("icon.png")` requirement. Verified by reviewer.
- [x] R1.2 (NIT) [pre-existing] crates/nova_menu/src/lib.rs
  `mods_panel_lists_catalog_demo_toggle_base_locked` - asserted only the
  `ModToggle` ids; no test pinned that the rendered `Text` nodes carry
  `meta.name`/`meta.description`, so a swapped field in `spawn_mod_row` would
  pass the suite.
  - Response: fixed on-branch - the test now also queries the rendered `Text`
    nodes and asserts "Demo Mod" (meta name, not id) and the description
    render. nova_menu 13 passed. Verified by reviewer.
