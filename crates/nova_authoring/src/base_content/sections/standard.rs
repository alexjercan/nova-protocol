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
/// Three cells of ship behind ONE health pool, which is the lance's whole
/// structural cost: a builder who mounts one is trading a spine's worth of
/// separately-killable sections for a single target. Above the turret baseline
/// because it is armoured capacitor casing, well below three reinforced hull
/// blocks (600) because it must stay a weak spot worth shooting at.
const RAILGUN_BASE_HEALTH: f32 = 180.0;

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

/// The gatling PDC's cadence, rounds per second out of its one muzzle. The
/// whole gunnery curve prices itself against this number (see
/// `KINETIC_PDC_BULLET_DAMAGE`).
const GATLING_FIRE_RATE: f32 = 100.0;

/// The twin PDC's cadence PER MUZZLE: half the gatling's, so its two streams
/// spend the shared magazine at the same total rate and the two mounts stay
/// the same gun in DPS terms. What the twin buys is coverage - two offset
/// streams walking onto a target - not more damage. A playtest knob;
/// `the_twin_mount_splits_the_same_total_rate_across_two_muzzles` pins the
/// relation so a retune of one mount cannot silently outgun the other.
const TWIN_FIRE_RATE: f32 = GATLING_FIRE_RATE * 0.5;

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
/// assembled at this scale needs no art transform on its unit-drawn parts
/// (the stow elevator's platform placement is authored at every scale, and
/// the housing is final-size art - see `TurretArt`).
const UNIT_TURRET_SCALE: f32 = 1.0;

/// One mount's authored geometry: the meshed parts and where each joint
/// stands, in unit-turret space. Turret parts are generated around their own
/// joint origins (`scripts/gen-section-parts.py`), so these offsets ARE the
/// assembly - the section gallery example poses candidates with the same
/// numbers, which is how they were read off in the first place.
struct TurretArt<'a> {
    /// The stow housing the assembly sinks into, worn by the fixed base
    /// joint. Sized to the host hull CELL it bolts onto (a 1x1 footprint,
    /// 0.5 tall, base on the joint origin), not to the mount, so unlike the
    /// unit-drawn parts it never takes the tree's scale transform; a
    /// differently sized mount authors its own housing.
    housing_mesh: &'a AssetRef<WorldAsset>,
    yaw_mesh: &'a AssetRef<WorldAsset>,
    pitch_mesh: &'a AssetRef<WorldAsset>,
    barrel_mesh: &'a AssetRef<WorldAsset>,
    /// The yaw turntable, raised so the pitch hinge clears the housing deck
    /// and the barrel sweeps above the lids (deployed pose).
    yaw_at: Vec3,
    /// The pitch hinge, above the turntable.
    pitch_at: Vec3,
    /// The barrel root, off the hinge.
    barrel_at: Vec3,
    /// One fire point per barrel, each just past its own tip. Every muzzle
    /// fires: the engine keeps a cadence timer and a bearing gate per muzzle
    /// over the section's one magazine, so a second entry here IS the
    /// double stream.
    muzzles_at: &'a [Vec3],
}

/// The gatling's one fire point, just past its barrel tip (the barrel part
/// reaches z -0.9 from its root).
const GATLING_MUZZLES: [Vec3; 1] = [Vec3::new(0.0, 0.0, -0.95)];

/// The twin's two fire points, one past each tube of the barrel block.
const TWIN_MUZZLES: [Vec3; 2] = [Vec3::new(0.12, 0.0, -0.95), Vec3::new(-0.12, 0.0, -0.95)];

/// The gatling mount, the default PDC: one rotary barrel cluster, one muzzle.
fn gatling_art(meshes: &BaseContentAssets) -> TurretArt<'_> {
    TurretArt {
        housing_mesh: &meshes.turret_housing,
        yaw_mesh: &meshes.turret_yaw,
        pitch_mesh: &meshes.turret_pitch,
        barrel_mesh: &meshes.turret_barrel,
        // Raised for the stow housing: the turntable head sits proud of the
        // deck (hole half-width 0.24 clears the 0.21 assembly radius) and
        // the pitch hinge lands 0.10 above the deck, so the level barrel
        // sweeps over the shut-lid plane and the -10 deg depression clears
        // the housing rim.
        yaw_at: Vec3::new(0.0, 0.8, 0.0),
        pitch_at: Vec3::new(0.0, 0.4, 0.0),
        barrel_at: Vec3::new(0.0, 0.02, -0.1),
        muzzles_at: &GATLING_MUZZLES,
    }
}

/// The twin mount: one barrel block carrying two tubes at x +-0.12, so two
/// muzzles - and two independent fire streams over the shared magazine.
fn twin_art(meshes: &BaseContentAssets) -> TurretArt<'_> {
    TurretArt {
        housing_mesh: &meshes.turret_housing,
        yaw_mesh: &meshes.turret_twin_yaw,
        pitch_mesh: &meshes.turret_twin_pitch,
        barrel_mesh: &meshes.turret_twin_barrel,
        // 0.05 unit lower than the gatling's raise: the twin's pitch sits
        // 0.45 up its pedestal, so both mounts land their pitch hinge at the
        // same 0.35 above the section origin - one housing, one deck
        // clearance, one stow sink for the pair.
        yaw_at: Vec3::new(0.0, 0.75, 0.0),
        pitch_at: Vec3::new(0.0, 0.45, 0.0),
        barrel_at: Vec3::new(0.0, 0.0, -0.2),
        muzzles_at: &TWIN_MUZZLES,
    }
}

/// Build a turret's kinematic joint tree: base(fixed, on the mount face) ->
/// yaw(Y, meshed) -> pitch(X, meshed, -10..90 deg) -> barrel(fixed, meshed) ->
/// one muzzle leaf (fixed, fire point) per entry in `art.muzzles_at`.
/// `fire_rate` is per-muzzle.
///
/// `mount` is the section's own half-height, and it is a PARAMETER because the
/// base offset is where the turret STANDS: hardcoded at the unit cube's -0.5, a
/// turret on a shorter mount planted its base below its own bottom face and
/// sank into whatever it was bolted to.
///
/// `scale` resizes the WHOLE assembly. It multiplies every joint offset AND
/// rides on every joint's render-mesh transform, because those are two halves
/// of one answer: scaling the meshes alone leaves the parts spaced for the
/// unscaled size, and scaling the offsets alone leaves full-size art in a
/// smaller arrangement. It reaches the base plate too - that plate is a default
/// primitive a full unit across (see `insert_turret_joint_render`), so a turret
/// mounted on anything but a unit cube wore a hull-sized dinner plate.
///
/// Every shipped caller is a PDC, which passes its own half-size and its own
/// size: the mount, the sockets and the gun agree by construction.
fn turret_joint_tree(
    art_spec: &TurretArt<'_>,
    fire_rate: f32,
    mount: f32,
    scale: f32,
) -> TurretJoint {
    // Authored at unit size and multiplied through, so the numbers stay the
    // ones the art was drawn against and `TurretArt` is the one place to read
    // them.
    let at = |offset: Vec3| offset * scale;
    let art = (scale != UNIT_TURRET_SCALE).then(|| RenderMeshTransform {
        scale: Vec3::splat(scale),
        ..default()
    });
    let muzzles = art_spec
        .muzzles_at
        .iter()
        .map(|&muzzle_at| TurretJoint {
            name: None,
            offset: at(muzzle_at),
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
        })
        .collect();

    // The elevator platform is the default joint primitive (a wide flat
    // disc) pressed into service: sized to ride inside the housing's 0.48
    // shaft mouth and lifted to sit just under the turntable, it reads as
    // the floor the assembly stands on and sinks with it.
    let platform = Some(RenderMeshTransform {
        position: Vec3::new(0.0, at(art_spec.yaw_at).y - 0.14 * scale, 0.0),
        scale: Vec3::splat(0.88 * scale),
        ..default()
    });

    TurretJoint {
        name: None,
        offset: Vec3::new(0.0, -mount, 0.0),
        axis: None,
        speed: std::f32::consts::PI,
        min: None,
        max: None,
        // The stow housing replaces the old default base plate. Authored at
        // the shipped mount size (see `TurretArt::housing_mesh`), so no art
        // transform rides it.
        render_mesh: Some(art_spec.housing_mesh.clone()),
        render_mesh_transform: None,
        muzzle: None,
        children: vec![TurretJoint {
            // The stow elevator: a fixed joint the `StowLift` track drives
            // by NAME (`SectionAnimation.node_prefix` resolves named joints
            // exactly like named scene nodes). Everything above it - yaw,
            // pitch, barrels, muzzles - rides it down into the housing.
            name: Some("stow_lift".to_string()),
            offset: Vec3::ZERO,
            axis: None,
            speed: std::f32::consts::PI,
            min: None,
            max: None,
            render_mesh: None,
            render_mesh_transform: platform,
            muzzle: None,
            children: vec![TurretJoint {
                name: None,
                offset: at(art_spec.yaw_at),
                axis: Some(Vec3::Y),
                speed: std::f32::consts::PI, // 180 degrees per second
                min: None,
                max: None,
                render_mesh: Some(art_spec.yaw_mesh.clone()),
                render_mesh_transform: art,
                muzzle: None,
                children: vec![TurretJoint {
                    name: None,
                    offset: at(art_spec.pitch_at),
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
                    render_mesh: Some(art_spec.pitch_mesh.clone()),
                    render_mesh_transform: art,
                    muzzle: None,
                    children: vec![TurretJoint {
                        name: None,
                        offset: at(art_spec.barrel_at),
                        axis: None,
                        speed: std::f32::consts::PI,
                        min: None,
                        max: None,
                        render_mesh: Some(art_spec.barrel_mesh.clone()),
                        render_mesh_transform: art,
                        muzzle: None,
                        children: muzzles,
                    }],
                }],
            }],
        }],
    }
}

use crate::base_content::assets::BaseContentAssets;

/// The bay's footprint: one cell across, two cells long down the firing axis.
/// The promoted tube art (`bay_tube.glb`) is drawn at this size, muzzle
/// toward -Z.
const BAY_CELLS: Vec3 = Vec3::new(1.0, 1.0, 2.0);

/// The lance's footprint: capacitor casing aft, bore and brake fore.
const LANCE_CELLS: Vec3 = Vec3::new(1.0, 1.0, 3.0);

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

/// How deep the stow lift sinks the assembly, in section units. Derived from
/// the tallest stowed column, not the owner's sketch: the twin's straight-up
/// barrel tops out 0.925 above the section origin (pitch hinge +0.35, barrel
/// block and muzzle points 0.575 above it), and the shut lids' underside sits
/// at +0.21, so 0.8 parks the tallest tip at +0.125 with clearance. The
/// excess column length rides below the mount's base plate INTO the hull the
/// mount is bolted to - which is what stowing into the ship means.
const PDC_STOW_SINK: f32 = 0.8;

/// How far each lid half slides to seal the deck: from its parked centre at
/// +-0.37 to +-0.13, where the two 0.26-wide slabs meet over the 0.48 shaft
/// mouth. The left lid is authored mirror-rotated, so ONE signed travel
/// serves both (`SectionAnimationMotion::Translate` slides in each node's
/// own rest frame).
const PDC_STOW_LID_TRAVEL: f32 = 0.24;

/// The shared PDC stow tracks: the `stow_lift` elevator JOINT (code-built,
/// named in `turret_joint_tree`) and the `stow_lid_*` nodes modelled into
/// `pdc_housing.glb`. The turret's stow state machine sequences the two
/// cues; these author only WHAT moves and how fast.
///
/// Deploy fast, stow lazy - the close times are the combat-relevant ones
/// (1 = stowed is these tracks' travelled pose): a threat pops the lids in
/// 0.25 s and raises the gun in 0.35 s, while the fold-away runs at an
/// unhurried service pace nobody is waiting on.
fn pdc_stow_tracks() -> Vec<SectionAnimation> {
    vec![
        SectionAnimation {
            cue: SectionAnimationCue::StowLift,
            node_prefix: "stow_lift".to_string(),
            motion: SectionAnimationMotion::Translate {
                offset: Vec3::new(0.0, -PDC_STOW_SINK, 0.0),
            },
            open_seconds: 0.9,
            close_seconds: 0.35,
        },
        SectionAnimation {
            cue: SectionAnimationCue::StowDoors,
            node_prefix: "stow_lid_".to_string(),
            motion: SectionAnimationMotion::Translate {
                offset: Vec3::new(-PDC_STOW_LID_TRAVEL, 0.0, 0.0),
            },
            open_seconds: 0.5,
            close_seconds: 0.25,
        },
    ]
}

/// One compact PDC prototype, parameterised on the MOUNT it wears and the
/// ROUND it loads.
///
/// The shipped PDCs share the mount box, the magazine and the ballistics, and
/// differ in exactly two authored things: the art assembly with its per-muzzle
/// cadence (gatling: one muzzle at full rate; twin: two muzzles at half), and
/// the round's type and per-hit damage. Sharing one builder is what keeps the
/// side-by-side comparison honest - a mount or cadence retune cannot drift one
/// copy against the other, so what the player feels is the punch-versus-rake
/// (or single-versus-twin-stream) difference and nothing else.
fn pdc_turret_prototype(
    meshes: &BaseContentAssets,
    art: &TurretArt<'_>,
    fire_rate: f32,
    // Per MOUNT and not per damage type: a kinetic and a pierce twin are the
    // same gun firing different ammunition.
    fire_sound: &AssetRef<AudioSource>,
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
            animations: pdc_stow_tracks(),
        },
        kind: SectionKind::Turret(TurretSectionConfig {
            root: turret_joint_tree(
                art,
                fire_rate,
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
            fire_sound: Some(fire_sound.clone()),
            dry_fire_sound: Some(meshes.turret_dry_fire_sound.clone()),
            // The housing this prototype already authors `pdc_stow_tracks` for
            // - so every mount that can fold has a voice for folding.
            stow_open_sound: Some(meshes.turret_stow_open_sound.clone()),
            stow_close_sound: Some(meshes.turret_stow_close_sound.clone()),
            ammo_capacity: Some(500),
            reload: Some(SectionReloadConfig {
                delay: 3.0,
                amount: 200,
            }),
        }),
    }
}

fn drive_mount_points(cells: UVec3) -> Vec<LinkPoint> {
    let half = (cells.as_vec3() - Vec3::ONE) * 0.5;
    let mut points: Vec<LinkPoint> = (0..cells.x)
        .flat_map(|x| (0..cells.y).map(move |y| (x, y)))
        .map(|(x, y)| LinkPoint {
            id: format!("base_{x}_{y}"),
            position: Vec3::new(
                x as f32 - half.x,
                y as f32 - half.y,
                -(cells.z as f32) * 0.5,
            ),
            normal: Vec3::NEG_Z,
        })
        .collect();
    points.sort_by_key(|point| {
        let radial = point.position.x * point.position.x + point.position.y * point.position.y;
        ((radial * 4.0).round() as i32, point.id.clone())
    });
    points
}

struct LargeDriveSpec<'a> {
    id: &'a str,
    name: &'a str,
    description: &'a str,
    cells: UVec3,
    health: f32,
    magnitude: f32,
    mesh: &'a AssetRef<WorldAsset>,
    /// The drive's own hum. The three sizes run 34 / 52 / 78 Hz and are
    /// separated by pitch alone.
    loop_sound: &'a AssetRef<AudioSource>,
    exhaust_offset: f32,
    exhaust_radius: f32,
    exhaust_inner_radius: f32,
    exhaust_height: f32,
}

fn large_thruster_prototype(meshes: &BaseContentAssets, spec: LargeDriveSpec<'_>) -> SectionConfig {
    SectionConfig {
        base: BaseSectionConfig {
            id: spec.id.to_string(),
            name: spec.name.to_string(),
            description: spec.description.to_string(),
            health: spec.health,
            impact_sound: Some(meshes.section_impact_sound.clone()),
            destroy_sound: Some(meshes.section_destroy_sound.clone()),
            collider: Some(SectionCollider::Cuboid {
                size: spec.cells.as_vec3(),
            }),
            link_points: drive_mount_points(spec.cells),
            hide_in_editor: false,
            damage_effects: DamageEffects(vec![
                DamageEffect::Cracks,
                DamageEffect::Sparks,
                DamageEffect::Plume,
            ]),
            animations: Vec::new(),
        },
        kind: SectionKind::Thruster(ThrusterSectionConfig {
            magnitude: spec.magnitude,
            render_mesh: Some(spec.mesh.clone()),
            render_mesh_transform: None,
            loop_sound: Some(spec.loop_sound.clone()),
            exhaust: Some(ThrusterExhaust {
                offset: Vec3::new(0.0, 0.0, spec.exhaust_offset),
                shape: ThrusterExhaustConfig {
                    exhaust_height: spec.exhaust_height,
                    exhaust_radius: spec.exhaust_radius,
                    exhaust_inner_height: spec.exhaust_height * 0.5,
                    exhaust_inner_radius: spec.exhaust_inner_radius,
                    ..default()
                },
                ..default()
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
                animations: Vec::new(),
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
                // ONE socket, on the mounting face. The authored bell opens
                // toward +Z and the plume fires out of it. So the flat forward
                // end is the only structure on the part - the other five faces
                // are drive and exhaust. Six sockets offered them
                // all as mating surfaces, and a builder would bolt a hull slab
                // onto the barrel or plate one across the nozzle.
                link_points: vec![LinkPoint {
                    id: "base".to_string(),
                    position: Vec3::NEG_Z * 0.5,
                    normal: Vec3::NEG_Z,
                }],
                hide_in_editor: false,
                animations: Vec::new(),
            },
            kind: SectionKind::Thruster(ThrusterSectionConfig {
                magnitude: 1.0,
                render_mesh: Some(meshes.thruster_bell.clone()),
                render_mesh_transform: None,
                loop_sound: Some(meshes.thruster_loop_sound.clone()),
                exhaust: Some(ThrusterExhaust {
                    offset: Vec3::new(0.0, 0.0, 0.51),
                    shape: ThrusterExhaustConfig {
                        exhaust_radius: 0.24,
                        exhaust_inner_radius: 0.07,
                        ..default()
                    },
                    ..default()
                }),
            }),
        },
        // Large-drive mass stays physical collider volume, but thrust follows
        // exhaust-face area and health follows a compressed, surface-like curve.
        // Linear volume scaling made the arena capital too fast and too durable.
        large_thruster_prototype(
            meshes,
            LargeDriveSpec {
                id: "vector_thruster_section",
                name: "Vector Thruster Section",
                description: "A 3x3x2 vectoring drive for larger ships.",
                cells: UVec3::new(3, 3, 2),
                health: 480.0,
                magnitude: 9.0,
                mesh: &meshes.thruster_vector,
                loop_sound: &meshes.thruster_vector_loop_sound,
                exhaust_offset: 0.886,
                exhaust_radius: 0.58,
                exhaust_inner_radius: 0.18,
                exhaust_height: 0.3,
            },
        ),
        large_thruster_prototype(
            meshes,
            LargeDriveSpec {
                id: "capital_thruster_section",
                name: "Capital Thruster Section",
                description: "A 5x5x3 capital drive for the largest ships.",
                cells: UVec3::new(5, 5, 3),
                health: 1250.0,
                magnitude: 25.0,
                mesh: &meshes.thruster_capital,
                loop_sound: &meshes.thruster_capital_loop_sound,
                exhaust_offset: 1.51,
                exhaust_radius: 1.1,
                exhaust_inner_radius: 0.35,
                exhaust_height: 0.5,
            },
        ),
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
                animations: Vec::new(),
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
                //
                // The cable-wrapped computer cell: the first controller with a
                // body of its own instead of an invisible cube. Every face
                // carries the same signal pattern, so its rotation never shows.
                render_mesh: Some(meshes.controller_core.clone()),
                render_mesh_transform: None,
                lock_on_sound: Some(meshes.controller_lock_on_sound.clone()),
                lock_off_sound: Some(meshes.controller_lock_off_sound.clone()),
                radar_deny_sound: Some(meshes.controller_radar_deny_sound.clone()),
                radar_retarget_sound: Some(meshes.controller_radar_retarget_sound.clone()),
                safety_on_sound: Some(meshes.controller_safety_on_sound.clone()),
                warn_lock_sound: Some(meshes.controller_warn_lock_sound.clone()),
                ammo_dry_sound: Some(meshes.controller_ammo_dry_sound.clone()),
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
                animations: Vec::new(),
            },
            kind: SectionKind::Hull(HullSectionConfig {
                render_mesh: Some(meshes.hull.clone()),
                render_mesh_transform: None,
            }),
        },
        // The cargo and tank hulls are the reinforced hull in different
        // clothes: same stats on purpose, so today they are a visual choice.
        // The models are the investment - when real hull TYPES arrive
        // (resources, cargo capacity), these two are where the stats land.
        SectionConfig {
            base: BaseSectionConfig {
                id: "cargo_hull_section".to_string(),
                damage_effects: DamageEffects(vec![DamageEffect::Cracks]),
                name: "Cargo Hull Section".to_string(),
                description: "A hull section packed with caged freight; every \
                              face reads the same."
                    .to_string(),
                health: 200.0,
                impact_sound: Some(meshes.section_impact_sound.clone()),
                destroy_sound: Some(meshes.section_destroy_sound.clone()),
                collider: None,
                link_points: unit_cube_link_points(),
                hide_in_editor: false,
                animations: Vec::new(),
            },
            kind: SectionKind::Hull(HullSectionConfig {
                render_mesh: Some(meshes.hull_cargo.clone()),
                render_mesh_transform: None,
            }),
        },
        SectionConfig {
            base: BaseSectionConfig {
                id: "tank_hull_section".to_string(),
                damage_effects: DamageEffects(vec![DamageEffect::Cracks]),
                name: "Tank Hull Section".to_string(),
                description: "A hull section carrying a pressure vessel in \
                              open frame rails."
                    .to_string(),
                health: 200.0,
                impact_sound: Some(meshes.section_impact_sound.clone()),
                destroy_sound: Some(meshes.section_destroy_sound.clone()),
                collider: None,
                link_points: unit_cube_link_points(),
                hide_in_editor: false,
                animations: Vec::new(),
            },
            kind: SectionKind::Hull(HullSectionConfig {
                render_mesh: Some(meshes.hull_tank.clone()),
                render_mesh_transform: None,
            }),
        },
        pdc_turret_prototype(
            meshes,
            &gatling_art(meshes),
            GATLING_FIRE_RATE,
            &meshes.turret_fire_sound,
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
            &gatling_art(meshes),
            GATLING_FIRE_RATE,
            &meshes.turret_fire_sound,
            "pdc_pierce_turret_section",
            "PDC Turret (Pierce)",
            "The same mount firing penetrators. Half the damage per hit, dealt \
             to EVERY section the round rakes through - closing fast buys depth, \
             not damage. Worse against one thin target, better against a deep \
             one.",
            DamageType::Pierce,
            PIERCE_PDC_BULLET_DAMAGE,
        ),
        pdc_turret_prototype(
            meshes,
            &twin_art(meshes),
            TWIN_FIRE_RATE,
            &meshes.turret_twin_fire_sound,
            "pdc_twin_kinetic_turret_section",
            "Twin PDC Turret (Kinetic)",
            "The same slugs from a two-barrel mount. Each tube fires at half \
             the gatling's cadence, so the magazine drains no faster - the \
             trade is two offset streams instead of one dense one.",
            DamageType::Kinetic,
            KINETIC_PDC_BULLET_DAMAGE,
        ),
        pdc_turret_prototype(
            meshes,
            &twin_art(meshes),
            TWIN_FIRE_RATE,
            &meshes.turret_twin_fire_sound,
            "pdc_twin_pierce_turret_section",
            "Twin PDC Turret (Pierce)",
            "Penetrators from the two-barrel mount: half per-hit damage dealt \
             through every layer, split across two offset streams at the same \
             total rate.",
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
                id: RAILGUN_LANCE_SECTION_ID.to_string(),
                name: "Railgun Lance".to_string(),
                description: "A spinal kinetic lance: no traverse, so the HULL \
                              aims it. Tapping the trigger COMMITS - the bolt \
                              walks the bore and the shot leaves when it \
                              arrives, whether or not the nose is still on the \
                              target. What leaves rakes through everything in \
                              the line, and shoves the ship that fired."
                    .to_string(),
                health: RAILGUN_BASE_HEALTH,
                impact_sound: Some(meshes.section_impact_sound.clone()),
                destroy_sound: Some(meshes.section_destroy_sound.clone()),
                // Three cells long, and the collider has to claim all of it:
                // the lance is the biggest single target on any ship carrying
                // one.
                collider: Some(SectionCollider::Cuboid { size: LANCE_CELLS }),
                link_points: lance_link_points(),
                hide_in_editor: false,
                // Capacitor banks and rails: it arcs and sparks as it fails.
                damage_effects: DamageEffects(vec![DamageEffect::Cracks, DamageEffect::Sparks]),
                animations: lance_charge_bolt(),
            },
            kind: SectionKind::Railgun(RailgunSectionConfig {
                render_mesh: Some(meshes.railgun_lance.clone()),
                render_mesh_transform: None,
                // ON the brake face. The recoil is applied at this point, not
                // at the centre of mass, so a lance bolted off the ship's axis
                // yaws it as well as pushing it - which is the whole reason a
                // builder puts one on the spine.
                muzzle_offset: Vec3::NEG_Z * (LANCE_CELLS.z * 0.5),
                // The alignment window, and the tell. Long enough that a
                // target being lanced can see the bolt climbing the bore and
                // break the line; short enough that a pilot who has already
                // set up the shot is not flying a straight line forever.
                charge_seconds: 1.5,
                slug_speed: 1500.0,
                // Dealt in FULL to every layer crossed, and priced to leave
                // NOTHING standing: the toughest thing in the catalog is a
                // reinforced hull block at 200, so this kills every shipped
                // section on the layer it crosses with half again to spare.
                // An aligned shot does not damage a column, it removes one.
                //
                // Past roughly 260 the extra buys nothing against content that
                // exists - depth is priced in `slug_power`, not here, and
                // everything already dies. The margin is for what a mod
                // authors, not for the base game.
                slug_damage: 300.0,
                // The owner's brief: through the entire ship. At the 3x
                // closing-speed ceiling a lance slug spends max_health/3 per
                // layer, so this crosses 27 reinforced hull blocks - past the
                // depth of anything that flies. Depth is NOT the cost of this
                // weapon; the commit, the recoil and the reload are.
                slug_power: 1800.0,
                // 1800 u of reach. A spinal gun outranges every mount on the
                // ship carrying it, which is what makes lining up worth doing.
                slug_lifetime: 1.2,
                // Raw per-shot impulse, in the same register as a thruster's
                // per-tick magnitude: about two thirds of a second of the
                // basic drive's full burn, delivered in one instant.
                recoil_impulse: 45.0,
                fire_sound: Some(meshes.railgun_fire_sound.clone()),
                charge_sound: Some(meshes.railgun_charge_sound.clone()),
                reload_sound: Some(meshes.railgun_reload_sound.clone()),
                // One shell in the air per gun, ever. The magazine IS the
                // design: a lance that could queue a second shot would be a
                // turret with a long fire rate.
                ammo_capacity: Some(1),
                // The tempo. Twelve quiet seconds return the shell, so a lance
                // fires roughly every thirteen and a half - and every one of
                // those is a decision rather than a trigger pull.
                reload: Some(SectionReloadConfig {
                    delay: 12.0,
                    amount: 1,
                }),
            }),
        },
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
                collider: Some(SectionCollider::Cuboid { size: BAY_CELLS }),
                // The same bay art at siege grade, so the same open muzzle.
                link_points: bay_link_points(),
                // Scene dressing, not player kit: this armoured finisher cuts
                // much deeper than standard ordnance, so the editor gallery
                // does not offer it.
                hide_in_editor: true,
                // The same tube art, so the same iris: see `bay_muzzle_door`.
                animations: bay_muzzle_door(),
            },
            kind: SectionKind::Torpedo(TorpedoSectionConfig {
                render_mesh: Some(meshes.torpedo_bay.clone()),
                render_mesh_transform: None,
                projectile_render_mesh: None,
                // The muzzle point on the door plane and the centred birth,
                // same as the standard bay: it launches out of the same tube.
                spawn_offset: Vec3::NEG_Z * BAY_CELLS.z * 0.5,
                // Aim the tube out of the open face. The launch axis is
                // the spawner's +Y and this turns it onto the section's -Z,
                // the one face `link_points` leaves unlinkable so it can be
                // a muzzle. Without it the tube ejected through its own roof.
                spawn_rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
                spawn_recess: BAY_CELLS.z * 0.5,
                fire_rate: 1.0,
                spawner_speed: 8.0,
                // Long enough to cross a backdrop arena, short enough that a
                // torpedo whose target died mid-flight cleans itself up.
                projectile_lifetime: 60.0,
                arm_time: 0.5,
                arm_distance: 5.0,
                // Dropped, then lit: the bay ejects it on a cold charge and the
                // motor catches once it is clear. See `ignition_delay`.
                ignition_delay: 0.6,
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
                door_sound: Some(meshes.torpedo_door_sound.clone()),
                detonation_sound: Some(meshes.torpedo_detonation_sound.clone()),
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
        node_prefix: "door_petal_".to_string(),
        // Past vertical, so the open petals read as a flared crown around
        // the dark throat rather than six posts.
        motion: SectionAnimationMotion::RotateX { degrees: 105.0 },
        open_seconds: 0.25,
        close_seconds: 0.7,
    }]
}

/// The lance's sockets: the breech plate, and one per cell on each flank -
/// thirteen in all.
///
/// The muzzle face (-Z) carries NONE, for the reason every bay's does not: a
/// lance fires down its own axis and cannot traverse off it, so a socket there
/// is an invitation to bolt a plate in front of the bore.
/// `no_lance_sockets_the_face_it_fires_through` holds it.
fn lance_link_points() -> Vec<LinkPoint> {
    let mut points = vec![LinkPoint {
        id: "positive_z".to_string(),
        position: Vec3::Z * (LANCE_CELLS.z * 0.5),
        normal: Vec3::Z,
    }];
    let flanks = [
        ("positive_x", Vec3::X),
        ("negative_x", Vec3::NEG_X),
        ("positive_y", Vec3::Y),
        ("negative_y", Vec3::NEG_Y),
    ];
    for (face, normal) in flanks {
        for (cell, z) in [("fore", -1.0), ("mid", 0.0), ("aft", 1.0)] {
            points.push(LinkPoint {
                id: format!("{face}_{cell}"),
                position: normal * 0.5 + Vec3::Z * z,
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
/// Both travel times are zero, and that is deliberate rather than unset. The
/// firing system SNAPS this track to the gameplay charge fraction every tick
/// (`charge_and_fire_railgun`), so the authored `charge_seconds` is the single
/// clock. A travel time here would be a second one, and the bolt would reach
/// the brake either before or after the shot it is supposed to announce.
fn lance_charge_bolt() -> Vec<SectionAnimation> {
    vec![SectionAnimation {
        cue: SectionAnimationCue::Charge,
        node_prefix: "charge_bolt".to_string(),
        // Breech (-0.06) to the back of the muzzle brake (-1.24): the length
        // of bore left to cross IS the charge left to run.
        motion: SectionAnimationMotion::Translate {
            offset: Vec3::NEG_Z * 1.18,
        },
        open_seconds: 0.0,
        close_seconds: 0.0,
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
            impact_sound: Some(meshes.section_impact_sound.clone()),
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
            // Born at the tube's centre: the 2 u torpedo exactly fills the
            // 2 u tube - nose on the door plane, tail at the back wall - and
            // slides its whole length out through the open iris.
            spawn_recess: BAY_CELLS.z * 0.5,
            fire_rate: 1.0,
            spawner_speed: 8.0,
            projectile_lifetime: 100.0,
            arm_time: 0.5,
            arm_distance: 5.0,
            // Dropped, then lit: the bay ejects it on a cold charge and the
            // motor catches once it is clear. See `ignition_delay`.
            ignition_delay: 0.6,
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
            door_sound: Some(meshes.torpedo_door_sound.clone()),
            // The blast IS the destruction voice: same wav as section
            // destruction (per-target authoring; playtest can diverge it).
            detonation_sound: Some(meshes.torpedo_detonation_sound.clone()),
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
            let name = format!("\"door_petal_{petal}\"");
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
            assert_eq!(door.node_prefix, "door_petal_", "{}", section.base.id);
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

    /// Every catalog turret that wears the stow housing declares both stow
    /// tracks, the lid track's prefix matches real named nodes in the
    /// committed `pdc_housing.glb`, and the lift track's prefix matches the
    /// named elevator joint in the turret's OWN tree. Art, tracks and rig
    /// are one contract: a renamed lid node, a renamed lift joint or a
    /// dropped track breaks here rather than as a gun that silently stops
    /// sinking - or one that can never deploy again.
    #[test]
    fn every_housed_turret_declares_the_stow_tracks_over_its_rig() {
        let glb = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/base/gltf/pdc_housing.glb"
        ))
        .expect("the promoted stow housing art");
        for lid in ["\"stow_lid_right\"", "\"stow_lid_left\""] {
            assert!(
                glb.windows(lid.len())
                    .any(|window| window == lid.as_bytes()),
                "pdc_housing.glb has no {lid} node"
            );
        }

        fn tree_has_named(joint: &TurretJoint, prefix: &str) -> bool {
            joint
                .name
                .as_deref()
                .is_some_and(|name| name.starts_with(prefix))
                || joint
                    .children
                    .iter()
                    .any(|child| tree_has_named(child, prefix))
        }

        let mut housed = 0;
        for section in crate::generation::build_section_catalog() {
            let SectionKind::Turret(turret) = &section.kind else {
                continue;
            };
            let wears_housing = turret
                .root
                .render_mesh
                .as_ref()
                .and_then(|mesh| mesh.path())
                .is_some_and(|path| path.contains("pdc_housing.glb"));
            if !wears_housing {
                continue;
            }
            housed += 1;
            let lift = section
                .base
                .animations
                .iter()
                .find(|track| track.cue == SectionAnimationCue::StowLift)
                .unwrap_or_else(|| {
                    panic!(
                        "{}: housed turret without a StowLift track",
                        section.base.id
                    )
                });
            assert!(
                tree_has_named(&turret.root, &lift.node_prefix),
                "{}: the lift track's prefix {:?} matches no named joint",
                section.base.id,
                lift.node_prefix
            );
            let doors = section
                .base
                .animations
                .iter()
                .find(|track| track.cue == SectionAnimationCue::StowDoors)
                .unwrap_or_else(|| {
                    panic!(
                        "{}: housed turret without a StowDoors track",
                        section.base.id
                    )
                });
            assert_eq!(doors.node_prefix, "stow_lid_", "{}", section.base.id);
        }
        assert!(
            housed >= 2,
            "both shipped PDCs wear the housing; found {housed}"
        );
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

    /// A lance cannot traverse, so the face it fires through is the strictest
    /// exit on the ship: a socket there would let the editor bolt a plate
    /// across the bore and the gun would shoot through its own hull, every
    /// shot. The same rule bays are held to, for a harder reason.
    #[test]
    fn no_lance_sockets_the_face_it_fires_through() {
        for section in crate::generation::build_section_catalog() {
            let SectionKind::Railgun(_) = &section.kind else {
                continue;
            };
            let firing = nova_ship::prelude::exit_normal(&section.kind)
                .expect("a lance declares an exit normal");
            for point in &section.base.link_points {
                assert!(
                    point.normal.dot(firing) < 0.5,
                    "`{}` sockets its own muzzle: `{}` faces {:?}, the lance fires {firing:?}",
                    section.base.id,
                    point.id,
                    point.normal,
                );
            }
        }
    }

    /// The muzzle offset is the recoil's lever arm, so it has to sit ON the
    /// brake face rather than merely somewhere forward. Off the face, the shot
    /// spawns inside the gun (or out in space) AND the torque the hull takes is
    /// wrong by the same distance - one number, two ways to be silently off.
    #[test]
    fn every_lance_puts_its_muzzle_on_the_brake_face() {
        for section in crate::generation::build_section_catalog() {
            let SectionKind::Railgun(railgun) = &section.kind else {
                continue;
            };
            assert_eq!(
                railgun.muzzle_offset,
                Vec3::NEG_Z * (LANCE_CELLS.z * 0.5),
                "`{}` fires from somewhere that is not its brake face",
                section.base.id,
            );
        }
    }

    /// One shell in the air per gun, ever. A lance with room for two is a
    /// turret with a slow fire rate, and the commit stops being a decision.
    #[test]
    fn every_lance_holds_exactly_one_shell() {
        for section in crate::generation::build_section_catalog() {
            let SectionKind::Railgun(railgun) = &section.kind else {
                continue;
            };
            assert_eq!(
                railgun.ammo_capacity,
                Some(1),
                "`{}` can queue a second lance shot",
                section.base.id,
            );
            let reload = railgun
                .reload
                .unwrap_or_else(|| panic!("`{}` never gets its shell back", section.base.id));
            assert_eq!(reload.amount, 1);
            assert!(
                reload.delay > railgun.charge_seconds,
                "`{}` reloads faster than it charges, so the reload is not the tempo",
                section.base.id,
            );
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
        for id in [
            "pdc_kinetic_turret_section",
            "pdc_pierce_turret_section",
            "pdc_twin_kinetic_turret_section",
            "pdc_twin_pierce_turret_section",
        ] {
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

    #[test]
    fn large_drives_use_face_thrust_and_surface_scaled_health() {
        let catalog = crate::generation::build_section_catalog();
        for (id, cells, health, magnitude) in [
            ("vector_thruster_section", UVec3::new(3, 3, 2), 480.0, 9.0),
            (
                "capital_thruster_section",
                UVec3::new(5, 5, 3),
                1250.0,
                25.0,
            ),
        ] {
            let drive = catalog
                .iter()
                .find(|section| section.base.id == id)
                .unwrap_or_else(|| panic!("`{id}` is in the catalog"));
            assert_eq!(drive.base.health, health);
            assert_eq!(drive.base.link_points.len(), (cells.x * cells.y) as usize);
            assert_eq!(
                drive.base.collider,
                Some(SectionCollider::Cuboid {
                    size: cells.as_vec3(),
                })
            );
            let SectionKind::Thruster(config) = &drive.kind else {
                panic!("`{id}` has the wrong section kind");
            };
            assert_eq!(config.magnitude, magnitude);
            assert!(config.render_mesh.is_some());
        }
    }

    /// The drive bolts on by its forward end and by nothing else.
    ///
    /// The authored bell lies on the Z axis and opens toward +Z, so the
    /// mounting end is -Z - NOT the -Y a turret stands on.
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
    fn the_basic_thruster_exhaust_fits_its_bell() {
        let drive = crate::generation::build_section_catalog()
            .into_iter()
            .find(|section| section.base.id == "basic_thruster_section")
            .expect("the basic thruster is in the catalog");
        let SectionKind::Thruster(config) = drive.kind else {
            panic!("the basic thruster has the wrong section kind");
        };
        let exhaust = config
            .exhaust
            .expect("the basic thruster authors its exhaust");
        assert_eq!(exhaust.offset, Vec3::new(0.0, 0.0, 0.51));
        assert_eq!(exhaust.shape.geometry, ThrusterExhaustShape::Cone);
        assert_eq!(exhaust.shape.exhaust_radius, 0.24);
        assert_eq!(exhaust.shape.exhaust_inner_radius, 0.07);
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
    /// unscaled size, which reads as a turret coming apart; scaling the offsets
    /// alone leaves full-size art in a smaller arrangement.
    #[test]
    fn a_scaled_turret_tree_scales_its_offsets_and_its_art_together() {
        let mesh = |name: &str| AssetRef::<WorldAsset>::from(name.to_string());
        let (housing, yaw, pitch, barrel) =
            (mesh("housing"), mesh("yaw"), mesh("pitch"), mesh("barrel"));
        let tree = |scale: f32| {
            turret_joint_tree(
                &TurretArt {
                    housing_mesh: &housing,
                    yaw_mesh: &yaw,
                    pitch_mesh: &pitch,
                    barrel_mesh: &barrel,
                    yaw_at: Vec3::new(0.0, 0.1, 0.0),
                    pitch_at: Vec3::new(0.0, 0.4, 0.0),
                    barrel_at: Vec3::new(0.0, 0.02, -0.1),
                    muzzles_at: &[Vec3::new(0.0, 0.0, -0.95)],
                },
                100.0,
                0.5,
                scale,
            )
        };

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
            match joints {
                // The base wears the housing: final-size art (its recipe is
                // the mount's own cell), so it never takes the tree scale.
                0 => {
                    assert!(a.render_mesh.is_some(), "the base wears the housing");
                    assert!(
                        a.render_mesh_transform.is_none() && b.render_mesh_transform.is_none(),
                        "the housing is authored at mount size, not rescaled"
                    );
                }
                // The stow elevator: named for its animation track, and its
                // platform placement is authored art at EVERY scale - the
                // whole transform rides the tree scale.
                1 => {
                    assert_eq!(a.name.as_deref(), Some("stow_lift"));
                    let art_a = a.render_mesh_transform.expect("the platform placement");
                    let art_b = b.render_mesh_transform.expect("the platform placement");
                    assert!(art_b.scale.abs_diff_eq(art_a.scale * 0.5, 1e-6));
                    assert!(art_b.position.abs_diff_eq(art_a.position * 0.5, 1e-6));
                }
                // The unit-drawn parts: meshed, no transform at unit scale
                // (so the shipped RON stays clean), the scale on each part's
                // transform at half.
                2..=4 => {
                    assert!(a.render_mesh.is_some(), "joint {joints}: meshed");
                    assert!(
                        a.render_mesh_transform.is_none(),
                        "joint {joints}: an unscaled tree authors no art transform"
                    );
                    assert_eq!(
                        b.render_mesh_transform.map(|art| art.scale),
                        Some(Vec3::splat(0.5)),
                        "joint {joints}: meshed but unscaled art"
                    );
                }
                // The muzzle: an invisible fire point.
                _ => {
                    assert!(a.render_mesh.is_none() && a.render_mesh_transform.is_none());
                }
            }
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
        assert_eq!(joints, 6, "base, lift, yaw, pitch, barrel, muzzle");
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

    fn catalog_turret(id: &str) -> TurretSectionConfig {
        crate::generation::build_section_catalog()
            .into_iter()
            .find(|section| section.base.id == id)
            .map(|section| match section.kind {
                SectionKind::Turret(turret) => turret,
                other => panic!("`{id}` is not a turret: {other:?}"),
            })
            .unwrap_or_else(|| panic!("the catalog ships `{id}`"))
    }

    /// Every fire rate in a turret tree, one per muzzle leaf.
    fn muzzle_rates(joint: &TurretJoint) -> Vec<f32> {
        let mut rates: Vec<f32> = joint
            .muzzle
            .as_ref()
            .map(|muzzle| muzzle.fire_rate)
            .into_iter()
            .collect();
        for child in &joint.children {
            rates.extend(muzzle_rates(child));
        }
        rates
    }

    /// Each mount's two PDCs exist to be COMPARED: mount one of each and the
    /// only difference the player can feel is the ROUND - its type and its
    /// per-hit damage. Mount, joint tree, fire rate and magazine must be
    /// identical, or the comparison measures something else. Debug strings
    /// stand in for structural equality (`TurretSectionConfig` has no
    /// `PartialEq`), which is enough to catch any other field drifting between
    /// them. Both the gatling pair and the twin pair are held to it.
    #[test]
    fn the_two_pdcs_differ_only_in_the_round_they_load() {
        for (kinetic_id, pierce_id) in [
            ("pdc_kinetic_turret_section", "pdc_pierce_turret_section"),
            (
                "pdc_twin_kinetic_turret_section",
                "pdc_twin_pierce_turret_section",
            ),
        ] {
            let kinetic = catalog_turret(kinetic_id);
            let mut pierce = catalog_turret(pierce_id);

            assert_eq!(kinetic.bullet_kind, DamageType::Kinetic);
            assert_eq!(pierce.bullet_kind, DamageType::Pierce);
            // The trade: a rake gives up per-hit damage for depth, so the slug
            // must stay the harder single hit. Without this the two guns would
            // be a strict upgrade rather than a choice.
            assert!(
                pierce.bullet_damage < kinetic.bullet_damage,
                "`{pierce_id}` must hit softer per contact ({} vs {})",
                pierce.bullet_damage,
                kinetic.bullet_damage
            );

            pierce.bullet_kind = kinetic.bullet_kind;
            pierce.bullet_damage = kinetic.bullet_damage;
            assert_eq!(
                format!("{kinetic:?}"),
                format!("{pierce:?}"),
                "`{kinetic_id}` and `{pierce_id}` must be the same gun apart \
                 from the round they load"
            );
        }
    }

    /// The twin is the gatling's coverage variant, not its upgrade: two
    /// muzzles, each at half the gatling's cadence, so both mounts spend the
    /// shared magazine at the same total rate and the choice between them is
    /// stream shape, not DPS. The mirrored muzzle offsets are the coverage:
    /// two streams a barrel-spacing apart instead of one dense one.
    #[test]
    fn the_twin_mount_splits_the_same_total_rate_across_two_muzzles() {
        let gatling = muzzle_rates(&catalog_turret("pdc_kinetic_turret_section").root);
        let twin = muzzle_rates(&catalog_turret("pdc_twin_kinetic_turret_section").root);
        assert_eq!(gatling.len(), 1, "the gatling fires one stream");
        assert_eq!(twin.len(), 2, "the twin fires two streams");
        assert_eq!(
            gatling.iter().sum::<f32>(),
            twin.iter().sum::<f32>(),
            "the two mounts must drain the shared magazine at the same total rate"
        );

        // Walk to the barrel joint: its children are the muzzle leaves.
        let root = catalog_turret("pdc_twin_kinetic_turret_section").root;
        let mut barrel = &root;
        while barrel.children.len() == 1 {
            barrel = &barrel.children[0];
        }
        let [port, starboard] = barrel.children.as_slice() else {
            panic!("the twin barrel carries two muzzle leaves");
        };
        assert!(
            port.offset.x > 0.0 && (port.offset.x + starboard.offset.x).abs() < 1e-6,
            "the twin's muzzles mirror across the barrel line ({} vs {})",
            port.offset.x,
            starboard.offset.x
        );
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
        for id in [
            "torpedo_section",
            "lance_torpedo_section",
            "heavy_torpedo_section",
        ] {
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
