# Review: Mod dependencies

- TASK: 20260715-142931
- BRANCH: feature/mod-dependencies

## Round 1

- VERDICT: REQUEST_CHANGES

Reviewed the branch diff against master with fresh eyes plus an independent
out-of-context pass. Both re-derived the algorithms and traced the flows. The
implementation is correct: Kahn's puts dependencies before dependents (indegree/
relaxation direction verified by hand on chain/diamond/cycle graphs); the merge
direction is right (dependent overlays dependency, last-wins); the disable block
via DIRECT enabled dependents is transitively sufficient; the install cycle
terminates (job recorded before resolution + the in-flight guard); every new
test fails if its mechanism is deleted; the mod_cache_install fixture change is a
legitimate realignment with the Gauntlet bundle description that landed on master
(commit 9ab88eef), not part of this feature; and the new `nova_menu ->
nova_mod_format` edge is the safe direction (no dependency cycle).

Findings (addressed on-branch since this cycle merges):

- [x] R1.1 (MAJOR) crates/nova_assets/src/portal.rs:1001-1011 + NOTES.md - the
  dependency SET install is best-effort, not atomic: the mod and its deps fetch
  in PARALLEL with no join, so if a dependency's download fails ASYNCHRONOUSLY
  (network/sha error mid-download, distinct from "absent from the portal" which
  the pre-fetch loop already fails), the dependent still commits, leaving it
  installed with an unmet dependency. NOTES implied cross-dependency atomicity it
  does not provide.
  - Response: fixed - corrected NOTES.md and the portal.rs comment to describe
    the real semantics: each mod's own files commit atomically (staged), but the
    dependency SET is best-effort; a dependency whose download fails surfaces as
    its OWN `Failed` job and, if the dependent is later enabled, the enable-time
    "depends on X, which is not installed" warning (on_mod_toggle) - so the
    partial state is surfaced, never silent, and each mod is retried
    independently. Filed a follow-up for a true atomic dependency-set install if
    demand appears.
- [x] R1.2 (MINOR) crates/nova_assets/src/lib.rs:558-565 - if a downloaded id
  equalled a catalog id, `by_id` would keep only one handle and the other bundle
  could merge twice; guarded in practice by the install-time shadow rejection.
  - Response: fixed - added a comment noting the shadow-rejection invariant that
    keeps `ordered` ids unique (defensive).
- [x] R1.3 (MINOR) crates/nova_assets/src/lib.rs:542-560 - the merge graph is
  built only from LOADED bundles, so a not-yet-loaded dependent contributes no
  edges and may briefly merge before its dependency; self-corrects on the
  loaded-event re-run.
  - Response: fixed - added a comment documenting the transient gap + the
    re-run that closes it.
- [x] R1.4 (NIT) crates/nova_mod_format/src/deps.rs:56-58 - the
  `topological_order` doc oversells the tiebreak ("equal-precedence mods keep
  input order"); a node blocked in an early Kahn round loses its slot to later
  independent nodes.
  - Response: fixed - softened the doc to match the honest test note (the
    tiebreak holds among nodes ready in the SAME round; the hard guarantee is
    only dependencies-before-dependents).

Known limitation recorded (not a blocker): `on_mod_toggle` builds its dependency
graph from `ModCatalog`, which excludes HIDDEN mods, so auto-enabling a
dependency on a hidden mod would wrongly warn "not installed". No hidden mods
ship today (the screenshot-reel was unshipped), and the merge order uses the
full catalog, so this is untriggered; noted in the close-out.

## Round 2

- VERDICT: APPROVE

All Round 1 findings addressed (R1.1 by correcting the overclaim + surfacing the
best-effort semantics and its safety net; R1.2-R1.4 by comments/doc). Re-ran the
full check suite: fmt clean, `cargo check --workspace --all-targets` clean, and
all affected tests green (nova_mod_format 9, nova_assets 46 + integration suites
incl. the 2 new install tests, nova_menu 40, nova_portal_gen 12, nova_modding 1).
