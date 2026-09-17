//! The torpedo bays, and the ordnance TYPES they load.
//!
//! A bay is the tube: the art, the cadence, the warhead, the six-round rack and
//! its idle reload. All of that is identical across both shipped bays, so the
//! only argument between them is which type is in the tube.
//!
//! A type is defined by how the torpedo FLIES. Blast damage, warhead toughness,
//! blast radius and magazine all stay on the bay, and the two shipped types are
//! deliberately identical in every one of them, which
//! `sections::ordnance_tests::torpedo_types_differ_only_in_how_the_ordnance_flies`
//! keeps true. What a type owns is the run-in - the weave, and the cruise cap
//! that pays for it:
//!
//! | | Lance | Serpent |
//! |---|---|---|
//! | cruise cap | 350 m/s | 320 m/s |
//! | weave half-angle | 0.00 rad | 0.44 rad |
//! | rounds one PDC spends to stop it | ~116 | ~390 |
//! | where that PDC finally kills it | ~1.14 km out | ~400 m out |
//! | time over a 3 km run-in | 9.10 s | 9.78 s |
//! | speed along the line | 313 m/s | 291 m/s |
//! | reach at the bay's 100 s lifetime | 31.3 km | 29.1 km |
//!
//! So the Lance is the torpedo you fire at something that will not shoot back:
//! it arrives ~7% sooner, and against a target RUNNING at the player's 150 m/s
//! speed cap it closes at 163 m/s where a Serpent manages 141 - ~16% faster.
//! And a defender meeting Lances is a defender whose point defense WORKS.
//!
//! **The cruise cap is AUTHORED as that price, because the weave does not pay
//! for itself.** A corkscrew is a longer path, so an evasive torpedo was
//! expected to arrive later for free. Measured on the real body it does not:
//! the path stretch is ~1.7% rather than the ideal 11%, and thrust is capped on
//! the ALONG-NOSE speed, so a weaving torpedo never reaches the taper band,
//! keeps its engine lit and settles FASTER. Give the Serpent the Lance's cap
//! and it arrives 1.5% SOONER, longer path and all - which is the control arm
//! in `nova_ship`'s
//! `bay::tests::evasion_costs_time_because_the_type_is_slower_not_because_the_path_is_longer`.
//! A weaving torpedo has to BE slower rather than fly just as fast behind a
//! shorter timer, so the price is the cap and not `projectile_lifetime`.

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_ship::prelude::*;

use crate::base_content::assets::BaseContentAssets;

/// Torpedo bay: the mid durability baseline (`nova_gameplay::damage` scales a
/// hit by section class, and Kinetic is 1.0 against every class, so this is
/// what a generalist round meets).
pub(super) const TORPEDO_BASE_HEALTH: f32 = 100.0;

/// The Lance bay's id. The Serpent bay is named by the generator, so its id
/// comes from `nova_ship` instead.
const LANCE_TORPEDO_SECTION_ID: &str = "lance_torpedo_section";

/// The node prefix of the iris petals inside `bay_tube.glb`. Producer and test
/// read the same name, so a renamed node fails loudly.
const DOOR_PETAL_NODE_PREFIX: &str = "door_petal_";

/// The bay's footprint: one cell across, two cells long down the firing axis.
/// The promoted tube art (`bay_tube.glb`) is drawn at this size, muzzle
/// toward -Z.
const BAY_CELLS: Vec3 = Vec3::new(1.0, 1.0, 2.0);

/// Cruise cap of the straight-running type, and the reference every other
/// torpedo speed here is quoted against.
const LANCE_MAX_SPEED: MetersPerSecond = MetersPerSecond(350.0);

/// The bay's sockets: the back plate, and one per cell on each flank - nine
/// in all, every closed face flush with the unit grid.
///
/// The muzzle face (-Z) carries NONE. A hole in the art is not a mating
/// surface: the face a bay fires through has no structure to bolt to, and
/// leaving it socketed let a builder plate over the muzzle - the section
/// mated, the salvo then launched inside its own ship. Dropping the socket is
/// what makes that placement impossible rather than merely unwise.
/// `no_bay_sockets_the_face_it_fires_through` holds every bay to it.
fn bay_link_points() -> Vec<LinkPoint> {
    let mut points = vec![LinkPoint {
        id: "positive_z".to_string(),
        position: Vec3::Z * (BAY_CELLS.z * 0.5),
        normal: Vec3::Z,
    }];
    let flanks = [
        ("positive_x", Vec3::X),
        ("negative_x", Vec3::NEG_X),
        ("positive_y", Vec3::Y),
        ("negative_y", Vec3::NEG_Y),
    ];
    // Fore is the muzzle cell; a socket per cell is what lets a neighbouring
    // unit section mate against either half of the tube.
    for (face, normal) in flanks {
        for (cell, z) in [("fore", -0.5), ("aft", 0.5)] {
            points.push(LinkPoint {
                id: format!("{face}_{cell}"),
                position: normal * 0.5 + Vec3::Z * z,
                normal,
            });
        }
    }
    points
}

/// The bay's muzzle-door track: the six iris petals modelled into
/// `bay_tube.glb` as the named nodes `door_petal_0..5`, folded outward on
/// their authored hinges. One authored fact shared by every bay that uses
/// the tube art, which is all of them.
///
/// The bay's fire path steers the `MuzzleDoor` cue across the cold-coast
/// window (`ignition_delay`, 0.6 s): the iris must be fully open well
/// before the drive lights, so it opens fast, and it closes at an
/// unhurried service pace once the torpedo is away.
fn bay_muzzle_door() -> Vec<SectionAnimation> {
    vec![SectionAnimation {
        cue: SectionAnimationCue::MuzzleDoor,
        node_prefix: DOOR_PETAL_NODE_PREFIX.to_string(),
        // Past vertical, so the open petals read as a flared crown around
        // the dark throat rather than six posts.
        motion: SectionAnimationMotion::RotateX { degrees: 105.0 },
        open_seconds: 0.25,
        close_seconds: 0.7,
    }]
}

/// One assault torpedo bay, named for the ORDNANCE it loads.
///
/// Everything a bay is - the tube art, the cadence, the warhead, the rack and
/// its idle reload - is identical across both shipped bays, so the only argument
/// between them is `torpedo_type` and the trade it carries. Keeping them one
/// builder is what makes that true by construction rather than by review: a
/// balance edit here lands on both types at once and cannot quietly become a
/// second, unmeasured difference.
fn torpedo_bay_prototype(
    meshes: &BaseContentAssets,
    id: &str,
    name: &str,
    description: &str,
    torpedo_type: TorpedoTypeConfig,
) -> SectionConfig {
    SectionConfig {
        base: BaseSectionConfig {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            // Torpedo bay: mid durability baseline.
            health: TORPEDO_BASE_HEALTH,
            destroy_sound: Some(meshes.section_destroy_sound.clone()),
            // Two cells long: the tube art earns its length, and the collider
            // has to claim it or half the bay would be a ghost.
            collider: Some(SectionCollider::Cuboid { size: BAY_CELLS }),
            // Back and flanks only: the tube fires out of -Z
            // (`spawn_offset`), so the bow face is the open muzzle.
            link_points: bay_link_points(),
            hide_in_editor: false,
            // A launcher is loading machinery: it arcs and sparks as it fails,
            // and the tube it fires down stays a tube.
            damage_effects: DamageEffects(vec![DamageEffect::Cracks, DamageEffect::Sparks]),
            // The iris petals modelled into the tube art: see `bay_muzzle_door`.
            animations: bay_muzzle_door(),
        },
        kind: SectionKind::Torpedo(TorpedoSectionConfig {
            render_mesh: Some(meshes.torpedo_bay.clone()),
            render_mesh_transform: None,
            projectile_render_mesh: None,
            // The muzzle point, ON the door plane at -1: the launch flash and
            // the spatial launch sound belong at the iris, not out in space.
            spawn_offset: Vec3::NEG_Z * BAY_CELLS.z * 0.5,
            // Aim the tube out of the open face. The launch axis is
            // the spawner's +Y and this turns it onto the section's -Z,
            // the one face `link_points` leaves unlinkable so it can be
            // a muzzle. Without it the tube ejected through its own roof.
            spawn_rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
            // Born at the tube's centre: the 20 m torpedo exactly fills the
            // 20 m tube - nose on the door plane, tail at the back wall - and
            // slides its whole length out through the open iris.
            spawn_recess: BAY_CELLS.z * 0.5,
            fire_rate: 1.0,
            spawner_speed: MetersPerSecond(80.0),
            projectile_lifetime: 100.0,
            arm_time: 0.5,
            arm_distance: Meters(50.0),
            // Dropped, then lit: the bay ejects it on a cold charge and the
            // motor catches once it is clear. See `ignition_delay`.
            ignition_delay: 0.6,
            nav_constant: 3.0,
            linear_damping: 0.8,
            blast_radius: Meters(300.0),
            // A direct hit can decide a small-craft fight, while structural
            // depth stops it from deleting a capital. Each destroyed section
            // transmits 65% of the remaining pressure; a surviving section
            // shields everything behind it.
            //
            // EQUAL ACROSS TYPES: a torpedo type decides how the ordnance
            // flies, never how hard it lands.
            blast_damage: 750.0,
            blast_effect: None,
            launch_effect: None,
            launch_sound: Some(meshes.torpedo_launch_sound.clone()),
            door_sound: Some(meshes.torpedo_door_sound.clone()),
            // The blast reuses the section-destruction wav. Authoring is
            // per target, so the two can diverge.
            detonation_sound: Some(meshes.torpedo_detonation_sound.clone()),
            // Above the hardest single PDC round (4.0 authored x the 2.0
            // Kinetic speed ceiling), so an intercept costs two or three
            // rounds instead of one lucky tap.
            projectile_health: 10.0,
            // The whole difference between the two shipped bays: see this
            // module's header for what each type costs and buys.
            torpedo_type,
            // The rack, and the alpha strike it buys: six away in six
            // seconds at the fire rate above. Saturation is what beats
            // point defense - attrition never does - so the burst is the
            // attacker's weapon and the reload below is only its floor.
            ammunition: AmmoCapacity::Limited(6),
            // Idle batch reload, not a hard magazine. Every launch resets the
            // delay; ten quiet seconds return one torpedo. A six-round rack is
            // therefore the alpha strike, followed by visible rearm cadence.
            //
            // One shipped PDC sustains 200 / (3 + 200/100) = 40 rounds/s when
            // each returned batch is fired immediately. At 369 rounds per
            // weaving intercept it answers 0.108 torpedoes/s, narrowly above
            // this bay's 0.1/s idle supply. The attacker wins by mounting more
            // bays than the defender has PDCs, never by waiting one mount out.
            // `no_torpedo_bay_out_sustains_a_point_defense_mount` pins it.
            reload: ReloadConfig::Batch(SectionReloadConfig {
                delay: 10.0,
                amount: 1,
            }),
        }),
    }
}

/// **Lance** - the straight-running bombardment torpedo: no weave at all, so
/// it flies the bare proportional-navigation intercept.
///
/// The cheaper torpedo to stop, and it does not pretend otherwise: a lead
/// solution solves a straight line exactly, so point defense kills it a full
/// envelope out instead of on its own doorstep. What it buys is the fastest
/// cruise of any assault torpedo and the shortest path to the target, which is
/// most of a second over a 3 km run and half again the closing speed on
/// anything running away.
fn lance() -> TorpedoTypeConfig {
    TorpedoTypeConfig {
        name: "Lance".to_string(),
        // Pale steel: cold, plain, unmistakably not the hot ordnance below.
        tint: Color::srgb(0.7, 0.78, 0.86),
        max_speed: LANCE_MAX_SPEED,
        // The bare intercept. The rate is unread at zero amplitude and is
        // authored zero so the config says "no weave" rather than "a weave
        // nobody spins".
        weave_angle: 0.0,
        weave_rate: 0.0,
    }
}

/// **Serpent** - the assault torpedo: the terminal weave that costs a defender
/// roughly three times the ammunition per intercept, bought with a cruise cap
/// 30 m/s under the [`lance`]'s.
///
/// The same warhead as the Lance, flown differently and more slowly. See
/// [`TorpedoTypeConfig`] for the sweep behind both numbers; this is the engine
/// default, so an un-authored bay carries the ordnance the rest of the combat
/// model is balanced against.
fn serpent() -> TorpedoTypeConfig {
    TorpedoTypeConfig::default()
}

/// The bay prototypes, in catalog order.
pub(super) fn prototypes(meshes: &BaseContentAssets) -> Vec<SectionConfig> {
    vec![
        torpedo_bay_prototype(
            meshes,
            TORPEDO_SECTION_ID,
            "Torpedo Bay (Serpent)",
            "The standard bay, loaded with Serpent assault torpedoes. They run \
             in on a terminal weave, so point defense spends roughly three \
             times the rounds to stop one and only kills it on the doorstep. \
             The corkscrew is a longer path: a Serpent arrives later than a \
             Lance and gains less on a target that is running.",
            serpent(),
        ),
        torpedo_bay_prototype(
            meshes,
            LANCE_TORPEDO_SECTION_ID,
            "Torpedo Bay (Lance)",
            "The same bay and the same warhead, loaded with Lance bombardment \
             torpedoes: no weave, the bare intercept. The shortest path there \
             is - it arrives soonest and keeps closing on a runner - which is \
             also the path point defense is built to solve. Ordnance for a \
             target that will not shoot back.",
            lance(),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every catalog bay that shows the tube art declares the muzzle-door
    /// track, and the track's node prefix matches real named nodes in the
    /// committed `bay_tube.glb` - the authored declaration and the art are
    /// one contract, so a renamed petal node or a dropped track breaks here
    /// rather than as doors that silently stop moving.
    #[test]
    fn every_tube_bay_declares_the_muzzle_door_over_the_iris_nodes() {
        let glb = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/base/gltf/bay_tube.glb"
        ))
        .expect("the promoted bay tube art");
        for petal in 0..6 {
            let name = format!("\"{DOOR_PETAL_NODE_PREFIX}{petal}\"");
            assert!(
                glb.windows(name.len())
                    .any(|window| window == name.as_bytes()),
                "bay_tube.glb has no {name} node"
            );
        }

        for section in crate::generation::build_section_catalog() {
            let SectionKind::Torpedo(bay) = &section.kind else {
                continue;
            };
            let tube_art = bay
                .render_mesh
                .as_ref()
                .and_then(|mesh| mesh.path())
                .is_some_and(|path| path.contains("bay_tube.glb"));
            if !tube_art {
                continue;
            }
            let door = section
                .base
                .animations
                .iter()
                .find(|track| track.cue == SectionAnimationCue::MuzzleDoor)
                .unwrap_or_else(|| {
                    panic!("{}: tube bay without a MuzzleDoor track", section.base.id)
                });
            assert_eq!(
                door.node_prefix, DOOR_PETAL_NODE_PREFIX,
                "{}",
                section.base.id
            );
            // The ejection waits for the open iris, so `open_seconds` is the
            // whole first-shot delay: authored positive (the door genuinely
            // travels) and under the fire interval (the door never becomes
            // the bay's real rate of fire).
            assert!(
                door.open_seconds > 0.0 && door.open_seconds < 1.0 / bay.fire_rate,
                "{}: the door gate must be shorter than the fire interval",
                section.base.id
            );
        }
    }

    /// A turret a builder can PLACE stands on the section it mounts through:
    /// its turntable sits on that section's own bottom face, not on a unit
    /// cube's.
    ///
    /// The joint tree hardcoded the unit cube's -0.5, so the compact PDC - a
    /// 0.3 mount box - planted its turntable 0.35 below its own underside and
    /// sank the gun into the hull it was bolted to. Checked over the catalog
    /// rather than over that one section, because the next mount authored at
    /// its own size would repeat it.
    ///
    /// No bay offers a mating surface across the face it fires through.
    ///
    /// A torpedo section carrying the plain hull block's full six-socket cube lets
    /// the editor bolt a section over the muzzle: the placement mates, and the
    /// salvo then launches inside its own ship. The rule is the
    /// SOCKET SET, so it is checked over every torpedo section in the catalog
    /// rather than over the one that prompted it - and the firing direction is
    /// read off each section's own `spawn_offset` rather than assumed, so a bay
    /// authored to fire some other way is held to the same rule.
    #[test]
    fn no_bay_sockets_the_face_it_fires_through() {
        for section in crate::generation::build_section_catalog() {
            let SectionKind::Torpedo(bay) = &section.kind else {
                continue;
            };
            let firing = bay.spawn_offset.normalize();
            for point in &section.base.link_points {
                assert!(
                    point.normal.dot(firing) < 0.5,
                    "`{}` sockets its own muzzle: `{}` faces {:?}, the bay fires {firing:?}",
                    section.base.id,
                    point.id,
                    point.normal,
                );
            }
        }
    }

    /// The promoted bay is a 1x1x2 tube. Its collider has to claim both cells
    /// (or half the bay is a ghost the editor and weapons fire through), its
    /// muzzle point has to sit ON the door plane (the launch flash and sound
    /// play at the spawner, and the door-gated ejection means the flash
    /// belongs at the iris), and its recess has to birth the torpedo INSIDE
    /// the tube so it slides out through the open door. Checked over every
    /// bay THIS module builds - the ships' own torpedo pods author their own
    /// hull-fitted shapes and are not the tube.
    #[test]
    fn every_bay_claims_both_cells_and_births_its_torpedo_inside_the_tube() {
        let catalog = crate::generation::build_section_catalog();
        for id in [TORPEDO_SECTION_ID, LANCE_TORPEDO_SECTION_ID] {
            let section = catalog
                .iter()
                .find(|section| section.base.id == id)
                .unwrap_or_else(|| panic!("the catalog ships `{id}`"));
            let SectionKind::Torpedo(bay) = &section.kind else {
                panic!("`{id}` is a torpedo bay");
            };
            assert_eq!(
                section.base.collider,
                Some(SectionCollider::Cuboid { size: BAY_CELLS }),
                "`{id}` claims its two cells"
            );
            assert_eq!(
                bay.spawn_offset.z,
                -BAY_CELLS.z * 0.5,
                "`{id}` puts its muzzle point off the door plane"
            );
            let birth = bay.spawn_offset.z + bay.spawn_recess;
            assert!(
                bay.spawn_recess > 0.0 && birth.abs() <= BAY_CELLS.z * 0.5,
                "`{id}` births its torpedo at {birth} - outside its own tube"
            );
            // Nine sockets: the back plate, and one per cell on each flank so
            // unit neighbours mate against either half of the tube.
            assert_eq!(section.base.link_points.len(), 9, "`{id}` sockets");
            for point in &section.base.link_points {
                assert!(
                    (point.position.dot(point.normal) - point.normal.abs().dot(BAY_CELLS) * 0.5)
                        .abs()
                        < 1e-6,
                    "`{id}` socket `{}` floats off its face",
                    point.id
                );
            }
        }
    }
}
