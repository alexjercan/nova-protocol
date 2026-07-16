# Retro: Decouple portal/publish tests from specific mods

- TASK: 20260716-155839
- BRANCH: refactor/portal-tests-synthetic (landed 77df131b)
- REVIEW ROUNDS: 1

## What went well

- Count-asserted scripted swaps (every replacement printed its count,
  scenario-id survival asserted) made ~40 mechanical renames across four
  files land without a single silent miss.
- The verify phase caught all three surprises (extra file in scope,
  inherited master red, id-charset gate) before review; none reached the
  reviewer as an unknown.
- The fixture design kept every production stage of the wire e2e (real
  generator, real shipped-catalog gate, real transport, real merge) - the
  decoupling cost essentially zero coverage.

## What went wrong

- Two of the three surprises share ONE root cause: head-truncated grep
  sweeps back in the audit/plan phase. The truncation hid
  mod_cache_install.rs's gauntlet usage from THIS task's plan, and hid
  the same file's `contains_key("demo")` guards from 155816's sweep -
  which put a RED TEST ON MASTER for the hours between 564ff12d and
  77df131b. The truncated-sweep lesson was already written during cycle
  1, but only applied FORWARD; the already-poisoned plans in the queue
  were never re-audited.
- The portal generator's mod-id charset (lowercase/digits/dash, no
  underscores) cost one failed run; the fixture was named from scenario
  conventions instead of checking the id gate first.

## What to improve next time

- When a lesson lands mid-flow, immediately re-audit the REMAINING queue
  against it (re-run the sweeps it invalidates), not just future work.
- Fixture ids must be built to the VALIDATING gate's rules, not to
  neighboring conventions; grep the validator before naming.

## Action items

- [x] Ledger: truncated-sweep-is-not-a-sweep bumped to x3 (moves to
      Pending promotions), sibling-change-leaves-stale-fixture x2,
      new mid-flow-lesson-reaudits-the-queue (x1), domain note on portal
      mod-id charset.
