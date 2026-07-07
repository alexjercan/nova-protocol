# Audit and finalize nova_gameplay crate boundary

- STATUS: CLOSED
- PRIORITY: 100
- TAGS: v0.3.1, refactor, crates

Confirm nova_gameplay is the umbrella for gameplay-specific plugins (sections, health, weapons, objectives) not ready for bevy_common_systems. Move misplaced modules in or out as needed. [new]

## Steps

- [x] Audit every module under crates/nova_gameplay/src for game-agnostic vs
      gameplay-specific code.
- [x] Confirm no gameplay-specific code is stranded in other crates (nothing to move in).
- [x] Codify the crate boundary policy in docs/architecture.md.
- [x] Capture promotion candidates as a tracked follow-up task instead of forcing a
      premature cross-repo extraction.

## Resolution (CLOSED)

Audited all of crates/nova_gameplay/src module by module (camera_controller, hud/*,
input/*, integrity/*, sections/*, plugin.rs, lib.rs). Finding: the boundary is clean.
Every spaceship/section/weapon/input/camera module is correctly placed, and nothing
gameplay-specific lives in the wrong crate, so there is nothing to move *in*.

A few modules are game-agnostic enough to *eventually* promote to bevy_common_systems
(hud/health, hud/objectives, the direction-visualizer materials in hud/velocity, and
the blast/impact damage in integrity). But bevy_common_systems is now a separate repo,
so "moving out" is a coordinated cross-repo change, not a local file move - and the
task itself defines nova_gameplay as the home for generic-leaning code "not ready for
bevy_common_systems". So those modules legitimately stay here for now.

What changed:
- Added a "Crate boundary policy" section to docs/architecture.md that writes down the
  three-tier rule (external bevy_common_systems -> nova_gameplay -> nova_core) and the
  audit finding.
- Created follow-up task 20260706-151804 (v0.4.0) listing the concrete promotion
  candidates, so the finding is tracked rather than lost.

Alternatives considered: actually extracting the candidates now. Rejected - it is a
cross-repo change out of scope for v0.3.1, and premature while those APIs are still
game-flavored and unstable.

Self-reflection: an "audit and finalize" task with a clean result is best finished by
writing the policy down (so the boundary stays finalized) plus a tracked follow-up,
rather than by forcing a code change to justify the cycle. No code changed; the
deliverable is the documented boundary decision.
