# Every example is playable by hand, or says why it is not

- STATUS: OPEN
- PRIORITY: 70
- TAGS: v0.11.0,example,docs

Epic: `20260818-220812`. Owner: "I want some examples where both the player and
autopilot can play them: e.g I want to be able to carve an asteroid by hand,
but also have the autopilot do it for screenshots/gif; but there are a lot of
examples that only work for autopilot, some of which are fine (e.g the
screenshot_combat one it's kind of made to be player only by autopilot for
screenshots, but there are some which don't do anything if I load them as a
player, maybe at least note in their description the purpose)".

## The two outcomes

Every example ends up in one of exactly two states:

1. **Playable.** A human loads it and can do the thing it demonstrates, with
   the autopilot as an ALTERNATIVE driver for captures - not the only driver.
2. **Declared rig.** It is a capture or gate harness and says so, in one line,
   in its description, where a human reading the list sees it before loading
   it.

There is no third state. An example that silently does nothing when a human
loads it is the defect being fixed.

## Audit

`examples/screenshots/` (20) and `examples/systems/` (23). For each: load it as
a player, and record which state it is in and what it would take to reach state
1. The audit is the deliverable of the first pass - do not start converting
before the list exists.

`carve_asteroids` is the owner's named example of one that SHOULD be playable:
carving a rock by hand is the thing the whole destruction epic shipped, and
right now it can only be watched.

## Where the line falls

`screenshot_combat` is explicitly fine as a rig - a scripted shot needs a
scripted camera, and making it playable would break the shot. Do not convert
rigs for symmetry. The test is whether a human loading it would expect to do
something; if the name promises a verb, it owes the verb.

## Also

- The description is the surface a human reads. Whatever lists examples for the
  player must show it - check that the description actually reaches them and
  fix it if it does not.
- Examples doubling as gates keep their gates. Playability is added alongside
  the assertions, never instead of them.

## Done when

- The audit table exists in this task, every example classified.
- Every state-1 example is loadable and playable by hand, verified by loading
  it - not by reading the code.
- Every state-2 example says what it is for, in one line.
