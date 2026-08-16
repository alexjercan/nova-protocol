# Load a scenario from the command line

- STATUS: IN_PROGRESS
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
