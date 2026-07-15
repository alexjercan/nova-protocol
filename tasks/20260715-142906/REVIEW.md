# Review: Mod cache + mods:// asset source + installed-set integration

- TASK: 20260715-142906
- BRANCH: feature/mod-cache

## Round 1

- VERDICT: REQUEST_CHANGES (one MAJOR)

Out-of-context review pass over the full ~1700-line diff; the implementer's
close-out claims were re-derived, not trusted, and all held: production
registration order traced main -> AppBuilder::new -> register_mods_source
before DefaultPlugins (AssetApp source rule verified in bevy source); the test
rig uses the literal production registration + systems; the or_else
run-condition reasoning verified against bevy_ecs' non-short-circuiting
condition evaluation (the deprecated-or/or_else semantics and consumed-ticks
subtlety are real); unloaded handles warn+skip on both arms; missing mods dir
degrades gracefully; wasm-IDB-unavailable degrades to decl rows without a
wedge; the reviewer re-ran the merge-arm sabotage themselves (two e2e tests
FAILED, revert -> green). Test re-runs: lib 30, mod_cache_install 4,
demo_scenario 11, webmods_validation 1; fmt/check/wasm32 check all green.

- [x] R1.1 (MAJOR) lib.rs:302-330 - the mods:// load path did not validate
  index records (a crafted installed.mods.ron with `../` in id/bundle raw-joins
  outside <data_root>/mods on native), and a downloaded bundle MANIFEST listing
  an underflowing `../` content path could escape independently of record
  validation.
  - Response: fixed in fbe3767e - (a) is_safe_id/is_safe_rel_path hoisted
    cfg-independent, enforced in the public cache API before dispatch AND in
    start_downloaded_loads (skip+warn); (b) SandboxedAssetReader wraps the
    native reader, rejecting any non-Normal path component. PREMISE CORRECTION
    (accepted): the live escape does not reproduce under default config -
    bevy's `unapproved_path_mode: Forbid` already rejects underflowing paths at
    the AssetServer; the sandbox stays as the reader-layer backstop so
    containment does not hinge on a config default. Three new tests, each
    sabotage-verified (record filter removed -> poisoned-index test FAILED;
    sandbox pass-through -> reader unit test FAILED).
- [x] R1.2 (MINOR) shipped/downloaded id collision produced duplicate rows and
  a both-merge on one toggle.
  - Response: fixed - skip-with-warn at both consumers (catalog row + merge),
    documented as NO SHADOWING in modding-ron-format.md, pinned by an e2e
    installing under the shipped id "demo" (sabotage-verified). "At load time"
    was not literally implementable (the shipped catalog asset is not loaded at
    Startup); the skip lives where the catalog is visible - accepted.
- [x] R1.3 (MINOR) wasm backend had no validation; `<id>/<path>` keys were
  ambiguous.
  - Response: fixed via the shared pre-dispatch gate (is_safe_id rejects `/`,
    excluding the ambiguity); documented + unit-pinned.
- [x] R1.4 (MINOR) IDB wrapper: no onblocked handler (future version bump
  wedges silently), leaked connections, and request-success != transaction
  commit undocumented.
  - Response: fixed - onblocked rejects, db.close() after every op, commit
    caveat comment addressed to 163508 on idb_put.
- [x] R1.5 (NIT) relative NOVA_MOD_CACHE_ROOT diverged between the reader and
  the fs helpers.
  - Response: fixed - std::path::absolute at the single env-read site.
- [x] R1.6 (NIT) the run-condition chain was copy-pasted into two rigs.
  - Response: fixed - public `installed_set_changed` condition shared by
    plugin and rigs; as a side effect it consumes both change ticks together,
    making the mark-system sabotage deterministically fail the arrival e2e too
    (the stale docstring was rewritten).
- [x] R1.7 (NIT) uninstall-while-enabled -> reinstall comes back enabled,
  undocumented.
  - Response: fixed - documented as deliberate (163508 decides pref-stripping).

## Round 2

- VERDICT: APPROVE

All seven responses verified: independent re-run confirms lib 32 passed,
mod_cache_install 7 passed (3 new security/collision pins), demo_scenario 11,
fmt clean, tree clean at fbe3767e; the sabotage matrix covers every new
mechanism (record filter, sandbox, collision skip, mark system). The R1.1
premise correction is accepted as reasoned, empirically-backed pushback - the
defense-in-depth outcome (server gate + reader sandbox + record validation) is
strictly stronger than the finding asked for. No new findings.
