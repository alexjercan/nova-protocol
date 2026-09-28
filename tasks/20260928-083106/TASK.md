# Prioritize combat and travel lock in shared target inset

- STATUS: CLOSED
- PRIORITY: 75
- TAGS: hud, targeting

Design and implement one corner RTT inset for both travel and combat lock. Combat lock takes priority; clearing it falls back to travel lock without changing either lock. Share target name, range, signed closing speed (m/s), and honest no-signal handling. Travel shows neutral details; combat adds combat details and preserves kill-camera semantics. Scope data ownership, target validity, HUD mode, existing call sites, proofs and rendered comparison before code edits. No runtime work until owner approves exact code-backed gate. Isolated branch; no merge without request.

## Delivery (PR #86 merged into master as 0d0c0e64f; Sprout and branches removed)

- `inset_subject` picks a live combat target, else a live travel target. `InsetSlotType` marks the slot.
- Combat framable > running kill cam > confirmed-destruction kill cam start > subject (Live / NO-SIGNAL / Hidden). A kill cam that expires falls to the subject in the same frame.
- Caption: `NAME[ - TAG]\nDST ..  CLS ..`. Combat uses the relation color. Travel uses quiet steel. The caption is blank while a kill cam runs.
- Order: `drive_inset_camera -> ApplyDeferred -> (drive_inset_frame_state, show_confirmed_destruction)`.
- Tests: priority/fallback, non-zoomable NO-SIGNAL over framable travel (replaces the old non-zoomable test), kill-cam hold before travel.
- Frames: `frames/inset_{shot,travel,killcam}.png`, report `FRAMES.md`. Review: `REVIEW.md`.
- Not verified: NO-SIGNAL frame render, 2x-scale render, per-frame cost. `CLS -0.0 m/s` is the existing signed-zero output of the shared formatter.
- PR #86 passed all seven CI checks. The review follow-up added the travel-lock state to the web switchboard before merge.
