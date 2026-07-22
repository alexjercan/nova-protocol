# Harden SetSkybox: warn-and-skip on a missing AssetServer instead of panicking (headless-safe)

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog, modding, scenario

## Story

Discovered during the Ledger per-chapter look task (20260722-214115): the
`SetSkybox` scenario action (`crates/nova_scenario/src/actions.rs:~324`) hard-
unwraps `world.resource::<AssetServer>()` and PANICS in any headless/minimal
context that lacks one. Production always has an AssetServer so this is not a
live-game bug, but it means every content rig that drives a SetSkybox-bearing
handler must add `AssetPlugin` + `init_asset::<Image>()` (as the ledger ch3/ch4
rigs now do). Mirror the graceful path one line below (which already
warn-and-returns on a missing scenario camera): use `get_resource::<AssetServer>()`
and warn-and-skip when absent, so future content rigs need no AssetPlugin.

Optional robustness/hardening; the production-faithful rig fix (give the rig the
AssetServer production has) is the current approach and works. Raise priority if
more mods start using mid-scenario SetSkybox.

## Steps

- [ ] Change the AssetServer access in SetSkyboxActionConfig::action to
      `get_resource` + warn-and-return (mirror the no-camera guard below it).
- [ ] Add/keep a nova_scenario unit pin: SetSkybox on a world with no
      AssetServer does not panic (fail-first: reverting the guard panics on the
      command flush).
- [ ] Once landed, the ledger ch3/ch4 rigs' `AssetPlugin`/`init_asset` additions
      become optional - leave or simplify deliberately.

## Definition of Done

- SetSkybox warns-and-skips on a missing AssetServer instead of panicking.
  (test: a nova_scenario unit test drives it headless without panic.)
- cargo check + the SetSkybox tests green. (cmd.)

## Notes

Surfaced by task 20260722-214115 (RETRO) under umbrella 20260722-212808. The
alternative considered and chosen for that task was the production-faithful rig
fix (AssetPlugin), per the `production-faithful-rigs` lesson.
