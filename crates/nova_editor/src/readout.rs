//! The rail's engineer readout: what the hull on the stage weighs, pushes with
//! and survives, what it would turn like, and what to do about the limit that
//! holds the turn rate down.
//!
//! A correct attitude ceiling nobody can see reads as arbitrary sluggishness,
//! which is the report the model was built to answer - so the build screen owes
//! the number while the hull is being assembled, and a number with no remedy
//! under it is only half of that. Change this module when the build screen owes
//! a builder another derived number.

use bevy::prelude::*;
use nova_events::units::prelude::*;
use nova_ship::prelude::*;

use crate::{
    config::{ShipReadout, ShipReadoutNote},
    node::{sections_of, EditContext, SectionNodes},
};

/// What the rail prints for the hull being built: the attitude envelope plus
/// the three sums every builder watches while placing.
///
/// One struct rather than four readouts, because they are read together: a
/// turn rate that dropped is explained by the mass that rose beside it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct ShipStats {
    /// The attitude envelope, or `None` while the hull has no sections.
    pub(crate) envelope: Option<AttitudeEnvelope>,
    /// Hull mass, at the only density a section has.
    pub(crate) mass: f32,
    /// Summed thruster magnitude.
    pub(crate) thrust: f32,
    /// Summed section health.
    pub(crate) health: f32,
    /// How many sections are on the hull.
    pub(crate) parts: usize,
}

/// Measure the posed parts.
///
/// Assembled from the same authored boxes the flown ship spawns, through the
/// same avian arithmetic, so the readout and the hull cannot disagree.
///
/// Takes the posed parts rather than a ship handle, so the arithmetic can be
/// exercised without a document.
pub(crate) fn preview_stats(parts: &[(Vec3, Quat, SectionConfig)]) -> ShipStats {
    if parts.is_empty() {
        return ShipStats::default();
    }

    // Density 1.0, the only density a section has: a section is solid ship, so
    // its mass IS its collider volume (`base_section.rs:376`).
    let properties = hull_mass_properties(parts.iter().map(|(position, rotation, config)| {
        (
            *position,
            *rotation,
            config.base.collider.unwrap_or_default().to_collider(),
            1.0,
        )
    }));
    let arm = structural_arm(
        properties.center_of_mass,
        parts.iter().map(|(position, rotation, config)| {
            (
                *position,
                *rotation,
                config.base.collider.unwrap_or_default().aabb_half_extents(),
            )
        }),
    );
    let total_torque: f32 = parts
        .iter()
        .filter_map(|(_, _, config)| match &config.kind {
            SectionKind::Controller(controller) => Some(controller.max_torque),
            _ => None,
        })
        .sum();

    ShipStats {
        envelope: Some(AttitudeEnvelope::new(
            total_torque,
            properties.principal_angular_inertia.max_element(),
            // Engine boundary: `structural_arm` measures the hull off its
            // collider half-extents, which are build-grid cells.
            Meters::from_engine(arm),
        )),
        mass: properties.mass,
        thrust: parts
            .iter()
            .filter_map(|(_, _, config)| match &config.kind {
                SectionKind::Thruster(thruster) => Some(thruster.magnitude),
                _ => None,
            })
            .sum(),
        health: parts.iter().map(|(_, _, config)| config.base.health).sum(),
        parts: parts.len(),
    }
}

/// The label column, in characters. Wide enough for the longest label plus the
/// space that separates it from its number, which is what lets the values line
/// up in a rail whose type is monospace.
const LABEL_W: usize = 7;

/// The block of numbers, one per line, values aligned.
///
/// A hull with no parts says so in words. `-` in every value column reads as a
/// readout that failed, which is the opposite of the truth: nothing has been
/// built yet.
pub(crate) fn stat_block(stats: &ShipStats) -> String {
    if stats.parts == 0 {
        return "no parts yet".to_string();
    }
    let turn = match stats.envelope {
        // A hull with no computer says so rather than printing a ceiling it
        // cannot reach - that is a different answer from "it is too big".
        Some(envelope) if envelope.ceiling() > 0.0 => format!("{:.2} rad/s2", envelope.ceiling()),
        _ => "no computer".to_string(),
    };
    [
        ("Turn", turn),
        ("Mass", format!("{:.1}", zeroed(stats.mass))),
        ("Thrust", format!("{:.0}", zeroed(stats.thrust))),
        ("HP", format!("{:.0}", zeroed(stats.health))),
        ("Parts", format!("{}", stats.parts)),
    ]
    .map(|(label, value)| format!("{label:<LABEL_W$}{value}"))
    .join("\n")
}

/// A sum with nothing in it, with its sign taken off.
///
/// Rust's `f32` sum starts from NEGATIVE zero, so a hull with no thrusters
/// reads `Thrust -0` - and the rail is not the place to explain signed zeros.
fn zeroed(value: f32) -> f32 {
    value + 0.0
}

/// The line under the block: which limit holds the turn rate down, and the one
/// thing that would raise it.
///
/// The limit word alone is a diagnosis with no remedy - `structure-limited`
/// tells a builder the hull is the problem and not what to do about it, and
/// the two ceilings want OPPOSITE answers, so guessing is worse than nothing.
pub(crate) fn limit_note(stats: &ShipStats) -> String {
    match stats.envelope {
        None => String::new(),
        Some(envelope) if envelope.ceiling() <= 0.0 => "no computer - fit one to steer".to_string(),
        Some(envelope) => match envelope.binds() {
            // More computers change nothing against the metal, and more metal
            // changes nothing against the computers.
            AttitudeLimit::Torque => "torque-limited - fit another computer".to_string(),
            AttitudeLimit::Structure => "structure-limited - shorten the hull".to_string(),
        },
    }
}

/// Repaint the rail's readout from the build state.
///
/// Runs every frame rather than on a change hook: the numbers move with any
/// section placed or deleted anywhere on the hull, and the lines they write are
/// compared before they are stored, so an unchanged build wakes nothing.
pub(crate) fn sync_ship_readout(
    context: Res<EditContext>,
    sections: Res<GameSections>,
    nodes: SectionNodes,
    mut block: Query<&mut Text, (With<ShipReadout>, Without<ShipReadoutNote>)>,
    mut note: Query<(&mut Text, &mut Node), With<ShipReadoutNote>>,
) {
    let parts: Vec<(Vec3, Quat, SectionConfig)> = context
        .ship()
        .map(|ship| {
            sections_of(ship, &nodes)
                .into_iter()
                .filter_map(|(_, _, section, transform)| {
                    let config = section.resolve(Some(&sections))?;
                    Some((transform.translation, transform.rotation, config.clone()))
                })
                .collect()
        })
        .unwrap_or_default();
    let stats = preview_stats(&parts);

    let lines = stat_block(&stats);
    for mut text in &mut block {
        if text.0 != lines {
            text.0.clone_from(&lines);
        }
    }

    let line = limit_note(&stats);
    // An empty note takes no room: a blank line under "no parts yet" would push
    // the rest of the block down for nothing.
    let display = if line.is_empty() {
        Display::None
    } else {
        Display::Flex
    };
    for (mut text, mut node) in &mut note {
        if text.0 != line {
            text.0.clone_from(&line);
        }
        if node.display != display {
            node.display = display;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hull(id: &str) -> SectionConfig {
        SectionConfig {
            base: BaseSectionConfig {
                id: id.to_string(),
                name: id.to_string(),
                health: 100.0,
                ..default()
            },
            kind: SectionKind::Hull(HullSectionConfig::default()),
        }
    }

    fn computer(max_torque: f32) -> SectionConfig {
        SectionConfig {
            base: BaseSectionConfig {
                id: "computer".to_string(),
                name: "computer".to_string(),
                health: 100.0,
                ..default()
            },
            kind: SectionKind::Controller(ControllerSectionConfig {
                max_torque,
                ..default()
            }),
        }
    }

    fn thruster(magnitude: f32) -> SectionConfig {
        SectionConfig {
            base: BaseSectionConfig {
                id: "thruster".to_string(),
                name: "thruster".to_string(),
                health: 50.0,
                ..default()
            },
            kind: SectionKind::Thruster(ThrusterSectionConfig {
                magnitude,
                ..default()
            }),
        }
    }

    fn build(parts: &[(f32, SectionConfig)]) -> Vec<(Vec3, Quat, SectionConfig)> {
        parts
            .iter()
            .map(|(z, config)| (Vec3::new(0.0, 0.0, *z), Quat::IDENTITY, config.clone()))
            .collect()
    }

    /// The reference hull on the build screen reads the number the model was
    /// calibrated on, and says the metal is what stops it.
    #[test]
    fn the_reference_hull_reads_its_structural_ceiling() {
        let build = build(&[
            (-1.0, hull("aft")),
            (0.0, computer(1501.0)),
            (1.0, hull("nose")),
        ]);
        let stats = preview_stats(&build);
        assert_eq!(stats.envelope.unwrap().binds(), AttitudeLimit::Structure);
        assert!(
            stat_block(&stats).starts_with("Turn   5.23 rad/s2"),
            "got {}",
            stat_block(&stats)
        );
        assert_eq!(limit_note(&stats), "structure-limited - shorten the hull");
    }

    /// A hull big enough to be torque-bound says so, which is the readout that
    /// makes fitting another computer a decision instead of a mystery.
    #[test]
    fn a_barge_reads_torque_limited() {
        let mut parts: Vec<(f32, SectionConfig)> = (0..45)
            .map(|index| (index as f32 - 22.0, hull("hull")))
            .collect();
        parts[22] = (0.0, computer(1501.0));
        let stats = preview_stats(&build(&parts));
        assert_eq!(stats.envelope.unwrap().binds(), AttitudeLimit::Torque);
        assert_eq!(limit_note(&stats), "torque-limited - fit another computer");
    }

    /// A hull with no flight computer is a different problem from a hull that
    /// is too big, and the readout has to tell them apart.
    #[test]
    fn a_computerless_hull_says_so() {
        let stats = preview_stats(&build(&[(-1.0, hull("aft")), (1.0, hull("nose"))]));
        assert!(stat_block(&stats).starts_with("Turn   no computer"));
        assert_eq!(limit_note(&stats), "no computer - fit one to steer");
    }

    /// An empty hull has not failed to measure anything - it has nothing on it
    /// yet, and `-` in every column reads as a broken readout.
    #[test]
    fn an_empty_hull_says_it_is_empty() {
        let stats = preview_stats(&[]);
        assert_eq!(stat_block(&stats), "no parts yet");
        assert_eq!(limit_note(&stats), "", "and offers no remedy for it");
    }

    /// The three sums beside the turn rate: what the hull weighs, what pushes
    /// it and what it survives, so a number that moved is explained by the
    /// number that moved with it.
    #[test]
    fn the_block_carries_mass_thrust_hp_and_a_part_count() {
        let stats = preview_stats(&build(&[
            (-1.0, thruster(120.0)),
            (0.0, computer(1501.0)),
            (1.0, hull("nose")),
        ]));
        assert_eq!(stats.parts, 3);
        assert_eq!(stats.thrust, 120.0);
        assert_eq!(stats.health, 250.0);
        // Unit boxes at density 1: three of them weigh three.
        assert!((stats.mass - 3.0).abs() < 0.01, "got {}", stats.mass);
        let block = stat_block(&stats);
        for line in ["Mass   3.0", "Thrust 120", "HP     250", "Parts  3"] {
            assert!(block.contains(line), "`{line}` is not in:\n{block}");
        }
    }

    /// A hull with no thrusters has no thrust, and it says `0`. Rust sums
    /// floats from negative zero, which put a minus sign in the rail.
    #[test]
    fn a_hull_with_no_thrusters_reads_a_plain_zero() {
        let stats = preview_stats(&build(&[(0.0, computer(1501.0)), (1.0, hull("nose"))]));
        assert!(
            stat_block(&stats).contains("Thrust 0\n"),
            "got:\n{}",
            stat_block(&stats)
        );
    }

    /// The values line up under one another: the block is read by scanning the
    /// column, and a ragged one is read by scanning every line.
    #[test]
    fn every_value_starts_in_the_same_column() {
        let stats = preview_stats(&build(&[(0.0, computer(1501.0)), (1.0, hull("nose"))]));
        for line in stat_block(&stats).lines() {
            let value = &line[LABEL_W..];
            assert!(
                !value.is_empty() && !value.starts_with(' '),
                "`{line}` does not start its value at column {LABEL_W}"
            );
        }
    }
}
