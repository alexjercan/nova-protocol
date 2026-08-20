# Phase B2: `stress_point_defense`, the targeting / point-defense case

Fills the biggest hole Phase A found: **targeting and point defense are
exercised by NOTHING** in the 34-subject coverage table. It is also meant to be
the low-variance instrument for candidate 3, the projectile broad phase
(`collect_collision_pairs<ProjectileHooks>`, 59.87 ms in the 4v4 trace).

**STATUS: MEASURED on a quiet box. It is the best instrument in the suite, and
the candidate it was built for is REJECTED on it.**

- **Smallest detectable improvement 6%**, against the 4v4's 46% and
  `broadside`'s 27% - but only with `NOVA_PERF_MAX_DELTA=0.015625` pinned.
  Free-running it is 39%, and the reason is one capture in eight sitting in the
  fixed-step clamp.
- **`collect_collision_pairs<ProjectileHooks>` is 0.192 ms/frame, 0.7%** of a
  26 ms traced frame carrying 2,256 colliders. Tripling the mounts triples the
  BVH and does not move the frame at all.
- **The frame is `PrepareAssets`**, 45% of it, and every millisecond of that is
  one extended material per live TORPEDO. The cost scales with
  `NOVA_STRESS_PD_BAYS` and is flat in `NOVA_STRESS_PD_MOUNTS`.
- **The range was NOT green.** Its rounds floor was drawn from the contended
  reading below and fails about two runs in five on a quiet host, the probe
  clean pass included. Fixed here.

See "What was measured" at the bottom for the numbers and the protocol.

## What the case does

Two hulls, both authored by the range, and no live fight anywhere in it.

- **The battery**, at the origin: a PLAYER hull, a spine of `reinforced_hull_section`
  cells with a `basic_controller_section` in the middle, one
  `pdc_kinetic_turret_section` standing on each cell. `infinite_ammo`, and
  **no input bindings at all**. That is the point: with nothing locked and
  nothing raised, every mount resolves to `MountAuthority::FlightComputer`, so
  the chain under measurement is the computer's own - ownership, per-turret
  assignment, aim, trigger - and not a held player trigger.
- **The launcher**, 220 u downrange: a wall of `torpedo_section` bays with
  `SpaceshipController::None` and `Allegiance::Enemy`. Controller-less on
  purpose - an AI would decide when to shoot, how to maneuver and what to shoot
  at, and every one of those is run-to-run drift. The range holds
  `TorpedoSectionInput` down itself (an unpiloted hull carries no `WeaponsHot`,
  so the safety never touches it) and strips the six-round magazine off each bay
  at build time, because `infinite_ammo` is a player-side cheat and this hull
  has no player.

The subsystems this actually loads, and why:

| subsystem | what makes it work |
|---|---|
| targeting scanner (`collect_lockable`) | walks every lockable body every frame, per player ship; the live torpedo population is its input size (turret bullets are excluded from it by construction) |
| PD threat list (`update_turret_point_defense`) | rebuilt per hull per frame, `O(ships x torpedoes)`, then sorted by time-to-impact |
| PD assignment | two passes over every turret, each testing arcs against the threat list: `O(turrets x threats)` |
| PD aim + trigger | one mount per assignment, and every mount that bears fires |
| projectile broad phase | the rounds those mounts spend, which is where the bodies are |

### The torpedoes are aimed PAST the hull, not at it

The one design decision worth arguing with. Every lane crosses the defender's
transverse plane on a ring between `GATE_NEAR` (60 u) and `GATE_FAR` (110 u) -
inside the 150 u point-defence envelope for the whole crossing, and far enough
out that the 30 u warhead cannot reach the hull - then fuzes `LANE_OVERRUN`
past it.

The alternative, homing the stream onto the defender, is the honest PD scene and
it was rejected: a leaker severs a section, and from that frame on the window is
measuring a different ship. That is exactly the run-to-run drift Phase A named as
the reason `wfc_arena` 4v4 cannot resolve better than 46%. The battery still gets
a real crossing target, still solves a real lead, and still shoots torpedoes down;
what it does not get is the ability to change its own scene.

Consequence worth stating: this case measures point defense ENGAGING, not point
defense DEFENDING. A leaker's damage path is out of scope here and stays covered
by `system_borrowed_battery` and `bug_neutralized_quiet`.

### Determinism

- **No RNG in the layout at all.** The lane is a pure function of the tube the
  torpedo left: the tube's offset across the wall picks the gate's bearing, its
  offset up the wall picks the gate's radius. Nothing draws from `bevy_rand` and
  nothing draws from `rand::rng()` (Nova rule 4 is satisfied by not needing a
  seed rather than by pinning one).
- **One residual source remains**, and it is inside the subject: a torpedo the
  battery kills loses a body section through the normal health pipeline, and
  `detach_destroyed_body` draws from the global `WyRand` for debris scatter.
  `nova_gameplay` seeds that with `EntropyPlugin::<WyRand>::default()` (OS
  entropy), so the debris pose differs between runs. Not reseeded: the root
  package does not depend on `bevy_rand`, and the effect is on where a few
  fragments go, not on how many bodies exist. If the measured spread turns out
  to be worse than the 4v4's, this is the first thing to rule out.
- **The window is bounded by the script**, not by whatever the scene happened to
  be doing. The capture is gated on the SATURATED scene
  (`ready_frametime(envelope_is_full)`), so the warm-up starts when the envelope
  is full rather than at `Playing` - a fixed warm-up from `Playing` would buy an
  arbitrary slice of the fill ramp, which is the coupling Phase A found in the
  4v4. The hold is then a FRAME count, not a duration, and it is longer when a
  capture is armed (`CAPTURE_HOLD_FRAMES` 1600 against `HOLD_FRAMES` 120), so
  the 180 + 900 window closes inside one hold and never spans a drain, a
  teardown or a reload.

## The knobs, and which one drives which cost

Two authored, one derived. Both authored knobs are overridable per run
(`NOVA_STRESS_PD_MOUNTS`, `NOVA_STRESS_PD_BAYS`) so one build sweeps without
moving what the range asserts - the floors are all per-mount or per-bay.

| knob | default | what it drives |
|---|--:|---|
| `DEFENSE_MOUNTS` | 12 | the assignment's turret loop, and through it the ROUNDS in the sky |
| `TORPEDO_BAYS` | 12 | the launch rate, and through it the live TORPEDO population - the scanner's and the threat list's input size |
| rounds / colliders | derived | recorded per cycle as marker payloads; what the broad phase actually indexes |

**The knob is BODIES, not meshes.** What `collect_collision_pairs` costs is
driven by how many entries are in the BVH, which is one per collider and has
nothing to do with how many distinct meshes those entities share (the 4v4's
9,900 `Mesh3d` entities resolve to 600 distinct meshes; that ratio is a render
fact and does not touch the broad phase). So the census the range records is
`With<Collider>`, and the assertion beside it is a ROUND count. On the first
functional run the two were 2179 and 1978: **91% of every body in the BVH was a
point-defence round**, which is exactly the isolation the candidate needs. Change
the broad phase and this number is what moves.

> **CORRECTION, this page, 2026-08-20.** The isolation is real and the census is
> the right census. What does not follow is that the FRAME moves with it. Swept,
> `NOVA_STRESS_PD_MOUNTS` 4 -> 12 takes the BVH from 859 colliders to 2,309 and
> takes `min_ms` from 18.06 ms to 17.09 - down, not up. The knob that moves the
> frame is `NOVA_STRESS_PD_BAYS`, and what it moves is the render world.
> Section "What was measured", part 4.

## What it asserts

Eight named claims, each a `probe_marker` reading `outcome: <slug>` beside its
assert, all eight on the `stress_point_defense` roster in
`crates/nova_probe_cli/tests/catalog_drift.rs` (`SYSTEMS_INVARIANTS` 136 -> 144).
Not one of them is a millisecond.

| slug | what it pins |
|---|---|
| both hulls stood up whole | exact section, mount and bay counts on exactly two roots |
| the computer took every mount | every mount at `MountAuthority::FlightComputer` - the precondition for measuring the right chain |
| the envelope filled with inbound ordnance | peak torpedoes inside `PD_ENVELOPE`, per bay |
| the battery was working the stream | peak mounts holding an assignment |
| the battery shot torpedoes down | intercepts > 0, counted by an observer on `TorpedoShotDownMarker` |
| the sky filled with point-defense rounds | peak live rounds, per mount - the broad-phase load claim, with the collider census as its payload |
| the sky drained to nothing | zero torpedoes and zero rounds BEFORE the teardown, on their own fuzes and lifetimes |
| the teardown left nothing behind | nothing survives `UnloadScenario` |

"The battery shot torpedoes down" is the one that stops the case quietly
measuring two thirds of itself: aim and trigger can both run forever without
ever connecting, and a range that only aimed would read exactly like one that
worked.

## Should it be a `probe scenario` subject?

**No.** `probe scenario` measures the GAME BINARY against a scenario id or a RON
path, and this range is not content: its scenario is built in Rust, is `hidden`,
and - decisively - two of its systems (the tube hold and the lane commit) are the
example's own. The game binary does not carry them, so the same id loaded through
`probe scenario` would stand up two hulls and then do nothing. It stays an
example subject, reached as `probe run stress_point_defense`.

## Tree changes

| file | change |
|---|---|
| `examples/systems/stress_point_defense.rs` | new |
| `Cargo.toml` | the `[[example]]` block (auto-discovery is off) |
| `crates/nova_probe_cli/tests/catalog_drift.rs` | the roster's eight slugs, `SYSTEMS_INVARIANTS` 136 -> 144 |
| `crates/nova_ship/src/sections/torpedo_section/mod.rs` | export `TorpedoShotDownMarker` through the module prelude - the intercept observer is an out-of-crate consumer and had no way to name it |
| `docs/development.md` | the `systems/` roll-call names the new range and its two sweep knobs |

The measuring lane then changed the range itself - `ROUNDS_PER_MOUNT` 100 -> 40,
the `INBOUND_PER_BAY` doc, and the new `NOVA_STRESS_PD_VIEW` knob with
`point_the_camera`. See "What was measured", part 6.

No gameplay system was added, moved or reordered. No CHANGELOG entry: a new
range is neither player- nor modder-visible.

## Checks

All green, in the sprout, at the settled tree:

- `cargo check --workspace --all-targets`
- `cargo fmt --all -- --check`
- `cargo test -p nova_probe_cli --test catalog_drift` (both tests)
- `cargo test --lib -p nova_ship` (677 passed)
- `CLIPPY_CONF_DIR=ci/wasm-clippy cargo clippy --workspace --exclude nova_probe_cli --target wasm32-unknown-unknown -- -D warnings`
- one live autopilot run, `NOVA_AUTOPILOT=1` under Xvfb, exit 0, clean log

Re-run by the measuring lane after part 6's changes, on the REAL display:

- `cargo check --features debug --example stress_point_defense`
- `cargo fmt --all -- --check`
- `cargo test -p nova_probe_cli --test catalog_drift` (both tests)
- unarmed autopilot run on `DISPLAY=:0`, exit 0, 1,428 rounds against the new
  floor of 480
- armed free-running capture, exit 0, 1,959 rounds
- 49 further armed captures across the sweep, view and repeat arms, no gate
  refusal in any of them

## The one reading taken, and why it is not a result

**CONTENDED. Do not cite it.** Taken while two other lanes were running Bevy
binaries and a rustc on the same RTX 3060 Ti; the sibling lane measured the same
4v4 shape at 291 ms against Phase A's 93 ms under that load. It is recorded here
only because it is what settled the scale constants.

> **CORRECTION, this page, 2026-08-20.** The claim that "the COUNTS in it are
> load-independent even though the timings are not" is FALSE, and it is the
> reason the range shipped with a floor a quiet box cannot meet. The ROUND count
> is a function of the host's frame rate: the trigger is decided once per FRAME
> and spent per fixed STEP, so how many rounds a held trigger buys depends on how
> many steps the host runs per frame. Free-running, twelve mounts peak anywhere
> between 708 and 2,425. The 1,978 below is what a 3.4 fps box produces, not what
> the range produces. Only the torpedo counts survive: peak inbound reads 74-86
> at twelve bays whatever the host does.

One cycle, 12 mounts against 12 bays, no capture armed:

| reading | value |
|---|--:|
| mounts up / bays up | 12 / 12 |
| peak inbound inside the envelope | 86 (floor 6/bay = 72) |
| peak mounts holding an assignment | 12 of 12 |
| torpedoes shot down in the cycle | 43 |
| peak point-defence rounds in the sky | 1978 (~165 per mount) |
| peak colliders in the world | 2179 |
| whole script, wall clock | 29.3 s (fill 9.3 s, drain 24.9 s) |

Reading those: the battery saturates (165 of a theoretical 200 rounds per mount,
so nearly every mount is firing nearly all the time), all twelve mounts find work,
and the rounds are 91% of the BVH.

## What was measured

Tree `cbc86980` plus this lane's changes to the range itself (part 6). Host:
RTX 3060 Ti, i9-12900F, NixOS, `dev` profile, vulkan.

**Protocol.** `DISPLAY=:0` - never `xvfb-run`, which adds ~13.7 ms of per-pixel
CPU copy at 720p and is what `notes-floor.md` retracted a headline over. Every
capture wears `WM_CLASS` class `nova-measure` and is moved to i3 workspace 3 by
an IPC `window::new` watcher matching on CLASS (the instance is empty, and
`for_window` is config-only on 4.25.1). Focus never moves, and
`notes-prepare.md` section 1 already showed hidden measures the same as visible.
1280x720 verified in every row, `present=immediate`, `WinitSettings::continuous`,
one capture per PROCESS, window 180 warm-up + 900 frames opening on
`envelope_is_full`. All three validity gates armed; none fired.

The repeat sets ran 14:23-14:48 with no build on this box since 10:58 and
nothing else running, and both predate the `render-off` lane. Nothing was
discarded for reference drift, because nothing was building.

**What WAS discarded: a whole first sweep, seven captures.** `probe run`'s
profiled pass rebuilds the subject with `debug,trace` and leaves that binary in
`target/debug/examples/`, so the next run driven by hand is a TRACED run wearing
the same path. It is loud once you look - each of those seven wrote a 400-900 MB
`trace-*.json` into the repo root - and silent if you do not. The sweep was
re-taken after an explicit `--features debug` rebuild and only the re-take is
reported here. Every set in this page was checked for stray traces afterwards;
only that one sweep is affected. **Check the binary's feature set, not just its
mtime, after any `probe run`.**

`probe run <subject> --repeat 8` was not used to take the sets. The captures are
the same one-process-each captures it spawns, driven directly so a run the fill
gate stalls (part 4) can be retried on its own instead of losing the set; the
gate and the bootstrap below read the same `frametime.csv` the report reads.
`probe run` WAS used for the traced pass.

Raw rows: `measurements/pd-stress-repeats.csv`, `-sweep.csv`, `-views.csv`,
`-framecost.txt`, `-trace.txt`.

### 1. Is it an instrument? YES - pinned. 6% against 46% and 27%.

Two repeat-8 sets, identical but for the fixed-step ceiling.

| statistic | free-running median | range % | cv | pinned median | range % | cv |
|---|--:|--:|--:|--:|--:|--:|
| mean | 30.146 ms | 15.0% | 5.3% | 32.246 ms | 10.0% | 3.4% |
| p50 | 27.231 ms | 18.1% | 6.1% | 30.127 ms | 12.3% | 4.3% |
| **min** | **17.531 ms** | **9.5%** | **3.0%** | **17.443 ms** | **13.1%** | **3.6%** |
| p99 | 54.254 ms | 239.2% | 59.7% | 63.893 ms | 15.3% | 4.9% |
| worst | 71.080 ms | 366.9% | 82.2% | 83.991 ms | 22.0% | 7.1% |

Smallest detectable improvement, by the scheme `NOTES.md` settled - resample `n`
with replacement, run the 20% gate, read the tail, separate two independent sets
at 1.96 sigma:

| reported statistic | n | tol | free-running | **pinned** |
|---|--:|--:|--:|--:|
| p99, median of admitted | 1 | - | 165% | 13% |
| p99, median of admitted | 5 | 20% | 75% | 8% |
| **p99, median of admitted** | **8** | **20%** | **39%** | **6%** |
| worst frame, median of admitted | 8 | 20% | 59% | 9% |
| mean, median of admitted | 8 | 20% | 9% | 5% |
| `min_ms`, median of admitted | 8 | 20% | 5% | 4% |

**Pinned, on the statistic the metric reports, this case resolves 6% where the
4v4 floors at 46% and `broadside` at 27%.** One capture resolves 13%, which is
better than eight of either. Free-running it resolves 39% - better than the 4v4,
worse than `broadside`.

**What the 39% is.** One capture in eight (`pd-A-5`) sat in the fixed-step clamp
- `fixed_steps max=16` where the other seven never exceed 5 - and read p99
179.71 ms against a set spanning 49.9-58.1. Its mean (31.83) and median (26.82)
are both well inside the 20% band, so the gate admits it. That is the failure
`NOTES.md` recorded on the 4v4, reproduced here: **the gate cannot see a clamp
spiral, because a spiral does not move the middle of the distribution.**
`NOVA_PERF_MAX_DELTA=0.015625` makes it unreachable, and all eight pinned
captures log `fixed_steps min=1 max=1 mean=1.000`.

The pin costs simulation speed: one 15.625 ms step per ~32 ms frame is a world
running at ~54% of wall clock. It is a measurement configuration, not what a
player feels. Every comparative number below is taken under it.

### 2. Free-running, the case's own LOAD moves 2.6x

Peak point-defence rounds in the sky, twelve mounts against twelve bays, same
build, same host, same window:

| set | peak rounds, eight captures | spread |
|---|---|--:|
| free-running | 934, 1044, 1054, 1288, 1302, 1630, 1762, 2425 | **2.6x** |
| pinned | 2236, 2243, 2252, 2253, 2259, 2261, 2274, 2401 | **7.4%** (six inside 1.7%) |

**The mechanism.** The whole point-defence chain - ownership, threat list,
assignment, aim, trigger - runs in `Update`
(`crates/nova_ship/src/input/point_defense/mod.rs:105`). `shoot_spawn_projectile`
runs in `FixedUpdate` (`turret_section/mod.rs:251`). So a trigger held for one
FRAME buys as many ticks of firing as that frame ran fixed STEPS, and free-running
that is a property of the host, not of the range. Pinned at one step per frame
the ratio is 1 by construction and the population is a scene property again.

This is why the range was red (part 6), and it is the one thing a reader has to
carry away about this case: **without the pin the subject is not the same subject
twice.**

### 3. Where the frame goes. Not the broad phase. Not point defense.

`framecost`, pinned set, median over the 36 in-window reports, twelve against
twelve:

| item | ms | share |
|---|--:|--:|
| **frame** | **32.783** | 100% |
| render world | 31.772 | **96.9%** |
| &nbsp;&nbsp;**`PrepareAssets`** | **14.659** | **44.7%** |
| &nbsp;&nbsp;`Prepare` | 10.899 | 33.2% |
| &nbsp;&nbsp;&nbsp;&nbsp;`Prepare/BindGroups` | 8.503 | 25.9% |
| &nbsp;&nbsp;`Render/graph` | 3.968 | 12.1% |
| &nbsp;&nbsp;`Render/submit+present` | 0.218 | 0.7% |
| main world | 19.429 | 59.3% |
| &nbsp;&nbsp;`PostUpdate` | 9.261 | 28.2% |
| &nbsp;&nbsp;`RunFixedMainLoop` | 6.445 | 19.7% |
| &nbsp;&nbsp;**`Update`** | **1.804** | **5.5%** |

The two worlds overlap, so a frame costs about the longer of them. **The render
world is the pacer at 96.9%, and the entire main world - which is where every
Nova system in this range lives - is 40% shorter than it.** Physics could go to
zero and the frame would not move.

The traced pass names the systems (`--features debug,trace`, 110 frames of the
saturated hold at 26.06 ms; tracing inflates uniformly, read the shares):

| ms/frame | share | system |
|--:|--:|---|
| 9.285 | 35.6% | `prepare_erased_assets<MeshMaterial3d<ExtendedMaterial<StandardMaterial, ThrusterExhaustMaterial>>>` |
| 6.327 | 24.3% | `bevy_pbr::material::prepare_material_bind_groups` |
| 5.668 | 21.7% | `bevy_ui::widget::text::text_system` |
| 5.173 | 19.8% | `avian3d::schedule::run_physics_schedule` (whole physics step) |
| 3.490 | 13.4% | `schedule: RenderGraph` |
| 1.571 | 6.0% | command flush of `turret_section::firing::shoot_spawn_projectile` |
| 0.986 | 3.8% | `gpu_preprocess::prepare_preprocess_bind_groups` |
| 0.530 | 2.0% | `narrow_phase::trigger_collision_events` |
| 0.275 | 1.1% | `update_narrow_phase<..., ProjectileHooks>` |
| **0.192** | **0.7%** | **`collect_collision_pairs<ProjectileHooks>`** |
| 0.038 | 0.1% | `targeting::contacts::update_contacts_and_locks` |
| 0.026 | 0.1% | `point_defense::assignment::update_turret_point_defense` |
| 0.021 | 0.1% | `aim::update_turret_target_joints_system` |
| 0.014 | 0.1% | `aim::update_turret_aim_point` |
| 0.017 | 0.1% | `ownership::update_point_defense_{ownership,aim,trigger}` |

**Candidate 3 is REJECTED on this case.** `collect_collision_pairs<ProjectileHooks>`
is 0.192 ms a frame with 2,256 colliders in the world, of which ~91% are rounds.
The 4v4's 59.87 ms is not this system being expensive per body; whatever it is,
this case - which was built to isolate it - says it is not the projectile BVH.

**The chain the range exists to load is 0.15 ms.** Targeting scan, threat list,
assignment, aim and trigger sum under 0.2 ms a frame, and the `Update` schedule
that contains all of them plus everything else is 1.80 ms, 5.5%. The case
succeeds at loading point defense and finds point defense free.

### 4. What the cost scales with: BAYS. Mounts are free.

Six arms, round-robined three times so drift hits every arm equally, pinned,
medians of three. Counts are the range's own peaks and the census taken inside
the window.

| arm | `min_ms` | p50 | mean | rounds | colliders | mesh inst | distinct mats | inbound |
|---|--:|--:|--:|--:|--:|--:|--:|--:|
| m4 b4 | **9.01** | 21.71 | 22.95 | 807 | 876 | 521 | 46 | 28 |
| m4 b12 | **18.06** | 31.09 | 32.21 | 670 | 859 | 596 | 118 | 86 |
| m4 b24 | **31.44** | 42.69 | 44.11 | 743 | 1366 | 988 | 234 | 169 |
| m8 b12 | **18.00** | 30.91 | 32.50 | 1351 | 1545 | 951 | 116 | 82 |
| m12 b12 | **17.09** | 29.62 | 31.62 | 2105 | 2309 | 1607 | 115 | 75 |
| m12 b24 | **31.10** | 43.42 | 44.95 | 2397 | 2773 | 1651 | 224 | 146 |

**Mounts, at twelve bays: 4 -> 8 -> 12.** Rounds 670 -> 1351 -> 2105, colliders
859 -> 1545 -> 2309. `min_ms` 18.06 -> 18.00 -> 17.09. **Three times the BVH,
and the frame goes DOWN 5%.** The marginal cost of a collider here is zero to
within the measurement: 1,450 more bodies for -0.97 ms.

**Bays, at four mounts: 4 -> 12 -> 24.** `min_ms` 9.01 -> 18.06 -> 31.44, a
straight line, `4.52 ms + 1.12 ms per bay`. Repeated at twelve mounts: 17.09 ->
31.10, the same slope. A bay puts ~9.2 torpedoes in the sky, so **0.12 ms a
frame per live torpedo.**

The framecost split says which phase each knob moves:

| arm | frame | `PrepareAssets` | `Prepare/BindGroups` | `RunFixedMainLoop` | `Update` | `Render/graph` |
|---|--:|--:|--:|--:|--:|--:|
| m4 b4 | 21.20 | 7.25 | 5.01 | 3.83 | 1.744 | 3.82 |
| m4 b12 | 32.52 | 15.28 | 9.04 | 4.13 | 1.753 | 3.68 |
| m4 b24 | 43.70 | 22.19 | 13.13 | 4.43 | 1.678 | 3.69 |
| m8 b12 | 31.84 | 14.90 | 8.55 | 4.89 | 1.774 | 3.76 |
| m12 b12 | 31.42 | 13.81 | 8.40 | 5.92 | 1.721 | 3.87 |
| m12 b24 | 44.42 | 22.20 | 12.85 | 6.12 | 1.791 | 4.14 |

- `PrepareAssets` +0.75 ms per BAY, flat in mounts (15.28 / 14.90 / 13.81).
- `Prepare/BindGroups` +0.41 ms per BAY, flat in mounts.
- `RunFixedMainLoop` is the ONLY item that tracks mounts: +0.22 ms per mount,
  3.83 -> 5.92 across three times the bodies. It is inside a main world already
  40% shorter than the render world, so none of it reaches the frame.
- `Update` is 1.68-1.79 ms in **every** arm - four mounts against four bays, and
  twelve against twenty-four with 146 inbound. The assignment's `O(turrets x
  threats)` does not appear.

**What a bay actually buys.** The census origin breakdown, twelve bays:
`Torpedo Controller` is **110 instances, 1 distinct mesh, 110 distinct
materials** - one private material per live torpedo, which is the
`SectionCracksPlugin` per-section material `notes-ablation.md` and
`notes-prepare.md` item 3 already named, now minted per PROJECTILE. Beside it,
`Thruster Exhaust` is 406 instances over 2 meshes, each carrying its own
`ExtendedMaterial<StandardMaterial, ThrusterExhaustMaterial>` - and a torpedo
under guidance genuinely changes its thruster input every frame, so the
read-before-write guard `8a26ae31` added does not fire. That is the 9.285 ms.

Distinct drawn materials by bay count: 46 / 118 / 234 at 4 / 12 / 24 bays, i.e.
9.4 per bay. `min_ms` at 1.12 ms per bay is **0.12 ms per distinct material**,
which sits beside the gallery's 0.082 ms on the same statistic. It is the
distinct-asset law from `notes-prepare.md` section 6, measured on a scene that
is nothing like a frozen gallery.

**One arm is unreachable, and the note previously implied it was not.**
`NOVA_STRESS_PD_MOUNTS=24` against twelve bays STALLS: the battery outguns the
launcher, the envelope never reaches `INBOUND_PER_BAY` and the `open the tubes`
step deadline fires at 90 sim seconds. Measured yield falls from 7.0 inbound a
bay at four mounts to 6.0-6.3 at twelve, against a floor of 6. Sweeping mounts UP
needs bays raised with them.

Twelve mounts against `NOVA_STRESS_PD_BAYS=4` DOES run, but it clears the gate by
one torpedo (peak 25 against a floor of 24) and should be read as a coin flip.
The single capture is worth quoting anyway, because it is the sweep's most
extreme separation of the two axes - and it was taken while the sibling
`render-off` lane held a core at 100%, so only its direction is safe:

| arm | `min_ms` | colliders | distinct mats |
|---|--:|--:|--:|
| m4 b4 (clean, x3) | 9.01 | 876 | 46 |
| **m12 b4** (contended, x1) | **8.36** | **2500** | **37** |

**2.9x the bodies, and the least-contended frame is LOWER**, on a box that was
building at the time. It also carries more rounds than any twelve-bay arm (2,417
against 2,105): with fewer torpedoes to kill, a mount keeps the assignment it was
handed and never stops firing.

### 5. The camera does not cost anything, at 720p or at 1440p

The owner flew a live run and reported that looking at the torpedoes is slower
than having them off camera. Tested. The range's camera is the loader's, at
`(0, 10, 20)` looking at the origin - **fixed and deterministic**, with the
launcher and most of every lane BEHIND it - so it cannot drift a capture; this
lane added `NOVA_STRESS_PD_VIEW` to move it deliberately, and
`point_the_camera` now pins whichever pose the run named.

Three poses, paired and interleaved, three passes, pinned:
`battery` (the default), `lanes` (everything in frame at once), `away` (empty
sky, nothing the range spawns inside the frustum).

| statistic | `lanes` / `battery` | `away` / `battery` |
|---|--:|--:|
| `min_ms`, 1280x720 | 1.067 (0.877-1.142) | 1.033 (0.854-1.102) |
| p50, 1280x720 | 1.096 (0.861-1.243) | 1.057 (0.678-1.586) |
| `min_ms`, 2560x1440 | - | 1.023 (0.854-1.031) |
| p50, 2560x1440 | - | 0.996 (0.992-1.104) |

**Every spread straddles 1.00.** Drawing nothing costs the same as drawing
everything, at four times the pixels as well.

The phase split is tighter than the frame and says the same thing without the
noise: `Render/graph` reads **4.203 / 4.006 / 4.196 ms** for battery / lanes /
away at 720p, and **4.464 / 4.566 ms** for battery / away at 1440p.
`Render/submit+present` at 1440p is 0.248 vs 0.250 ms. Emptying the frustum does
not move the draw phase, and `PrepareAssets` (44-46%) and `Prepare/BindGroups`
(26-27%) hold their shares in all five arms.

**Mechanism, and it is the useful part:** culling removes DRAW work, and this
frame is not draw work. It is asset PREPARATION, which runs over every live
material whether or not anything can see it. So "pixels do not bind" survives
here, for a reason that has nothing to do with fill - and the reason is also
why a torpedo off camera still costs its 0.12 ms.

Caveat, stated plainly: three pairs bound the frame-level effect at about
+/-15%, not tighter. What is excluded is anything large.

### 6. The range was not green, and what was changed

`ROUNDS_PER_MOUNT` was 100, so twelve mounts had to peak at 1,200 rounds. Part 2
is why that cannot hold: free-running, the peak lands anywhere in 708-2,425.
Measured red **three times in the eight-capture free-running set, and in the
probe CLEAN pass at 708** - the pass CI runs, on this host, at this SHA.

Changed here, in `examples/systems/stress_point_defense.rs`:

- `ROUNDS_PER_MOUNT` 100 -> **40**, drawn under the worst measured free-running
  yield (59 a mount) instead of from a theoretical 200. The claim is unchanged
  and its slug is unchanged; 480 rounds is still an unambiguous sky. Verified
  green unarmed (1,428 rounds) and armed free-running (1,959).
- `INBOUND_PER_BAY` left at 6. It is not just an assertion - it opens the
  capture window, and lowering it would start the window before the scene is
  saturated, which is the coupling this range was built to avoid. Its doc now
  records the measured yield and names the two unreachable sweep arms.
- **`NOVA_STRESS_PD_VIEW`** added (`battery` default, `lanes`, `away`) with
  `point_the_camera` holding the pose every frame, so a stray input cannot move
  the subject mid-capture.

### 7. What this rules OUT, with the number

- **The projectile broad phase, as a lever on this case.** 0.192 ms/frame,
  0.7%, at 2,256 colliders. Tripling the BVH via mounts moves `min_ms` by
  -0.97 ms.
- **The targeting scanner and the whole PD chain.** Under 0.2 ms summed; the
  `Update` schedule holding them is 1.68-1.79 ms in every arm of the sweep,
  including 146 inbound against twelve turrets.
- **Mesh instances and entity count.** 521 -> 1,651 instances across the sweep
  with `min_ms` set by bays alone.
- **The camera, and fill.** Part 5. Also 640x360 / 1280x720 / 1920x1080 at
  twelve against twelve: 29.1 / 29.8 / 30.6 ms, 9x the pixels for 5%.
- **Present.** `Render/submit+present` 0.218 ms, 0.7% of the frame, and 0.25 ms
  at 1440p.
- **Fixed-step amplification, once pinned.** `fixed_steps min=1 max=1
  mean=1.000` in all eight pinned captures and all 18 sweep captures.
- **The debris `WyRand`** the page names as the first thing to rule out on a bad
  spread. It is not the spread: pinned, the same scene with the same unseeded
  debris repeats to 1.7% on rounds and 3.6% cv on `min_ms`.

### 8. The ranked list

The frame is 32.8 ms pinned, 31.4 ms free-running, at the authored twelve
against twelve.

| # | lever | measured | kind | note |
|---|---|--:|---|---|
| 1 | **One `ThrusterExhaustMaterial` per live torpedo, re-prepared every frame** | **9.285 ms/frame traced, 35.6%**; `PrepareAssets` 44.7% of the frame and +0.75 ms per bay | presentation | The lead by a distance. Same system `8a26ae31` fixed for the gallery; the guard does not fire because a guided torpedo's thrust input really does change. Share one exhaust material across projectiles, or move the input off the material. |
| 2 | **One private cracks material per live torpedo** | `prepare_material_bind_groups` 6.327 ms traced, 24.3%; `Prepare/BindGroups` 25.9% and +0.41 ms per bay; census 110 materials over 110 instances | presentation | The `notes-prepare.md` item 3 finding, on projectiles. A torpedo that never takes damage does not need its own material. |
| 3 | HUD text layout | `text_system` 5.668 ms traced, 21.7% | presentation | Measured on the gallery at ratio 1.02 and dismissed; on a scene with a live contact list it is a fifth of the frame. Worth its own A/B before anyone acts on it. |
| 4 | A round is an entity with a collider and a rigid body | command flush of `shoot_spawn_projectile` 1.571 ms, 6.0%; `RunFixedMainLoop` +0.22 ms per mount | **GAMEPLAY** | The epic's rule puts this with the owner. Size it first: it is 6%, and the frame is render-bound, so it buys nothing until 1 and 2 land. |
| 5 | `collect_collision_pairs<ProjectileHooks>` | 0.192 ms, 0.7% | gameplay | Rejected. Do not spend a task on it from this evidence. |

1 and 2 are one finding twice: **a torpedo introduces about 9.4 distinct
materials and costs 0.12 ms a frame for as long as it is alive, whether or not
anything is looking at it.**

### 9. What is owed now

- **The CI budget question is UNRUN.** This range has still never run under
  `--render sw`. It was not skipped for time: the box was needed for the
  variance figure and then for the sweep, and a lavapipe run is a different
  contention profile that would have to wait for both. The drain is still the
  long pole (24.9 s of 29.3 s contended; 12.3 s of 28.1 s measured unarmed here,
  so the whole unarmed script is ~28 s on the GPU). `stress_torpedoes` is
  documented at 48 s under lavapipe. Run it before anyone trusts the 180 s
  process timeout.
- **`INBOUND_PER_BAY` bounds the mounts knob.** Raising bays with mounts is the
  workaround; whether the gate should be a fraction of the STEADY population
  rather than a fixed per-bay count is an open design question, and it is not
  free - it is what starts the window on a saturated scene.
- **`text_system` at 21.7%** wants the same treatment `notes-prepare.md` gave
  the HUD on the gallery: an interleaved A/B against `HudVisibility::Cinematic`.
  It is a fifth of the frame here and it was ruled out on a scene that had no
  contacts in it.
- **The dev book's `systems/` roll-call names two sweep knobs; there are three.**
  `docs/` is out of this lane's scope, so `NOVA_STRESS_PD_VIEW` is undocumented
  there. One line, on whoever next lands in `docs/development.md`.
