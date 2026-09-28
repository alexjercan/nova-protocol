//! Cargo canister HUD chips: one screen-projected amber chip per
//! [`CargoCanister`], reading what it holds ("4 Hull plate"), so a jettisoned
//! or drifting stack is found by eye and not only by its model.
//!
//! The chip shows only while the player ship is within [`CARGO_TAG_RANGE`] of
//! the canister and hides off-screen: a canister is a pickup near the ship,
//! not a destination, and the viewport edges stay reserved for threats and the
//! active objective. The shape - layer, pill, label leaf, despawn - is
//! [`anchored_chip`](super::anchored_chip)'s.
//!
//! Chrome tier: a pickup tag, not a flight instrument.

use bevy::prelude::*;
use nova_events::units::prelude::*;
use nova_gameplay::prelude::*;
use nova_ui::hud::{chip_node, chip_paint, ChipText, ChipTone};

use super::{anchored_chip::prelude::*, screen_indicator::prelude::*};

/// The canister chip layer marker, the tag range and
/// `CargoCanisterChipsHudPlugin`.
pub mod prelude {
    pub use super::{CargoCanisterChipHudMarker, CargoCanisterChipsHudPlugin, CARGO_TAG_RANGE};
}

/// How near the player ship must be for a canister's chip to show.
pub const CARGO_TAG_RANGE: Meters = Meters(100.0);

/// The chip stands off the canister so the label never sits on the model.
const CHIP_CLEARANCE: ScreenIndicatorClearance = ScreenIndicatorClearance {
    direction: Vec2::NEG_Y,
    gap_px: 8.0,
    min_px: 20.0,
};

const LABEL_FONT_PX: f32 = 12.0;

/// Marker for one canister chip layer (one per canister). The family tag every
/// canister chip system is gated on.
#[derive(Component, Debug, Clone, Reflect)]
pub struct CargoCanisterChipHudMarker;

/// UI bundle for one canister's chip layer, spawned hidden: the range pass
/// anchors it once the player ship is near.
fn cargo_canister_chip_hud(canister: Entity) -> impl Bundle {
    (
        Name::new("CargoCanisterChipHUD"),
        CargoCanisterChipHudMarker,
        anchored_chip_layer(canister),
        children![(
            Name::new("CargoCanisterChipUI"),
            AnchoredChipNodeMarker,
            screen_indicator_node(
                ScreenIndicatorConfig {
                    anchor: None,
                    size: ScreenIndicatorSize::Content,
                    offset: Vec2::ZERO,
                    offscreen: ScreenIndicatorOffscreen::Hide,
                },
                chip_node(),
            ),
            CHIP_CLEARANCE,
            chip_paint(ChipTone::Amber),
            children![(
                Name::new("CargoCanisterChipLabel"),
                anchored_chip_label(LABEL_FONT_PX, Color::NONE, ChipText::value(ChipTone::Amber)),
            )],
        )],
    )
}

/// Every canister grows its chip the moment it spawns.
fn setup_cargo_canister_chip(add: On<Add, CargoCanister>, mut commands: Commands) {
    commands.spawn(cargo_canister_chip_hud(add.entity));
}

/// Anchor each canister chip while the player ship is within
/// [`CARGO_TAG_RANGE`], and write what the canister holds. Without a player
/// ship every chip hides.
fn update_cargo_canister_chips(
    q_chips: Query<&AnchoredChipTarget, With<CargoCanisterChipHudMarker>>,
    mut q_nodes: Query<(&mut ScreenIndicatorAnchor, &ChildOf), With<AnchoredChipNodeMarker>>,
    mut q_labels: Query<(&mut Text, &ChildOf), With<AnchoredChipLabelMarker>>,
    q_canisters: Query<(&CargoCanister, &GlobalTransform)>,
    q_player: Query<&GlobalTransform, With<PlayerSpaceshipMarker>>,
) {
    let player = q_player.iter().next().map(GlobalTransform::translation);
    let range = CARGO_TAG_RANGE.to_engine();
    for (mut anchor, ChildOf(layer)) in &mut q_nodes {
        let Ok(target) = q_chips.get(*layer) else {
            continue;
        };
        let near = q_canisters.get(**target).is_ok_and(|(_, transform)| {
            player.is_some_and(|player| player.distance(transform.translation()) <= range)
        });
        let wanted = near.then_some(ScreenIndicatorAnchorKind::Entity(**target));
        if **anchor != wanted {
            **anchor = wanted;
        }
    }
    for (mut text, ChildOf(node)) in &mut q_labels {
        let Ok((_, ChildOf(layer))) = q_nodes.get(*node) else {
            continue;
        };
        let Ok(target) = q_chips.get(*layer) else {
            continue;
        };
        let Ok((canister, _)) = q_canisters.get(**target) else {
            continue;
        };
        let next = format!("{} {}", canister.count, canister.item.label());
        if **text != next {
            **text = next;
        }
    }
}

/// Draws one amber chip per [`CargoCanister`] near the player ship, reading
/// its count and item (Chrome tier). Adds the spawn and despawn observers and
/// runs the range and label pass in Update within [`super::NovaHudSystems`].
#[derive(Default)]
pub struct CargoCanisterChipsHudPlugin;

impl Plugin for CargoCanisterChipsHudPlugin {
    fn build(&self, app: &mut App) {
        trace!("CargoCanisterChipsHudPlugin: build");

        app.add_observer(setup_cargo_canister_chip);
        app.add_observer(despawn_anchored_chips::<CargoCanister, CargoCanisterChipHudMarker>);
        app.add_systems(
            Update,
            update_cargo_canister_chips.in_set(super::NovaHudSystems),
        );
    }
}
