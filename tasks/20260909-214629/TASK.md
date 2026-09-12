# Lifecycle defects: reloads, refused scenarios, failed assets and overlays

- STATUS: OPEN
- PRIORITY: 80
- TAGS: v0.14.0, bug, menu, scenario, review

## Goal

Fix the lifecycle and state-transition defects found by the 2026-09-09 read of
`nova_core`, `nova_menu`, `nova_scenario::loader`, `nova_assets` and
`nova_os_ui`. These paths are reached by ordinary store-player actions: return
to the menu, reload content, start with `--scenario`, load a broken mod or base
asset, refuse a scenario, press ESC on the outcome frame, alt-tab, open the
command shell, and resize the window.

Each defect group gets live proof that fails before its fix, then the fix. Keep
one commit per defect group. A group may add named invariants to an existing
systems range when that range already owns the complete player journey.

Owner (2026-09-09): "create systems examples for all edge cases and bugs we
find."

## Decisions

### Content reload policy

Use the Wesnoth-style policy. Reload content even when no disk change is known:

1. Leaving the editor for the main menu reloads content.
2. Leaving a gameplay scenario for the main menu reloads content.
3. Leaving the Mods screen reloads content.
4. F5 in the main menu remains the explicit manual reload.

The editor and gameplay both use `GameStates::Playing`, so one centralized
return path may cover them. The Mods screen remains a separate main-menu path.
Do not add a dirty-content condition.

The defect is the intermediate menu entry, not the unconditional reload. The
current `Playing -> MainMenu -> Loading -> MainMenu` route builds and loads a
backdrop that is immediately torn down. Return through
`Playing -> Loading -> MainMenu` instead, or otherwise suppress every transient
`OnEnter(MainMenu)` effect before the restart. There must be one loading screen,
no transient menu/backdrop load, and one final menu entry.

### Optional-mod safe mode

A broken optional mod must not strand an ordinary player at boot or require
manual cache edits.

- Split catalog/base loading from optional-mod loading. The base bundle and
  shared built-in assets are mandatory. An optional cataloged bundle must not
  remain a recursive dependency that makes the base `GameAssets` collection
  fail before its enabled/optional status can be considered.
- Detect failures from both cataloged optional mods and downloaded `mods://`
  bundles. Associate each failed asset or dependency with its owning mod and a
  useful reason.
- Collect all optional-mod failures, disable those mod IDs, persist the disabled
  set, and retry or continue without them. Keep the files installed.
- A failed downloaded bundle currently sits outside the main collection gate;
  it still needs the same quarantine, persistence and report even when base boot
  can continue without a state retry.
- Never retry forever. A retry that exposes another optional failure may disable
  and report that mod too; a mandatory/base failure takes the fatal path.
- After recovery reaches the main menu, show one modal `MODS DISABLED` report.
  List every disabled mod and its failure reason. `Continue` is an
  acknowledgement (`OK, I understand`): it dismisses the report and reveals the
  normal menu. It does not initiate recovery and does not re-enable anything.
- Show the aggregate report once per recovery episode. Do not show it on every
  later launch merely because the mods remain disabled. The Mods screen remains
  where the player can remove, update or deliberately retry them.

A mandatory base-asset failure is terminal. Replace the indeterminate loading
animation with a clear fatal report. Native offers Quit. WASM shows the same
failure information with browser-appropriate instructions to reload the page or
report the broken deployment; it must not show a fake Quit action. The failure
screen must work even when the boot font itself failed, using the default font
if necessary.

### Focus policy

Interactive gameplay automatically pauses on focus loss.

- In `Playing`, an unpaused interactive run opens the ordinary pause menu at the
  first playable frame after focus is lost.
- Focus regain never resumes automatically. The player selects Resume.
- A run already paused stays paused. An open NOVA OS remains the active modal.
  The main menu and asset-loading states do not acquire gameplay pause.
- Track current focus, not only a focus-event edge, so entering `Playing` while
  already unfocused still pauses once a live scenario exists.
- Windowless `--norender` and offscreen runs behave as focused. Ignore
  focus-driven pausing whenever `harness_env_active()` is true so rendered
  probes and software-rendered runs on an unfocused X display do not hang.
- An outcome remains the sole modal while it is shown. Its authored
  `auto_advance_secs` timer continues on `Time<Real>` while unfocused. If it
  advances while unfocused, load the next scenario normally, then transfer the
  outcome-owned pause into the ordinary pause menu without one unpaused
  simulation frame. The new scenario waits for Resume.

### Window policy

Fresh native installs default to borderless fullscreen. An existing store that
explicitly saved Windowed remains windowed. An older store with no window-mode
field receives the new default. WASM continues to fit its canvas and does not
expose the native mode row.

Windowed mode has a 640x600 minimum. The pause Settings panel uses responsive
width (`92%` with a 620 px maximum) and keeps its existing height/scroll cap so
it also fits a narrow WASM iframe whose size is not controlled by native resize
constraints.

## Defect groups and required behavior

### 1. Reload lifecycle, status UI and startup scenario

- [ ] `crates/nova_core/src/lib.rs:473` `setup_status_ui` runs on every
      `OnEnter(GameAssetsStates::Loaded)` with no teardown. A content restart
      adds another status-bar root; `insert_status_bar_item` then has an
      unsatisfied `Single<StatusBarRootMarker>`, and the new FPS/version items
      are missing. Keep exactly one root and one of each item across every
      reload. Teardown/rebuild across the asset-state restart is preferred when
      refreshed handles matter; spawn-once is acceptable only if it proves the
      same ownership and handle behavior.
- [ ] `crates/nova_menu/src/lib.rs:257` sends `ReloadContent` from
      `OnExit(GameStates::Playing)`. Preserve the unconditional reload, but do
      not enter and build a disposable menu/backdrop before it. The loading
      screen may appear; the transient backdrop and visible flash may not.
- [ ] `crates/nova_core/src/lib.rs:466,422` captures `boot_to_menu` once and
      reuses the startup choice after every content restart. A run launched with
      `--scenario <id>` must consume that launch request on the first successful
      boot only. Back to Main Menu then reloads content and stays in the menu
      rather than reopening the startup scenario forever.

### 2. Failed assets and safe mode

- [ ] `crates/nova_assets/src/plugin.rs:141,235`
      `OnEnter(GameAssetsStates::Failed)` only logs, while the loading screen
      despawns only on `Loaded`. Implement the optional-mod recovery and
      aggregate acknowledgement report described above. If the failed asset is
      mandatory/base, replace the animation with the fatal native/WASM report.

### 3. Refused scenario teardown

- [ ] `crates/nova_scenario/src/loader/lifecycle.rs:245,254` checks the content
      gate before teardown. A campaign's refused next scenario therefore leaves
      the previous scenario simulating under a blocking failure overlay with the
      cursor locked. Tear down before publishing the refusal: clear the event
      world, scoped entities, objectives/story/outcome/cheats and
      `CurrentScenario`; then set the failure report. `scenario_is_live` must be
      false, no previous simulation may tick, and the failure overlay must free
      the cursor like the outcome overlay. Main Menu remains the exit.

### 4. Outcome, pause and focus arbitration

- [ ] `crates/nova_menu/src/outcome.rs:114` and
      `crates/nova_menu/src/pause.rs:277` assume outcome and pause cannot race.
      ESC on the frame an `Outcome` action is still queued can open pause before
      the outcome resource changes; later `toggle_pause` returns early and
      leaves the pause panel over the outcome. Give outcome explicit precedence,
      order the systems, and reconcile an outcome when `PauseStates::Paused`
      already holds. Exactly one modal may remain.
- [ ] `crates/nova_menu/src/outcome.rs:209` advances on `Time<Real>`. Keep that
      authored timer running on focus loss, but use the focus policy above: if
      it expires while unfocused, the next scenario loads and immediately owns
      an ordinary player pause. No unpaused gameplay frame may pass.
- [ ] Add general interactive focus-loss pause behavior, including loss before
      scenario readiness, no automatic resume, NOVA OS/outcome precedence, and
      windowless/harness exemptions.

### 5. Menu ambience fallback

- [ ] `crates/nova_menu/src/ambience.rs:112` returns from the no-clean-backdrop
      branch without `UnloadScenario`; `load_menu_ambience` is the gameplay to
      menu teardown owner. A mod set with no clean backdrop can leave gameplay
      simulating behind the menu. Trigger unload before selecting a backdrop or
      spawning the fallback camera. The fallback path leaves no current
      scenario and exactly one fallback 3D camera.

### 6. Command shell and modal z ordering

- [ ] `crates/nova_menu/src/pause.rs:108` and `lib.rs:218` let `:` open the
      command shell on the main menu, freezing its backdrop and blocking its
      buttons. Gate the gesture on `GameStates::Playing`. It remains available
      over flight and the pause menu, subject to the existing input-mode and
      rebind guards.
- [ ] Verify the plausible z collision before fixing it:
      `crates/nova_os_ui/src/terminal/style.rs:190` gives
      `DRAWER_PANEL_Z` 11, tied with the pause Settings panel. Both are
      full-screen blockers during the transition from pause Settings to `:`.
      Give all coexisting modal layers deterministic, distinct z values even if
      the current traversal order happens to work. Preserve the intended order:
      HUD < outcome/failure < pause < pause Settings < active NOVA OS, with
      diagnostic/status exemptions above the NOVA OS and scenario loading above
      all of them.

### 7. Responsive pause Settings and fullscreen default

- [ ] Verify the plausible overflow at a rendered 500 px viewport, then replace
      the pause Settings panel's fixed 620 px width with the responsive policy
      above. Add the 640x600 native resize constraints and make borderless
      fullscreen the fresh-install default. Inspect the keybind rows and Back
      button at the narrow size; containment, not only computed root width, is
      the invariant.

## Proof matrix

Follow `examples/systems/README.md`: every assertion has an `outcome:` marker,
every slug is on the `crates/nova_probe_cli/tests/catalog_drift.rs` roster, and
new examples have explicit `Cargo.toml` blocks.

- [ ] `system_session_loop` from task `20260909-213441` carries the reload
      lifecycle group. Main menu -> New Game -> ESC -> Back to Main Menu -> New
      Game again, through real pointer clicks over `editor_app`. Add invariants
      for exactly one content restart, no transient backdrop load, one final
      backdrop, one status root with FPS/version after reload, no first-run
      entities/resources/HUD tiers, and `--scenario` returning permanently to
      the menu after its first boot.
- [ ] `bug_failed_assets` covers an optional cataloged mod failure, a downloaded
      mod failure, multiple failures aggregated into one report, persisted
      disablement, successful automatic recovery, acknowledgement-only
      Continue, and the mandatory/base native and WASM fatal presentations.
- [ ] `bug_refused_scenario` chains from a live scenario to a refused one and
      proves zero scoped entities, empty `CurrentScenario`, stopped simulation,
      cleared mirrors/event world, visible issue text, released cursor and a
      working Main Menu action.
- [ ] `bug_outcome_pause` queues an outcome behind a heavy spawn, presses ESC on
      the race frame, and proves outcome wins with one modal. It also loses
      focus during a timed outcome, proves the chain advances, and proves the
      next scenario gets no unpaused frame before the normal pause menu.
- [ ] `bug_menu_fallback` returns from gameplay with every backdrop absent or
      erroring and proves unload precedes the fallback camera.
- [ ] Extend `system_command_shell` with named invariants that `:` is inert on
      the main menu, opens during play and from pause, and the active NOVA OS has
      deterministic modal precedence.
- [ ] Extend `system_pause_settings` with a rendered 500 px viewport invariant,
      keybind-row and Back-button containment, 640x600 native constraints, and
      the fresh-install borderless default while an explicit saved Windowed
      choice survives restart.
- [ ] Run the affected ranges through `probe run ui` three times in a row after
      their shard is registered by task `20260909-213100`.

The two items originally marked plausible (modal z traversal and narrow-window
overflow) must be reproduced first. If current traversal or layout happens to
work, record that evidence and still pin the deterministic z and responsive
containment invariants; do not preserve equal modal z values or an uncapped
fixed-width panel.

## Commit groups

1. Direct content restart, status-bar ownership and one-shot startup boot.
2. Optional-mod loading isolation, quarantine/report and fatal base failure UI.
3. Refused-scenario teardown and cursor release.
4. Outcome/pause arbitration and focus-loss policy.
5. Menu fallback teardown.
6. Playing-only command shell and deterministic modal z ordering.
7. Responsive pause Settings, window constraints and borderless default.

Each commit includes its proof changes. Do not leave a regression range to a
later commit.

## Documentation

Apply `docs/keeping-docs-in-sync.md` in the same commits as behavior changes.
At minimum:

- add concise `[Unreleased]` changelog entries, collapsed by final user-visible
  behavior rather than one line per pre-release revision;
- update player documentation for menu reloads, automatic focus pause, safe
  mode, the command-shell availability and the fullscreen default where those
  behaviors are described;
- update architecture/project-tour claims for the asset loading split and state
  transitions;
- update rustdoc and module comments that currently claim every cataloged bundle
  is a recursive boot dependency, the command shell is global, focus does not
  pause, or a failed asset is always unrecoverable.

## Done when

- Every checkbox and named invariant above is complete.
- Optional broken mods cannot prevent reaching the main menu and are reported
  after automatic recovery.
- Mandatory/base failure is actionable on native and explanatory on WASM.
- Returning from editor or gameplay performs one unconditional content reload
  without a transient menu/backdrop; Mods Back and F5 retain the same reload
  policy.
- Outcome, pause, NOVA OS and focus transitions leave one deterministic modal
  and never leak an unpaused gameplay frame.
- The affected ranges pass in the `ui` shard three consecutive times.
- Affected unit/integration checks, docs builds and rendered output inspection
  pass.
