//! The pickup tick: one `+N <ITEM>` line per stack the player's intake takes,
//! stacked above the keybind dock for a moment and gone.
//!
//! A take already has a sound (the intake's `take_sound`) and a light (the
//! intake's warm blink); the tick is the part that says WHAT came aboard,
//! which neither of those can. It reads [`CargoCanisterTaken`], the same
//! event the sound plays on, so a take that the hold's room cut short lists
//! only what it took.
//!
//! One line per stack, newest at the bottom, at most [`TICK_LINES`] showing:
//! a fourth drops the oldest. Each line holds for [`TICK_SECS`] of virtual
//! time, so a paused game holds the tick. No sound of its own, and no tier
//! check: the hold's contents are news on every graphics tier.
//!
//! The column rides the bottom-centre strip the keybind dock owns, measuring
//! the dock's live height the way `cinematic_prompt` does. It never sits
//! lower than the top of the [`HudNotice`](super::hud_notice::HudNotice) box
//! either: a Procure job's `done` notice posts on the very take that fills
//! it, and the two must not print through each other.

use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;
use nova_ui::hud::{ChipText, ChipTone, CHIP_FONT};

use super::{
    hud_notice::{NOTICE_BOTTOM_PX, NOTICE_MAX_HEIGHT_PX},
    keybind_dock::prelude::{KeybindDockMarker, DOCK_BOTTOM_PX},
    NovaHudSystems,
};

/// The pickup tick column spawner, its markers and `PickupTickPlugin`.
pub mod prelude {
    pub use super::{pickup_tick_hud, PickupTickLine, PickupTickMarker, PickupTickPlugin};
}

/// How long one line shows, in virtual seconds.
const TICK_SECS: f32 = 1.5;

/// The most lines showing at once. A fourth take drops the oldest line.
const TICK_LINES: usize = 3;

/// Clearance between the keybind dock's top and the column, and between the
/// notice box's top and the column.
const TICK_GAP_PX: f32 = 8.0;

/// Marker for the column; spawned by [`pickup_tick_hud`] for the player's
/// ship.
#[derive(Component, Debug, Clone, Reflect)]
pub struct PickupTickMarker;

/// One `+N <ITEM>` line and how long it has shown. Public so a probe can read
/// what the player was told came aboard.
#[derive(Component, Debug, Clone, Reflect)]
pub struct PickupTickLine {
    /// Virtual seconds the line has shown.
    pub age_secs: f32,
}

/// UI bundle for the column: empty until a take fills it.
pub fn pickup_tick_hud() -> impl Bundle {
    (
        Name::new("PickupTick"),
        PickupTickMarker,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            bottom: Val::Px(NOTICE_BOTTOM_PX + NOTICE_MAX_HEIGHT_PX + TICK_GAP_PX),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::FlexStart,
            row_gap: Val::Px(2.0),
            ..default()
        },
        Pickable::IGNORE,
    )
}

/// Registers the tick: posts a line per stack the player's intake takes,
/// ages the lines on virtual time and keeps the column clear of the dock.
/// The column itself is a player-scoped widget spawned by `NovaHudPlugin`.
#[derive(Default)]
pub struct PickupTickPlugin;

impl Plugin for PickupTickPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<PickupTickMarker>();
        app.register_type::<PickupTickLine>();
        app.add_observer(post_pickup_tick);
        app.add_systems(
            Update,
            (age_pickup_ticks, keep_the_ticks_clear_of_the_dock).in_set(NovaHudSystems),
        );
    }
}

/// One line per stack the player's intake took, newest at the bottom; a
/// take on any other hull is not the player's news. Without the merged item
/// catalog the line names the item by its id.
fn post_pickup_tick(
    taken: On<CargoCanisterTaken>,
    items: Option<Res<GameItems>>,
    q_child_of: Query<&ChildOf>,
    q_player: Query<(), With<PlayerSpaceshipMarker>>,
    q_column: Query<(Entity, Option<&Children>), With<PickupTickMarker>>,
    q_lines: Query<(), With<PickupTickLine>>,
    mut commands: Commands,
) {
    if !q_child_of
        .iter_ancestors(taken.entity)
        .any(|hull| q_player.contains(hull))
    {
        return;
    }
    let Some((column, children)) = q_column.iter().next() else {
        return;
    };
    let mut showing: Vec<Entity> = children
        .map(|children| {
            children
                .iter()
                .filter(|line| q_lines.contains(*line))
                .collect()
        })
        .unwrap_or_default();
    for stack in &taken.stacks {
        let name = items.as_deref().map_or_else(
            || stack.item.as_str().to_string(),
            |items| items.design(&stack.item).name.clone(),
        );
        let line = format!("+{} {}", stack.count, name.to_uppercase());
        debug!("pickup_tick: {line}");
        while showing.len() >= TICK_LINES {
            commands.entity(showing.remove(0)).despawn();
        }
        let tone = ChipTone::Readout;
        let spawned = commands
            .spawn((
                Name::new("PickupTickLine"),
                PickupTickLine { age_secs: 0.0 },
                ChildOf(column),
                Text::new(line),
                TextFont::from_font_size(CHIP_FONT),
                TextLayout {
                    linebreak: LineBreak::NoWrap,
                    ..default()
                },
                ChipText::value(tone),
                TextColor(Color::NONE),
                Pickable::IGNORE,
            ))
            .id();
        showing.push(spawned);
    }
}

/// Age every line on the virtual clock and drop the ones past their dwell.
fn age_pickup_ticks(
    time: Res<Time<Virtual>>,
    mut q_lines: Query<(Entity, &mut PickupTickLine)>,
    mut commands: Commands,
) {
    for (entity, mut line) in &mut q_lines {
        line.age_secs += time.delta_secs();
        if line.age_secs >= TICK_SECS {
            commands.entity(entity).despawn();
        }
    }
}

/// Ride above the keybind dock and above the notice box, whichever stands
/// taller: the dock's height is measured, the notice box's is its ceiling.
fn keep_the_ticks_clear_of_the_dock(
    q_dock: Query<(&ComputedNode, &InheritedVisibility), With<KeybindDockMarker>>,
    mut q_column: Query<&mut Node, With<PickupTickMarker>>,
) {
    let dock = q_dock
        .iter()
        .filter(|(_, visible)| visible.get())
        .map(|(node, _)| node.size().y * node.inverse_scale_factor)
        .fold(0.0_f32, f32::max);
    let over_dock = if dock > 0.0 {
        DOCK_BOTTOM_PX + dock + TICK_GAP_PX
    } else {
        0.0
    };
    let bottom = Val::Px(over_dock.max(NOTICE_BOTTOM_PX + NOTICE_MAX_HEIGHT_PX + TICK_GAP_PX));
    for mut node in &mut q_column {
        if node.bottom != bottom {
            node.bottom = bottom;
        }
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use bevy::time::TimeUpdateStrategy;
    use nova_gameplay::test_support::test_items;

    use super::*;

    /// Virtual time each `app.update()` advances in this rig: the strategy
    /// asks for 0.5 s and `Time<Virtual>`'s `max_delta` clamps every frame to
    /// 0.25 s (the first frame advances 0.0).
    const FRAME_SECS: f32 = 0.25;

    /// The tick wiring on a manual clock with the fixture item catalog, a
    /// player hull with one intake, and another hull with one intake.
    fn tick_app() -> (App, Entity, Entity) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            0.5,
        )));
        app.insert_resource(GameItems::new(test_items().iter().cloned()));
        app.add_plugins(PickupTickPlugin);
        app.world_mut().spawn(pickup_tick_hud());
        let player = app
            .world_mut()
            .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker))
            .id();
        let player_intake = app.world_mut().spawn(ChildOf(player)).id();
        let other = app.world_mut().spawn(SpaceshipRootMarker).id();
        let other_intake = app.world_mut().spawn(ChildOf(other)).id();
        app.update();
        (app, player_intake, other_intake)
    }

    fn lines(app: &mut App) -> Vec<String> {
        let column = app
            .world_mut()
            .query_filtered::<&Children, With<PickupTickMarker>>()
            .single(app.world())
            .ok()
            .map(|children| children.iter().collect::<Vec<_>>())
            .unwrap_or_default();
        column
            .into_iter()
            .filter_map(|line| app.world().get::<Text>(line).map(|text| text.0.clone()))
            .collect()
    }

    /// Fire a take and apply what the observer queued, as a `commands`
    /// trigger in a system does.
    fn take(app: &mut App, intake: Entity, stacks: &[(&str, u32)]) {
        app.world_mut().trigger(CargoCanisterTaken {
            entity: intake,
            stacks: stacks
                .iter()
                .map(|(item, count)| TakenStack {
                    item: ItemDesignId::from(*item),
                    count: *count,
                })
                .collect(),
        });
        app.world_mut().flush();
    }

    fn advance(app: &mut App, secs: f32) {
        for _ in 0..(secs / FRAME_SECS).ceil() as usize {
            app.update();
        }
    }

    #[test]
    fn a_take_posts_one_line_per_stack_newest_last_and_the_lines_leave_after_their_dwell() {
        let (mut app, intake, _) = tick_app();
        let plate = ItemDesignId::from(ItemRoleType::HullPlate.id());
        let ore = ItemDesignId::from(ItemRoleType::StoneOre.id());
        take(&mut app, intake, &[(plate.as_str(), 2), (ore.as_str(), 30)]);
        app.update();

        let items = app.world().resource::<GameItems>();
        let plate_name = items.design(&plate).name.to_uppercase();
        let ore_name = items.design(&ore).name.to_uppercase();
        assert_eq!(
            lines(&mut app),
            vec![format!("+2 {plate_name}"), format!("+30 {ore_name}")]
        );

        advance(&mut app, TICK_SECS - 2.0 * FRAME_SECS);
        assert_eq!(lines(&mut app).len(), 2, "the lines hold through the dwell");
        advance(&mut app, 2.0 * FRAME_SECS + FRAME_SECS);
        assert!(
            lines(&mut app).is_empty(),
            "the lines leave after the dwell"
        );
    }

    #[test]
    fn a_fourth_line_drops_the_oldest_and_another_hulls_take_posts_nothing() {
        let (mut app, intake, other_intake) = tick_app();
        let plate = ItemDesignId::from(ItemRoleType::HullPlate.id());
        for count in 1..=4 {
            take(&mut app, intake, &[(plate.as_str(), count)]);
        }
        app.update();
        let shown = lines(&mut app);
        assert_eq!(shown.len(), TICK_LINES);
        assert!(
            shown[0].starts_with("+2 "),
            "the oldest line went: {shown:?}"
        );
        assert!(shown[2].starts_with("+4 "), "the newest is last: {shown:?}");

        advance(&mut app, TICK_SECS + 1.0);
        take(&mut app, other_intake, &[(plate.as_str(), 1)]);
        app.update();
        assert!(
            lines(&mut app).is_empty(),
            "another hull's take is not the player's news"
        );
    }
}
