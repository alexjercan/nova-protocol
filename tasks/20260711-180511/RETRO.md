# Retro: Settings menu content (graphics quality, keybinds, audio)

- TASK: 20260711-180511
- LANDED: 0fceba7c (squash)
- REVIEW ROUNDS: 2 (R1 REQUEST_CHANGES -> R2 APPROVE)

Process notes only; what/why/evidence live in TASK.md, findings in REVIEW.md.

## What went well

- Two parallel Explore agents mapped the two disjoint subsystems (menu UI vs
  audio/input/graphics backends) BEFORE any design, so the plan cited real
  file:line facts (GlobalVolume path, mod_prefs stack, the flight rig, no
  existing low-end mode) instead of guesses - verify-first-plan-steps applied
  preventively.
- Reused known-good stacks throughout: `mod_prefs` for cross-platform
  persistence, bevy's headless `ui_widgets::Slider`, the `ButtonValue`/`Selected`
  selection machinery, and example 14's autopilot harness to screenshot the
  live panel. Almost no new infrastructure.
- The rendered-panel eyeball (throwaway example driving the shipped app) was
  cheap and paid twice: confirmed the first layout, then re-confirmed the
  slider after the user's mid-flow control swap.
- The out-of-context review earned its keep: it found the write-amplification
  MAJOR that shared-session eyes had rationalized as fine.

## What went wrong (root causes)

- R1.1 (config file rewritten every drag frame): persistence was designed as
  "save whenever the resource changes", which was SAFE for the discrete
  segmented control I built first (one press = one change = one save). When the
  user asked for a slider mid-flow I swapped the control but did not revisit the
  save policy the discrete design had made safe - a continuous input fires a
  ValueChange per frame, so per-change saves became per-frame disk writes. Root
  cause: changing a control from discrete to continuous without re-auditing the
  systems that relied on its discreteness.
- R1.2 (parity test weaker than advertised): the keybind test asserted "rig
  binds KeyW" and "KEYBINDS has a Main Drive row" as two INDEPENDENT facts and
  called it parity; it never asserted the displayed string against the rig's
  key. Root cause: a "keep X in sync with Y" test that checks each side against
  a hardcoded literal instead of deriving one side from the other.
- Volume-control churn: I deliberated segmented-vs-slider at length and chose
  segmented unilaterally for "robustness/reuse"; the user immediately wanted a
  slider. I surfaced the PERSISTENCE scope via AskUserQuestion but not the
  control STYLE. Root cause: treating a user-facing interaction-style preference
  as an internal implementation call.

## What to improve next time

- When a control's input model changes (discrete -> continuous), re-audit every
  downstream consumer of the old model (persistence, change-detection, event
  counts) - do not assume a policy written for the old model still holds.
- A parity/sync test must cross-link: derive the expected value from source A
  and assert it against artifact B; never assert both against a constant.
- When I catch myself deliberating a user-facing control's STYLE (slider vs
  stepper vs segmented) at length, that is the signal to ask, not decide -
  the same discipline I already applied to persistence scope.

## Action items

- [x] Graphics-preset seam for the low-end mode recorded on tasks/20260525-133013
      (apply_graphics_quality is the hook; extend with particle/scatter gating).
- [x] Lessons appended to LESSONS.md (two new, three bumped).
- No follow-up code task: the deferred persistence-test NIT is documented in
  REVIEW.md R1.4 with rationale; not worth an injection layer.
