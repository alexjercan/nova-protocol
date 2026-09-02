//! What the gallery shows: the browsable slice of [`GameSections`] (category
//! plus text filter) and the readouts the focus view prints for one prototype.
//!
//! Change this module when a section kind appears, or when a stat belongs on
//! the focus card.

use bevy::prelude::*;
use nova_ship::prelude::*;
use nova_ui::prelude::*;

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
    /// Turret and railgun sections: anything that aims a gun of its own.
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

    /// The category a prototype of this kind lives under.
    pub(crate) fn of(kind: &SectionKind) -> Self {
        match kind {
            SectionKind::Hull(_) => Self::Structure,
            SectionKind::Thruster(_) => Self::Propulsion,
            SectionKind::Controller(_) => Self::Control,
            SectionKind::Turret(_) | SectionKind::Railgun(_) => Self::Weapons,
            SectionKind::Torpedo(_) => Self::Ordnance,
        }
    }

    /// Whether a prototype of this kind belongs in the category.
    pub(crate) fn accepts(self, kind: &SectionKind) -> bool {
        self == Self::All || self == Self::of(kind)
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

/// The tile's category line: the name of the CHIP this part sits under, so a
/// turret reads "weapons" both in the row that filters to it and on its own
/// tile. Two words for one group made the row look like a different axis from
/// the tiles.
pub(crate) fn kind_label(kind: &SectionKind) -> &'static str {
    GalleryCategory::of(kind).label()
}

/// The focus card's stat lines: the shared block every part has, then what its
/// kind actually does. Kept as label/value pairs so the card stays a plain
/// two-column list.
///
/// Keys are Title Case, as the inspector's field labels are: one screen said
/// `hp` and the next said `Health`, which reads as two different readouts.
pub(crate) fn stats(section: &SectionConfig) -> Vec<(String, String)> {
    let size = extent(section);
    let mut lines = vec![
        ("Kind".to_string(), kind_label(&section.kind).to_string()),
        (
            "Size".to_string(),
            format!("{:.2} x {:.2} x {:.2}", size.x, size.y, size.z),
        ),
        ("HP".to_string(), format!("{:.0}", section.base.health)),
        (
            "Sockets".to_string(),
            format!("{}", section.base.link_points.len()),
        ),
    ];
    lines.extend(behaviour(&section.kind));
    lines
}

/// The kind-specific half of the focus card.
fn behaviour(kind: &SectionKind) -> Vec<(String, String)> {
    match kind {
        SectionKind::Hull(_) => vec![("Role".to_string(), "passive structure".to_string())],
        SectionKind::Thruster(thruster) => {
            vec![("Thrust".to_string(), format!("{:.2}", thruster.magnitude))]
        }
        SectionKind::Controller(controller) => vec![
            // Torque, not turn rate: what this computer twists with is the
            // section's own number, and what a HULL does with it depends on
            // the hull. The rail's attitude readout answers that one, for the
            // ship actually being built.
            (
                "Torque".to_string(),
                format!("{:.0}", controller.max_torque),
            ),
            (
                "Steering Lag".to_string(),
                format!("{:.2} s", controller.steering_lag),
            ),
        ],
        SectionKind::Turret(turret) => vec![
            (
                "Damage".to_string(),
                format!("{:.1} {:?}", turret.bullet_damage, turret.bullet_kind),
            ),
            ("Muzzle".to_string(), units::speed(turret.muzzle_speed)),
            (
                "Ammo".to_string(),
                turret
                    .ammo_capacity
                    .map_or_else(|| "unlimited".to_string(), |ammo| format!("{ammo}")),
            ),
            (
                "Reload".to_string(),
                turret.reload.map_or_else(
                    || "none".to_string(),
                    |reload| format!("+{} / {:.1} s idle", reload.amount, reload.delay),
                ),
            ),
        ],
        SectionKind::Torpedo(torpedo) => vec![
            (
                "Blast".to_string(),
                format!(
                    "{:.0} @ {}",
                    torpedo.blast_damage,
                    units::distance(torpedo.blast_radius)
                ),
            ),
            (
                "Speed".to_string(),
                units::speed(torpedo.torpedo_type.max_speed),
            ),
            (
                "Ammo".to_string(),
                torpedo
                    .ammo_capacity
                    .map_or_else(|| "unlimited".to_string(), |ammo| format!("{ammo}")),
            ),
            (
                "Reload".to_string(),
                torpedo.reload.map_or_else(
                    || "none".to_string(),
                    |reload| format!("+{} / {:.1} s idle", reload.amount, reload.delay),
                ),
            ),
        ],
        SectionKind::Railgun(railgun) => vec![
            // Power, not a layer count: a lance is bounded by what it spends,
            // so this number is the whole answer to "how deep".
            (
                "Damage".to_string(),
                format!(
                    "{:.1} pierce @ {:.0} power",
                    railgun.slug_damage, railgun.slug_power
                ),
            ),
            (
                "Charge".to_string(),
                format!("{:.2} s", railgun.charge_seconds),
            ),
            ("Muzzle".to_string(), units::speed(railgun.slug_speed)),
            (
                "Recoil".to_string(),
                format!("{:.0} impulse", railgun.recoil_impulse),
            ),
            (
                "Reload".to_string(),
                railgun.reload.map_or_else(
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

    /// Every kind the gallery can list, one of each.
    fn one_of_each() -> Vec<SectionKind> {
        vec![
            SectionKind::Hull(HullSectionConfig::default()),
            SectionKind::Thruster(ThrusterSectionConfig::default()),
            SectionKind::Controller(ControllerSectionConfig::default()),
            SectionKind::Turret(TurretSectionConfig::default()),
            SectionKind::Torpedo(TorpedoSectionConfig::default()),
        ]
    }

    /// The word under a tile is the word on the chip that filters to it. Two
    /// words for one group ("weapon" on the tile, "Weapons" on the chip) read
    /// as two different ways of sorting the same parts.
    #[test]
    fn a_tile_reads_the_name_of_the_chip_it_sits_under() {
        for kind in one_of_each() {
            let category = GalleryCategory::of(&kind);
            assert!(
                GalleryCategory::ROW.contains(&category),
                "{category:?} is not a chip in the row"
            );
            assert_eq!(kind_label(&kind), category.label());
            assert!(category.accepts(&kind), "a chip must accept its own kind");
            assert!(GalleryCategory::All.accepts(&kind));
        }
    }

    /// One case rule across the editor: the focus card's keys are Title Case,
    /// as the inspector's field labels are.
    #[test]
    fn every_stat_key_reads_as_a_field_label() {
        for kind in one_of_each() {
            let part = section("part", "Part", kind, false);
            for (key, _) in stats(&part) {
                assert!(
                    key.chars().next().is_some_and(char::is_uppercase),
                    "`{key}` is not Title Case"
                );
            }
        }
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
