//! Everything First Shift puts on the map that the shared belt does not own:
//! the scenario ids, the temporary marks the lessons are flown against, the
//! crates, and the two poses the attack is filmed from.
//!
//! It is one module so a pacing revision is a numbers edit in one place. The
//! fixed belt - both planetoids, the rock plate, the far dressing - lives in
//! `stage` and is shared with chapter two; nothing here may move it.
//!
//! Layout provenance: `examples/playable/first_shift_map.rs`, the spatial
//! bench this stage was reviewed in.

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;

use super::super::stage;
use crate::scenario_helpers::prelude::*;

// --- scenario ids ------------------------------------------------------------

/// The player's cutter. Named, not `player_spaceship`: it is a crewed ship
/// with a part in the story, and chapter two flies the same one.
pub(super) const ID_CUTTER: &str = "cutter";
pub(super) const ID_CARRIER: &str = "carrier";
pub(super) const ID_WARSHIP: &str = "warship";
/// The invisible arrival gate around the inspection planetoid.
pub(super) const ID_APPROACH_RING: &str = "approach_ring";
/// The wreck's automatic beacon, put up in the epilogue.
pub(super) const ID_DISTRESS: &str = "distress_beacon";

// --- the cutter's launch -----------------------------------------------------

/// The cutter undocks off the carrier's port side, close enough that the hull
/// fills the mirror.
pub(super) const CUTTER_START_POS: Meters3 = Meters3::new(-1_100.0, 0.0, 2_500.0);

/// Soft manual-speed cap for the whole shift: close work stays controllable
/// without silently changing the helm after the first lesson. GOTO plans its
/// own speed independently.
pub(super) const CUTTER_SPEED_CAP: MetersPerSecond = MetersPerSecond(150.0);

// --- temporary marks ---------------------------------------------------------

/// One temporary navigation mark: a lit beacon the script puts up for one
/// lesson and takes down again.
///
/// Taking it down means DESPAWN, not just dropping the HUD chip. A finished
/// beacon left burning in the belt is a mark the player keeps flying to, and
/// by the end of the shift the route would be a line of them.
pub(super) struct TempMark {
    /// Scenario id, and the id the marker and despawn are addressed to.
    pub(super) id: &'static str,
    /// What the beacon and its HUD chip read.
    pub(super) label: &'static str,
    pub(super) position: Meters3,
    /// Trigger volume. A hand-flown mark wants a tight one; a mark the
    /// autopilot parks at must contain the park point.
    pub(super) area: Meters,
    /// Radar signature, for the marks LOCK and GOTO are taught against. A
    /// beacon without one is invisible to the targeting computer.
    pub(super) lock_signature: Option<Meters>,
}

impl TempMark {
    /// Put the mark up and point the HUD at it.
    pub(super) fn raise(&self) -> Vec<EventActionConfig> {
        vec![
            spawn_object(stage::sized_beacon(
                self.id,
                self.label,
                self.position,
                self.area,
                self.lock_signature,
            )),
            attach_objective_marker(self.id, self.label),
        ]
    }

    /// Take it down: the chip first, then the mark itself.
    pub(super) fn clear(&self) -> Vec<EventActionConfig> {
        vec![detach_objective_marker(self.id), despawn_object(self.id)]
    }

    /// OnEnter/OnExit of this mark's trigger volume by the cutter.
    pub(super) fn entered(&self) -> EventFilterConfig {
        entity_pair(self.id, ID_CUTTER)
    }
}

/// The launch leg's mark: a short hop out of the carrier's shadow, in open
/// space well short of the plate. Nothing but stick and throttle, and close
/// enough that a first-time pilot is not still braking when the next lesson
/// starts. The tight volume is what makes it a place rather than a direction.
pub(super) const WORK_MARK: TempMark = TempMark {
    id: "work_mark",
    label: "WORK MARK",
    position: Meters3::new(-500.0, 80.0, 900.0),
    area: Meters(300.0),
    lock_signature: None,
};

/// The RCS lesson, taught in OPEN SPACE and one axis at a time. Both marks sit
/// a few hundred metres off the launch mark: far enough that the translation
/// is a real maneuver at the 100 m/s RCS cap, close enough that it is a nudge
/// rather than a leg. The trigger is tight on purpose - the lesson is placing
/// the hull, and a wide sphere would pass a player who merely drifted past.
pub(super) const TRIM_LATERAL: TempMark = TempMark {
    id: "trim_mark_lateral",
    label: "TRIM A",
    position: Meters3::new(-200.0, 80.0, 900.0),
    area: Meters(100.0),
    lock_signature: None,
};

/// The second axis, straight up off the first. Same lesson, no new words.
pub(super) const TRIM_VERTICAL: TempMark = TempMark {
    id: "trim_mark_vertical",
    label: "TRIM B",
    position: Meters3::new(-200.0, 300.0, 900.0),
    area: Meters(100.0),
    lock_signature: None,
};

/// The first transit mark: the leg LOCK and GOTO are taught on, out west of
/// the plate in clear space. Sized for the autopilot, which parks 500 m short
/// of an unsized target, and lit on radar so there is something to lock.
pub(super) const TRANSIT_ONE: TempMark = TempMark {
    id: "transit_mark_one",
    label: "TRANSIT 1",
    position: Meters3::new(-1_600.0, 100.0, -3_600.0),
    area: stage::BEACON_AREA_RADIUS,
    lock_signature: Some(TRANSIT_SIGNATURE),
};

/// The second: the same gesture again with almost nothing said over it, and
/// the staging point the orbit detour is proposed from. Deliberately OUTSIDE
/// the inspection planetoid's arrival ring, so the detour is a decision rather
/// than something that happens on arrival.
pub(super) const TRANSIT_TWO: TempMark = TempMark {
    id: "transit_mark_two",
    label: "TRANSIT 2",
    position: Meters3::new(-2_600.0, 0.0, -4_600.0),
    area: stage::BEACON_AREA_RADIUS,
    lock_signature: Some(TRANSIT_SIGNATURE),
};

/// The work site the shift comes back to: back on the plate, in the roomiest
/// pocket the rock field has on the carrier's side of it. The last crate is
/// worked from here, and it is far enough clear of the rocks that a player
/// looking at the belt rather than the panel is not also about to hit one.
pub(super) const WORK_SITE: TempMark = TempMark {
    id: "work_site",
    label: "WORK SITE",
    position: Meters3::new(1_400.0, -100.0, -1_200.0),
    area: stage::BEACON_AREA_RADIUS,
    lock_signature: Some(TRANSIT_SIGNATURE),
};

/// The hold the shift ENDS at, and where the whole set piece is composed from:
/// the outer mark off the Meridian's starboard quarter, flown to with the
/// crates aboard. The set piece is staged against this one point, so every
/// number here is a camera decision:
///
/// - 3.06 km off the Meridian, which puts the largest hull in the game in the
///   frame whole rather than as a shape in the distance.
/// - 110 degrees round from the warship's firing mark, so the thing that comes
///   out of the belt arrives over the player's shoulder and not head on.
/// - 2.15 km off the torpedo lane between the two - a Breaker's blast reaches
///   450 m, and the mark's own 700 m volume is inside that margin, so no part
///   of the hold the player can arrive at is under the ordnance.
/// - 3.27 km clear of the nearest rock, because the player is meant to be
///   watching the sky.
///
/// It is lit on radar and sized for the autopilot: the leg home is a GOTO with
/// the crew talking over it, and the cutter parks itself where the shot is.
pub(super) const HOME_MARK: TempMark = TempMark {
    id: "home_mark",
    label: "MERIDIAN HOLD",
    position: Meters3::new(2_000.0, -600.0, 2_400.0),
    area: stage::BEACON_AREA_RADIUS,
    lock_signature: Some(TRANSIT_SIGNATURE),
};

/// Radar signature carried by the marks the targeting computer is taught on.
pub(super) const TRANSIT_SIGNATURE: Meters = Meters(300.0);

// --- the crates --------------------------------------------------------------

/// The three crates the shift is actually about, in the order they are
/// revealed - and revealed ONE AT A TIME, because three chips at once turns a
/// lesson in flying the plate into a shopping list.
///
/// The route runs from the plate's near edge, where the rocks are sparse and a
/// mistake costs nothing, into the middle of it, where they are not. The third
/// is the one the crew abandons for the orbit detour and comes back for.
pub(super) const CRATE_POSITIONS: [Meters3; 3] = [
    Meters3::new(-200.0, -60.0, -1_400.0),
    Meters3::new(200.0, 100.0, -3_000.0),
    Meters3::new(1_800.0, -120.0, -1_400.0),
];

/// Crate pickup radius: tight enough to require flying AT the crate, which is
/// the whole reason the thrusters are taught first.
pub(super) const CRATE_AREA_RADIUS: Meters = Meters(80.0);

/// One crate, by index into [`CRATE_POSITIONS`] (1-based, as the objective
/// text counts them).
pub(super) fn crate_id(number: usize) -> String {
    format!("crate_{number}")
}

/// The crate object itself. Despawns on pickup, which the salvage layer owns.
pub(super) fn crate_object(number: usize) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: crate_id(number),
            name: format!("Maintenance Crate {number}"),
            position: CRATE_POSITIONS[number - 1],
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::SalvageCrate(SalvageCrateConfig {
            size: Meters(15.0),
            area_radius: CRATE_AREA_RADIUS,
            pickup_sound: Some(AssetRef::from("self://sounds/salvage_pickup.wav")),
        }),
    }
}

// --- the inspection round ----------------------------------------------------

/// The detour's arrival gate: an invisible sphere on the small planetoid. GOTO
/// parks 500 m outside the geometric body (700-1 200 m), so the ring contains
/// every park point on every mesh seed; the widest orbit ring (1.82 km) is
/// inside it too, so holding the orbit cannot fall out of the gate.
pub(super) const APPROACH_RING_RADIUS: Meters = Meters(2_400.0);

// --- the warship -------------------------------------------------------------

/// Where the warship waits: off the large planetoid's flank, 3.9 km from its
/// centre - outside the widest mesh the body can grow (3.0 km) and behind it
/// from everything the shift is flown through. It is spawned there when the
/// beat opens, so nothing can have seen it earlier.
pub(super) const WARSHIP_HIDE_POS: Meters3 = Meters3::new(8_400.0, 250.0, -6_500.0);
/// Where it comes OUT to. Splitting the approach in two is what buys the set
/// piece its first beat: this mark is 1.5 km clear of the planetoid's widest
/// possible body and in plain view of the hold, so the plume is seen before
/// the ship is identified.
pub(super) const WARSHIP_EMERGE_POS: Meters3 = Meters3::new(7_600.0, 300.0, -3_200.0);
/// Where it shoots from: 6.6 km off the carrier, and broadside to a cutter
/// holding station off the carrier's quarter. The player watches the whole
/// thing from abeam.
pub(super) const WARSHIP_FIRING_POS: Meters3 = Meters3::new(3_700.0, 150.0, -2_200.0);
/// Where it goes afterwards. Nothing waits on this arrival - the order exists
/// to make the ship leave under thrust rather than blink out.
///
/// It leaves OUTBOUND, up and away from the belt. The obvious exit - back the
/// way it came, past the large planetoid - is a straight line through a body
/// whose mesh reaches three kilometres, and a `MoveShipTo` has no avoidance of
/// its own: the warship flew into the planetoid and died on the way out.
pub(super) const WARSHIP_EXIT_POS: Meters3 = Meters3::new(12_000.0, 2_000.0, 1_000.0);
/// How close the two approach legs park: the margin between the warship's own
/// hull and the mark. The default 500 m is fine for gameplay and far too loose
/// for staging - both marks are chosen for their sight lines. This hull is
/// 119 m from its centre of mass to its outer face, so 200 m of margin puts
/// its centre 319 m off the mark.
pub(super) const WARSHIP_APPROACH_STANDOFF: Meters = Meters(200.0);
/// How square the bore must be on the carrier before the guns are allowed to
/// speak. Two degrees at 6.6 km is a 230 m error - inside a hull this size.
pub(super) const WARSHIP_ALIGN_TOLERANCE: f32 = 2.0;

// --- the cinematic ----------------------------------------------------------
//
// Four shots, each anchored to the ship the beat is ABOUT and offset so that
// ship sits about twenty degrees off the view axis: near ground on one side of
// the frame, the thing it is looking at down the middle. All four are measured
// in WORLD axes, because two of the anchors are free to turn and a hull-local
// offset would compose the shot differently every run.

/// The entry shot, on the cutter, as the warship comes out from behind the
/// large body: the player's own hull in the near ground and the plume 8 km down
/// the frame. Set BEFORE the ship is identified, so the reveal is a shot rather
/// than a caption.
pub(super) const CINEMA_ENTRY_OFFSET: Meters3 = Meters3::new(-100.0, 25.0, 185.0);

/// The launch shot, on the WARSHIP, 350 m back down its own firing line: the
/// tubes fill the frame and six torpedoes leave straight away from the camera
/// toward a carrier seven kilometres out. This is the only shot the cutter is
/// not in, and it runs the length of the bay walk and no longer.
pub(super) const CINEMA_TUBES_OFFSET: Meters3 = Meters3::new(150.0, 90.0, -300.0);

/// The impact shot, on the MERIDIAN, 680 m off its far side: the carrier holds
/// the frame, the warship is a shape on the axis behind it, and the slugs and
/// the torpedoes both arrive down the middle.
pub(super) const CINEMA_IMPACT_OFFSET: Meters3 = Meters3::new(-285.0, 175.0, 595.0);

/// The last shot, back on the CUTTER and 240 m off its quarter, looking three
/// kilometres down the hold at a carrier that is about to stop existing.
///
/// The cut off the Meridian is not decoration. A camera anchored to a hull
/// loses its anchor when that hull dies, and the wreck goes in the same second
/// the last torpedo lands - so the end of the set piece would otherwise be the
/// camera snapping home by itself, on the frame that matters most. Leaving
/// early puts the player's own ship back in the composition and makes the kill
/// something they watch rather than something that happens to the camera.
pub(super) const CINEMA_DEATH_OFFSET: Meters3 = Meters3::new(225.0, 10.0, -85.0);
