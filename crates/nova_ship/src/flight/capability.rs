//! What the flight computer lets a hull do: the one live-controller verb gate
//! every consumer asks through, so a lit hint, a firing key, the radar and the
//! autopilot can never disagree about the same ship.

use bevy::prelude::*;
use nova_gameplay::prelude::*;

use crate::prelude::*;

/// Every LIVE flight computer in the world, with its (optional) withheld
/// verbs: the one query shape behind [`ship_grants_verb`].
///
/// A live flight computer is a controller section that still has its
/// [`PDController`] (preview controllers have none) and is not disabled. The PD
/// is required through the DATA rather than a `With` filter because the
/// autopilot reads these same rows for the hull's rotation authority, so one
/// query answers both and the two can never disagree about which computers
/// count.
///
/// [`WithheldVerbs`] is optional: a controller missing the component falls back
/// to the all-granted default rather than becoming ungovernable.
pub(crate) type LiveFlightComputers<'w, 's> = Query<
    'w,
    's,
    (
        &'static PDController,
        &'static ChildOf,
        Option<&'static WithheldVerbs>,
    ),
    (
        With<ControllerSectionMarker>,
        Without<SectionInactiveMarker>,
    ),
>;

/// Whether some live flight computer on `ship` grants `verb` (union across
/// computers). Doubles as the computer-present check: no live computer, no
/// grant.
///
/// The ONE answer to that question. The flight rig's maneuver observers, the
/// hint pass that lights their keys, the radar gate and the RCS primitive all
/// call this, so a dark hint and a dead key always mean the same thing.
pub(crate) fn ship_grants_verb(
    ship: Entity,
    verb: FlightVerb,
    computers: &LiveFlightComputers,
) -> bool {
    computers.iter().any(|(_, &ChildOf(parent), withheld)| {
        parent == ship && withheld.is_none_or(|withheld| withheld.granted(verb))
    })
}

/// Whether every live flight computer on `ship` withholds `verb` - the
/// FAIL-OPEN counterpart of [`ship_grants_verb`], and false on a hull that has
/// no live flight computer at all.
///
/// Point defence is the one capability read this way, deliberately: a hull with
/// no controller section is one of the bare rigs the examples and the section
/// ranges fly, and a range whose guns silently stood down would be a worse
/// failure than one that defends itself. Only an explicit `DisableVerb` /
/// `SetControllerVerb` takes the capability away.
pub(crate) fn ship_withholds_verb(
    ship: Entity,
    verb: FlightVerb,
    computers: &LiveFlightComputers,
) -> bool {
    let mut computed = false;
    for (_, &ChildOf(parent), withheld) in computers {
        if parent != ship {
            continue;
        }
        computed = true;
        if withheld.is_none_or(|withheld| withheld.granted(verb)) {
            return false;
        }
    }
    computed
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    /// The hull the gate is asked about.
    #[derive(Resource)]
    struct Subject(Entity);

    /// What [`ship_grants_verb`] answered for the subject, per verb.
    #[derive(Resource, Default)]
    struct Granted(Vec<FlightVerb>);

    /// What [`ship_withholds_verb`] answered for the subject's point defence.
    #[derive(Resource, Default)]
    struct WithholdsPointDefense(bool);

    /// Every verb the gate answers for. The match below is exhaustive, so a new
    /// [`FlightVerb`] variant stops compiling here, one line under the list it
    /// has to be added to, instead of quietly escaping the property tests.
    fn every_verb() -> [FlightVerb; 6] {
        let all = [
            FlightVerb::Stop,
            FlightVerb::Goto,
            FlightVerb::Orbit,
            FlightVerb::Lock,
            FlightVerb::Rcs,
            FlightVerb::PointDefense,
        ];
        for verb in all {
            match verb {
                FlightVerb::Stop
                | FlightVerb::Goto
                | FlightVerb::Orbit
                | FlightVerb::Lock
                | FlightVerb::Rcs
                | FlightVerb::PointDefense => {}
            }
        }
        all
    }

    fn collect_answers(
        subject: Res<Subject>,
        computers: LiveFlightComputers,
        mut granted: ResMut<Granted>,
        mut withholds: ResMut<WithholdsPointDefense>,
    ) {
        granted.0 = every_verb()
            .into_iter()
            .filter(|verb| ship_grants_verb(subject.0, *verb, &computers))
            .collect();
        withholds.0 = ship_withholds_verb(subject.0, FlightVerb::PointDefense, &computers);
    }

    /// A live flight computer, as `controller_section` builds one: the marker
    /// plus the [`PDController`] that makes it a computer.
    fn live_computer(world: &mut World, ship: Entity) -> Entity {
        world
            .spawn((
                ChildOf(ship),
                ControllerSectionMarker,
                PDController {
                    frequency: 4.0,
                    damping_ratio: 4.0,
                    max_angular_acceleration: 40.0,
                    sustained_angular_speed: f32::INFINITY,
                },
            ))
            .id()
    }

    /// The editor's view ship, as `preview_controller_section` builds one: a
    /// [`ControllerSectionMarker`] with NO [`PDController`].
    fn preview_computer(world: &mut World, ship: Entity) -> Entity {
        world.spawn((ChildOf(ship), ControllerSectionMarker)).id()
    }

    fn answers(world: &mut World, ship: Entity) -> (Vec<FlightVerb>, bool) {
        world.insert_resource(Subject(ship));
        world.init_resource::<Granted>();
        world.init_resource::<WithholdsPointDefense>();
        world.run_system_once(collect_answers).unwrap();
        (
            world.resource::<Granted>().0.clone(),
            world.resource::<WithholdsPointDefense>().0,
        )
    }

    /// The property the copies of this gate were supposed to share and did not:
    /// a controller section with no `PDController` - the editor's preview ship
    /// - is not a live flight computer, so EVERY verb answers the same "no".
    /// The Lock answer used to come from a copy that omitted the PD filter and
    /// said yes here.
    #[test]
    fn a_preview_controller_grants_no_verb_at_all() {
        let mut world = World::new();
        let ship = world.spawn_empty().id();
        preview_computer(&mut world, ship);

        let (granted, _) = answers(&mut world, ship);

        assert!(
            granted.is_empty(),
            "a preview controller carries no PD, so it is not a live flight \
             computer and grants nothing - these verbs said otherwise: {granted:?}"
        );
    }

    /// Delivery guard: the production default - a live computer carrying no
    /// `WithheldVerbs` - grants every verb, so the test above is measuring the
    /// missing PD and not a dead fixture.
    #[test]
    fn a_live_computer_with_no_withheld_set_grants_every_verb() {
        let mut world = World::new();
        let ship = world.spawn_empty().id();
        live_computer(&mut world, ship);

        let (granted, _) = answers(&mut world, ship);

        assert_eq!(
            granted,
            every_verb().to_vec(),
            "an absent WithheldVerbs is the all-granted default"
        );
    }

    /// The one deliberate divergence, pinned: point defence is read fail-open,
    /// so a hull with no live flight computer keeps its guns. Only a LIVE
    /// computer that withholds the verb stands them down.
    #[test]
    fn point_defence_stands_down_only_for_a_live_computer_that_withholds_it() {
        let mut world = World::new();

        let bare = world.spawn_empty().id();
        let (_, bare_withholds) = answers(&mut world, bare);
        assert!(
            !bare_withholds,
            "a bare rig with no controller section at all still defends itself"
        );

        let preview = world.spawn_empty().id();
        preview_computer(&mut world, preview);
        let (_, preview_withholds) = answers(&mut world, preview);
        assert!(
            !preview_withholds,
            "a preview controller is not a live computer, so it withholds nothing"
        );

        let live = world.spawn_empty().id();
        let computer = live_computer(&mut world, live);
        world.entity_mut(computer).insert(WithheldVerbs(
            [FlightVerb::PointDefense].into_iter().collect(),
        ));
        let (_, live_withholds) = answers(&mut world, live);
        assert!(
            live_withholds,
            "an explicit DisableVerb / SetControllerVerb is the only thing that \
             takes point defence away"
        );
    }
}
