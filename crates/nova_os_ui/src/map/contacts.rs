//! What the map knows about: contact kinds, their colours and codes, and the
//! [`MapContacts`] system param that derives the live list from the world.
//!
//! Codes are assigned per kind and held stable across frames so a player can
//! type one at the prompt and still mean the same ship a moment later.
//!
//! Touch this module when adding a kind of thing the map can show.

use bevy::{ecs::system::SystemParam, prelude::*};
use nova_events::{
    prelude::{EntityId, EntityTypeName, ASTEROID_TYPE_NAME},
    units::prelude::*,
};
use nova_gameplay::prelude::*;
use nova_hud::allegiance_markers::allegiance_color;
use nova_os::prelude::*;

use crate::terminal::{NOVA_OS_AMBER, NOVA_OS_PHOSPHOR};

/// What a map contact is, driving its color language and readout label.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum MapContactKind {
    OwnShip,
    Ally,
    Hostile,
    Objective,
    Terrain,
}

impl MapContactKind {
    pub(crate) fn label(self) -> &'static str {
        match self {
            MapContactKind::OwnShip => "OWN SHIP",
            MapContactKind::Ally => "ALLY",
            MapContactKind::Hostile => "HOSTILE",
            MapContactKind::Objective => "OBJECTIVE",
            MapContactKind::Terrain => "TERRAIN",
        }
    }

    /// Blip / readout tint, consistent with the allegiance-marker palette.
    pub(crate) fn color(self) -> Color {
        match self {
            MapContactKind::OwnShip => NOVA_OS_PHOSPHOR,
            MapContactKind::Ally => allegiance_color(Some(&Allegiance::Player)),
            MapContactKind::Hostile => allegiance_color(Some(&Allegiance::Enemy)),
            MapContactKind::Objective => NOVA_OS_AMBER,
            MapContactKind::Terrain => allegiance_color(Some(&Allegiance::Neutral)),
        }
    }

    pub(crate) fn note(self) -> &'static str {
        match self {
            MapContactKind::OwnShip => "That is you.",
            MapContactKind::Ally => "Friendly contact.",
            MapContactKind::Hostile => "Hostile contact.",
            MapContactKind::Objective => "Mission objective.",
            MapContactKind::Terrain => "Asteroid mass.",
        }
    }

    /// The code prefix for this kind (`SELF`, `ALLY`, `HOST`, `OBJ`, `AST`), the
    /// stem of the unique per-contact [`MapContactCode`] labels.
    pub(crate) fn code_prefix(self) -> &'static str {
        match self {
            MapContactKind::OwnShip => "SELF",
            MapContactKind::Ally => "ALLY",
            MapContactKind::Hostile => "HOST",
            MapContactKind::Objective => "OBJ",
            MapContactKind::Terrain => "AST",
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
        }
    }
}

/// Classify a non-player ship by its allegiance, shared by the contact model and
/// the code-minting pass so the two never disagree on a ship's kind.
pub(crate) fn ship_contact_kind(allegiance: Option<&Allegiance>) -> MapContactKind {
    match allegiance {
        Some(Allegiance::Enemy) => MapContactKind::Hostile,
        Some(Allegiance::Player) => MapContactKind::Ally,
        _ => MapContactKind::Terrain,
    }
}

/// A short, stable, human-typeable handle for a map contact (`SELF`, `HOST-1`,
/// `AST-2`), the LABEL shown in `map view` and the id `map goto <label>` resolves.
/// Minted once per entity per session by `assign_map_contact_codes` from the
/// contact kind + a stable index; never reassigned. The own ship is always
/// `SELF` (there is exactly one); every other kind gets a `PREFIX-n` code.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct MapContactCode(pub String);

/// One plotted contact: its live world position plus range/bearing relative to
/// the player ship.
#[derive(Clone)]
pub(crate) struct MapContact {
    pub(crate) entity: Entity,
    pub(crate) kind: MapContactKind,
    /// The unique, typeable label (`SELF`, `HOST-1`, ...). Falls back to the
    /// uppercased name until [`assign_map_contact_codes`] mints the real code.
    pub(crate) code: String,
    pub(crate) name: String,
    pub(crate) world_pos: Vec3,
    /// Range from the player ship in world units. Rendered through the shared
    /// player-facing distance policy (1 u = 10 m; meters/kilometers).
    pub(crate) range: f32,
    /// Bearing in the player's local frame: 0 dead ahead, +90 to starboard.
    pub(crate) bearing_deg: f32,
    /// Elevation ("mark") above/below the player's horizontal plane.
    pub(crate) mark_deg: f32,
}

impl MapContact {
    /// The PoC readout line: `KIND CODE / NAME - range X, bearing Y. note`.
    /// Range uses the shared player-facing distance policy (1 world unit =
    /// 10 m; meters below 1 km, kilometers above).
    pub(crate) fn readout(&self) -> String {
        if self.kind == MapContactKind::OwnShip {
            return format!(
                "{} {} / {} - range {}, bearing ---. {}",
                self.kind.label(),
                self.code,
                self.name,
                nova_ui::units::distance(Meters::ZERO),
                self.kind.note()
            );
        }
        format!(
            "{} {} / {} - range {}, bearing {:03.0} mark {:+03.0}. {}",
            self.kind.label(),
            self.code,
            self.name,
            nova_ui::units::distance(Meters::from_engine(self.range)),
            self.bearing_deg,
            self.mark_deg,
            self.kind.note(),
        )
    }

    /// The INFO cell for the `map view` table: range for the own ship, else
    /// range + bearing/mark (carrying what the old RANGE/BEARING columns
    /// showed). Range in the shared player-facing units (m/km).
    pub(crate) fn info_cell(&self) -> String {
        if self.kind == MapContactKind::OwnShip {
            return format!("range {}", nova_ui::units::distance(Meters::ZERO));
        }
        format!(
            "{}  {:03.0} mark {:+03.0}",
            nova_ui::units::distance(Meters::from_engine(self.range)),
            self.bearing_deg,
            self.mark_deg,
        )
    }
}

/// Bundled queries that enumerate every plottable contact. Reused by the app
/// systems and by the `map view` CLI so the two never drift.
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
}

impl MapContacts<'_, '_> {
    /// The player ship's entity, world position and orientation, if one exists.
    pub(crate) fn player_frame(&self) -> Option<(Entity, Vec3, Quat)> {
        self.player.iter().next().map(|(entity, gt, _)| {
            let (_, rot, pos) = gt.to_scale_rotation_translation();
            (entity, pos, rot)
        })
    }

    /// The minted code for an entity, or a fallback derived from `kind`/`name`
    /// for the one frame before [`assign_map_contact_codes`] mints it.
    pub(crate) fn code_for(&self, entity: Entity, kind: MapContactKind, name: &str) -> String {
        self.codes
            .get(entity)
            .ok()
            .map(|c| c.0.clone())
            .unwrap_or_else(|| {
                if kind == MapContactKind::OwnShip {
                    kind.code_prefix().to_string()
                } else {
                    name.to_uppercase()
                }
            })
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
        if let Some((player, _, _)) = self.player_frame() {
            out.push((player, MapContactKind::OwnShip, self.sort_key(player)));
        }
        for (entity, _, _, allegiance) in &self.ships {
            out.push((entity, ship_contact_kind(allegiance), self.sort_key(entity)));
        }
        for (entity, _, _) in &self.objectives {
            out.push((entity, MapContactKind::Objective, self.sort_key(entity)));
        }
        for (entity, _, type_name) in &self.terrain {
            if type_name.0 != ASTEROID_TYPE_NAME {
                continue;
            }
            out.push((entity, MapContactKind::Terrain, self.sort_key(entity)));
        }
        out
    }

    /// Resolve a typed label (case-insensitive) to its contact, for `map goto`.
    pub(crate) fn resolve(&self, label: &str) -> Option<MapContact> {
        let wanted = label.to_ascii_uppercase();
        self.collect()
            .into_iter()
            .find(|c| c.code.eq_ignore_ascii_case(&wanted))
    }

    /// Every contact label, own ship first then ascending range, for Tab
    /// completion of `map goto <label>`.
    pub(crate) fn labels(&self) -> Vec<String> {
        let mut list = self.collect();
        list.sort_by(|a, b| {
            let key = |c: &MapContact| (c.kind != MapContactKind::OwnShip, c.range);
            key(a)
                .partial_cmp(&key(b))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        list.into_iter().map(|c| c.code).collect()
    }

    /// The focus point the map orbits (the player, or the world origin).
    pub(crate) fn focus(&self) -> Vec3 {
        self.player_frame()
            .map(|(_, pos, _)| pos)
            .unwrap_or(Vec3::ZERO)
    }

    /// Enumerate every contact with live range/bearing, own ship first.
    pub(crate) fn collect(&self) -> Vec<MapContact> {
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
            contacts.push(MapContact {
                entity: player_entity,
                kind: MapContactKind::OwnShip,
                code: self.code_for(player_entity, MapContactKind::OwnShip, &name),
                name,
                world_pos: player_pos,
                range: 0.0,
                bearing_deg: 0.0,
                mark_deg: 0.0,
            });
        }
        for (entity, gt, name, allegiance) in &self.ships {
            let world_pos = gt.translation();
            let (range, brg, mark) = bearing(world_pos);
            let kind = ship_contact_kind(allegiance);
            let name = name
                .map(|n| n.as_str().to_string())
                .unwrap_or_else(|| "CONTACT".to_string());
            contacts.push(MapContact {
                entity,
                kind,
                code: self.code_for(entity, kind, &name),
                name,
                world_pos,
                range,
                bearing_deg: brg,
                mark_deg: mark,
            });
        }
        for (entity, gt, marker) in &self.objectives {
            let world_pos = gt.translation();
            let (range, brg, mark) = bearing(world_pos);
            let name = marker.label.to_uppercase();
            contacts.push(MapContact {
                entity,
                kind: MapContactKind::Objective,
                code: self.code_for(entity, MapContactKind::Objective, &name),
                name,
                world_pos,
                range,
                bearing_deg: brg,
                mark_deg: mark,
            });
        }
        for (entity, gt, type_name) in &self.terrain {
            if type_name.0 != ASTEROID_TYPE_NAME {
                continue;
            }
            let world_pos = gt.translation();
            let (range, brg, mark) = bearing(world_pos);
            let name = "ASTEROID".to_string();
            contacts.push(MapContact {
                entity,
                kind: MapContactKind::Terrain,
                code: self.code_for(entity, MapContactKind::Terrain, &name),
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
    let mut next: [u32; 5] = [0; 5];
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

/// Build the `map view` CLI rows from the contact model as a fixed-width
/// KIND/LABEL/INFO table (own ship first, then nearest-first) - the same shape
/// `ship view` prints, so a label copies straight into `map goto <label>`. A pure
/// function so it is unit-testable off a fixed list.
pub(crate) fn map_rows_from_contacts(contacts: &[MapContact]) -> Vec<TerminalRow> {
    let mut rows = vec![
        TerminalRow {
            kind: TerminalRowKind::Info,
            text: "LOCAL SPACE - contacts".to_string(),
        },
        TerminalRow {
            kind: TerminalRowKind::Dim,
            text: format!("Contacts: {}", contacts.len()),
        },
    ];
    if contacts.iter().all(|c| c.kind == MapContactKind::OwnShip) {
        rows.push(TerminalRow {
            kind: TerminalRowKind::Warn,
            text: "no contacts in local space".to_string(),
        });
    }

    // Own ship first, then by ascending range.
    let mut ordered: Vec<&MapContact> = contacts.iter().collect();
    ordered.sort_by(|a, b| {
        let key = |c: &MapContact| (c.kind != MapContactKind::OwnShip, c.range);
        key(a)
            .partial_cmp(&key(b))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Column widths: pad KIND and LABEL to the widest cell (header included) so the
    // monospace terminal lines the columns up. INFO is last, so it needs no pad.
    // Identical mechanism to `terminal_ship_rows`.
    const KIND_HEADER: &str = "KIND";
    const LABEL_HEADER: &str = "LABEL";
    const GUTTER: &str = "  ";
    let w_kind = ordered
        .iter()
        .map(|c| c.kind.label().len())
        .chain([KIND_HEADER.len()])
        .max()
        .unwrap_or(KIND_HEADER.len());
    let w_label = ordered
        .iter()
        .map(|c| c.code.len())
        .chain([LABEL_HEADER.len()])
        .max()
        .unwrap_or(LABEL_HEADER.len());

    rows.push(TerminalRow {
        kind: TerminalRowKind::Dim,
        text: format!("{KIND_HEADER:<w_kind$}{GUTTER}{LABEL_HEADER:<w_label$}{GUTTER}INFO"),
    });
    for contact in ordered {
        rows.push(TerminalRow {
            kind: TerminalRowKind::Output,
            text: format!(
                "{kind:<w_kind$}{GUTTER}{label:<w_label$}{GUTTER}{info}",
                kind = contact.kind.label(),
                label = contact.code,
                info = contact.info_cell(),
            ),
        });
    }
    rows
}

/// The `map view` rows for the terminal snapshot. Called by the NOVA OS keyboard
/// handler when it builds a command snapshot.
pub fn terminal_map_rows(contacts: &MapContacts) -> Vec<TerminalRow> {
    map_rows_from_contacts(&contacts.collect())
}
