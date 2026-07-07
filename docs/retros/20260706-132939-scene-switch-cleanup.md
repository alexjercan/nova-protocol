# Retro: scene-switch entity cleanup (task 20260525-132939)

## What was asked
Full cleanup on scene transition; no leftover entities between scenarios.

## What happened
Exhaustively swept every `commands.spawn` reachable during an active scenario and
confirmed each is torn down on a switch (explicit `ScenarioScopedMarker`, auto-scoped
via `on_add_entity_with`, child of a scoped entity, `TempEntity` self-despawn, or tied
to a `Remove<PlayerSpaceshipMarker>` observer). The bug was already resolved by the
scoping architecture, so no code change was needed. Documented the five-bucket cleanup
contract in docs/scenario-system.md to prevent regressions.

## Lessons
- A "bug" task can legitimately resolve to "already fixed" - but only after proving it.
  The exhaustive spawn-site sweep is what made the verification trustworthy rather than
  an assumption.
- When the fix is "nothing to change", the durable value is capturing the invariant
  (the cleanup contract) so the next person adding a spawn site knows the rule.
- `despawn()` is recursive in Bevy 0.19, so scoping a root scopes its whole subtree -
  worth stating explicitly since it changes what needs its own marker.
