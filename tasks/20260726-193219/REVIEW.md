# Review: NOVA OS CRT casing + glass depth pass

- TASK: 20260726-193219
- BRANCH: feature/nova-os-casing-glass

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

Clean round. The reviewer diffed `master...HEAD`, read TASK.md + NOTES.md, and
ran the DoD proof itself: `cargo test -p nova_gameplay drawer` PASSED (56 passed,
0 failed, incl. the new `drawer_monitor_has_physical_casing_details`), and
`cargo check -p nova_gameplay` compiles clean. It independently verified the
load-bearing claims: the WGSL/Rust uniform field order matches with
`corner_radius` appended last (`shader-uniform-field-order-must-match-wgsl`
respected); `BorderRadius` correctly moved into `Node.border_radius`; every
decorative overlay (screws, seam, vents, rim, glass, reflection) carries
`Pickable::IGNORE` so none can eat terminal input; ZIndex order is content(0) <
CRT overlay(1) < rim(2) < glass(3); the new test asserts new-to-branch markers,
so it would fail if the feature were reverted; no existing test was weakened
(the only removed lines are two `BorderColor::all(...)` swapped for richer
per-side colours); `spawn_drawer_shell_with_crt` correctly gained
`init_asset::<Image>()`. In-session re-derivation: the drawer suite and the
uniform field-order match were re-confirmed against the branch before adopting.

Observation (not a finding): the whole overlay stack now depends on the
`NOVA_OS_*_Z` constants staying monotonic (content < overlay < rim < glass);
current values are correct.

- [ ] R1.1 (NIT) crates/nova_gameplay/src/hud/drawer.rs:2910 - NOTES calls the
  glass reflection an "upper-left" catch, but positioned at left 6% / top 7%
  with 26% width it reads nearer centre in the AFTER capture (blending with the
  centred CRT glow). Cosmetic; falls under the owner's manual eyeball DoD.
  Optionally nudge it further into the corner or reword the note.
  - Response: Left to owner discretion; the soft radial catch reads as glass and
    the placement is within the manual eyeball item. No code change this round.

### Pending manual DoD items (owner acceptance, not resolved by this APPROVE)

- CRT overlay respects the screen rounding, no green bleed past the corner
  radius (AFTER capture inspection) - the capture supports this.
- The brand plate reads as stamped-in bottom-left (AFTER vs the PoC).
- Owner confirms the device reads as glass + moulded plastic.
