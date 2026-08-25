//! The left rail's widgets: the Scene tree rows and the ship settings rows.
//! The theme and the shared button shapes live in `nova_ui`; this module only
//! assembles editor-specific rows out of them.

use bevy::{picking::hover::Hovered, prelude::*, ui_widgets::Button};
use nova_ui::{
    prelude::{panel, ThemedButton, UiSkin},
    theme,
    widget::{checkbox, list_row_colors, ListRow},
};

use crate::config::{SceneRow, SkinToggleCheckbox, StyleChoice};

/// The cladding toggle: a tool row that is a SETTING rather than a mode, so it
/// carries the shared `checkbox` widget instead of a `ButtonValue`.
///
/// The row is the button and the box is a picture. `ui_widgets::Button` on the
/// box would swallow the press before the row saw it, and a 22px target in a
/// 150px rail is a worse one than the row it sits in.
pub(crate) fn skin_toggle_row(on: bool, skin: UiSkin) -> impl Bundle {
    (
        ThemedButton,
        Button,
        Hovered::default(),
        Node {
            width: percent(100),
            min_height: px(30),
            margin: UiRect::vertical(px(2)),
            padding: UiRect::axes(px(10), px(6)),
            border: UiRect::all(px(theme::BORDER_W)),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            column_gap: px(6),
            border_radius: BorderRadius::all(px(theme::RADIUS)),
            ..default()
        },
        BorderColor::all(theme::PHOSPHOR_MUTED),
        BackgroundColor(theme::SCREEN_0),
        children![
            (
                Text::new("Ship Skin"),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(theme::PHOSPHOR),
            ),
            (SkinToggleCheckbox, checkbox(on, skin)),
        ],
    )
}

/// One look in the list under the cladding toggle.
///
/// A `ListRow` and not a `themed_button`: the rows are a SELECTION over the
/// merged style catalog, which is what a list row paints, and the shared
/// reconciler then repaints this one from its own `Selected`/`Hovered` without
/// the editor owning any colour.
///
/// Deliberately COMPACT - 22px against a tool button's 34. The rail is 150px
/// wide on a 1024x768 window and the catalog is as long as the content merge
/// makes it, so five looks at tool height push Play off the bottom of the
/// screen. Measured, not guessed.
pub(crate) fn style_row(id: &str, name: &str, selected: bool, skin: UiSkin) -> impl Bundle {
    let (background, border) = list_row_colors(selected, false, skin);
    (
        ListRow,
        StyleChoice(id.to_string()),
        Button,
        Hovered::default(),
        Node {
            width: percent(100),
            min_height: px(22),
            margin: UiRect::bottom(px(2)),
            padding: UiRect::axes(px(10), px(2)),
            border: UiRect::all(px(theme::BORDER_W)),
            align_items: AlignItems::Center,
            border_radius: BorderRadius::all(px(theme::RADIUS)),
            ..default()
        },
        BorderColor::all(border),
        BackgroundColor(background),
        children![(
            Text::new(name.to_string()),
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(theme::PHOSPHOR),
        )],
    )
}

/// A root row's left padding, matching the other rails rows' `10px`-ish inset.
const INDENT_BASE: f32 = 8.0;
/// What one level of nesting is worth. Wide enough to read as a step, narrow
/// enough that a section (depth 2) still shows most of its id.
const INDENT_STEP: f32 = 9.0;

/// Row type size (px). One step under the panel type: the rail is a tree of
/// minted ids in 150px, and every point of size is a character of id the row
/// can show before it clips.
const ROW_TEXT: f32 = 11.0;

/// One row of the Scene tree: the scenario root, a ship, or a section of the
/// entered ship.
///
/// The same `ListRow` shape the look rows use, so the shared reconciler paints
/// the selection and the hover and this module owns no colour. `lead` is the
/// glyph in front of the label - one muted text node instead of an icon asset,
/// which is also what keeps the tree in the terminal look the rest of the
/// screen wears.
///
/// `depth` is spent on the row's LEFT PADDING rather than on drawn connectors.
/// Indentation is what the eye reads a tree by, and it costs no width in a
/// 150px rail: `|- ` in front of every child ate 18px of the label on every
/// row, and a minted id is exactly the thing that then ran out of room.
pub(crate) fn scene_row(
    depth: usize,
    lead: &str,
    label: &str,
    trail: &str,
    selected: bool,
    skin: UiSkin,
) -> impl Bundle {
    let (background, border) = list_row_colors(selected, false, skin);
    #[expect(
        clippy::cast_precision_loss,
        reason = "tree depth is single digits, not a precision question"
    )]
    let indent = px(INDENT_BASE + INDENT_STEP * depth as f32);
    // A minted id can outgrow the 150px rail. NoWrap + clip keeps every row
    // one line tall - a wrapped lead column stacked its glyphs vertically and
    // the tree stopped reading as a tree.
    let one_line = TextLayout {
        linebreak: LineBreak::NoWrap,
        ..default()
    };
    let row_font = TextFont {
        font_size: FontSize::Px(ROW_TEXT),
        ..default()
    };
    (
        ListRow,
        Button,
        Hovered::default(),
        Node {
            width: percent(100),
            min_height: px(22),
            margin: UiRect::bottom(px(2)),
            padding: UiRect {
                left: indent,
                right: px(6),
                top: px(2),
                bottom: px(2),
            },
            border: UiRect::all(px(theme::BORDER_W)),
            align_items: AlignItems::Center,
            column_gap: px(6),
            border_radius: BorderRadius::all(px(theme::RADIUS)),
            overflow: Overflow::clip(),
            ..default()
        },
        BorderColor::all(border),
        BackgroundColor(background),
        children![
            (
                Text::new(lead.to_string()),
                one_line,
                row_font.clone(),
                TextColor(theme::PHOSPHOR_MUTED),
            ),
            // The label is the half that may clip, so it sits in the shrinking
            // column: `flex_basis` 0 plus `min_width` 0 is what lets a long id
            // give its width back instead of pushing the trail out of the row.
            (
                Node {
                    flex_grow: 1.0,
                    flex_basis: px(0),
                    min_width: px(0),
                    overflow: Overflow::clip(),
                    ..default()
                },
                children![(
                    Text::new(label.to_string()),
                    one_line,
                    row_font.clone(),
                    TextColor(theme::PHOSPHOR),
                )],
            ),
            // Which one this is. Fixed at the row's right edge, because it is
            // the only thing telling a run of same-part rows apart.
            (
                Text::new(trail.to_string()),
                one_line,
                row_font,
                TextColor(theme::PHOSPHOR_MUTED),
            )
        ],
    )
}

/// What a Scene row's hover reveals: the word its icon stands for, and the id
/// the row itself had to clip.
///
/// Carried by the ROW rather than looked up from the node, so the hint and the
/// icon come out of the same pass over the document and cannot disagree.
#[derive(Component, Clone)]
pub(crate) struct SceneRowHint {
    /// One word: HULL, BEACON, SHIP - PLAYER.
    pub(crate) kind: String,
    /// The node's whole id.
    pub(crate) id: String,
}

/// The one hint panel, parked off-screen until a row is hovered.
#[derive(Component)]
pub(crate) struct SceneRowTooltip;

/// The hint's own layer. Above the floating windows (30), because a hint is
/// the frontmost thing on screen for as long as the pointer rests.
const TOOLTIP_Z: i32 = 40;
/// Gap between the rail's right edge and the hint.
const TOOLTIP_GAP: f32 = 8.0;

/// The hint panel: two lines, the kind over the id, absolutely positioned by
/// [`sync_scene_tooltip`].
///
/// Deaf to the pointer: it stands beside the row it describes, over the stage,
/// and a hint that blocked the placement raycast would make the rail's own
/// tree unusable to build beside.
pub(crate) fn scene_tooltip(skin: UiSkin) -> impl Bundle {
    (
        Name::new("Scene Row Hint"),
        SceneRowTooltip,
        GlobalZIndex(TOOLTIP_Z),
        Pickable::IGNORE,
        Node {
            display: Display::None,
            position_type: PositionType::Absolute,
            left: px(0),
            top: px(0),
            max_width: px(280),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::FlexStart,
            padding: UiRect::axes(px(8), px(5)),
            border: UiRect::all(px(theme::BORDER_W)),
            border_radius: BorderRadius::all(px(theme::RADIUS)),
            ..default()
        },
        panel(skin),
        children![
            (
                Name::new("Scene Row Hint Kind"),
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(10.0),
                    ..default()
                },
                TextColor(theme::PHOSPHOR_MUTED),
            ),
            (
                Name::new("Scene Row Hint Id"),
                Text::new(""),
                TextLayout {
                    linebreak: LineBreak::AnyCharacter,
                    ..default()
                },
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(theme::PHOSPHOR),
            )
        ],
    )
}

/// Put the hint beside the row under the pointer, and take it away again when
/// the pointer leaves.
///
/// Positioned from the ROW's own laid-out box rather than from the pointer, so
/// the hint sits still while the pointer moves inside a row - a hint that
/// chased the cursor is one more thing moving on a screen whose whole point is
/// the model standing in the middle of it.
pub(crate) fn sync_scene_tooltip(
    rows: Query<(&Hovered, &SceneRowHint, &ComputedNode, &UiGlobalTransform), With<SceneRow>>,
    mut tooltips: Query<(&mut Node, &Children), With<SceneRowTooltip>>,
    mut texts: Query<&mut Text>,
) {
    let hovered = rows
        .iter()
        .find(|(hovered, _, computed, _)| hovered.get() && computed.size().x > 0.0);
    for (mut node, children) in &mut tooltips {
        let Some((_, hint, computed, transform)) = hovered else {
            if node.display != Display::None {
                node.display = Display::None;
            }
            continue;
        };
        // Logical pixels: `Node` is written in them and the computed box is
        // not (see `ComputedNode::inverse_scale_factor`).
        let scale = computed.inverse_scale_factor();
        let row = Rect::from_center_size(transform.translation * scale, computed.size() * scale);
        let left = px(row.max.x + TOOLTIP_GAP);
        let top = px(row.min.y);
        if node.display != Display::Flex {
            node.display = Display::Flex;
        }
        if node.left != left {
            node.left = left;
        }
        if node.top != top {
            node.top = top;
        }
        for (index, wanted) in [hint.kind.as_str(), hint.id.as_str()]
            .into_iter()
            .enumerate()
        {
            let Some(line) = children.iter().nth(index) else {
                continue;
            };
            let Ok(mut text) = texts.get_mut(line) else {
                continue;
            };
            if text.0 != wanted {
                text.0 = wanted.to_string();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::{ecs::system::RunSystemOnce, math::Affine2};

    use super::*;

    /// A row with a laid-out box, hovered or not, plus the one hint panel.
    /// Returns the panel and its two text lines.
    fn tree(world: &mut World, hovered: bool) -> (Entity, Entity, Entity) {
        world.spawn((
            SceneRow(Entity::PLACEHOLDER),
            SceneRowHint {
                kind: "BEACON".to_string(),
                id: "beacon_home".to_string(),
            },
            Hovered(hovered),
            ComputedNode {
                size: Vec2::new(140.0, 22.0),
                inverse_scale_factor: 1.0,
                ..default()
            },
            UiGlobalTransform::from(Affine2::from_translation(Vec2::new(80.0, 100.0))),
        ));
        let kind = world.spawn(Text::new("")).id();
        let id = world.spawn(Text::new("")).id();
        let tooltip = world
            .spawn((
                SceneRowTooltip,
                Node {
                    display: Display::None,
                    ..default()
                },
            ))
            .add_children(&[kind, id])
            .id();
        (tooltip, kind, id)
    }

    /// A 150px row clips its id, so resting on one reveals what it says in
    /// full - and what its icon stood for.
    #[test]
    fn hovering_a_row_reveals_its_kind_and_its_whole_id() {
        let mut world = World::new();
        let (tooltip, kind, id) = tree(&mut world, true);

        world
            .run_system_once(sync_scene_tooltip)
            .expect("the sync runs");

        let node = world.get::<Node>(tooltip).expect("a node");
        assert_eq!(node.display, Display::Flex);
        assert_eq!(
            node.left,
            px(150.0 + TOOLTIP_GAP),
            "beside the row, clear of the rail"
        );
        assert_eq!(node.top, px(89.0), "level with the row");
        assert_eq!(world.get::<Text>(kind).expect("the kind").0, "BEACON");
        assert_eq!(world.get::<Text>(id).expect("the id").0, "beacon_home");
    }

    /// The pointer leaving takes the hint with it: a hint left standing over
    /// the stage is a hint about a row nobody is looking at.
    #[test]
    fn the_hint_goes_away_with_the_pointer() {
        let mut world = World::new();
        let (tooltip, _, _) = tree(&mut world, false);

        world
            .run_system_once(sync_scene_tooltip)
            .expect("the sync runs");

        assert_eq!(
            world.get::<Node>(tooltip).expect("a node").display,
            Display::None
        );
    }
}
