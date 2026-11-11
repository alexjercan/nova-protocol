//! What a slug LANDING looks like: one steel-blue penetration flash and one
//! dense spark burst at the entry face.
//!
//! The round's own light rides it down the range and the sweep's crater
//! records where it bit; what was missing was the instant between them. A
//! slug arrives at 15 km/s, so the hit is a single frame, and a single frame
//! has to be lit to be seen at all.
//!
//! ONE flash and ONE burst per slug, at the FIRST body it meets. A raking shot
//! reports a [`SurfaceImpact`] for every cell it crosses - twenty-eight on the
//! shipped corridor, inside one fixed tick - and lighting each of them would
//! throw a thousand sparks at a wall the player sees as one hit. The entry
//! face is where the shot arrived, which is the whole of what the flash says.

use nova_gameplay::transient_light::prelude::LightFlash;

use super::*;

/// Brightness of the penetration flash, in lumens. A third of the muzzle
/// flash and level with the light the slug carried in: the arrival is the
/// slug's own light dumped into the plating, not a second discharge.
const IMPACT_LIGHT_LUMENS: f32 = 300_000.0;

/// How far the penetration flash reaches, in world units (400 m): the face it
/// hit and the hull around it, so the struck ship is thrown into relief from
/// its own wound.
const IMPACT_LIGHT_RANGE: f32 = 40.0;

/// How long the penetration flash burns, in seconds. Shorter than the muzzle
/// flash: a discharge builds and dies, an arrival only dies.
const IMPACT_LIGHT_SECS: f32 = 0.15;

/// Sparks thrown at the entry face. Eight times the juice layer's hit burst
/// and over the muzzle's: a lance round meeting plate is the one hit in the
/// game that is supposed to read as material leaving a hull.
const IMPACT_SPARK_COUNT: u32 = 40;

/// How far the entry sparks fly, against the juice layer's 1.0 for a killing
/// hit. The slug's whole 15 km/s goes into the spall, so it leaves faster than
/// a PDC round's.
const IMPACT_SPARK_FORCE: f32 = 1.5;

/// A slug that has already landed once, so the cells behind the entry face
/// draw nothing more. Inserted by [`on_slug_impact_flash`] on the first hit
/// and never removed: the slug dies where it stops.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub(super) struct RailgunSlugStruck;

/// Flash and spark the entry face when a slug meets its first body.
///
/// The light goes through the transient-light cap and the sparks through the
/// spark plugin; nothing here reads the graphics tier. The slug is still alive
/// here: the sweep queues the hit before the despawn, and an observer's own
/// commands land before the next queued trigger runs, which is what lets the
/// marker gate the second cell of the same shot.
pub(super) fn on_slug_impact_flash(
    impact: On<SurfaceImpact>,
    q_slug: Query<
        (),
        (
            With<RailgunSlugProjectileMarker>,
            Without<RailgunSlugStruck>,
        ),
    >,
    mut commands: Commands,
) {
    let Some(slug) = impact.source.filter(|source| q_slug.contains(*source)) else {
        return;
    };
    debug!(
        "on_slug_impact_flash: slug {slug:?} struck {:?} at {}",
        impact.entity, impact.at
    );
    commands.entity(slug).try_insert(RailgunSlugStruck);
    commands.trigger(LightFlash {
        at: impact.at,
        color: RAIL_GLOW_COLOR,
        peak_intensity: IMPACT_LIGHT_LUMENS,
        range: IMPACT_LIGHT_RANGE,
        duration: IMPACT_LIGHT_SECS,
    });
    commands.trigger(ImpactSparks {
        at: impact.at,
        count: IMPACT_SPARK_COUNT,
        force: IMPACT_SPARK_FORCE,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every flash and every burst the observer asked for, in order.
    #[derive(Resource, Default)]
    struct Asked {
        flashes: Vec<LightFlash>,
        bursts: Vec<ImpactSparks>,
    }

    fn impact_app() -> App {
        let mut app = App::new();
        app.init_resource::<Asked>();
        app.add_observer(on_slug_impact_flash);
        app.add_observer(|flash: On<LightFlash>, mut asked: ResMut<Asked>| {
            asked.flashes.push(*flash);
        });
        app.add_observer(|burst: On<ImpactSparks>, mut asked: ResMut<Asked>| {
            asked.bursts.push(*burst);
        });
        app
    }

    /// Queue a hit and flush: the observer answers through `Commands`, so the
    /// flash and the burst land on the flush.
    fn strike(app: &mut App, target: Entity, source: Option<Entity>, kind: DamageType, at: Vec3) {
        app.world_mut().commands().trigger(SurfaceImpact {
            entity: target,
            source,
            kind,
            at,
        });
        app.world_mut().flush();
    }

    fn asked(app: &App) -> &Asked {
        app.world().resource::<Asked>()
    }

    #[test]
    fn a_slug_hit_asks_for_one_flash_and_one_burst_at_the_contact_point() {
        let mut app = impact_app();
        let plate = app.world_mut().spawn_empty().id();
        let slug = app.world_mut().spawn(RailgunSlugProjectileMarker).id();
        let at = Vec3::new(3.0, -2.0, 50.0);

        strike(&mut app, plate, Some(slug), DamageType::Pierce, at);

        let asked = asked(&app);
        assert_eq!(asked.flashes.len(), 1, "one slug hit is one flash");
        assert_eq!(asked.flashes[0].at, at, "the flash sits on the contact");
        assert_eq!(asked.flashes[0].color, RAIL_GLOW_COLOR);
        assert_eq!(asked.bursts.len(), 1, "one slug hit is one burst");
        assert_eq!(asked.bursts[0].at, at, "the burst sits on the contact");
        // Denser and harder than the juice layer's own hit burst, which is
        // what makes it read as the lance's and not as one more PDC hit.
        let juice = JuiceSettings::default().sparks.impact_count;
        assert!(
            asked.bursts[0].count > juice && asked.bursts[0].count > MUZZLE_SPARK_COUNT,
            "the arrival throws more than a hit's {juice} and the muzzle's {MUZZLE_SPARK_COUNT}"
        );
        assert!(
            asked.bursts[0].force > 1.0,
            "the spall leaves harder than a hit's"
        );
    }

    /// A raking shot reports every cell it crosses inside one tick. The entry
    /// face is lit; the twenty-seven behind it are not.
    #[test]
    fn a_rake_lights_the_entry_face_once_and_not_every_cell_behind_it() {
        let mut app = impact_app();
        let slug = app.world_mut().spawn(RailgunSlugProjectileMarker).id();
        let cells: Vec<Entity> = (0..4).map(|_| app.world_mut().spawn_empty().id()).collect();

        // All four hits queued, then ONE flush: the shape of a sweep's tick.
        for (depth, cell) in cells.iter().enumerate() {
            app.world_mut().commands().trigger(SurfaceImpact {
                entity: *cell,
                source: Some(slug),
                kind: DamageType::Pierce,
                at: Vec3::Z * depth as f32,
            });
        }
        app.world_mut().flush();

        let asked = asked(&app);
        assert_eq!(asked.flashes.len(), 1, "one shot is one flash");
        assert_eq!(
            asked.flashes[0].at,
            Vec3::ZERO,
            "the entry face is the one lit"
        );
        assert_eq!(asked.bursts.len(), 1, "one shot is one burst");
    }

    #[test]
    fn a_pdc_round_and_a_sourceless_hit_ask_for_none_of_the_railguns() {
        let mut app = impact_app();
        let plate = app.world_mut().spawn_empty().id();
        let round = app.world_mut().spawn(TurretBulletProjectileMarker).id();

        strike(&mut app, plate, Some(round), DamageType::Kinetic, Vec3::ONE);
        strike(&mut app, plate, None, DamageType::Explosive, Vec3::ONE);

        let asked = asked(&app);
        assert!(asked.flashes.is_empty(), "only a slug lights its hit");
        assert!(asked.bursts.is_empty(), "only a slug sparks its hit");
    }
}
