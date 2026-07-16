# Review: Tighten and re-section CHANGELOG.md

- TASK: 20260716-102950
- BRANCH: changelog-revamp

## Round 1

- VERDICT: REQUEST_CHANGES

Verified the regrouping is otherwise sound: no leftover Added/Changed/Fixed
headers, all 12 version headers and all 11 compare-links intact, subsystem
categorization consistent (canonical order applied), the one format break
tagged **(breaking)**, entries genuinely one line each. Cross-checked ~44
distinctive facts/numbers (topological deps, `SetSkybox`, WebGL2->WebGPU,
sha256, rev 4c81117, 17-24%, view_formats, 16384/24576, max_torque 100 -> 40,
lock range 2 km -> 20 km, PDC 20 -> 4, avian3d 0.7, x86_64-apple-darwin, ...)
against the new file - all present except one.

- [ ] R1.1 (BLOCKER) CHANGELOG.md 0.6.0 - the "Screenshot Reel capture set no
  longer ships in the game assets" entry (old 0.6.0 `### Changed`) was dropped
  entirely; it maps to no line in the new file. This violates the task's
  "no information lost" goal. Restore it as a one-line entry under a new
  `### Internals & Tooling` section for 0.6.0 (placed after `### Fixes`, per the
  canonical order used elsewhere): the capture scenario moved into the example
  that films it, so players and the web build stop downloading a capture tool.
  - Response: Restored as a one-line entry under a new `### Internals &
    Tooling` section for 0.6.0 (CHANGELOG.md:51), placed after `### Fixes`.

## Round 2

- VERDICT: APPROVE

- [x] R1.1 - verified: the Screenshot Reel entry is back at CHANGELOG.md:51 in
  a 0.6.0 `### Internals & Tooling` section, correctly ordered after Fixes.
  Bullet reconciliation now closes exactly: 93 content bullets = 94 original
  minus the one deliberate 0.2.0 merge ("game events + queue" and "scenario and
  modding capabilities" combined into one line), confirming no other silent
  drops. No new problems introduced. Task goal met.

