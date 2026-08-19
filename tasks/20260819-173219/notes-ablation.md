# Phase B3: what the frame is made of, by ablation

Nothing here makes anything faster and nothing here lands. Every knob the
measurements ran through is a THROWAWAY branch (`arena-ablation`); the
deliverable is the numbers and what they rule out.

**The answer, up front.** A ship costs the frame by being DRAWN, and almost all
of what it costs to draw is one thing: **every section mesh carries its own
private material asset**, so nothing about a ship batches with anything else.
On the static gallery that is 64% of the per-ship cost, and turning it off
halves the frame. It arrived on 2026-08-18, two days before this measurement,
in `0ee9cbb0`.

## 1. The instrument

### The knobs

All env-gated, all off by default, so an unset run is the shipped example.

| knob | what it removes |
|---|---|
| `ABL_NOGATE` | the capture's readiness gate and the driven walk - the window becomes a fixed number of frames from `Playing` |
| `ABL_PEACE` | `engage_range = 0`, `engage_delay = 1e9`: the arena roster patrols and never fires, so nobody dies and the result screen never opens |
| `ABL_NOCRACKS` | `SectionCracksPlugin` - the per-section-mesh private material clone |
| `ABL_SHARECRACKS` | keeps the cracked material TYPE, shares one instance per source material |
| `ABL_NOCLAD` / `--bare` | the skin: every plate and greeble |
| `ABL_NOAI` | `SpaceshipController::None` - hulls present, brains off |
| `ABL_FREEZE` | every dynamic body goes static |
| `ABL_HIDESHIPS` | `Visibility::Hidden` on every hull - present, simulated, colliding, NOT drawn |
| `ABL_NOTHRUST` | `thruster_shader_update_system`, the per-frame `Assets::get_mut` on every thruster's exhaust material |
| `ABL_NOWEAPONS` / `ABL_NODRIVES` | turret and bay parts / drive parts, dropped from the WFC tile set |
| `ABL_NOJUNK` / `ABL_NOROCKS` / `ABL_NOCHEV` / `ABL_NOHUD` / `ABL_ZERO` | arena dressing, team chevrons, HUD, and the gallery's one-ship floor |
| `NOVA_PERF_RES` / `NOVA_PERF_RENDER_SCALE` | already shipped; used for the resolution sweep |

One branch with env switches rather than one branch per hack: same experiment,
one build instead of a dozen.

### The window, and the fixed-step pin

90 warm-up + 200 captured frames, one capture per PROCESS, in a probe-style
profile sandbox. `NOVA_PERF_MAX_DELTA=0.015625` pins the fixed loop to
**exactly one step per frame** - verified in every capture as
`fixed_steps min=1 max=1 mean=1.000`.

That pin is what made the study possible. Un-pinned, the fixed loop is a
MULTIPLIER on everything else the frame costs: bevy clamps the frame's virtual
delta to `max_delta` and runs `delta / 15.625 ms` steps, so with `B` the cost of
a one-step frame and `s` the marginal cost of a step, the steady state is
`F = B / (1 - s / 15.625)`. Phase A rejected the loop as a CAUSE and was right;
what is left is that it is an amplifier, and an amplifier makes every arm's
number depend on every other cost in the arm.

It also stops a failure mode Phase A saw only partially. Un-pinned and under
load, a run can enter the 16-step ceiling and STAY there - 16 steps is 250 ms of
simulation and the frame costs more than that, so virtual time never catches up.
Measured directly, first capture of this phase: `fixed_steps min=16 max=16
mean=16.000`, mean 457.6 ms, for the whole window. That is a fixed point, not a
stutter.

On the static gallery the pin costs nothing - `free11`, the same scene with
bevy's 0.25 s ceiling back, reads **+1.5%** against the pinned reference - so
the gallery numbers are directly comparable with a hand-run.

### The paired design, and why the first matrix was thrown away

Two other lanes were running their own Bevy binaries and their own builds on the
same RTX 3060 Ti for the first half of this session (measured: 222% and 170% CPU
simultaneously, load average 7-19). The first matrix ran round-robin with one
reference capture per pass, and it was not enough: its `noclad` arm - which
strictly SUBTRACTS 11,660 entities and cannot be slower - came out **56%
slower**. That is the box, and it is the number to judge any single reading by.

Everything below therefore runs **interleaved**: a fresh capture of the
reference arm immediately before every arm capture, each arm divided by the
reference interpolated between its neighbours, and the reported figure is the
MEDIAN of the per-arm ratios with the min-max of those ratios beside it. A ratio
whose spread straddles 1.00 has measured nothing.

Once the other lanes stopped, the reference held to **1.00-1.00** across 28
captures. Absolute milliseconds here are still not comparable with Phase A's or
with a hand-run; the RATIOS are.

## 2. The gallery: `wfc_ships`, the owner's own subject

Static row, bodies frozen at spawn, `SpaceshipController::None`, nothing firing.
Reference is the 11-ship row. 54 captures, 2 passes.

| arm | ships | mean ms | ratio | spread | mesh entities | private materials |
|---|--:|--:|--:|--:|--:|--:|
| **empty gallery** | 0 | **16.74** | 0.131 | 0.13-0.13 | 0 | 0 |
| baseline | 3 | 61.82 | 0.483 | 0.48-0.49 | 3,423 | 760 |
| **baseline (reference)** | 11 | **126.85** | 1.000 | 1.00-1.00 | 12,572 | 2,652 |
| baseline | 17 | 201.46 | 1.586 | 1.58-1.59 | 19,784 | 4,116 |
| no private section material | 3 | 33.15 | 0.259 | 0.24-0.27 | 3,423 | 0 |
| **no private section material** | 11 | **66.00** | **0.520** | 0.50-0.54 | 12,572 | 0 |
| no private section material | 17 | 80.64 | 0.638 | 0.63-0.65 | 19,784 | 0 |
| hulls not DRAWN | 11 | 40.35 | 0.318 | 0.30-0.33 | 12,572 | 2,652 |
| no `ThrusterExhaustMaterial` write | 11 | 110.89 | 0.867 | 0.86-0.88 | 12,572 | 2,652 |
| no cladding (`--bare`) | 11 | 124.63 | 0.971 | 0.97-0.98 | 2,860 | 2,652 |
| no turrets, no torpedo bays | 11 | 130.26 | 1.041 | 1.03-1.05 | 13,370 | 2,652 |
| 160x90 (1/64 the pixels) | 11 | 116.71 | 0.917 | 0.91-0.92 | 12,572 | 2,652 |
| 2560x1440 (4x the pixels) | 11 | 160.80 | 1.275 | 1.26-1.29 | 12,572 | 2,652 |
| bevy's own fixed-step ceiling | 11 | 127.40 | 1.015 | 1.01-1.02 | 12,572 | 2,652 |

### The two curves

Least squares over ships = 0, 3, 11, 17:

| line | floor | per ship | R^2 |
|---|--:|--:|--:|
| as shipped | 21.5 ms | **10.37 ms** | 0.990 |
| private section material removed | 19.9 ms | **3.77 ms** | 0.982 |

**Linear in ship count, no interaction term** - the owner's reading, confirmed
on the owner's own subject. The owner's hand fit was ~8 ms per ship on a ~17 ms
floor; this host measures 10.4 ms per ship on a **measured** 16.74 ms floor.

The floor is its own finding and it is exactly where the owner's fit put it: an
empty gallery - 1,124 entities, zero meshes, zero sections - costs **16.74 ms a
frame**, 0.13 of the 11-ship scene. Whatever that is, it is not ships, and at 60
FPS it is the entire budget.

### The account of a ship, in milliseconds

| what | ms per ship | how it was measured |
|---|--:|---|
| **private per-section material** | **6.60** | shipped slope 10.37 minus no-cracks slope 3.77 |
| `ThrusterExhaustMaterial` re-prepared every frame | 1.54 | 13.3% of the 11-ship frame, over 11 ships |
| everything that is NOT drawing | 2.00 | hulls hidden: (40.35 - 16.74) / 11 |
| the rest of the render | 0.23 | the remainder |

**Two material defects are 79% of what a ship costs.** Sections, colliders,
health, link points, integrity, transform propagation - everything the hull IS
rather than everything it DRAWS - come to 2 ms a ship, and that is with the
hulls still colliding and still in the broad phase.

### What this eliminates

- **Cladding: 0.97 (-2.9%).** Removing 10,936 entities - every one of them a
  `destructible_body` with a `plate_collider` and a `Mesh3d` - changes nothing.
  The owner's hand test and this measurement agree. **Mesh instance count is not
  the driver, collider count is not the driver, and per-entity health and
  integrity work is not the driver.**
- **Weapons: 1.04 (+4.1%).** Dropping turrets and bays from the tile set does
  not help; the collapse fills the cells with hull instead and the ship carries
  MORE mesh entities (13,370 against 12,572). Per-mount machinery is not the
  cost.
- **Fill: not binding at 720p.** 1/64 the pixels buys 8%. The mechanism is not
  broken - 4x the pixels COSTS 27.5%, so the window really does resize - the GPU
  simply is not the constraint until somewhere above 1080p. Reading them
  together: CPU work of about 117 ms with GPU work of about 80 ms underneath it
  at 720p, and the GPU passing it by 1440p.

## 3. The arena: `wfc_arena`, the same answer under a live roster

4v4, peace window, one fixed step per frame. Reference 114-116 ms. 60 captures
in the scaling tier, 12 in the mechanism tier.

| arm | ratio | spread |
|---|--:|--:|
| 1v1 | 0.502 | 0.47-0.52 |
| 2v2 | 0.686 | 0.68-0.69 |
| **4v4 (reference)** | **1.000** | 1.00-1.00 |
| 6v6 | 1.279 | 1.26-1.39 |
| no private section material | **0.521** | - |
| no private section material AND no cladding | 0.503 | - |
| hulls not DRAWN | 0.267 | - |
| hulls not DRAWN, 1v1 instead of 4v4 | 0.242 | - |
| no cladding | 0.893 | 0.81-0.90 |
| no dressing (rocks, derelicts, planetoid) | 0.900 | 0.89-1.00 |
| 160x90 | 0.891 | 0.86-0.92 |
| `render_scale` 0.25 | 1.003 | 0.94-1.04 |
| no `ThrusterExhaustMaterial` write | 0.977 | 0.91-1.01 |
| bodies frozen static | 0.979 | 0.95-1.01 |
| no AI | 1.052 | 0.96-1.06 |
| no chevrons, no HUD | 1.013 | - |
| 2560x1440 | 2.645 | - |

**Fitted shape: linear in ship count, R^2 = 0.997**, `T = 0.353 + 0.0773 x
ships` in units of the 4v4 frame - a 35% constant and 9.1 ms per ship.

Read rows three and four of the mechanism block together. **Hidden hulls cost
0.267 at 4v4 and 0.242 at 1v1: the ship-count scaling is GONE.** Everything the
extra six ships cost, they cost by being drawn.

The private material costs the arena the same SHARE it costs the gallery - 0.52
either way - which is the strongest thing either subject says on its own: two
scenes with different dressing, different rosters and different cameras land on
the same halving. The thruster material matters less here (0.98 against the
gallery's 0.87), because an arena frame also carries a rock ring, three derelict
blobs and a planetoid that a gallery does not.

Two arena-only notes:

- **The peace window stands in for the fight.** The owner's ground truth is that
  the frame does not sag during combat; every arm above is a passive roster, and
  the per-ship line it produces is the same shape the owner measures in a live
  4v4 through the menu.
- **`render_scale 0.25` reads 1.00 where a 160x90 WINDOW reads 0.89.** The
  render-scale path allocates an offscreen target and blits it back; the blit
  costs about what the 16x pixel saving buys. It is not a contradiction, it is
  two different mechanisms, and the window sweep is the cleaner one.

## 4. The mechanism, named in source

The count instrument, on the 11-ship gallery:

```
sections=2324  mesh_entities=12572  distinct_meshes=681  distinct_materials=32
crack_entities=2652  distinct_crack_materials=2652  crack_assets=2652
```

**2,652 mesh entities, 2,652 distinct materials - one each.** Turn
`SectionCracksPlugin` off and the same 12,572 mesh instances draw through 147 to
413 shared materials, and the frame halves.

`crates/nova_ship/src/sections/damage_cracks.rs` says so in its own doc - "Per
section material clones" - and `resolve_pending_cracks` mints one
`SectionCracksMaterial` per pending mesh. Binning keys on the material, so 2,652
unique materials are 2,652 bins of one instance each: the worst case for
`write_binned_instance_buffers` and `prepare_preprocess_bind_groups`, the two
rows the old `20260819-123928` trace put at 24 ms and 16 ms a call.

**This is also why cladding is free.** `owning_section` RETURNS NONE on a
`SectionFixture`, so plates and greebles keep the shared material they were
authored with and batch normally. The 10,936 plate entities draw through 32
materials; the 2,652 section meshes draw through 2,652.

### Which half of it costs: the instance, not the type

`ABL_NOCRACKS` removes two things at once - the per-entity clone AND a second
material TYPE with its own pipeline. `ABL_SHARECRACKS` separates them: it keeps
`SectionCracksMaterial`, its shader and its pipeline, and only keys the clone by
its SOURCE material instead of minting one per mesh. (The LOOK is wrong under it
- one section's damage would crack every section drawn from the same source -
which is a design problem, not a performance one. It is here to price an axis.)

| gallery, ms | 3 ships | 11 ships | 17 ships | fitted ms per ship |
|---|--:|--:|--:|--:|
| as shipped, one material per mesh | 61.82 | 126.85 | 201.46 | **10.37** |
| shared instances, same material type | 39.47 | 75.26 | 103.54 | **4.95** |
| material type removed outright | 33.15 | 66.00 | 80.64 | **3.77** |

**Sharing the instances recovers 5.42 of the 6.60 ms per ship - 82% of the whole
win - with the cracked material and its shader still in the frame.** The
remaining 1.18 ms per ship is the cost of the extra material type itself.

So the axis is DISTINCT MATERIAL INSTANCES, not the extension, not the shader
and not the effect. That is the number any fix should be graded against, and it
says a fix does not have to remove the feature.

## 5. The regression, dated

The owner's suspicion - "feels like FPS for these things is lower than when we
initially added them" - is correct, and the diff is the bisect. No older build
was run.

`damage_cracks.rs` was added by **`0ee9cbb0`, "Give every destructible body
damage it wears in its own geometry", 2026-08-18** - the day before this
measurement. It replaced `damage_tint.rs`, which cloned per section too, with
one difference that decides the gallery:

```rust
// damage_tint.rs at 0ee9cbb0^, mark_section_meshes
let mode = match q_allegiance.get(root) {
    Ok(Allegiance::Player) => TintMode::Full,
    Ok(Allegiance::Enemy) => TintMode::DeadOnly,
    Ok(Allegiance::Neutral) | Err(_) => continue,
};
```

**An unaligned body got no clone.** `wfc_ships` hulls have no allegiance, so the
gallery went from ZERO private materials to one per section mesh on 2026-08-18.
`SectionCracksPlugin` has no such gate.

The arena hulls always carried an allegiance, so the arena always cloned; what
it gained was a second material TYPE with its own pipeline.

**One caveat, and it is real.** `0ee9cbb0` is a large commit and changed several
other paths, so "it regressed here" is established for THIS mechanism only. No
older build was run: the source diff and the `ABL_SHARECRACKS` arm together are
what carries the claim, and a bisect would add a number, not a verdict.

## 6. The one exaggerated case to build next

The three candidates on the table were 10,000 asteroids, 100v100 small ships,
and 1v1 with enormous hulls. **Field the enormous hulls, and nothing else.**

The measurement has one open question left. Ship count and section count are
perfectly correlated in everything measured here - a WFC hull is 211 to 213
sections whatever the roster - so no arm above can say whether the frame tracks
SHIPS or SECTIONS. Every other axis is already answered: pixels do not bind,
mesh instances do not bind (cladding), colliders do not bind (cladding, freeze),
AI does not bind, weapons do not bind.

- **10,000 asteroids** exercises many simple bodies with one mesh each and no
  hierarchy. It measures the broad phase and instance count - both of which
  cladding already eliminated - and it has no sections, so it cannot touch the
  mechanism this phase found.
- **100v100 small ships** multiplies every axis at once. It would be slow to
  build, slow to run, and would isolate nothing.
- **1v1 with enormous hulls** holds ship count, AI count, rigid-body count,
  camera framing and pixel coverage FIXED and varies only sections per hull.
  If the frame tracks sections, one 2,000-section hull costs what nine ordinary
  ones do. If it tracks ships, two hulls stay cheap however large they are.

It is also nearly free to build: `examples/playable/shared/wfc.rs` sizes the
collapse grid from three constants - `HALF_WIDTH = 4`, `HEIGHT = 5`,
`LENGTH = 11`, giving 220 half-grid cells and about 210 sections a hull. Doubling
them is one edit and about 8x the sections. Sweep `--ships 1` across grid sizes
against the same total section count reached by ship count, and the two lines
either lie on top of each other or they do not.

Run it with `ABL_NOCRACKS` as a second arm and the same sweep prices the fix.
