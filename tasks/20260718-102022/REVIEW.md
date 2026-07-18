# Review: Configurable section collider shape and size

- TASK: 20260718-102022
- BRANCH: section-collider-config

## Round 1

- VERDICT: APPROVE

Independently re-verified the load-bearing claims rather than trusting the
implementer summary:

- avian 0.7 collider semantics read from source
  (`avian3d-0.7.0/.../parry/mod.rs`): `cuboid(x,y,z)` halves each extent
  internally (full side lengths in), `sphere(r)` uses r, `capsule(r,length)`
  places endpoints at +/- length/2 so its Y half-extent is `length/2 + r`, and
  `cylinder(r,height)` is `height/2` half along Y. All four match
  `SectionCollider::aabb_half_extents()` exactly, and the unit cube maps to
  half-extents 0.5 - so the generalized lint reduces to the old `< 1.0`
  threshold for default content. Confirmed by `content_lint_gate` linting the
  whole shipped tree clean.
- Spawn-path coverage: the only production section-collider injection points are
  `base_section()` and `preview_section()`, both wired. The other
  `Collider::cuboid(1,1,1)` sites (salvage.rs:562, actions.rs:1260,
  glue.rs:312/659) are all `#[cfg(test)]` bodies that never go through the
  section config, so leaving them is correct, not a miss.
- Reflection: `BaseSectionConfig`/`SectionKind`/`HullSectionConfig` derive
  `Reflect` but none are `register_type`'d (they serialize via serde, not the
  reflection registry), so `SectionCollider` deriving-but-not-registering
  matches its siblings - no runtime reflection dependency introduced.
- Test strength: reverting the lint fix would break the new lint test (the
  tightened-cube case would spuriously error; the oversized case would miss an
  error), and removing the field would break the serde round-trip test. The
  tests genuinely pin behavior.

Backward compatibility is clean: `serde(default, skip_serializing_if =
"Option::is_none")` means omitted colliders never serialize, so
`content_ron_parity` is unaffected; `unwrap_or_default()` reproduces the old
unit cube.

Findings:

- [ ] R1.1 (MINOR) crates/nova_scenario/src/lint.rs (half_extents helper) - a
  `Prototype` section falls back to the unit cube instead of resolving the
  prototype's authored collider, so a future prototype with a non-cube collider
  would be lint-checked at the wrong size. No shipped prototype sets a custom
  collider today and the limitation is documented on the function and in
  docs/design, so this is acceptable to ship. Worth a follow-up only if/when a
  prototype gains a custom collider (thread the catalog into the linter).
  - Response: Acknowledged; conscious tradeoff, documented. Left as-is per the
    "resolve only what is in scope" principle the surrounding lint already
    follows. Not filing a separate task until a prototype actually needs it.

- [ ] R1.2 (NIT) mass-follows-volume behavior - a custom collider scales real
  mass via density. Correct default and documented on `SectionCollider` and in
  docs/design; noted here only so it is a deliberate, recorded decision.
  - Response: Intentional; documented.
