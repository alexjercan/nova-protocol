# Retrospective

## What worked

- Starting with an explicit catalog prevented a generic utility dump.
- Descriptive public names improved discoverability over local abbreviations.
- One `entity` helper correctly leaves lifecycle meaning on the event config.
- Hashing generated scenarios proved this was an ownership-only refactor.

## Fixes during implementation

- Added `set_number` after example migration exposed the common literal-write shape.
- Kept `player_enters` local because its player id is scenario policy.
- Updated an older balance-test fixture for the current `ScenarioConfig` schema.

## Next time

- Snapshot generated output before mechanical renames.
- Use syntax-aware replacement where constructor names overlap method names.
