# Prototypes - vendoring `bevy_common_systems` into nova

Ten migration steps. Each is one commit that leaves the workspace compiling.
Read `00-conventions.md` first; the rest assume it.

| # | Step | Crate | LOC | Depends on |
|---|---|---|---|---|
| 00 | Conventions, corrections to NOTES.md | - | - | - |
| 01 | Event engine + `EventKind` derive | `nova_events`, new `nova_events_macros` | 605 | - |
| 02 | Status bar + tween | `nova_ui` | 747 | - |
| 03 | Camera rigs + `math` | `nova_gameplay` | 1623 | - |
| 04 | Transform rigs | `nova_gameplay` | 837 | 03 (`math`) |
| 05 | PD controller + point velocity | `nova_gameplay` | 676 | 08 (`TempEntity` co-imports) |
| 06 | Mesh builder / explode / slice | `nova_gameplay` | 987 | 03 (`math`) |
| 07 | SFX playback + sound bank | `nova_gameplay` | 347 | - |
| 08 | Lifetime, cooldown, objectives | `nova_gameplay` | 519 | - |
| 09 | Inspector + wireframe | `nova_debug` | 379 | - |
| 10 | Delete the dependency | all | - | 01-09 |

~6.5k LOC copied. NOTES.md's "~4.9k" undercounts (it omits `meth`, the `mod.rs`
files, and counts `slice.rs` outside the total).

## Suggested order

`01 -> 02 -> 03 -> 04 -> 06 -> 08 -> 05 -> 07 -> 09 -> 10`

Rationale, where it differs from NOTES.md's:

- 03 before 04 and 06: `math` (`LerpSnap`, `spherical_to_cartesian`, `slerp`)
  lands with the camera and both need it.
- 08 before 05: `torpedo_section/mod.rs:16` and `turret_section/firing.rs:8`
  each import `TempEntity` and `rigid_body_point_velocity` on one `use` line.
  Doing 08 first means editing each line once.
- 02 and 09 are independent and can slot anywhere before 10.

## What each prototype gives you

Scope table with verified line counts, the exports that must survive by name,
manifest diffs, a file-and-line callsite list, the compile hazards that are
already known, the verification commands, and a done-when list.

They are not implementation plans - they do not tell you what to type. They
tell you what is true about the code so the compiler-assisted refactor does not
surprise you.

## Owner's rulings that shaped these

1. **Logic verbatim, layout free.** Same behavior, same constants, same
   ordering. New file and folder names where nova's tree wants them
   (`helpers/temp.rs` -> `lifetime.rs`, `camera_controller/` -> `camera/`,
   `meth` -> `math`). Stay close enough that the compiler drives the refactor.
2. **One rand.** Use the nova workspace version, 0.10.2. The port is three
   edits total (two `use rand::Rng` -> `RngExt`, one generic bound). Do **not**
   rewire onto `bevy_rand` to match `integrity/explode.rs`.
3. **Single task, not an epic.** Planned so it can run as a loop: all ten steps
   coded straight through, review at the end over the finished refactor.
4. **`nova_probe` takes `nova_events` directly.** The recorder records game
   events, so the event vocabulary is a first-class dep of it - not something
   to reach through `nova_gameplay`'s re-export. One of the two new graph
   edges this task adds; the other is `nova_events -> nova_events_macros`.
   Everything else stays edge-neutral.
