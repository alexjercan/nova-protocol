//! A section with real bites taken out of it.
//!
//! The last body in the game that could not lose geometry. A rock carves as
//! deep as the hit deserves, a ship's cladding carves one cell deep and stops,
//! and underneath the cladding sat an authored glTF model nothing could cut. So
//! a hull under sustained fire lost its skin and then stopped changing, however
//! much more it took.
//!
//! This reads a solid out of the section's own drawn mesh
//! ([`field_from_mesh`]), subtracts the ship's [`DamageMarks`] from it, and
//! swaps the mesh for what is left. The marks are the SHIP's, so a crater
//! crosses from a plate onto the hull under it and reads as one hole rather
//! than as two effects that happen to overlap.
//!
//! # What it costs, and what it costs it in
//!
//! The remeshed hull is NOT the authored hull. Surface nets over a
//! [`SECTION_FIELD_RESOLUTION`]-cell grid softens every hard edge the artist put
//! there, so a section that has been scratched draws its own shape at the
//! mesher's fidelity rather than at the model's. That is why this is authored
//! per section and not given to everything: a hull is a slab and survives it, a
//! turret is a mechanism whose silhouette IS the read and does not.
//!
//! # The collider does NOT follow
//!
//! Deliberately, and for the same reason the skin's does not. A section's
//! collider is authored, its mass is derived from that collider's volume, and
//! the link-point graph that decides what holds a ship together is built against
//! the authored shape. Swapping the collider under a carve would move a ship's
//! mass and inertia every time it was shot, and change which sections count as
//! attached to which. A rock can afford to re-derive both because a rock's
//! collider IS its mesh and nothing is bolted to it.
//!
//! The visible consequence is a shallow lie: a round can pass through the air
//! inside a deep crater and still be stopped. It is the same lie a shot-off
//! cladding plate already tells, and it is much cheaper than the alternative.

use bevy::prelude::*;
use nova_gameplay::prelude::{
    field_from_mesh, DamageMarks, Health, SectionMarker, SignedField, DAMAGE_PER_UNIT_VOLUME,
};

/// `DamageCarve`, `DamageCarvePlugin` and the section field resolution.
pub mod prelude {
    pub use super::{DamageCarve, DamageCarvePlugin, SECTION_FIELD_RESOLUTION};
}

/// Cells per axis in a section's carve field.
///
/// Lower than an asteroid's 32, and not to save memory - a section is a few
/// units across where a rock is tens, so the same cell count would give a
/// section cells a tenth the size and spend ten times the samples resolving
/// detail the mesher then flattens anyway. 24 puts a section's cells at roughly
/// a rock's, which is what keeps a crater in a hull looking like a crater in a
/// rock.
pub const SECTION_FIELD_RESOLUTION: usize = 24;

/// Marks a section whose drawn geometry is carved by the hits its ship takes.
///
/// Fitted from [`DamageEffect::Carve`](super::damage_effects::DamageEffect).
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct DamageCarve;

/// The carved solid behind ONE drawn mesh, and what it was last carved by.
#[derive(Component, Debug)]
struct CarvedSection {
    /// The solid, in the mesh's own space.
    field: SignedField,
    /// Where the field's grid sits in that space - authored art is modelled
    /// wherever the artist put it, so the grid is centred on the mesh rather
    /// than on its origin.
    offset: Vec3,
    /// How much of a mark's radius this section actually loses, in `0..=1`.
    ///
    /// See [`bite_of`]. A mark is priced at the CLADDING's toughness because it
    /// is one sphere shared by everything it reaches, and a hull is tougher than
    /// its own cladding - so it takes a smaller bite out of the same sphere.
    bite: f32,
    /// A fingerprint of the mark list already applied to `field`.
    applied: u64,
    /// Quantized solid volume at the last mesh swap.
    ///
    /// Sub-cell hits accumulate in the field without paying for a remesh the
    /// grid cannot yet show.
    meshed_volume: f32,
}

/// How much of a mark's radius a section of `health` hit points and `volume`
/// cubic units loses to it.
///
/// A mark is one sphere, sized by
/// [`DAMAGE_PER_UNIT_VOLUME`] - the toughness a ship's CLADDING is built at -
/// and shared by every body it reaches, so it cannot be priced per body. What a
/// tougher body does instead is take a proportionally smaller bite: the radius
/// scales by the cube root of the toughness ratio, because volume goes as the
/// cube of it.
///
/// A shipped hull section is 200 hit points to the cubic unit, two and a half
/// times its cladding, so it loses about three quarters of each mark's radius -
/// and spending its whole health bar carves about its whole volume, which is the
/// coupling `DAMAGE_PER_UNIT_VOLUME` claims and did not have here.
///
/// Never above 1: a section SOFTER than cladding still carves at the cladding's
/// rate. The alternative is a section that dissolves faster than the plate over
/// it, which reads as the armour protecting nothing.
fn bite_of(health: f32, volume: f32) -> f32 {
    if health <= 0.0 || volume <= 0.0 {
        return 1.0;
    }
    let toughness = health / volume;
    (DAMAGE_PER_UNIT_VOLUME / toughness).cbrt().min(1.0)
}

/// Marks a mesh no solid could be read out of, so it is not asked again.
///
/// Authored art is under no obligation to be closed
/// ([`field_from_mesh`](nova_gameplay::prelude::field_from_mesh) refuses when it
/// is not), and retrying every frame would pay the whole weld-and-check cost
/// per frame for a section that will never carve.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
struct CarveRefused;

/// A fingerprint of `marks`: enough to tell a list that moved from one that did
/// not.
///
/// NOT a count. Past the mark budget a hit MERGES into its nearest neighbour,
/// growing that mark's radius without growing the list.
fn signature(marks: &DamageMarks) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for mark in &marks.0 {
        for value in [mark.at.x, mark.at.y, mark.at.z, mark.radius] {
            hash ^= u64::from(value.to_bits());
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    hash
}

/// Carve every carving section's drawn meshes by the marks its ship carries.
///
/// Walks DOWN from the section rather than up from each mesh. A ship draws with
/// hundreds of mesh entities and carries a handful of carving sections, and the
/// walk only happens when the marks have actually moved.
#[expect(
    clippy::type_complexity,
    reason = "the mesh query carries the art, its frame and its carve state"
)]
fn carve_section_meshes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    q_sections: Query<(Entity, Option<&Health>), (With<DamageCarve>, With<SectionMarker>)>,
    q_parents: Query<&ChildOf>,
    q_children: Query<&Children>,
    q_marks: Query<(&DamageMarks, &GlobalTransform)>,
    mut q_art: Query<
        (
            Entity,
            &Mesh3d,
            &GlobalTransform,
            Option<&mut CarvedSection>,
            Has<CarveRefused>,
        ),
        Without<DamageCarve>,
    >,
) {
    for (section, health) in &q_sections {
        let Some((marks, owner)) = mark_owner(section, &q_parents, &q_marks) else {
            continue;
        };
        if marks.0.is_empty() {
            continue;
        }
        let stamp = signature(marks);
        // A section with no health pool at all is scenery, and scenery carves
        // at the cladding's rate because there is nothing else to price it by.
        let health = health.map_or(0.0, |health| health.max);

        for art in art_meshes(section, &q_children, &q_art) {
            let Ok((art, mesh, frame, carved, refused)) = q_art.get_mut(art) else {
                continue;
            };
            if refused {
                continue;
            }

            let mut carved = match carved {
                Some(carved) => carved,
                None => {
                    let Some(source) = meshes.get(&mesh.0) else {
                        continue;
                    };
                    let started = std::time::Instant::now();
                    let Some((field, offset)) = field_from_mesh(source, SECTION_FIELD_RESOLUTION)
                    else {
                        debug!("carve_section_meshes: {art:?} has no solid to carve, skipped");
                        commands.entity(art).insert(CarveRefused);
                        continue;
                    };
                    debug!(
                        "carve_section_meshes: solidified {art:?} at \
                         {SECTION_FIELD_RESOLUTION}^3 in {:.1} ms",
                        started.elapsed().as_secs_f32() * 1000.0
                    );
                    // The section's own toughness, measured against the solid
                    // that was just read out of its art rather than guessed.
                    let bite = bite_of(health, field.solid_volume());
                    let meshed_volume = field.solid_volume();
                    commands.entity(art).insert(CarvedSection {
                        field,
                        offset,
                        bite,
                        applied: 0,
                        meshed_volume,
                    });
                    // The insert lands next flush and this mesh is carved the
                    // frame after, which keeps the solidify cost out of the
                    // same frame as the first remesh.
                    continue;
                }
            };

            if carved.applied != stamp {
                carved.applied = stamp;

                // Marks are in the SHIP's frame and the field is in the mesh's,
                // so every mark crosses two transforms to get here. Uniform
                // scale is assumed on both, exactly as `record_damage_mark`
                // assumes it.
                let into_art = frame.affine().inverse();
                let scale = frame.scale().max_element().max(f32::EPSILON);
                let owner_scale = owner.scale().max_element();
                let (offset, bite) = (carved.offset, carved.bite);
                for mark in &marks.0 {
                    let at = into_art.transform_point3(owner.transform_point(mark.at)) - offset;
                    carved
                        .field
                        .subtract_sphere(at, mark.radius * bite * owner_scale / scale);
                }
            }

            if carved.field.solid_volume() >= carved.meshed_volume {
                continue;
            }

            let surface = carved.field.surface().build();
            if surface.count_vertices() == 0 {
                // Carved away entirely. Leaving the last mesh is the honest
                // answer: the section is still alive and still has to be drawn
                // as something.
                debug!("carve_section_meshes: {art:?} carved away entirely, mesh kept");
                carved.meshed_volume = carved.field.solid_volume();
                continue;
            }
            debug!(
                "carve_section_meshes: {art:?} of {section:?} remeshed to {} tri(s) by {} mark(s)",
                surface.count_vertices() / 3,
                marks.0.len()
            );
            // Back into the mesh's own space - the field is centred on the
            // art's bounds, and the entity's transform is not.
            let mut surface = surface;
            surface.translate_by(carved.offset);
            commands.entity(art).insert(Mesh3d(meshes.add(surface)));
            carved.meshed_volume = carved.field.solid_volume();
        }
    }
}

/// The nearest ancestor of `section` that remembers marks, and its frame.
fn mark_owner<'a>(
    section: Entity,
    q_parents: &Query<&ChildOf>,
    q_marks: &'a Query<(&DamageMarks, &GlobalTransform)>,
) -> Option<(&'a DamageMarks, &'a GlobalTransform)> {
    let mut current = section;
    loop {
        if let Ok(found) = q_marks.get(current) {
            return Some(found);
        }
        current = q_parents.get(current).ok()?.0;
    }
}

/// Every drawn mesh under `section`, including the ones a glTF scene hung there
/// after the section spawned.
fn art_meshes(
    section: Entity,
    q_children: &Query<&Children>,
    q_art: &Query<
        (
            Entity,
            &Mesh3d,
            &GlobalTransform,
            Option<&mut CarvedSection>,
            Has<CarveRefused>,
        ),
        Without<DamageCarve>,
    >,
) -> Vec<Entity> {
    let mut found = Vec::new();
    let mut stack = vec![section];
    while let Some(entity) = stack.pop() {
        if q_art.contains(entity) {
            found.push(entity);
        }
        if let Ok(children) = q_children.get(entity) {
            stack.extend(children.iter());
        }
    }
    found
}

/// Carves the drawn geometry of sections that author it.
#[derive(Default, Clone, Debug)]
pub struct DamageCarvePlugin {
    /// Whether carving runs. It is entirely a LOOK - the collider does not
    /// follow - so a headless server does not need it.
    pub render: bool,
}

impl Plugin for DamageCarvePlugin {
    fn build(&self, app: &mut App) {
        debug!("DamageCarvePlugin: build");

        app.register_type::<DamageCarve>();
        app.register_type::<CarveRefused>();
        if self.render {
            app.add_systems(Update, carve_section_meshes);
        }
    }
}

#[cfg(test)]
mod tests {
    use nova_gameplay::prelude::{DamageMark, DamageMarks};

    use super::*;

    /// A ship root carrying marks, one carving section under it, and one drawn
    /// mesh under that - the shape production builds.
    fn carve_app(mesh: Mesh) -> (App, Entity, Handle<Mesh>) {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>();
        app.add_plugins(DamageCarvePlugin { render: true });

        let handle = app.world_mut().resource_mut::<Assets<Mesh>>().add(mesh);
        let ship = app
            .world_mut()
            .spawn((
                DamageMarks::default(),
                Transform::default(),
                GlobalTransform::IDENTITY,
            ))
            .id();
        let section = app
            .world_mut()
            .spawn((
                SectionMarker,
                DamageCarve,
                ChildOf(ship),
                Transform::default(),
                GlobalTransform::IDENTITY,
            ))
            .id();
        let art = app
            .world_mut()
            .spawn((
                ChildOf(section),
                Mesh3d(handle.clone()),
                Transform::default(),
                GlobalTransform::IDENTITY,
            ))
            .id();
        (app, art, handle)
    }

    /// Record a hit on the ship, in the ship's own frame.
    fn hit(app: &mut App, at: Vec3, radius: f32) {
        let mut q = app.world_mut().query::<&mut DamageMarks>();
        let mut marks = q.single_mut(app.world_mut()).expect("the ship has marks");
        marks.add(DamageMark { at, radius });
    }

    /// THE claim: a hit takes material out of the section's drawn geometry.
    #[test]
    fn a_hit_takes_a_bite_out_of_a_section() {
        let (mut app, art, original) = carve_app(Mesh::from(Cuboid::new(1.0, 1.0, 1.0)));
        hit(&mut app, Vec3::new(0.5, 0.0, 0.0), 0.35);

        // Solidify lands on one frame, the carve on the next.
        app.update();
        app.update();

        let drawn = app
            .world()
            .get::<Mesh3d>(art)
            .map(|mesh| mesh.0.clone())
            .expect("the section is still drawn");
        assert_ne!(drawn, original, "a carved section draws a new mesh");

        let meshes = app.world().resource::<Assets<Mesh>>();
        let mesh = meshes.get(&drawn).expect("the new mesh exists");
        let bevy::mesh::VertexAttributeValues::Float32x3(positions) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION).expect("positions")
        else {
            panic!("a built mesh has positions");
        };
        let nearest = positions
            .iter()
            .map(|point| Vec3::from_array(*point).distance(Vec3::new(0.5, 0.0, 0.0)))
            .fold(f32::MAX, f32::min);
        assert!(
            nearest > 0.15,
            "material survived inside the crater, {nearest} from its centre"
        );
    }

    /// Authored art is under no obligation to be closed, and an open mesh must
    /// be refused ONCE rather than re-checked every frame.
    #[test]
    fn a_section_whose_art_has_no_inside_is_left_alone() {
        let sheet = Mesh::new(
            bevy::mesh::PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::default(),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![
                [-1.0f32, 0.0, -1.0],
                [1.0, 0.0, -1.0],
                [1.0, 0.0, 1.0],
                [-1.0, 0.0, 1.0],
            ],
        )
        .with_inserted_indices(bevy::mesh::Indices::U32(vec![0, 1, 2, 0, 2, 3]));

        let (mut app, art, original) = carve_app(sheet);
        hit(&mut app, Vec3::ZERO, 0.5);
        app.update();
        app.update();

        assert_eq!(
            app.world().get::<Mesh3d>(art).map(|mesh| mesh.0.clone()),
            Some(original),
            "an open mesh is drawn exactly as it was"
        );
        assert!(
            app.world().get::<CarveRefused>(art).is_some(),
            "and is not asked again"
        );
    }

    /// A section nobody has hit must not pay to solidify its art. The whole
    /// cost of this effect is on bodies that are actually under fire.
    #[test]
    fn an_unhit_section_builds_nothing() {
        let (mut app, art, original) = carve_app(Mesh::from(Cuboid::new(1.0, 1.0, 1.0)));
        app.update();
        app.update();

        assert!(app.world().get::<CarvedSection>(art).is_none());
        assert_eq!(
            app.world().get::<Mesh3d>(art).map(|mesh| mesh.0.clone()),
            Some(original)
        );
    }
}
