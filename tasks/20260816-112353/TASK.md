# Loading screen: measure what blocks, then show life

- STATUS: IN_PROGRESS
- PRIORITY: 69
- TAGS: v0.11.0,ui,performance,scenario

## Goal

Owner: "maybe we can also investigate the load screen is blocking, I would like
it to be like 'Loading ...' and something moves on the screen". They also called
the between-scenario load "the only minor inconvenience" in an otherwise fine
flow, so this is polish on a real annoyance.

## Investigate before fixing

Find out whether scenario loading actually blocks the main schedule, and where.
Do not assume. Candidates:

- asset loading not properly awaited through `AssetServer`
- the scenario spawn burst - a clad ship is ~400 colliders, and several ships
  plus asteroids land in one frame
- avian3d collider generation from meshes, which is synchronous
- the derived ship skin, which runs once per ship at spawn and generates plate
  geometry - new cost this release
- first-draw shader and pipeline compilation, which looks exactly like a hang

`nova_probe` documents an env-gated real wall-clock per-frame capture. Use it.

## The trap

**If the schedule is blocked, a spinner cannot spin.** It would freeze at
precisely the moment it is meant to reassure. So a truthful indicator requires
the blocking work to be chunked across frames first; the animation is a symptom
of the real fix, not a substitute for it. If loading does not block and is merely
slow, a spinner is correct on its own.

## Definition of done

- where the load time goes, with numbers - the most valuable output even if the
  fix is small
- whether it truly blocks or is merely slow
- two captured frames that DIFFER, proving the animation moves
- what remains blocking, if anything, and the size of finishing it

## Lane

sprout `load-screen`.
