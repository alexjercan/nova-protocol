# Replace reserved scenario variables with typed properties

- STATUS: DONE
- PRIORITY: 0
- TAGS: backlog, modding, scenario

## Goal

Replace magic engine-owned scenario variables with a typed, supported property API.

## Scope

- Add read-only properties for scenario elapsed time and player speed.
- Migrate scenario_elapsed and player_speed usage.
- Keep world writes behind validated scenario actions.
- Define compatibility behavior and update modding docs and player-path coverage.
