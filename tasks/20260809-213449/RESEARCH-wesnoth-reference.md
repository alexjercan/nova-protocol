# Research: Wesnoth wiki WML reference structure

Fetched live 2026-08-11. Pages: ReferenceWML, EventWML, ActionWML, FilterWML,
StandardUnitFilter, ScenarioWML, ConditionalActionsWML, plus
InterfaceActionsWML (fetched to capture the "(translatable)" convention).
All URLs resolved; no 404s.

## 1. ReferenceWML: catalog organization

- One hub page. Section order:
  1. The Wesnoth Markup Language (prose intro)
  2. How WML works (syntax primer)
  3. WML toplevel tags
  4. Other WML tags
  5. Predefined macros
  6. Other
  7. See Also
- Two access paths on the same page:
  - Alphabetical tag index (A-Z) near the top. Every tag links to the page
    that documents it, often with a section anchor.
  - Category lists: toplevel tags (campaign, scenario, era, units, ...) vs
    "other" tags (event, side, filters, actions). Grouped by scope, not
    alphabet.
- Link entry format: tag name is the link text, target page name doubles as
  the description. Examples verbatim:
  - "[abilities](/AbilitiesWML#The_.5Babilities.5D_tag)"
  - "[campaign](/CampaignWML#The_.5Bcampaign.5D_tag)"
  - "[event](/EventWML) how to describe an event"
- Anchors encode the literal tag: `#The_.5Babilities.5D_tag` is
  "The [abilities] tag" URL-escaped. One stable anchor per tag.
- Intro prose (verbatim): "The Wesnoth Markup Language (WML) is used to code
  almost everything in Wesnoth, including scenarios, units, savefiles, and
  the user interface layout. WML files are simple, human-readable text
  files, usually with the .cfg extension, with similarities to INI files
  and XML."
- Beginner off-ramp before the reference lists (verbatim): "See
  [BuildingScenarios], [BuildingCampaigns] and [BuildingUnits] for a
  tutorial style overview." Reference pages stay reference; tutorials live
  elsewhere and get pointed to.

## 2. Construct page anatomy (specimen: EventWML)

Section order on EventWML:

1. Prose intro: what the tag is, where it can appear. Verbatim: "This tag
   is a subtag of the [scenario], [unit_type] and [era] tags which is used
   to describe a set of actions which trigger at a certain point in a
   scenario. When used in a [scenario] tag (also includes [multiplayer] and
   [test]), the event only occurs in that scenario."
2. Mandatory key first ("The 'name' Key (Mandatory)"), with subsections for
   edge cases (variables in the name, custom events).
3. "Optional Keys and Tags": one entry per key, then one per sub-tag
   ([filter], [filter_second], [filter_condition], ...).
4. Semantics sections: "Actions triggered by [event]", "Nested Events",
   "Delayed Variable Substitution", "Multiplayer safety", "A Trap for the
   Unwary" (gotchas get their own named section).
5. Enumerations: "Predefined Events Without Filters" then "Predefined
   Events With Filters", one sub-heading per event name (e.g. "preload"),
   each with prose on trigger conditions.
6. "Miscellaneous Notes and Examples", then "See Also".

Key entry format: definition-list style, `key: prose`. No formal table.
Type and default live inside the prose. Verbatim entries:

- "first_time_only: Whether the event should be removed from the scenario
  after it is triggered. This key takes a boolean" (allowed values listed
  under it)
- "id: If an id is specified, then the event will not be added if another
  event with the same id already exists."
- "priority: If several '[event]' tags have the same name, then any with a
  high priority value will be triggered before events with lower
  priority. (Version 1.17.20 and later only)"

Examples: indented WML code blocks inline in the prose, right after the
concept they illustrate. Short one verbatim:

    name=attacker misses,defender misses

Cross-links: inline wiki links ("[FilterWML](/FilterWML)"); same-page keys
link by anchor ("[first_time_only](#first_time_only)").

Variants: [scenario]/[multiplayer]/[test] variants are stated in the intro
sentence, not given separate pages. On ScenarioWML the three variant tags
each get a section on the shared page ("The [scenario] tag", "The
[multiplayer] tag", "The [test] tag"); intro verbatim: "The top level tags
[multiplayer], [test], and [scenario] are addon module tags that are all
formatted the same way. The difference between these tags is the way that
the scenarios they describe are accessed."

## 3. ActionWML: large catalog on one page

- ActionWML is an index page, not a documentation page. Intro verbatim:
  "ActionWML is a summarizing term for all WML actions which can be used
  in events and some other places."
- Structure: short "Types of ActionWML" prose (conditional, direct
  gameplay, internal, interface, Lua), then a flat alphabetical index with
  letter headings A: through Z:.
- Each entry: tag name as link + one-line purpose, linking to the topical
  sub-page (with anchor) where the tag is actually documented. Verbatim
  entries from A:
  - "[abilities]" -> AbilitiesWML
  - "[about]" -> CreditsWML
  - "[achievement]" -> AchievementsWML
  - "[add_ai_behavior]" -> "Lua AI Legacy Methods Howto"
- So the real docs are grouped by purpose on sub-pages
  (InterfaceActionsWML, DirectActionsWML, InternalActionsWML,
  ConditionalActionsWML); ActionWML overlays an alphabetical lookup on top.
  Two navigation axes, one source of truth per tag.
- No separate TOC; the A-Z list is the TOC.
- Cross-references: inline, e.g. "see [FilterWML]" and "see
  [EventWML#Nested_Events]" (anchor-precise links into other pages).

## 4. Filters as a reusable cross-cutting concept

- FilterWML is a concept hub. Sections: Filtering in WML; Filtering Units /
  Locations / Sides / Abilities / Weapons / Vision; Filtering on WML data;
  Tutorial; See Also.
- Defines the concept once, verbatim: "A filter is a special WML block.
  Filters are used to describe a set of units, hexes, weapons or something
  else. Filters are defined as matching something if all the keys in the
  filter match that thing."
- Each filter kind gets 2-3 sentences plus a delegation line, verbatim:
  "See [StandardUnitFilter] for details." / "See [StandardLocationFilter]
  for details." / "See [StandardSideFilter] for details."
- The reuse convention is named explicitly, verbatim: "A
  StandardUnit(Location, Side, ...)Filter is the place where the set of
  such keys and tags can appear... the phrase 'standard unit filter' is
  used in place of the set of standard keys."
- StandardUnitFilter page then documents the full key set ONCE. Other pages
  never re-list filter keys; they write "standard unit filter" or "accepts
  a StandardUnitFilter" and link. Verbatim from StandardUnitFilter: many
  tags "accept a StandardUnitFilter directly as an argument, like [kill]
  and [have_unit] (which each accept a few additional keys of their own)."
- Reverse index via wiki machinery, verbatim: "Special:WhatLinksHere/
  StandardUnitFilter for tags which can contain a StandardUnitFilter."
- StandardUnitFilter key entries (verbatim):
  - "id: unit matches the given id... id= can be a comma-separated list,
    every unit with one of these ids matches."
  - "type: matches the unit's type name (can be a list of types)"
  - "upkeep: [Version 1.15.3 and later only] The upkeep of the unit. Can be
    either a non-negative number or one of the special values..."
  - "status: [Version 1.13.0 and later only] matches if the unit has the
    specified status active. This can be a comma-separated list..."
- Nested combinators documented as entries on the same page: [and], [or],
  [not], [filter_adjacent] ("with a StandardUnitFilter as argument...
  count: a number, range, or comma separated range; adjacent: a comma
  separated list of directions"), [filter_location], [filter_side].
  Filters compose recursively; the page shows it with an example:

      [kill]
          [not]
              id=Gwiti Ha'atel
          [/not]
          [not]
              id=Tanar
          [/not]
      [/kill]

- ConditionalActionsWML shows the consumer side: "[have_unit] links to
  StandardUnitFilter for selection criteria"; [have_location] ->
  StandardLocationFilter. Condition tags are thin wrappers over filters,
  documented as such.

## 5. Conventions worth copying

- (translatable) marker: prefix inside the key description. Verbatim from
  InterfaceActionsWML:
  - "message: (translatable) the text to display to the right of the image"
  - "caption: (translatable) the caption to display beside the image"
  - It stacks with version notes: "male_message, female_message: (Version
    1.13.2 and later only) (translatable) Used instead of message if the
    unit's gender matches"
- Type statements: prose, not a schema column. "This key takes a boolean";
  "Can be either a non-negative number or one of the special values";
  "a comma-separated list of...".
- Default notation: inline prose, sometimes version-split. Verbatim from
  ScenarioWML turns: "Use -1 to have no turn limit. Default value is -1 on
  wesnoth-1.13 and 100 on wesnoth-1.12." Also inline form: "when this is
  set to 'no'(default)". InterfaceActionsWML: speaker default "WML";
  bullet default is the Unicode bullet character (given literally).
- Version notes: standard phrase "(Version 1.x.y and later only)", italic,
  linking to a DevFeature page. Placed immediately at the start of the key
  description or after the key name. Same phrase everywhere.
- Deprecation: inline note naming the replacement. Verbatim: "(Version
  1.15.7 and later only) Prints a deprecation warning recommending to use
  [remove_event] instead." And: "needs_select=yes is deprecated, consider
  using manual variable syncing with [sync_variable]".
- Prose intro before key list: every page opens with 1-3 sentences: what
  the tag is, whose subtag it is, where it applies. Then mandatory keys,
  then optional keys, then semantics/gotchas, then examples, then See Also.
- Gotchas get named sections ("A Trap for the Unwary", "Multiplayer
  safety") instead of being buried in key descriptions.
- See Also footer on every page; links stay bidirectional (hub -> detail,
  detail -> hub and siblings).
- Anchor-per-tag URL scheme makes every construct deep-linkable from the
  A-Z index.

## 6. Reading order / difficulty progression

- The hub separates tutorial from reference explicitly: BuildingScenarios /
  BuildingCampaigns / BuildingUnits are offered "for a tutorial style
  overview" before the tag lists.
- "How WML works" (syntax primer) sits on the hub above the catalog, so a
  new reader hits syntax before tags.
- Within a construct page: mandatory keys first, optional next, advanced
  semantics (nesting, variable substitution, MP safety) after, predefined
  enumerations near the end, misc examples last. Difficulty rises down the
  page; a skimmer can stop after the key list.
- FilterWML and StandardUnitFilter end with a "Tutorial" section: reference
  first, guided walk-through appended, not interleaved.
- ConditionalActionsWML orders content by concept dependency: container
  tags ([if], [switch], [while], [for], [foreach], [repeat], [command])
  before condition tags ([true], [false], [have_unit], [variable], ...),
  with meta-conditions ([and]/[or]/[not]) last. Its intro (verbatim):
  "Conditional Actions WML is used to describe container actions that
  create branching and flow control for WML. The conditional actions act
  as gatekeepers, encapsulating other actions with conditions which must
  be met before an action can take place."
- Long example kept realistic, placed after the tag docs (verbatim,
  abridged):

      [if]
         [variable]
            name=we.gold
            greater_than=$they.gold
         [/variable]
         [else]
            [message]
               message=This will not be easy!
            [/message]
         [/else]
      [/if]

## Takeaways for a modding reference site

- One hub page: prose intro + syntax primer + category lists + flat A-Z
  index. Both axes link to a single canonical anchor per construct.
- Index pages index; topical pages document. Never both.
- Document each reusable filter/selector once on its own page, give it a
  name ("standard unit filter"), and have consumers say "accepts a
  StandardUnitFilter" + link. Provide a reverse "what links here" view.
- Key entry pattern: name, (translatable)?, (Version note)?, type in prose,
  default in prose, description. Mandatory keys before optional.
- Fixed page skeleton: intro -> mandatory keys -> optional keys/sub-tags ->
  semantics/gotchas (named sections) -> enumerations -> examples -> See
  Also. Difficulty increases down the page.
- Standard phrasing for version gates and deprecations; deprecation always
  names the replacement.
