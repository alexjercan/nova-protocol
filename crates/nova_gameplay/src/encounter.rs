//! Encounters: what the open world may put in the player's way on a long
//! flight, the seeded draw that picks one, and the record a save carries.
//!
//! An [`EncounterDesign`] is authored content: a weight, the player it may
//! fire for ([`EncounterPlayerType`]) and one kind from the closed
//! [`EncounterType`]. The mod merge publishes the effective designs as
//! [`GameEncounters`]; [`encounter_faults`] is the rule the merge and content
//! lint share. An empty catalog is valid: nothing fires.
//!
//! [`plan_encounter_draw`] is the one draw. It reads the world seed and the
//! count of encounters already fired, so the same world draws the same
//! encounter after the same number of them, whatever the frame rate, and a
//! Load neither repeats nor skips one.
//!
//! [`Encounters`] is the record: the count fired, the eligible travel
//! seconds since the last one, the hail offer that stands and the encounter
//! that is live. It holds no entity: it names bodies by [`GeneratedBodyId`],
//! so it outlives the cells that hold them. The open world crate owns the
//! director that advances it and puts the bodies in the world.

use std::{collections::BTreeMap, fmt};

use bevy::prelude::*;
use bevy_rand::prelude::WyRand;
use nova_events::prelude::GeneratedBodyId;
use rand::{RngExt, SeedableRng};

use crate::{
    hash::Fnv64,
    jobs::prelude::{Job, JobType},
};

/// Glob-import surface: `use nova_gameplay::encounter::prelude::*`.
pub mod prelude {
    pub use super::{
        encounter_faults, plan_encounter_draw, EncounterDesign, EncounterDesignId, EncounterDraw,
        EncounterLive, EncounterOffer, EncounterPlayerType, EncounterType, Encounters,
        GameEncounters, HailJobType, ENCOUNTER_GAP_S, HAIL_LINE_MAX_CHARS,
    };
}

/// The least and the most eligible travel seconds between two encounters,
/// both included. Each encounter draws its own gap inside it
/// ([`plan_encounter_draw`]). Long enough that space stays mostly empty,
/// short enough that a long flight may meet one.
pub const ENCOUNTER_GAP_S: (u32, u32) = (300, 900);

/// The most characters a hail's line may hold: what one comms line shows.
pub const HAIL_LINE_MAX_CHARS: usize = 120;

/// The authored id of an encounter, namespaced like a recipe:
/// `base/distress_call`. A save names the live encounter by it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct EncounterDesignId(pub String);

impl EncounterDesignId {
    /// The id as authored.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for EncounterDesignId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl From<&str> for EncounterDesignId {
    fn from(id: &str) -> Self {
        Self(id.to_string())
    }
}

/// Which player an encounter may fire for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EncounterPlayerType {
    /// Every hull.
    Any,
    /// Only a hull that carries a live weapon section: a fight is no
    /// encounter for a hull that cannot answer it.
    Armed,
}

impl EncounterPlayerType {
    /// Whether an encounter for this player may fire for a hull that is
    /// `armed` or not.
    pub const fn admits(self, armed: bool) -> bool {
        match self {
            Self::Any => true,
            Self::Armed => armed,
        }
    }
}

/// The job a hailing hull offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum HailJobType {
    /// Dock with the caller ([`JobType::Distress`]).
    Distress,
    /// Bring the caller goods and dock with it ([`JobType::Supply`]).
    Supply,
}

impl HailJobType {
    /// Whether a job of `kind` is the job this hail offers.
    pub fn offers(self, kind: &JobType) -> bool {
        matches!(
            (self, kind),
            (Self::Distress, JobType::Distress { .. }) | (Self::Supply, JobType::Supply { .. })
        )
    }
}

/// What an encounter is. Closed: each variant is code in the open world's
/// director, so content picks among them and tunes how often each fires.
#[derive(Clone, Debug, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub enum EncounterType {
    /// A stranded hull hails the player and offers a job the player accepts
    /// or declines in space.
    Hail {
        /// The job the caller offers.
        job: HailJobType,
        /// What the caller says on comms: one line, not empty, at most
        /// [`HAIL_LINE_MAX_CHARS`] characters.
        line: String,
    },
    /// Hostile hulls lie in wait ahead.
    Ambush,
    /// A derelict hull drifts ahead, there to be found and looted.
    Derelict,
}

/// One authored encounter. Every field is required.
#[derive(Clone, Debug, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct EncounterDesign {
    /// Its id.
    pub id: EncounterDesignId,
    /// Its share of the draw among the designs that may fire, above zero:
    /// a design of weight 3 fires three times as often as one of weight 1.
    pub weight: u32,
    /// The player it may fire for.
    pub player: EncounterPlayerType,
    /// What it is.
    pub kind: EncounterType,
}

/// The merged encounters, by id. Inserted by the mod merge.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct GameEncounters(pub BTreeMap<EncounterDesignId, EncounterDesign>);

impl GameEncounters {
    /// The catalog of `encounters`; a later one with an id replaces an
    /// earlier one.
    pub fn new(encounters: impl IntoIterator<Item = EncounterDesign>) -> Self {
        Self(
            encounters
                .into_iter()
                .map(|encounter| (encounter.id.clone(), encounter))
                .collect(),
        )
    }

    /// The encounter `id`, if the catalog holds it.
    pub fn get(&self, id: &EncounterDesignId) -> Option<&EncounterDesign> {
        self.0.get(id)
    }

    /// The encounter `id`, for an id a save check or a draw already
    /// resolved.
    ///
    /// # Panics
    ///
    /// When no encounter has the id: a live record named an encounter the
    /// loaded content lacks, which [`Encounters::validate`] refuses first.
    pub fn design(&self, id: &EncounterDesignId) -> &EncounterDesign {
        self.get(id)
            .unwrap_or_else(|| panic!("encounter '{id}' is not in the loaded encounter catalog"))
    }

    /// Every encounter, in id order.
    pub fn iter(&self) -> impl Iterator<Item = &EncounterDesign> {
        self.0.values()
    }
}

/// Every fault of `encounter` on its own, keyed in the message by its id:
/// an empty id, a weight of zero, and a hail line that is empty, runs over
/// more than one line or holds more than [`HAIL_LINE_MAX_CHARS`] characters.
pub fn encounter_faults(encounter: &EncounterDesign) -> Vec<String> {
    let id = &encounter.id;
    let mut faults = Vec::new();
    if id.0.trim().is_empty() {
        faults.push("an encounter has an empty id".to_string());
    }
    if encounter.weight == 0 {
        faults.push(format!(
            "the encounter '{id}' has a weight of 0; it must be above 0"
        ));
    }
    match &encounter.kind {
        EncounterType::Hail { line, .. } => {
            if line.trim().is_empty() {
                faults.push(format!("the encounter '{id}' hails with an empty line"));
            }
            if line.chars().any(|letter| letter == '\n' || letter == '\r') {
                faults.push(format!(
                    "the encounter '{id}' hails with more than one line"
                ));
            }
            let letters = line.chars().count();
            if letters > HAIL_LINE_MAX_CHARS {
                faults.push(format!(
                    "the encounter '{id}' hails with {letters} characters; a line holds at most \
                     {HAIL_LINE_MAX_CHARS}"
                ));
            }
        }
        EncounterType::Ambush | EncounterType::Derelict => {}
    }
    faults
}

/// What [`plan_encounter_draw`] drew for one encounter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EncounterDraw<'a> {
    /// The eligible travel seconds that pass before the encounter fires,
    /// inside [`ENCOUNTER_GAP_S`].
    pub gap_s: u32,
    /// The design that fires, or `None` when no design may fire for the
    /// player.
    pub design: Option<&'a EncounterDesign>,
}

/// Draw encounter number `fired` of a world of `seed`: the travel gap before
/// it and the design that fires, picked by weight among the `designs` that
/// admit a player who is `armed` or not ([`EncounterPlayerType::admits`]).
///
/// The draws come off one `WyRand` keyed by the seed and `fired`, so they
/// read the same every time they are asked for. The gap is the first draw
/// and does not read `designs` or `armed`: it stands while the catalog or
/// the hull changes. A new draw goes last, so the draws before it keep
/// their numbers. The pick walks `designs` in id order, so it is the same
/// for one catalog whatever order its packs loaded in.
pub fn plan_encounter_draw(
    seed: u32,
    fired: u32,
    designs: &GameEncounters,
    armed: bool,
) -> EncounterDraw<'_> {
    let key = Fnv64::new()
        .write(&seed.to_le_bytes())
        .write(b"encounter")
        .write(&fired.to_le_bytes())
        .finish();
    let mut rng = WyRand::from_seed(key.to_le_bytes());
    let gap_s = rng.random_range(ENCOUNTER_GAP_S.0..=ENCOUNTER_GAP_S.1);
    // Drawn whatever the catalog holds: an empty set must not shift a later
    // draw.
    let pick: u64 = rng.random();

    let eligible = || designs.iter().filter(|design| design.player.admits(armed));
    let total: u64 = eligible().map(|design| u64::from(design.weight)).sum();
    let design = (total > 0).then(|| {
        let mut left = pick % total;
        eligible()
            .find(|design| {
                let weight = u64::from(design.weight);
                if left < weight {
                    return true;
                }
                left -= weight;
                false
            })
            .expect("the pick is below the weight total")
    });
    EncounterDraw { gap_s, design }
}

/// The job a hailing hull offers while the player has not answered.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct EncounterOffer {
    /// The job, under a hail id whose poster is the caller.
    pub job: Job,
    /// The [`crate::clock::WorldClock`] reading the unanswered offer ends
    /// at.
    pub expires_at: f64,
}

/// The encounter in the world now.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct EncounterLive {
    /// The design that fired.
    pub design: EncounterDesignId,
    /// Every body it put in the world, at least one, none twice.
    pub bodies: Vec<GeneratedBodyId>,
    /// The [`crate::clock::WorldClock`] reading it fired at.
    pub started_at: f64,
}

/// The encounter record of one world. A session resource like
/// [`crate::jobs::Jobs`]: a world save carries it and a Load puts it back.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Encounters {
    /// How many encounters have fired: the number [`plan_encounter_draw`]
    /// draws the next one by.
    pub fired: u32,
    /// The eligible travel seconds since the last encounter fired.
    pub travel_seconds: f64,
    /// The hail offer the player has not answered, if one stands.
    pub offer: Option<EncounterOffer>,
    /// The encounter in the world, if one is live.
    pub live: Option<EncounterLive>,
}

impl Encounters {
    /// Refuse a record a Load could not have built, against the loaded
    /// `designs` and the clock at `clock_seconds`: travel seconds that are
    /// not finite or are negative; a live record under a design the catalog
    /// lacks, with no body, with a body listed twice or with a stamp that is
    /// not finite, is negative or stands ahead of the clock; an offer with
    /// no live record, with an end that is not finite or is negative, whose
    /// job is no hail job or one [`Job::validate`] refuses, whose caller is
    /// no body of the live record, or whose live design does not hail with
    /// that kind of job.
    pub fn validate(&self, designs: &GameEncounters, clock_seconds: f64) -> Result<(), String> {
        if !self.travel_seconds.is_finite() || self.travel_seconds < 0.0 {
            return Err(format!(
                "the encounters count {} s of travel, which is not a finite time from 0 up",
                self.travel_seconds
            ));
        }
        let live = match &self.live {
            Some(live) => {
                let Some(design) = designs.get(&live.design) else {
                    return Err(format!(
                        "the live encounter names the design '{}', which is no encounter",
                        live.design
                    ));
                };
                if live.bodies.is_empty() {
                    return Err(format!(
                        "the live encounter '{}' holds no body",
                        live.design
                    ));
                }
                for (index, body) in live.bodies.iter().enumerate() {
                    if live.bodies[..index].contains(body) {
                        return Err(format!(
                            "the live encounter '{}' lists the body '{body}' twice",
                            live.design
                        ));
                    }
                }
                if !live.started_at.is_finite()
                    || live.started_at < 0.0
                    || live.started_at > clock_seconds
                {
                    return Err(format!(
                        "the live encounter '{}' is stamped at {} s of play but the clock reads \
                         {clock_seconds} s",
                        live.design, live.started_at
                    ));
                }
                Some((live, design))
            }
            None => None,
        };
        let Some(offer) = &self.offer else {
            return Ok(());
        };
        let id = &offer.job.id;
        if !id.is_hail() {
            return Err(format!(
                "the encounters offer '{id}', which no hull hailed with"
            ));
        }
        offer.job.validate()?;
        if !offer.expires_at.is_finite() || offer.expires_at < 0.0 {
            return Err(format!(
                "the hail offer '{id}' ends at {} s of play, which is not a finite time from 0 up",
                offer.expires_at
            ));
        }
        let Some((live, design)) = live else {
            return Err(format!(
                "the encounters offer '{id}' with no live encounter"
            ));
        };
        if !live.bodies.contains(&id.poster) {
            return Err(format!(
                "the hail offer '{id}' names a caller that is no body of the live encounter '{}'",
                live.design
            ));
        }
        let hails = matches!(
            &design.kind,
            EncounterType::Hail { job, .. } if job.offers(&offer.job.kind)
        );
        if !hails {
            return Err(format!(
                "the hail offer '{id}' is a {} job, which the live encounter '{}' does not hail \
                 with",
                offer.job.kind.name(),
                live.design
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use nova_events::prelude::SectorCoord;

    use super::*;
    use crate::jobs::prelude::{JobId, JobReward};

    fn design(id: &str, weight: u32, player: EncounterPlayerType) -> EncounterDesign {
        EncounterDesign {
            id: id.into(),
            weight,
            player,
            kind: EncounterType::Derelict,
        }
    }

    fn hail(id: &str, job: HailJobType, line: &str) -> EncounterDesign {
        EncounterDesign {
            id: id.into(),
            weight: 1,
            player: EncounterPlayerType::Any,
            kind: EncounterType::Hail {
                job,
                line: line.to_string(),
            },
        }
    }

    fn body(stem: &str) -> GeneratedBodyId {
        GeneratedBodyId::new(SectorCoord::new(2, 0, -1), stem, 0)
    }

    fn distress_offer(caller: &GeneratedBodyId) -> EncounterOffer {
        EncounterOffer {
            job: Job {
                id: JobId::hail(caller.clone(), 4),
                kind: JobType::Distress {
                    caller: caller.clone(),
                },
                target: caller.clone(),
                leg_m: 8_000,
                threat: 0,
                reward: JobReward::flat(600),
            },
            expires_at: 1_300.0,
        }
    }

    /// A catalog with a distress hail, and a record with its offer standing.
    fn offered() -> (GameEncounters, Encounters) {
        let caller = body("encounter_4_caller");
        let designs = GameEncounters::new([
            hail("base/distress_call", HailJobType::Distress, "Mayday."),
            hail(
                "base/supply_call",
                HailJobType::Supply,
                "We are out of food.",
            ),
            design("base/ambush", 2, EncounterPlayerType::Armed),
        ]);
        let record = Encounters {
            fired: 5,
            travel_seconds: 12.5,
            offer: Some(distress_offer(&caller)),
            live: Some(EncounterLive {
                design: "base/distress_call".into(),
                bodies: vec![caller],
                started_at: 1_000.0,
            }),
        };
        (designs, record)
    }

    #[test]
    fn a_design_with_a_weight_and_a_short_line_has_no_fault() {
        assert!(
            encounter_faults(&design("base/derelict_find", 3, EncounterPlayerType::Any)).is_empty()
        );
        let longest = "a".repeat(HAIL_LINE_MAX_CHARS);
        assert!(
            encounter_faults(&hail("base/distress_call", HailJobType::Distress, &longest))
                .is_empty()
        );
    }

    #[test]
    fn a_zero_weight_an_empty_id_and_a_bad_hail_line_are_faults() {
        let faults = encounter_faults(&design("", 0, EncounterPlayerType::Any));
        assert_eq!(
            faults,
            vec![
                "an encounter has an empty id".to_string(),
                "the encounter '' has a weight of 0; it must be above 0".to_string(),
            ]
        );

        let empty = encounter_faults(&hail("mod/a", HailJobType::Supply, "  "));
        assert_eq!(
            empty,
            vec!["the encounter 'mod/a' hails with an empty line"]
        );

        let two_lines = encounter_faults(&hail("mod/a", HailJobType::Supply, "Mayday.\nMayday."));
        assert_eq!(
            two_lines,
            vec!["the encounter 'mod/a' hails with more than one line"]
        );

        // Counted in characters, not bytes: a 120-character line of two-byte
        // letters passes, and one more letter does not.
        let wide = "\u{e9}".repeat(HAIL_LINE_MAX_CHARS);
        assert!(encounter_faults(&hail("mod/a", HailJobType::Supply, &wide)).is_empty());
        let long = encounter_faults(&hail("mod/a", HailJobType::Supply, &format!("{wide}a")));
        assert_eq!(
            long,
            vec![
                "the encounter 'mod/a' hails with 121 characters; a line holds at most 120"
                    .to_string()
            ]
        );
    }

    #[test]
    fn the_same_seed_and_count_draw_the_same_gap_and_design() {
        let designs = GameEncounters::new([
            design("a", 1, EncounterPlayerType::Any),
            design("b", 1, EncounterPlayerType::Any),
            design("c", 1, EncounterPlayerType::Armed),
        ]);
        let mut gaps = std::collections::BTreeSet::new();
        let mut picks = std::collections::BTreeSet::new();
        for fired in 0..200 {
            let draw = plan_encounter_draw(0x5eed, fired, &designs, true);
            assert_eq!(draw, plan_encounter_draw(0x5eed, fired, &designs, true));
            assert!(
                (ENCOUNTER_GAP_S.0..=ENCOUNTER_GAP_S.1).contains(&draw.gap_s),
                "{draw:?}"
            );
            gaps.insert(draw.gap_s);
            picks.insert(draw.design.unwrap().id.clone());
        }
        assert!(gaps.len() > 100, "each encounter draws its own gap");
        assert_eq!(picks.len(), 3, "every design fires over 200 encounters");
        assert_ne!(
            (0..20)
                .map(|fired| plan_encounter_draw(1, fired, &designs, true).gap_s)
                .collect::<Vec<_>>(),
            (0..20)
                .map(|fired| plan_encounter_draw(2, fired, &designs, true).gap_s)
                .collect::<Vec<_>>(),
            "another world draws other gaps"
        );
    }

    #[test]
    fn the_gap_does_not_read_the_catalog_or_the_hull() {
        let full = GameEncounters::new([
            design("a", 1, EncounterPlayerType::Any),
            design("b", 5, EncounterPlayerType::Armed),
        ]);
        let empty = GameEncounters::default();
        for fired in 0..50 {
            let gap_s = plan_encounter_draw(7, fired, &full, true).gap_s;
            assert_eq!(plan_encounter_draw(7, fired, &full, false).gap_s, gap_s);
            assert_eq!(plan_encounter_draw(7, fired, &empty, true).gap_s, gap_s);
        }
    }

    #[test]
    fn a_design_fires_in_proportion_to_its_weight() {
        let designs = GameEncounters::new([
            design("heavy", 3, EncounterPlayerType::Any),
            design("light", 1, EncounterPlayerType::Any),
        ]);
        let draws = 4_000;
        let heavy = (0..draws)
            .filter(|fired| {
                plan_encounter_draw(0x5eed, *fired, &designs, false)
                    .design
                    .unwrap()
                    .id
                    .as_str()
                    == "heavy"
            })
            .count();
        // Three in four, with room for the spread of 4000 draws.
        assert!(
            (2_850..=3_150).contains(&heavy),
            "heavy fired {heavy} times"
        );
    }

    #[test]
    fn an_unarmed_player_never_draws_an_armed_design() {
        let designs = GameEncounters::new([
            design("ambush", 50, EncounterPlayerType::Armed),
            design("find", 1, EncounterPlayerType::Any),
        ]);
        let mut armed_picks = std::collections::BTreeSet::new();
        for fired in 0..500 {
            let unarmed = plan_encounter_draw(3, fired, &designs, false);
            assert_eq!(unarmed.design.unwrap().id.as_str(), "find");
            armed_picks.insert(
                plan_encounter_draw(3, fired, &designs, true)
                    .design
                    .unwrap()
                    .id
                    .clone(),
            );
        }
        assert_eq!(armed_picks.len(), 2, "an armed player draws both");
    }

    #[test]
    fn an_empty_or_fully_filtered_set_draws_no_design() {
        let empty = GameEncounters::default();
        let armed_only = GameEncounters::new([design("ambush", 2, EncounterPlayerType::Armed)]);
        for fired in 0..50 {
            assert_eq!(plan_encounter_draw(9, fired, &empty, true).design, None);
            assert_eq!(
                plan_encounter_draw(9, fired, &armed_only, false).design,
                None
            );
            assert!(plan_encounter_draw(9, fired, &armed_only, true)
                .design
                .is_some());
        }
    }

    #[test]
    fn a_record_with_a_live_hail_and_its_offer_is_valid() {
        let (designs, record) = offered();
        record.validate(&designs, 1_000.0).unwrap();
        Encounters::default()
            .validate(&GameEncounters::default(), 0.0)
            .unwrap();

        let mut answered = record;
        answered.offer = None;
        answered.validate(&designs, 2_000.0).unwrap();
    }

    #[test]
    fn validate_refuses_travel_seconds_that_are_negative_or_not_finite() {
        let (designs, record) = offered();
        for seconds in [-0.5, f64::NAN, f64::INFINITY] {
            let mut bad = record.clone();
            bad.travel_seconds = seconds;
            let refusal = bad.validate(&designs, 1_000.0).unwrap_err();
            assert!(refusal.contains("s of travel"), "{refusal}");
        }
    }

    #[test]
    fn validate_refuses_a_live_record_under_an_unknown_design() {
        let (designs, mut record) = offered();
        record.live.as_mut().unwrap().design = "mod/gone".into();
        let refusal = record.validate(&designs, 1_000.0).unwrap_err();
        assert!(refusal.contains("which is no encounter"), "{refusal}");
    }

    #[test]
    fn validate_refuses_a_live_record_with_no_body_or_a_body_twice() {
        let (designs, record) = offered();
        let mut none = record.clone();
        none.live.as_mut().unwrap().bodies.clear();
        let refusal = none.validate(&designs, 1_000.0).unwrap_err();
        assert!(refusal.contains("holds no body"), "{refusal}");

        let mut twice = record;
        let bodies = &mut twice.live.as_mut().unwrap().bodies;
        bodies.push(bodies[0].clone());
        let refusal = twice.validate(&designs, 1_000.0).unwrap_err();
        assert!(refusal.ends_with("twice"), "{refusal}");
    }

    #[test]
    fn validate_refuses_a_live_stamp_that_is_negative_not_finite_or_ahead_of_the_clock() {
        let (designs, record) = offered();
        for stamp in [-1.0, f64::NAN, f64::INFINITY, 1_000.5] {
            let mut bad = record.clone();
            bad.live.as_mut().unwrap().started_at = stamp;
            let refusal = bad.validate(&designs, 1_000.0).unwrap_err();
            assert!(refusal.contains("but the clock reads 1000 s"), "{refusal}");
        }
    }

    #[test]
    fn validate_refuses_an_offer_with_no_live_record() {
        let (designs, mut record) = offered();
        record.live = None;
        let refusal = record.validate(&designs, 1_000.0).unwrap_err();
        assert!(refusal.contains("with no live encounter"), "{refusal}");
    }

    #[test]
    fn validate_refuses_an_offer_whose_job_is_no_hail_job() {
        let (designs, mut record) = offered();
        let caller = body("encounter_4_caller");
        record.offer.as_mut().unwrap().job.id = JobId::slot(caller, 0, 0);
        let refusal = record.validate(&designs, 1_000.0).unwrap_err();
        assert!(refusal.contains("which no hull hailed with"), "{refusal}");
    }

    #[test]
    fn validate_refuses_an_offer_whose_job_is_not_valid() {
        let (designs, mut record) = offered();
        record.offer.as_mut().unwrap().job.reward = JobReward::flat(0);
        let refusal = record.validate(&designs, 1_000.0).unwrap_err();
        assert!(refusal.contains("pays nothing"), "{refusal}");

        let (designs, mut record) = offered();
        record.offer.as_mut().unwrap().job.kind = JobType::Procure {
            item: "IronOre".into(),
            count: 2,
        };
        let refusal = record.validate(&designs, 1_000.0).unwrap_err();
        assert!(
            refusal.contains("not a distress call or a supply job"),
            "{refusal}"
        );
    }

    #[test]
    fn validate_refuses_an_offer_end_that_is_negative_or_not_finite() {
        let (designs, record) = offered();
        for end in [-1.0, f64::NAN, f64::NEG_INFINITY] {
            let mut bad = record.clone();
            bad.offer.as_mut().unwrap().expires_at = end;
            let refusal = bad.validate(&designs, 1_000.0).unwrap_err();
            assert!(refusal.contains("ends at"), "{refusal}");
        }
        let mut past = record;
        past.offer.as_mut().unwrap().expires_at = 10.0;
        past.validate(&designs, 1_000.0)
            .expect("a save may fall on the frame an offer ends, before it is retired");
    }

    #[test]
    fn validate_refuses_an_offer_from_a_caller_the_live_encounter_does_not_hold() {
        let (designs, mut record) = offered();
        record.live.as_mut().unwrap().bodies = vec![body("encounter_4_wreck")];
        let refusal = record.validate(&designs, 1_000.0).unwrap_err();
        assert!(
            refusal.contains("no body of the live encounter"),
            "{refusal}"
        );
    }

    #[test]
    fn validate_refuses_an_offer_the_live_design_does_not_hail_with() {
        let (designs, record) = offered();
        for other in ["base/supply_call", "base/ambush"] {
            let mut bad = record.clone();
            bad.live.as_mut().unwrap().design = other.into();
            let refusal = bad.validate(&designs, 1_000.0).unwrap_err();
            assert!(refusal.contains("does not hail with"), "{refusal}");
        }
    }
}

#[cfg(all(test, feature = "serde"))]
mod serde_tests {
    use super::*;

    #[test]
    fn an_encounter_reads_from_its_authored_form_and_refuses_an_unknown_field() {
        let authored = r#"(
            id: "base/distress_call",
            weight: 3,
            player: Any,
            kind: Hail(job: Distress, line: "Mayday."),
        )"#;
        let design: EncounterDesign = ron::from_str(authored).unwrap();
        assert_eq!(
            design,
            EncounterDesign {
                id: "base/distress_call".into(),
                weight: 3,
                player: EncounterPlayerType::Any,
                kind: EncounterType::Hail {
                    job: HailJobType::Distress,
                    line: "Mayday.".to_string(),
                },
            }
        );

        let missing = r#"(id: "a", weight: 3, kind: Ambush)"#;
        assert!(ron::from_str::<EncounterDesign>(missing).is_err());
        let extra = r#"(id: "a", weight: 3, player: Any, kind: Ambush, zone: 2)"#;
        assert!(ron::from_str::<EncounterDesign>(extra).is_err());
        let extra_hail =
            r#"(id: "a", weight: 3, player: Any, kind: Hail(job: Supply, line: "x", pay: 2))"#;
        assert!(ron::from_str::<EncounterDesign>(extra_hail).is_err());
    }

    #[test]
    fn the_saved_record_refuses_an_unknown_field() {
        let saved = "(fired: 2, travel_seconds: 40.5, offer: None, live: None)";
        let record: Encounters = ron::from_str(saved).unwrap();
        assert_eq!(record.fired, 2);
        let extra = "(fired: 2, travel_seconds: 40.5, offer: None, live: None, rate: 1)";
        assert!(ron::from_str::<Encounters>(extra).is_err());
        let missing = "(travel_seconds: 40.5, offer: None, live: None)";
        assert!(ron::from_str::<Encounters>(missing).is_err());
    }
}
