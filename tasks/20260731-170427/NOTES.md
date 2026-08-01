# NOTES - KISS: nova_scenario

## Structure

Three files held more than one concern each and were split into folder modules.
Public paths are unchanged: every submodule is private, `mod.rs` re-exports what
the crate prelude and the module preludes already named.

| Before | After | Split by |
| --- | --- | --- |
| `actions.rs` (2908) | `actions/{mod,view,flow,mission,ship,spawn}.rs` | what the action touches: the enum + dispatch, presentation, scenario flow, mission surface, live-ship retune, spawn/despawn |
| `loader.rs` (2849) | `loader/{mod,clock,trackers,lifecycle,fixtures}.rs` | config/resources/plugin, the engine clock + pulse, the state-derived trackers, the load/unload observers |
| `lint.rs` (2124) | `lint/{mod,scenario,ship,fixtures}.rs` | issue types, scenario/campaign reference checks, ship + section structural checks |

`fixtures.rs` (both `#[cfg(test)]`) holds the config builders the submodule
tests shared before the split; without it each split file would have grown a
copy.

One deliberate narrowing: `loader::OrbitHold` and `loader::LockEcho` were `pub`
under `pub mod loader`, so they were reachable public paths even though neither
is in any prelude and nothing outside the crate names them. They are now
`pub(super)` inside the private `loader::trackers`. Every path any consumer
actually uses is unchanged.

Largest remaining file is `objects/asteroid.rs` at 1070 - one concern (the
asteroid scenario object: body, collider, gravity well, noise mesh), so it
stays whole. Nothing else is over 1000.

## Comments

- Grep `//.*[0-9]{8}-[0-9]{6}` over `crates/nova_scenario/` returns **zero**
  hits. Every provenance clause was deleted; the constraint it wrapped was kept
  and, where it guards a value, promoted to `NOTE:`.
- Dangling `review R1.x` references (provenance with the task id already
  stripped) were removed the same way.
- New `NOTE:` markers: the ungated PostUpdate section systems and the
  Unpaused-gated OnUpdate pulse (`loader/`), the deferred skybox insert
  (`actions/view.rs`), the wgpu zero-area guard and bevy's `camera_system`
  re-derive quirk (`render_scale.rs`), the scatter count that no graphics tier
  thins (`actions/spawn.rs`), the observer-vs-system-set gating note and the
  despawn-ordering note (`loader/lifecycle.rs`), and the gravity-settings
  self-init (`objects/asteroid.rs`).
- Deleted narration: `// Setup directional light`, `// Fire onstart event`,
  `// Apply all the commands in the queue`, and the like.

## Verification

- `cargo check --workspace --all-targets` clean, `cargo fmt --check` clean.
- `cargo test -p nova_scenario --lib`: 145 passed. `--test skybox_swap_e2e`: 1
  passed.
- Test-name parity: the 90 `#[test]` names in the three pre-split files are
  byte-identical to the 90 in the split tree (`diff` of the sorted lists).

## Defects found

None. This pass was moves, renames, and comment deletions only; no behavior
change, so no backlog task was opened.
