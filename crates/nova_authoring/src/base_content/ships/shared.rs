//! Semantic Kenney ship parts and the shipped Racer, CargoB, and CargoA assemblies.
//!
//! Part meshes are centered on tight primitive colliders. Structural edges come only from
//! authored link-point mates shared by the catalog prototypes and ship builders.

use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;
use nova_ship::prelude::*;

use crate::base_content::{
    assets::BaseContentAssets,
    sections::{ordnance, PDC_MOUNT_OFFSET},
};

#[derive(Clone, Copy, PartialEq)]
pub(super) enum ShipGrade {
    Player,
    Enemy,
}

#[derive(Clone, Copy)]
pub(super) enum PartRole {
    Hull,
    Thruster,
    Controller,
    Torpedo,
    Turret,
}

#[derive(Clone, Copy)]
pub(super) struct PartSpec {
    id: &'static str,
    prototype: &'static str,
    mesh: Option<&'static str>,
    pub(super) origin: Vec3,
    pub(super) bbox_min: Vec3,
    pub(super) bbox_max: Vec3,
    health: f32,
    role: PartRole,
}

impl PartSpec {
    pub(super) fn center(self) -> Vec3 {
        self.origin + (self.bbox_min + self.bbox_max) * 0.5
    }

    fn size(self) -> Vec3 {
        self.bbox_max - self.bbox_min
    }

    pub(super) fn mesh_offset(self) -> Vec3 {
        self.origin - self.center()
    }
}

/// The outward cardinal axis a turret module bolts along: the direction from
/// the part it mates to, toward the module's own centre.
///
/// This is the ONE number a mount's placement needs. The shared PDC always
/// bolts down by its local -Y, so the axis fixes both the rotation (which face
/// of the host the gun stands on) and the socket the host must offer. It used
/// to be a hand-authored `PartSide` - a flat quarter turn about Z that happened
/// to be right for the cargoa's nose cheeks and wrong for the cargob's pod
/// shoulders, where the gun actually stands on a TOP face.
fn turret_mount_axis(specs: &[PartSpec], edges: &[(usize, usize)], index: usize) -> Vec3 {
    let center = specs[index].center();
    edges
        .iter()
        .find_map(|&(a, b)| {
            let other = if a == index {
                b
            } else if b == index {
                a
            } else {
                return None;
            };
            Some(cardinal_axis(center - specs[other].center()))
        })
        .expect("a turret module mates to exactly one host part")
}

/// Where a part sits and how it is turned, in ship-root space.
///
/// Everything but a turret is authored axis-aligned at its own cut centre. A
/// turret is the one part whose pose is DERIVED: it stands on a face, so its
/// rotation is whatever puts its base plate against that face.
fn placement(specs: &[PartSpec], edges: &[(usize, usize)], index: usize) -> (Vec3, Quat) {
    let spec = specs[index];
    if !matches!(spec.role, PartRole::Turret) {
        return (spec.center(), Quat::IDENTITY);
    }
    let axis = turret_mount_axis(specs, edges, index);
    (spec.center(), Quat::from_rotation_arc(Vec3::Y, axis))
}

pub(super) const fn v(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

pub(super) const fn part(
    id: &'static str,
    prototype: &'static str,
    mesh: &'static str,
    origin: Vec3,
    bbox_min: Vec3,
    bbox_max: Vec3,
    health: f32,
    role: PartRole,
) -> PartSpec {
    PartSpec {
        id,
        prototype,
        mesh: Some(mesh),
        origin,
        bbox_min,
        bbox_max,
        health,
        role,
    }
}

/// A part with no art and no catalog entry of its own: a mount POINT on the
/// craft, filled by a shared prototype.
///
/// `prototype` is the catalog id the assembly bolts there, and the box is the
/// shared PDC's, because that is the only thing a module is now.
pub(super) const fn module(id: &'static str, center: Vec3, role: PartRole) -> PartSpec {
    PartSpec {
        id,
        prototype: PDC_KINETIC_TURRET_SECTION_ID,
        mesh: None,
        origin: center,
        bbox_min: v(-PDC_MOUNT_OFFSET, -PDC_MOUNT_OFFSET, -PDC_MOUNT_OFFSET),
        bbox_max: v(PDC_MOUNT_OFFSET, PDC_MOUNT_OFFSET, PDC_MOUNT_OFFSET),
        health: 0.0,
        role,
    }
}

fn mesh_ref(file: &str) -> AssetRef<WorldAsset> {
    AssetRef::from(format!("self://gltf/parts/{file}#Scene0"))
}

fn render_transform(spec: PartSpec) -> Option<RenderMeshTransform> {
    Some(RenderMeshTransform {
        position: spec.mesh_offset(),
        ..default()
    })
}

/// The sockets one authored adjacency gives a part: one per edge it takes part
/// in.
///
/// The normal is SNAPPED to a cardinal axis rather than left pointing at the
/// neighbour's centre. Cut parts sit wherever the craft's art put them, so a
/// raw centre-to-centre direction is oblique - the cargob's pod faces its
/// fuselage 36 degrees off -X - and anything mated onto that socket arrived
/// tilted by exactly that much, which is what made parts look like they only
/// fit the craft they were cut from. The snap is antisymmetric (see
/// [`cardinal_axis`]), so both ends of an edge stay exactly opposed.
///
/// Two parts of comparable size meet at the MIDPOINT between their centres,
/// which is their shared face. A turret does not: it is a small mount bolted
/// onto a big hull part, so the midpoint floats in open space well clear of the
/// gun's base plate. The socket offered to a turret therefore sits exactly
/// where that base plate lands - [`PDC_MOUNT_OFFSET`] back from the mount's own
/// centre - which is what lets one shared PDC bolt onto a craft whose parts
/// were cut at whatever size the art happened to be.
pub(super) fn link_points(
    specs: &[PartSpec],
    edges: &[(usize, usize)],
    index: usize,
) -> Vec<LinkPoint> {
    let center = specs[index].center();
    edges
        .iter()
        .filter_map(|&(a, b)| {
            let other = if a == index {
                b
            } else if b == index {
                a
            } else {
                return None;
            };
            let other_center = specs[other].center();
            // Snapped in SHIP space, not in the part's: both ends then read the
            // same axis off the same direction.
            let direction = cardinal_axis(other_center - center);
            let position = if matches!(specs[other].role, PartRole::Turret) {
                other_center - direction * PDC_MOUNT_OFFSET
            } else {
                (center + other_center) * 0.5
            };
            Some(LinkPoint {
                id: format!("to_{}", specs[other].id),
                position: position - center,
                normal: direction,
            })
        })
        .collect()
}

/// Title-case one snake_case part id: `pod_starboard` -> `Pod Starboard`.
fn title_case(id: &str) -> String {
    id.split('_')
        .map(|word| {
            let mut chars = word.chars();
            chars
                .next()
                .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn base_config(
    spec: PartSpec,
    family: &str,
    links: Vec<LinkPoint>,
    meshes: &BaseContentAssets,
) -> BaseSectionConfig {
    BaseSectionConfig {
        id: spec.prototype.to_string(),
        // Named for the craft it was cut off, not for its role alone: three
        // craft each contribute a "Nose" and a "Tail", and a bare role name
        // leaves a browser no way to tell them apart or to find the rest of a
        // set.
        name: format!("{family} // {}", title_case(spec.id)),
        description: format!("The {family}'s {} section.", spec.id.replace('_', " ")),
        health: spec.health,
        impact_sound: Some(meshes.section_impact_sound.clone()),
        destroy_sound: Some(meshes.section_destroy_sound.clone()),
        collider: Some(SectionCollider::Cuboid { size: spec.size() }),
        link_points: links,
        // Placeable now that editor placement MATES link points: a semantic
        // part only goes where its authored sockets say it does, which is what
        // hiding it was waiting for.
        hide_in_editor: false,
        damage_effects: role_damage_effects(spec.role),
        // Cut-cube art has no named animation nodes - the pod bays launch
        // through their cut hulls, not through the tube's iris.
        animations: Vec::new(),
    }
}

/// The damage looks a cut-cube part wears, by what the part IS.
///
/// Derived from the role rather than authored per part because these are cut
/// from whole craft by the hundred - the racer alone contributes dozens of hull
/// tiles - and every one of them would otherwise repeat the same list. A part
/// that wants something else says so by not coming through here.
fn role_damage_effects(role: PartRole) -> DamageEffects {
    match role {
        // Hull is material and nothing else, so it wears its damage in the
        // SURFACE rather than in the silhouette: it cracks, and the cladding
        // over it comes off a plate at a time.
        PartRole::Hull => DamageEffects(vec![DamageEffect::Cracks]),
        // Machinery whose shape has to survive to keep working: it sparks.
        // A turret never reaches here - it is a mount point, not a prototype -
        // but the arm keeps the role list exhaustive.
        PartRole::Controller | PartRole::Torpedo | PartRole::Turret => {
            DamageEffects(vec![DamageEffect::Cracks, DamageEffect::Sparks])
        }
        // A drive adds its plume guttering to that.
        PartRole::Thruster => DamageEffects(vec![
            DamageEffect::Cracks,
            DamageEffect::Sparks,
            DamageEffect::Plume,
        ]),
    }
}

fn hull_kind(spec: PartSpec) -> SectionKind {
    SectionKind::Hull(HullSectionConfig {
        render_mesh: spec.mesh.map(mesh_ref),
        render_mesh_transform: render_transform(spec),
    })
}

fn thruster_kind(spec: PartSpec, meshes: &BaseContentAssets) -> SectionKind {
    SectionKind::Thruster(ThrusterSectionConfig {
        magnitude: 1.0,
        render_mesh: spec.mesh.map(mesh_ref),
        render_mesh_transform: render_transform(spec),
        loop_sound: Some(meshes.thruster_loop_sound.clone()),
        exhaust: Some(ThrusterExhaust {
            offset: Vec3::new(0.0, 0.0, spec.size().z * 0.5),
            rotation: Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
            shape: ThrusterExhaustConfig {
                geometry: ThrusterExhaustShape::Rect,
                width: spec.size().x * 0.7,
                height: spec.size().y * 0.6,
                ..default()
            },
        }),
    })
}

fn controller_kind(spec: PartSpec, meshes: &BaseContentAssets) -> SectionKind {
    SectionKind::Controller(ControllerSectionConfig {
        steering_lag: 0.5,
        // The catalog controller's torque; see `sections::standard`.
        max_torque: 1501.0,
        render_mesh: spec.mesh.map(mesh_ref),
        render_mesh_transform: render_transform(spec),
        lock_on_sound: Some(meshes.controller_lock_on_sound.clone()),
        lock_off_sound: Some(meshes.controller_lock_off_sound.clone()),
        radar_deny_sound: Some(meshes.controller_radar_deny_sound.clone()),
        radar_retarget_sound: Some(meshes.controller_radar_retarget_sound.clone()),
        safety_on_sound: Some(meshes.controller_safety_on_sound.clone()),
        warn_lock_sound: Some(meshes.controller_warn_lock_sound.clone()),
        ammo_dry_sound: Some(meshes.controller_ammo_dry_sound.clone()),
        warn_hull_sound: Some(meshes.controller_warn_hull_sound.clone()),
        warn_hull_fraction: DEFAULT_WARN_HULL_FRACTION,
        rcs_loop_sound: Some(meshes.controller_rcs_loop_sound.clone()),
    })
}

fn torpedo_kind(
    spec: PartSpec,
    meshes: &BaseContentAssets,
    torpedo_type: TorpedoTypeConfig,
) -> SectionKind {
    SectionKind::Torpedo(TorpedoSectionConfig {
        render_mesh: spec.mesh.map(mesh_ref),
        render_mesh_transform: render_transform(spec),
        projectile_render_mesh: None,
        spawn_offset: Vec3::new(0.0, 0.0, -spec.size().z * 0.5 - 0.5),
        // Aim the tube out of the open face. The launch axis is
        // the spawner's +Y and this turns it onto the section's -Z,
        // the one face `link_points` leaves unlinkable so it can be
        // a muzzle. Without it the tube ejected through its own roof.
        spawn_rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
        // The cut-cube pods author no muzzle door, so nothing would reveal a
        // recessed birth - they launch at their muzzle as before.
        spawn_recess: 0.0,
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
        // The standard torpedo's damage (see `sections::standard`): a hit can
        // decide a small-craft fight, while structural depth attenuates it on a
        // capital. The cargob's tubes hit like the catalog's.
        blast_damage: 750.0,
        blast_effect: None,
        launch_effect: None,
        launch_sound: Some(meshes.torpedo_launch_sound.clone()),
        // The cut-cube pods author no muzzle door (see `spawn_recess` above),
        // so this is authored-and-inert on them: nothing reports, nothing
        // plays. It is here so a modelled pod that DOES grow an iris inherits
        // the servo rather than being silently the one bay without one.
        door_sound: Some(meshes.torpedo_door_sound.clone()),
        detonation_sound: Some(meshes.torpedo_detonation_sound.clone()),
        // Matches the catalog bay: above the hardest single PDC round, so an
        // intercept costs two or three rounds rather than one.
        projectile_health: 10.0,
        // What the pod LOADS, and the only thing the `_lance` variant of this
        // prototype changes (see `sections::ordnance`).
        torpedo_type,
        // The catalog bay's idle reload: six for the alpha strike, then one
        // after each quiet 10 s. Two pods sustain 0.2 torpedoes/s, below the
        // 0.216/s that two player-grade PDCs can answer.
        ammo_capacity: Some(6),
        reload: Some(SectionReloadConfig {
            delay: 10.0,
            amount: 1,
        }),
    })
}

/// Every catalog prototype one craft's parts contribute, named for `family`.
///
/// A turret module contributes NOTHING here. It carries no mesh of its own, so
/// the ten per-craft copies this used to emit - port and starboard across three
/// craft, doubled again for the scavenger grade - were the same PDC on the same
/// joint tree, and offering ten identical turrets buried the two that actually
/// differ. A module is now only a mount POINT: the craft names where the gun
/// goes, and the shared PDC is the gun.
///
/// A torpedo pod IS doubled, into a `_lance` variant loading the
/// straight-running type. Everything else about the pod - the art, the tube,
/// the warhead, the rack - is shared, so [`Ordnance`] picks which prototype a
/// ship references and nothing else about the ship moves.
pub(super) fn prototypes(
    specs: &[PartSpec],
    edges: &[(usize, usize)],
    family: &str,
    meshes: &BaseContentAssets,
) -> Vec<SectionConfig> {
    let mut output = Vec::new();
    for (index, &spec) in specs.iter().enumerate() {
        let links = link_points(specs, edges, index);
        let kind = match spec.role {
            PartRole::Hull => hull_kind(spec),
            PartRole::Thruster => thruster_kind(spec, meshes),
            PartRole::Controller => controller_kind(spec, meshes),
            PartRole::Torpedo => torpedo_kind(spec, meshes, ordnance::serpent()),
            // A mount point, not a part: the shared PDC fills it.
            PartRole::Turret => continue,
        };
        output.push(SectionConfig {
            base: base_config(spec, family, links.clone(), meshes),
            kind,
        });
        if matches!(spec.role, PartRole::Torpedo) {
            let mut base = base_config(spec, family, links, meshes);
            base.id = format!("{}{}", spec.prototype, Ordnance::Lance.prototype_suffix());
            base.name = format!("{} (Lance)", base.name);
            base.description = format!(
                "{} Loaded with straight-running Lance torpedoes.",
                base.description
            );
            output.push(SectionConfig {
                base,
                kind: torpedo_kind(spec, meshes, ordnance::lance()),
            });
        }
    }
    output
}

/// Which torpedo type a craft's pods are built with.
///
/// Build-time, exactly like [`ShipGrade`], and for the same reason the ship
/// catalog gives the raider corvette its own entry: which torpedo a hull
/// carries is a different ship to fight, not a flag a spawn flips. It is NOT a
/// grade - the two types are equals that trade evasion against what an
/// intercept costs the defender (see `sections::ordnance`).
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Ordnance {
    /// Straight-running bombardment torpedoes: what point defense can answer.
    Lance,
    /// Weaving assault torpedoes: the default, and the escalation.
    Serpent,
}

impl Ordnance {
    /// The prototype-id suffix this ordnance is authored under.
    fn prototype_suffix(self) -> &'static str {
        match self {
            Ordnance::Lance => "_lance",
            Ordnance::Serpent => "",
        }
    }
}

/// The scavenger grade's mount health, the one thing left of the old
/// `_light` turret prototype.
///
/// That prototype was a whole second gun - a quarter of the fire rate, slower
/// rounds, less per hit. There is one turret in the catalog now, so a raider's
/// guns are no longer softer than a player's; they are only easier to SHOOT
/// OFF, which is the same knob every other section on an enemy hull already
/// used.
const ENEMY_TURRET_HEALTH: f32 = 60.0;

pub(super) fn ship_sections(
    specs: &[PartSpec],
    edges: &[(usize, usize)],
    grade: ShipGrade,
    ordnance: Ordnance,
) -> Vec<SpaceshipSectionConfig> {
    specs
        .iter()
        .enumerate()
        .map(|(index, &spec)| {
            let mut modifications = Vec::new();
            let mut prototype = spec.prototype.to_string();
            if matches!(spec.role, PartRole::Torpedo) {
                prototype.push_str(ordnance.prototype_suffix());
            }
            if grade == ShipGrade::Enemy {
                match spec.role {
                    PartRole::Hull => modifications.push(SectionModification::SetHealth(
                        (spec.health * 0.58).max(35.0),
                    )),
                    PartRole::Thruster => {
                        modifications.push(SectionModification::SetHealth(25.0));
                    }
                    PartRole::Controller => {
                        modifications.push(SectionModification::SetHealth(140.0));
                    }
                    PartRole::Turret => {
                        modifications.push(SectionModification::SetHealth(ENEMY_TURRET_HEALTH));
                    }
                    // Ordnance is not graded: which torpedo a hull carries is
                    // its own catalog entry above, not this hull's tier.
                    PartRole::Torpedo => {}
                }
            }
            let (position, rotation) = placement(specs, edges, index);
            SpaceshipSectionConfig {
                id: spec.id.to_string(),
                position,
                rotation,
                source: SectionSource::Prototype(prototype),
                modifications,
            }
        })
        .collect()
}
