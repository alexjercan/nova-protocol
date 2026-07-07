# Audit and finalize nova_core crate as thin wiring layer

- STATUS: CLOSED
- PRIORITY: 100
- TAGS: v0.3.1, refactor, crates

nova_core should only assemble plugins from the other crates into the runnable game. Verify it contains no gameplay logic; move anything substantive into nova_gameplay or a dedicated crate. [new]

## Steps

- [x] Audit nova_core: lib.rs is thin wiring (AppBuilder), but core.rs (~1100 lines)
      is the whole spaceship editor scene - substantive gameplay/editor logic.
- [x] Decide the destination. Moving into nova_gameplay is impossible (the editor uses
      nova_scenario, which already depends on nova_gameplay -> would be a cycle), so
      extract to a dedicated nova_editor crate.
- [x] Relocate GameStates from nova_core to nova_gameplay so both nova_core and
      nova_editor can gate on it without a dependency cycle.
- [x] Create crates/nova_editor; move core.rs there as NovaEditorPlugin.
- [x] Rewire nova_core: depend on nova_editor, add NovaEditorPlugin in AppBuilder when
      no custom game plugins are supplied, slim its deps.
- [x] Verify: build --all-targets, clippy, fmt all green (examples + binary unchanged).

## Resolution (CLOSED)

nova_core held two very different things: lib.rs (AppBuilder + window/log/asset plugin
setup + status UI) which is legitimate thin wiring, and core.rs, a ~1100-line
spaceship editor scene (section-picker UI, ship building/placement, transition to the
scenario simulation, a hardcoded test scenario). The editor is substantive
gameplay/editor logic and does not belong in the wiring crate.

What changed:
- New crate crates/nova_editor exposing NovaEditorPlugin (the former core_plugin body,
  verbatim). Depends on nova_gameplay + nova_scenario + nova_assets.
- GameStates moved from nova_core to nova_gameplay. Both nova_core and nova_editor need
  it; putting it in the foundational gameplay crate avoids a nova_core <-> nova_editor
  cycle. It stays re-exported through every prelude, so examples and the binary that
  reference GameStates via nova_protocol::prelude did not change.
- nova_core now: AppBuilder assembles GameAssetsPlugin, NovaGameplayPlugin,
  NovaScenarioPlugin, and (when no custom game plugins are supplied) NovaEditorPlugin.
  Its dependency list dropped avian3d, bevy_hanabi, rand, and itertools (they left with
  the editor); it now needs only bevy + bevy_enhanced_input + the nova crates.

Why a new crate rather than nova_gameplay: the editor imports both nova_gameplay and
nova_scenario types, and nova_scenario already depends on nova_gameplay, so folding the
editor into nova_gameplay would create a circular dependency. A dedicated crate is the
only clean home for code that sits above both.

Behavior is unchanged: NovaEditorPlugin is still added only when no custom game plugins
are provided (examples opt out exactly as before), the plugin body is identical, and
GameStates is the same type in a new module. Verified with build --all-targets, clippy,
and fmt (all green), which covers the binary and all examples.

Self-reflection: the circular-dependency constraint made the destination
(dedicated crate, not nova_gameplay) a forced move rather than a preference - checking
the dependency direction early avoided a dead end. Relocating GameStates was the one
non-obvious step; keeping it in the preludes meant zero churn for downstream callers.
