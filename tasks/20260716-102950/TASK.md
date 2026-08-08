# Tighten and re-section CHANGELOG.md: terse subsystem-grouped entries

- STATUS: CLOSED
- PRIORITY: 80
- TAGS: spike, docs

## Outcome

CHANGELOG.md rewritten: Added/Changed/Fixed replaced with 11 subsystem sections
(canonical order, empties dropped per release), every entry collapsed to one
scannable line, `X -> Y` deltas, the one format break tagged **(breaking)**, and
the header line rewritten to describe the subsystem grouping honestly. All 12
version headers and 11 compare-links preserved. Review (REVIEW.md) caught one
dropped entry (Screenshot Reel), restored under a 0.6.0 Internals & Tooling
section; bullet reconciliation confirms no other loss. APPROVE in round 2.

## Goal

Make `CHANGELOG.md` scan. Replace the `Added` / `Changed` / `Fixed` axis with
Wesnoth-style subsystem sections, and collapse each multi-line paragraph entry
into a single terse line (use `X -> Y` for numeric deltas). The file stays the
complete, in-repo, greppable artifact for modders and power users; nothing is
dropped, only regrouped and shortened.

Proposed section set (drop any empty per release): Gameplay & Flight; Combat &
Weapons; Ships & Sections; Scenarios & Objectives; Modding & Mod Portal;
Interface & HUD; Web & Platform; Audio & Visuals; Performance; Fixes;
Internals & Tooling. Keep the SemVer note and `[Unreleased]`; rewrite the
"format is based on Keep a Changelog" line to describe the subsystem grouping
honestly instead of claiming strict Keep-a-Changelog.

Done = every existing release re-sectioned and tightened, entries one line
each, no information lost.

## Notes

Spike: tasks/20260716-102940/SPIKE.md
