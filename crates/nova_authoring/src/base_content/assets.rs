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
        }
    }
}
