# Modding reference

The complete catalog of the RON authoring vocabulary: every event, filter,
action, object kind and expression node the scenario engine understands, each
with its field table (types, defaults, units), a copyable snippet and its
cross-references. Everything the base game ships is written in this
vocabulary; anything it does, your content can do.

**Learning rather than looking up?** Follow
[Create your first scenario](../author-a-scenario/) and
[Publish a mod](../publish-a-mod/) first. Return here when you want the complete
format for campaigns, scenarios, sections, ships, styles, or more advanced
mission logic.

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
| Mod structure | [bundle and content files](../mod-files/), [`Campaign`](../campaigns/), [`Impact`](../impacts/), [`Scenario`](../scenarios/), [`Section`](../sections/), [`Ship`](../ships/), [`Style`](../styles/) |
| Handler fields | [`name`](../events/), [`once`](../scenarios/#once-a-beat-that-happens-one-time), [`filters`](../filters/), [`actions`](../actions/) |
| Events (24) | [`OnStart`](../events/#onstart), [`OnUpdate`](../events/#onupdate), [`OnTimerEnd`](../events/#ontimerend), [`OnDefeated`](../events/#ondefeated), [`OnDestroyed`](../events/#ondestroyed), [`OnNeutralized`](../events/#onneutralized), [`OnEnter`](../events/#onenter), [`OnExit`](../events/#onexit), [`OnGotoComplete`](../events/#player-maneuver-completion), [`OnStopComplete`](../events/#player-maneuver-completion), [`OnOrbitStart`](../events/#orbit-lifecycle), [`OnOrbitStable`](../events/#orbit-lifecycle), [`OnOrbitLap`](../events/#orbit-lifecycle), [`OnOrbitUnstable`](../events/#orbit-lifecycle), [`OnOrbitEnd`](../events/#orbit-lifecycle), [`OnTravelLockStart`](../events/#lock-lifecycle), [`OnTravelLockEnd`](../events/#lock-lifecycle), [`OnCombatLockStart`](../events/#lock-lifecycle), [`OnCombatLockEnd`](../events/#lock-lifecycle), [`OnShipOrderComplete`](../events/#onshipordercomplete), [`OnShipOrderInterrupted`](../events/#onshiporderinterrupted), [`OnShipOrderResumed`](../events/#onshiporderresumed), [`OnShipOrderCanceled`](../events/#onshipordercanceled), [`OnShipOrderFailed`](../events/#onshiporderfailed) |
| Filters (5) | [`Entity`](../filters/#entity), [`Timer`](../filters/#timer), [`ShipOrder`](../filters/#shiporder), [`Expression`](../filters/#expression), [`Conditional`](../filters/#conditional) (`Not` / `And` / `Or`) |
| Actions (42) | spawning: [`SpawnScenarioObject`](../actions/#spawnscenarioobject), [`ScatterObjects`](../actions/#scatterobjects), [`DespawnScenarioObject`](../actions/#despawnscenarioobject), [`CreateScenarioArea`](../actions/#createscenarioarea) - mission: [`Objective`](../actions/#objective), [`ObjectiveComplete`](../actions/#objectivecomplete), [`ObjectiveMarkerAttach`](../actions/#objectivemarkerattach), [`ObjectiveMarkerDetach`](../actions/#objectivemarkerdetach), [`StoryMessage`](../actions/#storymessage), [`HudReadout`](../actions/#hudreadout), [`HintEmphasisSet`](../actions/#hintemphasisset), [`HintEmphasisClear`](../actions/#hintemphasisclear) - pacing: [`Sequence`](../actions/#sequence) - flow: [`Outcome`](../actions/#outcome), [`NextScenario`](../actions/#nextscenario) - ships: [`SetSpeedCap`](../actions/#setspeedcap), [`SetControllerVerb`](../actions/#setcontrollerverb), [`SetAllegiance`](../actions/#setallegiance), [`SetInfiniteAmmo`](../actions/#setinfiniteammo), [`RefillAmmo`](../actions/#refillammo) - helm orders: [`MoveShipTo`](../actions/#moveshipto), [`ForceAlign`](../actions/#forcealign), [`StopShip`](../actions/#stopship), [`PatrolShip`](../actions/#patrolship), [`OrbitShip`](../actions/#orbitship), [`ClearShipOrder`](../actions/#clearshiporder) - forced fire: [`ForceRailgunFire`](../actions/#forcerailgunfire), [`ForceTorpedoFire`](../actions/#forcetorpedofire) - AI constraints: [`SetAILeash`](../actions/#setaileash), [`SetAIEngageRange`](../actions/#setaiengagerange), [`SetAIPointDefenseRange`](../actions/#setaipointdefenserange) - state: [`VariableSet`](../actions/#variableset), [`TimerStart`](../actions/#timerstart), [`TimerCancel`](../actions/#timercancel), [`DebugMessage`](../actions/#debugmessage) - view: [`SetCamera`](../actions/#setcamera), [`SetCameraAnchor`](../actions/#setcameraanchor), [`ReleaseCamera`](../actions/#releasecamera), [`Screenshot`](../actions/#screenshot), [`SetSkybox`](../actions/#setskybox) - control: [`SuspendPlayerControl`](../actions/#suspendplayercontrol), [`ResumePlayerControl`](../actions/#resumeplayercontrol) |
| Objects (7) | [`Anchor`](../objects/#anchor), [`Asteroid`](../objects/#asteroid), [`Planet`](../objects/#planet), [`Spaceship`](../objects/#spaceship), [`Beacon`](../objects/#beacon), [`SalvageCrate`](../objects/#salvagecrate), [`Light`](../objects/#light) (`Directional` / `Point`) |
| Damage effects (3) | [`Cracks`, `Sparks`, `Plume`](../sections/#damage-effects) - the looks a section wears as it is damaged, authored in `base.damage_effects` |
| Expression nodes (16) | values: [`Number`, `String`, `Boolean`](../expressions/#values-the-literal-types) - atoms: [`Literal`, `Name`, `Query`, `Parens`](../expressions/#factors-the-atoms) - terms: [`Factor`, `Multiply`, `Divide`](../expressions/#terms-multiply-divide) - expressions: [`Term`, `Add`, `Subtract`](../expressions/#expressions-add-subtract-the-value-root) - conditions: [`LessThan`, `GreaterThan`, `Equal`](../expressions/#conditions-the-boolean-root) |
| Base ids & assets | [section prototypes](../base-content/#section-prototypes), [scenario ids](../base-content/#scenario-ids), [ship ids](../ships/#base-ships), [style ids](../base-content/#skin-styles), [`dep://base/` assets](../base-content/#assets-what-depbase-can-reach) |

The wiki search (sidebar) indexes every construct name, so typing
`ScatterObjects` or `OnEnter` up there jumps straight to the right page.

## Every construct, A to Z

**A** - [`Add`](../expressions/#expressions-add-subtract-the-value-root) (expression node),
[`AI`](../objects/#the-controller) (ship controller),
[`Anchor`](../objects/#anchor) (object),
[`And`](../filters/#conditional) (filter combinator),
[`Asteroid`](../objects/#asteroid) (object)

**B** - [`Beacon`](../objects/#beacon) (object),
[`Boolean`](../expressions/#values-the-literal-types) (literal),
[`Box`](../actions/#scatterobjects) (scatter region)

**C** - [`Campaign`](../campaigns/) (content item),
[`ClearShipOrder`](../actions/#clearshiporder) (action),
[`Conditional`](../filters/#conditional) (filter),
[`Cracks`](../sections/#damage-effects) (damage effect),
[`CreateScenarioArea`](../actions/#createscenarioarea) (action)

**D** - [`DebugMessage`](../actions/#debugmessage) (action),
[`DespawnScenarioObject`](../actions/#despawnscenarioobject) (action),
[`Directional`](../objects/#light) (light method),
[`DisableVerb`](../objects/#the-sections-list) (section modification),
[`Divide`](../expressions/#terms-multiply-divide) (expression node)

**E** - [`Entity`](../filters/#entity) (filter),
[`Equal`](../expressions/#conditions-the-boolean-root) (condition),
[`Expression`](../filters/#expression) (filter)

**F** - [`Factor`](../expressions/#terms-multiply-divide) (expression node),
[`ForceAlign`](../actions/#forcealign),
[`ForceRailgunFire`](../actions/#forcerailgunfire),
[`ForceTorpedoFire`](../actions/#forcetorpedofire) (actions)

**G** - [`GreaterThan`](../expressions/#conditions-the-boolean-root) (condition)

**H** - [`HintEmphasisClear`](../actions/#hintemphasisclear),
[`HintEmphasisSet`](../actions/#hintemphasisset),
[`HudReadout`](../actions/#hudreadout) (actions)

**I** - [`Impact`](../impacts/) (content item),
[`Inline`](../objects/#the-sections-list) (section source)

**K** - [`Keyboard` / `Mouse` / `Gamepad`](../objects/#the-controller) (input bindings)

**L** - [`LessThan`](../expressions/#conditions-the-boolean-root) (condition),
[`Light`](../objects/#light) (object),
[`Literal`](../expressions/#factors-the-atoms) (expression node)

**M** - [`MoveShipTo`](../actions/#moveshipto) (action),
[`Multiply`](../expressions/#terms-multiply-divide) (expression node)

**N** - [`Name`](../expressions/#factors-the-atoms) (expression node),
[`NextScenario`](../actions/#nextscenario) (action),
[`non_combatant`](../objects/#the-controller) (AI controller field),
[`Not`](../filters/#conditional) (filter combinator),
[`Number`](../expressions/#values-the-literal-types) (literal)

**O** - [`Objective`](../actions/#objective),
[`ObjectiveComplete`](../actions/#objectivecomplete),
[`ObjectiveMarkerAttach`](../actions/#objectivemarkerattach),
[`ObjectiveMarkerDetach`](../actions/#objectivemarkerdetach) (actions),
[`OnCombatLockStart`](../events/#lock-lifecycle),
[`OnCombatLockEnd`](../events/#lock-lifecycle),
[`OnDefeated`](../events/#ondefeated),
[`OnDestroyed`](../events/#ondestroyed),
[`OnEnter`](../events/#onenter),
[`OnExit`](../events/#onexit),
[`OnGotoComplete`](../events/#player-maneuver-completion),
[`OnNeutralized`](../events/#onneutralized),
[`OnOrbitStart`](../events/#orbit-lifecycle), [`OnOrbitStable`](../events/#orbit-lifecycle), [`OnOrbitLap`](../events/#orbit-lifecycle), [`OnOrbitUnstable`](../events/#orbit-lifecycle), [`OnOrbitEnd`](../events/#orbit-lifecycle),
[`OnShipOrderCanceled`](../events/#onshipordercanceled),
[`OnShipOrderComplete`](../events/#onshipordercomplete),
[`OnShipOrderFailed`](../events/#onshiporderfailed),
[`OnShipOrderInterrupted`](../events/#onshiporderinterrupted),
[`OnShipOrderResumed`](../events/#onshiporderresumed),
[`OnStart`](../events/#onstart),
[`OnStopComplete`](../events/#player-maneuver-completion),
[`OnTimerEnd`](../events/#ontimerend),
[`OnTravelLockStart`](../events/#lock-lifecycle),
[`OnTravelLockEnd`](../events/#lock-lifecycle),
[`OnUpdate`](../events/#onupdate) (events),
[`once`](../scenarios/#once-a-beat-that-happens-one-time) (handler field),
[`OrbitShip`](../actions/#orbitship) (action),
[`Or`](../filters/#conditional) (filter combinator),
[`order_interruption`](../objects/#the-controller) (AI controller field),
[`Outcome`](../actions/#outcome) (action)

**P** - [`Parens`](../expressions/#factors-the-atoms) (expression node),
[`PatrolShip`](../actions/#patrolship) (action),
[`Planet`](../objects/#planet) (object),
[`Player`](../objects/#the-controller) (ship controller),
[`Plume`](../sections/#damage-effects) (damage effect),
[`Point`](../objects/#light) (light method),
[`Prototype`](../objects/#the-sections-list) (section source)

**Q** - [`Query`](../expressions/#factors-the-atoms) (expression node)

**R** - [`RefillAmmo`](../actions/#refillammo) (action),
[`ReleaseCamera`](../actions/#releasecamera) (action),
[`Rename`](../objects/#the-sections-list) (section modification),
[`ResumePlayerControl`](../actions/#resumeplayercontrol) (action),
[`Ring`](../actions/#scatterobjects) (scatter region)

**S** - [`SalvageCrate`](../objects/#salvagecrate) (object),
[`ScatterObjects`](../actions/#scatterobjects) (action),
[`Scenario`](../scenarios/) (content item),
[`Screenshot`](../actions/#screenshot) (action),
[`Section`](../sections/) (content item),
[`Sequence`](../actions/#sequence) (action),
[`SetAIEngageRange`](../actions/#setaiengagerange),
[`SetAILeash`](../actions/#setaileash),
[`SetAIPointDefenseRange`](../actions/#setaipointdefenserange) (actions),
[`SetAllegiance`](../actions/#setallegiance),
[`SetCamera`](../actions/#setcamera),
[`SetCameraAnchor`](../actions/#setcameraanchor),
[`SetControllerVerb`](../actions/#setcontrollerverb) (actions),
[`SetInfiniteAmmo`](../actions/#setinfiniteammo) (action),
[`SetAmmo`](../objects/#the-sections-list) (section modification),
[`SetHealth`](../objects/#the-sections-list) (section modification),
[`SetSkybox`](../actions/#setskybox),
[`SetSpeedCap`](../actions/#setspeedcap),
[`Ship`](../ships/) (content item),
[`ShipOrder`](../filters/#shiporder) (filter),
[`Sparks`](../sections/#damage-effects) (damage effect),
[`SpawnScenarioObject`](../actions/#spawnscenarioobject) (actions),
[`Spaceship`](../objects/#spaceship) (object),
[`StopShip`](../actions/#stopship),
[`StoryMessage`](../actions/#storymessage) (actions),
[`String`](../expressions/#values-the-literal-types) (literal),
[`Style`](../styles/) (content item),
[`Subtract`](../expressions/#expressions-add-subtract-the-value-root) (expression node),
[`SuspendPlayerControl`](../actions/#suspendplayercontrol) (action)

**T** - [`Term`](../expressions/#expressions-add-subtract-the-value-root) (expression node),
[`Timer`](../filters/#timer) (filter),
[`TimerCancel`](../actions/#timercancel),
[`TimerStart`](../actions/#timerstart) (actions)

**V** - [`VariableSet`](../actions/#variableset) (action)

Typed queries and their auto-updating watched variables also belong to the
vocabulary:
[queries and watched variables](../expressions/#queries-and-watched-variables).

## Extending the vocabulary

A construct that does not exist here cannot be authored in RON - adding an
event, filter, action or object kind is a small Rust change, covered in the
[developer book](../../dev/). If you add one, its entry lands on these pages
in the same task.
