//! The base game's ship section catalog: the stat block behind every hull,
//! thruster, controller, turret and torpedo section the player can mount.
//!
//! [`standard_section_prototypes`] is the single source of the shipped numbers, and the
//! tests pin the balance relations between them (durability ordering,
//! rounds-to-kill ceilings) so a retune cannot silently invert them.
//!
//! Touch this module when adding a section kind or retuning section stats.

use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;

use super::ordnance;

// Per-section-type durability baselines.
//
// Section TYPE governs how much damage a section effectively takes. These are
// the HEALTH half of that: the per-damage-type half is nova's resistance table
// (`nova_gameplay::damage`), which scales a hit by `(section class, damage
// type)`. Kinetic is 1.0 against every class there, so these numbers are what
// a generalist round meets.
//
// Thrusters are exposed propulsion and go down fast (take MORE); turrets are
// armored weapon mounts and shrug off MORE (take LESS); the controller core and
// the torpedo bay sit at the mid baseline. Direction follows the task title and
// is a playtest knob - flipping "fragile vs tough" is a one-line change here.
// Per-section variants (a reinforced hull, a light hull) may deviate from their
// type baseline on purpose; these are the values they start from.
const THRUSTER_BASE_HEALTH: f32 = 70.0;
const CONTROLLER_BASE_HEALTH: f32 = 100.0;
const TURRET_BASE_HEALTH: f32 = 130.0;
const TORPEDO_BASE_HEALTH: f32 = 100.0;

// Authored per-hit Kinetic damage of the shared PDC, a playtest knob. A
// point-defense profile: LOW per-hit, HIGH rate (100 rounds/s). At 4.0 the PDC
// does ~400 DPS, while a 60-HP scavenger section takes 15 rounds (~0.15 s of
// fire) instead of 3, so a burst visibly chips it down rather than popping it
// in a blink (playtest: "PDC destroys asteroids/objects with one bullet"). Was
// ~20.25 (the old emergent per-hit); the drop also slows ship TTK ~5x,
// consistent with a PDC and with the shakedown pirate still dying in a short
// burst.
//
// Rock is NOT priced here. An asteroid has no health pool since the carve pass:
// its durability is the material left in it, priced at `DAMAGE_PER_UNIT_VOLUME`
// (nova_gameplay::integrity::carve), which is calibrated AGAINST this number.
// Moving this moves every rock's time-to-kill with it.
//
// Every craft now mounts this one gun, so this number is the whole gunnery
// curve: a raider is made weaker by its HULL and its mount's health, not by a
// second, softer turret prototype.
const KINETIC_PDC_BULLET_DAMAGE: f32 = 4.0;

/// Authored per-hit damage of the Pierce PDC: HALF the Kinetic one.
///
/// The trade the two guns exist to show. A penetrator deals this to every layer
/// it rakes through and its damage never depletes, so against one thin target it
/// is strictly worse (2 vs 4, and it cannot ride the Kinetic speed curve to 8),
/// while a rake through three sections puts 6 into a ship the slug could only
/// put 4 into. Half is a round number, not a measured one - the first knob to
/// turn once the two are flown side by side.
const PIERCE_PDC_BULLET_DAMAGE: f32 = KINETIC_PDC_BULLET_DAMAGE * 0.5;

/// Side of the shared PDC turret's mount box - and the scale its art is
/// assembled at, which is the point of having one number: the collider, the
/// sockets and the gun agree, instead of a unit-cube turret balanced on a small
/// box. Turret art is drawn against the unit cube, so the scale IS the size.
///
/// Half a section: a weapon mount SITS ON a hull face and leaves room to aim at
/// the rest of it, where a unit-cube turret replaces the face outright.
const PDC_TURRET_SIZE: f32 = 0.5;

/// How far the shared PDC's base socket sits from the mount's own centre.
///
/// A mount bolts down by its base plate, so this is the ONLY offset a host
/// needs to know to put a socket where the gun will actually stand. Ship
/// builders read it to place the socket they offer a turret (see
/// `ships::shared::link_points`).
pub(crate) const PDC_MOUNT_OFFSET: f32 = PDC_TURRET_SIZE * 0.5;

/// How far a shipped turret's pitch hinge may DEPRESS below level (10 deg).
/// Every shipped mount sits ON a hull - the cargoa's nose cheeks most tightly -
/// so a deeper floor only swings the barrel back across its own ship.
const TURRET_DEPRESSION_LIMIT: f32 = std::f32::consts::PI / 18.0;

/// The size the turret art was drawn at: one whole section cube. A tree
/// assembled at this scale needs no art transform at all.
const UNIT_TURRET_SCALE: f32 = 1.0;

/// Build the shipped turret's kinematic joint tree: the exact chain the flat
/// config used to author 1:1. base(fixed, on the mount face) -> yaw(Y, meshed)
/// -> pitch(X, meshed, -10..90 deg) -> barrel(fixed, meshed) -> muzzle(fixed,
/// fire point). `fire_rate` is per-muzzle now; every other numeric value is
/// preserved from the old fields.
///
/// `mount` is the section's own half-height, and it is a PARAMETER because the
/// base offset is where the turret STANDS: hardcoded at the unit cube's -0.5, a
/// turret on a shorter mount planted its base below its own bottom face and
/// sank into whatever it was bolted to.
///
/// `scale` resizes the WHOLE assembly. It multiplies every joint offset AND
/// rides on every joint's render-mesh transform, because those are two halves
/// of one answer: scaling the meshes alone leaves the parts spaced for the size
/// they used to be, and scaling the offsets alone leaves full-size art in a
/// smaller arrangement. It reaches the base plate too - that plate is a default
/// primitive a full unit across (see `insert_turret_joint_render`), so a turret
/// mounted on anything but a unit cube wore a hull-sized dinner plate.
///
/// Every shipped caller is the shared PDC, which passes its own half-size and
/// its own size: the mount, the sockets and the gun agree by construction.
pub(crate) fn turret_joint_tree(
    yaw_mesh: &AssetRef<WorldAsset>,
    pitch_mesh: &AssetRef<WorldAsset>,
    barrel_mesh: &AssetRef<WorldAsset>,
    fire_rate: f32,
    mount: f32,
    scale: f32,
) -> TurretJoint {
    // Authored at unit size and multiplied through, so the numbers below stay
    // the ones the art was drawn against and there is one place to read them.
    let at = |offset: Vec3| offset * scale;
    let art = (scale != UNIT_TURRET_SCALE).then(|| RenderMeshTransform {
        scale: Vec3::splat(scale),
        ..default()
    });

    TurretJoint {
        offset: Vec3::new(0.0, -mount, 0.0),
        axis: None,
        speed: std::f32::consts::PI,
        min: None,
        max: None,
        render_mesh: None,
        // The base plate is a default primitive, not authored art, so the
        // transform is what sizes it.
        render_mesh_transform: art,
        muzzle: None,
        children: vec![TurretJoint {
            offset: at(Vec3::new(0.0, 0.1, 0.0)),
            axis: Some(Vec3::Y),
            speed: std::f32::consts::PI, // 180 degrees per second
            min: None,
            max: None,
            render_mesh: Some(yaw_mesh.clone()),
            render_mesh_transform: art,
            muzzle: None,
            children: vec![TurretJoint {
                offset: at(Vec3::new(0.0, 0.332706, 0.303954)),
                axis: Some(Vec3::X),
                speed: std::f32::consts::PI, // 180 degrees per second
                // Depression floor: every shipped turret is HULL-MOUNTED, so a
                // deep depression just aims the barrel into its own ship (the
                // cargoa's nose cheeks are the tightest case). 10 degrees is
                // enough to reach a target slightly below the mount without
                // the muzzle sweeping back across the bodywork. Elevation
                // stays at 90: straight up is the point-defense arc.
                min: Some(-TURRET_DEPRESSION_LIMIT),
                max: Some(std::f32::consts::FRAC_PI_2),
                render_mesh: Some(pitch_mesh.clone()),
                render_mesh_transform: art,
                muzzle: None,
                children: vec![TurretJoint {
                    offset: at(Vec3::new(0.0, 0.128437, -0.110729)),
                    axis: None,
                    speed: std::f32::consts::PI,
                    min: None,
                    max: None,
                    render_mesh: Some(barrel_mesh.clone()),
                    render_mesh_transform: art,
                    muzzle: None,
                    children: vec![TurretJoint {
                        offset: at(Vec3::new(0.0, 0.0, -1.2)),
                        axis: None,
                        speed: std::f32::consts::PI,
                        min: None,
                        max: None,
                        render_mesh: None,
                        render_mesh_transform: None,
                        muzzle: Some(MuzzleConfig {
                            fire_rate,
                            muzzle_effect: None,
                        }),
                        children: vec![],
                    }],
                }],
            }],
        }],
    }
}

use crate::base_content::assets::BaseContentAssets;

/// The unit cube's six sockets minus the one facing `open`.
///
/// A hole in the art is not a mating surface. The face a bay fires through
/// carries no structure to bolt to, and leaving it socketed let a builder
/// plate over the muzzle - the section mated, the salvo then launched inside
/// its own ship. Dropping the socket is what makes that placement impossible
/// rather than merely unwise.
fn unit_cube_link_points_without(open: Vec3) -> Vec<LinkPoint> {
    unit_cube_link_points()
        .into_iter()
        .filter(|point| point.normal.dot(open) < 0.5)
        .collect()
}

/// One compact PDC prototype, parameterised on the ROUND it loads.
///
/// The two shipped PDCs share the mount, the joint tree, the fire rate and the
/// magazine, and differ in the only two things a round's identity needs: its
/// type and its per-hit damage. Sharing one builder is what keeps the
/// side-by-side comparison honest - a mount or cadence retune cannot drift one
/// copy against the other, so what the player feels is the punch-versus-rake
/// difference and nothing else.
fn pdc_turret_prototype(
    meshes: &BaseContentAssets,
    id: &str,
    name: &str,
    description: &str,
    bullet_kind: DamageType,
    bullet_damage: f32,
) -> SectionConfig {
    SectionConfig {
        base: BaseSectionConfig {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            health: TURRET_BASE_HEALTH,
            impact_sound: Some(meshes.section_impact_sound.clone()),
            destroy_sound: Some(meshes.section_destroy_sound.clone()),
            // A turret is all function - the barrel has to point and the mount
            // has to turn - so it fails by sparking and never by losing a
            // piece of itself.
            damage_effects: DamageEffects(vec![DamageEffect::Cracks, DamageEffect::Sparks]),
            // The mount the shipped craft carry: a small box that sits ON a
            // hull face instead of standing in for one, which is what lets ONE
            // turret serve every craft. The ten per-craft copies that used to
            // sit beside it were the same gun on the same joint tree, and are
            // gone.
            collider: Some(SectionCollider::Cuboid {
                size: Vec3::splat(PDC_TURRET_SIZE),
            }),
            // ONE socket, on the base plate. A turret is bolted down, not
            // stacked: the other five faces of the mount box are gun, and a
            // full `box_link_points` set offered them all as mating surfaces -
            // so the editor would stand a second turret on the first one's
            // barrel, or bolt a hull slab across its traverse. The base is
            // where `turret_joint_tree` plants the assembly (`-mount` on Y), so
            // it is the one face that is structure.
            link_points: vec![LinkPoint {
                id: "base".to_string(),
                position: Vec3::NEG_Y * (PDC_TURRET_SIZE * 0.5),
                normal: Vec3::NEG_Y,
            }],
            hide_in_editor: false,
        },
        kind: SectionKind::Turret(TurretSectionConfig {
            root: turret_joint_tree(
                &meshes.turret_yaw,
                &meshes.turret_pitch,
                &meshes.turret_barrel,
                100.0,
                // The turret stands on THIS mount's face, not on a unit cube's,
                // and is assembled at THIS mount's size.
                PDC_TURRET_SIZE * 0.5,
                PDC_TURRET_SIZE,
            ),
            // Also the closing speed both curves read 1.0 at
            // (REFERENCE_CLOSING_SPEED), so a station-keeping duel with either
            // PDC lands exactly the authored per-hit below. Muzzle speed is
            // therefore NOT a range knob - moving it rebalances every weapon's
            // damage. Reach is bought with lifetime alone.
            muzzle_speed: 100.0,
            // 100 u/s x 2.0 s = 200 u (2.0 km) of reach, the top of the
            // intended 1-2 km PDC band. Lifetime is the ONLY reach knob a
            // turret has, and it is read back by the AI fire gate and by the
            // balance audit's threat envelope: see AI_FIRE_RANGE_FACTOR
            // (nova_ship/src/input/ai/guns.rs) for the constants that move
            // with it.
            projectile_lifetime: 2.0,
            bullet_damage,
            bullet_kind,
            projectile_render_mesh: None,
            fire_sound: Some(meshes.turret_fire_sound.clone()),
            dry_fire_sound: Some(meshes.turret_dry_fire_sound.clone()),
            ammo_capacity: Some(500),
            reload: Some(SectionReloadConfig {
                delay: 3.0,
                amount: 200,
            }),
        }),
    }
}

/// The section catalog, built against `meshes` for its render-mesh refs. The
/// single source of truth for the built-in sections; both the production
/// registry and the RON generator go through here.
pub fn standard_section_prototypes(meshes: &BaseContentAssets) -> Vec<SectionConfig> {
    let sections = vec![
        SectionConfig {
            base: BaseSectionConfig {
                id: REINFORCED_HULL_SECTION_ID.to_string(),
                // Material and nothing else, so the damage reads in the
                // surface: it cracks, and its cladding leaves plate by plate.
                damage_effects: DamageEffects(vec![DamageEffect::Cracks]),
                name: "Reinforced Hull Section".to_string(),
                description: "A reinforced hull section for spaceships.".to_string(),
                health: 200.0,
                impact_sound: Some(meshes.section_impact_sound.clone()),
                destroy_sound: Some(meshes.section_destroy_sound.clone()),
                collider: None,
                link_points: unit_cube_link_points(),
                hide_in_editor: false,
            },
            kind: SectionKind::Hull(HullSectionConfig {
                render_mesh: Some(meshes.hull.clone()),
                render_mesh_transform: None,
            }),
        },
        SectionConfig {
            base: BaseSectionConfig {
                id: BASIC_THRUSTER_SECTION_ID.to_string(),
                // A drive is machinery and a bell: it sparks and its
                // plume guts, and it never loses a piece of itself.
                damage_effects: DamageEffects(vec![
                    DamageEffect::Cracks,
                    DamageEffect::Sparks,
                    DamageEffect::Plume,
                ]),
                name: "Basic Thruster Section".to_string(),
                description: "A basic thruster section for spaceships.".to_string(),
                // Exposed propulsion: fragile, takes more damage per hit than
                // an armored mount.
                health: THRUSTER_BASE_HEALTH,
                impact_sound: Some(meshes.section_impact_sound.clone()),
                destroy_sound: Some(meshes.section_destroy_sound.clone()),
                collider: None,
                // ONE socket, on the mounting face. The default thruster has no
                // authored mesh: the runtime builds a barrel and a nozzle bell
                // on the Z axis (`insert_thruster_section_render`), the bell
                // opens toward +Z and the plume fires out of it. So the flat
                // forward end is the only structure on the part - the other
                // five faces are drive and exhaust. Six sockets offered them
                // all as mating surfaces, and a builder would bolt a hull slab
                // onto the barrel or plate one across the nozzle.
                link_points: vec![LinkPoint {
                    id: "base".to_string(),
                    position: Vec3::NEG_Z * 0.5,
                    normal: Vec3::NEG_Z,
                }],
                hide_in_editor: false,
            },
            kind: SectionKind::Thruster(ThrusterSectionConfig {
                magnitude: 1.0,
                render_mesh: None,
                render_mesh_transform: None,
                loop_sound: Some(meshes.thruster_loop_sound.clone()),
                exhaust: None,
            }),
        },
        SectionConfig {
            base: BaseSectionConfig {
                id: BASIC_CONTROLLER_SECTION_ID.to_string(),
                damage_effects: DamageEffects(vec![DamageEffect::Cracks, DamageEffect::Sparks]),
                name: "Basic Controller Section".to_string(),
                description: "A basic controller section for spaceships.".to_string(),
                // Command core: mid durability baseline.
                health: CONTROLLER_BASE_HEALTH,
                impact_sound: Some(meshes.section_impact_sound.clone()),
                destroy_sound: Some(meshes.section_destroy_sound.clone()),
                collider: None,
                link_points: unit_cube_link_points(),
                hide_in_editor: false,
            },
            kind: SectionKind::Controller(ControllerSectionConfig {
                steering_lag: 0.5,
                // What a hull DOES with this is `min(torque / inertia, load
                // limit / arm)`, so every hull that ships today is structure-
                // bound and this number is invisible on all of them - it has 12x
                // to 115x of headroom over what their metal takes. It is the
                // capital's knob, and it is provisional: the crossover it was
                // pinned against sits three times further out than the largest
                // hull in the game.
                max_torque: 1501.0,
                // Full flight-verb loadout by default (no WithheldVerbs on the
                // built controller). Scenarios withhold a verb via a
                // `DisableVerb` section modification or the `SetControllerVerb`
                // action (the shakedown's GOTO-off intro) rather than baking it
                // into this shared catalog entry, which the pirate reuses too.
                render_mesh: None,
                render_mesh_transform: None,
                lock_on_sound: Some(meshes.controller_lock_on_sound.clone()),
                lock_off_sound: Some(meshes.controller_lock_off_sound.clone()),
                radar_deny_sound: Some(meshes.controller_radar_deny_sound.clone()),
                radar_retarget_sound: Some(meshes.controller_radar_retarget_sound.clone()),
                safety_on_sound: Some(meshes.controller_safety_on_sound.clone()),
                rcs_loop_sound: Some(meshes.controller_rcs_loop_sound.clone()),
            }),
        },
        SectionConfig {
            base: BaseSectionConfig {
                id: LIGHT_HULL_SECTION_ID.to_string(),
                damage_effects: DamageEffects::default(),
                name: "Light Hull Section".to_string(),
                description: "A thin-walled hull section; scavenger grade.".to_string(),
                // A third of reinforced: the shakedown pirate should die in a
                // short burst, not a slugging match ("gentle" is data).
                health: 60.0,
                impact_sound: Some(meshes.section_impact_sound.clone()),
                destroy_sound: Some(meshes.section_destroy_sound.clone()),
                collider: None,
                link_points: unit_cube_link_points(),
                hide_in_editor: false,
            },
            kind: SectionKind::Hull(HullSectionConfig {
                render_mesh: Some(meshes.hull.clone()),
                render_mesh_transform: None,
            }),
        },
        pdc_turret_prototype(
            meshes,
            PDC_KINETIC_TURRET_SECTION_ID,
            "PDC Turret (Kinetic)",
            "A compact point-defense mount that fits any hull face. Slugs: the \
             hardest single hit, harder still on a charge, and they stop at \
             anything they cannot destroy.",
            DamageType::Kinetic,
            KINETIC_PDC_BULLET_DAMAGE,
        ),
        pdc_turret_prototype(
            meshes,
            "pdc_pierce_turret_section",
            "PDC Turret (Pierce)",
            "The same mount firing penetrators. Half the damage per hit, dealt \
             to EVERY section the round rakes through - closing fast buys depth, \
             not damage. Worse against one thin target, better against a deep \
             one.",
            DamageType::Pierce,
            PIERCE_PDC_BULLET_DAMAGE,
        ),
        torpedo_bay_prototype(
            meshes,
            "torpedo_section",
            "Torpedo Bay (Serpent)",
            "The standard bay, loaded with Serpent assault torpedoes. They run \
             in on a terminal weave, so point defense spends roughly three \
             times the rounds to stop one and only kills it on the doorstep. \
             The corkscrew is a longer path: a Serpent arrives later than a \
             Lance and gains less on a target that is running.",
            ordnance::serpent(),
        ),
        torpedo_bay_prototype(
            meshes,
            "lance_torpedo_section",
            "Torpedo Bay (Lance)",
            "The same bay and the same warhead, loaded with Lance bombardment \
             torpedoes: no weave, the bare intercept. The shortest path there \
             is - it arrives soonest and keeps closing on a runner - which is \
             also the path point defense is built to solve. Ordnance for a \
             target that will not shoot back.",
            ordnance::lance(),
        ),
        SectionConfig {
            base: BaseSectionConfig {
                id: "heavy_torpedo_section".to_string(),
                damage_effects: DamageEffects(vec![DamageEffect::Cracks, DamageEffect::Sparks]),
                name: "Siege Torpedo Bay Section".to_string(),
                description: "A capital-grade siege torpedo battery: slow salvo, \
                              armored ordnance, ship-killing blast."
                    .to_string(),
                health: TORPEDO_BASE_HEALTH,
                impact_sound: Some(meshes.section_impact_sound.clone()),
                destroy_sound: Some(meshes.section_destroy_sound.clone()),
                collider: None,
                // The same bay art at siege grade, so the same open muzzle.
                link_points: unit_cube_link_points_without(Vec3::NEG_Z),
                // Scene dressing, not player kit: this armoured finisher cuts
                // much deeper than standard ordnance, so the editor gallery
                // does not offer it.
                hide_in_editor: true,
            },
            kind: SectionKind::Torpedo(TorpedoSectionConfig {
                render_mesh: Some(meshes.torpedo_bay.clone()),
                render_mesh_transform: None,
                projectile_render_mesh: None,
                spawn_offset: Vec3::NEG_Z * 2.0,
                spawn_rotation: Quat::IDENTITY,
                fire_rate: 1.0,
                spawner_speed: 2.0,
                // Long enough to cross a backdrop arena, short enough that a
                // torpedo whose target died mid-flight cleans itself up.
                projectile_lifetime: 60.0,
                arm_time: 0.5,
                arm_distance: 5.0,
                nav_constant: 4.0,
                linear_damping: 0.4,
                // Siege pressure: the same 65% transmission rule as every
                // Explosive blast, but 2000 at the centre stays lethal through
                // more structural layers than the standard 750-point warhead.
                blast_radius: 45.0,
                blast_damage: 2000.0,
                blast_effect: None,
                launch_effect: None,
                launch_sound: Some(meshes.torpedo_launch_sound.clone()),
                detonation_sound: Some(meshes.section_destroy_sound.clone()),
                // Armored ordnance: a PDC burst (~800 DPS) cannot chew
                // through this inside the ~6 s closing window, so point
                // defense visibly hammers it and still loses.
                projectile_health: 5000.0,
                torpedo_type: ordnance::breaker(),
                ammo_capacity: None,
                reload: None,
            }),
        },
    ];
    sections
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
            impact_sound: Some(meshes.section_impact_sound.clone()),
            destroy_sound: Some(meshes.section_destroy_sound.clone()),
            collider: None,
            // Sides and backblast only: the tube fires out of -Z
            // (`spawn_offset`), so the bow face is the open muzzle.
            link_points: unit_cube_link_points_without(Vec3::NEG_Z),
            hide_in_editor: false,
            // A launcher is loading machinery: it arcs and sparks as it fails,
            // and the tube it fires down stays a tube.
            damage_effects: DamageEffects(vec![DamageEffect::Cracks, DamageEffect::Sparks]),
        },
        kind: SectionKind::Torpedo(TorpedoSectionConfig {
            render_mesh: Some(meshes.torpedo_bay.clone()),
            render_mesh_transform: None,
            projectile_render_mesh: None,
            spawn_offset: Vec3::NEG_Z * 2.0,
            spawn_rotation: Quat::IDENTITY,
            fire_rate: 1.0,
            spawner_speed: 1.0,
            projectile_lifetime: 100.0,
            arm_time: 0.5,
            arm_distance: 5.0,
            nav_constant: 3.0,
            linear_damping: 0.8,
            blast_radius: 30.0,
            // Expanse-style nuclear pressure: a direct hit can decide a
            // small-craft fight, while structural depth stops it from deleting
            // a capital. Each destroyed section transmits 65% of the remaining
            // pressure; a surviving section shields everything behind it.
            //
            // EQUAL ACROSS TYPES by owner direction: a torpedo type decides
            // how the ordnance flies, never how hard it lands.
            blast_damage: 750.0,
            blast_effect: None,
            launch_effect: None,
            launch_sound: Some(meshes.torpedo_launch_sound.clone()),
            // The blast IS the destruction voice: same wav as section
            // destruction (per-target authoring; playtest can diverge it).
            detonation_sound: Some(meshes.section_destroy_sound.clone()),
            // Above the hardest single PDC round (4.0 authored x the 2.0
            // Kinetic speed ceiling), so an intercept costs two or three
            // rounds instead of one lucky tap.
            projectile_health: 10.0,
            // The whole difference between the two shipped bays: see
            // `sections::ordnance` for what each type costs and buys.
            torpedo_type,
            // The rack, and the alpha strike it buys: six away in six
            // seconds at the fire rate above. Saturation is what beats
            // point defense - attrition never does - so the burst is the
            // attacker's weapon and the reload below is only its floor.
            ammo_capacity: Some(6),
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
            reload: Some(SectionReloadConfig {
                delay: 10.0,
                amount: 1,
            }),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    /// The per-craft turret modules used to be a KNOWN deviation excluded from
    /// this rule; they are gone, so every turret in the catalog is held to it.
    ///
    /// No bay offers a mating surface across the face it fires through.
    ///
    /// A torpedo section used to carry the plain hull block's full six-socket
    /// cube, so the editor would bolt a section over the muzzle: the placement
    /// mated, and the salvo then launched inside its own ship. The rule is the
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

    /// The placeable mounts bolt down by their base plate and by nothing else.
    ///
    /// Six sockets made every face of the mount BOX a mating surface, but five
    /// of them are gun: the editor would stand a second turret on the first
    /// one's barrel, or plate a hull slab across its traverse. One socket, on
    /// the face `turret_joint_tree` plants the assembly against, is what makes
    /// those placements impossible instead of merely unwise. Both shipped PDCs
    /// share the builder, so both are held to it.
    #[test]
    fn the_shared_mount_sockets_only_its_base_plate() {
        for id in ["pdc_kinetic_turret_section", "pdc_pierce_turret_section"] {
            let mount = crate::generation::build_section_catalog()
                .into_iter()
                .find(|section| section.base.id == id)
                .expect("the shared PDC mounts are in the catalog");
            let SectionKind::Turret(turret) = &mount.kind else {
                panic!("`{id}` is a turret");
            };
            let [base] = mount.base.link_points.as_slice() else {
                panic!(
                    "`{id}` carries {} sockets, not one",
                    mount.base.link_points.len()
                );
            };
            assert_eq!(base.normal, Vec3::NEG_Y, "`{id}`'s base plate faces down");
            assert_eq!(
                base.position.y, turret.root.offset.y,
                "`{id}`'s socket sits where the assembly is planted"
            );
        }
    }

    /// The drive bolts on by its forward end and by nothing else.
    ///
    /// The default thruster is built from primitives, not from authored art:
    /// the barrel and the nozzle bell lie on the Z axis and the bell opens
    /// toward +Z, so the mounting end is -Z - NOT the -Y a turret stands on.
    /// Six sockets made the barrel and the open nozzle mating surfaces too, and
    /// a generator would plate a hull slab across the exhaust.
    #[test]
    fn the_thruster_sockets_only_the_face_it_bolts_on_by() {
        let drive = crate::generation::build_section_catalog()
            .into_iter()
            .find(|section| section.base.id == "basic_thruster_section")
            .expect("the basic thruster is in the catalog");
        let [base] = drive.base.link_points.as_slice() else {
            panic!(
                "the thruster carries {} sockets, not one",
                drive.base.link_points.len()
            );
        };
        assert_eq!(base.normal, Vec3::NEG_Z, "the mounting face points forward");
        assert_eq!(
            base.position,
            Vec3::NEG_Z * 0.5,
            "the socket sits on that face, not inside the part"
        );
    }

    #[test]
    fn every_placeable_turret_stands_on_its_own_mount_face() {
        for section in crate::generation::build_section_catalog() {
            let SectionKind::Turret(turret) = &section.kind else {
                continue;
            };
            if section.base.hide_in_editor {
                continue;
            }
            let half_height = section
                .base
                .collider
                .unwrap_or_default()
                .aabb_half_extents()
                .y;
            assert!(
                (turret.root.offset.y + half_height).abs() < 1e-5,
                "`{}` plants its base at {} on a mount half {half_height} deep",
                section.base.id,
                turret.root.offset.y,
            );
        }
    }

    /// Scaling an assembly is TWO things at once: every joint's art and every
    /// joint's offset. Scaling the meshes alone leaves the parts spaced for the
    /// size they used to be, which reads as a turret coming apart; scaling the
    /// offsets alone leaves full-size art in a smaller arrangement.
    #[test]
    fn a_scaled_turret_tree_scales_its_offsets_and_its_art_together() {
        let mesh = |name: &str| AssetRef::<WorldAsset>::from(name.to_string());
        let (yaw, pitch, barrel) = (mesh("yaw"), mesh("pitch"), mesh("barrel"));
        let tree = |scale: f32| turret_joint_tree(&yaw, &pitch, &barrel, 100.0, 0.5, scale);

        let unit = tree(UNIT_TURRET_SCALE);
        let half = tree(0.5);

        // Walk both trees in lockstep down the shipped chain.
        let (mut a, mut b) = (&unit, &half);
        let mut joints = 0;
        loop {
            assert!(
                b.offset.abs_diff_eq(a.offset * 0.5, 1e-6) || joints == 0,
                "joint {joints}: offset {:?} is not half of {:?}",
                b.offset,
                a.offset,
            );
            match (&b.render_mesh, b.render_mesh_transform) {
                (Some(_), Some(art)) => assert_eq!(art.scale, Vec3::splat(0.5)),
                (Some(_), None) => panic!("joint {joints}: meshed but unscaled art"),
                _ => {}
            }
            assert!(
                a.render_mesh_transform.is_none(),
                "joint {joints}: an unscaled tree authors no art transform, so the \
                 shipped RON is unchanged"
            );
            joints += 1;
            match (a.children.first(), b.children.first()) {
                (Some(next_a), Some(next_b)) => {
                    a = next_a;
                    b = next_b;
                }
                (None, None) => break,
                _ => panic!("the two trees have different shapes"),
            }
        }
        assert_eq!(joints, 5, "base, yaw, pitch, barrel, muzzle");

        // The base plate is unmeshed, and it is exactly the part that used to
        // stay unit-sized: it carries the scale on its own transform.
        assert_eq!(
            half.render_mesh_transform.map(|art| art.scale),
            Some(Vec3::splat(0.5))
        );
    }

    /// "Variable damage by section type" as a checked invariant: section TYPE
    /// must drive durability, not sit at a uniform value. If someone flattens
    /// the baselines back to one number this fails, catching a silent
    /// regression of the feature.
    #[test]
    fn section_type_durability_ordering_holds() {
        // Thrusters take MORE damage than the baseline (fragile); turrets take
        // LESS (armored). The strict inequalities are the feature. Const
        // blocks so a flattening regression fails at COMPILE time; a const
        // panic cannot format values, so the messages name the constants.
        const {
            assert!(
                THRUSTER_BASE_HEALTH < CONTROLLER_BASE_HEALTH,
                "a thruster must be more fragile than the mid baseline \
                 (THRUSTER_BASE_HEALTH vs CONTROLLER_BASE_HEALTH)"
            );
        }
        const {
            assert!(
                CONTROLLER_BASE_HEALTH < TURRET_BASE_HEALTH,
                "a turret must be tougher than the mid baseline \
                 (CONTROLLER_BASE_HEALTH vs TURRET_BASE_HEALTH)"
            );
        }
        // The controller core and the torpedo bay share the mid baseline.
        assert_eq!(CONTROLLER_BASE_HEALTH, TORPEDO_BASE_HEALTH);
    }

    /// The two shipped PDCs exist to be COMPARED: mount one of each and the only
    /// difference the player can feel is the ROUND - its type and its per-hit
    /// damage. Mount, joint tree, fire rate and magazine must be identical, or
    /// the comparison measures something else. Debug strings stand in for
    /// structural equality (`TurretSectionConfig` has no `PartialEq`), which is
    /// enough to catch any other field drifting between them.
    #[test]
    fn the_two_pdcs_differ_only_in_the_round_they_load() {
        let turret = |id: &str| {
            crate::generation::build_section_catalog()
                .into_iter()
                .find(|section| section.base.id == id)
                .map(|section| match section.kind {
                    SectionKind::Turret(turret) => turret,
                    other => panic!("`{id}` is not a turret: {other:?}"),
                })
                .unwrap_or_else(|| panic!("the catalog ships `{id}`"))
        };
        let kinetic = turret("pdc_kinetic_turret_section");
        let mut pierce = turret("pdc_pierce_turret_section");

        assert_eq!(kinetic.bullet_kind, DamageType::Kinetic);
        assert_eq!(pierce.bullet_kind, DamageType::Pierce);
        // The trade: a rake gives up per-hit damage for depth, so the slug must
        // stay the harder single hit. Without this the two guns would be a
        // strict upgrade rather than a choice.
        assert!(
            pierce.bullet_damage < kinetic.bullet_damage,
            "the pierce PDC must hit softer per contact ({} vs {})",
            pierce.bullet_damage,
            kinetic.bullet_damage
        );

        pierce.bullet_kind = kinetic.bullet_kind;
        pierce.bullet_damage = kinetic.bullet_damage;
        assert_eq!(
            format!("{kinetic:?}"),
            format!("{pierce:?}"),
            "the two PDCs must be the same gun apart from the round they load"
        );
    }

    /// Anti-regression guard for the PDC one-shot fix: the player PDC's per-hit
    /// must stay low enough that the softest thing it shoots at takes a burst
    /// and not a blink. At the old ~20.25 a light hull section died in three
    /// rounds - 30 ms of trigger - which is the "PDC destroys objects with one
    /// bullet" playtest report.
    ///
    /// Anchored on the softest SHIPPED section, because the 100-HP asteroid the
    /// guard used to cite has not existed since rocks stopped carrying a health
    /// pool: a rock's durability is the material left in it, priced at
    /// `DAMAGE_PER_UNIT_VOLUME`, and the smallest one a scenario scatters is
    /// about 140 cubic units - some 280 rounds. The section is the tighter
    /// bound, so it is the one worth guarding.
    ///
    /// A loose guard and not a balance number: raise it consciously if playtest
    /// wants a punchier PDC.
    #[test]
    fn pdc_per_hit_stays_below_the_one_shot_ceiling() {
        /// `light_hull_section`: scavenger grade, and the least health any
        /// shipped section carries.
        const SOFTEST_SECTION_HEALTH: f32 = 60.0;
        const MIN_ROUNDS_TO_KILL: f32 = 12.0;
        const {
            assert!(
                KINETIC_PDC_BULLET_DAMAGE <= SOFTEST_SECTION_HEALTH / MIN_ROUNDS_TO_KILL,
                "PDC per-hit KINETIC_PDC_BULLET_DAMAGE would kill the softest \
                 shipped section in under MIN_ROUNDS_TO_KILL rounds - too close \
                 to a one-shot pop"
            );
        }
    }
}
