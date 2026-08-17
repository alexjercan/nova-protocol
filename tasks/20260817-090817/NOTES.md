# Design

Accepted 2026-08-17.

- Author controller authority as `max_angular_acceleration`, in rad/s2.
- Break the old `max_torque` RON field. No compatibility alias.
- Clamp raw PD acceleration independently on each principal-inertia axis, then multiply by that axis inertia and transform the torque to world space.
- Derive helm and planner turn rate directly from acceleration. Hull inertia does not change handling by default.
- Keep controller stacking, but stack acceleration authority instead of absolute torque.
- Set shipped controllers to the 0.5 rad/s2 baseline unless a prototype has an explicit handling reason to differ. The shared 800 torque value compensated for large hull inertia and is not a distinct handling contract.
- Extend the attitude system range with a 10x-inertia convergence invariant. Add physics integration coverage for the same property.
- Update the editor, mod format docs, developer docs, changelog, and generated base content with the breaking unit and field change.

# Verification

- Code review: accepted with no requested changes.
- `cargo test --lib -p nova_ship`: 646 passed.
- `cargo check --example attitude_hold --features debug`: passed.
- Focused `attitude_hold` scripted run: completed all three rounds in 25.1s. The 10x-inertia hull tracked at 0.029 rad lag.
- `cargo test -p nova_probe_cli --test catalog_drift`: 2 passed.
- `cargo run content -- lint`: 0 errors and 0 warnings.
- `cargo fmt --check`: passed.
- `cd web && npm run ci`: passed.
- Full systems probe was stopped during example 16 because it was too slow. The first `attitude_hold` attempt exposed an unfair 3.4s response window after a near-180-degree reload; the focused range now uses a slower moving command and an 8s response window. Other completed probe failures were outside this change (`ship_editor`).

# Playtest

- Owner verdict: the arena capital feels better and is controllable, including
  the reported flight path.
