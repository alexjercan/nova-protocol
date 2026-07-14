//! Shared button infrastructure for the editor UI: the themed `button` factory,
//! the selection/highlight machinery (`ButtonValue<T>` + `SelectedOption`), and
//! the observers that colour buttons on hover/press/select. Both the rail tools
//! and the component cards route through this so one click model drives them all.

use bevy::{
    picking::hover::Hovered,
    prelude::*,
    reflect::Is,
    ui::{InteractionDisabled, Pressed},
    ui_widgets::Button,
};

use crate::ui::theme;

/// Marks the currently-active button within a `ButtonValue<T>` selection group.
#[derive(Component)]
pub(crate) struct SelectedOption;

/// Marks an editor button so the colour observers pick it up.
#[derive(Component)]
pub(crate) struct EditorButton;

/// The value a settings button represents. Kept distinct from the `T` resource so a
/// button can carry a choice without being interpreted as - and clobbering - the resource
/// itself: on Bevy 0.19 a `#[derive(Resource)]` type is component-backed, so putting it on
/// a button entity is treated as a resource insert.
#[derive(Component, Debug, Clone)]
pub(crate) struct ButtonValue<T>(pub(crate) T);

/// Wire the button colour + selection observers. Called once from the plugin.
pub(crate) fn register(app: &mut App) {
    app.add_observer(button_on_interaction::<Add, Pressed>)
        .add_observer(button_on_interaction::<Remove, Pressed>)
        .add_observer(button_on_interaction::<Add, InteractionDisabled>)
        .add_observer(button_on_interaction::<Remove, InteractionDisabled>)
        .add_observer(button_on_interaction::<Insert, Hovered>);
    app.add_observer(on_add_selected)
        .add_observer(on_remove_selected);
}

fn button_on_interaction<E: EntityEvent, C: Component>(
    event: On<E, C>,
    mut q_button: Query<
        (
            &Hovered,
            Has<InteractionDisabled>,
            Has<Pressed>,
            Has<SelectedOption>,
            &mut BackgroundColor,
            &mut BorderColor,
            &Children,
        ),
        With<EditorButton>,
    >,
) {
    if let Ok((hovered, disabled, pressed, selected, mut color, mut border_color, children)) =
        q_button.get_mut(event.event_target())
    {
        if children.is_empty() {
            return;
        }
        if selected {
            *color = theme::SELECTED_FILL.into();
            border_color.set_all(theme::CYAN);
            return;
        }

        let hovered = hovered.get();
        let pressed = pressed && !(E::is::<Remove>() && C::is::<Pressed>());
        let disabled = disabled && !(E::is::<Remove>() && C::is::<InteractionDisabled>());
        match (disabled, hovered, pressed) {
            (true, _, _) => {
                *color = theme::PANEL.into();
                *border_color = theme::BORDER.into();
            }

            (false, true, true) => {
                *color = theme::SELECTED_FILL.into();
                border_color.set_all(theme::CYAN);
            }

            (false, true, false) => {
                *color = theme::PANEL_RAISED.into();
                border_color.set_all(theme::BORDER_BRIGHT);
            }

            (false, false, _) => {
                *color = theme::PANEL.into();
                *border_color = theme::BORDER.into();
            }
        }
    }
}

/// On a button press, copy the pressed button's `ButtonValue<T>` into the `T`
/// resource and move the `SelectedOption` marker to it.
pub(crate) fn button_on_setting<
    T: Resource + Component<Mutability = bevy::ecs::component::Mutable> + PartialEq + Clone,
>(
    event: On<Add, Pressed>,
    mut commands: Commands,
    // Each button carries its value as a `ButtonValue<T>` component (distinct from the T
    // resource, so a button never clobbers the resource), and clicking copies that value
    // into the `ResMut<T>` resource.
    selected: Option<Single<Entity, (With<ButtonValue<T>>, With<SelectedOption>)>>,
    q_t: Query<(Entity, &ButtonValue<T>), (Without<SelectedOption>, With<EditorButton>)>,
    mut setting: ResMut<T>,
) {
    let Ok((entity, value)) = q_t.get(event.event_target()) else {
        return;
    };

    if *setting != value.0 {
        if let Some(previous) = selected {
            commands
                .entity(previous.into_inner())
                .remove::<SelectedOption>();
        }
        commands.entity(entity).insert(SelectedOption);
        *setting = value.0.clone();
    }
}

fn on_add_selected(
    add: On<Add, SelectedOption>,
    mut q_color: Query<&mut BackgroundColor, (With<SelectedOption>, With<EditorButton>)>,
) {
    if let Ok(mut color) = q_color.get_mut(add.event_target()) {
        *color = theme::SELECTED_FILL.into();
    }
}

fn on_remove_selected(
    remove: On<Remove, SelectedOption>,
    mut q_color: Query<&mut BackgroundColor, With<EditorButton>>,
) {
    if let Ok(mut color) = q_color.get_mut(remove.event_target()) {
        *color = theme::PANEL.into();
    }
}

/// A themed rail button: full-width, 1px bordered, sharp corners.
pub(crate) fn button(text: &str) -> impl Bundle {
    (
        Node {
            width: percent(100),
            min_height: px(34),
            margin: UiRect::vertical(px(4)),
            padding: UiRect::axes(px(10), px(6)),
            border: UiRect::all(px(theme::BORDER_W)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border_radius: BorderRadius::all(px(theme::RADIUS)),
            ..default()
        },
        EditorButton,
        Button,
        Hovered::default(),
        BorderColor::all(theme::BORDER),
        BackgroundColor(theme::PANEL),
        children![(
            Text::new(text),
            TextFont {
                font_size: FontSize::Px(14.0),
                ..default()
            },
            TextColor(theme::TEXT),
            TextShadow::default(),
        )],
    )
}
