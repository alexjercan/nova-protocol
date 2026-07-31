//! NOVA OS `map` app: a terminal-launched schematic 3D minimap of local space.
//!
//! This is the visual counterpart of the `map view` CLI built-in. Both read the
//! same [`MapContacts`] model (player, allies, enemies, asteroids, objective
//! markers with live range/bearing). The app renders a small schematic 3D scene
//! - concentric distance rings and a central hub - through a dedicated
//! [`Camera3d`] on its own [`RenderLayers`] into an offscreen image, shown in the
//! app body; the interactive CONTACTS ride on top as projected clickable UI
//! blips (a nested 3D mesh would not be pickable through the NOVA OS CRT
//! composite, but UI buttons are - see `tasks/20260724-102320/DECISION.md`).
//!
//! The camera is a [`MapOrbit`] you drive with the keyboard (Q/E turn, R/F tilt)
//! plus WASD (move the focus around) and the wheel (zoom). Selecting a contact fills a
//! readout with kind / name / range / bearing; `G` sets a flight [`Autopilot`]
//! GOTO on the player ship that persists after the computer closes.
//!
//! The app trait ([`NovaOsAppRuntime`]) only hands apps discrete key presses and
//! no mouse, so all of the interaction runs as this module's OWN systems, gated
//! on the map app being the active NOVA OS surface.

use bevy::{
    camera::{visibility::RenderLayers, ImageRenderTarget, RenderTarget},
    ecs::system::SystemParam,
    input::mouse::{MouseMotion, MouseWheel},
    prelude::*,
    render::render_resource::{Extent3d, TextureFormat},
    // The activatable Button (fires `Activate` through the forwarded NOVA OS
    // pointer), matching the terminal's own buttons.
    ui_widgets::{Activate, Button},
};
use nova_events::prelude::{EntityId, EntityTypeName};
use nova_os::prelude::*;
use nova_ui::font::UiFont;

use crate::{
    hud::{
        allegiance_markers::allegiance_color,
        nova_os::{
            nova_os_font, nova_os_text_font, DRAWER_LINE_FONT_PX, NOVA_OS_AMBER, NOVA_OS_PHOSPHOR,
            NOVA_OS_PHOSPHOR_DIM, NOVA_OS_PHOSPHOR_MUTED, NOVA_OS_SCREEN, NOVA_OS_TEXT,
        },
    },
    prelude::*,
};

/// Glob-import surface: `use nova_gameplay::hud::nova_os_map::prelude::*`.
pub mod prelude {
    pub use super::{MapContactCode, NovaOsMapPlugin};
}

/// The launch word / stable id of the map app.
const MAP_APP_ID: &str = "map";
/// Render layer the map scene + camera live on, isolated from the world (0) and
/// the NOVA OS terminal RTT (20).
const MAP_LAYER: usize = 21;
/// The map camera renders BEFORE the NOVA OS offscreen pass (-20) so its image is
/// ready when the NOVA OS content samples it.
const MAP_CAMERA_ORDER: isize = -30;
/// Distance-ring radii (world units) drawn on the map floor as scale reference.
const MAP_RING_RADII: [f32; 3] = [40.0, 80.0, 120.0];
/// Orbit-radius zoom clamp (world units from the focus).
const MAP_RADIUS_MIN: f32 = 30.0;
const MAP_RADIUS_MAX: f32 = 520.0;
/// Default orbit framing when the app opens or `R` resets the view.
const MAP_RADIUS_DEFAULT: f32 = 170.0;
const MAP_THETA_DEFAULT: f32 = 0.8;
const MAP_PHI_DEFAULT: f32 = 0.62;

/// Footer hints while the map owns the screen (swapped in by the runtime).
const MAP_HINTS: &[&str] = &[
    "WASD: MOVE",
    "Q/E: TURN",
    "R/F: TILT",
    "DRAG: LOOK",
    "WHEEL: ZOOM",
    "[ / ]: CYCLE",
    "G: GOTO",
    "T: RESET",
    "ESC: BACK",
];

// ---------------------------------------------------------------------------
// Contact model (shared by the CLI and the app)
// ---------------------------------------------------------------------------

/// What a map contact is, driving its color language and readout label.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum MapContactKind {
    OwnShip,
    Ally,
    Hostile,
    Objective,
    Terrain,
}

impl MapContactKind {
    fn label(self) -> &'static str {
        match self {
            MapContactKind::OwnShip => "OWN SHIP",
            MapContactKind::Ally => "ALLY",
            MapContactKind::Hostile => "HOSTILE",
            MapContactKind::Objective => "OBJECTIVE",
            MapContactKind::Terrain => "TERRAIN",
        }
    }

    /// Blip / readout tint, consistent with the allegiance-marker palette.
    fn color(self) -> Color {
        match self {
            MapContactKind::OwnShip => NOVA_OS_PHOSPHOR,
            MapContactKind::Ally => allegiance_color(Some(&Allegiance::Player)),
            MapContactKind::Hostile => allegiance_color(Some(&Allegiance::Enemy)),
            MapContactKind::Objective => NOVA_OS_AMBER,
            MapContactKind::Terrain => allegiance_color(Some(&Allegiance::Neutral)),
        }
    }

    fn note(self) -> &'static str {
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
    fn code_prefix(self) -> &'static str {
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
    fn code_slot(self) -> usize {
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
fn ship_contact_kind(allegiance: Option<&Allegiance>) -> MapContactKind {
    match allegiance {
        Some(Allegiance::Enemy) => MapContactKind::Hostile,
        Some(Allegiance::Player) => MapContactKind::Ally,
        _ => MapContactKind::Terrain,
    }
}

/// A short, stable, human-typeable handle for a map contact (`SELF`, `HOST-1`,
/// `AST-2`), the LABEL shown in `map view` and the id `map goto <label>` resolves.
/// Minted once per entity per session by [`assign_map_contact_codes`] from the
/// contact kind + a stable index; never reassigned. The own ship is always
/// `SELF` (there is exactly one); every other kind gets a `PREFIX-n` code.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct MapContactCode(pub String);

/// One plotted contact: its live world position plus range/bearing relative to
/// the player ship.
#[derive(Clone)]
struct MapContact {
    entity: Entity,
    kind: MapContactKind,
    /// The unique, typeable label (`SELF`, `HOST-1`, ...). Falls back to the
    /// uppercased name until [`assign_map_contact_codes`] mints the real code.
    code: String,
    name: String,
    world_pos: Vec3,
    /// Range from the player ship in world units. Rendered through the shared
    /// player-facing distance policy (1 u = 10 m; metres/kilometres).
    range: f32,
    /// Bearing in the player's local frame: 0 dead ahead, +90 to starboard.
    bearing_deg: f32,
    /// Elevation ("mark") above/below the player's horizontal plane.
    mark_deg: f32,
}

impl MapContact {
    /// The PoC readout line: `KIND CODE / NAME - range X, bearing Y. note`.
    /// Range uses the shared player-facing distance policy (1 world unit =
    /// 10 m; metres below 1 km, kilometres above).
    fn readout(&self) -> String {
        if self.kind == MapContactKind::OwnShip {
            return format!(
                "{} {} / {} - range {}, bearing ---. {}",
                self.kind.label(),
                self.code,
                self.name,
                nova_ui::units::distance(0.0),
                self.kind.note()
            );
        }
        format!(
            "{} {} / {} - range {}, bearing {:03.0} mark {:+03.0}. {}",
            self.kind.label(),
            self.code,
            self.name,
            nova_ui::units::distance(self.range),
            self.bearing_deg,
            self.mark_deg,
            self.kind.note(),
        )
    }

    /// The INFO cell for the `map view` table: range for the own ship, else
    /// range + bearing/mark (carrying what the old RANGE/BEARING columns
    /// showed). Range in the shared player-facing units (m/km).
    fn info_cell(&self) -> String {
        if self.kind == MapContactKind::OwnShip {
            return format!("range {}", nova_ui::units::distance(0.0));
        }
        format!(
            "{}  {:03.0} mark {:+03.0}",
            nova_ui::units::distance(self.range),
            self.bearing_deg,
            self.mark_deg,
        )
    }
}

/// Bundled queries that enumerate every plottable contact. Reused by the app
/// systems and by the `map view` CLI so the two never drift.
#[derive(SystemParam)]
pub struct MapContacts<'w, 's> {
    player: Query<
        'w,
        's,
        (Entity, &'static GlobalTransform, Option<&'static Name>),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    ships: Query<
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
    objectives: Query<
        'w,
        's,
        (
            Entity,
            &'static GlobalTransform,
            &'static ObjectiveMarkerTarget,
        ),
    >,
    terrain: Query<
        'w,
        's,
        (Entity, &'static GlobalTransform, &'static EntityTypeName),
        Without<SpaceshipRootMarker>,
    >,
    /// The minted label of any contact that has one; read-only. Minting itself
    /// happens in [`assign_map_contact_codes`] via `Commands`.
    codes: Query<'w, 's, &'static MapContactCode>,
    /// The stable authored id of any contact that has one, used as the
    /// deterministic sort key when minting codes.
    ids: Query<'w, 's, &'static EntityId>,
}

impl MapContacts<'_, '_> {
    /// The player ship's entity, world position and orientation, if one exists.
    fn player_frame(&self) -> Option<(Entity, Vec3, Quat)> {
        self.player.iter().next().map(|(entity, gt, _)| {
            let (_, rot, pos) = gt.to_scale_rotation_translation();
            (entity, pos, rot)
        })
    }

    /// The minted code for an entity, or a fallback derived from `kind`/`name`
    /// for the one frame before [`assign_map_contact_codes`] mints it.
    fn code_for(&self, entity: Entity, kind: MapContactKind, name: &str) -> String {
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
    fn sort_key(&self, entity: Entity) -> String {
        self.ids
            .get(entity)
            .ok()
            .map(|id| id.0.clone())
            .unwrap_or_else(|| format!("{entity:?}"))
    }

    /// Every contact entity with its kind + stable sort key, for the code-minting
    /// pass. Uses the SAME classification as [`Self::collect`] (via
    /// [`ship_contact_kind`]) so labels never disagree with the rendered list.
    fn classified(&self) -> Vec<(Entity, MapContactKind, String)> {
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
            if type_name.0 != "asteroid" {
                continue;
            }
            out.push((entity, MapContactKind::Terrain, self.sort_key(entity)));
        }
        out
    }

    /// Resolve a typed label (case-insensitive) to its contact, for `map goto`.
    fn resolve(&self, label: &str) -> Option<MapContact> {
        let wanted = label.to_ascii_uppercase();
        self.collect()
            .into_iter()
            .find(|c| c.code.eq_ignore_ascii_case(&wanted))
    }

    /// Every contact label, own ship first then ascending range, for Tab
    /// completion of `map goto <label>`.
    fn labels(&self) -> Vec<String> {
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
    fn focus(&self) -> Vec3 {
        self.player_frame()
            .map(|(_, pos, _)| pos)
            .unwrap_or(Vec3::ZERO)
    }

    /// Enumerate every contact with live range/bearing, own ship first.
    fn collect(&self) -> Vec<MapContact> {
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
            if type_name.0 != "asteroid" {
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
fn assign_map_contact_codes(mut commands: Commands, contacts: MapContacts) {
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
fn map_rows_from_contacts(contacts: &[MapContact]) -> Vec<TerminalRow> {
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

/// Keep the terminal's arg-completion set in sync with the live contact labels,
/// so `map goto <TAB>` offers them. Only writes on a real change.
fn sync_map_arg_completions(
    contacts: MapContacts,
    mut runtime: ResMut<MapRuntime>,
    mut terminal: ResMut<NovaOsTerminal>,
) {
    let labels = contacts.labels();
    if labels == runtime.completion_labels {
        return;
    }
    runtime.completion_labels = labels.clone();
    terminal.merge_arg_completions([("map goto", labels)]);
}

/// Drain the arg-bearing `map goto` verb the terminal queued on submit, resolve
/// the label to a contact, and set a flight [`Autopilot`] GOTO on the player ship
/// (the same seam the in-app `G` key uses). Peeks the shared pending slot first so
/// it never swallows a `ship ...` verb.
fn apply_map_cli_commands(
    mut commands: Commands,
    mut terminal: ResMut<NovaOsTerminal>,
    contacts: MapContacts,
) {
    // Only consume an invocation this handler owns; leave `ship ...` for its system
    // (`cross-app-invocation-peek-before-take`).
    let owns = terminal
        .peek_pending_invocation()
        .is_some_and(|inv| inv.name == "map goto");
    if !owns {
        return;
    }
    let Some(invocation) = terminal.take_pending_invocation() else {
        return;
    };
    let Some(label) = invocation.args.first() else {
        terminal.extend_scrollback([TerminalRow {
            kind: TerminalRowKind::Error,
            text: format!("{}: expected a contact label", invocation.name),
        }]);
        return;
    };
    let Some((player, _, _)) = contacts.player_frame() else {
        terminal.extend_scrollback([TerminalRow {
            kind: TerminalRowKind::Error,
            text: "no live player ship".to_string(),
        }]);
        return;
    };

    let rows = match contacts.resolve(label) {
        Some(contact) if contact.kind == MapContactKind::OwnShip => vec![TerminalRow {
            kind: TerminalRowKind::Dim,
            text: format!("goto: {} is your own ship", contact.code),
        }],
        Some(contact) => {
            commands
                .entity(player)
                .insert(Autopilot::engage(AutopilotAction::Goto {
                    target: contact.entity,
                }));
            vec![TerminalRow {
                kind: TerminalRowKind::Info,
                text: format!(
                    "goto {} ({}): autopilot engaged, range {}",
                    contact.code,
                    contact.name,
                    nova_ui::units::distance(contact.range),
                ),
            }]
        }
        None => {
            let labels = contacts.labels();
            let mut rows = vec![TerminalRow {
                kind: TerminalRowKind::Error,
                text: format!("no such contact: {label}"),
            }];
            if !labels.is_empty() {
                rows.push(TerminalRow {
                    kind: TerminalRowKind::Dim,
                    text: format!("contacts: {}", labels.join("   ")),
                });
            }
            rows
        }
    };
    terminal.extend_scrollback(rows);
}

// ---------------------------------------------------------------------------
// The app + its runtime state
// ---------------------------------------------------------------------------

/// The `map` app: identity + static body shell. All interaction runs in this
/// module's systems (the trait gives no mouse and only discrete keys).
struct MapApp;

impl NovaOsAppRuntime for MapApp {
    fn id(&self) -> &'static str {
        MAP_APP_ID
    }
    fn title(&self) -> &'static str {
        "MAP / LOCAL SPACE"
    }
    fn hints(&self) -> &'static [&'static str] {
        MAP_HINTS
    }
    fn spawn_body(&self, body: &mut ChildSpawnerCommands, font: Handle<Font>) {
        // The viewport shows the offscreen map image (patched in once the RTT
        // exists); blips are added over it as absolutely-positioned children.
        body.spawn((
            MapViewportMarker,
            Node {
                flex_grow: 1.0,
                min_height: Val::Px(0.0),
                position_type: PositionType::Relative,
                overflow: Overflow::clip(),
                ..default()
            },
            ImageNode {
                image: Handle::default(),
                ..default()
            },
            BackgroundColor(MAP_VIEW_BG),
        ));
        body.spawn((
            MapReadoutMarker,
            Text::new("Select a contact for range and bearing."),
            nova_os_text_font(DRAWER_LINE_FONT_PX, font),
            TextColor(NOVA_OS_PHOSPHOR_MUTED),
            Node {
                min_height: Val::Px(26.0),
                ..default()
            },
        ));
    }
}

/// Dark fill behind the map image so it reads as a recessed screen.
const MAP_VIEW_BG: Color = Color::srgb_u8(0, 6, 3);

/// The app body's map viewport node (holds the RTT image + blip children).
#[derive(Component)]
struct MapViewportMarker;

/// The contact readout line under the viewport.
#[derive(Component)]
struct MapReadoutMarker;

/// The map's 3D camera (renders the schematic scene to the offscreen image).
#[derive(Component)]
struct MapCameraMarker;

/// The map camera's orbit state, driven directly (we own the spherical math
/// rather than routing through the shared `SphereOrbit` plugin, whose smoothed
/// input path did not rotate this render-to-texture camera in practice).
/// `theta` is the azimuth, `phi` the elevation above the plane, `radius` the
/// distance from `center`, the focus point WASD pans and selection recenters.
#[derive(Component)]
struct MapOrbit {
    theta: f32,
    phi: f32,
    radius: f32,
    center: Vec3,
}

/// The camera eye offset from the focus for a given orbit, on a Y-up sphere.
fn orbit_eye(radius: f32, theta: f32, phi: f32) -> Vec3 {
    let horizontal = radius * phi.cos();
    Vec3::new(
        horizontal * theta.sin(),
        radius * phi.sin(),
        horizontal * theta.cos(),
    )
}

/// The parent of every spawned map-scene entity (camera + proxy meshes), so the
/// whole scene tears down with one `despawn`.
#[derive(Component)]
struct MapSceneRoot;

/// Holds the distance rings + hub; its transform tracks the orbit center so the
/// scale reference surrounds the focused object (`map_focus_follow`).
#[derive(Component)]
struct MapFocusAnchor;

/// A projected contact blip (a clickable UI marker over the viewport image).
#[derive(Component)]
struct MapBlip {
    contact: Entity,
}

/// Live state of the running map app.
#[derive(Resource, Default)]
struct MapRuntime {
    active: bool,
    image: Option<Handle<Image>>,
    camera: Option<Entity>,
    scene_root: Option<Entity>,
    blips: bevy::platform::collections::HashMap<Entity, Entity>,
    selected: Option<Entity>,
    /// The selection the focus last recentered on, so selecting a NEW contact
    /// snaps the map onto it once (without fighting WASD panning after).
    focused_on: Option<Entity>,
    /// A transient "GOTO SET" note shown in the readout for a short time.
    goto_note: Option<(String, f32)>,
    /// The labels last pushed as `map goto` arg-completions, so the terminal is
    /// only marked changed when the set changes.
    completion_labels: Vec<String>,
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

/// Registers the `map` app and drives its scene, camera, blips and GOTO.
pub struct NovaOsMapPlugin;

impl Plugin for NovaOsMapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MapRuntime>();
        // Register the `map` command tree into the unified NOVA OS command
        // registry (created by NovaOsPlugin, added before this plugin): the launch
        // word `map` (which spawns the app), its `map view` CLI subcommand, and the
        // `MapApp` runtime, all declared together. `sync_nova_os_commands` mirrors
        // these into the terminal's command set.
        app.world_mut()
            .resource_mut::<NovaOsCommandRegistry>()
            .register(
                TerminalCommand::app(MAP_APP_ID, "Open the local-space map", MapApp)
                    .with_subcommand(TerminalCommand::cli(
                        "map view",
                        "Print local-space contacts",
                        CliOutput::Snapshot,
                    ))
                    .with_subcommand(
                        TerminalCommand::gameplay(
                            "map goto",
                            "Fly the ship to a contact label",
                            CommandArity::UpTo(1),
                        )
                        .with_arg_hint("<label>"),
                    ),
            );

        // Scene lifecycle runs unconditionally so it can tear down when the
        // computer closes; the interactive systems gate on the map being active.
        app.add_systems(
            Update,
            (
                assign_map_contact_codes,
                sync_map_arg_completions,
                apply_map_cli_commands,
                manage_map_scene,
                reconcile_map_target,
                map_input,
                map_focus_follow,
                drive_map_camera,
                project_map_blips,
                update_map_readout,
            )
                .chain()
                .in_set(NovaOsMapSystems),
        );
    }
}

/// System set for the map app's per-frame work.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct NovaOsMapSystems;

/// Whether the map app is the active NOVA OS surface right now.
fn map_is_active(pause: &State<PauseStates>, terminal: &NovaOsTerminal) -> bool {
    *pause.get() == PauseStates::NovaOs
        && terminal.active_mode() == TerminalMode::App { id: MAP_APP_ID }
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Spawn the schematic scene + camera on map open, tear it down on close.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_arguments)]
fn manage_map_scene(
    mut commands: Commands,
    pause: Res<State<PauseStates>>,
    terminal: Res<NovaOsTerminal>,
    mut runtime: ResMut<MapRuntime>,
    images: Option<ResMut<Assets<Image>>>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    q_player: Query<&GlobalTransform, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
) {
    let active = map_is_active(&pause, &terminal);
    if active == runtime.active {
        return;
    }
    runtime.active = active;

    if !active {
        // Tear the scene + blips down.
        if let Some(root) = runtime.scene_root.take() {
            commands.entity(root).despawn();
        }
        for (_, blip) in runtime.blips.drain() {
            commands.entity(blip).try_despawn();
        }
        runtime.camera = None;
        runtime.image = None;
        runtime.selected = None;
        runtime.focused_on = None;
        runtime.goto_note = None;
        return;
    }

    // Building the scene needs render assets; headless rigs skip it (the CLI +
    // lifecycle still work). `active` is already recorded above.
    let (Some(mut images), Some(mut meshes), Some(mut materials)) = (images, meshes, materials)
    else {
        return;
    };

    // The map opens framed on the player ship (the sim is frozen, so this stays
    // put); WASD pans the focus from here.
    let focus = q_player
        .iter()
        .next()
        .map(|gt| gt.translation())
        .unwrap_or(Vec3::ZERO);

    let image = images.add(new_map_image(UVec2::splat(64)));
    runtime.image = Some(image.clone());

    let ring_mesh: Vec<Handle<Mesh>> = MAP_RING_RADII
        .iter()
        .map(|r| meshes.add(Torus::new(r - 0.35, r + 0.35)))
        .collect();
    let ring_mat = materials.add(unlit(NOVA_OS_PHOSPHOR_DIM.with_alpha(0.5)));
    let hub_mesh = meshes.add(Sphere::new(1.6));
    let hub_mat = materials.add(unlit(NOVA_OS_PHOSPHOR));

    let scene_root = commands
        .spawn((
            MapSceneRoot,
            Name::new("NovaOsMapScene"),
            Transform::default(),
            Visibility::Visible,
        ))
        .id();

    let camera = commands
        .spawn((
            MapCameraMarker,
            Name::new("NovaOsMapCamera"),
            Camera3d::default(),
            Camera {
                order: MAP_CAMERA_ORDER,
                clear_color: ClearColorConfig::Custom(NOVA_OS_SCREEN),
                is_active: true,
                ..default()
            },
            // RenderTarget is a standalone component in this Bevy version (see the
            // NOVA OS RTT camera), not a `Camera` field.
            RenderTarget::Image(ImageRenderTarget {
                handle: image.clone(),
                scale_factor: 1.0,
            }),
            Transform::from_translation(
                focus + orbit_eye(MAP_RADIUS_DEFAULT, MAP_THETA_DEFAULT, MAP_PHI_DEFAULT),
            )
            .looking_at(focus, Vec3::Y),
            RenderLayers::layer(MAP_LAYER),
            MapOrbit {
                theta: MAP_THETA_DEFAULT,
                phi: MAP_PHI_DEFAULT,
                radius: MAP_RADIUS_DEFAULT,
                // Seed the focus on the player ship; WASD pans it from here.
                center: focus,
            },
            ChildOf(scene_root),
        ))
        .id();
    runtime.camera = Some(camera);

    // The distance rings + central hub live under a focus anchor that tracks the
    // orbit center (the selected object, or the player), so the scale reference
    // always surrounds whatever you are looking at (map_focus_follow moves it).
    let anchor = commands
        .spawn((
            MapFocusAnchor,
            Name::new("NovaOsMapFocus"),
            Transform::from_translation(focus),
            Visibility::Visible,
            ChildOf(scene_root),
        ))
        .id();
    for mesh in ring_mesh {
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(ring_mat.clone()),
            Transform::default(),
            RenderLayers::layer(MAP_LAYER),
            ChildOf(anchor),
        ));
    }
    commands.spawn((
        Mesh3d(hub_mesh),
        MeshMaterial3d(hub_mat),
        Transform::default(),
        RenderLayers::layer(MAP_LAYER),
        ChildOf(anchor),
    ));

    runtime.scene_root = Some(scene_root);
}

/// Keep the offscreen image sized 1:1 to the viewport node and the camera pass
/// active; patch the viewport `ImageNode` with the RTT handle.
#[allow(clippy::type_complexity)]
fn reconcile_map_target(
    runtime: Res<MapRuntime>,
    mut images: Option<ResMut<Assets<Image>>>,
    mut q_viewport: Query<(&ComputedNode, &mut ImageNode), With<MapViewportMarker>>,
    mut q_camera: Query<(&mut Camera, &mut Projection), With<MapCameraMarker>>,
) {
    let (Some(image), Some(images)) = (runtime.image.as_ref(), images.as_mut()) else {
        return;
    };
    let Ok((computed, mut node)) = q_viewport.single_mut() else {
        return;
    };
    if node.image != *image {
        node.image = image.clone();
    }
    let desired = computed.size().round().as_uvec2().max(UVec2::ONE);
    let needs_resize = images
        .get(image)
        .map(|img| img.size() != desired)
        .unwrap_or(true);
    if needs_resize {
        if let Some(mut img) = images.get_mut(image) {
            img.resize(Extent3d {
                width: desired.x,
                height: desired.y,
                depth_or_array_layers: 1,
            });
        }
        // Force the camera to re-derive its target info after the in-place swap
        // (`bevy-camera-ignores-runtime-rendertarget-swap`).
        if let Ok((_, mut projection)) = q_camera.single_mut() {
            projection.set_changed();
        }
    }
}

/// Drive the map camera transform from the orbit output. The orbit `center` is
/// the focus point the player pans with WASD (seeded to the player ship on open,
/// reset with `R`); this system must NOT overwrite it, or WASD would snap back
/// every frame.
fn drive_map_camera(mut q_camera: Query<(&mut Transform, &MapOrbit), With<MapCameraMarker>>) {
    let Ok((mut transform, orbit)) = q_camera.single_mut() else {
        return;
    };
    let eye = orbit.center + orbit_eye(orbit.radius, orbit.theta, orbit.phi);
    *transform = Transform::from_translation(eye).looking_at(orbit.center, Vec3::Y);
}

/// The point the map frames: the selected contact if one is picked, else the
/// player ship.
fn focus_point(contacts: &MapContacts, selected: Option<Entity>) -> Vec3 {
    selected
        .and_then(|sel| {
            contacts
                .collect()
                .into_iter()
                .find(|c| c.entity == sel)
                .map(|c| c.world_pos)
        })
        .unwrap_or_else(|| contacts.focus())
}

/// When a NEW contact is selected, snap the orbit center onto it once (so the
/// map + rings recenter on it); after that WASD is free to pan away. Every frame
/// keep the ring/hub anchor sitting on the current center.
fn map_focus_follow(
    mut runtime: ResMut<MapRuntime>,
    contacts: MapContacts,
    mut q_camera: Query<&mut MapOrbit, With<MapCameraMarker>>,
    mut q_anchor: Query<&mut Transform, With<MapFocusAnchor>>,
) {
    if !runtime.active {
        return;
    }
    let Ok(mut orbit) = q_camera.single_mut() else {
        return;
    };
    if runtime.selected != runtime.focused_on {
        if let Some(sel) = runtime.selected {
            if let Some(pos) = contacts
                .collect()
                .into_iter()
                .find(|c| c.entity == sel)
                .map(|c| c.world_pos)
            {
                orbit.center = pos;
            }
        }
        runtime.focused_on = runtime.selected;
    }
    if let Ok(mut anchor) = q_anchor.single_mut() {
        anchor.translation = orbit.center;
    }
}

/// Read mouse + keyboard while the map owns the screen: RMB-drag look, wheel
/// zoom, WASD move, `R` reset, `[`/`]` cycle selection, `G` set GOTO.
#[allow(clippy::too_many_arguments)]
fn map_input(
    pause: Res<State<PauseStates>>,
    terminal: Res<NovaOsTerminal>,
    mut runtime: ResMut<MapRuntime>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut motion: MessageReader<MouseMotion>,
    mut wheel: MessageReader<MouseWheel>,
    time: Res<Time>,
    contacts: MapContacts,
    mut commands: Commands,
    mut q_camera: Query<(&mut MapOrbit, &Transform), With<MapCameraMarker>>,
) {
    // Only touch input while the map owns the screen; at the terminal the mouse
    // and keys belong to the prompt (history scroll, PageUp/PageDown, etc.).
    if !map_is_active(&pause, &terminal) {
        return;
    }
    let motion_delta: Vec2 = motion.read().map(|m| m.delta).sum();
    let wheel_delta: f32 = wheel.read().map(|w| w.y).sum();
    let dt = time.delta_secs().max(1.0 / 240.0);

    // Decay the transient GOTO note.
    if let Some((_, remaining)) = runtime.goto_note.as_mut() {
        *remaining -= dt;
        if *remaining <= 0.0 {
            runtime.goto_note = None;
        }
    }

    if let Ok((mut orbit, transform)) = q_camera.single_mut() {
        // Keyboard orbit: Q/E turn (yaw), R/F tilt (pitch). This is the reliable
        // path - mouse-drag look is unreliable through the NOVA OS pointer
        // forwarding. Applied straight to the orbit angles (no smoothing layer).
        let turn = 1.6 * dt;
        if keys.pressed(KeyCode::KeyQ) {
            orbit.theta += turn;
        }
        if keys.pressed(KeyCode::KeyE) {
            orbit.theta -= turn;
        }
        if keys.pressed(KeyCode::KeyR) {
            orbit.phi = (orbit.phi + turn).min(1.45);
        }
        if keys.pressed(KeyCode::KeyF) {
            orbit.phi = (orbit.phi - turn).max(0.12);
        }
        // Mouse drag orbits, RIGHT button ONLY. LMB is the contact-select click
        // (the blip `Button` widget), so letting it orbit turned a small
        // press-with-motion into a drag that slid the blip out from under the
        // cursor and ate the selection. Gentle sensitivity so a small drag is a
        // small turn.
        if mouse_buttons.pressed(MouseButton::Right) {
            orbit.theta -= motion_delta.x * 0.0024;
            orbit.phi = (orbit.phi + motion_delta.y * 0.0024).clamp(0.12, 1.45);
        }
        // Wheel zooms the focus distance.
        if wheel_delta != 0.0 {
            orbit.radius =
                (orbit.radius * (1.0 - wheel_delta * 0.12)).clamp(MAP_RADIUS_MIN, MAP_RADIUS_MAX);
        }
        // WASD pans the focus RELATIVE TO THE MAP VIEW (the camera's heading on
        // the ground plane), not the ship: W moves into the screen, D screen-right.
        let mut pan = Vec2::ZERO;
        if keys.pressed(KeyCode::KeyW) {
            pan.y += 1.0;
        }
        if keys.pressed(KeyCode::KeyS) {
            pan.y -= 1.0;
        }
        if keys.pressed(KeyCode::KeyA) {
            pan.x -= 1.0;
        }
        if keys.pressed(KeyCode::KeyD) {
            pan.x += 1.0;
        }
        if pan != Vec2::ZERO {
            let flatten = |v: Vec3| Vec3::new(v.x, 0.0, v.z).normalize_or_zero();
            let forward = flatten(*transform.forward());
            let right = flatten(*transform.right());
            let speed = orbit.radius * 0.8 * dt;
            orbit.center += (forward * pan.y + right * pan.x) * speed;
        }
        // T re-frames on the selected object (or the player if nothing is picked).
        if keys.just_pressed(KeyCode::KeyT) {
            orbit.radius = MAP_RADIUS_DEFAULT;
            orbit.theta = MAP_THETA_DEFAULT;
            orbit.phi = MAP_PHI_DEFAULT;
            orbit.center = focus_point(&contacts, runtime.selected);
            runtime.focused_on = runtime.selected;
        }
    }

    // Cycle selection with [ and ].
    let list = contacts.collect();
    if !list.is_empty() {
        let forward = keys.just_pressed(KeyCode::BracketRight);
        let backward = keys.just_pressed(KeyCode::BracketLeft);
        if forward || backward {
            let current = runtime
                .selected
                .and_then(|sel| list.iter().position(|c| c.entity == sel));
            let len = list.len();
            let next = match current {
                Some(i) if forward => (i + 1) % len,
                Some(i) => (i + len - 1) % len,
                None => 0,
            };
            runtime.selected = Some(list[next].entity);
        }
    }

    // GOTO on the selected contact (skip own ship). Sets a flight autopilot on
    // the player ship directly - this intentionally bypasses the normal
    // `FlightVerb::Goto` grant check (fine for the PoC nav computer).
    if keys.just_pressed(KeyCode::KeyG) {
        if let (Some(sel), Some((player, _, _))) = (runtime.selected, contacts.player_frame()) {
            if let Some(contact) = list.iter().find(|c| c.entity == sel) {
                if contact.kind != MapContactKind::OwnShip {
                    commands
                        .entity(player)
                        .insert(Autopilot::engage(AutopilotAction::Goto { target: sel }));
                    runtime.goto_note = Some((format!("GOTO SET: {}", contact.name), 2.5));
                }
            }
        }
    }
}

/// Project each contact through the map camera into the viewport and keep a
/// clickable UI blip per contact in sync (position, color, selection ring).
#[allow(clippy::type_complexity)]
fn project_map_blips(
    mut commands: Commands,
    mut runtime: ResMut<MapRuntime>,
    ui_font: Option<Res<UiFont>>,
    contacts: MapContacts,
    time: Res<Time>,
    q_camera: Query<(&Camera, &GlobalTransform), With<MapCameraMarker>>,
    q_viewport: Query<(Entity, &ComputedNode), With<MapViewportMarker>>,
    mut q_blip: Query<(
        &mut Node,
        &mut Visibility,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
) {
    if !runtime.active {
        return;
    }
    let (Ok((camera, cam_gt)), Ok((viewport, computed))) = (q_camera.single(), q_viewport.single())
    else {
        return;
    };
    let size = computed.size();
    let list = contacts.collect();
    let font = nova_os_font(ui_font.as_deref());
    let pulse = 0.6 + 0.4 * (time.elapsed_secs() * 4.0).sin().abs();

    let mut seen = bevy::platform::collections::HashSet::new();
    for contact in &list {
        seen.insert(contact.entity);
        let projected = camera
            .world_to_viewport(cam_gt, contact.world_pos)
            .ok()
            .filter(|p| p.x >= 0.0 && p.y >= 0.0 && p.x <= size.x && p.y <= size.y);
        let selected = runtime.selected == Some(contact.entity);
        let mut base = contact.kind.color();
        if contact.kind == MapContactKind::Hostile {
            base = base.with_alpha(pulse);
        }

        let blip = if let Some(&blip) = runtime.blips.get(&contact.entity) {
            blip
        } else {
            let id = spawn_blip(&mut commands, viewport, contact, font.clone());
            runtime.blips.insert(contact.entity, id);
            id
        };
        if let Ok((mut node, mut vis, mut bg, mut border)) = q_blip.get_mut(blip) {
            match projected {
                Some(p) => {
                    node.left = Val::Px(p.x - MAP_BLIP_PX * 0.5);
                    node.top = Val::Px(p.y - MAP_BLIP_PX * 0.5);
                    *vis = Visibility::Inherited;
                }
                None => *vis = Visibility::Hidden,
            }
            bg.0 = base;
            *border = if selected {
                BorderColor::all(NOVA_OS_AMBER)
            } else {
                BorderColor::all(base.with_alpha(0.0))
            };
        }
    }

    // Drop blips whose contact vanished.
    let stale: Vec<Entity> = runtime
        .blips
        .keys()
        .copied()
        .filter(|c| !seen.contains(c))
        .collect();
    for contact in stale {
        if let Some(blip) = runtime.blips.remove(&contact) {
            commands.entity(blip).try_despawn();
        }
    }
}

/// Blip square side in pixels (border box), and its border width.
const MAP_BLIP_PX: f32 = 12.0;
const MAP_BLIP_BORDER_PX: f32 = 2.0;

/// Where the label pill starts, measured from the dot's PADDING edge - which is
/// where an absolutely-positioned child's `left` is measured from, i.e. already
/// inside the dot's border. Offsetting by the border width lands the pill exactly
/// on the dot's outer right edge, so the two are one unbroken hit target.
const MAP_LABEL_LEFT_PX: f32 = MAP_BLIP_PX - MAP_BLIP_BORDER_PX;

fn spawn_blip(
    commands: &mut Commands,
    viewport: Entity,
    contact: &MapContact,
    font: Handle<Font>,
) -> Entity {
    let color = contact.kind.color();
    let id = commands
        .spawn((
            MapBlip {
                contact: contact.entity,
            },
            Button,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(MAP_BLIP_PX),
                height: Val::Px(MAP_BLIP_PX),
                border: UiRect::all(Val::Px(MAP_BLIP_BORDER_PX)),
                // Round the blip into a dot rather than a square.
                border_radius: BorderRadius::MAX,
                ..default()
            },
            BorderColor::all(color.with_alpha(0.0)),
            BackgroundColor(color),
        ))
        // Selection goes through the Button `Activate` event (fires for the
        // forwarded NOVA OS pointer), not `Interaction` polling, which does not
        // update through the CRT-composited RTT.
        .observe(on_map_blip_click)
        .id();
    // The label rides beside the blip as a child node, in a dark backing pill -
    // the same shape the ship app's section labels use, and for the same two
    // reasons (task 20260730-123039): it reads clearly against the phosphor
    // scene, and it is a SOLID hit target rather than a box tight to the glyph
    // run. `Pointer<Click>` bubbles, so a click anywhere on the pill activates
    // the blip `Button` it is a child of.
    //
    // It starts at exactly the dot's right edge (see [`MAP_LABEL_LEFT_PX`]), so
    // dot and label are one unbroken target: the old `left: 16` left a 6 px dead
    // band between them that selected nothing. The 1 px vertical padding under
    // `top: -4` keeps the glyph baseline exactly where `top: -3` put it; the
    // glyphs shift 2 px left (18 -> 16 px from the dot's left edge), which is the
    // whole visual change.
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(MAP_LABEL_LEFT_PX),
            top: Val::Px(-4.0),
            padding: UiRect::axes(Val::Px(4.0), Val::Px(1.0)),
            border_radius: BorderRadius::all(Val::Px(3.0)),
            ..default()
        },
        BackgroundColor(NOVA_OS_SCREEN.with_alpha(0.82)),
        ChildOf(id),
        children![(
            // The blip carries its unique CODE (the `map goto <label>` handle),
            // not the freeform name, so the label you read is the label you type.
            Text::new(contact.code.clone()),
            nova_os_text_font(11.0, font),
            TextColor(color),
        )],
    ));
    commands.entity(viewport).add_child(id);
    id
}

/// Select a contact when its blip button is activated (click through the
/// forwarded NOVA OS pointer, or keyboard activation).
fn on_map_blip_click(
    activate: On<Activate>,
    q_blip: Query<&MapBlip>,
    mut runtime: ResMut<MapRuntime>,
) {
    if let Ok(blip) = q_blip.get(activate.entity) {
        runtime.selected = Some(blip.contact);
    }
}

/// Fill the readout from the current selection (or a GOTO flash).
fn update_map_readout(
    runtime: Res<MapRuntime>,
    contacts: MapContacts,
    mut q_readout: Query<(&mut Text, &mut TextColor), With<MapReadoutMarker>>,
) {
    if !runtime.active {
        return;
    }
    let Ok((mut text, mut color)) = q_readout.single_mut() else {
        return;
    };
    if let Some((note, _)) = &runtime.goto_note {
        text.0 = note.clone();
        color.0 = NOVA_OS_AMBER;
        return;
    }
    match runtime
        .selected
        .and_then(|sel| contacts.collect().into_iter().find(|c| c.entity == sel))
    {
        Some(contact) => {
            text.0 = contact.readout();
            color.0 = if contact.kind == MapContactKind::Hostile {
                NOVA_OS_AMBER
            } else {
                NOVA_OS_TEXT
            };
        }
        None => {
            text.0 = "Select a contact for range and bearing.".to_string();
            color.0 = NOVA_OS_PHOSPHOR_MUTED;
        }
    }
}

// ---------------------------------------------------------------------------
// Small render helpers
// ---------------------------------------------------------------------------

fn new_map_image(size: UVec2) -> Image {
    Image::new_target_texture(
        size.x.max(1),
        size.y.max(1),
        TextureFormat::Rgba8UnormSrgb,
        None,
    )
}

/// An unlit emissive-ish material so proxy meshes read at full color without a
/// light on the map layer.
fn unlit(color: Color) -> StandardMaterial {
    StandardMaterial {
        base_color: color,
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    }
}

#[cfg(test)]
mod tests {
    use bevy::{
        ecs::system::RunSystemOnce,
        state::app::StatesPlugin,
        ui::{ComputedNode, UiGlobalTransform},
    };

    use super::*;
    use crate::hud::nova_os_pointer_rig::{
        click_at, glass_px, glass_uv_showing, image_px_shown_at, nova_os_pointer_rig,
        pointer_image_px, settle, NovaOsPointerRig,
    };

    /// The map readout and INFO cell render range through the shared
    /// player-facing distance policy (1 world unit = 10 m), not raw `u`.
    #[test]
    fn map_range_renders_in_metres_and_kilometres() {
        let entity = Entity::PLACEHOLDER;
        // 50 world units = 500 m (below the km threshold).
        let near = MapContact {
            entity,
            kind: MapContactKind::Hostile,
            code: "HOST-1".to_string(),
            name: "RAIDER".to_string(),
            world_pos: Vec3::ZERO,
            range: 50.0,
            bearing_deg: 0.0,
            mark_deg: 0.0,
        };
        assert!(
            near.readout().contains("range 500 m,"),
            "near readout: {}",
            near.readout()
        );
        assert!(
            near.info_cell().starts_with("500 m  "),
            "near info cell: {}",
            near.info_cell()
        );

        // 150 world units = 1500 m -> 1.50 km.
        let far = MapContact {
            range: 150.0,
            ..near.clone()
        };
        assert!(
            far.readout().contains("range 1.50 km,"),
            "far readout: {}",
            far.readout()
        );

        // The own ship's zero-range placeholder also uses the new unit.
        let own = MapContact {
            kind: MapContactKind::OwnShip,
            range: 0.0,
            ..near
        };
        assert!(own.readout().contains("range 0 m,"), "{}", own.readout());
        assert_eq!(own.info_cell(), "range 0 m");
    }

    /// Spawn a scripted local-space scene: own ship at origin (facing -Z), a
    /// hostile dead ahead, an objective to starboard, an asteroid astern.
    fn scripted_world() -> (World, Entity, Entity) {
        let mut world = World::new();
        let player = world
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.0)),
                Name::new("NOVA"),
            ))
            .id();
        let raider = world
            .spawn((
                SpaceshipRootMarker,
                Allegiance::Enemy,
                GlobalTransform::from(Transform::from_xyz(0.0, 0.0, -50.0)),
                Name::new("RAIDER"),
            ))
            .id();
        world.spawn((
            ObjectiveMarkerTarget {
                label: "salvage".to_string(),
            },
            GlobalTransform::from(Transform::from_xyz(50.0, 0.0, 0.0)),
        ));
        world.spawn((
            EntityTypeName::new("asteroid"),
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 60.0)),
        ));
        (world, player, raider)
    }

    #[test]
    fn map_contacts_report_kinds_range_and_bearing() {
        let (mut world, _player, raider) = scripted_world();
        let contacts = world.run_system_once(|c: MapContacts| c.collect()).unwrap();

        // Own ship is enumerated first.
        assert_eq!(contacts[0].kind, MapContactKind::OwnShip);
        assert_eq!(contacts[0].range, 0.0);

        let find = |kind: MapContactKind| contacts.iter().find(|c| c.kind == kind).unwrap();
        let hostile = find(MapContactKind::Hostile);
        assert_eq!(hostile.entity, raider);
        assert!((hostile.range - 50.0).abs() < 0.01);
        // Dead ahead (-Z) reads bearing ~0.
        assert!(hostile.bearing_deg < 1.0 || hostile.bearing_deg > 359.0);

        let objective = find(MapContactKind::Objective);
        assert!((objective.range - 50.0).abs() < 0.01);
        // Starboard (+X) reads ~090.
        assert!((objective.bearing_deg - 90.0).abs() < 1.0);

        let asteroid = find(MapContactKind::Terrain);
        assert!((asteroid.range - 60.0).abs() < 0.01);
        // Astern (+Z) reads ~180.
        assert!((asteroid.bearing_deg - 180.0).abs() < 1.0);
    }

    #[test]
    fn map_view_rows_render_contacts_and_empty_state() {
        let (mut world, _player, _raider) = scripted_world();
        let contacts = world.run_system_once(|c: MapContacts| c.collect()).unwrap();
        let rows = map_rows_from_contacts(&contacts);
        let joined: String = rows
            .iter()
            .map(|r| r.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(joined.contains("LOCAL SPACE"));
        assert!(joined.contains("HOSTILE"));
        assert!(joined.contains("RAIDER"));
        assert!(joined.contains("OBJECTIVE"));

        // With only the own ship, the CLI reports no contacts.
        let own_only: Vec<MapContact> = contacts
            .into_iter()
            .filter(|c| c.kind == MapContactKind::OwnShip)
            .collect();
        let empty_rows = map_rows_from_contacts(&own_only);
        assert!(empty_rows.iter().any(|r| r.text.contains("no contacts")));
    }

    /// A denser world: two hostiles + two asteroids + one objective, so the
    /// per-kind indices actually count up and can collide if minting is wrong.
    fn crowded_world() -> World {
        let mut world = World::new();
        world.spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.0)),
            Name::new("NOVA"),
        ));
        for z in [-40.0, -80.0] {
            world.spawn((
                SpaceshipRootMarker,
                Allegiance::Enemy,
                GlobalTransform::from(Transform::from_xyz(0.0, 0.0, z)),
                Name::new("RAIDER"),
            ));
        }
        for x in [30.0, 70.0] {
            world.spawn((
                EntityTypeName::new("asteroid"),
                GlobalTransform::from(Transform::from_xyz(x, 0.0, 0.0)),
            ));
        }
        world.spawn((
            ObjectiveMarkerTarget {
                label: "salvage".to_string(),
            },
            GlobalTransform::from(Transform::from_xyz(0.0, 40.0, 0.0)),
        ));
        world
    }

    #[test]
    fn map_contact_codes_are_unique_and_stable() {
        let mut world = crowded_world();
        // Mint codes, then read them back off the contact model.
        world.run_system_once(assign_map_contact_codes).unwrap();
        let contacts = world.run_system_once(|c: MapContacts| c.collect()).unwrap();

        let codes: Vec<String> = contacts.iter().map(|c| c.code.clone()).collect();
        let unique: std::collections::HashSet<&String> = codes.iter().collect();
        assert_eq!(
            unique.len(),
            codes.len(),
            "every contact code is unique: {codes:?}"
        );

        // The own ship is the bare SELF; each other kind counts from 1.
        assert!(codes.contains(&"SELF".to_string()));
        assert!(codes.contains(&"HOST-1".to_string()) && codes.contains(&"HOST-2".to_string()));
        assert!(codes.contains(&"AST-1".to_string()) && codes.contains(&"AST-2".to_string()));
        assert!(codes.contains(&"OBJ-1".to_string()));

        // Re-running the pass must NOT reassign or add codes (stable per session).
        world.run_system_once(assign_map_contact_codes).unwrap();
        let again = world.run_system_once(|c: MapContacts| c.collect()).unwrap();
        let mut before = codes;
        let mut after: Vec<String> = again.iter().map(|c| c.code.clone()).collect();
        before.sort();
        after.sort();
        assert_eq!(before, after, "codes are stable across minting passes");
    }

    #[test]
    fn map_view_table_aligns_kind_label_info_columns() {
        let mut world = crowded_world();
        world.run_system_once(assign_map_contact_codes).unwrap();
        let contacts = world.run_system_once(|c: MapContacts| c.collect()).unwrap();
        let printed: Vec<String> = map_rows_from_contacts(&contacts)
            .into_iter()
            .map(|r| r.text)
            .collect();

        let header = printed
            .iter()
            .find(|r| r.starts_with("KIND"))
            .expect("a KIND/LABEL/INFO header row");
        assert!(header.contains("LABEL") && header.contains("INFO"));

        // Columns line up: the LABEL token starts at the SAME offset in the header
        // and in a data row (mirrors the `ship view` alignment assertion).
        let label_col = header.find("LABEL").unwrap();
        let hostile_row = printed
            .iter()
            .find(|r| r.starts_with("HOSTILE"))
            .expect("a hostile data row");
        assert!(
            hostile_row[label_col..].starts_with("HOST-"),
            "LABEL column is aligned: {hostile_row:?}",
        );
    }

    /// Register the `map`/`map goto` command tree into a bare terminal so `submit`
    /// queues the gameplay invocation the handler drains.
    fn terminal_with_map_goto() -> NovaOsTerminal {
        use nova_os::shell::{CommandArity, CommandDispatch, TerminalCommandSpec};
        let mut terminal = NovaOsTerminal::default();
        let mut specs = terminal.command_specs().to_vec();
        specs.push(TerminalCommandSpec {
            name: "map",
            summary: "Open the local-space map",
            arity: CommandArity::None,
            arg_hint: None,
            dispatch: CommandDispatch::App,
        });
        specs.push(TerminalCommandSpec {
            name: "map goto",
            summary: "Fly the ship to a contact label",
            arity: CommandArity::UpTo(1),
            arg_hint: Some("<label>"),
            dispatch: CommandDispatch::Gameplay,
        });
        terminal.set_commands(specs);
        terminal
    }

    #[test]
    fn map_goto_engages_autopilot_and_rejects_self_and_unknown() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        let player = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.0)),
                Name::new("NOVA"),
                MapContactCode("SELF".to_string()),
            ))
            .id();
        let raider = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                Allegiance::Enemy,
                GlobalTransform::from(Transform::from_xyz(0.0, 0.0, -50.0)),
                Name::new("RAIDER"),
                MapContactCode("HOST-1".to_string()),
            ))
            .id();

        app.insert_resource(terminal_with_map_goto());

        let submit = |app: &mut App, line: &str| {
            let mut terminal = app.world_mut().resource_mut::<NovaOsTerminal>();
            terminal.reset_prompt();
            terminal.insert_text(line);
            terminal.submit(&TerminalCommandSnapshot::default());
            app.world_mut()
                .run_system_once(apply_map_cli_commands)
                .unwrap();
        };

        // A real contact (case-insensitive): the autopilot targets the raider.
        submit(&mut app, "map goto host-1");
        let autopilot = app
            .world()
            .get::<Autopilot>(player)
            .expect("goto inserts an Autopilot on the player ship");
        assert!(
            matches!(autopilot.action, AutopilotAction::Goto { target } if target == raider),
            "the autopilot targets the labelled contact",
        );

        // Own ship: rejected, no autopilot change. Clear the autopilot first so we
        // can prove the SELF path does not set a new one.
        app.world_mut().entity_mut(player).remove::<Autopilot>();
        submit(&mut app, "map goto SELF");
        assert!(
            app.world().get::<Autopilot>(player).is_none(),
            "goto SELF must not engage an autopilot",
        );

        // Unknown label: rejected with an error row, still no autopilot.
        submit(&mut app, "map goto ZZZ");
        assert!(app.world().get::<Autopilot>(player).is_none());
        let printed: Vec<String> = app
            .world()
            .resource::<NovaOsTerminal>()
            .scrollback()
            .iter()
            .map(|r| r.text.clone())
            .collect();
        assert!(
            printed.iter().any(|r| r.contains("no such contact")),
            "an unknown label prints a not-found row: {printed:?}",
        );
    }

    /// The scene lifecycle tracks the active NOVA OS surface (headless: no
    /// render assets, so only the active flag toggles - the scene build is
    /// skipped, but open/close is proven).
    #[test]
    fn map_scene_activates_with_the_app_surface() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin));
        app.insert_state(PauseStates::NovaOs);
        app.init_resource::<MapRuntime>();
        app.insert_resource(NovaOsTerminal::default());

        // At the prompt: inactive.
        app.update();
        app.world_mut().run_system_once(manage_map_scene).unwrap();
        assert!(!app.world().resource::<MapRuntime>().active);

        // Launch the map app: active.
        app.world_mut()
            .resource_mut::<NovaOsTerminal>()
            .enter_app(MAP_APP_ID);
        app.world_mut().run_system_once(manage_map_scene).unwrap();
        assert!(app.world().resource::<MapRuntime>().active);

        // Exit back to the terminal: inactive again.
        app.world_mut().resource_mut::<NovaOsTerminal>().exit_app();
        app.world_mut().run_system_once(manage_map_scene).unwrap();
        assert!(!app.world().resource::<MapRuntime>().active);
    }

    /// With the asset stores present, opening the map actually builds the
    /// schematic scene (camera + proxy meshes + RTT image) and the per-frame
    /// systems run without panicking - the path a real GPU would render.
    #[test]
    fn map_scene_builds_and_drives_with_render_assets() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin, AssetPlugin::default()));
        app.init_asset::<Image>();
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
        app.insert_state(PauseStates::NovaOs);
        app.init_resource::<MapRuntime>();

        let mut terminal = NovaOsTerminal::default();
        terminal.enter_app(MAP_APP_ID);
        app.insert_resource(terminal);

        app.world_mut().spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.0)),
            Name::new("NOVA"),
        ));

        // Build the scene.
        app.world_mut().run_system_once(manage_map_scene).unwrap();
        {
            let runtime = app.world().resource::<MapRuntime>();
            assert!(runtime.active);
            assert!(runtime.scene_root.is_some(), "scene root spawned");
            assert!(runtime.image.is_some(), "RTT image created");
            assert!(runtime.camera.is_some(), "map camera spawned");
        }
        // A camera entity carries the render layer + orbit.
        let cameras = app
            .world_mut()
            .query_filtered::<(), (With<MapCameraMarker>, With<MapOrbit>)>()
            .iter(app.world())
            .count();
        assert_eq!(cameras, 1, "exactly one orbit map camera");

        // The per-frame systems run without panicking (no viewport UI node here,
        // so projection/reconcile early-return, but the code path is exercised).
        app.world_mut()
            .run_system_once(reconcile_map_target)
            .unwrap();
        app.world_mut().run_system_once(drive_map_camera).unwrap();
        app.world_mut().run_system_once(project_map_blips).unwrap();

        // Closing the app tears the scene down.
        app.world_mut().resource_mut::<NovaOsTerminal>().exit_app();
        app.world_mut().run_system_once(manage_map_scene).unwrap();
        assert!(app.world().resource::<MapRuntime>().scene_root.is_none());
        let remaining = app
            .world_mut()
            .query_filtered::<(), With<MapCameraMarker>>()
            .iter(app.world())
            .count();
        assert_eq!(remaining, 0, "camera despawned on close");
    }

    #[test]
    fn map_focus_follow_recenters_on_a_new_selection() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin, AssetPlugin::default()));
        app.init_asset::<Image>();
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
        app.insert_state(PauseStates::NovaOs);
        app.init_resource::<MapRuntime>();

        let mut terminal = NovaOsTerminal::default();
        terminal.enter_app(MAP_APP_ID);
        app.insert_resource(terminal);

        app.world_mut().spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.0)),
            Name::new("NOVA"),
        ));
        let raider = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                Allegiance::Enemy,
                GlobalTransform::from(Transform::from_xyz(90.0, 0.0, -30.0)),
                Name::new("RAIDER"),
            ))
            .id();

        // Build the scene (framed on the player).
        app.world_mut().run_system_once(manage_map_scene).unwrap();

        // Select the raider: the orbit center + ring anchor snap onto it.
        app.world_mut().resource_mut::<MapRuntime>().selected = Some(raider);
        app.world_mut().run_system_once(map_focus_follow).unwrap();

        let center = app
            .world_mut()
            .query_filtered::<&MapOrbit, With<MapCameraMarker>>()
            .single(app.world())
            .unwrap()
            .center;
        assert!(
            center.distance(Vec3::new(90.0, 0.0, -30.0)) < 0.01,
            "the map recenters on the selected contact",
        );
        let anchor = app
            .world_mut()
            .query_filtered::<&Transform, With<MapFocusAnchor>>()
            .single(app.world())
            .unwrap()
            .translation;
        assert!(
            anchor.distance(Vec3::new(90.0, 0.0, -30.0)) < 0.01,
            "the ring anchor follows the focus",
        );
    }

    #[test]
    fn map_goto_sets_autopilot_on_the_player_ship() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin, bevy::input::InputPlugin));
        app.insert_state(PauseStates::NovaOs);
        app.init_resource::<MapRuntime>();

        let mut terminal = NovaOsTerminal::default();
        terminal.enter_app(MAP_APP_ID);
        app.insert_resource(terminal);

        let player = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.0)),
                Name::new("NOVA"),
            ))
            .id();
        let target = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                Allegiance::Enemy,
                GlobalTransform::from(Transform::from_xyz(0.0, 0.0, -50.0)),
                Name::new("RAIDER"),
            ))
            .id();

        {
            let mut runtime = app.world_mut().resource_mut::<MapRuntime>();
            runtime.active = true;
            runtime.selected = Some(target);
        }
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyG);

        app.world_mut().run_system_once(map_input).unwrap();

        let autopilot = app
            .world()
            .get::<Autopilot>(player)
            .expect("GOTO inserts an Autopilot on the player ship");
        assert!(
            matches!(autopilot.action, AutopilotAction::Goto { target: t } if t == target),
            "the autopilot targets the selected contact",
        );
    }

    /// LMB is the contact-SELECT click (the blip `Button` widget's Primary
    /// activation), so it must NOT orbit-drag the map camera - otherwise a small
    /// press-with-motion drags the view and the blip slips out from under the
    /// cursor before the click lands. RMB stays the orbit-drag button.
    #[test]
    fn map_orbit_drag_is_rmb_only() {
        use bevy::input::mouse::MouseMotion;

        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            StatesPlugin,
            AssetPlugin::default(),
            bevy::input::InputPlugin,
        ));
        app.init_asset::<Image>();
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
        app.insert_state(PauseStates::NovaOs);
        app.init_resource::<MapRuntime>();

        let mut terminal = NovaOsTerminal::default();
        terminal.enter_app(MAP_APP_ID);
        app.insert_resource(terminal);

        app.world_mut().spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.0)),
            Name::new("NOVA"),
        ));
        app.world_mut().run_system_once(manage_map_scene).unwrap();

        let orbit_angles = |app: &mut App| {
            app.world_mut()
                .query_filtered::<&MapOrbit, With<MapCameraMarker>>()
                .single(app.world())
                .map(|o| (o.theta, o.phi))
                .unwrap()
        };
        let before = orbit_angles(&mut app);

        // Hold LMB and sweep the mouse: the camera must not orbit.
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.world_mut().write_message(MouseMotion {
            delta: Vec2::new(60.0, 40.0),
        });
        app.world_mut().run_system_once(map_input).unwrap();
        assert_eq!(
            orbit_angles(&mut app),
            before,
            "LMB drag must NOT orbit the map camera (it selects contacts)"
        );

        // Hold RMB and sweep the same delta: the camera must orbit.
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .release(MouseButton::Left);
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Right);
        app.world_mut().write_message(MouseMotion {
            delta: Vec2::new(60.0, 40.0),
        });
        app.world_mut().run_system_once(map_input).unwrap();
        assert_ne!(
            orbit_angles(&mut app),
            before,
            "RMB drag must still orbit the map camera"
        );
    }

    // -----------------------------------------------------------------------
    // Clicking contacts through the CRT composite (task 20260730-123039)
    // -----------------------------------------------------------------------

    /// Stand a map viewport up inside the rig's through-image content root,
    /// clipped exactly as the app body's is, and return it.
    fn rig_map_viewport(rig: &mut NovaOsPointerRig) -> Entity {
        let viewport = rig
            .app
            .world_mut()
            .spawn((
                MapViewportMarker,
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(0.0),
                    left: Val::Px(0.0),
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    // The production body clips its viewport; a fix that only
                    // works on an unclipped viewport is not a fix.
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(MAP_VIEW_BG),
            ))
            .id();
        rig.app
            .world_mut()
            .entity_mut(rig.content_root)
            .add_child(viewport);
        viewport
    }

    fn rig_contact(entity: Entity, code: &str) -> MapContact {
        MapContact {
            entity,
            kind: MapContactKind::Hostile,
            code: code.to_string(),
            name: code.to_string(),
            world_pos: Vec3::ZERO,
            range: 100.0,
            bearing_deg: 0.0,
            mark_deg: 0.0,
        }
    }

    /// Put a real map blip (the production `spawn_blip` markup and its real
    /// `Activate` observer) with its DOT centred on `image_px`.
    fn rig_place_blip(
        rig: &mut NovaOsPointerRig,
        viewport: Entity,
        contact: &MapContact,
        image_px: Vec2,
    ) -> Entity {
        let blip = rig
            .app
            .world_mut()
            .run_system_once_with(
                |input: In<(Entity, MapContact)>, mut commands: Commands| {
                    let (viewport, contact) = input.0;
                    spawn_blip(&mut commands, viewport, &contact, Handle::default())
                },
                (viewport, contact.clone()),
            )
            .expect("spawning a blip through the production path");
        let mut node = rig
            .app
            .world_mut()
            .get_mut::<Node>(blip)
            .expect("the blip has a Node");
        node.left = Val::Px(image_px.x - MAP_BLIP_PX * 0.5);
        node.top = Val::Px(image_px.y - MAP_BLIP_PX * 0.5);
        settle(&mut rig.app);
        blip
    }

    /// DoD 1: a click at a known place on the glass selects the contact the CRT
    /// is DISPLAYING there - in the middle of the viewport and out in a corner.
    ///
    /// This is the failing test the bug was found with. Before the fix the
    /// forwarded pointer applied the barrel INVERSE and ignored the shader's
    /// overscan entirely, so the corner case selected nothing (the pointer landed
    /// ~27 px away in x, more than twice the 12 px blip) while the centre case
    /// passed - exactly the asymmetry the owner reported between the map (blips
    /// spread across the viewport) and the ship app (blips clustered mid-screen).
    ///
    /// Each case also carries a DECOY contact at the other test point, so
    /// "selected something" cannot pass for "selected the right thing".
    #[test]
    fn map_contacts_select_where_the_crt_shows_them() {
        let centre_uv = Vec2::splat(0.5);
        // Far enough into the corner that the barrel residual is at its worst,
        // and still comfortably inside the glass.
        let corner_uv = Vec2::new(0.04, 0.05);

        for (what, aim_uv, decoy_uv) in [
            ("centre of the viewport", centre_uv, corner_uv),
            ("corner of the viewport", corner_uv, centre_uv),
        ] {
            let mut rig = nova_os_pointer_rig();
            rig.app.init_resource::<MapRuntime>();
            let viewport = rig_map_viewport(&mut rig);

            let target_id = rig.app.world_mut().spawn_empty().id();
            let decoy_id = rig.app.world_mut().spawn_empty().id();
            let target = rig_contact(target_id, "HOST-1");
            let decoy = rig_contact(decoy_id, "HOST-2");
            rig_place_blip(&mut rig, viewport, &target, image_px_shown_at(aim_uv));
            rig_place_blip(&mut rig, viewport, &decoy, image_px_shown_at(decoy_uv));

            click_at(&mut rig, glass_px(aim_uv));

            let selected = rig.app.world().resource::<MapRuntime>().selected;
            let intended = image_px_shown_at(aim_uv);
            assert_eq!(
                selected,
                Some(target_id),
                "clicking the {what} must select the contact the CRT shows there: \
                 the forwarded pointer sat on image px {:?}, the blip is centred on \
                 {intended:?}",
                pointer_image_px(&rig),
            );

            // ...and it must select it by landing on the DOT, not by drifting onto
            // some other part of the target. Without this the label pill's own
            // (deliberate) generosity would absorb a mis-mapped pointer and the
            // selection assertion above would pass right through the bug.
            let landed = pointer_image_px(&rig).expect("the pointer is on the image");
            assert!(
                landed.distance(intended) <= MAP_BLIP_PX * 0.5,
                "clicking the {what}, the forwarded pointer landed on image px \
                 {landed:?}, {:.1} px from the {intended:?} the CRT displays there - \
                 outside the {MAP_BLIP_PX} px dot it was aimed at",
                landed.distance(intended),
            );
        }
    }

    /// The laid-out border box of a node in image space.
    fn rig_rect(rig: &NovaOsPointerRig, entity: Entity) -> Rect {
        let world = rig.app.world();
        let node = world
            .get::<ComputedNode>(entity)
            .unwrap_or_else(|| panic!("{entity:?} never reached UI layout"));
        let xf = world
            .get::<UiGlobalTransform>(entity)
            .unwrap_or_else(|| panic!("{entity:?} has no UI transform"));
        Rect::from_center_size(xf.translation, node.size())
    }

    /// The blip's label node - its only child.
    fn rig_label_of(rig: &NovaOsPointerRig, blip: Entity) -> Entity {
        let children = rig
            .app
            .world()
            .get::<Children>(blip)
            .expect("the blip has a label child");
        assert_eq!(
            children.len(),
            1,
            "the blip's hit target is its dot plus ONE label child"
        );
        children[0]
    }

    /// DoD 3: the label is as clickable as the dot, which means the two targets
    /// TOUCH. The owner's comparison - "on labels, clicks work 99% of the time"
    /// in the ship app - is the bar, and the ship app's label is a padded backing
    /// pill starting 2 px from its dot. The map's bare text node started 4 px out
    /// with no padding of its own, leaving a dead band between dot and label that
    /// selects nothing, and a target tight to the glyph run on every side.
    ///
    /// Both halves are read from the LIVE tree and clicked through the real
    /// composite, so neither the gap nor the padding can be satisfied on paper.
    #[test]
    fn map_contact_label_and_dot_are_one_unbroken_target() {
        let aim_uv = Vec2::new(0.35, 0.45);
        let mut rig = nova_os_pointer_rig();
        rig.app.init_resource::<MapRuntime>();
        let viewport = rig_map_viewport(&mut rig);

        let target_id = rig.app.world_mut().spawn_empty().id();
        let contact = rig_contact(target_id, "HOST-1");
        let blip = rig_place_blip(&mut rig, viewport, &contact, image_px_shown_at(aim_uv));
        let label = rig_label_of(&rig, blip);

        let dot = rig_rect(&rig, blip);
        let label_box = rig_rect(&rig, label);
        assert!(
            label_box.min.x <= dot.max.x + 0.01,
            "the label starts at x {} but the dot ends at x {} - {:.1} px of dead \
             band between the two halves of one target",
            label_box.min.x,
            dot.max.x,
            label_box.min.x - dot.max.x,
        );

        // A solid, padded backing box like the ship app's pill, not a box tight to
        // the glyph run: the label must be taller than its own text.
        let label_frame = {
            let computed = rig
                .app
                .world()
                .get::<ComputedNode>(label)
                .expect("the label reached UI layout");
            computed.padding.min_inset
                + computed.padding.max_inset
                + computed.border.min_inset
                + computed.border.max_inset
        };
        assert!(
            label_frame.x > 0.0 && label_frame.y > 0.0,
            "the label carries no padding of its own ({label_frame:?}) - its hit \
             target is tight to the glyph run, unlike the ship app's pill",
        );
        assert!(
            rig.app.world().get::<BackgroundColor>(label).is_some(),
            "the label has no backing fill, so there is nothing solid to aim at",
        );

        // Every point across the seam - dot centre, the old dead band, label
        // centre - selects the contact. Positions come from the live rects, and
        // the glass position from the shader reference, never the production map.
        // Sweep the whole band from the dot's middle, across the old dead gap,
        // into the pill - at pixel CENTRES, since the shared edge itself is a
        // measure-zero boundary bevy's `contains_point` excludes from both rects.
        // Every one of these used to be dead between x=dot.max and x=dot.max+6.
        let y = dot.center().y;
        let mut x = dot.center().x + 0.5;
        let last = label_box.min.x + 6.0;
        let mut probed = 0;
        while x <= last {
            let image_px = Vec2::new(x, y);
            rig.app.world_mut().resource_mut::<MapRuntime>().selected = None;
            click_at(&mut rig, glass_px(glass_uv_showing(image_px)));
            assert_eq!(
                rig.app.world().resource::<MapRuntime>().selected,
                Some(target_id),
                "clicking image px {image_px:?} - between the dot's centre and 6 px \
                 into the label - must select the contact; the pointer landed on {:?}",
                pointer_image_px(&rig),
            );
            probed += 1;
            x += 1.0;
        }
        assert!(
            probed >= 12,
            "the sweep only probed {probed} points - it is not crossing the seam"
        );
    }

    /// A viewport inset inside the content root, so its clip rect is a real
    /// boundary rather than the image edge.
    fn rig_inset_map_viewport(rig: &mut NovaOsPointerRig, inset: Rect) -> Entity {
        let viewport = rig
            .app
            .world_mut()
            .spawn((
                MapViewportMarker,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(inset.min.x),
                    top: Val::Px(inset.min.y),
                    width: Val::Px(inset.width()),
                    height: Val::Px(inset.height()),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(MAP_VIEW_BG),
            ))
            .id();
        rig.app
            .world_mut()
            .entity_mut(rig.content_root)
            .add_child(viewport);
        viewport
    }

    /// Step 5 of the task: a warp fix that still loses edge contacts to clipping
    /// is not a fix. The map viewport is `Overflow::clip()`, and bevy's UI picking
    /// respects clip rects, so a blip straddling the viewport edge is pickable
    /// over its UNCLIPPED part and dead over the clipped part.
    ///
    /// Both halves are asserted: the visible half must select (that is the bug
    /// this guards), and the clipped half must NOT (otherwise the test would pass
    /// against a build that ignores clipping entirely).
    #[test]
    fn map_contacts_straddling_the_viewport_edge_are_pickable_over_their_visible_half() {
        let inset = Rect::new(120.0, 90.0, 900.0, 600.0);
        let mut rig = nova_os_pointer_rig();
        rig.app.init_resource::<MapRuntime>();
        let viewport = rig_inset_map_viewport(&mut rig, inset);

        let target_id = rig.app.world_mut().spawn_empty().id();
        let contact = rig_contact(target_id, "HOST-1");
        // Straddle the viewport's right edge: half the dot inside, half outside.
        // `place` is viewport-local; the rect below is in image space.
        let blip = rig_place_blip(
            &mut rig,
            viewport,
            &contact,
            Vec2::new(inset.max.x - inset.min.x, 200.0),
        );
        let dot = rig_rect(&rig, blip);
        assert!(
            dot.min.x < inset.max.x && dot.max.x > inset.max.x,
            "the rig meant to straddle the clip edge at x {} but the dot is {dot:?}",
            inset.max.x,
        );

        let probe = |rig: &mut NovaOsPointerRig, at: Vec2| {
            rig.app.world_mut().resource_mut::<MapRuntime>().selected = None;
            click_at(rig, glass_px(glass_uv_showing(at)));
            rig.app.world().resource::<MapRuntime>().selected
        };

        let inside = Vec2::new(inset.max.x - 3.5, dot.center().y);
        assert_eq!(
            probe(&mut rig, inside),
            Some(target_id),
            "the visible half of an edge contact (image px {inside:?}) must still \
             select it; the pointer landed on {:?}",
            pointer_image_px(&rig),
        );

        let outside = Vec2::new(inset.max.x + 3.5, dot.center().y);
        assert_eq!(
            probe(&mut rig, outside),
            None,
            "the clipped half (image px {outside:?}) draws nothing, so it must not \
             select either - otherwise this test does not exercise clipping",
        );
    }

    /// The overlap path: map contacts drift, so a label routinely lies over a
    /// neighbouring dot. UI picking resolves that by stacking order, and the
    /// TOPMOST node wins - deterministically, not by accident. Pinned so the
    /// bigger label pill this task introduced cannot quietly start swallowing its
    /// neighbours' clicks in some other order.
    #[test]
    fn overlapping_map_contacts_select_the_topmost() {
        let aim_uv = Vec2::new(0.45, 0.5);
        let mut rig = nova_os_pointer_rig();
        rig.app.init_resource::<MapRuntime>();
        let viewport = rig_map_viewport(&mut rig);

        let under_id = rig.app.world_mut().spawn_empty().id();
        let over_id = rig.app.world_mut().spawn_empty().id();
        let at = image_px_shown_at(aim_uv);
        // Spawned first = lower in the UI stack; the second sits exactly on top.
        rig_place_blip(&mut rig, viewport, &rig_contact(under_id, "HOST-1"), at);
        rig_place_blip(&mut rig, viewport, &rig_contact(over_id, "HOST-2"), at);

        click_at(&mut rig, glass_px(aim_uv));
        assert_eq!(
            rig.app.world().resource::<MapRuntime>().selected,
            Some(over_id),
            "two contacts stacked on the same pixel resolve to the topmost, not to \
             whichever the hit test happened to visit first",
        );
    }
}
