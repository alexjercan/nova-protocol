# nova_probe: continuous invariant assertions during autopilot runs (health/speed/state-machine bounds)

- STATUS: CLOSED
- PRIORITY: 72
- TAGS: v0.8.0, spike, tooling, testing

## Goal

Continuous invariant assertions during autopilot runs: a set of always-true
checks evaluated while a probe run plays - e.g. health never negative, speed
respects the configured cap, scenario acts/variables move monotonically where
the design says they must, entity counts stay bounded. Violations are recorded
as structured events on the run timeline (and can panic in strict mode);
results feed the correctness section and the `invariants held` auto-check of
the run report.

## Steps

- [x] Add `src/invariants.rs` to nova_probe: `nova_invariants()` preset,
      env-gated on `NOVA_PERF_INVARIANTS` (any value arms record+warn mode;
      the value `strict` panics on first violation; builder overrides
      `.strict(bool)` / `.monotonic([keys])` for tests and examples).
      `InvariantState` resource tracks violation count (T5's `invariants
      held` check reads it); every violation logs a warn AND lands on the
      run timeline as kind `invariant` when the recorder is armed (Option
      resource - invariants work without the recorder too).
- [x] The v1 invariant set, each derived from what the ENGINE guarantees
      (rule-inputs-rederive-from-engine), checked every frame in Last:
      (a) HEALTH BOUNDS: every `Health` has finite current/max and
      `0 <= current <= max` - bcs's on_damage clamps to exactly this
      (bcs health/mod.rs:135), so a violation is a real bug;
      (b) VELOCITY SANITY: every avian `LinearVelocity` finite (NaN = physics
      exploded); when the entity carries `FlightSpeedCap` (a SOFT taper gate,
      flight.rs:2126-2131 - autopilot and gravity legitimately exceed it),
      speed <= cap * SANITY_MULTIPLIER (one tunable const, default 10.0,
      documented as absurdity detection, not cap enforcement);
      (c) SCENARIO VARIABLES: every Number variable finite; REGISTERED
      monotonic keys never decrease (monotonicity is script discipline, not
      a type guarantee - so it is opt-in registration, never inferred);
      (d) ENTITY-COUNT SANITY: total world entities under one generous
      tunable bound (leak detector, default 200_000).
      DELIBERATELY OMITTED in v1: the ship-root == sum(sections) health
      aggregate (transient despawn windows make it schedule-flaky) and
      projectile-lifetime checks (DespawnEntityPlugin's own contract, bcs's
      to test).
- [x] Order the checks in Last BEFORE the recorder's variable-diff +
      run_end chain (expose the recorder's Last systems pub(crate)) so the
      exit frame records violations before the bracket closes; add a
      run-summary `invariant` entry (checks run, violations) next to
      run_end.
- [x] Deps: nova_probe += avian3d 0.7 (LinearVelocity; same pinned version
      as nova_gameplay); import FlightSpeedCap from nova_gameplay::flight.
- [x] Wire the examples: 08_scenario registers monotonic `beat` +
      `rocks_destroyed`; 10_playable registers `target_down` + `leg`; both
      add nova_invariants() next to nova_timeline() (inert without env).
- [x] Tests (would-it-fail-without-it, each check violated in isolation):
      negative/NaN/over-max Health -> violation recorded (+ timeline entry
      when recorder attached); NaN LinearVelocity -> violation; monotonic
      regression (1 -> 0) -> violation while 0 -> 1 -> 2 stays clean; NaN
      scenario variable -> violation; healthy rig -> zero violations;
      strict mode panics (#[should_panic]); unarmed -> no-op.
- [x] E2E healthy-run proof: 10_playable headless with recorder +
      invariants armed -> zero violations on the timeline, run completes.
- [x] Docs: wiki run-timeline section gains the invariants paragraph;
      CHANGELOG Unreleased entry; spike fix record append.
- [x] Verify: fmt; cargo test -p nova_probe; cargo check --workspace
      --all-targets --features debug; wasm check (invariants native-only
      alongside the recorder, stubs match).

## Notes

- Spike: tasks/20260719-112011/SPIKE.md. Chosen over golden timelines in the
  round-1 review adjudication (user, 2026-07-19); goldens deferred to backlog
  task 20260719-112245. Invariants catch always-been-wrong bugs a golden diff
  cannot, and are immune to host timing noise (llvmpipe vs dev GPU).
- Derive invariant bounds from the engine's decision constants (speed caps,
  health floors), not hand-written expected values
  (rule-inputs-rederive-from-engine).
- Depends on the recorder (20260719-112238): violations ride the same
  structured event stream.

## Close-out (2026-07-19, branch feature/probe-invariants)

Shipped `nova_probe::invariants` exactly per Steps: env-gated
(`NOVA_PERF_INVARIANTS`, `strict` panics), the four v1 checks (health
bounds, velocity finiteness + 10x-cap absurdity bound, variable finiteness
+ opt-in monotonic, entity-count leak bound), violations warn + land on the
timeline + tally in InvariantState for T5. Ordered before the recorder's
Last chain. Both examples wired with their design-monotonic variables.

Evidence: 35 nova_probe tests green FIRST run (8 new; each check violated
in isolation, strict panic pinned, unarmed no-op pinned, timeline
integration pinned); workspace all-targets + wasm checks clean; e2e armed
10_playable run completed with ZERO violations over the full 24 s window
(the healthy baseline the report's `invariants held` check will read).

Key decisions (alternatives considered):
- Speed cap as ABSURDITY bound (10x, one const), not enforcement - the
  engine's cap is a soft taper gate (flight.rs:2126); asserting the design
  cap would false-alarm on autopilot/gravity. rule-inputs-rederive-from-
  engine applied.
- Monotonicity strictly OPT-IN per example - the variable system makes no
  direction promise; inferring would fabricate a contract.
- Ship-root health aggregate SKIPPED in v1 (mid-despawn frames make
  root==sum schedule-flaky); revisit only with a despawn-quiescent gate.

Reflection: the surface map (Explore agent) flagged the soft-cap and
no-monotonic-guarantee facts BEFORE design, which prevented the natural
mistake (hard cap assertions that flake). Smooth cycle; no rework.
