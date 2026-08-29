# Round 6: remaining polish review

Owner round, 2026-08-29. This is the follow-up to `POLISH-AUDIT.md`, verified
against master at `c8b432cf` after the first bounded fix batch landed. It does
not repeat the original evidence. It records what closed, what changed under
the audit, and the order of the remaining work from simplest to hardest.

The presentation is `polish-review-2.html` beside this file.

## Delta since round 5

Commit `c8b432cf` closed the first six work items:

- Detached wreck meshes latch the burnt crack tier, including one-shot kills,
  and drop `SectionCracks` instead of grading a despawned section forever
  (`nova_ship/src/sections/damage_cracks.rs`, regression
  `wreck_art_stays_burnt_after_its_section_is_despawned`). This closes the
  pristine reset, unreachable burnt tier and dangling grader finding together.
- Beacon and light editor previews no longer combine emissive with `unlit`
  (`nova_editor/src/preview.rs`).
- Plate, keybind and placement-callout text no longer uses Bevy's default 4 px
  shadow (`nova_editor/src/ui/{plate,callout}.rs`, `nova_editor/src/keybind.rs`).
- Turret and torpedo first screens expose magazine capacity and reload delay /
  amount as whole rounds and seconds (`nova_editor/src/inspect.rs`).
- Reload controls refuse zero delay and zero rounds before the runtime assertion
  can see them (`inspect.rs`; test
  `weapons_open_on_valid_ammo_and_reload_controls`).
- The fixture and census comments now describe the material pipeline that
  exists (`nova_ship/src/sections/fixture.rs`,
  `nova_probe/src/capabilities/census.rs`).

Two round-5 claims changed independently before this review:

- Graphics quality is no longer unwired. Low / Medium / High now drive camera
  shake, hit flashes, particles and internal render scale
  (`nova_gameplay/src/settings.rs:170-255,356-389`; menu at
  `nova_menu/src/settings.rs:480-519`). There is still no independent shake,
  HUD-scale or brightness control.
- Both cubemaps are used as scenario backdrops: Broadside, Broadside Gunship,
  Lifeline and Final Tally use `cubemap_alt.png`. The dynamic `SetSkybox` action
  still has no authored caller (`assets/base/scenarios/*.content.ron`; no
  `SetSkybox` under `nova_authoring`).

## Remaining work, simplest first

Effort is implementation plus the cheapest honest validation, not line count.
`S` is a bounded change with local tests. `M` crosses a subsystem or needs a
visual pass. `L` changes a format, content pipeline or interaction model.

### 1. Bounded corrections

These can be taken independently. They do not need a visual direction pass.

1. **Gate the Backquote HUD action while NovaOS owns text input - S.** The
   `hud_cinematic` binding is still `Context::Always` on Backquote
   (`nova_hud/src/lib.rs:407-411`), while terminal input accepts printable
   Backquote. Keep the default binding; context-gate the action so typing cannot
   cycle the HUD. This is the clearest remaining correctness defect.
2. **Finish terminal editing basics - S-M.** The shared UI text field already
   implements Home / End (`nova_ui/src/widget/text_field.rs:347-348`), but
   NovaOS terminal input does not. Reuse those caret semantics, add the swallowed
   Ctrl-A/E/U/K chords, and let the first deliberate key skip the boot delay.
   Keep click-to-position as a pointer follow-up; it needs glyph-to-caret mapping
   and is not the same small keyboard change.
3. **Render the target inset at display resolution - S.** It renders a 512 px
   texture into a 256 px panel and runs the full marked-camera post chain
   (`nova_hud/src/target_inset.rs:63-66,562-577`). Start at 256 px. Keep bloom
   only if a screenshot comparison proves it buys readable target damage at
   that size.
4. **Make the placeholder hot nozzle emissive - S.** The fallback art calls the
   nozzle hot red but gives it only albedo
   (`nova_ship/src/sections/placeholder_art.rs:61-76`). This is low reach -
   authored parts replace it - but the intended value is unambiguous.
5. **Consolidate combat color tokens - S.** The remaining red literals still
   drift across edge indicators, lead pips, lock crosshairs, target focus,
   component lock and the inset (`nova_hud/src/{edge_indicators,turret_lead,
   lock_crosshairs,torpedo_target,component_lock,target_inset}.rs`). Preserve
   alpha and separate meanings; centralize the shared RGB families. This is a
   consistency change, not a request to make every warning the same opacity.
6. **Show combat-lock drop reasons in the existing log/comms path - S.**
   `CombatLockDropped` already carries TargetGone, OutOfRange, AllegianceFlip
   and IdleDecay (`nova_ship/src/input/targeting/state.rs:284` and
   `contacts.rs:337-350`). A short line explains a disappearing lock without a
   new HUD instrument.
7. **Show rated speed where speed is already shown - S.** `FlightSpeedCap` is a
   live component and the current speed chip already owns the register
   (`nova_ship/src/flight/state.rs:40`; `nova_hud/src/flight_status.rs`). Prefer
   a compact `current / rated` readout over another chip.

Recommended first cutoff: items 1-7. They are bounded and mostly reuse an
existing surface. The target-inset and placeholder changes still need one
rendered editor/game inspection before landing.

### 2. High-value medium work

8. **Install an explicit lighting baseline - M, highest visual leverage.** The
   scenario camera still gets the global Bevy ambient default, despite the
   loader saying an unlit scene renders black
   (`nova_scenario/src/loader/lifecycle.rs:253-270`; Bevy 0.19
   `GlobalAmbientLight::default` is white at brightness 80). Put a zero
   `AmbientLight` override on the scenario camera and add
   `GeneratedEnvironmentMapLight` from the same prepared cubemap. Bevy requires
   a power-of-two cubemap and an authored intensity; Nova's prepared cube view
   meets the shape requirement. Treat ambient and IBL as one visual change:
   disabling the wash without replacing the environment light can make the
   authored rigs look worse. Capture at least the two cubemaps, a metallic
   hull, a dielectric hull and an unlit-scene negative case.
9. **Add own-ship integrity to the HUD - M.** The root `Health` is already
   aggregated each frame (`nova_ship/src/sections/integrity.rs:623-706`), and no
   own-ship element consumes it. Put it near the flight status, not beside the
   target health bar, so ownership is not ambiguous.
10. **Add turn authority beside the integrity readout - M.**
    `ship_turn_rate` and `AttitudeEnvelope` already provide the rate and limiting
    reason (`nova_ship/src/flight/guidance.rs:322`; `physics/attitude.rs:56-121`).
    The glance surface should show the rate; NovaOS can own the full
    torque-limited / structure-limited explanation.
11. **Keep context under NovaOS - M.** Opening NovaOS still hides the entire HUD.
    Dim or de-emphasize it instead, then add direct MAP / SHIP / LOG / HELP
    buttons and an equivalent gamepad app row. `TerminalState::enter_app`
    already exists (`nova_os/src/terminal/state.rs:409`); production still
    reaches apps through parsed commands only. Preserve the tactical pause.
12. **Give wrecks an exit instead of a pop - M.** Detached pieces still carry a
    fixed 30 s `TempEntity` and hard-despawn
    (`nova_gameplay/src/integrity/explode.rs:75,342`). Add a final fade or
    dissolve interval. Do not make generic `TempEntity` fade: most temporary
    entities are not renderable wrecks.
13. **Correlate camera-shake samples - M.** The trauma and drift-free restore
    model is sound, but Apply still samples three independent random values per
    frame (`nova_gameplay/src/shake.rs:271-318`). Store a phase or noise state
    per camera and sample a smooth curve. Validate at 30, 60 and 144 Hz so the
    character is time-based rather than frame-based.
14. **Expose section base, AI pilot and torpedo-type groups in the editor - M.**
    `section_rows` still walks only `section_config(&config.kind)`
    (`nova_editor/src/inspect.rs:1893-1930`), so health, collider/mass, links,
    name and description never appear. `ship_rows` deliberately carries but
    hides `ShipNode.pilot` (`inspect/tests.rs`,
    `a_seeded_hull_reads_as_a_ship_rather_than_as_a_spawn_config`), and the
    torpedo type remains under All Fields. Add curated groups; do not flatten
    any of these trees onto the first screen.
15. **Complete the editor unit pass and derived readout - M.** Magnitude,
    max-torque, raw joint radians, gravitational mu and effective weapon range
    still mix units or omit them. Reuse `nova_authoring::balance` for DPS,
    range, tube count and TTK instead of duplicating formulas. Put the existing
    Turn / Mass / Thrust / HP / Parts block on ship nodes, where it describes
    the selected design.
16. **Add scene-boundary fades - M.** Loading screens fade, but menu-to-game and
    first menu-backdrop entry still transition by state set and despawn. A
    single screen-space fade owned by state transitions is cheaper and more
    consistent than teaching each scene to fade itself.
17. **Make free-fly motion time-based and smoothed - M.** The WASD camera writes
    raw target state directly (`nova_ship/src/camera/wasd.rs:202-210`). Add
    exponential position and angle smoothing with delta time; use the same rig
    for editor and menu backdrop so tuning has one owner.

### 3. Larger visual and HUD systems

18. **World-space friend-or-foe.** Allegiance still reaches HUD markers but no
    ship material. Add a small authored trim/running-light channel rather than
    recoloring whole hull materials. This needs a style/content contract and is
    `M-L`, not a material-system one-liner.
19. **Wreck effect carriers and damaged cladding.** Sparks still query the dead
    section (`damage_sparks.rs:166`), and fixtures intentionally stop crack
    ownership. A short-lived wreck-effects component can smolder after death;
    cladding needs its own authored damage policy. Route effect-family work to
    `20260822-204201`.
20. **HUD scale, comfort controls and de-clutter.** Fixed pixels and fixed
    offsets remain. Persisted HUD scale and independent shake/brightness
    controls are `M`; marker collision avoidance is `L`. Implement controls
    first, then measure whether overlap remains at common resolutions before
    building a general label solver. Graphics quality already disables shake at
    lower tiers, so an independent toggle is a comfort override, not a missing
    performance gate.
21. **Velocity cues.** Camera framing remains deliberately speed-invariant.
    Start with sparse camera-local dust or debris streaks that provide parallax
    without implying an atmosphere. This needs a vacuum-VFX review and belongs
    beside `20260822-204201`.
22. **Camera-relative hero lighting.** Authored key/rim/fill lights still aim at
    fixed world points. Follow the player or camera only for designated hero
    rigs; changing every scenario light into a follower would destroy authored
    staging.
23. **Skybox format and post pipeline.** The cubemaps remain raw PNG and the
    post chain remains `TonyMcMapface + Bloom::NATURAL`
    (`nova_ship/src/camera/post.rs:73`). Convert and inspect KTX2 first. Color
    grading, AO, AA and fog are separate visual decisions and should follow the
    IBL baseline, not land in the same unmeasurable pass.
24. **Material sharing and art variety.** Salvage crates and asteroids still
    mint per-entity materials; all asteroids share one texture. Share assets
    where animation permits, then add texture/roughness variants through the
    content catalog. Do not add runtime randomness to authored identities.
25. **Target-inset world context.** After the resolution fix, decide whether the
    inset should share the skybox/IBL or keep a diagnostic void. A skybox gives
    continuity; a void gives silhouette. Recommendation: shared IBL with a dim
    backdrop, not the full bright main-view sky.
26. **Remaining flight instruments.** Time-to-intercept, on-target angle, brake
    distance, gravity/SOI and torpedo armed state remain computable but unused.
    Add them only where they change a decision. TTI and arming are combat HUD;
    brake distance is autopilot HUD; gravity detail belongs in NovaOS.
27. **Point-defense and kinetic-damage legibility.** Assignment and
    closing-speed damage scaling remain hidden. These are mechanic explanations,
    not generic numbers. Route PD ownership to the combat-mode work in round 3;
    use a short kinetic-bonus cue rather than a permanent formula panel.

### 4. Interaction and content architecture

28. **NovaOS engineering app - L.** Put per-section health, exact ammunition,
    closing speed, gravity, envelope ceilings and the live build readout here.
    It is the depth surface; the HUD remains glanceable. This requires an app
    information architecture, navigation, gamepad path and live-query model.
29. **Docking / trade through NovaOS - L.** Route to station task
    `20260824-125943`. Repair and reload becoming dock services is a mechanics
    decision, not polish cleanup.
30. **Editor leftovers - L.** `SectionNode.modifications`, collapse threshold,
    scenario-root script data, light-kind switching and lock signatures need an
    editor information architecture. Do not solve them by turning All Fields
    into the default screen.
31. **Impact-point damage cue - L in its proper owner.** The ring remains a
    vacuum shockwave at the target origin because `HealthApplyDamage` carries no
    contact position (`nova_gameplay/src/integrity/health.rs:78-86`, ring at
    `juice.rs:642`). Delete the ring and carry contact data through the weapon
    hit path as part of `20260822-204201`; a broad event change touches every
    damage producer and its tests.
32. **Dynamic sky changes as content.** `SetSkybox` is implemented and tested,
    but no authored action calls it. Use it only when a mission beat needs a
    visual transition; do not add a sky swap merely to prove the API exists.

## Recommended scheduling

- **Next bounded polish batch:** items 1-7. One code batch, local tests, one
  target-inset/editor render inspection.
- **Next visual batch:** item 8 alone. IBL changes every material judgment, so
  capture and accept that baseline before tuning post, metals, sky compression
  or hero lights.
- **Next interface batch:** items 9-11. Integrity, turn authority and NovaOS
  context answer the audit's highest-value information gaps without filling the
  HUD with every available number.
- **Existing owners:** item 19/21/31 -> `20260822-204201`; independent settings
  controls -> `20260824-120527`; docking -> `20260824-125943`; audio remains
  `20260824-125955`; the out-of-fiction console remains `20260827-120347`.
- **Backlog after another playtest:** items 12-18 and 20-30. Their order depends
  on what remains visible after IBL and the first interface batch.

## Decision for the owner

There are two sensible next moves:

- **Bounded corrections first (recommended):** items 1-7. Consequence: several
  obvious frictions close with low risk, but the game's overall look does not
  move much.
- **Lighting baseline first:** item 8. Consequence: the largest visual gain
  arrives sooner, but it needs screenshot acceptance and may expose material
  tuning work immediately.

The recommendation follows the owner's stated rule: remove the simplest,
clearest defects first, then make the larger visual baseline the next reviewable
unit.
