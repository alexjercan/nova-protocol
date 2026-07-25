# Mirror OnNeutralized handlers in The Ledger webmod

- STATUS: CLOSED
- PRIORITY: 29
- TAGS: v0.9.0,modding,scenario,bug

## Flow State

- FLOW STEP: DONE
- PLAN STATUS: APPROVED

## Story

The neutralized-ship feature added `OnNeutralized` handling for the generated
base scenarios, but The Ledger is a hand-authored webmod under
`webmods/the-ledger/`. Its combat objectives and player death paths still only
react to `OnDestroyed`, so an armed Ledger ship that loses weapons and
thrusters can become combat-dead without completing the corresponding mod
objective or defeat/retry path.

Done means The Ledger's armed ship kill/defeat paths mirror the base campaign:
an armed target neutralizing counts as beaten, player neutralizing is a Defeat
retry where the chapter already has a player death retry, and the mod lint/tests
prove the changed hand-authored RON is valid.

## Steps

- [x] Audit `webmods/the-ledger/*.content.ron` for `OnDestroyed` handlers and
      classify which ids are armed ships versus pickups, unarmed cargo, or
      non-ship objects.
- [x] Add `OnNeutralized` sibling handlers in `ledger_ch2.content.ron` for
      `magpie_1`, `magpie_2`, and `player_spaceship`, mirroring the existing
      `OnDestroyed` flag/defeat actions.
- [x] Add `OnNeutralized` sibling handlers in `ledger_ch2b.content.ron` for
      `magpie_3`, `magpie_4`, and `player_spaceship`, mirroring the existing
      `OnDestroyed` flag/defeat actions.
- [x] Add an `OnNeutralized` player defeat sibling in `ledger_ch3.content.ron`;
      leave stealth watch area triggers and non-destroy completion paths
      unchanged unless the audit finds an armed ship `OnDestroyed` completion.
- [x] Add `OnNeutralized` siblings in `ledger_ch4.content.ron` for the
      `auditor` sell-path fight and `player_spaceship`, preserving the
      existing SELL/BURN terminal-act guards.
- [x] Add `OnNeutralized` siblings in `ledger_ch5_the_raid.content.ron` for the
      armed raid targets and `player_spaceship`, preserving existing objective
      flags, terminal acts, and retry target.
- [x] Add the missing `OnNeutralized` player retry in `ledger_ch1.content.ron`
      after the audit found chapter one also has an armed player ship and a
      player death retry.
- [x] Update The Ledger changelog or bundle version if the mod's local
      convention treats scenario behavior changes as a published mod release.
- [x] Add or extend Ledger scenario tests only where existing tests do not
      already cover the changed neutralize paths; prefer the existing
      `ledger_ch2_encounter`, `ledger_ch3_channel`, `ledger_ch4_ending`, and
      `ledger_ch5_raid` rigs.

## Definition of Done

- Every armed Ledger ship that has an `OnDestroyed` objective-completion path
  also has an `OnNeutralized` path with equivalent idempotent actions.
  (cmd: `rg -n "name: OnNeutralized" webmods/the-ledger/*.content.ron`)
- Player neutralization in Ledger chapters with player death retry declares
  Defeat and queues the same scenario retry with the same terminal guard shape.
  (test: existing or extended Ledger scenario tests fire `OnNeutralized` for
  the player in the touched chapters.)
- The Ledger content stays valid and balance-audit expectations remain
  understood. (cmd: `nix develop --command cargo run -p nova_assets --bin content -- lint --target the-ledger`)
- The touched Ledger behavior tests pass. (cmd: `nix develop --command cargo test -p nova_assets --test ledger_ch2_encounter --test ledger_ch3_channel --test ledger_ch4_ending --test ledger_ch5_raid`)
- Repository conformance is clean. (cmd: `tatr check --ledger LESSONS.md`)

## Notes

- The Ledger webmod files are hand-authored RON, not generated base content.
- Current checkout already has unrelated uncommitted mainline generator fixes:
  `LESSONS.md` and `crates/nova_assets/src/scenario*.rs`. Keep this task's
  edits explicit and do not stage unrelated files.

## Implementation Notes

- Added `OnNeutralized` player retry siblings in all five Ledger chapter files
  with player-death retries, including chapter one found during the audit.
- Added `OnNeutralized` objective/win siblings for armed enemies: ch2
  `magpie_1`/`magpie_2`, ch2b `magpie_3`/`magpie_4`, ch4 `auditor`, ch5
  `raider_1`..`raider_4` and `magpie_base`.
- Changed the counter-based ch2/ch2b/ch5 kill paths to seed and check per-target
  down flags, so a neutralized wreck later firing `OnDestroyed` cannot double
  increment/decrement the objective counters.
- Bumped The Ledger bundle version to `1.14.0`, added a changelog entry, and
  updated the live mod-authoring wiki version example.

## Verification

- `nix develop --command cargo fmt --check` passed. Nix also printed an ignored
  busy eval-cache warning.
- `nix develop --command cargo run -p nova_assets --bin content -- lint --target the-ledger`
  passed: 0 errors, 0 warnings, 0 findings; 1 existing Auditor balance ack.
- `nix develop --command cargo test -p nova_assets --test ledger_ch2_encounter --test ledger_ch3_channel --test ledger_ch4_ending --test ledger_ch5_raid`
  passed: 16 + 18 + 12 + 13 tests.
