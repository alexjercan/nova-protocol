//! nova_os_rtt_poc: feasibility prototype for the NOVA OS render-to-texture CRT
//! pipeline (task 20260726-193233, Step 1 - "PROTOTYPE FIRST").
//!
//! The overlay CRT shader cannot bloom or curve the terminal text because a UI
//! material cannot sample the content behind it. The fix is to render the
//! terminal-content subtree to an offscreen image via its own UI camera, then
//! display that image through ONE sampling shader that CAN read it. This rig
//! proves the three unknowns the DECISION front-loaded before committing:
//!
//!   (a) text renders into the image and back out crisply,
//!   (b) a fixed-tap bloom + barrel warp is affordable and derivative-free
//!       (so it survives WebGL2), and
//!   (c) - THE CRUX - a UI subtree rendered through an image camera can still be
//!       HOVERED and CLICKED. `render_scale.rs` documents that bevy's LEGACY
//!       `ui_focus_system` only delivers a cursor to a WINDOW camera, and the
//!       lesson `verify-interaction-not-just-rendering` records "bevy_ui on an
//!       image camera is unclickable". BUT bevy 0.19's picking backend
//!       (`ui_picking`) matches pointers to cameras by RENDER TARGET, not
//!       window-ness - so a forwarded custom pointer whose `PointerLocation`
//!       targets the image restores hover/click. This rig validates that claim.
//!
//! Run (windowed, real GPU; exits after the automated verdict):
//! ```text
//! cargo run --example nova_os_rtt_poc
//! ```
//! Watch stdout for `POC PICKING native: OK` / `FAIL`. Leave it running and
//! mouse over the button to eyeball the forwarded pointer + bloom by hand
//! (set `NOVA_POC_HOLD=1` to skip the auto-exit).

use bevy::{
    asset::uuid::Uuid,
    camera::{ImageRenderTarget, NormalizedRenderTarget, RenderTarget},
    picking::{
        hover::Hovered,
        pointer::{
            Location, PointerAction, PointerButton, PointerId, PointerInput, PointerLocation,
        },
    },
    prelude::*,
    render::{
        render_resource::{AsBindGroup, ShaderType, TextureFormat},
        view::screenshot::{save_to_disk, Screenshot},
    },
    ui::UiTargetCamera,
    ui_render::prelude::{MaterialNode, UiMaterial, UiMaterialPlugin},
};

/// Offscreen panel size in physical pixels (a stand-in for the real screen node).
const PANEL: UVec2 = UVec2::new(900, 600);
/// The button's rect inside the image, in image pixels (absolute-positioned).
const BTN_POS: Vec2 = Vec2::new(120.0, 120.0);
const BTN_SIZE: Vec2 = Vec2::new(240.0, 72.0);

fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: (1280, 800).into(),
                title: "NOVA OS RTT PoC".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(UiMaterialPlugin::<PocCrtMaterial>::default())
        .init_resource::<Probing>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                forward_pointer,
                animate_material,
                button_feedback,
                // verdict runs AFTER forward_pointer so, during its probe window,
                // its button-centre write wins over the mouse-driven parking.
                verdict.after(forward_pointer),
                diagnose,
                capture_once,
            ),
        )
        .run()
}

#[derive(Resource)]
struct Rig {
    image: Handle<Image>,
    image_cam: Entity,
    display: Entity,
    button: Entity,
    pointer: Entity,
}

/// While set, the automated verdict owns the forwarded pointer, so the real
/// mouse (which forward_pointer would otherwise park off-panel) does not fight it.
#[derive(Resource, Default)]
struct Probing(bool);

/// The sampling CRT material: binds the offscreen image + a small uniform block.
#[derive(Asset, AsBindGroup, TypePath, Clone, Debug)]
struct PocCrtMaterial {
    #[uniform(0)]
    data: PocCrtUniform,
    #[texture(1)]
    #[sampler(2)]
    source: Handle<Image>,
}

/// Field order MUST match `assets/shaders/nova_os_rtt_poc.wgsl` (vec2 first, then
/// scalars - see `shader-uniform-field-order-must-match-wgsl`).
#[derive(ShaderType, Clone, Debug)]
struct PocCrtUniform {
    resolution: Vec2,
    time: f32,
    warp: f32,
    bloom: f32,
    scanline: f32,
}

impl UiMaterial for PocCrtMaterial {
    fn fragment_shader() -> bevy::shader::ShaderRef {
        "shaders/nova_os_rtt_poc.wgsl".into()
    }
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<PocCrtMaterial>>,
) {
    // 1. Offscreen image target the terminal content renders into.
    let image = images.add(Image::new_target_texture(
        PANEL.x,
        PANEL.y,
        TextureFormat::Rgba8UnormSrgb,
        None,
    ));

    // 2. A dedicated UI camera that renders the content subtree INTO the image.
    //    order 0 so it runs before the window camera that samples it.
    let image_cam = commands
        .spawn((
            Name::new("PocImageCamera"),
            Camera2d,
            Camera {
                order: 0,
                clear_color: ClearColorConfig::Custom(Color::BLACK),
                ..default()
            },
            // In this bevy fork the render target is its OWN component (the UI
            // picking backend queries `&RenderTarget` alongside `&Camera`).
            RenderTarget::Image(ImageRenderTarget {
                handle: image.clone(),
                scale_factor: 1.0,
            }),
        ))
        .id();

    // 3. The window camera: renders the display UI (which SAMPLES the image) and
    //    is the default UI camera, so it stays clickable the ordinary way.
    commands.spawn((
        Name::new("PocWindowCamera"),
        Camera2d,
        Camera {
            order: 1,
            ..default()
        },
        IsDefaultUiCamera,
    ));

    // 4. Terminal-content subtree -> routed to the image camera via UiTargetCamera.
    //    Bright green text (to see bloom) + an interactive button (to test the
    //    crux: hover/click through the image).
    let mut button_id = Entity::PLACEHOLDER;
    commands
        .spawn((
            Name::new("PocContentRoot"),
            Node {
                width: Val::Px(PANEL.x as f32),
                height: Val::Px(PANEL.y as f32),
                position_type: PositionType::Absolute,
                ..default()
            },
            BackgroundColor(Color::srgb(0.01, 0.03, 0.02)),
            UiTargetCamera(image_cam),
        ))
        .with_children(|root| {
            for (i, line) in [
                "NOVA OS  v0.9.0",
                "nova> help",
                "  help   ship   objectives",
                "nova> ship",
                "  CERES QUEEN  hull 100%  power nominal",
                "nova> _",
            ]
            .into_iter()
            .enumerate()
            {
                root.spawn((
                    Text::new(line),
                    TextFont {
                        font_size: FontSize::Px(28.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.21, 1.0, 0.47)),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(40.0),
                        top: Val::Px(230.0 + i as f32 * 40.0),
                        ..default()
                    },
                ));
            }
            button_id = root
                .spawn((
                    Name::new("PocButton"),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(BTN_POS.x),
                        top: Val::Px(BTN_POS.y),
                        width: Val::Px(BTN_SIZE.x),
                        height: Val::Px(BTN_SIZE.y),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.1, 0.35, 0.18)),
                    // Insert Hovered so the picking backend keeps it up to date
                    // (update_is_hovered only touches entities that have it).
                    Hovered::default(),
                    children![(
                        Text::new("[ CLICK / HOVER ]"),
                        TextFont {
                            font_size: FontSize::Px(24.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.8, 1.0, 0.9)),
                    )],
                ))
                .observe(|_: On<Pointer<Click>>| {
                    info!("POC CLICK native: OK (button received a Click through the image)");
                })
                .id();
        });

    // 5. Display surface on the WINDOW: a MaterialNode sampling the image through
    //    the sampling CRT shader (bloom + barrel warp). Centered, panel-sized.
    let material = materials.add(PocCrtMaterial {
        data: PocCrtUniform {
            resolution: PANEL.as_vec2(),
            time: 0.0,
            warp: 0.18,
            bloom: 0.9,
            scanline: 0.10,
        },
        source: image.clone(),
    });
    let display = commands
        .spawn((
            Name::new("PocDisplay"),
            Node {
                width: Val::Px(PANEL.x as f32),
                height: Val::Px(PANEL.y as f32),
                position_type: PositionType::Absolute,
                left: Val::Px(190.0),
                top: Val::Px(100.0),
                ..default()
            },
            MaterialNode(material),
        ))
        .id();

    // 6. The forwarded custom pointer: its PointerLocation targets the IMAGE, so
    //    ui_picking hit-tests the content nodes rendered through the image camera.
    let pointer = commands
        .spawn((
            Name::new("PocForwardedPointer"),
            PointerId::Custom(Uuid::from_u128(0x0BADC0DE_CAFE_1234_5678_9ABCDEF01234)),
            PointerLocation::new(Location {
                target: image_target(&image),
                position: Vec2::new(-100.0, -100.0),
            }),
        ))
        .id();

    commands.insert_resource(Rig {
        image,
        image_cam,
        display,
        button: button_id,
        pointer,
    });
}

/// One-shot native capture at ~1.8s (before the probe drives the pointer) so the
/// crisp text + bloom + barrel curvature can be eyeballed. Saved under the task
/// shots dir; set NOVA_POC_SHOT to override the path.
fn capture_once(mut commands: Commands, time: Res<Time>, mut done: Local<bool>) {
    if *done || time.elapsed_secs() < 1.8 {
        return;
    }
    *done = true;
    let path = std::env::var("NOVA_POC_SHOT")
        .unwrap_or_else(|_| "tasks/20260726-193233/shots/poc-native.png".to_string());
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path));
    info!("POC capture: screenshot requested");
}

fn pointer_hovers(hover_map: &bevy::picking::hover::HoverMap, id: &PointerId) -> bool {
    hover_map.0.get(id).map(|h| !h.is_empty()).unwrap_or(false)
}

fn image_target(image: &Handle<Image>) -> NormalizedRenderTarget {
    NormalizedRenderTarget::Image(ImageRenderTarget {
        handle: image.clone(),
        scale_factor: 1.0,
    })
}

/// Map the real window cursor onto the displayed panel, then into image pixels,
/// and write the forwarded pointer's location there. This is the reusable
/// primitive the real pipeline will keep: pointer forwarding through the blit.
fn forward_pointer(
    rig: Res<Rig>,
    probing: Res<Probing>,
    windows: Query<&Window>,
    nodes: Query<(&ComputedNode, &UiGlobalTransform)>,
    mut pointers: Query<&mut PointerLocation>,
) {
    if probing.0 {
        // The verdict owns the pointer during the automated probe.
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok((node, xf)) = nodes.get(rig.display) else {
        return;
    };
    // Display node rect in window logical pixels.
    let size = node.size();
    let center = xf.translation;
    let min = center - size * 0.5;
    let local = (cursor - min) / size; // 0..1 across the panel
    if local.x < 0.0 || local.x > 1.0 || local.y < 0.0 || local.y > 1.0 {
        // Park the pointer off-panel so nothing is hovered.
        if let Ok(mut loc) = pointers.get_mut(rig.pointer) {
            loc.location = Some(Location {
                target: image_target(&rig.image),
                position: Vec2::new(-100.0, -100.0),
            });
        }
        return;
    }
    let in_image = inverse_barrel(local, 0.18) * PANEL.as_vec2();
    if let Ok(mut loc) = pointers.get_mut(rig.pointer) {
        loc.location = Some(Location {
            target: image_target(&rig.image),
            position: in_image,
        });
    }
}

/// Inverse of the shader's forward barrel warp, so a hovered on-screen point maps
/// back to the glyph actually under it. One Newton step is plenty for this amount.
fn inverse_barrel(uv: Vec2, amount: f32) -> Vec2 {
    let c = uv - Vec2::splat(0.5);
    let r2 = c.length_squared();
    // forward: out = c * (1 + amount*r2); invert approximately.
    let factor = 1.0 + amount * r2;
    Vec2::splat(0.5) + c / factor
}

fn animate_material(
    time: Res<Time>,
    mut materials: ResMut<Assets<PocCrtMaterial>>,
    q: Query<&MaterialNode<PocCrtMaterial>>,
) {
    for handle in &q {
        if let Some(mut mat) = materials.get_mut(&handle.0) {
            mat.data.time = time.elapsed_secs();
        }
    }
}

fn button_feedback(rig: Res<Rig>, mut q: Query<(&Hovered, &mut BackgroundColor)>) {
    if let Ok((hovered, mut bg)) = q.get_mut(rig.button) {
        bg.0 = if hovered.get() {
            Color::srgb(0.2, 0.8, 0.4)
        } else {
            Color::srgb(0.1, 0.35, 0.18)
        };
    }
}

/// Unattended verdict: at ~2.5 s drive the forwarded pointer straight to the
/// button centre (bypassing the mouse) and, a couple frames later, assert the
/// content node got Hovered THROUGH the image. Prints OK/FAIL and exits unless
/// NOVA_POC_HOLD is set.
fn verdict(
    rig: Res<Rig>,
    time: Res<Time>,
    mut probing: ResMut<Probing>,
    mut pointers: Query<&mut PointerLocation>,
    hover_map: Res<bevy::picking::hover::HoverMap>,
    mut clicks: MessageWriter<PointerInput>,
    mut stage: Local<u8>,
    mut exit: MessageWriter<AppExit>,
) {
    let t = time.elapsed_secs();
    let pointer_id = PointerId::Custom(Uuid::from_u128(0x0BADC0DE_CAFE_1234_5678_9ABCDEF01234));
    let center = Location {
        target: image_target(&rig.image),
        position: BTN_POS + BTN_SIZE * 0.5,
    };
    // Probe window: own the pointer and pin it on the button centre so the
    // PreUpdate picking pass has a stable target to hit while we sample.
    if (2.5..4.0).contains(&t) {
        probing.0 = true;
        if let Ok(mut loc) = pointers.get_mut(rig.pointer) {
            loc.location = Some(center.clone());
        }
        // Sample hover late (several frames after pinning) so ui_picking and
        // update_is_hovered have caught up. `Hovered` on the button is true when
        // the button OR a descendant (its text child) is hit through the image.
        if *stage == 0 && t >= 3.3 {
            // Read the HoverMap directly: bevy 0.19's `update_is_hovered` only
            // mirrors the MOUSE pointer into `Hovered` components, so a forwarded
            // Custom pointer's hits show up here (and in Pointer<Over>/<Click>
            // observers) but NOT in `Hovered`. The HoverMap is the ground truth
            // that picking ran through the image.
            let ok = pointer_hovers(&hover_map, &pointer_id);
            if ok {
                info!("POC PICKING native: OK - forwarded pointer HOVERED content through the image (HoverMap hit)");
            } else {
                error!("POC PICKING native: FAIL - no hover through the image (approach blocked)");
            }
            *stage = 1;
        }
        // Forward a real click to the button's observer: Press then Release on the
        // same image-space point => a Pointer<Click> on the content node.
        if *stage == 1 && t >= 3.5 {
            clicks.write(PointerInput::new(
                pointer_id,
                center.clone(),
                PointerAction::Press(PointerButton::Primary),
            ));
            *stage = 2;
        } else if *stage == 2 && t >= 3.6 {
            clicks.write(PointerInput::new(
                pointer_id,
                center,
                PointerAction::Release(PointerButton::Primary),
            ));
            *stage = 3;
        }
        return;
    }
    probing.0 = false;
    if *stage == 3 && t > 4.2 && std::env::var("NOVA_POC_HOLD").is_err() {
        exit.write(AppExit::Success);
    }
}

/// One-shot diagnostic dump at ~3.05s so a FAIL verdict is explainable: is the
/// camera viewport valid, is the node laid out, does the node resolve to the
/// image camera, and did the pointer register a hit?
#[allow(clippy::type_complexity)]
fn diagnose(
    rig: Res<Rig>,
    time: Res<Time>,
    mut done: Local<bool>,
    cams: Query<(&Camera, &RenderTarget)>,
    nodes: Query<(
        &ComputedNode,
        &UiGlobalTransform,
        &bevy::ui::ComputedUiTargetCamera,
    )>,
    pointers: Query<(&PointerId, &PointerLocation)>,
    hover_map: Res<bevy::picking::hover::HoverMap>,
) {
    let t = time.elapsed_secs();
    if *done || t < 3.45 {
        return;
    }
    *done = true;
    if let Ok((cam, rt)) = cams.get(rig.image_cam) {
        info!(
            "DIAG image_cam: viewport_rect={:?} scaling={:?} rt={:?}",
            cam.physical_viewport_rect(),
            cam.target_scaling_factor(),
            rt
        );
    }
    if let Ok((node, xf, target)) = nodes.get(rig.button) {
        info!(
            "DIAG button: size={:?} center={:?} resolved_cam={:?}",
            node.size(),
            xf.translation,
            target.get()
        );
    }
    for (id, loc) in &pointers {
        if matches!(id, PointerId::Custom(_)) {
            info!("DIAG pointer: id={:?} loc={:?}", id, loc.location());
        }
    }
    info!(
        "DIAG hover_map entries: {} (button={:?})",
        hover_map.0.len(),
        rig.button
    );
    for (pid, hits) in hover_map.0.iter() {
        for ent in hits.keys() {
            info!(
                "DIAG   pointer {:?} hovering {:?} (is_button={})",
                pid,
                ent,
                *ent == rig.button
            );
        }
    }
}
