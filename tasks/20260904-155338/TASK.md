# Find the railgun impact spike on a big hull

- STATUS: CLOSED
- PRIORITY: 66
- TAGS: v0.13.0, performance, combat

## Report

A huge frame spike lands at the moment a railgun slug HITS the big ship in
mainline First Shift (`first_shift_08_attack_salvo`). `wfc_arena` fires the
same weapon family and does not spike.

## The user's hypothesis

Mainline block ships mount the SIEGE lance, `wfc_arena` the standard one
(`crates/nova_authoring/src/base_content/sections/standard.rs`):

| | `slug_damage` | `slug_power` | `rake_radius` |
|---|---|---|---|
| `railgun_lance_section` (wfc_arena) | 300 | 1 800 | 10 m |
| `siege_railgun_lance_section` (block ships, `ships/block.rs:44`) | 500 | **360 000** | **30 m** |

200x the pierce budget down a bore three times as wide, so one shot rakes and
destroys far more sections at once. Suspected costs, in no order: the corridor
sweep, the carve, wreck-piece spawns and their colliders, avian mass-property
recomputation, the connected-component sever search, debris and particles,
collision-event volume.

## Follow-on from

`tasks/20260904-133640` quieted the destruction logging (that run showed one
frame destroying 812 sections). The logging is no longer the cost; the spike
survived it.

## Proof

### Instrument

The spike is on a mainline playable scene, which wires no `NovaProbePlugin`, so
the instrument was a bevy chrome trace of the whole autopilot run:

```sh
# in the sprout, host verified quiet first: no game process, no rustc,
# /proc/loadavg 1-min under 3.5, sampled every 5 s beside the run
nix develop --command cargo build --features debug,trace \
    --example first_shift_08_attack_salvo
DISPLAY=:99 NOVA_AUTOPILOT=1 TRACE_CHROME=<out>/trace.json \
    RUST_LOG=bevy_ecs=info BEVY_ASSET_ROOT=<sprout> \
    nix develop --command cargo run --features debug,trace \
    --example first_shift_08_attack_salvo
```

`RUST_LOG=bevy_ecs=info` is load-bearing: the game's default filter sets
`bevy_ecs=warn`, which emits no spans at all. The trace grows about 28 MB/s;
33 s of gameplay is 5.9 GB, streamed by throwaway analysers in the session
scratchpad (self time per span, per frame via the `schedule: name=Main` span,
and per 100 ms bucket across all thread ids).

Read every absolute figure below as `docs/performance.md` says to. This is a
traced dev build under Xvfb: `present_frames` alone is a 15-21 ms additive
window copy, and the fixed loop amplifies. The numbers RANK, and the
before/after ratio stands because both arms used the same instrument on the
same host. They are not an FPS claim.

Load, 1-min, sampled every 5 s: before arm 2.79-4.48, after arm 2.18-2.73.

### The spike is not one frame

The salvo impact costs two adjacent frames, then two seconds of aftermath:

| | main-world | what |
|---|---|---|
| `[12887.3..12994.8]` | 107.51 ms | first collapse frame |
| `[12995.6..13198.4]` | 202.76 ms | second collapse frame |
| `[14309..16299]` | 264-320 ms each | the wreckage going dynamic |

### Ranked attribution, collapse frame `[12995.6..13198.4]`, 202.76 ms

| self ms | total ms | calls | span |
|---|---|---|---|
| 69.07 | 152.02 | 2 | `system_commands: nova_gameplay::rounds::advance_rounds` |
| 36.41 | 36.41 | 743 | `system: nova_ship::sections::integrity::queue_depleted_section_sever` |
| 21.10 | 21.10 | 1 | `present_frames` (the Xvfb window copy, not the game) |
| ~24 | | 28 629 | avian per-entity insert/remove hooks, summed over 8 systems |
| 6.53 | 6.53 | 2 | `system: nova_gameplay::rounds::advance_rounds` (the rake sweep) |
| 1.60 | 1.60 | 567 | `nova_gameplay::integrity::explode::detach_destroyed_body` |
| 1.05 | 1.05 | 1690 | `nova_gameplay::juice::on_damage_juice` |
| 1.01 | 1.01 | 1 | `nova_ship::sections::integrity::sever_disconnected_structures` |
| 0.97 | 0.97 | 578 | `nova_gameplay::integrity::spew::spew_carved_material` |
| 0.93 | 0.93 | 1123 | `nova_ship::ship_audio::combat::on_surface_impact_play_sfx` |

By owner: nova_gameplay 86.31 ms / 14 823 calls, nova_ship 46.48 / 5 893,
`bevy_` 44.83 / 15 408, avian3d 23.98 / 28 629.

So the frame is a COMMAND FLUSH, not a sweep. 743 section despawns, 743
wreck-piece spawns, about 4046 carve shards and about 1100 carve closures land
in one flush: 4823 `TempEntity` inserts, of which roughly 85 percent are
shards (`mark_radius(200 hp)` is 2.29, so `spew_carved_material` throws the
clamped maximum of 7 per carve).

### Ruled OUT by number

- **The rake corridor sweep.** 6.53 ms, and that includes walking all 2079
  colliders of the carrier. The siege lance geometry is real - about 1124-1152
  of the carrier's 2079 build-grid cells lie inside the 30 m rake radius of a
  long-axis bore, against 264-306 at 10 m, and `slug_power` 360 000 against
  about 66.7 hp per reinforced cell buys some 5400 cells, so the corridor is
  destroyed in full - but SWEEPING it is not what costs.
- **The connected-component sever search.** `sever_disconnected_structures`
  1.01 ms. The whole hull split into 2 to 4 bodies five times in 33 s.
- **The carve and the spew observer.** Both under 1 ms of self time. Their cost
  is the entities they QUEUE, which lands in the flush above.
- **Logging.** Already fixed in `tasks/20260904-133640`; the spike survived it.
- **Two other 300 ms frames in the run are not the spike.**
  `nova_assets::merge::register_bundles` costs 326 ms at t~1.3 s and
  Update/SpawnScene 315 ms at ship spawn t~2.1 s. Load time, not combat.

### Fix landed: one centre-of-mass snapshot per root per frame

`queue_depleted_section_sever` is an `On<Add, HealthZeroMarker>` observer that
walked EVERY section and EVERY fixture in the world, filtered down to the one
root, to snapshot the pre-cut centre of mass - once per depleted section. A
capital hull depletes hundreds of sections into one flush, so the walk was
quadratic. It now snapshots once per root, on the first section of that root to
deplete, and later cuts measure their offset against that stored snapshot.

That is also the only self-consistent arithmetic: `apply_pending_sever_motion`
places the cut at `old_com_world + rotation * mean(cut_offsets)`, and
`old_com_world` comes from the FIRST section's snapshot, so an offset measured
against a centre of mass that had already moved was being added to a different
origin than the one it was taken from.

Same instrument, same scene, one traced run per arm (one sample each):

| `queue_depleted_section_sever` | calls | self ms |
|---|---|---|
| whole 33 s run, before | 3822 | 92.49 |
| whole 33 s run, after | 3802 | 3.08 |
| the salvo impact, before | 1088 | 55.18 |
| the salvo impact, after | 1063 | 0.87 |

Per call that is 24.2 us to 0.81 us. Distribution in the collapse frame, before:
p50 60.7 us, 567 of 743 calls over 20 us - the 567 being exactly the sections
that still had a collider to detach.

The impact stopped needing two frames. Before: 107.51 + 202.76 = 310.27 ms over
two frames for 1088 depletions. After: one frame `[13093.6..13329.6]` of
235.98 ms for 1063. The 74 ms difference is the 54 ms of observer plus the
second frame's own window copy. `detach_destroyed_body` is unchanged (2017
calls / 6.35 ms before, 2060 / 6.21 ms after), which is how we know the
workload itself did not move.

The behavioural fact is held down by
`every_cut_in_one_frame_measures_from_the_same_centre_of_mass` in
`crates/nova_ship/src/sections/integrity.rs`. It stands three unit-cube
sections in a row, depletes the far one, despawns it (the ordering a real
collapse has - `detach_destroyed_body` despawns inside the flush that depleted
the section), then depletes the near one, and requires both cuts to read +1 and
-1 from the same snapshot. On the old code the second cut reads -0.5. It asserts
no timing. `cargo test -p nova_ship --lib sections::integrity`: 38 passed.

### What is left, and it is a design question

After the fix the worst frames in the run are the AFTERMATH, and they are
avian, not nova. Frame `[14080.5..14415.3]`, 334.77 ms:

| self ms | calls | span |
|---|---|---|
| 43.65 | 72 | `solve_contacts<true>` |
| 42.03 | 72 | `solve_contacts<false>` |
| 34.87 | 12 | `nova_gameplay::rounds::advance_rounds` |
| 21.01 | 1386 | `par_for_each` VelocityIntegrationQuery |
| 18.41 | 72 | `warm_start` |
| 17.93 | 1386 | `par_for_each` SolverBodyInertia |
| 15.92 | 1386 | `par_for_each` SolverBody |
| 14.44 | 12 | `update_narrow_phase` |
| 12.57 | 12 | `prepare_contact_constraints` |

Twelve fixed steps in one frame: this is the fixed loop amplifying, exactly the
`F = B / (1 - s/T)` shape in `docs/performance.md`. The population driving it is
one collapse's worth of bodies - about 800 wreck pieces all becoming
`RigidBody::Dynamic` with a collider on the SAME frame, because
`CHUNK_GRACE_SECS` is 0.5 s and they were all born in one frame, plus about 4000
shards being simulated and then despawned.

Two options, neither taken here, both needing an owner's decision because they
change how a collapse LOOKS:

1. **Cap the shards.** They are 85 percent of the entities a collapse frame
   creates and they exist for 2.5 s of visual spice. A per-frame budget on
   `spew_carved_material`, or scaling the per-carve count down as the frame's
   carve count rises, cuts the flush and the solver population together.
2. **Stagger `ChunkGrace`.** Jitter the grace so 800 pieces do not all go
   dynamic on one frame. This does not reduce the work, it spreads it, which is
   what the frame-time tail actually cares about.

The remaining nova item, `system_commands: advance_rounds` at 111.42 ms after
the fix, IS that flush. It is not a hot loop to tighten; it is the cost of the
entities the two options above decide to stop creating.

### Range gap, not closed

`examples/systems/system_railgun_lance.rs` cannot see this. It mounts
`RAILGUN_LANCE_SECTION_ID`, the STANDARD lance, fires at six free-floating
plates pinned at `PLATE_HEALTH = 500.0` chosen so that no layer ever DIES
(destruction "pulls in render observers this range has nothing to say about"),
and wires `NovaProbePlugin::default().without_frametime()` because a one-shot
walk cannot fill the 900-frame window. So no range in the roster fires a siege
lance into a capital hull, and none records a collapse frame cost.

No range was added here. The behavioural fact this task fixed is only visible
inside the crate - `PendingSeverRoots` is private - so it belongs in the unit
test above, and `examples/systems/README.md` is explicit that a range asserts
the hardware-independent fact it can see from outside and never a millisecond.
The gap worth filling is a `stress_` range that stands up a capital hull, fires
one siege lance down its long axis, ASSERTS the corridor cell count destroyed,
and RECORDS the collapse frame cost as a `probe_marker` payload against a named
reference. That is a new roster entry and a permanent CI cost, so it is the
main session's call.
