//! The torpedo-lock reticle: a screen-projected indicator on the entity the
//! player's aim-assist currently locks (`SpaceshipPlayerTorpedoTargetEntity`).
//!
//! A thin consumer of the [`screen_indicator`](super::screen_indicator)
//! widget: the widget owns projection, sizing and visibility; this module
//! only spawns the reticle and drives its anchor from the lock resource.

use bevy::prelude::*;

use crate::prelude::*;

/// Minimum on-screen size (px) of the target reticle. This is its historical
/// fixed size: the reticle grows to match larger targets but never shrinks
/// below this, so small or distant targets still show a clearly visible,
/// clickable marker.
const MIN_RETICLE_PX: f32 = 32.0;

pub mod prelude {
    pub use super::{
        torpedo_target_hud, TorpedoTargetHudConfig, TorpedoTargetHudMarker, TorpedoTargetHudPlugin,
        TorpedoTargetReticleMarker,
    };
}

/// Marker for the full-screen reticle layer (the root the HUD setup spawns).
#[derive(Component, Debug, Clone, Reflect)]
pub struct TorpedoTargetHudMarker;

/// Marker for the reticle indicator node itself. Public so other HUD pieces
/// (e.g. the locked-target readout) can attach content to it.
#[derive(Component, Debug, Clone, Reflect)]
pub struct TorpedoTargetReticleMarker;

#[derive(Clone, Debug, Default)]
pub struct TorpedoTargetHudConfig {
    pub target_sprite: Handle<Image>,
}

/// UI bundle for the torpedo-lock reticle: a full-screen click-through layer
/// whose child is a screen-projected indicator sized to the locked target's
/// on-screen extent.
pub fn torpedo_target_hud(config: TorpedoTargetHudConfig) -> impl Bundle {
    debug!("torpedo_target_hud: config {:?}", config);

    (
        Name::new("TorpedoTargetHUD"),
        TorpedoTargetHudMarker,
        screen_indicator_layer(),
        children![(
            Name::new("TorpedoTargetReticle"),
            TorpedoTargetReticleMarker,
            screen_indicator(ScreenIndicatorConfig {
                anchor: None,
                size: ScreenIndicatorSize::ApparentSize {
                    min_px: MIN_RETICLE_PX,
                },
                offset: Vec2::ZERO,
                offscreen: ScreenIndicatorOffscreen::Hide,
            }),
            ImageNode::new(config.target_sprite.clone()),
        )],
    )
}

#[derive(Default)]
pub struct TorpedoTargetHudPlugin;

impl Plugin for TorpedoTargetHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            drive_reticle_anchor
                .in_set(super::NovaHudSystems)
                .before(ScreenIndicatorSystems),
        );
    }
}

/// Point the reticle indicator at the current lock; `None` (no lock) hides it
/// via the widget's anchor handling.
fn drive_reticle_anchor(
    res_target: Res<SpaceshipPlayerTorpedoTargetEntity>,
    mut q_reticle: Query<&mut ScreenIndicatorAnchor, With<TorpedoTargetReticleMarker>>,
) {
    for mut anchor in &mut q_reticle {
        **anchor = (**res_target).map(ScreenIndicatorAnchorKind::Entity);
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    #[test]
    fn torpedo_target_hud_spawns_the_reticle_indicator() {
        let mut world = World::new();
        let layer = world
            .spawn(torpedo_target_hud(TorpedoTargetHudConfig::default()))
            .id();

        let children = world
            .entity(layer)
            .get::<Children>()
            .expect("layer has the reticle child");
        assert_eq!(children.len(), 1);
        let reticle = world.entity(children[0]);
        assert!(reticle.contains::<TorpedoTargetReticleMarker>());
        assert!(reticle.contains::<ScreenIndicatorMarker>());
        assert_eq!(
            **reticle.get::<ScreenIndicatorAnchor>().unwrap(),
            None,
            "the reticle starts unanchored (hidden) until a lock exists"
        );
    }

    #[test]
    fn reticle_anchor_follows_the_lock_resource() {
        let mut world = World::new();
        world.insert_resource(SpaceshipPlayerTorpedoTargetEntity(None));
        let reticle = world
            .spawn((
                TorpedoTargetReticleMarker,
                screen_indicator(ScreenIndicatorConfig::default()),
            ))
            .id();

        world.run_system_once(drive_reticle_anchor).unwrap();
        assert_eq!(
            **world
                .entity(reticle)
                .get::<ScreenIndicatorAnchor>()
                .unwrap(),
            None
        );

        let target = world.spawn_empty().id();
        world.insert_resource(SpaceshipPlayerTorpedoTargetEntity(Some(target)));
        world.run_system_once(drive_reticle_anchor).unwrap();
        assert_eq!(
            **world
                .entity(reticle)
                .get::<ScreenIndicatorAnchor>()
                .unwrap(),
            Some(ScreenIndicatorAnchorKind::Entity(target))
        );

        world.insert_resource(SpaceshipPlayerTorpedoTargetEntity(None));
        world.run_system_once(drive_reticle_anchor).unwrap();
        assert_eq!(
            **world
                .entity(reticle)
                .get::<ScreenIndicatorAnchor>()
                .unwrap(),
            None,
            "dropping the lock clears the anchor so the widget hides the reticle"
        );
    }
}
