//! A popup: a small panel at a point, over a catcher that closes it.
//!
//! The context menu of a tree row and the list of a [`Select`](super::Select)
//! are both this. The popup closes on a [`popup_row`] press, on any press
//! outside its panel, and on [`close_popups`]. One popup is open at a time.

use bevy::{
    ecs::relationship::RelatedSpawnerCommands,
    picking::hover::Hovered,
    prelude::*,
    ui::UiScale,
    ui_widgets::{Activate, Button},
};

use super::{panel, ListRow, ThemedRadius, ThemedText, UiText};
use crate::theme::{density, UiColor, BORDER_W};

/// The catcher behind an open popup.
#[derive(Component)]
pub struct PopupRoot;

/// The panel of a popup, and the point it opens at in logical window pixels.
#[derive(Component)]
pub struct PopupPanel(pub Vec2);

/// A row of a popup. Pressing one closes the popup.
#[derive(Component)]
pub struct PopupRow;

/// The narrowest a popup panel draws.
const POPUP_MIN_W: f32 = 170.0;

/// Open a popup at `at`, in logical window pixels, on rung `z`. The caller adds
/// its rows to the returned panel, and any cleanup marker to the returned
/// catcher. Any popup already open is the caller's to close first.
pub fn spawn_popup(
    commands: &mut Commands,
    at: Vec2,
    z: i32,
    rows: impl FnOnce(&mut RelatedSpawnerCommands<ChildOf>),
) -> Entity {
    commands
        .spawn((
            Name::new("Popup"),
            PopupRoot,
            GlobalZIndex(z),
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn((
                Name::new("Popup Panel"),
                PopupPanel(at),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(at.x),
                    top: px(at.y),
                    min_width: px(POPUP_MIN_W),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(px(4)),
                    border: UiRect::all(px(BORDER_W)),
                    ..default()
                },
                ThemedRadius::control(),
                panel(),
            ))
            .with_children(rows);
        })
        .id()
}

/// One line of a popup: a label, and a muted hint such as its key.
pub fn popup_row(label: &str, hint: &str) -> impl Bundle {
    let text = |line: &str, colour: UiColor| {
        (
            UiText,
            Text::new(line.to_string()),
            TextLayout {
                linebreak: LineBreak::NoWrap,
                ..default()
            },
            TextFont {
                font_size: FontSize::Px(density::TEXT),
                ..default()
            },
            TextColor(Color::NONE),
            ThemedText::new(colour),
        )
    };
    (
        PopupRow,
        ListRow,
        Button,
        Hovered::default(),
        Node {
            min_height: px(density::ROW_H),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            column_gap: px(18),
            padding: UiRect::axes(px(8), px(0)),
            border: UiRect::all(px(BORDER_W)),
            ..default()
        },
        ThemedRadius::control(),
        BorderColor::all(Color::NONE),
        BackgroundColor(Color::NONE),
        children![text(label, UiColor::Primary), text(hint, UiColor::Label)],
    )
}

/// Close every open popup. An Escape rung calls this; the return says whether
/// there was one to close.
pub fn close_popups(commands: &mut Commands, roots: &Query<Entity, With<PopupRoot>>) -> bool {
    let mut closed = false;
    for root in roots {
        commands.entity(root).despawn();
        closed = true;
    }
    closed
}

/// Close the popup when one of its rows is pressed.
pub(super) fn close_popup_on_row(
    activate: On<Activate>,
    mut commands: Commands,
    rows: Query<(), With<PopupRow>>,
    roots: Query<Entity, With<PopupRoot>>,
) {
    if rows.contains(activate.entity) {
        close_popups(&mut commands, &roots);
    }
}

/// Close the popup on a press outside its panel, with any mouse button.
pub(super) fn close_popup_on_press(
    press: On<PointerPress>,
    mut commands: Commands,
    roots: Query<Entity, With<PopupRoot>>,
) {
    // The root's press bubbles to the window, and this observer hears it there
    // too: only the level the press started on closes the popup.
    let target = press.original_event_target();
    if press.entity == target && roots.contains(target) {
        close_popups(&mut commands, &roots);
    }
}

/// Keep each panel inside the window, once layout has measured it.
pub(super) fn place_popups(
    scale: Option<Res<UiScale>>,
    roots: Query<&ComputedNode, With<PopupRoot>>,
    mut panels: Query<(&PopupPanel, &ChildOf, &ComputedNode, &mut Node)>,
) {
    let scale = scale.map_or(1.0, |scale| scale.0);
    for (panel, parent, computed, mut node) in &mut panels {
        let Ok(root) = roots.get(parent.parent()) else {
            continue;
        };
        let to_px = computed.inverse_scale_factor();
        let room = (root.size() - computed.size()) * to_px;
        let at = (panel.0 / scale).min(room).max(Vec2::ZERO);
        if node.left != px(at.x) {
            node.left = px(at.x);
        }
        if node.top != px(at.y) {
            node.top = px(at.y);
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::{prelude::*, ui_widgets::Activate};

    use super::{popup_row, spawn_popup, PopupRoot, PopupRow};
    use crate::widget::{
        fixtures::{press, spawn_window, themed_app, CapturedLog},
        text_field::{text_field, TextFieldFocused, TextFieldSpec, TextFieldSubmitted},
    };

    #[test]
    fn a_press_outside_a_popup_closes_it_and_its_focused_field_without_a_warning() {
        let log = CapturedLog::default();
        let _guard = log.capture();
        let mut app = themed_app(crate::theme::config::PHOSPHOR_THEME_ID);
        app.update();
        let window = spawn_window(&mut app);
        let world = app.world_mut();
        let mut queue = bevy::ecs::world::CommandQueue::default();
        let mut commands = Commands::new(&mut queue, world);
        let root = spawn_popup(&mut commands, Vec2::ZERO, 1, |rows| {
            rows.spawn((
                text_field(TextFieldSpec::new("")),
                TextFieldFocused::at_end(""),
            ));
        });
        queue.apply(world);

        press(&mut app, root, window);

        assert!(app.world().get_entity(root).is_err(), "the popup closed");
        let said = log.contents();
        assert!(!said.contains("despawned"), "{said}");
        assert_eq!(
            app.world().resource::<Messages<TextFieldSubmitted>>().len(),
            1,
            "the field left once"
        );
    }

    #[test]
    fn pressing_a_popup_row_closes_the_popup() {
        let mut app = themed_app(crate::theme::config::PHOSPHOR_THEME_ID);
        app.update();
        let world = app.world_mut();
        let mut queue = bevy::ecs::world::CommandQueue::default();
        let mut commands = Commands::new(&mut queue, world);
        spawn_popup(&mut commands, Vec2::ZERO, 1, |rows| {
            rows.spawn(popup_row("Frame", "F"));
        });
        queue.apply(world);
        let row = world
            .query_filtered::<Entity, With<PopupRow>>()
            .single(world)
            .expect("the popup holds its row");

        world.trigger(Activate { entity: row });
        world.flush();

        assert_eq!(
            world
                .query_filtered::<(), With<PopupRoot>>()
                .iter(world)
                .count(),
            0,
            "the row press closes the popup"
        );
    }
}
