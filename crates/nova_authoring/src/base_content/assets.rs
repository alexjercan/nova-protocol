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
    /// Standard hull mesh.
    pub hull: AssetRef<WorldAsset>,
    /// Exposed bell body used by the basic thruster section.
    pub thruster_bell: AssetRef<WorldAsset>,
    /// Vectoring body used by the 3x3x2 drive.
    pub thruster_vector: AssetRef<WorldAsset>,
    /// Vectoring body used by the 5x5x3 capital drive.
    pub thruster_capital: AssetRef<WorldAsset>,
    pub turret_yaw: AssetRef<WorldAsset>,
    pub turret_pitch: AssetRef<WorldAsset>,
    pub turret_barrel: AssetRef<WorldAsset>,
    pub torpedo_bay: AssetRef<WorldAsset>,
    /// The turret fire sound, authored the same `self:/` way as the meshes.
    /// Serialized into the section config's `fire_sound` field so base turrets
    /// ship + reference their weapon sound through the scheme pipeline;
    /// `base/sounds/turret_fire.wav` resolves to the same handle the global
    /// bank loads, so the audible result is unchanged.
    pub turret_fire_sound: AssetRef<AudioSource>,
    /// The turret dry-fire click, authored like the fire sound.
    pub turret_dry_fire_sound: AssetRef<AudioSource>,
    /// The torpedo bay launch sound.
    pub torpedo_launch_sound: AssetRef<AudioSource>,
    /// The controller's radar/lock/safety feedback cues.
    pub controller_lock_on_sound: AssetRef<AudioSource>,
    pub controller_lock_off_sound: AssetRef<AudioSource>,
    pub controller_radar_deny_sound: AssetRef<AudioSource>,
    pub controller_radar_retarget_sound: AssetRef<AudioSource>,
    pub controller_safety_on_sound: AssetRef<AudioSource>,
    /// The controller's RCS fine-adjust loop: plays while the RCS primitive
    /// burns, player- or autopilot-driven.
    pub controller_rcs_loop_sound: AssetRef<AudioSource>,
    /// Per-target hit/destruction voices, shared by every catalog section;
    /// asteroids author the same two in scenario content.
    pub section_impact_sound: AssetRef<AudioSource>,
    pub section_destroy_sound: AssetRef<AudioSource>,
    /// The thruster engine hum.
    pub thruster_loop_sound: AssetRef<AudioSource>,
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
            hull: AssetRef::from("self://gltf/hull-01.glb#Scene0".to_string()),
            thruster_bell: AssetRef::from("self://gltf/shell_bell.glb#Scene0".to_string()),
            thruster_vector: AssetRef::from("self://gltf/shell_vector.glb#Scene0".to_string()),
            thruster_capital: AssetRef::from("self://gltf/shell_capital.glb#Scene0".to_string()),
            turret_yaw: AssetRef::from("self://gltf/turret-yaw-01.glb#Scene0".to_string()),
            turret_pitch: AssetRef::from("self://gltf/turret-pitch-01.glb#Scene0".to_string()),
            turret_barrel: AssetRef::from("self://gltf/turret-barrel-01.glb#Scene0".to_string()),
            torpedo_bay: AssetRef::from("self://gltf/torpedo-bay-01.glb#Scene0".to_string()),
            turret_fire_sound: AssetRef::from("self://sounds/turret_fire.wav".to_string()),
            turret_dry_fire_sound: AssetRef::from("self://sounds/dry_fire.wav".to_string()),
            torpedo_launch_sound: AssetRef::from("self://sounds/torpedo_launch.wav".to_string()),
            controller_lock_on_sound: AssetRef::from("self://sounds/lock_on.wav".to_string()),
            controller_lock_off_sound: AssetRef::from("self://sounds/lock_off.wav".to_string()),
            controller_radar_deny_sound: AssetRef::from("self://sounds/radar_deny.wav".to_string()),
            controller_radar_retarget_sound: AssetRef::from(
                "self://sounds/radar_retarget.wav".to_string(),
            ),
            controller_safety_on_sound: AssetRef::from("self://sounds/safety_on.wav".to_string()),
            controller_rcs_loop_sound: AssetRef::from("self://sounds/rcs_loop.wav".to_string()),
            section_impact_sound: AssetRef::from("self://sounds/impact.wav".to_string()),
            section_destroy_sound: AssetRef::from("self://sounds/explosion.wav".to_string()),
            thruster_loop_sound: AssetRef::from("self://sounds/thruster_loop.wav".to_string()),
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
