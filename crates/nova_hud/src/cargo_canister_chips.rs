//! Cargo canister HUD chips: one screen-projected content tag per
//! [`CargoCanister`], reading what it holds ("4 Hull plate") and its total
//! mass in a muted line under that, so a jettisoned or drifting stack is
//! found by eye and not only by its model.
//!
//! A canister with more than one item stack has no single name to show, so
//! the label reads "Mixed cargo" rather than naming only the first stack -
//! silently dropping every stack past the first would misreport what the
//! canister actually holds.
//!
//! The tag's look - dark scrim, amber accent bar, glow name, muted mass - follows
//! `examples/screenshots/loop_intake_compare.rs` (`spawn_tag`) rather than the
//! shared amber pill:
//! this is cargo content over a canister's own model, not a HUD readout.
//!
//! The chip shows only while the player ship is within [`CARGO_TAG_RANGE`] of
//! the canister and hides off-screen: a canister is a pickup near the ship,
//! not a destination, and the viewport edges stay reserved for threats and the
//! active objective. The layer, anchor and despawn are
//! [`anchored_chip`](super::anchored_chip)'s; the tag's own paint is not.
//!
//! Chrome tier: a pickup tag, not a flight instrument.

use bevy::prelude::*;
use nova_events::units::prelude::*;
use nova_gameplay::prelude::*;

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

const LABEL_FONT_PX: f32 = 15.0;

/// Font size of the muted mass line under the label.
const MASS_FONT_PX: f32 = 12.0;

/// The tag's dark scrim, `loop_intake_compare`'s `spawn_tag` background.
const SCRIM_COLOR: Color = Color::srgba(0.02, 0.03, 0.04, 0.8);

/// The accent bar's amber, `loop_intake_compare`'s `spawn_tag` bar colour.
const BAR_COLOR: Color = Color::srgb(0.95, 0.62, 0.15);
/// Width of the accent bar, logical pixels.
const BAR_WIDTH_PX: f32 = 4.0;

/// The item label's glow colour, `loop_intake_compare`'s cargo-name colour.
const LABEL_COLOR: Color = Color::srgb(0.45, 0.9, 0.98);
/// The mass line's muted colour, `loop_intake_compare`'s mass-line colour.
const MASS_COLOR: Color = Color::srgb(0.8, 0.82, 0.84);

/// Marker for one canister chip layer (one per canister). The family tag every
/// canister chip system is gated on.
#[derive(Component, Debug, Clone, Reflect)]
pub struct CargoCanisterChipHudMarker;

/// Marker for a canister chip's mass line - the muted leaf under the item
/// label. A separate marker from [`AnchoredChipLabelMarker`] (the item label
/// itself carries that one) so the update pass can write each line without
/// mistaking one for the other.
#[derive(Component, Debug, Clone, Reflect)]
struct CargoCanisterChipMassMarker;

/// What a canister chip reads: the item line ("4 Hull plate" or "Mixed cargo"
/// once it holds more than one stack) and its total mass.
fn cargo_canister_chip_text(canister: &CargoCanister) -> (String, String) {
    let mut stacks = canister.stacks();
    let label = match (stacks.next(), stacks.next()) {
        (Some((item, count)), None) => format!("{count} {}", item.label()),
        _ => "Mixed cargo".to_string(),
    };
    (label, kg_text(u64::from(canister.total_mass_g())))
}

/// UI bundle for one canister's chip layer, spawned hidden: the range pass
/// anchors it once the player ship is near.
///
/// `loop_intake_compare`'s `spawn_tag` shape: a dark scrim row holding a thin
/// amber accent bar beside a padded column of the two text lines.
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
                Node {
                    display: Display::Flex,
                    align_items: AlignItems::Stretch,
                    ..default()
                },
            ),
            CHIP_CLEARANCE,
            BackgroundColor(SCRIM_COLOR),
            children![
                (
                    Name::new("CargoCanisterChipBar"),
                    Node {
                        width: Val::Px(BAR_WIDTH_PX),
                        ..default()
                    },
                    BackgroundColor(BAR_COLOR),
                ),
                (
                    Name::new("CargoCanisterChipLines"),
                    Node {
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::axes(Val::Px(7.0), Val::Px(3.0)),
                        ..default()
                    },
                    children![
                        (
                            Name::new("CargoCanisterChipLabel"),
                            anchored_chip_label(LABEL_FONT_PX, LABEL_COLOR, ()),
                        ),
                        (
                            Name::new("CargoCanisterChipMass"),
                            CargoCanisterChipMassMarker,
                            anchored_chip_label(MASS_FONT_PX, MASS_COLOR, ()),
                        ),
                    ],
                ),
            ],
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
    mut q_texts: Query<
        (&mut Text, &ChildOf, Has<CargoCanisterChipMassMarker>),
        With<AnchoredChipLabelMarker>,
    >,
    q_lines: Query<&ChildOf>,
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
    for (mut text, ChildOf(lines), is_mass) in &mut q_texts {
        let Ok(ChildOf(node)) = q_lines.get(*lines) else {
            continue;
        };
        let Ok((_, ChildOf(layer))) = q_nodes.get(*node) else {
            continue;
        };
        let Ok(target) = q_chips.get(*layer) else {
            continue;
        };
        let Ok((canister, _)) = q_canisters.get(**target) else {
            continue;
        };
        let (label, mass) = cargo_canister_chip_text(canister);
        let next = if is_mass { mass } else { label };
        if **text != next {
            **text = next;
        }
    }
}

/// Draws one content tag per [`CargoCanister`] near the player ship, reading
/// its stacks and mass (Chrome tier). Adds the spawn and despawn observers and
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
