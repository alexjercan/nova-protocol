# Review: UI-rework spike - HTML demos (menu widget language + contextual HUD)

- TASK: 20260728-175726
- BRANCH: spike/ui-rework-demos

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

Out-of-context reviewer (fresh subagent, no sight of the implementing session)
ran the DoD `cmd:` proofs and inspected both demos, the icon import + NOTICE,
SPIKE.md/DECISION.md and the four refined child tasks. In-session re-verified the
load-bearing asset-path claim: the `ls` DoD proof passes and all six key-glyph
icons the HUD dock/cues reference (`T_{G,O,R,X,Z,Space}_Key_Alt.png`) exist on
disk; no referenced icon src is missing.

Verified clean (no findings): both demos are self-consistent (all situation
buttons + screen tabs wire to handlers; hash-init replays state; skin toggle
covers both skins); asset paths resolve; the scenario scroll fix is delivered
(`.list-scroll { overflow-y: auto }`); the NOTICE is accurate (CC0, author/
source, only the three keyboard styles, 98 png + 1 svg each, no gamepad leakage);
SPIKE.md/DECISION.md match what the demos show; all five children carry refined
Steps/DoD citing D1-D5 with no direction-level placeholders left; ticked steps
map to real artifacts; the two spike-seeded follow-up tasks exist.

- [x] R1.1 (NIT) tasks/20260728-175726/DECISION.md - the `- STATUS:` frontmatter
  that makes `tatr check` parse the decision was added in a follow-up commit
  after the initial spike-close commit shipped a bare prose `STATUS:` line.
  Already fixed on the branch; captured as a lesson (DECISION.md must use the
  bulleted `- STATUS:` list form, matching every other passing DECISION.md).
  - Response: Fixed before review completed (frontmatter block added). Lesson to
    fold at flow Finish.
- [x] R1.2 (NIT) examples/ui/hud_rework_poc.html - the comms ~5s dwell and
  objective ~1.2s pop are magic numbers; they are the durable spec and correctly
  mirror SPIKE.md sec 3 and child 175747's Steps. No change needed.
  - Response: Intentional; the timings are the accepted ruleset, carried into
    175747 for implementation.

### Pending manual items (owner sign-off, cleared at flow Finish)

1. DoD #1 manual: owner reviewed and accepted both demos in a browser.
   (Confirmed live: three review passes on demo 1, two on demo 2.)
2. DoD #2 manual: owner sign-off on the SPIKE.md accepted directions.
   (Confirmed live across the review passes.)

Both manual items were effectively cleared during the interactive spike (owner
drove the demo iterations and accepted each); recorded here for the Finish gate.
