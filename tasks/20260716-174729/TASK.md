# Gauntlet time-trial: visible run timer + clean-run bonus

- STATUS: CLOSED
- PRIORITY: 36
- TAGS: scenario, content, modding, hud, v0.8.0

## Story

As a player flying Gauntlet Run, I want a visible clock from START to FINISH
and recognition for a clean run, so that the course has a score to chase and a
reason to re-fly it.

Gauntlet 2.0 (20260716-124722, landed - the shipped bundle is at 1.2.0) ships
the course, hazards and outcome frames but NOT a clock, because the scenario
vocabulary has no timer readout. Since v0.7.0 the engine maintains
`scenario_elapsed` (live, pause-frozen, retry-reset scenario seconds readable
from expression filters), so the timekeeping half exists; what is missing is
the display half - nothing puts a scenario variable on the HUD. This task adds
that missing modding-surface piece and wires the gauntlet to it.

## Steps

- [x] Decide the HUD surface: a generic "show variable X on the HUD" scenario
      action (more reusable, benefits every mod) vs a purpose-built timer
      widget. Record the decision and why; the spike
      (tasks/20260716-174631/SPIKE.md) leans generic.
- [x] Implement the readout as scenario vocabulary (action to show/hide, bound
      to a variable such as `scenario_elapsed`, formatted mm:ss.s for time),
      respecting HUD visibility tiers and the pause/outcome freeze.
- [x] Wire Gauntlet: timer visible from the START gate, stopped and shown in
      the Victory banner text at FINISH.
- [x] Clean-run bonus: a crash counter variable (increment on player-ship
      hazard-zone OnEnter or on a damage signal) gates a Victory message
      variant ("CLEAN RUN" + time vs time only).
- [x] Extend `tests/gauntlet_course.rs` to pin the timer wiring (visible after
      START, stops at FINISH, clean-run variant gating).
- [x] Bump the gauntlet bundle version (minor - content rework) and re-publish;
      update the test's version assertion deliberately.
- [x] Docs in the same task: the new action goes into the scenario action
      reference (coordinate with 20260718-231555, which documents Gauntlet's
      whole vocabulary), CHANGELOG entry, gauntlet README.

## Step 1 - Design decision (GENERIC, MINIMAL)

Chose the GENERIC "show a scenario variable on the HUD" action over a
purpose-built timer widget (per the spike + the DoD "usable by any mod"). The
ONLY new engine piece is that one action + its sync + a HUD render module; the
clean-run counter and its gating reuse EXISTING vocabulary (VariableSet on a
`crash` variable, CreateScenarioArea hazard zones, and two crash-gated
`Outcome(Victory)` handlers).

The action mirrors the EXACT existing `StoryMessage -> StoryFeed (sync) ->
comms_panel HUD` pattern:

- `nova_scenario` gets a `HudReadout(HudReadoutActionConfig)` variant on
  `EventActionConfig` (actions.rs): `slot` (id), `variable`, `format`
  (`Number`/`Integer`/`Time`, Time = `mm:ss.s`), `label: Option<String>`,
  `visible: bool` (true shows/updates the slot, false clears it). Execution
  upserts a per-slot readout on the event world (`set_hud_readout` in world.rs),
  exactly as StoryMessage appends to the story log.
- The SYNC (world.rs `state_to_world_system`) copies the active readouts - each
  with its bound variable's CURRENT value read off the event world THAT FRAME
  (via `get_variable`, same as `scenario_elapsed`) - into a new nova_gameplay
  `HudReadouts` resource. Unlike the append-only story log this is rebuilt every
  frame (the value tracks a live variable), diff-guarded so it only writes when
  the set actually changes; an empty set (teardown) drops every row.
- `nova_gameplay` gets `hud/readout.rs`: `HudReadouts` resource +
  `HudReadoutEntry`/`HudReadoutFormat` (the format enum mirrored HUD-side, the
  same nova_scenario -> nova_gameplay split as StoryLine) + `HudReadoutPlugin`,
  a Startup-spawned Instrument-tier top-center strip that reconciles one row per
  active readout and updates each row's `Text` in place. Registered in
  hud/mod.rs like CommsPanelPlugin.

## Definition of Done

- A scenario-authorable HUD readout exists, documented, usable by any mod.
- Gauntlet shows a running clock, reports the final time on Victory, and
  distinguishes a clean run; retry resets the clock via `scenario_elapsed`
  semantics.
- Tests pin the behavior; the re-published bundle installs and updates cleanly
  from the portal.

## Notes

- Spike: tasks/20260716-174631/SPIKE.md (open question "visible timer /
  time-trial").
- This is the one v0.8.0 content task that needs a small engine/modding-surface
  addition (the readout) - accepted in the v0.8.0 plan as a modding-surface
  piece, not a gameplay feature. Keep the addition minimal.
- Dependency status: 20260716-124722 (Gauntlet 2.0) HAS landed; this decorates
  the shipped course.

## Grooming (2026-07-20): reprioritized 42 -> 36

Confirmed there is no existing HUD-readout action in nova_gameplay
(`grep -ri 'run_timer|ScenarioTimerHud|HudTimer' crates/nova_gameplay/src`
is empty), so this task carries a small NEW engine/modding-surface addition.
That brushes the v0.8.0 "no new features" theme - the plan explicitly accepts
it as a modding-surface piece, but it is the only content task needing code,
so it sits below the pure data/content polish (base campaign 152313, Ledger
152320) and below the tooling that unblocks content DX. Prereq: the
generic-vs-purpose-built decision in spike 20260716-174631 should land first.

## Close-out (2026-07-21)

Branch `feature/hud-readout-timer`. Delivered end to end on the sprout worktree.

What was added:

- Engine: the generic `HudReadout` scenario action
  (`crates/nova_scenario/src/actions.rs`) + `HudReadoutFormat`
  (Number/Integer/Time), its event-world upsert
  (`set_hud_readout`/`hud_readouts` field in `world.rs`), the per-frame sync in
  `state_to_world_system`, and lint coverage
  (`crates/nova_scenario/src/lint.rs`: empty-slot/empty-variable errors + the
  bound variable is tracked in the "never set" pass).
- HUD: `crates/nova_gameplay/src/hud/readout.rs` - `HudReadouts` resource,
  `HudReadoutEntry`, HUD-side `HudReadoutFormat`, and `HudReadoutPlugin` (a
  Startup-spawned Instrument-tier top-center strip reconciling one row per active
  readout, in-place text update). Registered in `hud/mod.rs`.
- Content: `webmods/gauntlet/gauntlet.content.ron` - a `HudReadout` timer on
  `scenario_elapsed` (Time, label TIME) from OnStart, a `crash` counter seeded to
  0, three `CreateScenarioArea` graze zones with `OnEnter -> VariableSet(crash =
  crash + 1)` (gated `gate < 8`), and the FINISH split into bookkeeping +
  TWO crash-gated `Outcome(Victory)` handlers. Bundle bumped 1.2.0 -> 1.3.0.
- Tests: `crates/nova_assets/tests/gauntlet_course.rs` extended (timer wiring,
  clean vs grazed Victory, graze counter, version); `nova_scenario` serde
  round-trip + action-drain tests; `nova_gameplay` readout format + reconcile
  tests.
- Docs: scenario action reference + authoring guide (`web/src/wiki/dev/`),
  CHANGELOG [Unreleased], `webmods/gauntlet/README.md`.

How the FINAL TIME on Victory is shown - FROZEN READOUT, not interpolation.
Resolved against the actual outcome-overlay code (`crates/nova_menu/src/lib.rs`,
`sync_outcome_overlay` + `sync_outcome_pause`): the outcome overlay is a
full-screen node with a 60%-alpha black scrim at `GlobalZIndex(9)` that dims but
does NOT hide the HUD, and the app stays in `GameStates::Playing` (the HUD is
only hidden `OnEnter(MainMenu)` via `hide_hud_chrome`). `scenario_elapsed` stops
ticking under the outcome pause, so the readout's last synced value simply holds.
The frozen TIME readout therefore shows the final time behind the banner, and the
banner text only varies the CLEAN-RUN line. No `{variable}` interpolation was
added to Outcome - the readout carries the number, keeping the engine addition
minimal. CAVEAT for a reviewer: the readout sits top-center under the scrim, so
the final time is DIMMED (~40% brightness), not full-brightness. If a playtest
finds that too faint, the follow-up is minimal `{variable}` interpolation in the
Outcome message - deliberately deferred, not built, to keep the addition minimal.

Clean-run mechanism (reuses existing vocabulary, no engine): `crash` seeded 0 in
OnStart; three hazard `CreateScenarioArea` sensor spheres (a touch wider than the
tightest rocks + the gravity well) each `OnEnter(player)` do `VariableSet(crash =
crash + 1)`, gated `gate < 8` so a post-finish graze cannot un-clean a win. At
FINISH the bookkeeping handler bumps `gate` to 8, then in the SAME event pulse
two `gate == 8`-and-`crash`-gated Victory handlers evaluate: `crash == 0` ->
CLEAN RUN banner, `crash > 0` -> plain finish. bcs `queue_system` runs handlers
sequentially and each filter reads live state, so the bookkeeping's `gate = 8`
write is visible to the two victory handlers registered after it.

Test/verify results (all green): `content lint` 0 errors (gauntlet clean; the 1
pre-existing warning/finding is the-ledger, acked); `cargo test -p nova_assets
--test gauntlet_course` 12/12; `nova_scenario` hud_readout tests 2/2;
`nova_gameplay` readout tests 3/3; `cargo build -p nova_scenario -p nova_gameplay
-p nova_assets` clean; `cargo doc` warning-free (no missing_docs on the new
public items); portal regenerates (`gauntlet 1.3.0`); `web` `npm run ci` green.

PENDING USER CONFIRMATION (manual playtest - I cannot judge this):

- FEEL / BALANCE: is the time-trial actually FUN to re-fly, and is the CLEAN-RUN
  bar tuned right? The graze-zone radii (16/18/30u) are a first guess - they may
  be too punishing (a clean run near-impossible) or too lax (every run clean).
  Whether the frozen dimmed readout reads clearly enough behind the Victory scrim
  is also a visual judgement only a human can make. NOT self-ticked.
