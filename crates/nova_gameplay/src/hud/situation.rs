//! What the player's ship is DOING right now, sensed once per frame for the
//! contextual HUD.
//!
//! Demo 2's ruleset (`web/design/hud_rework_poc.html`, `reflect()`) is written
//! against a handful of situations - autopilot burn, combat lock, weapons hot,
//! firing, low ammo - and every contextual rule keys off one of them. Sensing
//! them ONCE into [`HudSituations`] keeps the widgets dumb (a chip asks "is the
//! burn live?", not "is there an `Autopilot` with a `Goto` action on the single
//! player ship?") and keeps the ruleset readable in one place instead of
//! scattered across a dozen queries that could drift apart.
//!
//! No new gameplay state: every field is read from components the HUD already
//! consumed ([`Autopilot`], [`CombatLock`], [`WeaponsHot`], [`SectionAmmo`],
//! [`TurretSectionInput`]).

use bevy::prelude::*;

use crate::prelude::*;

/// The `HudSituations` sensing resource.
pub mod prelude {
    pub use super::HudSituations;
}

/// The live flight/combat situations the contextual HUD reacts to, resolved
/// every frame by [`sense_hud_situations`] from the player ship. All-false is
/// "idle cruise": the near-empty screen the ruleset starts from.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[reflect(Resource)]
pub struct HudSituations {
    /// The maneuver the autopilot is flying, if any (the mode chip's verb).
    pub maneuver: Option<FlightVerb>,
    /// A combat lock is held.
    pub combat_lock: bool,
    /// The weapons safety is off.
    pub weapons_hot: bool,
    /// A weapon trigger is down right now.
    pub firing: bool,
    /// Some player weapon group is at or below its low-ammo mark.
    pub low_ammo: bool,
}

impl HudSituations {
    /// Whether the ammo gauges are relevant: while shooting is on the table, or
    /// whenever a group is nearly dry (which is news even in cruise).
    pub fn ammo_relevant(&self) -> bool {
        self.weapons_hot || self.low_ammo
    }

    /// Whether nothing is going on - the near-empty idle-cruise screen.
    pub fn idle_cruise(&self) -> bool {
        *self == Self::default()
    }
}

/// Resolve [`HudSituations`] from the player ship. Exactly one player ship, the
/// same rule the other HUD drivers follow; with no ship every situation reads
/// false, so a torn-down flight rig leaves the HUD in idle cruise rather than
/// frozen mid-fight.
pub fn sense_hud_situations(
    mut situations: ResMut<HudSituations>,
    q_ship: Query<
        (
            Entity,
            Option<&Autopilot>,
            Option<&CombatLock>,
            Option<&WeaponsHot>,
        ),
        With<PlayerSpaceshipMarker>,
    >,
    q_sections: Query<
        (&ChildOf, Option<&SectionAmmo>, Option<&TurretSectionInput>),
        With<SectionMarker>,
    >,
) {
    let next = match q_ship.single() {
        Ok((ship, autopilot, combat, hot)) => {
            let weapons_hot = hot.is_some_and(|hot| hot.0);
            let mut firing = false;
            let mut low_ammo = false;
            for (&ChildOf(parent), ammo, trigger) in &q_sections {
                if parent != ship {
                    continue;
                }
                // A held trigger only counts as firing while the safety is off
                // - the section systems refuse the shot otherwise, so a pulsing
                // reticle would be a lie.
                firing |= weapons_hot && trigger.is_some_and(|trigger| trigger.0);
                low_ammo |= ammo.is_some_and(super::ammo_readout::is_low_ammo);
            }
            HudSituations {
                maneuver: autopilot.map(maneuver_verb),
                combat_lock: combat.is_some_and(|combat| combat.0.is_some()),
                weapons_hot,
                firing,
                low_ammo,
            }
        }
        Err(_) => HudSituations::default(),
    };

    // set_if_neq semantics by hand: only dirty the resource on a real change,
    // like `FlightVerbHints`.
    if *situations != next {
        *situations = next;
    }
}

/// The flight verb an engaged maneuver corresponds to, so the dock can light
/// the chip whose key produced it.
fn maneuver_verb(autopilot: &Autopilot) -> FlightVerb {
    match autopilot.action {
        AutopilotAction::Stop => FlightVerb::Stop,
        AutopilotAction::Goto { .. } | AutopilotAction::GotoPos { .. } => FlightVerb::Goto,
        AutopilotAction::Orbit { .. } => FlightVerb::Orbit,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sense_app() -> App {
        let mut app = App::new();
        app.init_resource::<HudSituations>();
        app.add_systems(Update, sense_hud_situations);
        app
    }

    fn situations(app: &App) -> HudSituations {
        *app.world().resource::<HudSituations>()
    }

    /// No flight rig at all reads as idle cruise, not as a stale fight.
    #[test]
    fn without_a_player_ship_every_situation_is_idle() {
        let mut app = sense_app();
        app.update();
        assert!(situations(&app).idle_cruise());
    }

    /// The engaged maneuver is reported as its VERB, which is what the dock
    /// needs to know which chip is hot.
    #[test]
    fn an_engaged_maneuver_reports_its_verb() {
        let mut app = sense_app();
        let ship = app.world_mut().spawn(PlayerSpaceshipMarker).id();
        app.update();
        assert_eq!(situations(&app).maneuver, None, "manual flight");

        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Stop));
        app.update();
        assert_eq!(situations(&app).maneuver, Some(FlightVerb::Stop));

        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::GotoPos {
                position: Vec3::X,
            }));
        app.update();
        assert_eq!(
            situations(&app).maneuver,
            Some(FlightVerb::Goto),
            "a positional GOTO is still the GOTO verb"
        );
    }

    /// Firing needs BOTH a held trigger and a hot safety - the section systems
    /// refuse the shot otherwise.
    #[test]
    fn firing_needs_the_safety_off_as_well_as_the_trigger() {
        let mut app = sense_app();
        let ship = app
            .world_mut()
            .spawn((PlayerSpaceshipMarker, WeaponsHot(false)))
            .id();
        app.world_mut()
            .spawn((SectionMarker, TurretSectionInput(true), ChildOf(ship)));
        app.update();
        assert!(!situations(&app).firing, "safety on: not firing");

        app.world_mut().entity_mut(ship).insert(WeaponsHot(true));
        app.update();
        assert!(situations(&app).firing, "safety off with the trigger down");
    }

    /// A nearly-dry magazine is news; a full one is not. Another ship's empty
    /// weapon is not the player's problem.
    #[test]
    fn low_ammo_reads_the_players_own_weapons_only() {
        let mut app = sense_app();
        let ship = app.world_mut().spawn(PlayerSpaceshipMarker).id();
        let other = app.world_mut().spawn_empty().id();
        let mine = app
            .world_mut()
            .spawn((SectionMarker, SectionAmmo::new(10), ChildOf(ship)))
            .id();
        app.world_mut().spawn((
            SectionMarker,
            SectionAmmo {
                rounds: 0,
                ..SectionAmmo::new(10)
            },
            ChildOf(other),
        ));
        app.update();
        assert!(
            !situations(&app).low_ammo,
            "a full own magazine and another ship's empty one are both quiet"
        );

        app.world_mut().entity_mut(mine).insert(SectionAmmo {
            rounds: 0,
            ..SectionAmmo::new(10)
        });
        app.update();
        assert!(situations(&app).low_ammo, "my own group ran dry");
    }
}
