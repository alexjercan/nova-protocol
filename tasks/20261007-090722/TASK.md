# Research v0.16.0 gameplay polish against shipped player flows

- STATUS: CLOSED
- PRIORITY: 70
- TAGS: v0.16.0,gameplay,spike

## User facts
- Prioritize enjoyment, clarity, and reliability of shipped play over adding more features. Research first, then propose small evidence-backed fixes.
- The earlier intermittent docking report is resolved from the owner's perspective and cannot currently be reproduced; do not reopen it without a new failure.

## Delivery
- Use the shipped-feature matrix to select a few high-impact core player journeys. Trace each from menu to observable outcome and enumerate friction in onboarding, travel, mining/intake, inventory/trade/repair, combat and UI only where the evidence warrants it.
- Reproduce bugs before proposing fixes. For usability gaps, identify actual controls and rendered states; do not infer player failure from pilot prose or accepted input. For performance, require matched repeats against a named reference before a timing claim.
- Propose a ranked short list: specific symptom, owner/entry point, before/after behavior, blast radius, focused proof and work estimate. Separate small fixes from persistence, item-modability, and other feature work. Obtain approval at each implementation gate.

## Agent findings
- Three polish fixes landed locally (`RESEARCH.md` section 8). Section 9 ranks the remaining source-backed candidates at `2febf1b49`: (A) a camera mode switch leaves the hull-steering rate live, (B) no in-flight player integrity readout, (C) no in-flight mining/TAB cue and no training pointer, (D) refused DOCK/GOTO/ORBIT give no reason. None is reproduced or rendered.
- Done when is not met. The open proof gaps are listed in `RESEARCH.md` section 9.4.

## Done when
- Owner has a code-backed prioritized polish plan and bounded reproduction/visual proof for proposed changes. Research does not authorize gameplay edits.
