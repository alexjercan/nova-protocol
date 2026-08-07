# Code review - nova_gameplay flight, physics, camera, sections, mesh, audio

Source: dedicated reviewer over `nova_gameplay/src/` excluding `hud/` and
`input/`, 2026-08-07. The deepest pass of the six (92 tool calls). Spot-verified.

## Bugs

### 1. Every fired bullet allocates a new Mesh and a new StandardMaterial

`crates/nova_gameplay/src/sections/turret_section/render.rs:129-130`:

```rust
None => {
    commands.entity(entity).insert((children![(
        Name::new("Bullet Projectile Render"),
        Mesh3d(meshes.add(Cuboid::new(0.05, 0.05, 0.3))),
        MeshMaterial3d(materials.add(Color::srgb(1.0, 0.9, 0.2))),
    ),],));
}
```

`insert_projectile_render` fires on `Add, TurretBulletProjectileMarker`.

**This is the production path, not a fallback.** Every shipped turret sets
`projectile_render_mesh: None` - `nova_assets/src/sections.rs:286,360,390` and
`nova_assets/src/scenario/craft.rs:257,275,345`.

The default turret fires at 100 rounds/s per muzzle
(`turret_section/config.rs:235`), so **one held trigger creates 100 mesh assets
and 100 material assets per second**, each forcing a GPU buffer upload and a
fresh material bind group / pipeline specialization. Handles drop on bullet
despawn, so it is churn rather than an unbounded leak, but the per-frame cost
scales with fire rate.

VERIFIED by read. Severity: bug. Fix is a cached handle pair in a resource.

This is almost certainly the largest single performance defect in the review,
and it sits directly under the probe's FPS baseline check.

### 2. One bad child aborts an entire explosion, leaving a live wreck

`crates/nova_gameplay/src/mesh/explode.rs:130` and the twin at `:144` -
a per-mesh failure inside the collect loop `return`s instead of `continue`s.

Failure: an explodable destroyed entity with one child whose `Mesh3d` handle is
still loading, or whose mesh is non-indexed so `explode_mesh` returns `None`,
produces **no `ExplodeFragments` at all** - and the fragments already built for
earlier children are discarded.

`integrity/explode.rs:129` (`despawn_destroyed_without_mesh`) explicitly skips
anything `With<Mesh3d>`, so the mesh-bearing wreck's only despawn path is the
fragment handler. Result: **a zero-health wreck lingers in the world with its
collider live.**

Severity: bug. Certain on the control flow and the missing despawn path;
likely on reaching the `None` arm with shipped content.

### 3. Unvalidated `fire_rate` panics on ship spawn

`crates/nova_gameplay/src/sections/turret_section/setup.rs:64`:

```rust
let interval = 1.0 / muzzle.fire_rate;
let mut timer = Timer::from_seconds(interval, TimerMode::Once);
```

`MuzzleConfig::fire_rate` is a plain required `f32` on the serde-deserialized
turret config. `fire_rate: 0.0` gives `interval = +inf`, and
`Duration::from_secs_f32(inf)` **panics** the moment the ship spawns. Negative
panics the same way.

`nova_scenario/src/lint/ship.rs:128` lints the hinge axis and muzzle presence
but never `fire_rate`. `spawn_turret_joint` has an explicit runtime backstop
for the degenerate axis (`setup.rs:33-42`) but none here.

**The sibling live-retune path already guards it** - `setup.rs:192` uses
`1.0 / muzzle.fire_rate.max(f32::EPSILON)`. VERIFIED: the asymmetry is real.
One path guarded, the other not.

Severity: bug. Reachable from mod content, so this belongs with the
untrusted-input cluster in `11-review-assets-scenario.md`.

`torpedo_section/bay.rs:63` is the same arithmetic without the `Duration`, so
no panic: `fire_rate: 0.0` makes the bay fire once and die silently for the
rest of the scenario; `-1.0` clamps the cooldown to 0 so it launches every
FixedUpdate tick.

### 4. Torpedo terminal guidance uses the wrong anchor

`crates/nova_gameplay/src/sections/torpedo_section/projectile.rs:37` -
`update_target_position` homes on the target root's raw
`Transform::translation` (the ship's **build-spot origin**) rather than
`live_structure_anchor`.

`sections/mod.rs:38-43` states the rule explicitly: "the root ORIGIN is just
the build spot ... aim targets, lock-cone origins and camera anchors should all
use this anchor". Every other consumer follows it -
`input/player/intent.rs:125`, `input/ai/acquisition.rs:170`,
`input/targeting/radar.rs:79`, `camera/framing.rs:52`.

Failure: shoot away a large enemy's forward half so its COM shifts ~18 u aft of
the root origin. The fuze trips on `distance < blast.radius * 0.5`
(`projectile.rs:88-93`), so with the default 30 u blast radius the torpedo
needs to reach within 15 u of the *origin* - which is now empty space more than
15 u from any surviving hull. **A clean miss on a stationary wreck.**

Severity: bug. Confidence: likely (delegated trace).

### 5 and 6. Two audio state-leak bugs

| Site | Defect |
| --- | --- |
| `audio/cues.rs:99` | `play_safety_engaged_cue`'s `Local<bool>` is process-global, not per-entity, and survives the death of the ship it tracked - contradicting the doc claim on `:93`. Die while `WeaponsHot(true)`; the new ship gets `WeaponsHot::default()` = false, whose `Added` matches `Changed<WeaponsHot>`, so a safety-engage click plays on the first frame of the new run with nothing disarmed |
| `audio/loops.rs:188,313` | Loop sinks are volume-driven only while the scenario is live (`SpaceshipSectionSystems` is gated on `scenario_is_live`, `nova_scenario/src/loader/lifecycle.rs:23`) but the sink entities are session-persistent and never silenced on unload. Menu ambience ship burning -> New Game -> the systems stop running while the sinks keep looping at their last non-zero volume, so **the engine hum roars unchanged through the whole scenario load** |

`audio/cues.rs:99` is the **fourth** independent instance of the stale-`Local`
pattern (see `10-review-hud-nova-os.md` finding F and the
`mode-keyed-reconciler-just-spawned-override` memory). See the cross-cutting
section below.

## Smells

| Site | Issue |
| --- | --- |
| `sections/thruster_section.rs:353` | Main-drive thrust is a raw impulse **never multiplied by `dt`**, so linear authority is proportional to tick rate while torque and RCS authority are not. The flight layer compensates by reading `dt` back out (`flight/autopilot.rs:202`, `flight/thrusters.rs:91`), so it is internally consistent - but halving `Time<Fixed>` from 64 to 32 Hz halves every ship's and torpedo's linear acceleration while `apply_torque` and `rcs_burn_system` (which explicitly computes `accel * dt * mass`, `flight/manual.rs:271`) are unaffected. **A tick-rate change silently rescales linear-vs-rotational authority and every autopilot gain tuned against it** |
| `torpedo_section/projectile.rs:94` | Two unordered systems in the same schedule both plain-`despawn()` the same torpedo. `torpedo_detonate_system` is in `SpaceshipSectionSystems`, `update_temp_entities` in `TempEntitySystems::Sync`, and `plugin.rs:128-152` gives them no edge and no flush between. A torpedo whose lifetime expires on the frame it fuzes gets two queued despawns; the second warns, or **hard-panics under the `FallbackErrorHandler(panic)` the autopilot and probe runs install**. The sibling `despawn_shot_down_torpedoes` (`:43`) already uses `try_despawn` for exactly this |
| `torpedo_section/projectile.rs:65` | `torpedo_detonate_system` requires `&TorpedoTargetPosition`, which a dumb-fired torpedo never receives (`input/player/intent.rs:209` inserts `TorpedoTargetChosen` alone when `CombatLock` is `None`). **A no-lock launch is physically incapable of detonating** - it flies for the 100 s lifetime, deals only a contact ding, and is silently deleted. The bay still spent the round. May be intended, but nothing says so |
| `torpedo_section/bay.rs:112` | `Without<SectionInactiveMarker>` can never exclude anything: `integrity/glue.rs:49` is the only writer and is guarded by `With<SectionMarker>`, which the spawner does not have. Disable a torpedo bay in place and its cooldown keeps ticking to ready. The filter reads as a live-safety gate and does nothing |
| `objectives.rs:123` | **`rebuild_lines` can never run.** `ObjectivesPanelMarker` appears only inside `objectives.rs` (bundle, `Single` query, its own unit test) - VERIFIED by grep across `crates/`, `src/`, `examples/`. The live objectives HUD is a separate panel (`nova_scenario/src/loader/lifecycle.rs:49-63`). `ObjectivesPlugin`'s only system is a permanent no-op |
| `audio/cues.rs:147` | `play_dry_fire_cue`'s `Local<HashMap<Entity, bool>>` is never pruned for despawned turrets, unlike the sibling `SfxThrottle` which has an explicit `prune_sfx_throttle` (`mixing.rs:195`). Memory only - entity generations mean no stale-latch misfire |

## Nits

- `transform/directional_sphere_orbit.rs:121` (same at `sphere_orbit.rs:124`) -
  `lerp_and_snap` interpolates a raw angle with no wrap handling, so a target
  crossing the `theta = +/-pi` seam sweeps the long way round. Latent: the
  velocity HUD constructs it with `smoothing: 0.0`, making the lerp factor
  exactly 1.0. **Set `smoothing` above 0 and the prograde pip whips a full
  circle whenever ship velocity crosses world +Z.**
- `math.rs:35,47` - `lerp_and_snap`'s snap threshold is an absolute
  `f32::EPSILON` (1.19e-7). At chase-camera scale (hundreds to thousands of
  units) the smallest representable f32 gap is ~6e-5, so **the snap branch can
  never be true** before the lerp lands on `to` by rounding.
- `camera/shake.rs:295-296` - `shake_offset` and `shake_kick` are fed the *same*
  random sample, so the rotational kick is perfectly correlated with the
  positional offset. The shake reads as 1-D jitter rather than the intended
  6-DOF rattle.

## Audited and clean

This is the most reassuring section in the review. The simulation core holds up.

- **`flight/guidance.rs`, `thrusters.rs`, `manual.rs`, `state.rs`** - the
  arrival rule, the QP balancer, the bisection projection, the RCS cap sign
  logic and every degenerate-input guard check out. Specifically:
  `balance_throttles` always returns `engines.len()` entries, so the
  `throttles[i]` indexing in both `autopilot.rs:884` and `manual.rs:149`
  **cannot panic**.
- **`physics/pd_controller.rs`** - the inertia-frame composition is correct and
  pinned by a closed-form oracle built from the dependency itself. (The
  `if angle > PI` wrap at `:122` is dead after the shortest-path negation, but
  harmless.)
- **`gravity.rs`** - `well_accel`, `circular_orbit_speed`, the `dominant_well`
  hysteresis, the `Local` buffer reuse and the well-death observer ordering are
  sound.
- **`integrity/*`** - the destruction cascade, overkill clamp, structural-death
  backstop and `try_insert`/`try_remove` despawn guards are correct. The
  reviewer specifically chased `effective_mass = m1*m2/(m1+m2)` for a NaN on
  static bodies: **it does not reach**, because avian computes finite mass
  properties for static bodies too (`MassPropertyPlugin` has no body-type
  filter).
- `damage.rs`, `juice.rs`, all of `camera/*`, `transform/point_rotation.rs`,
  `sections/mod.rs::local_pose_in_root`, `turret_section/{aim,firing}.rs` -
  no reachable defects. The camera write order is fully pinned by
  `CameraAuthorityPlugin`, and the ungated `PostUpdate` instance of
  `SpaceshipSectionSystems` is a documented deliberate exemption.
- `mesh/slice.rs`, `mesh/builder.rs`, `lifetime.rs`, `cooldown.rs`,
  `asset_ref.rs`, the rest of `audio/`, the rest of `sections/` - clean.
- **No reachable `unwrap`/`expect`/indexing panics in non-test code across the
  entire audited scope.** Every hit was inside `#[cfg(test)]`. Fourth
  independent confirmation of this result.

## Two patterns the review surfaced across areas

### The stale `Local<T>` pattern - four independent instances

| Site | From |
| --- | --- |
| `hud/nova_os/shell.rs:363` | `10-review-hud-nova-os.md` |
| `audio/cues.rs:99` | here |
| `audio/cues.rs:147` (unpruned, not stale) | here |
| the original recorded in memory | `mode-keyed-reconciler-just-spawned-override` |

`Local<T>` in Bevy is per-system-instance and process-lifetime. Every use that
tracks *entity* state is a latent bug the moment that entity can respawn. Two
sites in the tree already carry the correct `Added<Marker>` override
(`shell.rs:288,320`) and one carries an explicit prune (`mixing.rs:195`), so
the codebase knows both fixes.

**This is a CONVENTIONS.md rule candidate with a real violation count**, and it
should be routed to that workstream.

### Sections that lie about their guard

`torpedo_section/bay.rs:112` (`Without<SectionInactiveMarker>` that excludes
nothing), `objectives.rs:123` (a system that can never run),
`nova_ui/src/widget/panel.rs:112` (a `skin` parameter that is discarded),
`nova_ui/src/status_bar.rs:238` (an entity that is never rendered). Four
sites across three crates where the code states an intent the mechanism does
not deliver.

This is precisely the owner's third deletion target - "dead and lying surface".
It now has concrete instances rather than a category name.
