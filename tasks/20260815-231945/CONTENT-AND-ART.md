# Content and art audit - round 4 research for v0.12.0 planning, written 2026-08-24

Lane scope: thruster shells (task 20260817-090834), multi-cell sections
(THRUSTERS.md follow-up 3, now a v0.12.0 feature decision), particle vacuum
audit (task 20260822-204201), and the content generation flow. Read-only
research; nothing was built or run.

## 1. Thruster shells: candidates, generator, convention, promotion

### 1.1 Candidates

Seven committed .glb candidates in `art/part-candidates/shells/`:
`shell_bell`, `shell_gimbal`, `shell_twin`, `shell_paddle`, `shell_vector`
(1x1x1), `shell_bank` (3x3x1), `shell_capital` (5x5x3). Sources are JSON
recipes in `scripts/thruster-shell-recipes/*.json`; only the two large ones
declare a cells box (`shell_bank.json:3` is `[3, 3, 1]`,
`shell_capital.json:3` is `[5, 5, 3]`; the rest default to 1x1x1).

Owner review already happened once: task `20260817-013639` closure
(`tasks/20260817-013639/TASK.md:27-33`) records bell + vector KEPT, three
rejects replaced by gimbal, twin, paddle in the same anatomy. The gallery
notes agree: `examples/screenshots/screenshot_thruster_gallery.rs:232`
("exposed ringed bell (kept)") and `:272` ("petal vectoring shroud (kept)");
the large rows are labeled "art only" (`:287`, `:297`).

### 1.2 Generator and determinism gate

`scripts/gen-thruster-shells.py` (261 lines): imports gen-greebles.py's
primitive/recipe/verify layer (lines 62-76), owns the CELL frame (line 52,
`CELL = 1.0`), budget `MAX_TRIANGLES_PER_CELL = 450` scaled by cell volume
(lines 59, 118). `build_shell` (lines 79-124) enforces: geometry inside the
WxHxL cell box (lines 104-111), aft exhaust presence (lines 113-117),
triangle budget. `--check` (lines 145-167) rebuilds in memory and
byte-compares with the committed file; `--self-test` (lines 176-238) pins
budgets and byte reproducibility.

GAP: the --check gate is manual. `.github/workflows/ci.yaml` runs no python
step (grepped; also recorded unfixed in `tasks/20260817-013639/TASK.md:38-41`
- "ci.yaml runs no python step"). I did not run the script this session.

### 1.3 Thrust convention

- Thrust is applied along -Z; the bell opens +Z:
  `crates/nova_ship/src/sections/clearance.rs:64-66` (`exit_normal` returns
  `Vec3::Z` for `SectionKind::Thruster`).
- `SectionExit` carries that per live section (clearance.rs:80-90);
  `ShipExit` is one cell plus one face index (clearance.rs:94-100).
- The lane is a COLUMN of cells out of the ship (`exit_lanes`
  clearance.rs:160, `blocked_exits` clearance.rs:167,
  `placement_blocks_an_exit` clearance.rs:230). A lane fails as `Structure`
  or `Cladding` (`BlockedExitReason`, clearance.rs:104-110).
- `exit_pocket` (`crates/nova_ship/src/sections/shell_skin.rs:321`) keeps the
  lane cell bare of skin; `stands` (shell_skin.rs:339) and `walls`
  (shell_skin.rs:450) are the two functions that make today's drive invisible
  to the skin (no flank sockets).
- The generator's cell frame matches: exhaust at +Z, budget check
  `gen-thruster-shells.py:113-117` demands aft-half geometry.

### 1.4 What 1x1 promotion concretely requires

- The prototype today: `basic_thruster_section` in
  `crates/nova_authoring/src/base_content/sections/standard.rs:320-360`.
  `render_mesh: None` (line 355), one `base` link point at `-Z * 0.5`
  (lines 346-350), health `THRUSTER_BASE_HEALTH` (line 334).
- Runtime render: `insert_thruster_section_render`
  (`crates/nova_ship/src/sections/thruster_section.rs:662`); with
  `render_mesh` set it spawns a `WorldAssetRoot` child with an authored
  `RenderMeshTransform` (thruster_section.rs:698-712), else placeholder
  barrel + nozzle primitives (thruster_section.rs:713-731). The exhaust cone
  spawns for EVERY thruster either way (thruster_section.rs:734-745), so a
  promoted mesh keeps the plume for free.
- Steps: (1) move the picked .glb(s) from `art/part-candidates/shells/` to
  `assets/base/gltf/` (the greeble pattern - see
  `assets/base/gltf/greebles/README.md`), with the recipe as source and the
  --check idiom pointing at the new path; (2) register an
  `AssetRef<WorldAsset>` in
  `crates/nova_authoring/src/base_content/assets.rs` (pattern: `hull`,
  `turret_yaw` etc. at lines 22-26); (3) set `render_mesh` +
  `render_mesh_transform` on the builder in standard.rs; (4) run
  `cargo run content gen` and commit the regenerated
  `assets/base/sections/base.content.ron` (never hand-edit it); (5) verify
  with the gallery row plus a wfc_ships / wfc_arena render -
  `examples/playable/wfc_ships.rs` and `wfc_arena.rs` inherit the look
  through the prototype automatically.
- Per `tasks/20260817-090834/TASK.md:14-21`, each candidate must be checked
  first: exhaust geometry vs the -Z/+Z convention and the exit_pocket
  silhouette, triangle/material sanity at ship distance, determinism under
  --check. Task line 30-32 explicitly defers the large formats.
- If a promotion REPLACES the primitive look (rather than joining as a new
  prototype), no id changes; if it lands as new prototypes, see section 4 on
  ids. The one-socket test
  (`the_thruster_sockets_only_the_face_it_bolts_on_by`, standard.rs:695) only
  breaks if link points change - promotion alone does not touch it.

## 2. THRUSTERS.md follow-up 3: multi-cell sections

### 2.1 The question as written

`tasks/20260816-200255/THRUSTERS.md:304-307` (follow-up list, item 3):

> 3. **Multi-cell sections** (L) - one section spanning WxHxL cells: reading
>    (`PlacedPart` grows a footprint), wfc tiles, clearance (one lane per
>    exhaust column), integrity, collider, editor placement. Designed
>    separately; this spike only fixes its requirement.

The engine gap it names is stated at THRUSTERS.md:254-265 (section 4.3): "A
section occupies ONE cell today: `PlacedPart` has one position, `read_cells`
buckets it once, the wfc grid holds 'one section at one rotation' per cell."
Also relevant: 4.2 (mass = cell volume, thrust magnitude = cell volume, so
thrust/mass constant; exhaust as one bell per cell or one Rect sheet) and the
gallery mocks (screenshot_thruster_gallery.rs:280-297).

### 2.2 How sections and cells are represented today

- One placed part = one position + rotation + link points + optional exit:
  `PlacedPart` at `crates/nova_ship/src/sections/shell_skin.rs:798-810`.
- `read_cells` buckets each part into exactly ONE cell
  (shell_skin.rs:830-845); the lattice phase is voted per axis from section
  positions (`lattice_phase`, shell_skin.rs:860). `PLACEMENT_SNAP`
  (shell_skin.rs:125).
- `SkinStructure` is per-cell face facts (sockets [bool;6] + exits [bool;6]):
  shell_skin.rs:152-232. `insert_section` (line 180) already ADDS into a cell
  bucket - the READING half is multi-cell ready, as THRUSTERS.md:261-263
  claims: you can call it once per footprint cell with that cell's outward
  normals.
- PlacedPart constructors that would all need footprint awareness: the live
  skin spawner (shell_skin.rs:685), the editor ghost (nova_editor/snap.rs:271,
  nova_editor/skin.rs:101 and :249), probe snapshots
  (nova_probe/capabilities/snapshot.rs:578), clearance tests
  (clearance.rs:251, :263).
- Ship files store one position/rotation/prototype-source per section:
  `SpaceshipSectionConfig`,
  `crates/nova_scenario/src/objects/spaceship.rs:207-225`.

### 2.3 Prototype data model

`SectionConfig` = `BaseSectionConfig` (id, health, collider, link_points,
sounds, damage_effects) + `SectionKind` (standard.rs usage; base fields in
`crates/nova_ship/src/sections/base_section.rs`). There is NO footprint/size
field anywhere - size is implied by the collider and the unit-cell
convention. `SectionCollider` (base_section.rs:52-77) already authors
arbitrary Cuboid sizes, and mass IS collider volume at density 1
(base_section.rs:45-49 and :372), so a 3x3x1 collider gets mass 9 with zero
new code - THRUSTERS.md 4.2's stance falls out of shipped physics.

### 2.4 Editor placement

Placement is link-point mating only (`crates/nova_editor/src/snap.rs:1-7`);
refusals are `NoTargetSockets / NoPartSockets / Occupied / Ambiguous /
Overlap / BlockedExit` (snap.rs:35-52). Overlap uses the authored collider
AABB against non-mated sections (snap.rs:251-254, OVERLAP_EPSILON line 15);
clearance reuses `placement_blocks_an_exit`. So the editor is surprisingly
close to ready: a multi-cell prototype with link points on its exposed cell
faces and a correct big collider would snap, refuse overlaps, and respect
lanes - PER LINK POINT. What is missing is per-cell exit registration (a 3x3
face is one `exit: Option<Vec3>` today) and any UI concept of the footprint.

### 2.5 WFC generation

`examples/playable/shared/wfc.rs` (shared by wfc_ships and wfc_arena):
- `Tile` is one prototype at one rotation in ONE cell (wfc.rs:195-227);
  `TileBody.offset` exists only for the half-size PDC boundary stance
  (wfc.rs:222-226).
- `tile()` (wfc.rs:248) REJECTS any part whose rotated collider leaves the
  unit cell: wfc.rs:272-279 ("The body has to stay inside its own cell").
  So multi-cell prototypes do not break the generator - they are silently
  absent from the tile set until tiles learn footprints.
- The grid is a dense one-tile-per-cell array (wfc.rs:416-466); the
  socket-vs-blank adjacency rule (wfc.rs:561-580) and the erode passes
  (`erode_blocked_exits` wfc.rs:585, `erode_studs` wfc.rs:884) all assume
  one-cell tiles.

### 2.6 Hard problems, in order

1. WFC with footprints (hardest). Classic WFC has no variable-size tiles.
   Options: meta-tile decomposition (per-cell subtiles constrained to
   assemble one part - large state-space change to wfc.rs), or a pre/post
   pass that stamps big drives first and collapses around them (much
   cheaper, fits "capital stern with a 3x3x1" from THRUSTERS.md:312).
2. Per-cell exits in clearance. `SectionExit` is one Vec3 per section entity
   (clearance.rs:80-90); a 5x5 exhaust face needs 25 `ShipExit` columns.
   Everything that builds or consumes exits (clearance.rs:160-230, skin
   exit_pocket, editor refusal, wfc erode) must accept a set.
3. Footprint in the reading. `PlacedPart` grows a footprint (or a cells
   iterator); all six constructor sites in 2.2 change together; ship-file
   compatibility is fine (one position stays the anchor).
4. Authoring model. A footprint field on the prototype (or derived from the
   collider), link points on every exposed cell face (the flank-socket
   doctrine from THRUSTERS.md section 3 multiplies across faces), and the
   wfc `tile()` inset logic reworked (its common-inset rule wfc.rs:243-264
   is unit-cell arithmetic).
5. Integrity and destruction granularity. `derive_link_point_graph`
   (`crates/nova_ship/src/sections/link_points.rs:149`) mates fine with more
   link points, but one entity dying removes 75 cells at once; whether that
   is acceptable (probably yes - a drive block is one machine) is a design
   call, not code.
6. Exhaust render choice. One bell per exhaust cell vs one `Rect` sheet -
   already authorable (`ThrusterExhaustShape::Rect`,
   thruster_section.rs:153-162, rect builder :190).
7. Mass/physics: essentially free (2.3).

Note the dependency split THRUSTERS.md:263-265 insists on: flank sockets
(follow-up 1) and the 1x1 shell look (follow-up 2) pay off immediately and do
NOT need multi-cell; only the size family (follow-up 4) does.

## 3. Particle effects inventory (task 20260822-204201)

### 3.1 bevy_hanabi families (the literal particle effects)

Hanabi is registered once (`crates/nova_gameplay/src/plugin.rs:76-79`); only
two crates depend on it (nova_gameplay, nova_ship Cargo.toml). All shipped
content authors `None` for effect overrides (standard.rs:181, :502-503,
:574-575; ships/shared.rs:337-338), so every player-visible hanabi effect is
a built-in default. Three graphs exist:

1. Torpedo blast -
   `crates/nova_ship/src/sections/torpedo_section/render.rs:247-318`
   (`build_default_blast_effect`). Shared asset via `DefaultBlastEffect`
   (render.rs:222-232), one 512-particle instance per detonation
   (BLAST_CAPACITY render.rs:242), spawned as a `TempEntity(2.0)`
   (render.rs:367-374). Authored override: `TorpedoSectionConfig.blast_effect`
   (torpedo_section/mod.rs:120).
2. Torpedo launch puff - render.rs:395-503. Cold white-blue propellant flash,
   80 particles per launch, capacity 512 (render.rs:387), reset-driven.
   NOTE: unlike the blast and muzzle, the default asset is minted PER BAY
   SPAWNER (`effects.add` inside `insert_torpedo_spawner_effect`,
   render.rs:484-494) - a sharing cleanup candidate the audit should record.
   Override: `launch_effect` (mod.rs:132).
3. Turret muzzle flash -
   `crates/nova_ship/src/sections/turret_section/render.rs:353-409`
   (`build_default_muzzle_effect`), shared via `DefaultMuzzleEffect`
   (render.rs:417-425), one 2048-capacity instance per barrel (render.rs:345,
   sized to the 100 rounds/s PDC). Rendered as 3-logical-pixel SCREEN-SPACE
   dots (render.rs:404-408). Override: `TurretSectionConfig.muzzle_effect`
   (turret_section/config.rs:22).

### 3.2 Non-hanabi player-visible effect families the audit still owns

- Damage sparks: shared-mesh entities with velocity, thresholded on damage
  level (`crates/nova_ship/src/sections/damage_sparks.rs`, threshold :38,
  spawn :192).
- Damage cracks: material-driven surface fracture
  (`crates/nova_ship/src/sections/damage_cracks.rs`).
- Damage plume: modulates the exhaust of a hurt drive
  (`crates/nova_ship/src/sections/damage_plume.rs`).
- Thruster/torpedo exhaust plume: shader cone/rect, not particles
  (`crates/nova_ship/src/sections/thruster_section.rs:119-170`).
- Juice impact/destruction gizmo rings + camera shake
  (`crates/nova_gameplay/src/juice.rs:134-141`,
  `crates/nova_gameplay/src/shake.rs`).
The authored vocabulary tying sections to looks is `DamageEffect`
(`crates/nova_ship/src/sections/damage_effects.rs:59-74`: Cracks, Sparks,
Plume).

### 3.3 Torpedo baseline - confirmed landed

The vacuum retune is on master: commits `88d7322a` "Render torpedo blasts as
vacuum ejecta" and `b374c172` "Give torpedo blasts a brighter punch". The
graph comments say it outright: "A vacuum burst is a brief flash followed by
fast incandescent ejecta, not a lingering atmospheric cloud"
(render.rs:257-258); HDR white-gold cooling through amber to dim red with no
smoke phase (render.rs:262-268); velocity-oriented radial streaks
(render.rs:278-289, OrientMode::AlongVelocity :316); faster 12-60 front
(render.rs:302-305). No blast-radius sphere exists anywhere in
torpedo_section/render.rs (grepped). The two accepted refinement targets are
written into the task itself (`tasks/20260822-204201/TASK.md:36-39`):
Hanabi extraction delays first visible ejecta a few frames, and close views
expose square billboards. Both are directed to be fixed as common VFX
direction, not torpedo-only.

### 3.4 Graphics tiers and wasm

- `GraphicsBudget` (`crates/nova_gameplay/src/settings.rs:188-236`):
  `particles: bool` is the only particle lever; Low = no spawns + 0.7 render
  scale, Medium/High = full particles. All four hanabi spawn sites gate on it
  (torpedo render.rs:333-337, :406-410, :527-532; turret render.rs:30-40).
  Effect assets are lazy, so a particles-off run builds nothing
  (render.rs:220-221).
- Wasm: hanabi needs compute shaders, so the web build forces the WebGPU
  backend (`crates/nova_core/Cargo.toml:28-37`); hanabi panics without a
  render sub-app (`crates/nova_core/src/lib.rs:172-173`). Menu overlay had a
  double-render ghosting fix around hanabi bursts
  (`crates/nova_menu/src/ambience.rs:20-31`).
- The task's "burst concurrency limits / transient light budgets" have NO
  existing machinery: capacities are per-family consts, there is no global
  concurrent-burst cap and no dynamic lights in any effect (the torpedo task
  boundary forbids adding them without the audit).

### 3.5 What a vacuum-treatment audit must touch per family

For each of the three hanabi graphs plus sparks/plume/exhaust/juice:
(a) a stated vacuum role (flash / ejecta / vapor / fragments / momentum), as
the blast now has; (b) momentum correctness - blast inherits none of the
torpedo's velocity today, puff and muzzle carry `base_velocity` properties
(torpedo render.rs:469-470, turret render.rs:377-378); (c) capacity/instance
cost recorded (shared asset vs per-instance buffer, 3.1); (d) tier behavior
confirmed at the spawn gate; (e) deterministic capture idiom - the shipped
examples `examples/screenshots/loop_torpedo_blast.rs` (scripted, seeded,
re-capture reproduces frames) and `screenshot_torpedo_run.rs` are the
pattern for isolated/stress captures; (f) the two cross-cutting fixes
(extraction delay, billboard shape) verified on every family they touch, not
just the torpedo.

## 4. Content generation flow

- Pipeline confirmed: Rust builders in
  `crates/nova_authoring/src/base_content/` are the single definition;
  `cargo run content gen` (CLI: `crates/nova_authoring/src/cli.rs:1-113`,
  `run_gen` :104) serializes `crate::generation::content_files()`
  (`crates/nova_authoring/src/generation.rs:162`) into the committed
  `assets/base/**/*.content.ron` (e.g. `base/sections/base.content.ron`,
  generation.rs:165). Never hand-edit the RON; the
  `content_ron_parity` integration test
  (`crates/nova_authoring/tests/content_ron_parity.rs`) asserts the files
  match the builders.
- Lint: `cargo run content lint` runs every check in one pass - ids,
  geometry, references, balance audit, input overlap (cli.rs:17-34). CI runs
  the same walks via `crates/nova_authoring/tests/content_lint_gate.rs` and
  `balance_audit_gate.rs` in `.github/workflows/ci.yaml`.
- Ids: plain strings authored in the builder (e.g.
  `id: BASIC_THRUSTER_SECTION_ID` standard.rs:322). Consts live beside the
  builders; ONLY ids that engine crates must name get promoted to
  `crates/nova_ship/src/sections/catalog_ids.rs` (its own rule, lines 1-5;
  five section ids today, lines 16-24). Duplicate ids are a lint error
  (cli.rs:20-21). New prototypes = new builder entry + optional const +
  `content gen` + commit.
- Ship cast: `crates/nova_authoring/src/base_content/ships/mod.rs:32-46` -
  `cargoa` (armed corvette, `cargo_a.rs:3` "the campaign's light fighter -
  the corvette"), `cargoa_raider` (mod.rs:36; flown by raiders across the
  campaign, e.g. `base_content/scenarios/nova_protocol/broadside.rs:213`;
  player and scavenger both fly the cargoa,
  `scenarios/nova_protocol/shakedown/tests/pins.rs:292`), `racer` (unarmed
  civilian yacht, `racer.rs:3`, "Racer Yacht" mod.rs:63), plus `cargob` /
  `cargob_lance`. Section prototypes live in
  `base_content/sections/standard.rs` and `ordnance.rs`; ships in
  `base_content/ships/`.

## What this means for v0.12.0

Ordered suggestions:

1. Land task 20260817-090834 (1x1 promotion) first and standalone. It is
   mechanical: move picked glbs to `assets/base/gltf/`, asset ref, builder
   `render_mesh`, `content gen`, gallery + wfc render proof. Two candidates
   already carry an owner "kept" mark (bell, vector); re-confirm the picks
   rather than assuming them. Fold the missing CI line for
   `gen-thruster-shells.py --check` (and the greeble twin) into this task -
   the gate exists but nothing runs it.
2. Decide whether the flank-socket change (THRUSTERS.md follow-up 1) ships
   in v0.12.0. It is independent of multi-cell, pays off on the 1x1
   immediately, and is a prerequisite for big shells looking right; but it
   changes wfc adjacency legality (staggered nozzles die, mixed
   fitting-beside-drive banks need the bay-flank decision, THRUSTERS.md
   3.3.1) and replaces the one-socket test.
3. Multi-cell sections: SPIKE FIRST, not feature-first. The spike THRUSTERS.md
   itself sized it L and said "designed separately". The physics/skin/editor
   halves are genuinely close (mass free, SkinStructure reading ready, editor
   mating per link point), but the WFC representation question (meta-tiles vs
   stamp-then-collapse) and per-cell exits are real design forks with no
   obviously right answer in the codebase today. A one-lane spike that (a)
   picks the wfc strategy with a throwaway prototype on wfc.rs, (b) fixes the
   PlacedPart footprint shape, and (c) prices per-cell exits, converts an L
   unknown into two or three M implement lanes (reading+clearance, wfc,
   content+editor polish). Budget the feature as spike + 3 lanes, and keep
   `shell_bank`/`shell_capital` promotable art meanwhile.
4. Particle vacuum audit: start from the inventory in section 3 - it is
   small (three hanabi graphs, all defaults, plus four non-hanabi families).
   The two cross-cutting fixes (extraction delay, square billboards) touch
   every family and should be one lane before per-family retunes. Record the
   launch-puff per-bay asset minting as a cheap sharing fix. Define the
   missing budgets (burst concurrency, transient lights) BEFORE adding
   complexity - the task text demands it and nothing enforces them today.
5. Sequencing note: the vacuum audit and the shell promotion are disjoint
   (shader plume vs hanabi bursts vs mesh promotion) and can run as parallel
   lanes; multi-cell should wait for the spike verdict before claiming a
   release slot.
