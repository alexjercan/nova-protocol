# Redesign Autopilot pacing and probe contracts

- STATUS: OPEN
- PRIORITY: 45
- TAGS: v0.13.0,autopilot,probe

## Goal

Redesign Nova Autopilot pacing so walks use explicit state conditions by default and retain frame counts only when frame work is the real contract. Improve the API, probe surfaces, diagnostics, and examples, then retire the generic `frames` predicate if the resulting vocabulary makes it obsolete.

Scheduled into v0.13.0 (2026-08-31 planning round) as the cycle's internal
work.

## Context

A follow-up audit of task 20260824-011329 found 115 `frames(..)` call sites across 35 files, not only the autopilot example. The sites split into four groups:

- 26 tautological `frames(1)` waits.
- 43 pre-shot/render stillness waits.
- 26 gesture settles in `widget_zoo`, `system_nova_os`, and `pointer_pin`.
- 20 intentional frame counts for recording length, shader or raster synchronization, and deliberately weaker pre-assert gates.

The audit showed that deleting the helper mechanically would hide real contracts or replace deterministic work with wall-clock sleeps. The redesign must model those contracts honestly.

## Scope

- Re-audit the current tree from landed master before design work. Preserve exact call-site evidence with this task.
- Define clear Autopilot vocabulary for:
  - state and event conditions;
  - input press and release acknowledgement;
  - renderer or capture readiness;
  - deterministic media duration in frames;
  - deliberately weaker synchronization gates that keep later assertions meaningful.
- Add narrow public, read-only probe surfaces for the widget zoo, Nova OS raster/input flow, and pointer integration rig where state is currently hidden behind gesture settles.
- Investigate an observable render/capture readiness contract. Do not replace frame work with elapsed-time sleeps.
- Remove tautological waits and migrate each accidental frame-count call site to the narrowest real condition.
- Keep intentional recording-duration or assertion-separation counts explicit and named. If a generic `frames` predicate remains useful, document its constrained role; otherwise remove its definition, exports, tests, and all imports.
- Improve deadline failure messages so a failed beat identifies the expected state and relevant observed probe state.
- Update examples, automation-harness documentation, changelog, and task proof together.

## Constraints

- Preserve deterministic capture lengths and rendered output.
- Do not gate an assertion on the exact invariant it is intended to prove.
- Do not add sleeps or rename frame counts without changing their semantics.
- Keep subsystem probes read-only and avoid dependency cycles.
- Keep outcome slugs stable unless an intentional contract change is documented and reviewed.

## Done when

- Every remaining pacing call site is classified and uses vocabulary that states its real contract.
- Widget zoo, Nova OS, and pointer gesture walks wait on public state or event acknowledgement instead of arbitrary settle counts.
- Capture readiness has a tested observable contract, or the task records a proof-bearing decision that explicit frame work is required for specific sites.
- Tautological waits are gone.
- Intentional frame-duration and deliberately weak gates remain deterministic and are documented.
- The generic `frames` predicate is removed if no honest generic use remains; otherwise its reduced public contract and retained sites are explicitly justified.
- Focused unit tests and affected software-rendered probe ranges pass through `nix develop`.
- Generated reports and representative still/video artifacts are inspected, with commit-bound evidence kept under this task.
