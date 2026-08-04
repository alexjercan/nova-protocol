# Port the scripted autopilot driver into nova_autopilot

- PRIORITY: 97
- TAGS: v0.10.0, tooling, autopilot
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE
- PARENT: 20260802-120019
- DEPENDS ON: 20260802-183340

## Story

Port the scripted state driver into `nova_autopilot::autopilot`:
`AutopilotPlugin<S>` holds a `(state, seconds)` timeline, runs a per-frame input
closure after `InputSystems`, and reports done to the completion protocol.
Keeps `self_completing` (script owns the finish; an expired runway aborts) and
`loop_while_pending` (repeat the cycle while other collectors are pending,
announced by the `AutopilotLoop` message). Armed by `NOVA_AUTOPILOT`.

## Steps

- [x] `crates/nova_autopilot/src/autopilot.rs`: port `AutopilotPlugin<S>`,
      `AutopilotLoop`, the private `AutopilotState<S>` resource and the
      `autopilot_drive::<S>` exclusive system verbatim in behavior, retargeted
      onto this crate's `crate::completion` (`register`, `HarnessCompletion`,
      `AUTOPILOT`).
- [x] Same file: declare `pub const AUTOPILOT_ENV: &str = "NOVA_AUTOPILOT";`
      (mirroring `completion::DEADLINE_ENV`) and use it for the arming check.
      No `BCS_AUTOPILOT` alias, no fallback read.
- [x] Rewrite the module `//!` docs for this crate: the doctest imports
      `nova_autopilot::autopilot::AutopilotPlugin` by path (the prelude is
      still empty; populating it is `20260802-183355`), and names
      `NOVA_AUTOPILOT`.
- [x] `crates/nova_autopilot/src/lib.rs`: drop the outer `///` on
      `pub mod autopilot;` and leave the same NOTE the `completion` line
      carries - an outer doc concatenates ahead of the module's `//!` docs and
      re-resolves their intra-doc links in lib scope.
- [x] Add the four `#[cfg(test)]` tests on a shared rig: `App` +
      `MinimalPlugins` + `bevy::input::InputPlugin`, a local `TestState` enum,
      and `TimeUpdateStrategy::ManualDuration(1/60s)` so the timeline is
      frame-deterministic. Arm `NOVA_AUTOPILOT` once per test binary through a
      `std::sync::Once` helper (set only, never removed, one value - so the
      parallel test threads never disagree).

## Definition of Done

- The timeline advances the state machine and reports done once.
  (test: `autopilot_drives_the_timeline_and_reports_done`)
- A `just_pressed` poke from the input closure is visible to game systems.
  (test: `input_closure_press_survives_input_collection`)
- A self-completing script that never reports done aborts instead of passing.
  (test: `expired_self_completing_runway_error_exits`)
- Looping restarts the cycle while other collectors are pending and stops the
  moment they clear. (test: `loop_while_pending_resets_and_finishes_early`)

## Notes

- Parent: `20260802-120019`. Depends on the completion port.
- The `.after(InputSystems)` ordering is load-bearing: input collection clears
  `just_pressed` every frame. The press test therefore needs the real
  `InputPlugin` in the rig - without it `InputSystems` is empty and the test
  would pass with the ordering deleted.
- Source: `bevy-common-systems/src/debug/harness/autopilot.rs`.
- Time rig precedent: `crates/nova_gameplay/src/hud/emphasis.rs:287` -
  `Time` is rewritten from `Time<Real>` every frame, so a hand-advanced clock
  is stomped; `TimeUpdateStrategy::ManualDuration` is the seam that holds.
- Env rig precedent: `crates/nova_assets/tests/portal_install.rs:63`.
- Confirm at build time whether `MinimalPlugins` already carries
  `StatesPlugin`; add it explicitly if `init_state` transitions do not apply.
- No new dependencies: `bevy` covers `InputPlugin` and the time strategy.
- Loop semantics kept as-is: the early finish is gated on `loops > 0`, so a
  non-looping run still ends at its timeline's end; `loop_while_pending` +
  `self_completing` still warns and drops the loop.

## Close-out

**What/why.** `crates/nova_autopilot/src/autopilot.rs` now carries the scripted
driver: `AutopilotPlugin<S>`, `AutopilotLoop`, the private `AutopilotState<S>`
and the `autopilot_drive::<S>` exclusive system, behavior-identical to the BCS
source. The only deliberate deltas are the arming env
(`AUTOPILOT_ENV = "NOVA_AUTOPILOT"`, declared here, mirroring
`completion::DEADLINE_ENV`) and the completion path (`crate::completion`).
`lib.rs` lost the outer `///` on `pub mod autopilot;` for the same reason the
`completion` line lost its own - an outer doc concatenates ahead of the
module's `//!` docs and re-resolves their intra-doc links in lib scope.

**Alternatives.** Populating the prelude here was rejected: it is
`20260802-183355`'s job, so the doctest imports by full path instead. Keeping
the BCS `hold`/`input` API unchanged was deliberate - the migration task
(`20260802-183403`) rewrites call sites for the env rename only, not for a new
shape.

**Difficulties/diagnosis.**
- `MinimalPlugins` does NOT carry `StatesPlugin` in Bevy 0.19
  (`bevy_internal-0.19.0/src/default_plugins.rs:160`), resolving the plan's
  open question: the rig adds it explicitly.
- Two first-run failures, both test-rig bugs rather than port bugs.
  `Messages<AppExit>` is double-buffered, so an exit written mid-run is gone by
  the time a later frame drains it - `run()` now drains per frame and
  accumulates. And `Update` never observes the default state: the driver's
  first frame sets the first step and `StateTransition` applies it before
  `Update` runs, which is exactly the "no spurious OnExit/OnEnter" property the
  driver is written for, so the expectation is `[Playing, Over]`.

**Evidence.**
- `cargo test -p nova_autopilot`: 10 lib tests + 1 doctest, all green.
- Falsified the load-bearing ordering: flipping `.after(InputSystems)` to
  `.before(InputSystems)` fails
  `input_closure_press_survives_input_collection` and nothing else.
- `cargo fmt --check`, `cargo clippy --all-targets`, and
  `RUSTDOCFLAGS=-Dwarnings cargo doc --no-deps` are clean for the crate. The
  rustdoc run caught a `redundant_explicit_links` on
  `[`completion`](crate::completion)`; the link is now bare.
- `rg 'BCS_|bevy_common_systems' crates/nova_autopilot/` hits only the crate
  doc sentence that names what the crate must NOT depend on.

**Reflection.** The rig, not the port, was the whole cost - two of four tests
passed on the first compile and the other two failed on Bevy's message and
state-transition timing. Worth carrying into the screenshot/reel ports: build
the per-frame exit-draining `run()` helper first, and expect state assertions
to start one transition in.
