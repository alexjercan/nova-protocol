# Refresh frontend app images: redo the screenshot examples and recapture every capturable web image

- PRIORITY: 70
- TAGS: v0.10.0, web, assets, screenshot
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE
- PARENT: 20260802-115955
- DEPENDS ON: 20260804-093910

## Context

Replaces `20260724-082856`, which tried to refresh the shipped web imagery in
place. The shipped captures were deleted instead: every game-rendered PNG is
gone from `web/src/assets/` (only the authored `banner.png`, the five
`icon-*.png` and the font remain), so this task starts from an empty slate and
a full worklist rather than from "which of these 31 files is stale?".

The goal is not parity with what was deleted. The `screenshots/` examples are
rebuilt (`20260804-093910` reduced them to capture-only) and the shots they
pose are dull: default camera, no action, no lighting intent. Redo them so the
site's imagery reads as a game worth playing, one shot at a time.

The advisory coverage report is the input and the progress meter:

    python3 scripts/gen-web-screenshots.py --report

Its `capturable` class is this task's scope: 27 images. The `manual` (7 post-card
thumbnails) and `historical` (25 old-version news figures) classes are NOT in
scope and stay outstanding.

## Inputs

Every `capturable` gap, assigned to the example that will produce it after the
roster change settled in `DECISION.md` - one row is one child task.

| Producer | Was | Images |
| --- | --- | --- |
| `screenshot_scene` | `screenshot_reel` | `feature-gravity`, `wiki-gravity`, `wiki-sections` |
| `screenshot_flight` | `screenshot_orbit` | `tutorial-orbit`, `feature-autopilot`, `wiki-flight` |
| `screenshot_combat` | `screenshot_combat` + `screenshot_juice` | `feature-combat`, `feature-hud`, `wiki-combat`, `wiki-hud`, `tutorial-combat-lock`, `tutorial-radar-lock`, `wiki-radar`, `feature-juice`, `news-090-combat-readability`, `news-090-contextual-hud` |
| `screenshot_sections` | unchanged | the five `wiki-section-*` |
| `screenshot_ui` | `screenshot_ui` | `feature-editor`, `tutorial-menu`, `wiki-settings`, `news-090-scenario-campaigns` |
| `screenshot_nova_os` | `screenshot_nova_os` | `news-090-nova-os-terminal`, `news-090-nova-os-apps` |

Without a manifest slot today: `wiki-settings`, and all five `news-090-*`.
Currently ALIASES, and each becoming its own framed beat: `wiki-combat`,
`wiki-hud`, `wiki-flight`, `wiki-radar`.

Deleted and deliberately not replaced: `devlog5-target-viewfinder.png` and
`devlog5-radar-stance-slots.png` (the composite). The site references neither -
the report listed them "shipped but never referenced" - so the COMPOSITES entry
goes away with them unless a page asks for it again.

## Steps

One scene per step, built and then APPROVED BY THE OWNER before the next step
starts. Every scene step ends at the same gate: the owner runs the example
plainly (`cargo run --example <name> --features debug`, free-fly WASD camera,
no `NOVA_REEL`) and says whether it looks good. NOTHING is captured until every
scene has passed; the last step is the only one that produces a PNG.

Each scene step: write the new example and its scene RON, delete the example it
supersedes (with its `[[example]]` block in `Cargo.toml` and its entry in
`SCREENSHOTS`, `tests/examples_smoke.rs`), and update the producer name in
`scripts/gen-web-screenshots.py` plus the `screenshots/` roster in
`web/src/wiki/dev/development.md`.

- [x] Shared photo kit, landing with the first scene: a three-light photo rig
      (key + rim + fill, spawned by the example), the Kenney hull section lists
      lifted from `assets/base/scenarios/menu_scrapyard.content.ron`, and a
      near-field asteroid dressing helper. Bounded to those three things.
      It CANNOT be `examples/screenshots/kit.rs` - see Notes.
- [x] `screenshot_scene` ("Drydock drift"): planetoid at a distance where its
      surface reads, near-field rocks roughly 15-60 units out with radius
      variance, a hero Kenney racer posed foreground, two more hulls drifting
      on AI `orbit`. Supersedes `screenshot_reel`. First, because it settles the
      look every later scene inherits. -> OWNER APPROVAL.
- [x] `screenshot_combat` ("Rock hollow"): PROVE the two-faction fight first
      (an AI flight with `allegiance: Some(Player)` against a default-Enemy
      flight, no player) - if they will not engage, bring it back before
      building the set. Then the dense field, both flights with `engage_delay`
      so they arrive, and a player ship for the lock/HUD framings. Supersedes
      the old `screenshot_combat` and `screenshot_juice` (its scripted section
      blow moves here). -> OWNER APPROVAL.
- [x] `screenshot_flight` ("The ring"): gravity planetoid, player Kenney racer
      on the ORBIT autopilot, rocks along the ring for parallax, HUD on.
      Supersedes `screenshot_orbit`. -> OWNER APPROVED 2026-08-05, and it takes
      `feature-autopilot` and `wiki-flight` off `screenshot_combat` with it.
- [x] `screenshot_sections`: keep the frozen five-section ship (these document
      the ENGINE section prototypes, not Kenney hulls) and give it the kit's
      rig, tuned for macro work - a rim light carrying the silhouette. Re-frame
      all five closeups; the turret's yaw/pitch/barrel stack is the hard one.
      -> OWNER APPROVED 2026-08-05. The rig was simply missing here (this scene
      never got `kit::photo_rig()`), and the re-frame became a TURNTABLE: the
      camera holds one bearing inside the rig's good wedge and the SHIP yaws to
      present each section, because orbiting a camera around a world-space rig
      lights each of the five differently. The turret's authored rotation was
      left alone - it reads correctly from this bearing, but by tuning, not by
      construction.
- [x] `screenshot_ui`: add a settings-pane state and a Scenarios-picker state
      with a campaign expanded (copy the real pointer gesture from
      `examples/ui/menu_scenarios.rs`), and put a built ship in the editor
      state. -> OWNER APPROVED 2026-08-05. The whole walk moved to real pointer
      gestures, and each shot asserts the state it claims (panel laid out,
      chapter marked `Selected`, five sections on the preview ship). The
      editor's ship could not be built until the camera was PINNED with a
      `ScriptedCameraPose`: the free-fly controller rewrites the transform every
      frame, and from the editor's dead-on default pose every side face is
      edge-on, so every placement landed on the face the original camera saw.
      Ships `wiki-settings` and `news-090-scenario-campaigns` as well, both now
      in the manifest. Left alone, outside this task: the keybind rows render a
      tofu box between the keyboard and gamepad columns, in game as in the shot.
- [x] `screenshot_nova_os`: terminal state with a command run and inline
      completion, plus an apps state so it reads as more than a prompt. Decide
      whether the old fidelity-comparison beats still have a reader.
      -> OWNER APPROVED 2026-08-05. The terminal shot carries the `help` and
      `ship view` output plus the `lo`->`log` completion ghost; the apps shot is
      the ship schematic with all five sections labelled, CTL-1 selected and its
      detail panel open. The range ship gained a turret and a torpedo bay:
      three blocks in a line made a poor schematic and never showed the PDC/TRB
      cockpit codes the page's copy names. Fidelity beats: KEPT, their captures
      DROPPED. The four old names are referenced nowhere in `web/src/`, only in
      the closed fidelity task (20260726-180807), but the welcome and map beats
      are what exercise the map app and the RTT/wgsl schematic path, so a render
      panic there still fails the run. The ship app is left on its default CTL-1
      selection: `]` re-centres the orbit on the selected section, trading the
      whole-ship read for a closeup.
- [x] ONLY NOW, screenshots: frame the 27 beats across the six approved scenes,
      capture into staging, package with
      `python3 scripts/gen-web-screenshots.py`, review every image at its PAGE
      CROP (the site uses `aspect-ratio: 16/9; object-fit: cover`), fix
      rejected framings in the producing example's beat and recapture - never
      hand-fix a PNG - then commit the assets and open the rendered site.
      -> DONE 2026-08-05. 29 figures (the 27 planned plus the two the NOVA OS
      scene grew), captured from the six approved scenes into `target/reel` and
      packaged; the report now lists zero `capturable` gaps. The page crop is a
      no-op for these: every figure is 1920x1080 into a 16/9 `cover` box, and
      the script hard-errors on any other shape. No page markup changed - the
      site's placeholders swap themselves for the real image at runtime once the
      asset resolves (`web/src/site.ts`). No framing was rejected. Flagged, not
      fixed here: the screen-indicator labels collide in four shots
      (`wiki-radar`, `tutorial-radar-lock` stack WAYPOINT over its own distance
      readout; `feature-autopilot`'s FLIP label sits under the blip;
      `wiki-flight`'s SURVEY chip overlaps the debug fps/version chip). That is
      in-game HUD layout, identical in play, not a framing miss - it wants its
      own task.

## Definition of Done

- The coverage report lists zero `capturable` gaps; what remains outstanding is
  `manual` or `historical`.
  (cmd: `nix develop --command python3 scripts/gen-web-screenshots.py --report`)
- Every shipped game-rendered image names one declared producer example in the
  manifest; no image is an alias of another (`ALIASES` is empty).
  (cmd: `nix develop --command python3 scripts/gen-web-screenshots.py --report`)
- The six producers are cataloged, smoked and reach `Playing` headless without a
  panic; `screenshot_reel`, `screenshot_orbit` and `screenshot_juice` are gone.
  (test: `catalog_matches_disk`)
- The owner approved all six scenes, one at a time, BEFORE any capture ran.
  (manual: run each example plainly with no `NOVA_REEL` and confirm the look)
- The owner accepts the new shots at their actual page crop.
  (manual: inspect the locally rendered landing, tutorial, news and wiki pages)

## Notes

- Scope is the `capturable` class only. Post-card thumbnails (`manual`) and
  pre-v0.9.0 news figures (`historical`) stay outstanding on purpose.
- Framing fixes land in the example's scene/pose code, never on the PNG.
- Producers are capture-only (`20260804-093910`); `nova_probe` never enters this
  path (`20260802-120045` WONTDO).
- The packaging script and its `--report` flag already exist (`0ff077ff`); this
  task consumes them, it does not build them. Wiring the report into CI as a
  warning-only job is still unowned.
- Planning constraint, found in the code: `catalog_matches_disk`
  (`tests/examples_smoke.rs:120`) treats every `.rs` DIRECTLY under a category
  dir as an example and pins disk == the `[[example]]` catalog. The shared photo
  kit therefore CANNOT be `examples/screenshots/kit.rs`; it lives one level down
  and is pulled in with `#[path = ...] mod`, as
  `examples/sections/turret_section.rs:40` already does.
- Example-side lighting only. Authorable scenario lighting is `20260805-111534`
  and is not a dependency of any child.
- Scene 1 landed as `bb57a9d2` (kit) + `3b5a715f` (set), owner-approved
  2026-08-05. Two calls made against this plan's wording, both from what the
  renders showed:
  - The yard hulls PATROL, they do not `orbit`. A planetoid close enough to
    orbit fills the frame as a wall; at the distance where its surface reads it
    sits outside the orbit band the AI flies to, so an orbiting hull leaves the
    set. It is still a real (weak) well, since these are the gravity figures.
  - No scene RON. `kit::kenney_hull` derives each hull's 18-54 sections from the
    catalog at runtime, so a RON file would be a generated artifact maintained
    by hand - which is what the shipped `menu_*` / `broadside` files are. The
    scene is built in Rust, as `screenshot_sections` already does.
- Scene 2 turned into a two-act flight at the owner's call: the set opens 800
  units out, the player TRAVEL-locks a nav beacon, flies the real GOTO leg, and
  the beacon's own trigger area springs the ambush on arrival (`OnEnter`
  scenario data, so a plain run gets the fight by flying there). Two calls
  against this plan's wording, both from what the leg showed:
  - `feature-autopilot` and `wiki-flight` moved here (the burn and the
    flip-and-burn are beats of this leg), leaving `screenshot_flight` with
    `tutorial-orbit` alone. `ALIASES` is now empty, which is a DoD item.
    REVERSED 2026-08-05 by the owner after scene 4: both are back on
    `screenshot_flight`, which is the roster NOTES.md set out. `screenshot_combat`
    still flies its leg (it is the approach into the hollow, and the flip is where
    the script cuts) but no longer captures anything on it.
  - The radar picks the body nearest the AIM RAY, and asteroids are lockable at
    any range (their signature gate is bypassed by the well/ship branch), so a
    rock four kilometres out steals a travel lock aimed a few degrees off. The
    start sits nearly in line with the beacon for that reason, and the step
    waits on a lock ON THE BEACON so a wrong latch aborts the run by name.
- Scene 2, second pass (owner's polish list). Three changes and what each cost:
  - The script now CUTS from the flip straight to station in the hollow instead
    of flying the brake out - the fight used to open with the player still on
    autopilot. The cut sets the ship down INSIDE the beacon's trigger, so the
    ambush is still the scenario's to spring, and the run is 8 seconds shorter.
    A plain run flies the whole approach; only the capture makes the edit.
  - A friendly torpedo boat (cargo-B, the only hull in the catalog with bays)
    joins the ambush and the script fires a salvo through the production bay,
    guidance and fuze. Three findings, all in the example's comments: the run
    must clear the rock shell (a level shot flies into stone), the blast visual
    is a SOLID 60-unit sphere that swallows any camera close enough to see the
    ship it hit, and a torpedo alone cannot kill a section - the fuze goes 15
    units out and 100 blast damage with falloff leaves 70-100 health standing.
    So the beat is the run and the aftermath, with a scripted section death
    timed to the detonation.
  - The hollow is now a hollow: the rock shell starts at 48 units rather than
    28, which stops rocks landing on the raider and in front of every close
    subject. The field is a horizontal annulus, so the ordnance framings tip the
    lens UP - the only way to get sky behind a subject in this set.
  - OWNER'S PICK, 2026-08-05: `variant-juice-torpedo` and
    `variant-juice-aftermath` both SHIP, as `wiki-combat-torpedo` and
    `wiki-combat-aftermath` - two new figures on `web/src/wiki/combat-weapons.md`
    (the salvo in flight under Torpedoes, what the blast left under Damage
    types). `variant-combat-tight` stays a variant.
  - Variants, not extra images: the ordnance frames were staged by a capture run
    and named by no
    manifest entry, so they are candidates for `feature-juice` / `wiki-combat`
    to be picked at step 8 and cost nothing if they lose.
- Scene 3, `screenshot_flight` ("The ring"), built and APPROVED.
  What it cost, and the one open decision:
  - MEASURED the draw factor instead of assuming it: this planetoid's derived
    `BodyRadius` is ~91 units for an authored 20, a factor of 4.5. The first cut
    put the ring at 110 units, which is 19 units off a surface that big, and the
    shot was a frame of rock. Ring is now 320. The same factor applies to the
    scatter rocks, so an authored 2-7 is a field of 9-32 unit boulders - the
    debris band is authored 1-3 and sits OUTSIDE the ring, since a band inside it
    walls off the one thing the set is about.
  - Camera height decides where the body lands in frame: above the ring plane
    throws it to the top corner, below drops it out of frame, level puts it
    behind the subject. Every framing here is that dial.
  - The game's own follow camera cannot shoot an orbit - it looks down the track,
    and the body is 90 degrees off it. `tutorial-orbit` is therefore posed (high
    outboard quarter, hull up front, ring and spoke sweeping to the planetoid),
    which is a change from the old `screenshot_orbit`'s follow-camera shot.
  - ORBIT is a HELD state, so its frames are of a ship sitting on a ring. At the
    owner's call the scene grew a second act: the ship drops the ring for a real
    GOTO out to a survey beacon over the pole (polar so the path clears the body
    from every point on the ring), which is where the burn and the flip-and-burn
    come from.
  - The leg is shot by a camera that FLIES it (`LegCamera` + `drive_leg_camera`,
    re-solved every frame off the ship's live position and track), not by pinned
    poses. A pose cannot hold a transfer: the ship crosses at 65 u/s, so the
    ~0.3 s a framing beat holds is 20 units of travel, and the first cut of the
    flip beat pinned its camera 22 units ahead of the ship and the ship flew
    through it - an empty frame. Camera BEHIND for the departure burn (drive at
    the lens), AHEAD for the flip (braking fires down the track).
  - Every leg framing offsets along `lit_side()` - the key light's direction with
    the along-track component removed - rather than a world axis, because the rig
    is direction-only and a maneuvering ship changes attitude for a living. That
    is what fixed the dark hull in the flip frame.
  - LIGHTING IS PHASE-DEPENDENT, and the phase drifts per run: the insertion is a
    real burn whose duration moves with the (per-run, 79-108) derived body radius,
    so the ship is somewhere else on the ring each time and the outboard cameras
    photographed a black planetoid on the runs that landed on the night side. The
    fix is the START phase: the racer is parked on the rim light's horizontal
    bearing (`START_RADIAL`), so every outboard camera looks down the brightest
    lamp on the set at the body's lit face.
  - The tutorial figure's offsets have to be SMALL next to its aim distance. Ship
    and body are ~250 units apart with the camera ~50 off the ship, so every unit
    of lateral offset swings the pair further apart in frame; at `Y * 30` the
    planetoid was cropped off the top edge. It is now outboard + slightly up,
    aimed most of the way to the body: whole body, ring across it, spoke, ship on
    the far end of the spoke.
  - HUD DEFECT, found here and FIXED (owner's call): the screen-indicator widget
    clamped a chip's CENTRE to the inset viewport and only then subtracted half
    its width, so the beacon chip read `VEY 7.87 km` in any frame whose beacon
    was off-screen past a corner. `clamp_box_to_rect`
    (`crates/nova_gameplay/src/hud/screen_indicator.rs`) re-clamps a clamped
    indicator once its box is measured. A `Content` chip uses last frame's size,
    so a readout that changes width is off by that change for one frame.
  - OWNER'S PICK, 2026-08-05: this set takes `feature-autopilot` (the departure
    burn) and `wiki-flight` (the flip-and-burn), and `screenshot_combat` loses
    the two `shoot` calls on its leg. Step 4 is approved. The remaining five
    frames stay `variant-*`. What was staged for the pick:
  - Seven candidates for the flight and autopilot images -
    `variant-autopilot-goto` (the departure burn, ribbon out to the beacon, 250
    m/s), `variant-flight-departure` (clean wide, the lit planetoid with the ship
    crossing its limb under power), `variant-flight-flip` (the flip-and-burn,
    retro plumes lit, 640 m/s) and `variant-flight-arrival` (clean, parked off the
    glowing beacon at the end of the leg) off the leg; `variant-autopilot-ring` (close three-quarter of the insertion
    burn, two plumes lit, `AP ORBIT - BURN` up), `variant-flight-limb` (cinematic,
    the hull over the body) and `variant-flight-chase` (the ring curving away) off
    the ring. If they beat the Rock hollow's GOTO beats, `feature-autopilot` and
    `wiki-flight` move BACK to `screenshot_flight` in the manifest and in the
    Inputs table above.
- The generated rock mesh reaches well past its nominal radius (roughly 4x), so
  a planetoid authored at radius 30 draws a body about 120 units across. Size
  scene bodies from that, not from the authored number.
