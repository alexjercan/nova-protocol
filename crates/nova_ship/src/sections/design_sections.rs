//! The sections a ship's design places, kept on the ship root so a section
//! the ship has lost can still be named, coded and rebuilt.
//!
//! `nova_scenario`'s section spawn resolves the design and inserts
//! [`ShipDesignSections`] beside the sections it spawns. A lost section is a
//! design section with no live child of that id. A Ship pane rebuild raises a
//! [`SectionRebuildRequest`], and the scenario crate spawns that design
//! section again through the same per-section spawn the ship was built with.
//!
//! A Ship pane swap replaces a live design section with a catalog part of
//! the same family: the same [`SectionClass`] and the same
//! [`SectionFootprint`] ([`section_swap_family`]). [`plan_section_swap`]
//! checks the station and the payment, [`swap_build_fault`] runs the
//! generator's post-build checks on the ship the swap would leave, and a
//! [`SectionSwapRequest`] asks the scenario crate to edit the design and
//! respawn that one section under its old id.

use bevy::prelude::*;
use nova_gameplay::prelude::{
    plan_recipe_payment, GameItems, RecipePayment, RecipePaymentRefusalType, SectionClass,
    ShipInventory, StationPartner,
};

use super::prelude::*;

/// The design sections list, the rebuild and swap requests, the anchor rule
/// and the swap rules.
pub mod prelude {
    pub use super::{
        intake_in_dock_approach, plan_section_swap, rebuild_anchored, remove_build_fault,
        section_swap_family, ship_build_fault, swap_build_fault, DesignSection,
        RefitBuildFaultType, RefitPlacedSection, SectionFamilyFaultType, SectionRebuildRequest,
        SectionRemoveRequest, SectionSwapRefusalType, SectionSwapRequest, ShipDesignSections,
    };
}

/// One placed section of a ship's resolved design.
#[derive(Clone, Debug)]
pub struct DesignSection {
    /// The section's design-local id: the `EntityId` its live entity wears.
    pub id: String,
    /// The mount cell relative to the ship root, in build-grid cells.
    pub position: Vec3,
    /// The mount rotation relative to the ship root.
    pub rotation: Quat,
    /// The finished config the section spawns from, with every patch applied.
    pub config: SectionConfig,
}

/// Every section a ship's design places, in design order, whether or not it
/// is live. Inserted on the root by the section spawn of an authored ship; a
/// hand-built root has none.
#[derive(Component, Clone, Debug, Default)]
pub struct ShipDesignSections(pub Vec<DesignSection>);

impl ShipDesignSections {
    /// The design section with id `id`, or `None` when the design has none.
    pub fn get(&self, id: &str) -> Option<&DesignSection> {
        self.0.iter().find(|section| section.id == id)
    }

    /// Design section `id`'s class and its 1-based place among the design
    /// sections of that class, in id order: the index a section code shows.
    /// It reads the design and not the live sections, so it holds while a
    /// sibling is lost and when the section is rebuilt. `None` when the
    /// design has no section `id`.
    pub fn class_index(&self, id: &str) -> Option<(SectionClass, u32)> {
        let class = self.get(id)?.config.kind.class();
        let before = self
            .0
            .iter()
            .filter(|section| section.config.kind.class() == class && section.id.as_str() < id)
            .count();
        Some((class, before as u32 + 1))
    }
}

/// Asks to spawn design section `section` of ship `entity` again, at full
/// health with the ship's binding for it. Raised by a Ship pane rebuild after
/// the rebuild rule took the recipe; the scenario crate observes it.
#[derive(EntityEvent, Clone, Debug)]
pub struct SectionRebuildRequest {
    /// The ship root.
    pub entity: Entity,
    /// The design section id to spawn.
    pub section: String,
}

/// Whether `lost`, placed where its design puts it, mates with at least one
/// of the `live` sections: a rebuilt section with no mate would join no
/// structure.
///
/// Only the live sections whose sockets can reach the lost one's are read,
/// so a two-thousand-section hull costs a handful of comparisons.
pub fn rebuild_anchored(
    live: &[PlacedSectionLinkPoints<'_>],
    lost: PlacedSectionLinkPoints<'_>,
) -> bool {
    let reach = |section: &PlacedSectionLinkPoints<'_>| {
        section
            .link_points
            .iter()
            .map(|point| point.position.length())
            .fold(0.0_f32, f32::max)
    };
    let lost_reach = reach(&lost);
    let mut placed = vec![lost];
    placed.extend(live.iter().copied().filter(|section| {
        section.position.distance(lost.position) <= reach(section) + lost_reach + 0.01
    }));
    candidate_link_point_mates(&placed)
        .iter()
        .any(|mate| (mate.a.section_index == 0) != (mate.b.section_index == 0))
}

/// Asks to replace live design section `section` of ship `entity` with the
/// catalog part `prototype`, at full health with the section's binding.
/// Raised by a Ship pane swap after the swap rule took the recipe; the
/// scenario crate observes it, saves the ship's design inline with the new
/// part and respawns that one section under its old id.
#[derive(EntityEvent, Clone, Debug)]
pub struct SectionSwapRequest {
    /// The ship root.
    pub entity: Entity,
    /// The design section id to replace.
    pub section: String,
    /// The section prototype id, in [`GameSections`], to put in its place.
    pub prototype: String,
}

/// Asks to take live section `section` off ship `entity` for good. Raised
/// by a Ship pane delete after the delete rule put the section's parts in
/// the hold; the scenario crate observes it, saves the ship's design inline
/// without the section and takes the section off, and the skin re-clads
/// round the hole.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct SectionRemoveRequest {
    /// The ship root.
    pub entity: Entity,
    /// The live section to take off, a child of the root.
    pub section: Entity,
}

/// Why a part is not of a section's family, in check order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SectionFamilyFaultType {
    /// The part is another kind of section.
    OtherKind {
        /// The section's class.
        have: SectionClass,
        /// The offered part's class.
        offered: SectionClass,
    },
    /// The part fills another cell box.
    OtherFootprint {
        /// The section's footprint, in cells.
        have: UVec3,
        /// The offered part's footprint, in cells.
        offered: UVec3,
    },
    /// The part is the prototype the section already is.
    SamePart,
}

/// Whether `offered` may replace `current` in a swap: the same kind
/// ([`SectionKind::class`]), the same [`SectionFootprint`] read from the
/// authored collider, and another prototype. Same kind keeps the section's
/// code, binding slot and the ship's capability counts; same footprint keeps
/// the cells the section fills, so the part stands where the old one stood.
pub fn section_swap_family(
    current: &SectionConfig,
    offered: &SectionConfig,
) -> Result<(), SectionFamilyFaultType> {
    let (have, offered_class) = (current.kind.class(), offered.kind.class());
    if have != offered_class {
        return Err(SectionFamilyFaultType::OtherKind {
            have,
            offered: offered_class,
        });
    }
    let footprint = |config: &SectionConfig| {
        SectionFootprint::from_collider(config.base.collider.unwrap_or_default()).0
    };
    let (have, offered_cells) = (footprint(current), footprint(offered));
    if have != offered_cells {
        return Err(SectionFamilyFaultType::OtherFootprint {
            have,
            offered: offered_cells,
        });
    }
    if current.base.id == offered.base.id {
        return Err(SectionFamilyFaultType::SamePart);
    }
    Ok(())
}

/// Why a swap changes nothing, in check order.
#[derive(Clone, Debug, PartialEq)]
pub enum SectionSwapRefusalType {
    /// The ship has no docked partner.
    NotDocked,
    /// The partner is not a living station that offers Refit.
    NoRefitService,
    /// The station does not build the offered part, whatever parts the
    /// holds carry.
    NotBuilt,
    /// The offered part is not of the section's family.
    Family(SectionFamilyFaultType),
    /// The offered part's recipe cannot be paid, see [`plan_recipe_payment`].
    Payment(RecipePaymentRefusalType),
    /// The ship the swap leaves fails a post-build check it passed, see
    /// [`swap_build_fault`].
    Build(RefitBuildFaultType),
}

/// Plan swapping the live section `current` for the catalog part `offered`
/// while docked with `partner`, paid by the offered part's recipe.
///
/// `partner` is `None` while the ship is not docked; its `offers` is whether
/// it is a living station that offers Refit, and its `builds` must name
/// `offered`'s prototype. The recipe is paid by [`plan_recipe_payment`]:
/// hold parts first, the rest bought at ask. Checks run in
/// [`SectionSwapRefusalType`] order up to the payment; the post-build checks
/// are [`swap_build_fault`], which the caller runs last. A refusal changes
/// nothing.
pub fn plan_section_swap(
    items: &GameItems,
    current: &SectionConfig,
    offered: &SectionConfig,
    partner: Option<StationPartner<'_>>,
    own: &ShipInventory,
    own_cr: u32,
) -> Result<RecipePayment, SectionSwapRefusalType> {
    let partner = partner.ok_or(SectionSwapRefusalType::NotDocked)?;
    if !partner.offers {
        return Err(SectionSwapRefusalType::NoRefitService);
    }
    if !partner.builds.iter().any(|known| *known == offered.base.id) {
        return Err(SectionSwapRefusalType::NotBuilt);
    }
    section_swap_family(current, offered).map_err(SectionSwapRefusalType::Family)?;
    plan_recipe_payment(
        items,
        &offered.base.build,
        partner.stock,
        partner.credits_cr,
        own,
        own_cr,
    )
    .map_err(SectionSwapRefusalType::Payment)
}

/// One section as the refit build checks read it: placed in ship-root space
/// with its finished config.
#[derive(Clone, Copy, Debug)]
pub struct RefitPlacedSection<'a> {
    /// Section origin in ship-root space.
    pub position: Vec3,
    /// Section rotation in ship-root space.
    pub rotation: Quat,
    /// The finished config the section spawned from.
    pub config: &'a SectionConfig,
}

impl<'a> RefitPlacedSection<'a> {
    /// The section as the cell readings see it.
    pub(crate) fn placed_part(&self) -> PlacedPart<'a> {
        PlacedPart {
            position: self.position,
            rotation: self.rotation,
            link_points: &self.config.base.link_points,
            footprint: SectionFootprint::from_collider(
                self.config.base.collider.unwrap_or_default(),
            )
            .0,
            exit: exit_normal(&self.config.kind),
        }
    }

    /// The section as the socket graph sees it.
    pub(crate) fn link_points(&self) -> PlacedSectionLinkPoints<'a> {
        PlacedSectionLinkPoints {
            position: self.position,
            rotation: self.rotation,
            link_points: &self.config.base.link_points,
        }
    }
}

/// A post-build check the ship a swap or an add leaves fails, in check order.
#[derive(Clone, Debug, PartialEq)]
pub enum RefitBuildFaultType {
    /// The sockets no longer join the sections into one ship.
    Disconnected,
    /// The sockets are invalid or ambiguous, see [`LinkPointGraphError`].
    InvalidSockets,
    /// A section can no longer fire, launch or exhaust down a clear lane.
    BlockedExit,
    /// The drives push less than the ship's mass needs.
    Underpowered {
        /// The drives' summed thrust after the change.
        thrust: f32,
        /// The thrust the ship's mass needs.
        needed: f32,
    },
    /// The ship carries fewer flight computers than its mass needs.
    Undercontrolled {
        /// The flight computers it carries.
        controllers: usize,
        /// How many its mass needs.
        needed: usize,
    },
}

/// The first post-build check the ship `live` fails once section `index` is
/// replaced by `offered` and passed before, or `None` when the swap adds no
/// fault.
///
/// The checks are the open-world generator's own, see [`refit_build_fault`].
/// The swapped part fills the same cells, so the exit lanes are read on one
/// lattice each and compared cell for cell.
///
/// # Panics
///
/// When `index` is not in `live`.
pub fn swap_build_fault(
    live: &[RefitPlacedSection<'_>],
    index: usize,
    offered: &SectionConfig,
) -> Option<RefitBuildFaultType> {
    let mut after = live.to_vec();
    after[index].config = offered;
    let blocked = |sections: &[RefitPlacedSection<'_>]| {
        let placed: Vec<PlacedPart<'_>> = sections.iter().map(|s| s.placed_part()).collect();
        let (structure, _, occupied) = read_structure(&placed);
        blocked_exits(&structure, &ship_exits(&placed, &occupied)).len()
    };
    refit_build_fault(live, &after, || blocked(&after) > blocked(live))
}

/// The first post-build check the ship `live` fails once section `index` is
/// taken off and passed before, or `None` when the removal adds no fault.
///
/// The checks are [`refit_build_fault`]'s. A removal opens lanes and never
/// blocks one. On top of them, the last flight computer never comes off,
/// even from a ship that was already short of its count.
///
/// # Panics
///
/// When `index` is not in `live`.
pub fn remove_build_fault(
    live: &[RefitPlacedSection<'_>],
    index: usize,
) -> Option<RefitBuildFaultType> {
    let mut after = live.to_vec();
    after.remove(index);
    if let Some(fault) = refit_build_fault(live, &after, || false) {
        return Some(fault);
    }
    let measure = ShipMeasure::of(after.iter().map(|section| section.config));
    let before = ShipMeasure::of(live.iter().map(|section| section.config));
    (before.controllers > 0 && measure.controllers == 0).then(|| {
        RefitBuildFaultType::Undercontrolled {
            controllers: 0,
            needed: measure.needed_controllers(),
        }
    })
}

/// The first post-build check a whole built ship fails, or `None` when it
/// passes them all.
///
/// The same checks as [`refit_build_fault`], read absolutely rather than as
/// a change: one socket graph, clear exit lanes, the thrust floor and the
/// flight computer count. An authored start hull is held to it, so the ship a
/// new player is handed could pass every refit it is offered.
pub fn ship_build_fault(sections: &[RefitPlacedSection<'_>]) -> Option<RefitBuildFaultType> {
    let links: Vec<PlacedSectionLinkPoints<'_>> = sections
        .iter()
        .map(|section| section.link_points())
        .collect();
    if let Err(errors) = derive_link_point_graph(&links) {
        return Some(match errors.as_slice() {
            [LinkPointGraphError::Disconnected { .. }] => RefitBuildFaultType::Disconnected,
            _ => RefitBuildFaultType::InvalidSockets,
        });
    }
    let placed: Vec<PlacedPart<'_>> = sections.iter().map(|s| s.placed_part()).collect();
    let (structure, _, occupied) = read_structure(&placed);
    if !blocked_exits(&structure, &ship_exits(&placed, &occupied)).is_empty() {
        return Some(RefitBuildFaultType::BlockedExit);
    }
    ShipMeasure::of(sections.iter().map(|s| s.config))
        .shortfall()
        .map(|shortfall| match shortfall {
            ShipMeasureShortfallType::Underpowered { thrust, needed } => {
                RefitBuildFaultType::Underpowered { thrust, needed }
            }
            ShipMeasureShortfallType::Undercontrolled {
                controllers,
                needed,
            } => RefitBuildFaultType::Undercontrolled {
                controllers,
                needed,
            },
        })
}

/// The first cargo intake of `sections` that reaches a docking port's mating
/// face along that port's outward normal, as the indices `(intake, port)`, or
/// `None` when every intake stays behind every port.
///
/// A berthed ship parks against the port's face, so an intake that reaches
/// the face plane stands where the berthed hull does: the hulls collide and
/// push apart, and the dock motion gate refuses the berth. Read on the
/// section colliders' bounding boxes, in build-grid cells.
pub fn intake_in_dock_approach(sections: &[RefitPlacedSection<'_>]) -> Option<(usize, usize)> {
    // Below a hundredth of a cell is the same plane: positions are authored
    // on the grid, and a rotated box's extent carries float noise.
    const SAME_PLANE: f32 = 1e-2;
    let reach = |section: &RefitPlacedSection<'_>, normal: Vec3| {
        let half = section
            .config
            .base
            .collider
            .unwrap_or_default()
            .rotated_aabb_half_extents(section.rotation);
        section.position.dot(normal) + half.dot(normal.abs())
    };
    sections
        .iter()
        .enumerate()
        .filter(|(_, port)| matches!(port.config.kind, SectionKind::Docking(_)))
        .find_map(|(port_index, port)| {
            let normal = port.rotation * exit_normal(&port.config.kind)?;
            let face = reach(port, normal);
            sections
                .iter()
                .position(|intake| {
                    matches!(intake.config.kind, SectionKind::CargoIntake(_))
                        && reach(intake, normal) >= face - SAME_PLANE
                })
                .map(|intake_index| (intake_index, port_index))
        })
}

/// The first post-build check the ship `after` fails and `before` passed, or
/// `None` when the change adds no fault. `blocks_an_exit` answers whether the
/// change leaves a section unable to fire that could before; it runs only once
/// the sockets pass.
///
/// The checks are the open-world generator's own, on the same readings: one
/// socket graph ([`derive_link_point_graph`]), clear exit lanes
/// ([`blocked_exits`]), the thrust floor and the flight computer count
/// ([`ShipMeasure`]). A generated ship's role, advancement, size ceiling,
/// mirror symmetry and ruin floor are not checked: a refit answers to no
/// generation request, and changing one of a mirrored pair breaks the symmetry
/// by design. Only a NEW fault refuses, the rule
/// [`placement_blocks_an_exit`] gives a builder: a ship that already fails a
/// check is not made worse by a change that leaves it failing the same way.
pub(crate) fn refit_build_fault(
    before: &[RefitPlacedSection<'_>],
    after: &[RefitPlacedSection<'_>],
    blocks_an_exit: impl FnOnce() -> bool,
) -> Option<RefitBuildFaultType> {
    let sockets = |sections: &[RefitPlacedSection<'_>]| {
        let links: Vec<PlacedSectionLinkPoints<'_>> = sections
            .iter()
            .map(|section| section.link_points())
            .collect();
        derive_link_point_graph(&links)
            .err()
            .map(|errors| match errors.as_slice() {
                [LinkPointGraphError::Disconnected { .. }] => RefitBuildFaultType::Disconnected,
                _ => RefitBuildFaultType::InvalidSockets,
            })
    };
    if let (None, Some(fault)) = (sockets(before), sockets(after)) {
        return Some(fault);
    }
    if blocks_an_exit() {
        return Some(RefitBuildFaultType::BlockedExit);
    }

    // Each floor on its own, so a ship that was short of flight computers
    // is still refused a change that also leaves it short of thrust.
    let measure =
        |sections: &[RefitPlacedSection<'_>]| ShipMeasure::of(sections.iter().map(|s| s.config));
    let (before, after) = (measure(before), measure(after));
    if before.thrust >= before.needed_thrust() && after.thrust < after.needed_thrust() {
        return Some(RefitBuildFaultType::Underpowered {
            thrust: after.thrust,
            needed: after.needed_thrust(),
        });
    }
    if before.controllers >= before.needed_controllers()
        && after.controllers < after.needed_controllers()
    {
        return Some(RefitBuildFaultType::Undercontrolled {
            controllers: after.controllers,
            needed: after.needed_controllers(),
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use nova_events::units::prelude::{Meters, MetersPerSecond};

    use super::*;

    fn cube_points() -> Vec<LinkPoint> {
        [
            ("px", Vec3::X),
            ("nx", Vec3::NEG_X),
            ("py", Vec3::Y),
            ("ny", Vec3::NEG_Y),
            ("pz", Vec3::Z),
            ("nz", Vec3::NEG_Z),
        ]
        .into_iter()
        .map(|(id, normal)| LinkPoint {
            id: id.to_string(),
            position: normal * 0.5,
            normal,
            skin: LinkPointSkinType::Clad,
        })
        .collect()
    }

    /// Codes count the design, not the live hull: a turret's index is its
    /// place among the design's turrets by id, whatever else is lost.
    #[test]
    fn a_section_index_counts_its_class_in_the_design_by_id() {
        let section = |id: &str, kind: SectionKind| DesignSection {
            id: id.to_string(),
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            config: SectionConfig {
                base: BaseSectionConfig::default(),
                kind,
            },
        };
        let turret = || SectionKind::Turret(TurretSectionConfig::default());
        let hull = || SectionKind::Hull(HullSectionConfig::default());
        let design = ShipDesignSections(vec![
            section("pdc_starboard", turret()),
            section("bow", hull()),
            section("pdc_port", turret()),
            section("aft", hull()),
        ]);
        assert_eq!(
            design.class_index("pdc_port"),
            Some((SectionClass::Turret, 1))
        );
        assert_eq!(
            design.class_index("pdc_starboard"),
            Some((SectionClass::Turret, 2))
        );
        assert_eq!(design.class_index("bow"), Some((SectionClass::Hull, 2)));
        assert_eq!(design.class_index("missing"), None);
    }

    /// A cube beside a live cube mates with it; the same cube two cells away
    /// mates with nothing and would float free.
    #[test]
    fn a_lost_section_is_anchored_only_beside_a_live_one() {
        let points = cube_points();
        let place = |x: f32| PlacedSectionLinkPoints {
            position: Vec3::new(x, 0.0, 0.0),
            rotation: Quat::IDENTITY,
            link_points: &points,
        };
        let live = [place(0.0)];
        assert!(rebuild_anchored(&live, place(1.0)));
        assert!(!rebuild_anchored(&live, place(2.0)));
    }

    fn part(id: &str, kind: SectionKind, size: Vec3, link_points: Vec<LinkPoint>) -> SectionConfig {
        SectionConfig {
            base: BaseSectionConfig {
                id: id.to_string(),
                collider: Some(SectionCollider::Cuboid { size }),
                link_points,
                ..default()
            },
            kind,
        }
    }

    fn turret() -> SectionKind {
        SectionKind::Turret(TurretSectionConfig::default())
    }

    fn thruster(magnitude: f32) -> SectionKind {
        SectionKind::Thruster(ThrusterSectionConfig {
            magnitude,
            ..default()
        })
    }

    /// The family is the kind and the cell box: a half-cell turret takes any
    /// other one-cell turret, and refuses a drive, a two-cell turret and
    /// itself, each named.
    #[test]
    fn a_swap_takes_only_another_part_of_the_same_kind_and_footprint() {
        let half = Vec3::splat(0.5);
        let current = part("pdc_kinetic", turret(), half, cube_points());
        let sibling = part("pdc_pierce", turret(), Vec3::ONE, cube_points());
        assert_eq!(section_swap_family(&current, &sibling), Ok(()));

        let drive = part("drive", thruster(1.0), half, cube_points());
        assert_eq!(
            section_swap_family(&current, &drive),
            Err(SectionFamilyFaultType::OtherKind {
                have: SectionClass::Turret,
                offered: SectionClass::Thruster,
            })
        );
        let long = part(
            "long_gun",
            turret(),
            Vec3::new(1.0, 1.0, 2.0),
            cube_points(),
        );
        assert_eq!(
            section_swap_family(&current, &long),
            Err(SectionFamilyFaultType::OtherFootprint {
                have: UVec3::ONE,
                offered: UVec3::new(1, 1, 2),
            })
        );
        assert_eq!(
            section_swap_family(&current, &current.clone()),
            Err(SectionFamilyFaultType::SamePart)
        );
    }

    /// Knowledge before materials: a station that does not build the offered
    /// part refuses the swap before the family check and the payment, and
    /// one that builds it plans the same swap.
    #[test]
    fn a_swap_is_refused_before_payment_for_a_part_the_station_does_not_build() {
        use nova_gameplay::test_support::test_items;

        let items = test_items();
        let current = part("pdc_kinetic", turret(), Vec3::ONE, cube_points());
        let offered = part("pdc_pierce", turret(), Vec3::ONE, cube_points());
        let drive = part("drive", thruster(1.0), Vec3::ONE, cube_points());
        let empty = ShipInventory::new(&items, 10_000_000, []);
        let known = vec![offered.base.id.clone()];
        let other = vec![current.base.id.clone()];
        let partner = |builds| {
            Some(StationPartner {
                offers: true,
                builds,
                stock: &empty,
                credits_cr: 0,
            })
        };

        assert_eq!(
            plan_section_swap(&items, &current, &drive, partner(&other), &empty, u32::MAX),
            Err(SectionSwapRefusalType::NotBuilt),
            "a drive offered for a gun is refused as not built before its family"
        );
        assert_eq!(
            plan_section_swap(
                &items,
                &current,
                &offered,
                partner(&other),
                &empty,
                u32::MAX
            ),
            Err(SectionSwapRefusalType::NotBuilt)
        );
        assert!(
            plan_section_swap(&items, &current, &offered, partner(&known), &empty, 0).is_ok(),
            "a free recipe at a station that builds the part plans"
        );
    }

    /// A swap is refused only for a fault it adds: a gun with no socket would
    /// float free of the hull, and a weaker drive that drops a ship under the
    /// thrust floor is refused, while the same drive on a ship already under
    /// it is not.
    #[test]
    fn a_swap_is_refused_only_for_a_post_build_fault_it_adds() {
        let one = Vec3::ONE;
        let hull = part(
            "hull",
            SectionKind::Hull(HullSectionConfig::default()),
            one,
            cube_points(),
        );
        let gun = part("gun", turret(), one, cube_points());
        let strong = part("strong", thruster(10.0), one, cube_points());
        let weak = part("weak", thruster(0.01), one, cube_points());
        let at = |x: f32, config| RefitPlacedSection {
            position: Vec3::new(x, 0.0, 0.0),
            rotation: Quat::IDENTITY,
            config,
        };

        let floating = part("floating", turret(), one, Vec::new());
        let ship = [at(0.0, &hull), at(1.0, &gun), at(-1.0, &strong)];
        assert_eq!(swap_build_fault(&ship, 1, &gun), None);
        assert_eq!(
            swap_build_fault(&ship, 1, &floating),
            Some(RefitBuildFaultType::Disconnected)
        );
        assert!(matches!(
            swap_build_fault(&ship, 2, &weak),
            Some(RefitBuildFaultType::Underpowered { .. })
        ));

        let already_short = [at(0.0, &hull), at(1.0, &gun), at(-1.0, &weak)];
        assert_eq!(swap_build_fault(&already_short, 2, &weak), None);
    }

    /// A removal is refused when it would split the ship or take off its last
    /// flight computer, and a leaf gun comes off.
    #[test]
    fn a_removal_is_refused_when_it_splits_the_ship_or_takes_the_last_computer() {
        let one = Vec3::ONE;
        let hull = part(
            "hull",
            SectionKind::Hull(HullSectionConfig::default()),
            one,
            cube_points(),
        );
        let computer = part(
            "computer",
            SectionKind::Controller(ControllerSectionConfig {
                steering_lag: 0.2,
                max_torque: 1.0,
                render_mesh: None,
                render_mesh_transform: None,
            }),
            one,
            cube_points(),
        );
        let gun = part("gun", turret(), one, cube_points());
        let drive = part("drive", thruster(10.0), one, cube_points());
        let at = |x: f32, config| RefitPlacedSection {
            position: Vec3::new(x, 0.0, 0.0),
            rotation: Quat::IDENTITY,
            config,
        };
        // gun - hull - computer - drive, in a line.
        let ship = [
            at(1.0, &gun),
            at(0.0, &hull),
            at(-1.0, &computer),
            at(-2.0, &drive),
        ];
        assert_eq!(remove_build_fault(&ship, 0), None);
        assert_eq!(
            remove_build_fault(&ship, 1),
            Some(RefitBuildFaultType::Disconnected)
        );
        let tail = [at(1.0, &gun), at(0.0, &hull), at(-1.0, &computer)];
        assert!(matches!(
            remove_build_fault(&tail, 2),
            Some(RefitBuildFaultType::Undercontrolled { controllers: 0, .. })
        ));
    }

    /// The armored wreck a berth failed at: an intake on the starboard flank
    /// 1.5 cells aft of the starboard port reaches the port's face, so the
    /// berthed hull hits it. The same intake lying door up on the deck stays
    /// behind the face.
    #[test]
    fn an_intake_on_a_port_flank_stands_in_the_berth_and_one_on_the_deck_does_not() {
        let port = part(
            "dock",
            SectionKind::Docking(DockingSectionConfig::default()),
            Vec3::ONE,
            cube_points(),
        );
        let intake = part(
            "intake",
            SectionKind::CargoIntake(CargoIntakeSectionConfig {
                render_mesh: "intake.glb#Scene0".into(),
                render_mesh_transform: None,
                canister_mesh: "canister.glb#Scene0".into(),
                door_sound: "door.wav".into(),
                eject_sound: "eject.wav".into(),
                take_sound: "take.wav".into(),
                detection_range: Meters(40.0),
                capture_gap: Meters(1.0),
                aperture_width: Meters(8.0),
                aperture_height: Meters(8.0),
                eject_speed: MetersPerSecond(3.0),
            }),
            Vec3::new(3.0, 2.0, 1.0),
            cube_points(),
        );
        // Each opens through local -Z: a quarter turn about Y aims it to
        // starboard, a quarter turn about X aims the intake up.
        let starboard = Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2);
        let berth = RefitPlacedSection {
            position: Vec3::new(2.0, 0.0, 1.0),
            rotation: starboard,
            config: &port,
        };
        let flank = RefitPlacedSection {
            position: Vec3::new(2.0, 0.0, 2.5),
            rotation: starboard * Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
            config: &intake,
        };
        let deck = RefitPlacedSection {
            position: Vec3::new(0.0, 1.0, 1.5),
            rotation: Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
            config: &intake,
        };
        assert_eq!(
            intake_in_dock_approach(&[berth.clone(), flank]),
            Some((1, 0))
        );
        assert_eq!(intake_in_dock_approach(&[berth, deck]), None);
    }
}
