# Make screenshot_combat deterministic with scenario chapters

- STATUS: CLOSED
- PRIORITY: 96
- TAGS: v0.11.0, bug, testing, screenshots, autopilot, scenario

## Problem

`screenshot_combat` is one long live fight. Later screenshot beats inherit
uncontrolled state from earlier beats: active fire, rounds already in flight,
damaged or drifting ships, debris, blasts, and temporary effects.

The failure appears at `track the torpedoes in`: it intermittently reaches its
12-second deadline because the salvo disappears before any torpedo enters the
capture range. Existing logs prove launch/commit drift:

```text
combat: 2 torpedo bay(s) firing
combat: 1 torpedo(es) committed to the raider
```

Production behavior explains it:

- A torpedo has two 1 HP child sections.
- Any section death quietly shoots down the complete torpedo.
- Bullets can hit torpedoes. Projectile collision filtering excludes only the
  firing ship, not unrelated fire.
- Launch and scripted target commitment currently occur in separate steps, so a
  torpedo can die before commitment.

A longer timeout or retries would hide the problem. Production interception
must remain valid gameplay.

## Goal

Keep one `screenshot_combat` example and one visual story, but implement its
major beats as separate in-memory `ScenarioConfig` chapters. At each boundary,
load the next purpose-built scenario, wait for its cast and systems to become
ready, then run only that chapter's screenshot sequence.

This is the same example and process, not separate binaries or catalog content.
The chapter configs can remain private Rust builders in `screenshot_combat`.
They do not need to appear in the scenario picker.

## Why scenario chapters

`LoadScenario` already tears down the prior scenario's `ScenarioScopedMarker`
entities and resets scenario world state, objectives, story feed, emphasis, and
outcome before spawning the replacement. Therefore bullets, torpedoes, ships,
asteroids, debris, blasts, and scenario-owned temporary effects should leave
with the old chapter by ownership, rather than through a combat-specific cleanup
list.

The script still needs a readiness barrier because teardown and spawn use
deferred commands. A chapter load is complete only when:

- the new `CurrentScenario` id is active;
- required named actors and sections exist;
- the prior chapter's named actors are absent;
- transforms and physics have settled for the required frame count;
- chapter-specific preconditions hold.

If a transient survives scenario unload, treat that as an ownership bug and fix
its scenario scoping. Do not add an ever-growing screenshot cleanup list.

## Proposed chapters

1. **Approach**
   - Player, beacon, corridor, and hollow.
   - Real flight into the ambush reveal.
   - Ends after the arrival/establishing screenshots.

2. **Gun exchange**
   - Fresh player, raider, and selected supporting ships in authored poses.
   - Runs the radar lock, tracer, readability, and section-damage beats.
   - Hidden production-path damage injection is valid staging. Name the step
     for the visual result it prepares; it does not need to claim emergent AI
     damage.

3. **Ordnance**
   - Fresh lance and raider in exact authored poses, with only the actors needed
     for the torpedo frames.
   - No PDC/turret crossfire exists because this chapter does not spawn it or
     activate it.
   - Launch and commit the expected salvo through production bay, guidance,
     arming, fuze, and blast paths.

4. **Aftermath/neutralized wreck**
   - Either continue directly from the proved ordnance detonation when that
     continuity improves the image, or load a fresh staged aftermath chapter.
   - Hidden injected section damage is allowed. It must use the production
     damage/destruction path and be named as visual staging, not asserted as a
     measured torpedo damage result.

The final implementation may combine adjacent beats when state continuity is
intentional and asserted. It must not carry an uncontrolled live battle across a
chapter boundary.

## Autopilot vocabulary

Start with ordinary `StepBuilder` steps around `LoadScenario`. Add generic
chapter sugar only if the example reveals repeated, stable structure. Do not add
an abstraction solely for the word `chapter`.

A likely sequence is:

```rust
.step("load ordnance chapter")
.on_enter(load_ordnance_chapter)
.until(ordnance_chapter_ready())
.deadline(10.0)
.add()
.step("settle ordnance chapter")
.until(frames(SETTLE_FRAMES))
.add()
```

Chapter transitions must log the chapter id and readiness failures. Readiness
predicates should identify missing actors or leaked prior actors rather than
returning an unexplained false until timeout.

## Ordnance requirements

- The chapter spawns only the lance, raider, camera environment, lighting, and
  background needed for its images.
- Assert exact lance/raider identities and expected torpedo bay count.
- Launch and commit atomically if production scheduling permits. Otherwise use
  an asserted launch barrier: wait for the exact projectile count, then commit
  all before advancing.
- Assert every launched torpedo is committed to the intended raider.
- Track explicit outcomes: live, shot down, detonated, and target damage.
- The pre-fuze screenshot predicate must distinguish successful approach from
  total salvo loss and fail immediately with a useful message when the salvo is
  gone.
- Preserve production vulnerability, guidance, collision, arming, fuze, blast,
  and damage behavior.

## Scope

- Keep torpedoes vulnerable to bullets in normal gameplay.
- Keep the existing example and shipped screenshot names.
- Private in-memory scenario chapters are preferred over new catalog scenarios.
- First implementation uses an approach/gun-exchange scenario followed by a
  private ordnance scenario. Further chapter splits depend on rendered review.
- Do not disable collision or grant global invulnerability.
- Do not add retries or widen deadlines.
- Do not require bit-identical particle placement. Require deterministic actors,
  gameplay outcomes, framing, and bounded transient populations.

## Definition of done

- A test or focused harness proves loading a new chapter removes prior
  scenario-scoped projectiles, blasts, debris, and actors before readiness.
- Each chapter asserts its scenario id, required cast, forbidden prior cast, and
  settled state.
- The ordnance chapter always launches and commits the expected salvo count.
- The torpedo capture is reached through production guidance and fuze behavior.
- Total salvo loss fails immediately with an explicit outcome instead of a
  generic 12-second timeout.
- Hidden damage staging remains production-path and is described truthfully.
- Repeated focused `screenshot_combat` correctness runs complete without
  retries.
- The full screenshot category passes under the no-display/software-render CI
  path.
- Rendered approach, exchange, torpedo, aftermath, and wreck images are opened
  and approved.

## Verification

```bash
nix develop --command cargo run --features debug -- probe run screenshot_combat --correctness-only
nix develop --command cargo run --features debug -- probe run screenshots --correctness-only
```

Run the focused example repeatedly before the category sweep. Exit status alone
is not proof.
