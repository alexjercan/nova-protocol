//! A select: a button that shows one choice and drops the list of all of them.
//!
//! The list is a [`popup`](super::spawn_popup). A pick writes [`Select::chosen`]
//! and the button label, and triggers [`SelectChanged`] on the select.

use bevy::{
    picking::hover::Hovered,
    prelude::*,
    ui::UiScale,
    ui_widgets::{Activate, Button},
};

use super::{popup_row, spawn_popup, ListRow, ThemedRadius, ThemedText, UiText};
use crate::theme::{UiColor, BORDER_W};

/// The state of a [`select`].
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct Select {
    /// The list, in the order it drops.
    pub options: Vec<String>,
    /// The index of the shown option.
    pub chosen: usize,
    /// The rung the list opens on.
    pub z: i32,
}

/// The text of a [`select`] button.
#[derive(Component)]
pub struct SelectLabel;

/// A row of an open list: the select it belongs to, and the option it picks.
#[derive(Component)]
pub struct SelectOption {
    select: Entity,
    index: usize,
}

/// A [`select`] took a new choice.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct SelectChanged {
    /// The select.
    pub entity: Entity,
    /// The index it now shows.
    pub chosen: usize,
}

/// A select showing `options[chosen]`, whose list opens on rung `z`.
pub fn select(options: Vec<String>, chosen: usize, z: i32) -> impl Bundle {
    let label = options.get(chosen).cloned().unwrap_or_default();
    let text = |line: String, colour: UiColor| {
        (
            UiText,
            Text::new(line),
            TextLayout {
                linebreak: LineBreak::NoWrap,
                ..default()
            },
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(Color::NONE),
            ThemedText::new(colour),
        )
    };
    (
        Select { options, chosen, z },
        ListRow,
        Button,
        Hovered::default(),
        Node {
            min_height: px(24),
            flex_direction: FlexDirection::Row,
            flex_shrink: 0.0,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            column_gap: px(10),
            padding: UiRect::axes(px(8), px(2)),
            border: UiRect::all(px(BORDER_W)),
            ..default()
        },
        ThemedRadius::control(),
        BorderColor::all(Color::NONE),
        BackgroundColor(Color::NONE),
        children![
            (SelectLabel, text(label, UiColor::Primary)),
            text("v".to_string(), UiColor::Label),
        ],
    )
}

/// Drop the list under the select that was pressed.
pub(super) fn open_select(
    activate: On<Activate>,
    mut commands: Commands,
    scale: Option<Res<UiScale>>,
    selects: Query<(&Select, &ComputedNode, &UiGlobalTransform)>,
) {
    let Ok((select, computed, at)) = selects.get(activate.entity) else {
        return;
    };
    let scale = scale.map_or(1.0, |scale| scale.0);
    let bottom_left = at.translation + computed.size() * Vec2::new(-0.5, 0.5);
    let owner = activate.entity;
    let options = select.options.clone();
    spawn_popup(
        &mut commands,
        bottom_left * computed.inverse_scale_factor() * scale,
        select.z,
        |rows| {
            for (index, option) in options.iter().enumerate() {
                rows.spawn((
                    Name::new(format!("Select Option {option}")),
                    popup_row(option, ""),
                    SelectOption {
                        select: owner,
                        index,
                    },
                ));
            }
        },
    );
}

/// Commit a pick from an open list.
pub(super) fn pick_select_option(
    activate: On<Activate>,
    mut commands: Commands,
    options: Query<&SelectOption>,
    mut selects: Query<(&mut Select, &Children)>,
    mut labels: Query<&mut Text, With<SelectLabel>>,
) {
    let Ok(option) = options.get(activate.entity) else {
        return;
    };
    let Ok((mut select, children)) = selects.get_mut(option.select) else {
        return;
    };
    select.chosen = option.index;
    for child in children {
        if let Ok(mut text) = labels.get_mut(*child) {
            text.0.clone_from(&select.options[option.index]);
        }
    }
    commands.trigger(SelectChanged {
        entity: option.select,
        chosen: option.index,
    });
}

#[cfg(test)]
mod tests {
    use bevy::{prelude::*, ui_widgets::Activate};

    use super::{select, Select, SelectChanged, SelectLabel, SelectOption};
    use crate::widget::fixtures::themed_app;

    #[derive(Resource, Default)]
    struct Picked(Option<usize>);

    #[test]
    fn picking_an_option_writes_the_label_and_says_so() {
        let mut app = themed_app(crate::theme::config::PHOSPHOR_THEME_ID);
        app.init_resource::<Picked>();
        app.add_observer(|change: On<SelectChanged>, mut picked: ResMut<Picked>| {
            picked.0 = Some(change.chosen);
        });
        let owner = app
            .world_mut()
            .spawn(select(vec!["All".into(), "Hulls".into()], 0, 1))
            .id();
        app.update();
        let row = app
            .world_mut()
            .spawn(SelectOption {
                select: owner,
                index: 1,
            })
            .id();

        app.world_mut().trigger(Activate { entity: row });
        app.world_mut().flush();

        assert_eq!(app.world().resource::<Picked>().0, Some(1));
        assert_eq!(app.world().get::<Select>(owner).unwrap().chosen, 1);
        let world = app.world_mut();
        let label = world
            .query_filtered::<&Text, With<SelectLabel>>()
            .single(world)
            .expect("the select draws one label");
        assert_eq!(label.0, "Hulls");
    }
}
