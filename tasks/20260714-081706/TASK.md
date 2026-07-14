# Screenshot-showcase example: reel of mini-scenarios with camera choreography that generates the web site's screenshots

- STATUS: CLOSED
- PRIORITY: 55
- TAGS: v0.6.0, example, screenshot, web, testing

Spike: tasks/20260714-081636/SPIKE.md

Goal: a runnable example that plays a scripted reel of small showcase moments -
each sets up a mini-scene (a built ship, an orbit, a combat lock, a section blown
off, the three HUD tiers, the menu backdrop), choreographs the camera to frame it,
settles, and captures a PNG. The concrete north star: produce the ~17 named
screenshots the web site references as empty placeholders (feature-editor.png,
feature-autopilot.png, feature-gravity.png, feature-combat.png, feature-juice.png,
feature-hud.png, tutorial-*, wiki-*, thumb-devlog-*). See the spike doc for the
full list and source pages.

The capture primitive already exists: Bevy 0.19's `Screenshot::primary_window()`
+ `save_to_disk` observer, already wrapped as `nova_screenshot()` /
`ScreenshotPlugin` in `crates/nova_debug/src/harness.rs` and driven by the
autopilot harness the 12 curriculum examples use. So this task is mostly
composition: a sequence of named beats, per-beat camera framing, and writing each
shot to its web asset filename - NOT new capture tech. Reuse `BCS_AUTOPILOT` /
`BCS_SHOT` gating so it can run headless in the smoke suite too. Windowed + real
GPU is the simplest robust path (headless needs lavapipe on CI). The user also
described "spawns different random things and the camera moves to them" - support
a randomized attract-mode variant of the same reel for ad-hoc/marketing capture.

