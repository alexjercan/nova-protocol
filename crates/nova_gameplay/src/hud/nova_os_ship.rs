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
//! [`RenderLayers`] renders unlit green-phosphor proxy BLOCKS (built from each
//! section's authored [`SectionCollider`] + local transform) into an offscreen
//! image shown in the app body; the interactive SECTIONS ride on top as projected
//! clickable UI blips labelled with their code (a nested 3D mesh is not pickable
//! through the CRT composite, but a UI button is - the same reason `map` uses
//! blips). Orbit the camera with Q/E/R/F + drag + wheel; `[`/`]` cycle the
//! selection; `L` reloads and `P` repairs the selected section.
//!
//! Actions are instant and free for now, but they route through a single
//! [`ShipSectionCommand`] seam (CLI verb -> [`NovaOsCommandInvocation`], in-app key
//! -> message) so a future queued/over-time, resource-costed model can replace the
//! handler without touching the callers (DECISION fork 4).

use bevy::{
    camera::{visibility::RenderLayers, ImageRenderTarget, RenderTarget},
    ecs::system::SystemParam,
    input::mouse::{MouseMotion, MouseWheel},
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
        body.spawn((
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
        body.spawn((
            ShipReadoutMarker,
            Text::new("Select a section: click a block or press [ / ]."),
            nova_os_text_font(DRAWER_LINE_FONT_PX, font),
            TextColor(NOVA_OS_PHOSPHOR_MUTED),
            Node {
                min_height: Val::Px(26.0),
                ..default()
            },
        ));
    }
}

/// Dark fill behind the schematic image.
const SHIP_VIEW_BG: Color = Color::srgb_u8(0, 6, 3);

#[derive(Component)]
struct ShipViewportMarker;
#[derive(Component)]
struct ShipReadoutMarker;
#[derive(Component)]
struct ShipCameraMarker;
#[derive(Component)]
struct ShipSceneRoot;

/// A schematic block for one section (holds the section entity so its material
/// can track live status).
#[derive(Component)]
struct ShipBlock {
    section: Entity,
}

/// A projected, clickable section blip over the viewport.
#[derive(Component)]
struct ShipBlip {
    section: Entity,
}

/// The ship camera's orbit state (own spherical math, like `MapOrbit`, because
/// the shared smoothed orbit does not drive an RTT camera -
/// `verify-reused-driver-actually-moves`).
#[derive(Component)]
struct ShipOrbit {
    theta: f32,
    phi: f32,
    radius: f32,
    center: Vec3,
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
    /// Shared status-bucket materials for the blocks (nominal/critical/inactive),
    /// so live damage/repair recolours blocks without spawning materials per frame.
    mat_nominal: Option<Handle<StandardMaterial>>,
    mat_degraded: Option<Handle<StandardMaterial>>,
    mat_critical: Option<Handle<StandardMaterial>>,
    mat_inactive: Option<Handle<StandardMaterial>>,
    selected: Option<Entity>,
    /// The codes last pushed as arg-completions, so the terminal is only marked
    /// changed when the set changes.
    completion_codes: Vec<String>,
    /// A transient note (e.g. an in-app action result) shown in the readout.
    note: Option<(String, f32)>,
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
                update_ship_readout,
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
        .with_subcommand(TerminalCommand::gameplay(
            "ship section",
            "Show one section's detail",
            CommandArity::UpTo(1),
        ))
        .with_subcommand(TerminalCommand::gameplay(
            "ship reload",
            "Reload a weapon section",
            CommandArity::UpTo(1),
        ))
        .with_subcommand(TerminalCommand::gameplay(
            "ship repair",
            "Repair a section",
            CommandArity::UpTo(1),
        ))
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
    let mut map = bevy::platform::collections::HashMap::new();
    for verb in ["ship section", "ship reload", "ship repair"] {
        map.insert(verb, codes.clone());
    }
    // The `!=` gate above already ensured the set changed, so set unconditionally
    // here (this is the only path that marks the terminal changed).
    terminal.set_arg_completions(map.into_iter().collect());
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

/// Apply in-app [`ShipSectionCommand`] messages (the `L`/`P` action keys), and
/// flash the result in the app readout.
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

    // Status-bucket materials, reused across blocks and swapped as sections take
    // damage or are repaired (`update_ship_blocks`).
    runtime.mat_nominal = Some(materials.add(unlit(NOVA_OS_PHOSPHOR.with_alpha(0.85))));
    runtime.mat_degraded = Some(materials.add(unlit(NOVA_OS_PHOSPHOR_MUTED.with_alpha(0.85))));
    runtime.mat_critical = Some(materials.add(unlit(NOVA_OS_AMBER.with_alpha(0.9))));
    runtime.mat_inactive = Some(materials.add(unlit(NOVA_OS_PHOSPHOR_DIM.with_alpha(0.3))));

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
        },
        ChildOf(scene_root),
    ));

    // One unlit proxy block per section, at the section's local placement, sized
    // from its authored collider extents.
    let nominal = runtime.mat_nominal.clone().unwrap();
    for view in &views {
        let size = (view.half_extents * 2.0).max(Vec3::splat(0.2));
        let mesh = meshes.add(Cuboid::new(size.x, size.y, size.z));
        commands.spawn((
            ShipBlock {
                section: view.entity,
            },
            Mesh3d(mesh),
            MeshMaterial3d(nominal.clone()),
            Transform {
                translation: view.local.translation,
                rotation: view.local.rotation,
                scale: Vec3::ONE,
            },
            RenderLayers::layer(SHIP_LAYER),
            ChildOf(scene_root),
        ));
    }

    runtime.scene_root = Some(scene_root);
    // Default the selection to the first section so the readout is useful at once.
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
fn drive_ship_camera(mut q_camera: Query<(&mut Transform, &ShipOrbit), With<ShipCameraMarker>>) {
    let Ok((mut transform, orbit)) = q_camera.single_mut() else {
        return;
    };
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
        if mouse_buttons.pressed(MouseButton::Right) || mouse_buttons.pressed(MouseButton::Left) {
            orbit.theta -= motion_delta.x * 0.0024;
            orbit.phi = (orbit.phi + motion_delta.y * 0.0024).clamp(0.12, 1.45);
        }
        if wheel_delta != 0.0 {
            orbit.radius =
                (orbit.radius * (1.0 - wheel_delta * 0.12)).clamp(SHIP_RADIUS_MIN, SHIP_RADIUS_MAX);
        }
        if keys.just_pressed(KeyCode::KeyT) {
            orbit.theta = SHIP_THETA_DEFAULT;
            orbit.phi = SHIP_PHI_DEFAULT;
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

    // Actions on the selected section: L reload, P repair. Route through the same
    // message the readout reflects and the mutation handler applies.
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

/// Recolour each block to its section's live status (so a repair/reload or damage
/// is reflected), and tint the selected block toward amber.
fn update_ship_blocks(
    runtime: Res<ShipRuntime>,
    sections: ShipSections,
    mut q_block: Query<(&ShipBlock, &mut MeshMaterial3d<StandardMaterial>)>,
) {
    if !runtime.active {
        return;
    }
    let views = sections.collect();
    for (block, mut material) in &mut q_block {
        let Some(view) = views.iter().find(|v| v.entity == block.section) else {
            continue;
        };
        let handle = match view.status() {
            "neutralized" => &runtime.mat_inactive,
            "critical" => &runtime.mat_critical,
            "degraded" => &runtime.mat_degraded,
            _ => &runtime.mat_nominal,
        };
        if let Some(handle) = handle {
            if material.0 != *handle {
                material.0 = handle.clone();
            }
        }
    }
}

/// Project each section through the ship camera into the viewport and keep a
/// clickable, labelled UI blip per section in sync.
#[allow(clippy::type_complexity)]
fn project_ship_blips(
    mut commands: Commands,
    mut runtime: ResMut<ShipRuntime>,
    asset_server: Option<Res<AssetServer>>,
    sections: ShipSections,
    q_camera: Query<(&Camera, &GlobalTransform), With<ShipCameraMarker>>,
    q_viewport: Query<(Entity, &ComputedNode), With<ShipViewportMarker>>,
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
        if let Ok((mut node, mut vis, mut bg, mut border)) = q_blip.get_mut(blip) {
            match projected {
                Some(p) => {
                    node.left = Val::Px(p.x - SHIP_BLIP_PX * 0.5);
                    node.top = Val::Px(p.y - SHIP_BLIP_PX * 0.5);
                    *vis = Visibility::Inherited;
                }
                None => *vis = Visibility::Hidden,
            }
            bg.0 = color;
            *border = if selected {
                BorderColor::all(NOVA_OS_AMBER)
            } else {
                BorderColor::all(color.with_alpha(0.0))
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

fn spawn_ship_blip(
    commands: &mut Commands,
    viewport: Entity,
    view: &ShipSectionView,
    font: Handle<Font>,
) -> Entity {
    let color = view.status_color();
    let id = commands
        .spawn((
            ShipBlip {
                section: view.entity,
            },
            Button,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(SHIP_BLIP_PX),
                height: Val::Px(SHIP_BLIP_PX),
                border: UiRect::all(Val::Px(2.0)),
                border_radius: BorderRadius::MAX,
                ..default()
            },
            BorderColor::all(color.with_alpha(0.0)),
            BackgroundColor(color),
        ))
        // Selection through `Activate` (fires for the forwarded NOVA OS pointer),
        // not `Interaction` polling (`rtt-ui-select-via-activate-not-interaction`).
        .observe(on_ship_blip_click)
        .id();
    commands.spawn((
        Text::new(view.code.clone()),
        nova_os_text_font(11.0, font),
        TextColor(color),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(14.0),
            top: Val::Px(-3.0),
            ..default()
        },
        ChildOf(id),
    ));
    commands.entity(viewport).add_child(id);
    id
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

/// Fill the readout from the current selection (or a transient action note).
fn update_ship_readout(
    runtime: Res<ShipRuntime>,
    sections: ShipSections,
    mut q_readout: Query<(&mut Text, &mut TextColor), With<ShipReadoutMarker>>,
) {
    if !runtime.active {
        return;
    }
    let Ok((mut text, mut color)) = q_readout.single_mut() else {
        return;
    };
    if let Some((note, _)) = &runtime.note {
        text.0 = note.clone();
        color.0 = NOVA_OS_AMBER;
        return;
    }
    match runtime
        .selected
        .and_then(|sel| sections.collect().into_iter().find(|v| v.entity == sel))
    {
        Some(view) => {
            let ammo = view
                .ammo
                .as_ref()
                .map(|a| format!("  ammo {}/{}", a.rounds, a.capacity))
                .unwrap_or_default();
            text.0 = format!(
                "{} {}  {} {}  {}{}",
                view.code,
                section_kind_label(view.kind).to_lowercase(),
                view.integrity_pct(),
                view.meter(),
                view.status(),
                ammo,
            );
            color.0 = if view.status() == "nominal" {
                NOVA_OS_TEXT
            } else {
                NOVA_OS_AMBER
            };
        }
        None => {
            text.0 = "Select a section: click a block or press [ / ].".to_string();
            color.0 = NOVA_OS_PHOSPHOR_MUTED;
        }
    }
}

// ---------------------------------------------------------------------------
// Small render helpers
// ---------------------------------------------------------------------------

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
            "the handler flashes the result in the readout note",
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
}
