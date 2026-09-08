# Nova Review of the unpushed code range: death pyre, cladding shed, agent bench, example re-staging

- STATUS: CLOSED
- PRIORITY: 70
- TAGS: v0.13.0, review

## Goal

Run Nova Review over the code-carrying commits in `origin/master..master` (23
commits; 9 carry code). Story, lore, web, and task-only commits are out of scope
by owner instruction. The code range is 9169 diff lines, past the one-pass
limit, so it is split into six batches that run in sequence.

This file is the shared ledger: the plan, then one findings block per batch,
then the adjudicated verdict and the fix pass.

Owner framing (2026-09-08): review the unpushed changes on `master`; Rust code
is the interest, not story or frontend; check quality, bugs, and correctness; at
most two agents in flight at any time, queued one batch after another; record
findings here, then fix them on `master` and split the fixes into logical
commits. `--play` was requested for the cladding-shed ("greebles") addition, to
confirm it does not cost the frame.

## Rules for this run

- Two agents per batch, dispatched in one message, nothing else concurrent.
  - Lane A: correctness + performance. Holds the measurement slot.
  - Lane B: craft + contracts. Never runs a rendered example or a probe.
- Reviewers are read-only. No edits, no staging, no commits.
- No workspace test suite, no workspace Clippy.
- `assets/base/**/*.content.ron` is generated. Review the Rust builders under
  `crates/nova_authoring/src/`, not the RON.
- HEAD is the truth. The per-commit diffs show intent; a later commit may have
  already replaced code an earlier one added.
- Bundles live in the session scratchpad under `review-bundle/`; every lane in a
  batch gets the same path and does not re-derive the range.

## Code commits

Oldest first. The other 14 commits in the range touch only `web/`, `lore/`,
`docs/`, `tasks/`, and `scripts/`, and are excluded by owner instruction.

| Commit | Subject | Code lines |
|-|-|-|
| `9663e838` | Put mainline ships back on every living figure on the site | 1208+/175- |
| `a16270a7` | Give a death its fireball, and stand the loops where the world is | 2096+/308- |
| `b111fb91` | Stage the hero duel at 2 km and re-cut three section figures | 541+/90- |
| `3612967f` | Put guns in the landing duel and one launch in the bay loop | 120+/44- |
| `1448d4cc` | Add the agent bench: an agent plays a scenario and the referee scores it | 1563+/418- |
| `3f8163fa` | Shed cladding on death | 371+/54- |
| `615b239c` | Re-shoot the site's footage against shed cladding | 19+/9- |
| `48a3ba1d` | Spin debris about its own shape | 61+/4- |
| `5cd730f8` | Run the railgun shot at real speed too | 98+/8- |
| `a5da7efd` | Pin the debris pivot instead of only stating it | 145+/14- |

## Work groups

The nine code commits fall into four features. Bundles are cut by path against
`origin/master..master`, because HEAD is the truth and three of the features
were revised more than once inside the range.

| Group | Feature | Paths |
|-|-|-|
| G1 | Death pyre: the fireball a destroyed body throws | `nova_gameplay/src/integrity/{pyre,mod}.rs`, `nova_ship/.../torpedo_section/render.rs` |
| G2 | Cladding shed: debris detach, spin, and pivot | `nova_gameplay/src/integrity/explode.rs`, `nova_ship/src/sections/{fixture,shell_skin}.rs` |
| G3a | Agent bench: observation, gesture, referee, score | `crates/nova_bench`, `tools/nova_bench` |
| G3b | Bench substrate: channel apply/runner, terminal, snapshot, world | `nova_channel`, `nova_os_ui`, `nova_probe`, `nova_scenario`, `nova_authoring` |
| G4a | Playable arena and system ranges | `examples/playable`, `examples/systems` |
| G4b | Loop captures and shared staging kit | `examples/screenshots/loop_*.rs`, `examples/screenshots/shared` |
| G4c | Still captures | `examples/screenshots/screenshot_*.rs` |

## Batch plan

Run order is engine-first: the crate a feature stands on is judged before the
code that sits on it. The `--play` batch runs last, so the rendered lanes
inherit every fix decision the earlier batches produced.

| # | Batch | Bundles | Lines | Lanes |
|-|-|-|-|-|
| 1 | Death: pyre and cladding shed | `g1-pyre.diff`, `g2-debris.diff` | 1717 | correctness+performance / craft+contracts |
| 2 | Agent bench crate | `g3a-bench.diff` | 2175 | correctness+performance / craft+contracts |
| 3 | Bench substrate and playable ranges | `g3b-channel.diff`, `g4a-playable.diff` | 2045 | correctness+performance / craft+contracts |
| 4 | Loop captures and shared kit | `g4b-loops.diff` | 1839 | correctness+performance / craft+contracts |
| 5 | Still captures | `g4c-stills.diff` | 1393 | correctness+performance / craft+contracts |
| 6 | `--play` over the death feature | `g1-pyre.diff`, `g2-debris.diff` | 1717 | red team / feel |

### Not reviewed

`web/`, `lore/`, `docs/`, `scripts/`, and `tasks/` changes in this range, by
owner instruction. Generated art is not judged.

## Progress

| Batch | State |
|-|-|
| 1 Death: pyre and cladding shed | DONE - 5 MAJOR, 12 MINOR |
| 2 Agent bench crate | DONE - 2 MAJOR, 13 MINOR |
| 3 Bench substrate and playable ranges | DONE - 6 MAJOR, 7 MINOR |
| 4 Loop captures and shared kit | DONE - 6 MAJOR, 12 MINOR |
| 5 Still captures | DONE - 1 BLOCKER, 4 MAJOR, 16 MINOR |
| 6 `--play` over the death feature | DONE - 2 BLOCKER, 2 MAJOR, 5 MINOR |

## Batch 1 findings: death pyre and cladding shed

### Lane B: craft + contracts

1. **MAJOR** - `pyre.rs:461` - every destroyed asteroid now throws a section
   fireball. `light_the_pyre` observes `Add<IntegrityDestroyMarker>` and lights
   anything with a `GlobalTransform`.
   `nova_scenario/src/objects/asteroid_carve.rs:783` inserts that same marker on
   an exhausted rock, and its comment states the intent: "Reuse the common
   destruction cue seam without opting into its health or random-fragment
   finale." The node is iterated from a query that requires `&GlobalTransform`
   and carries no `IntegrityRoot`, so it takes `SECTION_PYRE` plus a
   `LightFlash` of 2.5 M intensity over 320 m. Shoot a rock to
   `remaining_world < CHUNK_MIN_VOLUME` on the gunnery range and it flashes and
   throws incandescent ejecta. Nothing claims this: the module doc argues the
   fireball is a hull's own vaporised mass, and the wiki says rocks carve.
   Adjudicator re-derived this claim against both files; it holds.
   Fix: gate the observer on what actually burns, not on the shared marker.
2. **MAJOR** - `docs/sections.md:457` - the chapter still says
   `NovaIntegrityPlugin` composes "eight generic pieces" and lists the old
   eight. `integrity/mod.rs:59-83` now adds nine. `docs/sections.md:1023` names
   `explode.rs` with no pyre row, and "How a body comes apart" describes a death
   as detach-only.
3. **MINOR** - `pyre.rs:126,146` - `capacity` is a field an author sets that
   `particles` already decides; the docstring states the invariant and nothing
   enforces it. Raise `HULK_PYRE.ejecta.particles` past its hand-written
   `capacity` and the burst is silently short.
4. **MINOR** - `pyre.rs:295` - `scatter()` is a third byte-for-byte copy of the
   draw in `torpedo_section/render.rs:326` and `:444`. Its own docstring calls
   it "the same three-component draw every burst in the game uses".
5. **MINOR** - `CHANGELOG.md:355` - the death entry is 211 characters joined,
   over the 200 cap.
6. **MINOR** - `CHANGELOG.md:390` - the Fixes entry credits a cladding pivot fix
   that never shipped broken. Measured against v0.12.0 the wreckage half is a
   real shipped bug; the cladding half was introduced and fixed inside this
   cycle by `3f8163fa8` -> `48a3ba1d` -> `a5da7efd`.
7. **MINOR** - `pyre.rs:45` - the module header ends on history ("an earlier cut
   read its own comments as metres"), which AGENTS.md bans for module comments.
   Related: every figure in the module is a bare `f32` silently in world units,
   with the metre value only in prose. Crate-wide gap, not one this change
   created: `to_engine`/`from_engine` appear nowhere in `nova_gameplay`.
8. **MINOR** - `pyre.rs:66` - `PyreEffectMarker` is `pub`, registered, and
   exported through three preludes, and its docstring promises a consumer ("so a
   range can count them") that a repo-wide grep does not find.
9. **MINOR** - `web/src/wiki/ships.md:39` - "What damage looks like" still
   describes a section death as a part coming off, with no flash. The cladding
   sentence at `:49` was updated; the death sentence was not.
10. **MINOR** - `CHANGELOG.md:358` - "burns three times as long" is unsupported.
    Core lifetime 1.94x, ejecta 1.75x, `TempEntity` 1.2x. Only `BLAST_LIGHT_SECS`
    is near 3x, and that is the light.

Lane B verified and reported as holding: no compatibility shim;
`despawn_dead_fixtures` was replaced rather than kept beside the shed; the one
lint suppression is `#[expect(..., reason)]`; plugin and set naming; inline
tests named as behaviour; the two `EffectAsset` pairs are parameterized through
`PyreScale`, not copy-paste; the debris kick and spin both draw from
`GlobalRng`; the pinned pivot reads `collider.center_of_mass()` in the dying
entity's own frame and is correct in both `explode.rs:364` and `fixture.rs:183`;
observer-vs-despawn ordering is safe; `refill_pyre_budget` is one refill per
render frame and root deaths correctly bypass the cap; no wasm-banned API and no
new feature gate needed; `cargo check -p nova_gameplay -p nova_ship` clean; no
runtime id renamed; the `tutorial.content.ron` delta matches its builder line
for line.

Lane B did not run: `content lint` (no content file in its bundles), any
rendered run or probe, `cargo test`, the web build, `mdbook build`.

### Lane A: correctness + performance (held the measurement slot)

1. **MAJOR (MEASURED)** - `pyre.rs:452` - the worst collapse frame regressed
   about 35 percent against the named reference, and the populations did not
   grow. `stress_hull_collapse` at HEAD `b8d17f99`, six direct autopilot runs,
   Xvfb 1280x720, host gated quiet (one-minute loads 1.03-1.73):

   | run | worst_frame_ms | fixed steps | window frames | peak entities | peak shards |
   |-|-|-|-|-|-|
   | 1 | 112.8 | 8 | 399 | 14923 | 3856 |
   | 2 | 86.6 | 5 | 392 | 15273 | 4222 |
   | 3 | 109.4 | 7 | 382 | 15190 | 4151 |
   | 4 | 90.7 | 5 | 393 | 15517 | 4425 |
   | 5 | 99.9 | 6 | 393 | 15437 | 4334 |
   | 6 | 105.4 | 7 | 400 | 15174 | 4170 |

   Median 102.7 ms, median 6.5 fixed steps. The named reference in
   `tasks/20260905-001635/TASK.md:805-830` is the same host, the same six-run
   protocol and the same pinned content: median 76.2 ms, median 5 steps,
   `window_frames` 391-402, `peak_shards` 4111-4573. `window_frames`,
   `peak_shards` and `peak_wreck_pieces` are flat, so the collapse is not making
   more debris - the flush frame is doing more work per frame.
   `stress_hull_collapse.rs:401` states the range carries no derived skin, so
   **this is the pyre, not the cladding shed**. Attribution is reasoned, not
   isolated: no A/B knob turns the pyre off without an edit. Caveats the lane
   named: the reference does not state its display arrangement, and
   `docs/performance.md` puts the Xvfb penalty near 13.7 ms at 720p, which could
   cover roughly half the delta.
2. **MAJOR (UNMEASURED)** - `fixture.rs:147` - the cladding shed has no
   per-frame budget, in the same change that gave the pyre one for exactly this
   reason. One torpedo over a clad hull zeroes every plate in radius in one
   frame; `shed_dead_fixtures` then converts each into an independent
   `RigidBody::Kinematic`: unparent, remove `Collider`, insert seven components,
   plus the `TempEntity` hook. Four archetype moves per plate plus a body avian
   integrates for `SHED_LIFETIME_SECS` 12 s. `docs/sections.md:324` says a hull
   wears hundreds of plates. `explode.rs`'s module doc gives the reason the
   wreck path refuses this shape. `pyre.rs:72-84` adds `PYRE_FRAME_CAP` against
   a smaller per-item cost in this same change. No range covers it:
   `stress_hull_collapse` declines a derived skin, so the one load range that
   could catch it cannot see a fixture at all.
3. **MAJOR** - `examples/systems/` - both halves ship with no example range and
   no `outcome:` slug. Nothing under `examples/systems/` and nothing in
   `crates/nova_probe_cli/tests/catalog_drift.rs` names the pyre or the shed;
   `system_destruction_finale`, `stress_hull_collapse` and
   `system_section_severing` were left as they were. A later change that stops a
   hull lighting, or stops it shedding, passes the whole catalog.
4. **MAJOR** - `pyre.rs:96` - the effect graphs are built lazily inside the
   death observer, so two full `ExprWriter` graphs plus two `format!` names plus
   two `Assets::add` land on the first root-death frame - the most-watched frame
   in the game, and in a collapse the same flush frame measured above. The
   laziness is deliberate per `pyre.rs:96-97`, but the cost is moved onto the
   worst possible frame rather than removed.
5. **MINOR** - `pyre.rs:186` - the section ejecta streak is longer than the
   distance the whole burst travels. `SECTION_PYRE` ejecta `length: 0.50` units
   (5 m) against a reach of 0.5 u/s for at most 0.55 s = 0.275 units (2.75 m). A
   fragment is ~1.8x longer than the distance it covers, so it never clears the
   core: one smear, not fragments. `HULK_PYRE` is proportioned the other way
   (2.20 length against 13.6 reach, ~6x). UNRENDERED.
6. **MINOR** - `pyre.rs:194` - `lumens`, `light_range`, `light_secs` and
   `linger` are the only four constants in the file with no metre cross-check,
   in the file whose stated history is a unit bug. `light_range` goes straight
   to `LightFlash.range`, documented as world units, so 32.0 is a 320 m glow
   around a compartment a few metres across and 170.0 is 1700 m around a 110 m
   hull. The neighbouring `torpedo_section/render.rs` does state its conversion.
7. **MINOR** - `pyre.rs:682` - `a_collapse_is_capped_but_the_hull_itself_never_is`
   asserts a count that depends on kill order and does not say so. The root
   bypasses the refusal at `:478` but still consumes a slot at `:481`. Kill the
   hull first and the frame produces twelve bursts, not fourteen.
8. **MINOR** - `fixture.rs:147` - a fixture with no collider is silently
   skipped; the sibling `detach_destroyed_body` treats the same absence as an
   error and says why. Reachable only from a mod today, but AGENTS.md requires a
   stated refusal, not a quiet no-op.
9. **MINOR** - `shell_skin.rs:1136` - `shed_dead_fixtures` is registered with no
   ordering beside a system that declares one, and it draws from the same
   `GlobalRng` stream as `detach_destroyed_body` with no order between them. A
   seeded replay may not reproduce the same tumble.
   `the_same_death_tumbles_the_same_way_twice` exists to protect that stream.

Lane A disproved and recorded so they are not raised again: a zero-volume stud
plate shedding at spawn (`ShellShape::volume()` special-cases the all-zero
stud); a shot-down torpedo taking the hulk pyre (it is `try_despawn`ed through
`TorpedoShotDownMarker`); shed cladding keeping `ColliderOf` and corrupting the
centre-of-mass walk (avian's remove observer strips it); the Low tier
withholding a flash (Low sets `transient_lights: 0` regardless). It re-derived
every size, speed, reach, capacity and lifetime constant against its stated
metre figure - all consistent except the four light constants above. 24 unit
tests green across the three affected filters.

Lane A did not check: the cladding shed's frame cost (UNMEASURED, no range);
isolated attribution of the 35 percent regression; `fps_within_baseline` and
`capture_simulated` (both `N/A - not claimed`, the range declares
`without_frametime()` - SKIPPED, not passed); any rendered verification;
`system_destruction_finale`, `system_section_severing`, `stress_many_structures`;
web and wasm paths; the release profile.

## Batch 2 findings: agent bench crate

### Lane A: correctness + performance

1. **MAJOR** - `observation.rs:68-71` - when the player's hull despawns, the view
   silently reframes onto an enemy ship and reports it as `me`.
   `ships.iter().find(controller == "Player").or_else(|| ships.first())`. A
   destroyed integrity root is despawned in the frame its marker lands
   (`explode.rs:176-194`), so the last snapshot has `outcome = Defeat` and no
   Player ship. The lane ran this through the real `condense`: `me.id =
   "raider_1"`, `me.health = 600/600`, `contacts = []` (the raider was filtered
   as `me` at `:76`), and the player's own round in flight re-attributed as
   `inbound` because `me_id` at `:100` is now the enemy. `Frame::of` is built
   from the enemy, so every `distance_m`/`bearing_deg`/`closing_mps` is measured
   from the enemy's nose. This is the final observation the agent gets AND the
   view `Referee::end` cuts `score.end` from (`referee.rs:329`) - the one
   artifact a human grades an open goal on. `Scorer::observe_me` correctly
   requires `controller == "Player"` (`score.rs:226`), so score and view
   disagree about who the player is.
   Fix: drop the `.or_else` fallback; `me: None` already degrades honestly and a
   test covers it.
2. **MAJOR** - `gesture.rs:183-188,237-242` - `aim.ticks` is unbounded input
   straight off the model. `parse_gesture` accepts any `u64 >= 1` and `expand`
   loops `0..*ticks`. `Referee::act` clamps only the STEP ticks
   (`referee.rs:211`); the aim's own count is never clamped, and the pi tool
   schema is `Type.Record(Type.String(), Type.Any())`
   (`tools/nova_bench/pi/index.ts:66-70`). A plausible unit slip
   (`"ticks": 2000000`, ticks for milliseconds) measured against the real
   `expand` with an 18000-tick budget: 2000001 lines, 140 MiB written to the
   game's stdin AND mirrored to `audit.jsonl`. The send loop checks no deadline,
   so `check_deadline` cannot interrupt it. One more zero is 1.4 GiB.
   `first + ticks - 1` at `:241` also overflows near `u64::MAX`.
   Pre-existing on this path; the diff adds `check_shared_keys` beside it but no
   bound. Behind the `debug` feature.
3. **MINOR** - `observation.rs:270` - the empty string is an accidental synonym
   for `expand: ["all"]`. `expands(expand, "bodies", "")` returns true when any
   asked key equals `""`. Verified: `observe {"expand": [""]}` opens every group.
   The comment at `:299-303` says `expanded` exists precisely so a typo cannot
   do this.
4. **MINOR** - `observation.rs:277-281` - with no ships at all, `frame` is
   `None` and every body is filed under `near` in full, defeating the tiering
   the module exists for. Verified: five bodies at 100 km give `near = 5,
   groups = 0`.
5. **MINOR** - `gesture.rs:84-93` - an orphan doc paragraph is glued onto
   `parse_gestures` with no blank line, so its public rustdoc opens with a
   heading for a table that no longer exists and credits the refusal to the
   expander, which does not do it - `check_shared_keys` does.
6. **MINOR** - `referee.rs:166-178` - the `ordered` map cannot preserve order.
   `serde_json` is pulled without `preserve_order` (`Cargo.toml:20`, nothing in
   `Cargo.lock`), so `Map` is a `BTreeMap` and keys come out alphabetically.
   Confirmed against real output. The name and the deliberate tick-first insert
   sequence promise what the crate cannot deliver.
7. **MINOR** - `observation.rs:789` - the fixture uses `"allegiance":
   "Hostile"`; the enum is `Player | Enemy | Neutral` (`relations.rs:26-33`).
   Inert today because `condense` does not branch on it, but `Scorer::observe`
   does (`score.rs:230`), so this fixture copied into a score test silently
   scores `kills: 0`.

Performance: no measurement taken and none warranted. `nova_bench` declares no
`nova_*` and no Bevy dependency; there is no plugin, system or `World` in the
crate, and every line runs in the referee's process between LLM turns. The real
scaling cost is TOKENS, not frames, and the two leaks are findings 3 and 4.

Lane A verified: snapshot list order is value-derived and sorted, never query
order, so no archetype-order leak into the observation; `round1` normalises
`-0.0` and NaN/inf reach JSON as `null`, not a panic; every contract key the
view reads exists upstream; no `outcome:` slug belongs to this change and
`catalog_drift.rs` correctly names none. 55 unit tests green.

Lane A did not check: any probe or rendered run (no per-frame path to justify
the slot); a live `bench play` against a real game process (the referee was
exercised only through its `Scripted` fake and a scratch harness); the
`Score.refusals` -> `bad_lines` + `cheated` rename as a format break (contracts
lane). Seed reproducibility was deliberately not raised: `--seed` is opt-in and
`replay.rs:6` states two plays of one seed are not byte-identical once rounds
fly - a declared decision predating this range.

### Lane B: craft + contracts

Lane B reached findings 1 and 2 of lane A independently, from a different
starting point. Both are corroborated by two lanes and are recorded once above.
Lane B adds these reachability details:
- `me` fallback: also reachable when the scenario's frame-budgeted spawn queue
  has not reached the player on tick 1, or from a hand-written
  `bench play <path>.ron`. `referee.rs:105-113` already detects the case and
  emits a `Note` to stderr, which the agent never sees. The `.or_else` predates
  this range; the change WIDENS its blast radius, because `bodies_view`,
  `focuses` and `is_near` are all now computed from that frame.
- `aim.ticks`: below the OOM threshold the same input silently defeats the tick
  budget. `expand`'s `end_tick = (start + ticks.max(1)).max(last)` follows the
  aim's own `last`, so one act steps the world past `--ticks 18000` and the run
  ends as `game_error` on the 120 s `STEP_TIMEOUT` rather than as `ticks`.

New findings:

8. **MINOR** - `src/pages/travel.md:25-26` - the GOTO park figure the agent is
   taught is 300 m; the engine default is 500 m.
   `FlightSettings::default().arrival_standoff` is `Meters(500.0).to_engine()`
   (`nova_ship/src/flight/state.rs:415`), and neither bench fixture nor the
   tutorial sets a per-ship override. An agent GOTOs a ship, reads
   `contacts[0].distance_m` (CENTRE to centre, so 500 m of clearance plus both
   hull radii), concludes the leg has not completed, and taps `autopilot_goto`
   again - which line 28 also tells it does nothing useful. Carried over
   verbatim from the old monolithic manual.
9. **MINOR** - `referee.rs:27` and `tools/nova_bench/pi/index.ts:104` - the
   default act length is spelled twice and only one copy is authoritative.
   `DEFAULT_ACT_TICKS = 30` is the referee's default, but the relay defaults it
   itself with `params.ticks ?? 30`, so the referee never sees an absent `ticks`
   from the pi agent. Change the constant to 20 and `baseline` and `cmd:` step
   20 while `pi` still steps 30 - the bench's whole purpose, comparing agents on
   one clock, quietly broken. The crate went to real trouble to make
   `manual::TOOLS` the single source for `--tools` and pinned it with a test;
   this constant got none.
10. **MINOR** - `observation.rs:38-43` - the first two rungs of `BANDS` can never
    be produced. `is_near` carries any body within `NEAR_SURFACE_M` (2000 m) in
    full and a near body never reaches `grouped`, so every summarised body has
    `surface_m > 2000` and `"<1km"` / `"1-2km"` are unreachable keys. Ask
    `observe {"expand": ["<1km.bow"]}` and `expanded` comes back empty - the
    exact stale-key ambiguity the field was added to remove. The threshold is
    authored twice.
11. **MINOR** - `lib.rs:48-59`, `referee.rs:19` - `check_shared_keys` and
    `shared_keys` are not in the prelude, so a consumer glob-importing
    `nova_bench::prelude` gets the parser and the expander but not the guard
    between them. `referee.rs:15-22` and `score.rs:13` reach around the prelude;
    `agent/pi.rs` says it both ways in one file. Crate-wide and predating this
    range; only the two new items are in scope.
12. **MINOR** - `gesture.rs:115` - the doc names a snapshot key that does not
    exist: "`shared` is the world's own `inputs.shared`", but `shared_keys:139`
    reads `snapshot["input"]["shared"]` and the channel emits it as `input`
    (`nova_channel/src/runner.rs:288,329`). Only `condense` renames it to
    `inputs` for the view.
13. **MINOR** - `lib.rs:29` - the crate-root rustdoc cites
    `tasks/20260824-125933/ARCHITECTURE.md` as "The design record", and this
    commit contradicted it in three named places: it still shows a `"refused"`
    block (`:125`) this change deleted, a `refusals` metric (`:196`) renamed to
    `bad_lines`, and a `--tools observe,act,finish` spawn line replaced by
    `manual::tool_list()`.
14. **MINOR** - `CHANGELOG.md:471-479` - revisions of an unreleased change were
    added beside its entry rather than collapsed into it. The bench has never
    shipped, yet two new lines frame themselves against an unreleased
    predecessor: the manual "splits into ... instead of riding in every prompt"
    and "counts unparsable wire lines instead of refusals". No released version
    had a monolithic prompt or a `refusals` metric. The commit does collapse
    correctly for the "pilot's view in meters" line and then adds three beside
    it. Separately, two PRE-EXISTING bench entries exceed 200 characters joined
    (212 and 202).
15. **MINOR** - `observation.rs:135-138` - the view emits `game_state` and
    `ui: {pause, computer}` on every observation and neither `manual.md`'s "The
    view" nor `docs/agent-bench.md`'s view list names them. An agent paused into
    NOVA OS sees `ui.computer` change with no documented meaning, while the
    `command` gesture it is told to use is the one that drives that shell.

Lane B verified: every scaled figure in `observation.rs` converts through
`METERS_PER_UNIT` exactly once, and every figure in the five pages and
`manual.md` is in metres. One raw engine-unit leak remains and was deliberately
NOT raised - `me.autopilot.engaged.target` is a bare world-space vec3 for
`AutopilotAction::GotoPos`, unreachable because no bench fixture or tutorial
beat issues a `ShipOrder`/`Move` on the player hull. `nova_bench` links no
`nova_*` crate by design, so the duplicated `METERS_PER_UNIT` and gesture
vocabulary are deliberate, not misplaced; no dependency edge was added. The
`refusals` -> `bad_lines` rename has no consumer outside the crate. The
`tutorial.content.ron` delta matches its builder one for one. `nova_bench` is
correctly excluded from the wasm32 clippy job and gated on
`cfg(not(target_arch = "wasm32"))` plus the `debug` feature. No `#[allow]`
anywhere in the crate. `cargo check -p nova_bench --all-targets` clean.
`docs/agent-bench.md` was updated in the same commit and its tables match the
code. The five `withheld_verbs` names match `tutorial/range.rs:106-112` exactly.

Lane B did not check: `content lint` (builds the whole game binary; the parity
claim rests on reading both diffs); the manual's empirical control-response
figures (27 px/deg, 18 deg/s, 80 m/s at 60 ticks, the 320-550 m/s orbit band,
turret 2000 m) - only the 300 m GOTO figure was checked, and it is wrong;
`tools/nova_bench/pi/index.ts` is type-checked NOWHERE - no `package.json`,
`tsconfig.json` or lockfile under `tools/nova_bench/`, and no CI job compiles
it, its only coverage being three string assertions in `manual.rs:113-136`.

## Batch 3 findings: bench substrate and playable ranges

### Lane B: craft + contracts

Verification run: `cargo check -p nova_channel -p nova_probe -p nova_scenario`
clean. `cargo run content lint` - 0 errors, 0 warnings, 0 findings, 9 scenarios
balance-audited, 1 acked.

1. **MAJOR** - `CHANGELOG.md:409-411` - the staged-strike entry states 500 m;
   the code stages at 1.5 km, and `CHANGELOG.md:320` says so.
   `wfc_arena.rs:2445` sets `STRIKE_STAGE_RANGE: f32 = 150.0` (world units =
   1500 m). The constant's own docstring names 500 m as the REJECTED earlier cut
   ("an earlier cut staged the pair at 500 m and let them close: by the end of
   the recording the two hulls were inside one sphere"). The Internals entry
   describes that superseded revision; the Web entry at `:320` describes the
   landed one. Two lines of one `[Unreleased]` block state the same figure three
   times apart. Also "the lances are cued" is plural for a beat that cues only
   the subject's lance and disarms every other
   (`disarm_the_rival_lances`, `wfc_arena.rs:2860`).
2. **MAJOR** - `wfc_arena.rs:2564` - `Strike`'s counters are cumulative from app
   start and unscoped, so every beat they gate can pass on evidence from before
   that beat opened. `count_strike_shots` counts EVERY lance discharge in the
   arena from the frame `arena_plugin` registers it (`:497`), with no subject
   filter and no staging gate; `stage_the_strike` (`:2756-2759`) sets `subject`
   and never zeroes `hits`/`shots`. Under capture the walk opens on
   `fight_within(STRIKE_CLOSE_BAND)`, so the AI fights freely for the whole
   approach (up to `FIGHT_DEADLINE_SECS = 100`), and by this change's own
   docstring at `:2853` the AI pulls its lance trigger "the moment the bore
   comes on ... which at the staged range is always". One AI lance shot leaves
   `shots > 0` forever, so "the lance fires" (`:3110-3115`) passes on its first
   evaluation and `cue_the_tubes` runs a frame later instead of
   `STRIKE_CHARGE_SECS = 0.9` later - the salvo buries the lance, exactly what
   that constant's docstring says the beat exists to prevent. Same shape on
   `Strike::hit`: a stray warhead within 300 m makes both salvo beats
   (`:3117-3130`) pass immediately, so the second `ScriptedTorpedoOrder` is cued
   a frame after the first instead of `STRIKE_SALVO_GAP_SECS = 1.4` later -
   inside the tube cooldown, defeating the saturation the two salvos exist for.
   Lane held no measurement slot, so the shipped webm is not proven wrong.
3. **MAJOR** - `nova_channel/src/runner.rs:287` - `applied[].state` disappeared
   from the emitted snapshot object with no version stamp moving.
   `snapshot()` merges `applied` into the object `nova_probe::capture_snapshot`
   stamps `"schema": SNAPSHOT_SCHEMA`, and `snapshot.rs:153-156` states the rule:
   "Bump it when a field changes meaning or disappears; adding a field does not
   need a bump". `state` disappeared and `SNAPSHOT_SCHEMA` is still `1`. The
   channel's own `CHANNEL_SCHEMA` cannot cover it - it is documented as
   versioning the channel's own error lines and is never written into a snapshot
   line. An existing driver reading `schema: 1` gets two mutually incompatible
   `applied` shapes and no way to tell them apart. The removal IS correctly
   marked `**(breaking)**` in `CHANGELOG.md:454-456` and no in-repo consumer
   read it, so nothing in this tree fails - external drivers get no signal.
4. **MAJOR** - `nova_channel/src/lib.rs:10-12` - the crate names an "executable
   schema reference" that still emits the removed shape.
   `tasks/20260820-174148/poc/mock_game.py:126-128` still acks
   `{"line","input","phase","state"}` with no `tick`, and `:164-172` still
   computes a `trigger_state` verdict - the two things this change deleted.
   `nova-channel.html:511` shows `"state":"Fired"` and `:541` documents
   `"state":"refused"`. A driver author building against the named reference
   gets the pre-change wire and finds out at runtime.
   `docs/agent-bench.md:168-172` is correct and could be the reference instead.
5. **MINOR** - `wfc_arena.rs:2516` - `Strike` and its two counters are not
   `#[cfg(feature = "debug")]` while everything else the staging owns is.
   `STRIKE_HIT_RANGE` (`:2421`), `struct Strike` (`:2516`), `count_strike_hits`
   (`:2544`), `count_strike_shots` (`:2564`) and the `if capturing()` block that
   registers them (`:495-497`) carry no cfg; `impl Strike` (`:2527`) and the
   rest do. `capturing()` is an env-var check, true in a default-features build
   too, so the CI default-features gate compiles a resource and two systems that
   run every frame incrementing counters nothing in that build can read.
6. **MINOR** - `nova_probe/src/capabilities/snapshot.rs:129` - the new imports
   bypass the prelude the same commit added them to. It reaches
   `NovaOsFlightLog` and friends through the module path while
   `nova_os_ui/src/terminal/mod.rs:64-74` exports exactly that set from
   `terminal::prelude`. The commit also duplicates the identical export list at
   `terminal/mod.rs:87-95` as a bare `pub use`, giving the same five names two
   re-export surfaces - which is why the bypass reads as legal.
7. **MINOR** - `nova_scenario/src/world.rs:140-145` - `is_playing_cinematic`
   restates three quarters of `is_skippable_at` (`:130-138`). The two must now
   be kept in step by hand; landing a fourth condition once leaves
   `mission.cinematic.playing` and `.skippable` disagreeing about one run.
8. **MINOR** - `CHANGELOG.md:456-458` - the snapshot's other four new fields got
   no entry while `input.shared` did. This range adds `mission.log`,
   `mission.cinematic`, `mission.cheats` (`snapshot.rs:509-566`) and each ship's
   `radar` (`:611-631`). `mission.cheats` in particular is what the bench's
   cheat detection stands on. The docs obligation IS met
   (`docs/agent-bench.md:129-176`); this is the changelog alone.

Lane B verified: the `apply.rs` rewrite is the correct simplification - `AckState`,
its two variants, `action_state()` and the `TriggerState` query are deleted
outright, removed from the crate prelude, no compatibility path kept, and the new
`an_input_ack_is_an_echo_with_no_verdict_field` test pins the exact four-key
object so a field cannot creep back. `radar_record` matches `RadarState`'s real
lifecycle, so the "null whenever the radar is not held" claim holds. No new crate
dependency edge. `STRIKE_HIT_RANGE` is authored `Meters(300.0)` and converted
with `to_engine()`, matching the Serpent's authored `blast_radius`. The two
renamed loop ids (`news-0110-point-defense` -> `loop-section-turret`, and the new
`loop-section-torpedo-bay`) both have live consumers and the frozen news aliases
are derived FROM the living name, the direction `keeping-docs-in-sync.md`
requires - no orphan. `nova_channel` and `nova_bench` are both `dep:`-optional
behind the root `debug` feature and both `--exclude`d in the wasm job.
`docs/performance.md:169-183` is still accurate. No bare `#[allow(`.

Lane B did not check: anything rendered - so the recorded loops, the actual beat
timing of finding 2, and the frame cost of the four new snapshot fields are
unverified by execution; `cargo check --workspace --all-targets` with default
features, so finding 5 is reasoned from the cfg attributes, not compiled;
`cd web && npm run ci`; `content gen` (deliberately not run - it writes).

### Lane A: correctness + performance (held the measurement slot)

Lane A reached lane B's findings 2 (Strike counters), 4 (stale executable
reference) and 5 (ungated cfg) independently. Corroborated; recorded once above.
Lane A adds that `drive_novaos.py` asserts on the deleted `state` field three
times (`:60-61`, `:72-74`, `:102`), so run against the real channel it now
raises `KeyError: 'state'` rather than reporting a failure - the in-repo
acceptance drivers the crate points at were not migrated.

New findings:

9. **MAJOR** - `nova_channel/src/apply.rs:234` - a `stop` sent while the
    action's context is lowered is dropped, leaving the synthesized key held
    down, and the ack no longer says so. A driver sends
    `{"input":"flight.main_drive","phase":"start"}` while Flight is live;
    `dispatch::apply` presses the bound `KeyCode` and records it in
    `DrivenPresses`, and bevy clears only the `just_*` edges, so the key stays
    down across frames. The driver then opens NOVA OS and sends `phase:"stop"`.
    `apply_input` takes the `!is_live(context)` branch, acks, and returns
    without ever calling `dispatch::apply(.., Release)`. When Flight comes back
    up **the drive is nailed on**, and the driver cannot know: the ack is now
    byte-identical to a release that landed, because the `state: "refused"` this
    diff deleted was the only signal. `DrivenPresses` is never serialized into
    the snapshot, and `input.live` answers "is the context down", not "was my
    release swallowed". `apply_section` has the same shape at `:255-261`.
    The module doc at `apply.rs:17-19` already states the invariant every
    synthesized event gets its Released twin; the named-input lane breaks it.
    Fix: gate only the PRESS on liveness. `held_source` (`dispatch.rs:166`)
    already resolves the source the press pushed, so an unconditional Release is
    correct and idempotent.
    The swallowed release predates this diff; what this diff added is the loss
    of the field that made it observable.
10. **MAJOR** - `nova_probe/src/capabilities/snapshot.rs:629` - `radar.dwell_fill`
    is published unguarded, so it reads 1.0 - "locked" - whenever the radar is
    held and nothing is dwelling. `RadarState::dwell_needed` is `0.0` when not
    dwelling (`targeting/state.rs:153`) and `dwell_fraction` returns `1.0` for a
    non-positive `needed` (`:161-163`). An agent holds `flight.radar_hold` with
    the ray on empty space and gets `dwell_target: null, dwell_secs: 0.0,
    dwell_needed: 0.0, dwell_fill: 1.0`. The bench manual tells the agent
    exactly what to do with that: "read `dwell_fill` instead and hold until it
    reaches 1" (`nova_bench/src/pages/targeting.md:23`, `manual.md:139,162`). It
    reaches 1 immediately, the agent releases, no lock is taken. The HUD does
    not make this mistake - it filters on `is_dwelling()` first
    (`nova_hud/src/lock_dwell_ring.rs:161`); the snapshot skips that guard.
    `radar_record` has no test, and the only `dwell_fill` fixture in the tree is
    the charging case.
11. **MINOR** - `nova_input/src/registry.rs:628` - `ActionName` is now written by
    every rig and read by nobody, and its docstring names the reader this diff
    deleted ("so a runtime caller - the process channel echoing an ack - can
    read a `TriggerState` back BY NAME"). That caller was `action_state`,
    removed here. A full-tree grep returns three hits: the prelude re-export,
    the `bundle` that stamps it, and the declaration.
12. **MINOR** - `nova_authoring/.../tutorial/mod.rs:567` - the `target_1_locked`
    guard is a bug fix with no test that fails without it.
    `cargo test -p nova_authoring --lib tutorial` passes 15/15 both with and
    without the filter: `a_target_shot_apart_ahead_of_its_lesson_moves_the_card_on`
    matches the handler by its `target_1_down` filter with `.any`, so the added
    filter is invisible to it, and no test mentions `VAR_TARGET_1_LOCKED`.
    Delete the line and the regression comes back green.
13. **MINOR** - `nova_channel/src/runner.rs:329` - the new wire surface ships
    with no test on either side. `runner.rs` has no `#[cfg(test)]` module at
    all; `cargo test -p nova_channel --lib` runs 13 tests, all in `apply` and
    `protocol`, none of which build an input block. `mission.cinematic` reads a
    brand-new public API, `NovaEventWorld::playing_cinematic`, which the 18
    cinematic tests never call. `ship.radar` is likewise uncovered. The
    consequence is already on the record: batch 2 found the bench reading
    `snapshot["input"]["shared"]` against a view that renames it, which one
    round-trip test on either end would have caught.

Performance, stated plainly.

MEASURED. `probe run system_torpedo_launch` at `b8d17f99`, dev profile, vulkan /
RTX 3060 Ti / 1280x720: GREEN, 7 of 8 checks PASS, `run_end` at frame 1847, 0
invariant violations, 153 s. Frame set: 900 frames, mean 21.08 ms, p50 20.83,
p95 24.52, p99 28.25, mean 47.4 fps, 1% low 35.4. `fps_within_baseline` is
**N/A - "no baseline"**, and `docs/performance.md` is explicit that dev-profile
numbers are not baselines, so this is a RECORD, not a verdict. The six new
bay-loop beats run on the smoke path and add ~2.6 s of walk; `loop_start` /
`loop_end` / `loop_written` are no-ops off the capture path, so the range does
not stall waiting for a webm. `probe run wfc_arena --correctness-only`: GREEN,
330 frames, 22 s.

UNMEASURED, reasoned from code. The one per-frame change on a non-capture path
is `CAMERA_BASE: 55.0 -> 20.0` with `CAMERA_PER_SPREAD: 0.85 -> 0.80`
(`wfc_arena.rs:1755-1757`) - a 2.75x closer standoff floor at merge on what the
file calls the busiest scene the site ships, with shed cladding and debris now
passing near the lens. A plausible fragment/overdraw cost on exactly the frame
the probe's fps pass records. The lane refused to assert a number: `probe-runs/`
holds no prior set for either range at any commit, `fps_within_baseline` returns
"no baseline" for both, and an honest A/B needs two cold `--release` builds with
no `target/release` in the tree. Host quiet throughout (load 0.30, no user game
process).

No new per-frame cost found elsewhere. `capture_snapshot` is on-demand, not a
system. The new `mission.log` walk is O(flight-log entries) per snapshot and its
producer is correctly gated on `resource_changed`. `radar_record` is one
`world.get::<RadarState>` per ship.

Lane A verified: `input_block` determinism - `InputBindings::actions` is a `Vec`
in registration order and `live`/`contexts`/`shared` are all sorted, so nothing
new bypasses the sort; `shared` is complete for its purpose via
`InputBindings::conflicts`. `sequences` is a `Vec` in start order, so the
cinematic `find` is deterministic. The tutorial `target_1_locked` set/guard pair
holds on every beat that can reach it, including the killed-in-the-gap path and
the restart reset. `count_strike_hits` uses `Added<T>` correctly in a system.
Tests green: `nova_channel` 13, `nova_probe` snapshot 11, `nova_scenario`
cinematic 18, `nova_authoring` tutorial 15, `nova_probe_cli` catalog_drift 2. No
new `outcome:` slug in either range and the roster matches both ways.

Lane A did not check: any release-profile frame set or A/B for the `wfc_arena`
camera constants (the cost claim is UNMEASURED); `stress_point_defense` and
`stress_torpedoes` were not measured; the bay-loop FRAMING was not rendered.
Lane A flags one contradiction it could not settle without frames -
`frame_the_bay_muzzle` says "the lens sits AFT of the door face ... looking
forward along the hull", but `eye` is local `z = -4.0` against a muzzle at
`z = -1.0`, and the file's own prior comment established local -Z as downrange.
By that reading the lens is ~30 m in FRONT of the door, on the side the comment
says costs the round. Either the comment or the pose is wrong. Carried to
batch 6.
It also left one smell unreported rather than ground it: the `NovaOsFlightLog`
"objective disappeared, therefore completed" inference feeding `score.rs:215-222`.

## Batch 4 findings: loop captures and shared staging kit

### Lane B: craft + contracts

1. **MAJOR** - `loop_damage_sequence.rs:127` - "110 m stem to stern" is the
   gunship's bounding-sphere DIAMETER, not its length. **Adjudicator re-derived
   from `assets/base/ships/base.content.ron` section positions**: `block_gunship`
   spans cells x -2..2, y -1.75..2, z -4..3.5 = 85 m long, 50 m beam, 48 m tall,
   bounding radius 54.7 m, diameter 109.5 m. `web/src/wiki/flight-autopilot.md:41`
   quotes the same hull as "its own 55.2 m hull" - 2 x 55.2 = 110.4, which is
   where the figure came from. Every frame-fraction claim built on it is wrong:
   at `STILL_STANDOFF = 135.0` the frame is 1.47 x 135 = 198 m wide and the 85 m
   hull is 43% of it, not "a little under two thirds". The figure drives four
   framings in this bundle (`loop_damage_sequence.rs:127,244`,
   `loop_torpedo_blast.rs:84`, `loop_goto_arrival.rs:75` - "half the frame
   width", actually 39%) and two outside it (`screenshot_railgun.rs:615`,
   `screenshot_menu.rs:52`), plus three in `wfc_arena.rs:1756,1783,2447`.
2. **MAJOR** - `loop_cockpit.rs:66` - the new framing is sized for a 110 m
   gunship; this loop flies the 70 m Utility Cutter. `load_scene` calls
   `ring::the_ring`, which is `the_ring_with_hull(.., "block_cutter")`
   (`shared/ring.rs:134`). **Adjudicator re-derived**: `block_cutter` is 70 m
   long, 50 m beam, 20 m tall. The docstring derives its stand-off from "150 m
   puts a 110 m hull across half the frame"; the vector it justifies puts the
   lens 176 m out, so the frame is ~259 m wide and the cutter is 27% of it.
   Line 84's "a 110 m ship reads as its 30 m beam" is wrong on both figures -
   the cutter's beam is 50 m and its height is 20 m.
3. **MINOR** - `shared/kit.rs:85` - `clad` is added, documented in the module
   header as one of the kit's two ship entry points, and called by nobody. The
   file's `#![allow(dead_code, reason = "one source, many example targets")]`
   hides it, and that reason does not apply because no producer uses it.
4. **MINOR** - `loop_command_shell.rs:271` - `the_prompt_reads` re-implements
   `nova_os_command_line_reads` (`nova_debug/src/harness.rs:395-397`), which is
   already in scope through `nova_protocol::prelude`. The local copy also forces
   the bespoke deep import
   `nova_protocol::nova_os_ui::nova_os::prelude::NovaOsTerminal` that the shared
   helper avoids.
5. **MINOR** - `loop_goto_arrival.rs:5` - the module doc and a step label still
   say "fixed cameras" after this change made them flying leg cameras.
   `start_cut_camera`/`drive_cut_camera`/`settle_camera` now all call
   `ring::leg`, inserting a `LegCamera` re-solved every frame. The file's own new
   `CUTS` docstring says the opposite in as many words.
6. **MINOR** - `shared/ring.rs:76` - `ORBIT_RADIUS`'s docstring still quotes the
   pre-planet orbit band. The set body is now
   `PlanetConfig::new(PlanetType::IceWorld, Meters(900.0), 4711)`, so the derived
   radius is 940.5 m and `orbit_band_floor` is 1.43 km. Line 76 still says
   "roughly 1.22 to 3.75 km"; the module header at `:29` was updated in the same
   commit to "1.41 to 3.75 km", so the file contradicts itself.
7. **MINOR** - `loop_turret_stow.rs:59` - `MOUNT_SEAT * 10.0` hand-writes
   `METERS_PER_UNIT`. `nova_events/src/units.rs:26-32` states the rule this
   breaks: "code that multiplies by it by hand is code that can forget to".
   Nothing at that line says the literal is the cell-to-metre crossing.
8. **MINOR** - `shared/{ring.rs:285, hollow.rs:338, drydock.rs:176}` - three
   byte-identical `ship()` helpers, all three edited identically by this change
   (`sections: Vec<...>` -> `hull: ShipHull`). All three set modules already
   include the kit by `#[path = "kit.rs"]`, which exists to hold exactly this.
9. **MINOR** - `loop_goto_arrival.rs:88` - the `CUTS` docstring states a
   constraint four of its own ten rows break: "the side offset kept under a
   third of it". Rows 1, 4, 6 and 9 are 0.344, 0.415, 0.363 and 0.386.
10. **MINOR** - `web/src/wiki/nova-os.md:16` - the figure note promises a
    monitor close the producing loop never records. `nova-os-open.webm` is an
    alias of `landing-cockpit`, whose script presses Tab, types `map`, presses
    Enter, holds 1.5 s and calls `loop_end` - no second Tab, no close. The note
    says "the close collapses the picture to a dying dot". Predates this range,
    but the change re-shot the loop.

Lane B verified: `cargo check` clean for both new examples WITH and WITHOUT
`--features debug` (the `-D warnings` default-features path), and clean for all
twelve consumers of the reworked kit API. `content lint` 0/0/0. **Loop-id
contract traced both directions**: all seven produced ids plus the new still are
present in `capture-web-media.sh`, `manifest.txt` and a page consumer, and no
consumer names an id nothing produces; the news-freeze alias rule holds
(`news-0120-blast` is a destination, never a source). Both new loops use
`AppBuilder`; ring randomness is seeded and the new `PLANETOID_SEED` removes a
per-run global-RNG dependency. Deadlines run on `Time<Real>`, so the paused-shell
step cannot hang. The four CHANGELOG entries this bundle produced are all under
200 characters and correctly under Web & Platform.

Lane B did not check: anything rendered - every framing finding is arithmetic
against the catalog and the code, not against a captured frame; `cd web && npm
run ci`; the encoded `.webm` contents; `content gen`. Two stale `shared/kit.rs`
references at `ring.rs:90,340` were traced by `git blame` to before
`origin/master` and are out of range.

### Lane A: correctness + performance (held the measurement slot)

This lane inspected the SHIPPED webm frames against `origin/master`'s, so its
top findings are defects in the artifact the site serves today, not predictions.

11. **MAJOR** - `loop_damage_sequence.rs:247-257` - the re-staged damage loop
    records the gunship STERN-ON, so none of the three damage events the loop
    exists to show are in frame. The walk breaks `BROADSIDE_CELLS` (all
    `x = -1.0`, port flank), kills `pdc_forward_port` and severs
    `(-1.0, 1.0, 1.0)` - every event on the port side. The recording pose moved
    from `(-62,23,54)` to `(-88,32,81)` while the subject also gained
    `Quat::from_rotation_y(-0.55)`. Sampled the shipped
    `landing-damage-sequence.webm` at n=5/55/100/150: **every frame is the drive
    bell filling the centre of the shot**. `origin/master`'s webm of the same
    5.533 s script at n=100 shows a broadside with the freed section and orange
    crack decals. The file's own new `frame_the_wreck` docstring diagnoses
    exactly this ("a constant world pose lands on whichever side happens to be
    turned toward the lens ... the drive bell, as it happens, with every hit on
    the far side") and fixes it for the STILL only - the still gets a hull-space
    re-solve, the loop keeps the constant world pose. The landing page's damage
    feature row now shows a stern.
12. **MAJOR** - `shared/kit.rs:59-71` - `catalog_ship` returns the catalog hull
    with its cladding ON, and the cladding hides the crack decals the damage
    loop and the damage still both exist to show. The loop switched from
    `catalog_hull` (bare cells, `skin: false`) to
    `ShipSource::Inline(kit::catalog_ship(...))`, which carries `skin: true`.
    Cracks are per-SECTION mesh material swaps into `SectionCracksMaterial`
    buckets (`nova_ship/src/sections/damage_cracks.rs`); the derived skin is a
    separate surface laid over those section meshes, so a swapped bucket is no
    longer the visible material. Evidence isolating cladding from framing: the
    NEW `wiki-ships-damage.png` is correctly framed on the port flank - the torn
    open aft bay is plainly visible - and **still carries no crack decals
    anywhere**, where `origin/master`'s loop showed them clearly on the bare
    hull.
13. **MAJOR** - `loop_goto_arrival.rs:141-150` - the settle beat holds an
    `along`-dominant bearing resolved from RESIDUAL DRIFT and records 1.5 s of an
    end-on hull on black. `settle_camera` installs `ring::leg(world, 40, 140,
    15, -20)` - 140 m almost entirely down the track. `ring::leg` resolves that
    through `ring::ship_heading` (`ring.rs:615-629`), which returns
    `velocity.try_normalize()` for ANY non-zero velocity, and the beat's own
    docstring concedes "parked is the autopilot's word for a ship that has
    stopped closing, not for one that has stopped moving". Sampled
    `goto-arrival.webm`: n=20 is a good three-quarter with the planetoid in shot;
    **n=150 and n=320 are the drive bell centred with the world out of frame
    entirely**. The same commit's `loop_cockpit.rs:78-97` names this exact
    failure for the cockpit leg while the arrival's `CUTS` put nearly the whole
    stand-off on `along` for all ten.
14. **MAJOR** - `loop_cockpit.rs:109` - the cockpit loop keeps the HUD up, so a
    dev build hash is baked into shipped site footage. The script's
    `.on_enter(ring::hud_instrument)` sets `HudVisibility::On`
    (**adjudicator confirmed at `ring.rs:312-314`**) rather than calling
    `hide_hud`, whose docstring says it exists "so the fps/version bar is out of
    shot". The shipped `landing-cockpit.webm` now reads
    `30 fps | v 0.12.0+a5da7efdb` in-frame, where `origin/master`'s read
    `v 0.11.0`. `command-shell-open.webm` has the same problem from a different
    source: its CRT header renders `NOVA OS v0.12.0+a5da7efdb // COMMANDS`
    twice. Every re-shoot changes that string, so the landing page carries a
    commit hash that ages the moment anything lands, and the fps counter puts
    capture-rig cadence on the marketing page.
15. **MINOR** - `loop_torpedo_blast.rs:368-370` - `no_torpedo_in_flight()`
    reports "the salvo has landed" when the TARGET is gone, not when the
    torpedoes are. `torpedo_range` short-circuits to `None` via `?` on a missing
    target, so the predicate flips true the instant the target root despawns
    even with a full salvo in flight - and the target root despawning is the
    entire point of the loop. The `or(...)`-fallback shape wearing a `?`.
16. **MINOR** - `shared/kit.rs:85` - `clad()` has zero callers. (Same finding as
    lane B item 3; corroborated. Lane A adds that it is the exact helper the
    cladding finding above needs a caller for.)
17. **MINOR** - `loop_goto_arrival.rs:33` - the cut installer runs AFTER the
    camera solver, so every cut lands one frame late.
    `(ring::drive_leg_camera, drive_cut_camera).chain()` orders the solver
    first, so `drive_leg_camera` has already posed the camera from the OLD
    bearing. With `CUT_INTERVAL = 0.65` at 30 fps that is one frame of 19-20 per
    cut, and the lerp makes it a smear rather than a cut.
    Fix: swap the chain order.
18. **MINOR** - `shared/ring.rs:457-465` - `engage_orbit` takes the first
    `GravityWell` by ARCHETYPE ITERATION ORDER
    (`query_filtered::<Entity, With<GravityWell>>().iter(world).next()`). One
    well today, so correct now; the moment the set gains a second body the
    choice becomes unstable and unnamed.

Performance: UNMEASURED, and unmeasurable as wired. All seven producers use
`NovaProbePlugin::default().without_frametime()`, so both new loops report
`capture_simulated N/A` and `fps_within_baseline N/A` in
`probe-runs/b8d17f990/*/checks.json`. N/A is unmeasured, not passed. The only
number obtained is a capture-cadence RECORD (n=331: min 27.5, p50 39.8, p90
45.1, p99 52.9, max 71.2 ms) which includes PNG readback and encode and is
therefore not a scene frame time; no named baseline exists, so no verdict is
asserted from it. Host quiet at both points (load 0.03, 0.29).

Reasoned, labelled unmeasured: **no wfc_arena-shaped overdraw regression here** -
the two stand-offs that changed moved AWAY from the subject
(`loop_torpedo_blast` ~244 -> ~322 m, `loop_damage_sequence` ~76 -> ~115 m),
the opposite direction from batch 3's `CAMERA_BASE 55 -> 20`. The ring set's
`planetoid()` swap (Asteroid -> IceWorld, radius 200 -> 900) plausibly adds draw
and shading cost across all five ring producers; unmeasured, not guessed. The
sizing claim checks out: the run log printed `radius Some(94.049995)` world
units = 940.5 m, matching 900 x 1.045 relief exactly. No per-frame allocation in
the staging systems.

Batch-carried questions, answered:
- Batch 3's cumulative-counter shape is NOT present here. `loop_turret_stow`'s
  beat gates read `TurretStow::phase()`, a level, not a counter. Live run walked
  Stowed -> Deployed -> Stowing -> Stowed cleanly.
- The `frame_the_bay_muzzle` AFT/FRONT sign contradiction has NO sibling in
  `shared/`. Every camera-pose helper was checked against its docstring;
  `lit_side` guards its degenerate case, `ship_heading` guards zero velocity,
  and the sign conventions in `leg`/`chase`/`lead` match their prose.
- `outcome:` markers: NO FINDING, and the brief's premise was wrong.
  `Cargo.toml`'s three-category contract makes `outcome:` a `systems/`
  requirement; `screenshots/` producers are graded on the WALK and judged by
  eye. No `probe_marker` exists in any `examples/screenshots/*.rs`. Both new
  examples ARE correctly rostered and `catalog_drift` passes 2/2.
- Frame cap: all safe against `LOOP_FRAME_CAP = 600`.

Lane A ran: `cargo check --features debug --examples` clean; `catalog_drift`
2/2; `probe run loop_turret_stow` OK 6/8 measured; `probe run loop_command_shell`
OK 6/8; smoke runs of three loops; a full `NOVA_CAPTURE=1` capture of
`loop_cockpit` (332 frames). Xvfb helper PID 872194 stopped by recorded PID.

Lane A did not check: live runs of `loop_torpedo_blast`, `loop_damage_sequence`,
`loop_goto_arrival`, `loop_spine_cut` - findings against those are from shipped
webm frames plus source, not from a run it drove; any repeat set or before/after
for the ice-planet swap (impossible as wired); `hollow::lead_torpedo_position`
(consumer is another lane's file). It explicitly flagged one judgement call:
whether the loop-vs-still framing split in `loop_damage_sequence` was a
deliberate owner decision - `frame_the_wreck` fixes the still and knowingly
leaves the loop on a constant pose, which could be intent. It reports it as a
defect because the loop's own module doc says the damaged side has to face the
lens.

## Batch 5 findings: still captures

### Lane B: craft + contracts

1. **MAJOR** - `screenshot_gravity.rs:6` - the producer is orphaned from the
   still pipeline and its own module doc says the opposite.
   `gen-web-screenshots.py` dropped `("wiki-gravity.png", "screenshot_gravity")`
   from `FIGURES` and replaced it with the alias
   `"wiki-gravity.png": "tutorial-orbit.png"` (`py:268`) - this producer's only
   manifest slot. At HEAD it writes exactly one file, `feature-gravity.png`
   (`:101`), and that name appears in NO manifest group.
   **Adjudicator verified**: `gen-web-screenshots.py --producers` returns 22
   entries and `screenshot_gravity` is not among them; `grep feature-gravity
   scripts/gen-web-screenshots.py` returns nothing. So
   `scripts/capture-web-shots.sh` never builds it, never runs it, and never
   captures it; run it by hand and the packager still copies nothing. The file's
   doc line 6 states "Ships one manifest image: `feature-gravity`", which is
   false, and `docs/development.md:354` still lists it and then asserts the
   scripts "name every producer and every file it writes", now false for this
   one.
   Fix: decide its fate and make one story true - add the `FIGURES` row and give
   a page a reference, or delete the example, its `Cargo.toml:373` block, the
   `docs/development.md:354` mention and the `docs/automation-harness.md:518`
   sample command.
2. **MAJOR** - `screenshot_scenario_picker.rs:174` - the "capture the campaigns
   news frame" step writes a PNG no packager path copies.
   `news-090-scenario-campaigns.png` is not in `FIGURES`; its only slot is
   `ALIASES` (`py:228`). The alias pass begins `if os.path.exists(stage_dir,
   alias): continue  # a distinct capture exists; process_group handled it`
   (`py:1106-1108`), but `process_group` iterates only `FIGURES` and
   `THUMBNAILS`, so for an alias-only name it handled nothing. The capture is
   staged, the alias loop sees it and `continue`s BEFORE the `frozen()` branch,
   so the run prints no line for that name at all, and the capture is silently
   discarded. The trap this hides: `NOVA_UNFREEZE=news-090
   scripts/capture-web-shots.sh` is the documented way to re-cut a post's
   figure, and on this name it does nothing at all, silently.
3. **MAJOR** - `screenshot_railgun.rs:159` - two new `NOVA_*` variables ship
   undeclared in the environment index. `AFTERMATH_ENV =
   "NOVA_RAILGUN_AFTERMATH"` (`:159`) and `LIVE_ENV = "NOVA_RAILGUN_LIVE"`
   (`:202`) are load-bearing - `capture-web-media.sh:101-102` sets them to choose
   between the two shipped railgun loops - and `docs/environment-variables.md`
   is "the INDEX ... Every `NOVA_*` variable the game reads", carrying an
   explicit enumeration of example-local knobs at `:176-178`. Neither name is
   there. A contributor re-records `loop-section-railgun-live` with the default
   slowed clock, `live_cut()` returns `false` for an unset variable silently,
   and the site carries two identical loops labelled "slowed" and "real speed".
   `tests/env_contract.rs` cannot catch it: its scan walks `crates/`, `src/` and
   `tests/` only.
4. **MINOR** - `screenshot_railgun.rs:606` - the HIT framing is off the target's
   PORT bow, not its starboard. In this file's vocabulary starboard is local
   `+X` (`boat_hull` puts `mount_starboard` at `x=+1`), the target spawns with
   `Quat::from_rotation_y(PI)` mapping local `+X` onto world `-X`, and
   `GAP.eye` is world `+X` - the hull's port side. `:617`'s "The slug crosses
   from the right" is wrong the same way: the shooter's projection on the
   camera's right vector is -207 m, so it travels left-to-right.
   `web/src/wiki/combat-weapons.md:56` says only "bow quarter", so the page has
   not inherited the error.
5. **MINOR** - `screenshot_railgun.rs:114` - two doc comments state the range as
   320 m; `TARGET_Z` is `Meters(-220.0)`. Wrong since the file was added, so a
   figure never derived rather than an edit gone stale.
6. **MINOR** - `screenshot_radar_lock.rs:171` - this change created a FOURTH
   byte-identical `ship()`. The edit (`sections: Vec<...>` -> `hull: ShipHull`)
   made `:171-193` character-for-character identical to the three copies in
   `shared/{ring,hollow,drydock}.rs`. Before the change it was at least a
   different function.
7. **MINOR** - `screenshot_railgun.rs:525` - `hold_the_boat` re-types
   `hollow::pin_player` byte for byte, and its own doc says so. Two more in this
   range: `set_time_scale` (`:713-717`) is byte-identical to
   `loop_turret_stow.rs:195-199`, both added here; and `screenshot_menu.rs:212`
   `named_ship` is byte-identical to `kit::ship_root` - that file simply does
   not include `kit.rs`. `docs/development.md:347-353` explicitly accepts
   duplication BETWEEN producers; what it does not accept is re-typing a helper
   the `shared/` tree already exports.
8. **MINOR** - `screenshot_torpedo_run.rs:71` - a constant was inserted under an
   existing doc block with no blank line, so `ORDNANCE_STANDOFF` now carries
   `torpedo_run_script`'s three-line contract ("Every capture is its OWN step
   held until the PNG is on disk") and `torpedo_run_script` has no doc at all.
9. **MINOR** - `screenshot_torpedo_run.rs:114` - the rock shell's stated
   thickness is half its authored spread. `NearField::action` writes
   `y_min: -y_spread, y_max: y_spread`, so a `y_spread` of 460 m spans 920 m.
   Also wrong at `shared/hollow.rs:70`.
10. **MINOR** - `screenshot_railgun.rs:313` - the `section`/`at` closure pair is
    written twice in one file (`:314-326` and `:370-382`), with a third copy in
    `shared/showcase.rs:37-49` and a fourth in
    `examples/systems/system_railgun_lance.rs:216-229` - the latter also
    duplicating this file's whole lance rig down to the reasoning.
11. **MINOR** - `screenshot_railgun.rs:646` - `BENCH.look_at` destructures and
    re-assembles `BENCH_AT`, which is `Copy` and legal directly.
12. **MINOR** - `screenshot_gravity.rs:14` - the module doc still says "walk both
    framings" and the clap `about` still reads "framed from the yard and from
    close in" after the second framing was deleted.
13. **MINOR** - `CHANGELOG.md` - lane B measured all 163 `[Unreleased]` entries
    with lines joined. The three still/loop-reshoot entries are 194, 198 and 194
    - inside the limit. SIX are over, all in other lanes' subsystems: "New Game
    starts Basic Training" (204), "**(breaking)** The game ships no campaign"
    (243), "**(breaking)** `StoryMessage` is `NarrativeCue`" (206), "A death
    burns" (211, already raised in batch 1), "Under capture only, `wfc_arena`
    stages its strike" (222), "A `range` bench fixture with no objective" (210).

**Craft verdict on the +877**: the growth is NOT misplaced game logic.
`screenshot_railgun.rs` builds with `AppBuilder`, keeps every gameplay behaviour
on the production path (a `ScriptedRailgunOrder` rather than a synthesized
trigger, `RailgunFired` observed rather than frames counted), and stages a range
because no shipped hull carries a lance. Roughly 300 of the 877 lines are doc
comments recording why each pose is where it is, which is the house idiom. What
should move out is small and specific: `hold_the_boat` and `set_time_scale` to
`shared/kit.rs`, and the duplicated closures to one file-level helper.
The real-speed staging does NOT introduce a second way to express a timing the
first way decided: the four env-or-const pairs each select between two genuinely
different values for one run, and both env readers reject a malformed value with
a panic rather than a silent default.

**Figures re-derived and CORRECT - the fix pass must not touch these**:
`railgun.rs:81` "53 cells" (the RON gives `block_gunship` exactly 53 sections);
`:86` "18 km reach" (15000 m/s x 1.2 s); `:17-18,106,114` charge 1.5 s, 15000
m/s, quarter speed = six seconds; `:614` "150 m out" (150.5); `:627` "117 m out"
(117.1); `:831-832` 0.13/0.05 = 2.6 s; `:7,460-462` bow-on via
`from_rotation_y(PI)` on a `-Z`-forward hull; `screenshot_menu.rs:46,52` "off
the starboard bow", "about 170 m out" (168.6), "a frame 250 m wide" - only the
"110 m stem to stern" clause on `:52` is the known-bad figure;
`screenshot_gravity.rs:84,86` "5.9 km back" (5929 m), "29 degrees apart" (27.6);
`screenshot_flip_burn.rs:106,136` lead values; `screenshot_torpedo_run.rs:98`
fuze 150 m against capture range 180 m.

Contract traces CLEAN: every still id these eight write has a manifest slot
except finding 1 and the `damage-levels*` family; the reverse direction is clean
with one PRE-EXISTING exception (`news-0110-damage-levels.png` is a `FIGURES`
row naming a producer that never writes it - frozen, so nothing breaks today,
and not caused by this range). `loop-section-railgun` and `-live` are both
`capture-web-media.sh` rows and both embedded by `wiki/sections/railgun.md`.
`wiki-gravity.png`'s new alias to `tutorial-orbit.png` is matched by the figure
note rewrite at `wiki/gravity-wells.md:15-18`. No runtime identifier in these
eight was renamed. `cargo check --features debug` clean over all eight, and
again with default features under `RUSTFLAGS="-D warnings"`. `content lint`
0/0/0.

Lane B did not check: anything rendered - every framing claim is arithmetic on
the pose constants, and a pose that is geometrically right can still be a bad
picture; the shipped PNG/webm binaries were not decoded; `web/tests/assets.test.js`
and `npm run ci`; the wasm32 job; `cargo test` anywhere including
`tests/env_contract.rs`.

### Lane A: correctness + performance (spent the slot on image inspection)

14. **BLOCKER** - `screenshot_radar_lock.rs:112` with the pose at `:252-258` -
    cladding the gunship without re-posing turned the shipped `wiki-radar.png`
    into a close-up of hull plating. `ship(...)` now takes
    `kit::catalog_ship(ships, "block_gunship")` - the catalog hull WITH its
    derived skin - but the "frame the radar instrument" beat still poses at
    `START_POSITION + Meters3::new(-36.0, 10.0, 60.0)`, a framing measured
    against the bare cell list. The clad hull is 85 x 50 x 48 m, so that lens
    sits 11 m outside the 25 m half-beam and 17.5 m past the 42.5 m half-length,
    roughly 15 m off the clad hull's corner and well inside the 54.7 m bounding
    radius the eye is only 70.7 m from.
    **Adjudicator opened both images and confirmed**: `origin/master`'s frame
    holds the whole corvette with its lit drive, cockpit glass and the nav
    bracket; HEAD's right half is a featureless grey plate with no drive, no
    cockpit and no readable hull, and the `0.0 m/s` chip is buried in the
    plating. `web/src/wiki/targeting-radar.md:6` ships it as that page's lead
    figure, whose note asks for "the hollow radar box and a lock landing".
    Fix: re-pose for the clad hull (past the ~55 m clad bounding radius plus
    margin), then re-shoot.
15. **MAJOR** - `screenshot_railgun.rs:114-118` and `:831-834` - slowing
    `Time<Virtual>` cannot spread the slug's flight over frames, so both loops
    promise footage they do not contain. The slug is a spawned physics body
    (`railgun_section/firing.rs:187-189`) at `slug_speed: 15000.0` m/s
    integrated inside `FixedUpdate`. `set_relative_speed(0.05)` does not
    subdivide the fixed step - it makes fixed ticks RARER while each still
    advances 1/64 s of world, i.e. **234 m of flight, more than the 220 m gap**.
    The whole flight lands inside one fixed tick at every scale and no rendered
    frame can hold the slug. Confirmed in the shipped media: in
    `loop-section-railgun.webm` frame 16 is the intact hull with the sight
    thread and no slug, and frame 18 already has the entry blown out and the
    wake fully extended; the slug appears in NO frame of either loop. So
    `SHOT_TIME_SCALE`'s "A twentieth of real time spreads it over roughly a
    dozen" is wrong, and `:831-834` promises "the muzzle flash, the slug
    crossing the gap" in footage recorded from `GAP`, whose lens has the gunboat
    and its muzzle BEHIND the camera (the gunboat projects to depth -61.9 m
    along the view axis). `LOOP_OPEN_CHARGE` at `:179-181` justifies `0.97` by
    "the muzzle flash is inside the window" for a loop whose muzzle is
    off-camera.
    Fix: say the scale spreads the DEBRIS and the fireball chain - the part that
    is true and earns the constant - and drop the muzzle/slug clauses.
16. **MINOR** - `screenshot_torpedo_run.rs:96-101` - `RUN_IN_CAPTURE_RANGE`'s
    stated reason is stale by ~120 m. The docstring says a proximity fuze goes
    off at `TORPEDO_FUZE_RANGE`, "so this is the last moment there is still a
    torpedo to photograph". Untrue since the contact fuze landed: a locked BODY
    is fuzed on its own skin at `CONTACT_FUZE = 3.0` world units
    (`torpedo_section/projectile.rs:124,205-207`), and that module says so at
    `:431-433`. The shipped `wiki-combat-torpedo.png` confirms it - the lead
    torpedo is about 140 m from the raider's origin, inside the claimed fuze
    range, still alive. Error is in the safe direction.
17. **MINOR** - `screenshot_torpedo_run.rs:86-93` - `RUN_IN_BIAS` does not do
    what its docstring says, and the shipped frame shows it. The comment says
    biasing back toward the hull "brings the ship being shot at to the centre";
    projecting the actual pose puts the raider at 25% from the left edge and 76%
    down, the torpedo at 80% right and 18% down - the corners-and-empty-middle
    arrangement the comment says the bias avoids.
18. **MINOR** - `screenshot_railgun.rs:813-816` - the live cut's "run the charge
    out" beat waits on state from before it opened.
    `.until(charge_at_least(loop_open_charge()))` resolves to `SIGHT_CHARGE`
    (0.80) on the live path, and "frame the sight" at `:790-794` already ran the
    charge to >= 0.80. The beat performs no wait, and its deadline can only fire
    in the one case where the charge has already COMPLETED - the gun fires
    early, `RailgunCharge` returns to `Ready`, `progress()` reads 0.0, and the
    beat stalls 30 s and error-exits. Failure is loud, not silent.
19. **MINOR** - `screenshot_flip_burn.rs:120-124` - the new "frame the flip"
    reason does not match the frame it produces. The comment says the offset
    fixes it so "the drive fires at the camera and the hull is read three-quarters
    on"; the shipped `wiki-flight.png` is a near-plan view of the flat hull -
    whole deck, hazard stripes, both bells' top faces, no plume.
    `ring::LegCamera`'s `up` is world Y, so the offset fixes the lens at 14.6°
    over the TRACK and nothing controls elevation over the hull's own deck,
    whose roll after an end-for-end swing is whatever the autopilot left.
20. **MINOR** - `screenshot_flip_burn.rs:135` and `screenshot_radar_lock.rs:271`
    - the `HudVisibility::On` version bar is in this bundle too, so
    `wiki-flight.png` and `tutorial-radar-lock.png` also ship the burned-in bar.
    **Adjudicator nuance**: the bar itself is PRE-EXISTING - `origin/master`'s
    `wiki-radar.png` reads `34 fps | v 0.10.0`. What is new is the `+<sha>`
    dev-hash suffix (`v 0.12.0+a5da7efdb`). The fix should target the hash
    suffix in capture mode, not necessarily the whole bar.
    Clean in this bundle and not to be touched: `screenshot_railgun:274`,
    `screenshot_menu:117`, `screenshot_scenario_picker:110`,
    `screenshot_damage_levels:186`, `screenshot_gravity:61` all call `hide_hud`,
    and `screenshot_torpedo_run` gained `hollow::hud_cinematic`, which is right.
21. **MINOR** - `screenshot_railgun.rs:821-853` - every stills capture encodes a
    webm nothing reads. The walk opens and closes `loop-section-railgun`
    unconditionally, so a stills run pays ~133 frames of 1280x720 readback plus
    a full ffmpeg encode and throws it away. UNMEASURED, and partly unavoidable:
    `LoopCapturePlugin` being armed is what pins `ManualDuration(1/30)` and makes
    the charge-fraction schedule deterministic. Worth a doc note so the next
    reader does not "optimise" it out and silently unpin the clock.

Lane A re-checked the three carried defects against its own files and cleared
two: the 110 m figure produces no ADDITIONAL mis-framing here (`GAP` at 150 m
gives a 221 m frame the 85 m hull fills 38%, `BACKDROP_EYE` 168.6 m gives 248 m
at 34% - both match the shipped pixels, only the stated fractions are wrong); no
constant world pose here sits against a re-oriented subject; and
`screenshot_damage_levels` **already handles the skin-hides-cracks problem
explicitly** with its bare last column (`:103-107`, `CLAD[level_index]` at
`:579`) - do not "fix" that one.

Lane A ran no rendered example and no probe at all: it spent the measurement
slot on image inspection per its brief, which is where its BLOCKER came from.
The one perf item is reasoned from code and labelled unmeasured.

Lane A did not check: `wiki-settings.png`, `wiki-controls.png`,
`wiki-first-scenario-picker.png`, the `damage-levels*.png` family,
`loop-section-hull.webm`, `tutorial-combat-lock.png`, `wiki-combat-aftermath.png`;
`screenshot_gravity`'s new pose is unverifiable because the producer ships no
image. It flags one item outside its bundle it did not chase: whether the
`wiki-scenarios-picker.png` figure note at `gen-web-screenshots.py:120-123` ("a
campaign's chapters indented under its header") is stale against the base-only
picker.

## Batch 6 findings: `--play` over the death feature

### Red team lane (code only)

1. **BLOCKER** - `pyre.rs:455-456`, registered unconditionally at
   `integrity/mod.rs:80` - `light_the_pyre` takes `ResMut<Assets<EffectAsset>>`
   and `ResMut<Assets<Image>>`, so **every death in a non-rendering app panics.
   MASTER IS RED RIGHT NOW.**
   `NovaGameplayPlugin` adds `HanabiPlugin` only under `if self.render`
   (`plugin.rs:90-92`) and `AppBuilder` sets `render: assembly !=
   Assembly::Headless`, but `NovaIntegrityPlugin` - and now `PyrePlugin` with it
   - is added unconditionally (`plugin.rs:113`). Observer parameters resolve
   BEFORE the body runs, so the lazy `PyreEffects` build and the
   `GraphicsBudget` early-return never get a chance.
   `SystemParamValidationError` for a missing resource is `skipped: false`, and
   `BevyError`'s default severity is `Severity::Panic`, so the process dies at
   the first kill.
   **ADJUDICATOR REPRODUCED IT**: `cargo test -p nova_ship --lib
   sections::integrity` -> **17 failed, 21 passed** (all 11 `ghost_ship_tests`
   plus 6 `physics_tests`). Also confirmed `pyre.rs` does not exist at
   `origin/master`, so THIS RANGE introduced the failure, and that the working
   tree carries no edits of mine. CI runs `cargo test --workspace --features
   debug` (`.github/workflows/ci.yaml:114`), so this fails the pipeline.
   Fix: the sibling in the same plugin already shows the shape -
   `spew_carved_material` (`integrity/spew.rs:496-497`) takes
   `Option<ResMut<Assets<Mesh>>>` with the docstring "this plugin ships inside
   `NovaIntegrityPlugin`, which a headless test app adds without any asset
   stores at all". Make `effects` and `images` `Option<ResMut<..>>` and return
   early when either is `None`.
2. **BLOCKER** - `fixture.rs:177` - `shed_dead_fixtures` uses the panicking
   `insert` on an entity the same frame can despawn; **the code it replaced used
   `try_despawn` for exactly this reason**. The removed `despawn_dead_fixtures`
   carried the comment "`try_despawn`: the section a fixture hangs on can die
   the same frame and take its children with it before this command lands." The
   replacement drops that guard: the greeble walk at `:199` uses `try_remove`,
   but the fixture's own chain at `:175-188` ends in `.insert((...))`.
   `EntityCommands::remove` is `queue_handled(_, warn)` and is safe;
   `EntityCommands::insert` is plain `queue`, which reaches the fallback handler
   with `Severity::Panic`. Four in-tree paths recursively despawn the ancestor
   in the same `Update`: `despawn_destroyed_that_does_not_detach`
   (`explode.rs:193`), `detach_destroyed_body`'s no-collider branch
   (`explode.rs:319-325`), `cleanup_empty_wreck_fragments`
   (`integrity.rs:526`), and `teardown_scenario_entities` on Retry/Next
   (`lifecycle.rs:105-107`). Because `shed_dead_fixtures` states NO ordering,
   which side its queue merges on is unspecified. Three separate in-repo
   comments name this exact race and use the `try_` form to survive it.
   Latent rather than observed - the lane could not force the despawn-first
   ordering from a test - but the panic path exists and the guard it removed was
   there on purpose.
3. **MINOR** - `fixture.rs:156` - a plain `Single<&mut WyRand, With<GlobalRng>>`
   makes the shed silently do nothing when the RNG is absent, and the shed is
   not cosmetic. `Single` fails validation with `skipped: true`, so the whole
   system is dropped with no error, and every spent plate keeps
   `HealthZeroMarker`, its `Collider` and its parent forever - precisely the
   state the module docstring says it exists to prevent. This already bit the
   change: `shell_skin.rs`'s own rig had to gain `EntropyPlugin` in this diff
   with the note "a rig without one leaves dead cladding bolted on". The sibling
   `fire_turret_bullets` uses `Option<Single<..>>` and documents the rule.
4. **MINOR** - `fixture.rs:98-100` - `ShedFixtureMarker(Entity)` records an id
   that is already dangling in the case its docstring names: when the shed
   happens because the section died, `detach_destroyed_body` despawns that
   section in the same or next flush and Bevy recycles indices. It is also
   `#[reflect(Component)]` with a bare `Entity` and no entity mapping. Nothing
   reads it today.
5. **MINOR** - `fixture.rs:194-203` - the shed MANUFACTURES the "fixture with no
   collider" state it cannot then handle. The descendant walk strips the
   `Collider` off every greeble riding a shed plate, and `q_dead` at `:150`
   requires `&Collider`, so any of those greebles that reaches zero health can
   never be shed and stays bolted to the drifting plate until the 12 s timer
   takes it. The known colliderless gap is therefore not only a mod edge - the
   system creates it on every plate it sheds.

**Attempts that HELD** (valuable negatives - the fix pass must not "fix" these):
`PyreEffects` survives a scenario reload (nothing clears `Assets<EffectAsset>`;
the second load reuses the same four graphs). Shed debris and pyre bursts do NOT
leak past teardown - `register_scenario_scoping` observes `Add<TempEntity>` and
scopes anything that gains one, catching both. `PYRE_FRAME_CAP` arithmetic is
sound: increment-only, reset unconditionally in `First`, no underflow, no
starvation. Light-flash flooding is capped by `light_the_flash`'s own counter.
No emitter is despawned before its particles at either size. A plate can never
throw a pyre (fixtures have no `ConnectedTo`, so they never reach
`IntegrityDestroyMarker`). A torpedo cannot draw `HULK_PYRE` (it carries
`TorpedoProjectileMarker`, not `SpaceshipRootMarker`, so it has no
`IntegrityRoot`). Nested and double sheds are safe. Degenerate fixture shapes
are clamped by `MIN_COLLIDER` and the `is_stud()` case. A fixture at the ship's
exact origin falls through to `Dir3::Y`, no NaN. Stale avian mass properties do
NOT poison the sever centre of mass - avian's remove hook drops `ColliderOf`.
The shed having no graphics-tier gate is DELIBERATE (`settings.rs:25-27`:
"debris are gameplay content, so no quality tier thins them"). Death on the
first frame is safe. No `ChildOf` cycle is reachable.
Suites green where they do not touch the headless panic: `sections::fixture`
7/7, `sections::shell_skin` 23/23, `sections::torpedo_section` 61/61,
`nova_gameplay integrity` 80/80, `nova_scenario objects::asteroid` 36/36.

Red team did not check: any rendered run (feel lane owns it); the full blast
radius of the headless panic beyond the 17 it reproduced - the real CI failure
count is at least 17 and probably larger; whether the headless panic reaches a
shipped player path (confirmed by construction, not by launching one); the exact
command-queue merge order that decides whether finding 2 fires; determinism of
the shared `GlobalRng` stream now an unordered system draws from it.

### Feel lane (held the measurement slot) - THE FPS ANSWER

**The cladding shed does NOT cost the frame. Measured twice, two ways, on a
quiet box against a named reference.**

**1. Direct attribution - the shed's own per-frame cost, isolated.** `wfc_arena`
is the one range where clad hulls die and it does wire a frame-time capture
(`wfc_arena.rs:437-448`). Its profiled pass traces every system, so
`shed_dead_fixtures` can be read on its own. Two traced runs at `b8d17f99`, real
display `:0`, dev, 1280x720, vulkan / RTX 3060 Ti:

| span | run A (503 frames) | run B (613 frames) |
|-|-|-|
| `shed_dead_fixtures` body | 8.3 us mean, 109.5 us worst | 7.6 us mean, 96.6 us worst |
| `shed_dead_fixtures` `(commands)` flush | 16.9 us mean, **722.6 us worst** | 10.1 us mean, 575.5 us worst |

The `(commands)` row IS the archetype churn the review was worried about. Its
worst single frame in the whole capture was **0.72 ms against a 20-25 ms
frame**; typical combined cost ~0.025 ms/frame, ~0.1%. It does not appear in
either run's top-15 systems. For scale, `light_the_pyre` fired 6 times at 16.9
us mean / 68.9 us worst. Traced numbers RANK rather than certify, but the ratio
to the frame is three orders of magnitude, so no inflation factor rescues the
concern.

**2. The repeat set.** `probe run wfc_arena --repeat 8 --display :0` at HEAD, 8
captures, 0 refused, 7 admitted, `refresh_cap: not_suspected`. Reference:
`tasks/20260819-173219/NOTES.md` Phase B2 §1 (`ddaf1997`, same window, display,
host, profile).

| statistic | ref `ddaf1997` | HEAD | delta | detection floor (n=8) |
|-|-:|-:|-:|-|
| mean | 20.13 ms | 22.76 ms | +13.1% | cv 5.9% |
| median | 17.94 ms | 19.90 ms | +10.9% | cv 7.8% |
| p99 | 47.69 ms | 58.73 ms | +23.2% | 20% |
| worst frame | 59.47 ms | 66.48 ms | +11.8% | **12%** |

The worst frame - the better statistic on this bounded window - moved +11.8%
against a 12% floor, i.e. **not detected**. **This is NOT a shed regression and
must not be reported as one**: eighteen days and the whole range separate the
trees, and the shed's own traced cost is ~0.1% of a frame.

**3. What the shed actually does to the world.** Census at HEAD, 4v4 armed:
29,241 entities, 14,298 mesh instances. **3,022 `SectionFixture` entities**;
cladding plus greebles are **54% of every drawn instance in the arena**. The
shed does not ADD any of that - it cuts the same entity loose, so its meshes
were already drawn. What changed is they are not freed on death; they persist
12 s. A DEFERRAL, not a new load. Also confirmed from `explode.rs:378-393`: when
a SECTION dies its fixtures are reparented onto the wreck piece and never enter
`shed_dead_fixtures` at all - the shed fires only for plates killed individually
while the section behind them stands. **The feared "a whole hull's hundreds of
plates in one frame" is not reachable through this path.**

6. **MAJOR** - `fixture.rs:147` - the shed still has no per-frame budget, and
   the cost is linear in a count nothing bounds. In the arena the binding is
   loose (0.72 ms worst flush) because it kills plates a few at a time, but
   `torpedo-blast.webm` shows the other regime: one detonation puts dozens of
   plates off a single hull inside half a second (frames 1.35-1.55 s). The
   ceiling is UNMEASURED because no range drives it; the absence of a bound is
   not. Fix: cap it in the same shape as `PYRE_FRAME_CAP`, but DEFER the
   overflow to the next frame rather than dropping it - a plate must still come
   off; the pyre may skip a fire, the shed may not.
7. **MAJOR** - `fixture.rs:48` - shed cladding leaves too slowly to clear the
   hull, so for about a second after a heavy hit the ship is inside a closed
   shell of its own plates and cannot be read. `SHED_KICK` is 15-40 m/s against
   `PIECE_KICK`'s 20-50 m/s for a whole section - 75-80% of a section's speed
   for an object a fraction of the size and hundreds of times as numerous.
   Looked at: `torpedo-blast.webm` at **t = 1.55 s** the hull is completely
   covered by a crust of ~60 detached-but-stationary plates, the silhouette is
   unreadable and plates interpenetrate each other and the hull. By t = 2.40 s
   they spread into a halo the ship sits inside; at 12 s they hang there another
   nine seconds. In combat that second is exactly when a player needs to read
   the target.
8. **MINOR** - `fixture.rs:1-14` - shed cladding reads as a crate spill, not as
   skin coming off a hull. The module says a plate is a sheet, but the shed
   pieces are near-CUBES with a hatch on one face, because the plate keeps its
   greebles as children (`fixture.rs:118-120`) and plate-plus-greebles is a
   block. Art note, not a code fix.
9. **MINOR** - `pyre.rs:178` - a section death often shows no core at all.
   `SECTION_PYRE.core` is a 3.4 m camera-facing flare spawned at the dying
   section's centre, which for any interior section is INSIDE the hull and
   occluded. `spine-cut.webm` 1.63-1.70 s: the hull goes warm cream with no
   visible flare, and fire only appears at 1.73 s as dashes escaping the
   silhouette. Worth a decision rather than a patch.

**REFUTED BY LOOKING** (batch 1 finding 5, `pyre.rs:186`): the prediction that
`SECTION_PYRE`'s ejecta would read as one smear because the streak is ~1.8x
longer than the burst's travel. The arithmetic is right; **the prediction is
wrong**. In `spine-cut.webm` 1.73-2.15 s and `torpedo-blast.webm` 1.25 s the
ejecta reads as clearly separate incandescent dashes with black between them.
The streaks contract before they overlap and the eye reads separation from the
gaps, not from clearing the core. **Do not "fix" this constant.**

**Two things the look got right**: the debris pivot fix (`48a3ba1d`/`a5da7efd`)
is visibly correct - every shed piece turns about itself, no swinging on a wire.
And the light does NOT wash the scene: a nearby asteroid's mean luminance rose
14.2 -> 15.6 (+10%) for three frames and returned; the far hull moved +0.25%. No
blowout in any of the four shipped loops.

Feel lane did not check: the shed's CEILING (no range drives a many-plates
frame); `PYRE_FRAME_CAP` under load (`light_the_pyre` fired 6 times in 613
frames, never approaching the cap); **a root/hull death - no `HULK_PYRE` was
confirmed in any loop, so "does a hull letting go read, and does its 1700 m
light wash the scene" is UNANSWERED**; a release-profile number;
`stress_hull_collapse` (the ~35% figure stands unchallenged, neither confirmed
nor refuted); the `CAMERA_BASE` 55 -> 20 pull, which is inside the HEAD arm and
outside the reference arm and so is one confound in the +11-13% central move.
Artifacts left: `probe-runs/b8d17f990/wfc_arena/` holds an 8-capture set and a
2.7 GB `trace.json` (gitignored). Nothing in the tree or index was touched.

## Adjudicated verdict and the fix pass

Six batches, twelve lanes, two at a time. Raw totals: **3 BLOCKER, 25 MAJOR,
65 MINOR**. Everything below was re-derived against HEAD before it was acted
on; a lane's claim on its own was never enough, and three lane claims did not
survive that (see "Refuted" below).

Fourteen commits, oldest first. Each closes the findings named beside it.

| Commit | Closes |
|-|-|
| `a391a848` Stop a death from panicking an app that does not draw | BLOCKER: `light_the_pyre` took `ResMut<Assets<..>>` unconditionally, so any death in a headless app was a `SystemParamValidationError`. `Option<ResMut<..>>` plus an early return. The real gap was wider than the lane's: `PyrePlugin::build` never registered `SoftDot`, and only the `#[cfg(test)]` fixture did - which is why 17 tests still failed after the first fix. |
| `1c63d3ac` Throw shed cladding clear of the hull, and bound the frame | MAJOR: shed plates were kicked at 1.5-4.0 and re-collided with the hull they left. `SHED_KICK` is 3.0-7.0, and `SHED_FRAME_CAP` bounds a stripped hull to 24 plates a frame with the rest carried over - proven by a test that strips a hull at once and counts both the frame and the total. |
| `3289fa0b` Stop asteroids throwing a hull's fireball | MAJOR: `IntegrityDestroyMarker` is a shared seam and an exhausted asteroid raises it. Rock does not burn: the material is inherited up the `ChildOf` chain, defaulting to metal. |
| `e366c32e` Build the pyre graphs before the frame that needs them | MAJOR: both effect graphs and the soft dot were minted inside the observer, on the frame of the first death. `warm_the_pyres` builds them at `Startup`. Recovers about half the collapse regression (median 102.7 -> 90.7 ms, fixed-step median 6.5 -> 5). |
| `c5bf89b2` Scope the staged strike's counters to the beat that reads them | MAJOR: `stage_the_strike` left `hits`/`shots` from the previous subject, so the arena's strike beat scored the last run. |
| `cb8d7e99` Keep the bench view on the player, and bound an aim | MAJOR x2: the observation fell back to the ENEMY's ship once the player's hull was gone, and an aim gesture accepted an unbounded tick count. |
| `ab60b299` Close three holes in what the channel tells a driver | MAJOR x3, all against SHIPPED v0.12.0 behavior, so all three carry changelog entries: a `stop` sent under a lowered context was swallowed with its press and left the key nailed down; `radar.dwell_fill` published 1.0 for "no dwell running", which the bench manual reads as a completed lock; `SNAPSHOT_SCHEMA` -> 2 for the two format breaks. |
| `77c76e5c` Re-pose the radar shot for the hull it now frames | MAJOR: the radar still's lens was posed for a hull the scene no longer stages. |
| `39f6af71` Correct the hull figure every framing was measured against | BLOCKER + MAJOR: "110 m stem to stern" is `block_gunship`'s bounding-sphere DIAMETER; the hull is 85 x 50 x 48 m. Nine docstrings in five producers and the arena were recomputed. Also `hide_status_bar` (the status bar carries the build commit, so every re-shoot baked a new hash into the two loops whose subject IS the HUD) and the alias-only capture `gen-web-screenshots.py` was silently discarding. |
| `b6f6f6c1` Shoot the damage loop at the flank the beats are on | MAJOR x2: a constant world pose photographed whichever side the subject's authored yaw presented - the drive bell, with every beat's damage behind it. Both framings now solve in the hull's frame through one `frame_from_hull`. And the volley has to take the PLATING off: `owning_section` ends its crack walk at a fixture by design, so a clad cell cracked alone cracks under its plating and photographs as an untouched ship. |
| `e532acd5` End the arrival on a hull, not on a drive bell | MAJOR: `ship_heading` accepted any velocity `try_normalize` would take, so an arrived ship's residual drift became the rig's whole bearing and the loop closed on two seconds of a nozzle aimed at the lens. Speed floor at 10 m/s, and the settle beat is a side-dominant chase rather than the montage's nose-on cut. |
| `47ca2e2f` Say what the railgun's slow clock actually buys | MINOR x3: three places promised the slug's flight spread over frames. A round advances in `FixedPostUpdate` with no transform interpolation and 320 m at 15 km/s is under two fixed steps, so a slower clock adds no poses between them. Verified in the shipped loop: the sight line stands to frame 16, and frame 17 has both the entry and a full-length wake. |
| `6609c3a6` Re-shoot the two loops the status bar was standing in | Carries `39f6af71`'s HUD fix into `landing-cockpit.webm` and `command-shell-open.webm`. |
| `2f21f01d` Document the pyre in the sections guide | MINOR: the integrity chapter stopped at `explode.rs`, so the half with the budget, the material gate and the root exemption had no entry. |

Four loops and two stills were re-shot so the code fixes reach the site:
`landing-damage-sequence.webm`, `goto-arrival.webm`, `landing-cockpit.webm`,
`command-shell-open.webm`, `wiki-ships-damage.png`, and the radar still.

### The `--play` answer: the cladding shed does not cost the frame

Measured two ways on a quiet box (loadavg 0.04-3.39; a first set taken at 3.4-4.4
was discarded). Worst observed command-flush attributable to the shed was
**0.72 ms against a 20-25 ms frame**, and about 0.1% typically. It is not the
frame's problem.

The collapse path was: `stress_hull_collapse` had regressed about 35%, and
`e366c32e` recovered about half of it by moving the graph build to `Startup`.
The residual sits inside run-to-run noise and is reported as such rather than
claimed.

### Refuted

- The pyre light "washes the scene": it does not. A nearby asteroid's mean
  luminance moved 14.2 -> 15.6 for three frames and returned; the far hull moved
  +0.25%. No blowout in any shipped loop.
- The damage loop "loses its cracks to the cladding" was reported as a game
  defect. It is not: `owning_section` ends the crack walk at a fixture
  deliberately, and a real round meets the plate's collider first. The producer
  was staging a hit the game cannot deliver.
- `Single<&mut WyRand>` in the shed path was called a panic risk. It matches
  `explode.rs`, which is the precedent for the same seam. Left alone.

### Not fixed, and why

- **`command-shell-open.webm` still carries a build hash.** The shell's own CRT
  header prints it, and that header is the loop's SUBJECT rather than chrome a
  producer may take down. Making the version string stable is a product
  decision, not a review fix.
- **`web/src/wiki/flight-autopilot.md:41`** calls 55.2 m "its own 55.2 m hull".
  The number is right - it is the arm to the outer face of the furthest section,
  which is what the standoff counts - but it reads as a length, and the hull is
  85 m long. Frontend, outside the owner's stated lane; flagged, not edited.
- **Five feel-lane questions stay open**: no root/hull death was staged in any
  loop, so `HULK_PYRE` and its 1700 m light are UNCONFIRMED; the shed has no
  measured ceiling (no range drives a many-plates frame); `PYRE_FRAME_CAP` was
  never approached under load (6 fires in 613 frames); no release-profile
  number was taken; and the `CAMERA_BASE` 55 -> 20 pull remains one confound in
  the +11-13% central move.
- The remaining MINOR findings were adjudicated as taste, as already-correct, or
  as churn against code that reads clearly. The commit trail above is the record
  of which ones were judged worth a change.

### Scope kept

Nothing outside the owner's lane was touched. `CHANGELOG.md` and the `web/`
lore tree carry uncommitted owner work throughout this run; every commit staged
explicit paths, and no index was left standing between tool calls.
