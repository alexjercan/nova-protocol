# Close the v0.13.0 review leftovers

- STATUS: CLOSED
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

## Re-check, 2026-09-12

Reviewed commit subjects and bodies from `v0.13.0` through HEAD, then checked
current code for every touched path. Work has started outside this task:

- `68c3ebe6f` replaced the duplicate targeting ray-cast passes with one
  `SensorContacts` pass per observer. The targeting perf item below is done.
- `47a4ab860` staged root deaths on both reference hulls and measured the hulk
  pyre scale. `abfcfead6` measured and replaced the flat section-pyre frame cap.
  The remaining pyre measurement questions stay open below.
- `f3267c18d` changed evade legs and cooldown duration, but the current exit
  edge still arms the cooldown on ANY Evade exit. It did not close that bug.
- `f3a78a434` protects seeded parts from stud erosion, but
  `keel_component` still sets `kept[start]` without checking `standing[start]`.
  It did not close the bow-seed resurrection bug.
- `a89931404` touched the cited flight page and WFC file for other review
  findings. The 55.2 m hull wording and `kept[start]` finding remain present.
- The other cited paths either did not change after this task was created or
  changed for unrelated work. Their findings remain open.

## Implementation, 2026-09-14

Branched from master `2deeb583d` in the `review-leftovers` worktree. Every
unchecked item was revalidated against that tree first; none had been closed
by a later commit, so no item is recorded obsolete. 31 commits, one per item
plus two the work uncovered. Every bug with a reproducible trigger got a
failing test first: the fix was applied, the test written, the production
lines temporarily reverted, the test observed to FAIL, and the lines restored
to green.

Three items took no test and say so on their line: two are pure deletions and
one is a log level.

No `bug_` range was added and no range was touched, so this task carries no
three-run range table. Every fix that could be pinned in a crate was pinned by
a focused unit test instead, as the task's own rule allows. The two example
fixes are producers, and both were RUN under Xvfb rather than `cargo check`ed;
each wrote its loop and its walk reached its own verdict.

## Bugs

- [x] `nova_scenario/src/objects/asteroid_carve.rs:413` a severed rock island
      gets no `RadarOccluder`, so cover you shot loose stops being cover.
      `40f39ee4f`. The ROCK carve path stamps `RadarOccluder` on the severed
      island; the hull-debris path is deliberately left alone, because a ship's
      debris is not the radar shadow its hull was. Pinned by
      `a_severed_island_still_stops_radio`.
- [x] `nova_ship/src/input/ai/behavior.rs:387` `evade.cooldown.trigger()` fires
      on ANY exit from Evade, including an occlusion flicker, so a hostile
      skimming a rock edge burns its evade clock.
      `b983f99bf`. The cooldown is armed on the weave that was FLOWN, not on
      the exit edge. Pinned by
      `an_evade_cut_short_with_its_legs_unflown_keeps_its_cooldown`.
- [x] `nova_events/src/lib.rs:85,87` `TIMER_KEY_FIELD_NAME` and
      `CINEMATIC_KEY_FIELD_NAME` are both `"key"`: a Timer filter lints clean
      against a cinematic handler and fires on the scene's ending.
      `038e0dc35`. The cinematic ending payload carries its own key name.
      Pinned by `a_timer_filter_does_not_answer_a_cinematic_ending`.
- [x] `nova_scenario/src/world.rs:898,725` `prune_finished_sequences`
      retains `run.stopped`, so a Cinematic stopped by a deadline burns its
      key for the scenario, and the refusal calls it a running sequence.
      `1320c7ed9`. Pinned by
      `a_deadlined_scene_refuses_a_restart_as_stopped_not_running`.
- [x] `nova_scenario/src/world.rs:764` `cancel_cinematic` logs `error!` for
      a scene that ended, where `web/src/create/actions.md:628` promises a
      quiet no-op.
      `17db4d782`. `error!` -> `debug!`, matching the documented contract; a
      key no `Cinematic` action plays is caught at lint, where an author can act
      on it. No test: the change is a log level.
- [x] `nova_scenario/src/actions/cinematic.rs:161` `CinematicTitle.seconds`
      has no lint arm: `0.0` posts an invisible card, `NaN` a card that
      never expires.
      `9e2aa7fe5`. Lint arm in `lint/scenario.rs`, pinned by
      `a_title_card_must_hold_for_a_positive_finite_time`.
- [x] `nova_hud/src/comms_panel.rs:246` `CommsQueue::pending` is unbounded.
      `6e4064ecb`. Cap 24, oldest dropped, one line per overflow episode.
      Derivation under "The comms backlog cap" below. Pinned by
      `the_waiting_backlog_is_sized_by_what_the_panel_can_show`,
      `a_backlog_past_its_cap_drops_the_oldest_waiting_line`,
      `an_overflow_episode_is_reported_once_and_a_later_one_again`.
      PENDING OWNER CONFIRMATION of the cap and the drop side.
- [x] `nova_hud/src/cinematic_title.rs:33,125` `ScreenCorner` derives and
      registers `Reflect` but is neither a Component nor a Resource.
      `a7295d5d4`. Both removed. Pure deletion, no test.
- [x] `nova_scenario/src/objects/area.rs:181` `AreaOccupancy` is keyed on
      the body avian stamps once; a re-key needs a `ColliderOf` observer.
      `45c793a45`. Observer re-keys the occupancy onto the new body. Pinned by
      `a_section_severed_inside_an_area_leaves_with_the_body_it_joins`.
- [x] `nova_wfc/src/collapse.rs:556` `keel_component` sets `kept[start]`
      ungated, resurrecting a bow seed cell erosion dropped.
      `d38274457`. `keel_component` now takes `standing` and refuses a start
      cell erosion dropped. Pinned by
      `a_bow_cell_erosion_dropped_stays_dropped_and_takes_its_island_with_it`.
- [x] `nova_wfc/src/check.rs:158,199` `unmated_contacts`' `exempt` callback
      is called with two argument orders.
      `63c3f86e2`. Both sites hand the pair lower index first. Pinned by
      `the_exemption_callback_always_sees_the_lower_section_index_first`.
- [x] `nova_input/src/registry.rs:637` `ActionName` is written by every rig
      and read by nobody; delete it or its stale docstring.
      `94198301d`. Deleted, with its export. Pure deletion, no test.
- [x] `nova_bench/src/observation.rs:284,61` an empty `expand` string is an
      accidental synonym for `["all"]`.
      `67289968e`. The whole-block test got its own function and the empty key
      is refused at the request boundary. Asserted inside the existing
      `parse_expand` request test.
- [x] `nova_bench/src/observation.rs:39,27,382` `"<1km"` and `"1-2km"`
      bands can never be produced.
      `8a8f26d23`. Both removed. Pinned by
      `every_rung_of_the_distance_ladder_can_hold_a_body`.
- [x] `nova_bench/src/referee.rs:166` + `Cargo.toml:20` the ordered map
      loses its order without `serde_json/preserve_order`.
      `dd2197b81`. Feature enabled. Pinned by
      `the_observation_leads_with_the_clock_and_the_budget`.
- [x] `nova_bench/src/referee.rs:27` + `tools/nova_bench/pi/index.ts:104`
      `DEFAULT_ACT_TICKS` spelled twice.
      `2fa5f6592`. Rust owns it; the extension omits `ticks` and takes the
      referee's step, per the handoff's accepted direction. TypeScript change,
      no Rust test; verified by reading the referee's `Option<u32>` default
      path.
- [x] `nova_probe_cli/src/native/env.rs:109` `clean_pass_env` does not size
      the correctness deadline from the step budget as the fps pass does.
      `1e022a1d7`. Sized from the supervisor timeout, docs updated. Pinned by
      `the_clean_deadline_is_sized_from_the_supervisor_timeout`.
- [x] `Cargo.toml` `wfc_arena` block: `stamps.rs:159` tests never compile in
      CI (no `test = true`).
      `e818b269a`. `test = true` on the example target.
- [x] `examples/playable/wfc_arena.rs:2517` `Strike` and its counters lack
      the `debug` feature gate the rest of the staging carries.
      `198184db6`.
- [x] `examples/screenshots/loop_goto_arrival.rs:33` the solver chains
      before the cut installer, so every cut lands one frame late.
      `e466c8600`. RUN, not checked: the pre-fix run stalled at `engage GOTO`
      and wrote no webm (`<scratch>/loopcap_before/`, empty); the fixed source
      wrote `goto-arrival.webm`, 835 KB, 1280x720 at 30 fps, and frames pulled
      back out of it with `ffmpeg` are correctly framed pictures of the scene.
      The stall was a SEPARATE defect - see "Found while running" below.

      LIMIT, stated: the one-frame lag itself was NOT graded frame by frame.
      Proving it on frames needs a before/after capture pair of the same cuts,
      and the "before" capture does not exist because the pre-fix producer
      could not finish a run. What is proved is the ordering - the cut
      INSTALLS a bearing and the rig SOLVES it, so a solver chained first
      spends each cut's opening frame on the bearing it just left - plus a
      producer that now completes and writes a correct loop.
- [x] `examples/screenshots/loop_torpedo_blast.rs:369` `no_torpedo_in_flight`
      flips true when the TARGET despawns with a salvo still flying.
      `47f5b5fcf`. Counted on the torpedoes themselves. Producer RUN under
      Xvfb: `torpedo-blast.webm`, 96 frames, 3.2 s at 30 fps,
      `autopilot: cycle complete, no panic`.

Already scheduled elsewhere: the SKIP SCENE prompt width
(`20260909-213350`), the pad L2 collision (`20260714-001140`), the missing
web CI job (`20260909-213100`).

### Found while running

- `c8b4d5115` - `nova_ship/src/flight/autopilot.rs`. A GOTO leg engaged on the
  frame the hull spawns completed without flying. Avian publishes a root's mass
  one tick after its colliders land; every plan divides by it, so a massless
  hull reads zero brake authority, the arrival leg's desired velocity collapses
  to zero, and that is the same shape as "the goal wants rest here and the ship
  is at rest". The leg is now HELD until the mass lands. This is what stalled
  `loop_goto_arrival`, and it is a released-version defect - the same
  `done`/`accel` shape is in `v0.13.2` - so it took a CHANGELOG entry. Pinned by
  `a_leg_engaged_before_the_hull_has_mass_is_not_already_complete`, which failed
  first on the line asserting the leg is still flying once the mass lands.
- `a03e1cd72` - `web/src/wiki/hud.md`. The page promised "Bursts wait in order
  instead of silently dropping lines that do not fit the visible stack", which
  the comms cap above makes false. Rewritten to name the 24-line cap and the
  oldest-drop. `6e4064ecb` shipped the engine-side doc with the code
  (`docs/scenario-system.md`) but missed the player page; this is the catch-up,
  kept separate so the one-commit-per-item rule reads honestly.

## Docs

- [x] `nova_bench/src/pages/travel.md:25` teaches GOTO parks "about 300 m";
      `FlightSettings::default().arrival_standoff` is 500 m.
      `54a995949`. Now "about 500 m of clear space", and it says the standoff is
      measured from the HULL or the surface flown to, not from a centre.
      Source: `crates/nova_ship/src/flight/state.rs:460`,
      `arrival_standoff: Meters(500.0).to_engine()`.
- [x] `web/src/wiki/hud.md:105` the comms section never explains channels
      (`MERIDIAN CONTROL / GUARD`).
      `edb5df1fe`. A channel sentence in the lead and a channels paragraph in
      the explanation: comms blue, crew phosphor, guard amber plus a faint
      `/ GUARD` tag. Read off the shipped channel catalog, not off the page.
- [x] `web/src/create/actions.md:591` Cinematic never tells an author about
      the SKIP SCENE prompt.
      `018e7b2ed`. Names what lands on screen (`<KEY>  SKIP SCENE`,
      bottom-centre, above the keybind dock), that the key is the player's live
      `cinematic_skip` binding, and that it appears when the 0.75 s hold-off
      expires.
- [x] `web/src/wiki/nova-os.md:17` promises a closing "dying dot" the loop
      never shows; press Tab twice in the producer or drop the line.
      `0bf6f33e9`. Dropped, and the caption now says what the loop records:
      the boot banner staggering out row by row and `map` launching the chart.
      `nova-os-open.webm` is an alias of `landing-cockpit`, produced by
      `loop_cockpit`, which ends on the MAP app - so the dot was never
      reachable without re-cutting the producer.
- [x] `web/src/wiki/flight-autopilot.md:41` "its own 55.2 m hull" reads as a
      length; the hull is 85 m long.
      `454b8cd10`. Now "its own 55.2 m arm - balance point to the outer face of
      its furthest section, not the 85 m the hull is long".
- [x] `examples/screenshots/screenshot_gravity.rs:14,41` "both framings"
      after the second was deleted; `feature-gravity.png` referenced by no
      page (decide: reference it or delete the example).
      `4e371048f`. DELETED, per the handoff's accepted direction.
      `feature-gravity.png` exists nowhere under `web/src/assets/` and no page
      references it; the producer is in no CI workflow and in no
      `catalog_drift.rs` entry. References removed from `Cargo.toml`,
      `scripts/gen-web-screenshots.py`, `docs/development.md`,
      `docs/automation-harness.md` and `web/src/site.ts`.
      `examples/screenshots/shared/drydock.rs` STAYS: `screenshot_hero_ship`
      still calls `drydock_drift`. Verified through `cargo metadata`:
      `screenshot_gravity` absent, `screenshot_hero_ship` present, 111 examples.
- [x] `nova_hud/src/lib.rs:5,168` the crate doc claims every widget is a
      `HudTier` layer; two cinematic widgets spawn untagged in Startup.
      `1f0ea68d6`. "almost every one is a `HudTier` layer", plus a paragraph
      naming `cinematic_title` and `cinematic_prompt` as the deliberate
      exceptions - `apply_hud_visibility` filters `With<HudTier>`, so a tagged
      cinematic widget would be hidden with the rest of the HUD.
- [x] `assets/base/base.bundle.ron:13` (generated: edit the builder) the
      comment argues from a load order the loader does not have.
      `141bf5e7e`. The task's "(generated: edit the builder)" note is itself
      WRONG: `base.bundle.ron` is HAND-AUTHORED. `generation::content_files()`
      produces `assets/base/**/*.content.ron` and does not produce the bundle
      manifest. Edited in place: a header block that says the list is NOT a
      dependency order, and that order decides only the Vec catalogs and
      intra-bundle duplicate precedence. `merge_bundles` routes items into
      id-keyed registries and resolves no reference at merge time.
      `cargo run content lint`: 0 error(s), 0 warning(s), 0 finding(s).

## Perf, measure first

Every number below is RELEASE profile unless it says otherwise, taken on a
quiet host (no user game process, one-minute load under 3.5, load sampled
beside each set). Measurement harnesses were temporary: each was added, run,
its output recorded here, and the file restored from a backup. None of them is
committed - `git status` is clean at the closing commit.

The rule the task set itself is followed literally: a flat number LEAVES THE
CODE ALONE, and the number is recorded either way. NO PERF ITEM CHANGED ANY
CODE. Three are flat, one is real and escalated to the owner with its number,
and one is two measurements plus an accepted-unmeasured with its reason.

- [x] `nova_scenario/src/world.rs:281,309` clones `story_messages` before
      the length compare and allocates the skip action string per frame.
      MEASURED, FLAT, LEFT ALONE.
      `nix develop --command cargo test -p nova_scenario --lib --release
      world::tests::measure_state_to_world_sync -- --nocapture`, host load
      1.26/3.69/3.65 (falling; the one-minute figure is the one this set ran
      under), 20 000 calls per row:

      | story lines | calls | total | per call |
      |-|-|-|-|
      | 10 | 20 000 | 20.80 ms | 1.040 us |
      | 40 | 20 000 | 76.95 ms | 3.848 us |
      | 100 | 20 000 | 187.70 ms | 9.385 us |

      The whole system, clones included, is 9.4 us per frame at 100 story
      lines - 0.06 percent of a 16.7 ms frame, and no shipped scenario posts
      100 lines. The clones are also what makes the write-on-diff correct: the
      resource is borrowed, cloned, and released before the HUD resource is
      taken mutably. Removing them would need a second borrow of `World`.
- [x] `nova_ship/src/input/targeting/contacts.rs:246` + `radar.rs:87` two
      `collect_lockable` ray-cast passes per frame while the radar is held.
      Closed by `68c3ebe6f`: one `SensorContacts` pass now publishes the answer
      per observing ship and both consumers read it.
- [x] `nova_scenario/src/objects/planet.rs:90` `PlanetVisual::build` inline
      on the spawn frame, about 50 ms per planet.
      MEASURED, LEFT ALONE, with a caveat recorded below.
      `nix develop --command cargo test -p nova_scenario --lib --release
      objects::planet_surface::tests::measure_planet_visual_build --
      --nocapture`, host load 0.74/1.39/2.41, five samples per row:

      | planet radius | body | subdivisions | build ms |
      |-|-|-|-|
      | 600 m | 633.0 m | 34 | 20.99, 20.40, 20.29, 20.60, 20.77 |
      | 900 m | 949.5 m | 52 | 35.82, 36.74, 37.61, 36.48, 37.28 |

      Split (generate, shape_noise, mesh) at 900 m: 0.0004, 3.28, 32.91 ms -
      the mesh build is the whole cost, and it grows with the square of the
      subdivision count.

      The review's "about 50 ms" is a DEV-profile figure. In release the
      largest shipped planet is about 36.5 ms. It is not a gameplay hitch: the
      build runs inside the atomic scenario load, which holds the simulation
      under `FreezeOwner::ScenarioLoad` and gates gameplay input and camera
      control off until the queued spawns and required glTF have settled
      (`crates/nova_scenario/src/loader/gate.rs`). The player is handed a
      complete scene at scenario time zero, so a planet's 36.5 ms is load time,
      not a dropped frame. LEFT ALONE.
- [x] `nova_editor/src/preview.rs:317` scrubbing a planet radius rebuilds
      the surface every frame.
      MEASURED, NOT FLAT, ESCALATED TO THE OWNER. Same harness and host:

      | planet radius | subdivisions | build ms |
      |-|-|-|-|
      | 600 m | 24 | 13.76, 13.75, 14.06, 13.74, 13.55 |
      | 900 m | 24 | 13.98, 13.97, 13.89, 13.94, 14.06 |

      `PLANET_EDITOR_SUBDIVISIONS` is 24 whatever the radius, so the cost is
      flat in radius and about 13.9 ms per scrub frame in release - the frame
      budget of a 60 fps frame spent before anything else in the editor runs,
      and worse in the dev profile the editor is actually used in.
      This is a real cost, and every remedy is an owner decision, so no code
      changed. See "Owner decisions still open" below.
- [x] `nova_hud/src/cinematic_title.rs:212,215` bare visibility writes
      where the idiom is `set_if_neq`.
      REASONED, FLAT, LEFT ALONE. Not measured with a clock, because the
      population is ONE entity: `q_card` is `With<CinematicTitleMarker>` and
      exactly one card exists for the session.

      One reader of the write DOES exist, against the first reading of this
      item:
      `bevy_camera-0.19.1/src/visibility/mod.rs:643`,
      `visibility_propagate_system`, whose changed query is
      `Or<(Changed<Visibility>, Changed<ChildOf>)>`. So the bare write does put
      the card in that query every frame. What it costs is one extra entity in
      one query: the body then compares `inherited_visibility.get()` against
      the value it just read and, when they match, propagates to no child.

      On the common path - no card playing - the system writes
      `Visibility::Hidden` and `continue`s, so the `Node`, `BorderColor` and
      `BackgroundColor` writes below it are not paid. One entity, one query
      match, one bool compare, per frame. LEFT ALONE, and recorded here so
      the next reader does not re-open it.
- [x] `nova_gameplay/src/integrity/pyre.rs` three feel questions remain
      unmeasured: the shed ceiling, a release-profile number, and the
      `CAMERA_BASE` confound. `47a4ab860` staged and measured root deaths on
      both reference hulls; `abfcfead6` measured the section-pyre frame cap
      under a collapse. Measure the remaining three or record them as accepted
      unmeasured.
      TWO MEASURED, ONE ACCEPTED UNMEASURED. No code changed.

      **1. The shed ceiling.** MEASURED, FLAT, LEFT ALONE.

      The review's complaint was that no range drives a many-plates frame:
      `stress_hull_collapse` declines a derived skin
      (`examples/systems/stress_hull_collapse.rs:600`), so the one load range
      that could see a fixture cannot. That is still true, and it is still the
      right call for that range - its subject is what a collapse CREATES. So
      the ceiling was measured where it lives instead, on the shed rig the
      crate already has.

      `nix develop --command cargo test -p nova_ship --lib --release
      sections::fixture::tests::measure_shed_ceiling -- --nocapture`, host load
      1.06/1.27/1.59, five samples per row. The frame banks 16 fixed steps
      (`Time<Virtual>`'s 0.25 s `max_delta` over a 1/64 s step), which is the
      frame the ceiling exists for:

      | dead plates | shed this frame | frame ms |
      |-|-|-|
      | 6 | 6 | 0.366, 0.380, 0.366, 0.247, 0.239 |
      | 24 | 24 | 0.346, 0.288, 0.287, 0.305, 0.289 |
      | 48 | 48 | 0.511, 0.319, 0.338, 0.298, 0.328 |
      | 96 | 48 | 0.390, 0.315, 0.372, 0.302, 0.404 |
      | 384 | 48 | 0.604, 0.719, 0.513, 0.424, 0.357 |

      `SHED_FRAME_CAP` is `SHED_TICK_CAP * 2` = 48 and it BINDS exactly there:
      96 and 384 dead plates both shed 48. The ceiling frame costs about
      0.33 ms of archetype moves, two percent of a 16.7 ms frame, and a backlog
      eight times the ceiling adds about 0.2 ms of query walk on top of it. The
      cap is doing its job at a cost that does not need lowering.

      LIMIT, stated: the rig is `MinimalPlugins` plus `NovaHealthPlugin`, so
      this is the SHED's own cost - unparent, remove `Collider`, insert the
      body components, walk the dressing - and not avian's integration of 48
      new rigid bodies for `SHED_LIFETIME_SECS`. The archetype moves are what
      the ceiling bounds and what the review named; the physics cost of the
      debris is the wreck path's, already covered by `stress_hull_collapse`.

      **2. A release-profile number.** MEASURED. This is the first
      release-profile reference this range has.

      Six direct autopilot runs, `NOVA_AUTOPILOT=1 DISPLAY=:99 nix develop
      --command cargo run --release --features debug --example
      stress_hull_collapse`, RTX 3060 Ti / Vulkan / Xvfb 1280x720, host quiet
      throughout (one-minute load 1.14 to 2.00):

      | run | worst frame ms | fixed steps | window frames | over timestep | peak entities | chips |
      |-|-|-|-|-|-|-|
      | 1 | 239.5 | 15 | 450 | 410 | 11832 | 196 |
      | 2 | 79.7 | 5 | 476 | 415 | 11840 | 92 |
      | 3 | 64.2 | 4 | 473 | 344 | 11910 | 108 |
      | 4 | 92.0 | 6 | 470 | 367 | 11826 | 159 |
      | 5 | 69.8 | 4 | 474 | 348 | 11827 | 35 |
      | 6 | 70.8 | 4 | 477 | 379 | 11905 | 153 |

      All six: exit 0, `hull_collapse: every collapse invariant held`, 720
      corridor cells destroyed, 720 wreck pieces at peak. Median 75.3 ms;
      median 70.8 ms with run 1 dropped, which is the process's first run and
      carries the shader and page-cache warm-up - it banked 15 fixed steps
      where every other run banked 4 to 6.

      Read it as a RECORD, not as a verdict on the 35 percent dev regression
      `20260908-004345` reported, because it is NOT comparable to that
      reference on either axis:

      - Profile. The reference is dev. `docs/performance.md` is explicit that
        dev-profile numbers are not baselines.
      - Content. `83f4da4de` made bullet chips GPU particles, so peak entities
        fell from the reference's 14 923-15 517 to 11 826-11 910 and the
        reference's `peak_shards` 4111-4573 has no counterpart - this range now
        reports 35 to 196 chips thrown. `abfcfead6` replaced the flat pyre
        frame cap with `6 * sqrt(condemned / 53)`, so the frame that condemns
        720 cells now lights 23 fires where the reference lit 6.

      Two populations moved and one throttle was replaced between the two
      sets. An A/B across them would not be measuring the pyre. What this set
      IS good for: it is the matched reference the NEXT change to this path
      gets compared against, on this host, at this commit, in release.

      One reading worth carrying forward, and it is not the worst frame: 344 to
      415 of about 470 settle-window frames overran the 15.62 ms timestep. The
      spike is not the story; the aftermath of 720 physical wreck pieces is.

      **3. The `CAMERA_BASE` confound.** ACCEPTED UNMEASURED, and for a reason
      the instrument gave rather than for cost.

      The question, from `20260908-004345`: batch 3 landed
      `CAMERA_BASE: 55.0 -> 20.0` with `CAMERA_PER_SPREAD: 0.85 -> 0.80` in
      `examples/playable/wfc_arena.rs` - a 2.75x closer standoff floor on what
      the file calls the busiest scene the site ships, with shed cladding and
      debris now passing near the lens. The lane called it a plausible
      fragment/overdraw cost and refused to assert a number.

      The A side of the A/B was run in full:

      `nix develop --command cargo run --features dev probe run wfc_arena
      --release --display :99 --repeat 3 --out <scratch>/probe-camera-20`

      It is GREEN and it measures nothing useful. `wfc_arena OK measured 7/8`
      in 1664 s; `process_exit`, `run_completed`, `reached_playing`,
      `invariants_held` (0 violations over 385 frames), `capture_simulated`
      (3 captures, 0 refused), `log_clean`, `artifacts_loadable` all PASS;
      `fps_within_baseline` N/A, no baseline. But the repeat gate DISCARDED all
      three captures:

      | capture | mean ms | median ms | p99 ms | worst ms | cluster median ms | cluster share | admitted |
      |-|-|-|-|-|-|-|-|
      | 1 | 28.2539 | 28.2689 | 32.3262 | 38.1919 | 28.2829 | 0.7861 | no |
      | 2 | 28.0165 | 28.0181 | 32.4594 | 33.0373 | 28.0511 | 0.7750 | no |
      | 3 | 28.1153 | 27.9349 | 33.1283 | 34.0863 | 27.9459 | 0.7556 | no |

      `refresh_cap: refresh_capped`, `admitted: 0`, `discarded: 3`,
      `p99_ms: null`. Three independent captures clustered about 76 to 79
      percent of their frames on a period they agree on to within one percent,
      which is `refresh_cap`'s definition of having measured the DISPLAY and
      not the game
      (`crates/nova_probe_cli/src/evaluation/frames.rs:356`, and the rule's own
      doc comment).

      So the B side was not run. A second 28-minute set the same gate discards
      cannot answer the question: on a set pinned to a constant period, a
      camera change that costs less than the headroom to that period is
      invisible, and "no difference" from a capped set is an unfalsifiable
      negative, not a measurement. Running it would have produced an answer
      shaped like evidence and made of nothing.

      What WOULD answer it: a GPU-side timing pass. The overdraw claim is a
      fragment-cost claim, and this harness times frames on the CPU
      (`crates/nova_probe/src/capabilities/framecost.rs`); `--norender` removes
      the renderer, and `--render sw` puts a software rasteriser in front of a
      900-frame window. None of the three sees a fragment bill. That is a
      harness gap, not a finding this task can close, and it is not this task's
      to open.

      No code changed for any of the three.

## The comms backlog cap

The one open decision the task named. The cap is DERIVED from what the panel
can display, not chosen by taste, and the derivation is in the code beside the
constant (`crates/nova_hud/src/comms_panel.rs:86-105`):

- A visible slot frees no sooner than `COMMS_DWELL_MIN_SECS +
  COMMS_FADE_OUT_SECS` = 3.0 + 0.4 = 3.4 s. That is the fastest the panel can
  retire one line.
- `COMMS_VISIBLE_CAP` = 3 slots run at once, so the panel retires at most three
  lines every 3.4 s.
- `COMMS_DWELL_MAX_SECS` = 30 s is the longest the panel will hold ONE line, so
  it is also the longest a cue can be worth waiting behind.

  3 slots x floor(30.0 / 3.4) = 3 x 8 = 24.

The last line of a full backlog comes up after 8 x 3.4 = 27.2 s, inside the
30 s window. A twenty-fifth would come up outside it, which is why it is
dropped rather than queued.

Overflow drops the OLDEST pending cue and logs once per overflow episode, per
the handoff's provisional preference. The episode latch resets when the backlog
comes back under the cap, so a scenario that floods the panel twice gets two
lines, not one and not thousands.

LANDED, PENDING OWNER CONFIRMATION. If the owner wants a different cap or the
newest dropped instead, the derivation above is the thing to argue with; the
three tests and the docs sentence move with it.

## Owner decisions still open

- The comms cap and drop side, above.
- The editor scrub. It is about 13.9 ms a frame in release and the three
  remedies are all owner calls:
  - Lower `PLANET_EDITOR_SUBDIVISIONS` below 24. A look decision.
  - Debounce the rebuild while the radius slider is dragged, and rebuild once
    on release. Changes what the editor SHOWS during a drag.
  - Cache the visual and scale it for a radius-only change. UNSAFE as the code
    stands: `PlanetType::relief_fraction`
    (`crates/nova_scenario/src/objects/planet_type.rs:428`) divides an authored
    absolute relief by the radius, so a radius scrub changes the surface's
    shape and not only its size. A cached mesh would be wrong for every planet
    that authors an absolute `relief`.
- The `CAMERA_BASE` confound, recorded as accepted unmeasured below.
- The six design calls the task carried for the owner are unchanged and are
  still listed at the end of this file. None of them was edited.

## Verification

Affected checks only. No workspace test run and no Clippy sweep; the full
suite OOMs this box.

### Rust, per crate, filtered

All through `nix develop --command ...` from the worktree root.

Re-run on the FINAL source of the branch, after every commit, all exit 0:

| command | result |
|-|-|
| `cargo test -p nova_input --lib` | ok, 52 passed, 0 failed |
| `cargo test -p nova_events --lib` | ok, 21 passed, 0 failed |
| `cargo test -p nova_hud --lib` | ok, 286 passed, 0 failed |
| `cargo test -p nova_bench --lib` | ok, 60 passed, 0 failed |
| `cargo test -p nova_probe_cli --lib` | ok, 150 passed, 0 failed |
| `cargo test -p nova_wfc --lib` | ok, 17 passed, 0 failed |
| `cargo test -p nova_scenario --lib` | ok, 439 passed, 0 failed |
| `cargo test -p nova_ship --lib flight::` | ok, 152 passed, 0 failed, 817 filtered |
| `cargo test -p nova_ship --lib input::ai` | ok, 148 passed, 0 failed, 821 filtered |
| `cargo test --features dev --example wfc_arena` | ok, 14 passed, 0 failed |

The last row is the proof for the `test = true` item: before `e818b269a` that
command built nothing, and four `stamps::stamp_tests::*` tests -
`arena_stamp_replaces_only_the_central_support_beam`,
`arena_stamps_one_capital_or_two_to_three_vector_drives`,
`the_stamp_moves_its_beam_when_the_grammar_retunes_the_grid`,
`seeded_large_drive_stamps_mate_to_generated_sterns` - had never once been
compiled in CI.

### Content

`nix develop --command cargo run content lint`: 0 error(s), 0 warning(s),
0 finding(s). Run for `141bf5e7e` (the hand-authored bundle manifest) and for
`9e2aa7fe5` (the new title-card lint arm).

No generated `assets/base/**/*.content.ron` was hand-edited; none needed
regenerating, because no builder changed.

### Examples, RUN and not merely checked

- `loop_goto_arrival` under `Xvfb :99`, `DISPLAY=:99`: wrote
  `goto-arrival.webm` (835 KB). The pre-fix run of the same producer stalled at
  `engage GOTO` and wrote nothing.
- `loop_torpedo_blast` under the same display: `torpedo-blast.webm`, 96 frames,
  3.2 s at 30 fps, `autopilot: cycle complete, no panic`,
  `harness completion: all collectors done, exiting`.
- `stress_hull_collapse`, release profile, six direct autopilot runs - see the
  pyre measurement below.

### Web

`cd web && nix develop --command npm ci` (630 packages; the sprout worktree had
no `node_modules`), then `npm run ci`.

`npm run ci` FAILS at `widgets.test.ts`, and the failure is PRE-EXISTING and
not mine:

```
crates/nova_gameplay/src/integrity/spew.rs:
/const SHARDS_PER_FRAME: usize = ([0-9]+);/ matched 0 declarations, not one
```

Master `83f4da4de Throw bullet chips as GPU particles and cap compute workers
at eight` removed that constant and did not update the widget test that reads
it. `83f4da4de` is an ancestor of this branch's point `2deeb583d`, and
`git grep SHARDS_PER_FRAME 2deeb583d -- <that file>` returns nothing, so the
branch INHERITED the failure. It is not fixed here: it belongs to whoever owns
the spew change, and fixing it would be a second finding in this task's
closing commit.

Everything around it was run separately and is GREEN:
`npm run format:check` ("All matched files use Prettier code style!"),
`npm run lint` (clean), `site.test.ts`, `story-reader.test.ts`,
`theme.test.ts`, `comic-page.test.ts`, `comic-renderer.test.ts`,
`comics.test.js`, `comic-script.test.js`, `ron.test.js`, `lore.test.js`,
`assets.test.js`, and `npm run build`
("webpack 5.108.4 compiled successfully in 14405 ms").

### Deliberately not run

- No workspace `cargo test` and no Clippy sweep: the box OOMs on the full
  suite, and the task's own instruction is affected checks only.
- No probe fleet run. `stress_hull_collapse` was run directly rather than
  through `probe run`, because the range declares `without_frametime()` and its
  frame numbers are its OWN recorded claim 7, not the probe's fps pass.
- No CI split edit. `20260909-213100` owns the probe shard split and is still
  open.

## Open questions before implementation, answered

These were the choices that could change a worker's implementation. All are
settled except the first, which is landed and awaiting confirmation.

- `CommsQueue::pending` cap and overflow: 24, oldest dropped, one log line per
  overflow episode. DERIVED, not chosen - see "The comms backlog cap" above.
  PENDING OWNER CONFIRMATION.
- Empty bench `expand`: INVALID INPUT, refused at the request boundary. The
  handoff's accepted direction, and the code agrees with it - the empty key
  named no group, so the two readings left ("open nothing", "open everything")
  were indistinguishable to the caller.
- The unreachable `"<1km"` and `"1-2km"` bands: REMOVED, per the handoff. The
  alternative - lowering the full-contact threshold so they can be reached -
  hides nearby contacts, which is the worse failure for an agent.
- `DEFAULT_ACT_TICKS`: RUST OWNS IT. The extension omits `ticks` and the
  referee applies its own default. No generated shared data.
- `feature-gravity.png`: DELETED, with its producer. Nothing referenced it.
- The measurement scenario and regression threshold for planet spawn and editor
  scrub: no threshold was invented. Both were measured in release against
  run-to-run noise (five samples, spread under 6 percent), the spawn cost was
  found to land inside the atomic load freeze and left alone, and the scrub
  cost was escalated with its number rather than fixed by taste.
- The six owner design calls below are still unresolved and were not edited.

## Commits

Branched from master `2deeb583d`, oldest first. One commit per item.

| commit | subject |
|-|-|
| `94198301d` | Delete the unread ActionName input component |
| `a7295d5d4` | Drop the unused Reflect registration on ScreenCorner |
| `e818b269a` | Build the wfc_arena example as a test target |
| `198184db6` | Gate the wfc_arena staged strike behind the debug feature |
| `dd2197b81` | Keep the bench observation header in the order the referee builds it |
| `1e022a1d7` | Size the probe correctness deadline from the supervisor timeout |
| `038e0dc35` | Give the cinematic ending payload its own key field |
| `1320c7ed9` | Refuse a deadlined sequence restart as stopped, not running |
| `17db4d782` | Make cancelling a scene that is not running a quiet no-op |
| `9e2aa7fe5` | Refuse a title card that holds for zero, negative or NaN seconds |
| `d38274457` | Stop the island walk resurrecting a bow cell erosion dropped |
| `63c3f86e2` | Hand the contact exemption its pair in one order |
| `8a8f26d23` | Drop the two distance bands nothing can be filed in |
| `67289968e` | Refuse an empty expand key instead of reading it as all |
| `2fa5f6592` | Let the bench extension omit ticks and take the referee's step |
| `45c793a45` | Move a severed collider's area occupancy to its new body |
| `6e4064ecb` | Bound the comms backlog at what the panel can show |
| `b983f99bf` | Spend the evade cooldown only on a weave that was flown |
| `40f39ee4f` | Keep a severed rock island opaque to radar |
| `c8b4d5115` | Hold a leg engaged before the hull has mass |
| `e466c8600` | Install an arrival cut before the rig solves it |
| `47f5b5fcf` | Count the salvo, not the range to a target that can die |
| `54a995949` | Teach the bench the standoff GOTO actually parks on |
| `edb5df1fe` | Say what a comms channel is on the HUD page |
| `a03e1cd72` | Stop the HUD page promising a lossless comms backlog |
| `018e7b2ed` | Tell a Cinematic author what skippable puts on the screen |
| `0bf6f33e9` | Caption the NOVA OS loop with what it records |
| `454b8cd10` | Name the gunship figure the standoff actually adds |
| `4e371048f` | Delete the gravity producer nothing reads |
| `1f0ea68d6` | Name the two HUD widgets that are not tier layers |
| `141bf5e7e` | Say what the bundle's file order actually decides |

Four items are player-visible and took a `CHANGELOG.md` entry under
`[Unreleased]`, all in `### Fixes` except the comms one:

- the travel order flown on the spawn frame (`c8b4d5115`),
- the severed island that still stops radio (`40f39ee4f`),
- the evade clock kept through a sight flicker (`b983f99bf`),
- the 24-line comms backlog and its oldest-drop (`6e4064ecb`,
  `### Interface & HUD`).

Nothing else changed player-visible behavior: the rest are lint arms, engine
contracts, producer staging, docs and deletions.

Three more commits close the task rather than an item: `0e53ee855` (this
record), `bfe25d3de` (two of those four changelog entries joined to 213 and
207 characters against the 200 the changelog rules set, so both were trimmed),
and the commit carrying this paragraph.

## Design calls for the owner

Untouched by this task. Listed as the review left them.

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
