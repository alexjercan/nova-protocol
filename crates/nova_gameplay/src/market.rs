//! What a trader's price is: the item's reference price times a scarcity
//! factor times a demand factor, clamped.
//!
//! The authored `ask_cr` and `bid_cr` of an [`ItemDesign`] are its reference
//! price. A trader that carries a [`TraderMarket`] moves it off that
//! reference by the content [`PriceTable`] the merge publishes as
//! [`GamePriceTable`]:
//!
//! - scarcity reads the trader's stock of the item over its target stock:
//!   a starved trader asks and bids more, a glutted one less, and one that
//!   keeps none of the item deals at the table's unkept factor;
//! - demand is the factor of the trader's cluster type for the item's
//!   category, times the factor of its civilization's role preference: each
//!   role's rule for the category, weighted by that role's share of the
//!   preference.
//!
//! The table also states the clocks of the economy: [`RestockRule`], the
//! play time a trader's stock takes to drift back to its target after the
//! trader thaws ([`restocked_count`]), and [`JobOfferRule`], the play time
//! an unheld job offer stands.
//!
//! The rules are keyed by [`ItemCategoryType`], never by item, so a mod item
//! with a category and a reference price prices itself. A pair with no rule
//! has a factor of one. A trader with no [`TraderMarket`] (an authored
//! scenario's ship or station, which has no cluster and no civilization)
//! deals at the reference.
//!
//! Bid never passes ask: [`price_table_faults`] refuses every factor whose
//! bid is above its ask, every factor is positive, and the clamp and the
//! rounding are monotone, so a product of such factors keeps a reference
//! bid at most its ask at most the priced ask.

use std::collections::BTreeMap;

use bevy::prelude::*;

use crate::{
    chart::prelude::ClusterType,
    civilization::prelude::{
        CivilizationId, CivilizationStances, StanceType, ENEMY_DECAY_SECONDS,
        PROVOKED_WINDOW_SECONDS,
    },
    inventory::prelude::{ItemCategoryType, ItemDesign, ItemDesignId},
};

/// Glob-import surface: `use nova_gameplay::market::prelude::*`.
pub mod prelude {
    pub use super::{
        delta_percent, market_summary, price, price_table_faults, refusal, restock_gap,
        restocked_count, ClusterPriceRule, GamePriceTable, ItemPrice, JobOfferRule, MarketLead,
        MarketSummary, PriceClamp, PriceFactor, PriceQuote, PriceTable, Refusal, RestockGap,
        RestockRule, RolePriceRule, ScarcityRule, ShipRoleType, TraderMarket, MARKET_LEADS,
    };
}

/// The four closed ship roles a civilization fields.
///
/// Here rather than beside the generator so a [`RolePriceRule`] can name a
/// role without the world crates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ShipRoleType {
    /// Unarmed traffic.
    Civilian,
    /// Unarmed miners with cargo intake.
    Industrial,
    /// Rough, low-tier fighting ships.
    Scavenger,
    /// Equipped fighting ships.
    Armored,
}

impl ShipRoleType {
    /// Every role, in the order a role array holds them.
    pub const ALL: [Self; 4] = [
        Self::Civilian,
        Self::Industrial,
        Self::Scavenger,
        Self::Armored,
    ];

    /// What a readout or a legend calls the role.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Civilian => "civilian",
            Self::Industrial => "industrial",
            Self::Scavenger => "scavenger",
            Self::Armored => "armored",
        }
    }
}

/// A multiplier on each side of a price: what a Buy pays and what a Sell
/// earns.
#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct PriceFactor {
    /// The multiplier on the ask.
    pub ask: f32,
    /// The multiplier on the bid. At most `ask`.
    pub bid: f32,
}

impl PriceFactor {
    /// The factor that leaves a price where it is.
    pub const ONE: Self = Self { ask: 1.0, bid: 1.0 };

    fn times(self, other: Self) -> Self {
        Self {
            ask: self.ask * other.ask,
            bid: self.bid * other.bid,
        }
    }

    fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            ask: self.ask + (other.ask - self.ask) * t,
            bid: self.bid + (other.bid - self.bid) * t,
        }
    }
}

/// The bounds the product of every factor is held to, on each side.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct PriceClamp {
    /// The lowest total multiplier. Above zero, at most one.
    pub min: f32,
    /// The highest total multiplier. At least one.
    pub max: f32,
}

/// How a trader's stock over its target stock moves its price.
///
/// At no stock the factor is `starved`; at the target it is one; at
/// `glut_share` times the target and past it, `glut`; linear between. A
/// trader whose target holds none of the item deals at `unkept` whatever
/// it holds.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct ScarcityRule {
    /// The factor at no stock. Each side at least one.
    pub starved: PriceFactor,
    /// The factor at `glut_share` times the target and past it. Each side
    /// at most one.
    pub glut: PriceFactor,
    /// The stock over the target where the glut factor is reached. Above one.
    pub glut_share: f32,
    /// The factor for an item the trader keeps none of.
    pub unkept: PriceFactor,
}

/// How fast a thawed trader's stock drifts back to its target.
///
/// A trader's stock does not change while its cell stands frozen; on the
/// thaw the play time it stood frozen is spent at once: each targeted item
/// moves toward its target by `target * elapsed / recovery_seconds` units,
/// never past it, see [`restocked_count`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct RestockRule {
    /// The play seconds an item takes to return to its target from none.
    /// Finite, above zero.
    pub recovery_seconds: f32,
}

/// How long a station's job offer stands before the slot posts a new one.
///
/// An offer the player does not hold expires after the window for its kind,
/// measured from its posting stamp (`nova_gameplay::jobs::SlotPosting`);
/// a held job never expires.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct JobOfferRule {
    /// The play seconds an unheld non-escort offer stands. Finite, above zero.
    pub offer_seconds: f32,
    /// The play seconds an unheld escort offer stands. Finite, above zero.
    pub escort_offer_seconds: f32,
}

/// The factor a trader in a cluster of one type deals one category at.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct ClusterPriceRule {
    /// The cluster type the rule applies in.
    pub cluster: ClusterType,
    /// The item category it prices.
    pub category: ItemCategoryType,
    /// The factor.
    pub factor: PriceFactor,
}

/// The factor a civilization that fields only one role deals one category
/// at. A civilization's factor weighs each role's by its preference share.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct RolePriceRule {
    /// The role.
    pub role: ShipRoleType,
    /// The item category it prices.
    pub category: ItemCategoryType,
    /// The factor.
    pub factor: PriceFactor,
}

/// The authored price rules every trader with a [`TraderMarket`] deals by.
///
/// Content: the base bundle states one, and a mod's table replaces it whole.
/// [`price_table_faults`] is the rule the merge and the lint share.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct PriceTable {
    /// The bounds of the total factor.
    pub clamp: PriceClamp,
    /// The stock rule.
    pub scarcity: ScarcityRule,
    /// The restock clock.
    pub restock: RestockRule,
    /// The job offer windows.
    pub jobs: JobOfferRule,
    /// The cluster-type rules, at most one per cluster type and category.
    pub clusters: Vec<ClusterPriceRule>,
    /// The role rules, at most one per role and category.
    pub roles: Vec<RolePriceRule>,
}

impl PriceTable {
    fn cluster_factor(&self, cluster: ClusterType, category: ItemCategoryType) -> PriceFactor {
        self.clusters
            .iter()
            .find(|rule| rule.cluster == cluster && rule.category == category)
            .map_or(PriceFactor::ONE, |rule| rule.factor)
    }

    fn role_factor(&self, role: ShipRoleType, category: ItemCategoryType) -> PriceFactor {
        self.roles
            .iter()
            .find(|rule| rule.role == role && rule.category == category)
            .map_or(PriceFactor::ONE, |rule| rule.factor)
    }

    fn scarcity_factor(&self, stock: u32, target: u32) -> PriceFactor {
        let rule = &self.scarcity;
        if target == 0 {
            return rule.unkept;
        }
        let share = stock as f32 / target as f32;
        if share <= 1.0 {
            rule.starved.lerp(PriceFactor::ONE, share)
        } else {
            let t = ((share - 1.0) / (rule.glut_share - 1.0)).min(1.0);
            PriceFactor::ONE.lerp(rule.glut, t)
        }
    }
}

/// The merged [`PriceTable`]: the base bundle's, or the last loaded mod's.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct GamePriceTable(pub PriceTable);

/// What a generated trader prices by: where it trades, who it trades for,
/// and the stock it keeps.
///
/// The open-world generator puts it on a living station and an intact ship
/// root; the frozen record carries it, so it crosses a retire, a thaw and a
/// Load with the body.
#[derive(Component, Clone, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct TraderMarket {
    /// The type of the cluster the trader was generated in.
    pub cluster: ClusterType,
    /// The civilization the trader belongs to: the one the player offends
    /// by neutralizing this hull.
    pub civilization: CivilizationId,
    /// Its civilization's preference for each role, in
    /// [`ShipRoleType::ALL`] order. Each at least zero, their sum above
    /// zero.
    pub role_preference: [f32; 4],
    /// The stock it keeps: what it spawned holding. An item it names
    /// nowhere is one it keeps none of.
    pub target: BTreeMap<ItemDesignId, u32>,
}

impl TraderMarket {
    /// The role with the highest preference, the first in
    /// [`ShipRoleType::ALL`] order on a tie.
    pub fn leading_role(&self) -> ShipRoleType {
        let mut leading = 0;
        for (index, preference) in self.role_preference.iter().enumerate() {
            if *preference > self.role_preference[leading] {
                leading = index;
            }
        }
        ShipRoleType::ALL[leading]
    }
}

/// What one item trades at: a Buy pays `ask_cr`, a Sell earns `bid_cr`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub struct ItemPrice {
    /// Credits a Buy pays for one.
    pub ask_cr: u32,
    /// Credits a Sell earns for one. At most `ask_cr`.
    pub bid_cr: u32,
}

impl ItemDesign {
    /// The authored reference price.
    pub fn reference_price(&self) -> ItemPrice {
        ItemPrice {
            ask_cr: self.ask_cr,
            bid_cr: self.bid_cr,
        }
    }
}

/// A trader's price for one item and the factors that made it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PriceQuote {
    /// The price a trade pays.
    pub price: ItemPrice,
    /// The stock factor.
    pub scarcity: PriceFactor,
    /// The cluster-type factor.
    pub cluster: PriceFactor,
    /// The civilization's role-preference factor.
    pub civilization: PriceFactor,
    /// The product of the three, after the clamp.
    pub total: PriceFactor,
    /// Whether the clamp moved either side.
    pub clamped: bool,
}

impl PriceQuote {
    /// The authored reference price with every factor one: what a trader
    /// without a [`TraderMarket`] deals at.
    pub fn reference(design: &ItemDesign) -> Self {
        Self {
            price: design.reference_price(),
            scarcity: PriceFactor::ONE,
            cluster: PriceFactor::ONE,
            civilization: PriceFactor::ONE,
            total: PriceFactor::ONE,
            clamped: false,
        }
    }
}

/// Price one `design` at a trader with `market` that holds `stock` of it:
/// the reference times scarcity times the cluster factor times the
/// civilization factor, each side clamped by `table`, rounded to the
/// nearest credit.
///
/// # Panics
///
/// On a market whose role preferences sum to zero or less: the generator
/// draws civilian and industrial preference above zero.
pub fn price(
    table: &PriceTable,
    design: &ItemDesign,
    market: &TraderMarket,
    stock: u32,
) -> PriceQuote {
    let target = market.target.get(&design.id).copied().unwrap_or(0);
    let scarcity = table.scarcity_factor(stock, target);
    let cluster = table.cluster_factor(market.cluster, design.category);
    let sum: f32 = market.role_preference.iter().sum();
    assert!(
        sum > 0.0,
        "price: a market's role preferences {:?} sum to {sum}, not above zero",
        market.role_preference
    );
    let civilization = ShipRoleType::ALL.iter().zip(market.role_preference).fold(
        PriceFactor { ask: 0.0, bid: 0.0 },
        |acc, (role, preference)| {
            let factor = table.role_factor(*role, design.category);
            let share = preference / sum;
            PriceFactor {
                ask: acc.ask + factor.ask * share,
                bid: acc.bid + factor.bid * share,
            }
        },
    );
    let raw = scarcity.times(cluster).times(civilization);
    let clamp = table.clamp;
    let total = PriceFactor {
        ask: raw.ask.clamp(clamp.min, clamp.max),
        bid: raw.bid.clamp(clamp.min, clamp.max),
    };
    let priced =
        |reference: u32, factor: f32| (f64::from(reference) * f64::from(factor)).round() as u32;
    PriceQuote {
        price: ItemPrice {
            ask_cr: priced(design.ask_cr, total.ask),
            bid_cr: priced(design.bid_cr, total.bid),
        },
        scarcity,
        cluster,
        civilization,
        total,
        clamped: total != raw,
    }
}

/// The stock of one item after a trader stood frozen `elapsed_seconds` of
/// play with `stock` of it and a target of `target`: the stock moved toward
/// the target by `target * elapsed / recovery` whole units, never past it.
/// A stock at its target is unchanged; a target of zero (an item the trader
/// keeps none of) moves nothing, so what the player sold it stays at the
/// unkept price.
pub fn restocked_count(stock: u32, target: u32, elapsed_seconds: f64, rule: RestockRule) -> u32 {
    let drift = (f64::from(target) * elapsed_seconds / f64::from(rule.recovery_seconds)).floor();
    // Past u32 the drift covers any gap, so the clamp alone decides.
    let drift = if drift >= f64::from(u32::MAX) {
        u32::MAX
    } else {
        drift as u32
    };
    if stock < target {
        stock.saturating_add(drift).min(target)
    } else {
        stock.saturating_sub(drift).max(target)
    }
}

/// The signed percent `price_cr` stands from `reference_cr`, rounded toward
/// zero: `-18` for a 33 cr bid on a 40 cr reference, `+40` for a 126 cr bid
/// on a 90 cr one. A reference of zero has no percent and reads as zero.
pub fn delta_percent(price_cr: u32, reference_cr: u32) -> i32 {
    if reference_cr == 0 {
        return 0;
    }
    let delta = i64::from(price_cr) - i64::from(reference_cr);
    i32::try_from(delta * 100 / i64::from(reference_cr)).expect("a percent of a u32 price fits")
}

/// How many items each lead of a [`MarketSummary`] lists.
pub const MARKET_LEADS: usize = 3;

/// One item of a market lead: what the trader deals it at and how far that
/// stands from the item's reference.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarketLead {
    /// The item.
    pub item: ItemDesignId,
    /// The bid a Sell earns, or the ask a Buy pays, in credits.
    pub price_cr: u32,
    /// `price_cr` against the reference on the same side, by
    /// [`delta_percent`].
    pub delta_percent: i32,
}

/// The Market strip's leads at one trader: the items it pays the most over
/// their reference for and the stocked items it sells the furthest under
/// theirs, at most [`MARKET_LEADS`] each.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MarketSummary {
    /// Every catalog item by its bid against the reference bid, highest
    /// first; a tie keeps the lower id first.
    pub pays_most: Vec<MarketLead>,
    /// Every item the trader holds by its ask against the reference ask,
    /// lowest first; a tie keeps the lower id first.
    pub sells_cheapest: Vec<MarketLead>,
}

/// The [`MarketSummary`] of a trader with `market` over `designs`, each
/// priced by [`price`] at the `stock` the trader holds of it. The trader
/// buys any catalog item, so every design competes for `pays_most`; it
/// sells only what it holds, so `sells_cheapest` reads stocked designs.
pub fn market_summary<'a>(
    table: &PriceTable,
    market: &TraderMarket,
    designs: impl IntoIterator<Item = &'a ItemDesign>,
    stock: impl Fn(&ItemDesignId) -> u32,
) -> MarketSummary {
    let mut pays = Vec::new();
    let mut sells = Vec::new();
    for design in designs {
        let held = stock(&design.id);
        let quote = price(table, design, market, held);
        pays.push(MarketLead {
            item: design.id.clone(),
            price_cr: quote.price.bid_cr,
            delta_percent: delta_percent(quote.price.bid_cr, design.bid_cr),
        });
        if held > 0 {
            sells.push(MarketLead {
                item: design.id.clone(),
                price_cr: quote.price.ask_cr,
                delta_percent: delta_percent(quote.price.ask_cr, design.ask_cr),
            });
        }
    }
    pays.sort_by(|a, b| {
        b.delta_percent
            .cmp(&a.delta_percent)
            .then_with(|| a.item.cmp(&b.item))
    });
    sells.sort_by(|a, b| {
        a.delta_percent
            .cmp(&b.delta_percent)
            .then_with(|| a.item.cmp(&b.item))
    });
    pays.truncate(MARKET_LEADS);
    sells.truncate(MARKET_LEADS);
    MarketSummary {
        pays_most: pays,
        sells_cheapest: sells,
    }
}

/// The Market strip's restock meter: the targeted item a trader is
/// furthest under its target on, and the play time away it takes to fill.
#[derive(Clone, Debug, PartialEq)]
pub struct RestockGap {
    /// The item.
    pub item: ItemDesignId,
    /// What the trader holds of it.
    pub stock: u32,
    /// What it keeps, from its [`TraderMarket::target`]. Above `stock`.
    pub target: u32,
    /// The play seconds its cell must stand frozen for the stock to reach
    /// the target, by the table's [`RestockRule`]; a trader restocks only
    /// on a thaw.
    pub seconds_away: f64,
}

/// The [`RestockGap`] of a trader with `market` at the `stock` it holds:
/// the targeted item with the lowest stock over target, the lower id on a
/// tie, or `None` while every targeted item is at or past its target.
pub fn restock_gap(
    table: &PriceTable,
    market: &TraderMarket,
    stock: impl Fn(&ItemDesignId) -> u32,
) -> Option<RestockGap> {
    let mut gap: Option<(f64, RestockGap)> = None;
    for (item, &target) in &market.target {
        let held = stock(item);
        if target == 0 || held >= target {
            continue;
        }
        let share = f64::from(held) / f64::from(target);
        if gap.as_ref().is_none_or(|(lowest, _)| share < *lowest) {
            gap = Some((
                share,
                RestockGap {
                    item: item.clone(),
                    stock: held,
                    target,
                    seconds_away: (1.0 - share) * f64::from(table.restock.recovery_seconds),
                },
            ));
        }
    }
    gap.map(|(_, gap)| gap)
}

/// Why a trader's civilization refuses the player, for the Market strip's
/// REFUSED badge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Refusal {
    /// The play seconds until the refusal ends on its own: the rest of a
    /// provocation's window, or of an Enemy by events' decay. `None` for a
    /// civilization Enemy by the seed, which only a settlement moves.
    pub seconds_left: Option<f64>,
}

/// Whether `civilization`, whose seed drew `seeded`, refuses the player at
/// `now`: its effective stance is Enemy. A provocation refuses for the rest
/// of [`PROVOKED_WINDOW_SECONDS`], an Enemy by events for the rest of
/// [`ENEMY_DECAY_SECONDS`] since it turned, and a seeded Enemy with no end.
pub fn refusal(
    stances: &CivilizationStances,
    civilization: &CivilizationId,
    seeded: StanceType,
    now: f64,
) -> Option<Refusal> {
    if stances.effective(civilization, seeded, now) != StanceType::Enemy {
        return None;
    }
    let record = stances.record(civilization);
    let provoked = record
        .and_then(|record| record.provoked_at)
        .filter(|at| now - at < PROVOKED_WINDOW_SECONDS)
        .map(|at| PROVOKED_WINDOW_SECONDS - (now - at));
    let decays = record
        .and_then(|record| record.enemy_since)
        .map(|since| (ENEMY_DECAY_SECONDS - (now - since)).max(0.0));
    Some(Refusal {
        seconds_left: provoked.or(decays),
    })
}

/// What is wrong with `table`, one line per fault; empty when the merge and
/// the lint accept it.
///
/// A clamp must hold one, with a minimum above zero. Every factor must be
/// finite, inside the clamp, and bid at most ask, so no rule can push a bid
/// above its ask. A starved factor may not lower a price and a glut factor
/// may not raise one. The glut share must be above one. The restock
/// recovery and the refusal window must be finite seconds above zero. A
/// cluster type or a role may name a category once.
pub fn price_table_faults(table: &PriceTable) -> Vec<String> {
    let mut faults = Vec::new();
    let PriceClamp { min, max } = table.clamp;
    if !(min.is_finite() && max.is_finite() && min > 0.0 && min <= 1.0 && max >= 1.0) {
        faults.push(format!(
            "the clamp {min}..{max} must be finite, hold 1 and start above 0"
        ));
    }
    let mut factor = |what: String, factor: PriceFactor| {
        let PriceFactor { ask, bid } = factor;
        if !(ask.is_finite() && bid.is_finite()) || ask < min || ask > max || bid < min || bid > max
        {
            faults.push(format!(
                "{what} has ask x{ask} and bid x{bid}, outside the clamp {min}..{max}"
            ));
        }
        if bid > ask {
            faults.push(format!(
                "{what} has a bid of x{bid} above its ask of x{ask}: it can push a bid above \
                 the ask"
            ));
        }
    };
    let scarcity = table.scarcity;
    factor("the starved factor".to_string(), scarcity.starved);
    factor("the glut factor".to_string(), scarcity.glut);
    factor("the unkept factor".to_string(), scarcity.unkept);
    for rule in &table.clusters {
        factor(
            format!("the {} rule for {:?}", rule.cluster.label(), rule.category),
            rule.factor,
        );
    }
    for rule in &table.roles {
        factor(
            format!("the {} rule for {:?}", rule.role.label(), rule.category),
            rule.factor,
        );
    }
    if scarcity.starved.ask < 1.0 || scarcity.starved.bid < 1.0 {
        faults.push(format!(
            "the starved factor {:?} lowers a price: a starved trader may not deal below its \
             reference",
            scarcity.starved
        ));
    }
    if scarcity.glut.ask > 1.0 || scarcity.glut.bid > 1.0 {
        faults.push(format!(
            "the glut factor {:?} raises a price: a glutted trader may not deal above its \
             reference",
            scarcity.glut
        ));
    }
    if !(scarcity.glut_share.is_finite() && scarcity.glut_share > 1.0) {
        faults.push(format!(
            "the glut share {} must be above 1",
            scarcity.glut_share
        ));
    }
    let recovery = table.restock.recovery_seconds;
    if !(recovery.is_finite() && recovery > 0.0) {
        faults.push(format!(
            "the restock recovery of {recovery} s must be finite and above 0"
        ));
    }
    let offer = table.jobs.offer_seconds;
    if !(offer.is_finite() && offer > 0.0) {
        faults.push(format!(
            "the job offer window of {offer} s must be finite and above 0"
        ));
    }
    let escort_offer = table.jobs.escort_offer_seconds;
    if !(escort_offer.is_finite() && escort_offer > 0.0) {
        faults.push(format!(
            "the escort job offer window of {escort_offer} s must be finite and above 0"
        ));
    }
    for (index, rule) in table.clusters.iter().enumerate() {
        if table.clusters[..index]
            .iter()
            .any(|other| other.cluster == rule.cluster && other.category == rule.category)
        {
            faults.push(format!(
                "the {} rule for {:?} is stated twice",
                rule.cluster.label(),
                rule.category
            ));
        }
    }
    for (index, rule) in table.roles.iter().enumerate() {
        if table.roles[..index]
            .iter()
            .any(|other| other.role == rule.role && other.category == rule.category)
        {
            faults.push(format!(
                "the {} rule for {:?} is stated twice",
                rule.role.label(),
                rule.category
            ));
        }
    }
    faults
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn factor(ask: f32, bid: f32) -> PriceFactor {
        PriceFactor { ask, bid }
    }

    #[track_caller]
    fn assert_near(actual: PriceFactor, expected: PriceFactor) {
        assert!(
            (actual.ask - expected.ask).abs() < 1e-5 && (actual.bid - expected.bid).abs() < 1e-5,
            "{actual:?} is not {expected:?}"
        );
    }

    fn table() -> PriceTable {
        PriceTable {
            clamp: PriceClamp {
                min: 0.25,
                max: 3.0,
            },
            scarcity: ScarcityRule {
                starved: factor(1.5, 1.25),
                glut: factor(0.9, 0.5),
                glut_share: 3.0,
                unkept: factor(1.0, 0.8),
            },
            restock: RestockRule {
                recovery_seconds: 1_000.0,
            },
            jobs: JobOfferRule {
                offer_seconds: 1_200.0,
                escort_offer_seconds: 3_600.0,
            },
            clusters: vec![
                ClusterPriceRule {
                    cluster: ClusterType::AsteroidRich,
                    category: ItemCategoryType::Raw,
                    factor: factor(0.5, 0.5),
                },
                ClusterPriceRule {
                    cluster: ClusterType::PlanetHeavy,
                    category: ItemCategoryType::Raw,
                    factor: factor(2.0, 2.0),
                },
            ],
            roles: vec![RolePriceRule {
                role: ShipRoleType::Armored,
                category: ItemCategoryType::Ammo,
                factor: factor(2.0, 1.5),
            }],
        }
    }

    fn design(id: &str, category: ItemCategoryType, ask_cr: u32, bid_cr: u32) -> ItemDesign {
        ItemDesign {
            id: id.into(),
            name: id.to_string(),
            about: String::new(),
            category,
            mass_g: 1_000,
            ask_cr,
            bid_cr,
        }
    }

    fn market(cluster: ClusterType, target: &[(&str, u32)]) -> TraderMarket {
        TraderMarket {
            cluster,
            civilization: CivilizationId {
                world_seed: 1,
                node: [0, 0, 0],
            },
            role_preference: [1.0, 1.0, 0.0, 0.0],
            target: target
                .iter()
                .map(|(id, count)| ((*id).into(), *count))
                .collect(),
        }
    }

    #[test]
    fn a_trader_at_its_target_with_no_rule_deals_at_the_reference() {
        let ore = design("ore", ItemCategoryType::Food, 40, 30);
        let quote = price(
            &table(),
            &ore,
            &market(ClusterType::RockOnly, &[("ore", 10)]),
            10,
        );
        assert_eq!(quote.price, ore.reference_price());
        assert_near(quote.total, PriceFactor::ONE);
        assert!(!quote.clamped);
    }

    #[test]
    fn scarcity_runs_from_starved_through_one_to_glut() {
        let table = table();
        let item = design("food", ItemCategoryType::Food, 100, 80);
        let at = |stock| {
            price(
                &table,
                &item,
                &market(ClusterType::RockOnly, &[("food", 10)]),
                stock,
            )
        };
        assert_near(at(0).scarcity, factor(1.5, 1.25));
        assert_eq!(
            at(0).price,
            ItemPrice {
                ask_cr: 150,
                bid_cr: 100
            }
        );
        assert_near(at(5).scarcity, factor(1.25, 1.125));
        assert_near(at(10).scarcity, PriceFactor::ONE);
        assert_near(at(20).scarcity, factor(0.95, 0.75));
        assert_near(at(30).scarcity, factor(0.9, 0.5));
        assert_near(at(300).scarcity, factor(0.9, 0.5));
        assert_eq!(
            at(300).price,
            ItemPrice {
                ask_cr: 90,
                bid_cr: 40
            }
        );
    }

    #[test]
    fn an_item_the_trader_keeps_none_of_deals_at_the_unkept_factor() {
        let item = design("food", ItemCategoryType::Food, 100, 80);
        for stock in [0, 1, 500] {
            let quote = price(&table(), &item, &market(ClusterType::RockOnly, &[]), stock);
            assert_near(quote.scarcity, factor(1.0, 0.8));
            assert_eq!(
                quote.price,
                ItemPrice {
                    ask_cr: 100,
                    bid_cr: 64
                }
            );
        }
    }

    #[test]
    fn ore_is_cheap_in_an_asteroid_rich_cluster_and_dear_in_a_planet_heavy_one() {
        let ore = design("IronOre", ItemCategoryType::Raw, 16, 12);
        let rich = price(&table(), &ore, &market(ClusterType::AsteroidRich, &[]), 0);
        let heavy = price(&table(), &ore, &market(ClusterType::PlanetHeavy, &[]), 0);
        assert_near(rich.cluster, factor(0.5, 0.5));
        assert_near(heavy.cluster, factor(2.0, 2.0));
        assert_eq!(
            rich.price,
            ItemPrice {
                ask_cr: 8,
                bid_cr: 5
            }
        );
        assert_eq!(
            heavy.price,
            ItemPrice {
                ask_cr: 32,
                bid_cr: 19
            }
        );
    }

    #[test]
    fn a_civilization_weighs_each_role_rule_by_its_preference_share() {
        let rounds = design("rounds", ItemCategoryType::Ammo, 100, 60);
        let mut armed = market(ClusterType::RockOnly, &[("rounds", 10)]);
        armed.role_preference = [1.0, 0.0, 0.0, 3.0];
        let quote = price(&table(), &rounds, &armed, 10);
        assert_near(quote.civilization, factor(1.75, 1.375));
        assert_eq!(
            quote.price,
            ItemPrice {
                ask_cr: 175,
                bid_cr: 83
            }
        );
        assert_eq!(armed.leading_role(), ShipRoleType::Armored);
    }

    #[test]
    fn the_clamp_holds_the_total_factor() {
        let mut table = table();
        table.clamp = PriceClamp { min: 0.3, max: 2.5 };
        let ore = design("ore", ItemCategoryType::Raw, 100, 100);
        let quote = price(
            &table,
            &ore,
            &market(ClusterType::PlanetHeavy, &[("ore", 10)]),
            0,
        );
        assert_near(quote.total, factor(2.5, 2.5));
        assert_eq!(
            quote.price,
            ItemPrice {
                ask_cr: 250,
                bid_cr: 250
            }
        );
        assert!(quote.clamped);
        let low = price(
            &table,
            &ore,
            &market(ClusterType::AsteroidRich, &[("ore", 1)]),
            90,
        );
        assert_near(low.total, factor(0.45, 0.3));
        assert!(low.clamped);
    }

    #[test]
    fn a_sound_table_has_no_faults() {
        assert_eq!(price_table_faults(&table()), Vec::<String>::new());
    }

    #[test]
    fn a_rule_whose_bid_factor_passes_its_ask_factor_is_refused() {
        let mut table = table();
        table.clusters[0].factor = factor(1.0, 1.4);
        let faults = price_table_faults(&table);
        assert_eq!(faults.len(), 1, "{faults:?}");
        assert!(faults[0].contains("above its ask"), "{faults:?}");
    }

    #[test]
    fn a_factor_outside_the_clamp_and_a_twice_stated_rule_are_refused() {
        let mut table = table();
        table.roles[0].factor = factor(4.0, 1.0);
        table.clusters.push(table.clusters[1]);
        let faults = price_table_faults(&table);
        assert_eq!(faults.len(), 2, "{faults:?}");
        assert!(faults[0].contains("outside the clamp"), "{faults:?}");
        assert!(faults[1].contains("stated twice"), "{faults:?}");
    }

    #[test]
    fn a_starved_factor_below_one_and_a_glut_share_of_one_are_refused() {
        let mut table = table();
        table.scarcity.starved = factor(0.9, 0.9);
        table.scarcity.glut_share = 1.0;
        let faults = price_table_faults(&table);
        assert_eq!(faults.len(), 2, "{faults:?}");
    }
    /// Restock spends the frozen play time at once: from none, half the
    /// recovery gives half the target; past the recovery the stock is at
    /// the target and no further; a glut falls at the same rate and stops
    /// at the target; an item the target does not keep never moves.
    #[test]
    fn a_thawed_stock_drifts_toward_its_target_by_the_elapsed_play_time() {
        let rule = RestockRule {
            recovery_seconds: 100.0,
        };
        assert_eq!(
            restocked_count(0, 10, 50.0, rule),
            5,
            "half the recovery, half the target"
        );
        assert_eq!(
            restocked_count(0, 10, 100.0, rule),
            10,
            "the whole recovery"
        );
        assert_eq!(
            restocked_count(0, 10, 1_000.0, rule),
            10,
            "never past the target"
        );
        assert_eq!(
            restocked_count(3, 10, 0.0, rule),
            3,
            "no play time, no drift"
        );
        assert_eq!(
            restocked_count(3, 10, 25.0, rule),
            5,
            "whole units, rounded down"
        );
        assert_eq!(restocked_count(30, 10, 50.0, rule), 25, "a glut sells down");
        assert_eq!(
            restocked_count(30, 10, 500.0, rule),
            10,
            "a glut stops at the target"
        );
        assert_eq!(
            restocked_count(7, 0, 1_000.0, rule),
            7,
            "an unkept item stays"
        );
        assert_eq!(
            restocked_count(10, 10, 1_000.0, rule),
            10,
            "a met target is unchanged"
        );
    }

    /// A recovery or either offer window of zero, below zero or not a number
    /// is a fault the lint and the merge name.
    #[test]
    fn the_restock_and_offer_clocks_must_be_positive_seconds() {
        let mut table = table();
        assert_eq!(price_table_faults(&table), Vec::<String>::new());
        table.restock.recovery_seconds = 0.0;
        table.jobs.offer_seconds = -1.0;
        table.jobs.escort_offer_seconds = f32::NAN;
        let faults = price_table_faults(&table);
        assert_eq!(faults.len(), 3, "{faults:?}");
        assert!(faults[0].contains("restock recovery"), "{faults:?}");
        assert!(faults[1].contains("job offer window"), "{faults:?}");
        assert!(faults[2].contains("escort job offer window"), "{faults:?}");
    }

    /// The percent is signed, truncated toward zero, and a zero reference
    /// reads as zero rather than dividing by it.
    #[test]
    fn delta_percent_is_the_signed_truncated_percent_off_the_reference() {
        assert_eq!(delta_percent(33, 40), -17);
        assert_eq!(delta_percent(126, 90), 40);
        assert_eq!(delta_percent(40, 40), 0);
        assert_eq!(delta_percent(1, 3), -66);
        assert_eq!(delta_percent(5, 0), 0);
    }

    /// Pays most ranks every catalog item by its bid over the reference bid,
    /// highest first; sells cheapest ranks only stocked items by their ask
    /// under the reference ask, lowest first; both stop at three.
    #[test]
    fn the_market_summary_leads_with_the_best_bid_and_the_cheapest_stocked_ask() {
        let table = table();
        let designs = [
            design("ore", ItemCategoryType::Raw, 16, 12),
            design("food", ItemCategoryType::Food, 100, 80),
            design("rounds", ItemCategoryType::Ammo, 100, 60),
            design("plate", ItemCategoryType::Repair, 40, 30),
            design("slug", ItemCategoryType::Ammo, 40, 30),
        ];
        let mut market = market(ClusterType::AsteroidRich, &[("food", 10), ("plate", 10)]);
        market.role_preference = [0.0, 0.0, 0.0, 1.0];
        let stock = |item: &ItemDesignId| match item.as_str() {
            "ore" => 5,
            "food" => 0,
            "plate" => 10,
            _ => 0,
        };
        let summary = market_summary(&table, &market, &designs, stock);
        let leads = |leads: &[MarketLead]| {
            leads
                .iter()
                .map(|lead| (lead.item.to_string(), lead.price_cr, lead.delta_percent))
                .collect::<Vec<_>>()
        };
        // Starved food bids x1.25; armored pays x1.5 on the unkept ammo
        // (x0.8), so rounds and slug tie at +20 and keep id order; ore and
        // plate fall off the three.
        assert_eq!(
            leads(&summary.pays_most),
            [
                ("food".to_string(), 100, 25),
                ("rounds".to_string(), 72, 20),
                ("slug".to_string(), 36, 20),
            ]
        );
        // Only ore and plate are stocked: ore asks half in the belt, plate at
        // its target asks the reference.
        assert_eq!(
            leads(&summary.sells_cheapest),
            [("ore".to_string(), 8, -50), ("plate".to_string(), 40, 0)]
        );
    }

    /// The gap names the targeted item with the lowest share of its target,
    /// its fill time is that share's rest of the recovery, an unkept or a
    /// glutted item never shows, and a trader at target has no gap.
    #[test]
    fn the_restock_gap_is_the_most_starved_targeted_item() {
        let table = table();
        let market = market(
            ClusterType::RockOnly,
            &[("food", 10), ("ore", 20), ("plate", 4)],
        );
        let stock = |item: &ItemDesignId| match item.as_str() {
            "food" => 5,
            "ore" => 5,
            "plate" => 9,
            _ => 100,
        };
        let gap = restock_gap(&table, &market, stock).expect("ore is starved");
        assert_eq!((gap.item.as_str(), gap.stock, gap.target), ("ore", 5, 20));
        assert!(
            (gap.seconds_away - 750.0).abs() < 1e-9,
            "{}",
            gap.seconds_away
        );
        let full = restock_gap(&table, &market, |item: &ItemDesignId| match item.as_str() {
            "plate" => 0,
            _ => 50,
        });
        assert_eq!(
            full.map(|gap| gap.item.to_string()),
            Some("plate".to_string())
        );
        assert_eq!(restock_gap(&table, &market, |_| 100), None);
    }

    /// A provocation refuses for the rest of its window, an Enemy by events
    /// for the rest of its decay, a seeded Enemy with no end, and a Neutral
    /// civilization does not refuse.
    #[test]
    fn a_refusal_counts_down_its_window_or_its_decay() {
        let id = CivilizationId {
            world_seed: 1,
            node: [0, 0, 0],
        };
        let mut stances = CivilizationStances::default();
        assert_eq!(refusal(&stances, &id, StanceType::Neutral, 10.0), None);
        assert_eq!(
            refusal(&stances, &id, StanceType::Enemy, 10.0),
            Some(Refusal { seconds_left: None })
        );
        stances.provoke(id, StanceType::Neutral, 100.0);
        let provoked = refusal(&stances, &id, StanceType::Neutral, 160.0).expect("provoked");
        assert_eq!(provoked.seconds_left, Some(PROVOKED_WINDOW_SECONDS - 60.0));
        assert_eq!(
            refusal(
                &stances,
                &id,
                StanceType::Neutral,
                100.0 + PROVOKED_WINDOW_SECONDS
            ),
            None,
            "the window has passed"
        );
        let mut stances = CivilizationStances::default();
        for _ in 0..3 {
            stances.offence(id, StanceType::Neutral, 200.0);
        }
        let enemy = refusal(&stances, &id, StanceType::Neutral, 500.0).expect("enemy by events");
        assert_eq!(enemy.seconds_left, Some(ENEMY_DECAY_SECONDS - 300.0));
    }
}
