# Rebuild the modding docs as a Wesnoth-style reference site

- STATUS: CLOSED
- PRIORITY: 37
- TAGS: v0.10.0, docs, web, modding

PROBLEM: The modding wiki page is bloated, and the actual authoring grammar is
documented nowhere - both benchmark modder runs reverse-engineered the whole
event/filter/action/object vocabulary by grepping the two worked mods. The
after-run GAPS.md is the evidence list: no section `kind` catalog, no base
prototype id catalog (the controller-cube failure), no event/action/filter
reference, no expression AST grammar, load-bearing rules living only in mod
comments (fail-closed filters, arm-inside-area, count-gate idiom).

Goal: a modding reference in the style of the Wesnoth wiki, as a proper web
page, not one long markdown file:

- Catalog sidebar of every event, action, filter, and object kind.
- A glossary to filter quickly to the right construct.
- One page per construct: in-depth description, field list with types,
  defaults and units, how to write it in RON, a worked example, and how it
  relates to the other constructs (what fires it, what it can target).
- Better visuals for how each thing is defined - shape of the RON, required
  vs serde-defaulted fields.
- A base content catalog: prototype ids with their section kinds ("Racer
  Controller", turrets, thrusters), scenario ids, asset paths that
  `dep://base/...` can reach.
- The current `modding.md` shrinks to an overview that routes into the
  reference.

Keep it un-stale by construction where possible: the constructs and the base
catalog are all in Rust (`nova_events`, `nova_scenario`, the `nova_authoring`
builders), so consider generating the catalog data the same way
`content -- gen` generates the RON, and hand-writing only the prose around it.
