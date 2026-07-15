//! Shared themed widgets: the `themed_button` factory, the selection/highlight
//! machinery (`ButtonValue<T>` + `Selected`), the observers that colour buttons
//! on hover/press/select, and small layout helpers (`panel_header`, `separator`).
//! One click + colour model for every screen (menu, editor, HUD chrome).

use bevy::{
    picking::hover::Hovered,
    prelude::*,
    reflect::Is,
    ui::{InteractionDisabled, Pressed},
    ui_widgets::Button,
};

use crate::theme;

/// Marks the currently-active button within a `ButtonValue<T>` selection group.
#[derive(Component)]
pub struct Selected;

/// Marks a themed button so the colour observers pick it up.
#[derive(Component)]
pub struct ThemedButton;

/// The value a settings button represents. Kept distinct from the `T` resource so a
/// button can carry a choice without being interpreted as - and clobbering - the resource
/// itself: on Bevy 0.19 a `#[derive(Resource)]` type is component-backed, so putting it on
/// a button entity is treated as a resource insert.
#[derive(Component, Debug, Clone)]
pub struct ButtonValue<T>(pub T);

/// Guard resource: the themed-widget observers are app-global, so the first
/// [`register`] call wins and later calls are no-ops (menu and editor both
/// register in the shipped app; doubled observers would write every colour
/// twice per interaction).
#[derive(Resource)]
struct WidgetObserversRegistered;

/// Wire the button colour + selection observers. Call from each app/plugin
/// that uses themed buttons; guarded, so independent plugins can coexist.
pub fn register(app: &mut App) {
    if app.world().contains_resource::<WidgetObserversRegistered>() {
        return;
    }
    app.insert_resource(WidgetObserversRegistered);
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
            Has<Selected>,
            &mut BackgroundColor,
            &mut BorderColor,
            &Children,
        ),
        With<ThemedButton>,
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
/// resource and move the `Selected` marker to it.
pub fn button_on_setting<
    T: Resource + Component<Mutability = bevy::ecs::component::Mutable> + PartialEq + Clone,
>(
    event: On<Add, Pressed>,
    mut commands: Commands,
    // Each button carries its value as a `ButtonValue<T>` component (distinct from the T
    // resource, so a button never clobbers the resource), and clicking copies that value
    // into the `ResMut<T>` resource.
    selected: Option<Single<Entity, (With<ButtonValue<T>>, With<Selected>)>>,
    q_t: Query<(Entity, &ButtonValue<T>), (Without<Selected>, With<ThemedButton>)>,
    mut setting: ResMut<T>,
) {
    let Ok((entity, value)) = q_t.get(event.event_target()) else {
        return;
    };

    if *setting != value.0 {
        if let Some(previous) = selected {
            commands.entity(previous.into_inner()).remove::<Selected>();
        }
        commands.entity(entity).insert(Selected);
        *setting = value.0.clone();
    }
}

fn on_add_selected(
    add: On<Add, Selected>,
    mut q_color: Query<&mut BackgroundColor, (With<Selected>, With<ThemedButton>)>,
) {
    if let Ok(mut color) = q_color.get_mut(add.event_target()) {
        *color = theme::SELECTED_FILL.into();
    }
}

fn on_remove_selected(
    remove: On<Remove, Selected>,
    mut q_color: Query<&mut BackgroundColor, With<ThemedButton>>,
) {
    if let Ok(mut color) = q_color.get_mut(remove.event_target()) {
        *color = theme::PANEL.into();
    }
}

/// A themed button: full-width, 1px bordered, sharp corners, crisp hover.
pub fn themed_button(text: &str) -> impl Bundle {
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
        ThemedButton,
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

/// A small uppercase section header (e.g. "COMPONENTS").
pub fn panel_header(text: &str) -> impl Bundle {
    (
        Text::new(text.to_uppercase()),
        TextFont {
            font_size: FontSize::Px(13.0),
            ..default()
        },
        TextColor(theme::CYAN),
        Node {
            margin: UiRect::bottom(px(8)),
            ..default()
        },
    )
}

/// A thin horizontal separator.
pub fn separator() -> impl Bundle {
    (
        Node {
            width: percent(100),
            height: px(theme::BORDER_W),
            margin: UiRect::vertical(px(8)),
            ..default()
        },
        BackgroundColor(theme::BORDER),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // `Resource` is component-backed in Bevy 0.19, so it also provides the
    // `Component` impl `button_on_setting` needs - deriving `Component` too would
    // conflict. This mirrors the editor's `SectionChoice` (Resource-only).
    #[derive(Resource, Clone, PartialEq, Eq, Debug, Default)]
    enum Choice {
        #[default]
        None,
        A,
        B,
    }

    /// Pressing a `ThemedButton` carrying `ButtonValue<T>` copies that value into
    /// the `T` resource and marks it `Selected`, moving the marker off any prior
    /// selection. This is the exact path the editor's component cards (and the
    /// menu's tool buttons) rely on - inserting `Pressed` must set the resource.
    #[test]
    fn pressing_a_valued_button_sets_the_resource_and_selection() {
        let mut app = App::new();
        app.insert_resource(Choice::None);
        app.add_observer(button_on_setting::<Choice>);

        // Two buttons in the same group; give them a child so the (unrelated)
        // colour observer's `Children` guard is satisfied when it also fires.
        let a = app
            .world_mut()
            .spawn((ThemedButton, ButtonValue(Choice::A)))
            .id();
        let b = app
            .world_mut()
            .spawn((ThemedButton, ButtonValue(Choice::B)))
            .id();

        // Press A -> resource is A, A is Selected.
        app.world_mut().entity_mut(a).insert(Pressed);
        assert_eq!(*app.world().resource::<Choice>(), Choice::A);
        assert!(app.world().entity(a).contains::<Selected>());

        // Press B -> resource is B, selection moved from A to B.
        app.world_mut().entity_mut(b).insert(Pressed);
        assert_eq!(*app.world().resource::<Choice>(), Choice::B);
        assert!(app.world().entity(b).contains::<Selected>());
        assert!(
            !app.world().entity(a).contains::<Selected>(),
            "the previous selection is cleared"
        );
    }
}
