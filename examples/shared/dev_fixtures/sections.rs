//! Fixture section prototypes: the two siege weapons the capital warship
//! fixture mounts.
//!
//! Neither ships in the base catalog. They are deliberately outside the
//! balanced numbers - a lance priced to cross a capital hull the long way and
//! a torpedo point defense cannot chew through - so they exist only here,
//! authored INLINE on the hull that carries them and never offered by the
//! section drawer.
//!
//! Art paths are the base bundle's, spelled as the merge resolves them: a
//! fixture is not bundle content, so `self://` would not rewrite.

use bevy::prelude::*;
use nova_protocol::prelude::*;

/// The bay's footprint: one cell across, two cells long down the firing axis.
/// The tube art is drawn at this size, muzzle toward -Z.
const BAY_CELLS: Vec3 = Vec3::new(1.0, 1.0, 2.0);

/// The lance's footprint: capacitor casing aft, bore and brake fore.
const LANCE_CELLS: Vec3 = Vec3::new(1.0, 1.0, 3.0);

/// The section health both siege weapons carry: the catalog railgun's, so the
/// fixture guns are no harder to shoot off than the balanced one.
const SIEGE_SECTION_HEALTH: f32 = 180.0;

fn art(path: &str) -> AssetRef<WorldAsset> {
    AssetRef::from(format!("base/gltf/{path}"))
}

fn sound(name: &str) -> AssetRef<AudioSource> {
    AssetRef::from(format!("base/sounds/{name}.wav"))
}

/// The capital-grade spinal lance.
///
/// The same weapon as the catalog's `railgun_lance_section` - same commit,
/// same recoil, same single shell over a twelve-second reload - at three
/// changed numbers: what the slug costs a layer, how deep it goes, and how
/// wide a corridor it spends that depth on.
pub fn siege_railgun_lance() -> SectionConfig {
    SectionConfig {
        base: BaseSectionConfig {
            id: "fixture_siege_railgun_lance".to_string(),
            name: "Siege Railgun Lance".to_string(),
            description: "A capital-grade spinal lance: 500 Pierce to every \
                          layer it rakes, a 360,000 pierce budget and a 30 m \
                          bore. Fixture ordnance, outside the balanced \
                          catalog."
                .to_string(),
            health: SIEGE_SECTION_HEALTH,
            destroy_sound: Some(sound("explosion")),
            // Three cells long, and the collider has to claim all of it.
            collider: Some(SectionCollider::Cuboid { size: LANCE_CELLS }),
            link_points: lance_link_points(),
            hide_in_editor: false,
            damage_effects: DamageEffects(vec![DamageEffect::Cracks, DamageEffect::Sparks]),
            animations: lance_charge_bolt(),
        },
        kind: SectionKind::Railgun(RailgunSectionConfig {
            render_mesh: Some(art("railgun_lance.glb#Scene0")),
            render_mesh_transform: None,
            // ON the brake face: the recoil is applied at this point, so a
            // lance bolted off the ship's axis yaws it as well as pushing it.
            muzzle_offset: Vec3::NEG_Z * (LANCE_CELLS.z * 0.5),
            charge_seconds: 1.5,
            slug_speed: MetersPerSecond(15_000.0),
            // Enough to clear the two large drives (480 and 1250) in a shot
            // or two rather than several.
            slug_damage: 500.0,
            // Priced to cross a capital hull the long way and keep going, so
            // a hull's depth stops being the thing that saves it.
            slug_power: 360_000.0,
            // Three units: on the unit lattice this takes the second ring of
            // neighbours as well, roughly seven cells across. Widening only
            // pays because `slug_power` is no longer the binding constraint.
            rake_radius: Some(Meters(30.0)),
            slug_lifetime: 1.2,
            recoil_impulse: 45.0,
            fire_sound: Some(sound("railgun_fire")),
            charge_sound: Some(sound("railgun_charge")),
            reload_sound: Some(sound("railgun_reload")),
            ammunition: AmmoCapacity::Limited(1),
            reload: ReloadConfig::Batch(SectionReloadConfig {
                delay: 12.0,
                amount: 1,
            }),
        }),
    }
}

/// The capital siege torpedo bay: the same tube art at siege grade, loaded
/// with the armored Breaker warhead.
pub fn siege_torpedo_bay() -> SectionConfig {
    SectionConfig {
        base: BaseSectionConfig {
            id: "fixture_siege_torpedo_bay".to_string(),
            name: "Siege Torpedo Bay".to_string(),
            description: "A capital siege battery: slow salvo, armored \
                          ordnance, ship-killing blast. Fixture ordnance, \
                          outside the balanced catalog."
                .to_string(),
            health: 100.0,
            destroy_sound: Some(sound("explosion")),
            collider: Some(SectionCollider::Cuboid { size: BAY_CELLS }),
            link_points: bay_link_points(),
            hide_in_editor: false,
            damage_effects: DamageEffects(vec![DamageEffect::Cracks, DamageEffect::Sparks]),
            animations: bay_muzzle_door(),
        },
        kind: SectionKind::Torpedo(TorpedoSectionConfig {
            render_mesh: Some(art("bay_tube.glb#Scene0")),
            render_mesh_transform: None,
            projectile_render_mesh: None,
            // The muzzle point on the door plane and the centred birth: it
            // launches out of the same tube the catalog bays do.
            spawn_offset: Vec3::NEG_Z * BAY_CELLS.z * 0.5,
            // The launch axis is the spawner's +Y; this turns it onto the
            // section's -Z, the one face `link_points` leaves unlinkable.
            spawn_rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
            spawn_recess: BAY_CELLS.z * 0.5,
            fire_rate: 1.0,
            spawner_speed: MetersPerSecond(80.0),
            // Long enough to cross a fixture arena, short enough that a
            // torpedo whose target died mid-flight cleans itself up.
            projectile_lifetime: 60.0,
            arm_time: 0.5,
            arm_distance: Meters(50.0),
            // Dropped, then lit: the bay ejects it on a cold charge and the
            // motor catches once it is clear.
            ignition_delay: 0.6,
            nav_constant: 4.0,
            linear_damping: 0.4,
            // The same 65% transmission rule as every Explosive blast, but
            // 2000 at the centre stays lethal through more structural layers
            // than the catalog bay's 750.
            blast_radius: Meters(450.0),
            blast_damage: 2000.0,
            blast_effect: None,
            launch_effect: None,
            launch_sound: Some(sound("torpedo_launch")),
            door_sound: Some(sound("bay_door")),
            detonation_sound: Some(sound("torpedo_detonate")),
            // Armored ordnance: a PDC burst (~800 DPS) cannot chew through
            // this inside the ~6 s closing window, so point defense visibly
            // hammers it and still loses.
            projectile_health: 5000.0,
            torpedo_type: breaker(),
            ammunition: AmmoCapacity::Limited(6),
            reload: ReloadConfig::Batch(SectionReloadConfig {
                delay: 10.0,
                amount: 1,
            }),
        }),
    }
}

/// **Breaker** - the capital siege warhead's flight.
///
/// Half the Serpent's weave amplitude at twice the catalog Lance's cruise, so
/// the path swings about as wide (the swing scales with both) while the
/// closing window through a point-defense envelope stays short.
pub fn breaker() -> TorpedoTypeConfig {
    TorpedoTypeConfig {
        name: "Breaker".to_string(),
        // Deep crimson, distinct from both catalog types in flight.
        tint: Color::srgb(0.75, 0.1, 0.12),
        max_speed: MetersPerSecond(700.0),
        weave_angle: 0.22,
        weave_rate: 1.4,
    }
}

/// The bay's sockets: the back plate, and one per cell on each flank - nine
/// in all, every closed face flush with the unit grid.
///
/// The muzzle face (-Z) carries NONE. A socket there lets a builder plate over
/// the muzzle: the section mates, and the salvo then launches inside its own
/// ship.
fn bay_link_points() -> Vec<LinkPoint> {
    axial_link_points(BAY_CELLS.z, &[("fore", -0.5), ("aft", 0.5)])
}

/// The lance's sockets: the breech plate, and one per cell on each flank -
/// thirteen in all. The muzzle face carries none, for the bay's reason.
fn lance_link_points() -> Vec<LinkPoint> {
    axial_link_points(LANCE_CELLS.z, &[("fore", -1.0), ("mid", 0.0), ("aft", 1.0)])
}

/// The socket set shared by both weapons: one breech plate on +Z, then one
/// socket per cell on each of the four flanks. Nothing on -Z, which is the
/// muzzle.
fn axial_link_points(length: f32, cells: &[(&str, f32)]) -> Vec<LinkPoint> {
    let mut points = vec![LinkPoint {
        id: "positive_z".to_string(),
        position: Vec3::Z * (length * 0.5),
        normal: Vec3::Z,
    }];
    for (face, normal) in [
        ("positive_x", Vec3::X),
        ("negative_x", Vec3::NEG_X),
        ("positive_y", Vec3::Y),
        ("negative_y", Vec3::NEG_Y),
    ] {
        for (cell, z) in cells {
            points.push(LinkPoint {
                id: format!("{face}_{cell}"),
                position: normal * 0.5 + Vec3::Z * *z,
                normal,
            });
        }
    }
    points
}

/// The lance's charge track: the `charge_bolt` node modelled into
/// `railgun_lance.glb`, walked from the breech end of the bore to the muzzle
/// brake.
///
/// Both travel times are zero, and that is deliberate: the firing system SNAPS
/// this track to the gameplay charge fraction every tick, so `charge_seconds`
/// is the single clock.
fn lance_charge_bolt() -> Vec<SectionAnimation> {
    vec![SectionAnimation {
        cue: SectionAnimationCue::Charge,
        node_prefix: "charge_bolt".to_string(),
        // Breech (-0.06) to the back of the muzzle brake (-1.24).
        motion: SectionAnimationMotion::Translate {
            offset: Vec3::NEG_Z * 1.18,
        },
        open_seconds: 0.0,
        close_seconds: 0.0,
    }]
}

/// The bay's muzzle-door track: the six iris petals modelled into
/// `bay_tube.glb` as the named nodes `door_petal_0..5`.
///
/// The iris must be fully open before the drive lights at `ignition_delay`
/// (0.6 s), so it opens fast and closes at a service pace.
fn bay_muzzle_door() -> Vec<SectionAnimation> {
    vec![SectionAnimation {
        cue: SectionAnimationCue::MuzzleDoor,
        node_prefix: "door_petal_".to_string(),
        motion: SectionAnimationMotion::RotateX { degrees: 105.0 },
        open_seconds: 0.25,
        close_seconds: 0.7,
    }]
}
