# RCS HUD indication on the velocity sphere (active palette + cap ring)

- STATUS: CLOSED
- PRIORITY: 3
- TAGS: v0.7.0, feature, hud, spike

## Goal

Diegetic indication that RCS fine-adjust mode is active, on the velocity sphere
that already orbits the player and shows speed/gravity (hud/velocity.rs):

- Give the sphere an RCS-active state with a distinct palette, reusing the
  existing autopilot-presence palette switch (manual white/blue -> engaged cyan)
  as the pattern.
- Optionally render the `rcs_cap` as a bounding ring/shell so the pilot can see
  the small speed ceiling their nudges settle at.
- Active when the ship is in RCS mode (SHIFT held / `RcsIntent` present).

## Scope decision (from planning)

- **Deliver the active palette now** (the primary diegetic "RCS is on" cue). It
  mirrors the proven autopilot-presence palette switch exactly, is testable
  headless, and is low-risk.
- **Split the cap ring into a follow-up** (seeded as a new tatr task). A "cap
  ring/shell" on a FIXED-radius shader sphere (the sphere's size is constant;
  speed drives a shader magnitude, not the physical radius) has genuinely
  underspecified visual semantics, is new geometry/shader work, and its "does it
  read right" cannot be verified headless - it needs a design pass + a playtest.
  Shipping unverifiable visual geometry here would be dishonest; the palette
  alone delivers the core indication.

## Steps

- [x] Add a distinct `VelocityHudPalette::RCS_ACTIVE` const in
  `crates/nova_gameplay/src/hud/velocity.rs` (near `ENGAGED`, velocity.rs:~74).
  Pick a hue clearly distinct from manual blue, autopilot cyan, and gravity
  yellow/orange - a violet/purple (e.g. `indicator srgba(0.72, 0.45, 1.0, 1.0)`,
  `sphere srgba(0.72, 0.45, 1.0, 0.2)`), matching the alpha convention of the
  other palettes.
- [x] Change `desired_velocity_palette(engaged: bool)` (velocity.rs:~79) to
  `desired_velocity_palette(engaged: bool, rcs_active: bool)` returning
  `RCS_ACTIVE` when `rcs_active`, else the existing engaged/default logic. RCS
  takes precedence, though in practice `RcsActive` and `Autopilot` are mutually
  exclusive on the player ship (entering RCS removes the autopilot). Keep it a
  pure fn for the unit test.
- [x] Update `sync_engaged_palette` (velocity.rs:358) to also read the target
  ship's `RcsActive` presence: add a `q_rcs: Query<(), With<RcsActive>>` (or
  fold into an existing target query) and pass `q_rcs.get(**target).is_ok()` to
  `desired_velocity_palette`. `RcsActive` is in `flight::prelude` (already
  imported at velocity.rs:18). The material-push logic (velocity.rs:393-404) is
  unchanged. Key on `RcsActive` (the player SHIFT modal), NOT `RcsIntent`: when
  the autopilot later drives `RcsIntent` it should still read as ENGAGED (the
  computer is flying), which keying on `RcsActive` gives for free.
- [x] Rename the system if "engaged" no longer fits (optional; `sync_engaged_palette`
  -> `sync_velocity_palette` reads better now that it covers RCS too - only if
  it does not sprawl the diff).
- [x] Unit-test the pure fn: `desired_velocity_palette(false,false)=default`,
  `(true,false)=ENGAGED`, `(_,true)=RCS_ACTIVE`.
- [x] Integration test mirroring `velocity_palette_follows_the_autopilot`
  (velocity.rs:~660): `velocity_palette_follows_rcs_active` - spawn a target with
  `RcsActive`, run `sync_engaged_palette`, assert the widget palette is
  `RCS_ACTIVE`; remove `RcsActive`, run again, assert it reverts to default; and
  assert an autopilot-engaged (no RcsActive) target still reads `ENGAGED`.
- [x] Seed the cap-ring follow-up as a new tatr task (v0.7.0, hud, spike; lower
  priority) with a note that it needs a visual-design pass + playtest, and record
  the split in `tasks/20260718-122923/NOTES.md` and the spike Fix record.

## Notes

Spike: tasks/20260718-122508/SPIKE.md. Depends on the RCS core primitive
(task 20260718-122906, CLOSED) - reads `RcsActive` (in `flight::prelude`).

Reference points verified during planning (via exploration):
- Palette switch pattern to mirror: `sync_engaged_palette` velocity.rs:358-406,
  helper `desired_velocity_palette` velocity.rs:79-85.
- Palettes: `VelocityHudPalette` (default manual, `GRAVITY`, `ENGAGED`)
  velocity.rs:40-74; color set via `material.base.base_color` (velocity.rs:396,401).
- Target resolution: `VelocityHudTargetEntity` velocity.rs:143; `RcsActive` etc.
  reachable via `flight::prelude` (velocity.rs:18).
- System registration: velocity.rs:172-181 (NovaHudSystems).
- Test harness: `palette_world`/`spawn_widget`/`palette_of` +
  `velocity_palette_follows_the_autopilot` velocity.rs:~650-680.
- Cap-ring reference (for the follow-up): torus pattern in
  hud/maneuver_instruments.rs; sphere magnitude scale is `speed / 100`
  (direction_shader_update_system velocity.rs:282-349).

Lessons applied: `changed-shared-observer-run-the-module-suites` (run the
`hud::` suite, not just the new test), `piped-cargo-masks-exit-code` (print the
tail, do not pipe cargo through grep).
