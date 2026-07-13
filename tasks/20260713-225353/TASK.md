# Author the world/meta wiki pages (factions, scenarios, modding coming-soon)

- STATUS: CLOSED
- PRIORITY: 15
- TAGS: web,wiki,content

## Goal

Author the world/meta wiki sub-pages: Factions, Scenarios, and Modding
(coming-soon). Real content from the code where it exists; Modding is a framed
"coming when the modding language lands" page. Each: content, manifest entry,
`.figure` placeholders, webpack registration.

Done when: the three pages are live through the manifest, content is accurate,
`cd web && npm run ci` is green.

## Notes

- Spike: tasks/20260713-225157/SPIKE.md. Depends on 20260713-225324 (infra).
- Factions from the relation model; Scenarios from `nova_scenario` + the shipped
  scenarios (asteroid field, menu ambience, sandbox, Shakedown).

- Authoritative code-sourced content for every page is in the content
  spike tasks/20260714-002013/SPIKE.md (audited from crates/ with file refs);
  author from it and fix the manifest summaries it flags (esp. Flight: no FA
  toggle / Z mode / RCS - manual is Newtonian, autopilot is the assist).
