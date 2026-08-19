# Phase B2: `stress_point_defense`, the targeting / point-defense case

Fills the biggest hole Phase A found: **targeting and point defense are
exercised by NOTHING** in the 34-subject coverage table. It is also meant to be
the low-variance instrument for candidate 3, the projectile broad phase
(`collect_collision_pairs<ProjectileHooks>`, 59.87 ms in the 4v4 trace).

**STATUS: built and green. NOT YET MEASURED.** The box carried three lanes'
Bevy binaries at once for this whole session, so no frame-time number taken here
is publishable and the smallest-detectable-improvement figure - the deliverable -
is deliberately absent. See "What is still owed" at the bottom.

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

## The one reading taken, and why it is not a result

**CONTENDED. Do not cite it.** Taken while two other lanes were running Bevy
binaries and a rustc on the same RTX 3060 Ti; the sibling lane measured the same
4v4 shape at 291 ms against Phase A's 93 ms under that load. It is recorded here
only because it is what settled the scale constants, and because the COUNTS in it
are load-independent even though the timings are not.

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

## What is still owed

The deliverable - **the smallest detectable improvement on this case** - is a
VARIANCE measurement, and contention inflates variance directly, so deriving it
from a busy box would understate the instrument rather than measure it. Owed, on
a quiet machine:

1. `probe run stress_point_defense --repeat 8` at the settled protocol (20%
   tolerance): mean, median, median admitted p99, and the bootstrap for the
   smallest detectable improvement. Compare against the 4v4's floor of 46% and
   `broadside`'s 27%; if it is not meaningfully better than 46% the case is not
   yet an instrument and this note has to say so plainly.
2. The traced pass, to answer whether
   `collect_collision_pairs<ProjectileHooks>` dominates the frame the way the
   4v4 trace suggests, and what share targeting and the assignment take.
3. The sweep, round-robined so drift hits every arm equally: `NOVA_STRESS_PD_MOUNTS`
   across at least 4 / 12 / 24 at fixed bays, and `NOVA_STRESS_PD_BAYS` across at
   least 4 / 12 / 24 at fixed mounts. That is what turns "the cost is high" into
   "the cost scales with THIS".
4. Consider `NOVA_PERF_MAX_DELTA=0.015625` on the sweep arms (one fixed step per
   frame, verified by the sibling lane to hold `fixed_steps` at exactly 1), since
   the sweep compares configurations rather than answering "what a player feels".

Two scale risks to watch when the measurement is taken, neither settled:

- **CI budget.** The correctness sweep runs `probe run --all --correctness-only`
  under lavapipe with `NOVA_AUTOPILOT_DEADLINE: 170` and a 180 s process timeout.
  This range walked its whole script in 29.3 s on the GPU under contention;
  `stress_torpedoes` is documented at 48 s under lavapipe. It should fit, but it
  has not been run under `--render sw` and it carries roughly twice the projectile
  count of the existing stress ranges. If it times out, `DEFENSE_MOUNTS` is the
  knob to cut - it is the one that sets the round population.
- **The drain is the long pole**, 24.9 s of the 29.3. It waits for the last
  torpedo launched before the tubes closed to fly its whole lane out and fuze.
  Shortening `LAUNCHER_STANDOFF` or `LANE_OVERRUN` buys that back if the budget
  is tight, at the cost of a shallower crossing.
