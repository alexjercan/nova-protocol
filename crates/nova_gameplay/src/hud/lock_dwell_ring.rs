//! The lock-on acquisition dwell ring (task 20260717-004302): a smooth radial
//! arc that fills clockwise around the PENDING target while the radar dwell
//! charges (mechanic in input/targeting.rs, 20260708-165703), and vanishes the
//! instant the lock snaps (the `LockOn` SFX is the audible half of the same
//! beat). nova's first [`UiMaterial`]: a trivial WGSL fragment
//! (`assets/shaders/lock_dwell_ring.wgsl`) driven by a `progress` uniform.
//!
//! A thin consumer of the [`screen_indicator`](mod@super::screen_indicator)
//! widget: one ring node whose anchor is driven to
//! [`RadarState::dwell_target`] each frame (so it rides the pending candidate,
//! which can differ from the still-committed lock during a re-designation),
//! and whose material `progress` tracks [`RadarState::dwell_fill`]. The widget
//! shows/hides and sizes the node from the anchor for free; the layer
//! spawns/despawns with the player ship via the hud/mod.rs observers.

use bevy::{
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
    ui_render::prelude::{MaterialNode, UiMaterial, UiMaterialPlugin},
};

use crate::prelude::*;

/// On-screen diameter (px) of the ring, sized to sit as a tight halo around
/// the pending target's reticle.
const RING_PX: f32 = 39.2;

/// Inner radius of the annulus in normalized node units (outer edge = 1.0):
/// a thin band near the rim.
const RING_INNER: f32 = 0.74;

/// Anti-alias / edge softness of the band and the leading fill edge.
const RING_SOFTNESS: f32 = 0.05;

/// Acquiring accent: a near-white ring that reads as "scanning / charging"
/// against the coloured lock reticles it fills toward. A feel knob (see the
/// arc doc).
const RING_COLOR: LinearRgba = LinearRgba::new(1.0, 1.0, 1.0, 0.9);

pub mod prelude {
    pub use super::{
        lock_dwell_ring_hud, LockDwellRingHudMarker, LockDwellRingHudPlugin, LockDwellRingMarker,
        LockDwellRingMaterial,
    };
}

/// The `UiMaterial` backing the ring. One uniform struct matching the WGSL
/// `LockDwellRingMaterial` layout at `@group(1) @binding(0)`.
#[derive(Asset, AsBindGroup, TypePath, Clone, Debug)]
pub struct LockDwellRingMaterial {
    #[uniform(0)]
    pub data: LockDwellRingUniform,
}

/// The packed uniform (straight-alpha colour + fill fraction + band geometry).
#[derive(ShaderType, Clone, Debug)]
pub struct LockDwellRingUniform {
    pub color: LinearRgba,
    pub progress: f32,
    pub inner: f32,
    pub softness: f32,
}

impl Default for LockDwellRingMaterial {
    fn default() -> Self {
        Self {
            data: LockDwellRingUniform {
                color: RING_COLOR,
                progress: 0.0,
                inner: RING_INNER,
                softness: RING_SOFTNESS,
            },
        }
    }
}

impl UiMaterial for LockDwellRingMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/lock_dwell_ring.wgsl".into()
    }
}

/// Marker for the full-screen ring layer (the root the HUD setup spawns).
#[derive(Component, Debug, Clone, Reflect)]
pub struct LockDwellRingHudMarker;

/// Marker for the single ring node whose anchor + material the driver updates.
#[derive(Component, Debug, Clone, Reflect)]
pub struct LockDwellRingMarker;

/// UI bundle for the ring layer: a full-screen click-through container holding
/// the one ring node (a [`screen_indicator`] carrying the [`MaterialNode`]).
/// The `material` handle is created by the setup observer so its `progress`
/// can be driven per frame.
pub fn lock_dwell_ring_hud(material: Handle<LockDwellRingMaterial>) -> impl Bundle {
    (
        Name::new("LockDwellRingHUD"),
        LockDwellRingHudMarker,
        screen_indicator_layer(),
        children![(
            Name::new("LockDwellRing"),
            LockDwellRingMarker,
            screen_indicator(ScreenIndicatorConfig {
                anchor: None,
                size: ScreenIndicatorSize::Fixed(Vec2::splat(RING_PX)),
                offset: Vec2::ZERO,
                offscreen: ScreenIndicatorOffscreen::Hide,
            }),
            MaterialNode(material),
        )],
    )
}

#[derive(Default)]
pub struct LockDwellRingHudPlugin;

impl Plugin for LockDwellRingHudPlugin {
    fn build(&self, app: &mut App) {
        debug!("LockDwellRingHudPlugin: build");

        app.add_plugins(UiMaterialPlugin::<LockDwellRingMaterial>::default());
        app.register_type::<LockDwellRingHudMarker>();
        app.register_type::<LockDwellRingMarker>();
        app.add_systems(Update, drive_lock_dwell_ring.in_set(super::NovaHudSystems));
    }
}

/// Point the ring at the pending dwell target and fill it to the dwell
/// fraction while a dwell is CHARGING; clear the anchor (the widget hides the
/// node) otherwise. Runs every frame off the player [`RadarState`], which only
/// exists while the radar gesture is held - so with no gesture the ring is
/// hidden for free.
fn drive_lock_dwell_ring(
    q_player: Query<&RadarState, With<PlayerSpaceshipMarker>>,
    mut q_ring: Query<
        (
            &mut ScreenIndicatorAnchor,
            &MaterialNode<LockDwellRingMaterial>,
        ),
        With<LockDwellRingMarker>,
    >,
    mut materials: ResMut<Assets<LockDwellRingMaterial>>,
) {
    let dwell = q_player
        .iter()
        .next()
        .filter(|radar| radar.is_dwelling())
        .and_then(|radar| {
            radar
                .dwell_target
                .map(|target| (target, radar.dwell_fill()))
        });

    for (mut anchor, material) in &mut q_ring {
        match dwell {
            Some((target, fill)) => {
                let want = Some(ScreenIndicatorAnchorKind::Entity(target));
                if anchor.0 != want {
                    anchor.0 = want;
                }
                if let Some(mut material) = materials.get_mut(&material.0) {
                    material.data.progress = fill;
                }
            }
            None => {
                if anchor.0.is_some() {
                    anchor.0 = None;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::{asset::AssetPlugin, ecs::system::RunSystemOnce};

    use super::*;

    /// A headless app with the asset system (but NOT the render-app
    /// `UiMaterialPlugin`, which needs a GPU): enough to drive the ring's
    /// anchor and mutate the material asset. Returns (app, player, ring,
    /// material handle).
    fn ring_app() -> (App, Entity, Entity, Handle<LockDwellRingMaterial>) {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<LockDwellRingMaterial>();

        let handle = app
            .world_mut()
            .resource_mut::<Assets<LockDwellRingMaterial>>()
            .add(LockDwellRingMaterial::default());
        let ring = app
            .world_mut()
            .spawn((
                LockDwellRingMarker,
                ScreenIndicatorAnchor(None),
                MaterialNode(handle.clone()),
            ))
            .id();
        let player = app
            .world_mut()
            .spawn((PlayerSpaceshipMarker, RadarState::default()))
            .id();
        (app, player, ring, handle)
    }

    fn anchor(app: &App, ring: Entity) -> Option<ScreenIndicatorAnchorKind> {
        app.world().get::<ScreenIndicatorAnchor>(ring).unwrap().0
    }

    fn progress(app: &App, handle: &Handle<LockDwellRingMaterial>) -> f32 {
        app.world()
            .resource::<Assets<LockDwellRingMaterial>>()
            .get(handle)
            .unwrap()
            .data
            .progress
    }

    fn set_radar(app: &mut App, player: Entity, radar: RadarState) {
        *app.world_mut().get_mut::<RadarState>(player).unwrap() = radar;
    }

    #[test]
    fn ring_anchors_the_pending_target_and_fills_while_dwelling() {
        let (mut app, player, ring, handle) = ring_app();
        let target = app.world_mut().spawn_empty().id();
        set_radar(
            &mut app,
            player,
            RadarState {
                dwell_target: Some(target),
                dwell_secs: 0.5,
                dwell_needed: 1.0,
                ..default()
            },
        );

        app.world_mut()
            .run_system_once(drive_lock_dwell_ring)
            .unwrap();

        assert_eq!(
            anchor(&app, ring),
            Some(ScreenIndicatorAnchorKind::Entity(target)),
            "the ring anchors the pending dwell target"
        );
        assert!(
            (progress(&app, &handle) - 0.5).abs() < 1e-6,
            "the material fill tracks the dwell fraction"
        );
    }

    #[test]
    fn ring_hides_when_no_dwell_is_charging() {
        let (mut app, player, ring, _) = ring_app();
        let target = app.world_mut().spawn_empty().id();

        // No gesture at all (RadarState default: dwell_needed 0): hidden.
        app.world_mut()
            .run_system_once(drive_lock_dwell_ring)
            .unwrap();
        assert_eq!(anchor(&app, ring), None, "no dwell -> no anchor -> hidden");

        // A COMPLETED dwell (secs >= needed) also reads as not charging: the
        // ring hides the instant the lock snaps.
        set_radar(
            &mut app,
            player,
            RadarState {
                dwell_target: Some(target),
                dwell_secs: 1.0,
                dwell_needed: 1.0,
                ..default()
            },
        );
        app.world_mut()
            .run_system_once(drive_lock_dwell_ring)
            .unwrap();
        assert_eq!(
            anchor(&app, ring),
            None,
            "a completed dwell hides the ring (the snap)"
        );
    }

    #[test]
    fn ring_follows_a_mid_dwell_re_designation() {
        // The ring tracks the PENDING candidate, which during a re-designation
        // differs from the still-committed lock - it shows where the NEW lock
        // is charging, not the old one.
        let (mut app, player, ring, _) = ring_app();
        let first = app.world_mut().spawn_empty().id();
        let second = app.world_mut().spawn_empty().id();

        set_radar(
            &mut app,
            player,
            RadarState {
                dwell_target: Some(first),
                dwell_secs: 0.2,
                dwell_needed: 1.0,
                ..default()
            },
        );
        app.world_mut()
            .run_system_once(drive_lock_dwell_ring)
            .unwrap();
        assert_eq!(
            anchor(&app, ring),
            Some(ScreenIndicatorAnchorKind::Entity(first))
        );

        set_radar(
            &mut app,
            player,
            RadarState {
                dwell_target: Some(second),
                dwell_secs: 0.1,
                dwell_needed: 1.0,
                ..default()
            },
        );
        app.world_mut()
            .run_system_once(drive_lock_dwell_ring)
            .unwrap();
        assert_eq!(
            anchor(&app, ring),
            Some(ScreenIndicatorAnchorKind::Entity(second)),
            "the ring moved to the new pending candidate"
        );
    }
}
