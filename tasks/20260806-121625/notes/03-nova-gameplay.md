# nova_gameplay

77,761 LOC / 169 files - half the workspace, 8 in-workspace dependents.

## Module map (`src/`)

| Module | LOC | Owns |
| --- | --- | --- |
| `hud` | 33,756 | **43% of the crate.** See breakdown below |
| `input` | 12,300 | player + AI + targeting |
| `sections` | 9,900 | ship parts |
| `flight` | 5,900 | manual flight + autopilot + guidance |
| `camera` | 3,600 | chase/WASD rigs, authority, shake, skybox, post |
| `audio` | 2,700 | SFX bank, mixing, combat/UI cues |
| `integrity` | 2,400 | health/disable/destroy |
| `asset_ref` | | path-string -> `Handle` resolution |
| `beacon` | | 2 types; barely a module |
| `cooldown`, `damage`, `gravity`, `juice`, `lifetime`, `math`, `mesh`, `objectives`, `physics`, `plugin`, `relations`, `settings`, `transform` | | as named |

`hud/` breakdown: widgets ~19k + `nova_os` 8.6k + `nova_os_ship` 3.2k +
`nova_os_map` 2.5k + `nova_os_pointer_rig`. **NOVA OS is a 14.3k-line
terminal/windowing runtime living under a folder named `hud`.**

Opaque names (all have good `//!` docs, so the cost is navigation only):
`juice`, `integrity/glue.rs`, `hud/emphasis.rs`, `hud/situation.rs`,
`hud/readout.rs` (a scenario-variable strip, not "the readouts"), and
`hud/nova_os/casing.rs` which has **no module doc at all**.

## The four seams - verified acyclic

Cross-seam edge counts, from `crate::` paths in non-test files:

```
FLIGHT->CORE  30      HUD->CORE  26      HUD->FLIGHT  4
NOVAOS->CORE   3      NOVAOS->HUD 2
CORE->FLIGHT   6      CORE->HUD   1      <- the only back-edges
```

Layer order: **CORE <- FLIGHT <- HUD <- NOVAOS**.

Every back-edge site, exhaustively:

| Site | Nature | Resolution |
| --- | --- | --- |
| `plugin.rs:107` `crate::input::SpaceshipInputPlugin` | composition root | lifts into the assembly crate |
| `plugin.rs:111` `crate::hud::NovaHudPlugin` | composition root | same |
| `plugin.rs:115` `crate::flight::NovaFlightPlugin` | composition root | same |
| `camera/framing.rs:200` `crate::flight::is_forward_aligned` | pure math helper | move to `math` |
| `sections/controller_section.rs:301` `.after(crate::flight::NovaFlightSystems)` | **real scheduling edge** | invert - `flight` declares `.before` - or use a shared set |
| `sections/controller_section.rs:225,261` | doc mentions of `crate::flight` only | reword |

Only `controller_section.rs:301` is a genuine design decision. Everything else
is mechanical.

Caveat: the edge counts come from `crate::`-prefixed paths. Sibling `super::`
and intra-module `use` may add edges. Re-verify before cutting.

### Seam contents

| Seam | Modules |
| --- | --- |
| CORE | sections, integrity, damage, relations, mesh, physics, transform, gravity, lifetime, math, asset_ref, cooldown, beacon, audio, settings, juice, objectives |
| FLIGHT | flight, input |
| HUD | `hud/` minus `nova_os*` (~19k) |
| NOVAOS | `hud/nova_os` 8.6k, `nova_os_map` 2.5k, `nova_os_ship` 3.2k, `nova_os_pointer_rig` |

## NOVA OS has no owner

The feature is smeared across three crates coupled by direct type imports:

| Where | What |
| --- | --- |
| `nova_os` (crate) | terminal model, shell, command registry, app runtime. No UI |
| `nova_gameplay/src/hud/nova_os*` | all the UI, ~14.3k |
| `nova_menu` | **state and settings.** `NovaOsMonitorSettings` init at `src/lib.rs:109`; `OnEnter/OnExit(PauseStates::NovaOs)` clock+cursor hooks at `lib.rs:185-190`; persistence at `settings_store.rs:86`; special-case at `pause.rs:51-54` |

The "nova_os owns no UI" rule is honored. The feature still has no single owner.
The only bevy-UI leak in nova_os is `src/app.rs:54`
`spawn_body(&self, body: &mut ChildSpawnerCommands, font: Handle<Font>)`.

## Coupling

`nova_events` appears 10 times in 9 files - and that is **correct**, not
under-use. Those 10 are the scenario-observable moments:

```
integrity/neutralize.rs:18,157   integrity/explode.rs:14   integrity/glue.rs:611
input/ai/behavior.rs:7           input/ai/passive.rs:7
hud/nova_os_ship/sections.rs:2   hud/nova_os_ship/tests.rs:8
hud/nova_os_map/contacts.rs:2    hud/nova_os_map/tests.rs:6
```

Intra-crate game logic correctly uses **observer-on-marker**: 46 files use
`On<Add|Remove|Insert, T>`. `On<Add, PlayerSpaceshipMarker>` appears in 14
places, `On<Add, IntegrityDestroyMarker>` in 11. Plus direct queries -
`camera/mode.rs` reads flight + integrity + sections; `hud/*` reads sections,
flight, input.

**Do not migrate this to events.** See `01-decisions.md`. The AGENTS.md line
is what needs fixing.

Deep-path imports are rare inside the crate (`math` x5, `hud` x2) but present
cross-crate: `plugin.rs:104` `nova_ui::status_bar::StatusBarPlugin`, plus
`nova_ui::hud::*` and `nova_os::shell::*` in 10 files.

## Plugin and SystemSet organization

- `plugin.rs:80-101` adds **13 leaf plugins directly** - `camera::wasd`, five
  `transform::*`, `lifetime::{Temp,Despawn}`, `mesh::ExplodeMesh`,
  `physics::PDController`. There is no `NovaTransformPlugin` or
  `NovaLifetimePlugin`.
- 70 `impl Plugin` against 27 `SystemSet` types. Only **6 sets** are in the
  top-level chain (`plugin.rs:128-153`), so `NovaFlightSystems`,
  `NovaGravitySystems` and `IntegritySystems` order only by luck.
- `input/mod.rs` is the model of how it should look: one plugin, one set,
  orienting module doc.

## `render: bool` is a lie

`plugin.rs:40` documents `NovaGameplayPlugin::render` as "whether the
render-side plugins (meshes, HUD, particles) are added". It is forwarded only to
`SpaceshipSectionPlugin` (`:109`). Hanabi (`:77`), skybox (`:85`), post (`:86`)
and the entire HUD (`:111`) are added unconditionally.

**The advertised headless mode does not exist.** Making it real unblocks
HUD-free tests.

## Size outliers

Production lines, tests excluded:

| File | Lines | Note |
| --- | --- | --- |
| `flight/autopilot.rs` | 939 | **zero `#[test]`** |
| `hud/nova_os_ship/scene.rs` | 840 | near-twin of `nova_os_map/scene.rs` (616) |
| `hud/mod.rs` | 839 | `:531-840` is per-widget spawn/material code belonging in the widget files |
| `hud/keybind_dock.rs` | 836 | |
| `input/ai/passive.rs` | 825 | |
| `hud/target_inset.rs` | 813 | |
| `input/ai/maneuver.rs` | 792 | |
| `hud/nova_os/spawn.rs` | 715 | |
| `sections/thruster_section.rs` | 704 | |
| `hud/nova_os/casing.rs` | 662 | no module doc |

Duplication: `hud/nova_os_map/scene.rs` and `hud/nova_os_ship/scene.rs` are
near-parallel orbit-camera / blip / pointer implementations, 616 + 840 lines.

## Dead surface

Of 622 pub items, **16 have exactly one non-definition reference** (definition
plus prelude re-export only): `NovaOsMapSystems`, `NovaOsShipSystems`,
`WASDCameraControllerSystems`, `ObjectiveId`, `TorpedoBlastEffectMarker`,
`MeshBuilder::subdivide` (`mesh/builder.rs:362`), `SoundBank::load_paths`
(`audio/registry.rs:72`), and 9 others.

One `#[allow(dead_code)]`: `hud/nova_os_ship/sections.rs:318`.

cfg usage is clean: 149 `#[cfg(test)]`, 13 serde, 13 debug, 4 wasm. No thickets.

## The seams now carry known bugs - added 2026-08-07

This module map and seam analysis were written **before** the code review. Six
reviewers then found defects inside the same code. A seam is not just a
dependency cut any more; three of the four carry defects that are cheaper to
fix before the move than after, because after the move the lines have shifted
and the reviewer's citations no longer resolve.

| Seam | Known defects in it | Source |
| --- | --- | --- |
| **NOVAOS** (`hud/nova_os*`, 14.3k) | Ctrl+letter types into the prompt (`hud/nova_os/input.rs:267`); `f32::MAX` scroll sentinel never cleared (`shell.rs:379`); physical-vs-logical scroll clamp (`input.rs:430`); scrollback rebuilt every keystroke (`shell.rs:344`); `crt.rs:219` relayouts every frame while the monitor is hidden; stale `Local<usize>` (`shell.rs:363`); `NovaOsShipSystems`/`NovaOsMapSystems` have no ordering edge | `10` |
| **HUD** (~19k) | `readout.rs:207` allocates two Strings per readout per frame before the equality compare; `nova_os_ship/scene.rs:750,772` unconditional colour writes | `10`, `13` |
| **FLIGHT** (`flight`, `input`) | The 16-line engine-spool duplicate at `flight/autopilot.rs:877` / `flight/manual.rs:142`, both copies carrying an O(ships x thrusters^2) per-tick scan; AI cadence timers ticked on `Update` while firing is `FixedUpdate` (`input/ai/mod.rs:107`) | `13`, `14` |
| **CORE** (sections, mesh, audio, objectives, ...) | Every bullet allocates a Mesh + StandardMaterial (`sections/turret_section/render.rs:129`); one bad child aborts a whole explosion and leaves a live wreck (`mesh/explode.rs:130,144`); `fire_rate: 0.0` panics on spawn (`turret_section/setup.rs:64`); torpedo homes on the build-spot origin (`torpedo_section/projectile.rs:37`); two stale-`Local` audio leaks (`audio/cues.rs:99,147`); `objectives.rs:123 rebuild_lines` can never run | `14` |

All citations above re-verified against the tree 2026-08-07. Full detail and
severity in `16-findings-master.md`.

**Consequence for the split order.** The order in `../NOTES.md` idea 4
(NOVAOS -> HUD -> FLIGHT -> CORE) still holds, but NOVAOS now carries the
densest defect cluster in the crate as well as the biggest navigability win.
See `17-lanes.md`.

## Testability

861 tests. Rigs are good: `integrity/test_support.rs`,
`input/player/test_support.rs`, `flight/tests/support.rs`. ~9.2k lines of test
in `tests/` dirs plus ~22k inline, so several big files are mostly tests
(`keybind_dock.rs` 1,072 of 1,908; `screen_indicator.rs` 853 of 1,485).

Hard to test:

- `flight/autopilot.rs` - 939 lines, **no unit tests**; covered only via sibling
  app rigs.
- Everything under `hud/nova_os*` needing a real render target. `nova_os/crt.rs:140`
  and `nova_os_map/scene.rs:55` explicitly skip headless; 11 files over 400
  lines carry zero tests.
- Input tangling - 13 systems outside `input/` read raw `Res<ButtonInput<..>>`:
  `hud/mod.rs:390`, `hud/comms_panel.rs:262`, `hud/nova_os_ship/scene.rs:340`,
  `hud/nova_os/input.rs:30,55,341`.
- State entanglement - gates on `GameStates` + `PauseStates` +
  `GameAssetsStates`, so any App test boots `StatesPlugin` and three machines.
