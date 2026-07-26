# Review: Spike - NOVA OS CRT monitor look and feel

- TASK: 20260726-193040
- BRANCH: (none - research spike on master, no code)

## Round 1

- VERDICT: APPROVE
- REVIEWER: in-session (research spike; no code diff - trivial-diff carve-out)

The deliverable is `SPIKE.md` plus three seeded direction-level tasks; there is
no code to run a check suite against. Reviewed the doc for the things a spike
must get right:

- Grounded in the real artifacts: the two hard constraints (UI materials cannot
  sample the content behind them; the UI blit `Camera2d` has no HDR/Bloom and
  must not get one) are verified against `assets/shaders/nova_os_crt.wgsl` and
  `crates/nova_scenario/src/render_scale.rs`, not assumed.
- Diverged before converging: four options (A RTT pipeline, B casing/glass, C
  scanline/grain, D do-nothing) with honest pros/cons, not a single guess.
- The recommendation (C + B now, A as the headline stretch, D kept honest) is
  concrete enough to plan without re-litigating, and names the deciding unknowns
  as open questions to resolve in the RTT task before committing.
- Seeded task IDs in "Next steps" match the created tasks
  (20260726-193155 / -193219 / -193233); each is coarse/direction-level with a
  `Spike:` backlink and `spike` tag, leaving Steps to `/plan`.

No BLOCKER/MAJOR. The spike correctly does not start implementation.
