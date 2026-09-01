//! What a lance and its shell LOOK like: the gun's body, the slug's dart, and
//! the flash the shot leaves at the bore.
//!
//! Gameplay says WHEN (the `RailgunFired` event, the slug's spawn); this says
//! what that looks like, and every system here is behind the plugin's `render`
//! flag so a headless server builds none of it.

use nova_gameplay::transient_light::prelude::LightFlash;

use super::*;

/// The slug's silhouette. Far bigger than a PDC dart and deliberately so: one
/// shell in the air at a time, crossing the whole engagement in a tick or two,
/// so it has to be legible for the couple of frames it exists.
const SLUG_RADIUS: f32 = 0.06;
/// Length of the slug's cylindrical body.
const SLUG_BODY: f32 = 0.5;
/// Length of the slug's nose cone. Most of the shell is point.
const SLUG_NOSE: f32 = 0.3;

/// How hard the slug's emissive burns over its base colour.
///
/// Hotter than a tracer's: a rail-driven shell leaves the bore incandescent,
/// and at the speed it travels the streak is most of what a player ever sees
/// of it.
const SLUG_EMISSIVE_GAIN: f32 = 40.0;

/// Brightness of the muzzle flash, in lumens.
///
/// Between the torpedo's ignition torch (100k) and its detonation, because
/// that is what the shot IS: a capacitor bank dumping into a bore, seen from
/// the ship that owns it.
const MUZZLE_LIGHT_LUMENS: f32 = 900_000.0;
/// How far the muzzle flash reaches, in world units. Long enough to throw the
/// firing ship's own hull into relief for a frame.
const MUZZLE_LIGHT_RANGE: f32 = 60.0;
/// How long the muzzle flash burns, in seconds. A discharge, not a burn.
const MUZZLE_LIGHT_SECS: f32 = 0.18;

/// The slug's shared mesh and material - one pair for the whole app, like the
/// turret's projectile art, because every lance in the game fires the same
/// shell.
#[derive(Resource, Debug)]
pub(super) struct RailgunSlugArt {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}

impl FromWorld for RailgunSlugArt {
    fn from_world(world: &mut World) -> Self {
        world.resource_scope(|world, mut meshes: Mut<Assets<Mesh>>| {
            let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
            // Nose down -Z: a round's transform IS its line of flight, so the
            // shared +Y body is turned onto that axis once, at build time.
            let mesh = meshes.add(
                super::super::nose_cone_mesh(SLUG_RADIUS, SLUG_BODY, SLUG_NOSE)
                    .rotated_by(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
            );
            // The Pierce colour, so a slug in flight reads as the same family
            // as the penetrator rounds a PDC loads - it is the extreme of that
            // weapon class, not a fourth damage type.
            let color = damage_type_color(DamageType::Pierce);
            let linear = color.to_linear();
            let material = materials.add(StandardMaterial {
                base_color: color,
                emissive: LinearRgba::rgb(
                    linear.red * SLUG_EMISSIVE_GAIN,
                    linear.green * SLUG_EMISSIVE_GAIN,
                    linear.blue * SLUG_EMISSIVE_GAIN,
                ),
                ..default()
            });
            Self { mesh, material }
        })
    }
}

/// Dress a fired slug. Its own entity carries the pose, so the art goes
/// straight on rather than into a child.
pub(super) fn insert_railgun_slug_render(
    add: On<Add, RailgunSlugProjectileMarker>,
    mut commands: Commands,
    art: Res<RailgunSlugArt>,
) {
    commands.entity(add.entity).insert((
        Mesh3d(art.mesh.clone()),
        MeshMaterial3d(art.material.clone()),
    ));
}

/// Spawn the lance's body: the authored scene, or the placeholder block a
/// section with no art wears.
pub(super) fn insert_railgun_section_render(
    add: On<Add, RailgunSectionMarker>,
    mut commands: Commands,
    placeholder: Res<PlaceholderArt>,
    asset_server: Res<AssetServer>,
    q_railgun: Query<
        (&RailgunSectionRenderMesh, &SectionRenderMeshTransform),
        With<RailgunSectionMarker>,
    >,
) {
    let entity = add.entity;
    trace!("insert_railgun_section_render: entity {:?}", entity);

    let Ok((render_mesh, render_mesh_transform)) = q_railgun.get(entity) else {
        error!(
            "insert_railgun_section_render: entity {:?} not found in q_railgun",
            entity
        );
        return;
    };

    match &**render_mesh {
        Some(asset_ref) => {
            let scene = asset_ref.resolve(&asset_server);
            let transform = render_mesh_transform
                .map(RenderMeshTransform::to_transform)
                .unwrap_or_default();
            commands.entity(entity).insert(children![(
                Name::new("Railgun Section Body"),
                transform,
                SectionRenderOf(entity),
                WorldAssetRoot(scene),
            )]);
        }
        None => {
            commands.entity(entity).insert(children![(
                Name::new("Railgun Section Body"),
                SectionRenderOf(entity),
                Mesh3d(placeholder.body.clone()),
                MeshMaterial3d(placeholder.structure_material.clone()),
            )]);
        }
    }
}

/// Light the bore when a lance fires.
///
/// The light IS the effect here, as it is for the torpedo's light-off: there
/// is no fireball for it to be the flash of, so it is unbudgeted and the
/// transient-light cap is its own limit.
pub(super) fn on_railgun_fired_flash(fired: On<RailgunFired>, mut commands: Commands) {
    trace!("on_railgun_fired_flash: railgun {:?}", fired.entity);
    commands.trigger(LightFlash {
        at: fired.muzzle,
        // The rails' own colour from the lance art (`railgun_lance.json`
        // `part_glow`), so the flash and the bore it comes out of agree.
        color: Color::srgb(0.45, 0.85, 1.0),
        peak_intensity: MUZZLE_LIGHT_LUMENS,
        range: MUZZLE_LIGHT_RANGE,
        duration: MUZZLE_LIGHT_SECS,
    });
}
