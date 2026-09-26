# Reproduce and diagnose violent docking capture

- STATUS: OPEN
- PRIORITY: 100
- TAGS: v0.15.0, gameplay, docking, bug

## User facts

- On master and other branches, docking sometimes makes the connected ships collide violently, fly apart and die. The owner cannot yet give a reliable reproduction.
- Investigate by running a small set of agent-driven `nova-bench` docking approaches, including the open-world Line Warship and a dockable derelict; try near the capture envelope's edge. Preserve failures and their replay artifacts.
- Do not assume this is a `FixedJoint` defect or implement a speculative physics fix.

## Agent findings (unverified cause)

- `crates/nova_ship/src/sections/docking_section/connection.rs` selects port pairs and spawns a `FixedJoint` from the current face midpoint and ship basis; `port.rs` owns the capture gates. A joint and the collision solver may interact at capture, but the reported blast has not been reproduced or attributed.
- `examples/systems/system_docking_ports.rs` proves controlled joint capture and helm handoff, not edge-case geometry, real multi-section colliders, or an intermittent high-energy event.
- `crates/nova_bench/scenarios/docking.content.ron` is a loose, hand-built tender/spar fixture and `crates/nova_bench/src/pages/docking.md` explains the shipped face-gap, facing and relative-motion gates. Open-world catalog ships include `block_line_warship` and a damaged frame tender with a docking collar; identify the actual IDs and intact collar before authoring any focused fixture.

## Delivery

1. Scout port selection, capture constraints, collision filtering, joint frames, update ordering, body velocity and the damage/destruction path. Record exact ownership and observations; do not edit gameplay while diagnosing.
2. Establish a clean master reference and run at least two distinct bounded docking approaches, first with the existing bench fixture, then with a disposable loose scenario using the actual Line Warship and a dockable generated-derelict design. Include near-boundary gap, facing, relative speed and spin if the fixture can express them. Keep artificial station props, combat and gravity out of the trigger unless evidence requires them.
3. Preserve seeds, model, budgets, action audits, score, game logs and exact replay for any failing run; separate failed approaches and pilot mistakes from accepted captures that explode.
4. If reproduced, propose a code-backed minimal fix with before/after graph, failure policy, and persistent regression proof. Get owner approval before adding runtime types, functions or tests. If not reproduced, record negative cases and what remains untested; do not claim the bug is fixed.

## Verification

- Show accepted port-pair metrics and joint creation, both roots' velocity/pose and damage before/after, and whether contact pairs overlap or impulses diverge. A clean exit or pilot prose is not proof.
- For a failure, replay the first audit with a stable seed and compare the same failure on master. If a fix is later approved, prove the original failure is absent without weakening capture eligibility and controlled-docking invariants.
- Run one game/GPU recording at a time and do not compete with the currently active heavy verification lane. No broad workspace checks.

## Done when

- The violent-capture failure is reproduced and the approved fix plus permanent regression proof pass, OR the documented bounded bench search records no reproduction with explicit remaining risk and a user-accepted follow-up. Keep the task OPEN until then.
