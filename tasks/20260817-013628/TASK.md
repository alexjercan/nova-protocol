# Load a scenario from the command line

- STATUS: CLOSED
- PRIORITY: 50
- TAGS: v0.11.0,cli,scenario

## Goal

Owner-approved: `--scenario <id>` on the GAME BINARY loads the scenario
straight past the menu - "especially for the CLI nerds that don't want to
use the main menu". Also serves webmod authors testing a scenario id in one
command.

## Shape

- Native binary flag (the game already has CLI subcommands - follow that
  structure); wasm untouched.
- Unknown id fails loudly and LISTS the available scenario ids.
- Rides the existing loader path (non-blocking load screen included).

## Done when

- `nova-protocol --scenario <id>` boots into that scenario, proven live
- unknown id errors with the id list
- dev wiki page updated; CHANGELOG entry
- the scenario_id example is NOT touched (coordinator retires it after this
  lands)

## Closure

Landed 2026-08-17, lane scenario-flag (opus). `nova-protocol --scenario
broadside` boots into the belt with the flight HUD and no menu; bogus ids
refuse with the sorted list of all 18 registered ids (mods and hidden
chapters included) and exit code 1. The refusal fires at OnEnter(Loaded) -
the earliest point the merged registry exists - because a pre-window check
would mean a second source of truth for bundle discovery. Wiki documents the
flag beside the Content CLI. wasm job command verified clean.

The scenario_id example retires separately (coordinator, after the taxonomy
lane lands).
