//! What the Ship pane knows about a section: its stable code, its
//! description, and the [`ShipSectionView`] that renders integrity and status
//! from live components.
//!
//! Codes are assigned per damage class and held stable so a typed code keeps
//! meaning the same section.
//!
//! Touch this module when adding a section fact the Ship pane displays.

use bevy::{ecs::system::SystemParam, prelude::*};
use nova_command::prelude::*;
use nova_events::prelude::EntityId;
use nova_gameplay::prelude::*;
use nova_input::prelude::InputSource;
use nova_ship::prelude::*;
use nova_ui::theme::UiColor;

use crate::{icons::SectionIconType, terminal::section_kind_from_markers};

/// A short, stable, human-readable handle for a ship section (`HULL-3`, `PDC-1`),
/// the label identity the Ship pane uses.
/// Assigned per session by `assign_section_codes` from the section kind + a
/// stable index; the underlying grid `EntityId` stays the section's real identity.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct SectionCode(pub String);

/// The code prefix for a section kind (`HULL`, `THR`, `CTL`, `PDC` for turrets,
/// `TRB` for torpedo bays, `RAIL` for railguns, `DOCK` for docking ports, `CGO`
/// for cargo intakes, `MNG` for mining emitters).
pub(crate) fn code_prefix(kind: SectionClass) -> &'static str {
    match kind {
        SectionClass::Hull => "HULL",
        SectionClass::Thruster => "THR",
        SectionClass::Controller => "CTL",
        SectionClass::Turret => "PDC",
        SectionClass::Torpedo => "TRB",
        SectionClass::Railgun => "RAIL",
        SectionClass::Docking => "DOCK",
        SectionClass::CargoIntake => "CGO",
        SectionClass::Mining => "MNG",
    }
}

/// A one-line "what it does" for a section kind, shown in the section panel.
pub(crate) fn kind_description(kind: SectionClass) -> &'static str {
    match kind {
        SectionClass::Hull => "Structural armour plating.",
        SectionClass::Thruster => "Main drive; provides thrust.",
        SectionClass::Controller => "Command core; runs the ship.",
        SectionClass::Turret => "Point-defence gun.",
        SectionClass::Torpedo => "Torpedo launch tube.",
        SectionClass::Railgun => "Spinal rail lance; the hull aims it.",
        SectionClass::Docking => "Docking port; locks onto another hull.",
        SectionClass::CargoIntake => "Cargo intake; receives drifting canisters.",
        SectionClass::Mining => "Mining emitter; cuts ore from a locked rock.",
    }
}

/// How many dense kind indices [`kind_index`] hands out. Moves with it: the
/// per-kind counter array is sized from this, so a new section kind is one
/// arm and one number, never a silently short array.
pub(crate) const KIND_COUNT: usize = 9;

/// A dense index for a section kind, for the per-kind next-index counters.
pub(crate) fn kind_index(kind: SectionClass) -> usize {
    match kind {
        SectionClass::Hull => 0,
        SectionClass::Thruster => 1,
        SectionClass::Controller => 2,
        SectionClass::Turret => 3,
        SectionClass::Torpedo => 4,
        SectionClass::Railgun => 5,
        SectionClass::Docking => 6,
        SectionClass::CargoIntake => 7,
        SectionClass::Mining => 8,
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
            Option<&SectionClass>,
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
    let mut next: [u32; KIND_COUNT] = [0; KIND_COUNT];

    let mut unassigned: Vec<(Entity, SectionClass, String)> = Vec::new();
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

/// A live player-ship section resolved for the Ship pane: its code, kind, name,
/// placement (its LOCAL transform relative to the ship root - the schematic scene
/// is anchored at the origin, so blocks AND their projected blips both live in
/// this local/scene space, independent of where the ship is flying in world
/// space), authored half-extents, integrity and ammo.
#[derive(Clone)]
pub struct ShipSectionView {
    /// The section's entity.
    pub entity: Entity,
    /// The section's [`SectionCode`] (`HULL-3`).
    pub code: String,
    /// What the section is.
    pub kind: SectionClass,
    /// The section's display name, or its code when it has none.
    pub name: String,
    /// The section's transform in its ship's local space.
    pub local: Transform,
    /// Half the authored collider size, in engine units.
    pub half_extents: Vec3,
    pub(crate) link_points: Vec<LinkPoint>,
    pub(crate) health: Option<Health>,
    pub(crate) ammo: Option<SectionAmmo>,
    pub(crate) bindings: Option<Vec<InputSource>>,
    pub(crate) inactive: bool,
    pub(crate) zero_health: bool,
    pub(crate) disabled: bool,
}

impl ShipSectionView {
    pub(crate) fn binding_text(&self) -> Option<String> {
        self.bindings.as_ref().map(|bindings| {
            if bindings.is_empty() {
                return "UNBOUND".to_string();
            }
            // Every source has a label - the fallback that printed a `Binding`
            // debug string went with the type that could fail to name one.
            bindings
                .iter()
                .map(InputSource::label)
                .collect::<Vec<_>>()
                .join(" / ")
        })
    }

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

    /// The theme colour of this status on the badge pip and condition bar.
    pub(crate) fn status_color(&self) -> UiColor {
        match self.status() {
            "nominal" => UiColor::Nominal,
            "degraded" => UiColor::Accent,
            "critical" => UiColor::Danger,
            _ => UiColor::Label,
        }
    }

    /// The integrity text (`41/100 HP` / `HP unknown`).
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
}

/// System-param that enumerates the live player-ship sections into
/// [`ShipSectionView`]s. Shared by the Ship pane's scene builder, panel and
/// interaction systems.
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
            Option<&'static SectionLinkPoints>,
            Option<&'static Health>,
            Option<&'static SectionAmmo>,
            // Nested so the whole row stays under the 15-item query-tuple cap.
            SectionKindQuery,
            SectionBindingQuery,
            (
                Has<SectionInactiveMarker>,
                Has<HealthZeroMarker>,
                Has<IntegrityDisabledMarker>,
            ),
        ),
        With<SectionMarker>,
    >,
}

/// The class + kind-marker columns needed to classify a section, grouped so the
/// enclosing section query stays within the query-tuple size limit.
pub(crate) type SectionBindingQuery = (
    Option<&'static SpaceshipThrusterInputBinding>,
    Option<&'static SpaceshipTurretInputBinding>,
    Option<&'static SpaceshipTorpedoInputBinding>,
    Option<&'static SpaceshipRailgunInputBinding>,
    Option<&'static SpaceshipMiningInputBinding>,
);

pub(crate) type SectionKindQuery = (
    Option<&'static SectionClass>,
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
    pub fn collect(&self) -> Vec<ShipSectionView> {
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
                    link_points,
                    health,
                    ammo,
                    (class, hull, controller, thruster, turret, torpedo),
                    (
                        thruster_bindings,
                        turret_bindings,
                        torpedo_bindings,
                        railgun_bindings,
                        mining_bindings,
                    ),
                    (inactive, zero_health, disabled),
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
                        link_points: link_points
                            .map(|points| points.0.clone())
                            .unwrap_or_default(),
                        health: health.cloned(),
                        ammo: ammo.copied(),
                        bindings: thruster_bindings
                            .map(|bindings| bindings.0.clone())
                            .or_else(|| turret_bindings.map(|bindings| bindings.0.clone()))
                            .or_else(|| torpedo_bindings.map(|bindings| bindings.0.clone()))
                            .or_else(|| railgun_bindings.map(|bindings| bindings.0.clone()))
                            .or_else(|| mining_bindings.map(|bindings| bindings.0.clone())),
                        inactive,
                        zero_health,
                        disabled,
                    })
                },
            )
            .collect();
        views.sort_by(|a, b| a.code.cmp(&b.code));
        views
    }

    /// Resolve a code (case-insensitive) to its section view, for the tests.
    #[cfg(test)]
    pub(crate) fn resolve(&self, code: &str) -> Option<ShipSectionView> {
        let wanted = code.to_ascii_uppercase();
        self.collect()
            .into_iter()
            .find(|view| view.code.eq_ignore_ascii_case(&wanted))
    }
}

/// The line under the panel title: the section family, its integrity and its
/// status, as `Weapon  Condition 20%  critical`.
pub(crate) fn panel_status_text(view: &ShipSectionView) -> String {
    format!(
        "{}  Condition {}  {}",
        SectionIconType::of(view.kind).label(),
        view.integrity_pct(),
        view.status(),
    )
}

/// Labelled facts and a short explanation of the selected section's role.
pub(crate) fn panel_detail_text(view: &ShipSectionView) -> String {
    let mut text = format!(
        "Integrity: {}\nRole: {}",
        view.health_text(),
        kind_description(view.kind)
    );
    if let Some(ammo) = view.ammo.as_ref() {
        text.push_str(&format!(
            "\nAmmunition: {} / {} rounds",
            ammo.rounds, ammo.capacity
        ));
    }
    if let Some(bindings) = view.binding_text() {
        text.push_str(&format!("\nControl: {bindings}"));
    }
    text
}

/// Whether Repair is valid for a section, plus a reason for a disabled
/// action. Uses the same [`plan_plate_repair`] check as [`repair_section`], so
/// the panel button never disagrees with the handler.
pub(crate) struct PanelActions {
    pub(crate) repair_enabled: bool,
    pub(crate) reason: Option<String>,
}

impl PanelActions {
    /// The no-selection state: nothing actionable, no reason.
    pub(crate) fn none() -> Self {
        Self {
            repair_enabled: false,
            reason: None,
        }
    }
}

/// The maximum whole-plate request supported by current integrity and stock.
pub(crate) fn plate_repair_limit(health: Option<&Health>, disabled: bool, stock: u32) -> u32 {
    let Some(health) = health.filter(|health| health.max > 0.0 && health.current > 0.0) else {
        return 0;
    };
    if disabled || health.current >= health.max {
        return 0;
    }
    stock.min(((health.max - health.current) / HULL_PLATE_HEALTH).ceil() as u32)
}

/// The panel state for a selected whole-plate request against live stock.
pub(crate) fn panel_action_state(
    view: &ShipSectionView,
    requested_plates: u32,
    stock: u32,
) -> PanelActions {
    let repair = plan_plate_repair(view.health.as_ref(), view.disabled, requested_plates, stock);
    PanelActions {
        repair_enabled: repair.is_ok(),
        reason: repair
            .err()
            .map(|refusal| repair_refusal_text(&view.code, refusal)),
    }
}

/// A request to repair a player-ship section from hull plates, raised by the
/// pane's repair key and panel button.
#[derive(Message, Clone, Copy, Debug)]
pub struct SectionRepairCommand {
    /// The target section entity.
    pub target: Entity,
    /// Exact whole plates selected when the command was raised.
    pub requested_plates: u32,
}

/// The note line for a refused repair; the panel and the handler share it.
fn repair_refusal_text(code: &str, refusal: PlateRepairRefusalType) -> String {
    match refusal {
        PlateRepairRefusalType::NoIntegrity => {
            format!("repair: {code} has no integrity to restore")
        }
        PlateRepairRefusalType::Destroyed => format!("repair: {code} is destroyed"),
        PlateRepairRefusalType::Full => format!("repair: {code} is at full integrity"),
        PlateRepairRefusalType::NoPlates => {
            "repair: no hull plates selected or in stock".to_string()
        }
        PlateRepairRefusalType::InsufficientPlates => {
            format!("repair: {code} request exceeds live hull plate stock")
        }
        PlateRepairRefusalType::ExcessPlates => {
            format!("repair: {code} request exceeds live missing integrity")
        }
    }
}

/// Repair a section from the player ship's hull plates by the
/// [`plan_plate_repair`] rule: spend the plates and set Health together, or
/// change nothing. `disabled` is true when the section carries
/// `IntegrityDisabledMarker`.
pub(crate) fn repair_section(
    code: &str,
    health: Option<&mut Health>,
    disabled: bool,
    requested_plates: u32,
    inventory: &mut ShipInventory,
) -> TerminalRow {
    let refused = |refusal| TerminalRow {
        kind: TerminalRowKind::Error,
        text: repair_refusal_text(code, refusal),
    };
    let Some(health) = health else {
        return refused(PlateRepairRefusalType::NoIntegrity);
    };
    match plan_plate_repair(
        Some(health),
        disabled,
        requested_plates,
        inventory.count(ItemType::HullPlate),
    ) {
        Ok(repair) => {
            inventory.remove(ItemType::HullPlate, repair.plates);
            health.current = repair.current;
            let noun = if repair.plates == 1 {
                "plate"
            } else {
                "plates"
            };
            TerminalRow {
                kind: TerminalRowKind::Info,
                text: format!(
                    "repaired {code}: {} hull {noun}, {:.0}/{:.0} HP",
                    repair.plates, health.current, health.max
                ),
            }
        }
        Err(refusal) => refused(refusal),
    }
}
