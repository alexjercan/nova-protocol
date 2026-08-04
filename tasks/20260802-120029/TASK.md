# Rebuild the example fleet on the predicate autopilot, per category contract

- PRIORITY: 80
- TAGS: v0.10.0, content, examples, testing, autopilot
- KIND: STORY
- ACTIVITY: PLANNING
- GATES: -
- RESOLUTION: SUPERSEDED
- DUPLICATE OF: 20260804-003244
- PARENT: 20260802-115955
- DEPENDS ON: 20260802-120025, 20260804-003244

## Story

The example fleet grew one example at a time and each category means something
different in practice while probe treats them almost alike. Give every category
an explicit contract, then rebuild the examples against it on the predicate
autopilot - so the runs are harder, cover more angles, and assert what they
claim.

| Category | Purpose | Probe |
| --- | --- | --- |
| `sections/` | Correctness of one section family, deeply. Multiple scenes, multiple sections, state transitions, repeated rounds. | correctness passes; no fps |
| `gameplay/` | Correctness AND profiling of full player paths. | correctness + fps |
| `ui/` | Correctness of the UI surfaces and the widget zoo: panes open, widgets build, navigation works, nothing ghosts. | correctness; no fps |
| `screenshots/` | Image production only. Not evidence of anything. | not probed |
| `perf/` | Frame-time baseline scenes. | fps-first |

Today's `sections/` runs are mostly one scene, one section, one beat. With
predicate steps a run can walk several rounds - spawn, damage, destroy,
reload the scene, re-enter, assert the invariant again - and assert the
values it depends on rather than sleeping past them. `gameplay/` runs get long
enough and looped enough to fill a real frame-time window without the current
per-example loop plumbing.

## Steps

- [ ] Write the category contract down (root `Cargo.toml` catalog comment plus
      the dev wiki example page): what a category proves, what probe does with
      it, what disqualifies an example from it.
- [ ] Teach `nova_probe` a per-category run policy instead of the
      `perf`-vs-everything-else split plus a hand-listed `fps_exempt`:
      categories declare whether they run correctness passes, frame-time
      passes, or neither, and `--all`/category expansion honors it.
      `screenshots/` stops being probe's problem.
- [ ] Rebuild `sections/`: each run walks multiple rounds across at least two
      scenes/section layouts with predicate-gated assertions on the values the
      section family owns (mass/COM, thrust, integrity, guidance, lock, range).
      Keep one example per section family; make each one harder, not thinner.
- [ ] Rebuild `gameplay/` to the roster the spike returns. The bar for a
      MAINLINE story scenario is "the game plays normally": drive the opening
      objectives, assert the scenario's own variables, collect frame data - not
      win it. Deep coverage (many objects, transitions, a real win/lose
      outcome) belongs to purpose-built test scenarios that carry no story.
      Every run gets invariants, probe markers per beat, and a declared loop
      point so the fps window is filled by repeated ACTIVITY.
- [ ] Rebuild `ui/`: prove the UI zoo and the shipped panes by DRIVING them
      with synthesized pointer input - click the widget, open the NOVA OS
      computer, exercise the RTT screen, page the menus/editor - then assert
      the live tree (nothing ghosts or duplicates on state change). Retire
      `nova_os_rtt_poc` (the RTT pipeline shipped); its coverage becomes an
      RTT element test alongside the other widget tests.
- [ ] Relocate the three `*_poc.html` design sources out of `examples/ui/`
      (`20260804-003301`) so the category holds only runnable examples.
- [ ] Reduce `screenshots/` to capture producers on the same driver: enter the
      scene, wait on a predicate, shoot, exit. No assertions, no fps wiring, no
      probe enrollment.
- [ ] Delete the per-example hacks the driver now owns: beat booleans, panic
      guards, `playing_since` offsets, reload-gate polls, ad-hoc runways.
- [ ] Run the fleet the way CI and probe will, and record the resulting report
      as the sprint's correctness+perf evidence.

## Definition of Done

- Every cataloged example declares a category whose contract it satisfies; a
  category mismatch (a `screenshots/` run enrolled in fps, a `sections/` run
  with no assertion) fails the catalog test.
  (test: `catalog_examples_satisfy_their_category_contract`)
- Probe resolves run policy per category, with `screenshots/` excluded from
  `--all`. (test: `category_run_policy_selects_passes_per_category`)
- Every `sections/` run covers at least two scenes and repeated rounds, and
  asserts through predicates rather than elapsed time.
  (cmd: `nix develop --command cargo run -p nova_probe -- run sections`)
- The `ui/` fleet drives real widgets with synthesized pointer input, asserts
  the live tree, and completes headlessly; `examples/ui/` holds only runnable
  examples. (cmd: `nix develop --command cargo run -p nova_probe -- run ui`)
- `gameplay/` runs fill a frame-time window without per-example loop plumbing.
  (cmd: `nix develop --command cargo run -p nova_probe -- run gameplay --fps`)
- No example carries a hand-rolled completion guard or beat-boolean script.
  (cmd: `! rg -n "run ended with the scripted run unfinished|playing_since" examples`)

## Notes

- Examples must be RUN, not only checked: `cargo check` misses duplicate
  component panics and UI ghosting (Xvfb :99).
- `render_scale_shot` stays out of probe (real-GPU pixel check, human eyes).
- New test-only scenarios are welcome; they need objects, transitions and an
  observable outcome, not story, comms or balance.
- Pointer input synthesis comes from `20260802-120025`; without it the `ui/`
  contract is assertion-only.
- Screenshot packaging stays in `scripts/gen-web-screenshots.py`
  (`20260802-120045` closed WONTDO); this task only guarantees the producers
  it invokes exist and are on the shared driver.
- Prefer extending an existing example over adding a thin new one; the fleet
  should get deeper, not wider.
