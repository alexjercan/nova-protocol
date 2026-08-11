# Research: Wesnoth wiki content-creation learning curve

Fetched live 2026-08-11. Sources:

- https://wiki.wesnoth.org/Create
- https://wiki.wesnoth.org/BuildingScenarios (ladder hub)
- https://wiki.wesnoth.org/BuildingScenariosSimple
- https://wiki.wesnoth.org/BuildingScenariosIntermediate
- https://wiki.wesnoth.org/BuildingScenariosAdvanced
- https://wiki.wesnoth.org/WML_for_Complete_Beginners (+ Chapter_1)
- https://wiki.wesnoth.org/ReferenceWML

Note: https://wiki.wesnoth.org/GettingStartedWithWML returns 404. The Create
page's current WML entry point is the "WML Tutorial" link -> page
`WML_for_Complete_Beginners`. Treat that as the replacement.

## 1. Create page as front door

Five sections, in reading order:

1. "Read this first!" -- technical floor: directory layout, userdata,
   EditingWesnoth, AddonStructure. Everyone passes through this.
2. "What can I create, and how?" -- routing by content type.
3. "What have others done?" -- community examples, forums.
4. "The world of Wesnoth" -- lore (timeline, geography, races) for writers.
5. "Miscellaneous" -- reference materials, add-on server.

Audience routing: one link row per creator type. Order inside section 2 (as
rendered today):

1. WML Tutorial (WML_for_Complete_Beginners)
2. Maps
3. Scenarios
4. Campaigns
5. Multiplayer scenarios and campaigns
6. Custom units
7. Distributing content
8. Art
9. Music
10. Writing
11. Artificial intelligence
12. Translations
13. Authoring tools
14. Maintenance tools

Ordering logic: language basics first, then content types in rough
dependency order (map -> scenario -> campaign -> units), then non-code
disciplines (art, music, writing), then specialist domains (AI,
translations, tooling). It is implicit beginner-to-advanced by placement,
not by labels; the page does not literally say "start here".

Voice samples (verbatim):

- "Players can create new maps, units, races, scenarios, art, music, and
  even entire campaigns!"
- "Access to the 'guts' of the game is both simple and difficult; if you
  have a UTF-8 text editor you have everything you need"
- "Remember you can always ask for assistance and collaborate with fellow
  content creators"

Takeaway: welcoming, low-barrier framing ("a text editor is everything you
need") + audience-segmented routing. Weakness worth fixing in our docs: no
explicit "if you are new, start with X" sentence; progression is only
implied by link order.

## 2. The Simple -> Intermediate -> Advanced ladder

Hub: BuildingScenarios frames the ladder ("Creating a scenario is no simple
task. Generally, you should have a premise mapped out for the scenario
before you begin.") and links the three rungs plus a fourth page,
BuildingScenariosBalancing. Every rung carries the same top navigation bar:
"BuildingScenariosSimple - BuildingScenariosIntermediate -
BuildingScenariosAdvanced", so the reader always sees where they are on the
ladder.

### Rung 1: BuildingScenariosSimple

- Prerequisite callout up front (verbatim): "Before reading this, it might
  prove useful to read something about the syntax of the Wesnoth Markup
  Language: SyntaxWML"
- Covers: scenario metadata (id, next_scenario), textdomain, time-of-day
  macros, a prestart event with [objectives], [side] with
  controller=human and a recruit list, and the campaign file that glues
  scenarios together ("Making It All Work").
- Teaching device: ONE incomplete scenario file grown across the page.
  Sections are literally connected with "continued below"/"continued above"
  markers so the reader knows all fragments are the same file.
- Handoffs: See Also -> BuildingMaps, ScenarioWML, SyntaxWML,
  BuildingScenarios; forward link to BuildingScenariosIntermediate and
  BuildingScenariosSamples.

### Rung 2: BuildingScenariosIntermediate

- Opening (verbatim): "In this tutorial we will dig somewhat deeper into
  the secrets of WML and scenario building: events, explaining the use of
  some special attributes, setting up somewhat more advanced sides, ..."
- Assumes rung 1: never restates basics; the nav bar plus "dig somewhat
  deeper" framing carries the assumption.
- Covers, in workflow order: events with [filter]s -> special attributes
  (event modifiers, scenario-wide settings) -> difficulty-branched sides
  (#ifdef EASY/NORMAL/HARD gold and recruit lists, AI tuning) -> visual
  polish ([item], story/[intro] sequences).
- Closes with a competence statement (verbatim): "Now you have enough
  information to make some interesting looking scenarios with tuned AI
  players." Then See Also -> BuildingScenariosAdvanced, EventWML,
  FilterWML, ScenarioWML, IntroWML.

### Rung 3: BuildingScenariosAdvanced

- Quick Navigation links back to Simple and Intermediate; part three of an
  explicit progression.
- Covers: advanced events, internal actions ([set_variable]), variables
  (concept -> manipulation -> use in unit filters), gettext and
  translation practice.
- Skippable-theory device (verbatim): "(This can be skipped if you're
  familiar with the concept of variables)"
- Hands off to reference instead of being exhaustive (verbatim): "See
  InternalActionsWML for a complete list of all tags"; links VariablesWML
  for depth; final section recommends ScenarioWML, SyntaxWML, ReferenceWML.

Ladder pattern: each rung teaches a working subset, ends by naming what the
reader can now do, and points at (a) the next rung and (b) the reference
pages that exhaustively cover what the rung only sampled.

## 3. Tutorial voice vs reference voice

Tutorial pages (Building*, WML_for_Complete_Beginners):

- Second person, conversational, analogy-heavy, one worked example.
- Sample (Advanced): "Variables are basically names. And with those names,
  we associate a certain value. You can compare this with the words,
  because words are associated with (several) objects..."
- Sample (Intermediate): "The [item] tag is actually very simple to use"
- Sample (Simple): "Macros are essentially WML shortcuts. They allow you to
  define certain pieces of code which can be re-used whenever they are
  needed."

Reference pages (ReferenceWML and per-tag pages):

- Index/table style, alphabetical tag index plus a hierarchical grouping
  (how WML works / toplevel tags / other tags / predefined macros, tools).
- Self-describing (verbatim): "This page is a collection of pointers to
  different common WML structures."
- Crucially, the reference links BACK to the tutorials (verbatim): "See
  BuildingScenarios, BuildingCampaigns and BuildingUnits for a tutorial
  style overview." The two layers cross-reference in both directions.

## 4. Pyramid / progressive disclosure

Four tiers, each one click "down" from the previous:

1. Create -- routing only, near-zero teaching content.
2. Course layer -- WML_for_Complete_Beginners: 11 ordered chapters
   (syntax -> project setup -> scenario -> events -> units -> variables ->
   macros -> logic), each chapter ending with a link to the next. Concepts
   introduced once, in order, with analogies.
3. Task-tutorial layer -- BuildingScenarios* ladder: same concepts applied
   to one concrete deliverable, sampled not exhausted.
4. Reference layer -- ReferenceWML + per-tag pages (EventWML, FilterWML,
   ScenarioWML, VariablesWML, InternalActionsWML, SyntaxWML): exhaustive
   key tables, no narrative.

Linking discipline: tutorials introduce a tag with one usage and
immediately name the reference page holding the complete key list ("See
InternalActionsWML for a complete list of all tags"). Big concepts (events,
variables, macros, difficulty branching) get their introduction exactly
once, in the tutorial tier; exhaustive detail lives only in tier 4.

## 5. Devices worth copying

- Prerequisite callout as literal first sentence ("Before reading this, it
  might prove useful to read...").
- Persistent ladder nav bar on every rung (Simple - Intermediate -
  Advanced) so position in the curve is always visible.
- One worked example file grown across the whole page, with explicit
  "continued below"/"continued above" seams. Reader ends the page with a
  complete, runnable artifact.
- Competence checkpoint sentence at the end of a rung ("Now you have enough
  information to make...") -- tells the reader what they just unlocked.
- Skippable-theory parentheticals for mixed audiences ("This can be skipped
  if you're familiar with the concept of variables").
- "See X for a complete list" pattern: teach one usage, link the exhaustive
  table, never inline it.
- See Also sections that split into "next rung" vs "reference depth" links.
- Bidirectional layer links: reference index points back at tutorials for
  newcomers who landed in the wrong tier.
- Chapter-per-concept course with forward-only chaining (each chapter ends
  "-> Chapter N+1"), separate from the task tutorials.
- Honest status markers: "(In Progress)" tags on Create, footer note that
  chapters 6-11 are partially finished.

## 6. Representative snippets

Teaching voice, beginner tier (WML_for_Complete_Beginners Chapter 1):

- "Think of it as a set of rules for how WML needs to be written in order
  for the game to understand it."
- "Think of the opening tag and the closing tag like the covers of a book:
  when you open the front cover, you know you're at the beginning."
- Pseudocode-first before real WML:

  ```
  [go]
      where=grocery_store
      get=bread
  [/go]
  ```

  "Tags tell the WML engine what to do generally (like telling your friend
  to 'go'), but without attributes to specify exactly what to do, the WML
  engine won't be able to do anything."

Incremental worked example, tutorial tier (Simple):

- "#textdomain wesnoth-Simple_Campaign" then "[scenario]" with: "Every
  scenario must be enclosed in a tag; the [scenario] tag is used for
  campaign scenarios." -- and the file continues section by section under
  "continued below" markers.

Goal-first example, tutorial tier (Intermediate):

- "Suppose you wanted Konrad to say "it's getting cold" when he moves to
  the location (4,8):"

  ```
  [event]
    name=moveto
    [filter]
      id=Konrad
      x=4
      y=8
    [/filter]
    [message]
      speaker=Konrad
      message= _ "It's getting cold"
    [/message]
  [/event]
  ```

  Pattern: state the player-visible goal in one sentence, then show the
  complete minimal block that achieves it.

Translation-practice rule, advanced tier: "The most important rule of all
is: Do not split sentences" -- advanced pages shift from syntax to craft
and pitfalls.

## Recommendations for our docs site

- Front door page: audience-segmented link rows like Create, but add the
  explicit "new? start here" sentence Create lacks.
- Three-rung task ladder per content type, each rung: prerequisite
  callout -> one growing worked example -> competence checkpoint -> See
  Also split into next-rung and reference links.
- Keep reference pages narrative-free key tables; enforce "teach one usage,
  link the full table" in tutorials.
- Cross-link both directions between tutorial and reference tiers.
- Goal-first example framing ("Suppose you wanted X... here is the block").
