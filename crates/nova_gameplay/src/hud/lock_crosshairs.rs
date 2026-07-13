//! The two-lock crosshair language of the deliberate-radar model (spike
//! 20260713-082207, task 20260713-082330), plus the radar's provisional cue
//! and the tap-clear toast:
//!
//! - WHITE crosshair on the [`TravelLock`] target - the nav designation. The
//!   COMBAT crosshair (the existing reticle in hud/torpedo_target.rs, kept
//!   slightly SMALLER so the two overlap cleanly on one body) stays
//!   relation-tinted - red on hostiles, the common case.
//! - A HOLLOW bordered box on the radar's provisional candidate while the
//!   search is held, colored by the LATCHED destination slot (white = travel,
//!   red = combat - decision D2 makes the routing visible before release),
//!   with the candidate's name so the release is informed (decision D7).
//! - A transient toast line naming what a tap-clear cleared (adversarial
//!   finding UX15 - the mode-scoped tap is invisible otherwise).

use bevy::prelude::*;

use crate::prelude::*;

pub mod prelude {
    pub use super::{
        lock_crosshairs_hud, LockCrosshairsHudMarker, LockCrosshairsHudPlugin, LockToastMarker,
        RadarCandidateMarker, TravelCrosshairMarker,
    };
}

/// On-screen minimum size (px) of the white travel crosshair - a little
/// LARGER than the combat reticle (`MIN_RETICLE_PX` 32) so an overlapped pair
/// reads as two rings.
const TRAVEL_CROSSHAIR_MIN_PX: f32 = 40.0;

/// Travel-lock white.
const TRAVEL_COLOR: Color = Color::srgba(1.0, 1.0, 1.0, 0.9);

/// Provisional-candidate box size (px, fixed - the hollow cue is a searching
/// aid, not a range readout).
const RADAR_BOX_PX: f32 = 48.0;

/// Provisional cue colors by latched slot.
const RADAR_TRAVEL_COLOR: Color = Color::srgba(1.0, 1.0, 1.0, 0.7);
const RADAR_COMBAT_COLOR: Color = Color::srgba(1.0, 0.35, 0.25, 0.8);

/// Toast lifetime (seconds) and fade.
const TOAST_SECONDS: f32 = 2.0;

/// Marker for the crosshairs layer root.
#[derive(Component, Debug, Clone, Reflect)]
pub struct LockCrosshairsHudMarker;

/// Marker for the white travel crosshair node.
#[derive(Component, Debug, Clone, Reflect)]
pub struct TravelCrosshairMarker;

/// Marker for the hollow provisional radar-candidate box.
#[derive(Component, Debug, Clone, Reflect)]
pub struct RadarCandidateMarker;

/// Marker for the radar box's name label.
#[derive(Component, Debug, Clone, Reflect)]
struct RadarCandidateLabelMarker;

/// One fading tap-clear toast line; `age` drives the fade.
#[derive(Component, Debug, Clone, Reflect)]
pub struct LockToastMarker {
    /// Seconds since the toast spawned.
    pub age: f32,
}

/// The crosshairs layer: the travel crosshair + the provisional radar box
/// (both screen-indicator nodes, hidden while their anchors are `None`) and
/// the toast stack.
pub fn lock_crosshairs_hud(target_sprite: Handle<Image>) -> impl Bundle {
    (
        Name::new("LockCrosshairsHUD"),
        LockCrosshairsHudMarker,
        screen_indicator_layer(),
        children![
            (
                Name::new("TravelCrosshair"),
                TravelCrosshairMarker,
                screen_indicator(ScreenIndicatorConfig {
                    anchor: None,
                    size: ScreenIndicatorSize::ApparentSize {
                        min_px: TRAVEL_CROSSHAIR_MIN_PX,
                    },
                    offset: Vec2::ZERO,
                    offscreen: ScreenIndicatorOffscreen::Hide,
                }),
                ImageNode::new(target_sprite).with_color(TRAVEL_COLOR),
            ),
            (
                Name::new("RadarCandidate"),
                RadarCandidateMarker,
                screen_indicator(ScreenIndicatorConfig {
                    anchor: None,
                    size: ScreenIndicatorSize::Fixed(Vec2::splat(RADAR_BOX_PX)),
                    offset: Vec2::ZERO,
                    offscreen: ScreenIndicatorOffscreen::Hide,
                }),
                // Hollow: a border-only box, so it reads as "provisional"
                // against the solid committed crosshairs.
                BorderColor::all(RADAR_TRAVEL_COLOR),
                children![(
                    Name::new("RadarCandidateLabel"),
                    RadarCandidateLabelMarker,
                    Text::new(""),
                    TextFont::from_font_size(11.0),
                    TextColor(RADAR_TRAVEL_COLOR),
                    Node {
                        position_type: PositionType::Absolute,
                        top: Val::Px(-16.0),
                        left: Val::Px(0.0),
                        ..default()
                    },
                )],
            ),
            (
                Name::new("LockToasts"),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    bottom: Val::Percent(30.0),
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
            ),
        ],
    )
}

/// The border box needs a real border width on its node; the widget owns
/// position/size, so patch the border in after spawn.
fn style_radar_box(mut q_box: Query<&mut Node, Added<RadarCandidateMarker>>) {
    for mut node in &mut q_box {
        node.border = UiRect::all(Val::Px(2.0));
    }
}

#[derive(Default)]
pub struct LockCrosshairsHudPlugin;

impl Plugin for LockCrosshairsHudPlugin {
    fn build(&self, app: &mut App) {
        debug!("LockCrosshairsHudPlugin: build");
        app.register_type::<LockToastMarker>();
        app.add_systems(
            Update,
            (
                style_radar_box,
                drive_travel_crosshair,
                drive_radar_candidate,
                spawn_lock_toasts,
                fade_lock_toasts,
            )
                .in_set(super::NovaHudSystems),
        );
    }
}

/// Point the white crosshair at the travel lock; `None` hides it via the
/// widget's anchor handling.
fn drive_travel_crosshair(
    q_player: Query<&TravelLock, With<PlayerSpaceshipMarker>>,
    mut q_crosshair: Query<&mut ScreenIndicatorAnchor, With<TravelCrosshairMarker>>,
) {
    let lock = q_player.iter().next().and_then(|lock| lock.0);
    for mut anchor in &mut q_crosshair {
        **anchor = lock.map(ScreenIndicatorAnchorKind::Entity);
    }
}

/// Point the hollow box at the radar's provisional candidate, colored by the
/// latched slot, labeled with the candidate's `Name` (falls back to the
/// entity id - modded bodies without names still get a cue).
#[allow(clippy::type_complexity)]
fn drive_radar_candidate(
    q_player: Query<Option<&RadarState>, With<PlayerSpaceshipMarker>>,
    q_names: Query<&Name>,
    mut q_box: Query<
        (&mut ScreenIndicatorAnchor, &mut BorderColor, &Children),
        With<RadarCandidateMarker>,
    >,
    mut q_label: Query<(&mut Text, &mut TextColor), With<RadarCandidateLabelMarker>>,
) {
    let radar = q_player.iter().next().flatten().copied();
    for (mut anchor, mut border, children) in &mut q_box {
        let candidate = radar.and_then(|radar| radar.candidate);
        **anchor = candidate.map(ScreenIndicatorAnchorKind::Entity);
        let color = match radar {
            Some(RadarState { combat: true, .. }) => RADAR_COMBAT_COLOR,
            _ => RADAR_TRAVEL_COLOR,
        };
        *border = BorderColor::all(color);
        for &child in children {
            if let Ok((mut text, mut label_color)) = q_label.get_mut(child) {
                let label = candidate
                    .map(|candidate| {
                        q_names
                            .get(candidate)
                            .map(|name| name.to_string())
                            .unwrap_or_else(|_| format!("{candidate:?}"))
                    })
                    .unwrap_or_default();
                if text.0 != label {
                    text.0 = label;
                }
                label_color.0 = color;
            }
        }
    }
}

/// Spawn a fading toast line per tap-clear.
fn spawn_lock_toasts(
    mut commands: Commands,
    mut toasts: MessageReader<LockClearedToast>,
    q_stack: Query<(Entity, &Name)>,
) {
    let Some(stack) = q_stack
        .iter()
        .find(|(_, name)| name.as_str() == "LockToasts")
        .map(|(entity, _)| entity)
    else {
        // No player HUD: drain quietly.
        toasts.read().for_each(|_| {});
        return;
    };
    for toast in toasts.read() {
        let (message, color) = if toast.combat {
            ("COMBAT LOCK CLEARED", RADAR_COMBAT_COLOR)
        } else {
            ("NAV LOCK CLEARED", TRAVEL_COLOR)
        };
        commands.entity(stack).with_child((
            Name::new("LockToast"),
            LockToastMarker { age: 0.0 },
            Text::new(message),
            TextFont::from_font_size(13.0),
            TextColor(color),
        ));
    }
}

/// Age and fade the toast lines, despawning them after [`TOAST_SECONDS`].
fn fade_lock_toasts(
    time: Res<Time>,
    mut commands: Commands,
    mut q_toasts: Query<(Entity, &mut LockToastMarker, &mut TextColor)>,
) {
    for (toast, mut marker, mut color) in &mut q_toasts {
        marker.age += time.delta_secs();
        if marker.age >= TOAST_SECONDS {
            commands.entity(toast).despawn();
            continue;
        }
        let alpha = (1.0 - marker.age / TOAST_SECONDS).clamp(0.0, 1.0);
        color.0 = color.0.with_alpha(alpha);
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    #[test]
    fn travel_crosshair_follows_the_travel_lock() {
        let mut world = World::new();
        let player = world.spawn((PlayerSpaceshipMarker, TravelLock(None))).id();
        let crosshair = world
            .spawn((
                TravelCrosshairMarker,
                screen_indicator(ScreenIndicatorConfig::default()),
            ))
            .id();

        world.run_system_once(drive_travel_crosshair).unwrap();
        assert_eq!(
            **world
                .entity(crosshair)
                .get::<ScreenIndicatorAnchor>()
                .unwrap(),
            None
        );

        let target = world.spawn_empty().id();
        world.get_mut::<TravelLock>(player).unwrap().0 = Some(target);
        world.run_system_once(drive_travel_crosshair).unwrap();
        assert_eq!(
            **world
                .entity(crosshair)
                .get::<ScreenIndicatorAnchor>()
                .unwrap(),
            Some(ScreenIndicatorAnchorKind::Entity(target)),
            "the white crosshair rides the travel lock"
        );

        world.get_mut::<TravelLock>(player).unwrap().0 = None;
        world.run_system_once(drive_travel_crosshair).unwrap();
        assert_eq!(
            **world
                .entity(crosshair)
                .get::<ScreenIndicatorAnchor>()
                .unwrap(),
            None,
            "clearing the lock hides the crosshair"
        );
    }

    #[test]
    fn radar_box_colors_by_the_latched_slot_and_labels_the_candidate() {
        let mut world = World::new();
        let candidate_entity = world.spawn(Name::new("SCAVENGER")).id();
        let player = world
            .spawn((
                PlayerSpaceshipMarker,
                RadarState {
                    combat: false,
                    candidate: Some(candidate_entity),
                },
            ))
            .id();
        let label = world
            .spawn((
                RadarCandidateLabelMarker,
                Text::new(""),
                TextColor(TRAVEL_COLOR),
            ))
            .id();
        let boxed = world
            .spawn((
                RadarCandidateMarker,
                screen_indicator(ScreenIndicatorConfig::default()),
                BorderColor::all(RADAR_TRAVEL_COLOR),
            ))
            .id();
        world.entity_mut(boxed).add_child(label);

        world.run_system_once(drive_radar_candidate).unwrap();
        assert_eq!(
            **world.entity(boxed).get::<ScreenIndicatorAnchor>().unwrap(),
            Some(ScreenIndicatorAnchorKind::Entity(candidate_entity))
        );
        assert_eq!(
            world.entity(label).get::<Text>().unwrap().0,
            "SCAVENGER",
            "the release is informed by the candidate's name (D7)"
        );

        // Raised latch: the cue turns combat-red (D2 - routing visible
        // before release).
        world.get_mut::<RadarState>(player).unwrap().combat = true;
        world.run_system_once(drive_radar_candidate).unwrap();
        let border = *world.entity(boxed).get::<BorderColor>().unwrap();
        assert_eq!(border, BorderColor::all(RADAR_COMBAT_COLOR));

        // Search closed: the box hides.
        world.entity_mut(player).remove::<RadarState>();
        world.run_system_once(drive_radar_candidate).unwrap();
        assert_eq!(
            **world.entity(boxed).get::<ScreenIndicatorAnchor>().unwrap(),
            None
        );
    }
}
