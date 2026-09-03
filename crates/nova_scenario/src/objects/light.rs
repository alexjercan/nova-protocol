//! Light scenario object: an authored key/rim/fill or point light. Replaces the
//! loader's former hardcoded top-down key light - a scene now looks like exactly
//! what it authored, and a scene that authors nothing renders black.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_events::prelude::*;

use crate::prelude::*;

/// The light scenario object, `LightConfig`, the `ThreePointRig`, the `aimed_light_base` helper and
/// `LightPlugin`.
pub mod prelude {
    pub use super::{
        aimed_light_base, light_scenario_object, LightConfig, LightPlugin, ThreePointRig,
    };
}

/// The scenario/modding RON surface for a light. Position and rotation come from
/// [`BaseScenarioObjectConfig`] like any other scenario object; this picks the
/// lighting METHOD and its per-method parameters.
///
/// A directional light is infinitely far away, so only its rotation matters -
/// `aim` exists because authoring "shine at this point" by hand is possible and
/// authoring the equivalent quaternion by hand is not.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum LightConfig {
    /// A sun: parallel rays, direction only. The key/rim/fill workhorse.
    Directional {
        /// Illuminance in lux (the engine's former default key light was 10000).
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
        aim: Option<Meters3>,
    },
    /// A positional lamp: a star, a hangar floodlight, a nebula glow.
    Point {
        /// Luminous intensity in lumens.
        intensity: f32,
        /// Distance past which the light contributes nothing.
        range: Meters,
        /// Source radius, softening the shadow terminator.
        radius: Meters,
        /// Light color.
        color: Color,
        /// Whether this light casts shadows.
        shadows: bool,
    },
}

/// Marks a scenario light, so [`LightPlugin`]'s insert observer can find it.
#[derive(Component, Clone, Debug, Reflect)]
pub struct LightMarker;

/// The authored light config, consumed by the insert observer.
#[derive(Component, Clone, Debug, Deref, Reflect)]
#[reflect(opaque)]
pub struct ScenarioLightConfig(pub LightConfig);

/// Build the light bundle from a [`LightConfig`]: the marker, the authored
/// config, and a body that holds its pose.
///
/// `RigidBody::Static` because a light is posed, not simulated. Same choice
/// `beacon_scenario_object` makes, for the same reason.
pub fn light_scenario_object(config: LightConfig) -> impl Bundle {
    trace!("light_scenario_object: config {:?}", config);

    (
        LightMarker,
        EntityTypeName::new(LIGHT_TYPE_NAME),
        ScenarioLightConfig(config),
        RigidBody::Static,
    )
}

/// Base config for a directional light placed at `from` and aimed at `target`.
///
/// The position is cosmetic for a directional light (only rotation is read), but
/// it keeps the authored numbers readable as a physical rig - "the key comes
/// from up and camera-left" rather than a quaternion.
pub fn aimed_light_base(
    id: &str,
    name: &str,
    from: Meters3,
    target: Meters3,
) -> BaseScenarioObjectConfig {
    BaseScenarioObjectConfig {
        id: id.to_string(),
        name: name.to_string(),
        position: from,
        // Engine boundary: `looking_at` poses a Bevy transform, so both ends
        // of the aim cross into world units to build the rotation.
        rotation: Transform::from_translation(from.to_engine())
            .looking_at(target.to_engine(), Vec3::Y)
            .rotation,
    }
}

/// The key/rim/fill offsets, colors and illuminances the screenshot set was
/// shot with (`examples/screenshots/shared/kit.rs`, before this task moved the
/// rig into authored content). Offsets are the rig's SHAPE in meters at scale
/// 1: a scene multiplies them by [`ThreePointRig::scale`].
///
/// Key is warm, high and camera-left, carrying the subject's main form, and is
/// the ONLY shadow caster - a second on a blocky hull reads as dirt rather than
/// depth. Rim is a cold hard edge from behind, separating a dark hull from a
/// dark skybox. Fill is cool and dim from the shadow side, so the unlit half
/// keeps detail without flattening the key.
const THREE_POINT_LIGHTS: [(&str, &str, Meters3, f32, Srgba, bool); 3] = [
    (
        "key",
        "Key Light",
        Meters3(Vec3::new(-60.0, 50.0, 60.0)),
        11000.0,
        Srgba::new(1.0, 0.96, 0.90, 1.0),
        true,
    ),
    (
        "rim",
        "Rim Light",
        Meters3(Vec3::new(30.0, 40.0, -80.0)),
        16000.0,
        Srgba::new(0.72, 0.86, 1.0, 1.0),
        false,
    ),
    (
        "fill",
        "Fill Light",
        Meters3(Vec3::new(70.0, -20.0, 40.0)),
        2600.0,
        Srgba::new(0.62, 0.72, 0.95, 1.0),
        false,
    ),
];

/// The repo's standard three-point key/rim/fill rig, as authored scenario
/// objects. This is the quality bar every relit scene is judged against, so it
/// ships as one helper rather than as thirty hand-copied triples.
///
/// A directional light has no falloff and no position - only its DIRECTION is
/// read - so scaling the rig uniformly leaves the lighting identical and only
/// changes how the authored numbers read next to the scene's own dimensions.
/// That is why [`scale`](Self::scale) is free: a 60 m hero shot and a 2 km
/// planetoid backdrop get the same light for the same numbers.
pub struct ThreePointRig {
    /// Prefix for each light's scenario id and name (`"{prefix}_key"`).
    pub prefix: String,
    /// The world point all three lights aim at.
    pub target: Meters3,
    /// Dimensionless multiplier on the rig's offsets. `1.0` is the screenshot
    /// set's original hero-shot rig verbatim.
    pub scale: f32,
}

impl ThreePointRig {
    /// The rig around `target`, with its offsets multiplied by `scale`.
    pub fn around(prefix: &str, target: Meters3, scale: f32) -> Self {
        Self {
            prefix: prefix.to_string(),
            target,
            scale,
        }
    }

    /// The three light objects, ready to spawn from a scenario's `OnStart`.
    pub fn objects(&self) -> Vec<ScenarioObjectConfig> {
        THREE_POINT_LIGHTS
            .iter()
            .map(
                |(role, name, offset, illuminance, color, shadows)| ScenarioObjectConfig {
                    base: aimed_light_base(
                        &format!("{}_{}", self.prefix, role),
                        name,
                        self.target + *offset * self.scale,
                        self.target,
                    ),
                    kind: ScenarioObjectKind::Light(LightConfig::Directional {
                        illuminance: *illuminance,
                        color: Color::Srgba(*color),
                        shadows: *shadows,
                        // The base rotation already aims it; `aim` is for
                        // hand-authored RON, where a quaternion is unwritable.
                        aim: None,
                    }),
                },
            )
            .collect()
    }

    /// The same three lights as `OnStart` spawn actions, for the scenes that
    /// assemble an action list rather than an object list.
    pub fn actions(&self) -> Vec<EventActionConfig> {
        self.objects()
            .into_iter()
            .map(EventActionConfig::SpawnScenarioObject)
            .collect()
    }
}

/// The light scenario object. `render` gates the actual Bevy light component, so
/// a headless tool spawns the posed entity without lighting anything.
pub struct LightPlugin {
    /// Whether to insert the real light component (false for headless tools).
    pub render: bool,
}

impl Plugin for LightPlugin {
    fn build(&self, app: &mut App) {
        trace!("LightPlugin: build");

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
            // Engine boundary: the transform this re-aims is Bevy's, so the
            // authored target crosses into world units to be looked at.
            let aimed = aim.map(|target| {
                Transform::from_translation(transform.translation)
                    .looking_at(target.to_engine(), Vec3::Y)
            });
            let mut entity_commands = commands.entity(entity);
            entity_commands.insert(DirectionalLight {
                illuminance,
                color,
                shadow_maps_enabled: shadows,
                ..default()
            });
            if let Some(aimed) = aimed {
                entity_commands.insert(aimed);
            }
        }
        LightConfig::Point {
            intensity,
            range,
            radius,
            color,
            shadows,
        } => {
            // Engine boundary: Bevy's falloff and source radius are world
            // units.
            commands.entity(entity).insert(PointLight {
                intensity,
                range: range.to_engine(),
                radius: radius.to_engine(),
                color,
                shadow_maps_enabled: shadows,
                ..default()
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Spawn one light object through the shared base bundle, exactly as the
    /// `SpawnScenarioObject` action does, into an app carrying [`LightPlugin`]
    /// at the given `render` setting.
    ///
    /// Spawn-then-insert, not one combined bundle, so the rig composes the two
    /// bundles the same way the action does.
    ///
    /// The app carries the REAL physics plugins and ticks several times, same
    /// shape as the `spawn` action's own harness (`MeshPlugin` because avian's
    /// collider-from-mesh backend reads `AssetEvent<Mesh>`). What that pins:
    /// the authored pose holds under the production plugin stack, the light's
    /// `RigidBody::Static` body staying inert across ticks. Every hand-authored
    /// mod light gets its rotation through `aim`, so that path is worth running
    /// against the plugins that could disturb it rather than `MinimalPlugins`
    /// alone.
    fn spawn_light(
        render: bool,
        base: BaseScenarioObjectConfig,
        config: LightConfig,
    ) -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            TransformPlugin,
            bevy::asset::AssetPlugin::default(),
            bevy::mesh::MeshPlugin,
            PhysicsPlugins::default(),
            LightPlugin { render },
        ));
        app.insert_resource(Gravity(Vec3::ZERO));
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f32(0.004),
        ));
        app.finish();
        let entity = app
            .world_mut()
            .spawn(base_scenario_object(&base))
            .insert(light_scenario_object(config))
            .id();
        for _ in 0..8 {
            app.update();
        }
        (app, entity)
    }

    /// A `Directional` light object carries the authored illuminance, color and
    /// shadow flag onto a real `DirectionalLight`, and an authored `aim` beats
    /// the base config's rotation.
    #[test]
    fn directional_light_object_inserts_authored_light() {
        let color = Color::srgb(1.0, 0.96, 0.90);
        // A base rotation deliberately NOT pointing at the origin, so the aim
        // assert cannot pass by coincidence.
        let base = BaseScenarioObjectConfig {
            id: "key".to_string(),
            name: "Key Light".to_string(),
            position: Meters3::new(-60.0, 50.0, 60.0),
            rotation: Quat::IDENTITY,
        };
        let (app, entity) = spawn_light(
            true,
            base.clone(),
            LightConfig::Directional {
                illuminance: 11000.0,
                color,
                shadows: true,
                aim: Some(Meters3::ZERO),
            },
        );

        let light = app
            .world()
            .get::<DirectionalLight>(entity)
            .expect("a Directional object inserts a DirectionalLight");
        assert_eq!(light.illuminance, 11000.0);
        assert_eq!(light.color, color);
        assert!(light.shadow_maps_enabled);
        assert!(
            app.world().get::<PointLight>(entity).is_none(),
            "a Directional object inserts no PointLight"
        );

        // A directional light shines down its own -Z, so aimed-at-origin means
        // the forward axis points from the light back to the origin.
        let transform = app.world().get::<Transform>(entity).unwrap();
        let expected =
            Transform::from_translation(base.position.to_engine()).looking_at(Vec3::ZERO, Vec3::Y);
        assert!(
            transform.rotation.angle_between(expected.rotation) < 1e-4,
            "aim overrides the base rotation: got {:?}, want {:?}",
            transform.rotation,
            expected.rotation
        );
        assert_eq!(
            transform.translation,
            base.position.to_engine(),
            "aiming keeps the authored position"
        );
        assert!(
            matches!(
                app.world().get::<RigidBody>(entity),
                Some(RigidBody::Static)
            ),
            "a light is posed, not simulated"
        );
    }

    /// A `Point` light object inserts a `PointLight` with the authored numbers -
    /// and with `render: false` a light object inserts NO light component at
    /// all, so headless tools spawn the posed entity without lighting anything.
    #[test]
    fn point_light_object_and_headless_render_flag() {
        let color = Color::srgb(1.0, 0.82, 0.6);
        let base = BaseScenarioObjectConfig {
            id: "lamp".to_string(),
            name: "Planetoid Glow".to_string(),
            position: Meters3::new(-600.0, 200.0, 900.0),
            rotation: Quat::IDENTITY,
        };
        let config = LightConfig::Point {
            intensity: 2_500_000.0,
            range: Meters(4_000.0),
            radius: Meters(120.0),
            color,
            shadows: false,
        };

        let (app, entity) = spawn_light(true, base.clone(), config.clone());
        let light = app
            .world()
            .get::<PointLight>(entity)
            .expect("a Point object inserts a PointLight");
        assert_eq!(light.intensity, 2_500_000.0);
        assert_eq!(light.range, 400.0, "4 km of falloff is 400 world units");
        assert_eq!(light.radius, 12.0);
        assert_eq!(light.color, color);
        assert!(!light.shadow_maps_enabled);
        assert!(
            app.world().get::<DirectionalLight>(entity).is_none(),
            "a Point object inserts no DirectionalLight"
        );

        let (headless, entity) = spawn_light(false, base, config);
        assert!(
            headless.world().get::<PointLight>(entity).is_none()
                && headless.world().get::<DirectionalLight>(entity).is_none(),
            "render: false inserts no light component at all"
        );
        assert!(
            headless.world().get::<LightMarker>(entity).is_some(),
            "the posed entity still spawns headless"
        );
    }

    /// Both variants survive a RON round-trip: hand-authored mod files are a
    /// supported input, not just the generated ones.
    #[cfg(feature = "serde")]
    #[test]
    fn light_config_ron_round_trip() {
        let directional = LightConfig::Directional {
            illuminance: 16000.0,
            color: Color::srgb(0.72, 0.86, 1.0),
            shadows: false,
            aim: Some(Meters3::new(1.0, 2.0, 3.0)),
        };
        let ron = ron::to_string(&directional).expect("serialize");
        match ron::from_str::<LightConfig>(&ron).expect("deserialize") {
            LightConfig::Directional {
                illuminance,
                color,
                shadows,
                aim,
            } => {
                assert_eq!(illuminance, 16000.0);
                assert_eq!(color, Color::srgb(0.72, 0.86, 1.0));
                assert!(!shadows);
                assert_eq!(aim, Some(Meters3::new(1.0, 2.0, 3.0)));
            }
            other => panic!("variant changed on round-trip: {other:?}"),
        }

        // `aim` is skipped when None, so an authored file may omit it entirely.
        let no_aim = ron::to_string(&LightConfig::Directional {
            illuminance: 1.0,
            color: Color::WHITE,
            shadows: true,
            aim: None,
        })
        .expect("serialize");
        assert!(
            !no_aim.contains("aim"),
            "an unset aim is not emitted: {no_aim}"
        );
        assert!(matches!(
            ron::from_str::<LightConfig>(&no_aim).expect("deserialize"),
            LightConfig::Directional { aim: None, .. }
        ));

        let point = LightConfig::Point {
            intensity: 2_500_000.0,
            range: Meters(4_000.0),
            radius: Meters(120.0),
            color: Color::srgb(1.0, 0.82, 0.6),
            shadows: true,
        };
        let ron = ron::to_string(&point).expect("serialize");
        match ron::from_str::<LightConfig>(&ron).expect("deserialize") {
            LightConfig::Point {
                intensity,
                range,
                radius,
                color,
                shadows,
            } => {
                assert_eq!(intensity, 2_500_000.0);
                assert_eq!(range, Meters(4_000.0));
                assert_eq!(radius, Meters(120.0));
                assert_eq!(color, Color::srgb(1.0, 0.82, 0.6));
                assert!(shadows);
            }
            other => panic!("variant changed on round-trip: {other:?}"),
        }
    }
}
