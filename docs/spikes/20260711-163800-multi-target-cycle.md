# Spike: Multi-target candidate set + target cycling (HUD + input)

- DATE: 20260711-163800
- STATUS: RECOMMENDED
- TAGS: spike, hud, targeting, input

## Question

Task 20260708-165705 (narrowed by the VATS-lite spike to the multi-target
half) wants the player to see the set of lockable ships, not just the single
best pick, and to move the active ship lock between them deliberately. What
is the state model for the candidate set, how is it rendered, and what input
cycles the lock - given that plain scroll is already taken by component
cycling? A good answer picks the set representation, the cycle input and its
interaction with the aim-driven lock, and the HUD treatment, concretely
enough for /plan to break into steps.

## Context

What exists (see docs/spikes/20260709-192358-component-lock-vats-lite.md and
docs/spikes/20260708-165647-weapons-hud.md for the arcs this sits in):

- `update_spaceship_target_input` (input/targeting.rs:268-409) already
  enumerates every lockable body each frame - dynamic bodies with
  signature-gated ranges, hostility via `relation()` (relations.rs) - then
  throws everything away except the single best pick: cone pick first,
  nearest-hostile signature fallback second, with 1.15x incumbent
  hysteresis. The lock lives in `SpaceshipPlayerTargetLock`
  (`Option<Entity>`).
- Component (subtarget) cycling shipped with the component fine-lock:
  `ComponentCycleNextInput`/`ComponentCyclePrevInput` bound to plain mouse
  wheel up/down plus `]`/`[` and gamepad dpad right/left
  (input/player.rs:505-533). A cycle press pins the selection
  (`ComponentLockMode::Pinned`, 2 s window) so aim-snap does not steal it
  back (targeting.rs:553-592).
- The screen-indicator widget (hud/screen_indicator.rs) renders
  entity-anchored, ApparentSize-scaled UI markers with offscreen Hide or
  ClampToEdge behavior. The component-lock HUD and verb cues are consumers.
- The bottom-left keybind cluster (hud/keybind_hints.rs) is a fixed column of
  `[KEY] VERB` rows fed by the `FlightVerbHints` resource; labels are derived
  from live `Bindings`.
- Sibling task 20260708-165704 (off-screen edge indicators) lands in the same
  flow and wants a notion of "things worth pointing at" - the candidate set
  built here is exactly that list, so the two tasks share state.

## Options considered

### Set representation

- **A. Ranked resource snapshot (chosen).** A `SpaceshipPlayerTargetCandidates`
  resource holding the top-N lockable ships, rebuilt each frame by the same
  system that already enumerates candidates. Matches the existing
  resource-per-player-targeting-fact pattern (`TargetLock`, `LockFocus`,
  `ComponentLock`); HUD and cycle input read one place; trivially consumable
  by the edge-indicator sibling.
- **B. Per-candidate marker component.** More ECS-idiomatic, but insert/remove
  churn every frame, ordering must live somewhere anyway, and every consumer
  re-derives the ranking. More moving parts for no benefit at N<=5.
- **C. No maintained set; enumerate at cycle-press time.** Cheapest, but then
  there is nothing to render as a target list, which is half the task.

### Cycle input

- **CTRL+scroll (chosen).** Plain scroll stays component cycling; CTRL layers
  "one level up" (ship vs section) on the same physical gesture, which is a
  clean mental model. User-preferred. Keyboard alt: CTRL+`]`/CTRL+`[` for
  symmetry; gamepad dpad up/down (dpad left/right is components).
- **Scroll overflow (scroll past last component moves to next ship).** One
  gesture for everything, but modal and surprising: overshooting a component
  cycle silently changes ships, which is destructive to the fine-lock the
  player was building (focus resets on lock change). Rejected.
- **Dedicated key only (e.g. T / Tab).** Works, but keys are scarcer than
  modifier+wheel, and it breaks the "same gesture, one level up" symmetry.
  Not taken as primary; can be added later if playtest wants it.

### Cycle vs the aim-driven lock

- **Pin the ship lock on cycle (chosen).** The cone pick recomputes every
  frame, so an unpinned cycled lock is stolen back within a frame. Mirror the
  component-lock solution exactly: a cycle press sets the lock and enters a
  pinned mode with a window (start at ~4 s, longer than the component pin
  since re-aiming a whole ship is slower); aim-driven acquisition resumes
  when the window expires, the pinned target dies, or the player aims at and
  thereby cone-picks a different candidate for a sustained moment. Simplest
  correct rule for now: while pinned, the picker does not overwrite the lock;
  pin refreshes on each cycle press.
- **Cycled lock is permanent until next cycle.** Diverges from the
  look-to-aim feel the game is built on; a forgotten pin means the reticle
  ignores the crosshair forever. Rejected.

### Membership and ranking

- Membership: hostile ships (relation Hostile, `SpaceshipRootMarker`) within
  their existing signature-gated lock range - the same filter the picker
  already applies, so no new range logic. Committed enemy torpedoes stay out
  of the cycle set (they are threats to run from, not targets to cycle to;
  the edge-indicator task points at them instead). The current lock is
  always a member while it remains lockable, even if it falls out of the
  top N by score.
- Ranking: reuse the picker's notion of desirability - angular distance to
  the aim ray first (on-screen, near crosshair ranks high), then distance.
  Top N = 5 (constant, feel knob). Cycle order is the ranked order; a cycle
  press steps from the current lock's index, wrapping.
- Stability: re-ranking every frame can reorder the list mid-cycle. Cheap
  fix: freeze the cycle order snapshot while the pin window is active (the
  list HUD may still re-rank visually; the cycle walks the frozen snapshot).
  Score hysteresis for the rendered order is a plan-time tuning detail.

### HUD treatment

- Candidates render as screen-indicator consumers: a small bracket marker per
  candidate ship (ApparentSize, min fallback), hostile-red but dimmer/thinner
  than the locked reticle so the active lock stays visually dominant; the
  candidate that is the current lock draws no extra marker (the reticle is
  already there). Offscreen: Hide - pointing at off-screen things is the
  sibling edge-indicator task's job, and double-marking would clutter edges.
- Keybind hints: two new rows in the bottom-left cluster, rendered like the
  verb rows: `[SCROLL] COMPONENT` (available when the component layer is
  focused/active) and `[CTRL+SCROLL] TARGET` (available when the candidate
  set has >= 2 entries). Labels derived from bindings where practical, like
  `FlightVerbHints` does.

## Recommendation

Promote the picker's transient enumeration into a `SpaceshipPlayerTargetCandidates`
resource (top-5 hostile ships, ranked aim-angle-then-distance, current lock
always included), rebuilt in the targeting pass right after the existing
enumeration so the work is shared. Render candidates as dim bracket
screen-indicators (Hide when offscreen). Add `TargetCycleNext/PrevInput`
bound to CTRL+mouse wheel (plus CTRL+`]`/`[`, gamepad dpad up/down); a cycle
press sets `SpaceshipPlayerTargetLock` to the next candidate in a frozen
snapshot of the ranked order and pins the lock for ~4 s against the
aim-driven picker, exactly mirroring the component-lock pin. Extend the
keybind cluster with `[SCROLL] COMPONENT` and `[CTRL+SCROLL] TARGET` rows.

Implementation note for /plan: plain-scroll component cycling must not fire
while CTRL is held - bevy_enhanced_input conditions (chord/block) decide
this; verify the chosen combinator actually suppresses the unmodified
binding, it is the one genuinely fiddly bit.

The candidate set is deliberately the data source for the sibling
edge-indicator task (20260708-165704): edge indicators point at off-screen
candidates, the active lock, and committed hostile torpedoes.

## Open questions

- Feel knobs: N (start 5), ship-pin window (start 4 s), whether aiming hard
  at another candidate should break the pin early (start: no, keep the rule
  simple; revisit in playtest).
- Whether the hint row should show the live wheel binding label or a fixed
  "SCROLL" string (the wheel has no key label in Bindings; fixed string is
  fine for v1).
- Gamepad dpad up/down may collide with future menu/cluster navigation;
  acceptable now, revisit with the gamepad pass.

## Next steps

- tatr 20260708-165705 (existing): implements this spike - candidate set
  resource, candidate HUD markers, CTRL+scroll cycle with pin, keybind hint
  rows. Spike link added to the task.
- tatr 20260708-165704 (existing, sibling): consumes the candidate set for
  off-screen edge indicators; same flow.

## Fix record

- 20260711, tatr 20260708-165705: shipped the candidate set
  (`SpaceshipPlayerTargetCandidates`, top-5 hostile ships ranked
  aim-angle-then-distance, `pinned_until` pin state), the bracket overlay
  (hud/target_candidates.rs), CTRL+scroll / CTRL+brackets cycle with a 4 s
  pin, and the two hint rows. Deviation from this spike: gamepad got NEXT
  only on DPadUp - DPadDown was already ORBIT (the collision this doc's
  open question predicted); prev stays keyboard/wheel. The Chord sits on
  binding entities, not the action, so the pad binding needs no modifier.
