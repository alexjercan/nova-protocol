//! Target inset: a corner HUD panel showing a live, magnified render-to-texture
//! close-up of the currently focused/locked body - a ship, torpedo or asteroid
//! flagged [`InsetZoomable`], but not a nav beacon - so the player can see which
//! section the fine-lock is selecting (and watch it take damage / explode
//! scope-style) instead of squinting at sub-pixel markers at range
//! (task 20260710-104421 + scope refinement 20260712-203345; design in
//! docs/spikes/20260710-104011-target-inset-view.md, Option A).
//!
//! Three pieces, all thin consumers of the existing targeting state
//! (input/targeting.rs) - this module adds no new targeting mechanics:
//!
//! - A second `Camera3d` that renders the live scene into an `Image` via the
//!   standalone [`RenderTarget`] component (the Bevy 0.19 RTT path, distinct
//!   from a second window-targeting camera, which blacks out the scene). The
//!   probe in task 20260710-104421 confirmed RTT coexists with the main
//!   camera's per-camera post-processing + skybox and trips none of the
//!   marker-filtered `Single<Camera>` queries.
//! - A corner [`ImageNode`] panel showing that texture, spawned with the player
//!   HUD (hud/mod.rs observers) and shown whenever a COMBAT LOCK exists.
//! - An in-scene emissive overlay on the fine-locked section, so the selection
//!   reads in BOTH the main view and the inset with no projection code.
//!
//! INSET-ON-LOCK (spike 20260713-110039 B1): the camera spawns/despawns and
//! the panel shows/hides with the [`CombatLock`] itself - during a radar
//! sweep the panel is the VIEWFINDER, and its presence is the "torpedoes
//! are guided" signal. The focus dwell gates only the component fine-lock
//! now. A lock on a non-zoomable body (beacon) holds the panel with the
//! NO-SIGNAL overlay instead of blinking (Q4a); the frame color + armed
//! corner ticks carry the weapons-safety state (Q5a). The camera is posed
//! each frame on the locked ship's [`live_structure_anchor`] from a
//! scope-like player-relative bearing.

use avian3d::prelude::{ColliderAabb, ComputedCenterOfMass, Sensor};
use bevy::{camera::RenderTarget, prelude::*, render::render_resource::TextureFormat};

use super::screen_indicator::target_world_aabb;
use crate::prelude::*;

pub mod prelude {
    pub use super::{
        target_inset_hud, InsetZoomable, TargetInsetArmedTickMarker, TargetInsetCameraMarker,
        TargetInsetCaptionMarker, TargetInsetHighlightAssets, TargetInsetHighlightMarker,
        TargetInsetHudMarker, TargetInsetHudPlugin, TargetInsetKillCam, TargetInsetLastFramed,
        TargetInsetNoSignalMarker, TargetInsetRenderTarget,
    };
}

/// Opt-in flag for bodies the target inset is allowed to scope: the lockable
/// physical/combat bodies (ships, committed torpedoes, asteroids), but NOT nav
/// beacons - a waypoint is not worth a close-up (user decision 2026-07-12,
/// spike 20260712-203235). Authored by observers on the kind markers
/// (SpaceshipRootMarker, TorpedoTargetChosen) and on asteroids in nova_scenario.
#[derive(Component, Debug, Clone, Reflect)]
pub struct InsetZoomable;

/// Square resolution (px) of the offscreen render texture. Small on purpose:
/// the inset renders the scene a second time, so it stays cheap.
const INSET_TEXTURE_PX: u32 = 512;

/// On-screen size (px) of the inset panel.
const INSET_PANEL_PX: f32 = 256.0;

/// Panel inset from the screen's right edge (px).
const INSET_MARGIN_PX: f32 = 12.0;

/// Panel inset from the screen's top edge (px): pushed below the bcs
/// status bar (FPS/latency row, top-right at 10 px - bcs ui/status.rs),
/// which the panel used to overlap (playtest, 2026-07-13). A feel knob.
const INSET_TOP_PX: f32 = 44.0;

/// Panel border thickness (px).
const INSET_BORDER_PX: f32 = 2.0;

/// Panel border tint while the weapons are HOT: the hot-metal lock red the
/// component markers use (`hud/component_lock.rs` MARKER_SELECTED_COLOR), so
/// the inset reads as part of the targeting family. The frame carries the
/// safety state (Q5a of spike 20260713-110039): this red + the armed corner
/// ticks while hot, the neutral tint below while safe.
const INSET_BORDER_HOT_COLOR: Color = Color::srgba(1.0, 0.45, 0.3, 0.95);

/// Panel border tint while the weapons are SAFE: quiet steel.
const INSET_BORDER_SAFE_COLOR: Color = Color::srgba(0.65, 0.7, 0.75, 0.8);

/// Armed corner tick size and thickness (px): four bars that appear at the
/// panel corners while the weapons are hot - the SHAPE half of the Q5a
/// shape+color redundancy (colorblind-safe).
const INSET_TICK_LEN_PX: f32 = 16.0;
const INSET_TICK_THICK_PX: f32 = 4.0;

/// Faction-line colors (playtest 2026-07-13): the relation palette the
/// retired reticle tint used, now living on the inset's rich surface.
const FACTION_HOSTILE_COLOR: Color = nova_ui::theme::semantic::THREAT;
const FACTION_OWN_COLOR: Color = nova_ui::theme::semantic::ALLY;
const FACTION_NEUTRAL_COLOR: Color = nova_ui::theme::semantic::NEUTRAL;

/// NO-SIGNAL overlay (Q4a): shown when a combat lock exists on a body the
/// inset cannot scope (a beacon - lockable, never zoomable), so the panel
/// holds steady instead of blinking during a sweep across it. Text-free: a
/// near-opaque dark cover with a pulsing hollow square.
const NO_SIGNAL_COVER_COLOR: Color = Color::srgba(0.02, 0.02, 0.035, 0.96);
const NO_SIGNAL_PULSE_HZ: f32 = 1.6;
const NO_SIGNAL_BOX_PX: f32 = 48.0;

/// Inset camera background (no skybox on the inset: a dark clear makes the
/// locked ship stand out, and avoids plumbing the scenario cubemap handle into
/// gameplay). A deep near-black blue.
const INSET_CLEAR_COLOR: Color = Color::srgb(0.02, 0.02, 0.035);

/// Half the size of a section's unit box (`Collider::cuboid(1,1,1)` /
/// `Cuboid::new(1,1,1)`, sections/base_section.rs): the framing radius pads the
/// section-center spread by this so the hull edge, not its center, frames.
const SECTION_HALF_EXTENT: f32 = 0.5;

/// Camera pull-back as a multiple of the target's framing radius: how much of
/// the panel the ship fills. A feel knob.
const INSET_FRAME_PADDING: f32 = 2.2;

/// Floor on the inset camera distance (world units), so a tiny single-section
/// wreck does not clip into the near plane. A feel knob.
const INSET_MIN_DISTANCE: f32 = 6.0;

/// Camera elevation as a fraction of its distance: a slight top-down tilt so
/// the hull reads instead of an edge-on silhouette. A feel knob.
const INSET_ELEVATION: f32 = 0.3;

/// Scale of the emissive highlight shell around the selected section's unit
/// box: slightly larger so it reads as an outline glow rather than replacing
/// the section. A feel knob.
const HIGHLIGHT_SCALE: f32 = 1.14;

/// Marker for the inset panel root (the `ImageNode`).
#[derive(Component, Debug, Clone, Reflect)]
pub struct TargetInsetHudMarker;

/// Marker for the NO-SIGNAL overlay child (Q4a).
#[derive(Component, Debug, Clone, Reflect)]
pub struct TargetInsetNoSignalMarker;

/// Marker for the pulsing hollow square inside the NO-SIGNAL overlay.
#[derive(Component, Debug, Clone, Reflect)]
struct TargetInsetNoSignalPulseMarker;

/// Marker for one armed corner tick (four exist; visible while hot, Q5a).
#[derive(Component, Debug, Clone, Reflect)]
pub struct TargetInsetArmedTickMarker;

/// Marker for the viewfinder's faction line: the locked target's name +
/// relation tag, colored by relation (playtest 2026-07-13 - the rich home
/// of the information the retired reticle relation-tint carried).
#[derive(Component, Debug, Clone, Reflect)]
pub struct TargetInsetCaptionMarker;

/// The panel's memory of the last camera-framed target and pose - the kill
/// cam's source material. Lives ON the panel entity (not a Local), so a
/// HUD respawn starts clean and a player-death teardown takes it along.
#[derive(Component, Debug, Clone, Reflect)]
pub struct TargetInsetLastFramed {
    /// The target that was framed.
    pub target: Entity,
    /// The camera pose it was framed with.
    pub pose: Transform,
}

/// The kill cam (spike 20260713-154023, option B): the framed target DIED,
/// so the panel holds this frozen pose while the fragments fly, then
/// closes. Presentation-only.
#[derive(Component, Debug, Clone, Reflect)]
pub struct TargetInsetKillCam {
    /// The frozen final camera pose.
    pub pose: Transform,
    /// Seconds of linger left.
    pub remaining: f32,
}

/// How long the kill cam holds the final shot (seconds). A feel knob:
/// long enough to watch the fragments scatter, short enough that the
/// panel never feels stuck.
const KILL_CAM_SECS: f32 = 2.0;

/// Marker for the offscreen inset camera. Deliberately carries none of the
/// scene-camera markers (SpaceshipCameraController, ScenarioCameraMarker,
/// WASDCameraController, ScreenIndicatorCamera, SfxListenerMarker), so it trips
/// no marker-filtered `Single<Camera>` query and is not a projection/audio
/// camera - just an RTT source.
#[derive(Component, Debug, Clone, Reflect)]
pub struct TargetInsetCameraMarker;

/// Marker for the emissive overlay spawned as a child of the fine-locked
/// section.
#[derive(Component, Debug, Clone, Reflect)]
pub struct TargetInsetHighlightMarker;

/// The section a highlight overlay belongs to (its parent), so the reconcile
/// can match live overlays against the current selection.
#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct TargetInsetHighlightOf(pub Entity);

/// The render-target image the inset camera draws into and the panel displays.
/// `None` until the player HUD sets it up (`Assets<Image>` exists at runtime,
/// not necessarily at plugin build).
#[derive(Resource, Debug, Clone, Default, Deref, DerefMut)]
pub struct TargetInsetRenderTarget(pub Option<Handle<Image>>);

/// Shared mesh + material for the section highlight shell, built once with the
/// player HUD so the reconcile allocates nothing per selection change.
#[derive(Resource, Debug, Clone)]
pub struct TargetInsetHighlightAssets {
    pub mesh: Handle<Mesh>,
    pub material: Handle<StandardMaterial>,
}

/// The emissive shell material: an unlit, additive-looking translucent red that
/// blooms in both the main view and the inset. Double-sided with no culling so
/// the shell reads as a glow around the section rather than a solid block.
pub fn highlight_material() -> StandardMaterial {
    StandardMaterial {
        base_color: Color::srgba(1.0, 0.35, 0.25, 0.22),
        emissive: LinearRgba::rgb(3.0, 0.7, 0.4),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        double_sided: true,
        cull_mode: None,
        ..default()
    }
}

/// Create the offscreen render target. Rgba8UnormSrgb with no view-format
/// override; `new_target_texture` sets the RENDER_ATTACHMENT | TEXTURE_BINDING
/// | COPY_DST usages.
///
/// Bevy's 3d/render_to_texture example uses Rgba8Unorm storage with an
/// Rgba8UnormSrgb view instead, but a `Some` view format fills the texture's
/// `view_formats`, and creating such a texture needs
/// `DownlevelFlags::VIEW_FORMATS` - absent on WebGL2, where it is a fatal
/// render validation error the moment the player HUD spawns. An sRGB-format
/// target with the default view goes through the same Rgba8UnormSrgb view end
/// to end, so native rendering is unchanged.
pub fn create_render_target(images: &mut Assets<Image>) -> Handle<Image> {
    let image = Image::new_target_texture(
        INSET_TEXTURE_PX,
        INSET_TEXTURE_PX,
        TextureFormat::Rgba8UnormSrgb,
        None,
    );
    images.add(image)
}

/// One armed corner tick: a small bar hugging a panel corner, hidden until
/// the weapons go hot (Q5a). `horizontal` picks the bar orientation;
/// `(right, bottom)` pick the corner.
fn armed_tick(horizontal: bool, right: bool, bottom: bool) -> impl Bundle {
    let (width, height) = if horizontal {
        (Val::Px(INSET_TICK_LEN_PX), Val::Px(INSET_TICK_THICK_PX))
    } else {
        (Val::Px(INSET_TICK_THICK_PX), Val::Px(INSET_TICK_LEN_PX))
    };
    let offset = Val::Px(2.0);
    let auto = Val::Auto;
    (
        Name::new("ArmedTick"),
        TargetInsetArmedTickMarker,
        Node {
            position_type: PositionType::Absolute,
            left: if right { auto } else { offset },
            right: if right { offset } else { auto },
            top: if bottom { auto } else { offset },
            bottom: if bottom { offset } else { auto },
            width,
            height,
            ..default()
        },
        BackgroundColor(INSET_BORDER_HOT_COLOR),
        Visibility::Hidden,
    )
}

/// The inset panel bundle: a corner-anchored node showing the render target,
/// starting Hidden (the lock-driven reconcile reveals it). `Chrome` tier +
/// `HudSelfDrivenVisibility` so it follows the HUD level yet the lock
/// reconcile owns its moment-to-moment visibility (the gravity-sphere
/// pattern). Children: the NO-SIGNAL overlay (Q4a), eight armed corner
/// ticks (Q5a - two per corner, an L each) and the viewfinder caption
/// (Q6a).
pub fn target_inset_hud(image: Handle<Image>) -> impl Bundle {
    (
        Name::new("TargetInsetHUD"),
        TargetInsetHudMarker,
        HudTier::Chrome,
        HudSelfDrivenVisibility,
        Node {
            position_type: PositionType::Absolute,
            // Top-right, below the status bar: clear of the FPS/latency row
            // (top-right), the objectives column (mid-right), the keybind
            // hints (bottom-left) and the dev inspector overlay (top-left).
            right: Val::Px(INSET_MARGIN_PX),
            top: Val::Px(INSET_TOP_PX),
            width: Val::Px(INSET_PANEL_PX),
            height: Val::Px(INSET_PANEL_PX),
            border: UiRect::all(Val::Px(INSET_BORDER_PX)),
            ..default()
        },
        BorderColor::all(INSET_BORDER_SAFE_COLOR),
        ImageNode::new(image),
        Visibility::Hidden,
        children![
            (
                Name::new("NoSignal"),
                TargetInsetNoSignalMarker,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(NO_SIGNAL_COVER_COLOR),
                Visibility::Hidden,
                children![(
                    Name::new("NoSignalPulse"),
                    TargetInsetNoSignalPulseMarker,
                    Node {
                        width: Val::Px(NO_SIGNAL_BOX_PX),
                        height: Val::Px(NO_SIGNAL_BOX_PX),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BorderColor::all(INSET_BORDER_SAFE_COLOR),
                )],
            ),
            (
                Name::new("InsetFactionLine"),
                TargetInsetCaptionMarker,
                Text::new(""),
                TextFont::from_font_size(12.0),
                TextColor(INSET_BORDER_SAFE_COLOR),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(6.0),
                    bottom: Val::Px(4.0),
                    ..default()
                },
            ),
            armed_tick(true, false, false),
            armed_tick(false, false, false),
            armed_tick(true, true, false),
            armed_tick(false, true, false),
            armed_tick(true, false, true),
            armed_tick(false, false, true),
            armed_tick(true, true, true),
            armed_tick(false, true, true),
        ],
    )
}

#[derive(Default)]
pub struct TargetInsetHudPlugin;

impl Plugin for TargetInsetHudPlugin {
    fn build(&self, app: &mut App) {
        debug!("TargetInsetHudPlugin: build");

        app.init_resource::<TargetInsetRenderTarget>();
        app.register_type::<TargetInsetHudMarker>();
        app.register_type::<TargetInsetCameraMarker>();
        app.register_type::<TargetInsetHighlightMarker>();
        app.register_type::<TargetInsetHighlightOf>();
        app.register_type::<InsetZoomable>();
        app.register_type::<TargetInsetNoSignalMarker>();
        app.register_type::<TargetInsetArmedTickMarker>();
        app.register_type::<TargetInsetCaptionMarker>();
        app.register_type::<TargetInsetLastFramed>();
        app.register_type::<TargetInsetKillCam>();

        // Author the zoomable flag on the kinds worth scoping as they spawn
        // (ships, committed torpedoes). Asteroids get it in nova_scenario;
        // beacons deliberately never do. Observer-per-kind mirrors the
        // scenario loader's on_add_entity_with pattern.
        app.add_observer(mark_inset_zoomable::<SpaceshipRootMarker>);
        app.add_observer(mark_inset_zoomable::<TorpedoTargetChosen>);

        app.add_systems(
            Update,
            (
                drive_inset_camera,
                drive_inset_frame_state,
                pulse_no_signal,
                sync_section_highlight,
            )
                .in_set(super::NovaHudSystems),
        );
    }
}

/// Insert [`InsetZoomable`] on any entity that gains the kind marker `T`, so
/// the inset may scope it. Idempotent (inserting a second time is a no-op).
fn mark_inset_zoomable<T: Component>(add: On<Add, T>, mut commands: Commands) {
    if let Ok(mut entity) = commands.get_entity(add.entity) {
        entity.insert(InsetZoomable);
    }
}

/// The scope-like inset camera pose: sit on the line from the target toward the
/// player (so the panel shows the face the player is shooting), pulled back by
/// the framing distance and lifted a little for a readable hull angle.
fn inset_camera_pose(target_anchor: Vec3, player_anchor: Vec3, radius: f32) -> Transform {
    let bearing = (player_anchor - target_anchor)
        .try_normalize()
        .unwrap_or(Vec3::Z);
    let distance = (radius * INSET_FRAME_PADDING).max(INSET_MIN_DISTANCE);
    let eye = target_anchor + bearing * distance + Vec3::Y * (distance * INSET_ELEVATION);
    Transform::from_translation(eye).looking_at(target_anchor, Vec3::Y)
}

/// Framing radius of the target from `anchor`: the distance from the anchor to
/// the farthest corner of the union of the body's non-sensor collider AABBs
/// ([`target_world_aabb`], which walks the subtree so a ship's section colliders
/// and a section-less torpedo/asteroid's own collider are both covered). Falls
/// back to the section half-extent when the body has no collider AABB (test
/// entities, or a body that has not built its colliders yet), keeping the pose
/// finite.
fn zoomable_framing_radius(
    target: Entity,
    anchor: Vec3,
    q_children: &Query<&Children>,
    q_aabb: &Query<&ColliderAabb, Without<Sensor>>,
) -> f32 {
    match target_world_aabb(target, q_children, q_aabb) {
        Some(aabb) => {
            let center = 0.5 * (aabb.min + aabb.max);
            let half_diagonal = 0.5 * (aabb.max - aabb.min).length();
            anchor.distance(center) + half_diagonal
        }
        None => SECTION_HALF_EXTENT,
    }
}

/// The inset camera bundle. Order -1 renders it before the main (order 0)
/// window camera into its own image target. Carries `PostProcessingCamera` so
/// its tonemapping/bloom look matches the main view (thruster glow, explosions);
/// no skybox (see [`INSET_CLEAR_COLOR`]).
fn inset_camera_bundle(image: Handle<Image>, pose: Transform) -> impl Bundle {
    (
        Name::new("Target Inset Camera"),
        TargetInsetCameraMarker,
        Camera3d::default(),
        Camera {
            order: -1,
            clear_color: ClearColorConfig::Custom(INSET_CLEAR_COLOR),
            ..default()
        },
        RenderTarget::Image(image.into()),
        pose,
        PostProcessingCamera,
    )
}

/// What the panel should do this frame, resolved before any side effects.
enum InsetPanelState {
    /// Live camera framing on a lock.
    Live { target: Entity, anchor: Vec3 },
    /// A lock on a non-zoomable body: panel holds with NO-SIGNAL.
    NoSignal,
    /// The framed target DIED: hold the frozen final shot (spike
    /// 20260713-154023 option B).
    KillCam { pose: Transform },
    /// Nothing to show.
    Hidden,
}

/// Spawn/despawn the inset camera and show/hide the panel with the COMBAT
/// LOCK (inset-on-lock, spike 20260713-110039 B1, user-confirmed: presence
/// of the inset IS the "not dumb-fire" signal, and during a radar sweep it
/// is the viewfinder). The focus dwell no longer gates the panel - it keeps
/// gating only the component fine-lock. One idempotent system (like the
/// component-marker reconcile) so every ordering of lock/section changes
/// converges; folding the lifecycle and the pose together avoids a
/// one-frame default-pose flash on spawn.
///
/// Four states: no lock (or chrome hidden) = panel hidden, camera gone;
/// lock on a zoomable, resolvable body = panel + live camera; lock on a
/// NON-zoomable body (a beacon) = panel with the NO-SIGNAL overlay, camera
/// gone (Q4a); and the KILL CAM (spike 20260713-154023): when the framed
/// target dies - it is DESPAWNED, the discriminator against tap-clear /
/// decay / allegiance-flip clears, whose targets remain alive - the panel
/// and camera hold the frozen final pose for [`KILL_CAM_SECS`], filming
/// the explosion fragments, then close. A fresh framable lock preempts the
/// linger instantly; hiding the HUD chrome tears everything down at once.
/// Presentation-only: no lock/safety/turret state is touched.
#[allow(clippy::type_complexity)]
fn drive_inset_camera(
    mut commands: Commands,
    time: Res<Time>,
    hud_visibility: Res<super::HudVisibility>,
    render_target: Res<TargetInsetRenderTarget>,
    q_anchor: Query<
        (
            &Transform,
            Option<&ComputedCenterOfMass>,
            Has<InsetZoomable>,
        ),
        Without<TargetInsetCameraMarker>,
    >,
    q_player: Query<
        (&Transform, Option<&ComputedCenterOfMass>, &CombatLock),
        (
            With<SpaceshipRootMarker>,
            With<PlayerSpaceshipMarker>,
            Without<TargetInsetCameraMarker>,
        ),
    >,
    q_children: Query<&Children>,
    q_aabb: Query<&ColliderAabb, Without<Sensor>>,
    q_alive: Query<Entity>,
    mut q_camera: Query<(Entity, &mut Transform), With<TargetInsetCameraMarker>>,
    mut q_panel: Query<
        (
            Entity,
            &mut Visibility,
            Option<&TargetInsetLastFramed>,
            Option<&mut TargetInsetKillCam>,
        ),
        With<TargetInsetHudMarker>,
    >,
    mut q_no_signal: Query<
        &mut Visibility,
        (
            With<TargetInsetNoSignalMarker>,
            Without<TargetInsetHudMarker>,
        ),
    >,
) {
    let chrome = hud_visibility.shows(HudTier::Chrome);
    let lock = q_player
        .iter()
        .next()
        .and_then(|(_, _, lock)| lock.0)
        .filter(|_| chrome);
    // `Some(Some(anchor))` = camera framing; `Some(None)` = NO-SIGNAL;
    // `None` = teardown-eligible.
    let framed = lock.map(|target| match q_anchor.get(target) {
        Ok((transform, com, true)) => Some((target, live_structure_anchor(transform, com))),
        // Not zoomable (beacon) or unresolved: the panel holds, no camera.
        _ => None,
    });

    let Some((panel, mut panel_visibility, last_framed, mut kill_cam)) = q_panel.iter_mut().next()
    else {
        return;
    };

    let state = match framed {
        Some(Some((target, anchor))) => InsetPanelState::Live { target, anchor },
        Some(None) => InsetPanelState::NoSignal,
        None => {
            if !chrome {
                // Chrome hidden: everything down at once, including a
                // running kill cam and the frame memory.
                InsetPanelState::Hidden
            } else if let Some(kill_cam) = kill_cam.as_mut() {
                kill_cam.remaining -= time.delta_secs();
                if kill_cam.remaining > 0.0 {
                    InsetPanelState::KillCam {
                        pose: kill_cam.pose,
                    }
                } else {
                    InsetPanelState::Hidden
                }
            } else if let Some(last) = last_framed.filter(|last| !q_alive.contains(last.target)) {
                // The framed target is GONE from the world: the death
                // discriminator (a cleared-but-alive target closes as
                // always). Enter the kill cam on its final pose.
                commands.entity(panel).insert(TargetInsetKillCam {
                    pose: last.pose,
                    remaining: KILL_CAM_SECS,
                });
                InsetPanelState::KillCam { pose: last.pose }
            } else {
                InsetPanelState::Hidden
            }
        }
    };

    // State bookkeeping: the frame memory exists only while live-framed
    // (stale memory must not resurrect a linger later); the kill cam ends
    // whenever anything other than KillCam is showing.
    match &state {
        InsetPanelState::Live { .. } => {
            if kill_cam.is_some() {
                commands.entity(panel).remove::<TargetInsetKillCam>();
            }
        }
        InsetPanelState::KillCam { .. } => {}
        InsetPanelState::NoSignal | InsetPanelState::Hidden => {
            if kill_cam.is_some() {
                commands.entity(panel).remove::<TargetInsetKillCam>();
            }
            if last_framed.is_some() {
                commands.entity(panel).remove::<TargetInsetLastFramed>();
            }
        }
    }

    panel_visibility.set_if_neq(if matches!(state, InsetPanelState::Hidden) {
        Visibility::Hidden
    } else {
        Visibility::Visible
    });
    let overlay_visibility = if matches!(state, InsetPanelState::NoSignal) {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    for mut visibility in &mut q_no_signal {
        visibility.set_if_neq(overlay_visibility);
    }

    let pose = match state {
        InsetPanelState::Live { target, anchor } => {
            let player_anchor = q_player
                .iter()
                .next()
                .map(|(transform, com, ..)| live_structure_anchor(transform, com))
                // No player anchor (teardown): fall back to a fixed bearing
                // so the pose stays finite rather than degenerate.
                .unwrap_or(anchor + Vec3::Z);
            let radius = zoomable_framing_radius(target, anchor, &q_children, &q_aabb);
            let pose = inset_camera_pose(anchor, player_anchor, radius);
            commands
                .entity(panel)
                .insert(TargetInsetLastFramed { target, pose });
            pose
        }
        InsetPanelState::KillCam { pose } => pose,
        InsetPanelState::NoSignal | InsetPanelState::Hidden => {
            // No second render: tear the camera down.
            for (camera, _) in &q_camera {
                commands.entity(camera).despawn();
            }
            return;
        }
    };

    if let Ok((_, mut transform)) = q_camera.single_mut() {
        *transform = pose;
    } else if let Some(image) = render_target.0.clone() {
        commands.spawn(inset_camera_bundle(image, pose));
    }
}

/// The frame carries the safety state (Q5a, shape + color): hot = the lock
/// red border + the armed corner ticks; safe = quiet steel, no ticks. The
/// caption is the FACTION line (playtest 2026-07-13, revising Q6a): the
/// locked target's name + relation, colored by relation - restoring on the
/// RICH surface the information the retired reticle relation-tint carried.
/// The gesture-time name+distance caption is gone (it read as clutter);
/// distance rides the radar box next to the bracket instead.
#[allow(clippy::type_complexity)]
fn drive_inset_frame_state(
    q_player: Query<
        (Option<&Allegiance>, &WeaponsHot, &CombatLock),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    q_names: Query<&Name>,
    q_allegiance: Query<Option<&Allegiance>>,
    mut q_frame: Query<&mut BorderColor, With<TargetInsetHudMarker>>,
    mut q_ticks: Query<
        &mut Visibility,
        (
            With<TargetInsetArmedTickMarker>,
            Without<TargetInsetHudMarker>,
        ),
    >,
    mut q_caption: Query<(&mut Text, &mut TextColor), With<TargetInsetCaptionMarker>>,
) {
    let Some((player_allegiance, hot, lock)) = q_player.iter().next() else {
        return;
    };

    let border = if hot.0 {
        INSET_BORDER_HOT_COLOR
    } else {
        INSET_BORDER_SAFE_COLOR
    };
    for mut frame in &mut q_frame {
        let next = BorderColor::all(border);
        if *frame != next {
            *frame = next;
        }
    }
    let tick_visibility = if hot.0 {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for mut visibility in &mut q_ticks {
        visibility.set_if_neq(tick_visibility);
    }

    let (caption, caption_color) = match lock.0 {
        Some(target) => {
            let name = q_names
                .get(target)
                .map(|name| name.to_string())
                .unwrap_or_else(|_| "CONTACT".to_string());
            let (tag, color) = match q_allegiance
                .get(target)
                .map(|allegiance| relation(player_allegiance, allegiance))
            {
                Ok(Relation::Hostile) => ("HOSTILE", FACTION_HOSTILE_COLOR),
                Ok(Relation::Own) => ("OWN", FACTION_OWN_COLOR),
                // A lock can outlive its entity by a frame; read as neutral.
                Ok(Relation::Neutral) | Err(_) => ("NEUTRAL", FACTION_NEUTRAL_COLOR),
            };
            (format!("{name} - {tag}"), color)
        }
        None => (String::new(), FACTION_NEUTRAL_COLOR),
    };
    for (mut text, mut color) in &mut q_caption {
        if text.0 != caption {
            text.0 = caption.clone();
        }
        if color.0 != caption_color {
            color.0 = caption_color;
        }
    }
}

/// Pulse the NO-SIGNAL hollow square so the overlay reads as "scanning, no
/// visual" rather than a stuck frame. Cheap alpha breathing on real time.
fn pulse_no_signal(
    time: Res<Time>,
    mut q_pulse: Query<&mut BorderColor, With<TargetInsetNoSignalPulseMarker>>,
) {
    let phase = (time.elapsed_secs() * NO_SIGNAL_PULSE_HZ * std::f32::consts::TAU).sin();
    let alpha = 0.35 + 0.4 * (0.5 + 0.5 * phase);
    for mut border in &mut q_pulse {
        *border = BorderColor::all(INSET_BORDER_SAFE_COLOR.with_alpha(alpha));
    }
}

/// Keep exactly one emissive highlight overlay on the fine-locked section, and
/// none otherwise. The selection is already focus-gated by the targeting layer
/// (`ComponentLock.section` is only `Some` while focused), so
/// this reconcile just mirrors it; a detached/despawned section drops its
/// overlay (a despawned section takes its child overlay with it, but a
/// re-selected sibling still needs the stale one cleared).
fn sync_section_highlight(
    mut commands: Commands,
    q_player: Query<&ComponentLock, With<PlayerSpaceshipMarker>>,
    assets: Option<Res<TargetInsetHighlightAssets>>,
    q_highlights: Query<(Entity, &TargetInsetHighlightOf), With<TargetInsetHighlightMarker>>,
    q_sections: Query<(), With<SectionMarker>>,
) {
    // Only highlight a section that still exists (attached or inactive-in-place
    // both keep the SectionMarker; despawn/detach removes it).
    let selected = q_player
        .iter()
        .next()
        .and_then(|component| component.section)
        .filter(|section| q_sections.get(*section).is_ok());

    // Drop overlays that no longer match the selection.
    for (overlay, of) in &q_highlights {
        if selected != Some(**of) {
            commands.entity(overlay).despawn();
        }
    }

    let Some(section) = selected else {
        return;
    };
    let Some(assets) = assets else {
        // Assets not built yet (no player HUD): nothing to spawn with.
        return;
    };
    let already = q_highlights.iter().any(|(_, of)| **of == section);
    if !already {
        commands
            .entity(section)
            .with_child(highlight_bundle(&assets, section));
    }
}

/// One emissive shell child, scaled just past the section's unit box.
fn highlight_bundle(assets: &TargetInsetHighlightAssets, section: Entity) -> impl Bundle {
    (
        Name::new("TargetInsetHighlight"),
        TargetInsetHighlightMarker,
        TargetInsetHighlightOf(section),
        Mesh3d(assets.mesh.clone()),
        MeshMaterial3d(assets.material.clone()),
        Transform::from_scale(Vec3::splat(HIGHLIGHT_SCALE)),
    )
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    // -- render target --

    /// WebGL2-safe invariant of the inset render target (v0.5.0 web crash
    /// regression): a non-empty `view_formats` needs
    /// `DownlevelFlags::VIEW_FORMATS`, which WebGL2 lacks, so
    /// `create_texture` fails validation and Bevy quits the app on game
    /// start. The target must be born plain sRGB with the default view.
    #[test]
    fn render_target_is_webgl2_safe() {
        let mut images = Assets::<Image>::default();
        let handle = create_render_target(&mut images);
        let image = images.get(&handle).expect("render target image exists");
        assert_eq!(
            image.texture_descriptor.format,
            TextureFormat::Rgba8UnormSrgb,
            "target renders and samples as sRGB"
        );
        assert!(
            image.texture_descriptor.view_formats.is_empty(),
            "non-empty view_formats is a fatal validation error on WebGL2"
        );
        assert!(
            image.texture_view_descriptor.is_none(),
            "no view override; the default view already has the sRGB format"
        );
    }

    // -- camera lifecycle --

    /// Build the focused rig: panel + player + focused target with `n`
    /// sections. Returns (world, player, target).
    fn rig(n: usize) -> (World, Entity, Entity) {
        let mut world = World::new();
        world.insert_resource(Time::<()>::default());
        world.insert_resource(super::super::HudVisibility::All);
        world.insert_resource(TargetInsetRenderTarget(Some(Handle::default())));
        world.spawn((Name::new("panel"), TargetInsetHudMarker, Visibility::Hidden));
        world.spawn((
            Name::new("no-signal"),
            TargetInsetNoSignalMarker,
            Visibility::Hidden,
        ));
        let player = world
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::from_xyz(0.0, 0.0, 0.0),
            ))
            .id();
        let target = world
            .spawn((
                SpaceshipRootMarker,
                InsetZoomable,
                Transform::from_xyz(0.0, 0.0, -50.0),
            ))
            .id();
        for i in 0..n {
            world.spawn((
                SectionMarker,
                ChildOf(target),
                GlobalTransform::from_xyz(0.0, 0.0, -50.0 + i as f32),
            ));
        }
        world.entity_mut(player).insert((
            CombatLock(Some(target)),
            LockFocus {
                target: Some(target),
                seconds: f32::MAX,
            },
        ));
        (world, player, target)
    }

    fn camera_count(world: &mut World) -> usize {
        world
            .query_filtered::<(), With<TargetInsetCameraMarker>>()
            .iter(world)
            .count()
    }

    fn panel_visibility(world: &mut World) -> Visibility {
        *world
            .query_filtered::<&Visibility, With<TargetInsetHudMarker>>()
            .iter(world)
            .next()
            .expect("panel exists")
    }

    fn overlay_visibility(world: &mut World) -> Visibility {
        *world
            .query_filtered::<&Visibility, With<TargetInsetNoSignalMarker>>()
            .iter(world)
            .next()
            .expect("overlay exists")
    }

    #[test]
    fn camera_and_panel_appear_the_moment_the_lock_exists() {
        // Inset-on-lock (spike 20260713-110039 B1, user-confirmed): the
        // panel is up whenever a combat lock exists - the focus dwell no
        // longer gates it. The rig's dwell is stripped to zero to prove it
        // (under the old focus gate this rig showed nothing for 1.5 s).
        let (mut world, player, target) = rig(3);
        world.get_mut::<LockFocus>(player).unwrap().seconds = 0.0;

        world.run_system_once(drive_inset_camera).unwrap();
        assert_eq!(
            camera_count(&mut world),
            1,
            "locked (dwell irrelevant): one inset camera"
        );
        assert_eq!(
            panel_visibility(&mut world),
            Visibility::Visible,
            "locked: panel shown at lock time"
        );
        assert_eq!(
            overlay_visibility(&mut world),
            Visibility::Hidden,
            "a live camera framing needs no NO-SIGNAL overlay"
        );

        // Delivery guard: clearing the LOCK tears everything down (the
        // positive state above proves the assertion can differ).
        world.get_mut::<CombatLock>(player).unwrap().0 = None;
        world.run_system_once(drive_inset_camera).unwrap();
        assert_eq!(camera_count(&mut world), 0, "no lock: camera despawned");
        assert_eq!(
            panel_visibility(&mut world),
            Visibility::Hidden,
            "no lock: panel hidden"
        );

        let _ = target;
    }

    #[test]
    fn camera_does_not_duplicate_across_frames() {
        let (mut world, _player, _target) = rig(3);
        world.run_system_once(drive_inset_camera).unwrap();
        world.run_system_once(drive_inset_camera).unwrap();
        world.run_system_once(drive_inset_camera).unwrap();
        assert_eq!(
            camera_count(&mut world),
            1,
            "the reconcile keeps exactly one inset camera"
        );
    }

    #[test]
    fn a_fresh_lock_retargets_the_camera_a_sweep_never_blinks_the_panel() {
        let (mut world, player, _target) = rig(3);
        world.run_system_once(drive_inset_camera).unwrap();
        assert_eq!(camera_count(&mut world), 1);

        // Re-lock to another ZOOMABLE body (a live sweep retarget): the one
        // camera stays, the panel never blinks - the viewfinder.
        let other = world
            .spawn((SpaceshipRootMarker, InsetZoomable, Transform::default()))
            .id();
        world.get_mut::<CombatLock>(player).unwrap().0 = Some(other);
        world.run_system_once(drive_inset_camera).unwrap();
        assert_eq!(camera_count(&mut world), 1, "retarget keeps one camera");
        assert_eq!(panel_visibility(&mut world), Visibility::Visible);
    }

    #[test]
    fn camera_absent_while_hud_chrome_is_hidden() {
        let (mut world, _player, _target) = rig(3);

        // Focused, but the HUD is hiding chrome (Minimal drops Chrome; None
        // drops everything): the inset panel is tier-hidden, so the second
        // render must not run either.
        world.insert_resource(super::super::HudVisibility::None);
        world.run_system_once(drive_inset_camera).unwrap();
        assert_eq!(
            camera_count(&mut world),
            0,
            "hidden chrome: no inset camera renders while the panel is hidden"
        );
        assert_eq!(panel_visibility(&mut world), Visibility::Hidden);

        // Delivery guard: showing chrome again brings the inset back, so the
        // assertion above is really gated on visibility.
        world.insert_resource(super::super::HudVisibility::All);
        world.run_system_once(drive_inset_camera).unwrap();
        assert_eq!(camera_count(&mut world), 1);
        assert_eq!(panel_visibility(&mut world), Visibility::Visible);
    }

    #[test]
    fn a_non_zoomable_lock_holds_the_panel_with_no_signal() {
        // A locked body that is NOT flagged InsetZoomable (a beacon) gets no
        // camera - but the panel HOLDS with the NO-SIGNAL overlay (Q4a), so
        // a sweep crossing a beacon never blinks the viewfinder (F5).
        let (mut world, _player, target) = rig(3);
        world.entity_mut(target).remove::<InsetZoomable>();

        world.run_system_once(drive_inset_camera).unwrap();
        assert_eq!(
            camera_count(&mut world),
            0,
            "a non-zoomable lock (beacon) renders no second view"
        );
        assert_eq!(
            panel_visibility(&mut world),
            Visibility::Visible,
            "the panel holds through the beacon (Q4a)"
        );
        assert_eq!(
            overlay_visibility(&mut world),
            Visibility::Visible,
            "NO-SIGNAL covers the stale render"
        );

        // Delivery guard: flagging it zoomable swaps the overlay for the
        // camera, so the assertions above are really gated on the flag.
        world.entity_mut(target).insert(InsetZoomable);
        world.run_system_once(drive_inset_camera).unwrap();
        assert_eq!(camera_count(&mut world), 1);
        assert_eq!(panel_visibility(&mut world), Visibility::Visible);
        assert_eq!(overlay_visibility(&mut world), Visibility::Hidden);
    }

    fn panel_entity(world: &mut World) -> Entity {
        world
            .query_filtered::<Entity, With<TargetInsetHudMarker>>()
            .iter(world)
            .next()
            .expect("panel exists")
    }

    fn camera_pose(world: &mut World) -> Transform {
        *world
            .query_filtered::<&Transform, With<TargetInsetCameraMarker>>()
            .iter(world)
            .next()
            .expect("inset camera exists")
    }

    #[test]
    fn the_kill_cam_holds_the_final_shot_when_the_target_dies() {
        // Spike 20260713-154023 option B: the framed target DYING (it is
        // despawned) freezes the panel on its final pose for
        // KILL_CAM_SECS instead of slamming shut, then closes.
        let (mut world, player, target) = rig(3);
        world.run_system_once(drive_inset_camera).unwrap();
        let final_pose = camera_pose(&mut world);
        let panel = panel_entity(&mut world);

        // The kill: the target despawns and the validity clear empties the
        // lock in the same breath (production ordering verified in the
        // plan: targeting runs before the HUD).
        world.despawn(target);
        world.get_mut::<CombatLock>(player).unwrap().0 = None;
        world.run_system_once(drive_inset_camera).unwrap();

        assert_eq!(
            panel_visibility(&mut world),
            Visibility::Visible,
            "the panel holds through the death"
        );
        assert_eq!(camera_count(&mut world), 1, "the camera keeps filming");
        assert_eq!(
            camera_pose(&mut world),
            final_pose,
            "the shot is FROZEN at the final pose"
        );
        assert_eq!(
            overlay_visibility(&mut world),
            Visibility::Hidden,
            "no NO-SIGNAL during the kill cam"
        );
        assert!(
            world.get::<TargetInsetKillCam>(panel).is_some(),
            "the kill cam is armed"
        );

        // A second frame mid-linger: still holding (the countdown needs
        // real time; the rig's clock has zero delta).
        world.run_system_once(drive_inset_camera).unwrap();
        assert_eq!(panel_visibility(&mut world), Visibility::Visible);

        // Expiry (forced, the ghost-test shape): everything closes.
        world
            .get_mut::<TargetInsetKillCam>(panel)
            .unwrap()
            .remaining = -1.0;
        world.run_system_once(drive_inset_camera).unwrap();
        assert_eq!(panel_visibility(&mut world), Visibility::Hidden);
        assert_eq!(camera_count(&mut world), 0, "expiry tears the camera down");
        // The state components clear (the removes are deferred one run).
        world.run_system_once(drive_inset_camera).unwrap();
        assert!(world.get::<TargetInsetKillCam>(panel).is_none());
        assert!(world.get::<TargetInsetLastFramed>(panel).is_none());
    }

    #[test]
    fn a_cleared_but_alive_target_does_not_linger() {
        // The discriminator: tap-clear / decay / allegiance-flip leave the
        // target ALIVE - the panel closes as it always did (the death case
        // above is the delivery guard that the linger machinery works).
        let (mut world, player, target) = rig(3);
        world.run_system_once(drive_inset_camera).unwrap();
        let panel = panel_entity(&mut world);

        world.get_mut::<CombatLock>(player).unwrap().0 = None;
        world.run_system_once(drive_inset_camera).unwrap();

        assert_eq!(
            panel_visibility(&mut world),
            Visibility::Hidden,
            "a cleared-but-alive target closes immediately"
        );
        assert_eq!(camera_count(&mut world), 0);
        assert!(world.get::<TargetInsetKillCam>(panel).is_none());
        let _ = target;
    }

    #[test]
    fn a_fresh_lock_preempts_the_kill_cam() {
        let (mut world, player, target) = rig(3);
        world.run_system_once(drive_inset_camera).unwrap();
        let panel = panel_entity(&mut world);

        world.despawn(target);
        world.get_mut::<CombatLock>(player).unwrap().0 = None;
        world.run_system_once(drive_inset_camera).unwrap();
        assert!(
            world.get::<TargetInsetKillCam>(panel).is_some(),
            "delivery guard: the kill cam was running"
        );

        // The live viewfinder always wins: a fresh zoomable lock re-frames.
        let next = world
            .spawn((
                SpaceshipRootMarker,
                InsetZoomable,
                Transform::from_xyz(10.0, 0.0, -30.0),
            ))
            .id();
        world.get_mut::<CombatLock>(player).unwrap().0 = Some(next);
        world.run_system_once(drive_inset_camera).unwrap();

        assert_eq!(camera_count(&mut world), 1);
        assert_eq!(panel_visibility(&mut world), Visibility::Visible);
        // The removes/inserts are deferred: settle one run, then the state
        // reflects the new framing.
        world.run_system_once(drive_inset_camera).unwrap();
        assert!(world.get::<TargetInsetKillCam>(panel).is_none());
        assert_eq!(
            world
                .get::<TargetInsetLastFramed>(panel)
                .map(|last| last.target),
            Some(next),
            "the frame memory follows the new lock"
        );
    }

    #[test]
    fn hiding_the_chrome_ends_the_kill_cam_immediately() {
        let (mut world, player, target) = rig(3);
        world.run_system_once(drive_inset_camera).unwrap();
        let panel = panel_entity(&mut world);

        world.despawn(target);
        world.get_mut::<CombatLock>(player).unwrap().0 = None;
        world.run_system_once(drive_inset_camera).unwrap();
        assert!(
            world.get::<TargetInsetKillCam>(panel).is_some(),
            "delivery guard: the kill cam was running"
        );

        world.insert_resource(super::super::HudVisibility::None);
        world.run_system_once(drive_inset_camera).unwrap();
        assert_eq!(panel_visibility(&mut world), Visibility::Hidden);
        assert_eq!(camera_count(&mut world), 0, "chrome-hide is immediate");
    }

    #[test]
    fn the_frame_carries_the_safety_state_and_the_faction_line() {
        // Q5a (shape + color for the safety state) + the faction line
        // (playtest 2026-07-13): name + relation tag, colored by relation,
        // whenever a lock exists - gesture-independent.
        let mut world = World::new();
        let target = world
            .spawn((
                Name::new("SCAVENGER"),
                Allegiance::Enemy,
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -100.0)),
            ))
            .id();
        let player = world
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::IDENTITY,
                WeaponsHot(false),
                CombatLock(Some(target)),
            ))
            .id();
        let frame = world
            .spawn((
                TargetInsetHudMarker,
                BorderColor::all(INSET_BORDER_SAFE_COLOR),
            ))
            .id();
        let tick = world
            .spawn((TargetInsetArmedTickMarker, Visibility::Hidden))
            .id();
        let caption = world
            .spawn((
                TargetInsetCaptionMarker,
                Text::new(""),
                TextColor(FACTION_NEUTRAL_COLOR),
            ))
            .id();

        // Safe, locked on an enemy: neutral frame, no ticks - but the
        // faction line reads immediately (no gesture needed).
        world.run_system_once(drive_inset_frame_state).unwrap();
        assert_eq!(
            *world.entity(frame).get::<BorderColor>().unwrap(),
            BorderColor::all(INSET_BORDER_SAFE_COLOR)
        );
        assert_eq!(
            *world.entity(tick).get::<Visibility>().unwrap(),
            Visibility::Hidden
        );
        assert_eq!(
            world.entity(caption).get::<Text>().unwrap().0,
            "SCAVENGER - HOSTILE",
            "the faction line shows at lock time, gesture-independent"
        );
        assert_eq!(
            world.entity(caption).get::<TextColor>().unwrap().0,
            FACTION_HOSTILE_COLOR,
            "colored by relation"
        );

        // Hot: red frame, ticks on (Q5a).
        world.entity_mut(player).insert(WeaponsHot(true));
        world.run_system_once(drive_inset_frame_state).unwrap();
        assert_eq!(
            *world.entity(frame).get::<BorderColor>().unwrap(),
            BorderColor::all(INSET_BORDER_HOT_COLOR),
            "hot: the frame goes lock-red (Q5a color)"
        );
        assert_eq!(
            *world.entity(tick).get::<Visibility>().unwrap(),
            Visibility::Inherited,
            "hot: the armed ticks appear (Q5a shape)"
        );

        // A neutral lock reads NEUTRAL (delivery guard for the relation
        // branch), and no lock clears the line.
        let rock = world
            .spawn(GlobalTransform::from_translation(Vec3::new(
                0.0, 0.0, -50.0,
            )))
            .id();
        world.get_mut::<CombatLock>(player).unwrap().0 = Some(rock);
        world.run_system_once(drive_inset_frame_state).unwrap();
        assert_eq!(
            world.entity(caption).get::<Text>().unwrap().0,
            "CONTACT - NEUTRAL",
            "an unnamed neutral body still gets a line"
        );
        world.get_mut::<CombatLock>(player).unwrap().0 = None;
        world.run_system_once(drive_inset_frame_state).unwrap();
        assert_eq!(world.entity(caption).get::<Text>().unwrap().0, "");
    }

    #[test]
    fn framing_radius_is_finite_for_a_section_less_body() {
        // A zoomable body with no collider AABB (a test torpedo/asteroid stand-in)
        // still yields a finite framing radius via the fallback, so the pose is
        // never NaN/degenerate.
        let mut world = World::new();
        let target = world.spawn_empty().id();
        let radius = world
            .run_system_once(
                move |q_children: Query<&Children>,
                      q_aabb: Query<&ColliderAabb, Without<Sensor>>| {
                    zoomable_framing_radius(target, Vec3::ZERO, &q_children, &q_aabb)
                },
            )
            .unwrap();
        assert!(radius.is_finite() && radius > 0.0);
    }

    // -- highlight --

    /// Build a world for the highlight reconcile: highlight assets (default
    /// handles, no real assets needed) + a focused target with `n` sections.
    fn highlight_rig(n: usize) -> (World, Entity, Vec<Entity>) {
        let mut world = World::new();
        world.insert_resource(TargetInsetHighlightAssets {
            mesh: Handle::default(),
            material: Handle::default(),
        });
        let target = world.spawn(SpaceshipRootMarker).id();
        let sections: Vec<Entity> = (0..n)
            .map(|_| world.spawn((SectionMarker, ChildOf(target))).id())
            .collect();
        let player = world
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                ComponentLock::default(),
            ))
            .id();
        (world, player, sections)
    }

    fn highlight_targets(world: &mut World) -> Vec<Entity> {
        let mut v: Vec<Entity> = world
            .query_filtered::<&TargetInsetHighlightOf, With<TargetInsetHighlightMarker>>()
            .iter(world)
            .map(|of| **of)
            .collect();
        v.sort();
        v
    }

    #[test]
    fn highlight_follows_the_component_lock_and_reverts() {
        let (mut world, player, sections) = highlight_rig(3);
        let (a, b) = (sections[0], sections[1]);

        // Nothing selected: no overlay.
        world.run_system_once(sync_section_highlight).unwrap();
        assert!(highlight_targets(&mut world).is_empty());

        // Select a: exactly one overlay, on a.
        world.get_mut::<ComponentLock>(player).unwrap().section = Some(a);
        world.run_system_once(sync_section_highlight).unwrap();
        assert_eq!(highlight_targets(&mut world), vec![a]);

        // Move to b: the a overlay is dropped, one overlay on b.
        world.get_mut::<ComponentLock>(player).unwrap().section = Some(b);
        world.run_system_once(sync_section_highlight).unwrap();
        assert_eq!(highlight_targets(&mut world), vec![b]);
    }

    #[test]
    fn highlight_does_not_duplicate_across_frames() {
        let (mut world, player, sections) = highlight_rig(2);
        world.get_mut::<ComponentLock>(player).unwrap().section = Some(sections[0]);
        world.run_system_once(sync_section_highlight).unwrap();
        world.run_system_once(sync_section_highlight).unwrap();
        assert_eq!(highlight_targets(&mut world), vec![sections[0]]);
    }

    #[test]
    fn highlight_clears_when_its_section_dies() {
        let (mut world, player, sections) = highlight_rig(2);
        let a = sections[0];
        world.get_mut::<ComponentLock>(player).unwrap().section = Some(a);
        world.run_system_once(sync_section_highlight).unwrap();
        assert_eq!(highlight_targets(&mut world), vec![a]);

        // The section despawns (destroyed): its overlay child goes with it, and
        // the selection no longer resolves, so the reconcile settles to empty.
        world.despawn(a);
        world.run_system_once(sync_section_highlight).unwrap();
        assert!(highlight_targets(&mut world).is_empty());
    }
}
