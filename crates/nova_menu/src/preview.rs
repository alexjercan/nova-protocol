//! The menu's 3D previews: the chart round a seed's start on the New Game
//! world step, the chart a save records on the Load screen, and the hull of
//! the selected start role on the role step.
//!
//! A preview is an image node carrying what it shows ([`MenuChartPreview`]
//! or [`MenuHullPreview`]). [`build_menu_previews`] gives each a scene of
//! its own on its own render layer, drawn by the shared builders in
//! `nova_hud::preview`, and a camera rendering into the node's image; the
//! image follows the node's size. A chart preview orbits under the pointer
//! the way the Map does, right-drag turning and the wheel zooming, and keeps
//! that turn and zoom when its view is redrawn; a hull preview turns on its
//! turntable. A chart preview labels each mark that has a code and the own
//! ship with a [`MenuChartLabel`] under the node, which
//! [`project_menu_chart_labels`] places over its mark. The scene goes with
//! the node: despawning the node drops its [`MenuPreviewScene`], whose hook
//! despawns the scene.
//!
//! A headless host with no render assets builds no scene and draws nothing;
//! the nodes still stand, so a test reads what a preview would show.

use bevy::{
    app::Propagate,
    camera::{visibility::RenderLayers, ImageRenderTarget, RenderTarget},
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    input::mouse::{MouseMotion, MouseWheel},
    picking::hover::Hovered,
    prelude::*,
};
use nova_events::prelude::SectorCoord;
use nova_gameplay::prelude::{new_render_target_image, resize_render_target};
use nova_hud::prelude::*;
use nova_interface::prelude::{zoom_radius, OrbitGesture};
use nova_ship::prelude::{insert_preview_section, DesignSection, PreviewRole};
use nova_ui::{
    theme::{ActiveUiTheme, UiColor},
    widget::{ThemedFill, ThemedText, UiText},
};

/// The render layer of the first preview's scene; preview slot `n` draws on
/// `MENU_PREVIEW_LAYER + n`, so no camera sees another preview's scene.
/// Above the interface's layers (20 to 22), the menu backdrop's (23) and
/// the station cards' (26 up): the menu and the Station pane never share a
/// screen, but a layer is cheap and a collision is silent.
const MENU_PREVIEW_LAYER: usize = 30;

/// The first preview's camera order; slot `n` renders at
/// `MENU_PREVIEW_CAMERA_ORDER - n`, before every interface camera.
const MENU_PREVIEW_CAMERA_ORDER: isize = -50;

/// The room a chart preview opens with round its grid: tighter than the
/// Map's chart, whose side panel takes part of the view.
const MENU_PREVIEW_MARGIN: f32 = 1.7;

/// The size a preview's image opens at; it follows the node once laid out.
const MENU_PREVIEW_PX: UVec2 = UVec2::new(320, 200);

/// A chart preview: the node draws `view`. Replace it to redraw.
#[derive(Component, Clone, Debug, PartialEq)]
pub(crate) struct MenuChartPreview(pub(crate) ChartSceneView);

/// A hull preview: the node draws a hull of these sections, turning.
#[derive(Component, Clone, Debug)]
pub(crate) struct MenuHullPreview(pub(crate) Vec<DesignSection>);

/// One label of a chart preview under its node: a mark's code or the own
/// ship's tag, hidden until projected.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub(crate) struct MenuChartLabel {
    /// Where the label points, in chart space.
    pub(crate) anchor: Vec3,
}

/// A chart preview's orbit, driven by the pointer over the node.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub(crate) struct MenuPreviewOrbit {
    pub(crate) radius: f32,
    pub(crate) theta: f32,
    pub(crate) phi: f32,
}

impl MenuPreviewOrbit {
    /// The opening orbit of a chart reaching `reach` cells out.
    fn chart(reach: i32) -> Self {
        Self {
            radius: chart_radius(reach, MENU_PREVIEW_MARGIN),
            theta: CHART_THETA,
            phi: CHART_PHI,
        }
    }
}

/// What kind of scene a preview node holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MenuPreviewType {
    Chart,
    Hull,
}

/// The scene built for a preview node: its root, its camera, the image the
/// camera renders into and the slot whose layer it draws on. Removing it
/// despawns the scene.
#[derive(Component, Clone, Debug)]
#[component(on_remove = drop_menu_preview_scene)]
pub(crate) struct MenuPreviewScene {
    root: Entity,
    camera: Entity,
    image: Handle<Image>,
    slot: usize,
    kind: MenuPreviewType,
}

fn drop_menu_preview_scene(mut world: DeferredWorld, context: HookContext) {
    let root = world
        .get::<MenuPreviewScene>(context.entity)
        .map(|scene| scene.root);
    if let Some(root) = root {
        world.commands().entity(root).try_despawn();
    }
}

/// The render layer preview slot `slot` draws on.
fn preview_layer(slot: usize) -> RenderLayers {
    RenderLayers::layer(MENU_PREVIEW_LAYER + slot)
}

/// The image node a preview draws into, sized by `node`.
pub(crate) fn preview_node(name: &'static str, node: Node) -> impl Bundle {
    (
        Name::new(name),
        Node {
            overflow: Overflow::clip(),
            ..node
        },
        ImageNode::default(),
        Hovered::default(),
    )
}

/// Build the scene of every preview node that has none yet or whose view
/// changed: despawn the old scene and labels, take the lowest free slot,
/// spawn the scene on its layer with a camera rendering into a new image,
/// label a chart's marks under the node, and hand the node the image. A
/// redrawn chart keeps the orbit the player turned it to, so a new seed
/// changes the marks and not the view; the camera still looks at the chart's
/// centre, the new start. A new chart opens on the chart's framing. A host
/// with no render assets builds nothing.
#[expect(
    clippy::too_many_arguments,
    reason = "one system reading the preview nodes, the theme and every render asset store"
)]
pub(crate) fn build_menu_previews(
    mut commands: Commands,
    theme: Res<ActiveUiTheme>,
    tiles: Option<Res<ChartTileTextures>>,
    images: Option<ResMut<Assets<Image>>>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    q_scenes: Query<&MenuPreviewScene>,
    q_labels: Query<(Entity, &ChildOf), With<MenuChartLabel>>,
    mut q_nodes: Query<
        (
            Entity,
            Option<&MenuChartPreview>,
            Option<&MenuHullPreview>,
            Option<&MenuPreviewScene>,
            Option<&MenuPreviewOrbit>,
            &mut ImageNode,
        ),
        Or<(Changed<MenuChartPreview>, Changed<MenuHullPreview>)>,
    >,
) {
    let (Some(mut images), Some(mut meshes), Some(mut materials)) = (images, meshes, materials)
    else {
        return;
    };
    let mut taken: Vec<usize> = q_scenes.iter().map(|scene| scene.slot).collect();
    for (node, chart, hull, old, turned, mut image_node) in &mut q_nodes {
        if let Some(old) = old {
            commands.entity(old.root).try_despawn();
            taken.retain(|slot| *slot != old.slot);
        }
        for (label, parent) in &q_labels {
            if parent.parent() == node {
                commands.entity(label).try_despawn();
            }
        }
        let slot = (0..)
            .find(|slot| !taken.contains(slot))
            .expect("an unbounded range holds a free slot");
        let layer = preview_layer(slot);
        let order = MENU_PREVIEW_CAMERA_ORDER - slot as isize;
        let scene = match (chart, hull) {
            (Some(chart), _) => {
                let Some(tiles) = tiles.as_deref() else {
                    continue;
                };
                let orbit = turned
                    .copied()
                    .unwrap_or_else(|| MenuPreviewOrbit::chart(chart.0.reach));
                let image = images.add(new_render_target_image(MENU_PREVIEW_PX));
                let root = commands
                    .spawn((
                        Name::new("Menu Chart Preview Scene"),
                        Transform::default(),
                        Visibility::Visible,
                    ))
                    .id();
                let camera = commands
                    .spawn((
                        Camera3d::default(),
                        Camera {
                            order,
                            clear_color: ClearColorConfig::Custom(theme.color(UiColor::Surface)),
                            ..default()
                        },
                        RenderTarget::Image(ImageRenderTarget {
                            handle: image.clone(),
                            scale_factor: 1.0,
                        }),
                        Transform::from_translation(orbit_eye(
                            orbit.radius,
                            orbit.theta,
                            orbit.phi,
                        ))
                        .looking_at(Vec3::ZERO, Vec3::Y),
                        layer.clone(),
                        ChildOf(root),
                    ))
                    .id();
                let placements = spawn_chart_scene(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    tiles,
                    &theme,
                    &chart.0,
                    layer,
                    root,
                );
                // Labels of marks that share a cell stack upward, as the
                // Map's do.
                let marks = chart.0.marks.iter().zip(&placements).map(|(mark, placed)| {
                    (
                        placed.at + Vec3::Y * (placed.slot as f32 * CHART_LABEL_STACK),
                        mark.code.as_str(),
                        mark.kind.color(),
                        12.0,
                    )
                });
                // A save puts the own ship at its cell's middle, where the
                // cell's marks stand, so its tag takes the slot over the
                // cell's labels instead of covering them.
                let you = chart.0.you.map(|you| {
                    let centre = chart.0.centre;
                    let cell = SectorCoord::new(
                        centre.x + (you.x / CHART_CELL).round() as i32,
                        centre.y + (you.y / CHART_CELL).round() as i32,
                        centre.z + (you.z / CHART_CELL).round() as i32,
                    );
                    let labelled = chart
                        .0
                        .marks
                        .iter()
                        .filter(|mark| mark.at == cell && !mark.code.is_empty())
                        .count();
                    let lift = if labelled == 0 {
                        CHART_MARK_LIFT * 0.5
                    } else {
                        CHART_MARK_LIFT + labelled as f32 * CHART_LABEL_STACK
                    };
                    (you + Vec3::Y * lift, "YOU", CHART_YOU_COLOR, 11.0)
                });
                for (anchor, text, color, size) in marks.chain(you) {
                    if text.is_empty() {
                        continue;
                    }
                    commands.spawn((
                        Name::new(format!("Menu Chart Label {text}")),
                        MenuChartLabel { anchor },
                        Node {
                            position_type: PositionType::Absolute,
                            padding: UiRect::axes(px(4), px(1)),
                            ..default()
                        },
                        Visibility::Hidden,
                        BackgroundColor(Color::NONE),
                        ThemedFill::alpha(UiColor::Void, 0.8),
                        Pickable::IGNORE,
                        ChildOf(node),
                        children![(
                            UiText,
                            Text::new(text),
                            TextFont {
                                font_size: FontSize::Px(size),
                                ..default()
                            },
                            TextColor(Color::NONE),
                            ThemedText::new(color),
                            Pickable::IGNORE,
                        )],
                    ));
                }
                commands.entity(node).insert(orbit);
                MenuPreviewScene {
                    root,
                    camera,
                    image,
                    slot,
                    kind: MenuPreviewType::Chart,
                }
            }
            (None, Some(hull)) => {
                let scene = spawn_hull_preview_scene(
                    &mut commands,
                    &mut images,
                    &theme,
                    &hull.0,
                    layer.clone(),
                    order,
                    true,
                );
                // The sections' meshes arrive as children over the next
                // frames; the propagated layer reaches them when they do.
                commands.entity(scene.model).insert(Propagate(layer));
                for section in &hull.0 {
                    let mut entity = commands.spawn((
                        Name::new(format!("Menu Hull Preview {}", section.id)),
                        Transform::from_translation(section.position)
                            .with_rotation(section.rotation),
                        Visibility::Inherited,
                        ChildOf(scene.model),
                    ));
                    insert_preview_section(&mut entity, &section.config, PreviewRole::Display);
                }
                MenuPreviewScene {
                    root: scene.root,
                    camera: scene.camera,
                    image: scene.image,
                    slot,
                    kind: MenuPreviewType::Hull,
                }
            }
            (None, None) => continue,
        };
        taken.push(slot);
        image_node.image = scene.image.clone();
        commands.entity(node).insert(scene);
    }
}

/// Keep the menu's key and rim lights on the union of its hull previews'
/// layers; none alive, none lit.
pub(crate) fn sync_menu_preview_lights(
    mut commands: Commands,
    q_scenes: Query<&MenuPreviewScene>,
    mut q_lights: Query<(Entity, &HullPreviewLight, &mut RenderLayers)>,
) {
    let layers = q_scenes
        .iter()
        .filter(|scene| scene.kind == MenuPreviewType::Hull)
        .fold(RenderLayers::none(), |layers, scene| {
            layers.union(&preview_layer(scene.slot))
        });
    sync_hull_preview_lights(
        &mut commands,
        HullPreviewOwnerType::Menu,
        &layers,
        &mut q_lights,
    );
}

/// Turn and zoom the chart preview under the pointer: a right-button drag
/// orbits, the wheel zooms, as the Map's viewer answers.
pub(crate) fn drive_menu_preview_orbits(
    time: Res<Time<Real>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut motion: MessageReader<MouseMotion>,
    mut wheel: MessageReader<MouseWheel>,
    mut q_orbits: Query<(&Hovered, &mut MenuPreviewOrbit)>,
) {
    // Drained every frame, so a frame with nothing under the pointer does
    // not hand its travel to the next.
    let motion: Vec2 = motion.read().map(|m| m.delta).sum();
    let wheel: f32 = wheel.read().map(|w| w.y).sum();
    let gesture = OrbitGesture {
        drag: mouse.pressed(MouseButton::Right).then_some(motion),
        ..default()
    };
    for (hovered, mut orbit) in &mut q_orbits {
        if !hovered.get() {
            continue;
        }
        if !gesture.is_idle() {
            let (theta, phi) = gesture.apply(time.delta_secs().min(0.25), orbit.theta, orbit.phi);
            orbit.theta = theta;
            orbit.phi = phi;
        }
        if wheel != 0.0 {
            orbit.radius = zoom_radius(orbit.radius, wheel, CHART_RADIUS_MIN, CHART_RADIUS_MAX);
        }
    }
}

/// Put each chart preview's camera on its orbit.
pub(crate) fn drive_menu_preview_cameras(
    q_nodes: Query<(&MenuPreviewOrbit, &MenuPreviewScene), Changed<MenuPreviewOrbit>>,
    mut q_cameras: Query<&mut Transform, With<Camera3d>>,
) {
    for (orbit, scene) in &q_nodes {
        let Ok(mut transform) = q_cameras.get_mut(scene.camera) else {
            continue;
        };
        *transform = Transform::from_translation(orbit_eye(orbit.radius, orbit.theta, orbit.phi))
            .looking_at(Vec3::ZERO, Vec3::Y);
    }
}

/// Place each chart preview's labels over their anchors through its camera,
/// and hide the ones off its image. A camera re-derives its target size a
/// frame after a resize and projects into the old size until then, so the
/// labels wait hidden for it.
pub(crate) fn project_menu_chart_labels(
    q_nodes: Query<(&MenuPreviewScene, &ComputedNode, &Children)>,
    q_cameras: Query<(&Camera, &GlobalTransform)>,
    mut q_labels: Query<(&MenuChartLabel, &mut Node, &mut Visibility)>,
) {
    for (scene, computed, children) in &q_nodes {
        let Ok((camera, cam_gt)) = q_cameras.get(scene.camera) else {
            continue;
        };
        let ready = camera.physical_target_size() == Some(computed.size().round().as_uvec2());
        let to_logical = computed.inverse_scale_factor();
        let size = computed.size() * to_logical;
        let mut labels = q_labels.iter_many_mut(children);
        while let Some((label, mut node, mut visibility)) = labels.fetch_next() {
            let projected = ready
                .then(|| camera.world_to_viewport(cam_gt, label.anchor).ok())
                .flatten()
                .map(|p| p * to_logical)
                .filter(|p| p.x >= 0.0 && p.y >= 0.0 && p.x <= size.x && p.y <= size.y);
            let Some(p) = projected else {
                visibility.set_if_neq(Visibility::Hidden);
                continue;
            };
            let (left, top) = (px(p.x + 6.0), px(p.y - 9.0));
            if node.left != left || node.top != top {
                node.left = left;
                node.top = top;
            }
            visibility.set_if_neq(Visibility::Inherited);
        }
    }
}

/// Render each preview only while its node shows: the role step's hull and
/// the Load details' chart stand hidden behind the other step and panel.
/// Size each shown preview's image to its node, once the node is laid out.
pub(crate) fn resize_menu_preview_targets(
    images: Option<ResMut<Assets<Image>>>,
    q_nodes: Query<(&ComputedNode, &InheritedVisibility, &MenuPreviewScene)>,
    mut q_cameras: Query<(&mut Camera, &mut Projection), With<Camera3d>>,
) {
    let Some(mut images) = images else {
        return;
    };
    for (computed, shown, scene) in &q_nodes {
        let Ok((mut camera, projection)) = q_cameras.get_mut(scene.camera) else {
            continue;
        };
        if camera.is_active != shown.get() {
            camera.is_active = shown.get();
        }
        let desired = computed.size().round().as_uvec2();
        if !shown.get() || desired.cmpeq(UVec2::ZERO).any() {
            continue;
        }
        resize_render_target(&mut images, &scene.image, desired, Some(projection));
    }
}
