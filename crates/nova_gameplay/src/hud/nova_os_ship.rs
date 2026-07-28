//! NOVA OS `ship` app: a terminal-launched schematic 3D viewer of the player
//! ship, plus the arg-bearing `ship` CLI verbs.
//!
//! Bare `ship` launches this app; `ship view` prints the status summary (built in
//! `nova_os.rs`); `ship section <id>` prints one section's detail; `ship reload
//! <id>` / `ship repair <id>` act on a section. Every section is addressed by a
//! short [`SectionCode`] (`HULL-3`, `PDC-1`, `TRB-1`), assigned stably per session
//! from the section kind + a stable index - the real ships use auto grid-coord
//! `EntityId`s (`cube_i0_j0_k0`) that are unique but unreadable, so the code is the
//! human/CLI/label handle (see `tasks/20260726-115339/DECISION.md`).
//!
//! The viewer follows the `map` app pattern: a dedicated [`Camera3d`] on its own
//! [`RenderLayers`] renders proxy BLOCKS (built from each section's authored
//! [`SectionCollider`] + local transform) into an offscreen image shown in the
//! app body. Each block is a dim, uniform-green fill wrapped in a bright box
//! outline (a `cuboid_edges` wireframe) with a gap, so adjacent sections read
//! apart instead of merging into one green blob; the block colour does NOT encode
//! status (see `tasks/20260728-115435/DECISION.md`). The interactive SECTIONS
//! ride on top as projected clickable UI blips (a nested 3D mesh is not pickable
//! through the CRT composite, but a UI button is - the same reason `map` uses
//! blips); each blip carries a per-kind glyph + its code, an integrity bar
//! (width = HP, colour = status) and, for weapons, ammo pips - that is where a
//! section's status now shows. Orbit the camera with Q/E/R/F + drag + wheel;
//! `[`/`]` cycle the selection; `L` reloads and `P` repairs the selected section.
//!
//! Actions are instant and free for now, but they route through a single
//! [`ShipSectionCommand`] seam (CLI verb -> [`NovaOsCommandInvocation`], in-app key
//! -> message) so a future queued/over-time, resource-costed model can replace the
//! handler without touching the callers (DECISION fork 4).

use bevy::{
    asset::RenderAssetUsages,
    camera::{visibility::RenderLayers, ImageRenderTarget, RenderTarget},
    ecs::system::SystemParam,
    input::mouse::{MouseMotion, MouseWheel},
    mesh::PrimitiveTopology,
    prelude::*,
    render::render_resource::{Extent3d, TextureFormat},
    ui_widgets::{Activate, Button},
};
use nova_events::prelude::EntityId;
use nova_os::prelude::*;

use crate::{
    hud::nova_os::{
        nova_os_font, nova_os_text_font, section_kind_from_markers, section_kind_label,
        DRAWER_LINE_FONT_PX, NOVA_OS_AMBER, NOVA_OS_PHOSPHOR, NOVA_OS_PHOSPHOR_DIM,
        NOVA_OS_PHOSPHOR_MUTED, NOVA_OS_SCREEN, NOVA_OS_TEXT,
    },
    prelude::*,
};

/// Glob-import surface: `use nova_gameplay::hud::nova_os_ship::prelude::*`.
pub mod prelude {
    pub use super::{NovaOsShipPlugin, SectionCode};
}

/// The launch word / stable id of the ship app.
const SHIP_APP_ID: &str = "ship";

/// Render layer the ship schematic scene lives on (isolated from the world on 0
/// and the map on 21).
const SHIP_LAYER: usize = 22;
/// The ship camera renders before the NOVA OS RTT (-20); distinct from the map
/// camera (-30) so the two never share an order even mid-teardown.
const SHIP_CAMERA_ORDER: isize = -31;

const SHIP_RADIUS_MIN: f32 = 3.0;
const SHIP_RADIUS_MAX: f32 = 400.0;
const SHIP_THETA_DEFAULT: f32 = 0.7;
const SHIP_PHI_DEFAULT: f32 = 0.5;
/// Exponential ease rate (1/s) for the orbit center chasing the selected
/// section. Frame-rate independent via `1 - exp(-k*dt)`; higher = snappier.
const SHIP_CENTER_EASE: f32 = 9.0;

/// Footer hints while the ship app owns the screen.
const SHIP_HINTS: &[&str] = &[
    "Q/E: TURN",
    "R/F: TILT",
    "DRAG: LOOK",
    "WHEEL: ZOOM",
    "[ / ]: SELECT",
    "L: RELOAD",
    "P: REPAIR",
    "T: RESET",
    "ESC: BACK",
];

// ---------------------------------------------------------------------------
// Section codes
// ---------------------------------------------------------------------------

/// A short, stable, human-typeable handle for a ship section (`HULL-3`, `PDC-1`),
/// the CLI/label identity the viewer and the `ship <verb> <id>` commands use.
/// Assigned per session by [`assign_section_codes`] from the section kind + a
/// stable index; the underlying grid `EntityId` stays the section's real identity.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct SectionCode(pub String);

/// The code prefix for a section kind (`HULL`, `THR`, `CTL`, `PDC` for turrets,
/// `TRB` for torpedo bays).
fn code_prefix(kind: SectionDamageClass) -> &'static str {
    match kind {
        SectionDamageClass::Hull => "HULL",
        SectionDamageClass::Thruster => "THR",
        SectionDamageClass::Controller => "CTL",
        SectionDamageClass::Turret => "PDC",
        SectionDamageClass::Torpedo => "TRB",
    }
}

/// A small schematic glyph for a section kind, prepended to the blip label to
/// reinforce the code prefix. Kept ASCII so the CRT font always renders it, and
/// distinct per kind so the sections read apart at a glance without new hues.
fn kind_glyph(kind: SectionDamageClass) -> &'static str {
    match kind {
        SectionDamageClass::Hull => "#",
        SectionDamageClass::Thruster => ">",
        SectionDamageClass::Controller => "@",
        SectionDamageClass::Turret => "T",
        SectionDamageClass::Torpedo => "^",
    }
}

/// A one-line "what it does" for a section kind, shown in the inspector panel.
fn kind_description(kind: SectionDamageClass) -> &'static str {
    match kind {
        SectionDamageClass::Hull => "Structural armour plating.",
        SectionDamageClass::Thruster => "Main drive; provides thrust.",
        SectionDamageClass::Controller => "Command core; runs the ship.",
        SectionDamageClass::Turret => "Point-defence gun.",
        SectionDamageClass::Torpedo => "Torpedo launch tube.",
    }
}

/// A dense index for a section kind, for the per-kind next-index counters.
fn kind_index(kind: SectionDamageClass) -> usize {
    match kind {
        SectionDamageClass::Hull => 0,
        SectionDamageClass::Thruster => 1,
        SectionDamageClass::Controller => 2,
        SectionDamageClass::Turret => 3,
        SectionDamageClass::Torpedo => 4,
    }
}

/// Assign a stable [`SectionCode`] to every player-ship section that lacks one.
/// Runs as a system (not an `Add` observer) so it sees sections inserted by the
/// deferred spawn inside the ship-root `Add` observer
/// (`require-default-lands-after-root-add-observer`). Existing codes are never
/// reassigned; a newly appearing section takes the next free index for its kind.
#[allow(clippy::type_complexity)]
fn assign_section_codes(
    mut commands: Commands,
    q_player: Query<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
    q_sections: Query<
        (
            Entity,
            &ChildOf,
            Option<&SectionCode>,
            Option<&EntityId>,
            Option<&SectionDamageClass>,
            Has<HullSectionMarker>,
            Has<ControllerSectionMarker>,
            Has<ThrusterSectionMarker>,
            Has<TurretSectionMarker>,
            Has<TorpedoSectionMarker>,
        ),
        With<SectionMarker>,
    >,
) {
    let Ok(ship) = q_player.single() else {
        return;
    };
    // The highest index already handed out per kind, so new sections continue the
    // sequence rather than colliding.
    let mut next: [u32; 5] = [0; 5];
    let mut unassigned: Vec<(Entity, SectionDamageClass, String)> = Vec::new();
    for (entity, child, code, id, class, hull, controller, thruster, turret, torpedo) in &q_sections
    {
        if child.0 != ship {
            continue;
        }
        let Some(kind) =
            section_kind_from_markers(class, hull, controller, thruster, turret, torpedo)
        else {
            continue;
        };
        if let Some(code) = code {
            if let Some(index) = code
                .0
                .rsplit('-')
                .next()
                .and_then(|tail| tail.parse::<u32>().ok())
            {
                let slot = &mut next[kind_index(kind)];
                *slot = (*slot).max(index);
            }
        } else {
            // Sort key: the stable authored id, so indices are deterministic across
            // runs regardless of ECS iteration order.
            let sort_key = id
                .map(|id| id.0.clone())
                .unwrap_or_else(|| format!("{entity:?}"));
            unassigned.push((entity, kind, sort_key));
        }
    }
    if unassigned.is_empty() {
        return;
    }
    unassigned.sort_by(|a, b| a.2.cmp(&b.2));
    for (entity, kind, _) in unassigned {
        let slot = &mut next[kind_index(kind)];
        *slot += 1;
        commands
            .entity(entity)
            .insert(SectionCode(format!("{}-{}", code_prefix(kind), *slot)));
    }
}

// ---------------------------------------------------------------------------
// Live section model (shared by the CLI verbs and the viewer)
// ---------------------------------------------------------------------------

/// A live player-ship section resolved for the app + CLI: its code, kind, name,
/// placement (its LOCAL transform relative to the ship root - the schematic scene
/// is anchored at the origin, so blocks AND their projected blips both live in
/// this local/scene space, independent of where the ship is flying in world
/// space), authored half-extents, integrity and ammo.
#[derive(Clone)]
struct ShipSectionView {
    entity: Entity,
    code: String,
    kind: SectionDamageClass,
    name: String,
    local: Transform,
    half_extents: Vec3,
    health: Option<Health>,
    ammo: Option<SectionAmmo>,
    inactive: bool,
    zero_health: bool,
}

impl ShipSectionView {
    /// The integrity fraction in `0..=1`, or `None` when the section has no health
    /// component / zero max.
    fn integrity(&self) -> Option<f32> {
        self.health
            .as_ref()
            .filter(|h| h.max > 0.0)
            .map(|h| (h.current.max(0.0) / h.max).clamp(0.0, 1.0))
    }

    /// A one-word status: neutralized / critical / degraded / nominal.
    fn status(&self) -> &'static str {
        if self.inactive || self.zero_health {
            return "neutralized";
        }
        match self.integrity() {
            Some(f) if f <= 0.25 => "critical",
            Some(f) if f <= 0.7 => "degraded",
            _ => "nominal",
        }
    }

    /// The phosphor colour the block + blip read at for this status.
    fn status_color(&self) -> Color {
        match self.status() {
            "neutralized" => NOVA_OS_PHOSPHOR_DIM.with_alpha(0.5),
            "critical" => NOVA_OS_AMBER,
            "degraded" => NOVA_OS_PHOSPHOR_MUTED,
            _ => NOVA_OS_PHOSPHOR,
        }
    }

    /// The `ship view`-style integrity text (`41/100 HP` / `HP unknown`).
    fn health_text(&self) -> String {
        match self.health.as_ref() {
            Some(h) if h.max > 0.0 => format!("{:.0}/{:.0} HP", h.current.max(0.0), h.max),
            Some(h) => format!("{:.0} HP", h.current.max(0.0)),
            None => "HP unknown".to_string(),
        }
    }

    /// The integrity percentage label (`41%`), or `--` when unknown.
    fn integrity_pct(&self) -> String {
        self.integrity()
            .map(|f| format!("{:.0}%", f * 100.0))
            .unwrap_or_else(|| "--".to_string())
    }

    /// A short ASCII integrity meter, `[####------]`.
    fn meter(&self) -> String {
        let filled = (self.integrity().unwrap_or(0.0) * 10.0).round() as usize;
        let filled = filled.min(10);
        format!("[{}{}]", "#".repeat(filled), "-".repeat(10 - filled))
    }
}

/// System-param that enumerates the live player-ship sections into
/// [`ShipSectionView`]s. Shared by the CLI verbs (`ship section/reload/repair`),
/// the arg-completion sync, the scene builder and the interaction systems.
#[derive(SystemParam)]
pub struct ShipSections<'w, 's> {
    player: Query<
        'w,
        's,
        (Entity, Option<&'static Name>),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    sections: Query<
        'w,
        's,
        (
            Entity,
            &'static ChildOf,
            &'static SectionCode,
            Option<&'static Name>,
            &'static Transform,
            Option<&'static SectionCollider>,
            Option<&'static Health>,
            Option<&'static SectionAmmo>,
            // Nested so the whole row stays under the 15-item query-tuple cap.
            SectionKindQuery,
            (Has<SectionInactiveMarker>, Has<HealthZeroMarker>),
        ),
        With<SectionMarker>,
    >,
}

/// The class + kind-marker columns needed to classify a section, grouped so the
/// enclosing section query stays within the query-tuple size limit.
type SectionKindQuery = (
    Option<&'static SectionDamageClass>,
    Has<HullSectionMarker>,
    Has<ControllerSectionMarker>,
    Has<ThrusterSectionMarker>,
    Has<TurretSectionMarker>,
    Has<TorpedoSectionMarker>,
);

impl ShipSections<'_, '_> {
    fn ship(&self) -> Option<(Entity, Option<String>)> {
        self.player
            .single()
            .ok()
            .map(|(e, name)| (e, name.map(|n| n.as_str().to_string())))
    }

    /// Collect the live sections, sorted by code for a stable order.
    fn collect(&self) -> Vec<ShipSectionView> {
        let Some((ship, _)) = self.ship() else {
            return Vec::new();
        };
        let mut views: Vec<ShipSectionView> = self
            .sections
            .iter()
            .filter(|(_, child, ..)| child.0 == ship)
            .filter_map(
                |(
                    entity,
                    _,
                    code,
                    name,
                    local,
                    collider,
                    health,
                    ammo,
                    (class, hull, controller, thruster, turret, torpedo),
                    (inactive, zero_health),
                )| {
                    let kind = section_kind_from_markers(
                        class, hull, controller, thruster, turret, torpedo,
                    )?;
                    Some(ShipSectionView {
                        entity,
                        code: code.0.clone(),
                        kind,
                        name: name
                            .map(|n| n.as_str().to_string())
                            .unwrap_or_else(|| code.0.clone()),
                        local: *local,
                        half_extents: collider.copied().unwrap_or_default().aabb_half_extents(),
                        health: health.cloned(),
                        ammo: ammo.copied(),
                        inactive,
                        zero_health,
                    })
                },
            )
            .collect();
        views.sort_by(|a, b| a.code.cmp(&b.code));
        views
    }

    /// Resolve a typed code (case-insensitive) to its section view. The live CLI
    /// handler resolves without touching `Health`/`Ammo` (to avoid a query
    /// conflict), so this convenience is used by the tests.
    #[allow(dead_code)]
    fn resolve(&self, code: &str) -> Option<ShipSectionView> {
        let wanted = code.to_ascii_uppercase();
        self.collect()
            .into_iter()
            .find(|view| view.code.eq_ignore_ascii_case(&wanted))
    }

    /// Every section code, for Tab completion of the `ship <verb> <id>` argument.
    fn codes(&self) -> Vec<String> {
        self.collect().into_iter().map(|view| view.code).collect()
    }
}

// ---------------------------------------------------------------------------
// CLI verb rows
// ---------------------------------------------------------------------------

/// The `ship section <id>` detail rows for one section.
fn section_detail_rows(view: &ShipSectionView) -> Vec<TerminalRow> {
    let mut rows = vec![
        TerminalRow {
            kind: TerminalRowKind::Info,
            text: format!("SECTION {} - {}", view.code, view.name),
        },
        TerminalRow {
            kind: TerminalRowKind::Output,
            text: format!("kind: {}", section_kind_label(view.kind).to_lowercase()),
        },
        TerminalRow {
            kind: status_row_kind(view.status()),
            text: format!(
                "integrity: {} {} {}",
                view.integrity_pct(),
                view.meter(),
                view.health_text()
            ),
        },
        TerminalRow {
            kind: status_row_kind(view.status()),
            text: format!("status: {}", view.status()),
        },
    ];
    if let Some(ammo) = view.ammo.as_ref() {
        rows.push(TerminalRow {
            kind: TerminalRowKind::Output,
            text: format!("ammo: {}/{}", ammo.rounds, ammo.capacity),
        });
    }
    rows
}

fn status_row_kind(status: &str) -> TerminalRowKind {
    match status {
        "neutralized" => TerminalRowKind::Error,
        "critical" => TerminalRowKind::Warn,
        _ => TerminalRowKind::Output,
    }
}

/// The multi-line body of the inspector panel for one section: kind + what it
/// does, integrity % + meter, HP text, status, and ammo for weapons.
fn panel_detail_text(view: &ShipSectionView) -> String {
    let mut text = format!(
        "kind: {}\n{}\n\nintegrity: {} {}\n{}\nstatus: {}",
        section_kind_label(view.kind).to_lowercase(),
        kind_description(view.kind),
        view.integrity_pct(),
        view.meter(),
        view.health_text(),
        view.status(),
    );
    if let Some(ammo) = view.ammo.as_ref() {
        text.push_str(&format!("\nammo: {}/{}", ammo.rounds, ammo.capacity));
    }
    text
}

/// Whether Repair / Reload are valid for a section, plus a reason for a disabled
/// action. Derived from the SAME conditions [`apply_action_to_section`] enforces
/// (Reload = a `Turret`/`Torpedo` with an ammo feed; Repair = `Health` with a
/// positive max), so the panel buttons never disagree with the handler.
struct PanelActions {
    repair_enabled: bool,
    reload_enabled: bool,
    reason: Option<String>,
}

impl PanelActions {
    /// The no-selection state: nothing actionable, no reason.
    fn none() -> Self {
        Self {
            repair_enabled: false,
            reload_enabled: false,
            reason: None,
        }
    }
}

fn panel_action_state(view: &ShipSectionView) -> PanelActions {
    let is_weapon = matches!(
        view.kind,
        SectionDamageClass::Turret | SectionDamageClass::Torpedo
    );
    let repair_enabled = view.health.as_ref().map(|h| h.max > 0.0).unwrap_or(false);
    let reload_enabled = is_weapon && view.ammo.is_some();

    // Surface why a disabled action is unavailable, mirroring the handler's text.
    let reason = if !is_weapon {
        Some(format!(
            "reload: {} is a {} section, no ammo feed",
            view.code,
            section_kind_label(view.kind).to_lowercase()
        ))
    } else if view.ammo.is_none() {
        Some(format!("reload: {} has unlimited ammo", view.code))
    } else if !repair_enabled {
        Some(format!("repair: {} has no integrity to restore", view.code))
    } else {
        None
    };

    PanelActions {
        repair_enabled,
        reload_enabled,
        reason,
    }
}

/// A "section not found" error row plus the list of valid codes.
fn unknown_code_rows(code: &str, codes: &[String]) -> Vec<TerminalRow> {
    let mut rows = vec![TerminalRow {
        kind: TerminalRowKind::Error,
        text: format!("no such section: {code}"),
    }];
    if !codes.is_empty() {
        rows.push(TerminalRow {
            kind: TerminalRowKind::Dim,
            text: format!("sections: {}", codes.join("   ")),
        });
    }
    rows
}

// ---------------------------------------------------------------------------
// Action seam (CLI verb + in-app key -> one handler)
// ---------------------------------------------------------------------------

/// A mutating action on a section. Instant/free today; the [`ShipSectionCommand`]
/// seam is where a future queued, resource-costed job model plugs in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShipAction {
    /// Refill a weapon section's ammo to capacity.
    Reload,
    /// Restore a section's integrity to full.
    Repair,
}

/// A request to apply a [`ShipAction`] to a section, raised by the in-app action
/// keys. The CLI verbs apply the same action directly through
/// [`apply_action_to_section`]; both paths converge on the same mutation.
#[derive(Message, Clone, Copy, Debug)]
pub struct ShipSectionCommand {
    /// The target section entity.
    pub target: Entity,
    /// What to do to it.
    pub action: ShipAction,
}

/// Apply an action to a section's live `Health` / `SectionAmmo`, returning the
/// result row. This is the single mutation point (arcade-instant today); a queued
/// model would enqueue a job here instead of mutating in place.
fn apply_action_to_section(
    action: ShipAction,
    code: &str,
    kind: SectionDamageClass,
    is_weapon: bool,
    health: Option<&mut Health>,
    ammo: Option<&mut SectionAmmo>,
) -> TerminalRow {
    match action {
        ShipAction::Reload => {
            if !is_weapon {
                return TerminalRow {
                    kind: TerminalRowKind::Error,
                    text: format!(
                        "reload: {code} is a {} section, no ammo feed",
                        section_kind_label(kind).to_lowercase()
                    ),
                };
            }
            match ammo {
                Some(ammo) => {
                    ammo.rounds = ammo.capacity;
                    TerminalRow {
                        kind: TerminalRowKind::Info,
                        text: format!("reloaded {code}: ammo {}/{}", ammo.rounds, ammo.capacity),
                    }
                }
                None => TerminalRow {
                    kind: TerminalRowKind::Dim,
                    text: format!("reload: {code} has unlimited ammo"),
                },
            }
        }
        ShipAction::Repair => match health {
            Some(health) if health.max > 0.0 => {
                health.current = health.max;
                TerminalRow {
                    kind: TerminalRowKind::Info,
                    text: format!(
                        "repaired {code}: integrity restored to {:.0} HP",
                        health.max
                    ),
                }
            }
            _ => TerminalRow {
                kind: TerminalRowKind::Error,
                text: format!("repair: {code} has no integrity to restore"),
            },
        },
    }
}

// ---------------------------------------------------------------------------
// App runtime + UI markers
// ---------------------------------------------------------------------------

struct ShipApp;

impl NovaOsAppRuntime for ShipApp {
    fn id(&self) -> &'static str {
        SHIP_APP_ID
    }
    fn title(&self) -> &'static str {
        "SHIP / SCHEMATIC"
    }
    fn hints(&self) -> &'static [&'static str] {
        SHIP_HINTS
    }
    fn spawn_body(&self, body: &mut ChildSpawnerCommands, font: Handle<Font>) {
        // The body is a Column; lay the viewport + inspector panel out as a Row
        // that grows to fill it: [ 3D viewport (flex-grow) | fixed-width panel ].
        body.spawn(Node {
            flex_grow: 1.0,
            min_height: Val::Px(0.0),
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                ShipViewportMarker,
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
                BackgroundColor(SHIP_VIEW_BG),
            ));
            spawn_ship_panel(row, font.clone());
        });
    }
}

/// Fixed width of the inspector panel column, in px.
const SHIP_PANEL_PX: f32 = 232.0;

/// Build the inspector-panel subtree (title, live detail, action row, note) as a
/// bordered CRT column. The three info text nodes carry a [`ShipPanelField`] so
/// one system can refresh them; the two buttons carry a [`ShipPanelButton`] and
/// route through the [`ShipSectionCommand`] seam via `Activate` observers.
fn spawn_ship_panel(parent: &mut ChildSpawnerCommands, font: Handle<Font>) {
    parent
        .spawn((
            ShipPanelMarker,
            Node {
                width: Val::Px(SHIP_PANEL_PX),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                padding: UiRect::all(Val::Px(10.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BorderColor::all(NOVA_OS_PHOSPHOR.with_alpha(0.36)),
            BackgroundColor(NOVA_OS_SCREEN),
        ))
        .with_children(|panel| {
            panel.spawn((
                ShipPanelField::Title,
                Text::new("INSPECTOR"),
                nova_os_text_font(DRAWER_LINE_FONT_PX, font.clone()),
                TextColor(NOVA_OS_PHOSPHOR),
            ));
            panel.spawn((
                ShipPanelField::Detail,
                Text::new("Select a section:\nclick a block or press [ / ]."),
                nova_os_text_font(DRAWER_LINE_FONT_PX - 3.0, font.clone()),
                TextColor(NOVA_OS_TEXT),
            ));
            // Action row: Repair + Reload buttons, each routed through the
            // ShipSectionCommand seam by an `Activate` observer.
            panel
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(6.0),
                    margin: UiRect::top(Val::Px(4.0)),
                    ..default()
                })
                .with_children(|row| {
                    row.spawn(panel_button_bundle(ShipPanelButton::Repair))
                        .observe(on_ship_repair_button)
                        .with_children(|b| {
                            b.spawn(panel_button_label("P Repair", font.clone()));
                        });
                    row.spawn(panel_button_bundle(ShipPanelButton::Reload))
                        .observe(on_ship_reload_button)
                        .with_children(|b| {
                            b.spawn(panel_button_label("L Reload", font.clone()));
                        });
                });
            panel.spawn((
                ShipPanelField::Note,
                Text::new(String::new()),
                nova_os_text_font(DRAWER_LINE_FONT_PX - 4.0, font),
                TextColor(NOVA_OS_PHOSPHOR_MUTED),
            ));
        });
}

/// The bundle for a CRT action button (enabled styling; recoloured per frame in
/// `update_ship_panel`). The `Activate` observer is attached by the caller.
fn panel_button_bundle(kind: ShipPanelButton) -> impl Bundle {
    (
        kind,
        Button,
        Node {
            padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        BorderColor::all(NOVA_OS_PHOSPHOR),
        BackgroundColor(NOVA_OS_PHOSPHOR.with_alpha(0.14)),
    )
}

/// The label bundle for a panel button.
fn panel_button_label(label: &str, font: Handle<Font>) -> impl Bundle {
    (
        Text::new(label.to_string()),
        nova_os_text_font(DRAWER_LINE_FONT_PX - 3.0, font),
        TextColor(NOVA_OS_PHOSPHOR),
    )
}

/// Dark fill behind the schematic image.
const SHIP_VIEW_BG: Color = Color::srgb_u8(0, 6, 3);

#[derive(Component)]
struct ShipViewportMarker;
/// The inspector-panel container.
#[derive(Component)]
struct ShipPanelMarker;
/// Which live text line of the panel a node is, so one system refreshes all three.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum ShipPanelField {
    Title,
    Detail,
    Note,
}
/// Which action a panel button raises, for its per-frame enabled styling.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum ShipPanelButton {
    Repair,
    Reload,
}
#[derive(Component)]
struct ShipCameraMarker;
#[derive(Component)]
struct ShipSceneRoot;

/// A schematic block for one section: a dim, uniform-green translucent fill. It
/// no longer tracks status by colour (status now rides the blip integrity bar,
/// see `tasks/20260728-115435/DECISION.md`); it stays green so the schematic
/// reads as a labelled wireframe rather than one green blob.
#[derive(Component)]
struct ShipBlock {
    section: Entity,
}

/// The bright box outline riding on a [`ShipBlock`] (a `LineList` of the cuboid's
/// 12 edges) - the separation cue that keeps adjacent sections from merging. Its
/// material tints amber while its section is selected; the section identity lives
/// on the parent [`ShipBlock`], which the outline derives via [`ChildOf`].
#[derive(Component)]
struct ShipBlockOutline;

/// A projected, clickable section blip over the viewport: a status-coloured dot
/// plus a labelled marker. HP/ammo/status detail lives in the inspector panel now
/// (see `tasks/20260728-125514/DECISION.md`), so the blip carries no bar or pips.
#[derive(Component)]
struct ShipBlip {
    section: Entity,
}

/// The ship camera's orbit state (own spherical math, like `MapOrbit`, because
/// the shared smoothed orbit does not drive an RTT camera -
/// `verify-reused-driver-actually-moves`).
#[derive(Component, Clone, Copy)]
struct ShipOrbit {
    theta: f32,
    phi: f32,
    radius: f32,
    /// The eased orbit center the camera currently looks at and orbits around.
    center: Vec3,
    /// Where `center` is easing toward - the selected section's local position,
    /// or `center_home` after a `T` reset.
    center_target: Vec3,
    /// The whole-ship centroid; `T` re-frames the ship by retargeting here.
    center_home: Vec3,
    /// The selection `center_target` was last set for. The center only retargets
    /// when `ShipRuntime.selected` diverges from this, so `T`'s home reframe is
    /// not immediately chased back to the still-selected section.
    centered_on: Option<Entity>,
}

fn orbit_eye(radius: f32, theta: f32, phi: f32) -> Vec3 {
    let horizontal = radius * phi.cos();
    Vec3::new(
        horizontal * theta.sin(),
        radius * phi.sin(),
        horizontal * theta.cos(),
    )
}

/// Live state of the running ship app.
#[derive(Resource, Default)]
struct ShipRuntime {
    active: bool,
    image: Option<Handle<Image>>,
    scene_root: Option<Entity>,
    blips: bevy::platform::collections::HashMap<Entity, Entity>,
    /// The uniform dim-green block fill, shared by every block (status no longer
    /// recolours blocks - it rides the blip bar instead).
    mat_fill: Option<Handle<StandardMaterial>>,
    /// The bright phosphor box outline, and its amber variant for the selected
    /// section; swapped per block in `update_ship_blocks`.
    mat_outline: Option<Handle<StandardMaterial>>,
    mat_outline_selected: Option<Handle<StandardMaterial>>,
    selected: Option<Entity>,
    /// The codes last pushed as arg-completions, so the terminal is only marked
    /// changed when the set changes.
    completion_codes: Vec<String>,
    /// A transient note (e.g. an in-app action result) shown in the panel.
    note: Option<(String, f32)>,
    /// Whether Repair / Reload are valid for the current selection, cached by
    /// `update_ship_panel` so the button `Activate` observers can no-op on a
    /// disabled action without re-deriving the section's validity.
    panel_repair_enabled: bool,
    panel_reload_enabled: bool,
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

/// Registers the `ship` app + CLI verbs and drives the schematic scene, blips and
/// section actions.
pub struct NovaOsShipPlugin;

impl Plugin for NovaOsShipPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShipRuntime>();
        app.add_message::<ShipSectionCommand>();

        // Register the `ship` command tree: bare `ship` launches the app; `ship
        // view` prints the summary (rows filled in nova_os.rs); the arg-bearing
        // verbs dispatch to the gameplay layer (`apply_ship_cli_commands`).
        app.world_mut()
            .resource_mut::<NovaOsCommandRegistry>()
            .register(ship_command_tree());

        app.add_systems(
            Update,
            (
                assign_section_codes,
                sync_ship_arg_completions,
                apply_ship_cli_commands,
                apply_ship_section_commands,
                manage_ship_scene,
                reconcile_ship_target,
                ship_input,
                drive_ship_camera,
                update_ship_blocks,
                project_ship_blips,
                update_ship_panel,
            )
                .chain()
                .in_set(NovaOsShipSystems),
        );
    }
}

/// The `ship` command tree: the app launch word, the `ship view` snapshot
/// subcommand, and the arg-bearing `section`/`reload`/`repair` gameplay verbs.
/// Shared by the plugin registration and the tests.
fn ship_command_tree() -> TerminalCommand {
    TerminalCommand::app(SHIP_APP_ID, "Open the ship computer", ShipApp)
        .with_subcommand(TerminalCommand::cli(
            "ship view",
            "Print ship status summary",
            CliOutput::Snapshot,
        ))
        .with_subcommand(
            TerminalCommand::gameplay(
                "ship section",
                "Show one section's detail",
                CommandArity::UpTo(1),
            )
            .with_arg_hint("<section>"),
        )
        .with_subcommand(
            TerminalCommand::gameplay(
                "ship reload",
                "Reload a weapon section",
                CommandArity::UpTo(1),
            )
            .with_arg_hint("<section>"),
        )
        .with_subcommand(
            TerminalCommand::gameplay("ship repair", "Repair a section", CommandArity::UpTo(1))
                .with_arg_hint("<section>"),
        )
}

/// System set for the ship app's per-frame work.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct NovaOsShipSystems;

/// Whether the ship app is the active NOVA OS surface right now.
fn ship_is_active(pause: &State<PauseStates>, terminal: &NovaOsTerminal) -> bool {
    *pause.get() == PauseStates::NovaOs
        && terminal.active_mode() == TerminalMode::App { id: SHIP_APP_ID }
}

// ---------------------------------------------------------------------------
// CLI + action systems
// ---------------------------------------------------------------------------

/// Keep the terminal's arg-completion set in sync with the live section codes, so
/// `ship repair <TAB>` offers them. Only writes on a real change.
fn sync_ship_arg_completions(
    sections: ShipSections,
    mut runtime: ResMut<ShipRuntime>,
    mut terminal: ResMut<NovaOsTerminal>,
) {
    let codes = sections.codes();
    if codes == runtime.completion_codes {
        return;
    }
    runtime.completion_codes = codes.clone();
    // Merge (not replace) so the `map goto` completions the map app owns survive;
    // the `!=` gate above already ensured this set changed.
    terminal.merge_arg_completions(
        ["ship section", "ship reload", "ship repair"]
            .into_iter()
            .map(|verb| (verb, codes.clone())),
    );
}

/// Drain the arg-bearing `ship` CLI verb the terminal queued on submit, apply it
/// against the live world, and append the result rows to the scrollback.
///
/// This resolves the section with a query that does NOT touch `Health`/
/// `SectionAmmo`, so the `&mut` health/ammo queries it also holds do not conflict
/// (a `ShipSections` here would read those components immutably and deadlock the
/// scheduler); integrity/ammo are read back through the mutable queries' `get`.
#[allow(clippy::type_complexity)]
fn apply_ship_cli_commands(
    mut terminal: ResMut<NovaOsTerminal>,
    q_player: Query<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
    q_sections: Query<
        (
            Entity,
            &ChildOf,
            &SectionCode,
            Option<&Name>,
            SectionKindQuery,
            (Has<SectionInactiveMarker>, Has<HealthZeroMarker>),
        ),
        With<SectionMarker>,
    >,
    mut q_health: Query<&mut Health>,
    mut q_ammo: Query<&mut SectionAmmo>,
) {
    // Peek first: the single pending slot is shared with the `map` gameplay verbs,
    // so only consume an invocation this handler owns (leave `map ...` for its own
    // system, `cross-app-invocation-peek-before-take`).
    let owns = terminal
        .peek_pending_invocation()
        .is_some_and(|inv| matches!(inv.name, "ship section" | "ship reload" | "ship repair"));
    if !owns {
        return;
    }
    let Some(invocation) = terminal.take_pending_invocation() else {
        return;
    };
    let Some(code) = invocation.args.first() else {
        terminal.extend_scrollback([TerminalRow {
            kind: TerminalRowKind::Error,
            text: format!("{}: expected a section id", invocation.name),
        }]);
        return;
    };
    let Ok(ship) = q_player.single() else {
        terminal.extend_scrollback([TerminalRow {
            kind: TerminalRowKind::Error,
            text: "no live player ship".to_string(),
        }]);
        return;
    };

    // Resolve the code (no Health/Ammo access here) and gather the valid codes for
    // the not-found listing.
    let wanted = code.to_ascii_uppercase();
    let mut codes: Vec<String> = Vec::new();
    let mut target: Option<(Entity, String, String, SectionDamageClass, bool, bool)> = None;
    for (
        entity,
        child,
        section_code,
        name,
        (class, hull, controller, thruster, turret, torpedo),
        (inactive, zero),
    ) in &q_sections
    {
        if child.0 != ship {
            continue;
        }
        codes.push(section_code.0.clone());
        if section_code.0.eq_ignore_ascii_case(&wanted) {
            if let Some(kind) =
                section_kind_from_markers(class, hull, controller, thruster, turret, torpedo)
            {
                target = Some((
                    entity,
                    section_code.0.clone(),
                    name.map(|n| n.as_str().to_string())
                        .unwrap_or_else(|| section_code.0.clone()),
                    kind,
                    inactive,
                    zero,
                ));
            }
        }
    }
    codes.sort();
    let Some((entity, code_str, name, kind, inactive, zero)) = target else {
        terminal.extend_scrollback(unknown_code_rows(code, &codes));
        return;
    };
    let is_weapon = matches!(
        kind,
        SectionDamageClass::Turret | SectionDamageClass::Torpedo
    );

    let rows = match invocation.name {
        "ship section" => {
            // Read integrity/ammo back through the mutable queries (read-only get).
            let view = ShipSectionView {
                entity,
                code: code_str,
                kind,
                name,
                local: Transform::default(),
                half_extents: Vec3::ONE,
                health: q_health.get(entity).ok().cloned(),
                ammo: q_ammo.get(entity).ok().copied(),
                inactive,
                zero_health: zero,
            };
            section_detail_rows(&view)
        }
        "ship reload" | "ship repair" => {
            let action = if invocation.name == "ship reload" {
                ShipAction::Reload
            } else {
                ShipAction::Repair
            };
            let mut health = q_health.get_mut(entity).ok();
            let mut ammo = q_ammo.get_mut(entity).ok();
            vec![apply_action_to_section(
                action,
                &code_str,
                kind,
                is_weapon,
                health.as_deref_mut(),
                ammo.as_deref_mut(),
            )]
        }
        _ => vec![TerminalRow {
            kind: TerminalRowKind::Error,
            text: format!("{}: unhandled ship verb", invocation.name),
        }],
    };
    terminal.extend_scrollback(rows);
}

/// Apply in-app [`ShipSectionCommand`] messages (the `L`/`P` action keys and the
/// panel buttons), and flash the result on the panel note line.
fn apply_ship_section_commands(
    mut messages: MessageReader<ShipSectionCommand>,
    mut runtime: ResMut<ShipRuntime>,
    q_view: Query<(
        &SectionCode,
        Option<&SectionDamageClass>,
        Has<HullSectionMarker>,
        Has<ControllerSectionMarker>,
        Has<ThrusterSectionMarker>,
        Has<TurretSectionMarker>,
        Has<TorpedoSectionMarker>,
    )>,
    mut q_health: Query<&mut Health>,
    mut q_ammo: Query<&mut SectionAmmo>,
) {
    for command in messages.read() {
        let Ok((code, class, hull, controller, thruster, turret, torpedo)) =
            q_view.get(command.target)
        else {
            continue;
        };
        let Some(kind) =
            section_kind_from_markers(class, hull, controller, thruster, turret, torpedo)
        else {
            continue;
        };
        let is_weapon = matches!(
            kind,
            SectionDamageClass::Turret | SectionDamageClass::Torpedo
        );
        let mut health = q_health.get_mut(command.target).ok();
        let mut ammo = q_ammo.get_mut(command.target).ok();
        let row = apply_action_to_section(
            command.action,
            &code.0,
            kind,
            is_weapon,
            health.as_deref_mut(),
            ammo.as_deref_mut(),
        );
        runtime.note = Some((row.text, 2.5));
    }
}

// ---------------------------------------------------------------------------
// Scene systems
// ---------------------------------------------------------------------------

/// Build the schematic block scene + camera on ship-app open, tear it down on
/// close.
#[allow(clippy::too_many_arguments)]
fn manage_ship_scene(
    mut commands: Commands,
    pause: Res<State<PauseStates>>,
    terminal: Res<NovaOsTerminal>,
    mut runtime: ResMut<ShipRuntime>,
    images: Option<ResMut<Assets<Image>>>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    sections: ShipSections,
) {
    let active = ship_is_active(&pause, &terminal);
    if active == runtime.active {
        return;
    }
    runtime.active = active;

    if !active {
        if let Some(root) = runtime.scene_root.take() {
            commands.entity(root).despawn();
        }
        for (_, blip) in runtime.blips.drain() {
            commands.entity(blip).try_despawn();
        }
        runtime.image = None;
        runtime.selected = None;
        runtime.note = None;
        return;
    }

    let (Some(mut images), Some(mut meshes), Some(mut materials)) = (images, meshes, materials)
    else {
        return;
    };

    let views = sections.collect();
    // Frame the whole ship: centroid of the section blocks + a radius that fits
    // the furthest one.
    let centroid = if views.is_empty() {
        Vec3::ZERO
    } else {
        views.iter().map(|v| v.local.translation).sum::<Vec3>() / views.len() as f32
    };
    let extent = views
        .iter()
        .map(|v| (v.local.translation - centroid).length() + v.half_extents.max_element())
        .fold(2.0_f32, f32::max);
    let radius = (extent * 2.6).clamp(SHIP_RADIUS_MIN, SHIP_RADIUS_MAX);

    let image = images.add(new_rtt_image(UVec2::splat(64)));
    runtime.image = Some(image.clone());

    // A dim, uniform-green fill for every block (status rides the blip bar now),
    // plus a bright edge outline and its amber selected variant. Shared handles,
    // so selecting a section only swaps its outline material, never respawns.
    runtime.mat_fill = Some(materials.add(unlit(NOVA_OS_PHOSPHOR.with_alpha(0.22))));
    runtime.mat_outline = Some(materials.add(unlit(NOVA_OS_PHOSPHOR.with_alpha(0.95))));
    runtime.mat_outline_selected = Some(materials.add(unlit(NOVA_OS_AMBER)));

    let scene_root = commands
        .spawn((
            ShipSceneRoot,
            Name::new("NovaOsShipScene"),
            Transform::default(),
            Visibility::Visible,
        ))
        .id();

    commands.spawn((
        ShipCameraMarker,
        Name::new("NovaOsShipCamera"),
        Camera3d::default(),
        Camera {
            order: SHIP_CAMERA_ORDER,
            clear_color: ClearColorConfig::Custom(NOVA_OS_SCREEN),
            is_active: true,
            ..default()
        },
        RenderTarget::Image(ImageRenderTarget {
            handle: image.clone(),
            scale_factor: 1.0,
        }),
        Transform::from_translation(
            centroid + orbit_eye(radius, SHIP_THETA_DEFAULT, SHIP_PHI_DEFAULT),
        )
        .looking_at(centroid, Vec3::Y),
        RenderLayers::layer(SHIP_LAYER),
        ShipOrbit {
            theta: SHIP_THETA_DEFAULT,
            phi: SHIP_PHI_DEFAULT,
            radius,
            center: centroid,
            center_target: centroid,
            center_home: centroid,
            // Treat the default selection as already centered at home, so the app
            // OPENS framed on the whole ship instead of chasing section 0 on frame 1.
            centered_on: views.first().map(|v| v.entity),
        },
        ChildOf(scene_root),
    ));

    // One proxy per section: a dim uniform-green fill shrunk to leave a GAP to its
    // neighbours, wrapped in a bright box OUTLINE at the full collider size. The
    // gap + outline are the separation cue that breaks up the "green blob".
    let fill_mat = runtime.mat_fill.clone().unwrap();
    let outline_mat = runtime.mat_outline.clone().unwrap();
    // One shared unit-cube edge mesh, scaled per block by the outline's transform.
    let edge_mesh = meshes.add(cuboid_edges());
    for view in &views {
        let full = (view.half_extents * 2.0).max(Vec3::splat(0.2));
        let fill = full * SHIP_BLOCK_FILL_SCALE;
        let mesh = meshes.add(Cuboid::new(fill.x, fill.y, fill.z));
        commands
            .spawn((
                ShipBlock {
                    section: view.entity,
                },
                Mesh3d(mesh),
                MeshMaterial3d(fill_mat.clone()),
                Transform {
                    translation: view.local.translation,
                    rotation: view.local.rotation,
                    scale: Vec3::ONE,
                },
                RenderLayers::layer(SHIP_LAYER),
                ChildOf(scene_root),
            ))
            .with_children(|block| {
                block.spawn((
                    ShipBlockOutline,
                    Mesh3d(edge_mesh.clone()),
                    MeshMaterial3d(outline_mat.clone()),
                    // Unit edges scaled to the FULL collider size (the fill inside
                    // is smaller, so the outline frames it with a visible gap).
                    Transform::from_scale(full),
                    RenderLayers::layer(SHIP_LAYER),
                ));
            });
    }

    runtime.scene_root = Some(scene_root);
    // Default the selection to the first section so the panel is useful at once.
    runtime.selected = views.first().map(|v| v.entity);
}

/// Keep the offscreen image sized 1:1 to the viewport node and patch the image
/// handle onto the viewport.
#[allow(clippy::type_complexity)]
fn reconcile_ship_target(
    runtime: Res<ShipRuntime>,
    mut images: Option<ResMut<Assets<Image>>>,
    mut q_viewport: Query<(&ComputedNode, &mut ImageNode), With<ShipViewportMarker>>,
    mut q_camera: Query<(&mut Camera, &mut Projection), With<ShipCameraMarker>>,
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
        if let Ok((_, mut projection)) = q_camera.single_mut() {
            projection.set_changed();
        }
    }
}

/// Drive the ship camera transform from its orbit state.
fn drive_ship_camera(
    time: Res<Time>,
    mut q_camera: Query<(&mut Transform, &mut ShipOrbit), With<ShipCameraMarker>>,
) {
    let Ok((mut transform, mut orbit)) = q_camera.single_mut() else {
        return;
    };
    // Frame-rate-independent exponential ease of the center toward its target, so
    // Q/E/R/F + drag orbit around the section being inspected.
    let dt = time.delta_secs().max(1.0 / 240.0);
    let alpha = 1.0 - (-SHIP_CENTER_EASE * dt).exp();
    orbit.center = orbit.center.lerp(orbit.center_target, alpha);
    let eye = orbit.center + orbit_eye(orbit.radius, orbit.theta, orbit.phi);
    *transform = Transform::from_translation(eye).looking_at(orbit.center, Vec3::Y);
}

/// Read the keyboard/mouse while the ship app owns the screen: orbit, cycle the
/// selection, and raise reload/repair on the selected section.
#[allow(clippy::too_many_arguments)]
fn ship_input(
    pause: Res<State<PauseStates>>,
    terminal: Res<NovaOsTerminal>,
    mut runtime: ResMut<ShipRuntime>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut motion: MessageReader<MouseMotion>,
    mut wheel: MessageReader<MouseWheel>,
    time: Res<Time>,
    sections: ShipSections,
    mut commands: MessageWriter<ShipSectionCommand>,
    mut q_camera: Query<&mut ShipOrbit, With<ShipCameraMarker>>,
) {
    if !ship_is_active(&pause, &terminal) {
        return;
    }
    let motion_delta: Vec2 = motion.read().map(|m| m.delta).sum();
    let wheel_delta: f32 = wheel.read().map(|w| w.y).sum();
    let dt = time.delta_secs().max(1.0 / 240.0);

    if let Some((_, remaining)) = runtime.note.as_mut() {
        *remaining -= dt;
        if *remaining <= 0.0 {
            runtime.note = None;
        }
    }

    if let Ok(mut orbit) = q_camera.single_mut() {
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
        // Mouse drag orbits, RIGHT button ONLY. LMB is the blip-select click (the
        // `Button` widget), so letting it orbit turned a small press-with-motion
        // into a drag that slid the blip out from under the cursor and ate the
        // selection.
        if mouse_buttons.pressed(MouseButton::Right) {
            orbit.theta -= motion_delta.x * 0.0024;
            orbit.phi = (orbit.phi + motion_delta.y * 0.0024).clamp(0.12, 1.45);
        }
        if wheel_delta != 0.0 {
            orbit.radius =
                (orbit.radius * (1.0 - wheel_delta * 0.12)).clamp(SHIP_RADIUS_MIN, SHIP_RADIUS_MAX);
        }
    }

    let list = sections.collect();
    if list.is_empty() {
        return;
    }
    // Cycle the selection with [ and ].
    let forward = keys.just_pressed(KeyCode::BracketRight);
    let backward = keys.just_pressed(KeyCode::BracketLeft);
    if forward || backward {
        let current = runtime
            .selected
            .and_then(|sel| list.iter().position(|v| v.entity == sel));
        let len = list.len();
        let next = match current {
            Some(i) if forward => (i + 1) % len,
            Some(i) => (i + len - 1) % len,
            None => 0,
        };
        runtime.selected = Some(list[next].entity);
    }

    // Reconcile the orbit center after any selection change this frame. This is
    // the single funnel for `[`/`]`, blip clicks, and the default selection: each
    // caller only sets `runtime.selected`, and the center chases it here.
    if let Ok(mut orbit) = q_camera.single_mut() {
        if keys.just_pressed(KeyCode::KeyT) {
            // Reset re-frames the whole ship: restore the default angles and
            // retarget the center home. Consuming the current selection
            // (centered_on = selected) makes the reframe STICK - the change check
            // below will not chase the still-selected section back (DECISION.md).
            orbit.theta = SHIP_THETA_DEFAULT;
            orbit.phi = SHIP_PHI_DEFAULT;
            orbit.center_target = orbit.center_home;
            orbit.centered_on = runtime.selected;
        } else if runtime.selected != orbit.centered_on {
            // Selection changed: ease the center onto the newly selected section's
            // local position (same scene frame as the blips, so no reprojection).
            if let Some(view) = runtime
                .selected
                .and_then(|sel| list.iter().find(|v| v.entity == sel))
            {
                orbit.center_target = view.local.translation;
            }
            orbit.centered_on = runtime.selected;
        }
    }

    // Actions on the selected section: L reload, P repair. Route through the same
    // message the panel buttons raise and the mutation handler applies.
    if let Some(sel) = runtime.selected {
        if keys.just_pressed(KeyCode::KeyL) {
            commands.write(ShipSectionCommand {
                target: sel,
                action: ShipAction::Reload,
            });
        }
        if keys.just_pressed(KeyCode::KeyP) {
            commands.write(ShipSectionCommand {
                target: sel,
                action: ShipAction::Repair,
            });
        }
    }
}

/// Tint the SELECTED section's box outline amber and leave every other block's
/// outline phosphor. Blocks no longer recolour by status - the fill stays a
/// uniform green and status rides the blip integrity bar (DECISION.md).
fn update_ship_blocks(
    runtime: Res<ShipRuntime>,
    mut q_outline: Query<(&ChildOf, &mut MeshMaterial3d<StandardMaterial>), With<ShipBlockOutline>>,
    q_block: Query<&ShipBlock>,
) {
    if !runtime.active {
        return;
    }
    let (Some(base), Some(selected)) = (&runtime.mat_outline, &runtime.mat_outline_selected) else {
        return;
    };
    for (parent, mut material) in &mut q_outline {
        // The section identity lives on the parent block, not the outline.
        let Ok(block) = q_block.get(parent.0) else {
            continue;
        };
        let want = if runtime.selected == Some(block.section) {
            selected
        } else {
            base
        };
        if material.0 != *want {
            material.0 = want.clone();
        }
    }
}

/// Project each section through the ship camera into the viewport and keep a
/// clickable, labelled UI blip per section in sync.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn project_ship_blips(
    mut commands: Commands,
    mut runtime: ResMut<ShipRuntime>,
    asset_server: Option<Res<AssetServer>>,
    sections: ShipSections,
    q_camera: Query<(&Camera, &GlobalTransform), With<ShipCameraMarker>>,
    q_viewport: Query<(Entity, &ComputedNode), With<ShipViewportMarker>>,
    q_blip: Query<&ShipBlip>,
    // Disjoint component queries so the dot's node / visibility / border /
    // background can each be updated by entity id without conflicting `&mut`.
    mut q_node: Query<&mut Node>,
    mut q_vis: Query<&mut Visibility>,
    mut q_border: Query<&mut BorderColor>,
    mut q_bg: Query<&mut BackgroundColor>,
) {
    if !runtime.active {
        return;
    }
    let (Ok((camera, cam_gt)), Ok((viewport, computed))) = (q_camera.single(), q_viewport.single())
    else {
        return;
    };
    let size = computed.size();
    let list = sections.collect();
    let font = nova_os_font(asset_server.as_deref());

    let mut seen = bevy::platform::collections::HashSet::new();
    for view in &list {
        seen.insert(view.entity);
        // Project the section's SCENE-space position (its local offset - the
        // scene root is anchored at the origin, so blocks and blips share this
        // frame). Projecting the world position would only line up when the ship
        // sits at the world origin, so blips would drift off the blocks in flight.
        let projected = camera
            .world_to_viewport(cam_gt, view.local.translation)
            .ok()
            .filter(|p| p.x >= 0.0 && p.y >= 0.0 && p.x <= size.x && p.y <= size.y);
        let selected = runtime.selected == Some(view.entity);
        let color = view.status_color();

        let blip = if let Some(&blip) = runtime.blips.get(&view.entity) {
            blip
        } else {
            let id = spawn_ship_blip(&mut commands, viewport, view, font.clone());
            runtime.blips.insert(view.entity, id);
            id
        };
        // A just-spawned blip is not queryable until the command flush, so its
        // updates simply wait a frame (the guards skip it).
        if q_blip.get(blip).is_err() {
            continue;
        }

        // The dot: position from the projection, fill coloured by status (so a
        // damaged section is spottable at a glance), amber border while selected.
        if let Ok(mut node) = q_node.get_mut(blip) {
            if let Some(p) = projected {
                node.left = Val::Px(p.x - SHIP_BLIP_PX * 0.5);
                node.top = Val::Px(p.y - SHIP_BLIP_PX * 0.5);
            }
        }
        if let Ok(mut vis) = q_vis.get_mut(blip) {
            *vis = match projected {
                Some(_) => Visibility::Inherited,
                None => Visibility::Hidden,
            };
        }
        if let Ok(mut bg) = q_bg.get_mut(blip) {
            bg.0 = color;
        }
        if let Ok(mut border) = q_border.get_mut(blip) {
            *border = if selected {
                BorderColor::all(NOVA_OS_AMBER)
            } else {
                // The same amber the spawn uses, just transparent (the border only
                // reads when selected).
                BorderColor::all(NOVA_OS_AMBER.with_alpha(0.0))
            };
        }
    }

    let stale: Vec<Entity> = runtime
        .blips
        .keys()
        .copied()
        .filter(|s| !seen.contains(s))
        .collect();
    for section in stale {
        if let Some(blip) = runtime.blips.remove(&section) {
            commands.entity(blip).try_despawn();
        }
    }
}

const SHIP_BLIP_PX: f32 = 12.0;
/// Fraction of the collider a block's green body fills; the remainder is the gap
/// to its neighbours that the bright outline frames.
const SHIP_BLOCK_FILL_SCALE: f32 = 0.86;

fn spawn_ship_blip(
    commands: &mut Commands,
    viewport: Entity,
    view: &ShipSectionView,
    font: Handle<Font>,
) -> Entity {
    let color = view.status_color();
    // The clickable dot: fill coloured by status (recoloured each frame in
    // `project_ship_blips`); its amber border marks the selection.
    let dot = commands
        .spawn((
            Button,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(SHIP_BLIP_PX),
                height: Val::Px(SHIP_BLIP_PX),
                border: UiRect::all(Val::Px(2.0)),
                border_radius: BorderRadius::MAX,
                ..default()
            },
            BorderColor::all(NOVA_OS_AMBER.with_alpha(0.0)),
            BackgroundColor(color),
        ))
        // Selection through `Activate` (fires for the forwarded NOVA OS pointer),
        // not `Interaction` polling (`rtt-ui-select-via-activate-not-interaction`).
        .observe(on_ship_blip_click)
        .id();

    // Label: the per-kind glyph + section code, in a dark backing pill so it reads
    // clearly against the phosphor scene instead of tiny green-on-green. Detail
    // (HP/ammo/status) lives in the inspector panel now, not on the blip.
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(14.0),
                top: Val::Px(-4.0),
                padding: UiRect::axes(Val::Px(4.0), Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(3.0)),
                ..default()
            },
            BackgroundColor(NOVA_OS_SCREEN.with_alpha(0.82)),
            ChildOf(dot),
        ))
        .with_children(|pill| {
            pill.spawn((
                Text::new(format!("{} {}", kind_glyph(view.kind), view.code)),
                nova_os_text_font(11.0, font),
                TextColor(NOVA_OS_TEXT),
            ));
        });

    commands.entity(dot).insert(ShipBlip {
        section: view.entity,
    });
    commands.entity(viewport).add_child(dot);
    dot
}

fn on_ship_blip_click(
    activate: On<Activate>,
    q_blip: Query<&ShipBlip>,
    mut runtime: ResMut<ShipRuntime>,
) {
    if let Ok(blip) = q_blip.get(activate.entity) {
        runtime.selected = Some(blip.section);
    }
}

/// Raise a Repair on the selected section when the panel button is clicked, unless
/// the panel marked repair disabled for it. Same seam as the `P` key and the CLI.
fn on_ship_repair_button(
    _activate: On<Activate>,
    runtime: Res<ShipRuntime>,
    mut commands: MessageWriter<ShipSectionCommand>,
) {
    if !runtime.panel_repair_enabled {
        return;
    }
    if let Some(target) = runtime.selected {
        commands.write(ShipSectionCommand {
            target,
            action: ShipAction::Repair,
        });
    }
}

/// Raise a Reload on the selected section when the panel button is clicked, unless
/// the panel marked reload disabled for it.
fn on_ship_reload_button(
    _activate: On<Activate>,
    runtime: Res<ShipRuntime>,
    mut commands: MessageWriter<ShipSectionCommand>,
) {
    if !runtime.panel_reload_enabled {
        return;
    }
    if let Some(target) = runtime.selected {
        commands.write(ShipSectionCommand {
            target,
            action: ShipAction::Reload,
        });
    }
}

/// Refresh the inspector panel from the current selection: title, live detail,
/// button enabled-state, and the note line (a transient action result, or the
/// reason a button is disabled). Caches the enabled flags for the observers.
fn update_ship_panel(
    mut runtime: ResMut<ShipRuntime>,
    sections: ShipSections,
    mut q_text: Query<(&ShipPanelField, &mut Text, &mut TextColor)>,
    mut q_button: Query<(&ShipPanelButton, &mut BorderColor, &mut BackgroundColor)>,
) {
    if !runtime.active {
        return;
    }
    let selected = runtime
        .selected
        .and_then(|sel| sections.collect().into_iter().find(|v| v.entity == sel));

    let (title, detail, detail_color, actions) = match &selected {
        Some(view) => (
            format!("{}  {}", view.code, view.name),
            panel_detail_text(view),
            if view.status() == "nominal" {
                NOVA_OS_TEXT
            } else {
                NOVA_OS_AMBER
            },
            panel_action_state(view),
        ),
        None => (
            "INSPECTOR".to_string(),
            "Select a section:\nclick a block or press [ / ].".to_string(),
            NOVA_OS_PHOSPHOR_MUTED,
            PanelActions::none(),
        ),
    };

    runtime.panel_repair_enabled = actions.repair_enabled;
    runtime.panel_reload_enabled = actions.reload_enabled;

    // Note line: a transient action result wins; else the disabled reason; else a
    // key hint when a section is selected.
    let (note, note_color) = if let Some((note, _)) = &runtime.note {
        (note.clone(), NOVA_OS_AMBER)
    } else if let Some(reason) = &actions.reason {
        (reason.clone(), NOVA_OS_PHOSPHOR_MUTED)
    } else if selected.is_some() {
        ("P repair   L reload".to_string(), NOVA_OS_PHOSPHOR_MUTED)
    } else {
        (String::new(), NOVA_OS_PHOSPHOR_MUTED)
    };

    for (field, mut text, mut color) in &mut q_text {
        let (value, tint) = match field {
            ShipPanelField::Title => (&title, NOVA_OS_PHOSPHOR),
            ShipPanelField::Detail => (&detail, detail_color),
            ShipPanelField::Note => (&note, note_color),
        };
        if text.0 != *value {
            text.0 = value.clone();
        }
        color.0 = tint;
    }

    for (button, mut border, mut background) in &mut q_button {
        let enabled = match button {
            ShipPanelButton::Repair => actions.repair_enabled,
            ShipPanelButton::Reload => actions.reload_enabled,
        };
        let (border_color, background_color) = if enabled {
            (NOVA_OS_PHOSPHOR, NOVA_OS_PHOSPHOR.with_alpha(0.14))
        } else {
            (NOVA_OS_PHOSPHOR_MUTED.with_alpha(0.35), NOVA_OS_SCREEN)
        };
        *border = BorderColor::all(border_color);
        background.0 = background_color;
    }
}

// ---------------------------------------------------------------------------
// Small render helpers
// ---------------------------------------------------------------------------

/// A `LineList` mesh of a unit cuboid's 12 edges (corners at +/-0.5, no face
/// diagonals), so a block can carry a crisp box outline instead of merging into
/// its neighbours. NORMAL and UV attributes are filled with placeholders: the
/// outline material is `unlit` and ignores them, but the mesh pipeline still
/// binds those vertex attributes.
fn cuboid_edges() -> Mesh {
    let c = 0.5;
    let corners = [
        Vec3::new(-c, -c, -c),
        Vec3::new(c, -c, -c),
        Vec3::new(c, c, -c),
        Vec3::new(-c, c, -c),
        Vec3::new(-c, -c, c),
        Vec3::new(c, -c, c),
        Vec3::new(c, c, c),
        Vec3::new(-c, c, c),
    ];
    // Four edges per end face + four connectors = the 12 cuboid edges.
    let edges = [
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 0),
        (4, 5),
        (5, 6),
        (6, 7),
        (7, 4),
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ];
    let positions: Vec<[f32; 3]> = edges
        .iter()
        .flat_map(|&(a, b)| [corners[a].to_array(), corners[b].to_array()])
        .collect();
    let count = positions.len();
    Mesh::new(PrimitiveTopology::LineList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; count])
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0]; count])
}

fn new_rtt_image(size: UVec2) -> Image {
    Image::new_target_texture(
        size.x.max(1),
        size.y.max(1),
        TextureFormat::Rgba8UnormSrgb,
        None,
    )
}

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
    use bevy::{ecs::system::RunSystemOnce, input::InputPlugin, state::app::StatesPlugin};

    use super::*;

    /// A terminal with the `ship` command tree registered (so the arg-bearing
    /// verbs resolve and set a pending invocation on submit).
    fn ship_terminal() -> NovaOsTerminal {
        let mut registry = NovaOsCommandRegistry::default();
        registry.register(ship_command_tree());
        let mut terminal = NovaOsTerminal::default();
        terminal.set_commands(registry.specs());
        terminal
    }

    /// The arg-bearing ship verbs register their `<section>` argument hint, so
    /// `<verb> help` renders a shell usage line naming the argument. Pins the
    /// registration-site `.with_arg_hint("<section>")` wiring end to end (the
    /// pure renderer is unit-tested in `nova_os`; this proves the ship tree
    /// actually feeds it the hint).
    #[test]
    fn ship_verb_help_names_the_section_argument() {
        for verb in ["ship section", "ship reload", "ship repair"] {
            let mut terminal = ship_terminal();
            terminal.insert_text(&format!("{verb} help"));
            terminal.submit(&TerminalCommandSnapshot::default());
            let want = format!("Usage: {verb} <section>");
            assert!(
                terminal.scrollback().iter().any(|row| row.text == want),
                "{verb} help should render `{want}`",
            );
        }
    }

    /// Spawn a scripted player ship: a hull, a turret (with ammo, critically
    /// damaged) and a healthy thruster. Sections carry NO `SectionCode` yet, so
    /// `assign_section_codes` mints them (HULL-1 / PDC-1 / THR-1).
    fn spawn_scripted_ship(world: &mut World) -> (Entity, Entity, Entity, Entity) {
        let ship = world
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.0)),
                Name::new("NOVA"),
            ))
            .id();
        let section_base = |name: &str, id: &str, xyz: Vec3| {
            (
                SectionMarker,
                Name::new(name.to_string()),
                EntityId::new(id.to_string()),
                Transform::from_translation(xyz),
                GlobalTransform::from(Transform::from_translation(xyz)),
                SectionCollider::Cuboid { size: Vec3::ONE },
                ChildOf(ship),
            )
        };
        let hull = world
            .spawn((
                section_base("Block Hull", "cube_a", Vec3::ZERO),
                HullSectionMarker,
                SectionDamageClass::Hull,
                Health {
                    current: 80.0,
                    max: 100.0,
                },
            ))
            .id();
        let turret = world
            .spawn((
                section_base("Bow gun", "cube_b", Vec3::new(2.0, 0.0, 0.0)),
                TurretSectionMarker,
                SectionDamageClass::Turret,
                Health {
                    current: 12.0,
                    max: 60.0,
                },
                SectionAmmo {
                    rounds: 2,
                    capacity: 6,
                },
            ))
            .id();
        let thruster = world
            .spawn((
                section_base("Main drive", "cube_c", Vec3::new(-2.0, 0.0, 0.0)),
                ThrusterSectionMarker,
                SectionDamageClass::Thruster,
                Health {
                    current: 100.0,
                    max: 100.0,
                },
            ))
            .id();
        (ship, hull, turret, thruster)
    }

    fn scrollback_text(app: &App) -> String {
        app.world()
            .resource::<NovaOsTerminal>()
            .scrollback()
            .iter()
            .map(|row| row.text.clone())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Submit a command line through the terminal (echo + resolve + queue), the
    /// way a real Enter does, so `apply_ship_cli_commands` can drain it.
    fn submit(app: &mut App, line: &str) {
        let mut terminal = app.world_mut().resource_mut::<NovaOsTerminal>();
        terminal.insert_text(line);
        terminal.submit(&TerminalCommandSnapshot::default());
    }

    #[test]
    fn section_codes_assigned_and_resolve() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        let (_ship, hull, turret, thruster) = spawn_scripted_ship(app.world_mut());

        app.world_mut()
            .run_system_once(assign_section_codes)
            .unwrap();

        // Codes minted per kind, stable-indexed by the authored id order.
        assert_eq!(app.world().get::<SectionCode>(hull).unwrap().0, "HULL-1");
        assert_eq!(app.world().get::<SectionCode>(turret).unwrap().0, "PDC-1");
        assert_eq!(app.world().get::<SectionCode>(thruster).unwrap().0, "THR-1");

        // A code resolves back to its section, case-insensitively.
        let resolved = app
            .world_mut()
            .run_system_once(|s: ShipSections| s.resolve("pdc-1").map(|v| v.entity))
            .unwrap();
        assert_eq!(resolved, Some(turret));

        // A second run is idempotent: no new codes, same values.
        app.world_mut()
            .run_system_once(assign_section_codes)
            .unwrap();
        assert_eq!(app.world().get::<SectionCode>(turret).unwrap().0, "PDC-1");
    }

    #[test]
    fn ship_reload_and_repair_apply_through_command_handler() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(ship_terminal());
        let (_ship, hull, turret, _thruster) = spawn_scripted_ship(app.world_mut());
        app.world_mut()
            .run_system_once(assign_section_codes)
            .unwrap();

        // `ship repair HULL-1` restores integrity through the handler.
        submit(&mut app, "ship repair HULL-1");
        app.world_mut()
            .run_system_once(apply_ship_cli_commands)
            .unwrap();
        assert_eq!(app.world().get::<Health>(hull).unwrap().current, 100.0);
        assert!(scrollback_text(&app).contains("repaired HULL-1"));

        // `ship reload PDC-1` refills ammo through the same seam (lowercase id).
        submit(&mut app, "ship reload pdc-1");
        app.world_mut()
            .run_system_once(apply_ship_cli_commands)
            .unwrap();
        assert_eq!(app.world().get::<SectionAmmo>(turret).unwrap().rounds, 6);
        assert!(scrollback_text(&app).contains("reloaded PDC-1"));

        // Reload on a non-weapon is a friendly error, not a mutation.
        submit(&mut app, "ship reload HULL-1");
        app.world_mut()
            .run_system_once(apply_ship_cli_commands)
            .unwrap();
        assert!(scrollback_text(&app).contains("no ammo feed"));

        // An unknown code reports and lists the valid codes.
        submit(&mut app, "ship repair BOGUS-9");
        app.world_mut()
            .run_system_once(apply_ship_cli_commands)
            .unwrap();
        assert!(scrollback_text(&app).contains("no such section: BOGUS-9"));
    }

    #[test]
    fn ship_section_detail_rows_from_live_data() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(ship_terminal());
        spawn_scripted_ship(app.world_mut());
        app.world_mut()
            .run_system_once(assign_section_codes)
            .unwrap();

        submit(&mut app, "ship section PDC-1");
        app.world_mut()
            .run_system_once(apply_ship_cli_commands)
            .unwrap();
        let text = scrollback_text(&app);
        assert!(text.contains("SECTION PDC-1 - Bow gun"), "{text}");
        assert!(text.contains("kind: turret"), "{text}");
        assert!(text.contains("ammo: 2/6"), "{text}");
        // 12/60 = 20% -> critical.
        assert!(text.contains("status: critical"), "{text}");
    }

    #[test]
    fn ship_verb_id_tab_completion() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<ShipRuntime>();
        app.insert_resource(ship_terminal());
        spawn_scripted_ship(app.world_mut());
        app.world_mut()
            .run_system_once(assign_section_codes)
            .unwrap();
        app.world_mut()
            .run_system_once(sync_ship_arg_completions)
            .unwrap();

        // `ship repair hu` Tab-completes to the HULL code (case-insensitive).
        let mut terminal = app.world_mut().resource_mut::<NovaOsTerminal>();
        terminal.insert_text("ship repair hu");
        assert!(terminal.complete());
        assert_eq!(terminal.prompt(), "ship repair HULL-1");
    }

    #[test]
    fn ship_action_keys_mutate_through_message_handler() {
        // The in-app L/P keys raise a ShipSectionCommand; the handler applies it
        // and flashes the result note. Pins the message path at its own boundary
        // (deleting apply_ship_section_commands must fail this).
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<ShipRuntime>();
        app.add_message::<ShipSectionCommand>();
        let (_ship, hull, turret, _thruster) = spawn_scripted_ship(app.world_mut());
        app.world_mut()
            .run_system_once(assign_section_codes)
            .unwrap();

        // Repair the hull through the message handler.
        app.world_mut().write_message(ShipSectionCommand {
            target: hull,
            action: ShipAction::Repair,
        });
        app.world_mut()
            .run_system_once(apply_ship_section_commands)
            .unwrap();
        assert_eq!(app.world().get::<Health>(hull).unwrap().current, 100.0);
        let note = app.world().resource::<ShipRuntime>().note.clone();
        assert!(
            note.map(|(text, _)| text.contains("repaired HULL-1"))
                .unwrap_or(false),
            "the handler flashes the result on the panel note line",
        );

        // Reload the turret through the message handler.
        app.world_mut().write_message(ShipSectionCommand {
            target: turret,
            action: ShipAction::Reload,
        });
        app.world_mut()
            .run_system_once(apply_ship_section_commands)
            .unwrap();
        assert_eq!(app.world().get::<SectionAmmo>(turret).unwrap().rounds, 6);
    }

    #[test]
    fn scene_blocks_use_local_space_when_ship_off_origin() {
        // Regression: the schematic scene is anchored at the origin and blocks sit
        // at each section's LOCAL offset; `project_ship_blips` projects that SAME
        // local offset, so blocks and blips stay aligned no matter where the ship
        // flies. Build the scene with the ship root far from the world origin and
        // assert the block sits at the local offset (not the off-origin world
        // position, which the old code projected the blips from).
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin, AssetPlugin::default()));
        app.init_asset::<Image>();
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
        app.insert_state(PauseStates::NovaOs);
        app.init_resource::<ShipRuntime>();
        let mut terminal = ship_terminal();
        terminal.enter_app(SHIP_APP_ID);
        app.insert_resource(terminal);

        let ship_world = Vec3::new(500.0, -200.0, 900.0);
        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::from_translation(ship_world),
                GlobalTransform::from(Transform::from_translation(ship_world)),
                Name::new("NOVA"),
            ))
            .id();
        let local_offset = Vec3::new(2.0, 0.0, 0.0);
        let turret = app
            .world_mut()
            .spawn((
                SectionMarker,
                Name::new("Bow gun"),
                EntityId::new("cube_b"),
                Transform::from_translation(local_offset),
                GlobalTransform::from(Transform::from_translation(ship_world + local_offset)),
                SectionCollider::Cuboid { size: Vec3::ONE },
                ChildOf(ship),
                TurretSectionMarker,
                SectionDamageClass::Turret,
                Health {
                    current: 60.0,
                    max: 60.0,
                },
            ))
            .id();
        app.world_mut()
            .run_system_once(assign_section_codes)
            .unwrap();
        app.world_mut().run_system_once(manage_ship_scene).unwrap();

        // The block is placed at the LOCAL offset, not the off-origin world pos.
        let block_pos = app
            .world_mut()
            .query_filtered::<(&ShipBlock, &Transform), With<ShipBlock>>()
            .iter(app.world())
            .find(|(block, _)| block.section == turret)
            .map(|(_, t)| t.translation);
        assert_eq!(
            block_pos,
            Some(local_offset),
            "the schematic block sits at the LOCAL offset, in the same space the blips project from",
        );
    }

    #[test]
    fn ship_app_launches_and_exits() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin));
        app.insert_state(PauseStates::NovaOs);
        app.init_resource::<ShipRuntime>();
        app.insert_resource(ship_terminal());

        // Bare `ship` launches the app (hands the screen over), not a CLI print.
        submit(&mut app, "ship");
        {
            let terminal = app.world().resource::<NovaOsTerminal>();
            assert_eq!(
                terminal.active_mode(),
                TerminalMode::App { id: SHIP_APP_ID }
            );
        }
        app.world_mut().run_system_once(manage_ship_scene).unwrap();
        assert!(app.world().resource::<ShipRuntime>().active);

        // Exiting returns to the prompt and tears the scene state down.
        app.world_mut().resource_mut::<NovaOsTerminal>().exit_app();
        app.world_mut().run_system_once(manage_ship_scene).unwrap();
        assert!(!app.world().resource::<ShipRuntime>().active);
        assert_eq!(
            app.world().resource::<NovaOsTerminal>().active_mode(),
            TerminalMode::Prompt
        );
    }

    #[test]
    fn ship_app_renders_blocks_and_selects_section() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            StatesPlugin,
            AssetPlugin::default(),
            InputPlugin,
        ));
        app.init_asset::<Image>();
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
        app.insert_state(PauseStates::NovaOs);
        app.init_resource::<ShipRuntime>();
        app.add_message::<ShipSectionCommand>();

        let mut terminal = ship_terminal();
        terminal.enter_app(SHIP_APP_ID);
        app.insert_resource(terminal);

        let (_ship, hull, _turret, _thruster) = spawn_scripted_ship(app.world_mut());
        app.world_mut()
            .run_system_once(assign_section_codes)
            .unwrap();

        // Build the scene from the live section colliders.
        app.world_mut().run_system_once(manage_ship_scene).unwrap();
        {
            let runtime = app.world().resource::<ShipRuntime>();
            assert!(runtime.active);
            assert!(runtime.scene_root.is_some(), "scene root spawned");
            assert!(runtime.image.is_some(), "RTT image created");
            assert!(
                runtime.selected.is_some(),
                "a section is selected by default"
            );
        }
        let blocks = app
            .world_mut()
            .query_filtered::<(), With<ShipBlock>>()
            .iter(app.world())
            .count();
        assert_eq!(blocks, 3, "one proxy block per live section");

        // Inspecting does not mutate gameplay state: the hull HP is unchanged.
        assert_eq!(app.world().get::<Health>(hull).unwrap().current, 80.0);

        // Cycling the selection with `]` advances to another section. Drive the
        // real input system through `run_system_once` so the pressed edge is not
        // cleared by InputPlugin's PreUpdate pass before the system reads it
        // (`nextstate-input-test-needs-clear-and-two-updates`).
        let before = app.world().resource::<ShipRuntime>().selected;
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::BracketRight);
        app.world_mut().run_system_once(ship_input).unwrap();
        let after = app.world().resource::<ShipRuntime>().selected;
        assert!(after.is_some() && after != before, "] cycles the selection");
    }

    /// LMB is the blip-SELECT button (the `Button` widget's Primary activation),
    /// so it must NOT orbit-drag the camera - a small press-with-motion has to
    /// stay a click, not become a drag that slides the blip out from under the
    /// cursor. RMB remains the orbit-drag button.
    #[test]
    fn ship_orbit_drag_is_rmb_only() {
        use bevy::input::mouse::MouseMotion;

        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            StatesPlugin,
            AssetPlugin::default(),
            InputPlugin,
        ));
        app.init_asset::<Image>();
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
        app.insert_state(PauseStates::NovaOs);
        app.init_resource::<ShipRuntime>();
        app.add_message::<ShipSectionCommand>();

        let mut terminal = ship_terminal();
        terminal.enter_app(SHIP_APP_ID);
        app.insert_resource(terminal);

        spawn_scripted_ship(app.world_mut());
        app.world_mut()
            .run_system_once(assign_section_codes)
            .unwrap();
        app.world_mut().run_system_once(manage_ship_scene).unwrap();

        let orbit_angles = |app: &mut App| {
            app.world_mut()
                .query_filtered::<&ShipOrbit, With<ShipCameraMarker>>()
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
        app.world_mut().run_system_once(ship_input).unwrap();
        assert_eq!(
            orbit_angles(&mut app),
            before,
            "LMB drag must NOT orbit the ship camera (it selects blips)"
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
        app.world_mut().run_system_once(ship_input).unwrap();
        assert_ne!(
            orbit_angles(&mut app),
            before,
            "RMB drag must still orbit the ship camera"
        );
    }

    /// Spawn a player ship whose root is OFF-origin and rotated, with each
    /// section's world pose (`GlobalTransform`) deliberately DIFFERENT from its
    /// local `Transform`. The orbit recenter reads the LOCAL frame (the scene is
    /// built from `Transform`, like the blips), so a regression that read the
    /// world frame would retarget to the wrong place - invisible at the origin
    /// (`spatial-fixture-off-the-trivial-point`). Returns (hull, turret, thruster);
    /// local translations are hull (3,2,-1), turret (9,2,-1), thruster (-3,2,-1),
    /// so the centroid is (3,2,-1) and the turret is well off it.
    fn spawn_offset_ship(world: &mut World) -> (Entity, Entity, Entity) {
        let root_world = Transform::from_translation(Vec3::new(120.0, -40.0, 75.0))
            .with_rotation(Quat::from_rotation_y(0.8));
        let ship = world
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                root_world,
                GlobalTransform::from(root_world),
                Name::new("NOVA"),
            ))
            .id();
        // Local Transform is the scene frame; GlobalTransform is the genuinely
        // composed world pose (`root_world * local`), so the two frames are
        // consistent AND clearly distinct - the recenter must read local.
        let section = |name: &str, id: &str, local: Vec3| {
            (
                SectionMarker,
                Name::new(name.to_string()),
                EntityId::new(id.to_string()),
                Transform::from_translation(local),
                GlobalTransform::from(root_world.mul_transform(Transform::from_translation(local))),
                SectionCollider::Cuboid { size: Vec3::ONE },
                ChildOf(ship),
            )
        };
        let hull = world
            .spawn((
                section("Block Hull", "cube_a", Vec3::new(3.0, 2.0, -1.0)),
                HullSectionMarker,
                SectionDamageClass::Hull,
                Health {
                    current: 80.0,
                    max: 100.0,
                },
            ))
            .id();
        let turret = world
            .spawn((
                section("Bow gun", "cube_b", Vec3::new(9.0, 2.0, -1.0)),
                TurretSectionMarker,
                SectionDamageClass::Turret,
                Health {
                    current: 12.0,
                    max: 60.0,
                },
                SectionAmmo {
                    rounds: 2,
                    capacity: 6,
                },
            ))
            .id();
        let thruster = world
            .spawn((
                section("Main drive", "cube_c", Vec3::new(-3.0, 2.0, -1.0)),
                ThrusterSectionMarker,
                SectionDamageClass::Thruster,
                Health {
                    current: 100.0,
                    max: 100.0,
                },
            ))
            .id();
        (hull, turret, thruster)
    }

    /// A ship app fixture built from [`spawn_offset_ship`], with the scene already
    /// managed. Returns the app plus (hull, turret, thruster).
    fn offset_ship_app() -> (App, Entity, Entity, Entity) {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            StatesPlugin,
            AssetPlugin::default(),
            InputPlugin,
        ));
        app.init_asset::<Image>();
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
        app.insert_state(PauseStates::NovaOs);
        app.init_resource::<ShipRuntime>();
        app.add_message::<ShipSectionCommand>();

        let mut terminal = ship_terminal();
        terminal.enter_app(SHIP_APP_ID);
        app.insert_resource(terminal);

        let (hull, turret, thruster) = spawn_offset_ship(app.world_mut());
        app.world_mut()
            .run_system_once(assign_section_codes)
            .unwrap();
        app.world_mut().run_system_once(manage_ship_scene).unwrap();
        (app, hull, turret, thruster)
    }

    fn ship_orbit(app: &mut App) -> ShipOrbit {
        *app.world_mut()
            .query_filtered::<&ShipOrbit, With<ShipCameraMarker>>()
            .single(app.world())
            .unwrap()
    }

    /// Advance the generic clock and drive the camera `frames` times, so the
    /// exponential center ease actually integrates over simulated time.
    fn drive_frames(app: &mut App, frames: usize, dt: f32) {
        for _ in 0..frames {
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(std::time::Duration::from_secs_f32(dt));
            app.world_mut().run_system_once(drive_ship_camera).unwrap();
        }
    }

    /// Selecting a non-default section retargets the orbit center to that section's
    /// LOCAL position, and `drive_ship_camera` eases the live center onto it. The
    /// app still OPENS framed on the whole-ship centroid. Fails if the ease is a
    /// no-op (`test-the-wiring-system-not-just-its-pure-helpers`).
    #[test]
    fn ship_orbit_recenters_on_selected_section() {
        let (mut app, _hull, turret, _thruster) = offset_ship_app();

        let centroid = Vec3::new(3.0, 2.0, -1.0);
        let orbit = ship_orbit(&mut app);
        assert!(
            orbit.center.abs_diff_eq(centroid, 1e-4)
                && orbit.center_target.abs_diff_eq(centroid, 1e-4),
            "app opens framed on the whole-ship centroid, not a section"
        );

        // Idling with no selection change must NOT drift off the whole-ship view
        // (the default selection is treated as already centered at home).
        app.world_mut().run_system_once(ship_input).unwrap();
        drive_frames(&mut app, 30, 1.0 / 60.0);
        let orbit = ship_orbit(&mut app);
        assert!(
            orbit.center.abs_diff_eq(centroid, 1e-3),
            "no selection change keeps the center on the whole-ship centroid, got {:?}",
            orbit.center
        );

        // Select the turret and reconcile: the center RETARGETS to the turret's
        // LOCAL translation (9,2,-1), not its world pose.
        let turret_local = Vec3::new(9.0, 2.0, -1.0);
        app.world_mut().resource_mut::<ShipRuntime>().selected = Some(turret);
        app.world_mut().run_system_once(ship_input).unwrap();
        let orbit = ship_orbit(&mut app);
        assert_eq!(
            orbit.centered_on,
            Some(turret),
            "reconcile records the new selection"
        );
        assert!(
            orbit.center_target.abs_diff_eq(turret_local, 1e-4),
            "center_target follows the selected section's LOCAL position, got {:?}",
            orbit.center_target
        );
        // The eased center has not jumped yet - it must be driven there over frames.
        assert!(
            orbit.center.abs_diff_eq(centroid, 1e-3),
            "center does not snap on selection; it eases"
        );

        // Drive the camera: the live center converges onto the target and leaves
        // the centroid behind. This is the assertion that fails if the ease no-ops.
        drive_frames(&mut app, 60, 1.0 / 60.0);
        let orbit = ship_orbit(&mut app);
        assert!(
            orbit.center.abs_diff_eq(turret_local, 1e-2),
            "eased center reaches the selected section, got {:?}",
            orbit.center
        );
        assert!(
            orbit.center.distance(centroid) > 1.0,
            "eased center actually moved off the centroid (ease is not a no-op)"
        );
    }

    /// `T` re-frames the whole ship (center home + default angles) and the reframe
    /// STICKS: the selection reconcile does not chase the still-selected section
    /// back on the next frame (DECISION.md).
    #[test]
    fn ship_reset_reframes_whole_ship_and_sticks() {
        let (mut app, _hull, turret, _thruster) = offset_ship_app();
        let centroid = Vec3::new(3.0, 2.0, -1.0);
        let turret_local = Vec3::new(9.0, 2.0, -1.0);

        // Select the turret and ease the center onto it.
        app.world_mut().resource_mut::<ShipRuntime>().selected = Some(turret);
        app.world_mut().run_system_once(ship_input).unwrap();
        drive_frames(&mut app, 60, 1.0 / 60.0);
        assert!(
            ship_orbit(&mut app).center.abs_diff_eq(turret_local, 1e-2),
            "precondition: center is on the turret before reset"
        );

        // Nudge the angles off default so T's angle reset is observable too.
        {
            let mut orbit = app
                .world_mut()
                .query_filtered::<&mut ShipOrbit, With<ShipCameraMarker>>()
                .single_mut(app.world_mut())
                .unwrap();
            orbit.theta += 0.5;
            orbit.phi = 0.3;
        }

        // Press T: retarget home, reset angles, and consume the selection.
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyT);
        app.world_mut().run_system_once(ship_input).unwrap();
        let orbit = ship_orbit(&mut app);
        assert!(
            orbit.center_target.abs_diff_eq(centroid, 1e-4),
            "T retargets the center to the whole-ship centroid, got {:?}",
            orbit.center_target
        );
        assert_eq!(
            orbit.centered_on,
            Some(turret),
            "T consumes the selection so the reframe is not chased back"
        );
        assert!(
            (orbit.theta - SHIP_THETA_DEFAULT).abs() < 1e-4
                && (orbit.phi - SHIP_PHI_DEFAULT).abs() < 1e-4,
            "T restores the default orbit angles"
        );

        // Release T and reconcile again with the turret STILL selected: the center
        // target must stay home, not snap back to the turret.
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::KeyT);
        app.world_mut().run_system_once(ship_input).unwrap();
        drive_frames(&mut app, 60, 1.0 / 60.0);
        let orbit = ship_orbit(&mut app);
        assert!(
            orbit.center_target.abs_diff_eq(centroid, 1e-4),
            "the whole-ship reframe STICKS while the section stays selected"
        );
        assert!(
            orbit.center.abs_diff_eq(centroid, 1e-2),
            "the eased center settles back on the centroid, got {:?}",
            orbit.center
        );
    }

    /// A bare [`ShipSectionView`] for the pure-helper tests.
    fn view_fixture(
        kind: SectionDamageClass,
        health: Option<Health>,
        ammo: Option<SectionAmmo>,
    ) -> ShipSectionView {
        ShipSectionView {
            entity: Entity::PLACEHOLDER,
            code: "PDC-1".to_string(),
            kind,
            name: "Bow gun".to_string(),
            local: Transform::default(),
            half_extents: Vec3::ONE,
            health,
            ammo,
            inactive: false,
            zero_health: false,
        }
    }

    #[test]
    fn kind_glyph_distinct_per_kind() {
        use bevy::platform::collections::HashSet;
        let kinds = [
            SectionDamageClass::Hull,
            SectionDamageClass::Thruster,
            SectionDamageClass::Controller,
            SectionDamageClass::Turret,
            SectionDamageClass::Torpedo,
        ];
        let glyphs: Vec<&str> = kinds.iter().map(|&k| kind_glyph(k)).collect();
        assert!(
            glyphs.iter().all(|g| !g.is_empty()),
            "every kind has a glyph: {glyphs:?}"
        );
        let unique: HashSet<&&str> = glyphs.iter().collect();
        assert_eq!(
            unique.len(),
            kinds.len(),
            "each kind's glyph is distinct: {glyphs:?}"
        );
    }

    #[test]
    fn blocks_stay_uniform_green_regardless_of_status() {
        // Regression pin on DECISION.md: the block FILL colour no longer encodes
        // status, so a critically-damaged section and a healthy one share the same
        // uniform-green fill material handle.
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin, AssetPlugin::default()));
        app.init_asset::<Image>();
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
        app.insert_state(PauseStates::NovaOs);
        app.init_resource::<ShipRuntime>();
        let mut terminal = ship_terminal();
        terminal.enter_app(SHIP_APP_ID);
        app.insert_resource(terminal);

        // Off-origin root (spatial-fixture-off-the-trivial-point).
        let ship_world = Vec3::new(500.0, -200.0, 900.0);
        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::from_translation(ship_world),
                GlobalTransform::from(Transform::from_translation(ship_world)),
                Name::new("NOVA"),
            ))
            .id();
        let section = |name: &str, id: &str, offset: Vec3| {
            (
                SectionMarker,
                Name::new(name.to_string()),
                EntityId::new(id.to_string()),
                Transform::from_translation(offset),
                GlobalTransform::from(Transform::from_translation(ship_world + offset)),
                SectionCollider::Cuboid { size: Vec3::ONE },
                ChildOf(ship),
            )
        };
        let hull = app
            .world_mut()
            .spawn((
                section("Block Hull", "cube_a", Vec3::ZERO),
                HullSectionMarker,
                SectionDamageClass::Hull,
                Health {
                    current: 100.0,
                    max: 100.0,
                },
            ))
            .id();
        let turret = app
            .world_mut()
            .spawn((
                section("Bow gun", "cube_b", Vec3::new(3.0, 0.0, 0.0)),
                TurretSectionMarker,
                SectionDamageClass::Turret,
                // 12/60 = critical.
                Health {
                    current: 12.0,
                    max: 60.0,
                },
                SectionAmmo {
                    rounds: 2,
                    capacity: 6,
                },
            ))
            .id();
        app.world_mut()
            .run_system_once(assign_section_codes)
            .unwrap();
        app.world_mut().run_system_once(manage_ship_scene).unwrap();

        let fill_of = |app: &mut App, sect: Entity| -> Handle<StandardMaterial> {
            app.world_mut()
                .query::<(&ShipBlock, &MeshMaterial3d<StandardMaterial>)>()
                .iter(app.world())
                .find(|(b, _)| b.section == sect)
                .map(|(_, m)| m.0.clone())
                .expect("block fill for section")
        };
        let hull_fill = fill_of(&mut app, hull);
        let turret_fill = fill_of(&mut app, turret);
        assert_eq!(
            hull_fill, turret_fill,
            "a critical section's fill uses the SAME uniform-green handle as a nominal one",
        );
    }

    #[test]
    fn blip_is_status_dot_with_labelled_marker() {
        // The blip is a status-coloured dot + a labelled marker; the integrity bar
        // and ammo pips are gone (HP/ammo live in the inspector panel now). A
        // critical turret's dot reads amber, and no ammo pips remain.
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Font>();

        let viewport = app.world_mut().spawn_empty().id();
        // 12/60 = 20% -> critical -> amber dot.
        let view = view_fixture(
            SectionDamageClass::Turret,
            Some(Health {
                current: 12.0,
                max: 60.0,
            }),
            Some(SectionAmmo {
                rounds: 2,
                capacity: 6,
            }),
        );
        let blip = app
            .world_mut()
            .run_system_once(move |mut commands: Commands| {
                spawn_ship_blip(&mut commands, viewport, &view, Handle::default())
            })
            .unwrap();

        // The dot fill reflects status (critical -> amber).
        let dot_bg = app
            .world()
            .get::<BackgroundColor>(blip)
            .expect("dot background")
            .0;
        assert_eq!(
            dot_bg, NOVA_OS_AMBER,
            "a critical section's dot reads amber"
        );

        // Some descendant label carries the kind glyph + code, and no ammo pips
        // survive anywhere on the blip.
        let label = format!("{} {}", kind_glyph(SectionDamageClass::Turret), "PDC-1");
        let texts: Vec<String> = app
            .world_mut()
            .query::<&Text>()
            .iter(app.world())
            .map(|text| text.0.clone())
            .collect();
        assert!(
            texts.iter().any(|t| *t == label),
            "the blip label reads '{label}', got {texts:?}",
        );
        assert!(
            !texts.iter().any(|t| t.contains('●') || t.contains('○')),
            "no ammo pips remain on the blip: {texts:?}",
        );
    }

    #[test]
    fn unknown_health_reads_nominal() {
        // A section with no `Health` reads nominal (green), never a misleading
        // damaged state - the edge the deleted bar/pips test used to pin, kept
        // because `status`/`status_color` now drive the blip dot and the panel.
        let unknown = view_fixture(SectionDamageClass::Thruster, None, None);
        assert_eq!(unknown.status(), "nominal");
        assert_eq!(unknown.status_color(), NOVA_OS_PHOSPHOR);
    }

    #[test]
    fn panel_action_state_gates_repair_and_reload() {
        // Hull: repairable, no ammo feed -> reload disabled with the handler's text.
        let hull = view_fixture(
            SectionDamageClass::Hull,
            Some(Health {
                current: 80.0,
                max: 100.0,
            }),
            None,
        );
        let a = panel_action_state(&hull);
        assert!(a.repair_enabled, "a hull with HP is repairable");
        assert!(!a.reload_enabled, "a hull has no ammo feed");
        assert!(
            a.reason.as_deref().unwrap().contains("no ammo feed"),
            "{:?}",
            a.reason
        );

        // Armed turret with ammo + HP: both enabled, no reason.
        let turret = view_fixture(
            SectionDamageClass::Turret,
            Some(Health {
                current: 12.0,
                max: 60.0,
            }),
            Some(SectionAmmo {
                rounds: 2,
                capacity: 6,
            }),
        );
        let a = panel_action_state(&turret);
        assert!(a.repair_enabled && a.reload_enabled, "armed turret: both");
        assert!(a.reason.is_none(), "no disabled reason: {:?}", a.reason);

        // Weapon with ammo but NO health: reload enabled, repair disabled w/ reason.
        let ghost = view_fixture(
            SectionDamageClass::Turret,
            None,
            Some(SectionAmmo {
                rounds: 0,
                capacity: 6,
            }),
        );
        let a = panel_action_state(&ghost);
        assert!(a.reload_enabled && !a.repair_enabled);
        assert!(
            a.reason
                .as_deref()
                .unwrap()
                .contains("no integrity to restore"),
            "{:?}",
            a.reason
        );
    }

    #[test]
    fn panel_detail_text_covers_live_fields() {
        let turret = view_fixture(
            SectionDamageClass::Turret,
            Some(Health {
                current: 12.0,
                max: 60.0,
            }),
            Some(SectionAmmo {
                rounds: 2,
                capacity: 6,
            }),
        );
        let text = panel_detail_text(&turret);
        assert!(text.contains("kind: turret"), "{text}");
        assert!(
            text.contains(kind_description(SectionDamageClass::Turret)),
            "{text}"
        );
        assert!(text.contains("20%"), "12/60 -> 20%: {text}");
        assert!(text.contains("status: critical"), "{text}");
        assert!(text.contains("ammo: 2/6"), "{text}");
    }

    #[test]
    fn panel_buttons_raise_ship_section_command() {
        // Each button's `Activate` observer routes a ShipSectionCommand for the
        // selected section - but only when the panel marked that action enabled.
        // Pins BOTH button entry points at their own boundary
        // (`pin-each-caller-not-just-shared-core`).
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<ShipRuntime>();
        app.add_message::<ShipSectionCommand>();
        let (_ship, hull, turret, _thruster) = spawn_scripted_ship(app.world_mut());
        app.world_mut()
            .run_system_once(assign_section_codes)
            .unwrap();

        // --- Reload button on the turret (starts at 2/6). ---
        let reload = app
            .world_mut()
            .spawn(ShipPanelButton::Reload)
            .observe(on_ship_reload_button)
            .id();
        app.world_mut().resource_mut::<ShipRuntime>().selected = Some(turret);

        // Disabled: activating the button is a no-op (ammo unchanged at 2).
        app.world_mut()
            .resource_mut::<ShipRuntime>()
            .panel_reload_enabled = false;
        app.world_mut().trigger(Activate { entity: reload });
        app.world_mut()
            .run_system_once(apply_ship_section_commands)
            .unwrap();
        assert_eq!(
            app.world().get::<SectionAmmo>(turret).unwrap().rounds,
            2,
            "a disabled reload button writes no command",
        );

        // Enabled: activating routes the command and refills ammo to capacity.
        app.world_mut()
            .resource_mut::<ShipRuntime>()
            .panel_reload_enabled = true;
        app.world_mut().trigger(Activate { entity: reload });
        app.world_mut()
            .run_system_once(apply_ship_section_commands)
            .unwrap();
        assert_eq!(
            app.world().get::<SectionAmmo>(turret).unwrap().rounds,
            6,
            "an enabled reload button routes through the ShipSectionCommand seam",
        );

        // --- Repair button on the hull (starts at 80/100), the other caller. ---
        let repair = app
            .world_mut()
            .spawn(ShipPanelButton::Repair)
            .observe(on_ship_repair_button)
            .id();
        app.world_mut().resource_mut::<ShipRuntime>().selected = Some(hull);

        app.world_mut()
            .resource_mut::<ShipRuntime>()
            .panel_repair_enabled = false;
        app.world_mut().trigger(Activate { entity: repair });
        app.world_mut()
            .run_system_once(apply_ship_section_commands)
            .unwrap();
        assert_eq!(
            app.world().get::<Health>(hull).unwrap().current,
            80.0,
            "a disabled repair button writes no command",
        );

        app.world_mut()
            .resource_mut::<ShipRuntime>()
            .panel_repair_enabled = true;
        app.world_mut().trigger(Activate { entity: repair });
        app.world_mut()
            .run_system_once(apply_ship_section_commands)
            .unwrap();
        assert_eq!(
            app.world().get::<Health>(hull).unwrap().current,
            100.0,
            "an enabled repair button routes through the ShipSectionCommand seam",
        );
    }

    #[test]
    fn update_ship_panel_reflects_selection() {
        // The live refresh system wires the pure helpers into the panel tree and
        // caches the button-enabled flags the observers read. Reverting it to a
        // no-op must fail this (the detail text would stay the placeholder and the
        // flags stay false).
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Font>();
        app.init_resource::<ShipRuntime>();
        let (_ship, hull, _turret, _thruster) = spawn_scripted_ship(app.world_mut());
        app.world_mut()
            .run_system_once(assign_section_codes)
            .unwrap();

        // Build the panel subtree under a root, the way `spawn_body` does.
        let root = app.world_mut().spawn_empty().id();
        app.world_mut()
            .run_system_once(move |mut commands: Commands| {
                commands
                    .entity(root)
                    .with_children(|parent| spawn_ship_panel(parent, Handle::default()));
            })
            .unwrap();

        {
            let mut runtime = app.world_mut().resource_mut::<ShipRuntime>();
            runtime.active = true;
            runtime.selected = Some(hull);
        }
        app.world_mut().run_system_once(update_ship_panel).unwrap();

        let field_text = |app: &mut App, want: ShipPanelField| -> String {
            app.world_mut()
                .query::<(&ShipPanelField, &Text)>()
                .iter(app.world())
                .find(|(field, _)| **field == want)
                .map(|(_, text)| text.0.clone())
                .expect("panel field text")
        };
        let detail = field_text(&mut app, ShipPanelField::Detail);
        assert!(
            detail.contains("kind: hull"),
            "detail reflects kind: {detail}"
        );
        assert!(detail.contains("status:"), "detail has status: {detail}");
        let title = field_text(&mut app, ShipPanelField::Title);
        assert!(title.contains("HULL-1"), "title reflects the code: {title}");

        // A hull caches reload DISABLED and repair ENABLED for the observers.
        let runtime = app.world().resource::<ShipRuntime>();
        assert!(
            !runtime.panel_reload_enabled,
            "a hull has no ammo feed -> reload disabled",
        );
        assert!(
            runtime.panel_repair_enabled,
            "a hull with HP is repairable -> repair enabled",
        );
    }
}
