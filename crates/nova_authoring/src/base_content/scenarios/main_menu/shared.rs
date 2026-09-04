//! Shared stage builders for private main-menu backdrop scenarios.

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_scenario::prelude::*;

use crate::base_content::ships;

/// The body the menu is framed around.
///
/// A TEMPERATE world, and deliberately the showiest type there is: this is the
/// game's first frame, and a blue-green world with a coastline says "space
/// game with real planets" before a single word of the menu is read. Every
/// other type is available to a backdrop that wants a different mood.
///
/// 900 m of mean radius puts the surface at 945 m, within 0.5% of the 940.6 m
/// body the rock here published, so the orbiter's ring and the well the menu
/// ambience flies against are unchanged.
pub(super) fn backdrop_planetoid(mass: f32) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "menu_planetoid".to_string(),
            name: "Menu Planetoid".to_string(),
            position: Meters3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Planet(
            PlanetConfig::new(MENU_PLANETOID_TYPE, Meters(900.0), MENU_PLANETOID_SEED)
                .anchored(mass),
        ),
    }
}

/// The menu world's type (see [`backdrop_planetoid`]).
pub(super) const MENU_PLANETOID_TYPE: PlanetType = PlanetType::Temperate;
/// Draws deep sea broken by pale-sand shelves and green continents, under a
/// bright cap. Picked against the menu's own key light, which is strong enough
/// to wash a shallower draw out to olive.
pub(super) const MENU_PLANETOID_SEED: u32 = 7;

/// The backdrop camera contract: every backdrop poses its OWN camera with a
/// `SetCamera` in its OnStart (lint makes a poseless backdrop an Error).
/// The menu derives nothing - the authored position IS the framing, fully
/// deterministic. The reference pose is `(0, 570, 1920)` m looking at the
/// origin (a 4:3 window then sees ~+-1,060 m at origin depth, 16:9 ~+-1,410);
/// a 4:3 half-width is ~0.55 x the camera's distance, so pull further back for
/// a wider stage.
///
/// The pose is an AUDIO decision as well as a framing one. A backdrop is
/// heard through the same rolloff flight uses
/// (`nova_gameplay::audio::SFX_FAR_DISTANCE`, 3.2 km to silence), so an actor
/// further out than that is mute however good the shot is. The duel and the
/// weave came IN for that reason - the first cut framed the duel from 3,130 m,
/// which put the whole fight past the rolloff's far end. The gauntlet and the
/// waystation did NOT: each is pinned by its own composition, and both say so
/// at their own `SetCamera`.
pub(super) fn backdrop_camera(position: Meters3) -> EventActionConfig {
    EventActionConfig::SetCamera(SetCameraActionConfig {
        position,
        look_at: Meters3::ZERO,
    })
}

/// A small AI ship on the orbit directive around the backdrop planetoid -
/// the proven menu actor (the ORBIT autopilot plans its ring from the
/// well's runtime geometry). Never `SpaceshipController::Player`: the
/// spaceship input sets are LIVE in MainMenu (see menu_ambience's warning).
pub(super) fn backdrop_orbiter(
    id: &str,
    name: &str,
    position: Meters3,
    // The silhouette knob: `true` flies the wide block hauler (the waystation
    // freighters), `false` the small block cutter (the scrapyard tug). Both
    // are hand-built cube ships in the industrial look - a backdrop asks the
    // player to read TONNAGE at two kilometres, and width reads at that
    // range where detail does not.
    cargo: bool,
) -> ScenarioObjectConfig {
    let hull = if cargo {
        ships::BLOCK_HAULER_SHIP_ID
    } else {
        ships::BLOCK_CUTTER_SHIP_ID
    };
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance: None,
            controller: SpaceshipController::AI(AIControllerConfig {
                orbit: Some("menu_planetoid".to_string()),
                ..Default::default()
            }),
            hull: ships::hull(hull),
            ..Default::default()
        }),
    }
}

/// The shared backdrop rig: the standard three-point key/rim/fill, aimed at the
/// planetoid every menu scene frames, scaled to the backdrop's ~2 km stage.
///
/// Every backdrop carries one - deleting the engine's hardcoded key light made
/// lighting authored content, and a menu scene that authors none renders black.
pub(super) fn backdrop_rig(prefix: &str) -> ThreePointRig {
    ThreePointRig::around(prefix, Meters3::ZERO, 20.0)
}

/// A warm positional lamp just off the planetoid's limb: the falloff a
/// directional light cannot give, so near dressing reads brighter than far.
pub(super) fn planetoid_glow(id: &str) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: "Planetoid Glow".to_string(),
            position: Meters3::new(-600.0, 200.0, 900.0),
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Light(LightConfig::Point {
            // Lumens at backdrop scale: the lamp sits ~1.1 km from the
            // planetoid and must still register against an 11000 lux key.
            intensity: 2_500_000.0,
            range: Meters(4_000.0),
            radius: Meters(120.0),
            color: Color::srgb(1.0, 0.82, 0.6),
            shadows: false,
        }),
    }
}

/// A static dressing beacon (label + warm little light). Below the orbit
/// plane and outside the planetoid's geometric radius, like everything
/// else in a backdrop.
pub(super) fn backdrop_beacon(
    id: &str,
    label: &str,
    position: Meters3,
    color: Color,
) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: label.to_string(),
            position,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Beacon(BeaconConfig {
            label: label.to_string(),
            radius: Meters(20.0),
            color,
            area_radius: None,
            lock_signature: None,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The menu backdrop kept its geometry across the move to a planet.
    ///
    /// Same derivation as the belt bodies
    /// (`the_belt_planets_keep_the_body_radius_their_rocks_published`): the
    /// rock here published a 940.6 m body radius, and the menu orbiter is
    /// posed against that, not against the 200 m designation in its config.
    #[test]
    fn the_menu_world_keeps_the_body_radius_its_rock_published() {
        let object = backdrop_planetoid(30_000.0);
        let ScenarioObjectKind::Planet(planet) = &object.kind else {
            panic!("the menu backdrop must be a planet");
        };
        let drift = (planet.body_radius().0 - 940.6) / 940.6;
        assert!(
            drift.abs() < 0.01,
            "the menu world measures {:.1} m against the 940.6 m its rock \
             published ({:.1}% off)",
            planet.body_radius().0,
            drift * 100.0
        );
    }
}
