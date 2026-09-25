# Add verified gameplay statistics to the wiki

- STATUS: CLOSED
- PRIORITY: 55
- TAGS: v0.15.0, docs, wiki, gameplay

## Goal

Give the player wiki clear, exact gameplay statistics without maintaining a
second technical specification in lore. Use compact item tables and useful
variant comparisons, following the style of a detailed game wiki such as
Factorio's. Keep the current prose, widgets, and illustrations useful together.

## Reference

[Technical specification](TECHNICAL_SPEC.md) is the original story-planning
capability audit, preserved byte-for-byte as task reference. It is not a current
specification, world canon, or a document to publish as another reference page.

- Implementation baseline: `91f67d8d12aed96f04414240dda671ae7435ab8c`.
- SHA-256: `d8c5c61954e52d9e1cada836ad41a156318f48ca23ad3d602a50f539ba865d64`.
- Its paths, line numbers, catalog ordering, defaults, and derived figures must
  be checked against the implementation at the time of this task.
- Re-derive claims from runtime behavior and authored content. Do not merely
  copy the tables or verify that a named field still exists.

## Scope

1. Inventory the reference's six sections against existing wiki, creator, and
   developer articles. Record what is already covered, what needs updating, and
   what should be discarded as stale or irrelevant to players.
2. Extend existing wiki pages rather than add a monolithic stats specification.
   Put compact stats and variant tables with the item they describe. Use named,
   current base configurations, not the audit's anonymous catalog-row examples.
3. Separate configured values, derived estimates, and measured results. State
   assumptions for acceleration, firing duration, nominal projectile travel,
   damage output, and other comparisons. Do not call nominal travel distance a
   guaranteed engagement range, or a derived result a measurement.
4. Identify base/default values and explain which can vary with a build, damage,
   scenario, or mod. Use explicit units and player-facing lengths in meters.
   Do not invent kilograms, newtons, explosive yields, or fictional technology
   from internal simulation quantities.
5. Avoid duplicate maintained values. Inspect the existing widget and test
   sources, then choose a small shared data or validation approach so tables,
   calculators, and defaults cannot silently disagree. Keep source provenance
   with numerical claims, including repository paths and line references where
   required by the documentation conventions.
6. Add sparse, clearly labeled world-background links only where lore answers a
   different useful question. Keep controls, numerical gameplay facts, and
   simulation exceptions in the wiki, not in lore.

## Likely article owners

- `web/src/wiki/sections/{thruster,controller,hull,turret,torpedo-bay,railgun}.md`:
  item statistics, fit constraints, and variant comparisons. These already have
  tables and widgets; extend or correct them rather than create parallel copies.
- `web/src/wiki/{flight-autopilot,gravity-wells}.md`: observable flight behavior,
  gameplay limits, and the simulation's gravity rules.
- `web/src/wiki/{combat-weapons,targeting-radar,ships}.md`: damage, targeting,
  defeat, and the practical meaning of weapon and ship figures.
- `docs/{sections,architecture,scenario-system}.md`: unique implementation
  explanations, if still valid and not already documented. Do not move the
  complete historical snapshot into the developer book.
- `web/src/create/`: content fields and scenario-authoring contracts. Player
  tables should not become a second schema reference.
- Story-specific unsupported dependencies belong with season or episode plans.
  The shared season outline already lists its main campaign-development needs.

## Constraints

- Coordinate with the ongoing wiki image work. Preserve its pages, captures,
  captions, and widget explanations; do not regenerate unrelated media.
- Lore owns fictional people, places, technology, and consequences. The wiki
  owns the game and what the player can do. Some brief context may overlap, but
  neither surface should carry a second copy of the other's maintained details.
- No gameplay rebalance, engine changes, or new simulation features are implied
  by this documentation task. Record inconsistencies before proposing changes.
- Keep the coverage audit, data-source decisions, and verification evidence
  with this task. Follow the documentation skill and release-baseline rules.

## Acceptance

- [x] Preserve the technical audit as task-scoped reference, outside lore.
- [x] Record a coverage audit and recheck retained claims against current sources.
- [x] Put useful exact statistics and comparisons on the relevant wiki pages.
- [x] Clearly label defaults, units, derivations, measurements, and assumptions.
- [x] Verify repeated numerical values against their authoritative source.
- [x] Preserve existing images and keep lore and gameplay documentation separate.
- [x] Run affected web checks and inspect rendered tables and widgets at desktop
  and mobile widths, including deployed-prefix links.

## Intake proof

The reference was moved unchanged from `web/src/lore/TECHNICAL_SPEC.md`; its
tracked predecessor was `lore/TECHNICAL_SPEC.md`. Both match the SHA-256 above.
The lore authoring guide no longer points at a separate technical specification.
No wiki article or gameplay code was changed when creating this task.

## Coverage audit

Rechecked against the sources at `ef1d6836b`, not copied from the reference.
C = already current in docs, U = updated here, D = discarded for the player
wiki, K = known gap kept open.

1. Scale, hulls, propulsion.
   - C: 10 m cell and engine unit; drive sizes, impulse and health
     (`sections/thruster.rs`, wiki `sections/thruster.md` table); 8g attitude
     budget (`physics/attitude.rs`, wiki `sections/controller.md`); RCS 5 g
     and 100 m/s (`flight/state.rs:475-476`, wiki `flight-autopilot.md`);
     500 m arrival standoff; gravity rules (wiki `gravity-wells.md`).
   - U: the `thruster-mass` widget and `sections/thruster.md` now label
     structure-only mass and upper-bound acceleration/G; its sprint time is a
     lower bound. Base hulls are clad
     (`ships/mod.rs:303`), plates weigh `SKIN_DENSITY` 0.25
     (`shell_skin.rs:96`), and flight divides by `ComputedMass`
     (`flight/authority.rs:153`).
   - D: the reference's ship acceleration table. Its mass premise omits
     skin, and rows 4-6 no longer match the catalog.
   - D: the manual main-drive speed cap. It is gone (`flight/manual.rs:38-40`).
   - K: no measured clad-ship mass. The railgun recoil scope also uses
     structure-only mass (`widgets.ts` `LANCE_RECOIL_IMPULSE` block).
     Unverified: whether the controller turn scopes need skin inertia.
2. Weapons. C: turret, torpedo bay and railgun tables match current
   builders. U: turret and railgun tables label reach as derived nominal
   travel. U: the stale 60 hp raider mount claim is removed; no base ship
   patches turret health, and the only turret patch is the tutorial trainer
   gun at 1,300 (`scenarios/tutorial/range.rs:88,135`). D: heavy torpedo and
   heavy railgun rows; they are dev fixtures
   (`examples/shared/dev_fixtures/sections.rs`), not base content.
3. Damage. C: kinetic 0.25-2.0 and pierce 0.5-3.0 curves, 300 base power,
   blast falloff and 65% transmission, neutralization. U: the six-layer
   Pierce cap is gone (`damage.rs` `pierce_remainder`); the `round-travel`
   widget runs without it, the v0.11.0 news instance keeps it through
   `data-ruleset="0.11.0"`, and `docs/sections.md` drops it.
4. Targeting and AI. C: signature lock ranges under a 200 km ceiling,
   torpedo contacts at about 25 km, 50 m debris, 1.15 hysteresis, 1.5 s
   fine lock, dwell curve, cover drops both slots (wiki `targeting-radar.md`).
   U: the 30 s idle lock decay is gone (`targeting/contacts.rs:56-58`); the
   radar trainer note and the HUD switchboard drop it, and the switchboard
   derives hot from raised or locked (`targeting/safety.rs:30`). U: enemy
   pilot gates table in wiki `combat-weapons.md` (detection capped at
   20 km and by signature, 4 km engage unless just hit, about 1 km
   standoff, 1.8 km derived fire gate, 1.5 km point defense, torpedo band). D: `Retreat` stub; not player-visible.
5. Campaign capabilities. D for the player wiki; creator scenario pages own
   actions and events. No change.
6. Unsupported systems. D; story planning only. No change.

Other findings, not changed here: base content omits some fields and relies
on Rust defaults (`ignition_delay`, `projectile_health`, turret joint
`speed`; Serpent is `TorpedoTypeConfig::default()`).

Data-source decision: widget constants stay in `web/src/widgets.ts` with
`path:line` comments. `web/tests/widgets.test.ts` reads the damage, radar
dwell, tap and cone declarations from `crates/` and fails on drift, and pins
the current and v0.11.0 Pierce walks. Page prose cites sources in comments.

Verification: `npm ci` and `npm run ci` in `web/` pass (format, lint, tests,
build); `mdbook build` passes. A mutated `PIERCE_BASE_POWER` fails the new
guard. A `PUBLIC_PATH=/nova-protocol/` build served under the prefix was read
at 1440 and 390 px: current `round-travel` has no layer cap, the v0.11.0 news
instance says "at most 6 sections deep (the v0.11.0 rule)", the switchboard
with only COMBAT LOCK on shows the reticle, ammo gauges and bore sight on, no
page text mentions a lock decay, and changed links resolve under the prefix.
An independent review rechecked every changed figure and citation against
`crates/`. A follow-up labelled the widget's G figure as an upper bound and
its 0-to-1,000 m/s time as a lower bound; `npm run ci` passed again. Known
layout debt, not from this change: the turret page's trigger
discipline table scrolls the page sideways at 390 px.

## Scheduling

Pulled onto the v0.15.0 board at priority 55 on 2026-09-21 by owner decision,
in epic `20260921-231507`.

It runs CONCURRENTLY with the two v0.15.0 spikes (`20260824-125943`,
`20260824-125938`). This is a truth audit of what the game does today. It is
not gated on what those spikes decide the game should become, and it does not
gate them. If a spike later changes a documented figure, that is the spike's
child task, not a reason to hold this one.
