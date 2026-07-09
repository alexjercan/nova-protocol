# Review: Hit feedback / game juice (camera shake, hit flash, impact FX)

- TASK: 20260708-162013
- BRANCH: feature/hit-feedback-juice

## Round 1

- VERDICT: APPROVE

Solid, well-scoped work that delivers the Goal: camera shake and impact/destruction
flash driven off the same `HealthApplyDamage` / `Add<IntegrityDestroyMarker>` seams
the audio layer uses, distance-attenuated and per-cell throttled, all tunable via a
reflected `JuiceSettings` resource. It correctly reuses the bcs `CameraShakePlugin`
(the exact "check bcs first" lesson from the audio retro) rather than hand-rolling a
drift-prone shake, and keeps everything wasm-safe with gizmos instead of particles
(`cargo check --target wasm32-unknown-unknown` is green). Pure helpers carry the
render-adjacent math and are unit-tested; `cargo test --workspace`, `clippy
--all-targets`, and `fmt --check` all pass. The subtlety/distance retune from the
playtest note is applied and documented.

No BLOCKER or MAJOR findings. The MINORs below are worth doing but at the
implementer's discretion.

- [x] R1.1 (MINOR) crates/nova_gameplay/src/juice.rs:~365 (`NovaJuicePlugin::build`) -
  only `JuiceSettings` is registered with `register_type`, but its nested
  `ShakeSettings` and `FlashSettings` are not. The bcs debug `WorldInspector` (and
  the future settings menu, the stated reason the resource is reflected) traverses
  the type registry, so the nested structs render as unregistered / non-editable
  until they are registered too. Add `.register_type::<ShakeSettings>()` and
  `.register_type::<FlashSettings>()` (and `JuiceEventKind` if you want it fully
  reflected). Note: no other Nova type calls `register_type` today, so registering
  the whole `JuiceSettings` tree (not just the root) is what makes this resource
  actually inspectable.
  - Response: Done. Registered `ShakeSettings`, `FlashSettings`, and `JuiceEventKind`
    alongside `JuiceSettings`, and derived `Reflect` on `JuiceEventKind` so it can be
    registered. `NovaJuicePlugin::build` now registers the full reflected tree.
- [x] R1.2 (MINOR) crates/nova_gameplay/src/juice.rs (tests) - the pure helpers are
  well covered, but nothing exercises the actual wiring: that `on_damage_juice` /
  `on_destroy_juice` feed trauma into `CameraShakeInput` and push a `Flash`, that a
  co-located same-frame burst collapses to one via the throttle, and that
  `master_enabled = false` suppresses both. AGENTS.md prefers integration tests where
  practical; a minimal-`App` test that spawns a camera + a damaged entity, triggers
  the events, and asserts `add_trauma > 0` / `ActiveJuiceFx.flashes.len()` would catch
  regressions the helper tests cannot (e.g. a future refactor dropping the trauma
  write). Add one observer-level test.
  - Response: Done. Added `emit_juice_*` integration tests over a minimal `App`:
    trauma+flash on a single event, per-cell burst collapse to one, distinct cells
    both firing, and `master_enabled = false` suppressing both effects.
- [ ] R1.3 (NIT) crates/nova_gameplay/src/juice.rs:~430/~460 - the impact kick/flash
  is placed at the damaged entity's `GlobalTransform`, not the true collision contact
  point the task mentions ("impact FX at collision points"). This is a fine
  approximation and matches how the audio layer sources its position, but if a precise
  spark location is wanted later, the contact manifold from the collision event would
  give it. Noted, not required for this task.
  - Response: Acknowledged, left as-is. Sourcing the entity transform matches the
    audio layer and reads correctly at gameplay scale; true contact-point FX can be a
    follow-up if desired.
