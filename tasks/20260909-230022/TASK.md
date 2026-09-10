# Nightly review

- STATUS: OPEN
- PRIORITY: 70
- TAGS: review


## Scope

Nightly review of 2026-09-09. Window is the last briefing at
2026-09-08T23:00:15+03:00 to HEAD (88a7445b4). 23 commits.

This task reviews only. No edit, no commit, no push, no release.

## Commits in window

```
88a7445b4 Illustrate the distress call and rescue decision
3d1117503 Update tasks
d101f1d43 Publish the comic independently of the site
e8228c7da Release v0.13.0
5348c632f Move the season-one story to the v0.14.0 board
9e69ac196 Compile the news loop producers without the debug feature
99c5dfd8c Add draft comic authoring and season-first browsing
22e681962 Close the v0.13.0 news post task
a5f7df1d6 Hold the GOTO margin on a light hull
7fdd25222 Draft the v0.13.0 news post with its media and widgets
f4ea1bce4 Read last night's review in the morning briefing
62043f3b5 Let the scenario sweep tolerate a root that already died
4be046035 Close the nightly review task
7482370f3 Record the nightly review of 2026-09-08
1956c26f2 Say what a dying section looks like in the ship wiki
78baad3eb Correct the Unreleased block against what v0.12.0 shipped
cc005e202 Re-shoot the nine stills that shipped the debug status bar
ba0cc418f Tell a driver the truth about the radar dwell and the aim cap
45372be15 Keep the status bar out of every capture that keeps the HUD
c5b8878aa Warm the pyre for the tier that runs, and let a rock throw rock
4797eab8b Add reusable facial expressions to the opening comic
faffdc56b Make the nightly review and report, and say it has one turn
b716d27b0 Place Baikal's card on the opening shot and plan season writing
```

## Groups

Ordered highest risk first. Line counts are text lines only; binary media is
excluded. `--play` is granted where the group changes rendering or feel.

| # | Group | Range | Lines | Play |
|-|-|-|-|-|
| G1 | Pyre tiering, rock spew, section fixtures | `c5b8878aa^..c5b8878aa` | 569+/155- | yes |
| G2 | Autopilot GOTO margin on a light hull | `a5f7df1d6^..a5f7df1d6` | 546+/104- | yes |
| G3 | Capture status bar, radar dwell, aim cap, probe snapshot | `45372be15^..ba0cc418f` | 298+/101- | no |
| G4 | Scenario sweep tolerance, debug-feature gating | `62043f3b5` + `9e69ac196` | 27+/3- | no |
| G5 | News-post loop producers (Rust examples, scripts) | 7fdd25222 rust/scripts subset | ~2350+ | no |
| G6 | News-post web widgets and page | 7fdd25222 web subset | ~4250+ | no |
| G7 | Release v0.13.0 and changelog corrections | 78baad3eb, 1956c26f2, 5348c632f, e8228c7da | ~150 | no |
| G8 | Comic build and independent deployment | d101f1d43 code subset | ~1030+ | no |
| G9 | Comic script DSL, lettering, panels, reader | 99c5dfd8c web subset | ~2500+ | no |
| G10 | Illustration library and episode art | 99c5dfd8c + 4797eab8b + b716d27b0 python subset | ~1100+ | no |
| G11 | Distress-call pages | 88a7445b4 code subset | ~780+ | no |

G5, G6, G9 exceed or approach the 2000-line refusal, so each is reviewed as a
path-scoped subset of its commit rather than the whole commit.

## Findings

Appended as each group is adjudicated.

### Baseline: master is green on check

`nix develop --command cargo check --workspace --all-targets --keep-going`
finished in 32.59 s with EXIT=0. One warning, and it is upstream
(`proc-macro-error2 v2.0.1` future-incompat). No lane ran the test suite or
Clippy; CI owns both.

### G1 - Pyre tiering, rock spew, section fixtures (`c5b8878aa`)

Lanes: correctness, contracts, craft, performance, feel. No lane held the
measurement slot: the workspace check owned the box, so `--play` ran statically
and nothing was rendered, timed or benchmarked. That is a skip, not a pass. The
commit's own 1.68 ms WGSL figure is unreproduced.

**MAJOR - `crates/nova_ship/src/sections/fixture.rs:82`, registered at
`crates/nova_ship/src/sections/shell_skin.rs:1148` - the shed cap moved from
per-frame to per-tick, so it no longer bounds a frame.**
Four lanes found this independently. Re-derived here: `shed_dead_fixtures` is
`app.add_systems(FixedUpdate, ...)` and spends `.take(SHED_TICK_CAP)` once per
invocation, so the 24 is per fixed step. `Time<Fixed>` is Bevy's default 64 Hz
and nothing in shipping code calls `set_max_delta` (the only callers in the
tree are `crates/nova_probe/src/capabilities/frametime.rs` and tests in
`crates/nova_ship/src/sections/ammo.rs`), so `Time<Virtual>::max_delta` is
Bevy's 0.25 s and one frame can bank 16 steps. A collapse frame therefore
performs up to `16 x 24 = 384` `try_remove` + `try_insert` archetype moves
where the old `Update` placement bounded it at 24, and the multiplier is
largest on the longest frame - the collapse frame the constant exists to
protect. The repo already records frames at the clamp:
`tasks/20260819-173219/measurements/fixed-steps.txt` has `wfc_arena#8
16:36@321.6ms`. The sibling module states the opposite rule for the same
reason and is unchanged: `crates/nova_gameplay/src/integrity/spew.rs:303`, "A
FRAME and not a fixed step: several fixed steps can flush into one frame".
Fix: keep the per-tick drain and add the frame ceiling back on top, with the
`First`-reset budget resource `PyreBudget` and `ShardBudget` already use.
The unit test cannot see it: `shed_app` pins `ManualDuration(timestep)`, which
is exactly one tick per `app.update()`.

**MAJOR - `crates/nova_gameplay/src/integrity/pyre.rs:705` - the warm-up moved
from `Startup` to `OnEnter(GameStates::Playing)`, and two live paths never
reach it.** Both re-derived here.
1. The main menu. `git show c5b8878aa^` has `app.add_systems(Startup,
   warm_the_pyres)`. The backdrop loads on `OnEnter(GameStates::MainMenu)`
   (`crates/nova_menu/src/lib.rs:161`), `PyrePlugin` is added unconditionally
   (`crates/nova_gameplay/src/integrity/mod.rs:80`), and the backdrop scenario
   is `assets/base/scenarios/menu_duel.content.ron` - "A gunship and a raider
   duel; a siege torpedo erases the winner; repeat." So the menu shows deaths
   in a state that is now never warmed, where before the commit `Startup` had
   already built all four graphs and the mask. That is a regression on the
   menu path, not merely an unextended improvement.
2. A tier raised mid-run. The pause overlay builds the same settings body
   (`crates/nova_menu/src/pause.rs:398` calls `build_settings_tabs`) and
   `button_on_setting::<GraphicsQuality>` is a global observer
   (`crates/nova_menu/src/lib.rs:140`), so a player who starts on Low and
   picks High never re-enters `Playing`. `drawable()` then passes, and the
   next death pays the whole cold path on the most-watched frame in the game.
`docs/sections.md:515` now asserts "Nothing about those graphs is built at the
death", which is false on both paths.
Fix: re-warm on `resource_changed::<GraphicsBudget>`, which is what the new
`SettingsSystems` set was added for, or move the warm-up to
`PostStartup.after(SettingsSystems)` so every app path warms once against a
settled budget. `warm_railgun_wake_art`
(`crates/nova_ship/src/sections/railgun_section/mod.rs:415`) has gap 2 as well,
but it predates this range.

**MAJOR (plausible, not re-derived) - `pyre.rs:555` - `Visibility::Hidden`
warms the WGSL and the compute pipelines but never the RENDER pipeline.**
The performance lane traced hanabi 0.19: `compile_effects` and
`prepare_init_update_pipelines` have no visibility gate, but
`specialized_render_pipelines.specialize` is reached only from
`emit_sorted_draw`, which iterates a view's visible entities
(`bevy_hanabi-0.19.0/src/render/mod.rs:5280`), and a hidden entity is never in
one. `crates/nova_core/src/lib.rs:642` sets
`synchronous_pipeline_compilation: true` for every backend, so that creation
blocks the render thread on native too - on the first frame a hull lets go.
`pyre.rs:566` claims the instances survive their frame "to be both compiled and
specialized". I did not follow the hanabi queue path myself, so treat the
mechanism as plausible and the doc claim as the safe half of the fix.

**MINOR - `fixture.rs:182-189` - right clock, wrong phase.** The docstring
justifies `FixedUpdate` with "Health empties on the fixed step". It does, but
in `FixedPostUpdate`: the `on_damage` observer raises `HealthZeroMarker` from
`crates/nova_gameplay/src/rounds.rs:242` and `damage.rs:797`, both
`FixedPostUpdate.after(PhysicsSystems::Last)`. `FixedUpdate` runs first, so a
plate killed in tick N is first seen in tick N+1 and stays visibly bolted on
one extra frame - under the old `Update` placement it came off in the same
frame the round landed. Fix: register in `FixedPostUpdate` after
`PhysicsSystems::Last`, or drop the "that is the clock the deaths arrive on"
claim.

**MINOR - `docs/sections.md:329` - the creator-facing drain rate is off by 24x
as written.** "`SHED_TICK_CAP` bounds it to 24 fixtures a tick" then "On the
fixed step the drain rate is 64 a second". The code docstring says "64 DRAINS a
second"; the doc dropped the noun. A creator reads 64 fixtures a second and
budgets four seconds to strip a 263-plate hull that takes about a sixth of one.

**MINOR - `fixture.rs:70-73` - "64 drains a second whatever the renderer is
managing ... on a software rasteriser drawing a second a frame just as much as
at 120 Hz" is false above the clamp.** At one frame a second Bevy advances
`Time<Virtual>` by `max_delta`, so 16 ticks run, not 64: 384 fixtures a second,
not 1 536, and the rest of the second is time the world never simulates.
`crates/nova_probe/src/stats.rs:353` already states this rule.

**MINOR - `pyre.rs:240` - the new "wider than the wreck" rationale does not hold
for the two largest shipped hulls.** The 85 m correction is right
(`block_gunship` spans 8.5 grid cells on Z). But `HULK_PYRE` is one fixed size
for every integrity root, and `assets/base/ships/base.content.ron` has
`block_warship` at 21.5 cells (215 m) and `block_carrier` at 36 cells (360 m)
against an ejecta reach of about 130 m. By the docstring's own criterion a
carrier death reads as the ship having merely broken. The v0.14.0 board runs
ranges on the carrier. The constants are pre-existing; what is new is a
docstring asserting a property the code does not deliver.

**MINOR - `crates/nova_gameplay/src/settings.rs:331` - `SettingsSystems` is
prelude-exported public API with no production consumer.** Two lanes. Its only
reference outside its own two `in_set` calls is the test at `settings.rs:504`;
the reader it was added for, `warm_the_pyres`, orders against a state
transition instead. It also spans `PostStartup` and `Update`, so a mod that
writes `.after(SettingsSystems)` in `FixedUpdate` silently gets no constraint.
Wiring the warm-up to it closes this and MAJOR-2 together.

**MINOR - `fixture.rs:270` - the `shed_app` docstring contradicts the one the
same commit left standing.** It says "One frame's manual delta is one timestep";
`shell_skin.rs:1208` says "The first tick of a manual clock is dt 0", and
`bevy_time-0.19.1/src/real.rs:99-105` agrees - `update_with_instant` returns
early when `last_update` is `None`. The rig is correct only because of its
trailing warm-up `app.update()`; the stated invariant is not.

**MINOR - `fixture.rs:637-640` - the assertion commented as pinning the drain
RATE does not pin it.** The comment says "Exactly the ticks the cap needs and
not one more"; the assertion is `shed_so_far(&app) < plates.len()`, which with
51 plates accepts anything from 0 to 50 where the expected value is 48. Fix:
`assert_eq!(shed_so_far(&app), SHED_TICK_CAP * (ticks - 1))`.

**MINOR - `pyre.rs:276` - one fact, two sources.** `PyreEffects::pair` still
returns `(PyrePair, PyreScale)` although `PyreSize::scale()` now answers the
scale, so `warm_the_pyres` writes `let (pair, _) = ...` to discard it. Add a
third size and updating only one of the two is a silent divergence.

**MINOR - `crates/nova_ship/src/sections/railgun_section/wake.rs:214-216` -
"synchronous on web and macOS" understates the platform this project measures
on.** `crates/nova_core/src/lib.rs:642` sets
`synchronous_pipeline_compilation: true` for every backend on purpose, so those
compiles block the render thread on native Linux too.

Dropped in adjudication: the "1.68 ms" and "five section shaders" figures are
unreproduced but not findings; the `inherited_material` / `inherited_drift` /
`inherited_motion` triplication is real but predates the range and only one of
the three moved here.

### G2 - Autopilot GOTO margin on a light hull (`a5f7df1d6`)

Lanes: correctness, contracts, craft, feel. No measurement slot; nothing was
rendered or timed. This commit landed at 15:35 and v0.13.0 was cut at 17:43 the
same day, so everything below SHIPPED.

**BLOCKER - `crates/nova_ship/src/flight/autopilot.rs:862` - a GOTO engaged
while closing at 2-5 m/s turns the ship tail-first and then never burns.**
Three lanes found this independently. Fully re-derived here.

`due` is `flip_point.is_none() && brake_accel > 0.0 && closing_speed >
stop_speed_epsilon`. `goto_flip_point` (`guidance.rs:101`) returns `None` for
three different reasons, and `due` distinguishes only one of them:
- `closing_speed < 0.5` u/s - too slow for the coast estimate to mean anything.
- `effective <= 0.0` - no stopping plan. Correctly filtered: the published
  `brake_accel` is already `accel * decel_margin - gravity`
  (`autopilot.rs:426`), the same quantity `goto_flip_point` calls `effective`.
- `coast <= 0.0` - braking really has begun. This is the one `due` means.

`stop_speed_epsilon` is 0.2 u/s (`state.rs:418`) and the estimate floor is 0.5
u/s, so for any closing speed in (0.2, 0.5) u/s = (2, 5) m/s outside the
standoff, `due` is true and the autopilot commits to a brake it should not.
The chain from there:
- `brake = Some((-velocity.normalize(), ...))`, so `aim_dir` at
  `autopilot.rs:1003` is retrograde and the hull slews at full `flip_pending`
  urgency.
- `firing_authority` (`autopilot.rs:782-800`) sums only engines within
  `align_cos` (0.95, about 18 degrees; 23 with hysteresis) of `error_dir`,
  which is still PROGRADE - the arrival curve wants `min_approach_speed`,
  15 m/s, toward the goal.
- Once the drive leaves that cone, `firing_authority` is 0, so `demand` at
  `autopilot.rs:1112` is 0 and the drive goes cold.
- With no thrust the closing speed never leaves (2, 5) m/s, so `flip_point`
  stays `None` and `due` stays true. The state is self-sustaining.
- `fine` is forced false (line 885), so the `(fine && firing_authority <= 0)`
  release path cannot end the leg either. A ship order has no timeout.

Result: nose-aft, engines cold, phase reading ALIGN, coasting at 3 m/s. From
4 km that is about 22 minutes and the leg never self-cancels. This is new -
before the commit `aim_dir` was always `error_dir`, so the same state pointed
the drive prograde and accelerated out of the band on the first tick. Every
GOTO from rest also crosses the band on spin-up; there it is a race between
3 m/s of acceleration and the hull's swing out of an 18-degree cone, which a
low-thrust or damaged hull loses. It reaches the AI too: a `GotoPos` patrol
leg re-engaged on a drifting hull can fly its whole leg backwards.

Fix: make the commitment mean "past the flip point", not "no flip estimate
published". Either give `ManeuverTelemetry` an explicit `braking: bool` set
only where `goto_flip_point` returned `None` for `coast <= 0.0`, or name the
0.5 floor as a shared constant and require `closing_speed >= that floor` in
`due`. Nothing at the autopilot level tests this gate.

**MAJOR - `crates/nova_ship/src/flight/autopilot.rs:885` - `fine` now requires
`brake.is_none()`, so a rest leg hunts a residual `settle_deadband` documents
as accepted.** A STOP always publishes `flip_point: None` with a positive
`brake_accel` (`autopilot.rs:542-552`) while `speed > stop_speed_epsilon`, so
`due` is true for the whole of a STOP and `fine` is false throughout.
`settle_deadband`'s own docstring states the contract as "released at up to
this band rather than hunted with attitude flips - that bounded creep is the
contract, and the price of not wobbling". At 0.5 u/s (5 m/s) the old code took
`fine` and accepted the crumb; the new code slews the hull to retrograde at
full urgency to chase it, and `done` can now only fire at `error_speed <= 0.2`
u/s. Worst on the tutorial range, where the RCS verb is withheld
(`tutorial/range.rs:105`) so the settle falls back to the main drive. The
existing guard `stop_accepts_a_crumb_without_pirouetting`
(`flight/tests/stop.rs:334`) uses 0.3 u/s, which is under the 0.4 u/s publish
hysteresis, so no telemetry is published and the suite stays green.
Fix: require the brake speed to be above `crumb_band` before it overrides
`fine`, and raise that test to 0.5 u/s so it sits above the hysteresis.

**MAJOR - `crates/nova_ship/src/flight/autopilot.rs:591` - the GOTO goal moved
to the target's centre of mass, but two of the three radius sources it is
subtracted from are still origin-measured.** `HullRadius` is COM-relative by
definition ("the distance from its live centre of mass to the outer FACE",
`hull_radius.rs:19`), so a ship target is right. `BodyRadius` is not - it is
the mesh's outermost vertex from the object ORIGIN
(`crates/nova_scenario/src/objects/asteroid.rs:246`) - and `well_position.0`,
`orbit_band_floor` and the ORBIT park handoff (`autopilot.rs:945`) are all
origin-based. A field rock is a dynamic convex hull of a noise-displaced,
per-seed-stretched mesh, so its `ComputedCenterOfMass` is not its origin, and
carving moves it further. The leg then parks one COM offset off the authored
margin on one side and short on the other; `ManeuverTelemetry::distance`
carries the same error onto the destination chip
(`crates/nova_hud/src/maneuver_instruments.rs:227`); and because the rock
tumbles, `rotation.mul_vec3(com)` sweeps the goal, the readout anchor and the
park point around its origin. Fix: shift to the COM only where the published
radius is COM-relative, and keep a `BodyRadius` or well target on its origin.

**MAJOR - `crates/nova_ship/src/input/ai/passive.rs:249` - the patrol advance
gate measures the ship's ORIGIN while the leg now rests its CENTRE OF MASS.**
Re-derived: `arrival_desired` uses `to_target = goal - com_world`
(`autopilot.rs:388`) and `rest_radius` is `arrival_standoff + HullRadius`
(COM-to-face), but the gate compares `transform.translation` against
`rest_radius + waypoint_slack` (`passive.rs:249`). On a hull whose COM is
offset from its origin - the commit's own comment cites 49 m aft on the block
warship - the origin rests that much further out than the COM. The default
`AI_WAYPOINT_SLACK` is 25 u = 250 m and absorbs it, but
`crates/nova_authoring/src/base_content/scenarios/main_menu/weave.rs:85`
authors `waypoint_slack: Some(Meters(50.0))`, leaving about a meter of
headroom on that hull: below the offset the route never advances, `on_station`
never latches, and the ship re-runs the same GOTO forever - the churn the gate
exists to prevent. Also stale: `web/src/create/objects.md:309` tells creators
"Below ~20 m risks stalling outside the advance gate", and the real floor is
now the hull's COM offset plus a margin. Fix: measure the gate from
`translation + rotation * ComputedCenterOfMass`.

**MAJOR (not re-derived) - `crates/nova_ship/src/flight/autopilot.rs:999` - the
comment says plan and execution choose the brake group from the same
direction; for GOTO they do not.** The plan calls `braking_plan(-closing_dir,
...)` (`autopilot.rs:415`) and that group's authority sets `brake_accel` and
the whole arrival curve; execution uses `-velocity.normalize_or_zero()`
(`autopilot.rs:865`), lateral included. The design comment 150 lines above says
so on purpose ("Not the closing line either", line 853), so the two comments
contradict each other and, on a hull whose retro cluster is off the nose axis,
`choose_group` can genuinely return a different group for each.

**MINOR - `crates/nova_ship/src/flight/tests/goto.rs:53` and `:95` - the claim
the commit is named for is not pinned.** The arrival assertion is
`distance <= standoff + 6.0 && distance >= standoff - 45.0`, which accepts the
ship finishing 45 u = 450 m INSIDE a 500 m standoff. The new unit tests cover
the three pure helpers only. The only artefact that watches the park point is
an `info!` in a capture example CI does not run, gated on `both_parked()`
within 120 s - so the gunship parking 47 m inside its standoff, the exact state
this commit fixed, would still pass and ship. Fix: tighten to a few world units
either side, and add a light-hull case.

**MINOR - `web/src/widgets.ts:792` and `:4431` - the news-post GOTO widget now
draws a flip line the game no longer flies.** Both sites hold `const lead =
Math.PI / turnRate + ARRIVAL_SPOOL_PAD`, commented `// autopilot.rs:209`, which
was exact until this commit added the settle term `tracking_lag *
ln(lag_angle / cone)`. With the shipped gains that is 0.57 s at 2 rad/s -
longer than the whole spool pad. The widget is the flagship explanation of the
verb on `/wiki/flight-autopilot/` and in the v0.13.0 news post.

**MINOR - `crates/nova_ship/src/flight/state.rs:356,362` - `attitude_deadband`
and `settle_deadband` silently gained a second role.** `crumb_band` is now also
the RCS settle's proportional scale (`autopilot.rs:748`), so it sets where the
hull parks; both docstrings still describe only the attitude hunt, and
`settle_deadband`'s states a one-sided constraint. Halving it through the
reflected inspector, exactly as the doc invites, also halves the settle time
constant.

**MINOR - `examples/screenshots/loop_goto_standoff.rs:387` - an acceleration
printed through the length type.** `Meters::from_engine(telemetry.brake_accel)`
under a `"brake {:.1} m/s2"` label. `MetersPerSecondSquared::from_engine`
exists and is used for this dimension elsewhere. The number is right only
because both quantity types share the factor of ten.

**MINOR - `CHANGELOG.md` - two v0.13.0 entries describe one change, and the
first was false as written until the second landed.** "GOTO parks a margin off
the target's SURFACE ... a warship and a shuttle sent to one mark now stop the
same distance clear of it" and "GOTO holds its margin: the leg flies from the
hull's centre of mass ..." are both in the shipped 0.13.0 block; the second is
a same-cycle revision of the first, which AGENTS.md says to collapse. Length
is fine - no entry in the released 0.13.0 block exceeds 200 characters (checked
all 165).

**MINOR - `crates/nova_ship/src/flight/autopilot.rs:873` - `flip_pending`
re-runs `choose_group` with arguments identical to the aim block's**, twice per
ship per tick, and the two can drift apart with no compile error.

**MINOR - `crates/nova_ship/src/flight/guidance.rs:356,368,393** - `slew_urgency`,
`flip_lead` and `spool_tail` are `pub(crate)` where every sibling helper with
the same reach is `pub(super)`. **`guidance.rs:386`** - `spool_tail` is the
closed-form integral of `thrusters::spool` but lives in `guidance` with only a
prose reference, so a change to `spool` cannot break it at compile time.
**`autopilot.rs:44`** - `RCS_RELEASE_DEFLECTION` is named 0.05 and its docstring
calls it the same threshold as the drive's, which stays a bare `0.05` literal
at line 917.

**MINOR - `docs/environment-variables.md:178`** does not list the new
`NOVA_STANDOFF_TRACE` on a page that opens "Every `NOVA_*` variable the game
reads, in one place". `tests/env_contract.rs` does not walk `examples/`, so
nothing fails.

Corrected during adjudication: `flight/guidance.rs` is not a new module in this
range - `54ebcc2a6` created it and this commit appends. The 251-character
changelog entry two lanes flagged was real at `a5f7df1d6` but the release
commit trimmed it; only the collapse-rule breach shipped.

## Interrupted and recovered

This review was killed mid-flight at 2026-09-09T23:51:03+03:00 by a
systemd control-group OOM stop that originated in the sibling project's
briefing run, not in this one. Cause, transcript references and the exact
lane roster are in `RECOVERY.md`; the raw lane reports are under `lanes/`.

Seven lanes had already finished when the process died and were never
adjudicated. They are adjudicated below, on 2026-09-10, against the same tree
they reviewed: `88a7445b4` is still `master`, so nothing needed re-basing and
nothing below is stale for that reason.

The review stays OPEN. Its scope is G1-G11; six groups never ran.

### Group status

| # | Group | Lanes | Status |
|-|-|-|-|
| G1 | Pyre tiering, rock spew, section fixtures | 5/5 | COMPLETE (no measurement slot) |
| G2 | Autopilot GOTO margin on a light hull | 4/4 | COMPLETE (no measurement slot) |
| G3+G4 | Capture status bar, radar dwell, aim cap, probe snapshot, sweep tolerance, debug gating | 3 (correctness, contracts, craft) | RECOVERED - PARTIAL, no feel or performance lane |
| G5a | News-post loop producers: the three largest | 2 (correctness, craft) | RECOVERED - PARTIAL, no contracts lane |
| G5b | News-post Rust and scripts outside those three | 2 (correctness, contracts) | RECOVERED - PARTIAL, no craft lane |
| G6 | News-post web widgets and page | 0 | NOT RUN |
| G7 | Release v0.13.0 and changelog corrections | 0 | NOT RUN (partly pre-empted by G2's changelog pass) |
| G8 | Comic build and independent deployment | 0 | NOT RUN |
| G9 | Comic script DSL, lettering, panels, reader | 0 | NOT RUN |
| G10 | Illustration library and episode art | 0 | NOT RUN |
| G11 | Distress-call pages | 0 | NOT RUN |

G3 and G4 were dispatched as one group; G5 was dispatched as two path-scoped
halves. The Groups table above this section is the plan, not the dispatch.

NO LANE IN THE WHOLE REVIEW HELD THE MEASUREMENT SLOT. Every claim about frame
cost, loop timing, encode budget or what a walk looks like is unverified across
G1-G5b, not only in G1 and G2.

### G3+G4 - recovered (`45372be15`, `ba0cc418f`, `62043f3b5`, `9e69ac196`)

Lanes: correctness, contracts, craft.

**MAJOR - `crates/nova_channel/src/apply.rs:262` - a `section` STOP is refused
when the mount is gone, so the source the press pushed is never lifted.**
Re-derived here. `apply_section` resolves `section_source` fresh per line and
returns `refuse` for BOTH phases when it resolves to nothing. On a release that
leaves the synthesized press down with nothing able to lift it:
`dispatch::press_source` states in its own docstring that "nothing here touches
`DrivenPresses`" (`crates/nova_input/src/dispatch.rs:128`), which is the record
the sibling lane's release path uses to recover. So a driver who sends
`section.<id> start`, loses the mount to
`integrity::explode::despawn_destroyed_that_does_not_detach`, then sends
`stop`, holds that key for the rest of the process, and every other consumer
bound to it reads it held. The bench view disagrees with the world:
`gesture::expand` does `held.remove(wire)` before the game sees the line
(`crates/nova_bench/src/gesture.rs:246`).
The defect predates the range (`apply_section` is from `983d076c6`); what is
in range is `ba0cc418f`'s 45-line test, which names this exact invariant and
covers only the lowered-context half - it keeps the section entity alive.
Fix: record the section's source at press time and let a Release lift the
recorded source unconditionally, refusing only an unresolvable Press. Add the
sibling test: press, despawn the section, release, assert the source is up.

**MAJOR - `crates/nova_bench/src/pages/targeting.md:22` - the agent-facing
dwell figures silently count the search window the next paragraph excludes.**
Re-derived. Over the shipped defaults (`lock_dwell_base` 0.6,
`lock_dwell_range_factor` 1.5, `lock_dwell_reference_range` 2000 world units;
`crates/nova_ship/src/input/targeting/state.rs:84-88`), `lock_dwell_secs`
gives 0.645 s at 1 km and 0.7125 s at 2500 m - about 39 and 43 ticks. The page
says "about a second inside a kilometre, about 60 ticks at 2500 m" and
attributes both to THE DWELL, then says four lines later that "the first 15
ticks of every hold do nothing but search". The totals are only right with the
search counted in. A driver reading `dwell_needed: 0.71` cannot reconcile it -
which is the confusion `ba0cc418f` set out to remove.
`web/src/wiki/targeting-radar.md:33` states the curve correctly.
Fix: say which number is which, and name the 0.6 s to 1.5 s curve.

**MAJOR - `docs/automation-harness.md:492` and
`crates/nova_autopilot/src/loops.rs:57-62` still scope the frame-clocked
deadline to "inside a loop"; the same commit's `harness.rs` says otherwise.**
Re-derived. `LoopCapturePlugin::build` inserts
`TimeUpdateStrategy::ManualDuration` for the whole app as soon as
`capture::capturing()` holds (`loops.rs:347-351`) - before any loop opens - and
`crates/nova_debug/src/harness.rs:30` (written in this range) says exactly that:
"from plugin build on and not just while a loop is open". The book and the
`loops.rs` module doc both still tell an author to budget only
`loop_start`..`loop_end` in frames. So a pre-loop beat sized in wall seconds is
silently a frame count: `examples/screenshots/loop_command_shell.rs:178`'s
`.deadline(30.0)` is 900 frames on an armed run.
Fix: rewrite the "One exception" paragraph and the `loops.rs` Cadence
paragraph to say the pin covers the armed run end to end.

**MAJOR - `docs/environment-variables.md` omits seven `NOVA_*` names on a page
that opens by claiming to index every one.** Two lanes, independently (the
G5b contracts lane found the same gap from the other side). Verified by
grep: `NOVA_STANDOFF_TRACE`, `NOVA_COLLAPSE_LOOP`, `NOVA_SIGHT_LOOP`,
`NOVA_WAKE_LOOP`, `NOVA_MENU_PATH`, `NOVA_REUSE_STAGE` and `NOVA_UNFREEZE` all
return 0 hits, and all seven live in `examples/` or `scripts/`.
`tests/env_contract.rs` cannot catch it - line 115 says `examples/` is
deliberately absent. Re-cutting the release media off this bullet drives
`stress_hull_collapse`, `system_lock_line_of_sight` and `railgun_wake_bench`
without their `*_LOOP=1`, no webm is written, and
`scripts/capture-web-media.sh:210` aborts naming a loop the page never listed.
SUPERSEDES the G2 MINOR above, which named only `NOVA_STANDOFF_TRACE`.
Fix: add the five example-local names to that bullet and the two shell-only
names to the shell bullet.

**MINOR - `crates/nova_scenario/src/loader/lifecycle.rs:115` - the
`try_despawn` tolerance ships with no test and no range that stages the case
it fixes.** The gate it repairs is real
(`crates/nova_probe_cli/src/evaluation/checks/log_clean.rs` fails a run that
logs a stale-entity command error), and the one-line change is the right one.
But nothing in the tree stages "a scenario ends on the same frame its ship
dies", so the regression can return with nothing to catch it.
Fix: a `lifecycle.rs` test that despawns a scoped root through `Commands` and
then runs the teardown over a query that still lists it.

**MINOR - `docs/agent-bench.md:99` still shows `"ticks": K` unbounded** after
`manual.md:101` gained the `1..=MAX_AIM_TICKS` cap that `parse_gesture`
enforces. `docs/keeping-docs-in-sync.md` routes `nova_bench` to this chapter.

**MINOR - `hud_instrument` landed as a THIRD identical copy, not in a single
home.** `examples/screenshots/shared/ring.rs:319`, `shared/hollow.rs:460` and a
private `screenshot_radar_lock.rs:287`, all byte-identical, each carrying the
same docstring - itself a fourth restatement of the rationale on
`hide_status_bar`. The private one does not answer `grep "pub fn
hud_instrument"`, so widening what an instrument framing means edits two of
three and ships the bar in `screenshot_radar_lock`'s still.
Fix: one `hud_instrument` in `crates/nova_debug/src/harness.rs` beside
`hide_hud` and `hide_status_bar`, exported from the harness prelude.

**MINOR - `examples/systems/stress_hull_collapse.rs:529` calls
`hide_status_bar` after `hide_hud`, which already covers it.** `hide_hud` sets
`HudVisibility::Cinematic` and `HudVisibility::shows()` is `On`-only
(`crates/nova_hud/src/lib.rs:149`), so Cinematic clears every tier including
Status - pinned by
`status_bar_persists_through_the_nova_os_but_cinematic_still_clears_it`. The
pair reads as a contract `hide_hud` does not have, and makes the fifteen
producers that call `hide_hud` alone look wrong.

**MINOR - `examples/playable/wfc_arena.rs:2759` - a nine-line body comment
that explains the previous revision.** It argues about counter scoping and
concludes the two zeroes under it do nothing ("the baseline the subject is
installed with, not the fix"). AGENTS.md: comment where the reason is not
recoverable, and explain constraints, not history.

### G5a - recovered (`7fdd25222`, the three largest loop producers)

Lanes: correctness, craft. No contracts lane.

**MAJOR - `examples/screenshots/loop_goto_standoff.rs:259` - `both_parked()`
reads every autopilot disengage as an arrival, so a failed leg closes the loop
green.** Re-derived. The gate is `engaged == 0` over `Autopilot`, and the
docstring states the contract as fact: "the computer disengages itself on
arrival, so a hull with no autopilot is a parked one". `autopilot_system`
removes `Autopilot` at seven sites besides the arrival
(`crates/nova_ship/src/flight/autopilot.rs:207,256,322,333,570,626` and the
arrival at 984): lost flight computer, no live engines, target gone, well gone,
no stable band. Any of those strips it from both hulls on the first tick, the
first polled frame sees `engaged == 0`, `report_the_park_points` prints a gap
nothing checks, and a two-second webm of two motionless hulls encodes at exit
0. The shipped `news-0130-goto-standoff.webm` is correct; this is the gate for
the next re-shoot.
Fix: gate on the measured gap against `ARRIVAL_MARGIN`, or at minimum `warn!`
when the printed gap does not match the margin the file calls "the proof of
the rule".

**MAJOR (mechanism confirmed, threshold unmeasured) -
`examples/screenshots/loop_helm_orders.rs` pins no seed for a producer whose
recorded subject is a gunfight.** Verified: the file contains no seed
reference, `scripts/capture-web-media.sh:108` gives it an empty env column, and
`crates/nova_gameplay/src/plugin.rs:85` takes OS entropy unless `NOVA_SEED` is
set. Turret muzzle spread draws from it, so the time the gunship needs is a
random variable. The shipped webm is 13.1 s = 392 of the 600-frame cap, most of
it the fight. A worse roll crossing 600 frames is arithmetic on an unmeasured
distribution - I did not run the producer - but every successful re-shoot is
different footage either way.
Honest context: no loop producer in the roster pins a seed. This one is called
out because it is the first whose subject is a fight to the kill.

**MAJOR - every deadline inside an open loop is at or above the recording
frame cap, so no named-beat abort can fire while recording.** Re-derived
exactly. `LOOP_FRAME_CAP` is 600 and `LOOP_FPS` is 30
(`crates/nova_autopilot/src/loops.rs:103,109`), so the cap is 20 s of
frame-clocked budget. Against it: `FIGHT_DEADLINE_SECS` and
`LEGS_DEADLINE_SECS` are both 120.0 = 3600 frames; `STEP_DEADLINE_SECS` is 30.0
= 900 frames; `BEAT_DEADLINE_SECS` is 20.0 = 600 frames, exactly the cap; and
`loop_hull_generate`'s six `hold` steps carry no deadline at all. A stalled
beat therefore reports "loop exceeded the 600-frame cap - shorten the loop, do
not raise the cap" instead of naming itself. The run still fails loudly; the
diagnostic is what is lost.
Fix: express in-loop deadlines as a fraction of `frame_cap / fps`.
Depends on the `harness.md` / `loops.rs` doc fix above landing first.

**MAJOR (craft) - `examples/screenshots/loop_helm_orders.rs:289` -
`give_the_order` re-implements `nova_scenario`'s private `install_ship_order`
and drops both of its guards.** Verified byte-parallel against
`crates/nova_scenario/src/actions/ship.rs:841-861`: same `cancel_ship_order`,
same insert-if-absent `ShipOrderReports`, same
`(ShipHelmOrder, ShipOrderHelmAuthority)` - minus `orderable_ship`, which
refuses a `PlayerSpaceshipMarker` ship by name and resolves through the
scenario-scoped lookup rather than an unscoped `EntityId` sweep. It also copied
the conditional insert without the comment that explains it. Nothing is wrong
today (the gunship is `SpaceshipController::AI`).
Fix: make `install_ship_order` public through the `nova_scenario` prelude and
call it.

**MINOR - `loop_goto_standoff.rs:419` - the park log fabricates zeroes for
reads that failed and never checks the number it prints.** `map_or(0.0, ..)` on
both `Position` and `HullRadius`, so an unresolved hull logs `parked 0 m ...
gap -40 m` as a measurement. The module doc calls this log "the proof of the
rule".

**MINOR - `loop_hull_generate.rs:715` - the two 1080p stills are shot two
frames after a 720p to 1080p resize.** `the_window_is_figure_sized()` advances
on `and(window_size_is(w, h), frames(2))` and the next step shoots on entry.
`frames(2)` is `window_size_is`'s recipe for reading a laid-out box, not for a
rendered frame; sixteen sibling producers put `SETTLE_FRAMES` (30,
`crates/nova_debug/src/harness.rs:160`) before a shot.

**MINOR - `loop_hull_generate.rs:94` - `ZONE_CYCLE` hard-codes a starting zone
that comes from generated content the walk never reads.** The cycle is correct
only while `pdc_kinetic_turret_section` carries no authored zone. It carries
none today (`assets/base/grammars/base.content.ron:41`), but that file is
generated from the Rust builders, and `next_zone` starts at `Bow` only from
`None`.

**MINOR - `loop_hull_generate.rs:483` - the wheel beat computes its scroll once
on entry with no per-frame retry**, unlike `press_the_zone_chip` and
`AutopilotPlugin::click_named`, which both re-drive each frame so a beat
recovers from a reflow instead of holding a stale coordinate.

**MINOR (craft, one theme) - the three producers re-derive what
`examples/screenshots/shared/` already owns.** Counted against the tree:
`refuse_broken` now exists in eight example files; `entity_by_id` in four, and
`loop_helm_orders` also carries a divergent `&World` twin so one file resolves
an id two ways; `POSE_EPSILON` / `pose_for_the_landing` /
`the_landing_camera_is_posed` are copies of `shared/ui_walk.rs:64,78,151`; the
menu-to-Sandbox-to-editor prologue is a verbatim 21-line copy of
`screenshot_editor.rs:155-180`; `GenerateGestures` is a second gesture trait
whose `hold`, `retype`, `scroll_the_rail_to` and `tick` are walk-agnostic; and
`named_node` (`:224`) re-derives `nova_autopilot`'s `ui_node_rect` without its
visibility filter or its duplicate-name warning, while `zone_chip` takes
`Children::last()` because `nova_editor`'s `PartZoneChip` is `pub(crate)`.
These are one consolidation task, not eight commits.

### G5b - recovered (`7fdd25222`, Rust and scripts outside those three)

Lanes: correctness, contracts. No craft lane.

**MAJOR - `scripts/gen-web-screenshots.py:242` and `:1060` - the four new
`CUTS` sources are named by no producer, so the sweep never stages them and
still exits 0.** Two lanes, independently; the outer session had already
verified it before it died. `producers()` walks `FIGURES` only, and none of
`asteroid-kinds-grid.png`, `planet-types-lineup.png`, `wfc-ships-row.png` or
`first-shift-ships.png` has a `FIGURES` row - each appears exactly once in the
tree, in `CUTS`. So `scripts/capture-web-shots.sh:56` never runs
`asteroid_kinds`, `planet_types`, `wfc_ships` or `first_shift_ships`,
`build_cuts` appends to `pending`, which only feeds a count, and `main()`
returns 0. `first_shift_ships` was taught to shoot its still IN THIS COMMIT and
nothing in the repo runs it under `NOVA_CAPTURE=1`.
This is the exact contract `producers()`'s own docstring and
`docs/development.md:378` state. The four figures are shipped and correct; what
is dead is the way to make them again.
Fix: give `CUTS` a producer column and walk it in `producers()`. Adding the
bench frames to `FIGURES` is the wrong fix - they carry the seed readout.

**MAJOR - `examples/systems/stress_hull_collapse.rs:217` -
`NOVA_COLLAPSE_LOOP=0` turns the loop mode ON.** Two lanes. Verified:
`loop_requested()` is `var_os(LOOP_ENV).is_some()`, while both switches added in
the SAME COMMIT parse the value - `sight_loop()`
(`examples/systems/system_lock_line_of_sight.rs:123`) and `loop_cut()`
(`examples/playable/railgun_wake_bench.rs:1030`) each match `"0"`/`"1"` and
panic on anything else, copying `live_cut()`. Setting it to `0` without
`--features debug` panics at `stress_hull_collapse.rs:309`; with `debug` it arms
`LoopCapturePlugin`, pins `ManualDuration`, and the two recorded frame-cost
claims report the recorder's step rather than the collapse's - the exact
contamination the module doc warns about. Nothing in `scripts/` or CI sets it
to `0` today.
Fix: match `"0"`/`"1"` and panic otherwise, as both siblings do. AGENTS.md's
explicit-authoring rule points the same way.

**MAJOR - `web/src/assets/loops/manifest.txt` was hand-appended and does not
match what the capture script writes.** Verified on all three counts. The
header pins `# captured at commit a5da7efdb`, which is 48 commits behind HEAD,
above 22 rows for footage produced at HEAD. Rows 52-54 - the three
`news-0130-*-before-after` imports - carry state `fresh`, but `package_import`
writes the literal `frozen` (`scripts/capture-web-media.sh:322-323`) and
`frozen` is the truthful value: nothing stages a hand-composed split. And the
row order is not the generator's write order (LOOPS, ALIASES, PENDING,
IMPORTED) - `news-0130-hull-collapse` and `news-0130-helm-orders` are LOOPS
rows sitting after the imports.
The next real sweep therefore produces a diff indistinguishable from a re-cut.
Nothing builds or renders off the state column.

**MINOR - `scripts/capture-web-media.sh:309` - `package_import` skips the
`1280x720` gate `package_loop` enforces.** It checks existence and the byte
budget only. A hand-composed split at 1920x1080 - the resolution every producer
shoots at, so the natural source size - ships with a success line. All seven
shipped imports are 1280x720, so this is latent. These are the only files in
the flow that cannot be restaged, so the size gate matters more here, not less.

**MINOR - `examples/systems/stress_hull_collapse.rs:1080` - the new recorded
drift/spin reading carries no `probe_marker` and no roster slug.** Two lanes.
Every other reading in `verify()` is a marker on the
`crates/nova_probe_cli/tests/catalog_drift.rs:513` roster, INCLUDING the two
that only record ("the collapse frame cost is recorded", "the debris the
collapse threw is recorded"). This one is a bare `info!`, so deleting it keeps
every test green and `probe run` never carries the number.

**MINOR - `loop_sections_compare`, `loop_belt_compare` and `loop_death_compare`
are on no roster in the capture flow.** Each is registered in `Cargo.toml` and
each writes a `news-0130-{sections,belt,death}-after.webm` (their `LOOP_NAME`
constants), but none appears in `LOOPS`, `ALIASES` or `PENDING` - each appears
exactly once in `scripts/capture-web-media.sh`, in the free-text source column
of an `IMPORTED` row. Re-cutting the "after" halves produces none of them and
reports success.

**MINOR - `docs/development.md:365-371` went stale on both of its claims.** The
"each ship a loop beside their stills" list still names only
`screenshot_railgun`, `screenshot_editor`, `wfc_arena` and
`system_torpedo_launch`; this range added pairs to `railgun_wake_bench`,
`stress_hull_collapse` and `system_lock_line_of_sight` - the last two being
`systems/` correctness ranges, which is the interesting half. And "which name
every producer and every file it writes" is false for the three compare
producers above.

**NIT - `scripts/gen-web-screenshots.py:236-241` - the `CUTS` comment says a
post figure's window "starts under" the readout and then that "the whole frame
is the window (0, 0, 1920, 1080)".** Muddled rather than false; the asteroid row
does keep its readout and the other three do not.

Dropped in adjudication: `examples/playable/first_shift_ships.rs:54` adding
`force_capture_resolution` / `hide_dev_overlays` ungated. The G5b correctness
lane called it a finding and the G5b contracts lane examined the same lines and
declined - correctly. `railgun_wake_bench.rs:127` and seven other producers do
exactly the same ungated. If the idiom is wrong it is a fleet question, not
this range's.

Downgraded in adjudication: the G5a craft lane's MAJOR on the 89 per-item
`#[cfg(feature = "debug")]` attributes across the three producers. The build
break it describes was real at `7fdd25222` and is REPAIRED by `9e69ac196`, two
commits later and inside this same review window; CI's default-features
`-D warnings` job now catches a recurrence. As a defect it is obsolete. As a
maintainability finding - one gate per module is the shape `shared/ui_walk.rs`
states as the rule and these files quote while not following - it stands, at
MINOR.

## Disposition

Every finding above, recovered and already-written, with what it costs a user,
what a fix risks, and what shape the work is. Confidence is against the tree at
`88a7445b4`, which is current `master`.

### Fix now

| Finding | Sev | Conf | Impact | Effort | Risk |
|-|-|-|-|-|-|
| G2 BLOCKER: GOTO at 2-5 m/s flies tail-first and never burns (`autopilot.rs:862`) | BLOCKER | re-derived | SHIPPED in v0.13.0. A player or AI leg engaged in that band coasts nose-aft with cold engines and no timeout. From 4 km, 22 minutes. | small: name the 0.5 floor, or publish an explicit `braking` flag | medium - touches the live arrival gate; needs the autopilot test it has none of |
| `NOVA_COLLAPSE_LOOP=0` arms the loop (`stress_hull_collapse.rs:217`) | MAJOR | confirmed | An operator turning the mode off the documented way panics or records a contaminated frame-cost claim | 6 lines, copy the sibling verbatim | none |
| `package_import` skips the resolution gate (`capture-web-media.sh:309`) | MINOR | confirmed | An off-spec hand-cut split ships with a success line | 4 lines, same file as above | none |
| `CUTS` sources reachable from no producer (`gen-web-screenshots.py:1060`) | MAJOR | confirmed, two lanes | The regeneration path for four SHIPPED news figures is dead and exits 0. The only recovered finding whose failure mode is silent | small: producer column plus one loop | low - the flow fails loud either way after the fix |
| Docs sweep: `environment-variables.md` (7 names), `automation-harness.md:492` + `loops.rs:57`, `targeting.md:22`, `agent-bench.md:99`, `development.md:365-371`, `gen-web-screenshots.py:236` nit | MAJOR x3, MINOR x3 | confirmed | Text only. Two of them (the env index, the frame-clocked deadline) are what a contributor reads before driving the capture pipeline or sizing a beat | one pass, no code | none |
| `stress_hull_collapse.rs:529` redundant `hide_status_bar` | MINOR | confirmed | Reads as a contract `hide_hud` does not have | delete one line | none |

The docs sweep supersedes the G2 MINOR on `environment-variables.md:178`; do
not fix that one separately.

### Schedule later

| Finding | Sev | Conf | Why not now |
|-|-|-|-|
| G2 MAJOR x3: `fine` requires `brake.is_none()`; GOTO goal on COM against origin-measured radii; patrol gate measures origin | MAJOR | re-derived | All SHIPPED. Each is a real behaviour defect but none strands a ship the way the BLOCKER does. The COM pair should land together - they are the same mistake on two sides |
| `section` STOP refused when the mount is gone (`apply.rs:262`) | MAJOR | confirmed | Bench/process-channel drivers only, and needs a mount to die between two driver lines. The only recovered finding that can corrupt a live process |
| `both_parked()` treats any disengage as arrival (`loop_goto_standoff.rs:259`) + the park log's fabricated zeroes (`:419`) | MAJOR + MINOR | confirmed | Capture-pipeline correctness. The shipped webm is right; this is the gate for the next re-shoot. Fix the two together |
| `loop_helm_orders` pins no seed | MAJOR | mechanism confirmed, threshold unmeasured | Wants a measurement slot to size the fight against the 600-frame cap. Fleet-wide question: no loop producer pins a seed |
| In-loop deadlines at or above the frame cap | MAJOR | confirmed arithmetic | Diagnostics only. Depends on the `harness`/`loops` doc fix landing first, so an author knows what a deadline counts |
| `manifest.txt` provenance + the three compare producers on no roster | MAJOR + MINOR | confirmed | The honest fix is one real sweep on a capture host. Patching three rows and the header by hand is the cheap half and leaves the row order wrong |
| `give_the_order` re-derives `install_ship_order` | MAJOR (craft) | confirmed | Nothing is wrong today; wants `install_ship_order` made public through the prelude |
| G1 MAJOR x3 and its seven MINORs; G2's remaining MINORs | mixed | see above | Already written; unchanged by this recovery |
| The `examples/screenshots/shared/` consolidation: `hud_instrument` x3, `refuse_broken` x8, `entity_by_id` x4 plus a divergent twin, the pose helpers, the editor prologue, `GenerateGestures`, `named_node` | MINOR (one theme) | confirmed | One task, not eight commits. No behaviour rides on any of it |
| `lifecycle.rs` try_despawn has no test; the drift/spin reading has no roster slug; `frames(2)` before the 1080p stills; `ZONE_CYCLE`'s content coupling; the single-shot wheel; `wfc_arena`'s history comment; the 89 per-item cfg gates | MINOR | confirmed | Maintainability batch |

### Accept risk

- `ZONE_CYCLE`'s coupling to generated grammar content, if a one-line note is
  added instead. It fails loudly, and the walk names the row it depends on.
- The 89 per-item `#[cfg]` attributes, IF the default-features `-D warnings`
  job is trusted to catch a recurrence. It caught nothing at `7fdd25222`
  because nobody ran it; it is in CI now.

### Close as invalid or obsolete

- `first_shift_ships.rs:54` ungated capture wiring. INVALID as a range finding:
  two lanes examined the same lines and disagreed, and the fleet idiom is on
  the side that declined.
- The G5a craft lane's build-break MAJOR. OBSOLETE as a defect - `9e69ac196`
  repaired it inside this same review window. Retained at MINOR as shape.

### Dependency order

1. The docs sweep first. It is free, and the `harness`/`loops` correction is
   what makes the in-loop deadline work legible to whoever does it.
2. `NOVA_COLLAPSE_LOOP` and `package_import`'s gate together - one file each,
   no dependants.
3. `CUTS` producers before any capture-pipeline work, because it is what makes
   a re-cut reproducible at all.
4. `both_parked()` + the park log before the next `news-0130-goto-standoff`
   re-shoot, not before.
5. The manifest sweep last of the capture items: it wants the pipeline above it
   to be correct first, or the sweep bakes in the same wrong rows.
6. The GOTO BLOCKER is independent of all of the above and is the only item
   here that a player can hit.

## Continued 2026-09-10: G6-G11

Dispatched two lanes at a time on 2026-09-10, against `88a7445b4` for G6/G7 and
against HEAD for G8-G11. The comic groups were RE-SCOPED as the earlier
disposition asked: the comic work has since landed at `8896f385f`, so those
lanes reviewed the completed episode rather than the superseded tree.

Still no measurement slot. Every claim below is from reading code, running unit
suites, or arithmetic - no rendered example, no probe, no pixel comparison.

| # | Group | Lane | Status |
|-|-|-|-|
| G6 | News-post web widgets | correctness | COMPLETE (path-scoped to the physics widgets) |
| G7 | Release v0.13.0 and changelog | contracts | COMPLETE |
| G8 | Comic build and deployment | contracts | COMPLETE |
| G9 | Comic DSL, lettering, panels, reader | correctness | COMPLETE |
| G10 | Illustration library and episode art | correctness | COMPLETE (re-scoped to HEAD) |
| G11 | Episode pages as content | contracts | COMPLETE (re-scoped to HEAD) |

### G11 - the two that ship to readers

**BLOCKER - `web/src/comics/season-1/episode-1/pages/*.ts` - 29 of 50 panel
`action` fields are illustrator directives, and they ship verbatim as the
reader-facing transcript.** `comic-script-build.js:199-213` builds the
transcript from `action`, and `comic-build.js:141-142` publishes it as both the
`<noscript>` fallback and the Transcript modal. So a screen-reader user reads
"Do not draw looping jet-fighter trails" (`page-11.ts:11`) and "Tomas is not
isolated as the scene's unkind person" (`page-09.ts:118`) instead of a
description. Panels 2C, 9C, 12C and 13B carry no visual description at all.
`README.md:66` already says production constraints belong in `NOTES.md`.

**BLOCKER - `page-04.ts:56` and four more - the transcript tells the
accessibility audience the plot the sighted reader is denied.** Five `action`
clauses name the attack, the rescue and the conspiracy; zero dialogue lines do.
`lore/seasons/season-1.md` states the rule: "The audience learns about the
attack alongside them ... no independent cutaway." Page 4 says "No attack or
pursuit appears" four pages before the distress call; page 15 says "not a
conveniently labelled conspiracy file". The POV contract is broken only for the
readers who cannot see the art.

**MAJOR - all 18 `purpose` fields become the page's `aria-label`**
(`comic-script-build.js:281` -> `comic-lettering.ts:280`). Shipped alt text
includes "Establish Nadia and Gantry as working colleagues, not doomed victims"
and "without an on-foot game promise".

**MAJOR - `page-05.ts:19`** - the released episode's transcript calls its own
location card "a draft stamp". MINOR: `page-07.ts:54` and `page-11.ts:11`
hand-write "No dialogue." that the DSL already appends; `README.md:70-87`'s
sample uses a dialogue id scheme no page uses.

### G9 - the comic engine

**MAJOR - `comic-lettering.ts:140,145` - a one-line balloon with a left or
right tail draws a malformed outline, and five ship in episode one.**
Re-derived arithmetically: `h = 42 + 27 * lines` and the tail re-attaches at
`0.65h + 10` while the next command is `V(h - 24)`, so the outline reverses
unless `h >= 97.14`. One line (`h = 69`) backtracks 9.85 px, two lines
(`h = 96`) by 0.40. Twelve left/right-tailed balloons ship.

**MAJOR - `comic-script.ts:86` - the speaker is the one id in the DSL with no
registry**, so `Lelia` or `Leila / Comms` builds green and ships a wrong
nameplate and a wrong transcript line. Every other id is checked.

**MAJOR - `comic-script-build.js:288-318` - lettering and card geometry is
never checked against the panel's own scene size.** An escaped balloon
overdraws the neighbouring panel (`comic-panels.ts:84-92`, `overflow: visible`);
an escaped card is silently clipped away while the transcript still lists it.
Zero violations in the shipped episode.

**MAJOR - the compositor has no automated test.** `comic-lettering.ts` and
`comic-panels.ts` are covered only by three `wrapDialogue` assertions.
`ComicPlayer`'s 290 lines have none. MINOR: DSL errors name no page or panel;
an unknown fragment silently opens page 1; the README documents the wrong
coordinate space for `label`; `compileScript`'s "Unknown scene" is unreachable
because Python raises `KeyError` first.

### G6 - the news-post widgets

**MAJOR - `widgets.ts` `zonePlacements` counts anchors on the 8-wide mirrored
ship; the collapse decides on the 4-wide starboard half.** Re-derived:
`Grid::starboard_half` sets `size = (half_width, height, length)` = (4, 5, 11)
(`grid.rs:43-52`, `nova_wfc/src/lib.rs:246-250`), and `lib.rs:138-139` refuses a
grammar whose half-width is short of a seeded drive - proof the half grid is
where placement is judged. A 5-wide capital drive reports 36 places and has
zero.

**MAJOR - the same widget intersects several zones per part; `zone` is
`Option<GrammarZone>`** in both `ship_grammar.rs:155` and `tiles.rs:33` ("The
only region of the hull this part may stand in"). The multi-select key row and
its two "impossible intersection" readouts describe an authoring path that does
not exist, and a test blesses it.

**MAJOR - `widgets.test.ts` pins ratios and signs, not the game constants**, so
`LANCE_RECOIL_IMPULSE`, `OCCLUSION_CONTACT_RANGE`, `ARRIVAL_STANDOFF`, the
spatial-audio constants and the sound-cue filenames all drift silently. Every
value is correct today; nothing would catch the next Rust edit.

MINOR: thirteen `file.rs:NNN` provenance citations point at the wrong lines,
two into `#[cfg(test)]` code; `collapseBudget.unchipped` is one crater high
because `spew.rs:567` spends the budget per chip, not per crater, and
`widgets.test.ts:612` pins the wrong value; `RAILGUN_BASE_HEALTH` is dead;
an HTML comment names an `AudioRoute` variant that does not exist.

### G7 - the release record

**MAJOR - `CHANGELOG.md:437-438` documents nine First Shift production scenes
that were deleted in the same cycle and never shipped.** Re-derived: at tag
`v0.13.0`, `examples/playable/` holds only `first_shift_map.rs` and
`first_shift_ships.rs`, and `first_shift_scene.rs` returns zero hits in the
tagged tree. `220509616` removed the six chapter entries and missed this one.

**MAJOR - `CHANGELOG.md:136-138` is a shipped format break with no marker.**
`spawn.rs:328` declares `asteroid_kinds: Vec<(String, u32)>` with no
`serde(default)`, where both its neighbours carry it. The field did not exist at
v0.12.0, and nine base scenarios plus two portal mods used `ScatterObjects`
then. The entry directly above IS marked `**(breaking)**`; this one is not, and
it breaks non-asteroid scatters the marked entry never mentions.

MINOR: the fleet-move entry carries its marker but drops the migration sentence
the news post has; `nova_mod_format/src/lib.rs:140` says the story campaign
ships `enabled_by_default` and the flag has zero users.

Verified and clean: all 165 entries under the 200-character cap, subsystem
order, the version bump across 29 manifests and both link refs, and 138
backticked identifiers.

### G8 and G10 - one finding, found from two sides

**MAJOR - no CI job runs any web or Python test.** `ci.yaml` has six jobs and no
Node step; the only `npm` in any workflow is `npm ci` + `npm run build:*` in the
two deploy workflows. `web/package.json` defines a full `ci` script chaining
fourteen suites - including `widgets.test.ts` and `test:deploy` - and none of it
runs anywhere. `scripts/test_deploy_pages.py:155` actively asserts
`assertNotIn('npm run test:deploy', comic)`, pinning the comic workflow out of
the one test written to catch a broken `publicPath`. The "Generated art is
deterministic" step (`ci.yaml:60`) covers `gen-greebles.py` and
`gen-thruster-shells.py` only - not the 62 Python illustration tests, not
`gen-lore-designs.py --check`, and not the comic generator, so ~19k lines of
committed generated art have no reproducibility gate. The G10 lane ran all of
it: 62 tests pass and the comic build is byte-identical across processes with
randomised hash seeds, so nothing is broken - only ungated.

### G10 - the illustration library

**MAJOR - "which body does this character have" is re-derived in three places
with three inconsistent coverage sets** (`portraits.py:194`, `aquila.py:23`,
`transfer.py:76`). `held_person('ivo')` and `guide_layers('nadia')` raise
`KeyError` for characters who have a registered body and appear in the episode.

**MAJOR - 35 of 50 scene functions are entered by no test**, including
`opening.coffee_break`, the sole caller of both authored facial expressions -
the feature `4797eab8b` is named for. Two tests assert expressions ABSENT and
none asserts one present. Four art modules have no test file.

**MAJOR - `ships.py:457` emits full polygon counts at fixed 3-decimal precision
regardless of draw scale**: 11.7 MB over 50 scenes, one 1 MB panel whose ship
occupies 85x48 px with 1,666 polygons.

MINOR: `lettering.speech()` has no caller in the shipped tree; `rail_grip_hand`
is duplicated verbatim inside `gripping_arm`; the pressure-window frame is
implemented three ways and one drops its `data-prop`; eleven dead `w = 1416`
assignments contradict the scenes they sit in; `render_scene` validates the
frame but never the art inside it; `build-comics.py:21` reads and writes with
the locale encoding.

### Groups now complete

G1-G11 have all run. The review's remaining hole is unchanged and is not a
coverage gap: NO LANE IN ANY GROUP HELD THE MEASUREMENT SLOT.

## Remaining review gaps

### Groups that never ran

G6-G11, and the smallest coverage that buys the most is NOT one lane per group:

- **G7 (release v0.13.0 and the changelog corrections, ~150 lines) - one
  contracts lane.** Cheapest group in the plan and the highest stakes per line:
  it is the shipped release record. G2 already pre-empted part of it (it read
  all 165 entries of the released block for length and found the collapse-rule
  breach), so the remaining surface is `78baad3eb`, `1956c26f2`, `5348c632f`
  and the release commit itself.
- **G6 (news-post web widgets and page) - one correctness lane, PATH-SCOPED to
  the widgets that assert game physics**, not the whole ~4250 lines. G2 already
  found a live defect of exactly this class (`web/src/widgets.ts:792,4431`
  draws a flip line the game no longer flies), which is the evidence that this
  slice is where the defects are. The prose and layout of the post are not
  worth a lane.
- **G8-G11 (the comic pipeline) - DEFER and re-scope, do not run at
  `88a7445b4`.** Those paths are under active uncommitted edit in the main
  checkout right now (`web/src/comics/**`, `scripts/nova_illustration/**`,
  `web/src/lore/**` are all modified, plus new episode art). Reviewing the
  committed tree would review a state the author has already moved past.
  Re-scope them against the range that lands the comic work.

That is two lanes to close the useful part of the gap, against the eighteen a
blind rerun of G6-G11 would cost.

### Gaps inside the groups that DID run

- **No measurement evidence anywhere in G1-G5b.** No lane held the slot; the
  workspace check owned the box. Every frame-cost, loop-timing, encode-budget
  and on-screen claim across all five groups is unverified, including G1's
  1.68 ms WGSL figure and the three recovered items that want a capture host
  (`loop_helm_orders`' seed, the in-loop deadlines, the manifest sweep). This
  is the single largest hole in the review and it is not closed by rerunning
  lanes - it needs a quiet host and a serialised measurement slot.
- G3+G4 ran without a feel or performance lane. Low cost: the range changes no
  rendering except capture framing.
- G5a ran without a contracts lane and G5b without a craft lane. Each half
  partly covers the other's gap - G5b contracts found the env and manifest
  contracts that span both, G5a craft found the shared-helper duplication that
  spans both - so the residual gap is small.
- Nothing ran the test suite or Clippy in any group. CI owns both, by contract.
