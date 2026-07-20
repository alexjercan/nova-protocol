# Retro: Balance audit rig

- TASK: 20260717-112656
- BRANCH: work/balance-audit (landed 8d445b0b)
- REVIEW ROUNDS: 2 (REQUEST_CHANGES -> APPROVE; 1 MAJOR + 5 MINOR)

## What went well

- Running the rig against real content BEFORE freezing its rules paid
  immediately: the first run's shakedown false-positive (a 395u mook vs a
  400u blanket) drove the self-scaling own-envelope predicate, and the
  refined rule surfaced one genuinely unknown finding (ch4's Auditor)
  while grading everything else clean. The tool was calibrated by its own
  first report, not by intuition.
- Routing the ch4 finding to a follow-up task with exact numbers instead
  of rebalancing the finale mid-cycle kept the cycle scoped and the
  drama-vs-fairness call where it belongs (a playtest).
- The only-flow-cycle with a round-1 REQUEST_CHANGES: the reviewer's
  torpedo-envelope MAJOR was a real rule evasion (tube-only ambushers),
  provable and fixable in one pass, with the fail-first test now
  permanent.

## What went wrong

- The MAJOR: I modeled the threat a hostile poses by its TURRETS because
  the dps number came from turrets - the torpedo tube was counted but not
  priced into the envelope. Root cause: the metric struct's field
  (max_effective_range) quietly became the RULE's input without re-asking
  "what makes a ship dangerous at range" - the tube's AI launch envelope
  (1000u, cooldown starts elapsed) was already documented in the ai.rs
  constants I had read in cycle 1.
- "Sustained dps" overclaimed (magazines + reloads = ~62%); the reviewer
  had to derive the reload math I should have priced or disclaimed when
  naming the number.

## What to improve next time

- When a derived metric feeds a graded RULE, re-derive the rule's meaning
  from the engine's decision constants (what does the AI actually DO at
  this range?), not from whichever fields the metric struct happens to
  have.

## Action items

- [x] LESSONS.md: new lesson rule-inputs-rederive-from-engine (x1).
- [x] Follow-up task 20260717-143806 (ch4 Auditor) carries the numbers.
