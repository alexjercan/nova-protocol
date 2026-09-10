# Close the v0.13.0 review leftovers

- STATUS: OPEN
- PRIORITY: 76
- TAGS: v0.14.0, review, bug, docs

## Goal

Close every finding the v0.13.0 review records reported and did not fix.
Mined 2026-09-09 from `20260905-231735`, `20260908-004345`,
`20260905-001635`, `20260903-000733`, and the nightly review
`20260908-230019`; each item was re-checked against HEAD and is still
open. Line numbers are from that check and will drift.

A bug with a reproducible trigger gets a `bug_` range or a unit test that
fails first, then the fix, one commit per item. Docs items are edited in
place. Perf items are measured before they are changed, and left alone if
the number is flat. Design calls go to the owner as a list, not as edits.

## Bugs

- [ ] `nova_scenario/src/objects/asteroid_carve.rs:413` a severed rock island
      gets no `RadarOccluder`, so cover you shot loose stops being cover.
- [ ] `nova_ship/src/input/ai/behavior.rs:387` `evade.cooldown.trigger()` fires
      on ANY exit from Evade, including an occlusion flicker, so a hostile
      skimming a rock edge burns its evade clock.
- [ ] `nova_events/src/lib.rs:85,87` `TIMER_KEY_FIELD_NAME` and
      `CINEMATIC_KEY_FIELD_NAME` are both `"key"`: a Timer filter lints clean
      against a cinematic handler and fires on the scene's ending.
- [ ] `nova_scenario/src/world.rs:898,725` `prune_finished_sequences`
      retains `run.stopped`, so a Cinematic stopped by a deadline burns its
      key for the scenario, and the refusal calls it a running sequence.
- [ ] `nova_scenario/src/world.rs:764` `cancel_cinematic` logs `error!` for
      a scene that ended, where `web/src/create/actions.md:628` promises a
      quiet no-op.
- [ ] `nova_scenario/src/actions/cinematic.rs:161` `CinematicTitle.seconds`
      has no lint arm: `0.0` posts an invisible card, `NaN` a card that
      never expires.
- [ ] `nova_hud/src/comms_panel.rs:246` `CommsQueue::pending` is unbounded.
- [ ] `nova_hud/src/cinematic_title.rs:33,125` `ScreenCorner` derives and
      registers `Reflect` but is neither a Component nor a Resource.
- [ ] `nova_scenario/src/objects/area.rs:181` `AreaOccupancy` is keyed on
      the body avian stamps once; a re-key needs a `ColliderOf` observer.
- [ ] `nova_wfc/src/collapse.rs:556` `keel_component` sets `kept[start]`
      ungated, resurrecting a bow seed cell erosion dropped.
- [ ] `nova_wfc/src/check.rs:158,199` `unmated_contacts`' `exempt` callback
      is called with two argument orders.
- [ ] `nova_input/src/registry.rs:637` `ActionName` is written by every rig
      and read by nobody; delete it or its stale docstring.
- [ ] `nova_bench/src/observation.rs:284,61` an empty `expand` string is an
      accidental synonym for `["all"]`.
- [ ] `nova_bench/src/observation.rs:39,27,382` `"<1km"` and `"1-2km"`
      bands can never be produced.
- [ ] `nova_bench/src/referee.rs:166` + `Cargo.toml:20` the ordered map
      loses its order without `serde_json/preserve_order`.
- [ ] `nova_bench/src/referee.rs:27` + `tools/nova_bench/pi/index.ts:104`
      `DEFAULT_ACT_TICKS` spelled twice.
- [ ] `nova_probe_cli/src/native/env.rs:109` `clean_pass_env` does not size
      the correctness deadline from the step budget as the fps pass does.
- [ ] `Cargo.toml` `wfc_arena` block: `stamps.rs:159` tests never compile in
      CI (no `test = true`).
- [ ] `examples/playable/wfc_arena.rs:2517` `Strike` and its counters lack
      the `debug` feature gate the rest of the staging carries.
- [ ] `examples/screenshots/loop_goto_arrival.rs:33` the solver chains
      before the cut installer, so every cut lands one frame late.
- [ ] `examples/screenshots/loop_torpedo_blast.rs:369` `no_torpedo_in_flight`
      flips true when the TARGET despawns with a salvo still flying.

Already scheduled elsewhere: the SKIP SCENE prompt width
(`20260909-213350`), the pad L2 collision (`20260714-001140`), the missing
web CI job (`20260909-213100`).

## Docs

- [ ] `nova_bench/src/pages/travel.md:25` teaches GOTO parks "about 300 m";
      `FlightSettings::default().arrival_standoff` is 500 m.
- [ ] `web/src/wiki/hud.md:105` the comms section never explains channels
      (`MERIDIAN CONTROL / GUARD`).
- [ ] `web/src/create/actions.md:591` Cinematic never tells an author about
      the SKIP SCENE prompt.
- [ ] `web/src/wiki/nova-os.md:17` promises a closing "dying dot" the loop
      never shows; press Tab twice in the producer or drop the line.
- [ ] `web/src/wiki/flight-autopilot.md:41` "its own 55.2 m hull" reads as a
      length; the hull is 85 m long.
- [ ] `examples/screenshots/screenshot_gravity.rs:14,41` "both framings"
      after the second was deleted; `feature-gravity.png` referenced by no
      page (decide: reference it or delete the example).
- [ ] `nova_hud/src/lib.rs:5,168` the crate doc claims every widget is a
      `HudTier` layer; two cinematic widgets spawn untagged in Startup.
- [ ] `assets/base/base.bundle.ron:13` (generated: edit the builder) the
      comment argues from a load order the loader does not have.

## Perf, measure first

- [ ] `nova_scenario/src/world.rs:281,309` clones `story_messages` before
      the length compare and allocates the skip action string per frame.
- [ ] `nova_ship/src/input/targeting/contacts.rs:246` + `radar.rs:87` two
      `collect_lockable` ray-cast passes per frame while the radar is held.
- [ ] `nova_scenario/src/objects/planet.rs:90` `PlanetVisual::build` inline
      on the spawn frame, about 50 ms per planet.
- [ ] `nova_editor/src/preview.rs:317` scrubbing a planet radius rebuilds
      the surface every frame.
- [ ] `nova_hud/src/cinematic_title.rs:212,215` bare visibility writes
      where the idiom is `set_if_neq`.
- [ ] `nova_gameplay/src/integrity/pyre.rs` five feel questions never
      measured: `HULK_PYRE` and its 1700 m light, the shed ceiling,
      `PYRE_FRAME_CAP` under load, a release-profile number, the
      `CAMERA_BASE` confound. Stage a root death in a range or record them
      as accepted unmeasured.

## Design calls for the owner

- `SECTION_PYRE.core` (`pyre.rs:219`) is a 3.4 m flare at the section
  centre, invisible for an interior section: re-place it, or accept
  ejecta-only.
- Grammar zones and keel roles (`ship_grammar.rs:105,219`) are closed Rust
  sets: make them authored content like channels, or document them closed.
- A grammar part at `weight: 0` (`lint/ship.rs:169`): lint Error today,
  `nova_wfc::runnable` accepts it; make them agree either way.
- `command-shell-open.webm` carries a build hash in the CRT header:
  stabilise the version string under capture, or accept it.
- `first_shift_ships` in `playable/` has only the free-fly affordance the
  category contract excludes: move it or widen the contract.
- `probe-runs/` holds about 52 GB of measurement history on the box: prune
  to a retained set or keep.
