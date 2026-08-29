# Round 5: polish audit - visuals, numbers, and the terminal

Owner round, 2026-08-28. This round is INTERNAL, a deviation from the record's
external-prior-art charter: the owner asked for a codebase sweep that finds
"most possible improvements we can make to have a more polished experience" -
visual details, missing stats surfaces in the editor and in flight, and a
position on the NovaOS terminal. Five parallel read-only lanes; every
file:line below was verified against master at 89091f2e.

The presentation is `polish-audit.html` beside this file. This file is the
evidence record; the page ranks and groups the same findings.

## The reported bug, confirmed: wrecks reset to pristine

Owner: "when a section dies and gets split from the ship, its material resets
to the pristine one; I would like to keep the burnt or destroyed material."

Root cause. `grade_section_cracks` treats a MISSING `DamageLevel` as
pristine. The death cascade despawns the section entity but reparents its
mesh children onto the wreck piece with their `SectionCracks { section }`
still pointing at the despawned entity. The next `Update`, the lookup misses,
`map_or(0.0, ...)` reads 0.0, and the "healed back to pristine" branch swaps
the clean source material back on.

- Reset site: `crates/nova_ship/src/sections/damage_cracks.rs:434-450`.
- Death cascade: `nova_gameplay/src/integrity/health.rs:145-150` ->
  `integrity/core.rs:179-196` -> `integrity/explode.rs:271-374` (reparent at
  :357-361, despawn at :373).
- Permanent: nothing removes `SectionCracks`, and `mark_section_meshes`
  excludes meshes that already carry it (damage_cracks.rs:324-336).
- The module doc claims the opposite behavior (damage_cracks.rs:7-8) and no
  test covers a despawned section (test list :556-1031).
- The codebase already contains the correct pattern for the same orphan case:
  the plume grader reads a missing section as "dead drive, plume out"
  (`thruster_section.rs:615-621`). Cracks makes the opposite choice with the
  same 0.0.

Compounding: the burnt tier is unreachable even before the reset. The killing
damage lands in `FixedPostUpdate` (rounds.rs:109-113) and the destroy cascade
completes in the same flush, so `derive_damage_level` (`erosion.rs:80-91`,
`Update`) never writes the final value and `grade_section_cracks` never sees
it. The shader's burnt term needs damage >= 0.78 (`section_cracks.wgsl:59-63`),
which only buckets 6 and 7 reach - so a section killed from above ~22% health
never renders burnt, and a one-shot kill leaves a textbook-clean wreck for its
whole 30 s life. The `BURNT_COLOR` comment describes a state the pipeline
cannot currently produce.

Fix shape: on detach, latch the wreck's meshes at their last bucket or force
the top bucket (bucket 7 reads as charred wreckage), and remove or retarget
`SectionCracks` so the per-frame grade loop stops iterating dangling entities.
Add the missing wreck-material test.

## Damage and wreck fidelity (beyond the bug)

- Sparks stop dead at the moment of death: `throw_damage_sparks` queries the
  section entity (damage_sparks.rs:170), which is despawned. A section
  sparking at max rate goes silent the instant it comes off. A wreck that
  keeps sparking or smoldering for a few seconds would sell the kill.
  Cross-ref: effect-family tuning belongs to 20260822-204201; the wreck
  CARRIER for effects does not exist yet and is this finding.
- Cladding and greebles never show damage: `owning_section` stops at
  `SectionFixture` (damage_cracks.rs:283-285), so a wreck wears perfectly
  clean skin plates.
- Wrecks vanish by hard despawn: `TempEntity(30.0)` (explode.rs:342,
  lifetime.rs:93-115). No fade, shrink, or dissolve; the pop is visible.
- The hit ring is a shockwave in vacuum, drawn in the wrong place. Impact
  feedback is a 2 px default-config gizmo ring, z-tested against the hull it
  just hit and LDR so it never blooms (juice.rs:599,641). It anchors at the
  hit entity's ORIGIN, not the impact point, and cannot do better:
  `HealthApplyDamage { entity, source, amount }` carries no position
  (integrity/health.rs:78-86, juice.rs:546-550), so the contact point is
  dropped before the cue fires. Owner feedback (2026-08-29, artifact comment):
  an expanding radial ring reads as a pressure wave, which the vacuum
  direction forbids, and the particle families already carry the impact - cut
  the ring rather than upgrade it, and move the per-hit cue to the contact
  point inside 20260822-204201, whose family inventory already owns the
  juice.rs rings. The module doc's "per-section emissive flash"
  (juice.rs:21-23) remains a valid non-shockwave successor.
- Stale comments: `fixture.rs:28-31` references `grade_section_tints`, which
  no longer exists; `nova_probe/src/capabilities/census.rs:238-241` claims
  cladding gets the cracks material and that the `StandardMaterial` is always
  replaced - wrong on both counts.
- The heal branch's only real caller today is the death path, where it does
  the wrong thing: no repair mechanic exists outside the NovaOS `ship repair`
  action (nova_os_ui/src/ship/sections.rs:560) and a probe.

## Light and atmosphere

- No image-based lighting anywhere. Zero hits for `EnvironmentMapLight` /
  `GeneratedEnvironmentMapLight` / `LightProbe`. The 4096 cubemap is backdrop
  only. Hull styles are authored near-dielectric (metallic 0.0-0.15,
  base.content.ron:15,26,327,338,563,574) - consistent with, and likely a
  workaround for, real metal rendering black without IBL. Bevy 0.19 ships
  `GeneratedEnvironmentMapLight`, which can derive IBL from the existing
  skybox handle; the insertion point is one place
  (nova_scenario/src/loader/lifecycle.rs:253-266). Largest single visual
  lever found in this round.
- Ambient light is the Bevy default (white, 80 lux) because nothing sets it.
  This contradicts the design claim "a scene that authors nothing renders
  black" (lifecycle.rs:268-270, objects/light.rs:2-3). An uncontrolled white
  wash sits under every authored three-point rig. One line to fix.
- The three-point rig aims at a fixed world point (light.rs:88-102,
  :117-142), so the ship's key/rim relationship drifts as the player flies
  and can end up flat-lit or backlit. A still-life rig for a flying game;
  consider camera- or ship-relative rim.
- Post chain is stock: `Tonemapping::TonyMcMapface` + `Bloom::NATURAL`
  verbatim (nova_ship/src/camera/post.rs:71-72). No color grading, no
  SSAO/AA/fog choices anywhere. Skybox brightness is the literal 1000.0 in
  three places (skybox.rs:76, lifecycle.rs:265, actions/view.rs:147-160) and
  not authorable per scenario, though the two shipped cubemaps differ.
- Two skyboxes total, raw PNG (about 402 MB VRAM each uncompressed); the
  pipeline already accepts ktx2/basis (mod_refs.rs:314). The tested
  `SetSkybox` action is used by no shipped scenario.
- No allegiance tinting exists in the world: `Allegiance` reaches no
  material system. Friend or foe is a HUD-triangle fact only; two ships in
  the same style are pixel-identical.
- Two live emissive+unlit dead materials in the editor previews:
  `nova_editor/src/preview.rs:143-147` (beacon) and :168-172 (light bulb) -
  the exact defect target_inset.rs fixed and documented (:249-255). Round 2
  recorded the target_inset instance; it is now fixed, the editor copies are
  not.
- Placeholder part materials are pure albedo; the nozzle documented as "hot
  red" (placeholder_art.rs:62) carries no emissive and never glows.
- Per-entity material mints: one `StandardMaterial` per salvage crate
  (salvage.rs:247-251) though all crates pulse on one shared clock; one
  material per asteroid (asteroid.rs:409-418) though only the mesh needs to
  be unique for carving. One rock texture in the whole game (asteroid.png,
  736x736).
- Camera shake is white noise resampled per frame - frame-rate-dependent
  character, reads as buzz; the code names the problem itself
  (juice.rs:114-116) and the response was to shrink amplitudes rather than
  switch to smooth noise (shake.rs:309-314).
- Nothing communicates velocity except the speed number and a 3-unit camera
  lean (framing.rs:114-117; framing is deliberately speed-invariant,
  :79-95). No ambient dust, no near-field parallax, no FOV response. A cheap
  camera-local dust layer is the classic fix and does not collide with
  20260822-204201's effect families.
- No scene-level fade anywhere: menu -> game is a raw state set plus
  despawns; the menu backdrop's first entry has a documented one-frame blank
  (ambience.rs:198-200). Loading screens are finished work by contrast.
- Settings built but unwired: `JuiceSettings` per-effect `enabled` flags
  exist "so a settings menu can bind to it later" (juice.rs:31-32); no HUD
  scale, no shake toggle, no brightness in the Graphics tab
  (settings.rs:480-509). Cross-ref: settings task 20260824-120527.
- Free-fly WASD camera has no smoothing, with the comment admitting it
  (wasd.rs:202-210); it frames the menu backdrop and the editor.

## HUD and UI consistency

- Every HUD size is a fixed logical-pixel literal (edge_indicators.rs:21-36,
  allegiance_markers.rs:66-85, turret_lead.rs:21, lock_crosshairs.rs:59);
  no `UiScale` is written anywhere, and `PersistedSettings` has no HUD-scale
  field. Same 24 px arrow on 1024x768 and 4K. The `screen_indicator` widget
  already offers `ApparentSize`/`WorldRadius` scaling; most consumers pass
  `Fixed`.
- Fixed chip offsets can collide: speed and mode chips 24 px apart
  (flight_status.rs:44-48); marker and chip offsets constant regardless of
  range; two hostiles at similar bearing stack with no de-clutter pass.
- Nine slightly-different combat reds and three ambers across nine files;
  the theme centralizes `semantic::THREAT` and admits the drift
  (theme.rs:103-106).
- Three bare `TextShadow::default()` uses in nova_editor (plate.rs:112,
  keybind.rs:150, callout.rs:81) - exactly the 4 px hard-shadow ghosting the
  nova_ui policy forbids and tests against (button.rs:718-721).
- The target inset pays full bloom + tonemapping on a 512 px RTT shown at
  256 px, and shows targets against a black void while the main view shows a
  nebula (target_inset.rs:63-66,565-579).

## The editor's numbers

Structure: the inspector is reflection-driven; `walk()` flattens a config,
`curate()` picks the first screen (inspect.rs:662-774, :1375-1401). Two
structural gaps drive most misses:

- `section_rows` walks only the KIND config; `BaseSectionConfig` is never
  walked, not even under All Fields (inspect.rs:1521-1526). So health,
  collider (mass and footprint), link points, name, description are
  invisible in the inspector; the gallery card shows most of them instead
  (gallery/catalog.rs:123-212).
- `ShipNode.pilot` (`AIControllerConfig`: patrol, orbit, leash, engage_delay,
  engage_range, pd_range, waypoint_slack, arrival_standoff;
  spaceship.rs:104-183) has NO UI anywhere, while a seeded-hull scenario
  object exposes the same config through its `CONTROLLER` pick - an
  asymmetry, not a policy.

The owner's example, confirmed: turret `ammo_capacity` and `reload.delay` /
`reload.amount` are All-Fields-only, unitless, with a 0.1 drag step on
integers (config.rs:190, ammo.rs:91-93). Same for the torpedo bay, plus its
whole `torpedo_type` block (name, tint, max_speed, weave) is All-Fields-only
(mod.rs:179,253-306). The inspector and the gallery focus card are exact
complements of each other's gaps: the card shows Ammo/Reload/HP but not fire
rate or lifetime; the inspector the reverse.

Derived stats: the rail readout already shows Turn / Mass / Thrust / HP /
Parts with the limit note (readout.rs:25-153) - the right register, hidden at
the scenario node. Not shown anywhere: thrust-to-mass or linear accel, total
DPS, effective range, total ammo, tube count, center of mass, max speed.
Every formula already exists in `nova_authoring/src/balance.rs` (dps :212,
effective range :213-215, tubes :217, TTK :411-414) - CI-only today.

Units:

- Thruster `magnitude` is impulse per fixed tick, not force
  (thruster_section.rs:474-483); the editor labels it with "" and nothing on
  screen says per tick. `max_torque` unitless while derived Turn is rad/s2.
- Radians vs degrees inside one panel: pose rotation is degrees and labeled;
  config `Quat`s are degrees unlabeled (Text rows are excluded from unit
  assignment, inspect.rs:284-292); turret joint speed/min/max are raw
  radians unlabeled (config.rs:70-82).
- `mass` on anchors/asteroids is the gravitational parameter mu in u^3/s^2
  per its own docs, shown unlabeled as "Mass" - the same word the rail uses
  for hull mass.
- No `range` field exists on a turret; reach is muzzle_speed x
  projectile_lifetime and the editor never shows the product.
- The editor accepts values the spawner rejects: reload.delay floored at 0.0
  by the `DELAY` spec but `SectionReload::from_config` debug-asserts
  delay > 0 (ammo.rs:117-125).
- Free-limit integer fields drag by 0.1 and snap back (inspect.rs:1176-1240).

Also unexposed: `SectionNode.modifications` (SetHealth, SetAmmo, Rename,
DisableVerb - node.rs:180-182, "the editor authors none yet"),
`ShipHull.collapse_threshold` hard-coded to default (scenario.rs:474-481),
scenario root shows only three read-only counts, asteroid/beacon
`lock_signature` All-Fields-only, Directional/Point light kind not
switchable.

## Diegetic stats: computed every frame, never shown

Exactly nine numeric readouts exist in flight (speed, AP mode, ETA+distance,
flip countdown, orbit radius, target DST, target CLS, edge-label distances,
FPS). The simulation computes far more. Ranked promotion candidates, each
with its authoritative source:

1. Own-ship integrity. `aggregate_ship_health` writes a root `Health` every
   frame (sections/integrity.rs:623,706); there is NO own-ship HP element in
   the HUD at all - only the locked target gets a bar.
2. Turn rate and its limit. `ship_turn_rate()` (guidance.rs:322) is consumed
   every frame by input slew; `AttitudeEnvelope` (attitude.rs:56-121) knows
   the ceiling and `AttitudeLimit::label()` knows WHY ("torque-limited" /
   "structure-limited"); `None` is a first-class ADRIFT state. Written "for
   a build screen that has to say WHY"; flight deserves the same.
3. Time-to-intercept. `lead_intercept_point()` solves t and throws it away
   (turret_section/aim.rs:~150); a TTI chip on the lead pip is free.
4. On-target angle. `muzzle_aim_error()` per tick (aim.rs:54), threshold
   TURRET_ON_TARGET_RAD ~0.92 deg.
5. Brake/stop data. `ManeuverTelemetry.brake_accel` carries its own doc "No
   HUD instrument reads this yet" (flight/state.rs:210); arrival curve
   v = sqrt(2 a margin d) gives brake distance.
6. Lock-drop reason. `CombatLockDropped { reason, idle_secs }`
   (targeting/state.rs:264-284) is a fully-formed diagnostic the player
   never sees; an ideal comms/log line.
7. Speed cap. `FlightSpeedCap` (flight/state.rs:40) - the player cannot see
   their own rated max speed; the soft taper (manual.rs:83-88) makes it a
   real "rated velocity".
8. Gravity. `GravityWell { mu, body_radius, soi_radius }` (gravity.rs:57) -
   direction sphere only today; magnitude and SOI distance are free.
9. Torpedo arming. `TorpedoArming.is_armed()` (torpedo_section/mod.rs:608) -
   point-blank duds are currently unexplained to the player. Gameplay
   legibility, not just flavor.

Notes for the implementer: the scenario readout strip
(nova_hud/src/readout.rs) is the cheapest existing numeric surface; the chip
recipe is `nova_ui::hud::{chip_node, text_chip}` + `nova_ui::units` +
`screen_indicator` + `HudTier::Instrument` + a `HudContextGate` from
`HudSituations`. Closing speed is computed but deliberately omitted from the
destination readout after playtest (maneuver_instruments.rs:218-224 comment);
exact ammo counts are deliberately pips-only (ammo_readout.rs:29-35). Both
decisions predate this round; do not steamroll them - route the numbers to
NovaOS instead (see below). `nova_info` is only the version string; the name
suggests a home it does not provide.

## The terminal: read and recommendation

Facts. NovaOS is an in-flight surface only (Tab, needs a live player ship;
the main menu is ordinary buttons with a NOVA OS footer label). Opening
hard-pauses the game and hides the whole HUD. 14 commands. The fiction is
carried hard: NOVACRT 9000 casing, boot POST, CRT shader, chin knobs, sound
set, a 334-line wiki page. Five capabilities are terminal-ONLY: autopilot to
a contact entity, section repair/reload, per-section rebinding, the full
flight log, the detailed objective list. Apps launch only by typing `map` or
`ship` - `enter_app` has no production caller - so a gamepad, which can open
the terminal and drive both apps, cannot reach them. Known frictions:
backquote both types and cycles HUD visibility (hud_cinematic is
Context::Always, nova_hud/src/lib.rs:406-411); Ctrl chords are swallowed but
unimplemented; no Home/End; first-open boot costs ~0.8 s.

Opinion, for the owner to weigh:

The terminal is not the gimmick; it is the identity. The whole UI already
speaks NovaOS - phosphor chips, CRT loading screens, the menu footer. What
makes it feel "out of context" is not that you type; it is that the game
STOPS and the HUD VANISHES, so using the ship's computer feels like leaving
the ship. The fix is to close the gap between HUD and terminal, not to
retire either.

1. Own the pause. Opening the terminal IS a tactical pause - fiction-friendly
   (the pilot looks down at the second seat). Keep it, but stop hiding the
   HUD behind it; dim it instead, so the world context stays.
2. Apps without typing. Put MAP / SHIP / LOG / HELP on the bezel as
   clickable function keys (F1-F4 row on the casing chin) and give the
   gamepad an app row. Typing stays the power path; the commands stay the
   fiction.
3. Give the nerd numbers a home: an `eng` app. The editor's stat_block
   (Turn / Mass / Thrust / HP / Parts + limit note) rendered live, plus the
   damage model, per-section HP table, envelope ceilings, gravity readout.
   This answers the owner's sciency ask AND makes the terminal genuinely
   useful: HUD gets glanceable chips, NovaOS gets depth. The two deliberate
   HUD omissions (closing speed, exact ammo) belong exactly here.
4. Echo results out. A command's outcome should leave a trace on the HUD
   after close (the comms ticker already exists); `map goto` already does
   this via the AP chip - make that the pattern.
5. Docking is the payoff. The stations task (20260824-125943) needs an
   interaction surface; NovaOS pages ("DOCKING", "TRADE") make stations feel
   like the same operating system the ship runs - the terminal stops being a
   side feature and becomes the game's interaction OS.
6. Sweep the frictions: the backquote conflict, Ctrl-A/E/U/K, Home/End,
   click-to-position, boot skip on first keypress.

Distinct from 20260827-120347 (CSGO-style console, action vocabulary): that
is out-of-fiction tooling; overlap today is essentially zero and should stay
that way. The console can reuse `nova_os::shell`'s matcher (UI-free).

## Mechanics observations (from the code, not asked for but requested)

- Closing-speed damage scaling (damage.rs:229) is invisible; surfacing it
  (a "kinetic bonus" readout or comms line) turns a hidden formula into a
  play axis.
- Point-defence assignment (ownership.rs:57-90) is invisible; ties into
  round 3's combat-mode question - mode legibility was its central issue.
- Repair/reload are free and instant via NovaOS; if stations land, they are
  the obvious resource/service hook.
- `SetSkybox` and per-scenario skybox brightness are finished machinery no
  content uses; one dusk-lit scenario would pay for them.

## What this round does not repeat

- Particle families and their vacuum roles: 20260822-204201 owns them; this
  round adds only the wreck-as-effect-carrier gap and the gizmo-ring flash.
- The console/action vocabulary: 20260827-120347.
- Audio: 20260824-125955.
- Combat-mode UX: round 3 of this record.
- Editor objectives/events: 20260825-223024 / 20260825-223035.
