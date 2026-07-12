//! Nav beacon HUD chips (task 20260712-093044): one screen-projected chip
//! per [`BeaconMarker`] entity - the beacon's label plus live distance to
//! the player ship - with the indicator widget's `ClampToEdge` path, so an
//! off-screen beacon's chip pins to the viewport edge and its chevron
//! points at it. The chip IS the game's direction-to-objective cue; the
//! scenario only spawns a beacon and the HUD does the rest.
//!
//! Chrome tier: beacons are guidance, not flight instruments.

use bevy::prelude::*;

use super::{screen_indicator::prelude::*, HudTier, NAV_CYAN};
use crate::prelude::*;

pub mod prelude {
    pub use super::{
        BeaconChipHudMarker, BeaconChipLabelMarker, BeaconChipTargetEntity, BeaconChipsHudPlugin,
    };
}

/// Chip footprint (px). Wide enough for "BEACON 1 1234m" on one line.
const CHIP_SIZE: Vec2 = Vec2::new(140.0, 16.0);

/// The chip floats above the beacon so the label never sits on the mesh.
const CHIP_OFFSET: Vec2 = Vec2::new(0.0, -28.0);

/// Inset (px) from the viewport edges while clamped. Matches the edge
/// indicators' frame so clamped beacon chips join the same visual ring.
const EDGE_MARGIN_PX: f32 = 30.0;

/// Chevron stroke geometry, the edge-indicator arrow language at chip scale.
const ARROW_PX: f32 = 16.0;
const STROKE_LEN_PX: f32 = 11.0;
const STROKE_THICK_PX: f32 = 2.0;

const LABEL_FONT_PX: f32 = 12.0;

/// Marker for one beacon chip layer (one per beacon).
#[derive(Component, Debug, Clone, Reflect)]
pub struct BeaconChipHudMarker;

/// The beacon entity this chip tracks.
#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct BeaconChipTargetEntity(pub Entity);

/// Marker for the chip's text node.
#[derive(Component, Debug, Clone, Reflect)]
pub struct BeaconChipLabelMarker;

/// UI bundle for one beacon's chip layer.
fn beacon_chip_hud(beacon: Entity) -> impl Bundle {
    (
        Name::new("BeaconChipHUD"),
        BeaconChipHudMarker,
        BeaconChipTargetEntity(beacon),
        HudTier::Chrome,
        screen_indicator_layer(),
        children![(
            Name::new("BeaconChipUI"),
            BeaconChipLabelMarker,
            screen_indicator(ScreenIndicatorConfig {
                anchor: Some(ScreenIndicatorAnchorKind::Entity(beacon)),
                size: ScreenIndicatorSize::Fixed(CHIP_SIZE),
                offset: CHIP_OFFSET,
                offscreen: ScreenIndicatorOffscreen::ClampToEdge {
                    margin_px: EDGE_MARGIN_PX,
                },
            }),
            Text::new(""),
            TextFont::from_font_size(LABEL_FONT_PX),
            TextLayout {
                linebreak: LineBreak::NoWrap,
                ..default()
            },
            TextColor(NAV_CYAN),
            children![beacon_chip_arrow()],
        )],
    )
}

/// An up-pointing chevron the widget rotates toward the beacon while the
/// chip is edge-clamped (the edge-indicator arrow language, chip-sized).
/// Hidden while the beacon is on-screen - the widget owns its visibility.
fn beacon_chip_arrow() -> impl Bundle {
    let stroke = |left: f32, degrees: f32| {
        (
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(left),
                top: Val::Px(ARROW_PX / 2.0 - STROKE_THICK_PX / 2.0),
                width: Val::Px(STROKE_LEN_PX),
                height: Val::Px(STROKE_THICK_PX),
                ..default()
            },
            UiTransform {
                rotation: Rot2::degrees(degrees),
                ..default()
            },
            BackgroundColor(NAV_CYAN),
            Pickable::IGNORE,
        )
    };

    (
        Name::new("BeaconChipArrow"),
        ScreenIndicatorArrowMarker,
        Node {
            position_type: PositionType::Absolute,
            // Park the chevron just above the label text, centered on the
            // chip's anchor point.
            left: Val::Px(-ARROW_PX / 2.0),
            top: Val::Px(-ARROW_PX - 2.0),
            width: Val::Px(ARROW_PX),
            height: Val::Px(ARROW_PX),
            ..default()
        },
        UiTransform::default(),
        Visibility::Hidden,
        Pickable::IGNORE,
        children![
            stroke(-0.5, -45.0),
            stroke(ARROW_PX - STROKE_LEN_PX + 0.5, 45.0),
        ],
    )
}

#[derive(Default)]
pub struct BeaconChipsHudPlugin;

impl Plugin for BeaconChipsHudPlugin {
    fn build(&self, app: &mut App) {
        debug!("BeaconChipsHudPlugin: build");

        app.register_type::<BeaconMarker>();
        app.register_type::<BeaconLabel>();

        app.add_observer(setup_beacon_chip);
        app.add_observer(remove_beacon_chip);
        app.add_systems(
            Update,
            update_beacon_chip_labels.in_set(super::NovaHudSystems),
        );
    }
}

/// Every beacon grows its chip the moment it spawns.
fn setup_beacon_chip(add: On<Add, BeaconMarker>, mut commands: Commands) {
    let beacon = add.entity;
    debug!("setup_beacon_chip: beacon {:?}", beacon);
    commands.spawn(beacon_chip_hud(beacon));
}

/// The chip layer dies with its beacon (despawn action, scenario unload -
/// any removal path).
fn remove_beacon_chip(
    remove: On<Remove, BeaconMarker>,
    mut commands: Commands,
    q_chips: Query<(Entity, &BeaconChipTargetEntity), With<BeaconChipHudMarker>>,
) {
    let beacon = remove.entity;
    for (chip, target) in &q_chips {
        if **target == beacon {
            trace!("remove_beacon_chip: despawning chip {:?}", chip);
            commands.entity(chip).despawn();
        }
    }
}

/// Label text: the beacon's name plus the live distance to the player ship
/// ("BEACON 1  420m"). Without a player (menu ambience, death gap) the
/// label alone shows - the chip is still a valid waypoint tag.
fn update_beacon_chip_labels(
    q_chips: Query<&BeaconChipTargetEntity, With<BeaconChipHudMarker>>,
    mut q_labels: Query<(&mut Text, &ChildOf), With<BeaconChipLabelMarker>>,
    q_beacons: Query<(&BeaconLabel, &GlobalTransform), With<BeaconMarker>>,
    q_player: Query<&GlobalTransform, With<PlayerSpaceshipMarker>>,
) {
    let player = q_player.iter().next();
    for (mut text, ChildOf(layer)) in &mut q_labels {
        let Ok(target) = q_chips.get(*layer) else {
            continue;
        };
        let Ok((label, beacon_transform)) = q_beacons.get(**target) else {
            continue;
        };
        let next = match player {
            Some(player_transform) => {
                let distance = player_transform
                    .translation()
                    .distance(beacon_transform.translation());
                format!("{}  {:.0}m", **label, distance)
            }
            None => (**label).clone(),
        };
        if **text != next {
            **text = next;
        }
    }
}
