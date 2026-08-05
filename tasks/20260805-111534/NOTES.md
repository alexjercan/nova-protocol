# NOTES: Authorable scenario lighting

## Problem Statement

Scenario content has no lighting vocabulary. `on_load_scenario` hardcodes
exactly one `DirectionalLight` (illuminance 10000, rotated straight down) at
`crates/nova_scenario/src/loader/lifecycle.rs:203`, spawned next to the
scenario camera. Nothing in the authored surface can add to it, replace it, or
move it. The only escape is Rust: `examples/screenshots/shared/kit.rs:62` runs
an `On<Add, DirectionalLight>` observer that despawns the loader's light and
spawns a three-point key/rim/fill rig. Mods cannot write Rust, so every shipped
scene and every mod scene is stuck on the flat top-down key - no rim, no fill,
no silhouette separation from the skybox, and almost nothing bright for the
camera's `Bloom::NATURAL` to work with.

Owner framing (2026-08-05): the fix belongs in the moddable EVENT/ACTION
vocabulary - a light is spawned at runtime by an event action, the same path a
spaceship or beacon takes through `SpawnScenarioObject`. NOT a static
`lights: [...]` block on `ScenarioConfig`.

Owner framing (2026-08-05, second pass): the task also ships a CONSUMER, so the
feature is visually provable rather than merely present:

- the three shipped menu backdrops (`menu_ambience`, `menu_scrapyard`,
  `menu_waystation`) get authored lighting;
- the screenshot examples that currently hardcode a photo rig author it in
  their `ScenarioConfig` instead;
- the owner visually inspects the resulting frames.

What this is NOT:

- not a rendering or post-processing overhaul (bloom, tonemapping, exposure
  stay as `PostProcessingDefaultPlugin` sets them);
- not a lighting preset system ("key+rim", "backlit", "flat") - the RON
  authors lights, not names;
- not relighting the campaign scenarios (`broadside`, `shakedown`, `lifeline`,
  `final_tally`) - backdrops and the screenshot scenes are the scope;
- not attaching lights to moving objects.

## Context

### The moddable action surface lives in `nova_scenario`, not `nova_mod_format`

`nova_mod_format` holds bundle manifests, the installed catalog and the portal
wire schema - not the scenario vocabulary. Mods reach the action vocabulary
because a bundle ships scenarios as `*.content.ron`, which deserialize straight
into `nova_scenario`'s serde types. So the authorable surface for this feature
is `EventActionConfig`.

| Layer | Where | Role |
| --- | --- | --- |
| Action vocabulary | `actions/mod.rs:42` | `EventActionConfig`, 22 variants, serde-derived - the mod-facing RON surface |
| Dispatch | `actions/mod.rs:90` | one match arm per variant into `EventAction::action` |
| Prelude | `actions/mod.rs:22` | every public action type is re-exported here (repo rule: new public items require prelude exports) |
| Spawn path | `actions/spawn.rs:129` | `ScenarioObjectConfig { base, kind }` -> `ScenarioObjectKind` -> `world.push_command(\|commands\| ...)` |
| Shared bundle | `actions/spawn.rs:95` `base_scenario_object` | `ScenarioScopedMarker` + `Name` + `EntityId` + `Transform` + `RigidBody::Dynamic` + `TransformInterpolation` + `Visibility::Visible` |
| Object kinds | `actions/spawn.rs:114` | `Asteroid`, `Spaceship`, `Beacon`, `SalvageCrate` |
| Object plugins | `objects/mod.rs:34` | one plugin per kind; `render: bool` threaded so headless tools skip visuals |
| Despawn | `actions/spawn.rs:31` | `DespawnScenarioObjectActionConfig` resolves by `EntityId` gated on `ScenarioScopedMarker` - kind-agnostic |
| Render-only action precedent | `actions/view.rs:153` | `SetSkyboxActionConfig`: a pure-render scenario action with no body, documented as a modding hook |
| Lint | `lint/scenario.rs:79,248,281` | walks `SpawnScenarioObject` actions per event |
| Loader stats | `loader/mod.rs:252` | `object_count` counts `SpawnScenarioObject` actions |
| The hardcoded light | `loader/lifecycle.rs:203` | unconditional, no `EntityId`, no `Name` - unreachable from RON, including from `DespawnScenarioObject` |

### Precedent that decides the shape

`beacon_scenario_object` (`objects/beacon.rs:66`) already overrides the shared
base bundle's `RigidBody::Dynamic` with `RigidBody::Static` and ships with the
base's `TransformInterpolation` still attached. A non-physical, never-moving
scenario object is therefore an established pattern, not a new one.

### The consumers, and what they already prove

`examples/screenshots/shared/kit.rs` holds the rig the feature must be able to
express. Its numbers are the reference target - three `DirectionalLight`s,
transforms authored as `from_translation(..).looking_at(Vec3::ZERO, Vec3::Y)`:

| Light | Position | Illuminance | Color | Shadows |
| --- | --- | --- | --- | --- |
| Key | `(-6, 5, 6)` | 11000 | `srgb(1.0, 0.96, 0.90)` | yes |
| Rim | `(3, 4, -8)` | 16000 | `srgb(0.72, 0.86, 1.0)` | no |
| Fill | `(7, -2, 4)` | 2600 | `srgb(0.62, 0.72, 0.95)` | no |

So the minimum authorable field set is: transform (position + rotation),
`color`, `illuminance`, `shadow_maps_enabled`.

Three examples add `kit::photo_rig()` and build their own `ScenarioConfig` in
code, which makes them drop-in consumers - the authored rig can carry the exact
same numbers, so the captured frames should be unchanged:

- `examples/screenshots/screenshot_scene.rs:70,92` (`drydock_drift`)
- `examples/screenshots/screenshot_combat.rs:178,557` (`rock_hollow`)
- `examples/screenshots/screenshot_flight.rs:118,248` (`the_ring`)

`screenshot_sections.rs`, `screenshot_nova_os.rs` and `render_scale_shot.rs`
build or load scenarios WITHOUT the rig, and stay on the default light.

The menu backdrops are Rust builders that emit generated RON:
`crates/nova_assets/src/scenario/menu.rs` (`menu_ambience` and siblings) ->
`assets/base/scenarios/*.content.ron`. Repo rule: edit the builder, run
`cargo run -p nova_assets --bin content -- gen`, commit both. Never hand-edit
the generated RON.

### Constraints carried verbatim

- Cross-subsystem communication through `nova_events`, not direct coupling.
- Imports through crate `prelude`; new public items require prelude exports.
- One plugin per subsystem; group systems with `SystemSet`.
- Base `assets/base/**/*.content.ron` is generated. Edit Rust builders, run
  `content -- gen`, commit both.
- Public items explain what and why; rustdoc intra-doc links for reachable
  types; keep `cargo doc --workspace --no-deps` warning-free.
- Bevy examples must be RUN (Xvfb :99), not just `cargo check`ed.
- Documentation: internals/format change -> `web/src/wiki/dev/modding-ron.md`
  and the scenario-authoring guides; user-visible behavior -> `CHANGELOG.md`.
- The `render: bool` flag exists so headless tools spawn objects without
  visuals; a light-bearing plugin, if any, must respect it.

### Owner answers (2026-08-05 quiz), carried verbatim

- Shape: `ScenarioObjectKind::Light`, not a sibling action.
- Default light: "we relight everything both in .rs code and in RON files to
  prove that it works + we can even add more lights to make it actually look
  cool (e.g I really like how the screenshot examples look like now with more
  lights)."
- Light types: "Let's add Directional + Point and we use Point somewhere in
  the main menu ones just because I think we should support 2 Lighting methods
  from the get go to allow easily changing it later."
- Lint: no new rule.
- The relight is not a compile-fix pass. Scenes are lit to LOOK good, judged by
  owner visual inspection, with the screenshot photo rig as the quality bar.

### The relight surface, counted

Deleting the loader light means every scene that RENDERS must author its own.

| Group | Count | Notes |
| --- | --- | --- |
| Shipped scenarios | 10 | via 7 Rust builders in `nova_assets/src/scenario*`; regenerate RON with `content -- gen` |
| Hand-authored mod RON | 9 | `webmods/the-ledger/*` (8), `webmods/gauntlet/gauntlet.content.ron`, `assets/mods/example/example.content.ron` - edited directly, NOT generated |
| Rendering examples | 13 | `examples/sections/*` (5), `examples/stress/*` (4), `examples/systems/*` (3), `examples/ui/hud_range.rs` - all build real apps via `AppBuilder::new` |
| Screenshot examples | 6 | 3 already carry `kit::photo_rig()` and migrate their exact numbers into authored RON; 3 (`screenshot_sections`, `screenshot_nova_os`, `render_scale_shot`) need lighting authored fresh |
| Editor play-test | 1 | `nova_editor/src/scenario.rs:31` - note the EDITOR view itself has its own light (`nova_editor/src/ui/mod.rs:91`) and is unaffected |
| Headless fixtures | ~4 | `nova_menu/src/tests/*` (MinimalPlugins), `nova_scenario/src/{lint,loader}/fixtures.rs` - never render; must compile, need no lights |

Third-party mods outside this repo cannot be relit and will render black until
their authors add light actions. This is a breaking mod-format change and ships
as one.

## Ideas

Ranked best-first. The owner quiz settled 1 and 2; 3-5 are the losers, kept
with the reason they lost.

### 1. `ScenarioObjectKind::Light(LightConfig)` - WON

A fifth object kind on the existing `SpawnScenarioObject` path.
`BaseScenarioObjectConfig` already supplies id, name, position and rotation -
which is the entire transform half of a light. `DespawnScenarioObject`,
`ScatterObjects` and the lint walk all work unchanged because they key on the
object envelope, not the kind. The kind-specific config is small:

- `Directional { illuminance, color, shadows }`
- `Point { intensity, range, radius, color, shadows }`

Cost: the shared `base_scenario_object` bundle attaches `RigidBody::Dynamic`
and `TransformInterpolation`. The light bundle overrides to `RigidBody::Static`
- exactly what `beacon_scenario_object` already does, so the pattern is proven
rather than invented.

Two light types ship together on the owner's call: supporting a second method
from the start keeps the enum shape honest and makes swapping a scene's
lighting method a one-line RON edit later.

### 2. Delete the loader's hardcoded light and relight every scene - WON

`lifecycle.rs:203` goes away entirely. No magic light, no fallback branch, no
reserved entity id: what a scene looks like is exactly what it authored. The
relight is the feature's proof - 38-odd scenes, lit deliberately, inspected by
eye.

Beat the fallback (idea 4) on the owner's explicit call after being shown the
blast radius: relighting everything is the demonstration, not a tax on it.

### 3. `EventActionConfig::SpawnLight` sibling action - LOST

A render-only action next to `SetSkybox`, with no physics body at all. More
honest about what a light is, and despawn still works free (the despawn action
is kind-agnostic - it matches `EntityId` under `ScenarioScopedMarker`).

Lost to 1: it re-declares id/name/position/rotation that the object envelope
already standardises, and it sits outside `ScatterObjects`, the lint walk and
`object_count`. One new enum variant beats one new parallel vocabulary.

### 4. Keep the light as a fallback when no light is authored - LOST

`if !scenario.authors_light() { spawn the old light }`. Zero breakage for the
9 hand-authored mod scenes and every third-party mod; shipped scenes stay
byte-identical on landing.

Lost to 2 on the owner's call. The cost it avoids (relighting 38 scenes) is
the deliverable the owner wants, and the magic light it preserves is the thing
the task exists to remove.

### 5. `default_lighting()` Rust helper instead of an engine default - LOST

Delete the engine light, export a helper returning the standard key-light
actions, and let Rust-side scenes opt in with one line.

Lost to 2 for a concrete reason: a helper is unreachable from hand-authored
RON, so the 9 mod scenes get the fully spelled-out block anyway and still
break. It buys terseness for the easy half and nothing for the hard half.

### 6. Named lighting presets ("key+rim", "backlit", "flat") - LOST

TASK.md's own open question, and its YAGNI-preferred answer at filing time.

Lost on the owner's reframing: presets are a naming layer over the thing that
does not exist yet. Author lights first; a preset, if ever wanted, is a
builder-side helper over authored lights, not a RON vocabulary.

## Code sketch

Illustrative, not final - the shape the owner reviewed before planning.

### `crates/nova_scenario/src/objects/light.rs` (new)

Mirrors `beacon.rs` deliberately: a config component on the entity plus an
`Add` observer that inserts the real component. That is not decoration - a
`match` cannot return one `impl Bundle` holding either a `DirectionalLight` or
a `PointLight`, and the observer split is what lets `render: false` skip the
light component entirely for headless tools.

```rust
//! Light scenario object: an authored key/rim/fill or point light. Replaces
//! the loader's former hardcoded top-down key light - a scene now looks like
//! exactly what it authored.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_events::prelude::*;

use crate::prelude::*;

pub mod prelude {
    pub use super::{light_scenario_object, LightConfig, LightPlugin, LIGHT_TYPE_NAME};
}

/// The scenario/modding RON type name for a light object.
pub const LIGHT_TYPE_NAME: &str = "light";

/// The scenario/modding RON surface for a light. Position and rotation come
/// from [`BaseScenarioObjectConfig`] like any other scenario object; this picks
/// the lighting METHOD and its per-method parameters.
///
/// A directional light is infinitely far away, so only its rotation matters -
/// `aim` exists because authoring "shine at this point" by hand is possible and
/// authoring the equivalent quaternion by hand is not.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum LightConfig {
    /// A sun: parallel rays, direction only. The key/rim/fill workhorse.
    Directional {
        /// Illuminance in lux (the shipped default key light was 10000).
        illuminance: f32,
        /// Light color.
        color: Color,
        /// Whether this light casts shadows. One shadow caster per scene is
        /// usually right; a second on a blocky hull reads as dirt, not depth.
        shadows: bool,
        /// When set, the light is aimed at this world point and the base
        /// config's `rotation` is ignored. `None` uses `rotation` directly.
        #[cfg_attr(
            feature = "serde",
            serde(default, skip_serializing_if = "Option::is_none")
        )]
        aim: Option<Vec3>,
    },
    /// A positional lamp: a star, a hangar floodlight, a nebula glow.
    Point {
        /// Luminous intensity in lumens.
        intensity: f32,
        /// Distance (world units) past which the light contributes nothing.
        range: f32,
        /// Source radius, softening the shadow terminator.
        radius: f32,
        /// Light color.
        color: Color,
        /// Whether this light casts shadows.
        shadows: bool,
    },
}

/// Marks a scenario light, so the insert observer can find it.
#[derive(Component, Clone, Debug, Reflect)]
pub struct LightMarker;

/// The authored config, consumed by `insert_light`.
#[derive(Component, Clone, Debug, Deref, Reflect)]
pub struct ScenarioLightConfig(pub LightConfig);

/// Build the light bundle from a [`LightConfig`].
///
/// `RigidBody::Static` overrides the shared base bundle's `Dynamic`: a light is
/// posed, not simulated. Same override `beacon_scenario_object` makes, for the
/// same reason.
pub fn light_scenario_object(config: LightConfig) -> impl Bundle {
    debug!("light_scenario_object: config {:?}", config);

    (
        LightMarker,
        EntityTypeName::new(LIGHT_TYPE_NAME),
        ScenarioLightConfig(config),
        RigidBody::Static,
    )
}

/// The light scenario object. `render` gates the actual light component, so a
/// headless tool spawns the posed entity without lighting anything.
pub struct LightPlugin {
    /// Whether to insert the real light component (false for headless tools).
    pub render: bool,
}

impl Plugin for LightPlugin {
    fn build(&self, app: &mut App) {
        debug!("LightPlugin: build");

        if self.render {
            app.add_observer(insert_light);
        }
    }
}

/// Insert the Bevy light the config names, and apply `aim` when authored.
fn insert_light(
    add: On<Add, LightMarker>,
    mut commands: Commands,
    q_light: Query<(&ScenarioLightConfig, &Transform), With<LightMarker>>,
) {
    let entity = add.entity;
    let Ok((config, transform)) = q_light.get(entity) else {
        error!("insert_light: entity {:?} not found in q_light", entity);
        return;
    };

    match **config {
        LightConfig::Directional {
            illuminance,
            color,
            shadows,
            aim,
        } => {
            let mut entity_commands = commands.entity(entity);
            entity_commands.insert(DirectionalLight {
                illuminance,
                color,
                shadow_maps_enabled: shadows,
                ..default()
            });
            if let Some(target) = aim {
                entity_commands.insert(
                    Transform::from_translation(transform.translation)
                        .looking_at(target, Vec3::Y),
                );
            }
        }
        LightConfig::Point {
            intensity,
            range,
            radius,
            color,
            shadows,
        } => {
            commands.entity(entity).insert(PointLight {
                intensity,
                range,
                radius,
                color,
                shadow_maps_enabled: shadows,
                ..default()
            });
        }
    }
}
```

### The three plumbing edits

```rust
// objects/mod.rs - the module, the prelude re-export, and the plugin
pub mod light;
// prelude: light::prelude::*
app.add_plugins(light::LightPlugin { render: self.render });

// actions/spawn.rs - the fifth kind
pub enum ScenarioObjectKind {
    Asteroid(AsteroidConfig),
    Spaceship(SpaceshipConfig),
    Beacon(BeaconConfig),
    SalvageCrate(SalvageCrateConfig),
    /// An authored light - the scene's own key, rim, fill or lamp.
    Light(LightConfig),
}

// actions/spawn.rs - the fifth dispatch arm
ScenarioObjectKind::Light(config) => {
    entity_commands.insert(light_scenario_object(config.clone()));
}

// loader/lifecycle.rs - the hardcoded DirectionalLight spawn is DELETED
```

### Authoring helper for the Rust builders

A directional light is authored as "where it shines FROM", which is a position
plus a look-at, not a quaternion. Builders get one small helper rather than
thirteen open-coded `looking_at(..).rotation` calls:

```rust
// nova_scenario/src/objects/light.rs
/// Base config for a directional light placed at `from` and aimed at `target`.
/// The position is cosmetic for a directional light (only rotation is read),
/// but it keeps the authored numbers readable as a physical rig.
pub fn aimed_light_base(id: &str, name: &str, from: Vec3, target: Vec3) -> BaseScenarioObjectConfig {
    BaseScenarioObjectConfig {
        id: id.to_string(),
        name: name.to_string(),
        position: from,
        rotation: Transform::from_translation(from).looking_at(target, Vec3::Y).rotation,
    }
}
```

### Consumer 1: a `.rs` scenario builder

`crates/nova_assets/src/scenario/menu.rs`, `menu_ambience` - the three-point rig
carried over from `kit.rs`, plus the `Point` light the owner asked for: a warm
lamp just off the planetoid, giving the rocks a falloff the sun cannot.

```rust
// The rig, authored as objects like everything else in the scene. Replaces the
// loader's former single top-down key light (deleted in this task).
let light = |id: &str, name: &str, from: Vec3, illuminance: f32, color: Color, shadows: bool| {
    ScenarioObjectConfig {
        base: aimed_light_base(id, name, from, Vec3::ZERO),
        kind: ScenarioObjectKind::Light(LightConfig::Directional {
            illuminance,
            color,
            shadows,
            aim: None, // the base rotation already aims it
        }),
    }
};

// Key: warm, high, camera-left - the only shadow caster.
objects.push(light(
    "menu_key", "Menu Key Light",
    Vec3::new(-120.0, 90.0, 110.0), 11000.0,
    Color::srgb(1.0, 0.96, 0.90), true,
));
// Rim: cold and hard from behind, separating the planetoid from the skybox.
objects.push(light(
    "menu_rim", "Menu Rim Light",
    Vec3::new(60.0, 70.0, -150.0), 16000.0,
    Color::srgb(0.72, 0.86, 1.0), false,
));
// Fill: cool and dim from the shadow side.
objects.push(light(
    "menu_fill", "Menu Fill Light",
    Vec3::new(140.0, -40.0, 80.0), 2600.0,
    Color::srgb(0.62, 0.72, 0.95), false,
));
// The lamp: a positional warm glow riding just off the planetoid's limb, so
// the scatter ring falls off with distance instead of reading uniformly lit.
objects.push(ScenarioObjectConfig {
    base: BaseScenarioObjectConfig {
        id: "menu_lamp".to_string(),
        name: "Menu Planetoid Glow".to_string(),
        position: Vec3::new(-60.0, 20.0, 90.0),
        rotation: Quat::IDENTITY,
    },
    kind: ScenarioObjectKind::Light(LightConfig::Point {
        intensity: 2_500_000.0,
        range: 400.0,
        radius: 12.0,
        color: Color::srgb(1.0, 0.82, 0.6),
        shadows: false,
    }),
});
```

### Consumer 2: a hand-authored `.ron` scenario

`webmods/the-ledger/ledger_ch1.content.ron`, added to the existing `OnStart`
actions list. This is the case `aim` exists for - a mod author writes the point
the light shines at, never a quaternion. Spelling matches what serde emits for
the shipped scenarios (`Srgba((red: ..))`, struct variants as
`Variant(field: val)`).

```ron
SpawnScenarioObject((
    base: (
        id: "ch1_key",
        name: "Freighter Key Light",
        position: (-80.0, 60.0, 70.0),
        rotation: (0.0, 0.0, 0.0, 1.0),
    ),
    kind: Light(Directional(
        illuminance: 11000.0,
        color: Srgba((
            red: 1.0,
            green: 0.96,
            blue: 0.90,
            alpha: 1.0,
        )),
        shadows: true,
        aim: Some((0.0, 0.0, 0.0)),
    )),
)),
SpawnScenarioObject((
    base: (
        id: "ch1_rim",
        name: "Freighter Rim Light",
        position: (40.0, 50.0, -90.0),
        rotation: (0.0, 0.0, 0.0, 1.0),
    ),
    kind: Light(Directional(
        illuminance: 16000.0,
        color: Srgba((
            red: 0.72,
            green: 0.86,
            blue: 1.0,
            alpha: 1.0,
        )),
        shadows: false,
        aim: Some((0.0, 0.0, 0.0)),
    )),
)),
```

Despawning one needs no new vocabulary - the existing action already resolves
any scoped `EntityId`:

```ron
DespawnScenarioObject((id: "ch1_key")),
```

### Deferred thread: the base object bundle

Owner observation (2026-08-05): `base_scenario_object` hands every scenario
object `RigidBody::Dynamic` + `TransformInterpolation`, which is strict once
non-physical objects exist - beacons already override to `Static`, and lights
make it two. Worth rethinking what the base is allowed to assume. Explicitly
NOT this task: lights follow the beacon precedent and override.

## Open assumptions

- The three-point rig numbers in `kit.rs` transfer to authored RON unchanged,
  so `screenshot_scene` / `screenshot_combat` / `screenshot_flight` capture
  visually identical frames after the migration. To be verified by running the
  examples under Xvfb and comparing, not by `cargo check`.
- A `RigidBody::Static` light with the base bundle's `TransformInterpolation`
  behaves inertly, as it does on beacons.
- No content-lint rule is needed because a black scene is now an authoring
  mistake the eye catches during the relight, not a class of bug the lint can
  usefully name.
