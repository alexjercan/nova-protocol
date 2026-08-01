# NOTES - 20260731-170340

KISS pass on `crates/nova_gameplay/src/input/` - the gameplay input layer
(player, AI, targeting). Both axes: all three oversized files split into
folder modules, comment rubric applied across every file.

## Structure axis

Before: 3 files held 11 820 of the area's 12 031 lines.

| Before | Lines |
|-|-|
| ai.rs | 5427 |
| targeting.rs | 3666 |
| player.rs | 2727 |
| reference.rs | 165 |
| mod.rs | 46 |

Each split is `mod`-level only: a folder module whose `mod.rs` keeps the
module doc, the plugin, the prelude and the umbrella markers, plus one
sibling file per concern. Public paths are unchanged: each `mod.rs`
re-exports every name the pre-split file made visible at the parent path
(`pub use self::{...}` for the public API, `pub(crate) use self::{...}` for
the input-action types; the three test-only helpers - `keyboard_label`,
`flight_input_rig` and `update_weapons_safety_for_tests` - are re-exported
under `#[cfg(test)]`, matching their only callers and keeping the lib target
warning-free), so
`input::targeting::CombatLock`, `input::player::binding_label` and
`input::ai::AITarget` still resolve exactly as before. The preludes and
`crate::prelude::*` are byte-identical to master, and no call site outside
`input/` needed repointing.

The re-exports are explicit name lists rather than globs: a
`pub(crate) use self::x::*;` glob is what produces the `ambiguous import
visibility` warnings the `nova_os_*` modules already carry, and this pass
should not add more.

### After

Prod/tests measured at each file's first `#[cfg(test)] mod` boundary.

| File | Lines | Prod | Tests | Concern |
|-|-|-|-|-|
| ai/acquisition.rs | 969 | 290 | 679 | who an AI ship fights: primary target, point-defense override, combat-state mirror |
| ai/behavior.rs | 1024 | 371 | 653 | the behavior state machine and its transition rules |
| ai/passive.rs | 825 | 160 | 665 | patrol / orbit / station-keep through the real autopilot |
| ai/maneuver.rs | 792 | 292 | 500 | standoff + jink steering, rotation command, gated thrust |
| ai/guns.rs | 630 | 271 | 359 | turret aim, burst cadence, line-of-fire, trigger |
| ai/torpedo.rs | 565 | 228 | 337 | launch envelope, per-bay cooldown, bay target |
| ai/threat.rs | 549 | 205 | 344 | under-fire memory and the evade clocks |
| ai/mod.rs | 164 | 164 | 0 | plugin, ship markers, engage grace, prelude |
| targeting/gesture.rs | 1076 | 170 | 906 | the CTRL hold/tap gesture end to end |
| targeting/contacts.rs | 984 | 422 | 562 | per-frame lock upkeep, drop reporting, threat ranking, focus dwell |
| targeting/radar.rs | 639 | 257 | 382 | the live radar search, pick rules, dwell curve |
| targeting/component_lock.rs | 493 | 297 | 196 | the section fine-lock: snap and pin |
| targeting/state.rs | 291 | 291 | 0 | lock components, settings, messages |
| targeting/safety.rs | 128 | 61 | 67 | weapons safety derivation and trigger interrupt |
| targeting/mod.rs | 140 | 140 | 0 | plugin, system set, state-insert observer, prelude |
| player/flight_rig.rs | 1001 | 577 | 424 | the always-on rig: burn, autopilot verbs, RCS |
| player/intent.rs | 705 | 218 | 487 | rotation command and the turret/torpedo target feeds |
| player/hints.rs | 646 | 341 | 305 | verb availability and keybind labels for the HUD |
| player/weapons.rs | 285 | 285 | 0 | content `input_mapping` bindings and their trigger observers |
| player/mod.rs | 127 | 21 | 106 | plugin, `PlayerSpaceshipMarker`, prelude |
| player/test_support.rs | 57 | 57 | 0 | shared test rig (`hint_world`, `spawn_flyable_ship`) |
| reference.rs | 165 | 139 | 26 | unchanged - the static keybind reference |
| mod.rs | 46 | 46 | 0 | unchanged - the input umbrella plugin |

No file exceeds 1500 lines, so DoD 4 needs no exception.

`player/mod.rs` reads as 21 prod lines only because the boundary rule hits its
`#[cfg(test)] mod test_support;` declaration; the file is 127 lines of plugin
and prelude with no test module of its own.

### Two judgement calls

- **`player/test_support.rs` exists** because `hint_world` and
  `spawn_flyable_ship` are used by BOTH the hint tests and the flight-rig
  gesture tests. Duplicating them would be worse, and it is not a new
  abstraction: the file is `#[cfg(test)]`-only and mirrors the crate's
  existing `integrity::test_support`.
- **`targeting/gesture.rs` was carved out of `radar.rs`** after the first cut
  left radar at 1693 lines. The live search (pick on the ray, charge the
  dwell) and the CTRL gesture that opens/commits/clears it are two concerns,
  and separating them puts each under 1100 lines.

### Visibility

Items that stayed private before now need to cross a module boundary. They
were widened to the narrowest visibility that works - `pub(super)` for
module-internal systems and constants, `pub(crate)` for the few struct fields
and methods on public components that sibling modules read (`AIThreat`,
`AIEvade`, `AIFireCadence`, `AIPatrolRoute`, `AIBehaviorState::engages`). No
item became `pub`; the crate's external API is byte-identical.

Test-only cross-module imports carry `#[cfg(test)]` so the lib target stays
warning-free.

The first cut repointed three out-of-module call sites
(`hud/key_glyphs.rs` and two in-tree test paths) instead of re-exporting.
Review round 1 called that correctly: the compiler was satisfied while the
public paths had silently moved. The re-exports above replace it, and all
three call sites are back to their master spelling.

## Comment axis

Provenance stripped everywhere: tatr IDs, bare dates, spike/decision/review
labels (`Q1a`, `D7`, `R1.1`, `MINOR 1`) and the three `docs/spikes/*.md` paths
(`docs/` is ephemeral scratch per AGENTS.md, so those pointers rot). The prose
that carried them stays - none of the surviving explanation depends on the
citation.

`grep -rnE '//.*[0-9]{8}-[0-9]{6}' crates/nova_gameplay/src/input/` returns
exactly one hit, and it is deliberate - DoD 3's list in full:

| Hit | Why it stays |
|-|-|
| `reference.rs:10` - `full remapping + key icons stay backlog (TODO: 20260710-231927)` | Deferred work with a live backlog task; the epic rubric's "keep as TODO with the tatr ID if one exists" row. |

Eleven comments were promoted to `NOTE:` - each guards a value or an engine
constraint that a future edit would otherwise silently break:

| Site | Guards |
|-|-|
| ai/mod.rs (x2) | threat sensing must be an observer (source resolves before despawn); torpedo commit must run before the trigger write |
| targeting/mod.rs | the state bundle rides the marker, not the spawn site |
| targeting/radar.rs | `min.max(max)` before `clamp` - `f32::clamp` panics on min > max |
| targeting/component_lock.rs (x2), player/weapons.rs (x3) | observers bypass system-set gating, so the pause freeze is hand-rolled |
| player/intent.rs | the command slews instead of jumping, or the PD saturates and the hull limit-cycles |
| ai/guns.rs | the unreachable `else` arm is belt-and-braces, not dead logic |

Twelve `// -- section --` separators were deleted (`grep -nE '^\s*//\s*-{2,}'`
returns 4 in `ai.rs`, 1 in `player.rs` and 7 in `targeting.rs` on master; zero
remain). Several had drifted into the wrong file during the split (a "radar
search" banner sitting in `component_lock.rs`), which is exactly the failure
mode they invite.

No comment that explains WHY was deleted; per the epic rubric only narration,
provenance and dead separators go.

## Evidence

- `cargo check --workspace --all-targets` - clean (the two `nova_os_*`
  ambiguous-import warnings predate this branch).
- `cargo fmt --check` - clean.
- `cargo test -p nova_gameplay --lib input::` - **180 passed, 0 failed**.
- Test count is conserved exactly: 97 + 28 + 54 = 179 `#[test]` fns before,
  179 after (plus `reference.rs`'s 1, which was never touched).
- Code lines were compared as a multiset against the pre-split files: every
  removed line is a declaration whose visibility changed or an import that
  moved. No executable line was altered.
- Parent-path reachability was probed directly: a throwaway `#[cfg(test)]`
  module importing `input::ai::{AIBehaviorState, AIFireCadence, AITarget,
  AIThreat, AITorpedoBay}`, `input::player::{binding_label,
  flight_rig_reserved_sources, FlightVerbHints, VerbHint}` and
  `input::targeting::{CombatLock, ComponentLock, RadarState, TravelLock,
  COMBAT_DECAY_SECS}` compiles clean. The probe was removed after the run - it
  proves the re-exports, it is not a test worth keeping.
- `cargo doc -p nova_gameplay --no-deps` - exactly one unresolved intra-doc
  link, `nova_assets` in the untouched `hud/key_glyphs.rs`. The
  `super::player::binding_label` link that round 1 found broken now resolves,
  and no other link into the moved modules is unresolved.

## Doc-surface sweep

Splitting the files invalidated every comment elsewhere in the tree that named
one of them by path. Repointed to the new module in `nova_gameplay`
(`flight.rs` x3, `audio.rs`, `hud/{component_lock,lock_dwell_ring,
target_inset}.rs`, `input/{reference.rs, ai/torpedo.rs, player/flight_rig.rs,
targeting/state.rs}`), in `nova_scenario` (`loader.rs` x2,
`objects/beacon.rs`), in `nova_assets` (`balance.rs` x2,
`scenario/broadside.rs`) and in the wiki (`web/src/wiki/dev/project-tour.md`,
whose project-tour table pointed at `input/{player,ai}.rs` and
`input/targeting.rs`).

`grep -rn 'input/player\.rs\|input/targeting\.rs\|input/ai\.rs'` over
`crates/` and `web/src` returns nothing. The `tasks/` tree is exempt as
append-only history.
