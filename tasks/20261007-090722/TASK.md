# Research v0.16.0 gameplay polish against shipped player flows

- STATUS: OPEN
- PRIORITY: 70
- TAGS: v0.16.0,gameplay,spike

## User facts
- Prioritize enjoyment, clarity, and reliability of shipped play over adding more features. Research first, then propose small evidence-backed fixes.
- The earlier intermittent docking report is resolved from the owner's perspective and cannot currently be reproduced; do not reopen it without a new failure.

## Delivery
- Use the shipped-feature matrix to select a few high-impact core player journeys. Trace each from menu to observable outcome and enumerate friction in onboarding, travel, mining/intake, inventory/trade/repair, combat and UI only where the evidence warrants it.
- Reproduce bugs before proposing fixes. For usability gaps, identify actual controls and rendered states; do not infer player failure from pilot prose or accepted input. For performance, require matched repeats against a named reference before a timing claim.
- Propose a ranked short list: specific symptom, owner/entry point, before/after behavior, blast radius, focused proof and work estimate. Separate small fixes from persistence, item-modability, and other feature work. Obtain approval at each implementation gate.

## Done when
- Owner has a code-backed prioritized polish plan and bounded reproduction/visual proof for proposed changes. Research does not authorize gameplay edits.
