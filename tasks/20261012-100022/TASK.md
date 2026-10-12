# Epic: located jobs on the Map

- STATUS: OPEN
- PRIORITY: 64
- TAGS: v0.16.0,epic,jobs

## User facts
- Stage 4 of the game expansion. Owner direction 2026-10-12: stations and ships post jobs; jobs live on the Map; accept and hand in only while docked; held jobs stay visible undocked. Depends on the world-first strand's open-world events and addressable generated bodies.
- Design: `tasks/20261012-095526/DESIGN.html` (owner-reviewed page, version 19); codebase facts in `tasks/20261012-095526/RESEARCH-CODEBASE.md`.

## Strand
Children carry the `Jobs:` title prefix and the `jobs` tag.
1. Job board on a station and job state in the save.
2. Job types: recover, escort, distress call, neutralize a named hull.

## Done when
- A player takes a job at one station, completes it in the world, hands it in at the same or another station, and the job survives a reload mid-way.

## Children
- `20261012-100023` Jobs: job board on a station and job state in the save
- `20261012-100024` Jobs: recover, escort, distress call and neutralize a named hull
