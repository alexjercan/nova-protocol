# Modding reference

The complete catalog of the RON authoring vocabulary: every event, filter,
action, object kind and expression node the scenario engine understands, each
with its field table (types, defaults, units), a copyable snippet and its
cross-references. Everything the base game ships is written in this
vocabulary; anything it does, your content can do.

**Learning rather than looking up?** Follow
[Create your first scenario](../author-a-scenario/) and
[Publish a mod](../publish-a-mod/) first. Return here when you want the complete
format for campaigns, scenarios, sections, or more advanced mission logic.

## How RON content is written

The five spelling rules behind every snippet in these pages:

- The loader is STRICT: an unknown or misspelled field is a hard parse
  error, not a silent default. Field names are load-bearing.
- Constructs are newtype enum variants: `Name((field: value, ...))` - double
  parens, even for one field. Event names are bare (`OnStart`); a few inner
  enums use single parens with named fields (`Light(Directional(...))`,
  `Box(min: .., max: ..)`).
- `Option` fields keep their variant: `Some(12.0)`, never a bare value.
  Omitting a defaulted field is always legal.
- Tuples: positions are `(x, y, z)`, rotations `(x, y, z, w)` quaternions;
  colors are tagged - `Srgba((red: .., green: .., blue: .., alpha: ..))`.
- Asset paths are SCHEMED strings, never bare: `self://` (your bundle),
  `dep://base/` (base game - see the
  [base content catalog](../base-content/)), `dep://<id>/` (a declared
  dependency).

## The reference pages

<div id="wiki-children"></div>

## The vocabulary by family

| family | constructs |
|---|---|
| Mod structure | [bundle and content files](../mod-files/), [`Campaign`](../campaigns/), [`Scenario`](../scenarios/), [`Section`](../sections/) |
| Events (10) | [`OnStart`](../events/#onstart), [`OnUpdate`](../events/#onupdate), [`OnTimerEnd`](../events/#ontimerend), [`OnDestroyed`](../events/#ondestroyed), [`OnNeutralized`](../events/#onneutralized), [`OnEnter`](../events/#onenter), [`OnExit`](../events/#onexit), [`OnOrbit`](../events/#onorbit), [`OnTravelLock`](../events/#ontravellock), [`OnCombatLock`](../events/#oncombatlock) |
| Filters (4) | [`Entity`](../filters/#entity), [`Timer`](../filters/#timer), [`Expression`](../filters/#expression), [`Conditional`](../filters/#conditional) (`Not` / `And` / `Or`) |
| Actions (24) | spawning: [`SpawnScenarioObject`](../actions/#spawnscenarioobject), [`ScatterObjects`](../actions/#scatterobjects), [`DespawnScenarioObject`](../actions/#despawnscenarioobject), [`CreateScenarioArea`](../actions/#createscenarioarea) - mission: [`Objective`](../actions/#objective), [`ObjectiveComplete`](../actions/#objectivecomplete), [`ObjectiveMarkerAttach`](../actions/#objectivemarkerattach), [`ObjectiveMarkerDetach`](../actions/#objectivemarkerdetach), [`StoryMessage`](../actions/#storymessage), [`HudReadout`](../actions/#hudreadout), [`HintEmphasisSet`](../actions/#hintemphasisset), [`HintEmphasisClear`](../actions/#hintemphasisclear) - flow: [`Outcome`](../actions/#outcome), [`NextScenario`](../actions/#nextscenario) - ships: [`SetSpeedCap`](../actions/#setspeedcap), [`SetControllerVerb`](../actions/#setcontrollerverb), [`SetAllegiance`](../actions/#setallegiance) - state: [`VariableSet`](../actions/#variableset), [`TimerStart`](../actions/#timerstart), [`TimerCancel`](../actions/#timercancel), [`DebugMessage`](../actions/#debugmessage) - view: [`SetCamera`](../actions/#setcamera), [`Screenshot`](../actions/#screenshot), [`SetSkybox`](../actions/#setskybox) |
| Objects (5) | [`Asteroid`](../objects/#asteroid), [`Spaceship`](../objects/#spaceship), [`Beacon`](../objects/#beacon), [`SalvageCrate`](../objects/#salvagecrate), [`Light`](../objects/#light) (`Directional` / `Point`) |
| Expression nodes (15) | values: [`Number`, `String`, `Boolean`](../expressions/#values-the-literal-types) - atoms: [`Literal`, `Name`, `Parens`](../expressions/#factors-the-atoms) - terms: [`Factor`, `Multiply`, `Divide`](../expressions/#terms-multiply-divide) - expressions: [`Term`, `Add`, `Subtract`](../expressions/#expressions-add-subtract-the-value-root) - conditions: [`LessThan`, `GreaterThan`, `Equal`](../expressions/#conditions-the-boolean-root) |
| Base ids & assets | [section prototypes](../base-content/#section-prototypes), [scenario ids](../base-content/#scenario-ids), [`dep://base/` assets](../base-content/#assets-what-depbase-can-reach) |

The wiki search (sidebar) indexes every construct name, so typing
`ScatterObjects` or `OnEnter` up there jumps straight to the right page.

## Every construct, A to Z

**A** - [`Add`](../expressions/#expressions-add-subtract-the-value-root) (expression node),
[`AI`](../objects/#the-controller) (ship controller),
[`And`](../filters/#conditional) (filter combinator),
[`Asteroid`](../objects/#asteroid) (object)

**B** - [`Beacon`](../objects/#beacon) (object),
[`Boolean`](../expressions/#values-the-literal-types) (literal),
[`Box`](../actions/#scatterobjects) (scatter region)

**C** - [`Campaign`](../campaigns/) (content item),
[`Conditional`](../filters/#conditional) (filter),
[`CreateScenarioArea`](../actions/#createscenarioarea) (action)

**D** - [`DebugMessage`](../actions/#debugmessage) (action),
[`DespawnScenarioObject`](../actions/#despawnscenarioobject) (action),
[`Directional`](../objects/#light) (light method),
[`DisableVerb`](../objects/#the-sections-list) (section modification),
[`Divide`](../expressions/#terms-multiply-divide) (expression node)

**E** - [`Entity`](../filters/#entity) (filter),
[`Equal`](../expressions/#conditions-the-boolean-root) (condition),
[`Expression`](../filters/#expression) (filter)

**F** - [`Factor`](../expressions/#terms-multiply-divide) (expression node)

**G** - [`GreaterThan`](../expressions/#conditions-the-boolean-root) (condition)

**H** - [`HintEmphasisClear`](../actions/#hintemphasisclear),
[`HintEmphasisSet`](../actions/#hintemphasisset),
[`HudReadout`](../actions/#hudreadout) (actions)

**I** - [`Inline`](../objects/#the-sections-list) (section source)

**K** - [`Keyboard` / `Mouse` / `Gamepad`](../objects/#the-controller) (input bindings)

**L** - [`LessThan`](../expressions/#conditions-the-boolean-root) (condition),
[`Light`](../objects/#light) (object),
[`Literal`](../expressions/#factors-the-atoms) (expression node)

**M** - [`Multiply`](../expressions/#terms-multiply-divide) (expression node)

**N** - [`Name`](../expressions/#factors-the-atoms) (expression node),
[`NextScenario`](../actions/#nextscenario) (action),
[`Not`](../filters/#conditional) (filter combinator),
[`Number`](../expressions/#values-the-literal-types) (literal)

**O** - [`Objective`](../actions/#objective),
[`ObjectiveComplete`](../actions/#objectivecomplete),
[`ObjectiveMarkerAttach`](../actions/#objectivemarkerattach),
[`ObjectiveMarkerDetach`](../actions/#objectivemarkerdetach) (actions),
[`OnCombatLock`](../events/#oncombatlock),
[`OnDestroyed`](../events/#ondestroyed),
[`OnEnter`](../events/#onenter),
[`OnExit`](../events/#onexit),
[`OnNeutralized`](../events/#onneutralized),
[`OnOrbit`](../events/#onorbit),
[`OnStart`](../events/#onstart),
[`OnTravelLock`](../events/#ontravellock),
[`OnUpdate`](../events/#onupdate) (events),
[`Or`](../filters/#conditional) (filter combinator),
[`Outcome`](../actions/#outcome) (action)

**P** - [`Parens`](../expressions/#factors-the-atoms) (expression node),
[`Player`](../objects/#the-controller) (ship controller),
[`Point`](../objects/#light) (light method),
[`Prototype`](../objects/#the-sections-list) (section source)

**R** - [`Rename`](../objects/#the-sections-list) (section modification),
[`Ring`](../actions/#scatterobjects) (scatter region)

**S** - [`SalvageCrate`](../objects/#salvagecrate) (object),
[`ScatterObjects`](../actions/#scatterobjects) (action),
[`Scenario`](../scenarios/) (content item),
[`Screenshot`](../actions/#screenshot) (action),
[`Section`](../sections/) (content item),
[`SetAllegiance`](../actions/#setallegiance),
[`SetCamera`](../actions/#setcamera),
[`SetControllerVerb`](../actions/#setcontrollerverb) (actions),
[`SetHealth`](../objects/#the-sections-list) (section modification),
[`SetSkybox`](../actions/#setskybox),
[`SetSpeedCap`](../actions/#setspeedcap),
[`SpawnScenarioObject`](../actions/#spawnscenarioobject) (actions),
[`Spaceship`](../objects/#spaceship) (object),
[`StoryMessage`](../actions/#storymessage) (action),
[`String`](../expressions/#values-the-literal-types) (literal),
[`Subtract`](../expressions/#expressions-add-subtract-the-value-root) (expression node)

**T** - [`Term`](../expressions/#expressions-add-subtract-the-value-root) (expression node)

**V** - [`VariableSet`](../actions/#variableset) (action)

Two engine-maintained variables also belong to the vocabulary:
[`scenario_elapsed` and `player_speed`](../expressions/#reserved-engine-variables).

## Extending the vocabulary

A construct that does not exist here cannot be authored in RON - adding an
event, filter, action or object kind is a small Rust change, covered in
[Extend the scenario engine](../../dev/guide-extend-scenarios/). If you add
one, its entry lands on these pages in the same task (see
[Keeping docs in sync](../../dev/keeping-docs-in-sync/)).
