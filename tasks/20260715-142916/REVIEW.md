# Review: Explore online tab - browse, install, update and uninstall in-game

- TASK: 20260715-142916
- BRANCH: feature/explore-tab

## Round 1

- VERDICT: REQUEST_CHANGES (one MAJOR)

Out-of-context review pass over the full diff (7812e619), including the
reviewer's own visual verdict on the scratchpad screenshots (PASS - tabs,
status tags, details/actions all correct; the menu-card sliver behind the
panel edge is the 142911 z-fix working, accepted as-is). Re-derived clean:
last_good loads through the SAME schema gate as a live fetch and installs
never read last_good (a tampered store injects display text only); the update
choreography survives concurrent-install, failed-uninstall and menu-closed
races; the clock is bevy's platform Instant (wasm-legal); the UI state machine
resolves selections only in the active tab's source; the Retry force-reset
cannot double-fetch; the choreography sabotage re-run bit on the exact assert.
Counts reproduced (menu 29, lib 44, portal_install 6, mod_cache_install 7,
demo_scenario 11; fmt/check/wasm-gate clean).

- [x] R1.1 (MAJOR) offline Update destroyed the install: stale (last_good)
  entries rendered FULL actions; Update ran the local uninstall, then the
  reinstall failed on the Ready-catalog requirement - an offline player lost
  a working mod.
  - Response: fixed in ee070560 at both layers - stale entries render
    Uninstall only + a muted offline note (catalog_ready bit into
    spawn_portal_actions), AND the handler's Install/Update arms refuse
    without Ready (defense in depth). Regression test covers both layers
    including synthetic action events firing nothing. Verified.
- [x] R1.2 (MINOR) the 256 KiB last_good cap was save-side only; startup
  slurped an unbounded user-writable file.
  - Response: fixed - metadata size check before reading (native) + string
    length check (wasm); oversized-store load case added to the cap test.
- [x] R1.3 (MINOR) the job-side wedge had no affordance and the docs
  overstated the shipped recovery (Dismiss only exists on Failed).
  - Response: fixed - `timeout_wedged_fetches` fails a Fetching job with no
    transport progress within PortalFetchTimeout (default 120s; each verified
    file resets the window - a strict superset of the requested flat clock,
    accepted); lands on the existing Retry/Dismiss surface. Commit stage
    deliberately not timed out (Committed carries no generation; the race
    analysis is documented on the constant). Docs and the residual-risk
    wording corrected: the only remaining wedge is a never-firing CATALOG
    callback, stated plainly. Test drives the real system across frames with
    an injected 50ms timeout; sabotage-verified (filter no-op -> FAILED).
- [x] R1.4 (MINOR) Update silently disabled an enabled mod (uninstall strips
  EnabledMods; reinstall commits disabled).
  - Response: fixed - UpdateRequest records the enabled bit and re-enables
    after the new record lands (own 30s window); disabled mods stay disabled.
    Both paths test-pinned.
- [x] R1.5 (NIT) Retry drops the stale selection for the fetch duration.
  Accepted as cosmetic; acknowledged in the close-out.

## Round 2

- VERDICT: APPROVE

Fix commit ee070560 verified: nova_menu 31 passed (+2), portal_install 7
passed (+1 wedge-timeout), lib 44, fmt clean, wasm gate re-run clean, worktree
clean. The progress-reset timeout window is a sound improvement over the
requested design and its late-success race analysis is documented where the
constant lives. No new findings. THE FAMILY GOAL STANDS: browse -> install ->
enable -> play is covered by menu event tests + the wire e2e + eyeballed
screenshots; web remains compile-gated pending a post-deploy manual session.
