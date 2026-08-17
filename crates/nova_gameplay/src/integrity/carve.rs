//! WHERE a body was hit, kept as spheres of material taken out of it.
//!
//! The other half of the damage picture. [`DamageLevel`](super::erosion) says
//! how far gone a body is and every effect that grades a whole body - scorch,
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

use bevy::prelude::*;

/// `CarveSpew`, `DamageMark`, `DamageMarks`, `mark_radius` and
/// `record_damage_mark`.
pub mod prelude {
    pub use super::{mark_radius, record_damage_mark, CarveSpew, DamageMark, DamageMarks};
}

/// Hit points a hit has to spend to take one cubic unit of material off.
///
/// Deliberately the toughness a ship's cladding is built at, so the two agree:
/// a hit big enough to kill one cell of skin is a hit that carves about one
/// cell of skin, and a plate never survives a mark that has already swallowed
/// it or vanishes under one that barely scratched it.
const DAMAGE_PER_UNIT_VOLUME: f32 = 80.0;

/// The smallest mark worth keeping, in units.
///
/// A sphere this small cannot reach a boundary sample of the cell it lands in,
/// so it would cost a slot in the budget and change nothing. Grazing fire is
/// the case: it should scorch, which is the level's job, not this one.
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
    /// Whether this mark already swallows `other`, which is when recording
    /// `other` would change no geometry anywhere.
    fn contains(&self, other: &Self) -> bool {
        self.at.distance(other.at) + other.radius <= self.radius
    }

    /// Grow this mark until it also covers `other`.
    ///
    /// The bounding sphere about the existing centre rather than the true
    /// bounding sphere of the pair: it is cheaper, it never moves a crater that
    /// has already been drawn, and because the radius only ever grows the carve
    /// stays monotone - the guarantee that lets a mark list be compacted at all.
    fn absorb(&mut self, other: &Self) {
        self.radius = self.radius.max(self.at.distance(other.at) + other.radius);
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
    /// Three outcomes, and all of them leave the carved volume the same or
    /// larger:
    ///
    /// - an existing mark already covers this one: nothing is recorded, which
    ///   is what stops a burst emptied into one hole from spending the whole
    ///   budget on the same crater;
    /// - there is room: it is appended;
    /// - there is not: the nearest existing mark grows to cover it. A distant
    ///   hit therefore blows the crater it lands nearest out of proportion,
    ///   which is the honest failure - a ship that has been shot two dozen
    ///   separate times is a ship that should look comprehensively holed.
    ///
    /// Returns whether the body's shape actually changed, which is what tells
    /// [`CarveSpew`] whether any material came off.
    pub fn add(&mut self, mark: DamageMark) -> bool {
        if self.0.iter().any(|existing| existing.contains(&mark)) {
            return false;
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
        nearest.absorb(&mark);
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

/// Remember that `target` was hit at `at` (WORLD space) for `amount`.
///
/// Queued rather than done here because the work is a walk up the hierarchy and
/// a transform read, neither of which a `Commands`-only caller has. Called from
/// [`apply_damage`](crate::damage::apply_damage), so every weapon marks its
/// target the same way and nothing has a private path to a body's shape.
///
/// Silently does nothing when no ancestor carries [`DamageMarks`]. That is the
/// normal case for most of a scenario - a bare prop has no shape to change -
/// and it is what keeps this off the critical path for everything that does not
/// opt in.
pub fn record_damage_mark(commands: &mut Commands, target: Entity, at: Vec3, amount: f32) {
    let world_radius = mark_radius(amount);
    if world_radius < MARK_MIN_RADIUS {
        return;
    }
    commands.queue(move |world: &mut World| {
        let Some(owner) = mark_owner(world, target) else {
            return;
        };
        let Some(frame) = world.get::<GlobalTransform>(owner).copied() else {
            return;
        };
        // Into the owner's frame, so the mark rides with the body. UNIFORM
        // scale is assumed: the position crosses on the affine, and the radius
        // has to be divided by that same scale by hand or a body drawn in unit
        // space (an asteroid's mesh node, scaled by its radius) would be carved
        // by a sphere its own size. A non-uniform scale would need the mark to
        // become an ellipsoid rather than just move, and nothing authors one.
        let scale = frame.scale().max_element().max(f32::EPSILON);
        let local = frame.affine().inverse().transform_point3(at);
        let Some(mut marks) = world.get_mut::<DamageMarks>(owner) else {
            return;
        };
        if !marks.add(DamageMark {
            at: local,
            radius: world_radius / scale,
        }) {
            // Already inside a crater: no material came off, so nothing should
            // be seen coming off.
            return;
        }
        // Material was removed, so material has to go somewhere. Announced in
        // WORLD terms because that is what a spectator sees; the mark itself
        // stays in the body's frame.
        world.trigger(CarveSpew {
            entity: owner,
            at,
            radius: world_radius,
        });
    });
}

/// Announces that a carve took material off `entity`, so something can be seen
/// leaving.
///
/// Fired only when a mark actually changed the body's shape - a hit into an
/// existing crater carves nothing and spews nothing. Both fields are in WORLD
/// space: a spectator does not care what frame the body keeps its marks in.
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

    /// A burst emptied into one hole must not spend the budget on one crater.
    #[test]
    fn a_hit_inside_an_existing_crater_records_nothing() {
        let mut marks = DamageMarks::default();
        marks.add(mark(Vec3::ZERO, 1.0));
        marks.add(mark(Vec3::X * 0.2, 0.3));

        assert_eq!(marks.0.len(), 1, "the second hit is already carved away");
        assert_eq!(marks.0[0].radius, 1.0, "and did not grow the first");
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

    /// A merged mark really does cover what it absorbed, which is what makes
    /// compacting safe: the geometry a caller derives afterwards is at least as
    /// carved as it would have been with both marks kept.
    #[test]
    fn a_merged_mark_still_covers_the_hit_it_absorbed() {
        let mut marks = DamageMarks::default();
        for step in 0..MARK_BUDGET {
            marks.add(mark(Vec3::X * step as f32 * 10.0, 0.5));
        }

        let late = mark(Vec3::new(0.0, 2.0, 0.0), 0.4);
        marks.add(late);

        assert!(
            marks.0.iter().any(|kept| kept.contains(&late)),
            "the hit is still carved by whichever mark absorbed it"
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
