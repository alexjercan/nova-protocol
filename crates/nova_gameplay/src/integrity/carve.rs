//! WHERE a body was hit, kept as spheres of material taken out of it.
//!
//! The other half of the damage picture. [`DamageLevel`](super::erosion) says
//! how far gone a body is and every effect that grades a whole body - cracks,
//! sparks - reads that. This says where the hits LANDED, and it is what any
//! effect that has to change a body's SHAPE reads instead.
//!
//! # Why a level could never carve
//!
//! A level is one number for a whole body, so the only geometry it can drive is
//! geometry that changes everywhere at once. Applied to a clad hull that comes
//! out as every plate sagging by the same proportion: the hull keeps its
//! outline and loses its relief, which reads as a smaller, plainer ship rather
//! than a damaged one. Damage has to ADD detail - a rim, a bite, a hole - and
//! nothing driven by a single scalar can put that detail anywhere in
//! particular.
//!
//! So a hit records a MARK: a sphere, centred where the hit landed, sized by
//! what the hit cost. Geometry is then whatever is left of the body once every
//! mark has been subtracted from it, which is the one operation that reads as
//! broken instead of worn.
//!
//! # The frame, and why marks are stored and not derived
//!
//! Marks live in the LOCAL frame of the body carrying them - a ship root, an
//! asteroid - so they ride along with it and a reload restores the same
//! damage. Nothing else records them, because nothing else could: a health
//! pool remembers how much was spent and cannot remember where.
//!
//! A hit is recorded on the nearest ancestor that carries [`DamageMarks`],
//! never on whatever collider it happened to meet. That is what makes a carve
//! CONTINUOUS: a ship's plates each derive their own geometry from the same
//! list, so two plates sharing a boundary sample compute the same depression at
//! it and the crater crosses the seam between them instead of stopping at it.
//!
//! # What a hit may take
//!
//! A mark is priced by the damage the hit actually SPENT, never by the damage
//! it asked for - see [`absorbed_by`]. The two differ every time a round
//! overkills what it hits, and the difference compounds: a slug that crosses a
//! plate is charged for the plate and then charged again, in full, for the hull
//! behind it. One round, priced twice.
//!
//! The same defect wearing a different hat is a blast, which asks for its
//! pressure once per collider it overlaps. A ship built out of hundreds of
//! them would grow one crater hundreds of times and throw the debris to match -
//! see [`record_blast_marks`], which sums a blast's contributions per body and
//! cuts them as one.

use bevy::prelude::*;

use super::health::prelude::{Health, HealthZeroMarker};

/// `CarveSpew`, `DamageMark`, `DamageMarks`, `mark_radius`,
/// `record_damage_mark` and `record_blast_marks`.
pub mod prelude {
    pub use super::{
        mark_radius, record_blast_marks, record_damage_mark, CarveSpew, DamageMark, DamageMarks,
        DAMAGE_PER_UNIT_VOLUME,
    };
}

/// Hit points a hit has to spend to take one cubic unit of material off.
///
/// Deliberately the toughness a ship's cladding is built at, so the two agree:
/// a hit big enough to kill one cell of skin is a hit that carves about one
/// cell of skin, and a plate never survives a mark that has already swallowed
/// it or vanishes under one that barely scratched it.
///
/// PUBLIC because a body tougher than cladding has to say so. A mark is one
/// sphere shared by everything it reaches, so it cannot be priced per body -
/// what a tougher body does instead is take a smaller bite out of the same
/// sphere, in proportion to `DAMAGE_PER_UNIT_VOLUME / its own toughness`. A
/// hull section is 200 hit points to the cubic unit, two and a half times the
/// cladding over it, and without that correction a hull would be carved away
/// entirely by a hit that left it at half health.
pub const DAMAGE_PER_UNIT_VOLUME: f32 = 80.0;

/// The smallest mark worth keeping, in units.
///
/// A sphere this small cannot reach a boundary sample of the cell it lands in,
/// so it would cost a slot in the budget and change nothing. Grazing fire is
/// the case: it should crack, which is the level's job, not this one.
const MARK_MIN_RADIUS: f32 = 0.15;

/// How many marks one body remembers.
///
/// A cap and not a hint: a capital in a long fight takes thousands of hits, and
/// every mark is re-subtracted from every plate whenever the list moves. Past
/// the budget a new mark is MERGED into its nearest neighbour rather than
/// dropped or rotated out - see [`DamageMarks::add`] - so damage still only
/// ever grows and an old crater cannot heal because something was shot
/// somewhere else.
const MARK_BUDGET: usize = 24;

/// One sphere of material a hit took out of a body.
#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
pub struct DamageMark {
    /// Centre, in the local frame of the body carrying the mark.
    pub at: Vec3,
    /// Radius, in the same units.
    pub radius: f32,
}

impl DamageMark {
    /// Whether `point` lands in material this crater already reaches.
    fn contains_point(&self, point: Vec3) -> bool {
        self.at.distance_squared(point) <= self.radius * self.radius
    }

    /// Add `other`'s paid volume without moving this crater.
    ///
    /// Radius cubed is proportional to sphere volume, so this conserves what
    /// the hit bought. A bounding sphere is deliberately wrong here: one
    /// distant round could otherwise inflate a crater across the whole body.
    fn absorb_volume(&mut self, other: &Self) {
        self.radius = (self.radius.powi(3) + other.radius.powi(3)).cbrt();
    }
}

/// Every hit a body remembers, in its own local frame.
///
/// Carried by the body whose SHAPE the marks change - a ship root, so its whole
/// skin reads one list. Nothing walks down from here; consumers walk up.
#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct DamageMarks(pub Vec<DamageMark>);

impl DamageMarks {
    /// Record `mark`, compacting if the budget is full.
    ///
    /// Three outcomes, and all of them add the hit's paid volume:
    ///
    /// - a hit centred inside an existing crater grows the nearest such crater;
    /// - a separate hit opens a crater while the budget has room;
    /// - at the budget, the nearest crater grows without moving its centre.
    ///
    /// Returns whether the body's shape changed. Every positive-radius hit
    /// changes it, which is what tells [`CarveSpew`] material came off.
    pub fn add(&mut self, mark: DamageMark) -> bool {
        if let Some(existing) = self
            .0
            .iter_mut()
            .filter(|existing| existing.contains_point(mark.at))
            .min_by(|a, b| {
                a.at.distance_squared(mark.at)
                    .total_cmp(&b.at.distance_squared(mark.at))
            })
        {
            existing.absorb_volume(&mark);
            return true;
        }
        if self.0.len() < MARK_BUDGET {
            self.0.push(mark);
            return true;
        }
        let Some(nearest) = self.0.iter_mut().min_by(|a, b| {
            a.at.distance_squared(mark.at)
                .total_cmp(&b.at.distance_squared(mark.at))
        }) else {
            return false;
        };
        nearest.absorb_volume(&mark);
        true
    }
}

/// The radius a hit costing `amount` carves.
///
/// A hit lands ON a surface, so what it takes out is a HEMISPHERE - hence the
/// `2/3` rather than `4/3` - of a volume priced at
/// [`DAMAGE_PER_UNIT_VOLUME`]. Pure, so the curve can be read without a running
/// app, and it is the whole coupling between what a weapon costs and what it
/// looks like it did.
pub fn mark_radius(amount: f32) -> f32 {
    if amount <= 0.0 {
        return 0.0;
    }
    let volume = amount / DAMAGE_PER_UNIT_VOLUME;
    (volume * 3.0 / (2.0 * std::f32::consts::PI)).cbrt()
}

/// What a hit of `amount` on `target` actually COST, which is all it may take
/// in material.
///
/// The first [`Health`] at or above the hit is the node that pays for it - the
/// same node [`on_damage`](super::health) charges - so a round asking for more
/// than a plate has left carves the plate and not the plate plus a bonus. A
/// node already spent pays nothing at all, and therefore carves nothing.
///
/// A body with NO pool anywhere up the chain spends the whole hit in material.
/// That is the asteroid rule rather than a fallback: a rock's remaining solid
/// IS its durability, and clamping it against a pool it does not have would
/// stop rocks carving altogether.
fn absorbed_by(world: &World, target: Entity, amount: f32) -> f32 {
    let mut current = target;
    loop {
        if let Some(health) = world.get::<Health>(current) {
            let spent = health.current <= 0.0 || world.get::<HealthZeroMarker>(current).is_some();
            return if spent {
                0.0
            } else {
                amount.min(health.current)
            };
        }
        let Some(&ChildOf(parent)) = world.get::<ChildOf>(current) else {
            return amount;
        };
        current = parent;
    }
}

/// Cut one crater of `world_radius` into `owner` at `at` (WORLD space) and
/// announce the `volume` that came off.
///
/// The one place a mark is written, so the frame conversion and the spew that
/// follows it cannot drift between the single-hit and the blast paths.
fn carve_body(world: &mut World, owner: Entity, at: Vec3, world_radius: f32, volume: f32) {
    let Some(frame) = world.get::<GlobalTransform>(owner).copied() else {
        return;
    };
    // Into the owner's frame, so the mark rides with the body. UNIFORM scale is
    // assumed: the position crosses on the affine, and the radius has to be
    // divided by that same scale by hand or a body drawn in unit space (an
    // asteroid's mesh node, scaled by its radius) would be carved by a sphere
    // its own size. A non-uniform scale would need the mark to become an
    // ellipsoid rather than just move, and nothing authors one.
    let scale = frame.scale().max_element().max(f32::EPSILON);
    let local = frame.affine().inverse().transform_point3(at);
    let Some(mut marks) = world.get_mut::<DamageMarks>(owner) else {
        return;
    };
    if !marks.add(DamageMark {
        at: local,
        radius: world_radius / scale,
    }) {
        return;
    }
    // Material was removed, so material has to go somewhere. Announced in WORLD
    // terms because that is what a spectator sees; the mark itself stays in the
    // body's frame.
    world.trigger(CarveSpew {
        entity: owner,
        at,
        radius: world_radius,
        volume,
    });
}

/// Remember that `target` was hit at `at` (WORLD space) for `amount`.
///
/// Queued rather than done here because the work is a walk up the hierarchy and
/// a transform read, neither of which a `Commands`-only caller has. Called from
/// [`apply_damage`](crate::damage::apply_damage), so every weapon marks its
/// target the same way and nothing has a private path to a body's shape.
///
/// The queue is FIFO and `apply_damage` queues this BEFORE it triggers the
/// health event, so [`absorbed_by`] reads the pool the hit is about to spend.
/// Two rounds landing in one flush interleave the same way, and each is priced
/// against what the first left.
///
/// Silently does nothing when no ancestor carries [`DamageMarks`]. That is the
/// normal case for most of a scenario - a bare prop has no shape to change -
/// and it is what keeps this off the critical path for everything that does not
/// opt in.
pub fn record_damage_mark(commands: &mut Commands, target: Entity, at: Vec3, amount: f32) {
    // Cheap pre-filter on the REQUESTED amount. Absorption only ever reduces
    // it, so a hit too small to carve at full price cannot become big enough
    // once clamped, and a graze never costs a command.
    if mark_radius(amount) < MARK_MIN_RADIUS {
        return;
    }
    commands.queue(move |world: &mut World| {
        let paid = absorbed_by(world, target, amount);
        let world_radius = mark_radius(paid);
        if world_radius < MARK_MIN_RADIUS {
            return;
        }
        let Some(owner) = mark_owner(world, target) else {
            return;
        };
        carve_body(
            world,
            owner,
            at,
            world_radius,
            paid / DAMAGE_PER_UNIT_VOLUME,
        );
    });
}

/// Record ONE crater per body for a blast centred at `at` (WORLD space) that
/// reached `hits`, each `(collider, pressure)`.
///
/// A blast is a single sphere of material removed. It damages every collider it
/// overlaps and that is correct, but a body that happens to be built out of two
/// hundred colliders must not therefore lose two hundred craters' worth of
/// material and throw two hundred piles of debris. Contributions are summed per
/// owning body - the material the blast really destroyed there - and cut as
/// one.
///
/// Summed rather than maxed because [`DAMAGE_PER_UNIT_VOLUME`] says what it
/// says: material lost is material paid for. Once [`absorbed_by`] has clamped
/// each contribution, the total is bounded by the hit points that were really
/// inside the sphere, and it is the same volume-add that sustained fire into
/// one spot already gets from [`DamageMark::absorb_volume`]. The crater is
/// capped at `max_radius`, the blast's own reach - a hole wider than the sphere
/// that made it is nonsense.
///
/// Queue this BEFORE the tick's health triggers: every body then resolves
/// against one pre-damage snapshot, which is the contract
/// [`NovaBlast`](crate::damage::NovaBlast) already states for its pressure
/// pass.
pub fn record_blast_marks(
    commands: &mut Commands,
    at: Vec3,
    max_radius: f32,
    hits: Vec<(Entity, f32)>,
) {
    commands.queue(move |world: &mut World| {
        // A linear scan, because a blast reaches a handful of BODIES however
        // many colliders it overlapped.
        let mut totals: Vec<(Entity, f32)> = Vec::new();
        for (target, amount) in hits {
            let paid = absorbed_by(world, target, amount);
            if paid <= 0.0 {
                continue;
            }
            let Some(owner) = mark_owner(world, target) else {
                continue;
            };
            match totals.iter_mut().find(|(entity, _)| *entity == owner) {
                Some((_, total)) => *total += paid,
                None => totals.push((owner, paid)),
            }
        }

        for (owner, paid) in totals {
            let world_radius = mark_radius(paid).min(max_radius);
            if world_radius < MARK_MIN_RADIUS {
                continue;
            }
            carve_body(
                world,
                owner,
                at,
                world_radius,
                paid / DAMAGE_PER_UNIT_VOLUME,
            );
        }
    });
}

/// Announces that a carve took material off `entity`, so something can be seen
/// leaving.
///
/// Fired only when a mark changed the body's shape. Repeated fire into one
/// crater grows it, so every paid hit announces what it removed. Both fields
/// are in WORLD space: a spectator does not care what frame the body keeps its
/// marks in.
///
/// An event rather than a direct spawn so the gameplay half stays free of
/// meshes and the look is replaceable: a mod that wants sparks, a puff, or
/// nothing at all observes this instead of patching the carve.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct CarveSpew {
    /// The body that lost the material.
    pub entity: Entity,
    /// Where the crater is, in world space.
    pub at: Vec3,
    /// The crater's radius, in world units.
    pub radius: f32,
    /// How much material came off, in cubic world units.
    ///
    /// What decides whether the material is worth simulating: dust below
    /// [`CHUNK_MIN_VOLUME`](super::chunk::CHUNK_MIN_VOLUME), a real body above
    /// it. Not derivable from `radius` by whoever observes this, because the
    /// two are not always the same claim - a severed piece announces the volume
    /// its own geometry holds, not the volume of a sphere.
    pub volume: f32,
}

/// The nearest entity at or above `target` that remembers marks.
fn mark_owner(world: &World, target: Entity) -> Option<Entity> {
    let mut current = target;
    loop {
        if world.get::<DamageMarks>(current).is_some() {
            return Some(current);
        }
        current = world.get::<ChildOf>(current)?.0;
    }
}

/// Registers the mark store's reflected types.
///
/// No systems: recording is a command and reading is each effect's own job, so
/// there is nothing here that runs every frame.
pub struct DamageMarksPlugin;

impl Plugin for DamageMarksPlugin {
    fn build(&self, app: &mut App) {
        debug!("DamageMarksPlugin: build");

        app.register_type::<DamageMarks>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mark(at: Vec3, radius: f32) -> DamageMark {
        DamageMark { at, radius }
    }

    /// The coupling the whole effect rests on: a bigger hit takes a bigger
    /// bite, and one that kills a cell of cladding carves about a cell.
    #[test]
    fn a_bigger_hit_carves_a_bigger_sphere() {
        assert_eq!(mark_radius(0.0), 0.0);
        assert!(mark_radius(20.0) < mark_radius(80.0));
        assert!(mark_radius(80.0) < mark_radius(500.0));

        let cell_killer = mark_radius(DAMAGE_PER_UNIT_VOLUME);
        assert!(
            (0.5..1.0).contains(&cell_killer),
            "a hit that kills one cell of skin carves about one cell, got {cell_killer}"
        );
    }

    /// A burst into one spot becomes one larger crater without spending the
    /// mark budget, and the radius preserves the volume every round paid for.
    #[test]
    fn repeated_hits_add_their_volume_to_one_crater() {
        let mut marks = DamageMarks::default();
        marks.add(mark(Vec3::ZERO, 1.0));
        marks.add(mark(Vec3::X * 0.2, 0.5));

        assert_eq!(marks.0.len(), 1, "the burst uses one crater");
        let expected = (1.0f32.powi(3) + 0.5f32.powi(3)).cbrt();
        assert!((marks.0[0].radius - expected).abs() < 1e-6);
    }

    #[test]
    fn a_separate_hit_opens_a_separate_crater_while_there_is_room() {
        let mut marks = DamageMarks::default();
        marks.add(mark(Vec3::ZERO, 0.5));
        marks.add(mark(Vec3::X * 2.0, 0.5));

        assert_eq!(marks.0.len(), 2);
    }

    /// Past the budget the list stops growing, and what is already carved stays
    /// carved: no mark is ever dropped or shrunk.
    #[test]
    fn a_full_list_merges_instead_of_forgetting() {
        let mut marks = DamageMarks::default();
        for step in 0..MARK_BUDGET {
            marks.add(mark(Vec3::X * step as f32 * 10.0, 0.5));
        }
        assert_eq!(marks.0.len(), MARK_BUDGET, "delivery guard: it filled up");
        let before: Vec<f32> = marks.0.iter().map(|mark| mark.radius).collect();

        marks.add(mark(Vec3::X * 3.0, 0.5));

        assert_eq!(marks.0.len(), MARK_BUDGET, "the list is capped");
        for (mark, was) in marks.0.iter().zip(before) {
            assert!(
                mark.radius >= was,
                "a merge may only grow a crater, never heal one"
            );
        }
    }

    /// Compaction is allowed to lose the distant hit's exact location, but it
    /// must preserve its paid volume without the old whole-body inflation.
    #[test]
    fn a_full_list_adds_volume_to_the_nearest_crater() {
        let mut marks = DamageMarks::default();
        for step in 0..MARK_BUDGET {
            marks.add(mark(Vec3::X * step as f32 * 10.0, 0.5));
        }

        let before = marks.0[0].radius;
        marks.add(mark(Vec3::Y * 2.0, 0.4));

        let expected = (before.powi(3) + 0.4f32.powi(3)).cbrt();
        assert!((marks.0[0].radius - expected).abs() < 1e-6);
        assert!(
            marks.0[0].radius < 1.0,
            "a distant hit did not span the body"
        );
    }

    /// A graze is not a bite. It cannot reach a boundary sample of the cell it
    /// lands in, so recording one would cost a budget slot for no change.
    #[test]
    fn a_graze_is_not_recorded() {
        let mut app = App::new();
        let body = app.world_mut().spawn(DamageMarks::default()).id();
        let mut commands = app.world_mut().commands();
        record_damage_mark(&mut commands, body, Vec3::ZERO, 0.01);
        app.world_mut().flush();

        assert!(app.world().get::<DamageMarks>(body).unwrap().0.is_empty());
    }

    /// The rule that makes a carve continuous: a hit on a child is remembered
    /// by the body that owns the shape, in THAT body's frame, so every one of
    /// its parts derives its own geometry from the same list.
    #[test]
    fn a_hit_on_a_part_is_remembered_by_the_body_that_owns_the_shape() {
        let mut app = App::new();
        let root = app
            .world_mut()
            .spawn((
                DamageMarks::default(),
                GlobalTransform::from_translation(Vec3::new(10.0, 0.0, 0.0)),
            ))
            .id();
        let section = app.world_mut().spawn(ChildOf(root)).id();
        let plate = app.world_mut().spawn(ChildOf(section)).id();

        let mut commands = app.world_mut().commands();
        record_damage_mark(&mut commands, plate, Vec3::new(11.0, 0.0, 0.0), 200.0);
        app.world_mut().flush();

        assert!(
            app.world().get::<DamageMarks>(plate).is_none(),
            "the part does not keep its own list"
        );
        let marks = &app.world().get::<DamageMarks>(root).unwrap().0;
        assert_eq!(marks.len(), 1, "the root remembers it");
        assert!(
            marks[0].at.abs_diff_eq(Vec3::X, 1e-5),
            "in the root's own frame, got {}",
            marks[0].at
        );
    }

    /// A body that keeps marks and no hit points spends the whole hit in
    /// material. A rock's remaining solid IS its durability, so a clamp against
    /// a pool it does not have would stop rocks carving.
    #[test]
    fn a_body_with_no_hit_points_spends_the_whole_hit_in_material() {
        let mut app = App::new();
        let root = app
            .world_mut()
            .spawn((DamageMarks::default(), GlobalTransform::IDENTITY))
            .id();
        let node = app.world_mut().spawn(ChildOf(root)).id();

        let mut commands = app.world_mut().commands();
        record_damage_mark(&mut commands, node, Vec3::ZERO, 600.0);
        app.world_mut().flush();

        let marks = &app.world().get::<DamageMarks>(root).unwrap().0;
        assert_eq!(marks.len(), 1);
        assert!(
            (marks[0].radius - mark_radius(600.0)).abs() < 1e-6,
            "a rock pays the full price, got {}",
            marks[0].radius
        );
    }

    /// THE pricing rule: a mark costs what the hit DESTROYED, not what the
    /// round asked for. A 100-damage slug crossing a 20 hp plate takes 20
    /// hit points of material there and carries the other 80 to whatever is
    /// behind it - the round, once, instead of the 180 the requested amount
    /// used to buy.
    #[test]
    fn a_mark_is_priced_by_what_the_hit_destroyed_not_by_what_it_asked_for() {
        let mut app = App::new();
        let root = app
            .world_mut()
            .spawn((DamageMarks::default(), GlobalTransform::IDENTITY))
            .id();
        let plate = app
            .world_mut()
            .spawn((ChildOf(root), Health::new(20.0)))
            .id();

        let mut commands = app.world_mut().commands();
        record_damage_mark(&mut commands, plate, Vec3::ZERO, 100.0);
        app.world_mut().flush();

        let marks = &app.world().get::<DamageMarks>(root).unwrap().0;
        assert_eq!(marks.len(), 1);
        assert!(
            (marks[0].radius - mark_radius(20.0)).abs() < 1e-6,
            "priced at the plate's 20 hp, not the round's 100: got {} want {}",
            marks[0].radius,
            mark_radius(20.0)
        );
    }

    /// A hit on a corpse costs nobody anything, so it takes no material and
    /// throws no debris. This is most of a second warhead into a chewed hull.
    #[test]
    fn a_hit_on_a_spent_node_takes_no_material() {
        // Both ways a node reads as spent: an emptied pool, and one the
        // destruction pipeline has already marked but not yet swept away.
        let drained = Health {
            current: 0.0,
            max: 50.0,
        };
        for (case, pool, marked) in [
            ("drained", drained, false),
            ("marked", Health::new(50.0), true),
        ] {
            let mut app = App::new();
            let root = app
                .world_mut()
                .spawn((DamageMarks::default(), GlobalTransform::IDENTITY))
                .id();
            let mut plate = app.world_mut().spawn((ChildOf(root), pool));
            if marked {
                plate.insert(HealthZeroMarker);
            }
            let plate = plate.id();

            let mut commands = app.world_mut().commands();
            record_damage_mark(&mut commands, plate, Vec3::ZERO, 500.0);
            app.world_mut().flush();

            assert!(
                app.world().get::<DamageMarks>(root).unwrap().0.is_empty(),
                "{case}: a corpse has nothing left to lose"
            );
        }
    }

    /// The command order the pricing rests on: a mark is queued before its own
    /// health event, so two rounds landing in one flush are each priced against
    /// what the one before it left. 60 into a 100 hp plate takes 60, the next
    /// 60 takes the remaining 40, and the crater holds exactly the plate.
    #[test]
    fn two_rounds_in_one_flush_are_each_priced_against_what_is_left() {
        use crate::damage::prelude::apply_damage;

        let mut app = App::new();
        app.add_plugins(super::super::health::NovaHealthPlugin);
        let root = app
            .world_mut()
            .spawn((DamageMarks::default(), GlobalTransform::IDENTITY))
            .id();
        let plate = app
            .world_mut()
            .spawn((ChildOf(root), Health::new(100.0)))
            .id();

        let mut commands = app.world_mut().commands();
        for _ in 0..2 {
            apply_damage(&mut commands, plate, None, 60.0, Some(Vec3::ZERO));
        }
        app.world_mut().flush();

        assert_eq!(app.world().get::<Health>(plate).unwrap().current, 0.0);
        let marks = &app.world().get::<DamageMarks>(root).unwrap().0;
        assert_eq!(marks.len(), 1, "both rounds hit the same spot");
        assert!(
            (marks[0].radius - mark_radius(100.0)).abs() < 1e-5,
            "the crater holds the plate and no more: got {} want {}",
            marks[0].radius,
            mark_radius(100.0)
        );
    }

    /// Most of a scenario has no shape to change, and nothing may fail because
    /// of it.
    #[test]
    fn a_hit_on_a_body_that_remembers_nothing_is_dropped() {
        let mut app = App::new();
        let prop = app.world_mut().spawn(GlobalTransform::IDENTITY).id();
        let mut commands = app.world_mut().commands();
        record_damage_mark(&mut commands, prop, Vec3::ZERO, 200.0);
        app.world_mut().flush();

        assert!(app.world().get::<DamageMarks>(prop).is_none());
    }
}
