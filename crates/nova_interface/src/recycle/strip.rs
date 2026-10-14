//! The Recycle pane's whole-hull strip: its block, its command and its timer.
//!
//! While the player ship is docked with a partner it may loot (neutralized,
//! or a derelict carrying [`LootableShipMarker`]) that is not a living
//! station, the block shows the parts the whole hull is expected to give, the
//! time it takes and a Strip button. A click raises a [`HullStripCommand`];
//! [`apply_hull_strip_commands`] plans it with [`plan_hull_strip`] and starts
//! a [`HullStripJob`] on the player ship. [`advance_hull_strip`] runs the job
//! on the game clock, with the pane open or shut, so the player starts it and
//! flies on; the TAB interface's pause holds it like any other game work.
//!
//! The job takes one live section at a time, the one [`next_strip_section`]
//! names: the farthest from the berth, the berth port last. When a section's
//! timer ends, [`plan_section_yield`] plans its parts: they go into the
//! player's hold, one `Stripped` report is queued, and a
//! [`SectionStripRequest`] takes the section off. A hold too full for them
//! holds the job at that section until there is room. The berth's parts end
//! the strip: the player lets go of the dock and the wreck despawns. The job
//! stops when the player undocks or docks with another hull; every section
//! already taken stays taken.
//!
//! Touch this module when changing what the strip block shows or what Strip
//! does.

use std::collections::BTreeMap;

use bevy::{ecs::system::SystemParam, prelude::*, ui_widgets::Activate};
use nova_events::prelude::EntityId;
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;
use nova_ui::{
    theme::UiColor,
    widget::{ButtonSpec, ThemedFill},
};

use super::RecycleRuntime;
use crate::{
    pane::{compact_button, play_menu_select, themed_label},
    ship::design_section_code,
};

/// The name of the Strip button.
pub const RECYCLE_STRIP_BUTTON: &str = "RecycleStripButton";

/// The name of the strip block's progress line.
pub const RECYCLE_STRIP_PROGRESS: &str = "RecycleStripProgress";

/// A request to strip the whole docked partner, raised by the Recycle pane's
/// Strip button.
#[derive(Message, Clone, Copy, Debug, Default)]
pub struct HullStripCommand;

/// A text of the strip block that [`update_recycle_strip`] refreshes.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum RecycleStripField {
    /// What the strip gives and how long it takes, or why it cannot start.
    Plan,
    /// The running strip's share, the section coming off and the time left.
    Progress,
}

/// The progress bar's fill.
#[derive(Component)]
pub(crate) struct RecycleStripFill;

/// The Strip button's row, hidden while a strip runs or one cannot start.
#[derive(Component)]
pub(crate) struct RecycleStripButtonRow;

/// The recipe of a live section its hull's design does not list: a
/// hand-built hull has no design. It gives no part and takes the shortest
/// strip.
static NO_RECIPE: BuildRecipe = BuildRecipe {
    credits: 0,
    parts: BTreeMap::new(),
    strip_yield: 0.0,
};

/// One live section of the docked partner.
struct LiveSection<'a> {
    entity: Entity,
    id: &'a str,
    health_share: f32,
    neighbours: &'a [Entity],
}

/// The docked partner a strip reads.
pub(crate) struct StripPartner<'a> {
    pub(crate) entity: Entity,
    /// The partner's port that holds the dock.
    berth: Entity,
    living_station: bool,
    lootable: bool,
    design: Option<&'a ShipDesignSections>,
    live: Vec<LiveSection<'a>>,
}

impl StripPartner<'_> {
    /// Whether the Recycle pane serves this partner: one the player may loot
    /// that is not a living station.
    pub(crate) fn recyclable(&self) -> bool {
        self.lootable && !self.living_station
    }

    /// The pane code of design section `id`, or the id when the partner has
    /// no design to number it from.
    fn code(&self, id: &str) -> String {
        self.design
            .and_then(|design| design_section_code(design, id))
            .unwrap_or_else(|| id.to_string())
    }

    fn target(&self, section: &LiveSection<'_>) -> StripTarget<'_> {
        let recipe = self
            .design
            .and_then(|design| design.get(section.id))
            .map_or(&NO_RECIPE, |design| &design.config.base.build);
        StripTarget {
            recipe,
            health_share: section.health_share,
        }
    }

    /// The facts [`plan_hull_strip`] reads.
    fn hull(&self) -> HullStripPartner<'_> {
        HullStripPartner {
            living_station: self.living_station,
            lootable: self.lootable,
            sections: self
                .live
                .iter()
                .map(|section| self.target(section))
                .collect(),
        }
    }

    /// The live section a strip takes next, by [`next_strip_section`] with
    /// ties in pane code order.
    fn next(&self) -> Option<&LiveSection<'_>> {
        let mut order: Vec<(usize, (String, u32))> = self
            .live
            .iter()
            .enumerate()
            .map(|(index, section)| (index, code_order(&self.code(section.id))))
            .collect();
        order.sort_by(|a, b| a.1.cmp(&b.1));
        let next = next_strip_section(
            self.berth,
            order.iter().enumerate().map(|(rank, (index, _))| {
                let section = &self.live[*index];
                (section.entity, rank, section.neighbours)
            }),
        )?;
        self.live.iter().find(|section| section.entity == next)
    }

    fn live(&self, entity: Entity) -> Option<&LiveSection<'_>> {
        self.live.iter().find(|section| section.entity == entity)
    }
}

/// The player ship, the partner it is docked with and every fact a strip
/// reads.
#[derive(SystemParam)]
pub(crate) struct StripSite<'w, 's> {
    player: Query<
        'w,
        's,
        (Entity, Option<&'static DockedShip>),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    connections: Query<'w, 's, &'static DockingConnection>,
    partners: Query<
        'w,
        's,
        (
            Option<&'static ShipDesignSections>,
            Option<&'static Children>,
            Has<LootableShipMarker>,
            Has<NeutralizedMarker>,
            Has<StationMarker>,
            Has<DerelictShipMarker>,
        ),
        With<SpaceshipRootMarker>,
    >,
    sections: Query<
        'w,
        's,
        (
            &'static EntityId,
            &'static Health,
            Option<&'static ConnectedTo>,
        ),
        With<SectionMarker>,
    >,
}

impl StripSite<'_, '_> {
    /// The player ship, or `None` when there is not exactly one.
    pub(crate) fn player(&self) -> Option<Entity> {
        self.player.single().ok().map(|(player, _)| player)
    }

    /// The partner the player ship is docked with, or `None` while it is not
    /// docked.
    ///
    /// # Panics
    ///
    /// On a docked player whose connection is gone: `DockedShip` leaves with
    /// its connection.
    pub(crate) fn partner(&self) -> Option<StripPartner<'_>> {
        let (player, docked) = self.player.single().ok()?;
        let docked = docked?;
        let connection = self.connections.get(docked.connection).unwrap_or_else(|_| {
            panic!(
                "player ship {player:?} is docked through {:?}, which has no DockingConnection",
                docked.connection
            )
        });
        let (entity, berth) = if connection.first_ship == player {
            (connection.second_ship, connection.second_section)
        } else {
            (connection.first_ship, connection.first_section)
        };
        let (design, children, lootable, neutralized, station, derelict) =
            self.partners.get(entity).ok()?;
        let live = children
            .into_iter()
            .flatten()
            .filter_map(|child| {
                let (id, health, connected) = self.sections.get(*child).ok()?;
                let health_share = if health.max > 0.0 {
                    health.current / health.max
                } else {
                    0.0
                };
                Some(LiveSection {
                    entity: *child,
                    id: id.0.as_str(),
                    health_share,
                    neighbours: connected.map_or(&[][..], |connected| connected.0.as_slice()),
                })
            })
            .collect();
        Some(StripPartner {
            entity,
            berth,
            living_station: station && !derelict,
            lootable: lootable || neutralized,
            design,
            live,
        })
    }
}

/// The note line for a strip that does not start.
fn strip_refusal_text(refusal: HullStripRefusalType) -> &'static str {
    match refusal {
        HullStripRefusalType::NotDocked => "strip: dock with a wreck first",
        HullStripRefusalType::Busy => "strip: a strip is already running",
        HullStripRefusalType::LivingStation => "strip: a living station is not salvage",
        HullStripRefusalType::NotLootable => "strip: the docked hull is not a wreck you may loot",
    }
}

/// The line for a strip held at a finished section.
fn strip_pause_text(code: &str, pause: HullStripPauseType) -> String {
    match pause {
        HullStripPauseType::HoldFull { need_g, free_g } => format!(
            "Held: hold full. {code} needs {} of hold, {} free",
            kg_text(need_g),
            kg_text(u64::from(free_g))
        ),
    }
}

/// The list order of a pane code: its prefix, then its number, so `HULL-2`
/// sorts before `HULL-10`. A code with no number sorts by its text alone.
fn code_order(code: &str) -> (String, u32) {
    match code.rsplit_once('-') {
        Some((prefix, index)) => match index.parse() {
            Ok(index) => (prefix.to_string(), index),
            Err(_) => (code.to_string(), 0),
        },
        None => (code.to_string(), 0),
    }
}

/// Parts by item name: `3 Section Panel, 1 Frame Spar`, or `nothing`.
pub(crate) fn strip_parts_text(items: &GameItems, parts: &BTreeMap<ItemDesignId, u32>) -> String {
    if parts.is_empty() {
        return "nothing".to_string();
    }
    parts
        .iter()
        .map(|(item, count)| format!("{count} {}", items.design(item).name))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Game seconds as `2 min 05 s`, or `45 s` under a minute.
fn duration_text(secs: f32) -> String {
    let secs = secs.ceil() as u32;
    if secs < 60 {
        return format!("{secs} s");
    }
    format!("{} min {:02} s", secs / 60, secs % 60)
}

/// The plan line: the sections, the expected parts and their mass, and the
/// time.
fn strip_plan_text(items: &GameItems, partner: &StripPartner<'_>, strip: &HullStrip) -> String {
    let mass_g: u64 = strip
        .parts
        .iter()
        .map(|(item, &count)| items.design(item).stack_mass_g(count))
        .sum();
    format!(
        "{} section{} left. Expected yield: {} ({}). Takes {} of game time; the berth port comes \
         off last and the wreck is gone.",
        partner.live.len(),
        if partner.live.len() == 1 { "" } else { "s" },
        strip_parts_text(items, &strip.parts),
        kg_text(mass_g),
        duration_text(strip.duration_secs),
    )
}

/// Build the strip block: a header, the plan line, the Strip button, the
/// progress line and bar.
pub(crate) fn spawn_recycle_strip(panel: &mut ChildSpawnerCommands) {
    panel.spawn(themed_label("Strip the wreck", 13.0, UiColor::Accent));
    panel.spawn((
        RecycleStripField::Plan,
        themed_label("", 12.0, UiColor::Body),
    ));
    panel
        .spawn((
            RecycleStripButtonRow,
            Node {
                flex_direction: FlexDirection::Row,
                display: Display::None,
                ..default()
            },
        ))
        .with_children(|row| {
            row.spawn((
                Name::new(RECYCLE_STRIP_BUTTON),
                compact_button(ButtonSpec::new("Strip the wreck").fit().primary()),
            ))
            .observe(on_recycle_strip_button);
        });
    panel.spawn((
        RecycleStripField::Progress,
        Name::new(RECYCLE_STRIP_PROGRESS),
        themed_label("", 12.0, UiColor::Primary),
    ));
    panel
        .spawn((
            Node {
                height: Val::Px(6.0),
                flex_shrink: 0.0,
                border_radius: BorderRadius::all(Val::Px(3.0)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::NONE),
            ThemedFill::alpha(UiColor::Secondary, 0.25),
        ))
        .with_children(|track| {
            track.spawn((
                RecycleStripFill,
                Node {
                    width: Val::Percent(0.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                ThemedFill::new(UiColor::Accent),
            ));
        });
}

/// Keep the strip block on the docked partner and the running job: the plan
/// line, the Strip button while a strip can start, and the progress line and
/// bar.
#[expect(
    clippy::type_complexity,
    reason = "the block's texts, its button row and its bar fill"
)]
pub(crate) fn update_recycle_strip(
    items: Option<Res<GameItems>>,
    site: StripSite,
    q_job: Query<&HullStripJob, With<PlayerSpaceshipMarker>>,
    mut q_field: Query<(&RecycleStripField, &mut Text)>,
    mut q_row: Query<&mut Node, (With<RecycleStripButtonRow>, Without<RecycleStripFill>)>,
    mut q_fill: Query<&mut Node, (With<RecycleStripFill>, Without<RecycleStripButtonRow>)>,
) {
    let Some(items) = items else {
        return;
    };
    let job = site.player().and_then(|player| q_job.get(player).ok());
    let partner = site.partner();
    let plan = plan_hull_strip(
        partner.as_ref().map(StripPartner::hull).as_ref(),
        job.is_some(),
    );
    let plan_line = match (&partner, &plan) {
        (Some(partner), Ok(strip)) => strip_plan_text(&items, partner, strip),
        (Some(partner), Err(HullStripRefusalType::Busy)) => format!(
            "{} section{} left.",
            partner.live.len(),
            if partner.live.len() == 1 { "" } else { "s" }
        ),
        (_, Err(refusal)) => strip_refusal_text(*refusal).to_string(),
        (None, Ok(_)) => unreachable!("a strip plans only with a docked partner"),
    };
    let progress_line = match job {
        Some(job) => {
            let step = job.step.as_ref();
            match (job.paused, step) {
                (Some(pause), Some(step)) => strip_pause_text(&step.code, pause),
                _ => format!(
                    "Stripping{}: {:.0}%, {} taken, {} left",
                    step.map(|step| format!(" {}", step.code))
                        .unwrap_or_default(),
                    job.progress() * 100.0,
                    job.taken,
                    duration_text(job.remaining_secs())
                ),
            }
        }
        None => "No strip running".to_string(),
    };
    for (field, mut text) in &mut q_field {
        let line = match field {
            RecycleStripField::Plan => &plan_line,
            RecycleStripField::Progress => &progress_line,
        };
        if text.0 != *line {
            text.0.clone_from(line);
        }
    }
    let display = if plan.is_ok() {
        Display::Flex
    } else {
        Display::None
    };
    for mut row in &mut q_row {
        if row.display != display {
            row.display = display;
        }
    }
    let width = Val::Percent(job.map_or(0.0, HullStripJob::progress) * 100.0);
    for mut fill in &mut q_fill {
        if fill.width != width {
            fill.width = width;
        }
    }
}

/// Raise a [`HullStripCommand`] when the Strip button is clicked.
fn on_recycle_strip_button(
    _activate: On<Activate>,
    mut strips: MessageWriter<HullStripCommand>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
) {
    strips.write(HullStripCommand);
    play_menu_select(&mut commands, bank.as_deref());
}

/// Apply [`HullStripCommand`]s by the [`plan_hull_strip`] rule: start a
/// [`HullStripJob`] on the player ship. Nothing moves until a section's timer
/// ends. A refusal starts nothing and shows its reason on the note line.
pub(crate) fn apply_hull_strip_commands(
    mut messages: MessageReader<HullStripCommand>,
    mut runtime: ResMut<RecycleRuntime>,
    mut commands: Commands,
    items: Option<Res<GameItems>>,
    site: StripSite,
    q_player: Query<Has<HullStripJob>, With<PlayerSpaceshipMarker>>,
) {
    if messages.is_empty() {
        return;
    }
    // One strip per frame: the job lands with the commands, so a second
    // command this frame would plan as not busy.
    messages.clear();
    let (Some(player), Some(items)) = (site.player(), items) else {
        return;
    };
    let busy = q_player.get(player).expect("the player ship has a root");
    let partner = site.partner();
    let strip = match plan_hull_strip(partner.as_ref().map(StripPartner::hull).as_ref(), busy) {
        Ok(strip) => strip,
        Err(refusal) => {
            info!("strip: refused: {refusal:?}");
            runtime.say(strip_refusal_text(refusal).to_string());
            return;
        }
    };
    let partner = partner.expect("a planned strip has a docked partner");
    info!(
        "strip: {:?} starts, {} sections, {:.0} s, expected {}",
        partner.entity,
        partner.live.len(),
        strip.duration_secs,
        strip_parts_text(&items, &strip.parts)
    );
    runtime.say(format!(
        "stripping the wreck: {}",
        duration_text(strip.duration_secs)
    ));
    commands
        .entity(player)
        .insert(HullStripJob::new(partner.entity, strip.duration_secs));
}

/// Run the player ship's [`HullStripJob`] on the game clock.
///
/// The job stops when the player is no longer docked with its partner. It
/// picks the next section by [`next_strip_section`] when it has none, or
/// when its section is gone some other way. When the section's timer ends,
/// [`plan_section_yield`] plans its parts: on a pause the job holds there;
/// otherwise the parts go into the hold, one `Stripped` report is queued, and
/// the section comes off by a [`SectionStripRequest`], or, for the berth, by
/// letting go of the dock and despawning the wreck.
///
/// # Panics
///
/// On a finished section whose partner has no scenario id: every scenario
/// ship and station wears one.
pub(crate) fn advance_hull_strip(
    time: Res<Time>,
    mut runtime: ResMut<RecycleRuntime>,
    mut commands: Commands,
    items: Option<Res<GameItems>>,
    site: StripSite,
    q_ids: Query<&EntityId>,
    mut q_player: Query<
        (
            &mut HullStripJob,
            &mut ShipInventory,
            &mut ShipInventoryReports,
        ),
        With<PlayerSpaceshipMarker>,
    >,
) {
    let Some(player) = site.player() else {
        return;
    };
    let Ok((mut job, mut own, mut reports)) = q_player.get_mut(player) else {
        return;
    };
    let Some(partner) = site
        .partner()
        .filter(|partner| partner.entity == job.partner)
    else {
        info!("strip: stopped by the undock after {} sections", job.taken);
        runtime.say(format!(
            "strip stopped: you undocked; {} section{} taken",
            job.taken,
            if job.taken == 1 { "" } else { "s" }
        ));
        commands.entity(player).remove::<HullStripJob>();
        return;
    };
    let Some(items) = items else {
        return;
    };
    // A section taken some other way (shot off) leaves the job for the next.
    if job
        .step
        .as_ref()
        .is_some_and(|step| partner.live(step.section).is_none())
    {
        job.step = None;
        job.paused = None;
    }
    if job.step.is_none() {
        let Some(next) = partner.next() else {
            // The berth is always live while the dock holds, so a docked
            // partner always has a next section.
            return;
        };
        let duration_secs = strip_duration_secs(partner.target(next).recipe);
        job.step = Some(HullStripStep {
            section: next.entity,
            section_id: next.id.to_string(),
            code: partner.code(next.id),
            elapsed_secs: 0.0,
            duration_secs,
        });
    }
    let step = job.step.clone().expect("the job has a step");
    if !step.is_done() {
        let step = job.step.as_mut().expect("the job has a step");
        step.elapsed_secs = (step.elapsed_secs + time.delta_secs()).min(step.duration_secs);
        if !step.is_done() {
            return;
        }
    }
    let section = partner.live(step.section).expect("the step is live");
    let parts = match plan_section_yield(&items, partner.target(section), &own) {
        Ok(parts) => parts,
        Err(pause) => {
            if job.paused != Some(pause) {
                info!("strip: held at {}: {pause:?}", step.code);
                job.paused = Some(pause);
            }
            return;
        }
    };
    let hull = q_ids
        .get(partner.entity)
        .unwrap_or_else(|_| panic!("strip: partner {:?} has no scenario id", partner.entity));
    for (item, &count) in &parts {
        own.add(&items, item, count);
    }
    let text = strip_parts_text(&items, &parts);
    info!("strip: {} of '{}' gave {text}", step.code, hull.0);
    reports.0.push(ShipInventoryReport::Stripped {
        hull: hull.0.clone(),
        section: step.code.clone(),
        parts,
    });
    job.paused = None;
    job.step = None;
    job.done_secs += step.duration_secs;
    job.taken += 1;
    if step.section != partner.berth {
        commands.trigger(SectionStripRequest {
            entity: step.section,
        });
        return;
    }
    // The berth comes off last: let go of the dock first, so the release is
    // the ordinary one, then the wreck leaves the world.
    info!(
        "strip: '{}' is stripped whole after {} sections",
        hull.0, job.taken
    );
    commands.trigger(DockingReleaseRequest { entity: player });
    commands.entity(partner.entity).despawn();
    commands.entity(player).remove::<HullStripJob>();
    runtime.say(format!(
        "the wreck is stripped and gone: {text} from its port"
    ));
}
