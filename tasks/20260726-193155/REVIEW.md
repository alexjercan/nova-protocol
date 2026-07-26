# Review: NOVA OS CRT scanline + grain realism pass

- TASK: 20260726-193155
- BRANCH: feat/nova-os-crt-scanline-grain

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

The out-of-context reviewer independently ran every DoD proof and verified the
one high-risk item - the Rust/WGSL uniform struct layout after inserting
`resolution: vec2` between `tint: vec4` and the f32 scalars (offsets line up, no
padding hole, no silent corruption). In-session re-verification: I had already
run `cargo test -p nova_gameplay drawer` (47 passed incl. the new wiring test),
`cargo fmt --check`, `cargo check`, the three greps, and captured + inspected the
AFTER render; the reviewer reached the same pass results and I re-confirmed the
field-order match myself.

Command results (both sessions): greps PASS; `cargo test -p nova_gameplay drawer`
-> `47 passed; 0 failed`; `cargo fmt --check` clean; `cargo check` exit 0.

Pending user check (manual DoD): confirm the AFTER capture
(`shots/nova-os-active.png` / `nova-os-welcome.png`) shows softer, resolution-aware
scanlines and a livelier-but-subtle grain with crisp text.

No BLOCKER/MAJOR/MINOR. Two NITs, both no-action (recorded for the record):

- NIT: the `res_y/res_x` fallback triggers on `resolution > 1.0` rather than
  `> 0.0`; harmless (arguably safer against a sub-pixel glitch), comment intent
  "zero before layout" is a hair looser than the code.
- NIT: because the fine grain is now interpolated, `spark = step(0.992, fine)`
  cross-fades in/out over the blend instead of hard-blinking - a reasonable,
  on-goal side effect of the analog shimmer.
