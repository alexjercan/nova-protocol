# Prototype-reference lint: unknown section prototype ids must fail the mod gates

- STATUS: OPEN
- PRIORITY: 55
- TAGS: v0.7.0, modding, testing

## Goal

`SectionSource::Prototype("<id>")` resolves at SPAWN, so a scenario
referencing a prototype that does not exist loads green through every
gate (webmods_validation, the portal generator) and ships ships that
half-spawn. Found while authoring The Ledger (task 20260716-123535):
`Prototype("basic_hull_section")` - which does not exist - passed
validation; only a manual catalog cross-check caught it. Add a lint that
walks loaded content's prototype references against the section registry
(base + the mod's own sections) in webmods_validation (and/or the portal
generator if it can know the shipped catalog's sections), failing loud
with the unknown id.

Direction-level; /plan breaks it into steps when picked up.

## Notes

- Discovered in: tasks/20260716-123535 close notes.
- The declared-but-not-loaded lesson family; same class as the
  validate-in-every-domain ledger entry.
