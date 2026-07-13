# Retro: Target inset view (RTT scope of the locked ship)

- TASK: 20260710-104421
- BRANCH: feature/target-inset-view (landed b2f2131)
- REVIEW ROUNDS: 2 (round 1 APPROVE + one MINOR; round 2 APPROVE after the fix)

Process notes only; what/why/evidence live in TASK.md and
tasks/20260710-104421/NOTES.md.

## What went well

- Probe-first paid off three times over. The spike demanded an RTT de-risk
  probe for one question (does a second camera black out the scene on 0.19).
  The probe answered that AND surfaced two things nobody had listed: the debug
  inspector egui bleeding into the RTT camera, and `BCS_SHOT` capturing a black
  frame. Both would otherwise have shown up later as confusing verification
  noise. A real runtime probe is worth more than its stated question.
- Grounding the API against the engine's own source, not memory, caught a plan
  error before any code: the plan wrote the 0.19 RTT API as `Camera { target }`;
  it is actually a standalone `RenderTarget` component (bevy 3d/render_to_texture
  example). One grep, zero rework.
- Independent visual re-derivation in review (the shared-session blind-spot
  guard) earned its keep: the unit tests prove the highlight ENTITY reconciles,
  not that it RENDERS. Cropping the inset region of a live capture confirmed the
  emissive shell actually draws - a claim the test suite could not make.
- The MINOR (inset kept rendering while the HUD hid chrome) was cheap and real,
  so it was fixed in-cycle with a delivery-guarded test rather than shipped as
  "discretionary".

## What went wrong

- The plan's `Camera { target }` API detail was stated as fact inside an
  otherwise verify-first step. The RTT approach was correctly gated behind a
  probe, but the specific constructor was asserted from a model of the API, not
  the API. Root cause: the same `verify-first-plan-steps` failure mode, now in
  its API-constructor variant - a concrete API shape is a mechanism claim and
  must cite the source or be phrased "confirm X".
- Environment friction cost time up front: a fresh sprout worktree does not
  share the main checkout's 290 GB `target/`, so a naive build would have
  rebuilt Bevy from scratch. Worked around with
  `CARGO_TARGET_DIR=<main>/target`. Also the session's shell cwd resets to the
  main checkout after every command, so every worktree command needed an
  absolute path or `cd <wt> && ...`. Neither is a defect, but both are
  predictable and worth knowing before starting.

## What to improve next time

- When a plan step names a concrete API constructor/signature (not just "use
  RTT"), treat it like any mechanism claim: cite the example/source or phrase
  it verify-first. "Spawn `Camera { target: ... }`" should have been "confirm
  the 0.19 RTT spawn shape against render_to_texture, then...".
- For headless visual checks of a loaded scene in this repo, skip `BCS_SHOT`
  (force-advances to Playing and captures before async assets load -> black).
  Inject a `Screenshot::primary_window` from the autopilot script at a settled
  moment (~+2 s) instead; that captures a real frame. Reusable for any HUD/VFX
  verification.
- For worktree work in this repo, set `CARGO_TARGET_DIR` to the main checkout's
  `target/` from the start to reuse the build cache.

## Action items

- [x] tatr 20260712-201603 filed: fix bcs `InspectorDebugPlugin` to assign
  `PrimaryEguiContext` only to window cameras (root fix for the egui bleed;
  nova carries a local workaround until then).
- [x] Lessons ledger updated (below).
