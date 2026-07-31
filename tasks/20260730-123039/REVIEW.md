# Review: NOVA OS map app: clicks miss their targets where the ship app's land

- TASK: 20260730-123039
- BRANCH: fix/nova-os-map-click-targets

## Round 1

- REVIEWER: out-of-context
- VERDICT: REQUEST_CHANGES

- [x] R1.1 (MAJOR) crates/nova_gameplay/src/hud/nova_os.rs:962 - the
  raster-collapse half of `nova_os_crt_screen_to_image_uv` has no behavioral
  coverage. `nova_os_pointer_mapping_matches_the_crt_shader_across_the_screen`
  only runs `power = 1.0`, where `open_h`/`open_w` are exactly 1 and the remap is
  the identity, so the divide is never exercised: flipping both `/ open_*` to
  `* open_*` leaves all 785 tests green (re-derived in-session). Loop the grid
  test over several powers (e.g. `[0.15, 0.35, 0.65, 1.0]`), comparing against
  `shader_sample_uv_reference` at the same power where the reference stays inside
  `[0,1]` and asserting `None` where it does not.
  - Response: Fixed in f1991c49. The sweep runs powers `[0.15, 0.35, 0.65, 1.0]`
    and asserts pointwise that the pointer calls a point off-picture exactly when
    the reference samples outside `[0,1]`. Two guards keep it from asserting
    vacuously: `on_picture > 0` per power, and `off_picture > 0` iff the raster is
    collapsed - derived from `NOVA_OS_CRT_POWER_OPEN_H/W`, not from `power < 1.0`,
    which is wrong at 0.65 where `open_h` has already reached 1 (my first attempt
    asserted exactly that and failed). Verified by re-running the reviewer's own
    sabotage against the COMMITTED fix: `/ open_*` -> `* open_*` now fails at
    power 0.15. Neutering the collapse guard itself also fails.
- [x] R1.2 (MINOR) crates/nova_gameplay/src/hud/nova_os.rs:893 -
  `forward_nova_os_pointer` reads the openness with `q_openness.single()` while
  `animate_nova_os_crt` (nova_os.rs:2533) reads the same query with
  `iter().next()`. With more than one `NovaOsRootMarker` entity the shader gets
  the first entity's openness and the pointer silently falls back to `1.0` - the
  two-definitions shape this task exists to remove, reintroduced in the very fix
  that removes it. Change line 893 to
  `q_openness.iter().next().map(|o| o.0).unwrap_or(1.0)`.
  - Response: Fixed in f1991c49, exactly as suggested, with the reason recorded at
    the call site.
- [x] R1.3 (MINOR) tasks/20260730-123039/TASK.md:180 - the recorded shader proof
  names `DISPLAY=:99 BCS_AUTOPILOT=1 cargo run --example screenshot_nova_os
  --features debug` and then claims it captured `nova-os-ship.png`. That command
  captures nothing: the capture is gated on `BCS_REEL`
  (examples/screenshots/screenshot_nova_os.rs:234). Two commands were actually
  run and the close-out conflated them. Correct the line to name the
  `NOVA_SHOT_DIR=target/reel BCS_REEL=1` capture form alongside the smoke form.
  - Response: Fixed in f1991c49. Both commands are now recorded separately with
    what each one produces, plus the probe verdict and its SKIPPED (not measured)
    checks.

Verified this round: `cargo fmt --all --check` clean, `cargo check --workspace
--all-targets` clean, `cargo test -p nova_gameplay --lib` 785 pass / 1
pre-existing ignore. The recorded corner miss was recomputed by hand from the two
mappings (0.0212 uv = 27.1 px x / 15.3 px y at 1280x720) and matches. Both
recorded sabotages reproduce: reverting the mapping fails 5 named tests,
reverting both label offsets fails 2. No test was deleted or weakened - the only
removal is the now-dead `nova_os_inverse_barrel`. The WGSL uniform's new
`overscan` field is appended last on both sides with no alignment hole, and the
shader was compiled for real by running the example under Xvfb (clean exit, no
wgpu/naga errors). All 7 `Button` sites under `hud/nova_os*.rs` were swept by
hand: none leaves a bare `Text` as its only hit target, so DoD 4 holds.

Not raised: there is no DECISION.md for moving the overscan into the uniform, but
NOTES.md carries the alternatives and the rationale, which covers a cold reader.
The deliberate non-mirroring of the degauss shear is documented on the helper and
read as an accepted tradeoff, not a gap.

Pending USER check (not resolved by any verdict):

- DoD 5, `manual:` - the owner clicks map contacts as reliably as ship sections.

## Round 2

- REVIEWER: out-of-context
- VERDICT: APPROVE

All three round-1 findings verified RESOLVED by a reviewer with no sight of the
fixes' reasoning. R1.1 was re-checked by applying the original sabotage
(`/ open_*` -> `* open_*`) against the committed fix and watching the sweep go
red; R1.2 by confirming no `single()` remains on that query and that the pointer
rig, which spawns no `NovaOsRootMarker`, still takes the `1.0` fallback so no
existing test shifted; R1.3 against the `BCS_REEL` gate in the example.

New findings, both introduced by the round-1 fix:

- [x] R2.1 (MINOR) crates/nova_gameplay/src/hud/nova_os.rs:8158 - the round-1
  sweep's `reference_on_picture` models only the shader's `in_bounds` gate, not
  its `collapsed` one. The fragment multiplies its output by BOTH
  (`rgb = (... ) * in_bounds * (1.0 - collapsed)`, nova_os_crt.wgsl:222) and
  neither implies the other: barrel-then-overscan is a net contraction here
  (0.93 against a barrel factor under 1.06), so a `cx` just past 1 still lands
  inside `[0,1]` after warping. The production helper correctly returns `None` on
  the earlier collapse test, so the two disagree across a real band - 186 grid
  points at power 0.15 and 814 at 0.35 on a 201x201 grid, and 0 at the 17x17 the
  test actually samples. The assertion passes only by luck of the sampling, and
  any grid refinement or power change turns it red against CORRECT code. Have the
  reference return the collapse flag too (or recompute `cx`/`cy` in the test) and
  compare against `!collapsed && in_bounds`.
  - Response: Fixed in 7de17b7c. Added `shader_draws_at`, which applies both
    gates and returns `Option<Vec2>` - the shader's whole "is anything drawn
    here, and from which texel" answer - and the sweep now compares `ours`
    against it directly. `crt_uv_grid` went from 17x17 to 201x201 so the
    divergence band is actually sampled rather than stepped over. Re-derived the
    numbers independently before accepting: 186 / 814 disagreeing points at
    powers 0.15 / 0.35 on 201x201, 0 at 17x17, exactly as reported. Verified the
    corrected test is genuinely stronger by dropping the collapse gate back out
    of the reference - it now fails at power 0.15, screen uv (0.38, 0.43).
- [x] R2.2 (NIT) crates/nova_gameplay/src/hud/nova_os.rs:8139 - the coverage
  comment says power 0.35 "is squeezed horizontally only"; it is squeezed
  VERTICALLY only (`open_w = smoothstep(0, 0.28, 0.35) = 1`, `open_h = 0.5576`).
  Swap the axis word so the comment matches the branch that power covers.
  - Response: Fixed in 7de17b7c.

Also re-verified this round: `cargo fmt --all --check` clean, `cargo check
--workspace --all-targets` clean, suite 785 pass / 1 pre-existing ignore, every
DoD test present and named as claimed, and the recorded probe re-run under Xvfb
reproducing the close-out's exact verdict (OK, 2/6 measured, four SKIPPED) - which
also re-proves the WGSL compiles.

## Round 3

- REVIEWER: out-of-context
- VERDICT: APPROVE

Both round-2 findings verified RESOLVED by a reviewer with no sight of the fixes.
R2.1 was re-derived from the WGSL fragment by hand (the two gates, the inclusive
bounds, the `max(_, 0.0008)` floor), then double-sabotaged: dropping `!collapsed`
turns the sweep red, and dropping it WITH `STEPS` back at 16 turns it green again
- so the grid refinement is load-bearing, not decoration. R2.2 was checked by
recomputing both smoothsteps at power 0.35.

The reviewer also re-derived the independence question this rig lives or dies on:
`shader_sample_uv_reference` / `shader_draws_at` share no code with
`nova_os_crt_screen_to_image_uv`, which carries its own `nova_os_smoothstep`, its
own named edge constants and its own `cmplt`/`cmpgt` bound tests - so a wrong
production helper still fails against the reference, as every sabotage run
demonstrates. Grid cost is a non-issue: 4 powers x 40401 points reports 0.00s in
the optimized test profile, whole lib suite 8.7s.

One finding, on the round-2 fix:

- [x] R3.1 (NIT) crates/nova_gameplay/src/hud/nova_os_pointer_rig.rs:100 -
  `shader_draws_at` re-declared the `smoothstep` closure and recomputed `cx`/`cy`,
  duplicating the same lines in `shader_sample_uv_reference`. A future shader
  change to an edge (0.65/0.28) or the epsilon could update one copy and leave the
  other stale, and the two would then disagree about the collapse band while still
  looking self-consistent. Transcribe the remap once and have both read it.
  - Response: Fixed in 1b5397f7 - both now call `shader_collapse_remap`, which
    carries the WGSL lines in its doc comment. Worth taking rather than waving
    off as a NIT: it is precisely the two-definitions shape this whole task
    exists to remove, one level down in the rig. Re-ran both sabotages after the
    refactor to confirm the pin did not soften - dropping the collapse gate from
    the reference fails, and the production remap's `/` -> `*` fails. Also fixed
    a module-header pointer naming a path the reference never lived at.

Verified this round: `cargo fmt --all --check` clean, `cargo check --workspace
--all-targets` clean, suite 785 pass / 1 pre-existing ignore.
