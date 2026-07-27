# Retro: NOVA OS CRT degauss + micro-effects polish

- TASK: 20260727-014148
- BRANCH: feature/nova-os-crt-degauss
- REVIEW ROUNDS: 1 (out-of-context, APPROVE)

Process observations only; what/why/evidence live in TASK.md + NOTES.md.

## What went well

- Understanding-first paid off: reading the parent RTT task (20260726-193233),
  its NOTES micro-effect inventory, and the actual `sync_nova_os_app_ui` /
  `animate_nova_os_crt` code BEFORE planning meant the degauss trigger landed at
  the one correct seam (past the diff-guard, where the coil sound already fires)
  with no false starts on placement.
- The deliberate omissions (vertical-hold jitter, phosphor-strike) were named in
  the plan and NOTES rather than silently dropped - the reviewer confirmed the
  scope matched, no "where is effect X" round.
- The new test is deterministic by construction (`run_system_once` +
  hand-advanced `Time<Real>`), so the decay assertions do not depend on the wall
  clock. Out-of-context reviewer verified it would fail with the trigger, decay,
  or settle removed - a meaningful pin, APPROVE round 1.
- Caught the stale DoD test filter (`drawer` -> `nova_os`, matched 0 tests after
  the rename) at the plan gate, not after a green-but-empty run.

## What went wrong

- Adding the required `ResMut<NovaOsDegauss>` param to `animate_nova_os_crt` and
  `sync_nova_os_app_ui` broke FIVE pre-existing unit tests (panic: resource
  absent). Root cause: I added the param and ran the suite before sweeping the
  test rigs that add those systems. The panic deferred a call-site sweep to
  test-run time that a grep would have surfaced instantly.
- The new test then panicked in the AssetServer: I hand-built its rig with
  AssetPlugin (needed for `Assets<NovaOsCrtMaterial>`) but forgot the
  `init_asset::<Font>()`/`<Image>()` the sibling `chin_controls_app` rig already
  has - the app-launch spawn loads a font. Root cause: reconstructed the rig from
  what I thought it needed instead of copying the nearest passing sibling whole.
  This is `reuse-known-good-stack` (x6, pending) hitting a 7th time.

## What to improve next time

- When a widely-run system gains a required `Res`/`ResMut`/param, immediately
  `grep` every `add_systems(.., <system>)` / test rig and register the resource
  BEFORE running - the compiler will not catch a missing resource, only a
  run-time panic will.
- Scaffold a new headless test rig by copying the nearest passing sibling rig
  verbatim, then mutate - do not reconstruct its plugin/asset registration from
  the system signature.

## Action items

- [x] Bumped `reuse-known-good-stack` (-> x7) in LESSONS.md.
- [x] Added `new-required-system-param-sweeps-all-rigs` lesson (x1) to LESSONS.md.
- No follow-up code tasks: the aesthetic tuning is owner manual acceptance, not a
  deferred code item.
