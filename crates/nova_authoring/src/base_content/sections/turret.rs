//! The point-defense mounts: one gun, four prototypes.
//!
//! The shipped PDCs share the mount box, the magazine and the ballistics, and
//! differ in exactly two authored things - the art assembly with its per-muzzle
//! cadence, and the round's damage type and per-hit damage. One builder
//! ([`pdc_turret_prototype`]) is what keeps the comparison honest.

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;

use crate::base_content::assets::BaseContentAssets;

/// Armoured weapon mount: shrugs off more than the mid baseline. The HEALTH
/// half of "variable damage by section type"; the other half is
/// `nova_gameplay::damage`'s resistance table.
pub(super) const TURRET_BASE_HEALTH: f32 = 130.0;

// Authored per-hit Kinetic damage of the shared PDC. A point-defense profile:
// LOW per-hit, HIGH rate (100 rounds/s). At 4.0 the PDC does ~400 DPS, and a
// 60-HP light section takes 15 rounds (~0.15 s of fire), so a burst visibly
// chips it down rather than destroying it in one hit. The low per-hit is also
// what keeps ship TTK long enough to read as point defense.
//
// Rock is NOT priced here. An asteroid has no health pool since the carve pass:
// its durability is the material left in it, priced at `DAMAGE_PER_UNIT_VOLUME`
// (nova_gameplay::integrity::carve), which is calibrated AGAINST this number.
// Moving this moves every rock's time-to-kill with it.
//
// Every craft mounts this one gun, so this number is the whole gunnery curve:
// a lighter craft is made weaker by its HULL and its mount's health, not by a
// second, softer turret prototype.
const KINETIC_PDC_BULLET_DAMAGE: f32 = 4.0;

/// The joint the `StowLift` track drives, and the node prefix the `StowDoors`
/// track drives. Builder and test read the same names, so a renamed joint or
/// lid fails loudly.
const STOW_LIFT_JOINT_NAME: &str = "stow_lift";
const STOW_LID_NODE_PREFIX: &str = "stow_lid_";

/// The two authoring-owned mount ids. Both single mounts are named by the
/// generator, so their ids come from `nova_ship` instead.
const PDC_TWIN_KINETIC_TURRET_SECTION_ID: &str = "pdc_twin_kinetic_turret_section";
const PDC_TWIN_PIERCE_TURRET_SECTION_ID: &str = "pdc_twin_pierce_turret_section";

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
/// streams walking onto a target - not more damage.
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

/// How far a shipped turret's pitch hinge may DEPRESS below level (10 deg).
/// Every shipped mount sits ON a hull, so a deeper floor only swings the barrel
/// back across its own ship.
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

/// The barrel id a single-barrel mount answers to.
pub const MUZZLE_MAIN: &str = "main";
/// The port barrel of a multi-barrel mount.
pub const MUZZLE_LEFT: &str = "left";
/// The starboard barrel of a multi-barrel mount.
pub const MUZZLE_RIGHT: &str = "right";

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
/// The id one barrel answers to in a patch: a single-barrel mount is `main`,
/// and a multi-barrel mount names its tubes by the side they sit on. The id is
/// what content aims a [`MuzzleConfigPatch`] at, so it has to read like the
/// gun and stay put when the art moves.
fn muzzle_id(muzzles_at: &[Vec3], muzzle_at: Vec3) -> String {
    if muzzles_at.len() == 1 {
        return MUZZLE_MAIN.to_string();
    }
    if muzzle_at.x < 0.0 {
        MUZZLE_LEFT.to_string()
    } else {
        MUZZLE_RIGHT.to_string()
    }
}

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
                id: muzzle_id(art_spec.muzzles_at, muzzle_at),
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
            name: Some(STOW_LIFT_JOINT_NAME.to_string()),
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
                    // deep depression just aims the barrel into its own ship.
                    // 10 degrees is enough to reach a target slightly below the
                    // mount without the muzzle sweeping back across the
                    // bodywork. Elevation stays at 90: straight up is the
                    // point-defense arc.
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

/// How deep the stow lift sinks the assembly, in section units. Derived from
/// the tallest stowed column rather than from the art: the twin's straight-up
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
            node_prefix: STOW_LIFT_JOINT_NAME.to_string(),
            motion: SectionAnimationMotion::Translate {
                offset: Vec3::new(0.0, -PDC_STOW_SINK, 0.0),
            },
            open_seconds: 0.9,
            close_seconds: 0.35,
        },
        SectionAnimation {
            cue: SectionAnimationCue::StowDoors,
            node_prefix: STOW_LID_NODE_PREFIX.to_string(),
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
            destroy_sound: Some(meshes.section_destroy_sound.clone()),
            // A turret is all function - the barrel has to point and the mount
            // has to turn - so it fails by sparking and never by losing a
            // piece of itself.
            damage_effects: DamageEffects(vec![DamageEffect::Cracks, DamageEffect::Sparks]),
            // A small box that sits ON a hull face instead of replacing one.
            // One prototype can therefore serve every mounting orientation.
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
            muzzle_speed: MetersPerSecond(1_000.0),
            // 1,000 m/s x 2.0 s = 2.0 km of reach, the top of the intended
            // 1-2 km PDC band. Lifetime is the ONLY reach knob a
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
            // The housing uses `pdc_stow_tracks`, so every folding mount also
            // carries open and close audio.
            stow_open_sound: Some(meshes.turret_stow_open_sound.clone()),
            stow_close_sound: Some(meshes.turret_stow_close_sound.clone()),
            ammunition: AmmoCapacity::Limited(500),
            reload: ReloadConfig::Batch(SectionReloadConfig {
                delay: 3.0,
                amount: 200,
            }),
        }),
    }
}

/// The mount prototypes, in catalog order.
pub(super) fn prototypes(meshes: &BaseContentAssets) -> Vec<SectionConfig> {
    vec![
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
            PDC_PIERCE_TURRET_SECTION_ID,
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
            PDC_TWIN_KINETIC_TURRET_SECTION_ID,
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
            PDC_TWIN_PIERCE_TURRET_SECTION_ID,
            "Twin PDC Turret (Pierce)",
            "Penetrators from the two-barrel mount: half per-hit damage dealt \
             through every layer, split across two offset streams at the same \
             total rate.",
            DamageType::Pierce,
            PIERCE_PDC_BULLET_DAMAGE,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

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
            assert_eq!(
                doors.node_prefix, STOW_LID_NODE_PREFIX,
                "{}",
                section.base.id
            );
        }
        assert!(
            housed >= 2,
            "both shipped PDCs wear the housing; found {housed}"
        );
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
            PDC_KINETIC_TURRET_SECTION_ID,
            PDC_PIERCE_TURRET_SECTION_ID,
            PDC_TWIN_KINETIC_TURRET_SECTION_ID,
            PDC_TWIN_PIERCE_TURRET_SECTION_ID,
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
                    assert_eq!(a.name.as_deref(), Some(STOW_LIFT_JOINT_NAME));
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
            (PDC_KINETIC_TURRET_SECTION_ID, PDC_PIERCE_TURRET_SECTION_ID),
            (
                PDC_TWIN_KINETIC_TURRET_SECTION_ID,
                PDC_TWIN_PIERCE_TURRET_SECTION_ID,
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
        let gatling = muzzle_rates(&catalog_turret(PDC_KINETIC_TURRET_SECTION_ID).root);
        let twin = muzzle_rates(&catalog_turret(PDC_TWIN_KINETIC_TURRET_SECTION_ID).root);
        assert_eq!(gatling.len(), 1, "the gatling fires one stream");
        assert_eq!(twin.len(), 2, "the twin fires two streams");
        assert_eq!(
            gatling.iter().sum::<f32>(),
            twin.iter().sum::<f32>(),
            "the two mounts must drain the shared magazine at the same total rate"
        );

        // Walk to the barrel joint: its children are the muzzle leaves.
        let root = catalog_turret(PDC_TWIN_KINETIC_TURRET_SECTION_ID).root;
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

    /// The PDC's per-hit stays low enough that the softest thing it shoots at
    /// takes a burst rather than one round: at ~20 per hit a light hull section
    /// dies in three rounds, which is 30 ms of trigger.
    ///
    /// Anchored on the softest SHIPPED section rather than on a rock. A rock
    /// carries no health pool - its durability is the material left in it,
    /// priced at `DAMAGE_PER_UNIT_VOLUME`, and the smallest one a scenario
    /// scatters is about 140 cubic units, some 280 rounds - so the section is
    /// the tighter bound.
    ///
    /// A loose guard rather than a balance number: raise it consciously.
    #[test]
    fn pdc_per_hit_stays_below_the_one_shot_ceiling() {
        /// `light_hull_section`: the least health any shipped section
        /// carries.
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
