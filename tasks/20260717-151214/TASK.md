# Rust Tally turrets share one rotation - each base should face the hull, not both point down

- STATUS: CLOSED
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

Verified at plan time: the gunship's turrets sit on the +-X FLANKS
(turret_dorsal (1,0,0), turret_ventral (-1,0,-1)) despite their
top/bottom names, both with the nose-mount roll Rx(-90) that points the
mount base (local -Y, verified from the GLB in 20260717-151208's review)
toward +Z - correct for the player's bow turret, wrong for side mounts.
The tubes at (+-1,0,-2) carry identity rotation (base pointing down) AND
their port/starboard ids are swapped (ship-local forward is -Z, up +Y =>
starboard is +X; "tube_port" sits at +1). Rz(-90) seats a +X mount's
base against the spine and turns a bay hatch outboard; Rz(+90) is the -X
mirror; a bay's launch/spawn offset is local -Z, unchanged by Rz rolls.

## Steps

- [x] crates/nova_assets/src/scenario/broadside.rs gunship(): the turret
  and tube helpers take a rotation; starboard (+X) mounts get
  Quat::from_rotation_z(-FRAC_PI_2), port (-X) mounts get
  Quat::from_rotation_z(FRAC_PI_2); rename the four section ids to
  match reality (turret_starboard, turret_port, tube_starboard at +X ->
  wait: starboard IS +X - the ids swap so the +X tube is named
  tube_starboard and the -X tube tube_port); comment the mount-roll rule.
- [x] Sweep the old ids repo-wide before regenerating (rename-id-sweep +
  sweep-then-delete): grep turret_dorsal/turret_ventral/tube_port/
  tube_starboard across crates/, examples/, webmods/, docs/, wiki - only
  the builder and the generated RON hold them (verified above; re-grep
  after the edit).
- [x] gen_content regenerate; parity + bundle-uniformity green; run it
  twice (generator stability).
- [x] Sweep the OTHER ships for the same pattern and report: the Auditor
  keeps a bow-mounted turret misnamed "turret_dorsal" (cosmetic, its
  Rx(-90) roll is CORRECT for a bow mount - report, do not churn the
  ledger bundle again); ledger magpies/corvettes/players are all bow
  mounts (correct rolls).
- [x] Docs: CHANGELOG (Fixes); NOTES.md with the axis derivations and
  the port/starboard convention; note that no automated test models
  mount rotation (the parity test pins builder->RON; the geometric
  correctness is derivation + review re-derivation, as the bay cycle
  did).
- [x] Verify: content_lint, balance_audit (ids unreferenced by acks/
  audit - hostile keying is the ship id), broadside_assault (11 tests -
  the tubes test counts prototypes, not ids), 19_broadside example
  compiles; cargo check --workspace --all-targets; fmt last.

## Close-out record

All six steps landed; the axis math, the id-swap discovery and the
out-of-scope Auditor naming note are in NOTES.md. Verification:
gen_content stable across two runs, content_ron_parity 2/2,
broadside_assault 11/11, content_lint clean, balance_audit 0 errors / 0
warnings / 2 acked, workspace --all-targets green, fmt last. Full suite
on CI per standing instruction.
