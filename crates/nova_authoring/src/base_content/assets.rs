//! Asset references consumed by built-in content builders.

use bevy::prelude::{AudioSource, Image, WorldAsset};
use nova_gameplay::prelude::AssetRef;

/// The render-mesh asset references the section catalog needs, as `AssetRef`s.
///
/// The catalog itself (`standard_section_prototypes`) is defined ONCE and is agnostic to how
/// these refs were sourced. Production no longer builds sections from `GameAssets`
/// handles - it loads the serialized catalog (`assets/base/sections/base.content.ron`)
/// via `nova_modding`; the only remaining source is the RON generator/parity test,
/// which builds them from asset PATHS (`from_paths`) so the serialized section
/// configs carry authorable paths instead of opaque handles.
pub struct BaseContentAssets {
    /// Default built-in skybox.
    pub cubemap: AssetRef<Image>,
    /// Alternate deep-field skybox used by later campaign chapters.
    pub cubemap_alt: AssetRef<Image>,
    /// Default procedural-asteroid surface texture.
    pub asteroid_texture: AssetRef<Image>,
    /// Standard hull mesh: the crew cell, a hatch on every face.
    pub hull: AssetRef<WorldAsset>,
    /// Cargo hull variant: caged freight bags, every face alike.
    pub hull_cargo: AssetRef<WorldAsset>,
    /// Tank hull variant: a pressure vessel in open frame rails.
    pub hull_tank: AssetRef<WorldAsset>,
    /// Controller core: the cable-wrapped computer cell.
    pub controller_core: AssetRef<WorldAsset>,
    /// Exposed bell body used by the basic thruster section.
    pub thruster_bell: AssetRef<WorldAsset>,
    /// Vectoring body used by the 3x3x2 drive.
    pub thruster_vector: AssetRef<WorldAsset>,
    /// Vectoring body used by the 5x5x3 capital drive.
    pub thruster_capital: AssetRef<WorldAsset>,
    /// The default PDC's parts (the gatling mount).
    pub turret_yaw: AssetRef<WorldAsset>,
    pub turret_pitch: AssetRef<WorldAsset>,
    pub turret_barrel: AssetRef<WorldAsset>,
    /// The twin PDC's parts: one barrel block, two muzzles.
    pub turret_twin_yaw: AssetRef<WorldAsset>,
    pub turret_twin_pitch: AssetRef<WorldAsset>,
    pub turret_twin_barrel: AssetRef<WorldAsset>,
    /// The stow housing both PDC mounts share: the pit the assembly sinks
    /// into, with the sliding lid nodes the `StowDoors` track drives.
    pub turret_housing: AssetRef<WorldAsset>,
    pub torpedo_bay: AssetRef<WorldAsset>,
    /// The spinal lance: one 1x1x3 body carrying the `charge_bolt` node the
    /// `Charge` track walks up the bore.
    pub railgun_lance: AssetRef<WorldAsset>,

    /// The turret fire sound, authored the same `self:/` way as the meshes.
    /// Serialized into the section config's `fire_sound` field so base turrets
    /// ship + reference their weapon sound through the scheme pipeline;
    /// `base/sounds/turret_fire.wav` resolves to the same handle the global
    /// bank loads, so the audible result is unchanged.
    pub turret_fire_sound: AssetRef<AudioSource>,
    /// The twin mount's round. The same family one size up, and the size is
    /// carried by pitch: the body sits lower and the mount rings lower than
    /// the gatling's. Authored per MOUNT, not per damage type - a kinetic and
    /// a pierce twin are the same gun firing different ammunition.
    pub turret_twin_fire_sound: AssetRef<AudioSource>,
    /// The turret dry-fire click, authored like the fire sound.
    pub turret_dry_fire_sound: AssetRef<AudioSource>,
    /// The retractable housing rising: lids parting, then the assembly up.
    pub turret_stow_open_sound: AssetRef<AudioSource>,
    /// The same housing folding away. A separate recording, not the rise
    /// played backwards - the fold is unhurried where the rise is not.
    pub turret_stow_close_sound: AssetRef<AudioSource>,
    /// The torpedo bay launch sound.
    pub torpedo_launch_sound: AssetRef<AudioSource>,
    /// The bay's muzzle iris: one servo and six petals, played on both edges
    /// of the door's travel.
    pub torpedo_door_sound: AssetRef<AudioSource>,
    /// The warhead. A hard front and a spray of fragments - deliberately not
    /// [`Self::section_destroy_sound`], which it used to borrow: a section
    /// failing is structural and a warhead is not.
    pub torpedo_detonation_sound: AssetRef<AudioSource>,
    /// The lance's discharge: the capacitor bank dumping, the slug leaving,
    /// and the hull taking the recoil, in the order the shot does them.
    pub railgun_fire_sound: AssetRef<AudioSource>,
    /// The lance's capacitor bank filling: a LOOP, played at a rate that rises
    /// with the charge, so the gun sounds like it is approaching the shot.
    pub railgun_charge_sound: AssetRef<AudioSource>,
    /// A shell going back into the lance: breech, rail, seat, lock. Written as
    /// four separable events so a pilot can hear how far through it is.
    pub railgun_reload_sound: AssetRef<AudioSource>,

    /// The controller's radar/lock/safety feedback cues.
    pub controller_lock_on_sound: AssetRef<AudioSource>,
    pub controller_lock_off_sound: AssetRef<AudioSource>,
    pub controller_radar_deny_sound: AssetRef<AudioSource>,
    pub controller_radar_retarget_sound: AssetRef<AudioSource>,
    pub controller_safety_on_sound: AssetRef<AudioSource>,
    /// The threat alarm: a hostile has this ship in its combat lock.
    pub controller_warn_lock_sound: AssetRef<AudioSource>,
    /// The magazine gauge inside the cockpit, alongside the gun's own
    /// dead-trigger click out on the mount.
    pub controller_ammo_dry_sound: AssetRef<AudioSource>,
    /// The hull alarm. Everything `warn_lock` is, an octave down and half the
    /// speed - slower is more serious, which is the opposite of how alarms
    /// usually escalate and is why this one lands.
    pub controller_warn_hull_sound: AssetRef<AudioSource>,
    /// The controller's RCS fine-adjust loop: plays while the RCS primitive
    /// burns, player- or autopilot-driven.
    pub controller_rcs_loop_sound: AssetRef<AudioSource>,
    /// Per-target hit/destruction voices, shared by every catalog section.
    /// HULL only: asteroids used to borrow this pair and now author
    /// `impact_rock` / `destroy_rock` at their own sites in scenario content,
    /// which is as far as the target-side half of "what hit what" goes without
    /// a material table. The round-side half (pierce, explosive) still has
    /// nowhere to be authored - see the task's material-table section.
    pub section_impact_sound: AssetRef<AudioSource>,
    pub section_destroy_sound: AssetRef<AudioSource>,
    /// The whole SHIP coming apart, which is a different event from the last
    /// section dying: structural collapse fires once on the root and the peel
    /// that follows runs for several frames under it. Authored on the hull, not
    /// on a section, because no section owns it.
    pub ship_collapse_sound: AssetRef<AudioSource>,
    /// The thruster engine hums, one per drive size. Three loops on one
    /// recipe at 34 / 52 / 78 Hz, capital to basic to vector: a pilot should
    /// hear the SIZE of what just lit its engines, and pitch is the only thing
    /// separating them.
    pub thruster_loop_sound: AssetRef<AudioSource>,
    pub thruster_vector_loop_sound: AssetRef<AudioSource>,
    pub thruster_capital_loop_sound: AssetRef<AudioSource>,
    /// The decoration models a skin style scatters, generated from the recipes
    /// in `scripts/greeble-recipes/`. The `greeble_*` four with no kit in their
    /// name are the scaffolding style's; the rest are the four authored kits.
    pub greeble_blister: AssetRef<WorldAsset>,
    pub greeble_block: AssetRef<WorldAsset>,
    pub greeble_mast: AssetRef<WorldAsset>,
    pub greeble_vent: AssetRef<WorldAsset>,
    /// The ARMOURED kit: a belt down the straight edges, a boss on the outer
    /// corners, a flush hatch on the flat panels, the one blister that breaks
    /// the plane, and the vocabulary batch (task 20260816-222644): a stub
    /// mast, a shuttered intake, a ready magazine, a chaff tube, an applique
    /// tile grid and the white rounds-count tally beside the gun wells.
    pub greeble_armoured_ammo_stripes: AssetRef<WorldAsset>,
    pub greeble_armoured_applique: AssetRef<WorldAsset>,
    pub greeble_armoured_cap: AssetRef<WorldAsset>,
    pub greeble_armoured_chaff: AssetRef<WorldAsset>,
    pub greeble_armoured_hatch: AssetRef<WorldAsset>,
    pub greeble_armoured_intake: AssetRef<WorldAsset>,
    pub greeble_armoured_magazine: AssetRef<WorldAsset>,
    pub greeble_armoured_mast: AssetRef<WorldAsset>,
    pub greeble_armoured_sensor: AssetRef<WorldAsset>,
    pub greeble_armoured_strake: AssetRef<WorldAsset>,
    /// The CIVILIAN kit, which the `civilian` style scatters: a livery rail, a
    /// cabin window row and skylight strip, a raked fin, a fairing, a faired
    /// intake, door, tank blister and comms dish, an advert panel, a registry
    /// mark and a nav beacon. Everything faired or painted - machinery never
    /// shows on a hull built to be sold.
    pub greeble_civilian_beacon: AssetRef<WorldAsset>,
    pub greeble_civilian_dish: AssetRef<WorldAsset>,
    pub greeble_civilian_door: AssetRef<WorldAsset>,
    pub greeble_civilian_fairing: AssetRef<WorldAsset>,
    pub greeble_civilian_fin: AssetRef<WorldAsset>,
    pub greeble_civilian_livery: AssetRef<WorldAsset>,
    pub greeble_civilian_registry: AssetRef<WorldAsset>,
    pub greeble_civilian_skylight: AssetRef<WorldAsset>,
    pub greeble_civilian_stripe: AssetRef<WorldAsset>,
    pub greeble_civilian_tank: AssetRef<WorldAsset>,
    pub greeble_civilian_vent: AssetRef<WorldAsset>,
    pub greeble_civilian_windows: AssetRef<WorldAsset>,
    /// The INDUSTRIAL kit: exposed services, ribbing, radiators and paint. Seven
    /// pieces, one per rule of the `industrial` style.
    pub greeble_industrial_duct: AssetRef<WorldAsset>,
    pub greeble_industrial_hatch: AssetRef<WorldAsset>,
    pub greeble_industrial_hazard_band: AssetRef<WorldAsset>,
    pub greeble_industrial_louvre: AssetRef<WorldAsset>,
    pub greeble_industrial_radiator: AssetRef<WorldAsset>,
    pub greeble_industrial_ribbing: AssetRef<WorldAsset>,
    pub greeble_industrial_stack: AssetRef<WorldAsset>,
    /// The SALVAGE kit: mismatched patches, a hand-run weld bead, lashed
    /// tankage and rigging, scavenged fittings off other ships, a kinked whip
    /// and a tow cleat. Fourteen pieces, and the doctrine holds - a hull reads
    /// as repaired because of where the pieces land and what they are made of,
    /// never because there are more of them.
    pub greeble_salvage_chain: AssetRef<WorldAsset>,
    pub greeble_salvage_cog_patch: AssetRef<WorldAsset>,
    pub greeble_salvage_dish: AssetRef<WorldAsset>,
    pub greeble_salvage_drum: AssetRef<WorldAsset>,
    pub greeble_salvage_grille: AssetRef<WorldAsset>,
    pub greeble_salvage_hook: AssetRef<WorldAsset>,
    pub greeble_salvage_hose: AssetRef<WorldAsset>,
    pub greeble_salvage_kills: AssetRef<WorldAsset>,
    pub greeble_salvage_net: AssetRef<WorldAsset>,
    pub greeble_salvage_patch_plate: AssetRef<WorldAsset>,
    pub greeble_salvage_patch_scab: AssetRef<WorldAsset>,
    pub greeble_salvage_patch_strip: AssetRef<WorldAsset>,
    pub greeble_salvage_weld_seam: AssetRef<WorldAsset>,
    pub greeble_salvage_whip: AssetRef<WorldAsset>,
}

impl BaseContentAssets {
    /// Generation source: the same asset paths `GameAssets` loads them from, so
    /// the serialized section configs carry authorable paths.
    pub fn from_paths() -> Self {
        Self {
            cubemap: AssetRef::from("self://textures/cubemap.png".to_string()),
            cubemap_alt: AssetRef::from("self://textures/cubemap_alt.png".to_string()),
            asteroid_texture: AssetRef::from("self://textures/asteroid.png".to_string()),
            hull: AssetRef::from("self://gltf/hull_personnel.glb#Scene0".to_string()),
            hull_cargo: AssetRef::from("self://gltf/hull_cargo.glb#Scene0".to_string()),
            hull_tank: AssetRef::from("self://gltf/hull_tank.glb#Scene0".to_string()),
            controller_core: AssetRef::from("self://gltf/core_wires.glb#Scene0".to_string()),
            thruster_bell: AssetRef::from("self://gltf/shell_bell.glb#Scene0".to_string()),
            thruster_vector: AssetRef::from("self://gltf/shell_vector.glb#Scene0".to_string()),
            thruster_capital: AssetRef::from("self://gltf/shell_capital.glb#Scene0".to_string()),
            turret_yaw: AssetRef::from("self://gltf/pdc_gatling_yaw.glb#Scene0".to_string()),
            turret_pitch: AssetRef::from("self://gltf/pdc_gatling_pitch.glb#Scene0".to_string()),
            turret_barrel: AssetRef::from("self://gltf/pdc_gatling_barrel.glb#Scene0".to_string()),
            turret_twin_yaw: AssetRef::from("self://gltf/pdc_twin_yaw.glb#Scene0".to_string()),
            turret_twin_pitch: AssetRef::from("self://gltf/pdc_twin_pitch.glb#Scene0".to_string()),
            turret_twin_barrel: AssetRef::from(
                "self://gltf/pdc_twin_barrel.glb#Scene0".to_string(),
            ),
            turret_housing: AssetRef::from("self://gltf/pdc_housing.glb#Scene0".to_string()),
            torpedo_bay: AssetRef::from("self://gltf/bay_tube.glb#Scene0".to_string()),
            railgun_lance: AssetRef::from("self://gltf/railgun_lance.glb#Scene0".to_string()),

            turret_fire_sound: AssetRef::from("self://sounds/turret_fire.wav".to_string()),
            turret_twin_fire_sound: AssetRef::from("self://sounds/pdc_twin_fire.wav".to_string()),
            turret_dry_fire_sound: AssetRef::from("self://sounds/dry_fire.wav".to_string()),
            turret_stow_open_sound: AssetRef::from("self://sounds/pdc_stow_open.wav".to_string()),
            turret_stow_close_sound: AssetRef::from("self://sounds/pdc_stow_close.wav".to_string()),
            torpedo_launch_sound: AssetRef::from("self://sounds/torpedo_launch.wav".to_string()),
            torpedo_door_sound: AssetRef::from("self://sounds/bay_door.wav".to_string()),
            torpedo_detonation_sound: AssetRef::from(
                "self://sounds/torpedo_detonate.wav".to_string(),
            ),
            railgun_fire_sound: AssetRef::from("self://sounds/railgun_fire.wav".to_string()),
            railgun_charge_sound: AssetRef::from("self://sounds/railgun_charge.wav".to_string()),
            railgun_reload_sound: AssetRef::from("self://sounds/railgun_reload.wav".to_string()),

            controller_lock_on_sound: AssetRef::from("self://sounds/lock_on.wav".to_string()),
            controller_lock_off_sound: AssetRef::from("self://sounds/lock_off.wav".to_string()),
            controller_radar_deny_sound: AssetRef::from("self://sounds/radar_deny.wav".to_string()),
            controller_radar_retarget_sound: AssetRef::from(
                "self://sounds/radar_retarget.wav".to_string(),
            ),
            controller_safety_on_sound: AssetRef::from("self://sounds/safety_on.wav".to_string()),
            controller_warn_lock_sound: AssetRef::from("self://sounds/warn_lock.wav".to_string()),
            controller_ammo_dry_sound: AssetRef::from("self://sounds/ammo_dry.wav".to_string()),
            controller_warn_hull_sound: AssetRef::from("self://sounds/warn_hull.wav".to_string()),
            controller_rcs_loop_sound: AssetRef::from("self://sounds/rcs_loop.wav".to_string()),
            section_impact_sound: AssetRef::from("self://sounds/impact.wav".to_string()),
            section_destroy_sound: AssetRef::from("self://sounds/explosion.wav".to_string()),
            ship_collapse_sound: AssetRef::from("self://sounds/destroy_ship.wav".to_string()),
            thruster_loop_sound: AssetRef::from("self://sounds/thruster_loop.wav".to_string()),
            thruster_vector_loop_sound: AssetRef::from(
                "self://sounds/thruster_vector_loop.wav".to_string(),
            ),
            thruster_capital_loop_sound: AssetRef::from(
                "self://sounds/thruster_capital_loop.wav".to_string(),
            ),
            greeble_blister: AssetRef::from(
                "self://gltf/greebles/placeholder_blister.glb#Scene0".to_string(),
            ),
            greeble_block: AssetRef::from(
                "self://gltf/greebles/placeholder_block.glb#Scene0".to_string(),
            ),
            greeble_mast: AssetRef::from(
                "self://gltf/greebles/placeholder_mast.glb#Scene0".to_string(),
            ),
            greeble_vent: AssetRef::from(
                "self://gltf/greebles/placeholder_vent.glb#Scene0".to_string(),
            ),
            greeble_armoured_ammo_stripes: AssetRef::from(
                "self://gltf/greebles/armoured_ammo_stripes.glb#Scene0".to_string(),
            ),
            greeble_armoured_applique: AssetRef::from(
                "self://gltf/greebles/armoured_applique.glb#Scene0".to_string(),
            ),
            greeble_armoured_cap: AssetRef::from(
                "self://gltf/greebles/armoured_cap.glb#Scene0".to_string(),
            ),
            greeble_armoured_chaff: AssetRef::from(
                "self://gltf/greebles/armoured_chaff.glb#Scene0".to_string(),
            ),
            greeble_armoured_hatch: AssetRef::from(
                "self://gltf/greebles/armoured_hatch.glb#Scene0".to_string(),
            ),
            greeble_armoured_intake: AssetRef::from(
                "self://gltf/greebles/armoured_intake.glb#Scene0".to_string(),
            ),
            greeble_armoured_magazine: AssetRef::from(
                "self://gltf/greebles/armoured_magazine.glb#Scene0".to_string(),
            ),
            greeble_armoured_mast: AssetRef::from(
                "self://gltf/greebles/armoured_mast.glb#Scene0".to_string(),
            ),
            greeble_armoured_sensor: AssetRef::from(
                "self://gltf/greebles/armoured_sensor.glb#Scene0".to_string(),
            ),
            greeble_armoured_strake: AssetRef::from(
                "self://gltf/greebles/armoured_strake.glb#Scene0".to_string(),
            ),
            greeble_civilian_beacon: AssetRef::from(
                "self://gltf/greebles/civilian_beacon.glb#Scene0".to_string(),
            ),
            greeble_civilian_dish: AssetRef::from(
                "self://gltf/greebles/civilian_dish.glb#Scene0".to_string(),
            ),
            greeble_civilian_door: AssetRef::from(
                "self://gltf/greebles/civilian_door.glb#Scene0".to_string(),
            ),
            greeble_civilian_fairing: AssetRef::from(
                "self://gltf/greebles/civilian_fairing.glb#Scene0".to_string(),
            ),
            greeble_civilian_fin: AssetRef::from(
                "self://gltf/greebles/civilian_fin.glb#Scene0".to_string(),
            ),
            greeble_civilian_livery: AssetRef::from(
                "self://gltf/greebles/civilian_livery.glb#Scene0".to_string(),
            ),
            greeble_civilian_registry: AssetRef::from(
                "self://gltf/greebles/civilian_registry.glb#Scene0".to_string(),
            ),
            greeble_civilian_skylight: AssetRef::from(
                "self://gltf/greebles/civilian_skylight.glb#Scene0".to_string(),
            ),
            greeble_civilian_stripe: AssetRef::from(
                "self://gltf/greebles/civilian_stripe.glb#Scene0".to_string(),
            ),
            greeble_civilian_tank: AssetRef::from(
                "self://gltf/greebles/civilian_tank.glb#Scene0".to_string(),
            ),
            greeble_civilian_vent: AssetRef::from(
                "self://gltf/greebles/civilian_vent.glb#Scene0".to_string(),
            ),
            greeble_civilian_windows: AssetRef::from(
                "self://gltf/greebles/civilian_windows.glb#Scene0".to_string(),
            ),
            greeble_industrial_duct: AssetRef::from(
                "self://gltf/greebles/industrial_duct.glb#Scene0".to_string(),
            ),
            greeble_industrial_hatch: AssetRef::from(
                "self://gltf/greebles/industrial_hatch.glb#Scene0".to_string(),
            ),
            greeble_industrial_hazard_band: AssetRef::from(
                "self://gltf/greebles/industrial_hazard_band.glb#Scene0".to_string(),
            ),
            greeble_industrial_louvre: AssetRef::from(
                "self://gltf/greebles/industrial_louvre.glb#Scene0".to_string(),
            ),
            greeble_industrial_radiator: AssetRef::from(
                "self://gltf/greebles/industrial_radiator.glb#Scene0".to_string(),
            ),
            greeble_industrial_ribbing: AssetRef::from(
                "self://gltf/greebles/industrial_ribbing.glb#Scene0".to_string(),
            ),
            greeble_industrial_stack: AssetRef::from(
                "self://gltf/greebles/industrial_stack.glb#Scene0".to_string(),
            ),
            greeble_salvage_chain: AssetRef::from(
                "self://gltf/greebles/salvage_chain.glb#Scene0".to_string(),
            ),
            greeble_salvage_cog_patch: AssetRef::from(
                "self://gltf/greebles/salvage_cog_patch.glb#Scene0".to_string(),
            ),
            greeble_salvage_dish: AssetRef::from(
                "self://gltf/greebles/salvage_dish.glb#Scene0".to_string(),
            ),
            greeble_salvage_drum: AssetRef::from(
                "self://gltf/greebles/salvage_drum.glb#Scene0".to_string(),
            ),
            greeble_salvage_grille: AssetRef::from(
                "self://gltf/greebles/salvage_grille.glb#Scene0".to_string(),
            ),
            greeble_salvage_hook: AssetRef::from(
                "self://gltf/greebles/salvage_hook.glb#Scene0".to_string(),
            ),
            greeble_salvage_hose: AssetRef::from(
                "self://gltf/greebles/salvage_hose.glb#Scene0".to_string(),
            ),
            greeble_salvage_kills: AssetRef::from(
                "self://gltf/greebles/salvage_kills.glb#Scene0".to_string(),
            ),
            greeble_salvage_net: AssetRef::from(
                "self://gltf/greebles/salvage_net.glb#Scene0".to_string(),
            ),
            greeble_salvage_patch_plate: AssetRef::from(
                "self://gltf/greebles/salvage_patch_plate.glb#Scene0".to_string(),
            ),
            greeble_salvage_patch_scab: AssetRef::from(
                "self://gltf/greebles/salvage_patch_scab.glb#Scene0".to_string(),
            ),
            greeble_salvage_patch_strip: AssetRef::from(
                "self://gltf/greebles/salvage_patch_strip.glb#Scene0".to_string(),
            ),
            greeble_salvage_weld_seam: AssetRef::from(
                "self://gltf/greebles/salvage_weld_seam.glb#Scene0".to_string(),
            ),
            greeble_salvage_whip: AssetRef::from(
                "self://gltf/greebles/salvage_whip.glb#Scene0".to_string(),
            ),
        }
    }
}
