//! What the gallery shows: the browsable slice of [`GameSections`] (category
//! plus text filter) and the readouts the focus view prints for one prototype.
//!
//! Change this module when a section kind appears, or when a stat belongs on
//! the focus card.

use bevy::prelude::*;
use nova_ship::prelude::*;

/// The category filter across the top of the gallery. One entry per section
/// kind plus [`GalleryCategory::All`]; the kinds are named for what the part
/// DOES, which is how a builder looks for it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum GalleryCategory {
    /// Every browsable prototype.
    #[default]
    All,
    /// Hull sections.
    Structure,
    /// Thruster sections.
    Propulsion,
    /// Controller sections.
    Control,
    /// Turret sections.
    Weapons,
    /// Torpedo bays.
    Ordnance,
}

impl GalleryCategory {
    /// Every category, in the order the filter row shows them.
    pub(crate) const ROW: [Self; 6] = [
        Self::All,
        Self::Structure,
        Self::Propulsion,
        Self::Control,
        Self::Weapons,
        Self::Ordnance,
    ];

    /// The button label.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Structure => "Structure",
            Self::Propulsion => "Propulsion",
            Self::Control => "Control",
            Self::Weapons => "Weapons",
            Self::Ordnance => "Ordnance",
        }
    }

    /// Whether a prototype of this kind belongs in the category.
    pub(crate) fn accepts(self, kind: &SectionKind) -> bool {
        match self {
            Self::All => true,
            Self::Structure => matches!(kind, SectionKind::Hull(_)),
            Self::Propulsion => matches!(kind, SectionKind::Thruster(_)),
            Self::Control => matches!(kind, SectionKind::Controller(_)),
            Self::Weapons => matches!(kind, SectionKind::Turret(_)),
            Self::Ordnance => matches!(kind, SectionKind::Torpedo(_)),
        }
    }
}

/// The catalog indices the gallery lists, in catalog order.
///
/// `hide_in_editor` is honoured exactly as the drawer honours it, so a
/// prototype hidden from one picker is hidden from both. The text filter is a
/// case-insensitive substring of the display name or the catalog id - the id
/// is what carries a part's ship family (`racer_nose`), so typing "racer"
/// narrows to one craft.
pub(crate) fn browsable(
    sections: &GameSections,
    category: GalleryCategory,
    filter: &str,
) -> Vec<usize> {
    let needle = filter.trim().to_lowercase();
    sections
        .iter()
        .enumerate()
        .filter(|(_, section)| !section.base.hide_in_editor)
        .filter(|(_, section)| category.accepts(&section.kind))
        .filter(|(_, section)| {
            needle.is_empty()
                || section.base.name.to_lowercase().contains(&needle)
                || section.base.id.to_lowercase().contains(&needle)
        })
        .map(|(index, _)| index)
        .collect()
}

/// The part's authored extent, used to fit its preview into a tile. The
/// collider is the only authored size a section carries; an unset one resolves
/// to the unit cube, exactly as it does in physics.
pub(crate) fn extent(section: &SectionConfig) -> Vec3 {
    section
        .base
        .collider
        .unwrap_or_default()
        .aabb_half_extents()
        * 2.0
}

/// The kind's one-word name, for the tile's category line.
pub(crate) fn kind_label(kind: &SectionKind) -> &'static str {
    match kind {
        SectionKind::Hull(_) => "structure",
        SectionKind::Thruster(_) => "propulsion",
        SectionKind::Controller(_) => "control",
        SectionKind::Turret(_) => "weapon",
        SectionKind::Torpedo(_) => "ordnance",
    }
}

/// The focus card's stat lines: the shared block every part has, then what its
/// kind actually does. Kept as label/value pairs so the card stays a plain
/// two-column list.
pub(crate) fn stats(section: &SectionConfig) -> Vec<(String, String)> {
    let size = extent(section);
    let mut lines = vec![
        ("kind".to_string(), kind_label(&section.kind).to_string()),
        (
            "size".to_string(),
            format!("{:.2} x {:.2} x {:.2}", size.x, size.y, size.z),
        ),
        ("hp".to_string(), format!("{:.0}", section.base.health)),
        (
            "sockets".to_string(),
            format!("{}", section.base.link_points.len()),
        ),
    ];
    lines.extend(behaviour(&section.kind));
    lines
}

/// The kind-specific half of the focus card.
fn behaviour(kind: &SectionKind) -> Vec<(String, String)> {
    match kind {
        SectionKind::Hull(_) => vec![("role".to_string(), "passive structure".to_string())],
        SectionKind::Thruster(thruster) => {
            vec![("thrust".to_string(), format!("{:.2}", thruster.magnitude))]
        }
        SectionKind::Controller(controller) => vec![
            (
                "turn accel".to_string(),
                format!("{:.2} rad/s2", controller.max_angular_acceleration),
            ),
            (
                "steering lag".to_string(),
                format!("{:.2} s", controller.steering_lag),
            ),
        ],
        SectionKind::Turret(turret) => vec![
            (
                "damage".to_string(),
                format!("{:.1} {:?}", turret.bullet_damage, turret.bullet_kind),
            ),
            (
                "muzzle".to_string(),
                format!("{:.0} u/s", turret.muzzle_speed),
            ),
            (
                "ammo".to_string(),
                turret
                    .ammo_capacity
                    .map_or_else(|| "unlimited".to_string(), |ammo| format!("{ammo}")),
            ),
            (
                "reload".to_string(),
                turret.reload.map_or_else(
                    || "none".to_string(),
                    |reload| format!("+{} / {:.1} s idle", reload.amount, reload.delay),
                ),
            ),
        ],
        SectionKind::Torpedo(torpedo) => vec![
            (
                "blast".to_string(),
                format!(
                    "{:.0} @ {:.0} u",
                    torpedo.blast_damage, torpedo.blast_radius
                ),
            ),
            (
                "speed".to_string(),
                format!("{:.0} u/s", torpedo.torpedo_type.max_speed),
            ),
            (
                "ammo".to_string(),
                torpedo
                    .ammo_capacity
                    .map_or_else(|| "unlimited".to_string(), |ammo| format!("{ammo}")),
            ),
            (
                "reload".to_string(),
                torpedo.reload.map_or_else(
                    || "none".to_string(),
                    |reload| format!("+{} / {:.1} s idle", reload.amount, reload.delay),
                ),
            ),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn section(id: &str, name: &str, kind: SectionKind, hidden: bool) -> SectionConfig {
        SectionConfig {
            base: BaseSectionConfig {
                id: id.to_string(),
                name: name.to_string(),
                hide_in_editor: hidden,
                ..default()
            },
            kind,
        }
    }

    fn catalog() -> GameSections {
        GameSections(vec![
            section(
                "reinforced_hull_section",
                "Reinforced Hull Section",
                SectionKind::Hull(HullSectionConfig::default()),
                false,
            ),
            section(
                "racer_nose",
                "Nose",
                SectionKind::Hull(HullSectionConfig::default()),
                false,
            ),
            section(
                "basic_thruster_section",
                "Basic Thruster Section",
                SectionKind::Thruster(ThrusterSectionConfig::default()),
                false,
            ),
            section(
                "heavy_torpedo_section",
                "Siege Torpedo Bay Section",
                SectionKind::Torpedo(TorpedoSectionConfig::default()),
                true,
            ),
        ])
    }

    /// The gallery hides what the drawer hides: a `hide_in_editor` prototype is
    /// scene dressing, and it must not be offered by either picker.
    #[test]
    fn hidden_prototypes_never_reach_the_gallery() {
        let listed = browsable(&catalog(), GalleryCategory::All, "");
        assert_eq!(listed, vec![0, 1, 2]);
        assert!(
            browsable(&catalog(), GalleryCategory::Ordnance, "").is_empty(),
            "the only ordnance in this catalog is hidden"
        );
    }

    #[test]
    fn the_category_row_narrows_to_one_kind() {
        assert_eq!(
            browsable(&catalog(), GalleryCategory::Structure, ""),
            vec![0, 1]
        );
        assert_eq!(
            browsable(&catalog(), GalleryCategory::Propulsion, ""),
            vec![2]
        );
    }

    /// The filter matches the ID as well as the name, which is what makes a
    /// ship family ("racer") findable - the semantic parts are all named for
    /// their role, not their craft.
    #[test]
    fn the_text_filter_matches_name_or_id_case_insensitively() {
        assert_eq!(
            browsable(&catalog(), GalleryCategory::All, "RACER"),
            vec![1]
        );
        assert_eq!(
            browsable(&catalog(), GalleryCategory::All, "thruster"),
            vec![2]
        );
        assert!(browsable(&catalog(), GalleryCategory::All, "zzz").is_empty());
        // Category and filter compose.
        assert!(browsable(&catalog(), GalleryCategory::Propulsion, "racer").is_empty());
    }

    /// An unset collider is the unit cube in physics, so it must be the unit
    /// cube here too - a preview fitted to a wrong extent reads as the wrong
    /// size next to its neighbours.
    #[test]
    fn an_unset_collider_measures_as_the_unit_cube() {
        let plain = section(
            "hull",
            "Hull",
            SectionKind::Hull(HullSectionConfig::default()),
            false,
        );
        assert_eq!(extent(&plain), Vec3::ONE);

        let mut sized = plain;
        sized.base.collider = Some(SectionCollider::Cuboid {
            size: Vec3::new(2.0, 1.0, 4.0),
        });
        assert_eq!(extent(&sized), Vec3::new(2.0, 1.0, 4.0));
    }
}
