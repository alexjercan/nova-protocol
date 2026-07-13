# Review: Disabled-in-place controller still torques toward its frozen command

- TASK: 20260709-155922
- BRANCH: fix/disabled-controller-torque

## Round 1

- VERDICT: APPROVE

Diff reviewed against master (`0d5eb88`): a one-line `Without<SectionInactiveMarker>`
filter on `sync_controller_section_forces` plus two flight physics tests.

Verifications performed (not taken on trust):

- **The fix is the complete seam.** `PDControllerOutput` has exactly one
  consumer in nova - `sync_controller_section_forces` - and bcs only writes it
  (the PD system computes into `&mut PDControllerOutput`). So gating the apply is
  sufficient; the "no bcs change needed" claim in TASK.md holds.
- **Both disable paths stop the torque.** A non-leaf disabled controller gets
  `SectionInactiveMarker` (now filtered out); a leaf disabled controller is
  despawned by the integrity core, so it has no output to apply. No hole.
- **The regression test genuinely catches the bug.** Reverted the filter and ran
  `a_disabled_controller_leaves_the_spin_untouched` - it FAILS without the fix
  (flight.rs:1491) and passes with it. Not vacuous. The paired
  `a_live_controller_damps_an_imposed_spin` is a real control case (the same
  imposed spin is damped when the controller is live), and the symmetric-top
  geometry makes the torque-free spin genuinely constant, so the tolerance is
  sound rather than luck.
- **No collateral.** Full flight module 30/30, controller module included; no
  existing test weakened or deleted; `cargo fmt --check` clean.
- Design matches the established pattern (the sibling
  `update_controller_section_rotation_input` and the flight systems already carry
  the same filter). The inline comment explains why accurately.

- [x] R1.1 (NIT) docs/ - the COM and overkill fixes each got a
  `docs/2026-07-09-*.md` note; this fix has only the inline comment, TASK.md
  Resolution, and the (pending) retro. A one-line filter arguably does not need a
  standalone doc, but add a short one if you want to keep the per-fix docs
  convention uniform. Non-blocking.
  - Response: Added tasks/20260709-155922/NOTES.md (what changed,
    why nova-side only, and the verification) to keep the per-fix convention
    uniform.

Step 3 (manual 11_com_range feel check) was consciously substituted with the
deterministic physics test and documented as such in TASK.md - accepted: the
pipeline-level test directly exercises the disable path and is a stronger,
non-manual regression net than modifying the example.
