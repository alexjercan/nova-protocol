# Review: HUD text anchored to moving objects twitches

- TASK: 20260710-231928
- BRANCH: fix/hud-projection-postupdate

## Round 1

- VERDICT: APPROVE

Verified against the spec and the spike doc:

- The PostUpdate slot is correct and the reasoning checks out against the
  actual dependency sources: bevy_ui 0.19 orders `UiSystems::Layout`
  before `TransformSystems::Propagate` (verified in the registry source),
  so the TransformHelper-composed poses are the only way to get this
  frame's camera + anchor poses into this frame's layout. The plan's
  original "after Propagate" slot was correctly rejected during
  implementation and the step rewritten (work-skill rule followed).
- The bcs `ChaseCameraSystems::Sync` vs `TransformSystems::Propagate`
  ambiguity is real (verified: bcs adds the systems with no constraint)
  and the additive `configure_sets` pin from nova's camera controller is
  a legitimate, push-free fix. The 12_hud_range example builds with the
  full game plugins, so the pin covers it too.
- The regression is behavioral, A/B-proven (54 px worst-case mismatch on
  the old Update schedule vs sub-pixel after), and carries delivery
  guards (camera must actually trail; indicator must be visible) per the
  231931 retro lesson - no null-assertion without stimulus proof.
- Holo drivers' render-clock ship reads are correct and properly scoped:
  wells stay raw (static, clocks agree - documented in code), plan
  geometry stays plan geometry. The added `Without<...>` filters are
  borrow-disjointness, not semantics.
- Dead `.before(ScreenIndicatorSystems)` constraints removed in all six
  driver files; reviewer confirmed every one of those registrations is in
  Update, where schedule order now provides the guarantee.
- Reviewer ran: full nova_gameplay lib suite 355/355; ASCII hygiene clean;
  implementer's workspace all-targets check accepted (12_hud_range's
  BEHAVIORAL script runs in CI's example smoke - its assertions recompute
  projections from post-frame poses, which the new placement matches more
  exactly, not less).
- TASK.md honest, including the two design assumptions the plan got wrong
  and how they were caught.

No findings. Clean branch; the family's render-clock story is now
symmetric with the raw-clock story from 20260711-103527/231930.
