# Retro: Gravity wells - bounded one-way gravity with sphere of influence

- TASK: 20260709-193338
- BRANCH: gravity-wells (squash-merged to master as 1378bee)
- REVIEW ROUNDS: 2 (round 1 REQUEST_CHANGES with 2 MAJOR, round 2 APPROVE
  with one in-round MINOR on the new debug overlay)

First half of the v0.5.0 gravity arc (spike 20260709-193147); the ORBIT
verb task builds directly on this substrate.

## What went well

- **Verifying spike claims against code at plan time.** The spike said
  asteroid sources "stay RigidBody::Static"; five minutes in
  base_scenario_object showed every scenario object is Dynamic. Catching
  that during planning turned a false premise into an explicit design
  question, and review round 1 resolved it the right way (wells go on
  rails at designation) instead of the code silently relying on a wrong
  assumption.
- **Reading the substrate API before implementing the plan's sketch.** The
  plan said "apply mass * a * dt via ComputedMass"; the avian 0.7 source
  showed Forces::apply_linear_acceleration - mass-independent, per-step,
  exactly gravity. One less query dependency and no dt handling, because
  the API was checked before the first line of the force system.
- **The adversarial review pass earned its MAJORs.** An independent
  fresh-eyes review (against the diff, the spike, and the avian source)
  found the two real problems the implementation missed: dynamic well
  sources and the dangling DominantWell after a destructible well dies -
  both exactly on the surface the next task (ORBIT) consumes. Reviewing
  the public-API contract from the consumer's seat, not the producer's,
  is what surfaced them.
- **Mid-flow user requests folded in cleanly.** The F11 debug overlay
  (SOI + core spheres, dominant-well links) was requested mid-cycle,
  landed as a step on the same branch, got its own review finding (missing
  init_resource) fixed in-round, and paid off immediately: the user's
  playtest that judged the SOI too small was done with the overlay's
  spheres visible.

## What went wrong

- **The merge landed with all six examples broken.** `cargo check
  --workspace` does not build example targets, so adding a required field
  to AsteroidConfig compiled clean everywhere the check looked while every
  `examples/*.rs` initializer was missing it. Diagnostics only surfaced
  after the squash-merge; fixed on master in 4c15870. Root cause: the
  verify step's definition of "the workspace compiles" was narrower than
  the code that actually ships from this repo.
- **A pure-tuning default needed a same-day playtest correction.** The
  SOI factor shipped at 4.0 straight from the spike's sanity math; the
  first real flyby showed the pull is only felt almost on the rock, and it
  went to 8.0 (29962bc). Not avoidable by more analysis - inverse-square
  reach is a feel question - but the retune cost nothing because strength
  and reach were separate knobs. Authoring tunables so playtest feedback
  maps to exactly one knob is what made this cheap.

## What to improve next time

- **When a struct gains a required field, grep the whole repo - including
  examples/ - for initializers before calling the change done**, and run
  `cargo check --workspace --examples` (or add `--all-targets`) as the
  standard check when a public config struct changes shape.
- Keep reviewing new public components from the consumer task's
  perspective; both MAJORs this cycle were invisible from inside the
  producing system.

## Action items

- [x] Examples fixed and the wider check run recorded here (4c15870).
- [ ] Playtest knobs to watch in the ORBIT task: soi_factor 8.0 (does the
  bigger reach make well handoffs noisy in dense fields?), Gravity Rock
  surface_gravity 3.0, and whether the debug overlay's core sphere matches
  where orbits actually feel stable.
- [x] Checked CI: it already covers examples (clippy --all-targets, and
  cargo test builds example binaries), so the escape was local-only - the
  lesson is for the local verify step, not the workflow.
