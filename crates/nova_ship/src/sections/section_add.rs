//! A section added at a free attachment point of a ship, at a station.
//!
//! A free attachment point is a socket of a live section that no section of
//! the ship's design mates, live or lost ([`free_link_points`]). Every hull
//! has them, base, generated or modded, with nothing authored for the
//! purpose, and a lost section's socket stays reserved for its rebuild. The
//! offered part mates the point with its [`natural_link_point`] at roll 0, so
//! it keeps the orientation it was drawn in.
//!
//! [`section_add_fit`] refuses an occupied point, a part with no socket, an
//! overlap and a post-build fault the add makes ([`RefitBuildFaultType`]).
//! [`plan_section_add`] checks the station, the binding the new section takes
//! ([`added_section_binds`]) and the payment, and a [`SectionAddRequest`] asks
//! the scenario crate to put the part in the ship's design under
//! [`added_section_id`] and spawn it.

use std::collections::BTreeSet;

use bevy::prelude::*;
use nova_gameplay::prelude::{
    plan_recipe_payment, GameItems, RecipePayment, RecipePaymentRefusalType, SectionClass,
    ShipInventory, StationPartner,
};
use nova_input::prelude::InputSource;

use super::{design_sections::refit_build_fault, prelude::*};

/// The free-point and fit rules, the bind rule, the id rule, the plan and the
/// add request.
pub mod prelude {
    pub use super::{
        add_build_fault, added_section_binds, added_section_id, free_link_points, plan_section_add,
        section_add_fit, section_add_pose, SectionAddFaultType, SectionAddPlan,
        SectionAddRefusalType, SectionAddRequest, ADDED_MINING_KEYS,
    };
}

/// The desk keys an added mining emitter takes, in order: the first one no
/// other emitter on the ship holds, so one press never deploys two. The same
/// keys, in the same order, the editor gives a placed emitter.
pub const ADDED_MINING_KEYS: [KeyCode; 4] =
    [KeyCode::KeyV, KeyCode::KeyB, KeyCode::KeyN, KeyCode::KeyC];

/// Asks to add the catalog part `prototype` to ship `entity` as design
/// section `section`, posed at `position` and `rotation` in ship-root space,
/// at full health with `bindings`. Raised by a Ship pane add after the add
/// rule took the recipe; the scenario crate observes it, saves the ship's
/// design inline with the new section and spawns it.
#[derive(EntityEvent, Clone, Debug)]
pub struct SectionAddRequest {
    /// The ship root.
    pub entity: Entity,
    /// The new design section id, see [`added_section_id`].
    pub section: String,
    /// The section prototype id, in [`GameSections`], to add.
    pub prototype: String,
    /// The mount position in ship-root space.
    pub position: Vec3,
    /// The mount rotation in ship-root space.
    pub rotation: Quat,
    /// The player binding the section takes, see [`added_section_binds`];
    /// `None` writes no binding.
    pub bindings: Option<Vec<InputSource>>,
}

/// How far from its origin a section reaches: its farthest socket or the
/// corner of its collider box, whichever is farther.
fn reach(section: &RefitPlacedSection<'_>) -> f32 {
    let sockets = section
        .config
        .base
        .link_points
        .iter()
        .map(|point| point.position.length())
        .fold(0.0_f32, f32::max);
    sockets.max(collider(section).half_extents().length())
}

fn collider(section: &RefitPlacedSection<'_>) -> PlacedSectionCollider {
    PlacedSectionCollider {
        position: section.position,
        rotation: section.rotation,
        collider: section.config.base.collider.unwrap_or_default(),
    }
}

/// The indices of the `design` sections that can touch `section`, leaving
/// out `skip`. Only these are read, so a two-thousand-section hull costs a
/// handful of comparisons per point.
fn neighbours(
    design: &[RefitPlacedSection<'_>],
    section: &RefitPlacedSection<'_>,
    skip: Option<usize>,
) -> Vec<usize> {
    let own = reach(section);
    design
        .iter()
        .enumerate()
        .filter(|(index, other)| {
            Some(*index) != skip
                && other.position.distance(section.position) <= own + reach(other) + 0.01
        })
        .map(|(index, _)| index)
        .collect()
}

/// The sections `subject` mates among `design[near]`, as indices into `near`
/// shifted by one, and the subject's sockets that mate, by index.
fn subject_mates(
    design: &[RefitPlacedSection<'_>],
    subject: &RefitPlacedSection<'_>,
    near: &[usize],
) -> (BTreeSet<usize>, BTreeSet<usize>) {
    let mut placed = vec![subject.link_points()];
    placed.extend(near.iter().map(|&index| design[index].link_points()));
    let mut sections = BTreeSet::new();
    let mut sockets = BTreeSet::new();
    for mate in candidate_link_point_mates(&placed) {
        for (own, other) in [(mate.a, mate.b), (mate.b, mate.a)] {
            if own.section_index == 0 && other.section_index != 0 {
                sections.insert(other.section_index);
                sockets.insert(own.link_point_index);
            }
        }
    }
    (sections, sockets)
}

/// The sockets of design section `section` that no other section of
/// `design` mates, by index, in authored order: its free attachment points.
///
/// `design` is every section of the ship's design, live or lost, so a socket
/// a lost section mates stays reserved for that section's rebuild.
///
/// # Panics
///
/// When `section` is not in `design`.
pub fn free_link_points(design: &[RefitPlacedSection<'_>], section: usize) -> Vec<usize> {
    let subject = &design[section];
    let near = neighbours(design, subject, Some(section));
    let (_, taken) = subject_mates(design, subject, &near);
    (0..subject.config.base.link_points.len())
        .filter(|point| !taken.contains(point))
        .collect()
}

/// Why a part does not fit at an attachment point, in check order.
#[derive(Clone, Debug, PartialEq)]
pub enum SectionAddFaultType {
    /// Another section of the design already mates the point.
    Occupied,
    /// The offered part has no socket to mate the point with.
    NoSockets,
    /// The part's collider box would interpenetrate a section of the design
    /// it does not mate.
    Overlap,
    /// The ship the add leaves fails a post-build check it passed.
    Build(RefitBuildFaultType),
}

/// Where `offered` stands when added at socket `point` of design section
/// `section`, as `(position, rotation)` in ship-root space, before the
/// overlap and post-build checks of [`section_add_fit`]: the point must be
/// free ([`free_link_points`]) and the part mates it with its
/// [`natural_link_point`] at roll 0 ([`snap_placement`]).
///
/// The Ship pane's ghost stands here, so a refused add still shows where it
/// would go.
///
/// # Panics
///
/// When `section` is not in `design` or `point` is not one of its sockets.
pub fn section_add_pose(
    design: &[RefitPlacedSection<'_>],
    section: usize,
    point: usize,
    offered: &SectionConfig,
) -> Result<(Vec3, Quat), SectionAddFaultType> {
    if !free_link_points(design, section).contains(&point) {
        return Err(SectionAddFaultType::Occupied);
    }
    let target = &design[section];
    let (target_position, target_normal) = link_point_pose(
        target.position,
        target.rotation,
        &target.config.base.link_points[point],
    );
    let source = natural_link_point(&offered.base.link_points, target_normal)
        .ok_or(SectionAddFaultType::NoSockets)?;
    Ok(snap_placement(
        target_position,
        target_normal,
        &offered.base.link_points[source],
        0,
    ))
}

/// Where `offered` stands when added at socket `point` of design section
/// `section`, as `(position, rotation)` in ship-root space, or why it does
/// not fit.
///
/// `design` is every section of the ship's design, live or lost; `live`
/// holds the indices of the live ones. The part stands at
/// [`section_add_pose`]. Its collider may overlap only a section it
/// mates ([`section_colliders_overlap`], the rule the ship lint and the
/// editor share), checked against the whole design so a lost section can
/// still be rebuilt. Then the generator's post-build checks run on the live
/// ship with the part and without it, and only a new fault refuses: one
/// socket graph, which also refuses a socket with two suitors, clear exit
/// lanes ([`placement_blocks_an_exit`]), the thrust floor and the flight
/// computer count.
///
/// # Panics
///
/// When `section` is not in `design` or `point` is not one of its sockets.
pub fn section_add_fit(
    design: &[RefitPlacedSection<'_>],
    live: &[usize],
    section: usize,
    point: usize,
    offered: &SectionConfig,
) -> Result<(Vec3, Quat), SectionAddFaultType> {
    let (position, rotation) = section_add_pose(design, section, point, offered)?;
    let added = RefitPlacedSection {
        position,
        rotation,
        config: offered,
    };

    let near = neighbours(design, &added, None);
    let (mated, _) = subject_mates(design, &added, &near);
    let overlaps = near.iter().enumerate().any(|(slot, &index)| {
        !mated.contains(&(slot + 1))
            && section_colliders_overlap(collider(&design[index]), collider(&added))
    });
    if overlaps {
        return Err(SectionAddFaultType::Overlap);
    }

    let before: Vec<RefitPlacedSection<'_>> = live.iter().map(|&index| design[index]).collect();
    if let Some(fault) = add_build_fault(&before, added) {
        return Err(SectionAddFaultType::Build(fault));
    }
    Ok((position, rotation))
}

/// The first post-build check the live ship `before` passed and fails once
/// `added` joins it, or `None` when the add makes no fault: one socket graph,
/// clear exit lanes ([`placement_blocks_an_exit`]), the thrust floor and the
/// flight computer count, as [`section_add_fit`] reads them.
pub fn add_build_fault(
    before: &[RefitPlacedSection<'_>],
    added: RefitPlacedSection<'_>,
) -> Option<RefitBuildFaultType> {
    let mut after = before.to_vec();
    after.push(added);
    let blocks = || {
        let ship: Vec<PlacedPart<'_>> = before.iter().map(|s| s.placed_part()).collect();
        placement_blocks_an_exit(&ship, &added.placed_part())
    };
    refit_build_fault(before, &after, blocks)
}

/// The player binding a new section of `class` takes, from the bindings the
/// ship's live sections carry: `bound` lists `(section id, class, sources)`
/// for every live section with a binding component.
///
/// The section joins its kind: it takes the binding of the first live
/// section of its class, by id, that has one. With none, a weapon takes an
/// empty binding the Ship pane's Rebind sets, and a thruster takes none, so
/// the flight computer keeps driving it. A mining emitter takes the first of
/// [`ADDED_MINING_KEYS`] no other emitter holds, since one press must deploy
/// one emitter. A section that takes no input takes none.
///
/// # Errors
///
/// [`SectionAddRefusalType::NoFreeKey`] for a mining emitter when every
/// mining key is held.
pub fn added_section_binds<'a>(
    class: SectionClass,
    bound: impl IntoIterator<Item = (&'a str, SectionClass, &'a [InputSource])>,
) -> Result<Option<Vec<InputSource>>, SectionAddRefusalType> {
    let siblings: Vec<(&str, &[InputSource])> = bound
        .into_iter()
        .filter(|(_, other, _)| *other == class)
        .map(|(id, _, sources)| (id, sources))
        .collect();
    match class {
        SectionClass::Hull
        | SectionClass::Controller
        | SectionClass::Docking
        | SectionClass::CargoIntake => Ok(None),
        SectionClass::Mining => ADDED_MINING_KEYS
            .into_iter()
            .map(InputSource::from)
            .find(|key| !siblings.iter().any(|(_, sources)| sources.contains(key)))
            .map(|key| Some(vec![key]))
            .ok_or(SectionAddRefusalType::NoFreeKey),
        SectionClass::Thruster
        | SectionClass::Turret
        | SectionClass::Torpedo
        | SectionClass::Railgun => {
            let first = siblings.iter().min_by_key(|(id, _)| *id);
            Ok(match (first, class) {
                (Some((_, sources)), _) => Some(sources.to_vec()),
                (None, SectionClass::Thruster) => None,
                (None, _) => Some(Vec::new()),
            })
        }
    }
}

/// The design id a new section of `class` takes on `design`: the greatest id
/// of its class followed by `+`, or `added_<class>` when the design has no
/// section of that class, with more `+` until no design section holds it.
///
/// Section codes count a class by id ([`ShipDesignSections::class_index`]),
/// so an id that sorts after every id of its class gives the new section the
/// next code, and no live section's code moves.
pub fn added_section_id(design: &ShipDesignSections, class: SectionClass) -> String {
    let last = design
        .0
        .iter()
        .filter(|section| section.config.kind.class() == class)
        .map(|section| section.id.as_str())
        .max();
    let mut id = match last {
        Some(last) => format!("{last}+"),
        None => format!("added_{}", format!("{class:?}").to_lowercase()),
    };
    while design.get(&id).is_some() {
        id.push('+');
    }
    id
}

/// Why an add changes nothing, in check order.
#[derive(Clone, Debug, PartialEq)]
pub enum SectionAddRefusalType {
    /// The ship has no docked partner.
    NotDocked,
    /// The partner is not a living station that offers Refit.
    NoRefitService,
    /// The part does not fit at the point, see [`section_add_fit`].
    Fit(SectionAddFaultType),
    /// The part is a mining emitter and every mining key is held, see
    /// [`added_section_binds`].
    NoFreeKey,
    /// The part's recipe cannot be paid, see [`plan_recipe_payment`].
    Payment(RecipePaymentRefusalType),
}

/// What an allowed add does: the pose, the binding and the payment.
#[derive(Clone, Debug, PartialEq)]
pub struct SectionAddPlan {
    /// The mount position in ship-root space.
    pub position: Vec3,
    /// The mount rotation in ship-root space.
    pub rotation: Quat,
    /// The binding the new section takes, see [`added_section_binds`].
    pub bindings: Option<Vec<InputSource>>,
    /// The recipe payment.
    pub payment: RecipePayment,
}

/// Plan adding the catalog part `offered` while docked with `partner`, paid
/// by its full recipe.
///
/// `fit` is [`section_add_fit`] for the chosen point, computed by the caller
/// so a list can keep it while only the holds change. `bound` is the ship's
/// live bindings, as [`added_section_binds`] reads them. `partner` is `None`
/// while the ship is not docked; its `offers` is whether it is a living
/// station that offers Refit. The recipe is paid by [`plan_recipe_payment`]:
/// hold parts first, the rest bought at ask. Checks run in
/// [`SectionAddRefusalType`] order. A refusal changes nothing.
pub fn plan_section_add<'a>(
    items: &GameItems,
    offered: &SectionConfig,
    fit: Result<(Vec3, Quat), SectionAddFaultType>,
    bound: impl IntoIterator<Item = (&'a str, SectionClass, &'a [InputSource])>,
    partner: Option<StationPartner<'_>>,
    own: &ShipInventory,
    own_cr: u32,
) -> Result<SectionAddPlan, SectionAddRefusalType> {
    let partner = partner.ok_or(SectionAddRefusalType::NotDocked)?;
    if !partner.offers {
        return Err(SectionAddRefusalType::NoRefitService);
    }
    let (position, rotation) = fit.map_err(SectionAddRefusalType::Fit)?;
    let bindings = added_section_binds(offered.kind.class(), bound)?;
    let payment = plan_recipe_payment(
        items,
        &offered.base.build,
        partner.stock,
        partner.credits_cr,
        own,
        own_cr,
    )
    .map_err(SectionAddRefusalType::Payment)?;
    Ok(SectionAddPlan {
        position,
        rotation,
        bindings,
        payment,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn hull() -> SectionConfig {
        part(
            "hull",
            SectionKind::Hull(HullSectionConfig::default()),
            Vec3::ONE,
            unit_cube_link_points(),
        )
    }

    fn at(x: f32, y: f32, config: &SectionConfig) -> RefitPlacedSection<'_> {
        RefitPlacedSection {
            position: Vec3::new(x, y, 0.0),
            rotation: Quat::IDENTITY,
            config,
        }
    }

    fn socket(config: &SectionConfig, id: &str) -> usize {
        config
            .base
            .link_points
            .iter()
            .position(|point| point.id == id)
            .unwrap_or_else(|| panic!("no socket '{id}'"))
    }

    /// A free point is a socket no design section mates, and a lost section
    /// still counts: its socket stays reserved for the rebuild.
    #[test]
    fn a_free_point_is_a_socket_no_design_section_mates() {
        let hull = hull();
        let design = [
            at(0.0, 0.0, &hull),
            at(1.0, 0.0, &hull),
            at(-1.0, 0.0, &hull),
        ];
        let free: Vec<&str> = free_link_points(&design, 0)
            .into_iter()
            .map(|point| hull.base.link_points[point].id.as_str())
            .collect();
        assert_eq!(
            free,
            ["positive_y", "negative_y", "positive_z", "negative_z"]
        );
    }

    /// A cube added at a free face lands one cell out and mates it; the
    /// same add at a mated face is refused as occupied.
    #[test]
    fn a_part_fits_at_a_free_point_and_is_refused_at_an_occupied_one() {
        let hull = hull();
        let drive = part(
            "drive",
            SectionKind::Thruster(ThrusterSectionConfig {
                magnitude: 10.0,
                ..default()
            }),
            Vec3::ONE,
            unit_cube_link_points(),
        );
        let design = [at(0.0, 0.0, &hull), at(-1.0, 0.0, &drive)];
        let live = [0, 1];

        let (position, rotation) =
            section_add_fit(&design, &live, 0, socket(&hull, "positive_y"), &hull)
                .expect("a cube fits on the free top face");
        assert!(position.abs_diff_eq(Vec3::Y, 1e-5), "{position:?}");
        assert!(rotation.abs_diff_eq(Quat::IDENTITY, 1e-5), "{rotation:?}");

        assert_eq!(
            section_add_fit(&design, &live, 0, socket(&hull, "negative_x"), &hull),
            Err(SectionAddFaultType::Occupied)
        );
    }

    /// A part whose box would sink into a section it does not mate is
    /// refused, and a socketless part has nothing to mate with.
    #[test]
    fn an_add_is_refused_for_an_overlap_or_a_part_with_no_socket() {
        let hull = hull();
        // Half a cell off the target's +X face, with no socket to mate.
        let block = part(
            "block",
            SectionKind::Hull(HullSectionConfig::default()),
            Vec3::ONE,
            Vec::new(),
        );
        let design = [at(0.0, 0.0, &hull), at(1.5, 0.0, &block)];
        let live = [0, 1];
        assert_eq!(
            section_add_fit(&design, &live, 0, socket(&hull, "positive_x"), &hull),
            Err(SectionAddFaultType::Overlap)
        );
        assert_eq!(
            section_add_fit(&design, &live, 0, socket(&hull, "positive_y"), &block),
            Err(SectionAddFaultType::NoSockets)
        );
    }

    /// The generator's floors judge the ship the add leaves: one more cell of
    /// mass drops a ship under its thrust floor and is refused.
    #[test]
    fn an_add_that_drops_the_ship_under_its_thrust_floor_is_refused() {
        let hull = hull();
        // Two cells of mass need 0.12; three need 0.18.
        let drive = part(
            "drive",
            SectionKind::Thruster(ThrusterSectionConfig {
                magnitude: 0.13,
                ..default()
            }),
            Vec3::ONE,
            unit_cube_link_points(),
        );
        let design = [at(0.0, 0.0, &hull), at(-1.0, 0.0, &drive)];
        assert!(matches!(
            section_add_fit(&design, &[0, 1], 0, socket(&hull, "positive_y"), &hull),
            Err(SectionAddFaultType::Build(
                RefitBuildFaultType::Underpowered { .. }
            ))
        ));
    }

    /// A new section joins its kind's binding; with no bound sibling a gun
    /// takes an empty binding and a drive none; an emitter takes the first
    /// free mining key and is refused when all are held.
    #[test]
    fn a_new_section_takes_its_kinds_binding_or_the_next_free_mining_key() {
        let lmb = [InputSource::Mouse(MouseButton::Left)];
        let space = [InputSource::Keyboard(KeyCode::Space)];
        let bound = [
            ("pdc_b", SectionClass::Turret, &lmb[..]),
            ("pdc_a", SectionClass::Turret, &space[..]),
        ];
        assert_eq!(
            added_section_binds(SectionClass::Turret, bound),
            Ok(Some(space.to_vec()))
        );
        assert_eq!(
            added_section_binds(SectionClass::Torpedo, bound),
            Ok(Some(vec![]))
        );
        assert_eq!(added_section_binds(SectionClass::Thruster, bound), Ok(None));
        assert_eq!(added_section_binds(SectionClass::Hull, bound), Ok(None));

        let v = [InputSource::Keyboard(KeyCode::KeyV)];
        assert_eq!(
            added_section_binds(
                SectionClass::Mining,
                [("beam", SectionClass::Mining, &v[..])]
            ),
            Ok(Some(vec![InputSource::Keyboard(KeyCode::KeyB)]))
        );
        let all: Vec<[InputSource; 1]> = ADDED_MINING_KEYS
            .into_iter()
            .map(|key| [InputSource::from(key)])
            .collect();
        let held = all
            .iter()
            .map(|sources| ("beam", SectionClass::Mining, &sources[..]));
        assert_eq!(
            added_section_binds(SectionClass::Mining, held),
            Err(SectionAddRefusalType::NoFreeKey)
        );
    }

    /// The new id sorts after every id of its class, so it takes the next
    /// code and no other section's code moves.
    #[test]
    fn an_added_section_takes_the_next_code_of_its_class() {
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
        let mut design = ShipDesignSections(vec![
            section("pdc_starboard", turret()),
            section(
                "added_turret",
                SectionKind::Hull(HullSectionConfig::default()),
            ),
            section("pdc_port", turret()),
        ]);

        let id = added_section_id(&design, SectionClass::Turret);
        assert_eq!(id, "pdc_starboard+");
        design.0.push(section(&id, turret()));
        assert_eq!(design.class_index(&id), Some((SectionClass::Turret, 3)));
        assert_eq!(
            design.class_index("pdc_starboard"),
            Some((SectionClass::Turret, 2))
        );

        assert_eq!(
            added_section_id(&design, SectionClass::Railgun),
            "added_railgun"
        );
        assert_eq!(
            added_section_id(
                &ShipDesignSections(vec![section("added_turret", SectionKind::Hull(default()))]),
                SectionClass::Turret
            ),
            "added_turret+"
        );
    }
}
