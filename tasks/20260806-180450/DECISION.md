# Decision: Vendor bevy-common-systems

- DATE: 20260806-182655
- STATUS: ACCEPTED
- TASK: 20260806-180450
- TAGS: migration, dependencies, nova_gameplay, nova_events, nova_ui, nova_debug

## Context

`bevy_common_systems` (BCS) was split out of nova too early. The split forced
bad shapes: features that are not actually generic (health) lived in the shared
crate, nova wrapped them, and the wrappers apologised in comments for what they
could not change. Two chunks have already come back (`nova_autopilot`,
`crates/nova_gameplay/src/integrity/`) and both read better as nova's.

The owner's plan is to absorb the rest, finish the game, and only then extract
a better `bevy-common-systems` from whatever turns out to be genuinely
copy-pastable.

NOTES.md holds the verified move map. Research for this decision confirmed it
against BCS `6f09461` and found four errors in it, each a compile error if
followed - recorded at the end of NOTES.md and in `prototypes/00-conventions.md`.

Scale: ~6.5k LOC copied across five crates, ~130 callsites repointed.

## Decision

Absorb BCS into nova as **ten sequential migration steps**, one per subsystem,
each a single commit that leaves the workspace compiling. Recorded as
`prototypes/01..10` with `00-conventions.md` for the shared rules and
`README.md` for the order.

Three rulings shape every step:

1. **Logic verbatim, layout free.** Same behavior, constants, ordering and
   guard comments. New file and folder names where nova's tree wants them
   (`helpers/temp.rs` -> `lifetime.rs`, `camera_controller/` -> `camera/`,
   `meth` -> `math`, `ui/objectives.rs` -> merged into `objectives.rs`). Stay
   close enough to the original that the compiler drives the refactor. If a
   step needs design thought, the scope has widened - stop and record it.
2. **One `rand`: the nova workspace version, 0.10.2.** The port is three edits
   total (`use rand::Rng` -> `RngExt` twice, one generic bound), because 0.10
   renamed the method trait. Not a `bevy_rand` rewrite.
3. **A single task, not a parent epic.** Planned so it can run as a loop task:
   all ten steps coded straight through, one review at the end over the
   finished refactor.

Order: `01 events -> 02 ui -> 03 camera -> 04 transform -> 06 mesh -> 08 small
-> 05 physics -> 07 audio -> 09 debug -> 10 teardown`. This differs from
NOTES.md's suggested order in two places, both to avoid editing an import line
twice (`math` lands with the camera; `TempEntity` lands before
`rigid_body_point_velocity` because two section files import them together).

## Alternatives considered

**Parent epic with per-crate member tasks.** Rejected by the owner. The steps
are sequential and share one branch's worth of context; splitting them into
records buys tracking granularity and costs a working tree per step.

**Collapse the `camera_controller` wrappers inline, per module.** NOTES.md
suggests it. Rejected in favor of an import rewrite only: merging nova's
`framing.rs` into BCS's `chase.rs` is a redesign, and this branch already
carries a subtle camera ordering fix (`cd1bff21`) that a merge would put at
risk. Filed as a follow-up.

**Rewire the copied RNG onto `bevy_rand`** to match
`integrity/explode.rs`. Rejected: it is a determinism redesign inside a
mechanical lift. The three-edit version keeps the diff reviewable.

**Rename `ObjectiveMarkerTarget` -> `ObjectiveMarker`** once BCS frees the
name. Deferred: a public API change across `nova_scenario` and the HUD, not
this task.

**Dead-code sweep during the copy.** Deferred. Copied files carry unused names
(`CameraShakeOutput`, `WASDCamera`, `EventHandlerIndex`, several `*Systems`
sets). Removing them while moving them makes the diff unreviewable.

## Consequences

- The workspace graph gains exactly **two** edges, both intended:
  `nova_events -> nova_events_macros` (the derive's only user) and
  `nova_probe -> nova_events`. Anything else means a module landed in the wrong
  crate.
- `nova_probe` takes `nova_events` **directly**, confirmed by the owner: the
  run recorder's job is to record game events, so the event vocabulary is a
  first-class dependency of it, not something to reach through
  `nova_gameplay`'s re-export. This retires the routing comment at
  `nova_probe/Cargo.toml:37-44`, which currently explains why the types are
  reached through `nova_gameplay` instead. Prototype 10a.
- Three deps move rather than appear: `serde_json` -> `nova_events`,
  `noise 0.9` -> `nova_gameplay`, `bevy-inspector-egui 0.37` -> `nova_debug`.
  `Cargo.lock` should lose exactly two packages.
- Every crate's `debug` feature reduces to `["bevy/track_location"]`; features
  left forwarding to nothing get deleted.
- Copied code meets `#![warn(missing_docs)]` for the first time (BCS has no
  such lint) and CI's `clippy --workspace --all-targets --features debug`.
- Plugin registration moves wholesale out of
  `nova_gameplay/src/plugin.rs:81-106`, so every step must **run** its examples
  under Xvfb `:99` - `cargo check` cannot see a plugin registered twice.
- The probe baseline to hold against is `tasks/20260805-185103/` (sections 5,
  systems 3, stress 4, ui 5, all OK).
