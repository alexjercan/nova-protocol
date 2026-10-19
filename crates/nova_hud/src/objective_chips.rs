//! The open world's standing objective column: fixed on-screen slots that read
//! "what do I do now", beside [`super::objective_stack`] which reads "what
//! just arrived".
//!
//! `objective_stack` is the scenario NOTIFICATION stack - a posted objective
//! pops once, is read, and leaves, and idle cruise shows no cue at all. This
//! module is the open world's standing answer, the same list
//! [`open_objectives`] feeds to the Map's OBJECTIVES block and the world save
//! header, so the three never disagree. It is always on screen once anything
//! is open; nothing here pops, breathes or cues - a status readout shows a
//! state, not a notification.
//!
//! Slot policy, fixed so a chip never moves when a sibling appears or leaves:
//! - slot 0: the role goal's next unmet milestone;
//! - slots 1..=3: the first three held jobs (`JOB-1`..`JOB-3`; a hand-in
//!   renumbers the jobs behind it into these slots, same as the Map);
//! - slot 4: `+k JOBS ON MAP` once a fourth job is held, hidden otherwise.
//!
//! An empty slot hides by [`Visibility::Hidden`], never `Display::None`:
//! `Display::None` collapses flex and would shunt the slots below it, which
//! defeats the fixed geometry this module exists to hold. The column hides
//! whole through its [`HudContextGate`] while nothing is open: the root's
//! `Visibility` belongs to `apply_hud_visibility`, which also hides it with
//! the HUD level and under the interface.

use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_ui::hud::{chip_node, ChipText, ChipTone, ThemedChip, CHIP_FONT};

use super::{HudContextGate, NovaHudSystems};

/// `objective_chips_hud`, its markers and [`ObjectiveChipsPlugin`].
pub mod prelude {
    pub use super::{
        objective_chips_hud, ObjectiveChipSlot, ObjectiveChipsHudMarker, ObjectiveChipsPlugin,
    };
}

/// How many slots the column holds: the milestone, three held jobs, and the
/// overflow count.
const OBJECTIVE_CHIP_SLOT_COUNT: usize = 5;

/// Distance (px) between one slot's top and the next's: a live rig
/// measurement of a one-line amber chip at [`CHIP_FONT`] is 25 px tall, plus 5
/// px clearance so no two slots' boxes can touch.
const OBJECTIVE_CHIP_PITCH_PX: f32 = 30.0;

/// Distance (px) from the right edge of the screen to the column.
const OBJECTIVE_CHIPS_RIGHT_PX: f32 = 12.0;

/// Where the column sits vertically, as a share of the window height.
const OBJECTIVE_CHIPS_TOP_PERCENT: f32 = 45.0;

/// A job title longer than this many characters is cut; the code, meter and
/// hand-in always show in full.
const JOB_TITLE_MAX_CHARS: usize = 32;

/// How many characters of an over-long title survive before the `...`.
const JOB_TITLE_CUT_CHARS: usize = 29;

/// Marker for the column root; spawned by [`objective_chips_hud`].
#[derive(Component, Debug, Clone, Reflect)]
pub struct ObjectiveChipsHudMarker;

/// One fixed slot in the column, `0..OBJECTIVE_CHIP_SLOT_COUNT`. Public so
/// rigs and screenshot examples can read a specific slot.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub struct ObjectiveChipSlot(pub usize);

/// UI bundle for the column: five fixed, independently positioned chips, each
/// hidden until [`update_objective_chips`] has something to show it.
pub fn objective_chips_hud() -> impl Bundle {
    (
        Name::new("ObjectiveChipsHUD"),
        ObjectiveChipsHudMarker,
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(OBJECTIVE_CHIPS_RIGHT_PX),
            top: Val::Percent(OBJECTIVE_CHIPS_TOP_PERCENT),
            ..default()
        },
        HudContextGate(false),
        Visibility::Hidden,
        Pickable::IGNORE,
        children![
            objective_chip_slot(0),
            objective_chip_slot(1),
            objective_chip_slot(2),
            objective_chip_slot(3),
            objective_chip_slot(4),
        ],
    )
}

/// One slot's chip: [`chip_node`]'s geometry pinned absolute at `index`'s
/// pitch, the amber quiet paint ([`ThemedChip::quiet`], [`ChipText::unit`] -
/// what [`nova_ui::hud::quiet_chip`] bundles, inlined here because it already
/// carries its own `Node` and this slot needs a positioned one instead).
fn objective_chip_slot(index: usize) -> impl Bundle {
    let mut node = chip_node();
    node.position_type = PositionType::Absolute;
    node.right = Val::Px(0.0);
    node.top = Val::Px(index as f32 * OBJECTIVE_CHIP_PITCH_PX);
    (
        ObjectiveChipSlot(index),
        ThemedChip::quiet(ChipTone::Amber),
        node,
        BackgroundColor(Color::NONE),
        BorderColor::all(Color::NONE),
        ChipText::unit(ChipTone::Amber),
        TextColor(Color::NONE),
        Text::new(""),
        TextFont::from_font_size(CHIP_FONT),
        TextLayout {
            linebreak: LineBreak::NoWrap,
            ..default()
        },
        Visibility::Hidden,
        Pickable::IGNORE,
    )
}

/// Builds the open-world objective column. Registers [`ObjectiveChipsHudMarker`]
/// and [`ObjectiveChipSlot`] and runs [`update_objective_chips`] in `Update`
/// within [`NovaHudSystems`]. Does not init [`Chart`] or [`Jobs`]: a scenario
/// legitimately runs with neither, and the column reads what exists rather
/// than manufacturing an owner resource the HUD does not own.
#[derive(Default)]
pub struct ObjectiveChipsPlugin;

impl Plugin for ObjectiveChipsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<ObjectiveChipsHudMarker>();
        app.register_type::<ObjectiveChipSlot>();
        app.add_systems(Update, update_objective_chips.in_set(NovaHudSystems));
    }
}

/// Fill the column from [`open_objectives`], or show nothing while the world
/// carries no [`Chart`] or no [`Jobs`] (a scenario). Writes a slot's `Text` or
/// `Visibility` only when the value differs from the node's own, so an
/// unchanged column never marks its nodes changed and a column spawned for a
/// new player ship fills on its first frame.
fn update_objective_chips(
    chart: Option<Res<Chart>>,
    jobs: Option<Res<Jobs>>,
    goal: Res<GoalProgress>,
    record: Res<GoalRecord>,
    items: Option<Res<GameItems>>,
    q_hold: Query<&ShipInventory, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
    mut q_gate: Query<&mut HudContextGate, With<ObjectiveChipsHudMarker>>,
    mut q_slots: Query<(&ObjectiveChipSlot, &mut Text, &mut Visibility)>,
) {
    let open = match (&chart, &jobs) {
        (Some(chart), Some(jobs)) => open_objectives(
            chart,
            jobs,
            items.as_deref(),
            q_hold.single().ok(),
            &goal,
            &record,
        ),
        _ => Vec::new(),
    };
    let texts = slot_texts(&open);
    let any_shown = texts.iter().any(Option::is_some);
    for mut gate in &mut q_gate {
        gate.set_if_neq(HudContextGate(any_shown));
    }
    for (slot, mut text, mut visibility) in &mut q_slots {
        match &texts[slot.0] {
            Some(line) => {
                if text.0 != *line {
                    text.0 = line.clone();
                }
                visibility.set_if_neq(Visibility::Inherited);
            }
            None => {
                visibility.set_if_neq(Visibility::Hidden);
            }
        }
    }
}

/// The text each of the five slots shows this frame, `None` for an empty one.
fn slot_texts(open: &[OpenObjectiveType]) -> Vec<Option<String>> {
    let mut slots = vec![None; OBJECTIVE_CHIP_SLOT_COUNT];

    slots[0] = open.iter().find_map(|objective| match objective {
        OpenObjectiveType::Milestone(line) => Some(milestone_chip_text(line)),
        OpenObjectiveType::Job { .. } => None,
    });

    let held_jobs: Vec<&OpenObjectiveType> = open
        .iter()
        .filter(|objective| matches!(objective, OpenObjectiveType::Job { .. }))
        .collect();
    for (slot, objective) in held_jobs.iter().take(3).enumerate() {
        let OpenObjectiveType::Job {
            code,
            title,
            have,
            need,
            done,
            hand_in,
            ..
        } = objective
        else {
            unreachable!("held_jobs is filtered to the Job variant");
        };
        slots[slot + 1] = Some(job_chip_text(code, title, *have, *need, *done, hand_in));
    }
    if held_jobs.len() > 3 {
        slots[4] = Some(format!("+{} JOBS ON MAP", held_jobs.len() - 3));
    }

    slots
}

/// A milestone's chip line: `Stations owned  0 / 1`.
fn milestone_chip_text(line: &GoalLine) -> String {
    format!("{}  {} / {}", line.label, line.have, line.need)
}

/// A job's chip line: `JOB-1  Bring 7 Water ice  0 / 7  > STN-1`, `DONE` for
/// the meter once its work is done, and the hand-in omitted while the chart
/// has not found that station.
fn job_chip_text(
    code: &str,
    title: &str,
    have: u32,
    need: u32,
    done: bool,
    hand_in: &str,
) -> String {
    let meter = if done {
        "DONE".to_string()
    } else {
        format!("{have} / {need}")
    };
    let hand_in = if hand_in.is_empty() {
        String::new()
    } else {
        format!("  > {hand_in}")
    };
    format!("{code}  {}  {meter}{hand_in}", truncate_job_title(title))
}

/// Cut a title past [`JOB_TITLE_MAX_CHARS`] chars to its first
/// [`JOB_TITLE_CUT_CHARS`] plus `...`; counted in chars, not bytes.
fn truncate_job_title(title: &str) -> String {
    let chars: Vec<char> = title.chars().collect();
    if chars.len() <= JOB_TITLE_MAX_CHARS {
        return title.to_string();
    }
    let mut cut: String = chars[..JOB_TITLE_CUT_CHARS].iter().collect();
    cut.push_str("...");
    cut
}

#[cfg(test)]
mod tests {
    use nova_events::prelude::{GeneratedBodyId, SectorCoord};

    use super::*;
    use crate::chip_layout_rig::{chip_layout_app, measure, settle};

    fn station() -> GeneratedBodyId {
        GeneratedBodyId::new(SectorCoord::ORIGIN, "cluster_0_0_0_station", 0)
    }

    fn job(station: &GeneratedBodyId, slot: u32, item: &str, count: u32) -> Job {
        Job {
            id: JobId {
                station: station.clone(),
                slot,
                posting: 0,
            },
            kind: JobType::Procure {
                item: item.into(),
                count,
            },
            target: station.clone(),
            reward_cr: 100,
        }
    }

    fn items() -> GameItems {
        GameItems::new([
            item_design("WaterIce", "Water ice"),
            item_design("IronOre", "Iron ore"),
            item_design("Scrap", "Scrap metal"),
            item_design("Fuel", "Fuel cells"),
        ])
    }

    fn item_design(id: &str, name: &str) -> ItemDesign {
        ItemDesign {
            id: id.into(),
            name: name.to_string(),
            about: String::new(),
            category: ItemCategoryType::Raw,
            mass_g: 1_000,
            ask_cr: 16,
            bid_cr: 12,
        }
    }

    fn goal() -> GoalProgress {
        GoalProgress {
            title: "A station network".to_string(),
            lines: vec![
                GoalLine::new("Trade jobs handed in", 4, 4),
                GoalLine::new("Stations owned", 0, 1),
            ],
        }
    }

    /// Slot entities by index, found under `root`.
    fn slots(app: &App, root: Entity) -> [Entity; OBJECTIVE_CHIP_SLOT_COUNT] {
        let children = app.world().entity(root).get::<Children>().unwrap().to_vec();
        let mut found = [Entity::PLACEHOLDER; OBJECTIVE_CHIP_SLOT_COUNT];
        for child in children {
            let slot = app
                .world()
                .entity(child)
                .get::<ObjectiveChipSlot>()
                .unwrap();
            found[slot.0] = child;
        }
        found
    }

    fn text(app: &App, entity: Entity) -> String {
        app.world().entity(entity).get::<Text>().unwrap().0.clone()
    }

    fn gate(app: &App, root: Entity) -> bool {
        app.world().entity(root).get::<HudContextGate>().unwrap().0
    }

    fn visibility(app: &App, entity: Entity) -> Visibility {
        *app.world().entity(entity).get::<Visibility>().unwrap()
    }

    /// The milestone and the first three held jobs fill their fixed slots and
    /// a fourth job folds into the overflow chip; a hand-in renumbers the
    /// jobs without moving any slot, and nothing open hides the column.
    #[test]
    fn the_objective_chips_keep_their_slots_and_fold_extra_jobs_into_one_chip() {
        let mut app = chip_layout_app();
        let station = station();
        let mut chart = Chart::default();
        chart.find_station(&station, false, false, None, 0.0);
        app.insert_resource(chart);

        let mut jobs = Jobs::default();
        jobs.accept(job(&station, 0, "WaterIce", 7));
        jobs.accept(job(&station, 1, "IronOre", 4));
        jobs.accept(job(&station, 2, "Scrap", 2));
        jobs.accept(job(&station, 3, "Fuel", 5));
        app.insert_resource(jobs);

        app.insert_resource(items());
        app.insert_resource(goal());
        app.insert_resource(GoalRecord::default());

        app.world_mut().spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            ShipInventory::new(&items(), 10_000, []),
        ));

        let root = app.world_mut().spawn(objective_chips_hud()).id();
        app.add_systems(Update, update_objective_chips);
        // A single-tick bevy test needs a warm-up before the first real frame.
        app.update();
        settle(&mut app);

        let slot = slots(&app, root);
        assert_eq!(text(&app, slot[0]), "Stations owned  0 / 1");
        assert_eq!(
            text(&app, slot[1]),
            "JOB-1  Bring 7 Water ice  0 / 7  > STN-1"
        );
        assert_eq!(
            text(&app, slot[2]),
            "JOB-2  Bring 4 Iron ore  0 / 4  > STN-1"
        );
        assert_eq!(
            text(&app, slot[3]),
            "JOB-3  Bring 2 Scrap metal  0 / 2  > STN-1"
        );
        assert_eq!(text(&app, slot[4]), "+1 JOBS ON MAP");
        for entity in slot {
            assert_eq!(visibility(&app, entity), Visibility::Inherited);
        }
        assert!(gate(&app, root), "an open objective opens the column");

        // Compare the vertical span only: the right-anchored chip's width
        // (and so its left edge) tracks the text it holds, but its pitch from
        // the column's top must not.
        let y_span = |app: &App, entity: Entity| {
            let rect = measure(app, entity).rect;
            (rect.min.y, rect.max.y)
        };
        let before: Vec<(f32, f32)> = slot.iter().map(|&entity| y_span(&app, entity)).collect();
        for (a, b) in before.iter().zip(before.iter().skip(1)) {
            assert!(a.1 <= b.0, "slot boxes overlap: {a:?} sits over {b:?}");
        }

        // Hand in the middle job: 3 remain, and the slots above it renumber.
        let mut jobs = app.world_mut().resource_mut::<Jobs>();
        jobs.complete(&JobId {
            station: station.clone(),
            slot: 1,
            posting: 0,
        });
        app.update();
        settle(&mut app);

        assert_eq!(
            text(&app, slot[1]),
            "JOB-1  Bring 7 Water ice  0 / 7  > STN-1"
        );
        assert_eq!(
            text(&app, slot[2]),
            "JOB-2  Bring 2 Scrap metal  0 / 2  > STN-1"
        );
        assert_eq!(
            text(&app, slot[3]),
            "JOB-3  Bring 5 Fuel cells  0 / 5  > STN-1"
        );
        assert_eq!(visibility(&app, slot[4]), Visibility::Hidden);

        let after: Vec<(f32, f32)> = slot.iter().map(|&entity| y_span(&app, entity)).collect();
        assert_eq!(
            before, after,
            "a hand-in must not move any slot's laid-out position"
        );

        // Clear the jobs and meet the goal: the whole column hides.
        app.world_mut().resource_mut::<Jobs>().clear();
        app.insert_resource(GoalRecord { met_at: Some(90.0) });
        app.update();
        settle(&mut app);

        assert!(!gate(&app, root), "nothing open closes the column");
    }
}
