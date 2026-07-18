# Review: Enemy-ship diegetic damage - black-out destroyed sections

- TASK: 20260718-181305
- BRANCH: feature/enemy-damage-tint

## Round 1

- VERDICT: APPROVE

Small, focused diff (one module + CHANGELOG + task file). Delivers the Goal:
enemy sections read burnt-black only when destroyed/disabled, with no
intermediate red or glow, while the player's full gradient is unchanged.

Independently verified:

- `damage_look` (the player Full path) is byte-identical to master
  (`git diff master...HEAD` does not touch it), so player behaviour is
  provably preserved. The two player end-to-end tests pass unchanged in
  behaviour.
- `SectionDamageTint` / `PendingSectionTint` are constructed only inside this
  module, so threading the new `mode` field needs no other call-site updates.
- Gate logic (damage_tint.rs:184-190): `Allegiance::Player -> Full`,
  `Enemy -> DeadOnly`, `Neutral | Err -> continue`. `Allegiance` is a required
  component of both ship markers, inserted synchronously at spawn, so it is
  present when capture runs (same timing guarantee the v1 player path relied
  on). Reading one `Allegiance` query rather than two marker queries is a clean
  choice and covers future non-AI enemies.
- Grading branch (damage_tint.rs:214-224): disabled section -> `DEAD_COLOR`
  (both modes); `DeadOnly` with `ratio <= 0.0` -> `DEAD_COLOR`, otherwise
  pristine. `ratio` is clamped to [0,1], so `<= 0.0` fires exactly at zero
  integrity. The read-before-mutate change-detection guard is preserved.
- The new test `enemy_section_blacks_out_only_when_destroyed_never_reddens`
  has a real delivery guard: it asserts the mesh IS captured as `DeadOnly`
  and DOES become `DEAD_COLOR` at 0 HP / when disabled, so the "pristine at
  partial health" assertion cannot pass trivially - it fails if the fix is
  reverted to the player-only gate or the DeadOnly branch is dropped.
- Full check suite: `cargo check -p nova_gameplay` clean (only a pre-existing
  `proc-macro-error2` future-incompat from a dependency, unrelated). Module
  tests: 5 passed, 0 failed.

Findings:

- [x] R1.1 (NIT) crates/nova_gameplay/src/sections/damage_tint.rs:39-41 -
  `TintMode` is the type of the `pub mode` field on the exported
  `SectionDamageTint`, but it is not re-exported in the module `prelude`
  (which lists `SectionDamageTint` and `SectionDamageTintPlugin`). A downstream
  reader of `tint.mode` would have to reach past the prelude for the type. No
  external consumer exists today, so this is discretionary; add `TintMode` to
  the `prelude` re-export for API consistency.
  - Response: Done - added `TintMode` to the module `prelude` re-export
    (damage_tint.rs:40). `cargo check -p nova_gameplay` stays clean. Verified
    by reviewer.
