# Document entity-filter id vs other_id semantics in the scenario docs

- STATUS: OPEN
- PRIORITY: 40
- TAGS: docs,web,modding

User feedback (2026-07-15): the "Author a scenario" guide (and the scenario docs
generally) list the entity-filter fields `id, type_name, other_id,
other_type_name` but never explain WHY there are two of each, or when to use
`id` vs `other_id`. Add a clear explanation - a table and/or a dedicated section.

## The idea to convey

Entity filters match against the firing event's info, which has a PRIMARY entity
(`id` / `type_name` - the event's subject) and an OTHER party
(`other_id` / `other_type_name`). Which is which depends on the event kind. The
classic example (user's): for `OnEnter`, `id` is the trigger AREA that was
entered and `other_id` is the entity that entered it.

## Grounded spec (verified against code - re-verify during work)

- Filter match: `EntityFilterConfig` (crates/nova_scenario/src/filters.rs:33-119)
  ANDs whichever of the four fields are `Some`, each compared to the event
  info's `data` map keyed "id"/"type_name"/"other_id"/"other_type_name"
  (crates/nova_events/src/lib.rs:24-27). Omitted fields are wildcards.
- Per-event meaning of id / other (crates/nova_events/src/lib.rs event infos;
  firing sites cited):

  | Event | `id` (+ type_name) | `other_id` (+ other_type_name) |
  |---|---|---|
  | OnStart, OnUpdate | (none) | (none) |
  | OnDestroyed | the destroyed object | (unused) |
  | OnEnter / OnExit | the trigger AREA | the entity that entered / exited |
  | OnOrbit | the well being orbited | the orbiting ship |
  | OnTravelLock / OnCombatLock | the locked target | the locking ship (player) |

  Firing sites: area.rs:60-64 / :94-98 (enter/exit), asteroid.rs:170-172
  (destroyed), loader.rs:292-296 (orbit), :394-405 (locks).
- IMPORTANT nuance to call out: actions do NOT read id/other_id - the event info
  is used ONLY for filtering. Actions like SpawnScenarioObject position by their
  own hardcoded config, not "at the entity that entered" (actions.rs:1675, 1776
  take `info` but ignore it). So id/other_id let you GATE on who did what; they
  do not let an action target the subject.
- Real example (assets/base/scenarios/shakedown_run.content.ron ~450-519):
  OnEnter filter `Entity((id: Some("beacon_1"), other_id: Some("player_spaceship")))`
  = "when the player ship enters the beacon_1 area".

## Steps

- [ ] Re-verify the per-event table and the "actions ignore info" nuance against
      the code (the anchors above may drift).
- [ ] In `web/src/wiki/dev/guide-author-scenario.md`: expand the filters section
      with the subject-vs-other explanation, the per-event table, the shakedown
      OnEnter example, and the "filter-only, actions do not consume it" note.
- [ ] In `web/src/wiki/dev/scenario-system.md` (reference): add a short
      cross-referenced note on the same fields (or link to the guide section) so
      the reference is not silent on it.
- [ ] Verify: `npm run ci` green; serve + eyeball the rendered table.

## Notes

Docs-only, on the markdown wiki pipeline (20260715-195621). No code change. Do
not invent semantics - every row must trace to a firing site.
