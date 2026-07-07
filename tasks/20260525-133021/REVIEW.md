# Review: Proportional navigation guidance for torpedo

- TASK: 20260525-133021
- BRANCH: feature/torpedo-pn-guidance

## Round 1

- VERDICT: APPROVE

Delivers the Goal: the ad-hoc pursuit + drift term is replaced with vector true
PN (`a = N·(Ω × V)`), so the torpedo leads a moving target. Clean structure - a
pure `pn_steer_direction` for the math, `torpedo_pn_guidance` writing a single
`TorpedoSteering` source of truth, and thin sync (orientation) / thrust (along the
nose) consumers. `nav_constant` is config-driven; target velocity comes from the
target's `LinearVelocity` and is zero on loss, so PN degrades to pursuit of the
frozen position (consistent with the 100004/120608 fixes).

Verified independently in the worktree:

- `cargo test -p nova_gameplay torpedo`: 12/12 pass, incl. the 3 PN tests. I
  re-derived the crossing-target case by hand: target crossing to +X yields a +X
  lead component - the sign is correct (`Ω × V`, not `V × Ω`).
- `cargo clippy -p nova_gameplay -p nova_assets`: clean.
- `cargo build --example 06_torpedo_range` (no debug): green - `nav_constant` wired
  at both `TorpedoSectionConfig` construction sites (Default + `nova_assets`).
- Range autopilot smoke (Xvfb): 4 fired, 4 armed, 3 detonated, cycle complete, no
  panic (one still in flight at cutoff, expected).

Degenerate/low-speed inputs are guarded (coincident target, stationary torpedo ->
finite unit vector). The spawn tuple correctly nests the two new components to stay
within Bevy's 15-element bundle limit (caught by the compiler on the first build,
fixed).

No BLOCKER/MAJOR. Two NITs, both tuning observations, no change requested.

- [ ] R1.1 (NIT) `nav_constant` default (3.0) and the guidance/controller coupling
  are feel-tuning: PN only achieves the intercept if the PD controller
  (`max_torque`) can turn the nose onto the commanded heading fast enough at the
  closing speeds involved. Best validated interactively in `06_torpedo_range`
  against the moving gate; the range is built for exactly this. No code change.
  - Response:
- [ ] R1.2 (NIT) The low-speed fallback threshold (`1e-4` on velocity length²,
  ~0.01 u/s) is very permissive, so PN runs almost immediately after spawn where
  the velocity direction is barely established. It works (the steering direction is
  scale-independent in `|V|`), but if early-flight wobble is ever observed, raising
  the threshold to hand off from pursuit to PN at a more meaningful speed is the
  knob. Not needed now.
  - Response:
