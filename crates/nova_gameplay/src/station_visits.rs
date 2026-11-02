//! Station visits: which generated ship flies to which station, what it
//! traded and posted there, and when it may visit again.
//!
//! [`StationVisits`] is a session resource like
//! [`JobPostings`](crate::jobs::JobPostings): the world save carries it, a
//! Load puts it back and a New Game clears it. Only the open world crate
//! writes it. A visit is bound when a ship takes a Dock order to a station
//! ([`StationVisits::bind`]), moves to [`StationVisitPhaseType::Docked`]
//! with its receipt on the frame the dock captures ([`StationVisits::dock`])
//! and ends when the ship lets go or the visit fails
//! ([`StationVisits::end`]). The receipt is written in the same frame as
//! the transfers it lists, so a visit trades and posts at most once,
//! whatever freezes, thaws or Loads come after.

use std::{collections::BTreeMap, fmt};

use bevy::prelude::*;
use nova_events::prelude::GeneratedBodyId;

use crate::{
    inventory::prelude::{ItemDesignId, ItemTradeType},
    jobs::prelude::{JobId, JobSourceType},
    trade_profile::prelude::TradeProfileDesignId,
};

/// Glob-import surface for the station visit types.
pub mod prelude {
    pub use super::{
        StationVisit, StationVisitId, StationVisitPhaseType, StationVisitReceipt,
        StationVisitRefusalType, StationVisits, VisitTradeLine,
    };
}

/// The stable id of one visit of one ship.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct StationVisitId {
    /// The ship that visits.
    pub ship: GeneratedBodyId,
    /// The ship's visit number: zero first, one more for each bind. A number
    /// is never reused.
    pub ordinal: u32,
}

impl fmt::Display for StationVisitId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}_visit_{}", self.ship, self.ordinal)
    }
}

/// One trade intention as the visit ran it.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct VisitTradeLine {
    /// The item.
    pub item: ItemDesignId,
    /// The direction, seen from the visiting ship.
    pub side: ItemTradeType,
    /// Units that moved: zero when the intention could not trade.
    pub count: u32,
    /// Credits the buyer paid the seller: zero when nothing moved.
    pub price_cr: u32,
}

/// What a docked visit did, written on the frame of its capture.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct StationVisitReceipt {
    /// Each trade intention of the profile, in order.
    pub trades: Vec<VisitTradeLine>,
    /// The jobs the visit posted, in order. A job the station's cap
    /// skipped is not listed.
    pub jobs: Vec<JobId>,
}

/// Where a pending visit stands.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum StationVisitPhaseType {
    /// Flying to the station under a Dock order.
    Approaching,
    /// Docked at the station since `docked_at_s`; the trades and the jobs
    /// ran once then.
    Docked {
        /// The `WorldClock` play seconds at the capture.
        docked_at_s: f64,
        /// What the visit traded and posted.
        receipt: StationVisitReceipt,
    },
}

/// A ship's pending visit to a station.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct StationVisit {
    /// The id of this visit.
    pub id: StationVisitId,
    /// The station the ship visits.
    pub station: GeneratedBodyId,
    /// The trade profile the ship carried at the bind. The visit runs it
    /// even if a script changes the ship's profile on the way.
    pub profile: TradeProfileDesignId,
    /// The `WorldClock` play seconds at the bind.
    pub bound_at_s: f64,
    /// Where it stands.
    pub phase: StationVisitPhaseType,
}

/// What the table keeps for one ship.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
struct ShipVisits {
    next_ordinal: u32,
    /// The play second the ship may next bind a visit at, set when its last
    /// visit ended.
    next_visit_at_s: f64,
    /// `None` means the ship has no visit pending.
    pending: Option<StationVisit>,
}

/// The pending visit of every ship that has one, and the next visit number
/// and visit time of every ship that has bound one. A session resource: a
/// world save carries it, a Load puts it back and a New Game clears it.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct StationVisits {
    ships: BTreeMap<GeneratedBodyId, ShipVisits>,
}

impl StationVisits {
    /// Whether no ship has bound a visit.
    pub fn is_empty(&self) -> bool {
        self.ships.is_empty()
    }

    /// Forget every ship.
    pub fn clear(&mut self) {
        self.ships.clear();
    }

    /// The play second `ship` may next bind a visit at, once it bound one.
    pub fn next_visit_at(&self, ship: &GeneratedBodyId) -> Option<f64> {
        self.ships.get(ship).map(|visits| visits.next_visit_at_s)
    }

    /// Whether `visit` is a number its ship has bound: pending or ended.
    pub fn issued(&self, visit: &StationVisitId) -> bool {
        self.ships
            .get(&visit.ship)
            .is_some_and(|visits| visit.ordinal < visits.next_ordinal)
    }

    /// Bind `ship` to `station` under `profile` at `now_s`, the live
    /// `WorldClock` reading, under the ship's next visit number. The number
    /// advances only on success. A refusal changes nothing. The caller must
    /// surface a refusal.
    pub fn bind(
        &mut self,
        ship: GeneratedBodyId,
        station: GeneratedBodyId,
        profile: TradeProfileDesignId,
        now_s: f64,
    ) -> Result<StationVisitId, StationVisitRefusalType> {
        if ship == station {
            return Err(StationVisitRefusalType::SelfVisit);
        }
        if !now_s.is_finite() || now_s < 0.0 {
            return Err(StationVisitRefusalType::InvalidBindTime);
        }
        if let Some(pending) = self.pending(&ship) {
            return Err(StationVisitRefusalType::Pending(pending.id.clone()));
        }
        if let Some(visit) = self.visiting(&station) {
            return Err(StationVisitRefusalType::StationBusy(visit.id.clone()));
        }
        let ordinal = self
            .ships
            .get(&ship)
            .map_or(0, |visits| visits.next_ordinal);
        let next_ordinal = ordinal
            .checked_add(1)
            .ok_or(StationVisitRefusalType::VisitNumbersExhausted)?;
        let entry = self.ships.entry(ship.clone()).or_insert(ShipVisits {
            next_ordinal,
            next_visit_at_s: now_s,
            pending: None,
        });
        let id = StationVisitId { ship, ordinal };
        entry.next_ordinal = next_ordinal;
        entry.pending = Some(StationVisit {
            id: id.clone(),
            station,
            profile,
            bound_at_s: now_s,
            phase: StationVisitPhaseType::Approaching,
        });
        Ok(id)
    }

    /// Record that `ship`'s approaching visit docked at `now_s` and did what
    /// `receipt` lists.
    ///
    /// # Panics
    ///
    /// When `ship` has no approaching visit: the open world docks a visit
    /// once, on the frame its capture is seen.
    pub fn dock(&mut self, ship: &GeneratedBodyId, now_s: f64, receipt: StationVisitReceipt) {
        let visit = self
            .ships
            .get_mut(ship)
            .and_then(|visits| visits.pending.as_mut())
            .unwrap_or_else(|| panic!("StationVisits docks '{ship}', which has no visit"));
        assert!(
            visit.phase == StationVisitPhaseType::Approaching,
            "StationVisits docks '{}' twice",
            visit.id
        );
        visit.phase = StationVisitPhaseType::Docked {
            docked_at_s: now_s,
            receipt,
        };
    }

    /// End `ship`'s pending visit, docked or not, and let it bind its next
    /// one from `next_visit_at_s`. Gives the visit back.
    ///
    /// # Panics
    ///
    /// When `ship` has no pending visit.
    pub fn end(&mut self, ship: &GeneratedBodyId, next_visit_at_s: f64) -> StationVisit {
        let visits = self
            .ships
            .get_mut(ship)
            .unwrap_or_else(|| panic!("StationVisits ends a visit of '{ship}', which has none"));
        let visit = visits
            .pending
            .take()
            .unwrap_or_else(|| panic!("StationVisits ends a visit of '{ship}', which has none"));
        visits.next_visit_at_s = next_visit_at_s;
        visit
    }

    /// The visit `ship` has pending, if any.
    pub fn pending(&self, ship: &GeneratedBodyId) -> Option<&StationVisit> {
        self.ships.get(ship)?.pending.as_ref()
    }

    /// The pending visit to `station`, if a ship has one.
    pub fn visiting(&self, station: &GeneratedBodyId) -> Option<&StationVisit> {
        self.pending_visits()
            .find(|visit| visit.station == *station)
    }

    /// Every pending visit, in ship id order.
    pub fn pending_visits(&self) -> impl Iterator<Item = &StationVisit> {
        self.ships.values().filter_map(|ship| ship.pending.as_ref())
    }

    /// Refuse a table no session could write: an entry with no bound visit,
    /// a visit filed under another ship, a visit number the ship has not
    /// reached, a ship that visits itself, two visits to one station, a
    /// stamp that is not finite, is negative or stands ahead of the clock at
    /// `clock_seconds`, a capture before the bind, or a receipt job that is
    /// not this visit's posting at this station.
    pub fn validate(&self, clock_seconds: f64) -> Result<(), String> {
        let stamp_ok = |at: f64| at.is_finite() && at >= 0.0 && at <= clock_seconds;
        let mut stations = Vec::new();
        for (ship, visits) in &self.ships {
            if visits.next_ordinal == 0 {
                return Err(format!("the visits of '{ship}' hold no bound visit"));
            }
            if !visits.next_visit_at_s.is_finite() || visits.next_visit_at_s < 0.0 {
                return Err(format!(
                    "the visits of '{ship}' allow the next visit at {} s of play",
                    visits.next_visit_at_s
                ));
            }
            let Some(visit) = &visits.pending else {
                continue;
            };
            if visit.id.ship != *ship {
                return Err(format!(
                    "the visits of '{ship}' hold '{}', which is another ship's visit",
                    visit.id
                ));
            }
            if visit.id.ordinal >= visits.next_ordinal {
                return Err(format!(
                    "the visits of '{ship}' hold '{}' but the next visit number is {}",
                    visit.id, visits.next_ordinal
                ));
            }
            if visit.id.ship == visit.station {
                return Err(format!("the visit '{}' is to its own ship", visit.id));
            }
            if stations.contains(&&visit.station) {
                return Err(format!(
                    "the visit '{}' is the second pending visit to '{}'",
                    visit.id, visit.station
                ));
            }
            stations.push(&visit.station);
            if !stamp_ok(visit.bound_at_s) {
                return Err(format!(
                    "the visit '{}' was bound at {} s of play but the clock reads \
                     {clock_seconds} s",
                    visit.id, visit.bound_at_s
                ));
            }
            let StationVisitPhaseType::Docked {
                docked_at_s,
                receipt,
            } = &visit.phase
            else {
                continue;
            };
            if !stamp_ok(*docked_at_s) || *docked_at_s < visit.bound_at_s {
                return Err(format!(
                    "the visit '{}' docked at {docked_at_s} s of play, bound at {} s, but the \
                     clock reads {clock_seconds} s",
                    visit.id, visit.bound_at_s
                ));
            }
            for job in &receipt.jobs {
                let own = job.station == visit.station
                    && matches!(&job.source, JobSourceType::Visit { visit: posted, .. } if *posted == visit.id);
                if !own {
                    return Err(format!(
                        "the visit '{}' lists '{job}', which it did not post",
                        visit.id
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Why [`StationVisits::bind`] refused a visit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StationVisitRefusalType {
    /// The ship already has this visit pending.
    Pending(StationVisitId),
    /// Another ship has this visit to the station pending.
    StationBusy(StationVisitId),
    /// The ship and the station are one body.
    SelfVisit,
    /// The bind time is not a finite, non-negative play second.
    InvalidBindTime,
    /// The ship has used every visit number.
    VisitNumbersExhausted,
}

impl fmt::Display for StationVisitRefusalType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending(id) => {
                write!(formatter, "The ship already has the visit '{id}' pending.")
            }
            Self::StationBusy(id) => {
                write!(
                    formatter,
                    "The station already has the visit '{id}' pending."
                )
            }
            Self::SelfVisit => write!(formatter, "A ship cannot visit itself."),
            Self::InvalidBindTime => write!(
                formatter,
                "The bind time is not a finite, non-negative play second."
            ),
            Self::VisitNumbersExhausted => {
                write!(formatter, "The ship has used every visit number.")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use nova_events::prelude::SectorCoord;

    use super::*;

    fn body(stem: &str, index: u32) -> GeneratedBodyId {
        GeneratedBodyId::new(SectorCoord::new(1, 0, -2), stem, index)
    }

    fn profile() -> TradeProfileDesignId {
        TradeProfileDesignId::from("base/test")
    }

    #[test]
    fn a_visit_number_is_never_reused_and_a_station_takes_one_visitor_at_a_time() {
        let ship_a = body("ship", 0);
        let ship_b = body("ship", 1);
        let station = body("station", 0);
        let mut visits = StationVisits::default();

        let first = visits
            .bind(ship_a.clone(), station.clone(), profile(), 10.0)
            .unwrap();
        assert_eq!(first.ordinal, 0);
        assert_eq!(
            visits.bind(ship_b.clone(), station.clone(), profile(), 10.0),
            Err(StationVisitRefusalType::StationBusy(first.clone()))
        );
        let before = visits.clone();
        assert_eq!(
            visits.bind(ship_a.clone(), body("station", 1), profile(), 11.0),
            Err(StationVisitRefusalType::Pending(first.clone()))
        );
        assert_eq!(visits, before);

        visits.dock(&ship_a, 20.0, StationVisitReceipt::default());
        assert!(visits.validate(20.0).is_ok());
        let ended = visits.end(&ship_a, 400.0);
        assert_eq!(ended.id, first);
        assert_eq!(visits.next_visit_at(&ship_a), Some(400.0));

        let second = visits
            .bind(ship_a.clone(), station.clone(), profile(), 500.0)
            .unwrap();
        assert_eq!(second.ordinal, 1);
        assert_ne!(second, first);
        assert!(visits.validate(500.0).is_ok());
        assert!(visits.validate(499.0).is_err());
    }
}
