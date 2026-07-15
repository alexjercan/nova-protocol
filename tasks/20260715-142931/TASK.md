# Mod dependencies: auto-install/auto-enable deps, topological merge order

- STATUS: CLOSED
- PRIORITY: 8
- TAGS: modding

Spike: tasks/20260714-202515/SPIKE.md (option AC)
Depends on: 20260715-142916 (Explore tab - install path exists). Backlog until
a mod actually declares a dependency.

## Decision (user, 20260715)

- DISABLE behavior: Block + warn (Factorio). Disabling a mod that other ENABLED
  mods (transitively -> in practice directly) depend on is REFUSED, with a
  warning naming the enabled dependents. The player disables those first. Never
  leaves an enabled mod with an unmet dependency. (Enable always auto-enables
  deps; this decision is only about the disable direction.)

## Plan / Steps

Staged commits per part (one cohesive feature, one review). `base` is an
IMPLICIT dependency (always seeded/first) - never listed in `dependencies`.

- [x] **Part 1 - engine-free `deps` helper** (`crates/nova_mod_format/src/deps.rs`,
  `pub mod deps`). Pure functions over an `id -> Vec<dep-id>` graph
  (`HashMap<String, Vec<String>>`):
  - `transitive_deps(graph, id) -> Vec<String>` - DFS post-order (dep before
    dependent), cycle-tolerant, excludes `id`.
  - `topological_order(ids, graph) -> TopoOrder { order, cycle }` - Kahn's with
    INPUT-ORDER tiebreak (deps before dependents; independent ids keep input
    order); a cycle emits the remaining ids in input order and sets `cycle`.
  - `dependents(id, enabled_ids, graph) -> Vec<String>` - the enabled ids that
    DIRECTLY list `id` (for the disable block), deterministic.
  - Unit tests: transitive (chain + diamond + cycle), topo (reorders a dependent
    after its dep regardless of input order; stable tiebreak; cycle flag),
    dependents.
- [x] **Part 2 - topological merge order** (`register_bundles`,
  `crates/nova_assets/src/lib.rs:481`). Build `id -> deps` from the enabled
  bundles' `meta.dependencies`; `topological_order` the id list (current
  catalog-then-downloaded order as the input/tiebreak); reorder `bundle_handles`
  to match; a `cycle` warns and keeps input order. Test in the merge unit tests:
  a bundle that depends on a later-in-catalog bundle merges AFTER it.
- [x] **Part 3 - enable/disable** (`on_mod_toggle`,
  `crates/nova_menu/src/lib.rs:2607`). Add `catalog: Option<Res<ModCatalog>>`
  (dep map from `ModInfo.meta.dependencies`).
  - Enable X: insert X + `transitive_deps` into `EnabledMods` (skip `base`
    and any id absent from the catalog, with a warn for a genuinely missing dep).
  - Disable X: if `dependents(X, enabled, graph)` is non-empty, BLOCK (leave
    enabled) and `warn!` naming them; else remove X.
  - Tests: enabling a mod auto-enables its transitive deps; disabling a
    depended-on mod is blocked + the dependent stays; disabling a leaf works.
- [x] **Part 4 - install-time resolution** (`on_install_portal_mod`,
  `crates/nova_assets/src/portal.rs:873`). Before creating the job, resolve
  `transitive_deps` from the RemoteCatalog (`PortalEntry.meta.dependencies`);
  for each dep NOT already in `DownloadedMods` or the shipped `InstalledCatalog`,
  recursively trigger `InstallPortalMod` (visited-guard against cycles); a dep
  absent from the portal AND not installed fails X with a clear
  `InstallStatus::Failed` reason. Test in `portal_install.rs`: installing X with
  a missing portal dep also installs the dep; a dep absent from the portal fails
  X naming the dep.
- [x] **Part 5 - dependency UI** (`spawn_details_meta` +
  `refresh_mod_details`, `crates/nova_menu/src/lib.rs:2202/2441`). Pass
  `ModCatalog` + `EnabledMods`; render each dep id with a status: `enabled`
  (cyan), `installed, disabled` (muted), `missing` (amber). Both Installed and
  Explore tabs. Test: the dep line shows the right status per dep.
- [x] **Part 6 - docs + verify**. `tasks/<id>/NOTES.md` (design: the helper,
  the block-on-disable rule, cycle handling, install recursion); update the
  modding reference doc + CHANGELOG; full check suite + `cargo fmt`.

## Notes

- Dependency data reachability (all via `.meta.dependencies`): merge -
  `BundleAsset.meta` (`Res<Assets<BundleAsset>>` in register_bundles); enable/UI -
  `ModInfo.meta` (`ModCatalog`); install - `PortalEntry.meta` (`RemoteCatalog`).
- `base` is implicit (nova_mod_format ModMeta doc): seeded by
  `seed_enabled_mods`, first in catalog order, no incoming dep edges -> stays
  first under Kahn. Do not add explicit base edges.
- Existing dep validation: `nova_portal_gen` rejects an unresolvable dep at
  publish (`crates/nova_portal_gen/src/lib.rs:248`, test
  `unresolvable_dependency_is_rejected`) - so a published portal has no missing
  deps and no cycles; the runtime code still guards defensively.
- No version constraints (ids only) - semver ranges are a future task.
- check-all-targets for any struct/signature change; wasm install path is
  deploy-gated only (static review + native tests).

Goal: make the `dependencies: [ids]` field (schema landed with the bundle-meta
task, validated by the portal generator) actually resolve. Installing a mod
from Explore pulls its missing deps from the same catalog first; enabling a mod
auto-enables its deps (Factorio behavior; disabling a dep warns about
dependents); merge order becomes dependency-respecting topological order with
catalog order as the tiebreak. No version constraints yet (ids only) - semver
ranges are a future task if real demand appears. UI: the details panel's
dependency list links/marks missing vs installed deps.

## Close-out

Shipped all six parts (design in NOTES.md; review in REVIEW.md - 2 rounds,
APPROVE after addressing one MAJOR + minors on-branch).

- What changed: engine-free `nova_mod_format::deps` (transitive_deps,
  topological_order, dependents); topological merge order in `register_bundles`;
  auto-enable + block-on-disable in `on_mod_toggle`; install-time transitive
  resolution in `on_install_portal_mod`; per-dependency status in the details
  pane. Docs (CHANGELOG, modding-ron, ModMeta) updated. Six staged commits.
- Alternatives considered: disable = block+warn (user decision) over
  cascade/allow; the engine-free helper in nova_mod_format (shared by merge,
  menu, portal, tests) over duplicating logic; install recursion + the existing
  in-flight guard over a bespoke dependency queue (cycles terminate for free
  with the job recorded first).
- Difficulty: a stale `mod_cache_install` fixture (a parallel Gauntlet change
  landed a new description on master without updating the test) was RED on the
  branch base; realigned it (not part of this feature, noted in the commit).
- Evidence: nova_mod_format 9 (deps unit tests), nova_assets 46 + integration
  (portal_install 9 incl. 2 new dep tests, mod_cache 7, demo_scenario 11, parity
  2, ...), nova_menu 40 (+4 dep tests), nova_portal_gen 12; fmt +
  `cargo check --workspace --all-targets` clean.
- Reflection: staging the feature into per-part commits kept each testable and
  made the out-of-context review navigable; the MAJOR it caught (install
  atomicity is per-mod, not per-dependency-set) was a documentation overclaim,
  not a logic bug - corrected to describe the best-effort-with-surfacing reality.
- Known limitations (recorded, not blockers):
  - `on_mod_toggle` builds its graph from `ModCatalog`, which excludes HIDDEN
    mods, so auto-enabling a dependency on a hidden mod would wrongly warn "not
    installed". No hidden mods ship today; merge order uses the full catalog, so
    untriggered. Fix if hidden mods ever become dependency targets.
  - Install is best-effort across the dependency SET (per-mod atomic); a
    follow-up could make it roll the dependent back if a dep's download fails.
  - No version constraints (ids only) - a future task if real demand appears.

