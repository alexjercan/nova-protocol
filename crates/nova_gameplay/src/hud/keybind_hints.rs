//! Contextual keybind hints (task 20260710-174646, spike
//! docs/spikes/20260710-174523-diegetic-instruments-keybind-hints.md):
//! nobody memorizes X/G/O/Z. Everything renders from the input layer's
//! [`FlightVerbHints`] resource - availability and key labels are resolved
//! where the verbs live, this module is a dumb view.
//!
//! - **Hint cluster**: a small column docked in the lower-left corner,
//!   one `[KEY] VERB` row per flight verb, nav-cyan when pressing it would
//!   do something, dimmed when not.
//! - **Anchored cues**: the hint sits on the thing you would act on -
//!   `[O] ORBIT` projected on the dominant well (absorbed from the
//!   hand-placed v1 cue in flight_status.rs), `[G] GOTO` on the aim lock
//!   while no maneuver is engaged.

use bevy::prelude::*;

use super::{screen_indicator::prelude::*, NAV_CYAN};
use crate::input::prelude::*;

pub mod prelude {
    pub use super::{
        keybind_hint_cluster_hud, verb_cues_hud, KeybindHintClusterMarker, KeybindHintsPlugin,
        VerbCuesHudMarker,
    };
}

/// Dimmed row color for a verb that is present but not currently available.
const DIM_COLOR: Color = Color::srgba(0.5, 0.55, 0.6, 0.5);

/// On-screen size of an anchored cue chip (px).
const CUE_SIZE: Vec2 = Vec2::new(96.0, 16.0);

/// The cue sits below its object so it reads as a caption, not a lock.
const CUE_OFFSET: Vec2 = Vec2::new(0.0, 48.0);

#[derive(Component, Debug, Clone, Reflect)]
pub struct KeybindHintClusterMarker;

/// A row of the cluster; the index picks the verb (see [`row_hint`]).
#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
struct KeybindHintRow(usize);

/// The verb names, in cluster display order (top to bottom).
const ROW_VERBS: [&str; 4] = ["STOP", "GOTO", "ORBIT", "CANCEL"];

#[derive(Component, Debug, Clone, Reflect)]
pub struct VerbCuesHudMarker;

/// Marker for the orbit cue chip (anchored on the dominant well).
#[derive(Component, Debug, Clone, Reflect)]
struct OrbitCueUIMarker;

/// Marker for the goto cue chip (anchored on the aim lock).
#[derive(Component, Debug, Clone, Reflect)]
struct GotoCueUIMarker;

/// UI bundle for the hint cluster: a fixed column in the lower-left
/// corner (the flight status that used to sit under it moved onto the
/// ship as chips, task 20260710-231926).
pub fn keybind_hint_cluster_hud() -> impl Bundle {
    let row = |index: usize| {
        (
            KeybindHintRow(index),
            Text::new(""),
            TextFont::from_font_size(12.0),
            TextColor(DIM_COLOR),
        )
    };

    (
        Name::new("KeybindHintClusterHUD"),
        KeybindHintClusterMarker,
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(8.0),
            left: Val::Px(8.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(2.0),
            ..default()
        },
        Pickable::IGNORE,
        children![row(0), row(1), row(2), row(3)],
    )
}

/// UI bundle for the anchored cues: one indicator layer with the orbit and
/// goto chips.
pub fn verb_cues_hud() -> impl Bundle {
    let cue = || {
        screen_indicator(ScreenIndicatorConfig {
            anchor: None,
            size: ScreenIndicatorSize::Fixed(CUE_SIZE),
            offset: CUE_OFFSET,
            offscreen: ScreenIndicatorOffscreen::Hide,
        })
    };

    (
        Name::new("VerbCuesHUD"),
        VerbCuesHudMarker,
        screen_indicator_layer(),
        children![
            (
                Name::new("OrbitCueUI"),
                OrbitCueUIMarker,
                cue(),
                Text::new(""),
                TextFont::from_font_size(12.0),
                TextColor(NAV_CYAN),
            ),
            (
                Name::new("GotoCueUI"),
                GotoCueUIMarker,
                cue(),
                Text::new(""),
                TextFont::from_font_size(12.0),
                TextColor(NAV_CYAN),
            ),
        ],
    )
}

#[derive(Default)]
pub struct KeybindHintsPlugin;

impl Plugin for KeybindHintsPlugin {
    fn build(&self, app: &mut App) {
        debug!("KeybindHintsPlugin: build");

        app.add_systems(
            Update,
            (update_hint_cluster, (drive_orbit_cue, drive_goto_cue)).in_set(super::NovaHudSystems),
        );
    }
}

fn row_hint(hints: &FlightVerbHints, index: usize) -> &VerbHint {
    match index {
        0 => &hints.stop,
        1 => &hints.goto,
        2 => &hints.orbit,
        _ => &hints.cancel,
    }
}

/// `[KEY] VERB` per row: cyan when available, dim when not, empty until
/// the flight rig exists (no rig, no keys, no hints).
fn update_hint_cluster(
    hints: Res<FlightVerbHints>,
    mut q_row: Query<(&KeybindHintRow, &mut Text, &mut TextColor)>,
    q_added: Query<(), Added<KeybindHintRow>>,
) {
    // Skip quiet frames, but never skip freshly spawned rows (a respawned
    // HUD may appear while the resource is unchanged).
    if !hints.is_changed() && q_added.is_empty() {
        return;
    }
    for (row, mut text, mut color) in &mut q_row {
        let hint = row_hint(&hints, **row);
        if hint.key.is_empty() {
            text.clear();
            continue;
        }
        **text = format!("[{}] {}", hint.key, ROW_VERBS[(**row).min(3)]);
        **color = if hint.available { NAV_CYAN } else { DIM_COLOR };
    }
}

/// `[O] ORBIT` on the dominant well while parking is on offer (the
/// resolver already retires the offer while orbiting).
fn drive_orbit_cue(
    hints: Res<FlightVerbHints>,
    mut q_ui: Query<(&mut ScreenIndicatorAnchor, &mut Text), With<OrbitCueUIMarker>>,
    q_added: Query<(), Added<OrbitCueUIMarker>>,
) {
    // Same guard shape as the cluster: the resource is the only input, so
    // quiet frames write nothing (an unconditional Text deref would
    // re-layout the chip every frame).
    if !hints.is_changed() && q_added.is_empty() {
        return;
    }
    for (mut anchor, mut text) in &mut q_ui {
        match (hints.orbit.available, hints.orbit.anchor) {
            (true, Some(well)) => {
                **anchor = Some(ScreenIndicatorAnchorKind::Entity(well));
                **text = format!("[{}] ORBIT", hints.orbit.key);
            }
            _ => {
                **anchor = None;
                text.clear();
            }
        }
    }
}

/// `[G] GOTO` on the aim lock while nothing is engaged - once a maneuver
/// runs the destination marker takes over and the cue would be noise.
fn drive_goto_cue(
    hints: Res<FlightVerbHints>,
    mut q_ui: Query<(&mut ScreenIndicatorAnchor, &mut Text), With<GotoCueUIMarker>>,
    q_added: Query<(), Added<GotoCueUIMarker>>,
) {
    if !hints.is_changed() && q_added.is_empty() {
        return;
    }
    for (mut anchor, mut text) in &mut q_ui {
        match (hints.goto.available && !hints.engaged, hints.goto.anchor) {
            (true, Some(lock)) => {
                **anchor = Some(ScreenIndicatorAnchorKind::Entity(lock));
                **text = format!("[{}] GOTO", hints.goto.key);
            }
            _ => {
                **anchor = None;
                text.clear();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    fn hints(orbit_available: bool, engaged: bool, well: Option<Entity>) -> FlightVerbHints {
        FlightVerbHints {
            stop: VerbHint {
                key: "X".into(),
                available: true,
                anchor: None,
            },
            goto: VerbHint {
                key: "G".into(),
                available: false,
                anchor: None,
            },
            orbit: VerbHint {
                key: "O".into(),
                available: orbit_available,
                anchor: well,
            },
            cancel: VerbHint {
                key: "Z".into(),
                available: engaged,
                anchor: None,
            },
            engaged,
        }
    }

    fn cluster_rows(world: &mut World) -> Vec<Entity> {
        let cluster = world.spawn(keybind_hint_cluster_hud()).id();
        world.entity(cluster).get::<Children>().unwrap().to_vec()
    }

    #[test]
    fn cluster_rows_show_labels_and_availability() {
        let mut world = World::new();
        let well = world.spawn_empty().id();
        world.insert_resource(hints(true, false, Some(well)));
        let rows = cluster_rows(&mut world);

        world.run_system_once(update_hint_cluster).unwrap();

        let text = |e: Entity| world.entity(e).get::<Text>().unwrap().0.clone();
        let color = |e: Entity| world.entity(e).get::<TextColor>().unwrap().0;
        assert_eq!(text(rows[0]), "[X] STOP");
        assert_eq!(color(rows[0]), NAV_CYAN, "available verbs light up");
        assert_eq!(text(rows[1]), "[G] GOTO");
        assert_eq!(color(rows[1]), DIM_COLOR, "no lock, GOTO stays dim");
        assert_eq!(text(rows[2]), "[O] ORBIT");
        assert_eq!(text(rows[3]), "[Z] CANCEL");
        assert_eq!(color(rows[3]), DIM_COLOR, "nothing engaged");
    }

    #[test]
    fn cluster_rows_stay_empty_without_a_flight_rig() {
        let mut world = World::new();
        world.insert_resource(FlightVerbHints::default());
        let rows = cluster_rows(&mut world);

        world.run_system_once(update_hint_cluster).unwrap();

        for row in rows {
            assert!(
                world.entity(row).get::<Text>().unwrap().0.is_empty(),
                "no rig, no keys, no hint rows"
            );
        }
    }

    fn spawn_cues(world: &mut World) -> (Entity, Entity) {
        let layer = world.spawn(verb_cues_hud()).id();
        let children = world.entity(layer).get::<Children>().unwrap();
        (children[0], children[1])
    }

    fn anchor_of(world: &World, entity: Entity) -> Option<ScreenIndicatorAnchorKind> {
        **world.entity(entity).get::<ScreenIndicatorAnchor>().unwrap()
    }

    #[test]
    fn orbit_cue_follows_the_resolvers_offer() {
        let mut world = World::new();
        let well = world.spawn_empty().id();
        world.insert_resource(hints(true, false, Some(well)));
        let (orbit_cue, _) = spawn_cues(&mut world);

        world.run_system_once(drive_orbit_cue).unwrap();
        assert_eq!(
            anchor_of(&world, orbit_cue),
            Some(ScreenIndicatorAnchorKind::Entity(well))
        );
        assert_eq!(
            world.entity(orbit_cue).get::<Text>().unwrap().0,
            "[O] ORBIT"
        );

        // Orbiting (the resolver retires the offer): the cue hides.
        world.insert_resource(hints(false, true, Some(well)));
        world.run_system_once(drive_orbit_cue).unwrap();
        assert_eq!(anchor_of(&world, orbit_cue), None);
    }

    #[test]
    fn goto_cue_shows_on_the_lock_and_hides_while_engaged() {
        let mut world = World::new();
        let lock = world.spawn_empty().id();
        let mut resource = hints(false, false, None);
        resource.goto = VerbHint {
            key: "G".into(),
            available: true,
            anchor: Some(lock),
        };
        world.insert_resource(resource.clone());
        let (_, goto_cue) = spawn_cues(&mut world);

        world.run_system_once(drive_goto_cue).unwrap();
        assert_eq!(
            anchor_of(&world, goto_cue),
            Some(ScreenIndicatorAnchorKind::Entity(lock))
        );
        assert_eq!(world.entity(goto_cue).get::<Text>().unwrap().0, "[G] GOTO");

        // A maneuver engages: the destination marker takes over.
        resource.cancel.available = true;
        resource.engaged = true;
        world.insert_resource(resource);
        world.run_system_once(drive_goto_cue).unwrap();
        assert_eq!(anchor_of(&world, goto_cue), None);
    }
}
