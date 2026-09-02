//! What a lance and its shell LOOK like: the gun's body, the slug's dart, and
//! the flash the shot leaves at the bore.
//!
//! Gameplay says WHEN (the `RailgunFired` event, the slug's spawn); this says
//! what that looks like, and every system here is behind the plugin's `render`
//! flag so a headless server builds none of it.
//!
//! Engine units throughout: meshes, tracer lengths and `PointLight::range` all
//! count world units, one of which is 10 m. Authored fields the art is sized
//! against - `slug_speed`, `muzzle_speed` - are quoted in the meters a creator
//! reads in the content file.

use nova_gameplay::transient_light::prelude::LightFlash;

use super::*;
use crate::sections::{nose_cone_mesh, turret_section::RoundTracer};

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
/// How far the muzzle flash reaches, in world units (600 m). Long enough to
/// throw the firing ship's own hull into relief for a frame.
const MUZZLE_LIGHT_RANGE: f32 = 60.0;
/// How long the muzzle flash burns, in seconds. A discharge, not a burn.
const MUZZLE_LIGHT_SECS: f32 = 0.18;

/// The rails' own colour, from the lance art (`railgun_lance.glb`'s `part_glow`
/// material). The charge glow, the muzzle flash and the bore they come out of
/// all wear it, so the whole cycle reads as one piece of hardware.
pub(super) const RAIL_GLOW_COLOR: Color = Color::srgb(0.45, 0.85, 1.0);

/// Peak brightness of the charge glow, in lumens, reached the instant before
/// the shot.
///
/// A sixth of the muzzle flash on purpose: the charge has to be seen building
/// without ever competing with what it builds to.
pub(super) const CHARGE_LIGHT_LUMENS: f32 = 150_000.0;
/// How far the charge glow reaches, in world units (200 m). Enough to light
/// the hull the lance is bolted to and no further - the charge is the firing
/// ship's business until the shot makes it everyone's.
const CHARGE_LIGHT_RANGE: f32 = 20.0;
/// Exponent the charge fraction is raised to before it drives the glow.
///
/// CUBED, so the bore is still nearly dark at half charge and most of the
/// light arrives in the last third of a second. A linear ramp reads as a gun
/// that is always warm; this one reads as a gun that is ABOUT TO GO OFF, which
/// is the thing the pilot and the target both need off one glance.
const CHARGE_LIGHT_CURVE: i32 = 3;

/// Sparks thrown off the brake on the shot: the arc flash of a bank dumping
/// into a bore.
const MUZZLE_SPARK_COUNT: u32 = 28;
/// Camera trauma the player's own lance puts through the hull when it fires.
///
/// Between the juice layer's hit and destroy kicks: bigger than being shot,
/// smaller than something dying, because that is what firing this gun is.
const FIRE_TRAUMA: f32 = 0.45;

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
                nose_cone_mesh(SLUG_RADIUS, SLUG_BODY, SLUG_NOSE)
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

/// Longest a slug's streak may be drawn, in world units (400 m).
///
/// The PDC's 4.0 is sized for a round authored at 1000 m/s; this one leaves
/// the bore at 15 km/s and would be clamped to a quarter of its own body. At
/// 60 fps the shutter asks for about 9 units and at 30 fps about 18, so this
/// only ever catches a real stall - which is what the clamp is for.
const SLUG_TRACER_MAX_LENGTH: f32 = 40.0;

/// Dress a fired slug.
///
/// The art goes in a CHILD, not on the round itself, because that is what
/// [`stretch_round_tracers`] needs: it scales and slides the art back along
/// the flight, and the round's own transform is the thing the sweep writes.
///
/// The streak is the point. A slug crosses roughly 25 units between two drawn
/// frames at 60 fps against an 0.8 unit body, so without it the one shot a
/// lance gets every thirteen seconds is drawn as a handful of disconnected
/// darts and the player never sees the line it made.
pub(super) fn insert_railgun_slug_render(
    add: On<Add, RailgunSlugProjectileMarker>,
    mut commands: Commands,
    art: Res<RailgunSlugArt>,
    budget: Option<Res<GraphicsBudget>>,
    mut wake: RailgunWakeSpawner,
    q_slug: Query<&Transform, With<RailgunSlugProjectileMarker>>,
) {
    let entity = add.entity;
    commands.entity(entity).insert(children![(
        Name::new("Railgun Slug Render"),
        Mesh3d(art.mesh.clone()),
        MeshMaterial3d(art.material.clone()),
        RoundTracer {
            half_length: (SLUG_BODY + SLUG_NOSE) * 0.5,
            max_length: SLUG_TRACER_MAX_LENGTH,
        },
    )]);

    // The light asks for its slot at the flush; the wake is spawn-less on the
    // Low tier, like every other particle effect. Absent budget (a
    // settings-less app) means full quality.
    light_railgun_slug(&mut commands, entity);
    if budget.as_deref().is_none_or(|budget| budget.particles) {
        let transform = q_slug.get(entity).copied().unwrap_or_default();
        wake.spawn(
            &mut commands,
            entity,
            transform,
            RailgunWakeTuning::default(),
        );
    }
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

    // The glow is spawned DARK and never despawned. A lance charges many times
    // a fight, and a light that is only ever a brightness and a position is
    // cheaper to leave in place than to build on every commit. A preview
    // section has no `RailgunCharge`, so nothing ever drives it and it stays
    // exactly this: off.
    commands.entity(entity).with_child((
        Name::new("Railgun Charge Glow"),
        RailgunChargeGlowMarker,
        Transform::default(),
        Visibility::Hidden,
        PointLight {
            color: RAIL_GLOW_COLOR,
            intensity: 0.0,
            range: CHARGE_LIGHT_RANGE,
            radius: 0.0,
            // The glow rides INSIDE the bore of the gun that owns it, so a
            // shadow map here would spend a cascade on the barrel occluding
            // it and light nothing else differently.
            shadow_maps_enabled: false,
            ..default()
        },
    ));
}

/// The travelling light that IS the charge: it rides the bore from the breech
/// to the brake as the capacitors fill, and arrives exactly where the muzzle
/// flash is about to be.
///
/// One per lance, spawned with the body. See [`drive_railgun_charge_glow`].
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub(super) struct RailgunChargeGlowMarker;

/// Ride every lance's charge glow up its bore and brighten it as the shot
/// nears.
///
/// The line comes from [`RailgunSectionConfig::muzzle_offset`] and NOT from the
/// art, so a modded lance with a longer bore gets a glow that ends at its OWN
/// brake. That also means the last frame of the charge puts this light on the
/// exact point [`on_railgun_fired_flash`] is about to fire from: the charge
/// does not cut to the flash, it becomes it.
///
/// The authored `charge_bolt` node walks the same stretch on the same clock
/// (see `lance_charge_bolt` in `nova_authoring`), so this reads as that bolt's
/// corona rather than as a second object.
pub(super) fn drive_railgun_charge_glow(
    // A DISABLED lance is skipped by `charge_and_fire_railgun`, so its
    // `RailgunCharge` freezes wherever it stood. Without this filter the bore
    // of a dead gun stays lit at that instant for the rest of the scenario,
    // while the sight and the capacitor loop - which both filter - go away.
    q_railgun: Query<
        (&RailgunCharge, &RailgunSectionConfigHelper, &Children),
        Without<SectionInactiveMarker>,
    >,
    mut q_glow: Query<
        (&mut Transform, &mut Visibility, &mut PointLight),
        With<RailgunChargeGlowMarker>,
    >,
) {
    for (charge, config, children) in &q_railgun {
        let progress = charge.progress(config.charge_seconds);
        for &child in children {
            let Ok((mut transform, mut visibility, mut light)) = q_glow.get_mut(child) else {
                continue;
            };
            transform.translation = config.muzzle_offset * progress;
            light.intensity = CHARGE_LIGHT_LUMENS * progress.powi(CHARGE_LIGHT_CURVE);
            *visibility = if progress > 0.0 {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
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
        color: RAIL_GLOW_COLOR,
        peak_intensity: MUZZLE_LIGHT_LUMENS,
        range: MUZZLE_LIGHT_RANGE,
        duration: MUZZLE_LIGHT_SECS,
    });
}

/// Throw the bore's arc flash, and kick the camera when the lance that fired
/// is the PLAYER'S OWN.
///
/// The sparks are unconditional: an enemy lance discharging is exactly the cue
/// a pilot needs, and perspective already thins a distant burst.
///
/// The kick is not. It is scoped to the player's ship rather than
/// distance-attenuated the way `nova_gameplay::juice` attenuates a hit,
/// because this is not a far event that happened to be close - it is a
/// capacitor bank bolted to the hull the camera is riding. Somebody else's
/// lance is felt when it LANDS, which the juice path already covers.
pub(super) fn on_railgun_fired_kick(
    fired: On<RailgunFired>,
    mut commands: Commands,
    q_gun: Query<&ChildOf, With<RailgunSectionMarker>>,
    q_player: Query<(), (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
    mut q_shake: Query<&mut CameraShakeInput, With<SfxListenerMarker>>,
) {
    trace!("on_railgun_fired_kick: railgun {:?}", fired.entity);
    commands.trigger(ImpactSparks {
        at: fired.muzzle,
        count: MUZZLE_SPARK_COUNT,
        force: 1.0,
    });

    let Ok(&ChildOf(spaceship)) = q_gun.get(fired.entity) else {
        return;
    };
    if !q_player.contains(spaceship) {
        return;
    }
    for mut shake in &mut q_shake {
        shake.add_trauma += FIRE_TRAUMA;
    }
}
