//! The themed button: paint model, interaction observers, live-skin
//! reconciler and the [`ButtonSpec`] build recipe.
//!
//! The visual is a pure function of `(skin, variant, state)` - see
//! [`button_paint`] - applied both by the per-interaction observers
//! (hover/press/disable/select) and by [`reconcile_button_skins`], the system
//! that restyles LIVE buttons when the `UiSkin` resource flips or a new button
//! is spawned.

use bevy::{
    ecs::relationship::RelatedSpawner,
    picking::hover::Hovered,
    platform::collections::HashSet,
    prelude::*,
    reflect::Is,
    ui::{InteractionDisabled, Pressed},
    ui_widgets::{Activate, Button},
};

use super::{
    paint::{drop_shadow, glow_shadow, grad2, grad3},
    Selected, UiText,
};
use crate::{skin::UiSkin, theme};

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
/// The value a settings button represents. Kept distinct from the `T` resource so a
/// button can carry a choice without being interpreted as - and clobbering - the resource
/// itself: on Bevy 0.19 a `#[derive(Resource)]` type is component-backed, so putting it on
/// a button entity is treated as a resource insert.
#[derive(Component, Debug, Clone)]
pub struct ButtonValue<T>(pub T);
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
        // Pressed dims the lit face (and drops the glow below): an inverted
        // button is already at full phosphor, so sinking is the only move left.
        if pressed {
            (theme::PHOSPHOR_LO, p, theme::INK)
        } else {
            (p, p, theme::INK)
        }
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
                if pressed {
                    (p.with_alpha(0.14), p, p)
                } else if hovered {
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
    // (which forbids a bevel GRADIENT) still holds. Pressing puts it out.
    let shadow = (!disabled && inverted && !pressed).then(|| glow_shadow(theme::PHOSPHOR));

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
    // Both sink the same way every other hardware face does: the bevel gradient
    // is inverted and the drop shadow goes away.
    if !disabled && selected {
        return Paint {
            bg: theme::AMBER_NOVA,
            border,
            text: theme::INK,
            radius,
            gradient: Some(if pressed {
                grad3(theme::AMBER_LO, theme::AMBER_NOVA, theme::AMBER_HI)
            } else {
                grad3(theme::AMBER_HI, theme::AMBER_NOVA, theme::AMBER_LO)
            }),
            shadow: (!pressed).then(|| glow_shadow(theme::AMBER_NOVA)),
            cursor_visible,
        };
    }
    if !disabled && matches!(variant, ButtonVariant::Primary) {
        return Paint {
            bg: theme::PHOSPHOR,
            border,
            text: theme::INK,
            radius,
            gradient: Some(if pressed {
                grad3(theme::PHOSPHOR_LO, theme::PHOSPHOR, theme::PHOSPHOR_HI)
            } else {
                grad3(theme::PHOSPHOR_HI, theme::PHOSPHOR, theme::PHOSPHOR_LO)
            }),
            shadow: (!pressed).then(|| glow_shadow(theme::PHOSPHOR)),
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
        // Ghost stays fill-less by contract, so the press cannot be a bevel: it
        // is a dark wash under the border instead.
        ButtonVariant::Ghost => Paint {
            bg: if pressed {
                Color::BLACK.with_alpha(0.22)
            } else if hovered {
                Color::WHITE.with_alpha(0.04)
            } else {
                Color::NONE
            },
            border: Color::WHITE.with_alpha(if pressed {
                0.3
            } else if hovered {
                0.22
            } else {
                0.12
            }),
            text: HW_TEXT,
            radius,
            gradient: None,
            shadow: None,
            cursor_visible,
        },
        ButtonVariant::Danger => {
            // Pressed is its OWN paint, like every other hardware variant:
            // sunk (no drop shadow) and darker. Collapsing it into `hovered`
            // left Exit with no press feedback on this skin only.
            if pressed || hovered {
                let lit = Color::srgb_u8(0x6b, 0x2a, 0x26);
                let dark = Color::srgb_u8(0x3a, 0x15, 0x12);
                Paint {
                    bg: theme::RED,
                    border,
                    text: Color::WHITE,
                    radius,
                    // Pressed inverts the gradient and drops the shadow: the
                    // face reads as sunk rather than raised.
                    gradient: Some(if pressed {
                        grad2(dark, lit)
                    } else {
                        grad2(lit, dark)
                    }),
                    shadow: (!pressed).then(drop_shadow),
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

    // NOTE: try_insert / try_remove, not insert / remove: a button can be despawned
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
pub(super) fn button_on_interaction<E: EntityEvent, C: Component>(
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
#[expect(
    clippy::type_complexity,
    reason = "one query term per button visual state"
)]
pub(super) fn reconcile_button_skins(
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
/// `reconcile_button_skins` on the frame it appears.
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
                // NOTE: no TextShadow. Bevy's `TextShadow` is a hard drop
                // shadow (no blur), so its default 4px black offset ghosts the
                // label on a bright/inverted fill instead of glowing. Crisp CLI
                // text needs no shadow.
            ));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget::fixtures::{bg, has_gradient, skin_app};

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
    }

    /// EVERY variant must give a press its own face, on BOTH skins. Two bugs
    /// live here: a variant that reacts on one skin only (the hardware Danger
    /// face - the Exit button - collapsed hover and press into one paint), and
    /// a variant that reacts on NEITHER (`Ghost`, which `segmented_option`
    /// builds, so the Graphics-preset and UI-skin rows had no press feedback at
    /// all). Parity alone would pass the second case, so assert both.
    #[test]
    fn press_reads_differently_from_hover_in_both_skins() {
        // The visually load-bearing parts of a Paint, comparable. The gradient
        // is compared by its STOPS, not its stop count: a variant that reacts
        // to press only through gradient colours must still read as reacting.
        let face = |skin, variant, hovered, pressed| {
            let p = button_paint(skin, variant, false, hovered, pressed, false);
            (
                format!("{:?}", p.bg),
                format!("{:?}", p.text),
                format!("{:?}", p.gradient),
                p.shadow.is_some(),
            )
        };

        let reacts_to_press =
            |skin, variant| face(skin, variant, true, false) != face(skin, variant, true, true);

        for variant in [
            ButtonVariant::Default,
            ButtonVariant::Primary,
            ButtonVariant::Danger,
            ButtonVariant::Ghost,
        ] {
            for skin in [UiSkin::Phosphor, UiSkin::Hardware] {
                assert!(
                    reacts_to_press(skin, variant),
                    "{variant:?} has no press feedback on {skin:?}"
                );
            }
            assert_eq!(
                reacts_to_press(UiSkin::Phosphor, variant),
                reacts_to_press(UiSkin::Hardware, variant),
                "{variant:?} gives press its own face on one skin but not the other"
            );
        }
    }

    // NOTE: `Resource` is component-backed in Bevy 0.19, so it also provides the
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
