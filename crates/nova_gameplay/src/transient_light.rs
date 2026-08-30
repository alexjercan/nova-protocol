//! A light that exists for a moment: the flash a detonation throws onto the
//! hulls beside it.
//!
//! Nothing in combat cast light before this module. A warhead that does not
//! light the ship next to it cannot read as a warhead whatever its particle
//! graph does, because the particles are the only thing in the frame that knows
//! the explosion happened - the hull, the rocks and the other ships are lit
//! exactly as they were the frame before.
//!
//! # Why a request and not a spawn
//!
//! A flash is asked for with [`LightFlash`] rather than spawned by the effect
//! that wants one. Real-time lights are the one visual cost that does not scale
//! down gracefully: a forward renderer pays per light per fragment it touches,
//! so ten flashes in one frame is not ten times one flash's cost, it is a
//! cliff. The cap therefore cannot live at the call sites, where every caller
//! would have to count the others.
//!
//! So the policy lives here, once: [`GraphicsBudget::transient_lights`] says
//! how many may burn at a time, this module's observer counts what is already
//! lit and drops the request when there is no room. A caller asks and does not
//! check. Dropping the newest rather than dimming or evicting the oldest is
//! deliberate - an evicted light is a flash that vanishes mid-fade, which reads
//! as a bug, while a dropped one is simply a flash that never happened in a
//! frame already full of them.
//!
//! # Why it is not a child of the thing that flashed
//!
//! The blast volume it belongs to is despawned within a tenth of a second (it
//! exists to resolve one overlap set), and the flash outlives it. So the light
//! is its own entity at a world position, and it cleans itself up.

use bevy::prelude::*;

use crate::prelude::{GraphicsBudget, TempEntity};

/// `LightFlash`, `TransientLight` and `TransientLightPlugin`.
pub mod prelude {
    pub use super::{LightFlash, TransientLight, TransientLightPlugin};
}

/// Asks for a brief light at a world position.
///
/// The request may be refused - see the module docs - so nothing may depend on
/// a light having appeared. It is a visual cue and never a gameplay fact.
#[derive(Event, Clone, Copy, Debug)]
pub struct LightFlash {
    /// Where it burns, in world space.
    pub at: Vec3,
    /// The colour it burns at.
    pub color: Color,
    /// Brightness at the first frame, in lumens ([`PointLight::intensity`]).
    pub peak_intensity: f32,
    /// How far it reaches, in world units ([`PointLight::range`]).
    pub range: f32,
    /// How long it burns, in seconds.
    pub duration: f32,
}

/// A light currently burning down. Spawned by this module's observer in answer
/// to a [`LightFlash`]; a range can count these to assert the cap held.
#[derive(Component, Clone, Copy, Debug, Reflect)]
#[reflect(Component)]
pub struct TransientLight {
    /// Seconds it has burned.
    pub age: f32,
    /// Seconds it burns for in total.
    pub duration: f32,
    /// The brightness it started at, in lumens.
    pub peak_intensity: f32,
}

/// Brightness left at progress `t` (0 at ignition, 1 at the end), as a fraction
/// of the peak.
///
/// Cubic and not linear, so the light is nearly gone by the time a third of its
/// life has passed. A detonation is a flash: most of the light is in the first
/// few frames, and a linear fade instead reads as a lamp being switched off.
/// Clamped at both ends, and pure so the curve can be read without a running
/// app.
fn flash_falloff(t: f32) -> f32 {
    let remaining = 1.0 - t.clamp(0.0, 1.0);
    remaining * remaining * remaining
}

/// Lights brief flashes on request, under one cap.
pub struct TransientLightPlugin;

impl Plugin for TransientLightPlugin {
    fn build(&self, app: &mut App) {
        trace!("TransientLightPlugin: build");

        app.register_type::<TransientLight>();
        app.add_observer(light_the_flash);
        app.add_systems(Update, burn_transient_lights);
    }
}

/// Answer a [`LightFlash`], or drop it because the frame is already full.
///
/// A settings-less app (an example, a test rig) has no [`GraphicsBudget`] and
/// gets the default one, which is full quality - the same fallback every other
/// budgeted effect takes.
///
/// The count and the spawn are one QUEUED command rather than a query and a
/// `Commands::spawn`, and that is the whole cap: a salvo triggers this observer
/// several times inside one frame, and a deferred spawn is invisible to a query
/// until the flush, so every request in the salvo would count the same zero and
/// every one of them would be honoured. Queued commands run in order against
/// the real world, so the second request sees the first one's light.
fn light_the_flash(flash: On<LightFlash>, mut commands: Commands) {
    let request = *flash;
    commands.queue(move |world: &mut World| {
        let cap = world.get_resource::<GraphicsBudget>().map_or_else(
            || GraphicsBudget::default().transient_lights,
            |budget| budget.transient_lights,
        );
        let lit = {
            let mut query = world.query_filtered::<(), With<TransientLight>>();
            query.iter(world).count()
        };
        if lit >= cap {
            trace!("light_the_flash: {lit} already lit, cap {cap} - dropped");
            return;
        }

        world.spawn((
            Name::new("Transient Light"),
            TransientLight {
                age: 0.0,
                duration: request.duration,
                peak_intensity: request.peak_intensity,
            },
            PointLight {
                color: request.color,
                intensity: request.peak_intensity,
                range: request.range,
                radius: 0.0,
                // A flash is over before a shadow map would earn its cost, and
                // a detonation's own light is the one place a missing shadow is
                // invisible: everything it touches is being lit from inside a
                // fireball that is itself unshadowed.
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_translation(request.at),
            TempEntity(request.duration),
        ));
    });
}

/// Burn every live flash down its curve. The entity itself is reaped by
/// [`TempEntity`], so this only writes brightness.
fn burn_transient_lights(
    time: Res<Time>,
    mut q_lit: Query<(&mut TransientLight, &mut PointLight)>,
) {
    let delta = time.delta_secs();
    for (mut lit, mut light) in &mut q_lit {
        lit.age += delta;
        let t = if lit.duration > 0.0 {
            lit.age / lit.duration
        } else {
            1.0
        };
        light.intensity = lit.peak_intensity * flash_falloff(t);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_falloff_starts_full_ends_dark_and_front_loads_the_light() {
        assert_eq!(flash_falloff(0.0), 1.0);
        assert_eq!(flash_falloff(1.0), 0.0);
        // A third of the way in, most of the light is already gone - which is
        // what makes it read as a flash rather than as a lamp.
        assert!(
            flash_falloff(1.0 / 3.0) < 0.3,
            "a third of the way in the flash must be under a third as bright"
        );
        assert!(flash_falloff(0.5) < flash_falloff(0.25), "monotonic");
    }

    #[test]
    fn the_falloff_clamps_outside_its_range() {
        assert_eq!(flash_falloff(-1.0), 1.0);
        assert_eq!(flash_falloff(2.0), 0.0);
    }

    /// A rig with the plugin and a budget, so the cap can be driven.
    fn light_app(cap: usize) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, TransientLightPlugin));
        app.insert_resource(GraphicsBudget {
            transient_lights: cap,
            ..GraphicsBudget::default()
        });
        app
    }

    fn lit_count(app: &mut App) -> usize {
        app.world_mut()
            .query_filtered::<(), With<TransientLight>>()
            .iter(app.world())
            .count()
    }

    fn ask_for_a_flash(app: &mut App) {
        app.world_mut().trigger(LightFlash {
            at: Vec3::ZERO,
            color: Color::WHITE,
            peak_intensity: 1000.0,
            range: 10.0,
            duration: 0.2,
        });
    }

    #[test]
    fn a_request_lights_one_flash() {
        let mut app = light_app(4);
        ask_for_a_flash(&mut app);
        app.update();
        assert_eq!(lit_count(&mut app), 1);
    }

    #[test]
    fn the_cap_drops_the_requests_past_it() {
        let mut app = light_app(2);
        for _ in 0..5 {
            ask_for_a_flash(&mut app);
        }
        app.update();
        assert_eq!(
            lit_count(&mut app),
            2,
            "the cap holds however many ask in one frame"
        );
    }

    #[test]
    fn a_zero_cap_lights_nothing() {
        let mut app = light_app(0);
        ask_for_a_flash(&mut app);
        app.update();
        assert_eq!(lit_count(&mut app), 0);
    }

    #[test]
    fn a_burning_flash_dims() {
        let mut app = light_app(4);
        // A long burn, so the claim under test is the CURVE and not whether
        // `TempEntity` reaped the light before the second read.
        app.world_mut().trigger(LightFlash {
            at: Vec3::ZERO,
            color: Color::WHITE,
            peak_intensity: 1000.0,
            range: 10.0,
            duration: 600.0,
        });
        app.update();
        let brightness = |app: &mut App| {
            let mut query = app.world_mut().query::<&PointLight>();
            query
                .iter(app.world())
                .next()
                .expect("the flash is lit")
                .intensity
        };
        let bright = brightness(&mut app);
        // Two more updates: bevy's first manual tick is dt 0, so one would age
        // the light by nothing at all.
        app.update();
        app.update();
        let dimmer = brightness(&mut app);
        assert!(
            dimmer < bright,
            "the flash must be dimming ({bright} -> {dimmer})"
        );
    }
}
