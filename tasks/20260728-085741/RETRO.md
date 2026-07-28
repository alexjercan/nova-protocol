# Retro: NOVA OS persistent header + main + footer

## What went well

- The feedback fixed placement ("header/footer always on screen, main in the
  middle") but left two load-bearing shape forks (the nova-ish breadcrumb word,
  and where the app close control lives given "right side is just FPS/ship" +
  "footer is just keybinds"). Surfacing those to the owner via AskUserQuestion
  with full-line previews BEFORE building - and recording them in DECISION.md -
  meant the header close-button-in-the-header choice was a decision, not a
  guess. This is exactly the miss the flow guideline warns about, avoided.
- The old layout already had the footer as a persistent sibling, so the change
  was mostly "promote the header the same way" - a small, well-shaped diff on a
  4k-line file. Keeping `NovaOsTopbarMarker`/`NovaOsLampMarker`/
  `NovaOsStatusMarker` on the new header node meant `drive_nova_os_topbar_fps`
  and the PoC structure test needed no change.
- The restructure made two hacks disappear rather than move: the app's own
  safe-area padding and the footer-reserve margin both became unnecessary once
  the app body fills `<main>` (already inset, already above the footer). Less
  code, not more.
- The `screenshot_nova_os` example already scripted welcome -> active -> map, so
  end-to-end visual proof of all three header states (`// SHELL`, `// APPS / MAP`,
  the app-only `[ ESC ]`) was one command, not a bespoke rig.

## What went wrong / difficulties

- First-pass tests covered the pure breadcrumb helper but not the live
  `reconcile_nova_os_header` behavior - the review (correctly) called that the
  central untested behavior of the task (DoD 2 + 4). Added an integration test
  that drives enter_app/exit and asserts brand text + close visibility.
- The review flagged that the sibling footer reconciler lacked the just-spawned
  `Added<>` override the new header reconciler had. The gap is masked today
  (`spawn_nova_os_footer` seeds defaults, `reset_session` forces Prompt), so it
  is not a live bug - but leaving two sibling systems with different respawn
  handling is a maintenance trap, so the footer got the same override.

## What to improve next time

- When adding a mode-keyed reconciler with a `Local` guard, give it the
  `Added<Marker>` just-spawned override in the SAME edit, and audit any sibling
  reconciler for the same pattern - they should match. A `Local` survives a UI
  teardown/respawn; the marker override is what makes a freshly spawned widget
  reconcile from a stale `Local`.
- For a UI-behavior task, write the live-tree reconciliation test alongside the
  pure-helper test in the first pass, not after review. The pure test proves the
  string; only the integration test proves the wiring (system registered, query
  targets, visibility toggled).
