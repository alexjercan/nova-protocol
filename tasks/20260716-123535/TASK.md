# Scenario content pack: two new playable scenarios, one published as a second portal mod

- STATUS: OPEN
- PRIORITY: 60
- TAGS: v0.7.0,scenario,content,spike


## Goal

Fill the v0.6.0 authoring platform with real content: two new playable
scenarios beyond the capital-combat vertical slice, each a distinct fantasy
rather than another mechanics tutorial (candidates to weigh when planning:
a defense/escort stand where the PDC screens a convoy, a salvage run gone
wrong, an asteroid-belt ambush). At least ONE ships as a second published
portal mod (webmods/, after gauntlet) so the portal pipeline - authoring,
nova_portal_gen validation, install-over-the-wire, enable/merge - gets
dogfooded end to end by a real release artifact.

Direction-level; /plan breaks it into steps when picked up. Authoring is
hand-written RON per the proven gauntlet path (the editor builder stays
backlog). Reuse the asset variety pack (20260716-123544) so the two
scenarios look like different places; coordinate with the vertical slice
(20260708-203659) on which fantasy it already covers.

## Notes

- Spike: tasks/20260716-122954/SPIKE.md (v0.7.0 release scope)
- Plan: docs/plans/20260716-v0.7.0-plan.md, strand 1
- If either scenario wants a shared enemy ship definition, that is the
  consumer that activates ship prototypes (20260714-134115).
