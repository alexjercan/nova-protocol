//! Balance audit over shipped scenario content (task 20260717-112656,
//! spike tasks/20260717-111808/SPIKE.md).
//!
//! Balance regressions do not fail loaders or lints: a scenario that
//! spawns a top-tier gunner on top of the player parses, loads and plays -
//! it is just unfair. This module derives the spike's fairness metrics
//! from the SHIPPED data (authored-vs-derived-values): per spawn group,
//! the hostile head-count, combined BURST dps (first-magazine rate;
//! reload cycles make true sustained lower, but every shipped TTK lands
//! inside the first magazine, so burst is the honest danger number),
//! weapon threat envelopes, distance from the player spawn, and
//! time-to-kill against the player ship's summed section health; per
//! scenario, the cover tiers (invulnerable anchors vs destructible chaff
//! vs scattered fields, each tier following its template's hardness).
//!
//! Two findings are graded, both static approximations chosen to be
//! trustworthy rather than clever:
//!
//! - ERROR `spawned-dead`: an armed hostile placed by OnStart INSIDE its
//!   own effective weapon range of the player spawn - the player is under
//!   accurate fire before they can move (the pre-rework ledger_ch2 shape).
//! - WARN `close-spawn`: the same inside-its-own-envelope predicate on a
//!   TRIGGERED handler. Mid-fight player position is unknowable statically,
//!   so the spawn point is the proxy; shipped arenas fight near their
//!   spawns. Scaling the threshold by the hostile's OWN weapon envelope
//!   keeps the rule honest at both ends: a light-turret mook 395u out
//!   (135u outside its 270u reach) is an approach, not an ambush, while a
//!   better-turret capital 301u out (inside its 450u reach) is on top of
//!   the fight the frame it exists - the pre-rework "wave 2 on top of you"
//!   shape.
//!
//! The per-scenario invariant PINS for the reworked encounters live in
//! their own tests (ledger_ch2_encounter.rs, broadside_assault.rs); this
//! module is the repo-wide generalization that also covers content nobody
//! hand-pinned (asteroid_field, the ledger's later chapters, future mods).
//! `balance_audit_gate` runs it in CI; the `content` CLI's `lint` runs it in
//! one pass with the reference checks (the balance audit was folded into
//! `lint`, task 20260718-152240).

use std::collections::HashMap;

use bevy::math::Vec3;
use nova_gameplay::prelude::{Allegiance, SectionConfig, SectionKind};
use nova_scenario::prelude::*;

/// Mirrors the AI's own shot-worth-taking margin (AI_FIRE_RANGE_FACTOR in
/// nova_gameplay/src/input/ai.rs): effective range = margin x muzzle_speed
/// x projectile_lifetime.
pub const EFFECTIVE_RANGE_MARGIN: f32 = 0.9;

/// Mirrors AI_TORPEDO_MAX_RANGE (nova_gameplay/src/input/ai.rs): the outer
/// edge of the AI launch envelope, whose per-bay cooldown starts ELAPSED -
/// a tube inside this range is a live opening threat (review R1.1).
pub const TORPEDO_ENVELOPE: f32 = 1000.0;

/// The section-prototype view a scenario's ships resolve against: the
/// last-wins overlay of base -> declared dependencies (in declared order)
/// -> the bundle's own sections. This matches the runtime merge for every
/// SHIPPED bundle today (each declares only `base`); the runtime
/// additionally topo-sorts TRANSITIVE dependency graphs and resolves
/// intra-bundle duplicate ids first-wins, which this static join does not
/// model - revisit if a shipped bundle grows non-base deps. The point
/// stands either way: a dependency can silently rebalance a base section
/// by id (mod-dependency-overrides-are-load-bearing), so the audit joins
/// through the overlay, never through base alone.
pub struct SectionCatalog(HashMap<String, SectionConfig>);

impl SectionCatalog {
    pub fn resolve(layers: &[&[SectionConfig]]) -> Self {
        let mut map = HashMap::new();
        for layer in layers {
            for section in *layer {
                map.insert(section.base.id.clone(), section.clone());
            }
        }
        Self(map)
    }

    pub fn get(&self, id: &str) -> Option<&SectionConfig> {
        self.0.get(id)
    }
}

/// A ship's derived combat numbers, summed over its resolved sections.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShipStats {
    /// Summed section health (per-section HP is the shipped damage model;
    /// the sum is the ship's total pool as the HUD aggregates it).
    pub hp: f32,
    /// BURST turret dps: sum of fire_rate x bullet_damage - the
    /// first-magazine rate (reloads make true sustained ~62% of this for
    /// the catalog turrets, but shipped TTKs land inside one magazine).
    /// Kinetic resistance is 1.0 everywhere in the shipped table, so
    /// authored damage IS applied damage for every catalog turret.
    pub dps: f32,
    /// The longest effective range among the ship's turrets
    /// ([`EFFECTIVE_RANGE_MARGIN`] x muzzle_speed x projectile_lifetime).
    pub max_effective_range: f32,
    /// Torpedo tubes are counted, not folded into dps: a tube's threat is
    /// blast area + guidance, not sustained fire.
    pub torpedo_tubes: usize,
}

impl ShipStats {
    /// How far this ship threatens the moment it exists: its longest turret
    /// reach, or the AI torpedo launch envelope if it carries tubes (the
    /// bay's first-launch cooldown starts elapsed). Zero = unarmed.
    pub fn threat_envelope(&self) -> f32 {
        let tube_reach = if self.torpedo_tubes > 0 {
            TORPEDO_ENVELOPE
        } else {
            0.0
        };
        self.max_effective_range.max(tube_reach)
    }
}

/// Sum the fire rate of every muzzle in a turret's joint tree. Fire rate is
/// per-muzzle since the joint-tree refactor (spike 20260717-214834); the shipped
/// turrets each carry one muzzle, so for the catalog this is that one rate.
fn turret_total_fire_rate(joint: &nova_gameplay::prelude::TurretJoint) -> f32 {
    let here = joint.muzzle.as_ref().map(|m| m.fire_rate).unwrap_or(0.0);
    here + joint
        .children
        .iter()
        .map(turret_total_fire_rate)
        .sum::<f32>()
}

/// Sum a ship's stats through the catalog. Unknown prototypes contribute
/// nothing (content_lint already errors on them; the audit stays total).
pub fn ship_stats(ship: &SpaceshipConfig, catalog: &SectionCatalog) -> ShipStats {
    let mut stats = ShipStats {
        hp: 0.0,
        dps: 0.0,
        max_effective_range: 0.0,
        torpedo_tubes: 0,
    };
    for section in &ship.sections {
        let resolved: Option<&SectionConfig> = match &section.source {
            SectionSource::Prototype(id) => catalog.get(id),
            SectionSource::Inline(config) => Some(config),
        };
        let Some(config) = resolved else { continue };
        // An authored SetHealth override wins over the prototype (last one
        // wins, like the runtime observers applying the list in order).
        let hp_override = section.modifications.iter().rev().find_map(|m| match m {
            SectionModification::SetHealth(hp) => Some(*hp),
            _ => None,
        });
        stats.hp += hp_override.unwrap_or(config.base.health);
        match &config.kind {
            SectionKind::Turret(turret) => {
                // Fire rate is per-muzzle now (spike 20260717-214834); burst DPS
                // sums every muzzle in the joint tree. The shipped turrets each
                // carry one muzzle, so this is unchanged for the catalog.
                stats.dps += turret_total_fire_rate(&turret.root) * turret.bullet_damage;
                stats.max_effective_range = stats
                    .max_effective_range
                    .max(EFFECTIVE_RANGE_MARGIN * turret.muzzle_speed * turret.projectile_lifetime);
            }
            SectionKind::Torpedo(_) => stats.torpedo_tubes += 1,
            _ => {}
        }
    }
    stats
}

/// One armed (or unarmed) hostile placed by a handler.
#[derive(Debug, Clone)]
pub struct HostileAudit {
    pub id: String,
    /// Distance from the player spawn to this hostile's spawn.
    pub distance: f32,
    pub stats: ShipStats,
}

/// The hostiles one handler places, labeled by its trigger.
#[derive(Debug, Clone)]
pub struct SpawnGroupAudit {
    /// A short trigger label ("OnStart", "OnEnter(area)", "OnUpdate", ...).
    pub trigger: String,
    pub on_start: bool,
    pub hostiles: Vec<HostileAudit>,
}

impl SpawnGroupAudit {
    pub fn combined_dps(&self) -> f32 {
        self.hostiles.iter().map(|h| h.stats.dps).sum()
    }
}

/// The scenario's cover inventory, by tier.
#[derive(Debug, Clone, Copy, Default)]
pub struct CoverAudit {
    /// Fixed invulnerable asteroids: the hard anchors the line-of-fire
    /// gate makes meaningful.
    pub invulnerable: usize,
    /// Fixed destructible asteroids: chaff.
    pub destructible: usize,
    /// Rocks placed by ScatterObjects fields whose template is
    /// invulnerable (the gauntlet's belt walls are exactly this).
    pub scattered_hard: usize,
    /// Rocks placed by ScatterObjects fields with destructible templates.
    pub scattered_soft: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BalanceSeverity {
    Error,
    Warn,
}

/// The graded rules, as stable identifiers the ack file names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingKind {
    SpawnedDead,
    CloseSpawn,
}

impl FindingKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            FindingKind::SpawnedDead => "spawned-dead",
            FindingKind::CloseSpawn => "close-spawn",
        }
    }
}

#[derive(Debug, Clone)]
pub struct BalanceFinding {
    pub severity: BalanceSeverity,
    pub kind: FindingKind,
    pub scenario: String,
    /// The offending hostile's scenario object id (what an ack names).
    pub hostile: String,
    pub message: String,
}

/// One ACKNOWLEDGED finding (crates/nova_assets/balance_acks.ron): a
/// WARN-grade finding a human decided is intended, with the reason and the
/// deciding task on record. ERRORs are never ackable - an ack pointing at
/// an error-grade finding simply does not match and surfaces as stale.
/// Stale acks (matching no live finding) surface as findings themselves,
/// so a rebalanced scenario cannot leave a dead exception rotting quietly.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct BalanceAck {
    pub bundle: String,
    pub scenario: String,
    pub hostile: String,
    /// A [`FindingKind::as_str`] value ("spawned-dead" / "close-spawn").
    pub kind: String,
    pub reason: String,
    pub task: String,
}

/// The repo's shipped acknowledgment list.
pub fn shipped_acks() -> Vec<BalanceAck> {
    ron::de::from_str(include_str!("../balance_acks.ron")).expect("balance_acks.ron parses")
}

/// Split `(bundle, finding)` pairs into (active, acked) under `acks`, and
/// return the stale acks (matched nothing). Only WARN-grade findings can
/// match an ack; every stale ack must be surfaced by the caller.
#[allow(clippy::type_complexity)]
pub fn partition_findings(
    findings: Vec<(String, BalanceFinding)>,
    acks: &[BalanceAck],
) -> (
    Vec<(String, BalanceFinding)>,
    Vec<(String, BalanceFinding, &BalanceAck)>,
    Vec<&BalanceAck>,
) {
    let matches = |ack: &BalanceAck, bundle: &str, finding: &BalanceFinding| {
        finding.severity == BalanceSeverity::Warn
            && ack.bundle == bundle
            && ack.scenario == finding.scenario
            && ack.hostile == finding.hostile
            && ack.kind == finding.kind.as_str()
    };
    let mut active = Vec::new();
    let mut acked = Vec::new();
    let mut used = vec![false; acks.len()];
    for (bundle, finding) in findings {
        // Each ack spends on ONE finding: duplicate findings (the same boss
        // spawned by two mutually exclusive branches) need one ack each.
        match (0..acks.len()).find(|&i| !used[i] && matches(&acks[i], &bundle, &finding)) {
            Some(i) => {
                used[i] = true;
                acked.push((bundle, finding, &acks[i]));
            }
            None => active.push((bundle, finding)),
        }
    }
    let stale = acks
        .iter()
        .zip(&used)
        .filter_map(|(ack, used)| (!used).then_some(ack))
        .collect();
    (active, acked, stale)
}

/// One combat scenario's derived balance sheet.
#[derive(Debug, Clone)]
pub struct ScenarioAudit {
    pub scenario: String,
    pub player: ShipStats,
    pub groups: Vec<SpawnGroupAudit>,
    pub cover: CoverAudit,
}

impl ScenarioAudit {
    /// Sustained seconds the player survives a group's combined aligned
    /// fire - the spike's TTK metric. Infinite (None) for unarmed groups.
    pub fn ttk_against(&self, group: &SpawnGroupAudit) -> Option<f32> {
        let dps = group.combined_dps();
        (dps > 0.0).then(|| self.player.hp / dps)
    }

    pub fn findings(&self) -> Vec<BalanceFinding> {
        let mut findings = Vec::new();
        for group in &self.groups {
            for hostile in &group.hostiles {
                let envelope = hostile.stats.threat_envelope();
                if envelope <= 0.0 {
                    // Unarmed: no turrets, no tubes.
                    continue;
                }
                if hostile.distance >= envelope {
                    continue;
                }
                if group.on_start {
                    findings.push(BalanceFinding {
                        severity: BalanceSeverity::Error,
                        kind: FindingKind::SpawnedDead,
                        scenario: self.scenario.clone(),
                        hostile: hostile.id.clone(),
                        message: format!(
                            "spawned-dead: '{}' opens the scenario {:.0}u from the player \
                             spawn, inside its own {:.0}u threat envelope - the player is \
                             under fire before their first input",
                            hostile.id, hostile.distance, envelope
                        ),
                    });
                } else {
                    findings.push(BalanceFinding {
                        severity: BalanceSeverity::Warn,
                        kind: FindingKind::CloseSpawn,
                        scenario: self.scenario.clone(),
                        hostile: hostile.id.clone(),
                        message: format!(
                            "close-spawn: '{}' ({}) spawns {:.0}u from the player spawn, \
                             inside its own {:.0}u threat envelope - a mid-fight \
                             reinforcement arriving on top of the fight",
                            hostile.id, group.trigger, hostile.distance, envelope
                        ),
                    });
                }
            }
        }
        findings
    }

    /// The one-scenario slice of the report table.
    pub fn report(&self) -> String {
        let mut out = format!(
            "{}: player {:.0}hp {:.0}dps | cover {} hard / {} soft / scattered {} hard {} soft\n",
            self.scenario,
            self.player.hp,
            self.player.dps,
            self.cover.invulnerable,
            self.cover.destructible,
            self.cover.scattered_hard,
            self.cover.scattered_soft,
        );
        for group in &self.groups {
            let closest = group
                .hostiles
                .iter()
                .map(|h| h.distance)
                .fold(f32::INFINITY, f32::min);
            let tubes: usize = group.hostiles.iter().map(|h| h.stats.torpedo_tubes).sum();
            let ttk = match self.ttk_against(group) {
                Some(ttk) => format!("{ttk:.1}s"),
                None => "-".to_string(),
            };
            out.push_str(&format!(
                "  {}: {} hostile(s), {:.0} dps, {} tube(s), closest {:.0}u, TTK vs player {}\n",
                group.trigger,
                group.hostiles.len(),
                group.combined_dps(),
                tubes,
                closest,
                ttk,
            ));
        }
        out
    }
}

fn trigger_label(event: &ScenarioEventConfig) -> String {
    let entity_id = event.filters.iter().find_map(|f| match f {
        EventFilterConfig::Entity(entity) => entity.id.clone(),
        _ => None,
    });
    match (&event.name, entity_id) {
        (EventConfig::OnStart, _) => "OnStart".to_string(),
        (name, Some(id)) => format!("{name:?}({id})"),
        (name, None) => format!("{name:?}"),
    }
}

/// Audit one scenario against its resolved catalog. `None` when the
/// scenario spawns no player-controlled ship (menu backdrops, authoring
/// demos): there is no one to be unfair to.
pub fn audit_scenario(
    scenario: &ScenarioConfig,
    catalog: &SectionCatalog,
) -> Option<ScenarioAudit> {
    let mut player: Option<(Vec3, ShipStats)> = None;
    for event in &scenario.events {
        for action in &event.actions {
            if let EventActionConfig::SpawnScenarioObject(config) = action {
                if let ScenarioObjectKind::Spaceship(ship) = &config.kind {
                    if matches!(ship.controller, SpaceshipController::Player(_)) {
                        player = Some((config.base.position, ship_stats(ship, catalog)));
                    }
                }
            }
        }
    }
    let (player_spawn, player_stats) = player?;

    let mut groups = Vec::new();
    let mut cover = CoverAudit::default();
    for event in &scenario.events {
        let mut hostiles = Vec::new();
        for action in &event.actions {
            match action {
                EventActionConfig::SpawnScenarioObject(config) => match &config.kind {
                    ScenarioObjectKind::Spaceship(ship)
                        if matches!(ship.controller, SpaceshipController::AI(_))
                            && !matches!(
                                ship.allegiance,
                                Some(Allegiance::Neutral) | Some(Allegiance::Player)
                            ) =>
                    {
                        hostiles.push(HostileAudit {
                            id: config.base.id.clone(),
                            distance: config.base.position.distance(player_spawn),
                            stats: ship_stats(ship, catalog),
                        });
                    }
                    ScenarioObjectKind::Asteroid(rock) if rock.invulnerable => {
                        cover.invulnerable += 1;
                    }
                    ScenarioObjectKind::Asteroid(_) => cover.destructible += 1,
                    _ => {}
                },
                EventActionConfig::ScatterObjects(scatter) => {
                    let hard = matches!(
                        &scatter.template.kind,
                        ScenarioObjectKind::Asteroid(rock) if rock.invulnerable
                    );
                    if hard {
                        cover.scattered_hard += scatter.count as usize;
                    } else {
                        cover.scattered_soft += scatter.count as usize;
                    }
                }
                _ => {}
            }
        }
        if !hostiles.is_empty() {
            groups.push(SpawnGroupAudit {
                trigger: trigger_label(event),
                on_start: matches!(event.name, EventConfig::OnStart),
                hostiles,
            });
        }
    }

    Some(ScenarioAudit {
        scenario: scenario.id.clone(),
        player: player_stats,
        groups,
        cover,
    })
}

/// Audit every combat scenario in the repo tree (base + assets/mods +
/// webmods), each against ITS bundle's resolved section overlay. Returns
/// `(bundle id, audit)` pairs in walk order.
pub fn audit_content_tree() -> Vec<(String, ScenarioAudit)> {
    audit_bundles_to_audits(&crate::lint_walk::audit_bundles())
}

/// Audit an already-walked set of bundles: each bundle's scenarios against the
/// catalog resolved from base + its declared deps + its own sections (the
/// runtime merge order). The single balance code path - [`audit_content_tree`]
/// feeds it the whole repo, and the unified content report
/// (`crate::content_report`) feeds it a target's walk (the target plus the
/// repo it depends on) so a `--target` audit sees the same numbers the tree
/// audit does.
pub fn audit_bundles_to_audits(
    bundles: &[crate::lint_walk::AuditBundle],
) -> Vec<(String, ScenarioAudit)> {
    let by_id: HashMap<&str, &crate::lint_walk::AuditBundle> =
        bundles.iter().map(|b| (b.id.as_str(), b)).collect();
    let base: &[SectionConfig] = by_id
        .get("base")
        .map(|b| b.sections.as_slice())
        .unwrap_or(&[]);

    let mut audits = Vec::new();
    for bundle in bundles {
        // base is implicit; declared deps layer over it in declared order;
        // the bundle's own sections win last (the runtime merge's order).
        let mut layers: Vec<&[SectionConfig]> = vec![base];
        for dep in &bundle.dependencies {
            if dep != "base" {
                if let Some(dep_bundle) = by_id.get(dep.as_str()) {
                    layers.push(dep_bundle.sections.as_slice());
                }
            }
        }
        layers.push(bundle.sections.as_slice());
        let catalog = SectionCatalog::resolve(&layers);
        for scenario in &bundle.scenarios {
            if let Some(audit) = audit_scenario(scenario, &catalog) {
                audits.push((bundle.id.clone(), audit));
            }
        }
    }
    audits
}

#[cfg(test)]
mod tests {
    use nova_gameplay::prelude::{BaseSectionConfig, TurretSectionConfig};

    use super::*;

    fn hull(id: &str, health: f32) -> SectionConfig {
        SectionConfig {
            base: BaseSectionConfig {
                id: id.to_string(),
                health,
                ..Default::default()
            },
            kind: SectionKind::Hull(Default::default()),
        }
    }

    fn turret(id: &str, health: f32, fire_rate: f32, damage: f32, speed: f32) -> SectionConfig {
        use nova_gameplay::prelude::{MuzzleConfig, TurretJoint};

        // A minimal one-muzzle tree carrying this test's fire rate; every other
        // field falls back to the default tree's shape.
        let root = TurretJoint {
            offset: Vec3::ZERO,
            axis: None,
            speed: std::f32::consts::PI,
            min: None,
            max: None,
            render_mesh: None,
            render_mesh_transform: None,
            muzzle: Some(MuzzleConfig {
                fire_rate,
                muzzle_effect: None,
            }),
            children: vec![],
        };
        SectionConfig {
            base: BaseSectionConfig {
                id: id.to_string(),
                health,
                ..Default::default()
            },
            kind: SectionKind::Turret(TurretSectionConfig {
                root,
                bullet_damage: damage,
                muzzle_speed: speed,
                projectile_lifetime: 5.0,
                ..Default::default()
            }),
        }
    }

    fn ship(controller: SpaceshipController, prototypes: &[&str]) -> SpaceshipConfig {
        SpaceshipConfig {
            controller,
            allegiance: None,
            sections: prototypes
                .iter()
                .enumerate()
                .map(|(i, p)| SpaceshipSectionConfig {
                    id: format!("s{i}"),
                    position: Vec3::ZERO,
                    rotation: bevy::math::Quat::IDENTITY,
                    source: SectionSource::Prototype(p.to_string()),
                    modifications: vec![],
                })
                .collect(),
        }
    }

    fn spawn_at(id: &str, position: Vec3, ship: SpaceshipConfig) -> EventActionConfig {
        EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: id.to_string(),
                name: id.to_string(),
                position,
                rotation: bevy::math::Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Spaceship(ship),
        })
    }

    fn player_controller() -> SpaceshipController {
        SpaceshipController::Player(Default::default())
    }

    fn ai_controller() -> SpaceshipController {
        SpaceshipController::AI(Default::default())
    }

    fn scenario_of(events: Vec<ScenarioEventConfig>) -> ScenarioConfig {
        ScenarioConfig {
            id: "audit_test".to_string(),
            name: "audit test".to_string(),
            description: String::new(),
            cubemap: "self://sky.png".into(),
            thumbnail: None,
            hidden: true,
            menu_backdrop: false,
            campaign: None,
            events,
        }
    }

    fn on_start(actions: Vec<EventActionConfig>) -> ScenarioEventConfig {
        ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            actions,
        }
    }

    /// The overlay is last-wins across layers - a dependency rebalancing a
    /// base id by override must be what the audit sees
    /// (mod-dependency-overrides-are-load-bearing).
    #[test]
    fn catalog_overlay_is_last_wins() {
        let base = [hull("h", 200.0)];
        let dep = [hull("h", 400.0)];
        let catalog = SectionCatalog::resolve(&[&base, &dep]);
        assert_eq!(catalog.get("h").unwrap().base.health, 400.0);
    }

    /// dps / range / hp / tube sums against hand-computed numbers.
    #[test]
    fn ship_stats_sum_the_resolved_sections() {
        let catalog =
            SectionCatalog::resolve(&[&[hull("h", 100.0), turret("t", 60.0, 25.0, 4.0, 60.0)]]);
        let stats = ship_stats(&ship(ai_controller(), &["h", "t"]), &catalog);
        assert_eq!(stats.hp, 160.0);
        assert_eq!(stats.dps, 100.0);
        assert_eq!(stats.max_effective_range, 0.9 * 60.0 * 5.0);
        assert_eq!(stats.torpedo_tubes, 0);
    }

    /// The fail-first for the ERROR rule, permanently in-tree: the
    /// pre-rework ledger_ch2 shape (an armed hostile opening inside its
    /// own range) MUST grade as spawned-dead; pushing it outside its range
    /// clears the finding (the delivery guard).
    #[test]
    fn an_armed_onstart_hostile_inside_its_range_is_spawned_dead() {
        let catalog =
            SectionCatalog::resolve(&[&[hull("h", 100.0), turret("t", 60.0, 100.0, 4.0, 100.0)]]);
        let build = |hostile_z: f32| {
            scenario_of(vec![on_start(vec![
                spawn_at(
                    "player_spaceship",
                    Vec3::ZERO,
                    ship(player_controller(), &["h"]),
                ),
                spawn_at(
                    "gunner",
                    Vec3::new(0.0, 0.0, hostile_z),
                    ship(ai_controller(), &["h", "t"]),
                ),
            ])])
        };

        // 175u inside a 450u effective range: the pre-rework shape.
        let audit = audit_scenario(&build(-175.0), &catalog).expect("player present");
        let findings = audit.findings();
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].severity, BalanceSeverity::Error);
        assert!(findings[0].message.contains("spawned-dead"));

        // The same hostile at 600u grades clean.
        let audit = audit_scenario(&build(-600.0), &catalog).expect("player present");
        assert!(audit.findings().is_empty(), "{:?}", audit.findings());
    }

    /// Review R1.1's fail-first, permanently in-tree: a TUBE-ONLY hostile
    /// has zero turret dps but the AI launch envelope opens immediately
    /// (the bay cooldown starts elapsed) - it must not evade the rules.
    #[test]
    fn a_tube_only_onstart_ambusher_is_spawned_dead() {
        use nova_gameplay::prelude::TorpedoSectionConfig;
        let tube = SectionConfig {
            base: BaseSectionConfig {
                id: "tube".to_string(),
                health: 100.0,
                ..Default::default()
            },
            kind: SectionKind::Torpedo(TorpedoSectionConfig::default()),
        };
        let catalog = SectionCatalog::resolve(&[&[hull("h", 100.0), tube]]);
        let scenario = scenario_of(vec![on_start(vec![
            spawn_at(
                "player_spaceship",
                Vec3::ZERO,
                ship(player_controller(), &["h"]),
            ),
            spawn_at(
                "bomber",
                Vec3::new(0.0, 0.0, -300.0),
                ship(ai_controller(), &["h", "tube"]),
            ),
        ])]);
        let audit = audit_scenario(&scenario, &catalog).expect("player present");
        let findings = audit.findings();
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].severity, BalanceSeverity::Error);
        assert!(findings[0].message.contains("bomber"));
    }

    /// A triggered close spawn is a WARN, not an ERROR (static proxy), and
    /// an unarmed close spawn is no finding at all.
    #[test]
    fn triggered_close_spawns_warn_and_unarmed_ships_pass() {
        let catalog =
            SectionCatalog::resolve(&[&[hull("h", 100.0), turret("t", 60.0, 100.0, 4.0, 100.0)]]);
        let triggered = ScenarioEventConfig {
            name: EventConfig::OnUpdate,
            filters: vec![],
            actions: vec![
                spawn_at(
                    "reinforcement",
                    Vec3::new(0.0, 0.0, -130.0),
                    ship(ai_controller(), &["h", "t"]),
                ),
                spawn_at(
                    "unarmed_drone",
                    Vec3::new(0.0, 0.0, -50.0),
                    ship(ai_controller(), &["h"]),
                ),
            ],
        };
        let scenario = scenario_of(vec![
            on_start(vec![spawn_at(
                "player_spaceship",
                Vec3::ZERO,
                ship(player_controller(), &["h"]),
            )]),
            triggered,
        ]);
        let audit = audit_scenario(&scenario, &catalog).expect("player present");
        let findings = audit.findings();
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].severity, BalanceSeverity::Warn);
        assert!(findings[0].message.contains("close-spawn"));
        assert!(findings[0].message.contains("reinforcement"));
    }

    /// The close-spawn threshold scales with the hostile's OWN envelope: a
    /// short-range mook beyond its reach is an approach, not an ambush.
    #[test]
    fn a_triggered_mook_outside_its_own_reach_is_clean() {
        let catalog = SectionCatalog::resolve(&[&[
            hull("h", 100.0),
            // Light-turret numbers: 270u effective reach.
            turret("t", 60.0, 25.0, 3.825, 60.0),
        ]]);
        let scenario = scenario_of(vec![
            on_start(vec![spawn_at(
                "player_spaceship",
                Vec3::ZERO,
                ship(player_controller(), &["h"]),
            )]),
            ScenarioEventConfig {
                name: EventConfig::OnUpdate,
                filters: vec![],
                actions: vec![spawn_at(
                    "approacher",
                    Vec3::new(0.0, 0.0, -395.0),
                    ship(ai_controller(), &["h", "t"]),
                )],
            },
        ]);
        let audit = audit_scenario(&scenario, &catalog).expect("player present");
        assert!(
            audit.findings().is_empty(),
            "395u vs a 270u reach must grade clean: {:?}",
            audit.findings()
        );
    }

    fn warn_finding(hostile: &str) -> BalanceFinding {
        BalanceFinding {
            severity: BalanceSeverity::Warn,
            kind: FindingKind::CloseSpawn,
            scenario: "s".to_string(),
            hostile: hostile.to_string(),
            message: "close-spawn: test".to_string(),
        }
    }

    fn ack_for(hostile: &str, kind: &str) -> BalanceAck {
        BalanceAck {
            bundle: "b".to_string(),
            scenario: "s".to_string(),
            hostile: hostile.to_string(),
            kind: kind.to_string(),
            reason: "test".to_string(),
            task: "t".to_string(),
        }
    }

    /// An ack silences exactly its matching WARN; duplicate findings need
    /// duplicate acks (one ack, two identical findings: one stays active).
    #[test]
    fn acks_match_one_warn_each() {
        let findings = vec![
            ("b".to_string(), warn_finding("x")),
            ("b".to_string(), warn_finding("x")),
        ];
        let acks = vec![ack_for("x", "close-spawn")];
        let (active, acked, stale) = partition_findings(findings, &acks);
        assert_eq!(acked.len(), 1);
        assert_eq!(active.len(), 1, "the second identical finding stays active");
        assert!(stale.is_empty());
    }

    /// The hard rule, fail-first: an ack can NEVER suppress an ERROR - the
    /// error stays active and the ack surfaces as stale.
    #[test]
    fn an_ack_never_suppresses_an_error() {
        let error = BalanceFinding {
            severity: BalanceSeverity::Error,
            kind: FindingKind::SpawnedDead,
            scenario: "s".to_string(),
            hostile: "x".to_string(),
            message: "spawned-dead: test".to_string(),
        };
        let acks = vec![ack_for("x", "spawned-dead")];
        let (active, acked, stale) = partition_findings(vec![("b".to_string(), error)], &acks);
        assert_eq!(active.len(), 1, "the error stays active");
        assert!(acked.is_empty());
        assert_eq!(stale.len(), 1, "the error-targeting ack is dead weight");
    }

    /// An ack matching nothing surfaces as stale.
    #[test]
    fn unmatched_acks_are_stale() {
        let acks = vec![ack_for("ghost", "close-spawn")];
        let (active, acked, stale) = partition_findings(vec![], &acks);
        assert!(active.is_empty() && acked.is_empty());
        assert_eq!(stale.len(), 1);
    }

    /// The shipped ack file parses and every entry names a valid kind.
    #[test]
    fn shipped_acks_parse_with_valid_kinds() {
        for ack in shipped_acks() {
            assert!(
                ["spawned-dead", "close-spawn"].contains(&ack.kind.as_str()),
                "unknown finding kind '{}' in balance_acks.ron",
                ack.kind
            );
        }
    }

    /// No player spawn = no audit (menu scenes are nobody's fight).
    #[test]
    fn scenarios_without_a_player_are_skipped() {
        let catalog = SectionCatalog::resolve(&[&[hull("h", 100.0)]]);
        let scenario = scenario_of(vec![on_start(vec![spawn_at(
            "orbiter",
            Vec3::ZERO,
            ship(ai_controller(), &["h"]),
        )])]);
        assert!(audit_scenario(&scenario, &catalog).is_none());
    }
}
