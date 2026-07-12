# Review: Tune PDC turret damage

- TASK: 20260712-172035
- BRANCH: fix/pdc-damage-tuning

## Round 1

- VERDICT: APPROVE

Data-only balance change (one const + a guard test); reviewed in-session with an
independent re-derivation of the load-bearing claims (review skill's re-derive
rule for a short diff):

- Guard math re-derived: ceiling = ASTEROID_HP / MIN_ROUNDS_TO_KILL = 100 / 12 =
  8.33; BETTER_TURRET_BULLET_DAMAGE = 4.0 <= 8.33 passes, and the old ~20.25
  fails - so the test genuinely pins the fix (falsifiable).
- Correct target: the player ship's turret is `better_turret_section`
  (nova_assets/scenario.rs:285,345), which is the const being lowered; the
  scavenger `light_turret` is untouched (still gentle).
- Scope: no code paths changed; `representative_kinetic_damage` is still used by
  the light turret so no dead import; the typed-damage core is unaffected.

Checks: cargo check --workspace --all-targets clean; nova_assets lib tests 16/16
(incl. the new guard); cargo fmt clean.

No findings. The exact value 4.0 is a playtest knob, not an invariant - the user
will confirm feel on re-test; the guard leaves headroom to ~8.3.
