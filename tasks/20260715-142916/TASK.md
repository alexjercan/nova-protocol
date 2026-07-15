# Explore online tab: fetch portal catalog, install/uninstall/update from the menu, offline handling

- STATUS: OPEN
- PRIORITY: 13
- TAGS: modding, menu, wasm

Spike: tasks/20260714-202515/SPIKE.md
Depends on: 20260715-142906 (download runtime) and 20260715-142911 (two-pane
screen).

Goal: the "Explore online (coming soon)" placeholder becomes real. The Explore
tab fetches `catalog.json` from the portal on tab open (spinner while loading;
on failure an error + retry button, falling back to the last cached catalog
with a "stale" note - cache the fetched catalog in the small-prefs store).
Rows mark already-installed entries and show an Update badge when the installed
version differs from the catalog's (v1: exact string compare). The details
panel gains Install / Uninstall / Update actions wired to the download runtime,
with progress + error states. Installing registers the mod into the installed
set live (existing re-merge); it appears on the Installed tab immediately.
THE GOAL of the spike family is met here: browse the portal, install the demo
mod, enable it, play it - on native and web.


Note from 163508's review (20260715, R1.3): the portal client has NO
timeout/cancel recovery - a transport callback that never fires wedges
RemoteCatalog::Fetching (re-fetch refused) and wedges an id in
Fetching/Committing (retry and uninstall refused). ehttp does call back on its
own error paths, so this needs a pathological failure - but the UI must own
the recovery surface: a cancel/retry affordance and/or a timeout that moves
the job to Failed. Design it into the Explore tab's states.

## Plan (20260715)

Everything below builds on landed contracts: the portal client's event API
(FetchPortalCatalog / InstallPortalMod / UninstallPortalMod; RemoteCatalog /
InstallJobs / DownloadedMods, all pub in nova_assets), and the mods screen's
markers (ModsActiveTab, ModsList, ModDetailsPanel, ModDetailsActions,
SelectedModId). nova_menu already depends on nova_assets.

- FETCH ON TAB OPEN: switching to Explore triggers FetchPortalCatalog when
  RemoteCatalog is Idle (Ready/Fetching left alone; Error shows retry).
- LIST STATES on the Explore tab: Fetching -> one muted "Fetching the mod
  portal catalog..." row; Error -> error row + a Retry themed_button
  (re-triggers fetch; if a LAST-GOOD catalog exists, its entries render below
  a muted "offline - showing the last fetched catalog" row); Ready -> one row
  per PortalEntry (meta name, "vX - by author" line) with a right-aligned
  STATUS tag instead of a checkbox: "installed" (id in DownloadedMods),
  "update" (installed and versions differ - exact string compare), or none.
  Rows are selectable (SelectedModId reuse; Explore selection keys into the
  remote catalog for details).
- LAST-GOOD CATALOG (offline fallback): extend nova_assets::portal -
  RemoteCatalog becomes { state, last_good: Option<PortalCatalog> }; on Ready
  persist the raw JSON via a mod_prefs-style small store (native file under
  the config dir / wasm localStorage; skip persisting if serialized size >
  256 KiB - a cap, not a quota); on startup load it into last_good. State
  transitions never clear last_good.
- DETAILS on Explore: meta fields as on Installed, plus the remote version
  line; ModDetailsActions shows by state: not installed -> "Install";
  installed -> "Uninstall" (+ "Update" when versions differ); job in flight ->
  progress text ("Downloading 2/3...", "Committing...") + no buttons; Failed
  -> the error text + "Retry" (re-trigger install) + "Dismiss" (clears the
  InstallJobs entry - the R1.3 recovery affordance; InstallJobs is pub).
  Catalog-fetch wedge recovery: the Error/Retry path force-resets
  RemoteCatalog.state to Idle before re-triggering.
- UPDATE = uninstall-then-install choreography: an UpdateRequested(id set)
  resource; the Update button triggers UninstallPortalMod and records the id;
  a small system fires InstallPortalMod once the id has left DownloadedMods
  AND PendingRemovals (the 163508 race guard) - then clears the request.
  Timeout: if the request outlives 30s (wall clock), drop it with a warn.
- INSTALLED-TAB parity: downloaded mods' rows get an "Uninstall" button in
  the details action area too (same trigger), so managing installs does not
  require the Explore tab.
- TESTS (nova_menu; minimal apps with inserted portal resources - no real
  transport): Explore tab with RemoteCatalog Ready renders entries + status
  tags; selecting a remote row shows its details; the action button triggers
  the RIGHT event with the RIGHT id (observer-capture assertions) for
  Install/Uninstall/Update; Failed job shows error + Retry/Dismiss and
  Dismiss clears the entry; Error-with-last_good renders the stale note +
  entries; the update choreography system fires install only after both
  guards clear (pure-ish unit with inserted resources). Portal-side: a unit
  test for last_good persistence round-trip.
- VISUAL VERIFY (first-class step per the 142911 retro): Xvfb screenshot of
  the Explore tab in Ready state (real portal client + the tiny_http/
  nova_portal_gen serving rig from portal_install if wiring is cheap, else a
  synthetic Ready catalog) - eyeball rows/tags/details/actions; capture into
  the scratchpad and Read it.
- THE GOAL: after this task the coming-soon placeholder is fully dead - a
  player can browse the portal, install gauntlet, enable it, and play it, on
  native and (compile-gated) web. State this in the close-out with the
  evidence chain (menu event tests + portal wire e2e + screenshots).
- DOCS: CHANGELOG (Added: Explore online tab); docs/mod-portal.md "browsing
  in game" sentence; the spike's Fix record entry comes at compound time.

Steps:
- [ ] 1. nova_assets::portal: last_good catalog (field + persistence helper +
  startup load + cap) + unit round-trip test.
- [ ] 2. nova_menu: Explore list states (fetch-on-open, Fetching/Error/Ready
  rows, status tags, stale-note fallback rendering).
- [ ] 3. nova_menu: Explore details + actions (Install/Uninstall/Update,
  progress, Failed + Retry/Dismiss, catalog-retry reset); Installed-tab
  Uninstall parity.
- [ ] 4. Update choreography (UpdateRequested resource + system + timeout).
- [ ] 5. Tests per plan (menu event/observer assertions; choreography unit;
  persistence round-trip).
- [ ] 6. Visual verify: Xvfb screenshots of Explore Ready (+ details) into
  the scratchpad; eyeball and record the verdict.
- [ ] 7. Docs + CHANGELOG; close-out with the goal-evidence chain.
- [ ] 8. Verify: fmt; check --workspace --all-targets; cargo test -p
  nova_menu, -p nova_assets (--lib, --test portal_install, --test
  mod_cache_install, --test demo_scenario); wasm gate (cargo check --target
  wasm32-unknown-unknown -p nova_assets -p nova_core -p nova_menu).

## Notes (plan)

- Relevant files: crates/nova_assets/src/portal.rs (RemoteCatalog:~states,
  InstallJobs, events), crates/nova_menu/src/lib.rs (tab/list/details systems
  and markers from 142911), crates/nova_assets/src/mod_prefs.rs (small-store
  idiom for last_good), crates/nova_assets/tests/portal_install.rs (serving
  rig to borrow for the screenshot step).
- Version compare is exact string inequality (spike decision; semver ordering
  deferred).
- The wasm runtime path remains compile-gated; a manual web session after
  deploy is the remaining honest gap (final flow report will say so).
