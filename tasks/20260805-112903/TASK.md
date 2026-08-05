# Re-light and re-frame screenshot_sections: the five wiki section closeups

- PRIORITY: 69
- TAGS: v0.10.0,screenshot,examples
- ACTIVITY: -
- GATES: -
- RESOLUTION: -
- PARENT: 20260805-105154
- DEPENDS ON: 20260805-112749

## Context

The five `wiki-section-*` macro closeups of the refresh (`20260805-105154`).
`screenshot_sections` already does the right thing structurally: it builds one
ship carrying all five ENGINE section types (controller, hull, thruster, turret,
torpedo bay), freezes the scene, and steps the reel camera to a closeup of each.
It keeps its shape - these are documentation figures, and a frozen ship on a
clean backdrop is exactly right for them.

What it lacks is light. Every scenario gets one straight-down
`DirectionalLight` (`crates/nova_scenario/src/loader/lifecycle.rs:203`), which
flattens a macro shot worse than any other framing: no rim, no separation from
the skybox, no read on the turret's stacked yaw/pitch/barrel geometry. This task
puts the kit's three-point rig on it and re-frames the five beats.

Deliberately NOT Kenney hulls: these figures document the engine's section
prototypes, which is what the wiki pages describe.

Depends on the photo kit from `20260805-112749`.

## Steps

- [ ] Put the kit's three-point rig on the scene, tuned for macro work (rim
      light carrying the silhouette against the skybox).
- [ ] Re-frame the five beats against the lit ship: `wiki-section-hull`,
      `-controller`, `-thruster`, `-turret`, `-torpedo-bay`. The turret is the
      hard one - its yaw/pitch/barrel stack needs an angle where all three read.
- [ ] Choose the backdrop deliberately: which of the two shipped cubemaps
      (`cubemap.png`, `cubemap_alt.png`) sits quietest behind a macro subject.
- [ ] Confirm the scene still freezes for every beat - a drifting subject is
      what a macro shot cannot afford.
- [ ] Hand it to the owner: run plainly, inspect each section closeup, verdict.

## Definition of Done

- The example builds and the catalog agrees with disk.
  (cmd: `nix develop --command cargo check --examples --features debug`)
- The scene reaches `Playing` headless without a panic.
  (test: `screenshots_reach_playing_without_panic`)
- The report still names `screenshot_sections` for all five shots.
  (cmd: `nix develop --command python3 scripts/gen-web-screenshots.py --report`)
- The owner inspects all five framings and accepts them as good enough to shoot.
  (manual: `cargo run --example screenshot_sections --features debug`, no NOVA_REEL)

## Notes

- No PNG is captured or committed in this task.
- Engine section prototypes, not Kenney cubes: the wiki pages document these
  five types.
- Smallest of the six scene tasks - it is a lighting and framing pass, not a
  rebuild.
