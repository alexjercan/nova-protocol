//! The always-on status bar in the top-right corner: FPS, build version, and
//! whatever else a metric is worth a corner of the screen.
//!
//! Nova owns it because it is UI chrome, and `nova_ui` is where nova's UI
//! chrome lives - the composition root (`nova_core`) spawns the root and its
//! items, and nothing about the widget is game-specific.
//!
//! An item is a [`StatusBarItemConfig`] pair of closures: `value_fn` reads the
//! world, `color_fn` tints the produced value. Keep the `value_fn` closures
//! cheap: they take `&World`, so they run in an exclusive system that blocks
//! all parallel system execution, once per frame. (`color_fn` takes the
//! produced value, not the world, and runs in an ordinary parallel system.)

use std::{any::Any, fmt::Display, sync::Arc};

use bevy::{platform::collections::HashMap, prelude::*};

/// Glob-import surface for the status bar: the spawn helpers, the item config
/// and the ready-made FPS/version closures.
pub mod prelude {
    pub use super::{
        status_bar, status_bar_item, status_bar_with_fps, status_fps_color_fn, status_fps_value_fn,
        status_version_color_fn, status_version_value_fn, StatusBarItemConfig, StatusBarItemMarker,
        StatusBarPlugin, StatusBarPluginSystems, StatusBarRootConfig, StatusBarRootMarker,
        StatusValue,
    };
}

/// The StatusBarRootMarker component is a marker component that indicates the root node of the status
/// bar UI.
#[derive(Component, Clone, Debug, Reflect)]
pub struct StatusBarRootMarker;

/// Spawn-time configuration for the status bar root. Empty today; it exists so
/// [`status_bar`] has a place to grow options without changing its signature.
#[derive(Clone, Debug, Default)]
pub struct StatusBarRootConfig {}

/// --- Status bar in top-right for FPS, latency, etc ---
pub fn status_bar(_config: StatusBarRootConfig) -> impl Bundle {
    (
        Name::new("StatusBarUIRoot"),
        StatusBarRootMarker,
        Node {
            width: Val::Auto,
            height: Val::Auto,
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            right: Val::Px(10.0),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::FlexEnd,
            ..default()
        },
    )
}

/// Marker component on a status bar item, spawned by [`status_bar_item`]. The
/// `Add` observer reads it to build the item's child nodes under the root.
#[derive(Component, Clone, Debug, Reflect)]
pub struct StatusBarItemMarker;

/// Anything a status bar item can display: printable, thread-safe, and
/// downcastable so a `color_fn` can recover the concrete type.
pub trait StatusValue: Any + Display + Send + Sync + 'static {}
impl<T> StatusValue for T where T: Any + Display + Send + Sync + 'static {}

/// The StatusBarItemConfig component defines a single item in the status bar.
#[derive(Debug, Clone, Default)]
pub struct StatusBarItemConfig<F, G>
where
    F: Fn(&World) -> Option<Arc<dyn StatusValue>> + Send + Sync + 'static,
    G: Fn(Box<&dyn Any>) -> Option<Color> + Send + Sync + 'static,
{
    /// Optional icon drawn left of the value.
    pub icon: Option<Handle<Image>>,
    /// Reads the current value out of the world, once per frame.
    pub value_fn: F,
    /// Tints the produced value; `None` leaves the previous color.
    pub color_fn: G,
    /// Static text drawn before the value.
    pub prefix: String,
    /// Static text drawn after the value (a unit, usually).
    pub suffix: String,
}

/// The item's icon handle, split out of [`StatusBarItemConfig`] onto the entity.
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub struct StatusBarItemIcon(pub Handle<Image>);

/// The item's prefix text.
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub struct StatusBarItemPrefix(pub String);

/// The item's suffix text.
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub struct StatusBarItemSuffix(pub String);

/// The item's `value_fn`, type-erased so it can live on a component.
#[derive(Component, Clone, Deref, DerefMut)]
pub struct StatusBarItemValueFnBoxed(
    pub Arc<dyn Fn(&World) -> Option<Arc<dyn StatusValue>> + Send + Sync>,
);

/// The item's `color_fn`, type-erased so it can live on a component.
#[derive(Component, Clone, Deref, DerefMut)]
pub struct StatusBarItemColorFnBoxed(pub Arc<dyn Fn(Box<&dyn Any>) -> Option<Color> + Send + Sync>);

/// Spawn one status bar item. It attaches itself to the [`status_bar`] root
/// through the `Add` observer, so spawn it as a plain top-level bundle.
pub fn status_bar_item<F, G>(config: StatusBarItemConfig<F, G>) -> impl Bundle
where
    F: Fn(&World) -> Option<Arc<dyn StatusValue>> + Send + Sync + 'static,
    G: Fn(Box<&dyn Any>) -> Option<Color> + Send + Sync + 'static,
{
    (
        Name::new("StatusBarItem"),
        StatusBarItemMarker,
        StatusBarItemIcon(config.icon.unwrap_or_default()),
        StatusBarItemPrefix(config.prefix),
        StatusBarItemSuffix(config.suffix),
        StatusBarItemValueFnBoxed(Arc::new(config.value_fn)),
        StatusBarItemColorFnBoxed(Arc::new(config.color_fn)),
    )
}

/// The value staged for this item by the last exclusive read; `None` renders
/// as `N/A`.
#[derive(Component, Clone, Deref, DerefMut)]
pub struct StatusBarItemValue(pub Option<Arc<dyn StatusValue>>);

/// Per-entity staging store for item values.
#[derive(Resource, Default, Clone)]
pub struct StatusBarStore {
    /// The last value read for each item entity.
    pub store: HashMap<Entity, Arc<dyn StatusValue>>,
}

/// System sets for the status bar plugin.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum StatusBarPluginSystems {
    /// Reads every item's `value_fn` and pushes the result into the item's text.
    Sync,
}

/// Drives the status bar: builds items on spawn and refreshes their values and
/// colors every frame.
pub struct StatusBarPlugin;

impl Plugin for StatusBarPlugin {
    fn build(&self, app: &mut App) {
        debug!("StatusBarPlugin: build");

        app.init_resource::<StatusBarStore>();

        app.add_observer(insert_status_bar_item);

        app.add_systems(
            Update,
            (update_status_bar_item_values, update_status_bar_item_ui)
                .chain()
                .in_set(StatusBarPluginSystems::Sync),
        );
    }
}

/// Run every item's `value_fn` against the world and stage the results.
///
/// Exclusive (`&mut World`), because the closures take `&World`: it blocks all
/// parallel system execution while it runs, every frame. Keep the closures cheap.
fn update_status_bar_item_values(world: &mut World) {
    let mut query =
        world.query_filtered::<(Entity, &StatusBarItemValueFnBoxed), With<StatusBarItemValue>>();
    let values: HashMap<_, _> = query
        .iter(world)
        .map(|(entity, value_fn)| (entity, value_fn(world)))
        .collect();

    let mut query =
        world.query_filtered::<(Entity, &mut StatusBarItemValue), With<StatusBarItemValue>>();
    for (entity, mut item) in query.iter_mut(world) {
        if let Some(value) = values.get(&entity) {
            **item = value.clone();
        }
    }
}

fn update_status_bar_item_ui(
    mut items: Query<(
        &StatusBarItemValue,
        &mut Text,
        &mut TextColor,
        &StatusBarItemColorFnBoxed,
    )>,
) {
    // Runs every frame for every item, and an item usually reports the same value
    // for seconds at a time. Both writes are guarded so an unchanged item does not
    // mark its `Text` changed and force a text re-layout plus a UI batch upload.
    for (value, mut text, mut color, color_fn) in &mut items {
        let next = value
            .as_ref()
            .map_or_else(|| "N/A".to_string(), |v| v.to_string());
        if **text != next {
            **text = next;
        }

        if let Some(v) = value.as_ref() {
            let v: &dyn Any = v.as_ref();

            if let Some(new_color) = (color_fn)(Box::new(v)) {
                color.set_if_neq(TextColor(new_color));
            }
        }
    }
}

fn insert_status_bar_item(
    add: On<Add, StatusBarItemMarker>,
    mut commands: Commands,
    q_item: Query<
        (
            &StatusBarItemIcon,
            &StatusBarItemPrefix,
            &StatusBarItemSuffix,
            &StatusBarItemValueFnBoxed,
            &StatusBarItemColorFnBoxed,
        ),
        With<StatusBarItemMarker>,
    >,
    root: Single<Entity, With<StatusBarRootMarker>>,
) {
    let entity = add.entity;
    trace!("insert_status_bar_item: entity {:?}", entity);

    let Ok((icon, prefix, suffix, value_fn, color_fn)) = q_item.get(entity) else {
        error!(
            "insert_status_bar_item: entity {:?} not found in q_item",
            entity
        );
        return;
    };

    let root = root.into_inner();

    commands.entity(root).with_children(|parent| {
        parent.spawn((
            Name::new(format!("StatusBarItem: {}-{}", **prefix, **suffix)),
            Node {
                width: Val::Auto,
                height: Val::Px(24.0),
                margin: UiRect::all(Val::Px(4.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(4.0),
                ..default()
            },
            children![
                (
                    Name::new("StatusBarItemIcon"),
                    ImageNode {
                        image: (**icon).clone(),
                        ..default()
                    },
                    Node {
                        width: Val::Px(16.0),
                        height: Val::Px(16.0),
                        ..default()
                    },
                ),
                (
                    Name::new("StatusBarItemPrefix"),
                    Text::new((**prefix).clone()),
                    TextFont {
                        font_size: FontSize::Px(14.0),
                        ..default()
                    },
                ),
                (
                    Name::new("StatusBarItemValue"),
                    StatusBarItemValue(None),
                    value_fn.clone(),
                    Text::new("N/A".to_string()),
                    TextFont {
                        font_size: FontSize::Px(14.0),
                        ..default()
                    },
                    color_fn.clone(),
                    TextColor(Color::WHITE),
                ),
                (
                    Name::new("StatusBarItemSuffix"),
                    Text::new((**suffix).clone()),
                    TextFont {
                        font_size: FontSize::Px(14.0),
                        ..default()
                    },
                )
            ],
        ));
    });
}

/// A ready-made "NN fps" status bar item, wiring [`status_fps_value_fn`] and
/// [`status_fps_color_fn`] with a `fps` suffix.
///
/// The FPS item is copied verbatim into every game; this collapses the
/// eight-line [`status_bar_item`] spawn to one call. Spawn it as a child (or
/// sibling) of a [`status_bar`] root:
///
/// ```rust
/// # use bevy::prelude::*;
/// # use nova_ui::status_bar::*;
/// fn setup(mut commands: Commands) {
///     commands.spawn(status_bar(StatusBarRootConfig::default()));
///     commands.spawn(status_bar_with_fps());
/// }
/// ```
///
/// The reading needs Bevy's `FrameTimeDiagnosticsPlugin` added, same as the
/// hand-rolled item.
pub fn status_bar_with_fps() -> impl Bundle {
    status_bar_item(StatusBarItemConfig {
        icon: None,
        value_fn: status_fps_value_fn(),
        color_fn: status_fps_color_fn(),
        prefix: "".to_string(),
        suffix: "fps".to_string(),
    })
}

/// Reads the averaged FPS out of Bevy's `DiagnosticsStore`, rounded to a whole
/// frame. Needs `FrameTimeDiagnosticsPlugin` added.
pub fn status_fps_value_fn(
) -> impl Fn(&World) -> Option<Arc<dyn StatusValue>> + Send + Sync + 'static {
    move |world: &World| {
        let store = world.resource::<bevy::diagnostic::DiagnosticsStore>();
        store
            .get(&bevy::diagnostic::FrameTimeDiagnosticsPlugin::FPS)
            .and_then(|d| d.average())
            .map(|v| v.round() as u32)
            .map(|fps| Arc::new(fps) as Arc<dyn StatusValue>)
    }
}

/// Traffic-lights the FPS reading: red under 30, yellow under 60, green above.
pub fn status_fps_color_fn() -> impl Fn(Box<&dyn Any>) -> Option<Color> + Send + Sync + 'static {
    move |value: Box<&dyn Any>| {
        let fps = (*value).downcast_ref::<u32>()?;
        let color = if *fps < 30 {
            Color::srgb(1.0, 0.0, 0.0)
        } else if *fps < 60 {
            Color::srgb(1.0, 1.0, 0.0)
        } else {
            Color::srgb(0.0, 1.0, 0.0)
        };
        Some(color)
    }
}

/// Displays a fixed build version; it never reads the world.
pub fn status_version_value_fn(
    version: impl Display + Clone + Send + Sync + 'static,
) -> impl Fn(&World) -> Option<Arc<dyn StatusValue>> + Send + Sync + 'static {
    move |_world: &World| Some(Arc::new(version.clone()) as Arc<dyn StatusValue>)
}

/// Paints the version item plain white.
pub fn status_version_color_fn() -> impl Fn(Box<&dyn Any>) -> Option<Color> + Send + Sync + 'static
{
    move |_value: Box<&dyn Any>| Some(Color::srgb(1.0, 1.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Counts the frames on which the reconciler marked an item's text changed.
    #[derive(Resource, Default)]
    struct Rewrites {
        text: usize,
        color: usize,
    }

    fn count_rewrites(
        mut rewrites: ResMut<Rewrites>,
        q_text: Query<(), Changed<Text>>,
        q_color: Query<(), Changed<TextColor>>,
    ) {
        rewrites.text += q_text.iter().count();
        rewrites.color += q_color.iter().count();
    }

    /// The reconciler runs every frame for every item, so the assertion that
    /// matters is a change-detection one: with the value unchanged, the second
    /// frame must not mark `Text` or `TextColor` changed. Asserting on the
    /// rendered string alone passes with unconditional writes.
    #[test]
    fn an_unchanged_status_item_is_not_rewritten() {
        let mut app = App::new();
        app.init_resource::<Rewrites>();
        app.add_systems(Update, (update_status_bar_item_ui, count_rewrites).chain());

        let item = app
            .world_mut()
            .spawn((
                StatusBarItemValue(Some(Arc::new("42".to_string()))),
                Text::new(String::new()),
                TextColor(Color::BLACK),
                StatusBarItemColorFnBoxed(Arc::new(|_| Some(Color::WHITE))),
            ))
            .id();

        // Frame 1 writes the new value and colour in.
        app.update();
        assert_eq!(app.world().get::<Text>(item).unwrap().0, "42");
        let after_first = (
            app.world().resource::<Rewrites>().text,
            app.world().resource::<Rewrites>().color,
        );
        assert_eq!(after_first, (1, 1), "the first frame writes both");

        // Frame 2 sees the same value and must touch neither.
        app.update();
        assert_eq!(
            (
                app.world().resource::<Rewrites>().text,
                app.world().resource::<Rewrites>().color,
            ),
            after_first,
            "an unchanged value must not mark Text or TextColor changed",
        );

        // A real change still lands.
        **app.world_mut().get_mut::<StatusBarItemValue>(item).unwrap() =
            Some(Arc::new("43".to_string()));
        app.update();
        assert_eq!(app.world().get::<Text>(item).unwrap().0, "43");
        assert_eq!(app.world().resource::<Rewrites>().text, after_first.0 + 1);
    }
}
