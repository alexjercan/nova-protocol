# Open world New Game: base world crate, line warship, seed modal

- STATUS: OPEN
- PRIORITY: 73
- TAGS: v0.15.0,gameplay,architecture,open-world

Fourth PR in the stack: #58 -> #59 (`nova-world-foundation`) -> #61
(`nova-world-generator-interface`) -> this branch (`nova-open-world-new-game`).
Parent spike: `20260824-125938`.

## User facts

- New Game launches the procedural open world. Basic Training stays under
  Lessons.
- New WASM-capable crate `nova_world_base` owns `NovaLayeredWorld`, the
  base-world session integration and the stable ids it needs.
  `NovaLayeredWorld` leaves `nova_authoring`. `UniformAsteroids` stays in the
  examples.
- Static `NovaWorldBasePlugin` installs `NovaWorldPlugin<NovaLayeredWorld>` at
  app construction on the default game path, after scenario. No runtime plugin
  insertion. Custom example paths keep installing their own generator. No
  `WorldConfig` means no streaming work.
- `ScenarioRole::OpenWorld`, not listed in the Scenarios picker. The base bundle
  declares the bootstrap as `new_game_scenario`; the menu hardcodes no id.
- Generated base `open_world` bootstrap and a large `block_line_warship`:
  connected hull, power, controls, several thrusters, six PDCs, one spinal
  railgun, two torpedo bays. PDC on mouse, railgun `R`, torpedoes `F`.
- New Game opens a modal in MainMenu: decimal u32 seed field, fixed profile
  summary, Randomize, Create, Cancel. Fresh seed from `bevy_rand` entropy on
  open. Invalid, blank or overflowing input is refused inline. Create inserts
  `OpenWorldSession`, clears `NewGameScenario`, sets `GameMode::NewGame`, goes
  to Playing once. Retry keeps the seed. No persistence.
- Determinism promise: same build and platform, pristine generation,
  independent of exploration order. No cross-platform or mutation claims.
- Generated asteroids and planets stay invulnerable. Derelicts may regenerate
  after retirement.

## Decisions

- Public API: `NovaWorldBasePlugin`, `OpenWorldSession { seed: u32 }`,
  `NovaLayeredWorld`, `OPEN_WORLD_SCENARIO_ID = "open_world"`,
  `BLOCK_LINE_WARSHIP_SHIP_ID = "block_line_warship"`.
- Sync placement (owner, 2026-09-23, Option A): ONE synchronization point in
  `Update`, ordered before `NovaWorldSystems::Cleanup`, not PreUpdate.
  StateTransition (`OnEnter(MainMenu)` loads a backdrop) runs after PreUpdate,
  so a PreUpdate sync could leave the config armed over a non-open-world
  scenario and trip the zero-observer panic. It is an exclusive system, so its
  config and `WorldObserver` writes are visible to Cleanup and Observe in the
  same frame. nova_world's fail-loud zero-observer rule stays.
- Private menu module `crates/nova_menu/src/world_setup.rs`.

## Done when

- The stacked PR is open against `nova-world-generator-interface`, rebased on
  its latest head, with the focused checks green.
- Start Basic Training (owner, 2026-09-23, Option A): the first-launch card
  sets `NewGameScenario(Some(TUTORIAL_SCENARIO_ID))`. The id moved to
  `nova_training` as the lowest shared owner. The declared start is now the
  open world, which needs a session only Create inserts.

## Verification

- Rebased on `d2b85014e`; everything below re-ran on the rebased head.
- Unit: `nova_world_base` 9 (visit-order determinism, arm, no rewrite,
  disarm on other role and no player, refuse no session and two players,
  the 128 km edge ceiling), `nova_world` 22,
  `nova_menu` lib 186 (6 modal tests, renamed Start Basic Training test),
  `nova_training` 38, `nova_authoring` lib filtered 43.
- Integration: `content_ron_parity`, `campaign_membership`,
  `content_lint_gate`, `nova_assets --test example_scenario`,
  `nova_probe_cli --test catalog_drift`, `scripts/check-probe-suites.py`.
- Content: `content gen` then `content lint`: 0 errors, 20 creative maps.
- Clippy `-D warnings`: native and `wasm32-unknown-unknown` for
  `nova_world_base`, `nova_menu`, `nova_core`, `nova_training`,
  `nova_authoring`.
- Probe `--correctness-only` on an RTX 3060 Ti under Xvfb: system_open_world,
  system_world_sectors, system_menu_boot, system_session_loop, system_command_shell,
  bug_failed_assets, screenshot_loading_fact, lesson_menu_mouse all OK.
  system_open_world recorded all nine outcomes: 6 turrets, 1 lance, 2 bays;
  125 sector roots; 34 rounds, 1 lance shot, 2 torpedoes; Retry kept the
  seed; 334 armed frames each ran Observe; 45 off-world frames with no config
  or sector root.
- Manual captures on hardware: the modal with a fresh seed, the inline
  refusal with Create greyed, the warship in the world at 54-60 fps, and
  the weapons firing.
