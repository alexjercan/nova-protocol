//! The left rail's widgets: the Scene tree rows and the ship settings rows.
//! The theme and the shared button shapes live in `nova_ui`; this module only
//! assembles editor-specific rows out of them.

use bevy::{
    picking::hover::Hovered,
    prelude::*,
    ui_widgets::{observe, Button},
};
use nova_ui::{
    prelude::{icon_button, panel, IconButtonType, ThemedButton, UiText, CLOSE_GLYPH},
    theme,
    theme::{density, UiColor},
    widget::{
        checkbox, checkbox_glyph, ListRow, ThemedBorder, ThemedFill, ThemedRadius, ThemedText,
    },
};
use nova_wfc::prelude::WfcZone;

use crate::{
    config::{
        PartChoice, PartTick, PartZoneChip, SceneRow, SkinToggleCheckbox, StyleChoice, StyleSwatch,
    },
    ui::layer,
};

/// The skin toggle: a tool row that is a SETTING rather than a mode, so it
/// carries the shared `checkbox` widget instead of a `ButtonValue`.
///
/// The row is the button and the box is a picture. `ui_widgets::Button` on the
/// box would swallow the press before the row saw it, and a 22px target in a
/// 150px rail is a worse one than the row it sits in.
pub(crate) fn skin_toggle_row(on: bool) -> impl Bundle {
    (
        ThemedButton,
        Button,
        Hovered::default(),
        Node {
            width: percent(100),
            min_height: px(density::CONTROL_H),
            padding: UiRect::axes(px(density::PANEL_PAD), px(0)),
            border: UiRect::all(px(theme::BORDER_W)),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            column_gap: px(6),
            ..default()
        },
        ThemedRadius::control(),
        BorderColor::all(Color::NONE),
        ThemedBorder::new(UiColor::Label),
        BackgroundColor(Color::NONE),
        ThemedFill::new(UiColor::Surface),
        children![
            (
                UiText,
                Text::new("Ship Skin"),
                TextFont {
                    font_size: FontSize::Px(density::TEXT),
                    ..default()
                },
                TextColor(Color::NONE),
                ThemedText::new(UiColor::Primary),
            ),
            (SkinToggleCheckbox, checkbox(on)),
        ],
    )
}

/// One style in the list under the skin toggle.
///
/// A `ListRow` and not a `themed_button`: the rows are a SELECTION over the
/// merged style catalog, which is what a list row paints, and the shared
/// reconciler then repaints this one from its own `Selected`/`Hovered` without
/// the editor owning any colour.
///
/// `colour` is the paint the style puts on a hull's top surface, shown as a
/// block in front of the name: the choice is a purely visual one, and five
/// words for five looks made a builder open each in turn to find out what they
/// meant.
///
/// Deliberately COMPACT - 22px against a tool button's 34. The rail is 150px
/// wide on a 1024x768 window and the catalog is as long as the content merge
/// makes it, so five styles at tool height push Play off the bottom of the
/// screen. Measured, not guessed. The block is sized to the row for the same
/// reason: the shared 22px `swatch` would set the height of the list.
pub(crate) fn style_row(id: &str, name: &str, colour: Color) -> impl Bundle {
    (
        ListRow,
        StyleChoice(id.to_string()),
        Button,
        Hovered::default(),
        Node {
            flex_basis: percent(48),
            flex_grow: 1.0,
            min_width: px(0),
            min_height: px(22),
            margin: UiRect::bottom(px(2)),
            padding: UiRect::axes(px(6), px(2)),
            border: UiRect::all(px(theme::BORDER_W)),
            align_items: AlignItems::Center,
            column_gap: px(6),
            ..default()
        },
        ThemedRadius::control(),
        BorderColor::all(Color::NONE),
        BackgroundColor(Color::NONE),
        children![
            (
                StyleSwatch(colour),
                Node {
                    width: px(STYLE_SWATCH),
                    height: px(STYLE_SWATCH),
                    flex_shrink: 0.0,
                    border: UiRect::all(px(theme::BORDER_W)),
                    border_radius: BorderRadius::all(px(2)),
                    ..default()
                },
                BorderColor::all(Color::NONE),
                ThemedBorder::alpha(UiColor::Primary, 0.4),
                BackgroundColor(colour),
            ),
            (
                UiText,
                Text::new(name.to_string()),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::NONE),
                ThemedText::new(UiColor::Primary),
            )
        ],
    )
}

/// One row of the Generate block's part list: whether the collapse may draw
/// this section, and what it is called.
///
/// A `ListRow` marked with `Selected`, like the style rows - so the shared
/// reconciler paints the tick state and the hover, and the row itself is the
/// answer the generator reads. The tick is a glyph rather than the 22px
/// `checkbox` widget: this list is as long as the catalog, and a 22px box per
/// row would push the button that uses it off the rail.
pub(crate) fn part_row(
    prototype: &str,
    name: &str,
    drawn: bool,
    zone: Option<WfcZone>,
) -> impl Bundle {
    (
        ListRow,
        PartChoice {
            prototype: prototype.to_string(),
            zone,
        },
        Button,
        Hovered::default(),
        Node {
            width: percent(100),
            min_height: px(18),
            margin: UiRect::bottom(px(1)),
            padding: UiRect::axes(px(5), px(1)),
            border: UiRect::all(px(theme::BORDER_W)),
            align_items: AlignItems::Center,
            column_gap: px(4),
            ..default()
        },
        ThemedRadius::control(),
        BorderColor::all(Color::NONE),
        BackgroundColor(Color::NONE),
        children![
            (
                PartTick,
                UiText,
                Text::new(checkbox_glyph(drawn)),
                TextFont {
                    font_size: FontSize::Px(11.0),
                    ..default()
                },
                TextColor(Color::NONE),
                ThemedText::new(UiColor::Primary),
                Node {
                    width: px(7),
                    flex_shrink: 0.0,
                    ..default()
                },
            ),
            (
                UiText,
                Text::new(name.to_string()),
                TextFont {
                    font_size: FontSize::Px(11.0),
                    ..default()
                },
                TextColor(Color::NONE),
                ThemedText::new(UiColor::Primary),
                // The name takes the slack so the chip stays hard right,
                // where it reads as a setting on the row rather than as part
                // of the name.
                Node {
                    flex_grow: 1.0,
                    ..default()
                },
            ),
            zone_chip(zone, drawn),
        ],
    )
}

/// The zone chip: the second control on a part row, and the only place a
/// builder says WHERE a part may stand.
///
/// A button inside a button, which is safe here because
/// [`on_part_choice`](crate::ui::on_part_choice) rules on the entity the
/// activation names rather than on the one it reaches: a press on the chip
/// carries the chip, the row's own observer does not recognise it, and the tick
/// does not flip under the press that cycled the zone.
///
/// Hidden while the row is unticked. A zone on a part the collapse may not draw
/// says nothing, and a chip on every row of a catalog-long list is a column of
/// noise.
pub(crate) fn zone_chip(zone: Option<WfcZone>, drawn: bool) -> impl Bundle {
    (
        PartZoneChip,
        Button,
        Hovered::default(),
        observe(crate::ui::on_part_zone),
        Node {
            display: if drawn { Display::Flex } else { Display::None },
            padding: UiRect::axes(px(4), px(0)),
            flex_shrink: 0.0,
            border: UiRect::all(px(theme::BORDER_W)),
            ..default()
        },
        ThemedRadius::control(),
        BorderColor::all(Color::NONE),
        ThemedBorder::new(UiColor::Label),
        children![(
            UiText,
            Text::new(zone_label(zone).to_string()),
            TextFont {
                font_size: FontSize::Px(10.0),
                ..default()
            },
            TextColor(Color::NONE),
            ThemedText::new(UiColor::Secondary),
        )],
    )
}

/// What a zone is called on a chip. `any` rather than a blank, so an unzoned
/// part reads as a decision rather than as a control nobody wired up. Short,
/// because the chip shares a 190px row with a section name that is already
/// close to filling it.
pub(crate) fn zone_label(zone: Option<WfcZone>) -> &'static str {
    match zone {
        None => "any",
        Some(WfcZone::Bow) => "bow",
        Some(WfcZone::Amidships) => "mid",
        Some(WfcZone::Stern) => "stern",
        Some(WfcZone::Dorsal) => "dorsal",
        Some(WfcZone::Ventral) => "ventral",
        Some(WfcZone::Flank) => "flank",
    }
}

/// The zone a press moves this row on to, wrapping back to unzoned.
pub(crate) fn next_zone(zone: Option<WfcZone>) -> Option<WfcZone> {
    match zone {
        None => Some(WfcZone::Bow),
        Some(WfcZone::Bow) => Some(WfcZone::Amidships),
        Some(WfcZone::Amidships) => Some(WfcZone::Stern),
        Some(WfcZone::Stern) => Some(WfcZone::Dorsal),
        Some(WfcZone::Dorsal) => Some(WfcZone::Ventral),
        Some(WfcZone::Ventral) => Some(WfcZone::Flank),
        Some(WfcZone::Flank) => None,
    }
}

/// The style block's side, in px. A square that fits inside a 22px row with its
/// padding, which is what keeps the list as short as it was without one.
const STYLE_SWATCH: f32 = 12.0;

/// The row's own delete, revealed by hover or by selection.
///
/// The pointer's way to the verb Del takes from the keyboard, on the surface
/// that already lists everything the document holds. Hidden until the row is
/// under the pointer or marked, so a 150px rail is not fifteen delete buttons
/// waiting to be misclicked.
///
/// The slot is a row's content height and the button a 22px square centred in
/// it: the button keeps its hit target across, and a revealed delete does not
/// make its row taller than the rows around it.
pub(crate) fn row_trash() -> impl Bundle {
    (
        SceneRowTrash,
        Node {
            display: Display::None,
            height: px(density::ROW_H - 2.0 * theme::BORDER_W),
            flex_shrink: 0.0,
            align_items: AlignItems::Center,
            ..default()
        },
        children![(
            Name::new("Scene Row Delete"),
            icon_button(IconButtonType::Delete, CLOSE_GLYPH, "Delete (Del)"),
            observe(super::on_row_trash),
        )],
    )
}

/// A Scene row's delete affordance.
#[derive(Component)]
pub(crate) struct SceneRowTrash;

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
    /// Which way the row's caret points. `None` on a leaf.
    pub(crate) open: Option<bool>,
    /// The mark the row wears, which the Inspector header repeats.
    pub(crate) lead: String,
}

/// The one hint panel, parked off-screen until a row is hovered.
#[derive(Component)]
pub(crate) struct SceneRowTooltip;

/// Gap between the rail's right edge and the hint.
const TOOLTIP_GAP: f32 = 8.0;

/// The hint panel: two lines, the kind over the id, absolutely positioned by
/// [`sync_scene_tooltip`].
///
/// Deaf to the pointer: it stands beside the row it describes, over the stage,
/// and a hint that blocked the placement raycast would make the rail's own
/// tree unusable to build beside.
pub(crate) fn scene_tooltip() -> impl Bundle {
    (
        Name::new("Scene Row Hint"),
        SceneRowTooltip,
        GlobalZIndex(layer::TOOLTIP_Z),
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
            ..default()
        },
        ThemedRadius::control(),
        panel(),
        children![
            (
                Name::new("Scene Row Hint Kind"),
                UiText,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(10.0),
                    ..default()
                },
                TextColor(Color::NONE),
                ThemedText::new(UiColor::Label),
            ),
            (
                Name::new("Scene Row Hint Id"),
                UiText,
                Text::new(""),
                TextLayout {
                    linebreak: LineBreak::AnyCharacter,
                    ..default()
                },
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::NONE),
                ThemedText::new(UiColor::Primary),
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
                open: None,
                lead: String::new(),
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
