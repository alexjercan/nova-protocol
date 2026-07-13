# Retro: Drive section placement in the editor autopilot

- TASK: 20260708-113000
- BRANCH: feat/editor-autopilot-placement
- PR: #46 (open against master, not merged)
- REVIEW ROUNDS: 1 (APPROVE)

See `tasks/20260708-113000/TASK.md`. The "possible extension" the 20260708-100000 retro named,
built the next turn - and it worked first try headless.

## What went well

- Chose the faithful path (real picking) and it paid off. The tempting shortcut was to expose an
  editor placement API and call it directly; instead I drove the actual `PointerInput` ->
  physics-raycast -> `on_click_spaceship_section` pipeline. That required zero editor changes (all
  public seams) AND gives real end-to-end coverage of picking + projection + placement. Faithful
  turned out to be *less* invasive than the shortcut, not more.
- Verified the preconditions before committing to the approach. Three facts decided feasibility
  and I checked each up front: picking is avian `PhysicsPickingPlugin` (so sections are pickable
  via their colliders), the editor camera is static during the autopilot (so `world_to_viewport`
  is deterministic), and section selection fires on `Add<Pressed>` (so inserting `Pressed` selects
  without a real UI click). Any one being false would have changed the design.
- Compiler-drove the bevy-0.19 API drift instead of spelunking. The registry `find`s kept coming
  up empty, so rather than fight the filesystem I wrote the plausible API and let the two compile
  errors (`Camera.target` is gone; `RenderTarget` is now a separate component) correct me. Two
  fast iterations beat a long doc hunt.
- Proved determinism before shipping. Placement timing (collider readiness, hover-before-press)
  is the kind of thing that is fine once and flaky forever; running it 3x and seeing 1->2 each
  time is what makes this a test worth keeping rather than a coin flip.

## What went wrong

- Nothing substantive. The only friction was the bevy API drift (`RenderTarget` moved off
  `Camera`), caught immediately by the compiler.

## What to improve next time

- For headless UI/input tests, spend the first five minutes confirming (a) which picking backend
  is active, (b) whether the driving camera is static, and (c) what state-change signal the target
  action listens for (`Activate` vs `Add<Pressed>` vs a resource). Those three answers determine
  whether pointer simulation is even viable and how to set state, and they are cheap to check.
- When a crate's source is hard to locate in the registry, prefer writing the call and reading the
  compiler error over grepping - the error names the exact type/path.

## Action items

- [ ] Possible extension: place thruster/turret sections too (exercises the input-binding branches
      of `on_click_spaceship_section`), and/or add an editor screenshot baseline.
- [ ] The pre-existing `hull_section.rs` `struct update` warning is still open (filed in the
      133008 retro) - now long-standing; worth a one-line cleanup task of its own.
