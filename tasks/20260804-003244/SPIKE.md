# Spike: decide the v0.10.0 example fleet roster

- DATE: 20260804-093707
- STATUS: RECOMMENDED
- TAGS: v0.10.0, spike, examples, testing

## Question

Before `20260802-120029` rewrites the fleet on the predicate autopilot, decide
WHAT the fleet should contain: which of the current runs to keep, retire,
rewrite or merge, which new purpose-built test scenarios to add, and how the
frame-time story is told.

## Context

### The fleet is 26 examples, not 22

`grep -c '^\[\[example\]\]' Cargo.toml` -> 26. Current split: `sections/` 7,
`gameplay/` 4, `ui/` 6, `screenshots/` 8, `perf/` 1. The task's "22" was stale.

### The task's premise did not survive contact

The task assumed "an autopilot pressing keys is unlikely to WIN" the mainline
story scenarios. On the current tree both already do, end to end:

- `examples/gameplay/broadside.rs`, 11 beats: dies -> Defeat overlay -> Retry
  -> reload clean -> kills the corvettes -> CHECKPOINT -> chains into
  `broadside_gunship` -> Victory capture.
- `examples/gameplay/lifeline.rs`, 19 beats: same lose/retry proof, three
  waves, Continue into `final_tally`, picket, epilogue, campaign-complete.

Two assists make it work: `teleport_player` (broadside:175, skipping the burn
to the hauler) and `jump_clock(world, 30.0)` (lifeline:238 and four more,
fast-forwarding wave timers - the scenario's own gates then fire for real and
the script asserts `relief_remaining` tracked the jump, lifeline:245).

So "can a robot win a story scenario" was never the question. The real
question is what an assisted win over 8000 lines of story RON actually buys,
and what it costs when the story changes.

### The owner's read, which replaces the premise

Mainline story scenarios are the content least suited to being a test fixture.
When `lifeline.content.ron` (5553 lines, 32 handlers) gets rebalanced, a
446-line example that asserts its wave timings gets rewritten with it - and the
rewrite proves nothing new. Story scenarios are tested by players. Examples
exist to test SYSTEMS, on scenarios built for that job and nothing else.

How much churn that actually is, measured rather than assumed: `broadside.rs`
has 11 commits ever and `lifeline.rs` 6, and only four commits in history
(`aeaa3761`, `d320e1dc`, `09463091`, `4a1c0274`) touched an example and story
content together - two of them the scenarios' own authoring commits. The
dominant churn driver has been harness migration (`8cf34ebf`, `56d43cb5`,
`59bd8419`, `bbe3e9df`), which is exactly what this sprint is finishing. So an
earlier draft's "the most volatile content in the repo, every rebalance is a
rewrite" was asserted, not measured, and the measured version is smaller. The
maintenance argument is real but not sufficient on its own; the decision rests
at least as much on what a run proves per line of fixture.

That flips the roster question from "which story runs survive" to "what
systems need a fixture, and what is the smallest scenario that exercises
each".

### The house style already exists

`examples/gameplay/scenario.rs` and `examples/gameplay/playable.rs` both build
a `ScenarioConfig` in Rust and load it with `LoadScenario` (scenario.rs:89,
playable.rs:167). Neither touches `assets/`. They are already story-free system
fixtures; the decision below promotes what they do from precedent to rule.

The type is `ScenarioConfig` (`crates/nova_scenario/src/loader/mod.rs:147`),
wrapped by `LoadScenario(pub ScenarioConfig)` at :223. Note that
`screenshot_reel`'s doc comment calls it "the same `Content` type the modding
loader uses" - there is no `Content` type, and that comment is stale. Do not
propagate the name.

### Renaming categories is NOT as cheap as it looks

Category strings are load-bearing in three places, not two:

- `crates/nova_probe/src/bin/probe/native/env.rs:65` - `if category == "perf"`
  selects the fps window.
- Probe test fixtures/assertions in `catalog.rs`, `aggregate.rs`, `spec.rs`,
  `fixtures.rs`.
- **`tests/examples_smoke.rs` (339 lines)** - the one that matters. It hardcodes
  the category directory layout, four per-category const lists (`SECTIONS:32`,
  `GAMEPLAY:43 = ["scenario", "playable", "broadside", "lifeline"]`, `UI`,
  `SCREENSHOTS`), a `NOT_SMOKED:78` list with a per-entry justification, one
  `#[test] fn <category>_reach_playing_without_panic` each, and
  `catalog_matches_disk:109` as an explicit drift gate that fails when disk,
  the Cargo catalog and the smoke lists disagree.

No CI workflow names a category, but `catalog_matches_disk` runs under a bare
`cargo test`, which CI does gate on. That makes each directory rename ATOMIC
with its `examples_smoke.rs` edit: a commit that moves `perf/` to `stress/`
without updating the consts leaves the tree red. The foundation task owns this
file.

### What the retirements would cost

`broadside` + `lifeline` are NOT the only evidence for the systems they touch.
All four already have unit/integration coverage:

| System | Existing coverage |
| --- | --- |
| Chaining (`Continue`) | `crates/nova_assets/tests/broadside_assault.rs:214`, `:659`; `crates/nova_scenario/src/loader/lifecycle.rs:797` |
| Defeat overlay | `crates/nova_menu/src/tests/outcome.rs:19`, `:278`, `:395`; `crates/nova_assets/tests/lifeline_convoy.rs:452` |
| Retry reload-clean | `crates/nova_menu/src/tests/pause.rs:228`; `lifecycle.rs:555`, `:726`; `lifeline_convoy.rs:505` |
| Victory/CHECKPOINT | `broadside_assault.rs:148`, `:464`; `outcome.rs:148` |

What the retirement actually loses is narrower and worth naming precisely: the
COMPOSED, rendered, click-the-real-button path through all four in one live
app. That is a distinct kind of evidence from a headless unit test, and it is
the thing `systems/outcomes` has to reproduce - not the systems themselves,
which are already pinned.

This matters for sizing: `systems/outcomes` is justified as an end-to-end
composition, not as emergency cover for a coverage cliff. There is no cliff.

`fps_exempt = ["broadside"]` (Cargo.toml:35) is the only entry in that list -
a wart that disappears with broadside.

## Options considered

### Mainline coverage

| Option | Verdict |
| --- | --- |
| **Status quo: keep `broadside`/`lifeline` exactly as they are, zero work** | REJECTED, but it is the cheapest live option and belongs on the ballot. It costs nothing today and the churn measured above is low. Rejected because the two runs are ~800 lines proving systems that headless tests already pin, they are the reason `fps_exempt` exists, and they block the category cleanup - not because they are on fire. |
| Keep the full assisted arcs, legitimize the assists as a harness vocabulary | REJECTED. Strictly worse than the status quo: same coupling, plus documentation work. |
| Split each mainline into a cheap smoke run + a deep assisted arc | REJECTED. Widens the fleet and still pins the deep half to story data. |
| Delete mainline coverage; systems are proven on purpose-built fixtures | CHOSEN. |

### Test scenario content location

| Option | Verdict |
| --- | --- |
| Example-owned `.content.ron` + `include_str!` (the `screenshot_reel` pattern) | REJECTED for fixtures. Exercises the RON parse path, but a grammar change breaks it at runtime, mid-run, instead of at compile time. |
| `ScenarioConfig` built in Rust (the `scenario.rs` / `playable.rs` pattern) | CHOSEN. The compiler catches grammar changes, and a code-built builder is a reusable `fn` - `stress/` and `sections/` can share one ship-builder with a count knob. |
| Shipped under `assets/base/scenarios` | REJECTED. Test fixtures would appear in the player-facing Scenarios picker and in the web build. |

The RON loader path stays covered: shipped scenarios are parsed on every real
boot, and `screenshot_reel` keeps its embedded-RON load.

### Category taxonomy

`gameplay/` was never a contract, it was a leftover. Options: keep the name
with a rewritten contract (minimal churn, but the name keeps implying the
story runs being removed); `systems/` + `stress/` with `perf/` retained
alongside; `systems/` + `stress/` with `stress/` absorbing `perf/`. The last
was chosen - one home for every frame-time claim.

## Recommendation

### Five categories

| Category | Proves | Probe |
| --- | --- | --- |
| `sections/` | One section family, deeply. Multiple scenes, repeated rounds. | correctness; no fps |
| `systems/` | Cross-cutting gameplay systems on story-free code-built fixtures. | correctness; no fps |
| `stress/` | Scale: many objects, and the frame-time window. Absorbs `perf/`. | correctness + fps |
| `ui/` | UI surfaces and the widget zoo, driven by synthesized pointer input. | correctness; no fps |
| `screenshots/` | Image production. Not evidence of anything. | not probed |

`gameplay/` and `perf/` are retired as directory names. Story scenarios get no
example coverage; they are tested by players.

### Per-example roster, all 26

**`sections/` - 7 in, 5 out (merge two, deepen all)**

| Example | Verdict | Why |
| --- | --- | --- |
| `controller_section` | KEEP, deepen | PD attitude control. Today one scene, one command. Needs multiple layouts and repeated rounds. |
| `thruster_section` | KEEP, deepen | Throttle -> impulse + plume. Same shape, same gap. |
| `hull_section` | KEEP, deepen, ABSORBS `com_range` | Owns the damage -> destroy pipeline. COM-follows-destruction is that pipeline's consequence, not a separate subject. |
| `com_range` | MERGE into `hull_section` | Its assertion (`assert_com_follows_sections`, com_range.rs:374) becomes a round after the destroy round. Deeper, not wider. |
| `turret_section` | KEEP, deepen | PDC tracking + firing. 734 lines incl. an interactive slider rig; the slider stays for human tuning, the probe path asserts. |
| `torpedo_section` | KEEP, deepen, ABSORBS `torpedo_guidance` | Both are the torpedo bay family. One example per family is the contract. |
| `torpedo_guidance` | MERGE into `torpedo_section` | Its PN closest-approach assertion becomes the lead-a-crosser round of the merged run. |

**`systems/` - 2 kept from `gameplay/`, 1 new, 2 retired**

| Example | Verdict | Why |
| --- | --- | --- |
| `scenario` -> `systems/scenario_grammar` | KEEP, rename | Already the model: code-built `ScenarioConfig` exercising `OnStart`/`OnDestroyed`/`OnUpdate`, filters, variables, arithmetic. |
| `playable` -> `systems/player_path` | KEEP, rename | Already predicate-driven with invariants + probe markers. The full player gesture chain: lock, kill, travel-lock, GOTO, arrive. |
| `systems/outcomes` | NEW | Replaces broadside/lifeline's system coverage. See below. |
| `broadside` | RETIRE | Mainline. Its systems move to `systems/outcomes`. |
| `lifeline` | RETIRE | Mainline. Same. |

**`stress/` - 1 moved, 3 new**

| Example | Verdict | Why |
| --- | --- | --- |
| `perf_baseline` -> `stress/scene_baseline` | MOVE from `perf/` | Loads a shipped SANDBOX scenario (`asteroid_field`, not story) via `NOVA_PERF_SCENARIO`. Stays the release-over-release number. |
| `stress/many_bodies` | NEW | N asteroids under physics + gravity + render. Proves nothing panics or desyncs at scale, and fills a frame-time window. |
| `stress/many_sections` | NEW | One ship with N sections. Mass/COM aggregation and the integrity graph at scale; shares `sections/`'s ship builder with a count knob. |
| `stress/many_projectiles` | NEW | Turret + torpedo saturation. Collision, particles and despawn churn at scale. |

**`ui/` - 6 in, 5 out**

| Example | Verdict | Why |
| --- | --- | --- |
| `widget_zoo` | KEEP, drive | Already functional; gains real pointer input and live-tree assertions. |
| `hud_range` | KEEP | Already predicate-driven (1030 lines). Screen-projected indicators. |
| `editor` | KEEP, deepen | Today one editor action. Needs a real build-and-inspect sequence. |
| `menu_newgame` | KEEP, narrow | Proves the boot flow and menu teardown. Assert only that gameplay state is reached - NOT `shakedown_run`'s content, which is story. |
| `menu_scenarios` | KEEP, deepen | Picker navigation + the pane-width verdict. Gains pointer driving. |
| `nova_os_rtt_poc` | RETIRE | The RTT pipeline shipped; a POC is not coverage. Becomes an RTT element test beside the other widget tests. |

**`screenshots/` - 8 in, 8 out, all reduced**

`screenshot_reel`, `screenshot_ui`, `screenshot_combat`, `screenshot_nova_os`,
`screenshot_sections`, `screenshot_juice`, `screenshot_orbit`,
`render_scale_shot`: all KEEP as capture producers only - enter, wait on a
predicate, shoot, exit. No assertions, no fps wiring, no probe enrollment.
`render_scale_shot` stays out of probe entirely (real-GPU pixel check, human
eyes).

**Net: 26 - 3 retired - 2 merged + 4 added = 25.** Retired 3 (`broadside`,
`lifeline`, `nova_os_rtt_poc`); merged 2 into their families (`com_range` ->
`hull_section`, `torpedo_guidance` -> `torpedo_section`); added 4
(`systems/outcomes` plus three `stress/` runs). Per-category totals: sections 5,
systems 3, stress 4, ui 5, screenshots 8 = 25. The fleet gets deeper, not wider.

### `systems/outcomes`: the replacement for the retired story coverage

The systems broadside/lifeline composed end-to-end - chaining, defeat,
retry-reload-clean, victory/checkpoint - are all generic, and all four are
already pinned headlessly (see the table above). What needs replacing is the
composition in a live app, not the systems. One code-built pair covers it:

- **Scenario A** (`outcome_probe_a`): one objective, one killable hostile, one
  player ship. `OnDestroyed` on the hostile completes the objective and posts
  Victory with a CHECKPOINT; player death posts Defeat.
- **Scenario B** (`outcome_probe_b`): the chain target. One object, one
  trivially-completable objective, so the chain's arrival is observable.

The run's beats: die -> assert the Defeat overlay -> Retry -> assert the
reload is clean (the hostile is back, the tally is zeroed) -> kill the hostile
-> assert the objective posted and the CHECKPOINT landed -> Continue -> assert
scenario B loaded. Roughly 50 lines of `ScenarioConfig` against 8000 lines of
story RON, and it never needs touching when the campaign is rebalanced.

### The profiling story

Frame data comes from `stress/` alone. `sections/`, `systems/`, `ui/` and
`screenshots/` do not carry fps windows - which is why they are free to be
short and assertion-dense rather than padded to fill a window.

- `stress/scene_baseline` measures a real shipped sandbox scene: the number
  that is comparable across releases.
- The three scale runs take a count knob and a declared `loop_from` point, so
  the window is filled by repeated ACTIVITY - spawn the swarm, run it, tear it
  down, loop - rather than by idling.

`fps_exempt` disappears: `stress/` runs fps, no other category does, and the
per-category run policy replaces the hand-listed exemption.

## Open questions

- The count knob's default per `stress/` run is a tuning question for the
  implementing task, not a roster question. Pick a value that fills the window
  on the CI box under llvmpipe and record it.
- `turret_section` carries a 203-line interactive slider submodule. It stays
  for human tuning, but if the deepened probe path never touches it, a later
  task may want it extracted to a shared dev-widget module. Not blocking.
- Whether `menu_newgame` booting `shakedown_run` counts as story coupling.
  Judged NO: it asserts reaching gameplay state, not scenario content. If that
  assertion ever grows into scenario internals, it has drifted.
- `widget_zoo` sits in `NOT_SMOKED` (`tests/examples_smoke.rs:75-78`) because it
  runs its own `App` with no `GameStates` at all. "Drive it with pointer input"
  is therefore NOT a free add-on: it needs either `GameStates` added to the zoo
  or an autopilot that can drive a stateless app. The UI task has to pick one,
  and the choice is not obvious enough to settle here.

## Next steps

Grouped by effort and kind rather than one-per-category, so each task is a
single sort of work.

- `20260804-093855` FOUNDATION: category contract (root `Cargo.toml` catalog
  comment AND the dev wiki example page, plus the 10 `web/src/wiki/dev/` pages
  naming `gameplay/`, `examples/perf`, `perf_baseline` or `broadside`), probe
  per-category run policy, and `tests/examples_smoke.rs`. Code and docs, not
  content; everything else depends on it.
- `20260804-093934` SYSTEMS: the code-built fixture builders and the
  `systems/` runs including `outcomes`. The largest piece.
- `20260804-093950` SECTIONS: deepen the five surviving section runs, absorbing
  `com_range` and `torpedo_guidance`.
- `20260804-094006` STRESS: move `perf_baseline`, add the three scale runs,
  retire `fps_exempt`.
- `20260804-094021` UI: pointer-driven widget and pane runs with live-tree
  assertions, plus the RTT element test that inherits `nova_os_rtt_poc`'s
  coverage.
- `20260804-093910` RETIRE + REDUCE: delete the mainline and POC runs, reduce
  `screenshots/` to capture-only, and delete the per-example hacks the driver
  now owns (`playing_since` is still live in `screenshot_orbit.rs:151`,
  `screenshot_juice.rs:205`, `screenshot_combat.rs:231`). Cheap and mechanical,
  but it must land AFTER SYSTEMS - `systems/outcomes` has to exist before the
  end-to-end composition is deleted.
- `20260804-095507` FLEET EVIDENCE: run the fleet the way CI and probe will and
  record the report as the sprint's correctness+perf evidence. Closes the epic.

Ordering is encoded in `DEPENDS ON`, not prose. The category rename must be
ATOMIC with its `tests/examples_smoke.rs` edit in each commit that moves a
directory, or `catalog_matches_disk` leaves a bare `cargo test` red.

`20260802-120029` is superseded by these seven and should be closed as such: its
category table is replaced by the spike's, and its Steps are redistributed
above. Its two named proof tests
(`catalog_examples_satisfy_their_category_contract`,
`category_run_policy_selects_passes_per_category`) and its `playing_since`
absence grep are carried forward into the successors' DoDs rather than lost
with it.

The `*_poc.html` relocation is owned by epic child `20260804-003301`, NOT by
`20260804-093910`; `094021` depends on it for its "only runnable examples"
end-state.

## Fix record

- [ ] `20260804-093855` FOUNDATION
- [ ] `20260804-093934` SYSTEMS
- [ ] `20260804-093950` SECTIONS
- [ ] `20260804-094006` STRESS
- [ ] `20260804-094021` UI
- [ ] `20260804-093910` RETIRE + REDUCE
- [ ] `20260804-095507` FLEET EVIDENCE (last)
