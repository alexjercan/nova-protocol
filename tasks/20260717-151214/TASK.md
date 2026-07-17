# Rust Tally turrets share one rotation - each base should face the hull, not both point down

- STATUS: OPEN
- PRIORITY: 42
- TAGS: v0.7.0,scenario,content,bug

User report (2026-07-17 playtest): Rust Tally (broadside_gunship, base
campaign finale) "turrets have bad rotation - I think they both had the
bottom down, they should have the bottom towards the ship." In the
builder (crates/nova_assets/src/scenario/broadside.rs, gunship()) both
turret sections get the same Quat::from_rotation_x(-FRAC_PI_2) while
sitting on opposite sides of the spine (offsets +X and -X) - so one
turret's base faces away from the hull. Work out the turret section's
local orientation (which local axis is the mount base) from the render
code / gltf mounting in turret_section.rs, then rotate each mount so its
base faces the hull block it is attached to. Builder-generated content:
edit the builder, run gen_content, parity test guards. Also sweep the
player/other ships' turret rotations for the same pattern and report
(fix only the Rust Tally here unless trivial).
