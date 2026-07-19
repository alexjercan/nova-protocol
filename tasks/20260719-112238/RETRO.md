# Retro: run-timeline recorder (correctness capture)

- TASK: 20260719-112238
- BRANCH: feature/probe-run-recorder (squash-landed as 4512b994)
- REVIEW ROUNDS: 1 (APPROVE; 2 NITs recorded, none blocking)

## What went well

- The pub(super) blocker was handled as a FORK-AND-ASK instead of a silent
  workaround: three options laid out (upstream getters / in-repo fire_probed
  sweep over ~18 sites / defer events), the user picked upstream, and the
  whole bcs v0.19.2 detour (patch, tests, doctest visibility pin, changelog,
  tag, push, five pin bumps) took one sitting. The 4-line upstream fix beat
  both workarounds on every axis; the dependency is the user's own crate,
  which made "fix the tool" the cheap path, not the expensive one.
- Every hook was verified in source BEFORE Steps were written (bevy's
  StateTransitionEvent-is-a-Message, the fire()->trigger path, GameEvent
  field visibility, NovaEventWorld's accessor gap) - and the entire
  recorder + 27 tests went green on the FIRST cargo test run. Zero compile
  surprises on a 600-line module touching four crates.
- The empirical stability probe answered the spike's open question in the
  same cycle (identical meaningful sequences same-host; per-frame pulse
  varies by design), and the very first armed 10_playable run surfaced a
  previously-invisible fact (onenter waypoint/prey at spawn) - the tool
  demonstrated its value before it even landed.
- The Explore agent's system map (delegated while sprouting) made the
  design fast, and its one imprecision - "no StateTransitionEvent-style
  event exists" when bevy provides exactly that as a Message - was caught
  because the map was treated as leads to verify, not facts.

## What went wrong

- Two plan details did not survive contact and were amended in-task: the
  planned nova_events dependency was unnecessary (GameEvent rides
  nova_gameplay's bcs re-export), and the stability-probe subject
  (08_scenario) was not wired for the recorder until mid-implementation.
  Both are the plan-vs-reality gap the amend-the-step rule exists for;
  neither cost more than minutes.
- The wiki run-timeline snippet was written before 08_scenario gained the
  plugin, so it names only 10_playable as wired - still true ("10_playable
  does") but incomplete. Caught nothing; noted for the T5/T6 docs sweep.

## What to improve next time

- When a plan step names a concrete probe SUBJECT ("run 08_scenario with
  the recorder"), verify at plan time that the subject satisfies the
  step's preconditions (is it wired?) - same shape as
  verify-first-plan-steps, applied to test subjects, not just APIs.

## Action items

- [x] Lesson added: upstream-api-gap-fix-beats-workaround (positive).
- [ ] T5 (20260719-112304) inherits the two review NITs: run_end's
      entries-before-end semantics, and optionally folding the per-frame
      onupdate pulse into run_end instead of streaming it.
