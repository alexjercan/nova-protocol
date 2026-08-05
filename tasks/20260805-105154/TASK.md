# Refresh frontend app images: redo the screenshot examples and recapture every capturable web image

- PRIORITY: 70
- TAGS: v0.10.0, web, assets, screenshot
- ACTIVITY: PLANNING
- GATES: -
- RESOLUTION: -
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
| `screenshot_flight` | `screenshot_orbit` | `tutorial-orbit` |
| `screenshot_combat` | `screenshot_combat` + `screenshot_juice` | `feature-combat`, `feature-hud`, `wiki-combat`, `wiki-hud`, `tutorial-combat-lock`, `tutorial-radar-lock`, `wiki-radar`, `feature-juice`, `news-090-combat-readability`, `news-090-contextual-hud`, `feature-autopilot`, `wiki-flight` |
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
- [ ] `screenshot_combat` ("Rock hollow"): PROVE the two-faction fight first
      (an AI flight with `allegiance: Some(Player)` against a default-Enemy
      flight, no player) - if they will not engage, bring it back before
      building the set. Then the dense field, both flights with `engage_delay`
      so they arrive, and a player ship for the lock/HUD framings. Supersedes
      the old `screenshot_combat` and `screenshot_juice` (its scripted section
      blow moves here). -> OWNER APPROVAL.
- [ ] `screenshot_flight` ("The ring"): gravity planetoid, player Kenney racer
      on the ORBIT autopilot, rocks along the ring for parallax, HUD on.
      Supersedes `screenshot_orbit`. -> OWNER APPROVAL.
- [ ] `screenshot_sections`: keep the frozen five-section ship (these document
      the ENGINE section prototypes, not Kenney hulls) and give it the kit's
      rig, tuned for macro work - a rim light carrying the silhouette. Re-frame
      all five closeups; the turret's yaw/pitch/barrel stack is the hard one.
      -> OWNER APPROVAL.
- [ ] `screenshot_ui`: add a settings-pane state and a Scenarios-picker state
      with a campaign expanded (copy the real pointer gesture from
      `examples/ui/menu_scenarios.rs`), and put a built ship in the editor
      state. -> OWNER APPROVAL.
- [ ] `screenshot_nova_os`: terminal state with a command run and inline
      completion, plus an apps state so it reads as more than a prompt. Decide
      whether the old fidelity-comparison beats still have a reader.
      -> OWNER APPROVAL.
- [ ] ONLY NOW, screenshots: frame the 27 beats across the six approved scenes,
      capture into staging, package with
      `python3 scripts/gen-web-screenshots.py`, review every image at its PAGE
      CROP (the site uses `aspect-ratio: 16/9; object-fit: cover`), fix
      rejected framings in the producing example's beat and recapture - never
      hand-fix a PNG - then commit the assets and open the rendered site.

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
  - `feature-autopilot` and `wiki-flight` MOVE HERE (the burn and the
    flip-and-burn are beats of this leg), leaving `screenshot_flight` with
    `tutorial-orbit` alone. `ALIASES` is now empty, which is a DoD item.
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
  - Variants, not extra images: `variant-juice-torpedo`, `variant-juice-aftermath`
    and `variant-combat-tight` are staged by a capture run and named by no
    manifest entry, so they are candidates for `feature-juice` / `wiki-combat`
    to be picked at step 8 and cost nothing if they lose.
- The generated rock mesh reaches well past its nominal radius (roughly 4x), so
  a planetoid authored at radius 30 draws a body about 120 units across. Size
  scene bodies from that, not from the authored number.
