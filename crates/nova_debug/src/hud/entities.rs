//! The Entities section of the Nova Debug window: the world's top-level
//! entities grouped by sector and by type.
//!
//! A row is an entity with no parent, or one whose parent is a [`SectorRoot`]
//! (the sector's own objects). Sections, plates and turrets are not rows;
//! they show under their ship when its row expands. The rows are grouped
//! only while the section is open, and a row builds its inspector only while
//! it is expanded.

use std::collections::BTreeMap;

use bevy::{ecs::world::EntityRef, prelude::*};
use bevy_inspector_egui::{bevy_inspector, egui};
use nova_events::prelude::{EntityId, GeneratedBodyId, Meters, Meters3, SectorCoord};
use nova_gameplay::prelude::{
    Allegiance, CargoCanister, ChartSectorEdge, DerelictShipMarker, GunRoundMarker,
    OwnedStationMarker, PlayerSpaceshipMarker, SpaceshipRootMarker, StationMarker,
    TorpedoProjectileMarker,
};
use nova_scenario::prelude::{AsteroidMarker, PlanetMarker};
use nova_ship::prelude::ShipWreckFragmentMarker;
use nova_world::prelude::{CurrentSector, SectorRoot};

/// What a row is, for grouping. The variant order is the display order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum DebugEntityType {
    Player,
    Ship,
    Station,
    Planet,
    Asteroid,
    Canister,
    Wreck,
    Projectile,
    SectorRoot,
    Camera,
    Ui,
    Other,
}

impl DebugEntityType {
    fn label(self) -> &'static str {
        match self {
            Self::Player => "Player",
            Self::Ship => "Ship",
            Self::Station => "Station",
            Self::Planet => "Planet",
            Self::Asteroid => "Asteroid",
            Self::Canister => "Canister",
            Self::Wreck => "Wreck",
            Self::Projectile => "Projectile",
            Self::SectorRoot => "Sector root",
            Self::Camera => "Camera",
            Self::Ui => "UI",
            Self::Other => "Other",
        }
    }
}

/// The type of `entity`: the first marker it carries.
///
/// The station check comes before the ship check, because a station root
/// also carries [`SpaceshipRootMarker`] (it is built as a ship).
pub(crate) fn debug_entity_type(entity: &EntityRef) -> DebugEntityType {
    if entity.contains::<PlayerSpaceshipMarker>() {
        DebugEntityType::Player
    } else if entity.contains::<StationMarker>() {
        DebugEntityType::Station
    } else if entity.contains::<SpaceshipRootMarker>() {
        DebugEntityType::Ship
    } else if entity.contains::<PlanetMarker>() {
        DebugEntityType::Planet
    } else if entity.contains::<AsteroidMarker>() {
        DebugEntityType::Asteroid
    } else if entity.contains::<CargoCanister>() {
        DebugEntityType::Canister
    } else if entity.contains::<ShipWreckFragmentMarker>() {
        DebugEntityType::Wreck
    } else if entity.contains::<TorpedoProjectileMarker>() || entity.contains::<GunRoundMarker>() {
        DebugEntityType::Projectile
    } else if entity.contains::<SectorRoot>() {
        DebugEntityType::SectorRoot
    } else if entity.contains::<Camera>() {
        DebugEntityType::Camera
    } else if entity.contains::<Node>() {
        DebugEntityType::Ui
    } else {
        DebugEntityType::Other
    }
}

/// Which sector a row belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SectorKey {
    /// Under the [`SectorRoot`] of this cell (or that root itself).
    Owned(SectorCoord),
    /// A top-level body in a streaming world, in the cell its position falls
    /// in. Nothing retires it with that cell.
    TopLevel(SectorCoord),
    /// No streaming world: the whole scene is one group.
    Scene,
    /// A streaming world, but the entity has no position.
    Unplaced,
}

impl SectorKey {
    fn group(self) -> SectorGroup {
        match self {
            Self::Owned(cell) | Self::TopLevel(cell) => SectorGroup::Cell(cell),
            Self::Scene => SectorGroup::Scene,
            Self::Unplaced => SectorGroup::Unplaced,
        }
    }
}

/// The sector of a row: the cell of the root that owns it, else the cell of
/// its position while a world streams with cells of `edge`.
pub(crate) fn sector_key(
    owner: Option<SectorCoord>,
    position: Option<Vec3>,
    edge: Option<Meters>,
) -> SectorKey {
    if let Some(cell) = owner {
        return SectorKey::Owned(cell);
    }
    let Some(edge) = edge else {
        return SectorKey::Scene;
    };
    match position {
        Some(position) => SectorKey::TopLevel(SectorCoord::containing(
            Meters3::from_engine(position),
            edge,
        )),
        None => SectorKey::Unplaced,
    }
}

/// A sector header in the By sector tab.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum SectorGroup {
    Cell(SectorCoord),
    Scene,
    Unplaced,
}

/// Which grouping the Entities section shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum EntitiesTab {
    #[default]
    BySector,
    ByType,
}

/// One top-level entity.
#[derive(Debug)]
pub(crate) struct Row {
    pub(crate) entity: Entity,
    pub(crate) kind: DebugEntityType,
    pub(crate) sector: SectorKey,
    label: String,
}

/// Every row in the world, in entity order.
pub(crate) fn collect_rows(world: &mut World) -> Vec<Row> {
    let edge = world.get_resource::<ChartSectorEdge>().map(|edge| edge.0);
    let mut query = world.query::<(EntityRef, Option<&ChildOf>)>();
    let mut rows = Vec::new();
    for (entity, parent) in query.iter(world) {
        let parent_cell = match parent {
            None => None,
            Some(parent) => match world.get::<SectorRoot>(parent.parent()) {
                Some(root) => Some(root.0),
                None => continue,
            },
        };
        let owner = entity
            .get::<SectorRoot>()
            .map(|root| root.0)
            .or(parent_cell);
        let position = entity
            .get::<GlobalTransform>()
            .map(GlobalTransform::translation);
        let kind = debug_entity_type(&entity);
        rows.push(Row {
            entity: entity.id(),
            kind,
            sector: sector_key(owner, position, edge),
            label: row_label(&entity, kind),
        });
    }
    rows
}

/// Name, scenario id, generated id and type, with the side and state tags a
/// ship or a station carries.
fn row_label(entity: &EntityRef, kind: DebugEntityType) -> String {
    let mut label = match entity.get::<Name>() {
        Some(name) => format!("{name} {}", entity.id()),
        None => format!("{}", entity.id()),
    };
    if let Some(id) = entity.get::<EntityId>() {
        label.push_str(&format!("  id:{}", id.0));
    }
    if let Some(body) = entity.get::<GeneratedBodyId>() {
        label.push_str(&format!("  {body}"));
    }
    label.push_str(&format!("  [{}", kind.label()));
    if matches!(kind, DebugEntityType::Ship | DebugEntityType::Station) {
        if let Some(side) = entity.get::<Allegiance>() {
            label.push_str(&format!(" {side:?}"));
        }
    }
    if kind == DebugEntityType::Ship && entity.contains::<DerelictShipMarker>() {
        label.push_str(" derelict");
    }
    if kind == DebugEntityType::Station && entity.contains::<OwnedStationMarker>() {
        label.push_str(" owned");
    }
    label.push(']');
    label
}

/// The Entities section: the two tabs over the rows.
pub(crate) fn entities_ui(world: &mut World, ui: &mut egui::Ui, tab: &mut EntitiesTab) {
    ui.horizontal(|ui| {
        ui.selectable_value(tab, EntitiesTab::BySector, "By sector");
        ui.selectable_value(tab, EntitiesTab::ByType, "By type");
    });
    let rows = collect_rows(world);
    match tab {
        EntitiesTab::BySector => by_sector_ui(world, ui, &rows),
        EntitiesTab::ByType => by_type_ui(world, ui, &rows),
    }
}

/// Sector -> type -> entity. The observer's current cell comes first.
fn by_sector_ui(world: &mut World, ui: &mut egui::Ui, rows: &[Row]) {
    let current = world
        .get_resource::<CurrentSector>()
        .map(|current| current.0);
    let mut groups: BTreeMap<SectorGroup, BTreeMap<DebugEntityType, Vec<&Row>>> = BTreeMap::new();
    for row in rows {
        groups
            .entry(row.sector.group())
            .or_default()
            .entry(row.kind)
            .or_default()
            .push(row);
    }
    let mut order: Vec<SectorGroup> = groups.keys().copied().collect();
    order.sort_by_key(|group| Some(*group) != current.map(SectorGroup::Cell));
    for group in order {
        let types = &groups[&group];
        let count: usize = types.values().map(Vec::len).sum();
        let title = match group {
            SectorGroup::Cell(cell) if Some(cell) == current => {
                format!("Sector {cell} (current) - {count}")
            }
            SectorGroup::Cell(cell) => format!("Sector {cell} - {count}"),
            SectorGroup::Scene => format!("Scene - {count}"),
            SectorGroup::Unplaced => format!("Unplaced - {count}"),
        };
        egui::CollapsingHeader::new(title)
            .id_salt(("nova_debug_sector", group))
            .show(ui, |ui| {
                for (kind, rows) in types {
                    egui::CollapsingHeader::new(format!("{} - {}", kind.label(), rows.len()))
                        .id_salt(("nova_debug_sector_type", group, *kind))
                        .show(ui, |ui| {
                            for row in rows {
                                let tag = match row.sector {
                                    SectorKey::TopLevel(_) => "  (top-level)",
                                    _ => "",
                                };
                                entity_row_ui(world, ui, row, tag);
                            }
                        });
                }
            });
    }
}

/// Type -> entity, with the sector in each label.
fn by_type_ui(world: &mut World, ui: &mut egui::Ui, rows: &[Row]) {
    let mut types: BTreeMap<DebugEntityType, Vec<&Row>> = BTreeMap::new();
    for row in rows {
        types.entry(row.kind).or_default().push(row);
    }
    for (kind, rows) in types {
        egui::CollapsingHeader::new(format!("{} - {}", kind.label(), rows.len()))
            .id_salt(("nova_debug_type", kind))
            .show(ui, |ui| {
                for row in rows {
                    let sector = match row.sector {
                        SectorKey::Owned(cell) => format!("  sector {cell}"),
                        SectorKey::TopLevel(cell) => format!("  sector {cell} (top-level)"),
                        SectorKey::Scene => String::new(),
                        SectorKey::Unplaced => "  unplaced".to_owned(),
                    };
                    entity_row_ui(world, ui, row, &sector);
                }
            });
    }
}

/// One row. Its inspector, children included, is built only while it is
/// expanded.
fn entity_row_ui(world: &mut World, ui: &mut egui::Ui, row: &Row, suffix: &str) {
    egui::CollapsingHeader::new(format!("{}{suffix}", row.label))
        .id_salt(("nova_debug_entity", row.entity))
        .show(ui, |ui| {
            bevy_inspector::ui_for_entity_with_children(world, row.entity, ui);
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    const EDGE: Meters = Meters(10_000.0);

    fn row_of(world: &mut World, entity: Entity) -> Row {
        collect_rows(world)
            .into_iter()
            .find(|row| row.entity == entity)
            .expect("the entity is a row")
    }

    #[test]
    fn a_sector_roots_child_lists_under_that_roots_cell() {
        let mut world = World::new();
        world.insert_resource(ChartSectorEdge(EDGE));
        let cell = SectorCoord::new(2, 0, -1);
        let root = world.spawn(SectorRoot(cell)).id();
        // Far from its cell: ownership, not position, decides.
        let rock = world
            .spawn((AsteroidMarker, GlobalTransform::default(), ChildOf(root)))
            .id();

        let row = row_of(&mut world, rock);
        assert_eq!(row.sector, SectorKey::Owned(cell));
        assert_eq!(row.kind, DebugEntityType::Asteroid);
        assert_eq!(row_of(&mut world, root).sector, SectorKey::Owned(cell));
    }

    #[test]
    fn a_top_level_body_lists_under_the_cell_its_position_falls_in() {
        let mut world = World::new();
        world.insert_resource(ChartSectorEdge(EDGE));
        let position = Meters3::new(25_000.0, 0.0, -4_000.0).to_engine();
        let ship = world
            .spawn((
                SpaceshipRootMarker,
                GlobalTransform::from_translation(position),
            ))
            .id();
        let section = world
            .spawn((GlobalTransform::from_translation(position), ChildOf(ship)))
            .id();

        let rows = collect_rows(&mut world);
        assert!(
            rows.iter().all(|row| row.entity != section),
            "a ship's section is not a row"
        );
        let row = row_of(&mut world, ship);
        assert_eq!(row.sector, SectorKey::TopLevel(SectorCoord::new(3, 0, 0)));
        assert_eq!(row.kind, DebugEntityType::Ship);
    }

    #[test]
    fn without_a_streaming_world_every_row_is_in_the_scene() {
        let mut world = World::new();
        let placed = world.spawn(GlobalTransform::default()).id();
        let bare = world.spawn_empty().id();

        assert_eq!(row_of(&mut world, placed).sector, SectorKey::Scene);
        assert_eq!(row_of(&mut world, bare).sector, SectorKey::Scene);
    }
}
