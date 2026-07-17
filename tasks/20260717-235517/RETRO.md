# Retro: Thruster exhaust square shape

- TASK: 20260717-235517
- LANDED: 2a18945c (squash)

## What went well

- Reading the shader FIRST (thruster_exhaust.wgsl) showed the glow is
  shape-agnostic (elongates along +Y via xz radius), so the change collapsed to
  "swap the mesh" - no shader edits, no new uniforms. Cheapest possible fix.
- Mirroring `new_cone`'s triangle template exactly made the winding provably
  correct (verified by hand AND an independent reviewer) - a square flame that
  would render inside-out was the one real risk, and copying the template killed
  it up front.
- The unit test discriminates square from cone via the (+-1,+-1) corner a cone
  can never produce - a test that actually fails if the shape regresses.

## What went wrong

- Nothing significant. One compile error from a missed struct literal
  (torpedo_section) - caught immediately by cargo check; the serde(default)
  attr does NOT make a manual-Default struct's literals optional.

## What to improve next time

- When adding a field to a config struct with a hand-written Default, grep for
  every `Struct { ` literal up front (not just Default) - serde/reflect defaults
  don't cover Rust struct-literal exhaustiveness.
- Naming nit worth heeding: `ThrusterExhaustConfig.geometry` nested under
  `ThrusterExhaust.shape` reads `shape.geometry`. If the wrapper's `shape` field
  is ever renamed to `config`, do it then; not worth churn now.
- Visual follow-up if it looks off: under thrust the square flame elongates
  edge-centers but not corners (pillow/+ cross-section). Fine for a glow.
