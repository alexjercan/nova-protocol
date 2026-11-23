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
//! Canisters that crowd one spot on screen share one LIST chip instead: three
//! or more whose chips would touch (closer than the label gap) collapse into
//! one tag at their centre, one row per item kind with the summed count. A
//! cluster that drifts apart keeps its list until the gap grows past a hold
//! margin, so a cluster on the threshold does not flicker. One or two
//! canisters keep their own chips.
//!
//! Chrome tier: a pickup tag, not a flight instrument.

use std::collections::BTreeMap;

use bevy::{
    ecs::entity::{EntityHashMap, EntityHashSet},
    prelude::*,
};
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;
use nova_ui::{
    theme::BORDER_W,
    widget::{metrics, ThemedRadius},
};

use super::{
    anchored_chip::prelude::*,
    screen_indicator::{prelude::*, UiProjection, LABEL_GAP_PX},
    HudTier,
};

/// The canister chip layer markers and `CargoCanisterChipsHudPlugin`.
pub mod prelude {
    pub use super::{
        CargoCanisterChipHudMarker, CargoCanisterChipsHudPlugin, CargoCanisterListHudMarker,
        CargoCanisterListMembers,
    };
}

/// The chip stands off the canister so the label never sits on the model.
const CHIP_CLEARANCE: ScreenIndicatorClearance = ScreenIndicatorClearance {
    direction: Vec2::NEG_Y,
    gap_px: 8.0,
    min_px: 20.0,
};

/// The item label size: the floating chips' label size.
const LABEL_FONT_PX: f32 = metrics::TEXT;

/// Font size of the muted mass line under the label.
const MASS_FONT_PX: f32 = metrics::TEXT_LABEL;

/// The padding round a tag's text lines: the floating chips' tight gap.
const LINES_PAD_PX: f32 = metrics::GAP_S;

/// The tag's dark scrim, `loop_intake_compare`'s `spawn_tag` background.
const SCRIM_COLOR: Color = Color::srgba(0.02, 0.03, 0.04, 0.8);

/// The accent bar's amber, `loop_intake_compare`'s `spawn_tag` bar colour.
const BAR_COLOR: Color = Color::srgb(0.95, 0.62, 0.15);
/// Width of the accent bar, logical pixels: twice the theme hairline.
const BAR_WIDTH_PX: f32 = 2.0 * BORDER_W;

/// The item label's glow colour, `loop_intake_compare`'s cargo-name colour.
const LABEL_COLOR: Color = Color::srgb(0.45, 0.9, 0.98);
/// The mass line's muted colour, `loop_intake_compare`'s mass-line colour.
const MASS_COLOR: Color = Color::srgb(0.8, 0.82, 0.84);

/// The fewest canisters in one on-screen cluster that share a list chip.
/// One or two keep their own chips: a list of two saves no room.
pub(crate) const LIST_MIN_CANISTERS: usize = 3;

/// The most rows a list chip draws. With more item kinds than this, the last
/// row counts the kinds it does not name.
pub(crate) const LIST_MAX_ROWS: usize = 5;

/// How far (px) past the label gap two canisters of one list may drift before
/// their link breaks. Without it a cluster whose chips sit right on the gap
/// joins and splits on alternate frames.
const LINK_HOLD_PX: f32 = metrics::GAP_XL;

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

/// Marker for one list chip layer: the shared tag of one cluster of
/// canisters.
#[derive(Component, Debug, Clone, Reflect)]
pub struct CargoCanisterListHudMarker;

/// The canisters a list chip stands for, sorted. Written each frame by the
/// grouping pass; a member that despawns leaves at once.
#[derive(Component, Debug, Clone, Default, PartialEq, Eq, Deref, Reflect)]
pub struct CargoCanisterListMembers(Vec<Entity>);

/// Marker for a list chip's node: the indicator the grouping pass anchors.
#[derive(Component, Debug, Clone, Reflect)]
struct CargoCanisterListNodeMarker;

/// A list chip's row text leaf, by its index from the top.
#[derive(Component, Debug, Clone, Copy, Reflect)]
struct CargoCanisterListRow(usize);

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

/// One row of a list chip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CargoListRow {
    /// One item kind and the count summed over the list's canisters.
    Item(String),
    /// How many item kinds the rows above do not name.
    More(usize),
}

/// The rows of a list chip over `canisters`: one per item kind with its count
/// summed over every stack of every canister, most first (ties by name). Past
/// [`LIST_MAX_ROWS`] kinds, the last row counts the kinds left.
pub(crate) fn cargo_list_rows<'a>(
    items: &GameItems,
    canisters: impl IntoIterator<Item = &'a CargoCanister>,
) -> Vec<CargoListRow> {
    let mut counts: BTreeMap<&ItemDesignId, u32> = BTreeMap::new();
    for canister in canisters {
        for (item, count) in canister.stacks() {
            *counts.entry(item).or_default() += count;
        }
    }
    let mut kinds: Vec<(u32, &str)> = counts
        .into_iter()
        .map(|(item, count)| (count, items.design(item).name.as_str()))
        .collect();
    kinds.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(b.1)));
    let named = if kinds.len() > LIST_MAX_ROWS {
        LIST_MAX_ROWS - 1
    } else {
        kinds.len()
    };
    let mut rows: Vec<CargoListRow> = kinds[..named]
        .iter()
        .map(|(count, name)| CargoListRow::Item(format!("{count} {name}")))
        .collect();
    if named < kinds.len() {
        rows.push(CargoListRow::More(kinds.len() - named));
    }
    rows
}

/// One canister chip as the grouping pass sees it: where its canister
/// projects (UI px) and how big its own chip is.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CanisterSpot {
    pub(crate) canister: Entity,
    pub(crate) centre: Vec2,
    pub(crate) size: Vec2,
}

/// Whether the chips of `a` and `b` come closer than `gap` px on either side.
fn chips_touch(a: &CanisterSpot, b: &CanisterSpot, gap: f32) -> bool {
    let reach = (a.size + b.size) / 2.0 + Vec2::splat(gap);
    let apart = (a.centre - b.centre).abs();
    apart.x < reach.x && apart.y < reach.y
}

/// The on-screen clusters of `spots`: canisters whose chips come within the
/// label gap of each other, chained. Two canisters in one list last frame
/// (`held` maps a canister to its list) stay linked up to [`LINK_HOLD_PX`]
/// further apart. Every canister is in exactly one cluster; each cluster is
/// sorted, and the clusters come in order of their first canister.
pub(crate) fn cluster_canisters(
    spots: &[CanisterSpot],
    held: &EntityHashMap<Entity>,
) -> Vec<Vec<Entity>> {
    let mut root: Vec<usize> = (0..spots.len()).collect();
    fn find(root: &mut [usize], mut at: usize) -> usize {
        while root[at] != at {
            root[at] = root[root[at]];
            at = root[at];
        }
        at
    }
    for i in 0..spots.len() {
        for j in (i + 1)..spots.len() {
            let same_list = matches!(
                (held.get(&spots[i].canister), held.get(&spots[j].canister)),
                (Some(a), Some(b)) if a == b
            );
            let gap = if same_list {
                LABEL_GAP_PX + LINK_HOLD_PX
            } else {
                LABEL_GAP_PX
            };
            if chips_touch(&spots[i], &spots[j], gap) {
                let (a, b) = (find(&mut root, i), find(&mut root, j));
                root[a.max(b)] = a.min(b);
            }
        }
    }
    let mut clusters: BTreeMap<usize, Vec<Entity>> = BTreeMap::new();
    for i in 0..spots.len() {
        let at = find(&mut root, i);
        clusters.entry(at).or_default().push(spots[i].canister);
    }
    let mut clusters: Vec<Vec<Entity>> = clusters.into_values().collect();
    for cluster in &mut clusters {
        cluster.sort();
    }
    clusters
}

/// The dark scrim row every canister tag is: a thin amber accent bar beside
/// a padded column of text lines. `node` is the indicator node it rides on.
fn tag_node(node: Node) -> Node {
    Node {
        display: Display::Flex,
        align_items: AlignItems::Stretch,
        ..node
    }
}

/// The accent bar at a tag's left edge.
fn tag_bar() -> impl Bundle {
    (
        Name::new("CargoCanisterChipBar"),
        Node {
            width: Val::Px(BAR_WIDTH_PX),
            ..default()
        },
        BackgroundColor(BAR_COLOR),
    )
}

/// The column the tag's text lines stand in.
fn tag_lines() -> Node {
    Node {
        flex_direction: FlexDirection::Column,
        padding: UiRect::all(Val::Px(LINES_PAD_PX)),
        ..default()
    }
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
            ScreenLabelType::Canister,
            screen_indicator_node(
                ScreenIndicatorConfig {
                    anchor: None,
                    size: ScreenIndicatorSize::Content,
                    offset: Vec2::ZERO,
                    offscreen: ScreenIndicatorOffscreen::Hide,
                },
                tag_node(Node::default()),
            ),
            CHIP_CLEARANCE,
            // A box like every HUD chip, so it takes their themed corner.
            ThemedRadius::control(),
            BackgroundColor(SCRIM_COLOR),
            children![
                tag_bar(),
                (
                    Name::new("CargoCanisterChipLines"),
                    tag_lines(),
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

/// One list row leaf: hidden until the writer gives it a row.
fn list_row(index: usize) -> impl Bundle {
    (
        Name::new("CargoCanisterListRow"),
        CargoCanisterListRow(index),
        Text::new(""),
        TextFont::from_font_size(LABEL_FONT_PX),
        nova_ui::widget::UiText,
        TextLayout {
            linebreak: LineBreak::NoWrap,
            ..default()
        },
        TextColor(LABEL_COLOR),
        Node {
            display: Display::None,
            ..default()
        },
        Pickable::IGNORE,
    )
}

/// UI bundle for one list chip layer standing for `members`, anchored at
/// `anchor`: the canister tag's shape with one row per item kind.
fn cargo_canister_list_hud(members: Vec<Entity>, anchor: Vec3) -> impl Bundle {
    (
        Name::new("CargoCanisterListHUD"),
        CargoCanisterListHudMarker,
        CargoCanisterListMembers(members),
        HudTier::Chrome,
        screen_indicator_layer(),
        children![(
            Name::new("CargoCanisterListUI"),
            CargoCanisterListNodeMarker,
            ScreenLabelType::Canister,
            screen_indicator_node(
                ScreenIndicatorConfig {
                    anchor: Some(ScreenIndicatorAnchorKind::Point(anchor)),
                    size: ScreenIndicatorSize::Content,
                    offset: Vec2::ZERO,
                    offscreen: ScreenIndicatorOffscreen::Hide,
                },
                tag_node(Node::default()),
            ),
            CHIP_CLEARANCE,
            ThemedRadius::control(),
            BackgroundColor(SCRIM_COLOR),
            children![
                tag_bar(),
                (
                    Name::new("CargoCanisterListLines"),
                    tag_lines(),
                    Children::spawn(SpawnIter((0..LIST_MAX_ROWS).map(list_row))),
                ),
            ],
        )],
    )
}

/// Every canister grows its chip the moment it spawns.
fn setup_cargo_canister_chip(add: On<Add<CargoCanister>>, mut commands: Commands) {
    commands.spawn(cargo_canister_chip_hud(add.entity));
}

/// A canister that leaves (taken by an intake, destroyed, unloaded) leaves
/// its list at once; a list left with fewer than [`LIST_MIN_CANISTERS`] comes
/// down, and the grouping pass shows the rest as single chips.
fn drop_canister_from_lists(
    remove: On<Remove<CargoCanister>>,
    mut commands: Commands,
    mut q_lists: Query<(Entity, &mut CargoCanisterListMembers)>,
) {
    for (list, mut members) in &mut q_lists {
        if !members.contains(&remove.entity) {
            continue;
        }
        members.0.retain(|&member| member != remove.entity);
        if members.len() < LIST_MIN_CANISTERS {
            commands.entity(list).despawn();
        }
    }
}

/// The canisters an intake on the player ship has in its reach, as the last
/// fixed intake pass published them. Empty without a player ship.
fn player_reach(
    q_player: &Query<Entity, With<PlayerSpaceshipMarker>>,
    readiness: &CargoPickupReadiness,
) -> EntityHashSet {
    q_player
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
        .unwrap_or_default()
}

/// Anchor each canister chip while its canister is in the reach of an intake
/// on the player ship, and fold each on-screen cluster of
/// [`LIST_MIN_CANISTERS`] or more into one list chip at the cluster's centre.
/// A list keeps its entity while its members overlap, so its fitted place
/// holds from frame to frame. Without a player ship every chip hides.
///
/// Clusters are read off last frame's camera pose and chip sizes: the
/// indicator pass that places the chips runs later in the frame.
#[expect(
    clippy::type_complexity,
    clippy::too_many_arguments,
    reason = "one query per chip family and the projection inputs"
)]
fn group_cargo_canister_chips(
    mut commands: Commands,
    q_chips: Query<(&AnchoredChipTarget, &Children), With<CargoCanisterChipHudMarker>>,
    mut q_nodes: Query<
        (&mut ScreenIndicatorAnchor, &ComputedNode),
        (
            With<AnchoredChipNodeMarker>,
            Without<CargoCanisterListNodeMarker>,
        ),
    >,
    mut q_lists: Query<
        (Entity, &mut CargoCanisterListMembers, &Children),
        With<CargoCanisterListHudMarker>,
    >,
    mut q_list_nodes: Query<&mut ScreenIndicatorAnchor, With<CargoCanisterListNodeMarker>>,
    q_canisters: Query<(&CargoCanister, &GlobalTransform)>,
    q_player: Query<Entity, With<PlayerSpaceshipMarker>>,
    q_camera: Query<(&Camera, &GlobalTransform), With<ScreenIndicatorCamera>>,
    ui_scale: Res<UiScale>,
    readiness: Res<CargoPickupReadiness>,
) {
    let in_reach = player_reach(&q_player, &readiness);
    let projection = q_camera
        .iter()
        .next()
        .map(|(camera, pose)| UiProjection::new(camera, pose, &ui_scale));
    let view = projection.and_then(|projection| projection.view_size());

    // Each shown canister's chip, where its canister projects in view.
    let mut spots = Vec::new();
    for (target, children) in &q_chips {
        if !in_reach.contains(&**target) {
            continue;
        }
        let (Some(projection), Some(view)) = (projection, view) else {
            continue;
        };
        let Ok((_, transform)) = q_canisters.get(**target) else {
            continue;
        };
        let Some(centre) = projection.point(transform.translation()) else {
            continue;
        };
        if centre.x < 0.0 || centre.y < 0.0 || centre.x > view.x || centre.y > view.y {
            continue;
        }
        let Some(size) = children.iter().find_map(|child| {
            q_nodes
                .get(child)
                .ok()
                .map(|(_, computed)| computed.size() * computed.inverse_scale_factor())
        }) else {
            continue;
        };
        spots.push(CanisterSpot {
            canister: **target,
            centre,
            size,
        });
    }

    let held: EntityHashMap<Entity> = q_lists
        .iter()
        .flat_map(|(list, members, _)| members.iter().map(move |&member| (member, list)))
        .collect();
    let groups: Vec<Vec<Entity>> = cluster_canisters(&spots, &held)
        .into_iter()
        .filter(|cluster| cluster.len() >= LIST_MIN_CANISTERS)
        .collect();
    let grouped: EntityHashSet = groups.iter().flatten().copied().collect();

    // Single chips: shown in reach, unless a list stands for them.
    for (target, children) in &q_chips {
        let shown = q_canisters.contains(**target)
            && in_reach.contains(&**target)
            && !grouped.contains(&**target);
        let wanted = shown.then_some(ScreenIndicatorAnchorKind::Entity(**target));
        for child in children.iter() {
            if let Ok((mut anchor, _)) = q_nodes.get_mut(child) {
                if **anchor != wanted {
                    **anchor = wanted;
                }
            }
        }
    }

    // Lists: each group takes the list that shares most of its canisters, so
    // a list keeps its entity (and its fitted place) as members come and go.
    let mut free: Vec<(Entity, usize)> = q_lists.iter().map(|(list, _, _)| (list, 0)).collect();
    for group in groups {
        let centroid = group
            .iter()
            .filter_map(|&member| q_canisters.get(member).ok())
            .map(|(_, transform)| transform.translation())
            .sum::<Vec3>()
            / group.len() as f32;
        let best = free
            .iter()
            .enumerate()
            .filter_map(|(at, (list, _))| {
                let (_, members, _) = q_lists.get(*list).ok()?;
                let shared = members
                    .iter()
                    .filter(|m| group.binary_search(m).is_ok())
                    .count();
                (shared > 0).then_some((shared, at))
            })
            .max()
            .map(|(_, at)| at);
        match best {
            Some(at) => {
                let (list, _) = free.swap_remove(at);
                let Ok((_, mut members, children)) = q_lists.get_mut(list) else {
                    continue;
                };
                if members.0 != group {
                    members.0 = group;
                }
                let wanted = Some(ScreenIndicatorAnchorKind::Point(centroid));
                for child in children.iter() {
                    if let Ok(mut anchor) = q_list_nodes.get_mut(child) {
                        if **anchor != wanted {
                            **anchor = wanted;
                        }
                    }
                }
            }
            None => {
                trace!(
                    "group_cargo_canister_chips: a list of {} canisters",
                    group.len()
                );
                commands.spawn(cargo_canister_list_hud(group, centroid));
            }
        }
    }
    for (list, _) in free {
        commands.entity(list).despawn();
    }
}

/// Write what each single chip's canister holds, and each list chip's rows.
#[expect(
    clippy::type_complexity,
    reason = "the label and row queries walk the same tag shape"
)]
fn write_cargo_canister_texts(
    q_chips: Query<&AnchoredChipTarget, With<CargoCanisterChipHudMarker>>,
    q_nodes: Query<&ChildOf, With<AnchoredChipNodeMarker>>,
    mut q_texts: Query<
        (&mut Text, &ChildOf, Has<CargoCanisterChipMassMarker>),
        (With<AnchoredChipLabelMarker>, Without<CargoCanisterListRow>),
    >,
    q_parents: Query<&ChildOf>,
    q_lists: Query<&CargoCanisterListMembers>,
    mut q_rows: Query<
        (
            &CargoCanisterListRow,
            &ChildOf,
            &mut Text,
            &mut TextColor,
            &mut Node,
        ),
        Without<AnchoredChipLabelMarker>,
    >,
    q_canisters: Query<&CargoCanister>,
    items: Res<GameItems>,
) {
    for (mut text, ChildOf(lines), is_mass) in &mut q_texts {
        let Ok(ChildOf(node)) = q_parents.get(*lines) else {
            continue;
        };
        let Ok(ChildOf(layer)) = q_nodes.get(*node) else {
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

    let mut rows_of: EntityHashMap<Vec<CargoListRow>> = EntityHashMap::default();
    for (row, ChildOf(lines), mut text, mut color, mut node) in &mut q_rows {
        let Some(list) = q_parents
            .get(*lines)
            .ok()
            .and_then(|ChildOf(node)| q_parents.get(*node).ok())
            .map(|ChildOf(layer)| *layer)
        else {
            continue;
        };
        let Ok(members) = q_lists.get(list) else {
            continue;
        };
        let rows = rows_of.entry(list).or_insert_with(|| {
            cargo_list_rows(
                &items,
                members
                    .iter()
                    .filter_map(|&member| q_canisters.get(member).ok()),
            )
        });
        let (next, ink, display) = match rows.get(row.0) {
            Some(CargoListRow::Item(line)) => (line.clone(), LABEL_COLOR, Display::Flex),
            Some(CargoListRow::More(left)) => (format!("+{left}"), MASS_COLOR, Display::Flex),
            None => (String::new(), LABEL_COLOR, Display::None),
        };
        if **text != next {
            **text = next;
        }
        if color.0 != ink {
            color.0 = ink;
        }
        if node.display != display {
            node.display = display;
        }
    }
}

/// Draws one content tag per [`CargoCanister`] in the reach of the player
/// ship's intakes, reading its stacks and mass, and one list tag per
/// on-screen cluster of [`LIST_MIN_CANISTERS`] or more (Chrome tier). Adds
/// the spawn and despawn observers and runs the grouping and text passes in
/// Update within [`super::NovaHudSystems`], once the mod merge has published
/// [`GameItems`]. Reads [`CargoPickupReadiness`], which
/// `CargoIntakeSectionPlugin` inits.
#[derive(Default)]
pub struct CargoCanisterChipsHudPlugin;

impl Plugin for CargoCanisterChipsHudPlugin {
    fn build(&self, app: &mut App) {
        trace!("CargoCanisterChipsHudPlugin: build");

        app.add_observer(setup_cargo_canister_chip);
        app.add_observer(despawn_anchored_chips::<CargoCanister, CargoCanisterChipHudMarker>);
        app.add_observer(drop_canister_from_lists);
        // NovaHudSystems runs from the first boot frame; the catalog lands
        // only at the merge, and never on a refused base game.
        app.add_systems(
            Update,
            (group_cargo_canister_chips, write_cargo_canister_texts)
                .chain()
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
        app.init_resource::<UiScale>();
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
        app.init_resource::<UiScale>();
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

    fn spot(canister: Entity, x: f32, y: f32) -> CanisterSpot {
        CanisterSpot {
            canister,
            centre: Vec2::new(x, y),
            size: Vec2::new(90.0, 30.0),
        }
    }

    /// Three canisters whose chips touch on screen share one list chip; a
    /// touching pair and a lone canister each keep their own chips.
    #[test]
    fn three_touching_canisters_share_one_list_and_a_pair_stays_single() {
        let [a, b, c, d, e, f] = [0, 1, 2, 3, 4, 5]
            .map(Entity::from_raw_u32)
            .map(Option::unwrap);
        let spots = [
            spot(a, 100.0, 100.0),
            spot(b, 150.0, 110.0),
            spot(c, 240.0, 100.0),
            spot(d, 700.0, 100.0),
            spot(e, 760.0, 120.0),
            spot(f, 400.0, 600.0),
        ];
        let lists: Vec<Vec<Entity>> = cluster_canisters(&spots, &EntityHashMap::default())
            .into_iter()
            .filter(|cluster| cluster.len() >= LIST_MIN_CANISTERS)
            .collect();
        let mut chain = vec![a, b, c];
        chain.sort();
        assert_eq!(
            lists,
            vec![chain],
            "the chained three form one list; the pair and the single stay chips"
        );
    }

    /// Canisters of one list stay linked a hold margin past the label gap, so
    /// a cluster sitting on the gap does not split and rejoin frame to frame.
    /// Two canisters not in one list link only inside the gap.
    #[test]
    fn a_list_holds_its_canisters_a_margin_past_the_label_gap() {
        let [a, b] = [0, 1].map(Entity::from_raw_u32).map(Option::unwrap);
        let apart = 90.0 + LABEL_GAP_PX + LINK_HOLD_PX / 2.0;
        let spots = [spot(a, 0.0, 0.0), spot(b, apart, 0.0)];
        assert_eq!(
            cluster_canisters(&spots, &EntityHashMap::default()).len(),
            2,
            "past the label gap two free canisters do not link"
        );
        let list = Entity::from_raw_u32(9).unwrap();
        let held: EntityHashMap<Entity> = [(a, list), (b, list)].into_iter().collect();
        assert_eq!(
            cluster_canisters(&spots, &held).len(),
            1,
            "inside the hold margin one list keeps both"
        );
    }

    /// A list has one row per item kind with the count summed over every
    /// canister, most first; past the row cap the last row counts the kinds
    /// it does not name.
    #[test]
    fn a_list_sums_each_item_and_caps_its_rows() {
        let items = nova_gameplay::test_support::test_items();
        let plate: ItemDesignId = ItemRoleType::HullPlate.into();
        let ore: ItemDesignId = ItemRoleType::IronOre.into();
        let summed = [
            CargoCanister::new(&items, &plate, 2),
            CargoCanister::new(&items, &plate, 3),
            CargoCanister::new(&items, &ore, 1),
        ];
        assert_eq!(
            cargo_list_rows(&items, &summed),
            vec![
                CargoListRow::Item("5 Hull plate".to_string()),
                CargoListRow::Item("1 Iron ore".to_string()),
            ]
        );

        let kinds: Vec<CargoCanister> = items
            .iter()
            .filter(|item| item.mass_g <= 100_000)
            .take(LIST_MAX_ROWS + 2)
            .map(|item| CargoCanister::new(&items, &item.id, 1))
            .collect();
        assert_eq!(
            kinds.len(),
            LIST_MAX_ROWS + 2,
            "the catalog holds enough kinds"
        );
        let rows = cargo_list_rows(&items, &kinds);
        assert_eq!(rows.len(), LIST_MAX_ROWS, "the list stops at the row cap");
        assert_eq!(
            rows.last(),
            Some(&CargoListRow::More(3)),
            "the last row counts the kinds it does not name"
        );
    }
}
