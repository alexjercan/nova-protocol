# Decision: Nova owns autopilot tooling until reuse proves otherwise

- DATE: 20260802-120145
- STATUS: ACCEPTED
- TASK: 20260802-115955
- TAGS: decision, autopilot, tooling, crates

## Context

Nova's automation began as a generic BCS debug harness, but Nova now layers
example scripts, probe timelines, invariants, profiling capture, screenshot
reels, and web packaging on it. Keeping BCS and Nova pins synchronized slows
Nova-specific changes. The owner explicitly requested a dedicated
`nova_autopilot` crate and a Nova-first design for v0.10.0.

## Decision

Create `nova_autopilot` inside this workspace. Move the automation driver and
the completion contract Nova needs into it, then evolve the API around Nova's
real gameplay, capture, and probe consumers. Do not make corresponding BCS
changes during this release. Consider extraction only after another project has
a concrete compatible use.

## Alternatives considered

- Continue extending BCS first, then bump Nova. This preserves generic reuse but
  retains the synchronization cost and forces Nova requirements through a
  generic design before they are understood.
- Keep thin Nova wrappers over the BCS implementation. This leaves ownership and
  completion semantics split across repositories, which is the problem the
  release intends to remove.
- Duplicate only new Nova features while keeping the driver in BCS. This creates
  two coupled harness layers and makes failures harder to locate.
- Defer automation changes. This blocks the showcase, capture, regression, and
  profiling pipeline that defines v0.10.0.

## Consequences

- Nova can change scripts, checkpoints, probe integration, and capture behavior
  in one repository and one task flow.
- BCS remains unchanged and may later diverge from Nova's API.
- Nova owns maintenance, docs, tests, and migration of its environment contract.
- A future extraction must be evidence-driven and may require reconciling the
  evolved Nova API with BCS compatibility needs.
