//! What the map knows about: contact kinds, their colours and codes, and the
//! [`MapContacts`] system param that derives the live list from the world.
//!
//! Codes are assigned per kind and held stable across frames, so a label names
//! the same ship a moment later.
//!
//! Touch this module when adding a kind of thing the map can show.

use bevy::{ecs::system::SystemParam, prelude::*};
use nova_events::{
    prelude::{EntityId, EntityTypeName, ASTEROID_TYPE_NAME, PLANET_TYPE_NAME},
    units::prelude::*,
};
use nova_gameplay::prelude::*;
use nova_ship::prelude::{BodyRadius, HullEnvelopeRadius};
use nova_ui::theme::UiColor;

use crate::icons::BodyIconType;

/// What a map contact is, driving its color language and panel label.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MapContactKind {
    /// The player ship.
    OwnShip,
    /// A ship on the player's side.
    Ally,
    /// A ship on the enemy side.
    Hostile,
    /// A mission objective marker.
    Objective,
    /// An asteroid.
    Terrain,
    /// A ship on no side.
    Neutral,
    /// A planet.
    Planet,
}

impl MapContactKind {
    /// The upper-case kind word the contact panel prints.
    pub fn label(self) -> &'static str {
        match self {
            MapContactKind::OwnShip => "OWN SHIP",
            MapContactKind::Ally => "ALLY",
            MapContactKind::Hostile => "HOSTILE",
            MapContactKind::Objective => "OBJECTIVE",
            MapContactKind::Terrain => "TERRAIN",
            MapContactKind::Neutral => "NEUTRAL",
            MapContactKind::Planet => "PLANET",
        }
    }

    /// The theme colour a contact of this kind reads in: its blip icon, its
    /// legend entry and its contact panel.
    pub(crate) fn color(self) -> UiColor {
        match self {
            MapContactKind::OwnShip => UiColor::Primary,
            MapContactKind::Ally => UiColor::Info,
            MapContactKind::Hostile => UiColor::Danger,
            MapContactKind::Objective => UiColor::Accent,
            MapContactKind::Terrain | MapContactKind::Neutral | MapContactKind::Planet => {
                UiColor::Secondary
            }
        }
    }

    /// What the contact panel says a contact of this kind is.
    pub(crate) fn note(self) -> &'static str {
        match self {
            MapContactKind::OwnShip => "That is you.",
            MapContactKind::Ally => "Friendly contact.",
            MapContactKind::Hostile => "Hostile contact.",
            MapContactKind::Objective => "Mission objective.",
            MapContactKind::Terrain => "Asteroid mass.",
            MapContactKind::Neutral => "Ship on no side.",
            MapContactKind::Planet => "Planetary body.",
        }
    }

    /// The code prefix for this kind (`SELF`, `ALLY`, `HOST`, `OBJ`, `AST`,
    /// `NEU`, `PLN`), the
    /// stem of the unique per-contact [`MapContactCode`] labels.
    pub(crate) fn code_prefix(self) -> &'static str {
        match self {
            MapContactKind::OwnShip => "SELF",
            MapContactKind::Ally => "ALLY",
            MapContactKind::Hostile => "HOST",
            MapContactKind::Objective => "OBJ",
            MapContactKind::Terrain => "AST",
            MapContactKind::Neutral => "NEU",
            MapContactKind::Planet => "PLN",
        }
    }

    /// A dense index for this kind, for the per-kind next-index counters used when
    /// minting codes.
    pub(crate) fn code_slot(self) -> usize {
        match self {
            MapContactKind::OwnShip => 0,
            MapContactKind::Ally => 1,
            MapContactKind::Hostile => 2,
            MapContactKind::Objective => 3,
            MapContactKind::Terrain => 4,
            MapContactKind::Neutral => 5,
            MapContactKind::Planet => 6,
        }
    }
}

/// Classify a non-player ship by how it stands to the player's side, shared by
/// the contact model and the code-minting pass so the two never disagree on a
/// ship's kind. A Neutral ship answering the `player`'s fire is Hostile while
/// it does; `player` is a placeholder when no player ship exists.
pub(crate) fn ship_contact_kind(player: Entity, ship: RelationParty<'_>) -> MapContactKind {
    let viewer = RelationParty {
        entity: player,
        allegiance: Some(&Allegiance::Player),
        retaliation: None,
    };
    match ship_relation(viewer, ship) {
        Relation::Hostile => MapContactKind::Hostile,
        Relation::Own => MapContactKind::Ally,
        Relation::Neutral => MapContactKind::Neutral,
    }
}

/// A short, stable, human-readable handle for a map contact (`SELF`, `HOST-1`,
/// `AST-2`), the LABEL shown on its blip and in the contact panel.
/// Minted once per entity per session by `assign_map_contact_codes` from the
/// contact kind + a stable index; never reassigned. The own ship is always
/// `SELF` (there is exactly one); every other kind gets a `PREFIX-n` code.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct MapContactCode(pub String);

/// One plotted contact: its live world position plus range/bearing relative to
/// the player ship.
#[derive(Clone)]
pub struct MapContact {
    /// The contact's entity.
    pub entity: Entity,
    /// What the contact is.
    pub kind: MapContactKind,
    /// What the contact is drawn as.
    pub(crate) body: BodyIconType,
    /// The unique, typeable label (`SELF`, `HOST-1`, ...), as
    /// `assign_map_contact_codes` minted it.
    pub code: String,
    /// The contact's display name.
    pub name: String,
    /// The contact's live world position, in engine units.
    pub world_pos: Vec3,
    /// Range from the player ship in world units - it is measured off live
    /// transforms. It crosses to meters where it is rendered, and the shared
    /// distance policy formats it from there.
    pub range: f32,
    /// Bearing in the player's local frame: 0 dead ahead, +90 to starboard.
    pub(crate) bearing_deg: f32,
    /// Elevation ("mark") above/below the player's horizontal plane.
    pub(crate) mark_deg: f32,
}

impl MapContact {
    /// The range from the player ship through the shared player-facing
    /// distance policy: 1 world unit is 10 m, meters below 1 km and
    /// kilometers above.
    pub(crate) fn range_text(&self) -> String {
        nova_ui::units::distance(Meters::from_engine(self.range))
    }

    /// The bearing and mark from the player ship. The own ship has neither.
    pub(crate) fn bearing_text(&self) -> String {
        if self.kind == MapContactKind::OwnShip {
            return "Bearing ---".to_string();
        }
        format!(
            "Bearing {:03.0} mark {:+03.0}",
            self.bearing_deg, self.mark_deg
        )
    }
}

/// Bundled queries that enumerate every plottable contact.
#[derive(SystemParam)]
pub struct MapContacts<'w, 's> {
    pub(crate) player: Query<
        'w,
        's,
        (Entity, &'static GlobalTransform, Option<&'static Name>),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    pub(crate) ships: Query<
        'w,
        's,
        (
            Entity,
            &'static GlobalTransform,
            Option<&'static Name>,
            Option<&'static Allegiance>,
            Option<&'static RetaliationTarget>,
        ),
        (With<SpaceshipRootMarker>, Without<PlayerSpaceshipMarker>),
    >,
    pub(crate) objectives: Query<
        'w,
        's,
        (
            Entity,
            &'static GlobalTransform,
            &'static ObjectiveMarkerTarget,
        ),
    >,
    pub(crate) terrain: Query<
        'w,
        's,
        (Entity, &'static GlobalTransform, &'static EntityTypeName),
        Without<SpaceshipRootMarker>,
    >,
    /// The minted label of any contact that has one; read-only. Minting itself
    /// happens in [`assign_map_contact_codes`] via `Commands`.
    pub(crate) codes: Query<'w, 's, &'static MapContactCode>,
    /// The stable authored id of any contact that has one, used as the
    /// deterministic sort key when minting codes.
    pub(crate) ids: Query<'w, 's, &'static EntityId>,
    /// A ship's live containment radius, published every fixed tick.
    pub(crate) envelopes: Query<'w, 's, &'static HullEnvelopeRadius>,
    /// An authored body's bounding radius - asteroids, planetoids, beacons.
    pub(crate) bodies: Query<'w, 's, &'static BodyRadius>,
}

impl MapContacts<'_, '_> {
    /// How big `entity` actually is, in world units: a ship's live hull
    /// envelope, else its authored body radius, else nothing - which is the
    /// honest answer for a nav marker.
    pub(crate) fn radius_of(&self, entity: Entity) -> Option<f32> {
        self.envelopes
            .get(entity)
            .map(|envelope| **envelope)
            .ok()
            .or_else(|| self.bodies.get(entity).map(|body| **body).ok())
            .filter(|radius| *radius > 0.0)
    }

    /// The player ship's entity, world position and orientation, if one exists.
    pub(crate) fn player_frame(&self) -> Option<(Entity, Vec3, Quat)> {
        self.player.iter().next().map(|(entity, gt, _)| {
            let (_, rot, pos) = gt.to_scale_rotation_translation();
            (entity, pos, rot)
        })
    }

    /// The minted code for an entity. The own ship is always `SELF`; any
    /// other contact has none until [`assign_map_contact_codes`] mints it.
    ///
    /// No fallback to the contact's name: a streamed ship's name says whose
    /// it is, and a blip keeps the label it spawned with.
    pub(crate) fn code_for(&self, entity: Entity, kind: MapContactKind) -> Option<String> {
        match self.codes.get(entity) {
            Ok(code) => Some(code.0.clone()),
            Err(_) if kind == MapContactKind::OwnShip => Some(kind.code_prefix().to_string()),
            Err(_) => None,
        }
    }

    /// A stable sort key for an entity: its authored [`EntityId`] when present,
    /// else its bits, so minted indices are deterministic within a session.
    pub(crate) fn sort_key(&self, entity: Entity) -> String {
        self.ids
            .get(entity)
            .ok()
            .map(|id| id.0.clone())
            .unwrap_or_else(|| format!("{entity:?}"))
    }

    /// Every contact entity with its kind + stable sort key, for the code-minting
    /// pass. Uses the SAME classification as [`Self::collect`] (via
    /// [`ship_contact_kind`]) so labels never disagree with the rendered list.
    pub(crate) fn classified(&self) -> Vec<(Entity, MapContactKind, String)> {
        let mut out = Vec::new();
        let player = self.player_frame().map(|(player, _, _)| player);
        if let Some(player) = player {
            out.push((player, MapContactKind::OwnShip, self.sort_key(player)));
        }
        for (entity, _, _, allegiance, retaliation) in &self.ships {
            let kind = ship_contact_kind(
                player.unwrap_or(Entity::PLACEHOLDER),
                RelationParty {
                    entity,
                    allegiance,
                    retaliation,
                },
            );
            out.push((entity, kind, self.sort_key(entity)));
        }
        for (entity, _, _) in &self.objectives {
            out.push((entity, MapContactKind::Objective, self.sort_key(entity)));
        }
        for (entity, _, type_name) in &self.terrain {
            if !is_terrain(type_name) {
                continue;
            }
            out.push((entity, terrain_kind(type_name), self.sort_key(entity)));
        }
        out
    }

    /// The focus point the map orbits (the player, or the world origin).
    pub(crate) fn focus(&self) -> Vec3 {
        self.player_frame()
            .map(|(_, pos, _)| pos)
            .unwrap_or(Vec3::ZERO)
    }

    /// Enumerate every contact with live range/bearing, own ship first. A
    /// contact waits for its minted code, so it shows on the pass after
    /// [`assign_map_contact_codes`] first sees it.
    pub fn collect(&self) -> Vec<MapContact> {
        let (player_entity, player_pos, player_rot) = match self.player_frame() {
            Some(frame) => frame,
            None => (Entity::PLACEHOLDER, Vec3::ZERO, Quat::IDENTITY),
        };
        let inv = player_rot.inverse();
        let bearing = |world_pos: Vec3| -> (f32, f32, f32) {
            let rel = world_pos - player_pos;
            let range = rel.length();
            // Player local frame: forward is -Z, starboard +X, up +Y.
            let local = inv * rel;
            let horiz = (local.x * local.x + local.z * local.z).sqrt();
            let mut brg = local.x.atan2(-local.z).to_degrees();
            if brg < 0.0 {
                brg += 360.0;
            }
            let mark = local.y.atan2(horiz.max(f32::EPSILON)).to_degrees();
            (range, brg, mark)
        };

        let mut contacts = Vec::new();
        if let Some((_, _, name)) = self.player.iter().next() {
            let name = name
                .map(|n| n.as_str().to_string())
                .unwrap_or_else(|| "NOVA".to_string());
            let code = self
                .code_for(player_entity, MapContactKind::OwnShip)
                .expect("the own ship is always SELF");
            contacts.push(MapContact {
                entity: player_entity,
                kind: MapContactKind::OwnShip,
                body: BodyIconType::Ship,
                code,
                name,
                world_pos: player_pos,
                range: 0.0,
                bearing_deg: 0.0,
                mark_deg: 0.0,
            });
        }
        for (entity, gt, name, allegiance, retaliation) in &self.ships {
            let world_pos = gt.translation();
            let (range, brg, mark) = bearing(world_pos);
            let kind = ship_contact_kind(
                player_entity,
                RelationParty {
                    entity,
                    allegiance,
                    retaliation,
                },
            );
            let Some(code) = self.code_for(entity, kind) else {
                continue;
            };
            let name = name
                .map(|n| n.as_str().to_string())
                .unwrap_or_else(|| "CONTACT".to_string());
            contacts.push(MapContact {
                entity,
                kind,
                body: BodyIconType::Ship,
                code,
                name,
                world_pos,
                range,
                bearing_deg: brg,
                mark_deg: mark,
            });
        }
        for (entity, gt, marker) in &self.objectives {
            let world_pos = gt.translation();
            let Some(code) = self.code_for(entity, MapContactKind::Objective) else {
                continue;
            };
            let (range, brg, mark) = bearing(world_pos);
            let name = marker.label.to_uppercase();
            contacts.push(MapContact {
                entity,
                kind: MapContactKind::Objective,
                body: BodyIconType::Objective,
                code,
                name,
                world_pos,
                range,
                bearing_deg: brg,
                mark_deg: mark,
            });
        }
        for (entity, gt, type_name) in &self.terrain {
            if !is_terrain(type_name) {
                continue;
            }
            let world_pos = gt.translation();
            let (range, brg, mark) = bearing(world_pos);
            let kind = terrain_kind(type_name);
            let Some(code) = self.code_for(entity, kind) else {
                continue;
            };
            let name = terrain_name(type_name).to_string();
            contacts.push(MapContact {
                entity,
                kind,
                body: if kind == MapContactKind::Planet {
                    BodyIconType::Planet
                } else {
                    BodyIconType::Asteroid
                },
                code,
                name,
                world_pos,
                range,
                bearing_deg: brg,
                mark_deg: mark,
            });
        }
        contacts
    }
}

/// Mint a stable [`MapContactCode`] for every contact that lacks one. Runs as a
/// system (like `assign_section_codes`) so it sees entities spawned this frame;
/// existing codes are never reassigned, and a new contact takes the next free
/// index for its kind. The own ship is always the bare `SELF` prefix (exactly
/// one); every other kind gets `PREFIX-n`.
pub(crate) fn assign_map_contact_codes(mut commands: Commands, contacts: MapContacts) {
    // The highest index already handed out per kind, so new contacts continue the
    // sequence rather than colliding.
    let mut next: [u32; 7] = [0; 7];
    let mut unassigned: Vec<(Entity, MapContactKind, String)> = Vec::new();
    for (entity, kind, sort_key) in contacts.classified() {
        if let Ok(existing) = contacts.codes.get(entity) {
            if let Some(index) = existing
                .0
                .rsplit('-')
                .next()
                .and_then(|tail| tail.parse::<u32>().ok())
            {
                let slot = &mut next[kind.code_slot()];
                *slot = (*slot).max(index);
            }
        } else {
            unassigned.push((entity, kind, sort_key));
        }
    }
    if unassigned.is_empty() {
        return;
    }
    // Deterministic order: by the stable authored id, so indices match across runs.
    unassigned.sort_by(|a, b| a.2.cmp(&b.2));
    for (entity, kind, _) in unassigned {
        let code = if kind == MapContactKind::OwnShip {
            // Exactly one own ship: the bare prefix, no index.
            kind.code_prefix().to_string()
        } else {
            let slot = &mut next[kind.code_slot()];
            *slot += 1;
            format!("{}-{}", kind.code_prefix(), *slot)
        };
        commands.entity(entity).insert(MapContactCode(code));
    }
}

/// Whether an entity type name is a body the tactical map draws as terrain.
///
/// The map's terrain query is every named entity that is not a ship, so it
/// has to filter by name. Two names qualify: a rock and a world. A kind that
/// is added and NOT listed here vanishes from the map silently, which is why
/// this is one function rather than a comparison repeated per call site.
fn is_terrain(type_name: &EntityTypeName) -> bool {
    matches!(type_name.0.as_str(), ASTEROID_TYPE_NAME | PLANET_TYPE_NAME)
}

/// The contact kind of one piece of terrain.
fn terrain_kind(type_name: &EntityTypeName) -> MapContactKind {
    if type_name.0 == PLANET_TYPE_NAME {
        MapContactKind::Planet
    } else {
        MapContactKind::Terrain
    }
}

/// What the map calls one piece of terrain.
fn terrain_name(type_name: &EntityTypeName) -> &'static str {
    if type_name.0 == PLANET_TYPE_NAME {
        "PLANET"
    } else {
        "ASTEROID"
    }
}
