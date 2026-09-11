//! The lock-on acquisition dwell ring: a smooth radial
//! arc that fills clockwise around the PENDING target while the radar dwell
//! charges (mechanic in input/targeting/radar.rs), and vanishes the
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
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;

use crate::prelude::*;

/// Floor on the ring's on-screen diameter (px): the halo a reticle at its own
/// floor gets, and the size a dwell on a target carrying no reticle yet draws
/// at. The number IS the old fixed size, so a distant contact's ring is
/// unchanged.
const RING_PX: f32 = 39.2;

/// The ring's diameter as a fraction of the live reticle it haloes.
///
/// The ring was a fixed 39.2 px - exactly 0.98 of the travel crosshair's 40 px
/// floor - which is a tight halo only while the crosshair sits AT that floor.
/// The crosshairs track apparent size, so on a close carrier the reticle is
/// hundreds of pixels across and the fixed ring is a dot lost inside it.
const RING_RETICLE_SCALE: f32 = 0.98;

/// Inner radius of the annulus in normalized node units (outer edge = 1.0):
/// a thin band near the rim.
const RING_INNER: f32 = 0.74;

/// Anti-alias / edge softness of the band and the leading fill edge.
const RING_SOFTNESS: f32 = 0.05;

/// Acquiring accent: a near-white ring that reads as "scanning / charging"
/// against the coloured lock reticles it fills toward. A feel knob (see the
/// arc doc).
const RING_COLOR: LinearRgba = LinearRgba::new(1.0, 1.0, 1.0, 0.9);

/// The `lock_dwell_ring_hud` spawner, the ring marker and material, and `LockDwellRingHudPlugin`.
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
    /// The packed uniform bound at `@group(1) @binding(0)`.
    #[uniform(0)]
    pub data: LockDwellRingUniform,
}

/// The packed uniform (straight-alpha colour + fill fraction + band geometry).
#[derive(ShaderType, Clone, Debug)]
pub struct LockDwellRingUniform {
    /// Straight-alpha ring colour.
    pub color: LinearRgba,
    /// Fill fraction, 0..1, that the ring sweeps clockwise as the lock-on dwell charges.
    pub progress: f32,
    /// Inner radius of the ring band, as a fraction of the quad.
    pub inner: f32,
    /// Edge softness (antialias width) of the band.
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
                // Re-sized every frame from the live reticle on the dwell
                // target; this is the floor it starts and falls back to.
                size: ScreenIndicatorSize::Fixed(Vec2::splat(RING_PX)),
                offset: Vec2::ZERO,
                offscreen: ScreenIndicatorOffscreen::Hide,
            }),
            MaterialNode(material),
        )],
    )
}

/// Drives the lock-dwell ring: the shader ring that fills over the pending
/// dwell target while a radar gesture is charging.
/// Adds the [`LockDwellRingMaterial`] UI-material plugin, registers the ring
/// markers, and runs `drive_lock_dwell_ring` in Update within
/// [`super::NovaHudSystems`].
#[derive(Default)]
pub struct LockDwellRingHudPlugin;

impl Plugin for LockDwellRingHudPlugin {
    fn build(&self, app: &mut App) {
        trace!("LockDwellRingHudPlugin: build");

        app.add_plugins(UiMaterialPlugin::<LockDwellRingMaterial>::default());
        app.register_type::<LockDwellRingHudMarker>();
        app.register_type::<LockDwellRingMarker>();
        app.add_systems(Update, drive_lock_dwell_ring.in_set(super::NovaHudSystems));
    }
}

/// The widest live reticle already drawn on `target`, in logical pixels.
///
/// The dwell ring haloes whatever the player can see on the pending contact -
/// the travel crosshair, the combat reticle, or nothing at all during a fresh
/// sweep. `ComputedNode::size` is PHYSICAL, so it is divided back to the
/// logical pixels the indicator writes as `Val::Px`.
fn reticle_px_on(target: Entity, reticles: &[(&ScreenIndicatorAnchor, &ComputedNode)]) -> f32 {
    reticles
        .iter()
        .filter(|(anchor, _)| ***anchor == Some(ScreenIndicatorAnchorKind::Entity(target)))
        .map(|(_, computed)| {
            let size = computed.size() * computed.inverse_scale_factor();
            size.x.max(size.y)
        })
        .fold(0.0f32, f32::max)
}

/// Point the ring at the pending dwell target, size it to the reticle already
/// on that target, and fill it to the dwell fraction while a dwell is
/// CHARGING; clear the anchor (the widget hides the node) otherwise. Runs
/// every frame off the player [`RadarState`], which only exists while the radar
/// gesture is held - so with no gesture the ring is hidden for free.
fn drive_lock_dwell_ring(
    q_player: Query<&RadarState, With<PlayerSpaceshipMarker>>,
    // `Without` the ring itself: the reticles are read while the ring's own
    // anchor is written, and the two queries must be provably disjoint.
    q_travel: Query<
        (&ScreenIndicatorAnchor, &ComputedNode),
        (With<TravelCrosshairMarker>, Without<LockDwellRingMarker>),
    >,
    q_combat: Query<
        (&ScreenIndicatorAnchor, &ComputedNode),
        (
            With<TorpedoTargetReticleMarker>,
            Without<LockDwellRingMarker>,
        ),
    >,
    mut q_ring: Query<
        (
            &mut ScreenIndicatorAnchor,
            &mut ScreenIndicatorSize,
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

    let reticles: Vec<(&ScreenIndicatorAnchor, &ComputedNode)> =
        q_travel.iter().chain(q_combat.iter()).collect();

    for (mut anchor, mut size, material) in &mut q_ring {
        match dwell {
            Some((target, fill)) => {
                let want = Some(ScreenIndicatorAnchorKind::Entity(target));
                if anchor.0 != want {
                    anchor.0 = want;
                }
                let diameter = (reticle_px_on(target, &reticles) * RING_RETICLE_SCALE).max(RING_PX);
                size.set_if_neq(ScreenIndicatorSize::Fixed(Vec2::splat(diameter)));
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
                ScreenIndicatorSize::Fixed(Vec2::splat(RING_PX)),
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
    /// The ring haloes whatever reticle is on the dwell target, so it stays a
    /// tight band at any target size. A fixed 39 px ring is a dot lost inside
    /// the 300 px crosshair a close carrier wears.
    #[test]
    fn the_dwell_ring_haloes_the_live_reticle() {
        let ring_diameter = |reticle_px: Option<f32>| -> f32 {
            let (mut app, player, ring, _) = ring_app();
            let target = app.world_mut().spawn_empty().id();
            if let Some(px) = reticle_px {
                app.world_mut().spawn((
                    TravelCrosshairMarker,
                    ScreenIndicatorAnchor(Some(ScreenIndicatorAnchorKind::Entity(target))),
                    ComputedNode {
                        size: Vec2::splat(px),
                        ..ComputedNode::DEFAULT
                    },
                ));
            }
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
            match *app.world().get::<ScreenIndicatorSize>(ring).unwrap() {
                ScreenIndicatorSize::Fixed(size) => size.x,
                other => panic!("the ring is sized in pixels, got {other:?}"),
            }
        };

        // A close carrier's crosshair: the ring tracks it instead of sitting
        // inside it as a dot.
        assert!((ring_diameter(Some(300.0)) - 300.0 * RING_RETICLE_SCALE).abs() < 1e-3);
        // A reticle at its own floor keeps the ring at the size it has always
        // drawn at...
        assert_eq!(ring_diameter(Some(40.0)), RING_PX);
        // ...and a fresh sweep with no reticle yet floors there too.
        assert_eq!(ring_diameter(None), RING_PX);
    }
}
