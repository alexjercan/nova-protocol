//! Shared themed widgets, rendered from the NOVA OS palette ([`crate::theme`])
//! in one of two skins ([`crate::skin::UiSkin`]): the phosphor CLI terminal look
//! (default) and the light-3D hardware casing.
//!
//! The heart is the [`ThemedButton`]: one click + colour model for every screen
//! (menu, editor, HUD chrome). Its visual is a pure function of
//! `(skin, variant, state)` - see [`button_paint`] - applied both by the
//! per-interaction observers (hover/press/disable/select) and by
//! [`reconcile_button_skins`], the system that restyles LIVE widgets when the
//! `UiSkin` resource flips or a new button is spawned. Small layout helpers
//! (`panel_header`, `separator`) render the rest of the chrome.

use bevy::{
    ecs::relationship::RelatedSpawner,
    picking::hover::Hovered,
    platform::collections::HashSet,
    prelude::*,
    reflect::Is,
    text::FontSource,
    ui::{InteractionDisabled, Pressed},
    ui_widgets::{Button, SliderValue},
};

use crate::{font::UiFont, skin::UiSkin, theme};

// ============================ components =====================================

/// Marks the currently-active button within a `ButtonValue<T>` selection group.
#[derive(Component)]
pub struct Selected;

/// Marks a themed button so the colour observers + skin reconciler pick it up.
#[derive(Component)]
pub struct ThemedButton;

/// The emphasis a themed button carries. Absent = [`ButtonVariant::Default`].
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    /// Neutral button.
    #[default]
    Default,
    /// Primary call-to-action: solid phosphor (phosphor skin) / phosphor
    /// gradient (hardware), always inverted glyphs.
    Primary,
    /// Destructive action: red family.
    Danger,
    /// Border-only, transparent fill.
    Ghost,
}

/// Marks the primary label text span inside a themed button, so the paint code
/// can recolour it (e.g. inverted [`theme::INK`] on selection) without touching
/// the key-chip or block-cursor spans.
#[derive(Component)]
pub struct ButtonLabel;

/// Marks the `> ` block-cursor span (block buttons only): shown (opaque) on
/// hover/selection, hidden (transparent) otherwise.
#[derive(Component)]
pub struct ButtonCursor;

/// Marks any text span whose face should route through [`UiFont`] (Iosevka
/// Term). [`apply_ui_font`] fills the handle once the resource exists.
#[derive(Component)]
pub struct UiText;

/// The value a settings button represents. Kept distinct from the `T` resource so a
/// button can carry a choice without being interpreted as - and clobbering - the resource
/// itself: on Bevy 0.19 a `#[derive(Resource)]` type is component-backed, so putting it on
/// a button entity is treated as a resource insert.
#[derive(Component, Debug, Clone)]
pub struct ButtonValue<T>(pub T);

/// Guard resource: the themed-widget observers + systems are app-global, so the
/// first [`register`] call wins and later calls are no-ops (menu and editor both
/// register in the shipped app; doubled observers would write every colour
/// twice per interaction).
#[derive(Resource)]
struct WidgetObserversRegistered;

// ============================ registration ===================================

/// Wire the button colour observers, the skin reconciler and the font router.
/// Call from each app/plugin that uses themed widgets; guarded, so independent
/// plugins can coexist.
pub fn register(app: &mut App) {
    if app.world().contains_resource::<WidgetObserversRegistered>() {
        return;
    }
    app.insert_resource(WidgetObserversRegistered);
    // The global skin (owned/persisted by settings in task 20260728-175738;
    // init_resource is idempotent so registering here keeps the widget layer
    // self-contained for tests + slim apps).
    app.init_resource::<UiSkin>();

    app.add_observer(button_on_interaction::<Add, Pressed>)
        .add_observer(button_on_interaction::<Remove, Pressed>)
        .add_observer(button_on_interaction::<Add, InteractionDisabled>)
        .add_observer(button_on_interaction::<Remove, InteractionDisabled>)
        .add_observer(button_on_interaction::<Insert, Hovered>)
        .add_observer(button_on_interaction::<Add, Selected>)
        .add_observer(button_on_interaction::<Remove, Selected>);

    // Interactive list rows (mods/scenarios): repaint on hover/selection.
    app.add_observer(list_row_on_interaction::<Insert, Hovered>)
        .add_observer(list_row_on_interaction::<Add, Selected>)
        .add_observer(list_row_on_interaction::<Remove, Selected>);

    app.add_systems(
        Update,
        (
            reconcile_button_skins,
            reconcile_list_row_skins,
            apply_ui_font,
            sync_slider_meters,
        ),
    );
}

// ============================ paint model ====================================

/// The full visual of a button in a given `(skin, variant, state)`. Applied
/// identically by the interaction observers and the skin reconciler, so the two
/// paths can never disagree.
struct Paint {
    /// Solid background (phosphor skin) or the base under the gradient (hardware).
    bg: Color,
    border: Color,
    /// Colour of the [`ButtonLabel`] span.
    text: Color,
    radius: f32,
    /// `Some` on the hardware skin: the moulded-face gradient.
    gradient: Option<BackgroundGradient>,
    /// `Some` on the hardware skin: the drop shadow giving the face depth.
    shadow: Option<BoxShadow>,
    /// Block-cursor visibility (hover/selected).
    cursor_visible: bool,
}

/// Light green-grey of hardware-face button text (demo `#dcefe0`).
const HW_TEXT: Color = Color::srgb_u8(0xdc, 0xef, 0xe0);
/// Softened red text on a hovered danger face (demo `#ffd9d5`).
const DANGER_TEXT_HOT: Color = Color::srgb_u8(0xff, 0xd9, 0xd5);
/// Brightened phosphor label text on a hovered phosphor button (demo `#d6ffe4`).
const PHOSPHOR_HOVER_TEXT: Color = Color::srgb_u8(0xd6, 0xff, 0xe4);

fn grad3(top: Color, mid: Color, bottom: Color) -> BackgroundGradient {
    BackgroundGradient(vec![LinearGradient::degrees(
        180.0,
        vec![
            ColorStop::percent(top, 0.0),
            ColorStop::percent(mid, 55.0),
            ColorStop::percent(bottom, 100.0),
        ],
    )
    .into()])
}

fn grad2(top: Color, bottom: Color) -> BackgroundGradient {
    BackgroundGradient(vec![LinearGradient::degrees(
        180.0,
        vec![
            ColorStop::percent(top, 0.0),
            ColorStop::percent(bottom, 100.0),
        ],
    )
    .into()])
}

/// The moulded-face drop shadow (`--drop`): outset only, since Bevy 0.19
/// `BoxShadow` has no inset shadows (the demo's inner rim/undercut are
/// approximated by the face gradient, matching the NOVA OS casing).
fn drop_shadow() -> BoxShadow {
    BoxShadow::new(
        Color::srgba(0.0, 0.0, 0.0, 0.55),
        Val::ZERO,
        Val::Px(2.0),
        Val::ZERO,
        Val::Px(8.0),
    )
}

/// A coloured glow drop shadow (selected/primary faces). Kept subtle so it
/// reads as a lit element, not a blurry halo that fights the text.
fn glow_shadow(color: Color) -> BoxShadow {
    BoxShadow::new(
        color.with_alpha(0.22),
        Val::ZERO,
        Val::ZERO,
        Val::ZERO,
        Val::Px(7.0),
    )
}

/// The pure visual function: `(skin, variant, state) -> Paint`.
fn button_paint(
    skin: UiSkin,
    variant: ButtonVariant,
    disabled: bool,
    hovered: bool,
    pressed: bool,
    selected: bool,
) -> Paint {
    let cursor_visible = hovered || selected;
    match skin {
        UiSkin::Phosphor => phosphor_paint(
            variant,
            disabled,
            hovered,
            pressed,
            selected,
            cursor_visible,
        ),
        UiSkin::Hardware => hardware_paint(
            variant,
            disabled,
            hovered,
            pressed,
            selected,
            cursor_visible,
        ),
    }
}

fn phosphor_paint(
    variant: ButtonVariant,
    disabled: bool,
    hovered: bool,
    pressed: bool,
    selected: bool,
    cursor_visible: bool,
) -> Paint {
    let p = theme::PHOSPHOR;
    // Primary reads like a permanent selection (solid phosphor, inverted glyphs).
    let inverted = selected || matches!(variant, ButtonVariant::Primary);

    let (bg, border, text) = if disabled {
        (p.with_alpha(0.02), p.with_alpha(0.12), p.with_alpha(0.3))
    } else if inverted {
        (p, p, theme::INK)
    } else {
        match variant {
            ButtonVariant::Danger => {
                let r = theme::RED;
                if pressed {
                    (r.with_alpha(0.2), r, r)
                } else if hovered {
                    (r.with_alpha(0.16), r, DANGER_TEXT_HOT)
                } else {
                    (r.with_alpha(0.06), r.with_alpha(0.5), r)
                }
            }
            ButtonVariant::Ghost => {
                if hovered || pressed {
                    (p.with_alpha(0.06), p.with_alpha(0.4), p)
                } else {
                    (Color::NONE, p.with_alpha(0.25), p)
                }
            }
            _ => {
                if pressed {
                    (p.with_alpha(0.2), p, p)
                } else if hovered {
                    (p.with_alpha(0.12), p, PHOSPHOR_HOVER_TEXT)
                } else {
                    (p.with_alpha(0.05), p.with_alpha(0.4), p)
                }
            }
        }
    };

    // The inverted (selected/primary) phosphor face carries the PoC glow - a
    // shadow, not a gradient, so the "phosphor is a flat CLI element" contract
    // (which forbids a bevel GRADIENT) still holds.
    let shadow = (!disabled && inverted).then(|| glow_shadow(theme::PHOSPHOR));

    Paint {
        bg,
        border,
        text,
        radius: theme::RADIUS,
        gradient: None,
        shadow,
        cursor_visible,
    }
}

fn hardware_paint(
    variant: ButtonVariant,
    disabled: bool,
    hovered: bool,
    pressed: bool,
    selected: bool,
    cursor_visible: bool,
) -> Paint {
    let radius = theme::RADIUS_HW;
    let border = theme::CASE_EDGE;

    // Selected -> amber gradient; Primary -> phosphor gradient; both inverted.
    if !disabled && selected {
        return Paint {
            bg: theme::AMBER_NOVA,
            border,
            text: theme::INK,
            radius,
            gradient: Some(grad3(theme::AMBER_HI, theme::AMBER_NOVA, theme::AMBER_LO)),
            shadow: Some(glow_shadow(theme::AMBER_NOVA)),
            cursor_visible,
        };
    }
    if !disabled && matches!(variant, ButtonVariant::Primary) {
        return Paint {
            bg: theme::PHOSPHOR,
            border,
            text: theme::INK,
            radius,
            gradient: Some(grad3(
                theme::PHOSPHOR_HI,
                theme::PHOSPHOR,
                theme::PHOSPHOR_LO,
            )),
            shadow: Some(glow_shadow(theme::PHOSPHOR)),
            cursor_visible,
        };
    }

    if disabled {
        return Paint {
            bg: theme::CASE_1,
            border,
            text: HW_TEXT.with_alpha(0.34),
            radius,
            gradient: Some(grad3(theme::CASE_3, theme::CASE_1, theme::CASE_0)),
            shadow: Some(drop_shadow()),
            cursor_visible,
        };
    }

    match variant {
        ButtonVariant::Ghost => Paint {
            bg: if hovered {
                Color::WHITE.with_alpha(0.04)
            } else {
                Color::NONE
            },
            border: Color::WHITE.with_alpha(if hovered { 0.22 } else { 0.12 }),
            text: HW_TEXT,
            radius,
            gradient: None,
            shadow: None,
            cursor_visible,
        },
        ButtonVariant::Danger => {
            if hovered || pressed {
                Paint {
                    bg: theme::RED,
                    border,
                    text: Color::WHITE,
                    radius,
                    gradient: Some(grad2(
                        Color::srgb_u8(0x6b, 0x2a, 0x26),
                        Color::srgb_u8(0x3a, 0x15, 0x12),
                    )),
                    shadow: Some(drop_shadow()),
                    cursor_visible,
                }
            } else {
                Paint {
                    bg: theme::CASE_1,
                    border,
                    text: DANGER_TEXT_HOT,
                    radius,
                    gradient: Some(grad3(theme::CASE_3, theme::CASE_1, theme::CASE_0)),
                    shadow: Some(drop_shadow()),
                    cursor_visible,
                }
            }
        }
        _ => {
            if pressed {
                Paint {
                    bg: theme::CASE_0,
                    border,
                    text: HW_TEXT,
                    radius,
                    gradient: Some(grad2(theme::CASE_0, theme::CASE_1)),
                    shadow: None,
                    cursor_visible,
                }
            } else if hovered {
                Paint {
                    bg: theme::CASE_HOT_MID,
                    border,
                    text: Color::WHITE,
                    radius,
                    gradient: Some(grad3(
                        theme::CASE_HOT_HI,
                        theme::CASE_HOT_MID,
                        theme::CASE_HOT_LO,
                    )),
                    shadow: Some(drop_shadow()),
                    cursor_visible,
                }
            } else {
                Paint {
                    bg: theme::CASE_1,
                    border,
                    text: HW_TEXT,
                    radius,
                    gradient: Some(grad3(theme::CASE_3, theme::CASE_1, theme::CASE_0)),
                    shadow: Some(drop_shadow()),
                    cursor_visible,
                }
            }
        }
    }
}

// ============================ apply / observers ==============================

type LabelText<'w, 's> =
    Query<'w, 's, &'static mut TextColor, (With<ButtonLabel>, Without<ButtonCursor>)>;
type CursorText<'w, 's> =
    Query<'w, 's, &'static mut TextColor, (With<ButtonCursor>, Without<ButtonLabel>)>;

/// Apply a computed [`Paint`] to one button: its own fill/border/radius +
/// gradient/shadow (inserted or removed to switch skins) + its label/cursor
/// spans' colours.
#[allow(clippy::too_many_arguments)]
fn apply_paint(
    paint: Paint,
    commands: &mut Commands,
    entity: Entity,
    bg: &mut BackgroundColor,
    border: &mut BorderColor,
    node: &mut Node,
    children: &Children,
    q_label: &mut LabelText,
    q_cursor: &mut CursorText,
) {
    *bg = paint.bg.into();
    border.set_all(paint.border);
    node.border_radius = BorderRadius::all(px(paint.radius));

    // try_insert / try_remove, not insert / remove: a button can be despawned
    // the SAME frame the reconciler paints it (a menu/state teardown despawns
    // its buttons while this deferred command is still queued). The plain forms
    // error via the fallback handler ("Entity despawned") - which the smoke
    // examples promote to a panic; the try_ forms silently no-op on a dead
    // entity. (Repo idiom, e.g. nova_gameplay integrity/glue.rs.)
    let mut ent = commands.entity(entity);
    match paint.gradient {
        Some(g) => {
            ent.try_insert(g);
        }
        None => {
            ent.try_remove::<BackgroundGradient>();
        }
    }
    match paint.shadow {
        Some(s) => {
            ent.try_insert(s);
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
            // The cursor keeps its phosphor hue; only its alpha toggles.
            *tc =
                TextColor(theme::PHOSPHOR.with_alpha(if paint.cursor_visible { 1.0 } else { 0.0 }));
        }
    }
}

/// The button colour observer: on any hover/press/disable/select change,
/// recompute + apply the button's paint for the current skin. Generic over the
/// event `E` and component `C` so one body handles Add/Remove/Insert of
/// `Pressed`, `InteractionDisabled`, `Hovered` and `Selected`; the removed
/// component still reads present inside its own `Remove` observer, so its state
/// is forced false there.
fn button_on_interaction<E: EntityEvent, C: Component>(
    event: On<E, C>,
    skin: Res<UiSkin>,
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

    let paint = button_paint(*skin, variant, disabled, hovered.get(), pressed, selected);
    apply_paint(
        paint,
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

/// Restyle LIVE themed buttons: on a `UiSkin` change repaint every button;
/// otherwise paint only the just-spawned ones (`Added<ThemedButton>`, an
/// override that must defer past the deferred-spawn flush - hence a SYSTEM, not
/// an `Add` observer - per lesson mode-keyed-reconciler-just-spawned-override).
#[allow(clippy::type_complexity)]
fn reconcile_button_skins(
    skin: Res<UiSkin>,
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
    let restyle_all = skin.is_changed();
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
        let paint = button_paint(*skin, variant, disabled, hovered.get(), pressed, selected);
        apply_paint(
            paint,
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

/// Route every [`UiText`] span through the shared [`UiFont`] (Iosevka Term) once
/// the resource exists: on the resource landing, set every span; afterwards, set
/// each newly spawned span. A headless rig with no `UiFont` keeps the engine
/// default font.
fn apply_ui_font(
    font: Option<Res<UiFont>>,
    added: Query<Entity, Added<UiText>>,
    mut all: Query<&mut TextFont, With<UiText>>,
) {
    let Some(font) = font else {
        return;
    };
    if font.is_changed() {
        let handle = font.handle();
        for mut tf in &mut all {
            tf.font = FontSource::Handle(handle.clone());
        }
    } else {
        for entity in &added {
            if let Ok(mut tf) = all.get_mut(entity) {
                tf.font = FontSource::Handle(font.handle());
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

// ============================ button factories ===============================

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
            min_height: 34.0,
            font_size: 14.0,
        }
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
/// optional block cursor and key-chip, as children; the initial colours are the
/// phosphor idle face and get corrected to the live skin by
/// [`reconcile_button_skins`] on the frame it appears.
pub fn button(spec: ButtonSpec) -> impl Bundle {
    let ButtonSpec {
        text,
        variant,
        block,
        key,
        min_height,
        font_size,
    } = spec;

    let justify = if block {
        JustifyContent::FlexStart
    } else {
        JustifyContent::Center
    };

    // Phosphor idle face (default skin) - the reconciler repaints on spawn.
    let idle = phosphor_paint(variant, false, false, false, false, false);

    (
        Node {
            width: percent(100),
            min_height: px(min_height),
            margin: UiRect::vertical(px(4)),
            padding: UiRect::axes(px(12), px(6)),
            border: UiRect::all(px(theme::BORDER_W)),
            justify_content: justify,
            align_items: AlignItems::Center,
            column_gap: px(8),
            border_radius: BorderRadius::all(px(theme::RADIUS)),
            ..default()
        },
        ThemedButton,
        variant,
        Button,
        Hovered::default(),
        BorderColor::all(idle.border),
        BackgroundColor(idle.bg),
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
                    TextColor(theme::PHOSPHOR.with_alpha(0.0)),
                ));
            }
            parent.spawn((
                ButtonLabel,
                UiText,
                Text::new(text),
                TextFont {
                    font_size: FontSize::Px(font_size),
                    ..default()
                },
                TextColor(idle.text),
                // No TextShadow: Bevy's `TextShadow` is a hard drop shadow (no
                // blur), so its default 4px black offset ghosts the label on a
                // bright/inverted fill instead of glowing. Crisp CLI text needs
                // no shadow.
            ));
            if let Some(key) = key {
                parent.spawn(key_chip(&key, font_size));
            }
        })),
    )
}

/// The trailing amber key-chip span (`Enter`/`Esc`), bordered per the demo.
fn key_chip(text: &str, font_size: f32) -> impl Bundle {
    (
        Node {
            margin: UiRect::left(px(8)),
            padding: UiRect::axes(px(5), px(1)),
            border: UiRect::all(px(theme::BORDER_W)),
            border_radius: BorderRadius::all(px(theme::RADIUS)),
            ..default()
        },
        BorderColor::all(theme::AMBER_NOVA.with_alpha(0.5)),
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.3)),
        children![(
            UiText,
            Text::new(text.to_string()),
            TextFont {
                font_size: FontSize::Px((font_size - 2.0).max(10.0)),
                ..default()
            },
            TextColor(theme::AMBER_NOVA),
        )],
    )
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

// ============================ layout helpers =================================

/// A small uppercase section header (e.g. "COMPONENTS"), flat phosphor. No
/// TextShadow: a hard drop shadow makes the header feel floaty; a terminal
/// header should read as imprinted on the screen.
pub fn panel_header(text: &str) -> impl Bundle {
    (
        UiText,
        Text::new(text.to_uppercase()),
        TextFont {
            font_size: FontSize::Px(13.0),
            ..default()
        },
        TextColor(theme::PHOSPHOR),
        Node {
            margin: UiRect::bottom(px(8)),
            ..default()
        },
    )
}

/// A thin horizontal separator (phosphor-muted rule).
pub fn separator() -> impl Bundle {
    (
        Node {
            width: percent(100),
            height: px(theme::BORDER_W),
            margin: UiRect::vertical(px(8)),
            ..default()
        },
        BackgroundColor(theme::PHOSPHOR_MUTED.with_alpha(0.5)),
    )
}

// ============================ shared widget zoo ==============================
// The rest of the demo-1 widget set. These read the CURRENT skin at spawn (the
// button family above is the live-reactive part - the whole in-game UI is mostly
// buttons). Their in-screen consumers (Settings/Mods/Scenarios) mount them in
// task 20260728-175738, which is where a live-restyle marker for them lands if
// the owner wants the setting to reskin them in place too.

/// Semantic colour families for [`badge`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BadgeKind {
    /// Phosphor green - nominal / online.
    Green,
    /// Amber - warning.
    Amber,
    /// Blue - info.
    Blue,
    /// Red - fault.
    Red,
    /// Muted grey - idle / neutral tag.
    Mute,
}

impl BadgeKind {
    fn color(self) -> Color {
        match self {
            BadgeKind::Green => theme::PHOSPHOR,
            BadgeKind::Amber => theme::AMBER_NOVA,
            BadgeKind::Blue => theme::BLUE,
            BadgeKind::Red => theme::RED,
            BadgeKind::Mute => theme::PHOSPHOR_MUTED.with_alpha(0.9),
        }
    }
}

/// A status badge. Phosphor skin: a bracketed `[TAG]` text span in the family
/// colour, no chrome. Hardware skin: a bordered, tinted chip.
pub fn badge(kind: BadgeKind, text: &str, skin: UiSkin) -> impl Bundle {
    let color = kind.color();
    let phosphor = skin.is_phosphor();
    let label = if phosphor {
        format!("[{}]", text.to_uppercase())
    } else {
        text.to_uppercase()
    };
    let (border, bg, border_w) = if phosphor {
        (Color::NONE, Color::NONE, 0.0)
    } else {
        (
            color.with_alpha(0.4),
            color.with_alpha(0.06),
            theme::BORDER_W,
        )
    };
    (
        Node {
            padding: UiRect::axes(px(if phosphor { 2.0 } else { 8.0 }), px(3.0)),
            border: UiRect::all(px(border_w)),
            border_radius: BorderRadius::all(px(theme::RADIUS)),
            align_items: AlignItems::Center,
            ..default()
        },
        BorderColor::all(border),
        BackgroundColor(bg),
        children![(
            UiText,
            Text::new(label),
            TextFont {
                font_size: FontSize::Px(10.0),
                ..default()
            },
            TextColor(color),
        )],
    )
}

/// A 22px checkbox. Off: bordered empty square. On: inverted (filled fill, dark
/// `x`). Phosphor uses 2px corners + phosphor; hardware uses 5px + a recessed
/// case face turning phosphor when on.
pub fn checkbox(on: bool, skin: UiSkin) -> impl Bundle {
    let phosphor = skin.is_phosphor();
    let radius = if phosphor { theme::RADIUS } else { 5.0 };
    let (bg, border, glyph) = checkbox_colors(on, skin);
    (
        Node {
            width: px(22),
            height: px(22),
            flex_shrink: 0.0,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(px(theme::BORDER_W)),
            border_radius: BorderRadius::all(px(radius)),
            ..default()
        },
        BorderColor::all(border),
        BackgroundColor(bg),
        children![(
            UiText,
            Text::new(checkbox_glyph(on)),
            TextFont {
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor(glyph),
        )],
    )
}

/// The `(background, border, glyph)` colours of a checkbox in `(on, skin)` - the
/// single source both [`checkbox`] and an in-place restyle (the mods screen's
/// enable-checkbox sync) paint from.
pub fn checkbox_colors(on: bool, skin: UiSkin) -> (Color, Color, Color) {
    if on {
        (theme::PHOSPHOR, theme::PHOSPHOR, theme::INK)
    } else if skin.is_phosphor() {
        (
            Color::NONE,
            theme::PHOSPHOR.with_alpha(0.4),
            theme::PHOSPHOR,
        )
    } else {
        (theme::CASE_0, theme::CASE_EDGE, theme::PHOSPHOR)
    }
}

/// The checkbox glyph: `x` when checked, empty otherwise.
pub fn checkbox_glyph(on: bool) -> &'static str {
    if on {
        "x"
    } else {
        ""
    }
}

/// The panel PAINT: a bordered surface's colours - phosphor is a dark screen
/// face with an inner phosphor border + top glow, hardware a case gradient +
/// drop shadow. It carries NO `Node`, so add it to your own sized `Node` (use
/// [`panel_node`] for a plain content-sized column, or a modal's own
/// width/height/padding Node that sets `border` + `border_radius`). Spawn
/// children (a `panel_head`, rows, ...) into that node.
pub fn panel(skin: UiSkin) -> impl Bundle {
    if skin.is_phosphor() {
        (
            BorderColor::all(theme::PHOSPHOR.with_alpha(0.16)),
            BackgroundColor(theme::SCREEN_0),
            BoxShadow(vec![]),
            // PoC phosphor panel: a soft phosphor glow blooming down from the top
            // edge (demo `radial-gradient(ellipse 70% 60% at 50% 0%, phosphor .10,
            // transparent 70%)`).
            BackgroundGradient(vec![Gradient::from(RadialGradient::new(
                UiPosition::TOP,
                RadialGradientShape::FarthestSide,
                vec![
                    ColorStop::percent(theme::PHOSPHOR.with_alpha(0.06), 0.0),
                    ColorStop::percent(Color::NONE, 70.0),
                ],
            ))]),
        )
    } else {
        (
            BorderColor::all(theme::CASE_EDGE),
            BackgroundColor(theme::CASE_1),
            drop_shadow(),
            // PoC panel: linear-gradient(168deg, case-2 0%, case-0 88%, #05080a).
            BackgroundGradient(vec![LinearGradient::degrees(
                168.0,
                vec![
                    ColorStop::percent(theme::CASE_2, 0.0),
                    ColorStop::percent(theme::CASE_0, 88.0),
                    ColorStop::percent(theme::CASE_EDGE, 100.0),
                ],
            )
            .into()]),
        )
    }
}

/// A ready-made panel `Node` for a plain content-sized panel: a column with the
/// panel border + [`theme::PANEL_RADIUS`] corners. Pair with [`panel`] for the
/// paint. Sized panels (modals) supply their own `Node` (width/height/padding)
/// with the same `border` + `border_radius` and add [`panel`].
pub fn panel_node() -> Node {
    Node {
        flex_direction: FlexDirection::Column,
        border: UiRect::all(px(theme::BORDER_W)),
        border_radius: BorderRadius::all(px(theme::PANEL_RADIUS)),
        ..default()
    }
}

/// A pill toggle switch (demo `.toggle`): a 44x22 track with a sliding dot.
/// Off: the dot sits left, muted. On: the dot slides right and the track tints
/// phosphor (phosphor skin) with a glowing dot. Phosphor uses 2px square-ish
/// corners; hardware uses a rounded pill + a recessed well look. Not a
/// `ThemedButton` - toggles carry their own click handler and re-spawn on flip.
pub fn toggle(on: bool, skin: UiSkin) -> impl Bundle {
    let phosphor = skin.is_phosphor();
    let radius = if phosphor { theme::RADIUS } else { 11.0 };
    let dot_radius = if phosphor { 1.0 } else { 9.0 };
    let (bg, border) = if on {
        (theme::PHOSPHOR.with_alpha(0.14), theme::PHOSPHOR)
    } else if phosphor {
        (
            Color::srgba(0.0, 0.0, 0.0, 0.4),
            theme::PHOSPHOR.with_alpha(0.35),
        )
    } else {
        (Color::srgba(0.0, 0.0, 0.0, 0.5), theme::CASE_EDGE)
    };
    let dot = if on {
        theme::PHOSPHOR
    } else if phosphor {
        theme::PHOSPHOR_MUTED
    } else {
        Color::srgb_u8(0x55, 0x63, 0x6c)
    };
    // The "on" dot glows (phosphor skin); the off dot is flat.
    let dot_shadow = if on && phosphor {
        Some(BoxShadow::new(
            theme::PHOSPHOR.with_alpha(0.35),
            Val::ZERO,
            Val::ZERO,
            Val::ZERO,
            Val::Px(5.0),
        ))
    } else {
        None
    };
    (
        Node {
            width: px(44),
            height: px(22),
            flex_shrink: 0.0,
            border: UiRect::all(px(theme::BORDER_W)),
            border_radius: BorderRadius::all(px(radius)),
            // The dot slides right when on, left when off.
            justify_content: if on {
                JustifyContent::FlexEnd
            } else {
                JustifyContent::FlexStart
            },
            align_items: AlignItems::Center,
            padding: UiRect::all(px(2)),
            ..default()
        },
        BorderColor::all(border),
        BackgroundColor(bg),
        Children::spawn(SpawnWith(move |parent: &mut RelatedSpawner<ChildOf>| {
            let mut dot_entity = parent.spawn((
                Node {
                    width: px(16),
                    height: px(16),
                    border_radius: BorderRadius::all(px(dot_radius)),
                    ..default()
                },
                BackgroundColor(dot),
            ));
            if let Some(shadow) = dot_shadow {
                dot_entity.insert(shadow);
            }
        })),
    )
}

/// A panel header row: a glowing uppercase title, a rule that fills the row, and
/// an optional trailing tag chip (phosphor-muted bordered).
pub fn panel_head(title: &str, tag: Option<&str>, _skin: UiSkin) -> impl Bundle {
    let title = title.to_uppercase();
    let tag = tag.map(str::to_string);
    (
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(10),
            padding: UiRect::axes(px(15), px(11)),
            border: UiRect::bottom(px(theme::BORDER_W)),
            ..default()
        },
        BorderColor::all(theme::PHOSPHOR.with_alpha(0.18)),
        Children::spawn(SpawnWith(move |parent: &mut RelatedSpawner<ChildOf>| {
            parent.spawn((
                UiText,
                Text::new(title),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(theme::PHOSPHOR),
                // Flat, no drop shadow - imprinted on the screen, not floating.
            ));
            // The rule: a thin flex-filling phosphor-muted line.
            parent.spawn((
                Node {
                    flex_grow: 1.0,
                    height: px(theme::BORDER_W),
                    ..default()
                },
                BackgroundColor(theme::PHOSPHOR_MUTED.with_alpha(0.6)),
            ));
            if let Some(tag) = tag {
                parent.spawn((
                    Node {
                        padding: UiRect::axes(px(7.0), px(2.0)),
                        border: UiRect::all(px(theme::BORDER_W)),
                        border_radius: BorderRadius::all(px(4.0)),
                        ..default()
                    },
                    BorderColor::all(theme::PHOSPHOR.with_alpha(0.4)),
                    children![(
                        UiText,
                        Text::new(tag),
                        TextFont {
                            font_size: FontSize::Px(10.0),
                            ..default()
                        },
                        TextColor(theme::PHOSPHOR),
                    )],
                ));
            }
        })),
    )
}

/// A list row container (spawn an icon / text / trailing widget into it).
/// Transparent with an inset border; selection tints amber (hardware) or inverts
/// phosphor. Not a `ThemedButton` - rows carry their own click handlers.
pub fn list_row(selected: bool, skin: UiSkin) -> impl Bundle {
    let (bg, border) = list_row_colors(selected, false, skin);
    (
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(12),
            padding: UiRect::axes(px(13), px(10)),
            border: UiRect::all(px(theme::BORDER_W)),
            border_radius: BorderRadius::all(px(if skin.is_phosphor() {
                theme::RADIUS
            } else {
                7.0
            })),
            ..default()
        },
        BorderColor::all(border),
        BackgroundColor(bg),
    )
}

/// The `(background, border)` of a list row in a given `(selected, hovered,
/// skin)` - the single source both [`list_row`] and the [`ListRow`] reconciler
/// paint from.
pub fn list_row_colors(selected: bool, hovered: bool, skin: UiSkin) -> (Color, Color) {
    let phosphor = skin.is_phosphor();
    match (selected, phosphor) {
        (true, true) => (theme::PHOSPHOR.with_alpha(0.14), theme::PHOSPHOR),
        (true, false) => (theme::CASE_2, theme::AMBER_NOVA.with_alpha(0.5)),
        (false, true) if hovered => (
            theme::PHOSPHOR.with_alpha(0.06),
            theme::PHOSPHOR.with_alpha(0.2),
        ),
        (false, true) => (Color::NONE, theme::PHOSPHOR.with_alpha(0.14)),
        (false, false) if hovered => (Color::WHITE.with_alpha(0.06), Color::WHITE.with_alpha(0.1)),
        (false, false) => (Color::WHITE.with_alpha(0.02), Color::WHITE.with_alpha(0.05)),
    }
}

/// Marks an INTERACTIVE list row (a `list_row` + `Button` + `Hovered`): the
/// reconciler repaints it from its `Selected`/`Hovered` state + the skin, so
/// clicking or hovering highlights it live (mods/scenarios rows). A plain
/// `list_row` without this marker is a static display row (the widget_zoo).
#[derive(Component)]
pub struct ListRow;

fn paint_list_row(
    skin: UiSkin,
    selected: bool,
    hovered: bool,
    bg: &mut BackgroundColor,
    border: &mut BorderColor,
) {
    let (b, br) = list_row_colors(selected, hovered, skin);
    *bg = b.into();
    border.set_all(br);
}

/// Repaint one [`ListRow`] on a hover/selection change (the removed component
/// still reads present in its own `Remove` observer, so it is forced false).
fn list_row_on_interaction<E: EntityEvent, C: Component>(
    event: On<E, C>,
    skin: Res<UiSkin>,
    mut q: Query<
        (
            &Hovered,
            Has<Selected>,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        With<ListRow>,
    >,
) {
    if let Ok((hovered, selected, mut bg, mut border)) = q.get_mut(event.event_target()) {
        let selected = selected && !(E::is::<Remove>() && C::is::<Selected>());
        paint_list_row(*skin, selected, hovered.get(), &mut bg, &mut border);
    }
}

/// Restyle LIVE list rows on a `UiSkin` change, and paint just-spawned rows
/// (`Added<ListRow>`) - the same override the button reconciler uses.
#[allow(clippy::type_complexity)]
fn reconcile_list_row_skins(
    skin: Res<UiSkin>,
    mut q: Query<
        (
            Entity,
            &Hovered,
            Has<Selected>,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        With<ListRow>,
    >,
    added: Query<Entity, Added<ListRow>>,
) {
    let restyle_all = skin.is_changed();
    let just_added: HashSet<Entity> = added.iter().collect();
    if !restyle_all && just_added.is_empty() {
        return;
    }
    for (entity, hovered, selected, mut bg, mut border) in &mut q {
        if !restyle_all && !just_added.contains(&entity) {
            continue;
        }
        paint_list_row(*skin, selected, hovered.get(), &mut bg, &mut border);
    }
}

/// Number of segments in a phosphor slider's block-meter.
pub const SLIDER_SEGMENTS: usize = 24;

/// Marks one segment of a phosphor slider block-meter; the tuple is its index.
/// A driver (e.g. a slider's value-change system) reads the index + the current
/// fraction through [`slider_meter_color`] to light the bar up to the value.
#[derive(Component)]
pub struct SliderBlock(pub usize);

/// The colour of block `index` for a slider at `fraction`: full phosphor if the
/// bar is lit (below the value), dim phosphor otherwise.
pub fn slider_meter_color(index: usize, fraction: f32) -> Color {
    let lit = (fraction.clamp(0.0, 1.0) * SLIDER_SEGMENTS as f32).round() as usize;
    if index < lit {
        theme::PHOSPHOR
    } else {
        theme::PHOSPHOR.with_alpha(0.16)
    }
}

/// A re-skin of the audio Slider's track (the existing `bevy_ui_widgets::Slider`
/// keeps its value/range behaviour; this is the visual). Phosphor: a bordered
/// track filled with a segmented BLOCK-METER (a row of [`SliderBlock`] bars lit
/// up to `fraction`, no knob), matching the demo's dotted fill. Hardware: a
/// solid phosphor-dim fill. `fraction` in `[0, 1]`. Spawn as the slider's child.
pub fn slider_track(fraction: f32, skin: UiSkin) -> impl Bundle {
    let phosphor = skin.is_phosphor();
    let fraction = fraction.clamp(0.0, 1.0);
    let (height, radius, track_bg, track_border) = if phosphor {
        (
            14.0,
            theme::RADIUS,
            Color::srgba(0.0, 0.0, 0.0, 0.5),
            theme::PHOSPHOR.with_alpha(0.32),
        )
    } else {
        (
            10.0,
            6.0,
            Color::srgba(0.0, 0.0, 0.0, 0.55),
            theme::CASE_EDGE,
        )
    };
    (
        Node {
            width: percent(100),
            height: px(height),
            border: UiRect::all(px(theme::BORDER_W)),
            border_radius: BorderRadius::all(px(radius)),
            align_items: AlignItems::Center,
            padding: if phosphor {
                UiRect::all(px(2))
            } else {
                UiRect::ZERO
            },
            column_gap: if phosphor { px(2) } else { px(0) },
            ..default()
        },
        BorderColor::all(track_border),
        BackgroundColor(track_bg),
        Children::spawn(SpawnWith(move |parent: &mut RelatedSpawner<ChildOf>| {
            if phosphor {
                for i in 0..SLIDER_SEGMENTS {
                    parent.spawn((
                        SliderBlock(i),
                        Node {
                            flex_grow: 1.0,
                            height: percent(100),
                            border_radius: BorderRadius::all(px(1)),
                            ..default()
                        },
                        BackgroundColor(slider_meter_color(i, fraction)),
                    ));
                }
            } else {
                parent.spawn((
                    Node {
                        width: percent(fraction * 100.0),
                        height: percent(100.0),
                        border_radius: BorderRadius::all(px(radius)),
                        ..default()
                    },
                    BackgroundColor(theme::PHOSPHOR_DIM),
                ));
            }
        })),
    )
}

/// The bordered/recessed container of a segmented control. Spawn
/// [`segmented_option`]s into it, pairing each with a `ButtonValue<T>` (and
/// `Selected` on the active one) for a functional settings row, or use the
/// display-only [`segmented`] convenience.
pub fn segmented_container(skin: UiSkin) -> impl Bundle {
    let phosphor = skin.is_phosphor();
    let (bg, border) = if phosphor {
        (
            Color::srgba(0.0, 0.0, 0.0, 0.35),
            theme::PHOSPHOR.with_alpha(0.25),
        )
    } else {
        (Color::srgba(0.0, 0.0, 0.0, 0.4), theme::CASE_EDGE)
    };
    (
        Node {
            flex_direction: FlexDirection::Row,
            column_gap: px(3),
            padding: UiRect::all(px(3)),
            border: UiRect::all(px(theme::BORDER_W)),
            border_radius: BorderRadius::all(px(if phosphor { theme::RADIUS } else { 7.0 })),
            align_self: AlignSelf::Start,
            ..default()
        },
        BorderColor::all(border),
        BackgroundColor(bg),
    )
}

/// One option of a segmented control: a segment-sized ghost `ThemedButton`. The
/// caller adds a `ButtonValue<T>` (+ `Selected` on the active option) so the
/// shared `button_on_setting::<T>` observer drives the setting on click.
pub fn segmented_option(label: &str) -> impl Bundle {
    let mut spec = ButtonSpec::new(label);
    spec.variant = ButtonVariant::Ghost;
    spec.min_height = 28.0;
    spec.font_size = 12.0;
    button(spec)
}

/// A display-only segmented control: a [`segmented_container`] of
/// [`segmented_option`]s with `active` selected. For a FUNCTIONAL settings row,
/// build the container yourself and add `ButtonValue<T>` to each option.
pub fn segmented(options: &[&str], active: usize, skin: UiSkin) -> impl Bundle {
    let options: Vec<String> = options.iter().map(|s| s.to_string()).collect();
    (
        segmented_container(skin),
        Children::spawn(SpawnWith(move |parent: &mut RelatedSpawner<ChildOf>| {
            for (i, opt) in options.iter().enumerate() {
                let mut ent = parent.spawn(segmented_option(opt));
                if i == active {
                    ent.insert(Selected);
                }
            }
        })),
    )
}

/// Light the phosphor block-meter of any `Slider` wearing a [`slider_track`]:
/// when a `SliderValue` changes, recolour its [`SliderBlock`] children to the
/// new fraction. Registered by [`register`], so a `bevy_ui_widgets::Slider` +
/// `slider_track` visual gets the reactive meter for free (settings volume, the
/// widget_zoo). A slider whose track is the hardware solid fill has no
/// `SliderBlock` children, so this is a no-op for it.
fn sync_slider_meters(
    changed: Query<(&Children, &SliderValue), Changed<SliderValue>>,
    mut blocks: Query<(&SliderBlock, &mut BackgroundColor)>,
) {
    for (kids, value) in &changed {
        for &child in kids {
            if let Ok((block, mut bg)) = blocks.get_mut(child) {
                *bg = slider_meter_color(block.0, value.0).into();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A headless app with the widget observers + skin reconciler registered and
    /// the `Update` schedule available, so `app.update()` drives the live paint.
    fn skin_app(skin: UiSkin) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        register(&mut app);
        app.insert_resource(skin);
        app
    }

    fn only_button(app: &mut App) -> Entity {
        let mut q = app
            .world_mut()
            .query_filtered::<Entity, With<ThemedButton>>();
        q.iter(app.world())
            .next()
            .expect("a ThemedButton is spawned")
    }

    fn bg(app: &App, entity: Entity) -> Color {
        app.world()
            .entity(entity)
            .get::<BackgroundColor>()
            .unwrap()
            .0
    }

    fn has_gradient(app: &App, entity: Entity) -> bool {
        app.world().entity(entity).contains::<BackgroundGradient>()
    }

    fn block_colors(app: &mut App, slider: Entity) -> Vec<Color> {
        let kids: Vec<Entity> = app
            .world()
            .entity(slider)
            .get::<Children>()
            .unwrap()
            .iter()
            .collect();
        let mut q = app
            .world_mut()
            .query_filtered::<&BackgroundColor, With<SliderBlock>>();
        kids.into_iter()
            .filter_map(|c| q.get(app.world(), c).ok().map(|bg| bg.0))
            .collect()
    }

    /// DoD 1 (task 20260729-105359): `sync_slider_meters` lights a slider's
    /// phosphor block-meter from its `SliderValue` - so a `Slider` wearing
    /// `slider_track` reacts to drags with no per-site code. Fails if the system
    /// is unregistered or the block recolour is wrong.
    #[test]
    fn sync_slider_meters_lights_blocks_from_value() {
        let mut app = skin_app(UiSkin::Phosphor);
        let slider = app
            .world_mut()
            .spawn((SliderValue(0.0), slider_track(0.0, UiSkin::Phosphor)))
            .id();
        app.update();
        assert!(
            block_colors(&mut app, slider)
                .iter()
                .all(|c| *c == theme::PHOSPHOR.with_alpha(0.16)),
            "every bar is dim at value 0"
        );

        // Raise the value; `SliderValue` is immutable, so it is re-inserted (as
        // the real widget does), which fires `Changed` and lights the bars.
        app.world_mut().entity_mut(slider).insert(SliderValue(1.0));
        app.update();
        assert!(
            block_colors(&mut app, slider)
                .iter()
                .all(|c| *c == theme::PHOSPHOR),
            "every bar is lit at value 1.0"
        );
    }

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

    /// The phosphor skin renders a button as a CLI element: a flat
    /// phosphor-tinted fill, a 1px phosphor border and NO bevel gradient/shadow -
    /// and selection INVERTS (solid phosphor fill, dark ink glyphs). Pins the
    /// state table so a future tweak that reintroduces a gradient on the phosphor
    /// skin (a bevelled button on glass) fails here.
    #[test]
    fn phosphor_button_states_render_cli_markers() {
        let mut app = skin_app(UiSkin::Phosphor);
        let btn = app.world_mut().spawn(button(ButtonSpec::new("Go"))).id();
        app.update();

        // Idle: flat fill + phosphor border, no bevel.
        assert_eq!(bg(&app, btn), theme::PHOSPHOR.with_alpha(0.05));
        assert_eq!(
            app.world().entity(btn).get::<BorderColor>().unwrap().top,
            theme::PHOSPHOR.with_alpha(0.4)
        );
        assert!(
            !has_gradient(&app, btn),
            "phosphor is a flat CLI element, not a bevel"
        );
        assert!(!app.world().entity(btn).contains::<BoxShadow>());
        assert_eq!(label_color(&mut app, btn), theme::PHOSPHOR);

        // Selected inverts: solid phosphor fill, ink glyphs.
        app.world_mut().entity_mut(btn).insert(Selected);
        app.update();
        assert_eq!(bg(&app, btn), theme::PHOSPHOR);
        assert_eq!(label_color(&mut app, btn), theme::INK);
        assert!(!has_gradient(&app, btn), "inverted phosphor is still flat");
    }

    /// The hardware skin renders a button as a moulded control: a case-gradient
    /// face + a drop shadow + soft (7px) corners. Pins the bevel so a regression
    /// that drops the gradient (flat button on the hardware skin) fails here.
    #[test]
    fn hardware_button_states_render_bevel() {
        let mut app = skin_app(UiSkin::Hardware);
        let btn = app.world_mut().spawn(button(ButtonSpec::new("Go"))).id();
        app.update();

        assert!(has_gradient(&app, btn), "hardware face is a gradient bevel");
        assert!(
            app.world().entity(btn).contains::<BoxShadow>(),
            "hardware has depth"
        );
        assert_eq!(
            app.world().entity(btn).get::<BorderColor>().unwrap().top,
            theme::CASE_EDGE
        );
        assert_eq!(
            app.world().entity(btn).get::<Node>().unwrap().border_radius,
            BorderRadius::all(px(theme::RADIUS_HW))
        );

        // Selected -> amber gradient, inverted ink glyphs (still a bevel).
        app.world_mut().entity_mut(btn).insert(Selected);
        app.update();
        assert!(has_gradient(&app, btn));
        assert_eq!(label_color(&mut app, btn), theme::INK);
    }

    /// Flipping the `UiSkin` resource restyles buttons ALREADY in the tree, and a
    /// button spawned on a frame with NO skin change is still painted for the
    /// current skin (the `Added<ThemedButton>` override). This second half FAILS
    /// with only the `skin.is_changed()` path wired, which is the whole point of
    /// the override (lesson mode-keyed-reconciler-just-spawned-override).
    #[test]
    fn skin_switch_restyles_spawned_widgets() {
        let mut app = skin_app(UiSkin::Phosphor);
        let first = app.world_mut().spawn(button(ButtonSpec::new("First"))).id();
        app.update();
        assert!(!has_gradient(&app, first), "phosphor: flat");

        // Flip to hardware: the LIVE button restyles to a bevel.
        *app.world_mut().resource_mut::<UiSkin>() = UiSkin::Hardware;
        app.update();
        assert!(
            has_gradient(&app, first),
            "live button restyled to hardware bevel"
        );

        // A button spawned now - a frame with NO skin change - must still be
        // painted hardware by the just-spawned override.
        let second = app
            .world_mut()
            .spawn(button(ButtonSpec::new("Second")))
            .id();
        app.update();
        assert!(
            has_gradient(&app, second),
            "just-spawned button painted for the current skin without a skin change"
        );
        let _ = only_button; // keep the helper referenced for future single-button tests
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
