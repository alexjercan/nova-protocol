//! The two-lock crosshair language of the deliberate-radar model (spikes
//! 20260713-082207 + 20260713-110039): show, don't tell.
//!
//! - WHITE crosshair on the [`TravelLock`] target - the nav designation. The
//!   COMBAT crosshair (the existing reticle in hud/torpedo_target.rs, kept
//!   slightly SMALLER so the two overlap cleanly on one body) is always
//!   combat-RED: the on-object lock language is purely slot-colored (user
//!   decision 2026-07-13, task 20260713-124000 - red bracket = combat lock,
//!   white bracket = travel lock; the relation tint and the reticle corner
//!   pips are retired, since a visible combat reticle already implies
//!   weapons-hot).
//! - A HOLLOW bordered box riding the live lock while a radar gesture is
//!   ENGAGED (past the hold threshold; nothing renders inside the tap
//!   window - F11), colored by the engaged slot, with a DISTANCE-ONLY label
//!   identical for both slots (playtest 2026-07-13 revising Q6a: the name
//!   read as clutter and the slot asymmetry as a bug; names live on the
//!   inset's faction line and the readout).
//! - An UNLATCH GHOST per tap-clear (Q7a): the crosshair visibly pops off
//!   the target - scale up, fade out - in the slot's color; the staged
//!   double tap reads as two distinct pops. Replaces the old text toast
//!   (the LockOff cue in audio.rs is its sound).
//! - A brief centered red flash when the radar is DENIED (no Lock
//!   capability, F7/Q8a; pairs with the deny buzz).
//!
//! The old "WEAPONS HOT ..." status text block is GONE: the inset frame
//! (+ the hot-shifted lead pips) carries the safety state, the inset's
//! presence is the guided-torpedo signal, and "TORP: DUMB" died without
//! replacement (the red reticle's presence anywhere IS the guided cue).

use bevy::prelude::*;

use crate::prelude::*;

pub mod prelude {
    pub use super::{
        lock_crosshairs_hud, LockCrosshairsHudMarker, LockCrosshairsHudPlugin,
        LockUnlatchGhostMarker, RadarCandidateMarker, RadarDenyFlashMarker, TravelCrosshairMarker,
    };
}

/// On-screen minimum size (px) of the white travel crosshair - a little
/// LARGER than the combat reticle (`MIN_RETICLE_PX` 32) so an overlapped pair
/// reads as two rings.
const TRAVEL_CROSSHAIR_MIN_PX: f32 = 40.0;

/// Apparent-size multiplier of the travel crosshair vs the combat reticle
/// (which tracks at 1.0): keeps an overlapped pair concentric at any target
/// size, not just at the min-px floor (playtest 2026-07-13). A feel knob.
const TRAVEL_CROSSHAIR_SCALE: f32 = 1.35;

/// Travel-lock white.
const TRAVEL_COLOR: Color = Color::srgba(1.0, 1.0, 1.0, 0.9);

/// Radar box size (px, fixed - the hollow cue is a searching aid, not a
/// range readout).
const RADAR_BOX_PX: f32 = 48.0;

/// Radar cue colors by engaged slot.
const RADAR_TRAVEL_COLOR: Color = Color::srgba(1.0, 1.0, 1.0, 0.7);
const RADAR_COMBAT_COLOR: Color = Color::srgba(1.0, 0.35, 0.25, 0.8);

/// Unlatch ghost: lifetime (s), how far it grows (fraction of its start
/// size) and the start sizes per slot (matching the crosshair each ghost
/// stands in for, so the pop starts exactly where the crosshair was).
const GHOST_SECONDS: f32 = 0.7;
const GHOST_GROWTH: f32 = 0.8;
const GHOST_TRAVEL_PX: f32 = TRAVEL_CROSSHAIR_MIN_PX;
const GHOST_COMBAT_PX: f32 = 32.0;

/// Radar-denied flash: a centered hollow box, red, gone fast (Q8a).
const DENY_FLASH_SECONDS: f32 = 0.35;

/// Marker for the crosshairs layer root.
#[derive(Component, Debug, Clone, Reflect)]
pub struct LockCrosshairsHudMarker;

/// The crosshair sprite the unlatch ghosts are stamped from, stored on the
/// layer root at setup so runtime spawns need no asset plumbing.
#[derive(Component, Debug, Clone, Reflect)]
pub struct LockGhostSprite(pub Handle<Image>);

/// Marker for the white travel crosshair node.
#[derive(Component, Debug, Clone, Reflect)]
pub struct TravelCrosshairMarker;

/// Marker for the hollow radar box (the radar-active adornment).
#[derive(Component, Debug, Clone, Reflect)]
pub struct RadarCandidateMarker;

/// Marker for the radar box's travel-sweep label (name + distance).
#[derive(Component, Debug, Clone, Reflect)]
struct RadarCandidateLabelMarker;

/// One unlatch ghost: `age` drives the grow-and-fade pop.
#[derive(Component, Debug, Clone, Reflect)]
pub struct LockUnlatchGhostMarker {
    /// Seconds since the ghost spawned.
    pub age: f32,
    /// Which slot popped (drives color and start size).
    pub combat: bool,
}

/// The centered radar-denied flash node; `remaining` counts down its life
/// (zero or below = hidden).
#[derive(Component, Debug, Clone, Reflect)]
pub struct RadarDenyFlashMarker {
    /// Seconds of flash left.
    pub remaining: f32,
}

/// The crosshairs layer: the travel crosshair + the radar box (both
/// screen-indicator nodes, hidden while their anchors are `None`) and the
/// deny flash. Unlatch ghosts are spawned into this layer at runtime.
pub fn lock_crosshairs_hud(target_sprite: Handle<Image>) -> impl Bundle {
    (
        Name::new("LockCrosshairsHUD"),
        LockCrosshairsHudMarker,
        LockGhostSprite(target_sprite.clone()),
        screen_indicator_layer(),
        children![
            (
                Name::new("TravelCrosshair"),
                TravelCrosshairMarker,
                screen_indicator(ScreenIndicatorConfig {
                    anchor: None,
                    size: ScreenIndicatorSize::ApparentSize {
                        min_px: TRAVEL_CROSSHAIR_MIN_PX,
                        // Rendered a step LARGER than the combat reticle
                        // (scale 1.0) so an overlapped pair on a big/close
                        // body stays two concentric rings instead of two
                        // same-size sprites shimmering over each other
                        // (playtest 2026-07-13).
                        scale: TRAVEL_CROSSHAIR_SCALE,
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
                // Hollow: a border-only box around the solid committed
                // crosshair - "the radar is live and retargeting".
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
                Name::new("RadarDenyFlash"),
                RadarDenyFlashMarker { remaining: 0.0 },
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    top: Val::Percent(50.0),
                    width: Val::Px(RADAR_BOX_PX),
                    height: Val::Px(RADAR_BOX_PX),
                    margin: UiRect {
                        left: Val::Px(-RADAR_BOX_PX * 0.5),
                        top: Val::Px(-RADAR_BOX_PX * 0.5),
                        ..default()
                    },
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BorderColor::all(RADAR_COMBAT_COLOR),
                Visibility::Hidden,
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
        app.register_type::<LockUnlatchGhostMarker>();
        app.register_type::<RadarDenyFlashMarker>();
        app.register_type::<LockGhostSprite>();
        app.add_systems(
            Update,
            (
                style_radar_box,
                drive_travel_crosshair,
                drive_radar_candidate,
                spawn_unlatch_ghosts,
                fade_unlatch_ghosts,
                flash_radar_deny,
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

/// The radar-active adornment: while a gesture is ENGAGED the hollow box
/// rides the LIVE LOCK - the engaged slot's current target - not the raw
/// candidate (keep-last means the candidate can be `None` over empty space
/// while the lock still holds; the adornment must not blink there). Inside
/// the tap window nothing renders (F11). The label is DISTANCE ONLY and
/// identical for both slots (playtest 2026-07-13: name + distance read as
/// clutter, and combat/travel behaving differently read as a bug; the
/// target's name lives on the inset viewfinder's faction line and the
/// readout).
#[allow(clippy::type_complexity)]
fn drive_radar_candidate(
    q_player: Query<
        (
            &GlobalTransform,
            Option<&RadarState>,
            &TravelLock,
            &CombatLock,
        ),
        With<PlayerSpaceshipMarker>,
    >,
    q_positions: Query<&GlobalTransform>,
    mut q_box: Query<
        (&mut ScreenIndicatorAnchor, &mut BorderColor, &Children),
        With<RadarCandidateMarker>,
    >,
    mut q_label: Query<(&mut Text, &mut TextColor), With<RadarCandidateLabelMarker>>,
) {
    let player = q_player.iter().next();
    let engaged = player
        .and_then(|(_, radar, ..)| radar.copied())
        .and_then(|radar| radar.engaged.map(|slot| (radar, slot)));
    for (mut anchor, mut border, children) in &mut q_box {
        let (target, color) = match (player, engaged) {
            (Some((_, _, travel, combat)), Some((radar, slot))) => {
                let slot_target = match slot {
                    RadarSlot::Travel => travel.0,
                    RadarSlot::Combat => combat.0,
                };
                (
                    radar.candidate.or(slot_target),
                    match slot {
                        RadarSlot::Combat => RADAR_COMBAT_COLOR,
                        RadarSlot::Travel => RADAR_TRAVEL_COLOR,
                    },
                )
            }
            _ => (None, RADAR_TRAVEL_COLOR),
        };
        **anchor = target.map(ScreenIndicatorAnchorKind::Entity);
        *border = BorderColor::all(color);
        let label = target
            .and_then(|target| {
                let ship = player.map(|(ship, ..)| ship)?;
                let position = q_positions.get(target).ok()?;
                Some(format!(
                    "{:.0}m",
                    ship.translation().distance(position.translation())
                ))
            })
            .unwrap_or_default();
        for &child in children {
            if let Ok((mut text, mut label_color)) = q_label.get_mut(child) {
                if text.0 != label {
                    text.0 = label.clone();
                }
                label_color.0 = color;
            }
        }
    }
}

/// Spawn an unlatch ghost per tap-clear (Q7a): a crosshair stamp on the
/// cleared target that grows and fades - the wordless "the lock let go".
/// The toast message always carries the target (the tap only fires on a
/// `Some` slot), so there is nothing to pop for a `None`.
fn spawn_unlatch_ghosts(
    mut commands: Commands,
    mut toasts: MessageReader<LockClearedToast>,
    q_layer: Query<(Entity, &LockGhostSprite), With<LockCrosshairsHudMarker>>,
) {
    let Some((layer, sprite)) = q_layer.iter().next() else {
        // No player HUD: drain quietly.
        toasts.read().for_each(|_| {});
        return;
    };
    for toast in toasts.read() {
        let Some(target) = toast.target else {
            continue;
        };
        let (color, size) = if toast.combat {
            (RADAR_COMBAT_COLOR, GHOST_COMBAT_PX)
        } else {
            (TRAVEL_COLOR, GHOST_TRAVEL_PX)
        };
        commands.entity(layer).with_child((
            Name::new("LockUnlatchGhost"),
            LockUnlatchGhostMarker {
                age: 0.0,
                combat: toast.combat,
            },
            screen_indicator(ScreenIndicatorConfig {
                anchor: Some(ScreenIndicatorAnchorKind::Entity(target)),
                size: ScreenIndicatorSize::Fixed(Vec2::splat(size)),
                offset: Vec2::ZERO,
                offscreen: ScreenIndicatorOffscreen::Hide,
            }),
            ImageNode::new(sprite.0.clone()).with_color(color),
        ));
    }
}

/// Grow and fade the unlatch ghosts, despawning them after
/// [`GHOST_SECONDS`]. The widget re-applies `ScreenIndicatorSize` to the
/// node every frame, so the growth mutates the size component, not the
/// node.
fn fade_unlatch_ghosts(
    time: Res<Time>,
    mut commands: Commands,
    mut q_ghosts: Query<(
        Entity,
        &mut LockUnlatchGhostMarker,
        &mut ScreenIndicatorSize,
        &mut ImageNode,
    )>,
) {
    for (ghost, mut marker, mut size, mut image) in &mut q_ghosts {
        marker.age += time.delta_secs();
        let t = marker.age / GHOST_SECONDS;
        if t >= 1.0 {
            commands.entity(ghost).despawn();
            continue;
        }
        let start = if marker.combat {
            GHOST_COMBAT_PX
        } else {
            GHOST_TRAVEL_PX
        };
        *size = ScreenIndicatorSize::Fixed(Vec2::splat(start * (1.0 + GHOST_GROWTH * t)));
        let base = if marker.combat {
            RADAR_COMBAT_COLOR
        } else {
            TRAVEL_COLOR
        };
        image.color = base.with_alpha(base.alpha() * (1.0 - t));
    }
}

/// Flash the centered deny box while [`RadarDenied`] burns down (Q8a): the
/// visual half of the deny cue (the buzz is audio.rs's).
fn flash_radar_deny(
    time: Res<Time>,
    mut denied: MessageReader<RadarDenied>,
    mut q_flash: Query<(&mut RadarDenyFlashMarker, &mut Visibility, &mut BorderColor)>,
) {
    let denied_now = denied.read().next().is_some();
    for (mut flash, mut visibility, mut border) in &mut q_flash {
        if denied_now {
            flash.remaining = DENY_FLASH_SECONDS;
        }
        if flash.remaining > 0.0 {
            flash.remaining -= time.delta_secs();
            let alpha = (flash.remaining / DENY_FLASH_SECONDS).clamp(0.0, 1.0);
            *border = BorderColor::all(RADAR_COMBAT_COLOR.with_alpha(alpha));
            visibility.set_if_neq(Visibility::Visible);
        } else {
            visibility.set_if_neq(Visibility::Hidden);
        }
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

    /// A world with a player (at the origin), the radar box + label, and
    /// one named target 100 u ahead. Returns (world, player, target, box,
    /// label).
    fn box_world() -> (World, Entity, Entity, Entity, Entity) {
        let mut world = World::new();
        let target = world
            .spawn((
                Name::new("SCAVENGER"),
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -100.0)),
            ))
            .id();
        let player = world
            .spawn((
                PlayerSpaceshipMarker,
                GlobalTransform::IDENTITY,
                TravelLock(None),
                CombatLock(None),
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
        (world, player, target, boxed, label)
    }

    fn box_anchor(world: &World, boxed: Entity) -> Option<ScreenIndicatorAnchorKind> {
        **world.entity(boxed).get::<ScreenIndicatorAnchor>().unwrap()
    }

    #[test]
    fn radar_box_rides_the_live_lock_with_a_distance_only_label() {
        let (mut world, player, target, boxed, label) = box_world();

        // Open search inside the tap window: nothing renders (F11).
        world.entity_mut(player).insert(RadarState {
            engaged: None,
            candidate: Some(target),
            acquired: false,
            ..default()
        });
        world.run_system_once(drive_radar_candidate).unwrap();
        assert_eq!(
            box_anchor(&world, boxed),
            None,
            "inside the tap window nothing renders (F11)"
        );

        // Engaged travel sweep: the box rides the pick, white, and the
        // label is DISTANCE ONLY (playtest 2026-07-13 - the name read as
        // clutter; it lives on the inset's faction line now).
        world.get_mut::<RadarState>(player).unwrap().engaged = Some(RadarSlot::Travel);
        world.get_mut::<TravelLock>(player).unwrap().0 = Some(target);
        world.run_system_once(drive_radar_candidate).unwrap();
        assert_eq!(
            box_anchor(&world, boxed),
            Some(ScreenIndicatorAnchorKind::Entity(target))
        );
        assert_eq!(
            world.entity(label).get::<Text>().unwrap().0,
            "100m",
            "the sweep label is distance only"
        );

        // Keep-last: the candidate drops over empty space but the lock
        // holds - the adornment must ride the lock, not blink.
        world.get_mut::<RadarState>(player).unwrap().candidate = None;
        world.run_system_once(drive_radar_candidate).unwrap();
        assert_eq!(
            box_anchor(&world, boxed),
            Some(ScreenIndicatorAnchorKind::Entity(target)),
            "keep-last: the adornment rides the held lock over empty space"
        );

        // A combat sweep goes red with the SAME distance-only label - the
        // slots no longer differ (the old asymmetry read as a bug).
        world.get_mut::<RadarState>(player).unwrap().engaged = Some(RadarSlot::Combat);
        world.get_mut::<RadarState>(player).unwrap().candidate = Some(target);
        world.get_mut::<CombatLock>(player).unwrap().0 = Some(target);
        world.run_system_once(drive_radar_candidate).unwrap();
        assert_eq!(
            *world.entity(boxed).get::<BorderColor>().unwrap(),
            BorderColor::all(RADAR_COMBAT_COLOR)
        );
        assert_eq!(
            world.entity(label).get::<Text>().unwrap().0,
            "100m",
            "combat sweeps carry the same distance-only label"
        );

        // Search closed: the box hides.
        world.entity_mut(player).remove::<RadarState>();
        world.run_system_once(drive_radar_candidate).unwrap();
        assert_eq!(box_anchor(&world, boxed), None);
    }

    #[test]
    fn unlatch_ghosts_pop_grow_and_expire() {
        let mut world = World::new();
        world.init_resource::<Messages<LockClearedToast>>();
        world.insert_resource(Time::<()>::default());
        let target = world.spawn_empty().id();
        world.spawn((LockCrosshairsHudMarker, LockGhostSprite(Handle::default())));

        world
            .resource_mut::<Messages<LockClearedToast>>()
            .write(LockClearedToast {
                combat: true,
                target: Some(target),
            });
        world.run_system_once(spawn_unlatch_ghosts).unwrap();

        let (ghost, anchor, size) = {
            let mut q = world.query::<(
                Entity,
                &ScreenIndicatorAnchor,
                &ScreenIndicatorSize,
                &LockUnlatchGhostMarker,
            )>();
            let (ghost, anchor, size, marker) = q.iter(&world).next().expect("a ghost spawned");
            assert!(marker.combat);
            (ghost, **anchor, *size)
        };
        assert_eq!(
            anchor,
            Some(ScreenIndicatorAnchorKind::Entity(target)),
            "the ghost pops where the crosshair was"
        );
        assert_eq!(
            size,
            ScreenIndicatorSize::Fixed(Vec2::splat(GHOST_COMBAT_PX)),
            "the ghost starts at the combat reticle size"
        );

        // Age it past its life: it despawns (the growth in between is the
        // same code path; the terminal state is the contract).
        world.get_mut::<LockUnlatchGhostMarker>(ghost).unwrap().age = GHOST_SECONDS + 0.01;
        world.run_system_once(fade_unlatch_ghosts).unwrap();
        assert!(
            world.get_entity(ghost).is_err(),
            "an expired ghost despawns"
        );
    }

    #[test]
    fn the_deny_flash_lights_and_burns_down() {
        let mut world = World::new();
        world.init_resource::<Messages<RadarDenied>>();
        world.insert_resource(Time::<()>::default());
        let flash = world
            .spawn((
                RadarDenyFlashMarker { remaining: 0.0 },
                Visibility::Hidden,
                BorderColor::all(RADAR_COMBAT_COLOR),
            ))
            .id();
        // Registered ONCE so the MessageReader cursor persists across runs
        // (run_system_once rebuilds the system and would re-read the same
        // message - the registered-system lesson, LESSONS.md).
        let system = world.register_system(flash_radar_deny);

        world
            .resource_mut::<Messages<RadarDenied>>()
            .write(RadarDenied);
        world.run_system(system).unwrap();
        assert_eq!(
            *world.entity(flash).get::<Visibility>().unwrap(),
            Visibility::Visible,
            "a denied hold flashes the box"
        );

        // Burn it down: with the message consumed and the timer forced out,
        // the flash hides again (delivery guard above proves it lit).
        world
            .get_mut::<RadarDenyFlashMarker>(flash)
            .unwrap()
            .remaining = -1.0;
        world.run_system(system).unwrap();
        assert_eq!(
            *world.entity(flash).get::<Visibility>().unwrap(),
            Visibility::Hidden
        );
    }
}
