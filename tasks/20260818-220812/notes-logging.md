# Making the log readable by hand

DONE 2026-08-20. The use case is the owner reading a console by eye to check
what the game did. Everything below is judged against that, not against a
tidiness target.

## The number

A `--features debug` boot-to-menu, `examples/systems/system_menu_boot.rs` under
`NOVA_AUTOPILOT` on Xvfb `:99`, RUST_LOG unset. Same binary, same scenario
(`menu_gauntlet` then `shakedown_run`), exit 0 both times.

| level | before | after |
| --- | --: | --: |
| ERROR | 19 | 19 |
| WARN | 2 | 2 |
| INFO | 21 | 21 |
| DEBUG | 318 | 20 |
| TRACE | 0 | 0 |
| **total** | **360** | **62** |

**360 -> 62 lines, and DEBUG 318 -> 20.** Nothing was deleted: the same run
with `RUST_LOG=nova=trace` prints 784 lines, 722 of them TRACE.

The AFTER figure INCLUDES two crates that printed nothing at all before, so the
re-levelling paid for the filter fix and 82% of the reduction on top.

Where the 318 came from, which is the whole argument:

| lines | source |
| --: | --- |
| 142 | `SpawnScenarioObject: spawning '<id>'` |
| 76 | `asteroid_scenario_object` (config dump + mesh timing) |
| 38 | `<X>Plugin: build` |
| 16 | `on_add_explodable_entity` |
| 10 | `ScatterObjects` (1 header + 9 per-copy drops) |
| 10 | `# Current Variables:` + one line per variable |
| 26 | everything else |

218 of 318 were one object being spawned. **0 of 318 came from `nova_ship` or
`nova_hud`.**

## 1. The filter

`log_filter_str` named NINE crates. The workspace has TWENTY-TWO. The thirteen
it missed sat at the `LogPlugin` INFO default while their neighbours were at
DEBUG - so `nova_ship`, the busiest gameplay crate in the game, and `nova_hud`
contributed nothing to a debug boot. That is not a theory: `grep -c
"nova_ship\|nova_hud" before.log` is 0.

The failure is invisible from the console. A line that never prints looks
exactly like a line that never ran, which is why this drifted for so long
without anyone filing it.

Fixed by inverting the list into a PREFIX. `EnvFilter` matches a directive
against the target with `starts_with`, not equality
(`tracing-subscriber-0.3.23/src/filter/env/directive.rs:246`), and every
workspace crate is named `nova_*` - so a single `nova=debug` covers all
twenty-two.

**The rule for a new crate: there is nothing to do.** A crate added tomorrow is
covered the day it is added. Never list crates one at a time again.

Four tests in `crates/nova_core/src/lib.rs` hold the shape, including
`the_filter_never_names_a_single_nova_crate` - a `nova_<crate>=` directive
anywhere in the string now fails a test rather than silently muting its twelve
neighbours.

The third-party clamp is unchanged in effect (`wgpu=error`, `bevy_ecs=warn`,
`bevy_time=warn`, `naga=warn`, `bevy_render` at info/warn by feature). Release
still leaves the nova crates at INFO.

## 2. Headless boot noise

The three bevy diagnostics from `tasks/20260819-173219/notes-render-off.md` are
now clamped, in `HEADLESS_FILTER`, applied ONLY when `AppBuilder::headless()`
built the app (`log_plugin` takes `render`).

That gating is the point. All three fire only when there is no render sub-app,
so a rendering run keeps every one of those targets at its normal level and no
real error is hidden anywhere it could occur.

The two `bevy_render` clamps are MODULES, not the crate, and each was checked
against the pinned source before being silenced:

- `bevy_render::extract_resource` - `=off`. Its only two log statements are the
  missing-render-app ERROR and its WARN sibling (`extract_resource.rs:48,70`).
- `bevy_render::texture` - `=off`. Its only log statement is the
  `CompressedImageFormatSupport` warning (`texture/mod.rs:54`).
- `bevy_gizmos_render` - `=error`, not off, because that crate DOES carry real
  errors (`pipeline_3d.rs:211`). Its remaining warnings are line-style
  complaints raised while DRAWING, which a run with no renderer cannot reach.

## 3. What was re-levelled

130 `debug!` sites moved to `trace!`, in four groups plus thirteen judged
individually. Nothing was deleted.

Tree-wide that inverts the ratio the epic opened on - `debug!` 214 -> 84,
`trace!` 99 -> 229 - which is the shape a healthy tree has, because per-item
lines vastly outnumber per-operation ones.

| count | pattern | why |
| --: | --- | --- |
| 82 | `debug!("<X>Plugin: build")` across 7 crates | Per PLUGIN. Fires on every boot, scales with the plugin list, and tells a reader nothing the plugin list does not. 38% of every `debug!` in the tree was this one line. |
| 20 | `debug!("<x>: config {:?}", config)` | Per ITEM, and it dumps a whole struct. This is literally the owner's "debug logs of variables like the values" - see the `AsteroidConfig` dump in `before.log`. |
| 14 | `setup_/remove_/add_hud_*: entity {:?}` in `nova_hud` | Per WIDGET per ENTITY. |
| 1 site | `on_add_explodable_entity` (`nova_gameplay/integrity/explode.rs:193`) | Per entity - 16 lines on the measured boot, and a `trace!` five lines above already logs the same event. |

Individually judged:

- `nova_scenario/src/actions/spawn.rs` - `SpawnScenarioObject` and
  `DespawnScenarioObject` per-object lines. 142 of the 318.
- `nova_scenario/src/objects/asteroid.rs` - both `asteroid_scenario_object`
  lines, the config dump and the per-rock mesh timing. The owner named this one.
- `nova_scenario/src/objects/asteroid_carve.rs` - `carve_surface` per-node
  timing, `collect_asteroid_field_seeds`, the per-node `exhausted` line, and
  `sever_piece`'s dropped-piece line. Each already has a frame-level summary
  beside it that carries the totals.
- `nova_hud/src/{beacon_chips,allegiance_markers}.rs` - per-beacon and
  per-marker setup.
- `nova_hud/src/lib.rs` - `despawn_player_hud<M>`, per widget type.

## 4. Compaction

Three sites. Every summary carries a COUNT.

**Scatter fields** (`nova_scenario/src/actions/spawn.rs`). The header line
announced what was ASKED for, before the loop ran, and each dropped copy got its
own line. Now one line after the fact, reporting what actually landed:

```
ScatterObjects: scattered 26 of 26 'gauntlet_rock_' object(s), 0 dropped for separation (seed ...)
```

A field that comes out short now says so on one line instead of making the
reader count. Per-copy spawn and drop lines are a `RUST_LOG=nova=trace` away.

**Scenario variables** (`nova_scenario/src/world.rs`). Was a `# Current
Variables:` header plus one line per variable, every time any one of them
changed - N+1 lines that said "something changed" and left the reader to diff
them by eye. Now one line naming what actually moved:

```
scenario variables: 9 changed of 9 live (beat=Number(1.0), beat_gate=Number(0.0), crates_left=...)
```

Sorted, because `HashMap` order is arbitrary and an unsorted line would reorder
between two identical runs and defeat a log diff. Removals are counted too
(`<key>=<removed>`): teardown CLEARS the table, and a diff that only looked at
what is present would have reported "0 changed" on the one transition that
dropped everything.

**Whole-scenario spawns.** No new machinery - `on_load_scenario` ALREADY emitted
`loaded scenario '<id>' with N handler(s) and M object(s)`. It was invisible
under 142 per-object lines. Demoting those made the existing summary the thing
you see.

The AFTER log now reads as a narrative: scenario loaded with its counts, six
scatter fields with their counts, the variable delta, and the handful of state
transitions that followed.

## 5. Deliberately left at debug

- **Scenario actions** (`SetCamera`, `SetSkybox`, `SetAllegiance`,
  `ForceTorpedoLaunch`, `Outcome`, `NextScenario`, `ObjectiveMarker*`, ~27
  lines). One line per authored action firing is exactly per-operation, and it
  is the trace of a scenario's behaviour the owner reads these logs FOR.
- **Autopilot state changes** (`nova_ship/src/flight/autopilot.rs`, 9) and
  **flight input** (`flight_rig.rs`, 15). Per player action or per disengage
  decision. They do not fire on boot at all - zero of them appear in either
  measurement.
- **Integrity events** (`nova_gameplay/integrity/core.rs`, section destruction,
  structural collapse). One line per thing dying.
- `update_point_defense_ownership` and `update_turret_point_defense`. Both look
  per-item - they sit in a per-mount loop - but both are guarded on a real
  change, so they log DECISIONS, not frames. `assignment.rs:299` already carries
  a comment saying so.
- `AutopilotPlugin: build` and `ScreenshotPlugin: build`, the two `Plugin: build`
  lines NOT demoted. Unlike the other 82 they are env-gated: they fire only when
  `NOVA_AUTOPILOT` / `NOVA_SCREENSHOT` is active, once per run, and they report
  the step count and target that run is about to use.
- `on_load_scenario` keeps BOTH its lines - one before setup, one after. They
  bracket the load, so a scenario that dies midway shows the opening line with no
  closing one.

## 6. Defects found, reported not fixed

**A clean successful boot prints 19 ERRORs, and all 19 are one handled
condition.**

```
nova_scenario::filters: VariableFilterConfig: failed to evaluate condition: UndefinedVariable("beat")        x12
nova_scenario::filters: VariableFilterConfig: failed to evaluate condition: UndefinedVariable("open_step")   x6
nova_scenario::filters: VariableFilterConfig: failed to evaluate condition: UndefinedVariable("scav_posted") x1
```

`crates/nova_scenario/src/filters.rs:218`. The doc comment three lines above it
says the filter "fails closed (false) on an evaluation error" - so this is a
DESIGNED outcome being reported at the level reserved for something being wrong.
A scenario filter gating on a variable a later beat sets will read it as
undefined until then, every time it is evaluated.

This is now the single loudest thing in the log and the only ERROR a healthy run
produces, which is exactly the state that teaches a reader to ignore ERROR. Left
alone because it needs a call this lane should not make: either `UndefinedVariable`
is expected and belongs at debug while genuine type errors stay at error, or the
shipped scenarios are authoring filters against unset variables and the fix is in
the content.

**Warnings for a condition the code documents as supported.** Three sites warn
when a resource is absent, each directly under a comment explaining that the
absence is deliberate and must not panic:

- `nova_scenario/src/actions/mission.rs:342` - `HintEmphasisSet: no HintEmphasis
  resource (HUD not loaded)`
- the `HintEmphasisClear` sibling
- `nova_scenario/src/actions/flow.rs:78` - `Outcome: no CurrentOutcome resource
  (scenario loader not loaded)`

Every headless rig that runs a scenario script warns on each of these. Same class
as the above: an expected-and-handled condition is not a warning.

Checked and found CORRECT, for the record: `pose_camera: no scenario camera
present yet` (`nova_debug/src/harness.rs:465`) reads like the same class, but its
doc says the script is supposed to be gated on `scenario_camera_present`, so
reaching it means the author forgot the gate. That is a real authoring problem
and belongs at warn.

## What this cost, and the box

One `cargo check`, three example builds and four runs. The sibling
`torpedo-materials` lane was building `--examples` when this started, so all
reading and editing was done first and the check was run at load 2.15 with zero
rustc processes. The two later builds went in at load ~17 while that lane was in
`cargo build` (not measuring) - permitted, since only MEASUREMENT has to be
serialised, and a log LINE COUNT is deterministic content that does not vary
with load the way a frame time does.

`cargo test --lib -p nova_core --features debug`: 9 passed.

## Next time

The filter drifted because adding a crate and updating a filter string are two
separate acts and only one of them is forced. Any allowlist keyed on crate names
has that shape. The prefix directive removes the second act entirely, which is
the only version of this fix that stays fixed - a lint or a checklist item would
just be a third thing to forget.

The re-levelling has no such guard, and it will drift: nothing stops the next
`debug!` in a spawn path. The cheapest available brake is the boot-line count
itself - 62 is now a number a probe could assert on, and a range that fails when
a debug boot crosses (say) 100 lines would catch the next 142-line regression the
week it lands rather than a year later.
