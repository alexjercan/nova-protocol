//! The destruction pipeline over the [`components`](super::components) graph:
//! disable at zero health, destroy leaves, prune, cascade - plus the collision
//! that starts it.
//!
//! The lifecycle, all observer-driven:
//!
//! - a fast contact between two rigid bodies deals kinetic damage from the
//!   impulse the solver settled on (`deal_contact_impact_damage`);
//! - a node whose health hits zero is disabled;
//! - structure adapters may destroy it immediately (ships do); otherwise a
//!   disabled LEAF - or a disabled [`IntegrityRoot`] - is destroyed, which is
//!   the [`IntegrityDestroyMarker`] seam nova's [`explode`](super::explode)
//!   reacts to;
//! - destroying a node prunes it from its neighbours' [`ConnectedTo`] lists,
//!   re-deriving leaf markers and cascading through the structure.
//!
//! Impact damage goes through [`apply_damage`], the one point at which any
//! weapon enters the health store. Blast damage is the
//! [`NovaBlast`](crate::damage::NovaBlast) volume's job and lives in
//! [`crate::damage`].

use std::collections::BTreeMap;

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_events::{prelude::EntityTypeName, units::prelude::*};

use super::{components::prelude::*, health::prelude::*};
use crate::damage::prelude::{apply_damage, DamageType};

/// `IntegrityCorePlugin` and `IntegritySystems`.
pub mod prelude {
    pub use super::{IntegrityCorePlugin, IntegritySystems};
}

/// Feel constants for ramming, not physical ones. The restitution coefficient
/// splits an impact between the impulse term and the energy the collision
/// absorbs (0.5 is half-elastic, so both contribute); the two modifiers convert
/// those into hit points.
///
/// These are the reference numbers [`representative_kinetic_damage`](crate::damage::representative_kinetic_damage)
/// authors turret damage against, so a slug at a typical engagement speed hurts
/// about as much as it did when damage was purely emergent. Changing one means
/// re-authoring the other.
const RESTITUTION_COEFFICIENT: f32 = 0.5;
const IMPULSE_DAMAGE_MODIFIER: f32 = 0.1;
const ENERGY_DAMAGE_MODIFIER: f32 = 0.05;

/// The closing speed at a contact under which a touch is FREE, whatever is
/// touching: two hulls may dock, settle, scrape and rest against each other
/// without trading a hit point.
///
/// Universal and stated in meters per second, because it is a statement about
/// what a ship is built to survive rather than about any one hull: a docking
/// clamp takes a 5 m/s arrival, and a mass floor would make the same arrival
/// safe for a skiff and lethal for a carrier.
///
/// Only the EXCESS over this is a ram. A contact at 6 m/s is not "a 6 m/s ram
/// minus a little", it is a 1 m/s one, so the curve leaves the safe speed
/// smoothly instead of stepping over it.
const SAFE_CONTACT_SPEED: MetersPerSecond = MetersPerSecond(5.0);

/// The share of the health a section was BUILT with that its own structure
/// soaks before an impact's ENERGY term does anything at all.
///
/// Per section rather than flat, because a reinforced hull plate is built to
/// take what a light one is not, and "built to take" is exactly what its
/// maximum health says. Read it as the rule it is: an impact that would cost a
/// section less than one percent of itself did not happen to it.
///
/// The IMPULSE term is not absorbed. A hard shove is transmitted through
/// whatever is in the way whether or not the metal survives it.
const ABSORBED_HEALTH_FRACTION: f32 = 0.01;

/// System set for the leaf derivation, so gameplay can order graph edits around
/// it. `glue` builds the section graph inside it; `neutralize` reads the result
/// after it.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct IntegritySystems;

/// The generic destruction core: collision damage in, [`IntegrityDestroyMarker`]
/// out. Requires [`NovaHealthPlugin`] and avian's `PhysicsPlugins`.
pub struct IntegrityCorePlugin;

impl Plugin for IntegrityCorePlugin {
    fn build(&self, app: &mut App) {
        trace!("IntegrityCorePlugin: build");

        app.register_type::<IntegrityRoot>();
        app.register_type::<ConnectedTo>();
        app.register_type::<IntegrityLeafMarker>();
        app.register_type::<IntegrityDisabledMarker>();
        app.register_type::<IntegrityDestroyMarker>();

        app.init_resource::<ImpactTally>();
        app.init_resource::<DestructionTally>();

        app.add_observer(on_collider_of_spawn_insert_collision_events);
        app.add_observer(on_health_depleted_insert_disabled);
        app.add_observer(tally_a_destroyed_node);
        app.add_observer(destroy_a_disabled_leaf);
        app.add_observer(destroy_a_disabled_node_that_became_a_leaf);
        app.add_observer(destroy_the_structure_of_a_disabled_root);
        app.add_observer(prune_a_destroyed_node_from_its_neighbours);

        app.add_systems(Update, derive_integrity_leaves.in_set(IntegritySystems));
        // AFTER the physics step, like `advance_rounds`: a contact's impulse
        // does not exist until the solver has settled it.
        app.add_systems(
            FixedPostUpdate,
            deal_contact_impact_damage.after(PhysicsSystems::Last),
        );
        app.add_systems(Last, (report_impact_tally, report_destruction_tally));
    }
}

/// A frame's worth of ram damage, reported as one line.
///
/// WHY THE PER-CONTACT LINE CANNOT BE `debug!`: `--features debug` puts the
/// nova crates at DEBUG by default (`nova_core::log_filter_str`), and a salvo
/// against a block ship lands HUNDREDS of contacts per frame - every plate of
/// every section that a burst touches. A line each is not a log, it is a
/// denial of service against the terminal, and it hides the events a person
/// opened the log for. The per-contact detail is still there at `trace!`, one
/// module at a time (`RUST_LOG=nova_gameplay::integrity=trace`).
#[derive(Resource, Default)]
struct ImpactTally {
    contacts: usize,
    damage: f32,
    worst: f32,
    worst_at: Option<Entity>,
}

/// A frame's worth of destruction, reported as one line, for the same reason
/// [`ImpactTally`] exists: a structural collapse peels a hundred sections and
/// each one used to cost three or four lines across this module and
/// [`explode`](super::explode).
///
/// Counted by type name rather than by entity, because that is the figure a
/// person reading a death is after - "107 reinforced hull sections" says what
/// died; a hundred and seven entity ids do not.
#[derive(Resource, Default)]
struct DestructionTally {
    nodes: usize,
    kinds: BTreeMap<String, usize>,
}

/// Count a destroyed node for the frame summary.
fn tally_a_destroyed_node(
    add: On<Add, IntegrityDestroyMarker>,
    mut tally: ResMut<DestructionTally>,
    q_destroyed: Query<Option<&EntityTypeName>, With<IntegrityDestroyMarker>>,
) {
    let Ok(type_name) = q_destroyed.get(add.entity) else {
        return;
    };

    tally.nodes += 1;
    let kind = type_name.map_or_else(|| "unnamed".to_string(), |name| name.0.clone());
    *tally.kinds.entry(kind).or_default() += 1;
}

/// `"3 nodes"`, `"1 node"`. A tally line reads as a sentence or it is noise of
/// a quieter kind.
fn plural(count: usize, singular: &str) -> String {
    if count == 1 {
        singular.to_string()
    } else {
        format!("{singular}s")
    }
}

fn report_impact_tally(mut tally: ResMut<ImpactTally>) {
    if tally.contacts == 0 {
        return;
    }

    debug!(
        "impacts: {} {} for {:.2} damage (worst {:.2} on {:?})",
        tally.contacts,
        plural(tally.contacts, "contact"),
        tally.damage,
        tally.worst,
        tally.worst_at
    );
    *tally = ImpactTally::default();
}

fn report_destruction_tally(mut tally: ResMut<DestructionTally>) {
    if tally.nodes == 0 {
        return;
    }

    let kinds = tally
        .kinds
        .iter()
        .map(|(kind, count)| format!("{kind} x{count}"))
        .collect::<Vec<_>>()
        .join(", ");
    debug!(
        "integrity: destroyed {} {} ({kinds})",
        tally.nodes,
        plural(tally.nodes, "node")
    );
    *tally = DestructionTally::default();
}

/// Opt a health-bearing collider into collision events, so impacts against it
/// are reported at all.
fn on_collider_of_spawn_insert_collision_events(
    add: On<Add, ColliderOf>,
    mut commands: Commands,
    q_collider: Query<Entity, (With<ColliderOf>, With<Health>)>,
) {
    let entity = add.entity;
    if q_collider.get(entity).is_err() {
        return;
    }

    trace!(
        "on_collider_of_spawn: enabling collision events on {:?}",
        entity
    );
    commands.entity(entity).insert(CollisionEventsEnabled);
}

/// Damage both sides of every touching contact from the impulse the SOLVER
/// exchanged there.
///
/// Runs after the physics step, in [`FixedPostUpdate`], because the numbers it
/// reads only exist once the solver has run: a contact point carries both the
/// approach speed avian measured before the solve and the normal impulse it
/// settled on. Nothing here recomputes either.
///
/// WHY NOT THE `CollisionStart` OBSERVER: two hulls do not meet at one point.
/// A carrier pair touches on hundreds of collider pairs in a frame, and the
/// observer charged every one of them the whole ship-to-ship effective mass -
/// so two carriers closing at docking speed traded tens of hit points per
/// contact, hundreds of times, and shredded each other standing still. The
/// solver already divides ONE impact across the contacts that carry it, which
/// is the figure this wants; summing its impulses is the whole ram.
///
/// A sensor is solved by nothing and exchanges no impulse, so a blast volume
/// or a passing round deals nothing here without needing to be excluded - its
/// damage is its own system's job.
///
/// Damage enters as KINETIC, which is what a ram is: solid meeting solid, and
/// the one class whose debris is chips off the surface.
fn deal_contact_impact_damage(
    mut commands: Commands,
    mut tally: ResMut<ImpactTally>,
    collisions: Collisions,
    q_collider: Query<(Option<&Health>, &GlobalTransform), With<ColliderOf>>,
) {
    for pair in collisions.iter() {
        if pair.body1.is_none() || pair.body2.is_none() {
            continue;
        }
        let Ok((health1, where1)) = q_collider.get(pair.collider1) else {
            continue;
        };
        let Ok((health2, where2)) = q_collider.get(pair.collider2) else {
            continue;
        };
        // One pass over the manifolds for both sides: the impulse and the
        // energy are the contact's, and only the absorption is the section's.
        let (impulse, energy) = pair
            .manifolds
            .iter()
            .flat_map(|manifold| manifold.points.iter())
            .map(|point| contact_bite(point.normal_impulse, -point.normal_speed))
            .fold((0.0, 0.0), |(impulse, energy), (this, that)| {
                (impulse + this, energy + that)
            });
        if impulse <= f32::EPSILON {
            continue;
        }

        for (target, source, health, at) in [
            (pair.collider1, pair.collider2, health1, where2),
            (pair.collider2, pair.collider1, health2, where1),
        ] {
            let absorbed = health.map_or(0.0, |health| absorbed_energy(health.max));
            let amount = impact_damage(impulse, energy, absorbed);
            if amount <= f32::EPSILON {
                continue;
            }

            trace!(
                "contact impact: collider {target:?} rammed by {source:?} for {amount:.2} \
                 (impulse {impulse:.2}, energy {energy:.2}, absorbed {absorbed:.2})"
            );
            tally.contacts += 1;
            tally.damage += amount;
            if amount > tally.worst {
                tally.worst = amount;
                tally.worst_at = Some(target);
            }
            // Where the OTHER collider is, which for a contact is where it hit
            // to within the half-cell the carve is quantized to anyway.
            apply_damage(
                &mut commands,
                target,
                Some(source),
                amount,
                DamageType::Kinetic,
                Some(at.translation()),
            );
        }
    }
}

/// The unsafe share of one contact point: the part of the solved normal
/// impulse that belongs to the approach ABOVE [`SAFE_CONTACT_SPEED`], and the
/// energy that share dissipates.
///
/// For a head-on collision the normal impulse is `m (1 + e) v`, so the energy
/// it dissipates, `0.5 m (1 - e^2) v^2`, is `0.5 (1 - e) J v` with no mass term
/// left in it at all. That is why the solver's impulse is enough: the pair's
/// effective mass is already inside the number avian handed back, and nothing
/// here has to guess at it per collider.
pub fn contact_bite(normal_impulse: f32, approach_speed: f32) -> (f32, f32) {
    let excess = approach_speed - SAFE_CONTACT_SPEED.to_engine();
    if excess <= 0.0 || normal_impulse <= 0.0 {
        return (0.0, 0.0);
    }
    let impulse = normal_impulse * (excess / approach_speed);
    (
        impulse,
        0.5 * (1.0 - RESTITUTION_COEFFICIENT) * impulse * excess,
    )
}

/// The normal impulse a head-on collision exchanges, from the effective mass of
/// the pair.
///
/// The closed form of what the solver settles on for a real contact, for the
/// one caller that has no solver to ask: the turret's damage authoring, which
/// prices a round before any of it exists.
pub fn contact_impulse(effective_mass: f32, closing_speed: f32) -> f32 {
    effective_mass * (1.0 + RESTITUTION_COEFFICIENT) * closing_speed
}

/// The energy one section soaks before the energy term bites, from the health
/// it was BUILT with. See [`ABSORBED_HEALTH_FRACTION`].
fn absorbed_energy(max_health: f32) -> f32 {
    ABSORBED_HEALTH_FRACTION * max_health / ENERGY_DAMAGE_MODIFIER
}

/// Hit points one side of a contact takes: the impulse term, which nothing
/// absorbs, plus whatever is left of the energy term after the section's own
/// structure has soaked what it is built to soak. Pure, so the turret's damage
/// authoring can call the same formula.
pub fn impact_damage(impulse: f32, energy: f32, absorbed_energy: f32) -> f32 {
    impulse * IMPULSE_DAMAGE_MODIFIER + (energy - absorbed_energy).max(0.0) * ENERGY_DAMAGE_MODIFIER
}

/// Disable a node the moment its health reaches zero.
fn on_health_depleted_insert_disabled(add: On<Add, HealthZeroMarker>, mut commands: Commands) {
    trace!("integrity: entity {:?} depleted, disabling", add.entity);
    commands.entity(add.entity).insert(IntegrityDisabledMarker);
}

/// Destroy a node that is disabled while ALREADY a leaf.
fn destroy_a_disabled_leaf(
    add: On<Add, IntegrityDisabledMarker>,
    mut commands: Commands,
    q_disabled: Query<(), (With<IntegrityDisabledMarker>, With<IntegrityLeafMarker>)>,
) {
    if q_disabled.get(add.entity).is_err() {
        return;
    }

    trace!("integrity: disabled leaf {:?} destroyed", add.entity);
    commands.entity(add.entity).insert(IntegrityDestroyMarker);
}

/// Destroy an already-disabled node that has just BECOME a leaf - the chain
/// reaction.
fn destroy_a_disabled_node_that_became_a_leaf(
    add: On<Add, IntegrityLeafMarker>,
    mut commands: Commands,
    q_destroyed: Query<(), (With<IntegrityDisabledMarker>, With<IntegrityLeafMarker>)>,
) {
    if q_destroyed.get(add.entity).is_err() {
        return;
    }

    trace!(
        "integrity: {:?} became a disabled leaf, cascading",
        add.entity
    );
    commands.entity(add.entity).insert(IntegrityDestroyMarker);
}

/// Destroy a disabled [`IntegrityRoot`] outright: the whole structure dies with
/// its root, leaf or not.
fn destroy_the_structure_of_a_disabled_root(
    add: On<Add, IntegrityDisabledMarker>,
    mut commands: Commands,
    q_destroyed: Query<(), (With<IntegrityDisabledMarker>, With<IntegrityRoot>)>,
) {
    if q_destroyed.get(add.entity).is_err() {
        return;
    }

    trace!("integrity: disabled root {:?} destroyed", add.entity);
    commands.entity(add.entity).insert(IntegrityDestroyMarker);
}

/// Prune a destroyed node from its neighbours' [`ConnectedTo`] lists. Mutating a
/// neighbour's list marks it `Changed`, so [`derive_integrity_leaves`]
/// re-evaluates whether it has become a leaf - which, if it is also disabled,
/// drives the chain reaction.
///
/// The disjoint `With`/`Without` filters keep the two `ConnectedTo` accesses
/// sound. A neighbour destroyed in the same frame is skipped, which is harmless:
/// it is going away anyway.
fn prune_a_destroyed_node_from_its_neighbours(
    add: On<Add, IntegrityDestroyMarker>,
    q_destroyed: Query<&ConnectedTo, With<IntegrityDestroyMarker>>,
    mut q_neighbors: Query<&mut ConnectedTo, Without<IntegrityDestroyMarker>>,
) {
    let entity = add.entity;
    let Ok(connected) = q_destroyed.get(entity) else {
        return;
    };

    for neighbor in connected.0.clone() {
        if let Ok(mut neighbor_connections) = q_neighbors.get_mut(neighbor) {
            neighbor_connections.retain(|&node| node != entity);
        }
    }
}

/// Re-derive leaf markers whenever a node's [`ConnectedTo`] changes - on the
/// initial build, or when a neighbour is pruned. One or zero neighbours is a
/// leaf.
fn derive_integrity_leaves(
    mut commands: Commands,
    q_nodes: Query<(Entity, &ConnectedTo), Changed<ConnectedTo>>,
) {
    for (entity, connected) in &q_nodes {
        if connected.len() <= 1 {
            commands.entity(entity).try_insert(IntegrityLeafMarker);
        } else {
            commands.entity(entity).try_remove::<IntegrityLeafMarker>();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The avian-free core plus the health store, so tests drive destruction
    /// from real damage without stepping physics.
    fn integrity_core_app() -> App {
        let mut app = App::new();
        app.add_plugins(NovaHealthPlugin);
        app.add_observer(on_health_depleted_insert_disabled);
        app.add_observer(destroy_a_disabled_leaf);
        app.add_observer(destroy_a_disabled_node_that_became_a_leaf);
        app.add_observer(destroy_the_structure_of_a_disabled_root);
        app.add_observer(prune_a_destroyed_node_from_its_neighbours);
        app.add_systems(Update, derive_integrity_leaves);
        app
    }

    /// A lone node has no neighbours, so it is a leaf from the first update and
    /// dies the moment its health runs out.
    #[test]
    fn a_lone_node_is_a_leaf_and_dies_when_depleted() {
        let mut app = integrity_core_app();
        let node = app
            .world_mut()
            .spawn((Health::new(10.0), ConnectedTo::default()))
            .id();
        app.update();
        assert!(app.world().get::<IntegrityLeafMarker>(node).is_some());

        app.world_mut().trigger(HealthApplyDamage {
            entity: node,
            source: None,
            amount: 10.0,
        });
        app.world_mut().flush();

        assert!(app.world().get::<IntegrityDisabledMarker>(node).is_some());
        assert!(app.world().get::<IntegrityDestroyMarker>(node).is_some());
    }

    /// Leafness is DERIVED, not latched: a node that gains a second neighbour
    /// loses the marker again. Without the removal branch a node welded into
    /// the middle of a structure would stay destroyable as a leaf.
    #[test]
    fn a_leaf_that_gains_a_second_neighbour_stops_being_one() {
        let mut app = integrity_core_app();
        let world = app.world_mut();
        let hub = world.spawn(ConnectedTo(vec![])).id();
        let first = world.spawn(ConnectedTo(vec![hub])).id();
        world.entity_mut(hub).insert(ConnectedTo(vec![first]));
        app.update();
        assert!(
            app.world().get::<IntegrityLeafMarker>(hub).is_some(),
            "one neighbour is a leaf"
        );

        let second = app.world_mut().spawn(ConnectedTo(vec![hub])).id();
        app.world_mut()
            .entity_mut(hub)
            .insert(ConnectedTo(vec![first, second]));
        app.update();

        assert!(
            app.world().get::<IntegrityLeafMarker>(hub).is_none(),
            "two neighbours is not a leaf any more"
        );
    }

    /// The generic core leaves an interior node disabled until it becomes a
    /// leaf. Structure adapters can choose immediate destruction instead.
    #[test]
    fn a_depleted_interior_node_is_disabled_but_survives() {
        let mut app = integrity_core_app();
        let world = app.world_mut();
        let left = world
            .spawn((Health::new(10.0), ConnectedTo::default()))
            .id();
        let middle = world
            .spawn((Health::new(10.0), ConnectedTo::default()))
            .id();
        let right = world
            .spawn((Health::new(10.0), ConnectedTo::default()))
            .id();
        world.entity_mut(left).insert(ConnectedTo(vec![middle]));
        world
            .entity_mut(middle)
            .insert(ConnectedTo(vec![left, right]));
        world.entity_mut(right).insert(ConnectedTo(vec![middle]));
        app.update();

        assert!(
            app.world().get::<IntegrityLeafMarker>(middle).is_none(),
            "two neighbours is not a leaf"
        );

        app.world_mut().trigger(HealthApplyDamage {
            entity: middle,
            source: None,
            amount: 10.0,
        });
        app.world_mut().flush();

        assert!(app.world().get::<IntegrityDisabledMarker>(middle).is_some());
        assert!(
            app.world().get::<IntegrityDestroyMarker>(middle).is_none(),
            "an interior node is deactivated, not destroyed"
        );
    }

    /// Destroying a leaf prunes it from its neighbour, which turns the
    /// neighbour into a leaf - and if that neighbour is already disabled, it
    /// dies too. This is the chain reaction.
    #[test]
    fn destroying_a_leaf_cascades_into_a_disabled_neighbour() {
        let mut app = integrity_core_app();
        let world = app.world_mut();
        let outer = world
            .spawn((Health::new(10.0), ConnectedTo::default()))
            .id();
        let inner = world
            .spawn((Health::new(10.0), ConnectedTo::default()))
            .id();
        let anchor = world
            .spawn((Health::new(10.0), ConnectedTo::default()))
            .id();
        world.entity_mut(outer).insert(ConnectedTo(vec![inner]));
        world
            .entity_mut(inner)
            .insert(ConnectedTo(vec![outer, anchor]));
        world.entity_mut(anchor).insert(ConnectedTo(vec![inner]));
        app.update();

        // Kill the interior node first: disabled, but it holds.
        app.world_mut().trigger(HealthApplyDamage {
            entity: inner,
            source: None,
            amount: 10.0,
        });
        app.world_mut().flush();
        assert!(app.world().get::<IntegrityDestroyMarker>(inner).is_none());

        // Now kill the leaf outside it. The prune leaves `inner` with one
        // neighbour, so it becomes a leaf - and it is already disabled.
        app.world_mut().trigger(HealthApplyDamage {
            entity: outer,
            source: None,
            amount: 10.0,
        });
        app.world_mut().flush();
        app.update();

        assert!(app.world().get::<IntegrityDestroyMarker>(outer).is_some());
        assert!(
            app.world().get::<IntegrityDestroyMarker>(inner).is_some(),
            "pruning the outer leaf made the disabled interior node a leaf, so it cascaded"
        );
    }

    /// A disabled root takes the whole structure with it, leaf or not.
    #[test]
    fn a_disabled_root_is_destroyed_even_with_neighbours() {
        let mut app = integrity_core_app();
        let root = app
            .world_mut()
            .spawn((Health::new(10.0), IntegrityRoot))
            .id();
        app.update();

        app.world_mut().trigger(HealthApplyDamage {
            entity: root,
            source: None,
            amount: 10.0,
        });
        app.world_mut().flush();

        assert!(app.world().get::<IntegrityDestroyMarker>(root).is_some());
    }

    /// One contact's bite, from the two figures the solver hands back.
    fn bite(effective_mass: f32, speed: f32, absorbed: f32) -> f32 {
        let (impulse, energy) = contact_bite(contact_impulse(effective_mass, speed), speed);
        impact_damage(impulse, energy, absorbed)
    }

    /// The ram formula is monotone in both inputs and zero at rest - the
    /// property the turret's damage authoring leans on.
    #[test]
    fn impact_damage_grows_with_mass_and_speed_and_is_zero_at_rest() {
        assert_eq!(bite(100.0, 0.0, 0.0), 0.0);
        assert!(bite(100.0, 20.0, 0.0) > bite(100.0, 10.0, 0.0));
        assert!(bite(200.0, 10.0, 0.0) > bite(100.0, 10.0, 0.0));
    }

    /// A touch under the safe contact speed is free at any mass, which is what
    /// lets two hulls dock, settle and rest against each other. The old speed
    /// floor was 3.16 m/s and mass-blind in the other direction: damage was
    /// linear in mass, so a carrier pair drifting together traded tens of hit
    /// points on every one of hundreds of contacts.
    #[test]
    fn a_contact_under_the_safe_speed_is_free_however_heavy_it_is() {
        let safe = SAFE_CONTACT_SPEED.to_engine();
        for mass in [1.0_f32, 25.0, 2360.0, 100_000.0] {
            assert_eq!(bite(mass, safe, 0.0), 0.0, "{mass} kg at the safe speed");
            assert_eq!(bite(mass, safe * 0.64, 0.0), 0.0, "{mass} kg docking");
        }
        assert!(
            bite(2360.0, safe * 1.5, 0.0) > 0.0,
            "and half again over it is a ram"
        );
    }

    /// The curve leaves the safe speed smoothly: just over it is a small bite,
    /// not the whole speed's worth.
    #[test]
    fn the_bite_is_taken_on_the_excess_not_the_whole_approach() {
        let safe = SAFE_CONTACT_SPEED.to_engine();
        let (impulse, _) = contact_bite(contact_impulse(100.0, safe * 1.01), safe * 1.01);
        let (whole, _) = contact_bite(contact_impulse(100.0, safe * 1.01), f32::MAX);
        assert!(
            impulse < whole * 0.02,
            "a 1 percent overspeed spends 1 percent of the impulse, got {impulse} of {whole}"
        );
    }

    /// A section soaks energy in proportion to what it was BUILT to take, so
    /// the same scrape costs a light plate and shrugs off a reinforced one -
    /// and the impulse term goes through both.
    #[test]
    fn a_tougher_section_soaks_more_of_the_same_energy() {
        let speed = SAFE_CONTACT_SPEED.to_engine() * 3.0;
        let light = bite(50.0, speed, absorbed_energy(60.0));
        let reinforced = bite(50.0, speed, absorbed_energy(200.0));
        assert!(
            reinforced < light,
            "the reinforced plate takes less: {reinforced} against {light}"
        );
        assert!(
            reinforced > 0.0,
            "but the impulse still lands: {reinforced}"
        );
    }
}

#[cfg(test)]
mod physics_tests {
    use super::*;
    use crate::test_support::{integrity_physics_app, settle};

    /// A ship-shaped body: a rigid root with one child collider that carries
    /// the hit points, which is where a section's health lives.
    fn spawn_hull(app: &mut App, at: Vec3, hp: f32) -> Entity {
        let body = app
            .world_mut()
            .spawn((RigidBody::Dynamic, Transform::from_translation(at)))
            .id();
        app.world_mut()
            .spawn((
                ChildOf(body),
                Collider::sphere(1.0),
                ColliderDensity(1.0),
                Health::new(hp),
            ))
            .id()
    }

    fn hurt(app: &App, collider: Entity) -> f32 {
        let health = app.world().get::<Health>(collider).expect("alive");
        health.max - health.current
    }

    fn push(app: &mut App, collider: Entity, velocity: Vec3) {
        let body = app.world().get::<ColliderOf>(collider).unwrap().body;
        app.world_mut().get_mut::<LinearVelocity>(body).unwrap().0 = velocity;
    }

    /// Two identical hulls a fifth of a unit apart, nose to nose, closing at
    /// `closing` world units per second. Health is far from zero on both, so
    /// the destroy pipeline stays out of the measurement. Returns what each
    /// side lost after `ticks`.
    fn head_on(closing: f32, ticks: usize) -> (f32, f32) {
        let mut app = integrity_physics_app();
        let left = spawn_hull(&mut app, Vec3::new(-1.1, 0.0, 0.0), 1.0e6);
        let right = spawn_hull(&mut app, Vec3::new(1.1, 0.0, 0.0), 1.0e6);
        settle(&mut app);
        push(&mut app, left, Vec3::X * closing * 0.5);
        push(&mut app, right, Vec3::NEG_X * closing * 0.5);
        for _ in 0..ticks {
            app.update();
        }
        (hurt(&app, left), hurt(&app, right))
    }

    /// The whole point of the safe contact speed: two hulls may come together,
    /// touch and rest against each other without trading a hit point, however
    /// many contact frames that takes.
    #[test]
    fn two_hulls_touching_under_the_safe_speed_trade_nothing() {
        let docking = SAFE_CONTACT_SPEED.to_engine() * 0.8;
        let (left, right) = head_on(docking, 240);
        assert_eq!(
            (left, right),
            (0.0, 0.0),
            "a docking touch cost hit points: {left} / {right}"
        );
    }

    /// A real ram spends hit points on BOTH bodies - a contact has two sides -
    /// and spends more of them the harder it is.
    #[test]
    fn a_ram_spends_hit_points_on_both_sides_and_more_of_them_the_faster_it_is() {
        let safe = SAFE_CONTACT_SPEED.to_engine();
        let (left, right) = head_on(safe * 8.0, 8);
        assert!(left > 0.0 && right > 0.0, "one-sided ram: {left} / {right}");
        assert!(
            (left - right).abs() < left * 1.0e-3,
            "identical hulls must split a ram evenly: {left} / {right}"
        );
        let (harder, _) = head_on(safe * 16.0, 8);
        assert!(
            harder > left * 2.0,
            "twice the closing speed must cost more than twice: {harder} against {left}"
        );
    }
}
