# Document entity-filter id vs other_id semantics in the scenario docs

- STATUS: CLOSED
- PRIORITY: 40
- TAGS: docs, web, modding

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

- [x] Re-verify the per-event table and the "actions ignore info" nuance against
      the code (the anchors above may drift).
- [x] In `web/src/wiki/dev/guide-author-scenario.md`: expand the filters section
      with the subject-vs-other explanation, the per-event table, the shakedown
      OnEnter example, and the "filter-only, actions do not consume it" note.
- [x] In `web/src/wiki/dev/scenario-system.md` (reference): add a short
      cross-referenced note on the same fields (link to the guide section).
- [x] (added, user feedback) Rework section 7 "Load and test it": the honest
      mod-content play-test loop, tie it to Make & publish a mod, drop the
      misleading "exercise it headless" (08_scenario builds config in Rust).
- [x] Verify: `npm run ci` green; serve + eyeball the rendered table + section 7.

## Notes

Docs-only, on the markdown wiki pipeline (20260715-195621). No code change. Do
not invent semantics - every row traces to a firing site.

Mid-work the user expanded scope: section 7 was "kind of trash" - "exercise it
headless" made no sense (the `08_scenario` example builds its `ScenarioConfig`
in Rust, so it cannot test an authored RON file), and "ship it as mod content"
needed to lean on the Make-and-publish-a-mod guide (the two pages are two halves
of one job). Reworked it around the real loop: a scenario is mod content, so
testing = list it in a bundle (base is always enabled, quickest), boot into it
(there is NO scenario picker yet - task 20260715-200828 is still OPEN - so you
repoint `NEW_GAME_SCENARIO_ID` or chain in via `NextScenario`), and watch events
with `DebugMessage` + `--features dev`; shipping to others is the make-a-mod
flow.

### What changed and why
- guide-author-scenario.md section 3 (Entity): added the subject vs other-party
  framing, a per-event table (verified against nova_events event infos + the
  firing sites in area.rs/asteroid.rs/loader.rs), the OnEnter classic pairing,
  the shakedown RON example, and the KEY nuance that the fields are filter-only
  and never reach actions (actions.rs 1675/1776 take `info` but ignore it).
- guide-author-scenario.md section 2: trimmed the old id/other_id aside to a
  forward-link to the new table.
- guide-author-scenario.md section 7: full rework (above); section 8 gains a
  "no scenario picker yet" sharp edge.
- scenario-system.md (reference): the Entity filter bullet now states the
  subject/other + filter-only semantics and links the guide table; the pair-event
  aside no longer implies the subject is always the same kind.

### Verification
`npm run ci` green; served + headless-eyeballed the guide - the per-event Entity
table renders (columns Event / id-type_name subject / other_id-other_type_name
other party), and the section-2 forward link + section-7 rework render correctly.

### Self-reflection
The user's instinct was right that guide + make-a-mod are coupled; the fix was to
make section 7 explicitly the "short version" of make-a-mod rather than duplicate
it. Worth checking whether a task should own a note about the still-missing
scenario picker (200828) so the docs can drop the `NEW_GAME_SCENARIO_ID` hack
once it lands.
