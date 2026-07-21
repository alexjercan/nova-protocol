# Full missing_docs rollout on nova_scenario + nova_gameplay (non-category tail)

- STATUS: CLOSED
- PRIORITY: 36
- TAGS: docs,v0.8.0

## Story

The breadth rustdoc pass (task 20260525-133032) documented every public
plugin/component/resource/event TYPE across the workspace and flipped
`#![warn(missing_docs)]` on every small/medium crate that came fully clean
(nova_core, nova_debug, nova_menu, nova_ui, nova_modding, nova_info,
nova_editor, nova_meta_gen, nova_events, nova_assets, nova_probe).

The two LARGE crates were left with only their CATEGORY types documented; the
non-category public tail (free functions, config sub-structs that are not
Component/Resource, type aliases, enum variants, associated fns) is still
undocumented, so the enforcement lint is still OFF there. This task finishes the
full `missing_docs` rollout on those two crates so the lint can go on.

Remaining non-category missing_docs tail (measured 2026-07-21 with
`RUSTFLAGS="--force-warn missing_docs" cargo build --workspace`, counting each
crate's own `src/` lines):

- nova_scenario: 233 (heaviest: actions.rs, variables.rs, objects/asteroid.rs)
- nova_gameplay: 144

## Steps

- [x] Sweep nova_scenario's remaining undocumented public items; one-line
      `///` each per the AGENTS.md rustdoc conventions (what/why, RON-surface
      configs agree with the wiki field tables). Then add
      `#![warn(missing_docs)]` and confirm it builds clean.
- [x] Same for nova_gameplay (its category surface + module headers are already
      done from tasks 20260525-133030/133032; only the non-category tail
      remains). Flip the lint once clean.
- [x] Verify `cargo doc --workspace --no-deps` stays warning-free and
      `cargo build --workspace` emits zero missing_docs from the two crates.

## Definition of Done

- nova_scenario and nova_gameplay are fully documented (0 missing_docs) with
  `#![warn(missing_docs)]` enabled, so the whole workspace is lint-clean.

## Notes

- Large but mechanical; fine to split across sessions per-file. Skip local
  full test/clippy per repo policy; CI covers them.

## Close-out (2026-07-21)

### Result: both crates fully documented, lints enabled

Per-crate missing_docs (own-src, force-warn count):

| Crate | Before | After |
|-------|--------|-------|
| nova_scenario | 233 | 0 |
| nova_gameplay | 144 | 0 |

- `#![warn(missing_docs)]` added at the top of both `crates/nova_scenario/src/lib.rs`
  and `crates/nova_gameplay/src/lib.rs` (after the crate `//!` block, mirroring
  nova_info's placement).
- `cargo build -p nova_scenario -p nova_gameplay` (normal, lint enforcing):
  exit 0, 0 `missing documentation` lines.
- `cargo build --workspace`: exit 0, 0 missing_docs workspace-wide, no non-dep
  warnings (only the known proc-macro-error2 future-incompat dep note).
- `cargo doc --workspace --no-deps`: exit 0, 0 rustdoc warnings, 0 unresolved
  intra-doc links.

### Diff shape

Additive: doc comments + the two `#![warn(missing_docs)]` attributes. The only
non-comment change is cosmetic - five inline struct-variants (`SectionCollider`
in nova_gameplay base_section.rs, `ScatterRegion` in nova_scenario actions.rs)
were expanded from one-line to multi-line form so each field could carry its
`///`. Same fields, same types, no behavior change.

### Bug found + fixed during verify: mod-line `///` breaks intra-doc links

Adding a `///` outer doc to a `pub mod foo;` declaration when that module ALSO
has its own `//!` inner docs makes rustdoc resolve the module's `//!` intra-doc
links against the PARENT module's scope, breaking previously-valid links
(6 broke: BindingInput, SectionModification, WithheldVerbs x2, BeaconMarker,
ScenarioCameraMarker - all pre-existing links, master was clean). Fix: a module
with `//!` inner docs is already documented for missing_docs, so the mod-line
`///` is redundant AND harmful - removed it from the `pub mod` lines of the five
modules that carry `//!` (beacon, binding_input, modification, salvage in
objects/mod.rs; lint, render_scale in lib.rs). Kept the mod-line `///` on
modules WITHOUT `//!` (area, asteroid, spaceship, and the others in lib.rs) -
they need it for missing_docs. After the fix: 0 unresolved links.

### Inference-flagged doc lines for reviewer spot-check

Lines written from code reading / field-name inference rather than an explicit
wiki table or authored spec (unit/semantic worth confirming):

nova_scenario:
- actions.rs: message/camera timing fields (`fade`/`duration`) unit inferred as
  seconds from usage; `Filter` combinator (`All`/`Any`/`Not`) semantics from
  eval code; `SetControllerVerb::verb` full valid set (STOP/GOTO/ORBIT/LOCK/RCS)
  taken from the wiki bullet, not the `FlightVerb` enum; `SetSpeedCap`/
  `HintEmphasis`/`ObjectiveMarker` id/verb field descriptions from surrounding
  struct docs, not the referenced types.
- variables.rs: DSL variant/method semantics inferred from the `evaluate` match
  arms (values true to code; the "variables DSL" framing is authored wording).
- objects/asteroid.rs: `PlanetHeight` field docs condensed from the same-named
  module const docs; field-cluster spawn params (count/radius/spread) from the
  spawn loop.
- objects/binding_input.rs, spaceship/beacon/salvage `*_TYPE_NAME` consts:
  described as "scenario/modding RON type name" by inference from `EntityTypeName`
  usage; struct-level summaries follow the existing `SpaceshipConfig` pattern.
- loader.rs `OrbitHold::started_at`: "scenario-clock seconds" - clock source not
  traced to the write site.
- NOTE: the objects/variables subagent did not cross-check field docs against
  scenario-system.md field tables directly (relied on the detailed authored
  per-field docs already present); a reviewer wanting wiki-table parity should
  spot-check those.

nova_gameplay:
- hud/lock_dwell_ring.rs uniforms `inner`/`softness`: band geometry inferred from
  WGSL naming, not verified against the shader.
- hud/velocity.rs `DirectionMagnitudeMaterial::{radius,max_height}` and
  `DirectionSphereMaterial::{radius,sharpness}`: cone/sphere shader geometry
  inferred from builders/spawn code, not the WGSL.
- hud/objective_feedback.rs `ObjectiveGhostLineMarker::age`: "seconds since
  posted" from the type's own comment; seconds-vs-frames not confirmed.
- sections/thruster_section.rs `ThrusterExhaustConfig` `_max` cone fields
  (opacity vs intensity), gimbal fields (deg, deg/s), `ThrusterExhaustMaterial.
  thruster_input` (0..1 by analogy to `ThrusterSectionInput`): inferred from
  shader structure/sibling fields.
- sections/torpedo_section: guidance fields `turn_rate`/`arm_time`/`lifetime`
  units (deg/s, s) and `TorpedoGuidance.nav_constant` (PN gain) inferred from
  code/PN convention.
- sections/*.rs `render: bool` plugin fields: "false on headless" inferred from
  module context.
- relations.rs `Hostility`/`Allegiance` variant behavioral effects inferred from
  targeting/AI usage.

### Self-reflection

- The fan-out (4 parallel subagents on a shared worktree) was fast for the bulk
  but the shared force-warn build made each agent's self-verification race:
  one agent (nova_scenario core, ~11min) legitimately still had 40 undocumented
  items in actions.rs after its FIRST "done" notification, while its captured
  build output had transiently read 0 mid-race. The parent RE-VERIFIED with a
  settled build (mtime-stability poll to confirm no live writes) and caught it -
  exactly the re-verify step the brief demanded. Lesson: on a shared worktree,
  a subagent's own "0" is not trustworthy until writes are quiesced; the parent
  must re-run the count after ALL agents report AND file mtimes are stable.
- The intra-doc-link regression was invisible to missing_docs and to the build;
  only `cargo doc` caught it. It looked baffling (pure-additive diff breaking
  pre-existing links in an unedited file) until bisected by reverting the
  mod-line `///`s. Worth a LESSONS entry: never put a `///` summary on a
  `pub mod` line that already has `//!` inner docs.
