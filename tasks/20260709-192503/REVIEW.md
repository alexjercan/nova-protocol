# Review: Hybrid lock acquisition (aim cone + signature range)

- TASK: 20260709-192503
- BRANCH: feature/signature-acquisition (commits 4bec616, 1ca888a, ebac6f5)

## Round 1

- VERDICT: APPROVE

Verified independently: fmt clean, `cargo check --workspace` green, 12
targeting tests + 20 input + 35 hud tests pass. The three-commit split
(mechanical move / mechanical rename / behavior) made each diff reviewable
in isolation, exactly as the plan asked. The move is faithful (pick_target
and the system body unchanged textually); the torpedo commit's `.after(
SpaceshipTargetingSystems)` preserves the old same-module chain semantics.
The fallback is direction-blind by design and hostile-gated, so the
asteroid/torpedo non-acquisition invariants hold (tested); a hostile both in
cone and in range resolves to the cone pick (tested). The old resource name
carried the TODO from closed task 20260706-162913; this rename+move is that
refactor, and the Resolution says so.

- [x] R1.1 (MINOR) docs/retros/20260709-screen-indicator-widget.md - the living
  widget doc still names `SpaceshipPlayerTorpedoTargetEntity` in its
  consumer descriptions; the type no longer exists. Update the name (the
  dated spike docs are historical decision records and stay as written).
  - Response: fixed in 70fc8d5 - widget doc renamed; spike docs left as historical records.

## Round 2

- VERDICT: APPROVE

R1.1 verified; no code changes, docs only. No new findings.
