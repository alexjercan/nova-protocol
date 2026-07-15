# Explore online tab: fetch portal catalog, install/uninstall/update from the menu, offline handling

- STATUS: CLOSED
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
- [x] 1. nova_assets::portal: last_good catalog (field + persistence helper +
  startup load + cap) + unit round-trip test.
- [x] 2. nova_menu: Explore list states (fetch-on-open, Fetching/Error/Ready
  rows, status tags, stale-note fallback rendering).
- [x] 3. nova_menu: Explore details + actions (Install/Uninstall/Update,
  progress, Failed + Retry/Dismiss, catalog-retry reset); Installed-tab
  Uninstall parity.
- [x] 4. Update choreography (UpdateRequested resource + system + timeout).
- [x] 5. Tests per plan (menu event/observer assertions; choreography unit;
  persistence round-trip).
- [x] 6. Visual verify: Xvfb screenshots of Explore Ready (+ details) into
  the scratchpad; eyeball and record the verdict. VERDICT: PASS - see the
  close-out.
- [x] 7. Docs + CHANGELOG; close-out with the goal-evidence chain.
- [x] 8. Verify: fmt; check --workspace --all-targets; cargo test -p
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

## Close-out (20260715)

THE GOAL IS MET - the "Explore online (coming soon)" placeholder is dead, and
the evidence chain for browse -> install -> enable -> play is:

- BROWSE: `explore_ready_lists_entries_with_status_tags` pins rows, wire meta
  and the installed/update tags against an inserted Ready catalog; the Xvfb
  screenshot `explore-ready.png` shows the same screen fed by the REAL
  pipeline - nova_portal_gen tree (gauntlet + two synthetic mods) served over
  localhost HTTP, fetched by the production `EhttpTransport` through the real
  `PortalPlugin`, with a pre-seeded local cache providing the "installed" and
  "update" tags.
- INSTALL: the menu's Install button is pinned to fire `InstallPortalMod`
  with the selected id (observer capture); the pre-existing wire e2e
  (`portal_fetch_install_enable_uninstall_over_the_wire`) proves that exact
  event installs gauntlet over a real socket into the real cache, that
  ENABLING it registers `gauntlet_run` into `GameScenarios` (the playable
  surface), and that uninstall reverses everything. The menu event tests +
  that e2e compose the full player flow; installing registers live via the
  existing DownloadedMods re-merge, so the mod appears on the Installed tab
  immediately (`installed_tab_details_offer_uninstall_for_downloaded_mods`
  covers the Installed-tab side).
- UPDATE/UNINSTALL: Uninstall pinned to `UninstallPortalMod` with the right
  id from both tabs; Update pinned to the uninstall-then-install choreography
  with both guards (DownloadedMods AND PendingRemovals) and the 30s timeout.
- VISUAL: `explore-ready.png` + `explore-details.png` (scratchpad), captured
  via a throwaway autopilot harness (14_screenshot_ui pattern; deleted, not
  committed) on Xvfb :99. Eyeballed with the Read tool: tabs, three rows,
  right-aligned muted "installed" / amber "update" tags, gauntlet
  default-selected with description + Install button; warp-beacons selected
  showing Uninstall + Update. VERDICT: PASS. (The top-left physics
  diagnostics overlay in the shots is the debug-feature dev overlay, not
  shipped UI.)
- WEB: compile-gated only - `cargo check --target wasm32-unknown-unknown -p
  nova_assets -p nova_core -p nova_menu` passes; a manual browser session
  after deploy remains the honest gap.

Decisions (deviations from the plan, all small):

- The last-good store's NATIVE file lives under the mod cache's data root
  (`<data_root>/portal_catalog.json`, honoring `NOVA_MOD_CACHE_ROOT`), not
  the config dir the plan sketched: the catalog is cached WIRE DATA, not a
  preference, and the cache root's test override is what keeps the
  integration rigs (which run the real plugin against localhost) from
  writing localhost catalogs into the developer's real store. The wasm half
  is localStorage as planned; the 256 KiB cap sits in the pure `save_to` so
  the unit test pins it.
- Idle renders the same muted "Fetching the mod portal catalog..." note as
  Fetching: in production Idle lasts one frame on tab open (the fetch
  trigger flips it), and portal-less rigs/slim apps read better with the
  note than a blank list.
- A pending update renders "Updating..." in the action area (not explicitly
  planned): it closes the visible gap between the uninstall settling and the
  deferred install firing, where the pane would otherwise flash "Install".
- The stored last-good value is the RAW fetched JSON, re-gated by
  `decode_catalog` (schema probe included) at startup load - a store written
  by an older/newer build can never smuggle an unsupported schema in
  (`stale_last_good_with_unknown_schema_is_dropped_at_load`).

Counts (all green, plus fmt --check clean and cargo check --workspace
--all-targets clean):

- nova_menu: 29 (was 19; +10 new - ready rows/tags, fetch-on-open x2 states,
  error+stale fallback, error-without-last_good, action-event captures,
  choreography guards, timeout, Failed Retry/Dismiss, in-flight progress,
  Installed-tab uninstall parity; the old placeholder tab-switch test
  rewritten in place to the fetch-state rendering).
- nova_assets --lib: 44 (was 41; +3 new - last_good round-trip, size cap,
  stale-schema drop at load).
- nova_assets --test portal_install: 6 (count unchanged; the wire e2e
  extended with last_good stamping + on-disk store assertions).
- nova_assets --test mod_cache_install: 7; --test demo_scenario: 11 (both
  untouched, still green).
- wasm gate: cargo check --target wasm32-unknown-unknown -p nova_assets -p
  nova_core -p nova_menu: clean.

Sabotage A/B (each run: exactly one test failed, naming its mechanism;
reverted, suite green again):

- STATUS TAGS: `portal_status_tag` hardwired to "installed" ->
  `explore_ready_lists_entries_with_status_tags` FAILED ("downloaded at a
  different version string"); revert -> 29 green.
- UPDATE-CHOREOGRAPHY GUARD: the PendingRemovals check replaced with `false`
  -> `update_choreography_fires_only_after_both_guards_clear` FAILED at the
  "removal still pending" assert (the 163508 race guard); revert -> green.
- R1.3 RETRY RESET: the `state = Idle` force-reset removed from
  `on_catalog_retry` -> `catalog_error_renders_retry_and_the_stale_fallback`
  FAILED its "reset before re-trigger" capture assert; revert -> green.

Difficulties / residual risk:

- No real blockers; the 142911 action-area contract and the 163508 event API
  composed as designed. The one design wrinkle was keeping integration rigs
  hermetic against the new persistence - solved by rooting the store under
  the cache override (first deviation above).
- RESIDUAL (what actually ships after review round 1's R1.3 fix): an INSTALL
  whose transport callback never fires now times out into `Failed` (the
  `PortalFetchTimeout` stall window, Fetching-scoped) and lands on the
  Retry/Dismiss surface; the update choreography has its own per-stage 30s
  timeout. The one remaining wedge is the CATALOG fetch itself: a
  never-firing catalog callback leaves the list on the Fetching note with no
  affordance until restart (ehttp calls back on all of its own error paths,
  so this needs a pathological transport). A catalog-side timeout/cancel
  would need its own generation story and stays out of scope.

Reflection: the landed-contract discipline paid off - this task wrote zero
transport code and still got a real-wire screenshot by composing
nova_portal_gen + python http.server + `NOVA_PORTAL_URL` + a pre-seeded
cache index, all production paths. What could have gone better: the
`RemoteCatalog` enum-to-struct refactor touched every consumer match at
once; landing `RemoteCatalogState` as a separate commit-sized step first
would have made the menu diff easier to review. Next time budget the visual
step's rig BEFORE writing the UI - knowing the screenshot would come from a
real portal shaped the decision to keep row/tag rendering purely
resource-driven (no test-only branches), which is also what made the menu
tests cheap.

## Review round 1 (20260715)

Out-of-context review: REQUEST_CHANGES (1 MAJOR, 3 MINOR, 1 NIT). All
addressed in one commit on this branch:

- R1.1 (MAJOR, fixed): stale last-good entries rendered FULL actions, so an
  offline Update would uninstall a working mod and then fail its install
  half against the non-Ready catalog - destroying the install until
  connectivity returned. Fixed at BOTH layers: `spawn_portal_actions` takes
  a `catalog_ready` bit and, for fallback-rendered entries, withholds
  Install/Update (Uninstall - purely local - stays) under a muted "offline -
  reconnect to install or update" note; and `on_portal_action` refuses the
  Install/Update arms without `RemoteCatalogState::Ready` (defense in
  depth). Regression test `stale_entries_offer_no_install_or_update` pins
  the missing buttons, the note, AND that a synthetic stale Update/Install
  action fires nothing (no uninstall, no request, no install).
- R1.2 (MINOR, fixed): the 256 KiB cap now gates the LOAD side too - native
  `load_from` checks `fs::metadata` size before reading a byte, the wasm
  read checks the string's byte length; oversized stores are dropped with a
  warn. The cap unit test gained the planted-oversized-store load case.
- R1.3 (MINOR, fixed): the job-side wedge now has a real timeout -
  `timeout_wedged_fetches` fails an install whose `Fetching` stage stalls
  past `PortalFetchTimeout` (default 120s; progress per verified file resets
  the window) into `Failed("timed out waiting for the portal")`, landing on
  the existing Retry/Dismiss surface. Scoped to Fetching ON PURPOSE and
  documented on the constant: `Committing` is a local commit whose
  `Committed` message carries no generation - timing it out could race a
  late success into "installed + a stale dismissable Failed entry"
  (consistent but confusing); within Fetching no `Committed` can be in
  flight and late `File` callbacks are generation-dropped, so the abort is
  clean. Test `a_wedged_file_fetch_times_out_into_failed` drives the REAL
  system across frames with a 50ms injected `PortalFetchTimeout` and a
  transport that drops file callbacks on the floor; sabotage A/B (filter
  no-op'd): the test hung to its 60s pump deadline and FAILED, revert green
  in 0.06s. The Dismiss doc no longer overstates.
- R1.4 (MINOR, fixed): `UpdateRequested` entries became two-stage
  `UpdateRequest { since, re_enable, install_fired }`: the Update handler
  records the enabled bit BEFORE the uninstall strips it, and the
  choreography re-inserts the id into `EnabledMods` once the NEW record
  lands in `DownloadedMods` (the existing change-gated save persists it); a
  disabled mod stays disabled. Each stage gets its own 30s timeout window.
  Test `update_preserves_the_enabled_bit` (+ the disabled path pinned in the
  actions test's tail).
- R1.5 (NIT, accepted as cosmetic, no code change): the catalog Retry
  force-reset to Idle drops the stale rows (and with them the player's
  selection) for the fetch's duration; the refreshed catalog re-runs the
  default selection. Accepted: preserving selection across the reset would
  special-case the repair logic for a sub-second window.

Verify after the fixes: fmt --check clean; nova_menu 31 (+2), nova_assets
--lib 44 (cap test extended in place), portal_install 7 (+1: the wedge
timeout), wasm gate (nova_assets, nova_core, nova_menu) clean.
