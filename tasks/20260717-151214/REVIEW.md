# Review: Rust Tally mount rolls

- TASK: 20260717-151214
- BRANCH: fix/rust-tally-mounts

## Round 1

- VERDICT: APPROVE

- [ ] R1.1 (MINOR) tasks/20260717-151214/NOTES.md:15-18 - the "launch/spawn
  axis" claim is wrong on the launch half, and the code comment repeats it
  (crates/nova_assets/src/scenario/broadside.rs:211-212). The bay's launch
  DIRECTION is its local +Y, not -Z:
  crates/nova_gameplay/src/sections/torpedo_section/mod.rs:624-625 is
  `let spawner_direction = projectile_rotation * Vec3::Y;` under the comment
  "The spawner launches along its +Y (the bay's 'up', as authored)" - the
  same fact 20260717-151208's review pinned (its R1.4: "the mesh door and
  the launch axis are both local +Y"), which this NOTES cites while
  contradicting it. What Rz(-+90) leaves unchanged is the spawn OFFSET
  (0,0,-2), so the spawn POINTS are genuinely identical to pre-fix
  (ship-local (+-1,0,-4), re-derived below). The exit kick, though, rotates
  from ship +Y (up, out the old up-facing hatch) to ship +-X (outboard, out
  the new outboard hatch), and the torpedo's initial orientation rolls with
  it - benign (spawner_speed 1.0 vs max_speed 35, PN guidance takes over,
  kick points away from the hull either way) but not "launch behavior
  unchanged". Suggested change: reword both the comment and NOTES - the -Z
  spawn offset and spawn points are unchanged; the +Y launch kick turns
  from up to outboard (out the hatch either way); AI launch gating is on
  the HULL bearing (ai.rs:1596-1604 `ai_torpedo_envelope` dots ship
  `transform.forward()`, ai.rs:1698), so cadence and envelope are the parts
  that are genuinely unchanged.
  - Response: acknowledged - recorded in NOTES for a future naming sweep;
    nothing keys on the hull ids. fixed - NOTES now states the +Y launch kick (rotating to
    outboard, benign) and that the spawn OFFSET is what Rz preserves.

- [ ] R1.2 (MINOR) tasks/20260717-151214/NOTES.md:35-38 - the admitted gap
  ("no automated test models mount ROTATION") deserves a follow-up task,
  not only an admission: this is the second wrong-mount-roll bug in two
  days (Auditor bay half-embedded, 20260717-151208; now four gunship
  mounts), and the class is mechanically lintable. All shipped section
  rotations are quarter-turns, so `rotation * Vec3::NEG_Y` is exactly
  axis-aligned, and content_lint's section-overlap pass (added last task)
  already walks every ship's section list. Suggested change: file a
  follow-up task for a content_lint "mount-base adjacency" check - for
  turret/torpedo sections, error unless rotation * (0,-1,0) points from
  the section's cell to an occupied neighbor cell. That one check would
  have caught the Auditor bay, both old gunship turret rolls, and both
  identity tubes at build time, and it ends the class the way the overlap
  lint ended its class.
  - Response: fixed - follow-up task 20260717-162121 filed (mount-base
    adjacency lint) with the fail-first recipe, born on this branch.

- [ ] R1.3 (MINOR) CHANGELOG.md:35 - the turret ENGAGEMENT-ARC change is
  undisclosed (NOTES.md is silent on it too). The rolls do not just
  re-seat visuals: the pitch clamps (better_turret min -30deg / max
  +90deg, assets/base/sections/base.content.ron:66-67) are enforced in
  the mount's own frame (SmoothLookRotation min/max on the pitch base,
  turret_section.rs:549-555), and the AI holds fire per turret when the
  barrel cannot align (ai.rs:1512-1516, alignment <= AI_FIRE_ALIGNMENT
  0.95). Moving the yaw axis from ship-forward (old Rx(-90)) to outboard
  changes each turret's reachable set from "everything forward of 30deg
  abaft the beam, both turrets identically" to "own side's hemisphere plus
  30deg across the centerline": a beam target now draws ONE turret instead
  of two, dead-ahead draws both comfortably (previously both sat at the
  +90 pitch-clamp boundary), and dead-astern is newly covered by both
  (previously a blind cone). Thematically right for a broadside gunship
  and probably favorable in the head-on fight the scenario stages, but it
  is a real combat-behavior change that the docs present as pure geometry.
  Suggested change: one sentence in NOTES (and ideally the CHANGELOG
  entry) disclosing the arc shift.
  - Response: fixed - the CHANGELOG line discloses the arc change as the
    intended broadside behavior.

- [ ] R1.4 (NIT) crates/nova_assets/src/scenario/broadside.rs:244-246 -
  same class of misnaming, same function, unreported: under the very
  convention this fix enforces (ship-local forward -Z), the gunship's
  "hull_bow" sits at +Z (directly ahead of the thruster, i.e. astern) and
  "hull_aft" at -2Z (the actual bow); the player ship's
  hull_front/hull_back (broadside.rs:136-137) and the corvette's
  hull_front (broadside.rs:195) are swapped the same way. The task's
  sweep mandate covered turret ROTATIONS plus the four renamed ids, and
  NOTES reports the Auditor's misnamed turret_dorsal - but not these.
  Nothing keys on the ids (repo grep: no references outside the builder
  and generated RON), so report-only per the Auditor precedent. Suggested
  change: add them to the follow-up/report list so the next content churn
  window renames them.
  - Response:

No BLOCKER or MAJOR: the rotations, ids, spawn geometry and the turret
machinery's tolerance of rolled mounts all check out under independent
re-derivation (below). R1.1-R1.3 are docs-honesty/follow-up items on a
correct implementation, R1.4 is a report-only observation. APPROVE.

### Verification record

Re-derived, not taken on trust:

- Mount base = local -Y, RE-VERIFIED from raw GLB vertex data (not
  accepted from the prior review): parsed
  assets/base/gltf/torpedo-bay-01.glb (404 verts) - a full-footprint slab
  occupies y in [-1.0,-0.9] pre-scale (x,z spanning [-1,1]) and the raised
  hatch detail sits at y in [+0.94,+1.0] (x ~ +-0.15, z ~ +-0.5); the node
  transform Ry(180) * scale 0.5 preserves Y. Base -Y, hatch +Y, exactly as
  20260717-151208's review documented. The TURRET's -Y base is pinned by
  the Rust side: better_turret's base_offset is (0,-0.5,0) (the -Y face of
  the unit cube, base.content.ron:67-71) and the rotator chain stacks
  upward from it (yaw_offset +0.1Y, pitch_offset +0.33Y); parsed
  turret-yaw-01.glb confirms the turntable mesh spans local y [0,+0.999]
  pre-scale - entirely ABOVE its node origin. The turret builds +Y off a
  -Y base.
- Quat re-derivation (file order (x,y,z,w), cross-checked in 151208's
  review): starboard RON value (0,0,-0.70710677,0.70710677) = Rz(-90),
  which maps -Y -> -X (base into the spine for the +X mounts at x=+1,
  spine at x=0) and +Y -> +X (hatch/turntable outboard); port
  (0,0,0.70710677,0.70710677) = Rz(+90) maps -Y -> +X and +Y -> -X, the
  exact mirror for the x=-1 mounts. Both leave +-Z fixed. Verified against
  the regenerated RON: turret_starboard (1,0,0) and tube_starboard
  (1,0,-2) carry Rz(-90); turret_port (-1,0,-1) and tube_port (-1,0,-2)
  carry Rz(+90).
- Port/starboard convention: forward -Z with up +Y (right-handed) puts
  starboard at +X; independently confirmed by the builder itself ("Ships
  spawn with -Z forward", broadside.rs:49-55; player at z=40 faces the
  hauler at z=-450 under Quat::IDENTITY; thrusters at +Z, bow turrets at
  -Z). All four new ids match their offsets; the old tube_port at +X was
  indeed on the starboard side.
- Torpedo spawn points: bay spawn_offset (0,0,-2) local
  (base.content.ron:194-198), spawner pose composed as section_rotation *
  offset (local_pose_in_root, torpedo_section/mod.rs), and Rz(-+90) fixes
  -Z, so spawn = (+-1,0,-2) + (0,0,-2) = ship-local (+-1,0,-4) - byte-
  identical to the pre-fix identity-rotation spawn points. Checked against
  all nine gunship unit cubes (controller (0,0,0), hull_bow (0,0,1),
  hull_mid (0,0,-1), hull_aft (0,0,-2), thruster (0,0,2), turrets (1,0,0)
  and (-1,0,-1), tubes (+-1,0,-2)): nearest is the tube's own cube, 1.5u
  beyond its -Z face; every other cube is >= 1 away on x or z. Clear of
  every hull cube, and ProjectileHooks filters owner contacts regardless.
- Turret yaw/pitch machinery under a rolled mount (the 1d risk item): NO
  world-up assumption anywhere in the chain. The section entity carries
  the authored rotation (nova_scenario/src/objects/spaceship.rs:234,
  `Transform::from_translation(section.position).with_rotation(section.rotation)`)
  and the whole rotator chain (rotator_base -> yaw_base -> yaw ->
  pitch_base -> pitch -> barrel -> muzzle) hangs under it
  (turret_section.rs:505-616), so the yaw axis IS the mount's local +Y
  wherever the roll points it. `update_turret_target_yaw_system`
  (turret_section.rs:820-894) inverts the yaw base's own composed global
  transform (`world_to_yaw_base = yaw_chain.to_matrix().inverse()`, line
  872) and solves phi/theta from target coordinates in that local frame;
  the pitch system (896-967) does the same in the pitch base's frame; the
  sync systems apply the outputs as pure local-axis rotations
  (`Quat::from_euler(EulerRot::YXZ, out, 0, 0)` about local Y, line 982).
  Aim points come in as world positions (AI writes
  TurretSectionTargetInput, ai.rs:1328-1334) and are transformed per
  turret. A rolled mount therefore aims correctly; the only frame-relative
  behavior is the INTENDED one (pitch clamps relative to the mount - see
  R1.3). The AI fire gate is also per turret and muzzle-relative
  (ai.rs:1509-1516 dots the muzzle's actual forward against the lead
  point, holding fire below cos 0.95), so an off-arc turret holds instead
  of spraying.
- AI torpedo launch gating: `ai_torpedo_envelope` (ai.rs:1596-1604) dots
  the SHIP's forward (ai.rs:1698, `*transform.forward()`) - the tube rolls
  do not touch launch cadence, envelope, or the alignment gate.
- Old-id sweep (repo-wide, -I, .git and tasks/ history excluded):
  turret_dorsal/turret_ventral survive only in
  webmods/the-ledger/ledger_ch4.content.ron:209,302 - the Auditor's bow
  turret, documented out of scope in NOTES (its Rx(-90) bow roll is
  correct; only the name is stale). tube_port/tube_starboard are reused
  (swapped) as the NEW ids; their only non-history hits are the builder
  (broadside.rs:250-251) and the regenerated RON, with the +X tube now
  named tube_starboard. Sweep clean as claimed.
- Docs vs diff: CHANGELOG entry matches the change (rolls, outboard
  hatches, id swap) except the undisclosed arc shift (R1.3); NOTES'
  geometry section is correct except the launch-axis conflation (R1.1);
  the no-rotation-test admission is honest (R1.2 asks for the follow-up).
  Confirmed NOTES' claim that broadside_assault counts prototypes, not
  ids (crates/nova_assets/tests/broadside_assault.rs:414-421 filters on
  `SectionSource::Prototype(p) if p == "torpedo_section"`), so the rename
  is invisible to it - and correspondingly nothing pins the new ids
  either.

Commands run (worktree
/home/alex/.cache/sprouts/nova-protocol/fix/rust-tally-mounts):

- `cargo run -p nova_assets --bin gen_content` -> wrote all five
  scenario RONs; `git status --porcelain` after -> empty (no drift; the
  committed RON is byte-exact builder output).
- `cargo test -p nova_assets --test content_ron_parity` ->
  `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s`
- `cargo test -p nova_assets --test broadside_assault` ->
  `test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s`
- `cargo run -p nova_assets --bin content_lint` ->
  `content_lint: clean (1 warning(s))` (the warning is the pre-existing
  ledger dual-spawn WARN, unchanged)
- `cargo run -p nova_assets --bin balance_audit` ->
  `balance_audit: 11 combat scenario(s), 0 error(s), 0 warning(s), 2 acked`
  (both acks are the pre-existing 20260717-143806 close-spawn acks)
- `cargo check --workspace --all-targets` ->
  `Finished \`dev\` profile [optimized + debuginfo] target(s) in 0.55s`
  (only the pre-existing proc-macro-error2 future-incompat note)

Per standing instruction (skip-local-tests-and-clippy), the full test
suite and clippy were not run locally; the targeted tests and content
gates above only. Full suite runs on CI.
