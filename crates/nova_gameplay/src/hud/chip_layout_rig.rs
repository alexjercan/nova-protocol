//! Live-tree UI layout rig for the world-anchored HUD chips (task
//! 20260730-122909).
//!
//! Test-only support module. It builds an `App` carrying bevy_ui's real taffy
//! layout AND bevy_text's real measurement, spawns a chip bundle exactly as the
//! game's observers do, runs frames and reads the resulting `ComputedNode`s.
//!
//! Why a live tree rather than a widget-tree assert: the bug this rig was born
//! for is a LAYOUT bug. `Text` on the same entity as `children!` takes that
//! entity off taffy's leaf path, so the text measure is dropped and the chip's
//! bordered fill collapses to its padding while the glyphs still render at full
//! length. No amount of inspecting the spawned components can see that - only a
//! real layout pass produces the number.
//!
//! The assertions here are DERIVED from the live tree (`ComputedNode` sizes,
//! paddings, borders and `UiGlobalTransform` centres), never from re-running the
//! production geometry formula - see `test-must-not-reuse-the-formula-under-test`
//! in `LESSONS.md`.

use bevy::{
    asset::{AssetApp, AssetPlugin, RenderAssetUsages},
    camera::{Camera, ComputedCameraValues, RenderTargetInfo},
    image::{CompressedImageFormats, Image, ImageSampler, ImageType, TextureAtlasLayout},
    math::{Rect, UVec2},
    prelude::*,
    text::TextPlugin,
    ui::{ComputedNode, UiGlobalTransform, UiPlugin},
};

/// Rig viewport, big enough that no chip is ever clamped or wrapped by it.
const RIG_TARGET: UVec2 = UVec2::new(1280, 720);

/// An app with the real UI layout stack: asset server (the default font lives
/// in `Assets<Font>`), text measurement, taffy layout, transform propagation,
/// plus a camera carrying a render target so UI roots have something to lay out
/// against.
///
/// `bevy_text`'s `default_font` feature is on in this workspace and the chips
/// pass no font handle, so the glyph measurement here is the production one.
pub(super) fn chip_layout_app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        TransformPlugin,
        // UiPlugin pulls in the accessibility and picking backends and runs
        // `ui_focus_system` in PreUpdate, so their resources have to exist or
        // every frame panics on parameter validation.
        bevy::a11y::AccessibilityPlugin,
        bevy::input::InputPlugin,
        bevy::picking::PickingPlugin,
        bevy::picking::InteractionPlugin,
        TextPlugin,
        UiPlugin,
    ));

    // The glyph atlas and the image-content-size pass both take `Assets<Image>`;
    // in the game those collections come from the render plugins this rig has no
    // use for, so register them directly.
    app.init_asset::<Image>().init_asset::<TextureAtlasLayout>();

    app.world_mut().spawn((
        Camera2d,
        Camera {
            computed: ComputedCameraValues {
                target_info: Some(RenderTargetInfo {
                    physical_size: RIG_TARGET,
                    scale_factor: 1.0,
                }),
                ..default()
            },
            ..default()
        },
    ));

    app
}

/// Decode a real PNG from the repo's `assets/` tree straight into
/// `Assets<Image>` and return its handle.
///
/// The asset SERVER cannot serve a rig: its loads are asynchronous and nothing
/// here pumps the IO task pool, so a `server.load` handle never gains pixel
/// data. Anything that reads image CONTENT (the keycap trim scan) therefore
/// needs the bytes decoded synchronously, exactly as the runtime image loader
/// would - same sRGB flag, same `RenderAssetUsages::default()` (`MAIN_WORLD |
/// RENDER_WORLD`, which is what keeps the data readable in the main world at
/// all).
pub(super) fn load_png(app: &mut App, asset_path: &str) -> Handle<Image> {
    let path = std::path::Path::new("../../assets").join(asset_path);
    let bytes =
        std::fs::read(&path).unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
    let image = Image::from_buffer(
        &bytes,
        ImageType::Extension("png"),
        CompressedImageFormats::NONE,
        true,
        ImageSampler::Default,
        RenderAssetUsages::default(),
    )
    .unwrap_or_else(|error| panic!("decoding {}: {error}", path.display()));
    app.world_mut().resource_mut::<Assets<Image>>().add(image)
}

/// Run enough frames for text measurement and layout to settle: measurement
/// lands in `UiSystems::Content`, the glyph buffers in `UiSystems::PostLayout`,
/// so a single frame can report a pre-measurement box.
pub(super) fn settle(app: &mut App) {
    for _ in 0..4 {
        app.update();
    }
}

/// The measured geometry of one node, in physical pixels, read off the live
/// tree. `rect` is the node's border box in UI-global space.
#[derive(Debug, Clone, Copy)]
pub(super) struct MeasuredNode {
    pub size: Vec2,
    pub rect: Rect,
    /// Border + padding at the box's minimum corner (left, top).
    pub frame_min: Vec2,
    /// Border + padding at the box's maximum corner (right, bottom).
    pub frame_max: Vec2,
}

/// Read a laid-out node's measured geometry. Panics with a readable message if
/// the entity never reached layout.
pub(super) fn measure(app: &App, entity: Entity) -> MeasuredNode {
    let world = app.world();
    let computed = world
        .get::<ComputedNode>(entity)
        .unwrap_or_else(|| panic!("{entity:?} has no ComputedNode - it never reached UI layout"));
    let transform = world
        .get::<UiGlobalTransform>(entity)
        .unwrap_or_else(|| panic!("{entity:?} has no UiGlobalTransform"));
    let centre = transform.translation;
    MeasuredNode {
        size: computed.size,
        rect: Rect::from_center_size(centre, computed.size),
        frame_min: computed.border.min_inset + computed.padding.min_inset,
        frame_max: computed.border.max_inset + computed.padding.max_inset,
    }
}

impl MeasuredNode {
    /// The content box: the border box minus this node's own border and
    /// padding. This is where in-flow children live.
    pub fn content_rect(&self) -> Rect {
        Rect {
            min: self.rect.min + self.frame_min,
            max: self.rect.max - self.frame_max,
        }
    }
}

/// Lay `text` out as a bare LEAF text node and return its measured size.
///
/// This is the INDEPENDENT value the chip assertions compare against: a real
/// taffy+cosmic measurement of the same string at the same font size, produced
/// by a node that is definitely on the leaf path. It deliberately does not know
/// anything about `chip_node`'s geometry, so a chip that dropped its text
/// measure cannot satisfy it by accident.
pub(super) fn measure_text_leaf(app: &mut App, text: &str, font_px: f32) -> Vec2 {
    let probe = app
        .world_mut()
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                ..default()
            },
            Text::new(text),
            TextFont::from_font_size(font_px),
            TextLayout {
                linebreak: LineBreak::NoWrap,
                ..default()
            },
        ))
        .id();
    settle(app);
    let size = measure(app, probe).size;
    app.world_mut().entity_mut(probe).despawn();
    settle(app);
    assert!(
        size.x > 0.0 && size.y > 0.0,
        "the rig measured {text:?} to nothing ({size:?}) - text measurement is not wired"
    );
    size
}

/// The chip family's core invariant: the bordered, filled pill BACKS its whole
/// label.
///
/// Three derived checks, none of which re-runs the production geometry:
///
/// 1. the chip's box is at least an independently measured layout of the same
///    string (see [`measure_text_leaf`]) plus the chip's OWN measured padding
///    and border - this is what a collapsed chip fails;
/// 2. the label node really carries that whole string (its measured box matches
///    the independent one), so check 1 cannot pass against a truncated label;
/// 3. the label node's rect sits inside the chip's measured content box, so a
///    correctly-sized but misplaced pill still fails.
///
/// Returns the two measurements so a caller can record the numbers.
pub(super) fn assert_chip_backs_its_label(
    app: &mut App,
    chip: Entity,
    label: Entity,
    expected_text: &str,
    font_px: f32,
    what: &str,
) -> (MeasuredNode, MeasuredNode) {
    let reference = measure_text_leaf(app, expected_text, font_px);

    let chip_box = measure(app, chip);
    let label_box = measure(app, label);
    let content = chip_box.content_rect();

    let needed = reference + chip_box.frame_min + chip_box.frame_max;
    assert!(
        chip_box.size.x + 0.5 >= needed.x && chip_box.size.y + 0.5 >= needed.y,
        "{what}: the chip fill {:?} is smaller than an independent layout of \
         {expected_text:?} ({reference:?}) plus the chip's own padding/border \
         (needs at least {needed:?}) - the background covers only part of the text",
        chip_box.size
    );

    assert!(
        (label_box.size - reference).abs().max_element() <= 0.5,
        "{what}: the label node measured {:?} but an independent layout of \
         {expected_text:?} is {reference:?} - the label is not carrying the whole string",
        label_box.size
    );

    assert!(
        label_box.rect.min.x + 0.5 >= content.min.x
            && label_box.rect.min.y + 0.5 >= content.min.y
            && label_box.rect.max.x <= content.max.x + 0.5
            && label_box.rect.max.y <= content.max.y + 0.5,
        "{what}: the label rect {:?} spills out of the chip's content box {content:?}",
        label_box.rect
    );

    (chip_box, label_box)
}

/// Assert `child` is an IN-FLOW child sitting inside `chip`'s content box - the
/// shape the objective chip's diamond glyph moved to, so the same fill and
/// border back the mark as back the label.
pub(super) fn assert_child_sits_in_the_pill(app: &App, chip: Entity, child: Entity, what: &str) {
    let position_type = app
        .world()
        .get::<Node>(child)
        .unwrap_or_else(|| panic!("{what}: {child:?} has no Node"))
        .position_type;
    assert_eq!(
        position_type,
        PositionType::Relative,
        "{what}: the glyph must be an in-flow flex item so the pill grows around it"
    );

    let content = measure(app, chip).content_rect();
    let child_box = measure(app, child);
    // The WHOLE box, not just its centre - a glyph half out of the fill would
    // pass a centre check while looking exactly like the bug this fixes. The
    // rect is the node's LAYOUT box; a `UiTransform` rotation (the diamond's 45
    // degrees) spins the paint inside it without changing the box, pushing the
    // corners about `side * (sqrt(2) - 1) / 2` (~1.7 px for the 8 px diamond)
    // past each edge. The chip's 9/4 px padding absorbs that, and the content
    // box already excludes it.
    assert!(
        content.contains(child_box.rect.min) && content.contains(child_box.rect.max),
        "{what}: the glyph box {:?} is not inside the chip's content box {content:?}",
        child_box.rect
    );
}

/// Find the single descendant of `root` carrying `T`. Panics unless there is
/// exactly one, so a rig never silently asserts against the wrong node.
pub(super) fn only_descendant_with<T: Component>(app: &mut App, root: Entity) -> Entity {
    let mut found = Vec::new();
    let mut stack = vec![root];
    while let Some(current) = stack.pop() {
        if app.world().get::<T>(current).is_some() {
            found.push(current);
        }
        if let Some(children) = app.world().get::<Children>(current) {
            stack.extend(children.iter());
        }
    }
    assert_eq!(
        found.len(),
        1,
        "expected exactly one descendant of {root:?} with {}, found {}",
        std::any::type_name::<T>(),
        found.len()
    );
    found[0]
}

#[cfg(test)]
mod tests {
    use nova_ui::hud::chip_node;

    use super::*;

    /// The mechanism behind task 20260730-122909, pinned against the engine
    /// rather than against theory: taffy runs a node's measure function only on
    /// the LEAF path, so putting `Text` on a node that also has children drops
    /// the text measure and collapses the box to its padding plus border, while
    /// the glyphs still render at full length.
    ///
    /// This is why both world-anchored chips moved their label into a `Text`
    /// CHILD. If a future bevy measures container text too, this test fails and
    /// the chips can be simplified again.
    #[test]
    fn taffy_drops_the_text_measure_when_a_text_node_has_children() {
        let mut app = chip_layout_app();

        let leaf = app
            .world_mut()
            .spawn((
                chip_node(),
                Text::new("BEACON 1"),
                TextFont::from_font_size(12.0),
                TextLayout {
                    linebreak: LineBreak::NoWrap,
                    ..default()
                },
            ))
            .id();
        let container = app
            .world_mut()
            .spawn((
                chip_node(),
                Text::new("BEACON 1"),
                TextFont::from_font_size(12.0),
                TextLayout {
                    linebreak: LineBreak::NoWrap,
                    ..default()
                },
                children![Node {
                    position_type: PositionType::Absolute,
                    ..default()
                }],
            ))
            .id();
        settle(&mut app);

        let leaf_box = measure(&app, leaf);
        let container_box = measure(&app, container);
        let frame = leaf_box.frame_min + leaf_box.frame_max;

        assert!(
            leaf_box.size.x > frame.x + 1.0,
            "the childless chip measured its text (got {:?}, frame {frame:?})",
            leaf_box.size
        );
        assert_eq!(
            container_box.size, frame,
            "the same chip WITH a child collapses to exactly its padding+border \
             - the text measure was dropped"
        );
    }
}
