//! What the ship app knows about a section: its stable code, its glyph and
//! description, and the [`ShipSectionView`] that renders integrity, status and
//! meters from live components.
//!
//! Codes are assigned per damage class and held stable so a typed code keeps
//! meaning the same section.
//!
//! Touch this module when adding a section fact the ship app displays.

use bevy::{ecs::system::SystemParam, prelude::*};
use nova_events::prelude::EntityId;
use nova_os::prelude::*;

use crate::{
    hud::nova_os::{
        section_kind_from_markers, section_kind_label, NOVA_OS_AMBER, NOVA_OS_PHOSPHOR,
        NOVA_OS_PHOSPHOR_DIM, NOVA_OS_PHOSPHOR_MUTED,
    },
    prelude::*,
};

/// A short, stable, human-typeable handle for a ship section (`HULL-3`, `PDC-1`),
/// the CLI/label identity the viewer and the `ship <verb> <id>` commands use.
/// Assigned per session by `assign_section_codes` from the section kind + a
/// stable index; the underlying grid `EntityId` stays the section's real identity.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct SectionCode(pub String);

/// The code prefix for a section kind (`HULL`, `THR`, `CTL`, `PDC` for turrets,
/// `TRB` for torpedo bays).
pub(crate) fn code_prefix(kind: SectionDamageClass) -> &'static str {
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
pub(crate) fn kind_glyph(kind: SectionDamageClass) -> &'static str {
    match kind {
        SectionDamageClass::Hull => "#",
        SectionDamageClass::Thruster => ">",
        SectionDamageClass::Controller => "@",
        SectionDamageClass::Turret => "T",
        SectionDamageClass::Torpedo => "^",
    }
}

/// A one-line "what it does" for a section kind, shown in the inspector panel.
pub(crate) fn kind_description(kind: SectionDamageClass) -> &'static str {
    match kind {
        SectionDamageClass::Hull => "Structural armour plating.",
        SectionDamageClass::Thruster => "Main drive; provides thrust.",
        SectionDamageClass::Controller => "Command core; runs the ship.",
        SectionDamageClass::Turret => "Point-defence gun.",
        SectionDamageClass::Torpedo => "Torpedo launch tube.",
    }
}

/// A dense index for a section kind, for the per-kind next-index counters.
pub(crate) fn kind_index(kind: SectionDamageClass) -> usize {
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
#[expect(
    clippy::type_complexity,
    reason = "one query term per section kind the code assignment reads"
)]
pub(crate) fn assign_section_codes(
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
pub(crate) struct ShipSectionView {
    pub(crate) entity: Entity,
    pub(crate) code: String,
    pub(crate) kind: SectionDamageClass,
    pub(crate) name: String,
    pub(crate) local: Transform,
    pub(crate) half_extents: Vec3,
    pub(crate) health: Option<Health>,
    pub(crate) ammo: Option<SectionAmmo>,
    pub(crate) inactive: bool,
    pub(crate) zero_health: bool,
}

impl ShipSectionView {
    /// The integrity fraction in `0..=1`, or `None` when the section has no health
    /// component / zero max.
    pub(crate) fn integrity(&self) -> Option<f32> {
        self.health
            .as_ref()
            .filter(|h| h.max > 0.0)
            .map(|h| (h.current.max(0.0) / h.max).clamp(0.0, 1.0))
    }

    /// A one-word status: neutralized / critical / degraded / nominal.
    pub(crate) fn status(&self) -> &'static str {
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
    pub(crate) fn status_color(&self) -> Color {
        match self.status() {
            "neutralized" => NOVA_OS_PHOSPHOR_DIM.with_alpha(0.5),
            "critical" => NOVA_OS_AMBER,
            "degraded" => NOVA_OS_PHOSPHOR_MUTED,
            _ => NOVA_OS_PHOSPHOR,
        }
    }

    /// The `ship view`-style integrity text (`41/100 HP` / `HP unknown`).
    pub(crate) fn health_text(&self) -> String {
        match self.health.as_ref() {
            Some(h) if h.max > 0.0 => format!("{:.0}/{:.0} HP", h.current.max(0.0), h.max),
            Some(h) => format!("{:.0} HP", h.current.max(0.0)),
            None => "HP unknown".to_string(),
        }
    }

    /// The integrity percentage label (`41%`), or `--` when unknown.
    pub(crate) fn integrity_pct(&self) -> String {
        self.integrity()
            .map(|f| format!("{:.0}%", f * 100.0))
            .unwrap_or_else(|| "--".to_string())
    }

    /// A short ASCII integrity meter, `[####------]`.
    pub(crate) fn meter(&self) -> String {
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
    pub(crate) player: Query<
        'w,
        's,
        (Entity, Option<&'static Name>),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    pub(crate) sections: Query<
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
pub(crate) type SectionKindQuery = (
    Option<&'static SectionDamageClass>,
    Has<HullSectionMarker>,
    Has<ControllerSectionMarker>,
    Has<ThrusterSectionMarker>,
    Has<TurretSectionMarker>,
    Has<TorpedoSectionMarker>,
);

impl ShipSections<'_, '_> {
    pub(crate) fn ship(&self) -> Option<(Entity, Option<String>)> {
        self.player
            .single()
            .ok()
            .map(|(e, name)| (e, name.map(|n| n.as_str().to_string())))
    }

    /// Collect the live sections, sorted by code for a stable order.
    pub(crate) fn collect(&self) -> Vec<ShipSectionView> {
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
    pub(crate) fn resolve(&self, code: &str) -> Option<ShipSectionView> {
        let wanted = code.to_ascii_uppercase();
        self.collect()
            .into_iter()
            .find(|view| view.code.eq_ignore_ascii_case(&wanted))
    }

    /// Every section code, for Tab completion of the `ship <verb> <id>` argument.
    pub(crate) fn codes(&self) -> Vec<String> {
        self.collect().into_iter().map(|view| view.code).collect()
    }
}

// ---------------------------------------------------------------------------
// CLI verb rows
// ---------------------------------------------------------------------------
/// The `ship section <id>` detail rows for one section.
pub(crate) fn section_detail_rows(view: &ShipSectionView) -> Vec<TerminalRow> {
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

pub(crate) fn status_row_kind(status: &str) -> TerminalRowKind {
    match status {
        "neutralized" => TerminalRowKind::Error,
        "critical" => TerminalRowKind::Warn,
        _ => TerminalRowKind::Output,
    }
}

/// The multi-line body of the inspector panel for one section: kind + what it
/// does, integrity % + meter, HP text, status, and ammo for weapons.
pub(crate) fn panel_detail_text(view: &ShipSectionView) -> String {
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
pub(crate) struct PanelActions {
    pub(crate) repair_enabled: bool,
    pub(crate) reload_enabled: bool,
    pub(crate) reason: Option<String>,
}

impl PanelActions {
    /// The no-selection state: nothing actionable, no reason.
    pub(crate) fn none() -> Self {
        Self {
            repair_enabled: false,
            reload_enabled: false,
            reason: None,
        }
    }
}

pub(crate) fn panel_action_state(view: &ShipSectionView) -> PanelActions {
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
pub(crate) fn unknown_code_rows(code: &str, codes: &[String]) -> Vec<TerminalRow> {
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
pub(crate) fn apply_action_to_section(
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
