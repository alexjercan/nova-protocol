# The editor build UI runs at 118 ms mean and hitches for 2.4 seconds

- STATUS: OPEN
- PRIORITY: 92
- TAGS: v0.11.0,performance,bug,editor

Epic: `20260818-220812`. Found by `20260818-221027` while auditing what the
probe actually measures. Evidence: `tasks/20260818-221027/REPORT.md` section 2.

## The finding

The editor's BUILD UI - the surface a player uses to assemble a ship - runs at a
**mean of 118 ms with a 2378 ms worst frame**, dev build, 1280x720.

That is a worse tail than any stress range in the tree, on a screen a player
sits in for minutes at a time, and it was on nobody's list.

Measured, one run:

```text
21:34:41  nova perf: warm-up done, capturing 900 frames
21:36:27  nova perf: label=ship_editor mean=117.851ms max=2377.557ms
21:36:48  on_load_scenario: loaded scenario 'editor_sandbox'
```

## How it hid

`probe run ship_editor` has been writing a `frametime.csv` row labelled
`ship_editor` whose numbers are the build UI, and no reader could tell. The
capture opens on `GameStates::Playing`, and the EDITOR runs inside `Playing`
(`crates/nova_editor/src/lib.rs:107`), so the window closed 21 seconds before
the sandbox it was named after even loaded.

`FrameTimePlugin::ready_when` (landed with the harness) fixes the aim - the
capture now holds until `CurrentScenario` names `editor_sandbox`. The
consequence is that the build UI's cost is now UNMEASURED again, because the
number that was accidentally covering it has been pointed somewhere else. It
needs its own case.

## What to do

1. Give the build UI its own profiling case and its own budget. It is a player
   surface; it belongs in the coverage table as one.
2. Then find the 2378 ms frame. A 2.4 second hitch is not a tuning problem, it
   is one thing. The gallery, the previews and the palette all render real
   meshes; a preview render or a palette rebuild on selection is the obvious
   first place to look, but this has not been profiled and the guess is worth
   nothing without a measurement.
3. The mean matters separately from the tail. 118 ms is 8 fps as a STEADY
   state, which suggests the UI is doing per-frame work proportional to
   something it should be caching.

## Do not confuse this with `20260819-001252`

That task is the sandbox collapsing to 2 fps after 30-45 s of play. This is the
build UI, before Play is ever pressed. They are different surfaces and there is
no evidence they share a cause - though if one turns out to explain the other,
that is worth knowing and either task can absorb the other.

Note the tension to resolve: the 2 FPS investigation found that pressing F1 from
the collapsed sandbox returns to the editor at **65 fps**. That is the same
editor. Either the two measurements disagree, or the build UI is only expensive
under the gallery interaction the `ship_editor` walk drives. Establish which
before optimising anything.
