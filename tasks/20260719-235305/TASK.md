# Spike: harness exit coordination - collectors (autopilot/capture/screenshot) negotiate the exit instead of racing clocks; scene looping for measurement windows

- STATUS: CLOSED
- PRIORITY: 65
- TAGS: v0.8.0,spike,tooling,testing


## Outcome (2026-07-20)

SPIKE.md written, adjudicated with the user (per-example loop opt-in;
reload frames excluded from scene stats and reported as their own line;
fps pass ALWAYS split - the review found fps-on-clean-pass numbers were
contaminated by recorder flush I/O), and honestly reviewed (R1-R5: the
assert-then-done ordering gap, upstream blast radius, deadline
arithmetic, the T2 self-correction, the demoted net's residual scope).

Cut: S1 20260720-000609 (p62, bcs completion protocol + adoption),
S2 20260720-000616 (p61, split fps pass + scene looping + reload lines);
20260719-233732 re-slotted to p59 as the safety net. Flow starts on S1.
