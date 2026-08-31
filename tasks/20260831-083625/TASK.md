# Model the sections to the thruster's standard

- STATUS: OPEN
- PRIORITY: 80
- TAGS: v0.13.0,art,content,ship

## Goal

Give the sections real models. The thruster is the quality bar - it is the
one section that already reads as built hardware - and everything else
should come up to it.

Owner framing (2026-08-31): "new model for the torpedo bay and actually
maybe models for all sections (except the thrusters which are now the
baseline in terms of quality for sections)".

## Start with the torpedo bay, because it is a placeholder

`assets/base/gltf/torpedo-bay-01.glb` is a UNIT CUBE. One node, one mesh,
bounds -1..1 scaled by 0.5. It has no tube, no mouth and no directionality
at all, which is why nothing about the launch reads as coming out of a
launcher.

The launch mechanic is already built and waiting for it. A torpedo coasts
inert for `ignition_delay` seconds (0.6 by default) with its colliders
disabled and guidance suspended - see task 20260822-204201 stage 4. That
window is exactly where an emergence animation lives, so this is art hanging
off a hook that already exists, not new systems work.

The bay fires out of its -Z face. `link_points` deliberately leaves that
face unlinkable so it can be a muzzle, and the authored `spawn_rotation`
now turns the launch axis onto it. Model the tube mouth THERE.

Wanted, in the owner's words: "torpedo bay with doors that open to let the
torpedo out and stuff like that". So the bay wants an animation track, not
just a mesh - which is the first section that does. Decide how an authored
section declares an animation before modelling the second one.

## The rest

- Every section except the thruster, which is the reference.
- The PDC turret is a special case: the stow wish (`20260831-083622`, kept
  in the backlog as a future promise) wants more geometry around the mount
  for a turret to disappear into. Do not wait on it - model the mount with
  room for a future stow and record the shape decision here.
- Keep the sections readable at gameplay distance and at the silhouette
  level - a player has to tell a turret from a bay at a glance, which is the
  same constraint the damage effects were written against
  (`damage_sparks.rs`: "a player has to be able to tell at a glance that it
  is the thing shooting at them").

## Watch for

- Section colliders come from the spec, not the mesh, so a prettier mesh
  must not silently change hitboxes. Check `collider:` per section.
- `render_mesh_transform` is visual-only and does not move the launch point.
  Do not use it to hide a spawn-point mismatch.
- Asset coverage checks are advisory and exit 0. Missing art warns, it does
  not gate.

## Done when

- The torpedo bay is a launcher with a tube, and a torpedo is seen leaving
  it rather than appearing beside it.
- An authored section can declare an animation, and the bay doors use it.
- Every non-thruster section has a model at the thruster's standard.
- Silhouettes stay readable at combat range.

## Decisions

- One style for all sections. Skins add faction detail on top. Hull and
  controller interiors read as machinery and wires when exposed.
- The bay is `bay_tube`, 1x1x2. The -Z muzzle face stays open and unlinkable.
  One back socket plus eight flank sockets (two cells x four faces). A 1x1x2
  section centred on an integer cell sits off the section grid, so authored
  ships seat bays with a half-cell shift.
- The gatling mount is THE PDC. The twin mount is a variant, not a
  replacement: two muzzles, each at half the gatling rate, so both mounts
  spend ammo at the same total rate. Both mounts ship in kinetic and pierce.
- Hulls become three same-stat prototypes: personnel (default), cargo, tank.
  The models are the investment; distinct hull TYPES come later if wanted.
- The controller catalog prototype gets the `core_wires` cube. Ships author
  their own controller visuals, so only the editor shows it.
- Dropped candidates stay in `art/part-candidates/sections/` as the record
  of the gallery round. Promoted stems build straight into
  `assets/base/gltf/` via `scripts/gen-section-parts.py`.
- `assets/base/base.bundle.ron` is hand-authored, not generated -
  `content gen` does not write it and `89091f2e` edited it by hand. The
  eleven new resources are declared there directly. All `*.content.ron`
  changes went through `content gen` only.
- Section animation is PROCEDURAL DATA, not glTF clips: a track on the
  base config names a cue, scene nodes by name prefix, a motion over the
  rest pose, and travel times. Kind systems steer cue targets only. The
  full argument is `animation-research.md` in this folder; clips remain
  a future `motion` variant behind the same declaration.
- No launch recoil on the ship, by fiction: the ship's RCS counters the
  ejection kick off screen (the Expanse turret treatment), so the
  ejection charge moves the torpedo only. The railgun is the weapon
  whose recoil is a feature, and it applies its impulse for real.

## Landed

1. `c4719077` - the candidate gallery. `scripts/gen-section-parts.py` builds
   23 parametric parts; `screenshot_section_gallery` renders them beside the
   shipped originals for the pick.

2. `dc82bd30` - promotion and integration. The eleven winners move to
   `assets/base/gltf/` (byte-identical renames; `--check` stays byte-clean).
   Catalog rebuilt on the new models: hull trio, controller core, four PDCs
   (`pdc_twin_*_turret_section` are new), every bay a 1x1x2 tube with
   `spawn_offset` 2.5 out of the muzzle. Bays reseated with the half-cell
   shift in the showcase ship, the VFX range, the torpedo-launch example and
   the shape bench. New `screenshot_section_trials` is the live-fire
   acceptance range; `screenshot_section_weapons` gains the twin closeup.

3. `0c64e2cf` - the WFC generator learns two-cell parts. Regression from 2,
   found by the owner running `wfc_arena`: the solver was single-cell by
   construction, so the 1x1x2 bay produced no tiles, every generated hull
   came out bay-less, and the arena's draft panicked past its seed cap. A
   multi-cell part now collapses as a chain of segment tiles joined across
   their shared faces (a joint accepts exactly its own next segment; the
   pruning passes drop a part's cells together), and the arena's drive stamp
   carves by body rather than by position. The segment chain is the
   mechanism the spinal railgun (`20260824-125947`) will reuse. Proof: two
   segment tests plus the stamp suite (5 pass); both examples' walks pass
   under Xvfb - the arena drafts bay-armed hulls again and fights to its
   capture, and the clad posed row skins cleanly.

4. `357cb815` - the iris muzzle doors. The part generator learns named
   nodes; `bay_tube.glb` grows a closed six-petal iris on the muzzle face
   (hinge frames on the hexagon edges, recessed throat behind). Sections
   declare animation tracks on the base config; the generic
   `SectionAnimationPlugin` resolves rigs on scene-ready and drives
   progress; the bay's fire path holds the `MuzzleDoor` cue open across
   the cold coast plus a 0.3 s linger. Owner approved the look from the
   live frames before the land. Proof: 8 behaviour tests (driver, hold,
   and a catalog test binding every tube bay's track to real petal
   nodes inside the cold-coast window), `content lint` 0 errors, the
   torpedo range's door round runs the full cycle live -
   `bay-doors-{shut,open}.png` in this folder.

5. `46872c5e` - the torpedo launches out of the tube. The held trigger is
   the door intent (weapons safety and an empty magazine keep the iris
   shut), the ejection waits for the track to read fully open, and the
   round is born at the tube's centre - `spawn_offset` becomes the muzzle
   point on the door plane, the new `spawn_recess` births the torpedo
   behind it - so it slides its whole length out through the open petals.
   Tap fire cycles the door, a shot into a closing door reopens it from
   wherever it was, held fire keeps the bay gaping. No ship recoil, per
   the RCS fiction. Proof: 21 bay + catalog tests, lint 0, the torpedo
   range and section trials walks run clean live, and
   `bay-doors-emerging.png` in this folder catches both torpedoes
   mid-emergence with their tails still in the tubes.

## Proof

- 12 `standard.rs` tests pass, including: the twin splits the gatling total
  across two mirrored muzzles; every bay claims both cells, keeps nine
  sockets on its faces, and launches clear of its tube.
- `content lint` 0 errors. `gen-section-parts.py --check`: 23 parts match a
  fresh build byte for byte. `cargo check --examples --features debug` clean.
- Six live runs under Xvfb, all exit 0, renders inspected:
  `screenshot_section_trials` (both PDC lanes scar their columns - the twin
  shows two tracer streams - and the bay torpedo flies 70u and erases the
  marked section; walk asserts all three), `screenshot_section_weapons`,
  `screenshot_section_gallery`, `loop_vfx_range`, `system_torpedo_launch`.
- Stills in this folder: `section-trials-{range,twin,launch}.png`,
  `wiki-section-{turret,turret-twin,torpedo-bay}.png`.

## Remaining

- PDC stow animation (future promise `20260831-083622`): barrel points up,
  the mount slides down, a cover closes over the face. May want a 1x1x2
  mount like the bay. Needs a slide/translate `motion` variant and a
  `Stow` cue - deliberate skips of the door round, recorded here.
- Changelog entry for the remodel when the release entry is written.
