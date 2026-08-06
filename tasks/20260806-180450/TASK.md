# Vendor bevy-common-systems

- PRIORITY: 0
- TAGS: v0.10.0
- ACTIVITY: WORKING
- GATES: PLAN
- RESOLUTION: -

## Problem

I don't want to depdend on `bevy-common-systems`.

## Context

Basically I feel like we sometimes make bad code decisions because BCS is
another crate and the thing is that __some__ features are not really generic
(see what happened with health for instance). Now sure, there are common things
like the math module or the camera module, but I still feel like we should
first of all vendor everything ourselves and once the game is "DONE" see what
parts of the game are "copy-pastable" to other games. That way we can create
a better `bevy-common-systems` from this crate; I think my idea was right to
split bcs from nova, but I did it too early and with too many dependencies that
are game specific; What we will do is what NOTES.md says: just copy paste bcs
in here, and you know just migrate it nicely using compiler assited refactoring.

## Plan context

`DECISION.md` (ACCEPTED, `20260806-182655`) rules that BCS is absorbed into
nova as **ten sequential migration steps**, one per subsystem, each a single
commit that leaves the workspace compiling. `NOTES.md` holds the verified move
map; `prototypes/00-conventions.md` holds the shared rules and three
corrections to NOTES.md; `prototypes/01..10` hold the per-step scope tables,
callsite inventories and compile hazards.

This plan is those ten steps as ten Steps with subtasks. Ten read-only planning
lanes (one per prototype) re-verified the prototypes against current `master`
and found roughly twenty additional callsites and five wrong claims; each is
folded into the Step it belongs to and marked **[lane]**.

Runs **directly on master**, no sprout worktree - owner's instruction. One
commit per Step.

## Inputs

| Input | Where | Note |
| --- | --- | --- |
| The ruling | `DECISION.md` | logic verbatim, layout free; one rand (0.10.2); single task, not an epic |
| Shared rules | `prototypes/00-conventions.md` | the per-step loop, lint facts, verification, dead-surface ruling |
| Move map | `NOTES.md` | input, not authority; three claims corrected in `00-conventions.md` |
| Per-step scope | `prototypes/01..10` | verified line counts, exports that must survive, manifest diffs, callsites |
| BCS source | `/home/alex/personal/bevy-common-systems` @ `6f09461` | the **working copy**, not the cargo checkout |
| Prior art | `crates/nova_gameplay/src/integrity/`, `crates/nova_autopilot/` | what "absorbed" looks like here |
| Probe baseline | `tasks/20260805-185103/` | sections 5, systems 3, stress 4, ui 5, all OK |

## Steps

Order is `DECISION.md`'s: `01 -> 02 -> 03 -> 04 -> 06 -> 08 -> 05 -> 07 -> 09
-> 10`. Two deviations from NOTES.md, both to avoid editing one import line
twice: `math` lands with the camera (03 before 04 and 06), and `TempEntity`
lands before `rigid_body_point_velocity` (08 before 05).

**The rule that governs every step: narrow globs, never delete them.** ~15
files brace engine names, `GameObjectives`, `TriangleMeshBuilder` and
`WASDCameraController` onto one `use bevy_common_systems::prelude` line. A step
removes only the names it has just given a nova home and leaves the BCS import
standing. A surviving `bevy_common_systems` hit in a file a step touched is
expected until step 10, not evidence the step was left incomplete.

- [x] 1. **Event engine -> `nova_events` + new `nova_events_macros`.**
      Prototype 01. `nova_events` is a leaf; nothing else can drop the dep
      first.
      - a. New crate `crates/nova_events_macros/{Cargo.toml,src/lib.rs}` from
        `bevy_common_systems_macros` (44 L, `proc-macro = true`,
        `quote`/`syn`(full)/`proc-macro2`), `publish = false`. Add
        `"crates/nova_events_macros"` to root `Cargo.toml` members
        (`:324-355`), alphabetically beside `crates/nova_events`. It is the
        derive's only user - not a workspace-wide dep.
      - b. Copy `src/modding/events.rs` (561 L) ->
        `crates/nova_events/src/engine.rs`. Rewrite the rustdoc at `:193` AND
        the **live test code** at `:417`
        (`#[derive(Clone, bevy_common_systems_macros::EventKind)]`) - **[lane]**
        the prototype calls both rustdoc; `:417` is a compile error if missed.
      - c. Drop `src/modding/registry.rs` (494 L, zero references - verified).
      - d. `crates/nova_events/Cargo.toml`: remove `bevy_common_systems`
        (`:12`), add `serde_json = "1"` (**required**: `events.rs:156` types
        `data: Option<serde_json::Value>`, `:162` calls `to_value`) and
        `nova_events_macros = { path = ... }`. Nothing selects
        `nova_events/debug` - **[lane]** verified only `nova_debug` selects
        `bevy_common_systems/debug` - so delete the feature at `:18` and its
        forwards rather than leave a no-op knob.
      - e. `crates/nova_events/src/lib.rs`: `pub mod engine;`, swap the glob at
        `:12` for `use crate::engine::*;`, append the engine names to the
        explicit prelude at `:17-26`, and re-export
        `pub use nova_events_macros::EventKind;` **paired with the trait
        `EventKind`** - the derive expands to `impl EventKind for #name` and
        needs the trait in scope at the derive site. **[lane]** the derive is
        not in `modding/events.rs::prelude`; BCS paired them at its crate root
        (`src/lib.rs:33`), so the pairing is added by hand.
      - f. Add a nova ownership docstring (pattern:
        `crates/nova_gameplay/src/integrity/mod.rs`) and satisfy
        `#![warn(missing_docs)]` on the copied `pub mod prelude` and
        `GameEventInfo`'s pub fields.
      - g. Repoint pure-engine callsites: `nova_gameplay`
        `integrity/neutralize.rs:18,158`, `integrity/glue.rs:610`;
        `nova_assets/src/scenario/shakedown/tests/walk.rs:5`; `nova_scenario`
        `filters.rs:2` (the `modding::prelude` sub-prelude - after dropping
        `registry.rs` it is exactly the engine names), `world.rs:4`
        (`EventWorld` - **[lane]** omitted by the prototype),
        `loader/lifecycle.rs:791`, `benches/scenario_dispatch.rs:26`.
      - h. **Narrow** the mixed imports, do not repoint them wholesale
        - **[lane]**: `objects/asteroid.rs:3` (`CommandsGameEventExt` +
        `TriangleMeshBuilder`), `:763` (not 762), `objects/area.rs:3,170`,
        `filters.rs:202`, `world.rs:411,485` (`GameObjectives`, **not** engine
        - leave alone), `loader/lifecycle.rs:716,795` (`:795` is
        `GameObjectives`), `world.rs:438,492,517,544,564`, `actions/*`,
        `loader/{clock,trackers}.rs`.
      - i. **[lane]** `nova_assets` **cannot** drop its dev-dep here: all nine
        `tests/*.rs` brace `GameObjectives` with the engine names. That
        deletion moves to Step 6 (prototype 08). Prototype 01's "Done when"
        for `nova_assets/Cargo.toml:72` is wrong.
      - j. `cargo fmt`, verify, commit.

- [x] 2. **Status bar + tween -> `nova_ui`.** Prototype 02. Independent of
      01; slots anywhere before 10.
      - a. Copy BCS `src/ui/status.rs` -> `crates/nova_ui/src/status_bar.rs`
        verbatim, keeping the exclusive-`&mut World` staging guard comment.
      - b. Copy BCS `src/tween/mod.rs` -> `crates/nova_ui/src/tween.rs`
        verbatim **including `mod tests` at `:258`**.
      - c. Rewrite the doctests (`status.rs:274`, `tween.rs:25`) to
        `use nova_ui::prelude::*;`. De-link `tween.rs:3,12`'s intra-doc links
        to `crate::meth::lerp::LerpSnap` and `crate::transform` into plain
        prose - `nova_ui` must not gain an edge to `nova_gameplay`.
      - d. Nova ownership docstrings on both module headers.
      - e. **[lane]** `#![warn(missing_docs)]` is a real sweep here, not one
        line: ~15 undocumented pub items in `status.rs`
        (`StatusBarRootConfig:29`, `StatusBarItemMarker:50`, `StatusValue:52`,
        all five `StatusBarItemConfig` fields `:62-66`,
        `StatusBarItem{Icon,Prefix,Suffix,ValueFnBoxed,ColorFnBoxed}:70-84`
        with pub tuple fields, `StatusBarItemValue:103`,
        `StatusBarStore`+`store:106-107`, `StatusBarPluginSystems:111`,
        `StatusBarPlugin:115`, `status_bar_item:86`,
        `status_{fps,version}_*_fn:293,305,319,325`) plus `tween.rs`'s
        `TweenValue`, `TweenOnComplete`, `TweenSystems`, `TweenFinished`,
        `Tween` fields and `TweenPlugin`.
      - f. `crates/nova_ui/src/lib.rs`: `pub mod status_bar; pub mod tween;`
        in alpha order, extend the crate docstring, and add **only the eight
        status names** to the explicit prelude at `:26`. **[lane]** no tween
        name enters the prelude: `grep -rn Tween crates/` finds exactly two
        hits, the registration at `nova_gameplay/src/hud/mod.rs:301` and a doc
        comment at `hud/nova_os/mod.rs:26` saying the slide is deliberately
        **not** driven by `Tween`. `TweenPlugin` is registered and unused;
        copy it verbatim per the dead-surface ruling and reach it by crate
        path.
      - g. Delete the eight status names from
        `crates/nova_gameplay/src/lib.rs:77-85` (`status_bar`,
        `status_bar_item`, four `status_*_fn`, `StatusBarItemConfig`,
        `StatusBarRootConfig` - **[lane]** eight, not five). **Keep the
        comment at `:69-76`** that records why the list is explicit.
      - h. `crates/nova_core/src/lib.rs`: add the `nova_ui` import (**[lane]**
        `nova_core` currently reaches `nova_ui` only from
        `loading_screen.rs:21`; `lib.rs` has none), leave the call bodies at
        `:283-296` unchanged.
      - i. Move registration: `StatusBarPlugin` at
        `nova_gameplay/src/plugin.rs:105` -> `nova_ui::status_bar::`;
        `TweenPlugin` at `nova_gameplay/src/hud/mod.rs:301` ->
        `nova_ui::tween::`. Each stays registered exactly once, from the same
        file as before.
      - j. Confirm `crates/nova_ui/Cargo.toml` is untouched. `cargo fmt`,
        verify, commit.

- [x] 3. **Camera rigs + `math` -> `nova_gameplay`.** Prototype 03. Lands
      `math` for Steps 4 and 5. Carries one of the three `rand` ports.
      - a. Copy `src/meth/{lerp,sphere}.rs` (143 L) ->
        `crates/nova_gameplay/src/math.rs`; drop the 70-line difficulty-ramp
        doc, keep the `powi(7)` NOTE, write a two-line nova docstring;
        `pub mod math;` in `lib.rs`. **Export `direction_to_spherical`**
        alongside `LerpSnap`, `spherical_to_cartesian` and `slerp` -
        **[lane]** `00-conventions.md` names only three symbols and NOTES.md
        calls this one unreferenced, but
        `transform/directional_sphere_orbit.rs:91,113` calls it; omitting it
        breaks Step 4.
      - b. Copy the six rig files into
        `crates/nova_gameplay/src/camera_controller/` as `chase.rs` (241 L),
        `shake.rs` (578), `skybox.rs` (138), `post.rs` (73), `wasd.rs` (219),
        `wasd_controller.rs` (from `helpers/wasd.rs`, 231).
      - c. Intra-file imports: `chase.rs` -> `crate::math::LerpSnap`;
        `shake.rs` -> `super::chase::ChaseCameraSystems` and
        `use rand::RngExt` (`rand` 0.10.2 renamed the method trait; two of the
        three edits are here and in `shake.rs:62,64`);
        `wasd_controller.rs` -> `super::wasd::{WASDCamera, WASDCameraInput}`.
      - d. Rewrite the **six** BCS doc mentions - **[lane]** the prototype says
        four: `chase.rs:28`, `shake.rs:42`, `skybox.rs:34`, `post.rs:16`,
        `camera/wasd.rs:19` are doctest `use` lines; `helpers/wasd.rs:4` and
        `meth/mod.rs:77` are prose. Nova ownership docstrings; `missing_docs`
        sweep.
      - e. Wire `camera_controller/mod.rs`: declare the six modules, fold their
        prelude blocks into `camera_controller::prelude`.
      - f. Repoint in-crate: `camera_controller/{framing:8,handback:7,mode:6,
        rig:6}.rs`, `authority.rs:20,86`, `hud/screen_indicator.rs:22,1365`,
        `hud/velocity.rs:16`, `input/player/{flight_rig,hints,intent,
        test_support}.rs`, `sections/{controller_section:5,
        thruster_section:10}.rs`, `sections/turret_section/{aim:6,setup:7}.rs`.
        **[lane]** also the test constructions at `authority.rs:87-88,103-105,
        129-130,181`, `mode.rs:250`, `handback.rs:145`.
      - g. Move registration `plugin.rs:81,82,84,86,87` onto
        `crate::camera_controller::` paths. Move the eight camera names from
        `lib.rs:77-85` into the `super::` block.
      - h. Repoint out-of-crate: `nova_debug/src/harness.rs:78` -
        **[lane] narrow** the `use bevy_common_systems::{...}` block at
        `nova_debug/src/lib.rs:17-23` to its `debug::{...}` half rather than
        deleting it, or Step 9's aliases are orphaned;
        `nova_scenario/src/actions/view.rs:10,146,531`;
        `nova_scenario/src/loader/lifecycle.rs:5` (**[lane]** missing from the
        prototype - supplies `PostProcessingCamera` at `:193`, and the same
        line is shared with Step 1);
        `nova_scenario/tests/skybox_swap_e2e.rs:7,29`;
        `nova_editor/src/ui/mod.rs:114` comment (**[lane]** missing - `:108`
        uses `PostProcessingCamera` via `nova_gameplay::prelude`).
      - i. `git mv camera_controller camera` **in the same commit**; fix
        `lib.rs:7,18,94`, `plugin.rs:113`, `mod.rs:44` prelude doc, and
        `mod.rs:109-112`'s "bcs's three" -> "nova's three". Keep
        `SpaceshipCameraControllerPlugin` and every `Spaceship*` name.
      - j. Confirm the `cd1bff21` camera-ordering fix in `authority.rs`
        survives byte-identically. `cargo fmt`, verify, **run** examples,
        commit.

- [x] 4. **Transform rigs -> `nova_gameplay`.** Prototype 04. Needs Step 3's
      `math`.
      - a. Copy the five rig files + `mod.rs` (837 L) ->
        `crates/nova_gameplay/src/transform/`; rewrite `mod.rs`'s doc as a nova
        ownership docstring.
      - b. Fix **three** `crate::meth::prelude::*` imports - **[lane]** the
        prototype names one: `random_sphere_orbit.rs:9`, `sphere_orbit.rs:10`,
        `directional_sphere_orbit.rs:12` -> `crate::math::{...}` including
        `direction_to_spherical`. Leave `random_sphere_orbit.rs:7`'s
        `rand::prelude::*` and confirm it builds against 0.10.2 (the third
        `rand` site).
      - c. `missing_docs` on the five `pub mod prelude` blocks and any
        undocumented pub item.
      - d. `lib.rs`: `pub mod transform;` + `transform::prelude::*` in the
        `super::` block; delete `PointRotation`, `PointRotationOutput`,
        `DirectionalSphereOrbitOutput` from `lib.rs:79-85`.
      - e. `plugin.rs:89,91,93,95,96` -> `crate::transform::prelude::`, keeping
        the `// for debug to have a random orbiting object` comment.
      - f. Repoint `turret_section/aim.rs:488`, and **[lane]** add
        `crate::transform::prelude::*` where the narrowed BCS glob no longer
        supplies transform names: `hud/velocity.rs:16`
        (`DirectionalSphereOrbit{,Input,Output}`),
        `turret_section/setup.rs:7` (`SmoothLookRotation`),
        `input/player/intent.rs:6` (`PointRotationOutput`), `aim.rs:6`.
      - g. `cargo fmt`, verify, **run** examples, commit. **[lane]** only
        `point_rotation.rs` carries tests (5), so pair the `transform::` filter
        with the turret filter.

- [ ] 5. **Mesh builder / explode / slice -> `nova_gameplay`.** Prototype 06.
      Needs Step 3's `math`. Carries the `noise` dep move and one `rand` port.
      - a. Copy `src/mesh/{builder,explode,slice,mod}.rs` (987 L) ->
        `crates/nova_gameplay/src/mesh/`, all 10 tests included (builder 3,
        explode 3, slice 4 - **[lane]** `slice.rs` is not import-free; it has
        `use bevy::prelude::*;` at `:9`).
      - b. `mesh/mod.rs`: `pub mod builder; pub mod explode; mod slice;` +
        prelude + nova ownership docstring; rewrite the BCS mention at
        `mod.rs:13` (**[lane]** the string count is 2, not 1 -
        `builder.rs:16` doctest and `mod.rs:13` rustdoc).
      - c. `builder.rs`: `use crate::math::slerp;` (was
        `crate::meth::prelude::*` at `:30`, used at `:326-328`); fix the
        doctest at `:16`; `missing_docs` sweep.
      - d. `explode.rs`: `use rand::RngExt;` (`:10`) and
        `fn random_unit_vector(rng: &mut impl RngExt)` (`:159`) - the generic
        bound, the third of the three `rand` edits. **Do not** rewire onto
        `bevy_rand`.
      - e. `crates/nova_gameplay/Cargo.toml`: add `noise = { version = "0.9" }`
        (a move from BCS, not a new package).
      - f. `lib.rs`: `pub mod mesh;` + `mesh::prelude::*` in the `super::`
        block. **[lane]** nothing to delete from `lib.rs:77-87` - no mesh name
        is in the by-name list today.
      - g. Repoint `plugin.rs:100` -> `mesh::prelude::ExplodeMeshPlugin`;
        `integrity/explode.rs:13-15` (names at `:14`, and this line also welds
        `TempEntity` from Step 6 and `CommandsGameEventExt` from Step 1 -
        expect a partially migrated import, not a clean BCS one);
        `nova_scenario/src/objects/asteroid.rs:3` (split
        `CommandsGameEventExt`). **[lane]** two more `TriangleMeshBuilder`
        users reach it through globs and are missing from the prototype's
        three-row table: `sections/thruster_section.rs:10` (uses at
        `:183,185,220`) and `hud/velocity.rs:16` (use at `:559`).
      - h. `cargo fmt`, verify, **run** examples, probe, commit.

- [ ] 6. **Lifetime, cooldown, objectives -> `nova_gameplay`.** Prototype 08.
      Runs before Step 7 so the co-imported `use` lines are edited once.
      - a. Copy BCS `helpers/temp.rs` + `helpers/despawn.rs` ->
        `crates/nova_gameplay/src/lifetime.rs`; `pub mod lifetime;` + prelude
        entries; move registration off `plugin.rs:98-99`.
      - b. Repoint `TempEntity` in **three** files - **[lane]** the prototype
        names two: `sections/torpedo_section/mod.rs:16`,
        `sections/turret_section/firing.rs:8`, and `integrity/explode.rs:12-14`.
      - c. Copy `time/cooldown.rs` -> `crates/nova_gameplay/src/cooldown.rs`;
        drop BCS `time/mod.rs`; keep `Cooldown` in the `lib.rs` re-export list.
      - d. Copy BCS `ui/objectives.rs` **whole, panel half included**, merged
        with nova's `objective_marker.rs` into
        `crates/nova_gameplay/src/objectives.rs`; delete
        `pub mod objective_marker;`. **[lane] raised this as a design fork; it
        is not one.** Nova has no objectives panel
        (`hud/mod.rs:293-298`), so `objectives_panel`,
        `ObjectivesPanelConfig`, `ObjectivesPanelMarker`, `ObjectiveMarker`,
        `ObjectiveId` and `rebuild_lines` arrive dead. That is exactly the
        dead-surface ruling in `00-conventions.md` ("copy them verbatim in this
        pass; a dead-code sweep is a separate follow-up"), and it is what keeps
        the copy verbatim, keeps BCS's only test
        (`the_panel_renders_one_line_per_objective`) copyable per the same
        conventions, and keeps `hud/mod.rs:293-298`'s comment true. Dropping
        the panel would empty `ObjectivesPlugin` to a bare `init_resource`,
        empty `ObjectivesPluginSystems::Sync` and delete the test - a redesign.
        Do not drop it.
      - e. Keep `ObjectiveMarkerTarget` under its current name (the rename is
        deferred in `DECISION.md`); reword `objective_marker.rs:19-21` so it
        stops naming a crate the workspace will not have, while keeping the
        reason the name is not `ObjectiveMarker` - which stays true, because
        6d copies `ObjectiveMarker` in.
      - f. Repoint objectives callsites: `hud/nova_os/{components,content,
        input,lists,mod:62,tests/mod}.rs`, `hud/objective_stack.rs:36,556,962`;
        `nova_scenario` globs (`lib.rs:41`,
        `actions/{mod,mission,flow,spawn,ship,view}.rs`) and explicit sites
        (`world.rs:411,485`, `loader/clock.rs` x5, `loader/trackers.rs` x3,
        `objects/{area:170,asteroid:763,salvage:343}.rs`,
        `loader/lifecycle.rs:716,795`); `nova_assets/tests/*.rs` (nine braced
        lists) + `src/scenario/shakedown/tests/walk.rs:28`.
      - g. **Now** delete the `nova_assets` dev-dep at `Cargo.toml:72` and its
        comment at `:70-71` - deferred from Step 1 because those nine tests
        brace `GameObjectives` with the engine names.
      - h. Docs: `lib.rs:1-10` module list, `nova_scenario/src/actions/
        mission.rs:13-14`, `nova_scenario/src/world.rs:58`,
        `nova_probe/src/invariants.rs:38`. `cargo fmt`, verify, **run**
        examples, commit.

- [ ] 7. **PD controller + point velocity -> `nova_gameplay`.** Prototype 05.
      After Step 6.
      - a. Copy `physics/pd_controller.rs` (575 L) ->
        `crates/nova_gameplay/src/physics/pd_controller.rs` verbatim, 13 tests
        included.
      - b. Copy **only** `rigid_body_point_velocity` + its doctest (`:28`) ->
        `physics/rigid_body.rs`; drop `destructible_body`, the
        `use crate::health::prelude::*` at `:6` and the `crate::integrity::*`
        intra-doc links (nova already owns `integrity`); doctest ->
        `use nova_gameplay::prelude::*;`.
      - c. New `physics/mod.rs`: nova ownership docstring, two `pub mod`, and a
        prelude re-exporting **seven** names - **[lane]** the prototype lists
        six and omits `PDControllerOutput`, which
        `sections/controller_section.rs:342,347` uses through the BCS glob and
        which is **not** in `lib.rs`'s by-name list today, so a
        "does the prelude still export the same names" check will not catch it
        going missing. No BCS radial-gravity header.
      - d. `lib.rs`: `pub mod physics;` + `physics::prelude::*` in the
        `super::` block; delete the five PD names from `lib.rs:77-84`, keeping
        the no-globs comment.
      - e. Repoint registration in **three** places - **[lane]** the prototype
        names one: `plugin.rs:102`, `input/ai/passive.rs:709`,
        `flight/tests/control.rs:177`.
      - f. Fix the glob users: `sections/controller_section.rs:5`,
        `input/player/flight_rig.rs:6`, and **[lane]**
        `sections/turret_section/aim.rs:6` (uses `rigid_body_point_velocity`
        at `:131`) and `input/player/hints.rs:5` (uses `PDController` at
        `:220`); `torpedo_section/bay.rs:215` inherits via `use super::*`.
        These are glob-vs-glob ambiguity errors, not silent failures.
      - g. Rewrite the co-imports at `torpedo_section/mod.rs:16` and
        `turret_section/firing.rs:8` - one edit each, because Step 6 landed
        `TempEntity` first and left the BCS line standing.
      - h. Reword `sections/base_section.rs:324` and `gravity.rs:32`.
        **[lane]** `base_section.rs:324` is claimed by prototype 10e too -
        it is handled here, so Step 10 must not redo it. `missing_docs` +
        `cargo fmt`, verify, **run** examples, probe, commit.

- [ ] 8. **SFX playback + sound bank -> `nova_gameplay`.** Prototype 07.
      - a. Copy BCS `src/audio/registry.rs` ->
        `crates/nova_gameplay/src/audio/registry.rs` with its 4 tests; repoint
        the doctests at `:18,130`; drop its `pub mod prelude`.
      - b. Copy BCS `src/audio/mod.rs` ->
        `crates/nova_gameplay/src/audio/sfx.rs`; repoint the doctest at `:16`;
        delete `pub mod registry;` and `pub mod prelude` (nova's audio module
        has no sub-prelude); keep `use bevy::{audio::Volume, prelude::*};`.
      - c. Nova ownership docstrings on both.
      - d. `crates/nova_gameplay/src/audio/mod.rs`: **[lane]** follow the local
        convention at `:44-65` - private `mod registry; mod sfx;` plus an
        explicit `pub use self::{registry::SoundBank, sfx::{PlaySfx,
        SfxCommandsExt, SfxMasterVolume, SfxPlugin}};`, not `pub mod`. Rewrite
        the header docstring at `:3-7` so it stops claiming BCS owns playback.
      - e. `lib.rs`: delete `PlaySfx`, `SfxCommandsExt`, `SfxPlugin`,
        `SoundBank` from the BCS list (`:82-84`); add them plus
        `SfxMasterVolume` to the `audio::{...}` entry (`:89-93`).
      - f. **[lane] registration does not move and this step does not touch
        `plugin.rs`.** `SfxPlugin` is already added inside
        `NovaAudioPlugin::build` at `audio/mod.rs:208-211` behind an
        `if !app.is_plugin_added::<SfxPlugin>()` guard. Keep the guard
        verbatim. `DECISION.md`'s "registration moves wholesale out of
        `plugin.rs:81-106`" does not apply here.
      - g. Repoint the five direct callsites, **narrowing** them:
        `hud/comms_panel.rs:21`, `hud/nova_os/{input.rs:9,shell.rs:2,
        sound.rs:2}`, `hud/nova_os/tests/mod.rs:29`. Three keep a shrunken BCS
        import for `GameObjectives`/`Objective` - fine, Step 6 already landed
        those, so shrink them the rest of the way here. `nova_menu`,
        `nova_scenario` and `nova_assets` reach the names through globs and
        need no edit; `nova_scenario/src/objects/salvage.rs:343` is a different
        name set.
      - h. `missing_docs` + `cargo fmt`, verify, **run** examples, commit.

- [ ] 9. **Inspector + wireframe -> `nova_debug`.** Prototype 09. Carries the
      `bevy-inspector-egui` dep move.
      - a. `crates/nova_debug/Cargo.toml`: add
        `bevy-inspector-egui = { version = "0.37" }`, non-optional
        (**[lane]** `inspector.rs:262`'s test module imports
        `bevy_inspector_egui::bevy_egui::EnableMultipassForPrimaryContext`, so
        test builds need it). Leave the BCS line at `:19` for Step 10.
      - b. Copy `src/debug/inspector.rs` (313 L, 4 tests) and `wireframe.rs`
        (66 L) verbatim. Neither carries an inner
        `#[cfg(feature = "debug")]`; the gate was BCS `src/lib.rs:11`.
      - c. Rewrite `wireframe.rs:9`'s doctest; nova ownership docstrings;
        `missing_docs` sweep.
      - d. `lib.rs`: `pub mod inspector; pub mod wireframe;` beside
        `gravity/harness/screenshot/sections`; alias from crate paths
        (`use crate::{inspector::DebugEnabled as InspectorEnabled,
        wireframe::DebugEnabled as WireframeEnabled};`) in place of the
        `debug::{...}` half of the `use bevy_common_systems::{...}` block at
        `:17-23`, which Step 3 has already narrowed. Do not touch prelude
        exports. **[lane]** three `DEBUG_TOGGLE_KEYCODE` consts coexist after
        the copy (`inspector.rs:12`, `wireframe.rs:23`, `lib.rs:73`) -
        module-scoped, no error, but do not re-export the copied ones.
      - e. **[lane] preserve the registration order.** Both copied plugins
        `insert_resource(DebugEnabled(true))` in `build`; `lib.rs:120-121`
        overrides them with `DEBUG_LAYER_STARTS_ON` (false) **only because it
        runs after `:99,100`**. Reordering silently inverts F11.
      - f. Reword the crate docstring at `lib.rs:1-9`; repoint
        `harness.rs:426,431` to `crate::{inspector,wireframe}::DebugEnabled`
        and drop the BCS name from `:411-413`. Leave `harness.rs:78` - Step 3
        owns it.
      - g. `cargo fmt`, verify under `--features debug`, commit.

- [ ] 10. **Delete the dependency.** Prototype 10. Nothing is copied. Do not
      start until Steps 1-9 have landed.
      - a. **10a `nova_probe`:** `recorder.rs:49,478` ->
        `nova_events::engine::{GameEvent, GameEventInfo}`; reword the doc at
        `:26`. Add `nova_events` to `crates/nova_probe/Cargo.toml` as a
        **direct** dep (owner's ruling; the second of the two intended new
        edges) and rewrite the routing comments at `:37-49`. The
        `invariants.rs:44` / `capture.rs:20-26` stragglers were fixed in
        `261c7e71` - confirm, **do not re-fix**.
      - b. **10b `nova_gameplay/src/lib.rs`:** delete `:32`
        (`pub use bevy_common_systems;`) and the re-export block at
        **`:77-86`** (**[lane]** not `:69-83`); every remaining name moves into
        the `super::` block. Rewrite the crate docstring at `:10` and the
        comment at `:69-76` so it carries the explicit-list lesson forward -
        do **not** opportunistically switch anything to a glob.
      - c. **10c `plugin.rs`:** `:20` -> `use crate::prelude::*;`; reword `:6`
        and `:49`; assert no BCS path survives `:81-105`.
      - d. **10d `nova_core/src/lib.rs:231,233`:** delete the
        `bevy_common_systems=` log-filter terms only. Check the crate list
        against the workspace members before changing more than that.
      - e. **10e prose sweep:** the prototype's twelve-row table, minus
        `base_section.rs:324` (Step 7), plus **[lane]** five it misses:
        `nova_gameplay/src/camera_controller/mod.rs:9` (post-`git mv`:
        `camera/mod.rs`), `nova_gameplay/src/audio/mod.rs:3`,
        `nova_scenario/src/world.rs:58`, `nova_debug/src/harness.rs:412`,
        `nova_assets/Cargo.toml:70-71`. Reword; keep
        `nova_assets/src/persist.rs:16`'s historical citation.
      - f. **10f manifests:** delete the dep from all five (`nova_gameplay:22`,
        `nova_scenario:15`, `nova_events:12`, `nova_assets:72`,
        `nova_debug:19`); reduce each `debug` feature to
        `["bevy/track_location"]`; delete any feature left with no caller. Do
        **not** rewire to `nova_debug/debug`. Keep `dirs` and `web-sys` - both
        are already direct nova deps for other reasons.
      - g. **10g docs outside the code tree** - **[lane]** the prototype's
        `--include='*.rs' --include='*.toml'` grep cannot see these, and the
        hyphen spelling `bevy-common-systems` escapes the underscore grep
        entirely: `AGENTS.md:35-37`, `web/src/wiki/dev/development.md:223`,
        and the other five wiki pages plus `assets/base/sounds/README.md` that
        the recount found. `CHANGELOG.md` and `web/src/news/0.{3,4,5,6}.0.md`
        are historical and stay.
      - h. **10h licenses and lock:** re-run `cargo-about` and confirm the
        third-party manifest is unchanged (BCS was `publish = false`;
        `nova_events_macros` must be too). Diff `Cargo.lock` and confirm the
        only removals are `bevy_common_systems` and
        `bevy_common_systems_macros`.
      - i. Final verification: both greps empty, workspace + debug check +
        clippy + fmt clean, crate graph gained exactly two edges, **every**
        example RUNS under Xvfb `:99`, probe verdicts match the
        `tasks/20260805-185103/` baseline. Commit.

## Definition of Done

Per-step proofs are grouped by the Step that earns them. Every step also
re-runs `nix develop --command cargo check --workspace --all-targets` and
`nix develop --command cargo fmt --check` before its commit; those two are
stated once here rather than eleven times.

- Every step leaves the workspace compiling and formatted
  (cmd: `nix develop --command cargo check --workspace --all-targets && nix develop --command cargo fmt --check`)

Step 1:
- `crates/nova_events` names BCS nowhere and `nova_events_macros` is a
  workspace member
  (cmd: `! grep -rn bevy_common_systems crates/nova_events/ && grep -qn '"crates/nova_events_macros"' Cargo.toml`)
- The copied engine tests pass in their new home, including
  `attribute_less_derive_defaults_to_no_payload` and the `:417` derive
  (test: `engine::`)
- Scenario dispatch semantics are unchanged (test: `filters::`)
- The bench still compiles
  (cmd: `nix develop --command cargo check -p nova_scenario --benches`)

Step 2:
- The tween tests pass unmodified in `nova_ui` (test: `tween::tests`)
- The status names no longer resolve through `nova_gameplay`
  (cmd: `! grep -n 'status_bar\|StatusBar' crates/nova_gameplay/src/lib.rs`)
- `nova_ui` gained no dependency
  (cmd: `git diff --exit-code crates/nova_ui/Cargo.toml`)
- `missing_docs` is satisfied under the CI lint invocation
  (cmd: `nix develop --command cargo clippy -p nova_ui --all-targets --features debug`)
- The doctests resolve against the new crate path
  (cmd: `nix develop --command cargo test -p nova_ui --doc`)

Step 3:
- `crates/nova_gameplay/src/math.rs` exports all four symbols Steps 4 and 5
  need
  (cmd: `grep -qn 'LerpSnap' crates/nova_gameplay/src/math.rs && grep -qn 'fn spherical_to_cartesian' crates/nova_gameplay/src/math.rs && grep -qn 'fn direction_to_spherical' crates/nova_gameplay/src/math.rs && grep -qn 'fn slerp' crates/nova_gameplay/src/math.rs`)
- No camera or math name resolves through BCS
  (cmd: `! grep -rnEi 'bevy_common_systems.*(chase|shake|skybox|post_process|wasd|meth|LerpSnap|camera_controller)' crates/ examples/ --include='*.rs'`)
- `camera_controller/` is gone and `camera/` holds the six rigs plus nova's
  five modules
  (cmd: `test ! -d crates/nova_gameplay/src/camera_controller && test -f crates/nova_gameplay/src/camera/chase.rs`)
- The camera rigs and the `cd1bff21` authority ordering pass
  (test: `camera::`)
- The skybox bridge still passes
  (test: `nix develop --command cargo test -p nova_scenario --test skybox_swap_e2e`)
- A camera-heavy example boots with no duplicate-plugin panic
  (cmd: `nix develop --command bash -c 'xvfb-run -a --server-num=99 cargo run --example screenshot_scene'`)

Step 4:
- Point-rotation's five tests pass in their new home (test: `transform::`)
- The turret rig still passes with the local `SmoothLookRotationPlugin`
  (test: `sections::turret_section::aim`)
- No transform name reaches BCS
  (cmd: `! grep -rnE 'bevy_common_systems.*(PointRotation|SphereOrbit|SmoothLookRotation)' crates/ examples/`)
- No new dep or lock change
  (cmd: `git diff --exit-code crates/nova_gameplay/Cargo.toml Cargo.lock`)
- The turret example slews under Xvfb
  (cmd: `nix develop --command bash -c 'xvfb-run -a --server-num=99 cargo run --example turret_section'`)

Step 5:
- The ten copied mesh tests pass (test: `mesh::`)
- No mesh name reaches BCS
  (cmd: `! grep -rnE 'bevy_common_systems.*(Explode|TriangleMeshBuilder)' crates/ examples/`)
- Exactly one `noise` version resolves
  (cmd: `nix develop --command cargo tree -p nova_gameplay -i noise`)
- The asteroid consumers still pass
  (test: `nix develop --command cargo test -p nova_scenario --lib objects::asteroid`)
- Debris geometry is unchanged against the `20260805-185103` probe baseline
  (manual: user judgement)

Step 6:
- The objectives, lifetime and cooldown tests pass, BCS's panel test included
  (test: `objectives::`)
- The HUD readers still pass (test: `hud::nova_os`)
- `objective_marker.rs` is gone and nothing still names it
  (cmd: `test ! -e crates/nova_gameplay/src/objective_marker.rs && ! grep -rn objective_marker crates/`)
- No lifetime/cooldown/objectives name reaches BCS
  (cmd: `! grep -rnE 'bevy_common_systems.*(TempEntity|DespawnEntity|Cooldown|GameObjectives|Objective)' crates/`)
- `nova_assets` no longer dev-depends on BCS
  (cmd: `! grep -n bevy_common_systems crates/nova_assets/Cargo.toml`)
- The scenario layer still passes
  (test: `nix develop --command cargo test -p nova_scenario --lib actions::`)

Step 7:
- The 13 PD tests pass in their new home (test: `physics::`)
- `PDControllerOutput` is exported from the new prelude
  (cmd: `grep -qn PDControllerOutput crates/nova_gameplay/src/physics/mod.rs`)
- The `rigid_body_point_velocity` doctest passes on the nova path
  (cmd: `nix develop --command cargo test -p nova_gameplay --doc rigid_body`)
- `destructible_body` and the `crate::health` import did not come across
  (cmd: `! grep -rn 'fn destructible_body\|use crate::health' crates/nova_gameplay/src/physics/`)
- `PDControllerPlugin` is registered from `plugin.rs` on the local path, and no
  BCS path survives beside it (cmd: `grep -qn 'PDControllerPlugin' crates/nova_gameplay/src/plugin.rs`)
- Nothing outside `plugin.rs`, `input/ai/passive.rs` and `flight/tests/control.rs`
  adds it - a fourth registration panics the run (cmd: `nix develop --command bash -c 'xvfb-run -a --server-num=99 cargo run --example controller_section'`)
- The section and flight consumers pass (test: `sections::`, `flight::`)
- Attitude control shows no drift against the `20260805-185103` probe baseline
  (manual: user judgement)

Step 8:
- The four `registry.rs` tests pass in their new home
  (test: `audio::registry::tests`)
- The copied modules are wired privately per the local convention, and the
  existing `is_plugin_added` guard still fronts the now-local `SfxPlugin`
  (cmd: `grep -qnE '^mod sfx;' crates/nova_gameplay/src/audio/mod.rs`)
- Registration did not move and no second `add_plugins` appeared - a duplicate
  panics the run (cmd: `nix develop --command bash -c 'xvfb-run -a --server-num=99 cargo run --example turret_section'`)
- No SFX name reaches BCS
  (cmd: `! grep -rnE 'bevy_common_systems.*(SoundBank|PlaySfx|Sfx)' crates/`)
- The downstream `SoundBank<UiSfx>` users still resolve
  (cmd: `nix develop --command cargo check -p nova_assets -p nova_menu --all-targets`)
- The copied doctests run
  (cmd: `nix develop --command cargo test -p nova_gameplay --doc audio`)
- Sound plays once, not twice, in a running example
  (manual: user judgement)

Step 9:
- The four inspector tests pass in their new home (test: `inspector::`)
- No BCS debug path remains
  (cmd: `! grep -rn 'bevy_common_systems::debug' crates/`)
- No harness code came across
  (cmd: `! grep -rnE 'AutopilotPlugin|harness' crates/nova_debug/src/inspector.rs crates/nova_debug/src/wireframe.rs`)
- `bevy-inspector-egui = "0.37"` is a direct non-optional `nova_debug` dep
  (cmd: `grep -qn 'bevy-inspector-egui' crates/nova_debug/Cargo.toml`)
- The debug-featured workspace builds and CI's only debug lint pass is clean
  (cmd: `nix develop --command cargo clippy --workspace --all-targets --features debug`)
- F11 still raises inspector panel, avian gizmos and wireframe as one layer,
  starting OFF (manual: user judgement)

Step 10:
- No `bevy_common_systems` in code or manifests
  (cmd: `! grep -rn bevy_common_systems crates/ examples/ Cargo.toml Cargo.lock --include='*.rs' --include='*.toml' --include='Cargo.lock'`)
- No BCS mention in either spelling anywhere outside task records and the
  historical CHANGELOG/news posts
  (cmd: `! grep -rniE 'bevy.common.systems' . --exclude-dir=tasks --exclude-dir=.git --exclude-dir=target --exclude-dir=news --exclude=CHANGELOG.md`)
- `Cargo.lock` lost exactly two packages, `bevy_common_systems` and
  `bevy_common_systems_macros`
  (cmd: `git diff --stat Cargo.lock`, reviewed against the two-package
  expectation - and `! grep -n bevy_common_systems Cargo.lock`)
- `nova_probe` depends on `nova_events` directly
  (cmd: `grep -qnE '^nova_events = \{ path' crates/nova_probe/Cargo.toml`)
- The workspace is clean at both feature settings and under the CI lint
  (cmd: `nix develop --command cargo check --workspace --all-targets --features debug && nix develop --command cargo clippy --workspace --all-targets --features debug`)
- The crate graph gained exactly two edges, `nova_events ->
  nova_events_macros` and `nova_probe -> nova_events`, and nothing else
  (manual: user judgement, on `cargo tree` before/after)
- The `cargo-about` third-party manifest is unchanged
  (manual: user judgement)
- Every example RUNS under Xvfb `:99` with no double-registration panic
  (manual: user judgement)
- Probe verdicts across `sections/`, `systems/`, `stress/` and `ui/` match the
  `tasks/20260805-185103/` baseline - sections 5, systems 3, stress 4, ui 5,
  all OK (manual: user judgement)

## Notes

**Runs directly on master, no sprout worktree** (owner's instruction). One
commit per Step, each leaving the workspace compiling.

**Test filters, not the suite.** `bcs-no-full-test-suite`: the full suite OOMs
the box. Every `test:` proof above runs as
`nix develop --command cargo test -p <crate> --lib <filter>`. `cargo` only ever
runs via `nix develop --command`.

**Examples must be RUN, not checked.** `cargo check` cannot see a plugin
registered twice or a component inserted twice. Steps 2-9 run the examples
their subsystem touches; Step 10 runs the whole catalog.

**Two proofs the lanes proposed were already green on `master` and are not
used.** `grep -rn LerpSnap crates/ | grep -v src/math.rs` returns nothing today
(there is no `LerpSnap` in-tree yet), so it proves nothing; Step 3's DoD uses a
positive delivery guard on `math.rs` instead. Likewise a bare
`grep bevy_common_systems Cargo.lock` is red today for the right reason and is
kept, but the "lost exactly two packages" half needs the diff, not the grep.

**Absence-grep scoping.** `tasks/` and `.git` are excluded because task prose
legitimately names BCS. `CHANGELOG.md` and `web/src/news/*` are excluded
because their mentions are historical release notes that must not be rewritten.
The recount found 192 total mentions today across 13 markdown files plus the
code; the code-and-manifest grep and the docs grep are separate DoD lines
because prototype 10's single `--include='*.rs' --include='*.toml'` command
cannot see `AGENTS.md` or the wiki, and its underscore spelling cannot see the
hyphenated git URLs.

**Assumption: the `nova_gameplay` re-export block is emptied incrementally.**
Steps 2, 3, 4, 7 and 8 each delete their own names from `lib.rs:77-86` as they
land. Step 10 deletes whatever is left plus the `pub use bevy_common_systems;`
line. Every lane independently expected some *other* step to do this; stated
here so no name is deleted twice or left behind. The same applies to
`plugin.rs:81-106` - each step moves its own registration; Step 10 only
asserts the block is BCS-free.

**Line numbers drift.** Step 2 removes eight names from `lib.rs:77-85` and one
`add_plugins` line from `plugin.rs`; every later step's recorded line numbers
in those two files shift by roughly nine. The names are the anchor, not the
line numbers.

**Risk: the plan is a 6.5k-LOC mechanical lift whose only real gate is the
compiler.** The failure mode is not a wrong line - the compiler catches those -
it is a *silent* behavior change the compiler cannot see: a plugin registered
twice, a `DebugEnabled` resource whose insertion order inverted (Step 9e), the
`cd1bff21` camera ordering perturbed (Step 3j), or a system set reordered
during a file move. That is why every step runs its examples and why Steps 5, 7
and 10 probe against the `20260805-185103` baseline. If a step turns out to
need design thought rather than a compiler fix, `00-conventions.md`'s rule
applies: the scope has widened - stop and record it rather than deciding it
inside the lift.

**One design fork was raised and resolved inside the existing rulings, not
deferred:** Step 6d copies BCS's objectives panel half in dead, because
`00-conventions.md` already rules that dead surface is copied verbatim in this
pass. If the owner would rather drop the panel, that is a scope change to
raise before Step 6, not during it.

**Deferred by `DECISION.md`, do not do here:** the `camera_controller` wrapper
collapse, the `ObjectiveMarkerTarget` -> `ObjectiveMarker` rename, the
`bevy_rand` rewire, and the dead-code sweep over `CameraShakeOutput`,
`WASDCamera`, `EventHandlerIndex`, `BlastDamageConfig` and the unused `*Systems`
sets.

## Progress

### Step 1 - DONE (`nova_events` + `nova_events_macros`)

**What.** `modding/events.rs` -> `crates/nova_events/src/engine.rs`;
`bevy_common_systems_macros` -> `crates/nova_events_macros` (workspace member,
`publish = false`); `registry.rs` dropped; `nova_events` swapped
`bevy_common_systems` for `serde_json` + `nova_events_macros` and lost its
no-op `debug` feature. Every pure-engine callsite repointed, every mixed one
narrowed.

**One hazard the plan did not name, and how it was resolved.** Adding the
engine names to `nova_events::prelude` made them collide with the still-standing
`use bevy_common_systems::prelude::*;` in ~11 `nova_scenario` files that ALSO
glob `nova_events::prelude::*` - `E0659 ambiguous name`, not a silent failure.
The plan's "narrow globs, never delete them" rule still governs; the fix was to
narrow each one to the non-engine names it actually supplied. Probing that (drop
the glob, compile, read the unresolved list) showed the BCS glob in
`nova_scenario/src/{lib,events,actions/*,loader/*}.rs` supplied **only** engine
names, so it narrows to nothing there; `actions/flow.rs` and
`benches/scenario_dispatch.rs` needed a test-scoped `GameObjectives` kept.
Expect the same probe to be the cheapest way to narrow a glob in Steps 3-9.

**Two plan corrections.**

- `engine.rs:251` called `init_resource::<super::registry::EventHandlerRegistry>`
  inside `GameEventsPlugin::build`. Dropping `registry.rs` (1c) means dropping
  that line too - the prototype does not mention it. Nothing reads the resource.
- The Step 1 DoD grep `! grep -rn bevy_common_systems crates/nova_events/` also
  catches the **nova ownership docstring** the conventions ask for. Both
  docstrings say why nova owns the engine without naming the crate it came from,
  which is what `crates/nova_gameplay/src/integrity/mod.rs` does anyway.
- `walk.rs:5`'s `CommandsGameEventExt` became a redundant import (the name now
  arrives through the `use super::*` chain) and was dropped from the braced list.

**Evidence.** `cargo check --workspace --all-targets` clean; `cargo fmt --check`
clean; `clippy -p nova_events -p nova_events_macros --all-targets` clean;
`nova_events --lib` 4/4 (the `:417` derive included); `nova_events --doc` 1/1;
`nova_scenario --lib filters::` 3/3; `check -p nova_scenario --benches` clean;
`Cargo.lock` gained exactly one package, `nova_events_macros`.
`examples/stress/many_sections.rs`'s unused-import warning is pre-existing
(confirmed against a stashed tree), not from this step.

### Step 2 - DONE (status bar + tween -> `nova_ui`)

**What.** BCS `ui/status.rs` -> `crates/nova_ui/src/status_bar.rs` and
`tween/mod.rs` -> `crates/nova_ui/src/tween.rs`, both verbatim (the exclusive-
`&mut World` guard comment, the `try_*` despawn-race NOTE and all 11 tween
tests included). Both wired as `pub mod`; the eight status names nova calls
moved from the `nova_gameplay` BCS re-export list into `nova_ui::prelude`; no
tween name entered any prelude. `StatusBarPlugin` re-registered from
`plugin.rs` and `TweenPlugin` from `hud/mod.rs` on `nova_ui::` paths - same
file, same count. `nova_core` now names `nova_ui::status_bar` directly instead
of inheriting the names through `nova_gameplay::prelude`.

**Three plan corrections.**

- **2c is wrong about the doctests.** It says rewrite both to
  `use nova_ui::prelude::*;`, but 2f caps the prelude at the eight names nova
  calls - which excludes `status_bar_with_fps` (the very item the `status.rs`
  doctest demonstrates) and every tween name. Both doctests use the module path
  (`nova_ui::status_bar::*`, `nova_ui::tween::*`) instead. That keeps 2f's
  narrow prelude and still proves the copied code compiles on a nova path.
- **The Step 2 DoD's clippy line does not run:** `nova_ui` has no `debug`
  feature, so `clippy -p nova_ui --all-targets --features debug` is a hard
  cargo error, not a lint failure. Ran `clippy -p nova_ui --all-targets`
  instead; `--features debug` only ever meant the workspace-wide CI pass.
- **Every "run the example" DoD command in this plan is missing what makes the
  run terminate**, and Steps 3-10 will hit it too. The recorded form is
  `xvfb-run -a --server-num=99 cargo run --example <x>`; the autopilot harness
  is inert unless `NOVA_AUTOPILOT` is set AND the crate is built with
  `--features debug` (`nova_debug/src/harness.rs:20,43,153`). Without both, the
  example boots correctly and then idles forever - it does not fail, which is
  the trap. Cost 30 wasted minutes here. The working form is
  `NOVA_AUTOPILOT=1 xvfb-run -a --server-num=99 cargo run --example <x> --features debug`.
  Second trap in the same command: **the exit code is not the verdict.** This
  `xvfb-run` wrapper fails its own teardown `kill` and returns 1 even for
  `xvfb-run -a --server-num=99 true` (verified). Read the app log for
  `autopilot: cycle complete, no panic` and the beat `PASS` lines instead; a
  DoD line that shells the command and trusts `$?` reports every run red.
- **Moving code out of BCS silently drops its logs, and no step in the plan
  owns that.** `nova_core`'s `log_filter_str` (`:229-238`) names crates
  explicitly, and `bevy_common_systems=debug/trace` is on that list while
  `nova_ui` is not - so `StatusBarPlugin: build` and the item `trace!`s
  vanished the moment the module changed crates. Caught only by grepping the
  harness log for `StatusBar` and getting zero hits. Added `nova_ui` to both
  filter strings here. **Every later step must do the same check**, and Step
  10d - which currently says only "delete the `bevy_common_systems=` terms" -
  should be read as: the deletion is safe only once each moved module's new
  crate is on the list. `nova_gameplay`, `nova_debug` and `nova_events` are
  already on it, so Steps 3-9 are covered; Step 10a's `nova_probe` is not.
  (Six workspace crates are absent from the filter for unrelated historical
  reasons - `nova_editor`, `nova_menu`, `nova_modding`, `nova_mod_format`,
  `nova_os`, `nova_probe`. Pre-existing, deliberately not widened here.)
- **One prose line outside the plan's callsite list named the crate**:
  `hud/nova_os/mod.rs:26`'s animation-clock rationale cited "the bcs `Tween`"
  and `bcs tween::advance_tweens`. It is this step's subject, the fn name was
  wrong anyway (`advance_tween`), and leaving it would only defer the same edit
  to Step 10e - reworded here.

**The `missing_docs` sweep was the bulk of the diff**, as 2e predicted:
~19 undocumented pub items in `status_bar.rs` (both `prelude` blocks, the five
`StatusBarItemConfig` fields, the five type-erased item components, the store
field, the `Sync` set variant and the four `status_*_fn` helpers). `tween.rs`
needed only its `prelude` block - BCS documented that file properly.

**Evidence.** `cargo check --workspace --all-targets` clean;
`cargo fmt --check` clean; `clippy -p nova_ui --all-targets` and
`cargo doc -p nova_ui --no-deps` add zero warnings (the 4 clippy +
2 rustdoc hits are pre-existing, in `hud.rs`/`widget`/`font`, none in the two
copied files - so the de-linked `crate::meth::lerp::LerpSnap` /
`crate::transform` rustdoc links left nothing broken behind);
`nova_ui --lib tween::` 11/11 unmodified; `nova_ui --doc` 5/5 including both
rewritten doctests; `! grep 'status_bar\|StatusBar' nova_gameplay/src/lib.rs`
holds; `git diff --exit-code crates/nova_ui/Cargo.toml` clean - no new dep, no
new graph edge.

`examples/ui/hud_range.rs` RAN under Xvfb `:99` with the autopilot harness and
reported `PASS - indicators track their anchors and hide when they die` /
`autopilot: cycle complete, no panic (t=5.8s)`. The registration move is proved
positively, not just by absence of a panic: the log carries exactly one
`nova_ui::status_bar: StatusBarPlugin: build` and one
`nova_ui::tween: TweenPlugin: build` - right crate path, once each.

**Not run:** the workspace-wide `clippy --workspace --all-targets --features
debug` (CI's lint gate) and any test outside `nova_ui`, per the standing
skip-local-suite instruction; `check --workspace --all-targets` covers
compilation and CI owns the rest.

### Step 3 - DONE (camera rigs + `math` -> `nova_gameplay`)

**What.** BCS `meth/{lerp,sphere}.rs` -> `crates/nova_gameplay/src/math.rs` (the
`powi(7)` NOTE and both sphere tests kept, the 70-line difficulty-ramp doc
dropped, all four symbols exported including `direction_to_spherical`). The six
rigs -> `camera_controller/` as `chase/shake/skybox/post/wasd/wasd_controller.rs`,
then `git mv camera_controller camera` in the same commit. The five camera
registrations moved off `plugin.rs:81-87` onto `crate::camera::` paths; the
eight camera names moved out of `lib.rs`'s BCS re-export list and now arrive
through `camera::prelude::*`. `camera/mod.rs` folds the six rig preludes into
its own.

**Four plan corrections.**

- **3f is short two files and the two it names hardest are the ones that
  matter.** `hud/screen_indicator.rs:22,1365` are *explicit* BCS imports
  (`ChaseCameraSystems`, and `ChaseCamera`/`ChaseCameraInput`/`ChaseCameraPlugin`
  in a test) - and so is `camera/authority.rs:86`'s test module, which the plan
  does not list at all. Left alone they still COMPILE: they resolve to BCS's
  types, so the authority test would have gone on ordering BCS's `SystemSet`s
  while the app ordered nova's, and it would still have passed. `cargo check` is
  blind to this whole class. The Step 3 DoD grep
  (`bevy_common_systems.*(chase|shake|...)`) is what caught all three - it is the
  only proof in this step that finds a silent divergence, not a compile error.
  Every later step should run its own name-scoped version of it.
- **`camera/rig.rs` had no `crate::prelude::*`**, only the BCS glob (the plan
  treats all four in-crate glob users the same). Narrowing it needed
  `use super::chase::ChaseCamera;` PLUS a new `use crate::prelude::*;` for
  `PointRotationOutput`.
- **`camera/mode.rs` still needs one BCS name**, `PointRotationInput` (Step 4's).
  Its glob narrows to that single import, not to nothing.
- **`nova_debug/src/lib.rs:17-23` supplies two names, not just the `debug::`
  half** the plan's 3h names: `InspectorDebugPlugin` and `WireframeDebugPlugin`
  at `:96-97`. Narrowed to both halves; Step 9 replaces the `debug::` aliases and
  Step 10 the two plugins.

**Hazards met.** The E0659 wave Step 1 predicted landed again, this time in
`camera/{framing,handback,mode,rig}.rs` (both globs live, `ChaseCamera*`
ambiguous). Step 1's probe - drop the glob, compile, read the unresolved list -
was again the cheapest fix and is now used three times. Six `camera_controller`
path references outside the module (`hud/mod.rs` x2,
`hud/screen_indicator.rs:1395`, `camera/{handback,mode}.rs` test mods) plus three
prose ones needed the rename chased; `git mv` early, before wiring, made the
compiler name every one.

**Prose swept here rather than deferred to 10e**, on Step 2's precedent (the
subject moved, so the sentence is wrong now, not at Step 10): all 12 `bcs` lines
in `camera/authority.rs` (including the test name
`the_chain_composes_with_every_bcs_camera_plugin`), `camera/framing.rs` x4,
`camera/{handback,mode}.rs` x3, `hud/screen_indicator.rs:226`, `juice.rs:11`,
`plugin.rs:6`, and the eight skybox-observer mentions across `nova_scenario`,
`nova_editor`, `nova_core` and `nova_assets`. Also
`web/src/wiki/dev/architecture.md:18`, which named `camera_controller` in the
crate table - a wiki row 10g's greps would have caught, but the rename makes it
stale NOW.

**Log filter.** No action needed (the Step 2 check): the rigs moved into
`nova_gameplay`, which is already on both `log_filter_str` lists. Confirmed
positively in the run log, not by absence.

**Evidence.** `cargo check --workspace --all-targets` clean; `cargo fmt --check`
clean; `clippy -p nova_gameplay --all-targets` adds zero warnings (all 34 are
pre-existing, none in `camera/` or `math.rs`); `cargo doc -p nova_gameplay
--no-deps` likewise (the two unresolved links in `camera/mod.rs:69,74` predate
this step). `nova_gameplay --lib camera::` 28/28 including
`the_chain_composes_with_every_camera_plugin` and the two `authority` ordering
tests; `--lib math::` 2/2; `--lib juice` 22/22; `--doc camera` 5/5 (all five
rewritten doctests); `nova_scenario --lib actions::view` 10/10;
`nova_scenario --test skybox_swap_e2e` 1/1. `git diff --exit-code
crates/nova_gameplay/Cargo.toml Cargo.lock` clean - no new dep, no new edge.

The `cd1bff21` ordering survives byte-identically: the only diff in
`authority.rs` outside comments is the two import blocks and the test rename;
both `configure_sets` calls are unchanged.

`examples/screenshots/screenshot_scene.rs` and `examples/sections/hull_section.rs`
RAN under Xvfb `:99` with the harness, both `autopilot: cycle complete, no
panic`. The registration move is proved positively: the log carries exactly one
`build` line for each of `WASDCameraPlugin`, `WASDCameraControllerPlugin`,
`ChaseCameraPlugin`, `SkyboxPlugin`, `PostProcessingDefaultPlugin`,
`CameraShakePlugin`, `SpaceshipCameraControllerPlugin` and
`CameraAuthorityPlugin`, every one on a `nova_gameplay::camera::*` path, and zero
`bevy_common_systems::camera` lines.

**Not run:** the workspace-wide `clippy --features debug` CI gate and any test
outside the filters above, per the standing skip-local-suite instruction. No
probe - Step 3's DoD does not ask for one (Steps 5, 7 and 10 do).

### Step 4 - DONE (transform rigs -> `nova_gameplay`)

**What.** The five BCS rigs + `mod.rs` -> `crates/nova_gameplay/src/transform/`
verbatim (823 L, `point_rotation`'s 5 tests included). `mod.rs` carries a nova
ownership docstring saying these drive `Transform` as *outputs* and why the
smoothing constants are gameplay decisions. Five registrations moved off
`plugin.rs:88-96` onto `crate::transform::prelude::` (the "for debug to have a
random orbiting object" comment kept). `PointRotation`, `PointRotationOutput`
and `DirectionalSphereOrbitOutput` left `lib.rs`'s BCS re-export list and now
arrive through `transform::prelude::*`. `SphereRandomOrbitPlugin` keeps its
inconsistent name, per the prototype.

**Two plan corrections.**

- **4b names one `crate::meth::prelude::*` import as needing
  `spherical_to_cartesian`; all three also need `LerpSnap`.** The glob supplied
  it invisibly. `sphere_orbit.rs:116-117` and
  `directional_sphere_orbit.rs:117-118` call `lerp_and_snap` on `f32`, so both
  imports are `crate::math::{spherical_to_cartesian, LerpSnap}` (plus
  `direction_to_spherical` in the directional one). `random_sphere_orbit.rs`
  genuinely needs only `spherical_to_cartesian`.
- **4f under-counts the glob users by one and over-counts by three.**
  `camera/mode.rs:6` is the extra: Step 3 narrowed it to exactly
  `PointRotationInput`, which is a transform name, so it is Step 4's to
  repoint - and it is the ONLY site the DoD grep caught, because it compiled
  fine pointing at BCS's type while the app registered nova's plugin. The same
  silent-divergence class Step 3 flagged. Going the other way,
  `input/player/intent.rs`, `turret_section/setup.rs` and `input/player/hints.rs`
  needed no narrowed BCS import at all: with the transform names gone their
  remaining BCS names (`PDController` and friends) already arrive through
  `crate::prelude`'s by-name list, so the glob was deleted outright rather than
  shrunk. `hud/velocity.rs` narrows to `TriangleMeshBuilder` alone (Step 5's)
  and gains `transform::prelude::*`; `turret_section/aim.rs` narrows to
  `rigid_body_point_velocity` alone (Step 7's).

**Hazards met.** The E0659 wave landed a third time, in the four files that
glob both preludes. Step 1's probe (replace the BCS glob with a marker, compile,
read the unresolved list, put back only those names) resolved it in one pass and
is now the standard move for this plan. `random_sphere_orbit.rs:7`'s
`use rand::prelude::*;` compiled unchanged against rand 0.10.2 exactly as
prototype 04 predicted - verified, not assumed.

**missing_docs sweep.** Nine items: the five `pub mod prelude` blocks, the five
`*Systems` enums and their `Sync` variants, `RandomSphereOrbitOutput`,
`SphereOrbitOutput`, and `SphereOrbitInput` + both its fields.

**Evidence.** `cargo check --workspace --all-targets` clean;
`cargo fmt --check` clean; `clippy -p nova_gameplay --all-targets --features
debug` adds zero warnings. `--lib transform::` 5/5, `--lib
sections::turret_section::aim` 11/11, `--doc` 5/5. Both absence proofs green:
the transform-name grep over `crates/ examples/` is empty, and
`git diff --exit-code crates/nova_gameplay/Cargo.toml Cargo.lock` is clean - no
new dep, no new edge. `examples/sections/turret_section.rs` RAN under Xvfb `:99`
and converged to a 0.2 deg aim error, exit 0 - the sharpest proof, since
`SmoothLookRotation` drives turret facing. `controller_section`, `player_path`
and `scene_baseline` also RAN with zero panic / duplicate-plugin lines (they
have no exit condition, so each was cut at its timeout after booting through
registration).

**Not run:** the workspace-wide `clippy --features debug` CI gate and any test
outside the filters above, per the standing skip-local-suite instruction. No
probe - Step 4's DoD does not ask for one.

One pre-existing warning is NOT from this step and was confirmed against a
clean stash: `examples/stress/many_sections.rs:37` unused `ComputedMass` /
`ComputedCenterOfMass`.

### Next

Step 5 (mesh builder / explode / slice -> `nova_gameplay`), prototype 06.
`crate::math::slerp` is in place for `builder.rs`. Step 5 adds the `noise` dep
and finally gives `hud/velocity.rs:16` and `sections/thruster_section.rs:10` a
nova home for `TriangleMeshBuilder`, which are the last two names holding those
BCS imports open.
