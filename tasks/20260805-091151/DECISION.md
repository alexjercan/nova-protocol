# Decision: the driven pointer is authoritative for the whole run

- DATE: 20260805-104403
- STATUS: ACCEPTED
- TASK: 20260805-091151
- TAGS: automation, testing, nova_autopilot

## Context

A `click_named` beat intermittently produced no `Activate` (~1 in 3 full
`examples_smoke` runs, in more than one example), stalling the release beat
until its deadline.

The recorded diagnosis - the press landing before the hover resolves - is
WRONG, and the tree says so: window events become `PointerInput` a frame later
in `First` with the Move stamping the Press location
(`bevy_picking/src/input.rs:121-160`), and `ProcessInput -> Backend -> Hover`
is chained (`bevy_picking/src/lib.rs:401-410`). Warping and pressing in one
call is self-consistent.

The real mechanism, reproduced deterministically by injecting one stray
`CursorMoved` between the beats: `Pointer<Click>` - the only path to
`Activate` - dispatches from `previous_hover_map`
(`bevy_picking/src/events.rs:963`), so any pointer event that moves the hover
off the widget between the press beat and the release beat silently cancels
the click. The press is never the problem. The driven pointer simply is not
authoritative: a real X pointer event lands in the same stream and outvotes it.

Full evidence, run commands and numbers: `NOTES.md`. Rig and prototype:
`prototype/`.

## Decision

**Pin the driven pointer.** While a run is driven, `nova_autopilot`
re-asserts its own last synthesized cursor position every frame in `First` -
AFTER `bevy_winit` writes the frame's real events and BEFORE
`PickingSystems::Input` consumes them - so the last `Move` in every batch is
the driver's and the hover maps always agree with where the script pointed.

One place in `nova_autopilot`. No call-site changes, no script churn, no new
vocabulary.

What chose it: it is the only candidate that clears the rig. Under an injected
stray event the `ui/` category goes 4/5 FAIL (the fifth never clicks); with the
pin it goes 5/5 pass, and 5/5 pass without the rig too.

Whatever lands must carry a check that FAILS without the pin - the failure is
intermittent, so a green run is not evidence. The rig in `prototype/` is the
shape of that check; its final form (an integration test in `nova_autopilot`
versus a harness knob) is the plan's call.

## Alternatives considered

- **Observed-state predicates (`hovered_named` / `pressed_named`) instead of
  the pin.** Rejected as the fix: a stray event can still land between the
  observed press and the release, so the rig kills it too. Worth doing on its
  own merits (it retires the epic's frame-count anti-pattern and turns a 90 s
  mystery stall into a beat that names the widget), which is a separate task,
  not this one - YAGNI.
- **Split every call site into hover-then-click beats.** The gap it closes is
  BEFORE the press; the rig shows the press always lands. It also leaves the
  trap armed for the next call site and grows every script.
- **Defer the press one frame inside `click_named`.** Rejected by evidence for
  the same reason, and it needs driver machinery `on_enter` cannot express.
- **One Xvfb display per smoke example.** Narrows one ambient source in CI
  only; does nothing for a local run, and does not close the class.

## Consequences

- A driven app stops responding to a real mouse for the duration of the run.
  Intended: it also fixes a developer nudging the mouse during a local run.
- `nova_autopilot` gains a resource holding the pinned position and a `First`
  system ordered against `PickingSystems::Input`, so the crate now takes an
  ordering dependency on `bevy_picking`'s public set. It already depends on
  `bevy` whole, so no new dependency.
- Open, and stated rather than closed: WHICH ambient X event fires in CI is
  unproven. 218 runs across three ambient shapes (40 sequential, 40 concurrent,
  138 suite-shaped) never produced one; the owner's reproduction was a full
  workspace suite run, which this box did not replicate. The fix is therefore
  robust to the class rather than aimed at one source, and the ambient trigger
  stays an open question rather than a claimed finding.
