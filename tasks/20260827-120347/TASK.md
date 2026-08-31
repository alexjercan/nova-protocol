# The console and the action vocabulary: reach into the world by name

- STATUS: OPEN
- PRIORITY: 50
- TAGS: v0.13.0,console,input,tooling

Split out of `20260820-174148`, which now covers `input` only.

The channel's second vocabulary: reaching into the world by name rather than
pressing a key. Deferred because it should be designed once, together with the
in-game console, rather than bolted onto stdin now and reconciled later.

## Why it waits

`action` is not a stdin feature. It is a command layer that happens to be
reachable from stdin when there is no window. In a GUI there is no keyboard for
it - it belongs to a console, CSGO-style, NovaOS-styled but not the in-fiction
NovaOS and not limited to the player ship. Building the stdin half first would
mean designing arming, classification and eligibility twice.

Likely shape: `action <name> <args>` as one console sub-command, so the console
owns dispatch and stdin is one more front end onto it.

## What is already decided

Recorded in `tasks/20260820-174148/design.html` (sections "Q4", "Two subjects",
"Arming", "Where eligibility does need a class"). Carry it here, do not redo it.

- Every action is documented, cheats in their own catalog section. An
  undocumented action is worse than a documented cheat.
- `cfg(debug)` gating is not available: the channel ships in release, so gating
  would make the vocabulary differ between builds.
- Two subjects, kept apart. The RUN is cheated - set by origin, at runtime,
  one-way. The SCENARIO is a creative map - set by its script, computed at lint
  time, never authored. Only the first accuses anyone.
- Origin decides the mark: scenario/mod never marks, `input` never marks,
  `command` never marks, `action` always marks.
- Arming is one deliberate act that sets the one-way bit, Minecraft-style, so
  the mark is never discovered mid-run.
- Eligibility needs a class on 8 of the 26 `EventActionConfig` variants, not all
  26: SpawnScenarioObject, ScatterObjects, DespawnScenarioObject, SetSpeedCap,
  SetControllerVerb, SetAllegiance, ForceTorpedoLaunch, CreateScenarioArea. The
  other 18 are bookkeeping and presentation - classing them all would make every
  scenario a creative map.
- `Outcome` and `NextScenario` need no class: bookkeeping in a script, instant
  win and level skip from a console, and the origin rule already catches them.

## Also here

`infinite_ammo` stops being a field on `SpaceshipController::Player` and becomes
an action. Verified: no shipped scenario sets it true - all six base
`.content.ron` and the example mod pass `false`; only `debug`-feature examples
pass `true`. Cost: the flag decides whether `SectionAmmo` is attached AT SPAWN,
so as an action it must add and remove the component on live entities.

## Depends on

`20260820-174148` landed in v0.12.0 with the transport, the registry and the
snapshot. The line schema keeps the `action` key reserved, so adding it is
additive. Scheduled into v0.13.0 (2026-08-31 planning round).
