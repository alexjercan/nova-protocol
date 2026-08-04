# Notes: Deepen sections/ to multi-scene multi-round runs, merging com_range and torpedo_guidance

Goal in one line: seven `sections/` runs become five - one per section family -
and each survivor walks several rounds across at least two scenes instead of
one beat behind a wall-clock runway.

## What changes

Before: `examples/sections/` holds seven runs. Two of them are subject-split
rather than family-split: `com_range` (423 lines) tests a consequence of
`hull_section`'s damage pipeline, and `torpedo_guidance` (301 lines) tests the
guidance half of `torpedo_section`'s bay. Most runs are one scene, one section,
one beat.

After: five runs, one per family. Each spawns, drives, damages, destroys,
reloads, re-enters and re-asserts, gating every beat on the value it depends on
rather than sleeping past it. The absorbed assertions live on inside the
absorbing run.

| Run | Lines today | Change |
|-|-:|-|
| `controller_section` | 227 | deepen: PD attitude across layouts and rounds |
| `thruster_section` | 234 | deepen: throttle -> impulse + plume across rounds |
| `hull_section` | 247 | deepen, ABSORBS `com_range` |
| `turret_section` | 734 (+203 slider) | deepen: tracking + firing across scenes |
| `torpedo_section` | 529 | deepen, ABSORBS `torpedo_guidance` |

## Surfaces

| File | Why |
|-|-|
| `examples/sections/com_range.rs` | DELETED. `assert_com_follows_sections` (:374) ports into `hull_section` as a round after the destroy round. It asserts four things: COM drift < 0.3 from the attached-section centroid, local COM moved aft (z > 2.4), the root keeps `TransformInterpolation`, and the `ChaseCamera` anchor tracks the live COM within 0.5. |
| `examples/sections/torpedo_guidance.rs` | DELETED. Its PN closest-approach assertion becomes `torpedo_section`'s lead-a-crosser round. |
| `examples/sections/hull_section.rs` | Absorbs the above; gains rounds. |
| `examples/sections/torpedo_section.rs` | Absorbs the above; gains rounds. |
| `examples/sections/turret_section/slider.rs` | 203-line interactive tuning submodule. STAYS. Not this task's problem. |
| Shared ship builder (new module) | Extracted `fn`s with a count knob, so `stress/many_sections` (094006) reuses them. |
| `Cargo.toml` | Two `[[example]]` blocks deleted. |
| `tests/examples_smoke.rs` | `SECTIONS:32` loses two entries. Atomic with the deletions. |
| `crates/nova_debug/src/sections.rs` | Existing section debug helpers - read before writing new ones. |

## Data and interfaces

The shared builder is the interface that outlives this task:

```rust
/// A ship with `count` sections of the given family, at a deterministic
/// layout. The count knob is what stress/many_sections needs; sections/
/// runs pass small numbers.
pub fn ship_with_sections(kind: SectionKind, count: usize) -> ScenarioConfig;
```

Assertions stay world-action `fn`s run on step entry, the shape `com_range`
already uses:

```rust
#[cfg(feature = "debug")]
fn assert_com_follows_sections(world: &mut World);
```

Predicates come from `nova_debug::harness`: `section_gone(id)`,
`scenario_variable_is(key, v)`, `player_ship_present()`.

## Sketches

Illustrative only.

```diff
 // hull_section.rs
 nova_autopilot()
     .step("damage").until(hull_integrity_below(0.5)).add()
     .step("destroy").until(section_gone("hull_fore")).add()
+    // ported from com_range.rs:374 - COM-follows-destruction is this
+    // pipeline's consequence, not a separate subject
+    .step("com_follows").on_enter(assert_com_follows_sections)
+        .until(frames(1)).add()
+    .step("reload").on_enter(reload_scene).until(player_ship_present()).add()
+    .step("round_two")...
```

## Shape

```
      one example per section FAMILY
      ------------------------------
controller_section   thruster_section   hull_section   turret_section   torpedo_section
                                            ^                                ^
                                            |                                |
                                     com_range (del)            torpedo_guidance (del)
                                     COM-follows-destroy        PN closest-approach
                                     -> a round after           -> the lead-a-crosser
                                        the destroy round          round

per run:  spawn -> drive -> damage -> destroy -> ASSERT -> reload -> re-enter -> ASSERT
          (every arrow is a predicate, not an elapsed-time step)

shared:   ship_with_sections(kind, count)  ---> also stress/many_sections
```

## Consequences and open questions

- Merging is a net LOSS of isolation: when `hull_section` fails, the failure
  could now be damage, destruction, COM or the camera. Mitigated by the ported
  assertion's own panic messages, which already name their subject precisely.
- Deepening is the sprint's least mechanical work. Five runs each need new
  scenes and new rounds invented, against sections whose behavior has to be
  read first. This is the task most likely to be larger than its priority
  suggests.
- `sections/` carries no fps window (per 093855), so no run needs padding to
  fill a capture. Existing runways can go.
- RESOLVED (owner, 2026-08-04), and the ambiguity dissolves rather than getting
  answered: the bound is a NAMED INVARIANT LIST per run, not a scene count.
  27 invariants across five runs - 14 exist today, 5 arrive with the merges, 8
  are new. Scenes and rounds became means: a run gets a second scene only if an
  invariant needs one. "Two scenes" was a proxy for depth and an unfalsifiable
  one; the list is the stopping rule.
- The invariant lists in the Steps are DRAFTED from what each run asserts today
  (every existing one cited by line). They are worth arguing about at planning,
  because they are the definition of done - a wrong list is a wrong task.
- OPEN: `turret_section` is already 734 lines before deepening, plus the
  203-line slider submodule. Deepening it further may be the point at which the
  slider extraction stops being optional. Noted as not-blocking, but it is the
  same file.
- RESOLVED (owner, 2026-08-04): build the ship fixture LOCALLY here; do not
  extract. `094006` is the third caller and owns the extraction. The dependency
  edge `093950 -> 094006` survives but now means "094006 needs to SEE this
  shape in order to extract", not "094006 consumes a builder this task
  published". `094006` also gained an edge to `093934` for the same reason.
