//! The themed button: paint application, interaction observers, live-theme
//! reconciler and the [`ButtonSpec`] build recipe.
//!
//! The visual is a LOOKUP - `theme.button(variant, state)` - not a function
//! written here: the paint of all four variants in all six states is authored
//! data (`crates/nova_ui/src/theme/base.rs`, serialized into the base mod).
//! This module decides which state a button is IN and puts the resolved paint
//! on it, from both the per-interaction observers (hover/press/disable/select)
//! and [`reconcile_button_themes`], the system that restyles LIVE buttons when
//! the theme changes or a new button is spawned.

use bevy::{
    ecs::relationship::RelatedSpawner,
    picking::hover::Hovered,
    platform::collections::HashSet,
    prelude::*,
    reflect::Is,
    ui::{InteractionDisabled, Pressed},
    ui_widgets::{Activate, Button},
};

use super::{Selected, UiText};
use crate::theme::{
    ActiveUiTheme, ButtonState, ResolvedState, ThemeButton, UiColor, UiMetric, BORDER_W,
};

/// Marks a themed button so the colour observers + theme reconciler pick it up.
#[derive(Component)]
pub struct ThemedButton;

/// The emphasis a themed button carries. Absent = [`ButtonVariant::Default`].
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    /// Neutral button.
    #[default]
    Default,
    /// Primary call-to-action: the theme's lit face, in every live state.
    Primary,
    /// Destructive action: red family.
    Danger,
    /// Border-only, transparent fill.
    Ghost,
}

/// Marks the primary label text span inside a themed button, so the paint code
/// can recolour it (e.g. inverted glyphs on selection) without touching the
/// key-chip or block-cursor spans.
#[derive(Component)]
pub struct ButtonLabel;

/// Marks the `> ` block-cursor span (block buttons only): shown (opaque) on
/// hover/selection, hidden (transparent) otherwise.
#[derive(Component)]
pub struct ButtonCursor;
/// The value a settings button represents. Kept distinct from the `T` resource so a
/// button can carry a choice without being interpreted as - and clobbering - the resource
/// itself: on Bevy 0.19 a `#[derive(Resource)]` type is component-backed, so putting it on
/// a button entity is treated as a resource insert.
#[derive(Component, Debug, Clone)]
pub struct ButtonValue<T>(pub T);
type LabelText<'w, 's> =
    Query<'w, 's, &'static mut TextColor, (With<ButtonLabel>, Without<ButtonCursor>)>;
type CursorText<'w, 's> =
    Query<'w, 's, &'static mut TextColor, (With<ButtonCursor>, Without<ButtonLabel>)>;

/// Which theme table a [`ButtonVariant`] paints from.
fn table(variant: ButtonVariant) -> ThemeButton {
    match variant {
        ButtonVariant::Default => ThemeButton::Default,
        ButtonVariant::Primary => ThemeButton::Primary,
        ButtonVariant::Danger => ThemeButton::Danger,
        ButtonVariant::Ghost => ThemeButton::Ghost,
    }
}

/// Apply a resolved state to one button: its own fill/border/radius +
/// gradient/shadow (inserted or removed to switch themes) + its label/cursor
/// spans' colours.
#[expect(
    clippy::too_many_arguments,
    reason = "one reconciler over every part of a button"
)]
fn apply_paint(
    paint: &ResolvedState,
    cursor_visible: bool,
    theme: &ActiveUiTheme,
    commands: &mut Commands,
    entity: Entity,
    bg: &mut BackgroundColor,
    border: &mut BorderColor,
    node: &mut Node,
    children: &Children,
    q_label: &mut LabelText,
    q_cursor: &mut CursorText,
) {
    *bg = paint.fill.base.into();
    border.set_all(paint.border);
    node.border_radius = BorderRadius::all(px(theme.metric(UiMetric::Radius)));

    // `try_insert` / `try_remove`, not `insert` / `remove`: a button can be despawned
    // the SAME frame the reconciler paints it (a menu/state teardown despawns
    // its buttons while this deferred command is still queued). The plain forms
    // error via the fallback handler ("Entity despawned") - which the smoke
    // examples promote to a panic; the try_ forms silently no-op on a dead
    // entity. (Repo idiom, e.g. nova_gameplay integrity/glue.rs.)
    let mut ent = commands.entity(entity);
    match &paint.fill.gradient {
        Some(g) => {
            ent.try_insert(g.clone());
        }
        None => {
            ent.try_remove::<BackgroundGradient>();
        }
    }
    match &paint.shadow {
        Some(s) => {
            ent.try_insert(s.clone());
        }
        None => {
            ent.try_remove::<BoxShadow>();
        }
    }

    for &child in children {
        if let Ok(mut tc) = q_label.get_mut(child) {
            *tc = TextColor(paint.text);
        }
        if let Ok(mut tc) = q_cursor.get_mut(child) {
            // The cursor keeps the theme's primary ink; only its alpha toggles.
            *tc = TextColor(
                theme.color_alpha(UiColor::Primary, if cursor_visible { 1.0 } else { 0.0 }),
            );
        }
    }
}

/// The button colour observer: on any hover/press/disable/select change,
/// look up + apply the button's paint in the active theme. Generic over the
/// event `E` and component `C` so one body handles Add/Remove/Insert of
/// `Pressed`, `InteractionDisabled`, `Hovered` and `Selected`; the removed
/// component still reads present inside its own `Remove` observer, so its state
/// is forced false there.
pub(super) fn button_on_interaction<E: EntityEvent, C: Component>(
    event: On<E, C>,
    theme: Res<ActiveUiTheme>,
    mut commands: Commands,
    mut q_button: Query<
        (
            Option<&ButtonVariant>,
            &Hovered,
            Has<InteractionDisabled>,
            Has<Pressed>,
            Has<Selected>,
            &mut BackgroundColor,
            &mut BorderColor,
            &mut Node,
            &Children,
        ),
        With<ThemedButton>,
    >,
    mut q_label: LabelText,
    mut q_cursor: CursorText,
) {
    let entity = event.event_target();
    let Ok((variant, hovered, disabled, pressed, selected, mut bg, mut border, mut node, children)) =
        q_button.get_mut(entity)
    else {
        return;
    };
    if children.is_empty() {
        return;
    }

    let removing = E::is::<Remove>();
    let pressed = pressed && !(removing && C::is::<Pressed>());
    let disabled = disabled && !(removing && C::is::<InteractionDisabled>());
    let selected = selected && !(removing && C::is::<Selected>());
    let variant = variant.copied().unwrap_or_default();

    let state = ButtonState::resolve(disabled, pressed, selected, hovered.get());
    apply_paint(
        theme.button(table(variant), state),
        hovered.get() || selected,
        &theme,
        &mut commands,
        entity,
        &mut bg,
        &mut border,
        &mut node,
        children,
        &mut q_label,
        &mut q_cursor,
    );
}

/// Restyle LIVE themed buttons: on a theme change repaint every button;
/// otherwise paint only the just-spawned ones (`Added<ThemedButton>`, an
/// override that must defer past the deferred-spawn flush - hence a SYSTEM, not
/// an `Add` observer - per lesson mode-keyed-reconciler-just-spawned-override).
///
/// This is the ONLY thing that paints a button. [`button`] spawns an unpainted
/// shell, so there is one path from state to pixels instead of a factory that
/// guesses an idle face and a reconciler that corrects it - which is what used
/// to flash a phosphor button on the hardware look for a frame.
#[expect(
    clippy::type_complexity,
    reason = "one query term per button visual state"
)]
pub(super) fn reconcile_button_themes(
    theme: Res<ActiveUiTheme>,
    mut commands: Commands,
    mut q_button: Query<
        (
            Entity,
            Option<&ButtonVariant>,
            &Hovered,
            Has<InteractionDisabled>,
            Has<Pressed>,
            Has<Selected>,
            &mut BackgroundColor,
            &mut BorderColor,
            &mut Node,
            &Children,
        ),
        With<ThemedButton>,
    >,
    added: Query<Entity, Added<ThemedButton>>,
    mut q_label: LabelText,
    mut q_cursor: CursorText,
) {
    let restyle_all = theme.is_changed();
    let just_added: HashSet<Entity> = added.iter().collect();
    if !restyle_all && just_added.is_empty() {
        return;
    }

    for (
        entity,
        variant,
        hovered,
        disabled,
        pressed,
        selected,
        mut bg,
        mut border,
        mut node,
        children,
    ) in &mut q_button
    {
        if !restyle_all && !just_added.contains(&entity) {
            continue;
        }
        if children.is_empty() {
            continue;
        }
        let variant = variant.copied().unwrap_or_default();
        let state = ButtonState::resolve(disabled, pressed, selected, hovered.get());
        apply_paint(
            theme.button(table(variant), state),
            hovered.get() || selected,
            &theme,
            &mut commands,
            entity,
            &mut bg,
            &mut border,
            &mut node,
            children,
            &mut q_label,
            &mut q_cursor,
        );
    }
}

/// On a button activation, copy the activated button's `ButtonValue<T>` into
/// the `T` resource and move the `Selected` marker to it.
///
/// `Activate` (release over the button), not `Add, Pressed` (mouse-down), so a
/// valued button cancels like every other button in the UI: press, drag off,
/// release commits nothing.
pub fn button_on_setting<
    T: Resource + Component<Mutability = bevy::ecs::component::Mutable> + PartialEq + Clone,
>(
    event: On<Activate>,
    mut commands: Commands,
    // Each button carries its value as a `ButtonValue<T>` component (distinct from the T
    // resource, so a button never clobbers the resource), and clicking copies that value
    // into the `ResMut<T>` resource.
    selected: Option<Single<Entity, (With<ButtonValue<T>>, With<Selected>)>>,
    q_t: Query<(Entity, &ButtonValue<T>), (Without<Selected>, With<ThemedButton>)>,
    mut setting: ResMut<T>,
) {
    let Ok((entity, value)) = q_t.get(event.entity) else {
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

/// The build recipe for a [`ThemedButton`]. Construct with [`ButtonSpec::new`]
/// and the chainable modifiers.
#[derive(Clone)]
pub struct ButtonSpec {
    /// The button label.
    pub text: String,
    /// Emphasis.
    pub variant: ButtonVariant,
    /// Left-aligned full-width with a `> ` hover/selected cursor (menu/list style).
    pub block: bool,
    /// Optional trailing amber key-chip (`Enter`/`Esc`).
    pub key: Option<String>,
    /// Size to the label instead of filling the row.
    pub fit: bool,
    /// Minimum height in px.
    pub min_height: f32,
    /// Label font size in px.
    pub font_size: f32,
}

impl ButtonSpec {
    /// A default 34px / 14px neutral button.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            variant: ButtonVariant::Default,
            block: false,
            key: None,
            fit: false,
            min_height: 34.0,
            font_size: 14.0,
        }
    }

    /// Size to the label rather than filling the row.
    ///
    /// A full-width button in a WRAPPING row takes a line to itself, which
    /// turns a bar of segments into a stack of them.
    pub fn fit(mut self) -> Self {
        self.fit = true;
        self
    }

    /// The larger main-menu sizing (40px / 16px).
    pub fn menu(mut self) -> Self {
        self.min_height = 40.0;
        self.font_size = 16.0;
        self
    }

    /// Primary call-to-action.
    pub fn primary(mut self) -> Self {
        self.variant = ButtonVariant::Primary;
        self
    }

    /// Destructive action.
    pub fn danger(mut self) -> Self {
        self.variant = ButtonVariant::Danger;
        self
    }

    /// Border-only ghost.
    pub fn ghost(mut self) -> Self {
        self.variant = ButtonVariant::Ghost;
        self
    }

    /// Left-aligned block button with a `> ` cursor.
    pub fn block(mut self) -> Self {
        self.block = true;
        self
    }

    /// Trailing amber key-chip.
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }
}

/// Build the button bundle from a [`ButtonSpec`]. Spawns the label span, plus an
/// optional block cursor and key-chip, as children, UNPAINTED:
/// [`reconcile_button_themes`] paints it on the frame it appears, which is the
/// single path from state to pixels.
///
/// An EMPTY `text` spawns no label span at all, which is how a caller that
/// spawns its own content (an icon) gets a button with nothing else in it.
pub fn button(spec: ButtonSpec) -> impl Bundle {
    let ButtonSpec {
        text,
        variant,
        block,
        key,
        fit,
        min_height,
        font_size,
    } = spec;

    let justify = if block {
        JustifyContent::FlexStart
    } else {
        JustifyContent::Center
    };

    (
        Node {
            width: if fit { Val::Auto } else { percent(100) },
            min_height: px(min_height),
            margin: UiRect::vertical(px(4)),
            padding: UiRect::axes(px(12), px(6)),
            // The border WIDTH is layout, so it is set here; its colour, the
            // fill and the radius are the theme's and land on the reconciler's
            // first pass.
            border: UiRect::all(px(BORDER_W)),
            justify_content: justify,
            align_items: AlignItems::Center,
            column_gap: px(8),
            ..default()
        },
        ThemedButton,
        variant,
        Button,
        Hovered::default(),
        BorderColor::all(Color::NONE),
        BackgroundColor(Color::NONE),
        Children::spawn(SpawnWith(move |parent: &mut RelatedSpawner<ChildOf>| {
            if block {
                parent.spawn((
                    ButtonCursor,
                    UiText,
                    Text::new("> "),
                    TextFont {
                        font_size: FontSize::Px(font_size),
                        ..default()
                    },
                    TextColor(Color::NONE),
                ));
            }
            // An EMPTY label spawns nothing, not an empty span: the button
            // lays its children out with a `column_gap`, so a zero-width span
            // would still push whatever the caller spawns inside off centre.
            // That is how an icon button (the settings keycap chips) says it
            // brings its own content.
            if !text.is_empty() {
                parent.spawn((
                    ButtonLabel,
                    UiText,
                    Text::new(text),
                    TextFont {
                        font_size: FontSize::Px(font_size),
                        ..default()
                    },
                    TextColor(Color::NONE),
                    // No TextShadow. Bevy's `TextShadow` is a hard drop
                    // shadow (no blur), so its default 4px black offset ghosts
                    // the label on a bright/inverted fill instead of glowing.
                    // Crisp CLI text needs no shadow.
                ));
            }
            if let Some(key) = key {
                parent.spawn(key_chip(&key, font_size));
            }
        })),
    )
}

/// The trailing amber key-chip span (`Enter`/`Esc`), bordered per the demo.
///
/// Public because a chip is how this UI draws A KEY, and buttons are not the
/// only surface that names one: the editor's menu rows and its key legend say
/// the same thing about the same keyboard, and a second drawing of a chip
/// would be a second answer to what a key looks like.
pub fn key_chip(text: &str, font_size: f32) -> impl Bundle {
    (
        KeyChip,
        Node {
            margin: UiRect::left(px(8)),
            padding: UiRect::axes(px(5), px(1)),
            border: UiRect::all(px(BORDER_W)),
            ..default()
        },
        BorderColor::all(Color::NONE),
        BackgroundColor(Color::NONE),
        children![(
            KeyChipLabel,
            UiText,
            Text::new(text.to_string()),
            TextFont {
                font_size: FontSize::Px((font_size - 2.0).max(10.0)),
                ..default()
            },
            TextColor(Color::NONE),
        )],
    )
}

/// Marks a [`key_chip`] so the theme reconciler paints it and repaints it live.
#[derive(Component)]
pub struct KeyChip;

/// Marks a [`key_chip`]'s legend span, so the reconciler recolours the legend
/// without reaching into whatever else a caller spawned beside it.
#[derive(Component)]
pub struct KeyChipLabel;

/// Paint keycap chips from the theme's `key_chip` role, on a theme change and
/// on spawn - the same two triggers every other family reconciles on.
pub(super) fn reconcile_key_chips(
    theme: Res<ActiveUiTheme>,
    mut q: Query<
        (
            Entity,
            &mut BackgroundColor,
            &mut BorderColor,
            &mut Node,
            &Children,
        ),
        With<KeyChip>,
    >,
    added: Query<Entity, Added<KeyChip>>,
    mut q_label: Query<&mut TextColor, With<KeyChipLabel>>,
) {
    let restyle_all = theme.is_changed();
    let just_added: HashSet<Entity> = added.iter().collect();
    if !restyle_all && just_added.is_empty() {
        return;
    }
    let paint = theme.key_chip();
    for (entity, mut bg, mut border, mut node, children) in &mut q {
        if !restyle_all && !just_added.contains(&entity) {
            continue;
        }
        *bg = paint.fill.base.into();
        border.set_all(paint.border);
        node.border_radius = BorderRadius::all(px(theme.metric(UiMetric::Radius)));
        for &child in children {
            if let Ok(mut tc) = q_label.get_mut(child) {
                *tc = TextColor(paint.text);
            }
        }
    }
}

/// A default themed button (34px / 14px, neutral). The one-arg convenience the
/// editor + menu spawn.
pub fn themed_button(text: &str) -> impl Bundle {
    button(ButtonSpec::new(text))
}

/// The larger main-menu button (40px / 16px), routed through the same
/// [`ThemedButton`] observers so every button in the game shares one path.
pub fn menu_button(text: &str) -> impl Bundle {
    button(ButtonSpec::new(text).menu())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        theme::{HARDWARE_THEME_ID, PHOSPHOR_THEME_ID},
        widget::fixtures::{bg, hardware, has_gradient, phosphor, select, themed_app},
    };

    fn label_color(app: &mut App, entity: Entity) -> Color {
        let children: Vec<Entity> = app
            .world()
            .entity(entity)
            .get::<Children>()
            .unwrap()
            .iter()
            .collect();
        let mut q = app
            .world_mut()
            .query_filtered::<&TextColor, With<ButtonLabel>>();
        for child in children {
            if let Ok(tc) = q.get(app.world(), child) {
                return tc.0;
            }
        }
        panic!("a ButtonLabel span exists");
    }

    /// `base/phosphor` renders a button as a CLI element: a flat
    /// phosphor-tinted fill, a 1px phosphor border and NO bevel gradient or
    /// shadow - and selection INVERTS (solid phosphor fill, dark ink glyphs).
    /// Pins the authored state table so an edit that reintroduces a gradient on
    /// the terminal look (a bevelled button on glass) fails here.
    #[test]
    fn phosphor_button_states_render_cli_markers() {
        let mut app = themed_app(PHOSPHOR_THEME_ID);
        let btn = app.world_mut().spawn(button(ButtonSpec::new("Go"))).id();
        app.update();

        let theme = phosphor();
        assert_eq!(bg(&app, btn), theme.color_alpha(UiColor::Primary, 0.05));
        assert_eq!(
            app.world().entity(btn).get::<BorderColor>().unwrap().top,
            theme.color_alpha(UiColor::Primary, 0.4)
        );
        assert!(
            !has_gradient(&app, btn),
            "phosphor is a flat CLI element, not a bevel"
        );
        assert!(!app.world().entity(btn).contains::<BoxShadow>());
        assert_eq!(label_color(&mut app, btn), theme.color(UiColor::Primary));

        // Selected inverts: solid phosphor fill, ink glyphs.
        app.world_mut().entity_mut(btn).insert(Selected);
        app.update();
        assert_eq!(bg(&app, btn), theme.color(UiColor::Primary));
        assert_eq!(label_color(&mut app, btn), theme.color(UiColor::Inverted));
        assert!(!has_gradient(&app, btn), "inverted phosphor is still flat");
    }

    /// `base/hardware` renders a button as a moulded control: a case-gradient
    /// face + a drop shadow + soft (7px) corners. Pins the bevel so an edit
    /// that drops the gradient (a flat button on the casing look) fails here.
    #[test]
    fn hardware_button_states_render_bevel() {
        let mut app = themed_app(HARDWARE_THEME_ID);
        let btn = app.world_mut().spawn(button(ButtonSpec::new("Go"))).id();
        app.update();

        let theme = hardware();
        assert!(has_gradient(&app, btn), "hardware face is a gradient bevel");
        assert!(
            app.world().entity(btn).contains::<BoxShadow>(),
            "hardware has depth"
        );
        assert_eq!(
            app.world().entity(btn).get::<Node>().unwrap().border_radius,
            BorderRadius::all(px(theme.metric(UiMetric::Radius)))
        );

        // Selected -> amber gradient, inverted ink glyphs (still a bevel).
        app.world_mut().entity_mut(btn).insert(Selected);
        app.update();
        assert!(has_gradient(&app, btn));
        assert_eq!(label_color(&mut app, btn), theme.color(UiColor::Inverted));
    }

    /// Moving the SELECTION restyles buttons ALREADY in the tree, and a button
    /// spawned on a frame with NO theme change is still painted for the current
    /// theme (the `Added<ThemedButton>` override). The second half FAILS with
    /// only the `theme.is_changed()` path wired, which is the whole point of the
    /// override (lesson mode-keyed-reconciler-just-spawned-override).
    #[test]
    fn theme_switch_restyles_spawned_widgets() {
        let mut app = themed_app(PHOSPHOR_THEME_ID);
        let first = app.world_mut().spawn(button(ButtonSpec::new("First"))).id();
        app.update();
        assert!(!has_gradient(&app, first), "phosphor: flat");

        select(&mut app, HARDWARE_THEME_ID);
        app.update();
        assert!(
            has_gradient(&app, first),
            "live button restyled to the hardware bevel"
        );

        // A button spawned now - a frame with NO theme change - must still be
        // painted hardware by the just-spawned override.
        let second = app
            .world_mut()
            .spawn(button(ButtonSpec::new("Second")))
            .id();
        app.update();
        assert!(
            has_gradient(&app, second),
            "just-spawned button painted for the current theme without a theme change"
        );
    }

    /// EVERY variant must give a press its own face, in BOTH base themes. Two
    /// bugs live here: a variant that reacts in one theme only (the hardware
    /// Danger face - the Exit button - collapsed hover and press into one
    /// paint), and a variant that reacts in NEITHER (`Ghost`, which
    /// `segmented_option` builds, so the Graphics-preset and theme rows had no
    /// press feedback at all). Parity alone would pass the second case, so
    /// assert both.
    ///
    /// Now a check on authored CONTENT rather than on code: it is the base
    /// themes' role tables that must keep pressed distinct from hovered.
    #[test]
    fn press_reads_differently_from_hover_in_both_base_themes() {
        // The visually load-bearing parts of a resolved state, comparable. The
        // gradient is compared by its STOPS, not its stop count: a variant that
        // reacts to press only through gradient colours must still read as
        // reacting.
        let face = |theme: &ActiveUiTheme, variant, state| {
            let p = theme.button(table(variant), state);
            (
                format!("{:?}", p.fill.base),
                format!("{:?}", p.text),
                format!("{:?}", p.fill.gradient),
                p.shadow.is_some(),
            )
        };
        let reacts_to_press = |theme: &ActiveUiTheme, variant| {
            face(theme, variant, ButtonState::Hovered) != face(theme, variant, ButtonState::Pressed)
        };

        let phosphor = phosphor();
        let hardware = hardware();
        for variant in [
            ButtonVariant::Default,
            ButtonVariant::Primary,
            ButtonVariant::Danger,
            ButtonVariant::Ghost,
        ] {
            for theme in [&phosphor, &hardware] {
                assert!(
                    reacts_to_press(theme, variant),
                    "{variant:?} has no press feedback in {}",
                    theme.id()
                );
            }
            assert_eq!(
                reacts_to_press(&phosphor, variant),
                reacts_to_press(&hardware, variant),
                "{variant:?} gives press its own face in one base theme but not the other"
            );
        }
    }

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

    /// Activating a `ThemedButton` carrying `ButtonValue<T>` copies that value
    /// into the `T` resource and marks it `Selected`, moving the marker off any
    /// prior selection. This is the exact path the editor's component cards (and
    /// the menu's tool buttons) rely on.
    #[test]
    fn activating_a_valued_button_sets_the_resource_and_selection() {
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

        // flush: the observer moves `Selected` through `Commands`, and a bare
        // `World::trigger` outside a schedule leaves that queue unapplied.
        app.world_mut().trigger(Activate { entity: a });
        app.world_mut().flush();
        assert_eq!(*app.world().resource::<Choice>(), Choice::A);
        assert!(app.world().entity(a).contains::<Selected>());

        app.world_mut().trigger(Activate { entity: b });
        app.world_mut().flush();
        assert_eq!(*app.world().resource::<Choice>(), Choice::B);
        assert!(app.world().entity(b).contains::<Selected>());
        assert!(
            !app.world().entity(a).contains::<Selected>(),
            "the previous selection is cleared"
        );

        // CANCEL: mouse-down alone must commit nothing (press, drag off,
        // release). Only `Activate` - release over the button - commits.
        app.world_mut().entity_mut(a).insert(Pressed);
        app.world_mut().flush();
        assert_eq!(
            *app.world().resource::<Choice>(),
            Choice::B,
            "a bare press must not commit the setting"
        );
        assert!(!app.world().entity(a).contains::<Selected>());
    }
}
