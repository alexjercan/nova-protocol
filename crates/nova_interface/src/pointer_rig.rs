//! Test-only support: the live-tree pointer rig for the TAB interface panes,
//! the UI cue and node churn recorders the pane tests read, and an independent
//! transcription of the command CRT shader's sample-UV chain for the CRT
//! mapping tests.
//!
//! [`pane_pointer_rig`] stands up the real UI, picking and widget stack over a
//! window camera and drives bevy's own mouse pointer from window pixels, so
//! layout, the UI stack, hit testing and `Activate` are all the shipped ones.
//! A helper-level assert would pass against a hit target that agreed with
//! itself; only the whole chain says what a click at a known place selects.
//!
//! The shader reference ([`shader_sample_uv_reference`] / [`shader_draws_at`])
//! is written from the WGSL, never from the production mapping helper it
//! checks.

use bevy::{
    asset::AssetPlugin,
    camera::{Camera, ComputedCameraValues, RenderTargetInfo},
    image::{Image, TextureAtlasLayout},
    input::{
        mouse::{MouseButton, MouseButtonInput},
        ButtonState,
    },
    math::{UVec2, Vec2},
    prelude::*,
    text::TextPlugin,
    ui::UiPlugin,
    ui_widgets::ButtonPlugin,
    window::{CursorMoved, PrimaryWindow, Window, WindowEvent, WindowResolution},
};
use nova_gameplay::{
    audio::UI_SFX_FILES,
    prelude::{PlaySfx, SoundBank, UiSfx},
};

/// Agreement budget between the pointer's mapping and the shader's, in image
/// pixels, over the whole screen. Half a pixel: below the level anything can be
/// clicked at, and far below the 12 px blips this bug lost.
pub(super) const CRT_MAP_BUDGET_PX: f32 = 0.5;

/// Independent transcription of `assets/shaders/nova_os_crt.wgsl`'s sample-UV
/// chain: the power-collapse remap, then `barrel()`, then the overscan pull. It
/// answers "which image texel does the CRT DISPLAY under this point on the
/// glass?", which is what the forwarded pointer has to agree with.
///
/// Written from the shader source, NOT from `nova_os_crt_screen_to_image_uv`, so
/// a production helper that drifts from the picture cannot satisfy it
/// (`test-must-not-reuse-the-formula-under-test`). The degauss shear is
/// deliberately absent - see that helper's docs.
pub(super) fn shader_sample_uv_reference(uv: Vec2, warp: f32, overscan: f32, power: f32) -> Vec2 {
    let sample_uv = shader_collapse_remap(uv, power);
    // fn barrel(uv, amount) { centered * (1.0 + amount * dot(centered, centered)) }
    let centered = sample_uv - Vec2::splat(0.5);
    let warped_raw = Vec2::splat(0.5) + centered * (1.0 + warp * centered.length_squared());
    // let warped = (warped_raw - 0.5) * material.overscan + 0.5;
    (warped_raw - Vec2::splat(0.5)) * overscan + Vec2::splat(0.5)
}

/// The shader's answer to "is anything DRAWN at this point on the glass, and if
/// so from which image texel?" - i.e. [`shader_sample_uv_reference`] plus the two
/// gates the fragment multiplies its output by:
///
/// ```wgsl
/// if cy < 0.0 || cy > 1.0 || cx < 0.0 || cx > 1.0 { collapsed = 1.0; }
/// let in_bounds = f32(warped.x >= 0.0 && ... && warped.y <= 1.0);
/// rgb = (rgb + analog_add + rim_add) * in_bounds * (1.0 - collapsed);
/// ```
///
/// These are two SEPARATE tests and neither implies the other:
/// barrel-then-overscan is a net contraction here (0.93 against a barrel factor
/// under 1.06), so a `cx` just past 1 still lands inside `[0,1]` after warping.
/// A reference that checked only `in_bounds` disagreed with the (correct)
/// production helper at 186 grid points at power 0.15 and 814 at 0.35 on a
/// 201x201 grid - invisible at 17x17 purely by luck of the sampling.
pub(super) fn shader_draws_at(uv: Vec2, warp: f32, overscan: f32, power: f32) -> Option<Vec2> {
    let sample_uv = shader_collapse_remap(uv, power);
    let collapsed = !(0.0..=1.0).contains(&sample_uv.x) || !(0.0..=1.0).contains(&sample_uv.y);
    let warped = shader_sample_uv_reference(uv, warp, overscan, power);
    let in_bounds = (0.0..=1.0).contains(&warped.x) && (0.0..=1.0).contains(&warped.y);
    (!collapsed && in_bounds).then_some(warped)
}

/// The shader's power-collapse remap, transcribed ONCE - both the sample-UV
/// reference and the draws-here reference read it, so a future shader change to
/// an edge or the epsilon cannot update one copy and leave the other stale
/// while the pair still looks self-consistent.
///
/// ```wgsl
/// let open_h = smoothstep(0.0, 0.65, material.power);
/// let open_w = smoothstep(0.0, 0.28, material.power);
/// let cy = (in.uv.y - 0.5) / max(open_h, 0.0008) + 0.5;
/// let cx = (in.uv.x - 0.5) / max(open_w, 0.0008) + 0.5;
/// ```
fn shader_collapse_remap(uv: Vec2, power: f32) -> Vec2 {
    let smoothstep = |edge1: f32, x: f32| {
        let t = (x / edge1).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    };
    let open_h = smoothstep(0.65, power);
    let open_w = smoothstep(0.28, power);
    Vec2::new(
        (uv.x - 0.5) / open_w.max(0.0008) + 0.5,
        (uv.y - 0.5) / open_h.max(0.0008) + 0.5,
    )
}

/// A grid of screen-local UVs covering centre, edges and corners.
///
/// Fine enough to land inside the narrow band where the shader's `collapsed` and
/// `in_bounds` gates disagree: a 17x17 grid missed it entirely at every swept
/// power, which is exactly how a reference that modelled only one of the two
/// gates passed.
pub(super) fn crt_uv_grid() -> Vec<Vec2> {
    let mut grid = Vec::new();
    const STEPS: usize = 200;
    for iy in 0..=STEPS {
        for ix in 0..=STEPS {
            grid.push(Vec2::new(
                ix as f32 / STEPS as f32,
                iy as f32 / STEPS as f32,
            ));
        }
    }
    grid
}

/// Size of the content root the pane content under test goes in.
pub(super) const RIG_CONTENT: UVec2 = UVec2::new(1280, 720);

/// The window the content root is inset in. Deliberately larger than the
/// content and off-centre, so a hit test that quietly assumes the content is
/// the whole window, or is centred in it, fails here.
pub(super) const RIG_WINDOW: UVec2 = UVec2::new(1600, 900);

/// Top-left of the content root in window pixels. Asymmetric on purpose (see
/// [`RIG_WINDOW`]).
pub(super) const RIG_PANEL_MIN: Vec2 = Vec2::new(210.0, 120.0);

/// The live UI tree plus the handles a test needs to place content and click.
pub(super) struct PanePointerRig {
    pub app: App,
    /// The window-space subtree the content under test goes in, sized to
    /// [`RIG_CONTENT`] at [`RIG_PANEL_MIN`], so a child at `Val::Px(p)` sits at
    /// window pixel `RIG_PANEL_MIN + p`.
    pub content_root: Entity,
}

/// Build the TAB interface's picking path: the real UI, text, picking, widget
/// and mouse-input stack over one window camera, with no CRT and no forwarded
/// pointer. The panes draw in window space, so bevy's own mouse pointer is the
/// one that selects.
pub(super) fn pane_pointer_rig() -> PanePointerRig {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        TransformPlugin,
        bevy::a11y::AccessibilityPlugin,
        // `ui_picking` refuses any node whose `InheritedVisibility` is not
        // computed-true, and nothing else in a MinimalPlugins app propagates it.
        bevy::camera::visibility::VisibilityPlugin,
        bevy::input::InputPlugin,
        // The mouse pointer and its `WindowEvent` reader.
        bevy::picking::input::PointerInputPlugin,
        bevy::picking::PickingPlugin,
        bevy::picking::InteractionPlugin,
        TextPlugin,
        UiPlugin,
        // `Button` -> `Activate` lives here; in the game it arrives with
        // DefaultPlugins' `UiWidgetsPlugins`. Only the button half is added:
        // the group's text-input plugin wants IME messages and an `InputFocus`
        // resource this rig has no window backend to supply.
        ButtonPlugin,
    ));
    // `WindowPlugin` registers this in a real app; `MinimalPlugins` has no
    // window backend, and without it no click here would go anywhere.
    app.add_message::<WindowEvent>();
    app.init_asset::<Image>().init_asset::<TextureAtlasLayout>();
    // `VisibilityPlugin`'s bounds pass reads the mesh collections, which in the
    // game arrive with the render plugins this rig has no use for.
    app.init_asset::<Mesh>()
        .init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
    // The primary window: `ui_picking` normalizes a camera's window target
    // against it.
    app.world_mut().spawn((
        Window {
            resolution: WindowResolution::new(RIG_WINDOW.x, RIG_WINDOW.y),
            ..default()
        },
        PrimaryWindow,
    ));
    app.world_mut().spawn((
        Camera2d,
        Camera {
            order: 0,
            computed: ComputedCameraValues {
                target_info: Some(RenderTargetInfo {
                    physical_size: RIG_WINDOW,
                    scale_factor: 1.0,
                }),
                ..default()
            },
            ..default()
        },
        IsDefaultUiCamera,
    ));
    let content_root = app
        .world_mut()
        .spawn(Node {
            position_type: PositionType::Absolute,
            left: Val::Px(RIG_PANEL_MIN.x),
            top: Val::Px(RIG_PANEL_MIN.y),
            width: Val::Px(RIG_CONTENT.x as f32),
            height: Val::Px(RIG_CONTENT.y as f32),
            ..default()
        })
        .id();
    settle(&mut app);
    PanePointerRig { app, content_root }
}

/// Run enough frames for layout, the UI stack and picking to settle.
pub(super) fn settle(app: &mut App) {
    for _ in 0..4 {
        app.update();
    }
}

/// Move the real mouse to a window pixel. The mouse pointer reads the
/// `CursorMoved` event, which `bevy_winit` writes for a real move beside the
/// window's cursor position.
pub(super) fn move_cursor_to(rig: &mut PanePointerRig, window_px: Vec2) {
    let (entity, mut window) = rig
        .app
        .world_mut()
        .query_filtered::<(Entity, &mut Window), With<PrimaryWindow>>()
        .single_mut(rig.app.world_mut())
        .expect("the rig has a primary window");
    window.set_physical_cursor_position(Some(window_px.as_dvec2()));
    rig.app
        .world_mut()
        .write_message(WindowEvent::CursorMoved(CursorMoved {
            window: entity,
            position: window_px,
            delta: None,
        }));
    settle(&mut rig.app);
}

/// Press and release the left mouse button at `window_px`, exactly as the player
/// does: the cursor moves, then button events arrive for the mouse pointer.
///
/// The `WindowEvent` wrapper ONLY - the half `bevy_picking` itself reads to
/// press the mouse pointer. `bevy_winit` writes a concrete `MouseButtonInput`
/// twin beside it for a real click, but a SYNTHESIZED one does not, so a rig
/// writing both would pass a reader of the twin that goes dead under every
/// driven run.
pub(super) fn click_at(rig: &mut PanePointerRig, window_px: Vec2) {
    move_cursor_to(rig, window_px);
    for state in [ButtonState::Pressed, ButtonState::Released] {
        let window = rig
            .app
            .world_mut()
            .query_filtered::<Entity, With<PrimaryWindow>>()
            .single(rig.app.world())
            .expect("the rig has a primary window");
        rig.app
            .world_mut()
            .write_message(WindowEvent::MouseButtonInput(MouseButtonInput {
                button: MouseButton::Left,
                state,
                window,
            }));
        settle(&mut rig.app);
    }
}

/// Every UI cue played since the last clear, in order.
#[derive(Resource, Default)]
pub(super) struct HeardCues(pub Vec<UiSfx>);

/// Load the UI sound bank and record each cue it plays into [`HeardCues`], by
/// handle identity, with no audio device.
pub(super) fn hear_ui_cues(app: &mut App) {
    app.init_asset::<AudioSource>();
    let bank = SoundBank::load(app.world().resource::<AssetServer>(), UI_SFX_FILES);
    app.insert_resource(bank);
    app.init_resource::<HeardCues>();
    app.add_observer(
        |sfx: On<PlaySfx>, bank: Res<SoundBank<UiSfx>>, mut heard: ResMut<HeardCues>| {
            let cue = UI_SFX_FILES
                .iter()
                .map(|(cue, _)| *cue)
                .find(|cue| bank.get(*cue) == sfx.handle)
                .expect("every played cue comes from the UI bank");
            heard.0.push(cue);
        },
    );
}

/// Take the cues heard since the last take.
pub(super) fn take_cues(app: &mut App) -> Vec<UiSfx> {
    std::mem::take(&mut app.world_mut().resource_mut::<HeardCues>().0)
}

/// UI nodes spawned, despawned and rewritten since the last take.
#[derive(Resource, Default, Clone, Copy, PartialEq, Eq, Debug)]
pub(super) struct NodeChurn {
    /// Nodes added.
    pub spawned: usize,
    /// Nodes removed.
    pub despawned: usize,
    /// Nodes whose `Node`, `Text` or `Visibility` was written, spawns included.
    pub rewritten: usize,
}

/// Count node churn into [`NodeChurn`] every frame, in `Last`, so it sees
/// every write the frame made.
pub(super) fn track_node_churn(app: &mut App) {
    app.init_resource::<NodeChurn>();
    app.add_systems(Last, count_node_churn);
}

#[expect(
    clippy::type_complexity,
    reason = "one detector over the three written components"
)]
fn count_node_churn(
    q_added: Query<(), Added<Node>>,
    mut removed: RemovedComponents<Node>,
    q_written: Query<
        (),
        (
            With<Node>,
            Or<(Changed<Node>, Changed<Text>, Changed<Visibility>)>,
        ),
    >,
    mut churn: ResMut<NodeChurn>,
) {
    churn.spawned += q_added.iter().count();
    churn.despawned += removed.read().count();
    churn.rewritten += q_written.iter().count();
}

/// Take the node churn counted since the last take.
pub(super) fn take_churn(app: &mut App) -> NodeChurn {
    std::mem::take(&mut *app.world_mut().resource_mut::<NodeChurn>())
}
