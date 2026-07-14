# Review: RON scenario/mod format + built-in port

- TASK: 20260525-133028 (family: 133029, 083326, 103622, 091336)
- BRANCH: modding-language (12 commits ahead of master)

Reviewed out-of-context with three independent fresh-eyes agents (correctness,
tests, design) plus implementer re-verification of two load-bearing claims
(parity test is a genuine non-circular drift guard; the Binding<->BindingInput
round-trip is faithful). Correctness: clean, no findings. Design: sound, crate
boundary faithful to spike 091336. Tests: strong, one real coverage gap.

## Round 1

- VERDICT: REQUEST_CHANGES

- [x] R1.1 (MAJOR) crates/nova_scenario/src/actions.rs:~1153 - the `ScatterObjects`
  *action* spawn loop has no headless test. Only `ScatterRegion::sample` is unit-
  tested in isolation; the loop that clones the template `count` times, sets ids,
  samples positions, randomizes asteroid radius, and spawns is exercised solely by
  the one non-asserting windowed example (`12_menu_newgame`). asteroid_field /
  asteroid_next runtime is likewise unverified. Add a `NovaEventWorld` test (mirror
  `despawn_action_removes_the_scoped_object_by_id`): fire a `ScatterObjectsConfig`
  with a small count into a `NovaEventWorld`, drain the queue, and assert `count`
  scoped entities spawned with in-bounds transforms and the expected id prefix.
  - Response: Added `scatter_action_spawns_count_objects_in_region` - fires an
    8-count Box scatter into a `NovaEventWorld`, drains via `state_to_world_system`,
    and asserts exactly 8 `AsteroidMarker` entities spawn with positions inside the
    region, radii in `[1,3]`, ids under `rock_`, and unique (no collision). Passes
    feature-on and off. Verified.
- [x] R1.2 (MINOR) crates/nova_scenario/src/actions.rs:~740 - the scatter RON
  round-trip test asserts only `id_prefix`/`count`/`seed`/`asteroid_radius`, never
  `region` or `template.kind` - the nested-enum fields most likely to regress. It
  would pass even if `region` deserialized to the wrong variant. Assert `back.region`
  (Ring with its four values) and the template asteroid's texture path survive.
  - Response: Strengthened - the round-trip now matches `back.region` against the
    exact `Ring` values and asserts the template asteroid's `texture.path()`
    round-trips, panicking on a variant change. Verified.
- [x] R1.3 (MINOR) docs/modding-ron-format.md - "the engine's default build stays
  serde-free" overclaims. Because `nova_modding` enables `nova_scenario/serde`
  unconditionally and the game depends (via `nova_assets`) on `nova_modding`, Cargo
  feature unification turns `bevy/serialize` on for the shipped binary, not just
  tests. True only for a crate in isolation (`cargo build -p nova_scenario`). Add a
  one-line note that the shipped binary does enable `bevy/serialize` (cheap, but
  real) since the loader is always present.
  - Response: Doc corrected - the bullet now distinguishes per-crate isolation from
    the shipped binary and notes the latter pays for `bevy/serialize` (small, real,
    at runtime). Done.
- [x] R1.4 (MINOR) tasks/20260714-083326/TASK.md - the body still frames Tier 2
  (ship/section/binding serialization) as a "follow-on task," but the code delivers
  it (`shakedown_run.scenario.ron` exists, ships + bindings serialize). Reconcile the
  body so a future reader isn't told ship porting is pending.
  - Response: Added an "Outcome" section recording BOTH tiers shipped (AssetRef
    covers all asset kinds; BindingInput handles Tier 2 bindings; all built-ins
    ported), superseding the "tier 2 as follow-on" recommendation. Done.
- [ ] R1.5 (NIT) crates/nova_assets/tests/scenario_ron_parity.rs:38-43 - the
  write-on-missing + `continue` branch means a `git rm` of a committed RON that lands
  together with a broken builder regenerates the broken file and passes silently.
  Deliberate regeneration workflow, but consider failing in CI when an expected file
  is missing rather than writing it. Low risk (files are committed).
  - Response: WON'T-FIX (accepted). The write-on-missing branch IS the generation
    workflow (delete + re-run regenerates); the files are committed and the demo test
    would fail to load a missing scenario anyway. Left as-is; the duplication spike
    (110502) reworks this authoring layer wholesale.
- [x] R1.6 (NIT) crates/nova_assets/src/scenario.rs - `shakedown_run_is_registered`
  (and the `dummy_assets`/`real_sections` helpers) were deleted. This is NOT a
  weaken-to-pass: the contract ("New Game's hardcoded `shakedown_run` is registered")
  is now covered better by `demo_scenario.rs` through the real RON loader. Acceptable;
  noting the deletion was intentional and re-covered.
  - Response: Confirmed intentional - the contract is re-covered (and strengthened,
    real loader vs dummy handles) by `demo_scenario.rs`. No change needed.

## Round 2

- VERDICT: APPROVE

All R1 BLOCKER/MAJOR/MINOR findings resolved and verified (R1.1 test added and
passing; R1.2 test strengthened; R1.3 doc corrected; R1.4 task reconciled). R1.5 is
a won't-fix nit (accepted, superseded by the 110502 spike); R1.6 needed no change.
No new problems introduced by the round-2 changes (test-and-docs only; workspace and
`nova_scenario` tests green feature-on and off). Branch is approved; merging is the
user's call.
