# Retro: Vendor bevy-common-systems

- TASK: 20260806-180450
- BRANCH: master (no sprout - owner's instruction; `f3cf3150..HEAD`)
- REVIEW ROUNDS: 1 (APPROVE, no BLOCKER, no MAJOR)

## What went well

- **Ten prototypes, ten commits, one Step each.** Every commit left the
  workspace compiling and was independently provable. The plan's order
  (`01 02 03 04 06 08 05 07 09 10`) held with no re-ordering.
- **The glob-narrowing probe.** Drop the BCS glob, compile, read the unresolved
  list. Found in Step 1, reused in Steps 3-9. Cheapest way to learn what a
  wildcard import actually supplied.
- **Name-scoped absence greps caught what `cargo check` cannot.** Step 3's
  `bevy_common_systems.*(chase|shake|...)` grep found three callsites that still
  compiled against BCS types while the app used nova's - a silent
  `SystemSet`-ordering divergence a green build would have shipped. Every later
  step ran its own version.
- **Byte-diffing each copied module against BCS `6f09461`.** Zero logic,
  constant, ordering or guard-comment deltas; the reviewer re-derived the
  load-bearing `cd1bff21` camera ordering independently and agreed.
- **Disclosure over waving through.** Six DoD criteria turned out to be wrong;
  the implementer wrote each one up in `## Progress` instead of quietly passing
  it. All six surfaced at review as MINOR, none as a trust problem.

## What went wrong

- **The flow advanced to REVIEWING with Steps 7-10 undelivered** (`7ea2e4c3`,
  rewound by `cb56bcaf`). Steps 1-6 were each complete and provable, which made
  the record *look* finishable; the exit condition for WORKING is all ten Steps,
  not all delivered Steps.
- **One task record held an epic.** ~8k lines of diff, ~6.5k LOC vendored, ten
  independently landable commits, 1415 lines of TASK.md. `DECISION.md` ruled
  "single task, not an epic" and that ruling was sound at the time - the ten
  steps share one inseparable end state (BCS deleted) and no intermediate
  ordering is releasable on its own. What it did not price in is that the record
  becomes the bottleneck: the plan was written before any step ran, so its DoD
  lines aged for ten commits.
- **Six DoD criteria were wrong** (R1.3-R1.7 plus 10g's citation): a package
  count, a test count, a `--features debug` flag on a crate with no such
  feature, four example-run commands that never terminate and whose exit code is
  never the verdict, and a "nothing else registers it" claim contradicted by
  five `#[cfg(test)]` registrations. All were written at plan time against
  unverified assumptions and none was amended when disclosed.
- **30 minutes lost to an inert autopilot harness.** `cargo run --example X`
  without `NOVA_AUTOPILOT=1` *and* `--features debug` boots correctly and then
  idles forever. It does not fail. Second trap in the same command: this
  `xvfb-run` wrapper returns 1 even for `xvfb-run -a --server-num=99 true`.
- **A crate move silently drops its logs.** `nova_core::log_filter_str` names
  crates explicitly; moving `status_bar` out of BCS killed its `trace!`s with no
  compile error. Caught only by grepping the harness log for `StatusBar` and
  getting zero hits.
- **One review round for eighteen findings, all MINOR/NIT.** No rework - but the
  volume is the cost of a plan whose proofs were never run before being written
  down as proofs.

## What to improve next time

- **Amend a DoD line at the moment you disclose it is wrong.** Writing "the
  criterion says two, it is six, here is why six is better" in `## Progress` is
  honest but still ships a false criterion as the task's stated proof. Fix the
  line, keep the note.
- **A DoD command should be one you have run.** Every one of the six wrong
  criteria was authored from reading, not from execution. For a long plan,
  smoke-run the shape of each proof command at plan time even against the
  pre-change tree.
- **Never let a proof trust `$?` when the runner lies.** State the verdict
  string (`autopilot: cycle complete, no panic`) in the DoD line itself.
- **A "move a module between crates" step owns four sweeps, not one:** the
  compiler, the name-scoped absence grep, the log filter, and the prose. Steps
  2 and 3 discovered the last two the hard way and every later step paid it
  forward.
- **Ten Steps in one record want a per-Step gate.** The premature REVIEWING
  advance and the aged DoD lines are the same failure: nothing forced a
  checkpoint between Step 6 and Step 7.

## Diagnose

- **Breadth.** Inherently large, and correctly so - the deliverable is "BCS is
  gone", which no subset achieves. The independently landable split *did*
  happen, as ten commits; what did not follow it was the record. Not a weak
  ownership boundary and not scope found late: `NOTES.md` and the ten prototypes
  had the move map before any code changed, and the ~20 extra callsites the
  planning lanes found were folded in before Step 1.
- **Churn.** Near zero implementation rework - one review round, no BLOCKER, no
  MAJOR, no fix cycle. The rework that did occur is the `cb56bcaf` rewind, and
  the plan-time question that would have prevented it is not `plan`'s
  from-scratch challenge but a DoD one: the task had no per-Step completion gate
  between WORKING and REVIEWING, so "six of ten Steps done" and "done" were
  indistinguishable from the record's ACTIVITY alone.
- **Context.** One observed pressure point: the ten-lane read-only planning
  fan-out (one agent per prototype) was a deliberate delegation to keep 1415
  lines of plan out of one context, and it worked - it surfaced ~20 callsites
  and five wrong claims. The implementation ran the ten Steps sequentially in
  one lane with the record as the handoff medium; the rewind is the only sign
  that medium strained. Next time: split the record at the same seam the commits
  already use, so each Step is its own resumable unit.

## Action items

- Follow-ups from review round 1 - all MINOR/NIT, none blocking, all deferred to
  their own tasks or to the dead-surface sweep `00-conventions.md` already
  defers:
  - R1.1 re-add the three `rigid_body_point_velocity` tests from BCS
    `src/physics/rigid_body.rs:65-101`.
  - R1.2 scope `nova_gameplay/src/lib.rs:79`'s glob guard comment to *foreign*
    preludes.
  - R1.8-R1.10, R1.17 doc drift: `nova_events` row in `AGENTS.md` and the wiki,
    the missing `nova_events_macros` row and graph edge, the `persist.rs`
    dangling "Modelled as", and `automation-harness.md`'s ghost
    "shared-helpers crate".
  - R1.11-R1.16, R1.18 convention nits: two orphan sub-preludes, three drifting
    `DEBUG_TOGGLE_KEYCODE` consts, the two dropped `debug` features, mixed
    addressing in `plugin.rs`, `camera/mod.rs` visibility, the crate docstring
    module list, and the vacuous `git diff --exit-code` proof form.
- The owner's `manual:` checks recorded in `## Progress` still want the owner's
  judgement (debris geometry, attitude drift, single sound, F11 layer, crate
  graph edges, `cargo-about` manifest, 23/23 example runs, probe verdicts).
- `nova_core::log_filter_str` is missing six workspace crates for unrelated
  historical reasons (`nova_editor`, `nova_menu`, `nova_modding`,
  `nova_mod_format`, `nova_os`, `nova_probe`). Pre-existing, flagged by Step 10,
  wants its own task.
- Dead surface copied verbatim (`CameraShakeOutput`, `WASDCamera`,
  `EventHandlerIndex`, `RandomSphereOrbit`'s components) - the sweep
  `00-conventions.md` defers.

## Landing message

```
refactor: vendor bevy-common-systems into the workspace
```

Absorb every `bevy-common-systems` subsystem into the crate that owns it and
delete the dependency. The event engine and its derive macro go to
`nova_events` + a new `nova_events_macros`; the status bar and tween to
`nova_ui`; camera rigs, math, transform rigs, the mesh toolkit, lifetime,
cooldown, objectives, the PD controller, point velocity and SFX playback to
`nova_gameplay`; the inspector and wireframe layers to `nova_debug`.

Ten sequential steps, one commit each, every one leaving the workspace
compiling. Each module was copied verbatim and byte-diffed against BCS
`6f09461` - no logic, constant, ordering or guard-comment changes. `Cargo.lock`
loses the two BCS packages plus the four rand-0.9 transitives nothing else
pulled.
