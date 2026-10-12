# World first: New Game harness takes the wizard seed; chapters declare Void

- STATUS: OPEN
- PRIORITY: 96
- TAGS: v0.16.0,world-first

## User facts
- Epic `20261012-095526`. After the world block lands, the `open_world` scenario becomes the New Game harness: `Generated { seed: None }` means the seed comes from the New Game modal. Every shipped chapter, backdrop and tutorial declares `Void` and behaves exactly as today.
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Agent findings
- New Game inserts `OpenWorldSession { seed }` then enters Playing (`crates/nova_menu/src/world_setup.rs:324-370`); `start_new_game_scenario` picks the base-declared start or a picker override (`crates/nova_menu/src/menu_ui.rs:581-624`).
- `resume_world` re-inserts the session from the save header (`crates/nova_world_base/src/save/session.rs:234-268`); Retry streams the same world again (`crates/nova_world_base/src/lib.rs:150-156`).

## Delivery gate
- The harness reads its seed from the session when the block says `None`; a `Generated { seed: Some }` scenario launched from the picker creates no save folder on desktop unless the note decided otherwise. Migrate every base scenario builder to declare its block; regenerate RON; migrate the hand-written example mod.
- What may break: Retry, Load, the leave flow and the web build (no save module). What must fail loudly: a `Generated { seed: None }` scenario started with no session.

## Verification
- New Game, Retry, Load and a picker-launched chapter each reach Playing in a live run under Xvfb; the main-menu backdrop rotation still runs with no `WorldConfig`.

## Done when
- The open world is one more scenario on the harness path, no scenario is special-cased by role, and the example mod lints clean.
