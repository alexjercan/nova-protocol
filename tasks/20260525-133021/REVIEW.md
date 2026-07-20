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

## Round 2

- VERDICT: APPROVE

Round 2 triggered by a user report that PN still "flies off randomly, always
thrusting, never turning toward the target even when stationary" - i.e. Round 1
approved a law that did not work from the game's real launch state. Findings and
their fixes, verified against the diff and fresh headless runs:

- [x] R2.1 (BLOCKER) [fixed in-review] velocity-anchored PN cannot recover from the real
  launch (slow, sideways out of the bay): `normalize(V + N*(Omega x V))` chases
  its own velocity when Omega is small and V points away from the target. Fixed by
  anchoring on the line of sight: constant-bearing lead + clamped PN LOS-rate
  damping. Verified: `pn_points_at_a_stationary_target_from_a_sideways_launch`
  (law) and `pn_turns_a_sideways_launch_onto_a_stationary_target` (closed loop);
  in-game 06 range now kills stationary gates in ~0.9s per shot.
- [x] R2.2 (BLOCKER) [fixed in-review] unbounded speed left the turning circle larger than
  the proximity fuze (observed 19-21u standoff at 60+ u/s, 0 detonations on a
  crossing target). Fixed with `max_speed` thrust gating on the along-nose speed
  plus `linear_damping` on the body (an along-nose gate alone gets pumped past the
  cap by turning; a total-speed gate alone leaves the torpedo ballistic - both
  variants were measured before landing on the pair). Verified: in-game 07 speeds
  hold 30-33 and the crosser is killed twice in the window;
  `thrust_tapers_to_zero_at_cruise_speed` covers the taper.
- [x] R2.3 (MAJOR) [fixed in-review] the Round 1 closed-loop tests initialized the torpedo
  already flying at the target at speed 60, so they proved the law's happy path
  and missed both defects. Replaced with a thrust-along-nose + cap + drag model
  starting from the real launch state (1 u/s sideways, nose forward).

Checks re-run: 18 torpedo tests green, clippy clean (crates + examples), no-debug
build green, 06 + 07 headless runs green with detonations as described above.
