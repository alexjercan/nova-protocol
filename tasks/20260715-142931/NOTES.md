# Mod dependencies - design notes

Task 20260715-142931. Make the `dependencies: [ids]` field resolve: install-time
pull, enable-time auto-enable, block-on-disable, topological merge order, and a
dependency status UI. Ids only - no version constraints.

## The shared helper (`nova_mod_format::deps`)

Engine-free, so the asset merge, the menu and their tests share one
implementation. Pure functions over an `id -> Vec<dep-id>` graph
(`DepGraph = HashMap<String, Vec<String>>`); an id absent from the graph has no
deps; `base` is implicit and never appears.

- `transitive_deps(graph, id)` - DFS post-order (a dependency before the mods
  that need it), cycle-tolerant, excludes `id`. Used by enable + install.
- `topological_order(ids, graph) -> { order, cycle }` - Kahn's algorithm with a
  STABLE input-order tiebreak (the caller passes catalog order); a cycle emits
  the remaining ids in input order and sets `cycle`. Used by the merge.
- `dependents(id, enabled_ids, graph)` - the enabled ids that DIRECTLY declare
  `id`. Used to block a disable.

## The four behaviours

1. **Merge order** (`register_bundles`). A mod's `Content` overlays its
   dependencies' (last-wins by id), so a dependency must merge BEFORE its
   dependents. Build the graph from the enabled bundles' `meta.dependencies` and
   `topological_order` the id list (catalog-then-download order as the stable
   tiebreak); reorder the bundle handles. `base` has no incoming edges and is
   first in catalog order, so it stays first. A cycle warns and falls back to
   input order.
2. **Enable / disable** (`on_mod_toggle`). Enabling a mod inserts it plus its
   `transitive_deps` into `EnabledMods` (Factorio). Disabling is BLOCKED (user's
   call) when `dependents` finds an enabled mod that still needs it - warn naming
   them, leave it enabled. `base` is never toggled or auto-enabled. A declared
   dep that is not installed warns but does not block enabling (it just will not
   merge).
3. **Install** (`on_install_portal_mod`). Before fetching, resolve
   `transitive_deps` from the portal (`PortalEntry.meta.dependencies`); a
   transitive dep that is neither installed nor in the portal FAILS the install
   naming it (up front, before anything commits); the missing ones are pulled
   via recursive `InstallPortalMod` triggers. The job is recorded BEFORE
   resolving deps so a dependency cycle is broken by the existing in-flight guard
   when a dep re-triggers this id.

   Atomicity is PER MOD, not across the dependency set. Each mod's own files
   commit atomically through the staged 142906 cache; the mod and its deps then
   download in PARALLEL with no join. So if a dependency's download fails
   asynchronously (a network/sha error mid-download - distinct from the
   absent-from-portal case above), the dependent still installs, leaving it
   present with an unmet dependency. That partial state is SURFACED, never
   silent: the failed dep shows its own `Failed` job (retryable), and enabling
   the dependent later warns "depends on X, which is not installed" and does not
   pretend the mod is whole. A true atomic dependency-SET install (roll the
   dependent back if any dep's download fails) is a possible follow-up if demand
   appears; best-effort-with-surfacing is the deliberate scope here.
4. **UI** (`spawn_details_meta`). Each declared dependency renders on its own
   line coloured by `dep_status`: enabled (cyan), installed-disabled (muted),
   missing (amber). Both the Installed and Explore tabs.

## Why these choices

- Disable = block + warn (user decision): never strand an enabled mod without a
  dependency. Cascade and warn-but-allow were the alternatives.
- Install recursion + in-flight guard rather than a bespoke queue: the existing
  "an install of X is already in flight" guard already dedupes and, with the job
  recorded first, terminates dependency cycles for free.
- The portal generator already rejects unresolvable deps and cycles AT PUBLISH
  (`nova_portal_gen`), so a real portal is clean; the runtime guards are
  defensive (wire data / hand-installed sets could still carry a bad graph).
- No version constraints: ids only, per the task. Semver ranges are a future
  task if real demand appears.

## Tests

- `nova_mod_format::deps` - 9 unit tests (transitive chain/diamond/cycle,
  topological reorder + stable tiebreak + cycle flag, dependents).
- merge order - a dependent overlays its dependency regardless of catalog order.
- enable/disable - auto-enable a dependency; block-then-allow the disable.
- install - auto-install a portal dependency; fail naming an unavailable dep.
- UI - `dep_status` classifies enabled/installed-disabled/missing; the details
  assertions render `<dep> - enabled` / `  none`.

## Deliberately out of scope

- Version constraints / semver ranges.
- A confirm prompt before auto-installing deps (they install silently, per the
  task).
- Cascade-disable (the block-and-warn path was chosen instead).
