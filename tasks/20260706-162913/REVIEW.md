# Review: Extract torpedo module; unhardcode blast params

- TASK: 20260706-162913
- BRANCH: refactor/torpedo-module

## Round 1

- VERDICT: APPROVE

A clean, behavior-preserving refactor plus the blast-param unhardcoding.

**Split.** `torpedo_section.rs` (~1300 lines) is now a `torpedo_section/` directory
with the module name unchanged - the public path (`torpedo_section::prelude`,
`TorpedoSectionPlugin`) is untouched. `mod.rs` keeps config/plugin/prelude/components
and the bay launcher; `projectile.rs` holds the in-flight systems (target tracking,
arming, detonation, PN guidance); `render.rs` holds the render observers. Extracted
systems are `pub(super)`, and `mod.rs` glob-imports both submodules, so the plugin
and the tests reach them. The submodule code is a verbatim move (done by line-range
extraction), so no logic drifted in the split.

**Blast params.** The `BLAST_RADIUS` / `BLAST_DAMAGE` consts are replaced by
`blast_radius` / `blast_damage` on `TorpedoSectionConfig`, carried onto the
projectile via a new `TorpedoBlast` component that `torpedo_detonate_system` reads.
Defaults match the old constants (30 / 100), so behavior is identical; the in-game
section config and the two detonation tests set the component.

Verified independently in the worktree:

- `cargo test -p nova_gameplay`: 29/29 pass (same set as before the refactor).
- `cargo clippy -p nova_gameplay -p nova_assets`: clean - notably no unused-import
  or visibility warnings from the split.
- `cargo build --example 06_torpedo_range --example 07_torpedo_guidance --features debug`: green.
- Headless smoke (Xvfb): behavior unchanged - 06 = 3 fired/armed/detonated, 07 = 2
  detonations, no panic; identical to pre-refactor.

The "separate plugin for the targeting system" TODO is reasonably dropped rather
than implemented: target *selection* already lives in `input/player.rs`, and the
projectile-side `update_target_position` is just position tracking that belongs with
the other flight systems - a one-system plugin would be ceremony. The rationale is
recorded in TASK.md.

No BLOCKER/MAJOR. One NIT.

- [ ] R1.1 (NIT) `projectile.rs` groups guidance, arming, and detonation together.
  A finer split (guidance vs. detonation) is possible, but at ~250 lines the file is
  cohesive ("what the projectile does in flight") and further splitting would scatter
  closely-related systems. Fine as is.
  - Response:
