//! The spinal lance: no traverse, so the HULL aims it.
//!
//! Tapping the trigger COMMITS - the bolt walks the bore and the shot leaves
//! when it arrives, whether or not the nose is still on the target. What leaves
//! rakes through everything in the line, and shoves the ship that fired.

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_ship::prelude::*;

use crate::base_content::assets::BaseContentAssets;

/// Three cells of ship behind ONE health pool, which is the lance's whole
/// structural cost: a builder who mounts one is trading a spine's worth of
/// separately-killable sections for a single target. Above the turret baseline
/// because it is armoured capacitor casing, well below three reinforced hull
/// blocks (600) because it must stay a weak spot worth shooting at.
const RAILGUN_BASE_HEALTH: f32 = 180.0;

/// The lance's footprint: capacitor casing aft, bore and brake fore.
const LANCE_CELLS: Vec3 = Vec3::new(1.0, 1.0, 3.0);

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

/// What one lance grade costs its target. Everything else - the art, the
/// bore, the commit, the recoil, the single shell and the twelve-second
/// reload - is the WEAPON, and is shared by construction below.
struct LanceSpec<'a> {
    id: &'a str,
    name: &'a str,
    description: &'a str,
    /// Damage dealt in full to every layer the slug crosses.
    slug_damage: f32,
    /// The pierce budget: the only bound on how deep one shell goes.
    slug_power: f32,
    /// How wide a corridor the budget is spent on.
    rake_radius: Meters,
}

/// One spinal lance, named for the GRADE it is built at.
///
/// A grade is a second PROTOTYPE, never a spawn-time knob, for the reason the
/// catalog draws that line everywhere else (see `ships`). One builder for all
/// of them means a tempo or recoil edit lands on every grade and cannot become
/// an unmeasured extra difference.
fn railgun_lance_prototype(meshes: &BaseContentAssets, spec: LanceSpec<'_>) -> SectionConfig {
    SectionConfig {
        base: BaseSectionConfig {
            id: spec.id.to_string(),
            name: spec.name.to_string(),
            description: spec.description.to_string(),
            health: RAILGUN_BASE_HEALTH,
            destroy_sound: Some(meshes.section_destroy_sound.clone()),
            // Three cells long, and the collider has to claim all of it: the
            // lance is the biggest single target on any ship carrying one.
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
            // ON the brake face. The recoil is applied at this point, not at
            // the centre of mass, so a lance bolted off the ship's axis yaws it
            // as well as pushing it - which is the whole reason a builder puts
            // one on the spine.
            muzzle_offset: Vec3::NEG_Z * (LANCE_CELLS.z * 0.5),
            // The alignment window, and the tell. Long enough that a target
            // being lanced can see the bolt climbing the bore and break the
            // line; short enough that a pilot who has already set up the shot
            // is not flying a straight line forever.
            charge_seconds: 1.5,
            slug_speed: MetersPerSecond(15_000.0),
            slug_damage: spec.slug_damage,
            slug_power: spec.slug_power,
            // Wider is NOT free: against dense material the power budget binds,
            // so a rake the budget cannot pay for removes the same total and
            // spends it sideways on the entry face instead of forward through
            // the hull. See `system_railgun_lance` invariant 9.
            rake_radius: Some(spec.rake_radius),
            // 18 km of reach. A spinal gun outranges every mount on the ship
            // carrying it, which is what makes lining up worth doing.
            slug_lifetime: 1.2,
            // Raw per-shot impulse, in the same register as a thruster's
            // per-tick magnitude: about two thirds of a second of the basic
            // drive's full burn, delivered in one instant.
            recoil_impulse: 45.0,
            fire_sound: Some(meshes.railgun_fire_sound.clone()),
            charge_sound: Some(meshes.railgun_charge_sound.clone()),
            reload_sound: Some(meshes.railgun_reload_sound.clone()),
            // One shell in the air per gun, ever. The magazine IS the design: a
            // lance that could queue a second shot would be a turret with a
            // long fire rate.
            ammunition: AmmoCapacity::Limited(1),
            // The tempo. Twelve quiet seconds return the shell, so a lance
            // fires roughly every thirteen and a half - and every one of those
            // is a decision rather than a trigger pull.
            reload: ReloadConfig::Batch(SectionReloadConfig {
                delay: 12.0,
                amount: 1,
            }),
        }),
    }
}

/// The lance prototype.
pub(super) fn prototypes(meshes: &BaseContentAssets) -> Vec<SectionConfig> {
    vec![railgun_lance_prototype(
        meshes,
        LanceSpec {
            id: RAILGUN_LANCE_SECTION_ID,
            name: "Railgun Lance",
            description: "A spinal kinetic lance: no traverse, so the HULL \
                              aims it. Tapping the trigger COMMITS - the bolt \
                              walks the bore and the shot leaves when it \
                              arrives, whether or not the nose is still on the \
                              target. What leaves rakes through everything in \
                              the line, and shoves the ship that fired.",
            // Dealt in FULL to every layer crossed. It clears every hull,
            // controller, turret and bay in the catalog outright - the
            // toughest of those is a reinforced hull block at 200 - so an
            // aligned shot takes a column out of a ship rather than
            // damaging one.
            //
            // It does NOT clear the two large drives: `vector_thruster_
            // section` is 480 and `capital_thruster_section` is 1250, both
            // older than this gun. A lance cripples a capital drive over
            // several shots instead of removing it, which is the intended
            // read - depth is priced in `slug_power`, lethality here - but
            // it is the number to revisit if a lance is meant to answer a
            // capital hull in one pass.
            slug_damage: 300.0,
            // Sized to pass through an entire ship. At the 3x
            // closing-speed ceiling a lance slug spends max_health/3 per
            // layer, so this crosses 27 reinforced hull blocks - past the
            // depth of anything that flies. Depth is NOT the cost of this
            // weapon; the commit, the recoil and the reload are.
            slug_power: 1800.0,
            // ONE UNIT, from the range's measurement bank. On the unit
            // lattice every hull is built on it takes the face and
            // diagonal neighbours of the cell it bores through and stops
            // short of the second ring at 1.5. The corridor is three cells
            // wide, which is a hole you can see and not a ship you delete.
            rake_radius: Meters(10.0),
        },
    )]
}

#[cfg(test)]
mod tests {
    use super::*;

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
                railgun.ammunition,
                AmmoCapacity::Limited(1),
                "`{}` can queue a second lance shot",
                section.base.id,
            );
            let reload = railgun
                .reload
                .batch()
                .unwrap_or_else(|| panic!("`{}` never gets its shell back", section.base.id));
            assert_eq!(reload.amount, 1);
            assert!(
                reload.delay > railgun.charge_seconds,
                "`{}` reloads faster than it charges, so the reload is not the tempo",
                section.base.id,
            );
        }
    }
}
