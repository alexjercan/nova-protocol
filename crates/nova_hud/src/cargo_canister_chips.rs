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
//! The chip shows only while the canister is in the reach of one of the
//! player ship's cargo intakes, as the last fixed intake pass published it in
//! [`CargoPickupReadiness`] (`in_reach`: in front of the face, within its
//! `pull_range`), and hides off-screen. The label, the pickup sight's line and
//! the pull share that one rule: a canister is a pickup near the ship, not a
//! destination, and the viewport edges stay reserved for threats and the
//! active objective. The layer, anchor and despawn are
//! [`anchored_chip`](super::anchored_chip)'s; the tag's own paint is not.
//!
//! Chrome tier: a pickup tag, not a flight instrument.

use bevy::{ecs::entity::EntityHashSet, prelude::*};
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;
use nova_ui::widget::{metrics, ThemedRadius};

use super::{anchored_chip::prelude::*, screen_indicator::prelude::*};

/// The canister chip layer marker and `CargoCanisterChipsHudPlugin`.
pub mod prelude {
    pub use super::{CargoCanisterChipHudMarker, CargoCanisterChipsHudPlugin};
}

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
fn cargo_canister_chip_text(items: &GameItems, canister: &CargoCanister) -> (String, String) {
    let mut stacks = canister.stacks();
    let label = match (stacks.next(), stacks.next()) {
        (Some((item, count)), None) => format!("{count} {}", items.design(item).name),
        _ => "Mixed cargo".to_string(),
    };
    (label, kg_text(u64::from(canister.total_mass_g(items))))
}

/// UI bundle for one canister's chip layer, spawned hidden: the reach pass
/// anchors it once the canister is in the reach of the player ship's
/// intakes.
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
            // A box like every HUD chip, so it takes their themed corner.
            ThemedRadius::control(),
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
                        // A HUD chip's padding: the bar is the tag's edge, the
                        // lines its content.
                        padding: UiRect::axes(Val::Px(metrics::PAD_X), Val::Px(metrics::GAP_S)),
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
fn setup_cargo_canister_chip(add: On<Add<CargoCanister>>, mut commands: Commands) {
    commands.spawn(cargo_canister_chip_hud(add.entity));
}

/// Anchor each canister chip while a pair the last fixed intake pass
/// published puts it in the reach of an intake on the player ship, and write
/// what the canister holds. Without a player ship every chip hides.
fn update_cargo_canister_chips(
    q_chips: Query<&AnchoredChipTarget, With<CargoCanisterChipHudMarker>>,
    mut q_nodes: Query<(&mut ScreenIndicatorAnchor, &ChildOf), With<AnchoredChipNodeMarker>>,
    mut q_texts: Query<
        (&mut Text, &ChildOf, Has<CargoCanisterChipMassMarker>),
        With<AnchoredChipLabelMarker>,
    >,
    q_lines: Query<&ChildOf>,
    q_canisters: Query<&CargoCanister>,
    q_player: Query<Entity, With<PlayerSpaceshipMarker>>,
    readiness: Res<CargoPickupReadiness>,
    items: Res<GameItems>,
) {
    let in_reach: EntityHashSet = q_player
        .iter()
        .next()
        .map(|ship| {
            readiness
                .pairs
                .iter()
                .filter(|pair| pair.ship == ship && pair.in_reach)
                .map(|pair| pair.canister)
                .collect()
        })
        .unwrap_or_default();
    for (mut anchor, ChildOf(layer)) in &mut q_nodes {
        let Ok(target) = q_chips.get(*layer) else {
            continue;
        };
        let shown = q_canisters.contains(**target) && in_reach.contains(&**target);
        let wanted = shown.then_some(ScreenIndicatorAnchorKind::Entity(**target));
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
        let Ok(canister) = q_canisters.get(**target) else {
            continue;
        };
        let (label, mass) = cargo_canister_chip_text(&items, canister);
        let next = if is_mass { mass } else { label };
        if **text != next {
            **text = next;
        }
    }
}

/// Draws one content tag per [`CargoCanister`] in the reach of the player
/// ship's intakes, reading its stacks and mass (Chrome tier). Adds the spawn
/// and despawn observers and runs the reach and label pass in Update within
/// [`super::NovaHudSystems`], once the mod merge has published [`GameItems`].
/// Reads [`CargoPickupReadiness`], which `CargoIntakeSectionPlugin` inits.
#[derive(Default)]
pub struct CargoCanisterChipsHudPlugin;

impl Plugin for CargoCanisterChipsHudPlugin {
    fn build(&self, app: &mut App) {
        trace!("CargoCanisterChipsHudPlugin: build");

        app.add_observer(setup_cargo_canister_chip);
        app.add_observer(despawn_anchored_chips::<CargoCanister, CargoCanisterChipHudMarker>);
        // NovaHudSystems runs from the first boot frame; the catalog lands
        // only at the merge, and never on a refused base game.
        app.add_systems(
            Update,
            update_cargo_canister_chips
                .run_if(resource_exists::<GameItems>)
                .in_set(super::NovaHudSystems),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Before the mod merge publishes `GameItems`, Update skips the pass and
    /// the app does not panic; once the catalog lands, the label line reads
    /// the canister's authored item name.
    #[test]
    fn canister_chips_wait_for_the_merged_item_catalog() {
        let fixture = ItemDesign {
            id: ItemDesignId::from("fixture_core"),
            name: "Fixture core".to_string(),
            about: "A catalog-only item for tests.".to_string(),
            category: ItemCategoryType::Parts,
            mass_g: 5_000,
            price_lot: 1,
            ask_cr: 60,
            bid_cr: 45,
        };
        let catalog = GameItems::new(
            nova_gameplay::test_support::test_items()
                .iter()
                .cloned()
                .chain([fixture]),
        );

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<CargoPickupReadiness>();
        app.add_plugins(CargoCanisterChipsHudPlugin);

        app.world_mut().spawn((
            CargoCanister::new(&catalog, &ItemDesignId::from("fixture_core"), 2),
            CargoCanisterRuntimeId(0),
            GlobalTransform::default(),
        ));

        let label_of = |app: &mut App, want_mass: bool| -> Entity {
            app.world_mut()
                .query_filtered::<(Entity, Has<CargoCanisterChipMassMarker>), With<AnchoredChipLabelMarker>>()
                .iter(app.world())
                .find_map(|(entity, is_mass)| (is_mass == want_mass).then_some(entity))
                .expect("chip label exists")
        };
        let label = label_of(&mut app, false);
        let mass = label_of(&mut app, true);
        let initial = app.world().get::<Text>(label).unwrap().0.clone();

        app.update();
        app.update();
        assert_eq!(
            app.world().get::<Text>(label).unwrap().0,
            initial,
            "without GameItems the update pass must not touch the label"
        );

        app.insert_resource(catalog);
        app.update();
        assert_eq!(app.world().get::<Text>(label).unwrap().0, "2 Fixture core");
        assert_eq!(app.world().get::<Text>(mass).unwrap().0, kg_text(10_000));
    }

    /// The chip shows for every canister an intake on the player ship has in
    /// its reach, and for none out of reach or in another ship's reach.
    #[test]
    fn a_label_shows_for_every_canister_in_reach_and_none_out_of_reach() {
        let items = nova_gameplay::test_support::test_items();
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<CargoPickupReadiness>();
        app.insert_resource(items.clone());
        app.add_plugins(CargoCanisterChipsHudPlugin);

        let player = app.world_mut().spawn(PlayerSpaceshipMarker).id();
        let intake = app.world_mut().spawn_empty().id();
        let other_ship = app.world_mut().spawn_empty().id();
        let canister = |app: &mut App| {
            app.world_mut()
                .spawn((
                    CargoCanister::new(&items, &ItemRoleType::HullPlate.into(), 1),
                    CargoCanisterRuntimeId(0),
                    GlobalTransform::default(),
                ))
                .id()
        };
        let reached = [canister(&mut app), canister(&mut app), canister(&mut app)];
        let out = canister(&mut app);
        let others = canister(&mut app);
        let pair = |ship, canister, in_reach| CargoPickupPair {
            ship,
            intake,
            canister,
            in_reach,
            ready: false,
        };
        let mut pairs: Vec<CargoPickupPair> = reached
            .iter()
            .map(|&canister| pair(player, canister, true))
            .collect();
        pairs.push(pair(player, out, false));
        pairs.push(pair(other_ship, others, true));
        app.world_mut().resource_mut::<CargoPickupReadiness>().pairs = pairs;
        app.update();

        let world = app.world_mut();
        let shown: Vec<Entity> = world
            .query_filtered::<(&ScreenIndicatorAnchor, &ChildOf), With<AnchoredChipNodeMarker>>()
            .iter(world)
            .filter_map(|(anchor, &ChildOf(layer))| {
                let target = world.get::<AnchoredChipTarget>(layer)?;
                (**anchor == Some(ScreenIndicatorAnchorKind::Entity(**target))).then_some(**target)
            })
            .collect();
        let mut expected = reached.to_vec();
        expected.sort();
        let mut shown = shown;
        shown.sort();
        assert_eq!(shown, expected, "only the canisters in reach are labelled");
    }
}
