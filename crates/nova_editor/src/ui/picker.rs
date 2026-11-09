//! The colour picker: a window that opens beside the swatch that was pressed.
//!
//! A square picks saturation across and value down, a strip under it picks
//! the hue, and a second strip picks the alpha where the format holds one (a
//! theme's palette is six-digit `#rrggbb`, so its picker has none). A hex box
//! takes a typed colour, and the preview shows the colour the picker opened
//! on beside the colour it holds now.
//!
//! Every pick writes the document at once, so the stage and the swatch show
//! it live, and no step is made while the pointer is down. The release makes
//! the step, and every step since the picker opened folds into that one, so a
//! picker session is one undo step. Closing the window settles it the same
//! way. Esc takes the colour back to the one the picker opened on.

use bevy::{
    ecs::{relationship::RelatedSpawnerCommands, system::RunSystemOnce},
    picking::Pickable,
    prelude::*,
    ui::{
        BackgroundGradient, ColorStop, InterpolationColorSpace, LinearGradient, UiGlobalTransform,
    },
    ui_widgets::{observe, Activate},
};
use nova_ui::{
    prelude::{text_field, TextFieldFocused, TextFieldSpec, TextFieldValue},
    theme,
    theme::UiColorType,
    widget::{ThemedBorder, ThemedRadius},
};

use crate::{
    config::EditorSays,
    history::{
        cancel_edits_since, commit_edit_now, fold_edits_since, revert_to_last_commit, EditHistory,
    },
    inspect::{colour_text, write_field},
    ui::{
        inspector::{EditTargets, InspectorField, InspectorSwatch},
        window::{window_frame, EditorWindowLayer, TOP_MARGIN, WINDOW_W},
    },
};

/// The square's height. The window is the Inspector's width, so this keeps the
/// square about two parts wide to one tall: room to place a pick, short enough
/// that the window fits beside a swatch low in the panel.
const SQUARE_H: f32 = 140.0;
/// A strip's height.
const STRIP_H: f32 = 14.0;
/// The height a picker with an alpha strip stands, for keeping it on screen.
const PICKER_H: f32 = 290.0;
/// The gap between the swatch and the window beside it.
const SWATCH_GAP: f32 = 8.0;
/// The side of the ring that marks the pick in the square.
const RING: f32 = 10.0;
/// The width of the bar that marks the pick on a strip.
const BAR: f32 = 3.0;

/// The picker: which field it writes, and the colour it holds.
#[derive(Component, Clone, Debug)]
pub(crate) struct ColourWindow {
    /// The field the swatch it opened from writes: the same edit the row's
    /// box makes, so the two cannot disagree.
    pub(crate) field: InspectorField,
    /// Whether the format holds an alpha, and so the picker has a strip for it.
    alpha: bool,
    /// The colour as the picker holds it. Kept here rather than read back from
    /// the document: a grey has no hue, and a black no saturation, and the
    /// square and the strip still have to stand somewhere.
    hsva: Hsva,
    /// The colour it opened on: the preview's left half.
    opened: Color,
    /// The history revision it opened at. Every step since folds into one, and
    /// Esc takes them all back.
    since: Option<u64>,
}

impl ColourWindow {
    /// The colour it holds.
    fn colour(&self) -> Color {
        self.hsva.into()
    }
}

/// One of the areas a press picks in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PickAreaType {
    /// Saturation across, value down.
    Square,
    /// The hue, across.
    Hue,
    /// The alpha, across.
    Alpha,
}

/// An area a press picks in, and the window it belongs to.
#[derive(Component, Clone, Copy)]
pub(crate) struct PickArea {
    window: Entity,
    area: PickAreaType,
}

/// The mark that shows where an area's pick stands.
#[derive(Component, Clone, Copy)]
pub(crate) struct PickMark {
    window: Entity,
    area: PickAreaType,
}

/// The preview's right half: the colour the picker holds now.
#[derive(Component, Clone, Copy)]
pub(crate) struct PickPreview {
    window: Entity,
}

/// The picker's hex box.
#[derive(Component, Clone, Copy)]
pub(crate) struct PickHex {
    window: Entity,
}

/// `hsva` moved to where a pick at `at` puts it: `at` runs 0 to 1 across the
/// area and 0 to 1 down it.
pub(crate) fn picked(hsva: Hsva, area: PickAreaType, at: Vec2) -> Hsva {
    let at = at.clamp(Vec2::ZERO, Vec2::ONE);
    match area {
        PickAreaType::Square => Hsva {
            saturation: at.x,
            value: 1.0 - at.y,
            ..hsva
        },
        PickAreaType::Hue => Hsva {
            hue: at.x * 360.0,
            ..hsva
        },
        PickAreaType::Alpha => Hsva {
            alpha: at.x,
            ..hsva
        },
    }
}

/// `held` moved to `colour`, keeping the hue and saturation the colour no
/// longer carries: a pick down to black must not throw the hue away.
fn followed(held: Hsva, colour: Color) -> Hsva {
    let mut next = Hsva::from(colour);
    if next.value <= 0.0 {
        next.hue = held.hue;
        next.saturation = held.saturation;
    } else if next.saturation <= 0.0 {
        next.hue = held.hue;
    }
    next
}

/// Open the picker beside the swatch that was pressed, or put it away when it
/// is already open on that swatch: the swatch is the only control the row has,
/// so it is both the way in and the way out.
///
/// The colour it opens on is the one the SWATCH is painted, which is the value
/// the Inspector last read off the document.
pub(crate) fn on_open_colour_window(
    activate: On<Activate>,
    mut commands: Commands,
    swatches: Query<(
        &InspectorField,
        &InspectorSwatch,
        &BackgroundColor,
        &ComputedNode,
        &UiGlobalTransform,
    )>,
    layer: Option<Single<Entity, With<EditorWindowLayer>>>,
    open: Query<(Entity, &ColourWindow)>,
    screen: Option<Single<&Window>>,
    history: Option<Res<EditHistory>>,
) {
    let Ok((field, swatch, painted, node, transform)) = swatches.get(activate.entity) else {
        return;
    };
    let Some(layer) = layer else {
        return;
    };
    let mut reopening = false;
    for (window, open) in &open {
        reopening |= open.field == *field;
        commands.entity(window).despawn();
    }
    if reopening {
        return;
    }
    let size = screen.map_or(Vec2::new(1024.0, 768.0), |screen| screen.size());
    let corner = (transform.translation - node.size() * 0.5) * node.inverse_scale_factor;
    let at = beside(corner, size);
    let picker = ColourWindow {
        field: field.clone(),
        alpha: swatch.alpha,
        hsva: Hsva::from(painted.0),
        opened: painted.0,
        since: history.and_then(|history| history.revision()),
    };
    let label = swatch.label.clone();
    commands.entity(*layer).with_children(|layer| {
        spawn_picker(layer, picker, &label, at);
    });
}

/// Where a picker goes for a swatch whose top-left corner is `corner`, on a
/// screen `size` logical px: to the swatch's left, level with it, and on
/// screen. Clear of the rail where the screen allows; at a narrow width it
/// runs under the Inspector it came from instead.
fn beside(corner: Vec2, size: Vec2) -> Vec2 {
    let left = (corner.x - SWATCH_GAP - WINDOW_W).max(crate::ui::RAIL_W + SWATCH_GAP);
    let top = (corner.y - SWATCH_GAP)
        .min(size.y - PICKER_H - SWATCH_GAP)
        .max(TOP_MARGIN);
    Vec2::new(left, top)
}

/// The picker, in one place: the frame, the preview and hex box, the square
/// and the strips.
fn spawn_picker(
    layer: &mut RelatedSpawnerCommands<ChildOf>,
    picker: ColourWindow,
    label: &str,
    at: Vec2,
) {
    let colour = picker.colour();
    let opened = picker.opened;
    let alpha = picker.alpha;
    let field = picker.field.clone();
    window_frame(layer, "Colour Window", label, at, picker, |body, window| {
        body.spawn((
            Name::new("Colour Window Head"),
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(8),
                ..default()
            },
        ))
        .with_children(|head| {
            head.spawn((
                Name::new("Colour Window Compare"),
                Node {
                    flex_grow: 1.0,
                    height: px(22),
                    flex_direction: FlexDirection::Row,
                    border: UiRect::all(px(theme::BORDER_W)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                ThemedRadius::control(),
                BorderColor::all(Color::NONE),
                ThemedBorder::alpha(UiColorType::Text, 0.4),
            ))
            .with_children(|compare| {
                compare.spawn((
                    Name::new("Colour Window Was"),
                    Node {
                        flex_grow: 1.0,
                        ..default()
                    },
                    BackgroundColor(opened),
                ));
                compare.spawn((
                    Name::new("Colour Window Preview"),
                    PickPreview { window },
                    Node {
                        flex_grow: 1.0,
                        ..default()
                    },
                    BackgroundColor(colour),
                ));
            });
            head.spawn(Node {
                width: px(92),
                flex_shrink: 0.0,
                ..default()
            })
            .with_children(|slot| {
                // The row's own field: a typed hex is the same write and the
                // same step as typing it into the row.
                slot.spawn((
                    Name::new("Colour Window Hex"),
                    PickHex { window },
                    field,
                    text_field(TextFieldSpec::new(colour_text(colour)).max_chars(9).tool()),
                ));
            });
        });
        spawn_area(
            body,
            window,
            PickAreaType::Square,
            SQUARE_H,
            (
                BackgroundColor(hue_colour(Hsva::from(colour))),
                BackgroundGradient(vec![
                    LinearGradient::to_right(vec![
                        ColorStop::auto(Color::WHITE),
                        ColorStop::auto(Color::WHITE.with_alpha(0.0)),
                    ])
                    .in_color_space(InterpolationColorSpace::Srgba)
                    .into(),
                    LinearGradient::to_bottom(vec![
                        ColorStop::auto(Color::BLACK.with_alpha(0.0)),
                        ColorStop::auto(Color::BLACK),
                    ])
                    .in_color_space(InterpolationColorSpace::Srgba)
                    .into(),
                ]),
            ),
        );
        spawn_area(
            body,
            window,
            PickAreaType::Hue,
            STRIP_H,
            BackgroundGradient::from(
                LinearGradient::to_right(
                    [0.0, 60.0, 120.0, 180.0, 240.0, 300.0, 360.0]
                        .map(|hue| ColorStop::auto(Color::hsv(hue, 1.0, 1.0)))
                        .to_vec(),
                )
                .in_color_space(InterpolationColorSpace::Srgba),
            ),
        );
        if alpha {
            spawn_area(
                body,
                window,
                PickAreaType::Alpha,
                STRIP_H,
                alpha_gradient(colour),
            );
        }
    });
}

/// One area a press picks in, with its mark.
fn spawn_area(
    body: &mut RelatedSpawnerCommands<ChildOf>,
    window: Entity,
    area: PickAreaType,
    height: f32,
    paint: impl Bundle,
) {
    let name = match area {
        PickAreaType::Square => "Square",
        PickAreaType::Hue => "Hue",
        PickAreaType::Alpha => "Alpha",
    };
    body.spawn((
        Name::new(format!("Colour Window {name}")),
        PickArea { window, area },
        Node {
            width: percent(100),
            height: px(height),
            flex_shrink: 0.0,
            border: UiRect::all(px(theme::BORDER_W)),
            ..default()
        },
        BorderColor::all(Color::NONE),
        ThemedBorder::alpha(UiColorType::Text, 0.4),
        paint,
        observe(on_pick_press),
        observe(on_pick_drag),
        observe(on_pick_drag_end),
        observe(on_pick_release),
    ))
    .with_children(|area_node| {
        let (width, height, radius) = match area {
            PickAreaType::Square => (px(RING), px(RING), px(RING)),
            PickAreaType::Hue | PickAreaType::Alpha => (px(BAR), percent(100), px(0)),
        };
        area_node.spawn((
            Name::new(format!("Colour Window {name} Mark")),
            PickMark { window, area },
            Node {
                position_type: PositionType::Absolute,
                width,
                height,
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(radius),
                ..default()
            },
            BorderColor::all(Color::WHITE),
            BoxShadow::new(Color::BLACK, px(0), px(0), px(1), px(1)),
            // The press belongs to the area under the mark.
            Pickable::IGNORE,
        ));
    });
}

/// The pure hue a colour sits on: the square's ground.
fn hue_colour(hsva: Hsva) -> Color {
    Hsva::new(hsva.hue, 1.0, 1.0, 1.0).into()
}

/// The alpha strip: the colour from clear to whole.
fn alpha_gradient(colour: Color) -> BackgroundGradient {
    BackgroundGradient::from(
        LinearGradient::to_right(vec![
            ColorStop::auto(colour.with_alpha(0.0)),
            ColorStop::auto(colour.with_alpha(1.0)),
        ])
        .in_color_space(InterpolationColorSpace::Srgba),
    )
}

/// A press picks where it lands.
fn on_pick_press(press: On<PointerPress>, mut commands: Commands) {
    if press.button != PointerButton::Primary {
        return;
    }
    let Some(at) = press.hit.position else {
        return;
    };
    let area = press.entity;
    let at = at.truncate() + Vec2::splat(0.5);
    commands.queue(move |world: &mut World| pick_at(world, area, at));
}

/// A drag picks where the pointer is, inside the area or not: a pick held past
/// the edge stays on the edge.
fn on_pick_drag(
    drag: On<PointerDrag>,
    mut commands: Commands,
    areas: Query<(&ComputedNode, &UiGlobalTransform)>,
) {
    if drag.button != PointerButton::Primary {
        return;
    }
    let Ok((node, transform)) = areas.get(drag.entity) else {
        return;
    };
    let size = node.size();
    if size.x <= 0.0 || size.y <= 0.0 {
        return;
    }
    let physical = drag.pointer.position / node.inverse_scale_factor;
    let at = transform.inverse().transform_point2(physical) / size + Vec2::splat(0.5);
    let area = drag.entity;
    commands.queue(move |world: &mut World| pick_at(world, area, at));
}

/// A drag's end settles it.
fn on_pick_drag_end(end: On<PointerDragEnd>, mut commands: Commands, areas: Query<&PickArea>) {
    if let Ok(area) = areas.get(end.entity) {
        let window = area.window;
        commands.queue(move |world: &mut World| settle(world, window));
    }
}

/// A press that never dragged settles on its release.
fn on_pick_release(release: On<PointerRelease>, mut commands: Commands, areas: Query<&PickArea>) {
    if let Ok(area) = areas.get(release.entity) {
        let window = area.window;
        commands.queue(move |world: &mut World| settle(world, window));
    }
}

/// Pick at `at` in `area` (0 to 1 across, 0 to 1 down), and write the colour
/// into the document. No step: the release makes it.
pub(crate) fn pick_at(world: &mut World, area: Entity, at: Vec2) {
    let Some(&PickArea { window, area }) = world.get::<PickArea>(area) else {
        return;
    };
    let Some(mut picker) = world.get_mut::<ColourWindow>(window) else {
        return;
    };
    picker.hsva = picked(picker.hsva, area, at);
    let text = colour_text(picker.colour());
    let field = picker.field.clone();
    let wrote = world.run_system_once(move |mut targets: EditTargets, mut says: EditorSays| {
        let written = targets.edit(&field, |root, path, optional| {
            write_field(root, path, optional, &text)
        });
        if let Err(reason) = written {
            says.flag(reason);
        }
    });
    if let Err(fault) = wrote {
        error!("editor: a colour pick could not write: {fault}");
    }
}

/// Make the picker's edits one step: the step for what changed since the last
/// one, folded into every step since the picker opened.
pub(crate) fn settle(world: &mut World, window: Entity) {
    let Some(picker) = world.get::<ColourWindow>(window) else {
        return;
    };
    settle_field(world, picker.field.edit_label(), picker.since);
}

/// [`settle`], for a picker that is going and has left its label and start.
fn settle_field(world: &mut World, label: String, since: Option<u64>) {
    commit_edit_now(world, label.clone());
    if let Some(since) = since {
        fold_edits_since(world, since, label);
    }
}

/// A picker that closes - by its button, by its swatch, by the row going -
/// settles what it picked.
pub(crate) fn settle_on_close(
    remove: On<Remove<ColourWindow>>,
    mut commands: Commands,
    windows: Query<&ColourWindow>,
) {
    let Ok(picker) = windows.get(remove.entity) else {
        return;
    };
    let label = picker.field.edit_label();
    let since = picker.since;
    commands.queue(move |world: &mut World| settle_field(world, label, since));
}

/// Esc on an open picker: the colour goes back to the one it opened on, and
/// the picker closes. What it picked leaves no step.
pub(crate) fn revert_picker(world: &mut World, window: Entity) {
    let Some(since) = world.get::<ColourWindow>(window).map(|picker| picker.since) else {
        return;
    };
    revert_to_last_commit(world);
    if let Some(since) = since {
        cancel_edits_since(world, since);
    }
    if let Ok(entity) = world.get_entity_mut(window) {
        entity.despawn();
    }
}

/// Show what the DOCUMENT says, and close a picker whose row has gone.
///
/// The source is the Inspector's own swatch: it is painted from the document
/// every frame, so a colour changed by a typed hex, by undo or by anything else
/// reaches the picker without a second read of the node. A swatch that no
/// longer exists means the panel is on another node, and the picker goes.
#[expect(
    clippy::type_complexity,
    reason = "disjoint paint queries over the picker's own parts"
)]
pub(crate) fn sync_colour_windows(
    mut commands: Commands,
    mut windows: Query<(Entity, &mut ColourWindow)>,
    swatches: Query<(&InspectorField, &BackgroundColor), With<InspectorSwatch>>,
    mut grounds: Query<(&PickArea, &mut BackgroundColor), Without<InspectorSwatch>>,
    mut strips: Query<(&PickArea, &mut BackgroundGradient)>,
    mut previews: Query<
        (&PickPreview, &mut BackgroundColor),
        (Without<InspectorSwatch>, Without<PickArea>),
    >,
    mut marks: Query<(&PickMark, &mut Node)>,
    mut hexes: Query<(&PickHex, &mut TextFieldValue), Without<TextFieldFocused>>,
) {
    for (entity, mut picker) in &mut windows {
        let Some(colour) = swatches
            .iter()
            .find(|(field, _)| **field == picker.field)
            .map(|(_, painted)| painted.0)
        else {
            commands.entity(entity).despawn();
            continue;
        };
        // Compared as the hex the document holds: a pick between two 8-bit
        // steps must not be pulled back to the step every frame it moves.
        if colour_text(colour) != colour_text(picker.colour()) {
            let held = picker.hsva;
            picker.hsva = followed(held, colour);
        }
        let hsva = picker.hsva;
        let colour = picker.colour();
        for (area, mut ground) in &mut grounds {
            let wanted = hue_colour(hsva);
            if area.window == entity && area.area == PickAreaType::Square && ground.0 != wanted {
                ground.0 = wanted;
            }
        }
        for (area, mut strip) in &mut strips {
            if area.window == entity && area.area == PickAreaType::Alpha {
                let wanted = alpha_gradient(colour);
                if *strip != wanted {
                    *strip = wanted;
                }
            }
        }
        for (preview, mut painted) in &mut previews {
            if preview.window == entity && painted.0 != colour {
                painted.0 = colour;
            }
        }
        for (mark, mut node) in &mut marks {
            if mark.window != entity {
                continue;
            }
            let (across, down) = match mark.area {
                PickAreaType::Square => (hsva.saturation, Some(1.0 - hsva.value)),
                PickAreaType::Hue => (hsva.hue / 360.0, None),
                PickAreaType::Alpha => (hsva.alpha, None),
            };
            let half = if down.is_some() { RING } else { BAR } * 0.5;
            let left = percent(across.clamp(0.0, 1.0) * 100.0);
            if node.left != left {
                node.left = left;
                node.margin.left = px(-half);
            }
            if let Some(down) = down {
                let top = percent(down.clamp(0.0, 1.0) * 100.0);
                if node.top != top {
                    node.top = top;
                    node.margin.top = px(-half);
                }
            }
        }
        let hex = colour_text(colour);
        for (box_of, mut text) in &mut hexes {
            if box_of.window == entity && text.0 != hex {
                text.0.clone_from(&hex);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The square picks saturation across and value down; a strip picks
    /// across. A pick past an edge stays on it.
    #[test]
    fn a_pick_reads_across_and_down() {
        let red = Hsva::new(0.0, 1.0, 1.0, 1.0);
        let pale = picked(red, PickAreaType::Square, Vec2::new(0.25, 0.5));
        assert!((pale.saturation - 0.25).abs() < 1e-6);
        assert!((pale.value - 0.5).abs() < 1e-6);
        let blue = picked(red, PickAreaType::Hue, Vec2::new(2.0 / 3.0, 0.9));
        assert!((blue.hue - 240.0).abs() < 1e-3);
        assert_eq!(blue.value, 1.0, "the hue strip leaves the value alone");
        let edge = picked(red, PickAreaType::Alpha, Vec2::new(1.4, 0.0));
        assert_eq!(edge.alpha, 1.0);
    }

    /// The picker opens to the left of its swatch and level with it, stays on
    /// screen for a swatch low in the panel, and stays off the rail at a
    /// narrow width.
    #[test]
    fn a_picker_opens_beside_its_swatch() {
        let screen = Vec2::new(1280.0, 720.0);
        let at = beside(Vec2::new(1000.0, 300.0), screen);
        assert_eq!(
            at,
            Vec2::new(1000.0 - SWATCH_GAP - WINDOW_W, 300.0 - SWATCH_GAP)
        );
        let low = beside(Vec2::new(1000.0, 700.0), screen);
        assert!(
            low.y + PICKER_H <= screen.y,
            "a low swatch keeps it on screen: {low}"
        );
        let narrow = beside(Vec2::new(400.0, 300.0), screen);
        assert!(
            narrow.x >= crate::ui::RAIL_W,
            "it never covers the rail: {narrow}"
        );
    }

    /// A pick down to black keeps its hue and saturation, so the square and
    /// the strip do not jump back to red when the value comes up again.
    #[test]
    fn a_black_keeps_the_hue_it_was_picked_on() {
        let held = Hsva::new(200.0, 0.6, 0.5, 1.0);
        let followed = followed(held, Color::BLACK);
        assert_eq!(followed.hue, 200.0);
        assert_eq!(followed.saturation, 0.6);
        assert_eq!(followed.value, 0.0);
    }
}
