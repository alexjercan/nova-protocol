# Rust Tally mount rolls - design record

Task 20260717-151214. User report: the gunship's turrets "both had the
bottom down, they should have the bottom towards the ship".

## The geometry

A section's mount base is its local -Y (established from the GLB vertex
data in 20260717-151208's review; the bay hatch and the turret turntable
sit at +Y). The gunship's four side mounts sat on the +-X flanks but
carried spine-end rolls: both turrets had the player-ship bow roll
Rx(-90) (base toward +Z - correct only for a mount at the END of the
spine) and both tubes identity (base straight down). Fix: starboard (+X)
mounts roll Rz(-90) (-Y -> -X, base to hull; +Y hatch/turntable
outboard), port (-X) mounts roll Rz(+90). What Rz leaves untouched is the
SPAWN OFFSET (local (0,0,-2), still ship-forward: spawn points are
byte-identical at ship-local (+-1,0,-4), clear of every hull cube). The
LAUNCH KICK is along the bay's local +Y (torpedo_section/mod.rs:625), so
it rotates from ship-up to OUTBOARD - benign (1.0 m/s kick, PN guidance
takes over, and outboard is away from the hull either way); review R1.1
corrected an earlier claim here that the launch axis was -Z.

Ship-local forward is -Z with up +Y, so starboard is +X: the tube ids
were SWAPPED (tube_port sat at +X) and the turret ids claimed top/bottom
mounts (dorsal/ventral) on flank positions. All four ids now name their
real sides. Old ids swept repo-wide before regeneration: they lived only
in the builder and the generated RON (regenerated); the Auditor's
separate bow turret is misnamed "turret_dorsal" too - cosmetic, its
Rx(-90) bow roll is CORRECT, and renaming it would churn the ledger
bundle a third time today, so it is reported here, not changed.

## Verification

- gen_content run twice: identical output (generator stable); parity +
  bundle-uniformity 2/2.
- broadside_assault 11/11 (the tubes test counts prototypes, not ids -
  unaffected by the rename, as planned).
- No automated test models mount ROTATION yet - and this was the second
  wrong-roll bug in two days, so the gap is now a filed task rather than
  an admission: 20260717-162121 (mount-base adjacency content lint,
  seeded by review R1.2).
- Also reported, not changed (same class as the Auditor's misnamed bow
  turret): the gunship's hull_bow sits astern (+Z) and hull_aft at the
  actual bow, and the player/corvette hull_front/hull_back ids are
  equally decorative. Nothing keys on these ids; renaming is cosmetic
  churn, recorded here for a future naming sweep.
- The roll CHANGES engagement arcs (disclosed in the CHANGELOG): pitch
  clamps are mount-frame, so the pair now covers beam/astern like a real
  broadside gunship (dead-ahead both comfortable, one gun per beam,
  astern covered) instead of two forward guns and a blind stern.
  Thematically the fix, not a side effect.
- content_lint clean; balance_audit 0/0/2 acked (section ids are not
  audit keys); workspace --all-targets green; fmt last. Full suite on CI.
