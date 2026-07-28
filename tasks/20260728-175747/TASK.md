# Contextual HUD: show-by-relevance + grow-in-use

- STATUS: OPEN
- PRIORITY: 34
- TAGS: v0.9.0,ui,hud,gameplay

## Story

Full HUD always-on feels bloated; no HUD is unplayable; the `~` levels are a
manual workaround. Per the accepted spike ruleset, make importance-driven
visibility automatic: elements appear when their situation is live (combat
lock, weapons hot, AP burn, objective posted) and grow while in direct use,
shrinking back after. Decide what happens to the `~` tiers (keep as an
override, or simplify) per the spike.

## Steps (direction-level - refined at spike close)

- [ ] DIRECTION: implement the spike's visibility/emphasis ruleset on top of
      the existing HudTier machinery (Instrument/Chrome/Status).
- [ ] DIRECTION: size-emphasis animation for in-direct-use elements per
      demo 2, with revert timing per the accepted ruleset.
- [ ] Refine into real Steps/DoD from the accepted SPIKE.md before any
      implementation.

## Definition of Done (direction-level - refined at spike close)

1. Refined at spike close. Must include at minimum: App-driven tests of the
   visibility rules (situation event -> shown/grown -> reverts), a
   `cargo run -p nova_probe -- run <example>` pass on a playable example, and
   a manual owner playtest verdict.
