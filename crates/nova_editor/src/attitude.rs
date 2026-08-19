//! What the ship on the stage would turn like, and which limit says so.
//!
//! A correct attitude ceiling nobody can see reads as arbitrary sluggishness,
//! which is the report the model was built to answer - so the build screen owes
//! the number while the hull is being assembled. Change this module when the
//! attitude model gains a term a builder has to see.

use bevy::prelude::*;
use nova_scenario::prelude::*;
use nova_ship::prelude::*;

use crate::config::{AttitudeReadout, PlayerSpaceshipConfig};

/// The preview hull's attitude envelope, or `None` while it has no sections.
///
/// Assembled from the same authored boxes the flown ship spawns, through the
/// same avian arithmetic, so the readout and the hull cannot disagree.
pub(crate) fn preview_envelope(
    player_config: &PlayerSpaceshipConfig,
    sections: &GameSections,
) -> Option<AttitudeEnvelope> {
    let resolve = |section: &SpaceshipSectionConfig| match &section.source {
        SectionSource::Inline(config) => Some(config.clone()),
        SectionSource::Prototype(id) => sections.get_section(id).cloned(),
    };
    let parts: Vec<(Vec3, Quat, SectionConfig)> = player_config
        .sections
        .values()
        .filter_map(|section| {
            resolve(section).map(|config| (section.position, section.rotation, config))
        })
        .collect();
    if parts.is_empty() {
        return None;
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

    Some(AttitudeEnvelope::new(
        total_torque,
        properties.principal_angular_inertia.max_element(),
        arm,
    ))
}

/// The readout: the ceiling, then the word for why it is that and not more.
///
/// Two lines rather than one sentence - the rail is 150 px wide, and a single
/// line wraps inside "rad/s2".
///
/// A hull with no computer says so rather than printing a ceiling it cannot
/// reach - "no computer" is the answer to "why will it not turn", and it is a
/// different answer from "it is too big".
pub(crate) fn readout_line(envelope: Option<AttitudeEnvelope>) -> String {
    match envelope {
        None => "Turn  -".to_string(),
        Some(envelope) if envelope.ceiling() <= 0.0 => "Turn  no computer".to_string(),
        Some(envelope) => format!(
            "Turn  {:.2} rad/s2\n{}",
            envelope.ceiling(),
            envelope.binds().label()
        ),
    }
}

/// Repaint the rail's attitude row from the build state.
///
/// Runs every frame rather than on a change hook: the envelope moves with any
/// section placed or deleted anywhere on the hull, and the line it writes is
/// compared before it is stored, so an unchanged build wakes nothing.
pub(crate) fn sync_attitude_readout(
    player_config: Res<PlayerSpaceshipConfig>,
    sections: Res<GameSections>,
    mut readout: Query<&mut Text, With<AttitudeReadout>>,
) {
    let line = readout_line(preview_envelope(&player_config, &sections));
    for mut text in &mut readout {
        if text.0 != line {
            text.0.clone_from(&line);
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::platform::collections::HashMap;

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

    fn build(parts: &[(f32, SectionConfig)]) -> PlayerSpaceshipConfig {
        let mut sections = HashMap::default();
        for (index, (z, config)) in parts.iter().enumerate() {
            sections.insert(
                Entity::from_raw_u32(index as u32 + 1).unwrap(),
                SpaceshipSectionConfig {
                    id: format!("s{index}"),
                    position: Vec3::new(0.0, 0.0, *z),
                    rotation: Quat::IDENTITY,
                    source: SectionSource::Inline(config.clone()),
                    modifications: vec![],
                },
            );
        }
        PlayerSpaceshipConfig {
            sections,
            ..default()
        }
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
        let envelope = preview_envelope(&build, &GameSections::default()).unwrap();
        assert_eq!(envelope.binds(), AttitudeLimit::Structure);
        assert_eq!(
            readout_line(Some(envelope)),
            "Turn  5.23 rad/s2\nstructure-limited"
        );
    }

    /// A hull big enough to be torque-bound says so, which is the readout that
    /// makes fitting another computer a decision instead of a mystery.
    #[test]
    fn a_barge_reads_torque_limited() {
        let mut parts: Vec<(f32, SectionConfig)> = (0..45)
            .map(|index| (index as f32 - 22.0, hull("hull")))
            .collect();
        parts[22] = (0.0, computer(1501.0));
        let build = build(&parts);
        let envelope = preview_envelope(&build, &GameSections::default()).unwrap();
        assert_eq!(envelope.binds(), AttitudeLimit::Torque);
        assert!(
            readout_line(Some(envelope)).ends_with("torque-limited"),
            "got {}",
            readout_line(Some(envelope))
        );
    }

    /// A hull with no flight computer is a different problem from a hull that
    /// is too big, and the readout has to tell them apart.
    #[test]
    fn a_computerless_hull_says_so() {
        let build = build(&[(-1.0, hull("aft")), (1.0, hull("nose"))]);
        let envelope = preview_envelope(&build, &GameSections::default()).unwrap();
        assert_eq!(readout_line(Some(envelope)), "Turn  no computer");
        assert_eq!(readout_line(None), "Turn  -");
    }
}
