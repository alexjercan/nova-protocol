# Give a defeated New Game player a safe Load last save action

- STATUS: CLOSED
- PRIORITY: 88
- TAGS: v0.16.0,gameplay,ui

## User facts
- After the player dies in a saved New Game, offer a way to reload the last valid save. Do not overwrite that save on death.
- This task authorizes planning only. Implement after an exact code-backed design gate and approval.

## Agent findings
- The open-world bootstrap has no defeat outcome (`crates/nova_authoring/src/base_content/scenarios/open_world.rs:36-53`). Player removal disarms the world and spends its save session (`crates/nova_world_base/src/lib.rs:252-260`; `crates/nova_world_base/src/save/session.rs:290-331`). Leaving then yields `SAVE FAILED` although the last save remains intact (`crates/nova_menu/src/leave.rs:193-197`). Source-backed, not a reproduced player-facing run.
- `crates/nova_menu/src/leave.rs:118-171` already implements a no-write Load last save route. The old intermittent docking report is resolved; do not bundle it.
- Research and proof plan: `tasks/20261007-090722/RESEARCH.md` candidate 1.

## Delivery gate
- Reproduce death -> visible state -> Esc/leave with an isolated named world and inspect the final frame and preserved disk save. Define who owns the defeat prompt, default focus, actions and their labels for native saved worlds and web seed-only worlds. Define whether death leave bypasses save failure or changes the failure wording, and preserve genuine I/O failures. Specify the before/after flow and implementation gate before code edits.
- Remove the unprompted dead-world recovery gap if reproduced; preserve last good save, Load refusal and explicit Retry semantics. No silent fallback or overwrite on death.

## Verification
- Assert death never writes a new generation; Load last save reopens the previous player/sector state; menu/exit works after death without mislabelling expected no-save behavior as a disk failure. Verify actual write failures still fail visibly. Capture one rendered native death-to-reload path and check web's seed-only route separately.

## Done when
- Reproduced symptom, approved UX/error contract, focused state/save tests, rendered recovery flow, and explicit evidence limits are recorded. No implementation is authorized by this task record alone.
