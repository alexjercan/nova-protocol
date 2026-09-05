# Scenario vocabulary inventory

What a scenario, a campaign and a mod can say today, verified against the tree
at `3af5d051`. This is the baseline the story rewrite is measured from.
`GAMEPLAY_PLAN.md` lists what the story asks for; this file lists what exists,
what it costs to add one more, and where the seams are.

## 1. Where the vocabulary lives

| Concern | File |
| --- | --- |
| Handler triggers | `crates/nova_scenario/src/events.rs` |
| Handler predicates | `crates/nova_scenario/src/filters.rs` |
| Action enum and dispatch | `crates/nova_scenario/src/actions/mod.rs` |
| Action payloads | `crates/nova_scenario/src/actions/{flow,mission,sequence,ship,spawn,timer,view}.rs` |
| Read-only queries | `crates/nova_scenario/src/queries.rs` |
| Spawnable objects | `crates/nova_scenario/src/objects/*.rs` |
| Scenario document | `crates/nova_scenario/src/loader/mod.rs` (`ScenarioConfig`) |
| Name kinds for lint and editor | `crates/nova_scenario/src/names.rs` |
| Offline checks | `crates/nova_scenario/src/lint/scenario.rs` (3 176 lines) |
| Editor reflection | `crates/nova_editor/src/event.rs` (2 547 lines) |
| Mod content kinds | `crates/nova_modding/src/lib.rs` (`Content`) |
| Bundle and portal formats | `crates/nova_mod_format/src/lib.rs` |
| Base campaign source | `crates/nova_authoring/src/base_content/scenarios/nova_protocol/` |

## 2. Events (24)

A handler names exactly one trigger. Payload fields are read by filters.

| Group | Events |
| --- | --- |
| Lifetime | `OnStart`, `OnUpdate` |
| Destruction | `OnDestroyed`, `OnDefeated`, `OnNeutralized` |
| Timers | `OnTimerEnd` |
| Areas | `OnEnter`, `OnExit` |
| Player maneuvers | `OnGotoComplete`, `OnStopComplete` |
| Orbit | `OnOrbitStart`, `OnOrbitStable`, `OnOrbitLap`, `OnOrbitUnstable`, `OnOrbitEnd` |
| Player locks | `OnTravelLockStart`, `OnTravelLockEnd`, `OnCombatLockStart`, `OnCombatLockEnd` |
| Scripted helm orders | `OnShipOrderComplete`, `OnShipOrderInterrupted`, `OnShipOrderResumed`, `OnShipOrderCanceled`, `OnShipOrderFailed` |

Gaps that matter to the story: nothing fires when a ship is HIT but alive, when
a section dies, when a pickup is collected, when a player interacts with an
object, or when a cinematic or a narrative line finishes.

## 3. Filters (5)

`Entity`, `Conditional` (`Not`/`And`/`Or`), `Expression`, `Timer`, `ShipOrder`.

All five fail closed: a set field with no payload to read is a mismatch, and an
expression over an undefined variable is `false` with a debug log. That rule is
load-bearing and every new filter must keep it.

Gaps: no group filter (an authored set of ids), no section filter, no
interaction filter.

## 4. Actions (42)

| Group | Actions |
| --- | --- |
| Bookkeeping | `DebugMessage`, `VariableSet`, `TimerStart`, `TimerCancel` |
| Mission surface | `Objective`, `ObjectiveComplete`, `ObjectiveMarkerAttach`, `ObjectiveMarkerDetach`, `HintEmphasisSet`, `HintEmphasisClear`, `StoryMessage`, `HudReadout` |
| World | `SpawnScenarioObject`, `ScatterObjects`, `DespawnScenarioObject`, `CreateScenarioArea`, `SetSkybox` |
| Ship state | `SetSpeedCap`, `SetInfiniteAmmo`, `RefillAmmo`, `SetControllerVerb`, `SetAllegiance` |
| Helm orders | `MoveShipTo`, `ForceAlign`, `StopShip`, `PatrolShip`, `OrbitShip`, `ClearShipOrder` |
| AI knobs | `SetAILeash`, `SetAIEngageRange`, `SetAIPointDefenseRange` |
| Scripted fire | `ForceRailgunFire`, `ForceTorpedoFire` |
| Camera and input | `SetCamera`, `SetCameraAnchor`, `ReleaseCamera`, `SuspendPlayerControl`, `ResumePlayerControl`, `Screenshot` |
| Flow | `NextScenario`, `Outcome`, `Sequence` |

`Sequence` is the only nesting arm. `EventActionConfig::walk` and
`walk_filters` are the single walkers every reader goes through, so a second
nesting arm cannot be honoured by one reader and missed by the others.

Every action is classed `Bookkeeping` or `Injection` by `own_injection`, which
drives the creative-map badge. A `Sequence` takes the strongest class of its
steps.

## 5. Queries (2 properties)

- `Scenario(Elapsed)`: live unpaused seconds.
- `Entity { id }.Speed`: one entity's speed in m/s.

Both are readable inline inside an expression, or declared as a `watch` that
publishes into a read-only variable each update. A query names exactly one
entity by exact id and is strict about it.

## 6. Objects (7)

`Anchor`, `Asteroid`, `Spaceship`, `Beacon`, `SalvageCrate`, `Light`, `Planet`.
Areas exist but only as a runtime action (`CreateScenarioArea`), not as a
declared object.

A `Spaceship` is a hull id plus a controller (`None`, `Player`, `AI`) and an
allegiance (`Player`, `Enemy`, `Neutral`). A `None` controller takes scripted
helm orders and scripted shots; an `AI` controller has judgement and knobs.

## 7. The scenario document

`ScenarioConfig`: `id`, `name`, `description`, `cubemap`, `skybox_brightness`,
`thumbnail`, `hidden`, `menu_backdrop`, `watches`, `events`. Objects are not a
field: a scenario spawns its world from `OnStart` handlers.

## 8. Mod capabilities

A mod is a directory plus a `*.bundle.ron` manifest listing its content files,
because `load_folder` is broken on wasm and a bundle can never enumerate its
own directory. `ModMeta` carries name, description, author, version,
dependencies, icon and screenshots. `base` is an implicit dependency of every
mod, resolved topologically by `nova_mod_format::deps`.

Content kinds a mod may declare (`nova_modding::Content`): `Section`,
`Scenario`, `Campaign`, `Style`, `Ship`, `Impact`. Merge is by id: a mod that
re-declares a base id replaces it.

So a mod can ship hulls, parts, looks, impact sounds, scenarios and campaigns.
It cannot ship code, new action kinds, new object kinds or new HUD widgets.
Everything a mod wants to say has to be sayable in the vocabulary above, which
is why the vocabulary is the product.

## 9. What it costs to add one action today

Adding `SetSkybox` touched, and adding any action still touches:

1. the payload struct in an `actions/*.rs` module,
2. `EventActionConfig` enum arm,
3. `EventAction` dispatch match arm,
4. `actions::prelude` export list,
5. `own_injection` exhaustive match,
6. `nova_editor::event::leaf_config`,
7. `nova_editor::event::leaf_config_mut`,
8. `ActionChoice` enum arm,
9. the `ActionChoice` menu order list,
10. the `ActionChoice` label match,
11. the `ActionChoice` icon match,
12. the `ActionChoice` default-value match,
13. the `EventActionConfig -> ActionChoice` match,
14. `lint::scenario::check_action`,
15. `web/src/create/actions.md`.

Nine of those fifteen are exhaustive matches over the same enum in two crates,
and four of them are the same three facts (label, icon, default) written in
three separate matches. Only points 1, 14 and 15 carry information; the rest is
transcription that a reviewer has to read to be sure nothing was dropped.

That is the refactor target. The classification (point 5) is worth keeping
exhaustive because it asks a real question, but it should be answered beside
the action, not in a distant match.

## 10. Gap against Nova Protocol scenario one

Story beats 1 (An ordinary shift) and 2 (The strike), from `STORY.md`.

Carried by what exists: the junk field, the moonlet, the carrier, the cutter
and its crew, GOTO and STOP teaching, crate recovery, the scripted warship that
comes out of the shadow, its torpedoes and railgun slugs, camera authority,
control suspension, and the outcome.

Missing, and needed to play beat 2 as written:

| Need | Why nothing today carries it |
| --- | --- |
| A skip-safe scene | A `Sequence` cannot be cancelled, and a skipped one leaves the camera and input wherever the cursor stopped. |
| A fragmentary guard-channel line | `StoryMessage` has one presentation. The story turns on the crew hearing "...roster..." and not the sentence. |
| A crew channel distinct from the work channel | Same reason: the cast already fakes it with a "Copilot - Cabin" speaker string. |
| An authored sound cue | No action plays one. |
| A camera that moves between poses | `SetCamera` snaps. |
| A chapter title card | No HUD banner. |

Everything else the story's later beats need is listed in `GAMEPLAY_PLAN.md`
and is deliberately out of this pass.

## 11. What the story pass changed

The sections above are the BASELINE at `3af5d051` and are left as measured.
This is the delta, so a reader can tell the inventory from the state.

| | baseline | now |
| --- | --- | --- |
| Events | 24 | 26 (`OnCinematicFinished`, `OnCinematicSkipped`) |
| Filters | 5 | 6 (`Cinematic`) |
| Actions | 42 | 45 (`Cinematic`, `CancelCinematic`, `PlaySound`) |
| Nesting arms | `Sequence` | `Sequence`, `Cinematic`, both read through `step_chain` / `step_chain_mut` |

Renamed: `StoryMessage` -> `NarrativeCue`, with a required `channel`
(`Comms` / `Crew` / `Guard`). `SetCamera` and `SetCameraAnchor` gained an
optional `blend`.

Section 9's fifteen edits are now five: the row in
`actions/registry.rs`, the payload struct, the editor's stock value, the lint,
and `web/src/create/actions.md`. The nine exhaustive matches are generated.

Five of the six gaps in section 10 are closed: the skip-safe scene, the
fragmentary guard line, the crew channel, the authored sound cue, and the
camera that moves. The chapter title card is not - the shift opens on a comms line rather
than a banner, and nothing in the story needs one yet.
