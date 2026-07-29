//! The contextual keybind **dock** and the anchored verb cues.
//!
//! Nobody memorizes X/G/O/Z, and nobody reads a seven-row wall of
//! bracketed-key-plus-verb text while flying either. Both surfaces render from the input
//! layer's [`FlightVerbHints`] resource - availability and key labels are
//! resolved where the verbs live, this module is a dumb view.
//!
//! - **Dock** (this task, 20260728-175742): a single row of icon chips docked
//!   bottom-centre, one per flight verb, each a real keycap PICTURE (see
//!   [`super::key_glyphs`]) plus a short verb word. Three states read at a
//!   glance - dim (present, not actionable), available (full phosphor), hot
//!   (inverted, the verb is what the ship is doing). It replaces the lower-left
//!   text cluster, which carried the bulk of the HUD's on-screen text.
//! - **Anchored cues**: the hint sits on the thing you would act on - ORBIT
//!   projected on the dominant well, GOTO on the aim lock while no maneuver is
//!   engaged - now the same keycap chip rather than `[O] ORBIT` text.
//!
//! The scenario spotlight ([`HintEmphasis`], driven by the
//! `HintEmphasisSet`/`HintEmphasisClear` actions) is unchanged in meaning and
//! in its verb vocabulary: it now pulses a dock CHIP instead of a text row. The
//! tutorial scenarios address verbs by name and keep working untouched.

use bevy::{platform::collections::HashSet, prelude::*};
use nova_ui::hud as chip;

use super::{screen_indicator::prelude::*, NovaHudAssets, OBJECTIVE_GOLD};
use crate::input::prelude::*;

/// Glob-import surface: `use nova_gameplay::hud::keybind_dock::prelude::*` re-exports the public API of this module.
pub mod prelude {
    pub use super::{
        keybind_dock_hud, verb_cues_hud, DockChip, DockChipState, HintEmphasis, KeybindDockMarker,
        KeybindDockPlugin, VerbCuesHudMarker, DOCK_VERBS,
    };
}

/// The keycap picture's on-screen size (px) inside a chip - demo 2's `.ki`.
/// Square, because the ART is square: every PNG under
/// `assets/input-prompts/keyboard/Alt/` is a 128x128 canvas, and the wide caps
/// (Ctrl, Shift, Space) are drawn smaller INSIDE it rather than on a wider
/// canvas. So a wide cap's legend is small at this size by construction - that
/// is the art, not a squash, and the verb word beside it carries the meaning.
const GLYPH_PX: f32 = 22.0;

/// The (smaller) keycap size on an anchored cue, which sits over the world and
/// must not eclipse the thing it labels.
const CUE_GLYPH_PX: f32 = 20.0;

/// Chip text size (px) - the dock is a glance surface, the word is a reminder.
const CHIP_TEXT_PX: f32 = 11.0;

/// The cue sits below its object so it reads as a caption, not a lock.
const CUE_OFFSET: Vec2 = Vec2::new(0.0, 48.0);

/// The verb names, in dock display order (left to right). The component-cycle
/// chip documents the wheel gesture (task 20260708-165705): plain scroll steps
/// the component fine-lock; CTRL+scroll steps the ship lock through the tracked
/// candidates.
pub const DOCK_VERBS: [&str; 7] = [
    "STOP",
    "GOTO",
    "ORBIT",
    "CANCEL",
    "RADAR",
    "COMPONENT",
    "RCS",
];

/// Emphasis pulse rate and the alpha bands it sweeps. The emphasized chip
/// renders PURE OBJECTIVE_GOLD hue at all times and only its alpha pulses: the
/// first cut cross-mixed the availability color toward gold, and a lit chip's
/// phosphor->gold RGB path passes through a washed near-white blend every cycle
/// - unreadable at 11 px (playtest 2026-07-12, task 20260712-152340).
/// Availability still reads from the band: available chips sweep the bright
/// band, unavailable chips a dim one (spotlight, not a state change). ~1 Hz -
/// present in peripheral vision, not a strobe.
const EMPHASIS_PERIOD_SECS: f32 = 1.0;
const EMPHASIS_ALPHA_AVAILABLE: (f32, f32) = (0.7, 1.0);
const EMPHASIS_ALPHA_UNAVAILABLE: (f32, f32) = (0.3, 0.5);

/// Marker for the bottom-centre keybind dock root; spawned by
/// [`keybind_dock_hud`].
#[derive(Component, Debug, Clone, Reflect)]
pub struct KeybindDockMarker;

/// One dock chip; the index picks the verb (see [`DOCK_VERBS`] and
/// [`verb_hint`]). Public so range examples and screenshot rigs can assert on
/// the dock's contents.
#[derive(Component, Debug, Clone, Copy, Deref, DerefMut, Reflect)]
pub struct DockChip(pub usize);

/// A dock chip's rendered state, written every update from availability (and,
/// for `Hot`, from what the ship is actually doing). Public so tests and the
/// follow-up situational-emphasis task (20260728-175747) can read and drive it.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
pub enum DockChipState {
    /// The verb exists but pressing it now would do nothing.
    #[default]
    Dim,
    /// Pressing it now does something.
    Available,
    /// The verb is what the ship is doing right now (inverted chip).
    Hot,
}

/// The keycap picture inside a dock chip or cue.
#[derive(Component, Debug, Clone, Reflect)]
struct ChipGlyph;

/// The fallback key TEXT inside a chip, shown only when the bound key has no
/// keycap art (an exotic rebind) - so the dock degrades instead of rendering an
/// empty box.
#[derive(Component, Debug, Clone, Reflect)]
struct ChipKeyText;

/// The verb word inside a dock chip.
#[derive(Component, Debug, Clone, Reflect)]
struct ChipLabel;

/// The verbs the scenario wants the player's eyes on (task 20260712-093831):
/// names from [`DOCK_VERBS`], set/cleared by the `HintEmphasisSet`/
/// `HintEmphasisClear` scenario actions and cleared wholesale on scenario
/// teardown. Emphasis is a SPOTLIGHT, not a state change - it never alters
/// availability; an unavailable chip pulses from dim, still clearly "not yet".
#[derive(Resource, Debug, Clone, Default, Reflect)]
#[reflect(Resource)]
pub struct HintEmphasis {
    verbs: HashSet<String>,
}

impl HintEmphasis {
    /// Emphasize `verb` (a [`DOCK_VERBS`] name). Unknown verbs are refused with
    /// a warning - a typo in scenario data should be loud, not a silently dead
    /// handler.
    pub fn set(&mut self, verb: &str) -> bool {
        if !DOCK_VERBS.contains(&verb) {
            warn!(
                "HintEmphasis: '{}' is not a dock verb (chips: {:?})",
                verb, DOCK_VERBS
            );
            return false;
        }
        self.verbs.insert(verb.to_string());
        true
    }

    /// Drop the emphasis on `verb` (no-op when it was not emphasized).
    pub fn clear(&mut self, verb: &str) {
        self.verbs.remove(verb);
    }

    /// Drop every emphasis (scenario teardown).
    pub fn clear_all(&mut self) {
        self.verbs.clear();
    }

    /// Whether `verb` is currently emphasized.
    pub fn contains(&self, verb: &str) -> bool {
        self.verbs.contains(verb)
    }

    /// Whether nothing is emphasized (the pulse system's early-out).
    pub fn is_empty(&self) -> bool {
        self.verbs.is_empty()
    }
}

/// Marker for the anchored verb-cue layer (the orbit and goto cue chips
/// projected on their objects); spawned by [`verb_cues_hud`].
#[derive(Component, Debug, Clone, Reflect)]
pub struct VerbCuesHudMarker;

/// Marker for the orbit cue chip (anchored on the dominant well).
#[derive(Component, Debug, Clone, Reflect)]
struct OrbitCueUIMarker;

/// Marker for the goto cue chip (anchored on the aim lock).
#[derive(Component, Debug, Clone, Reflect)]
struct GotoCueUIMarker;

/// The paint a chip carries in `state`: fill, border, text. `Hot` inverts -
/// phosphor slab, dark ink text - which is why it reads instantly at the edge
/// of vision (demo 2 `.vchip.hot`).
fn chip_paint(state: DockChipState) -> (Color, Color, Color) {
    match state {
        DockChipState::Dim => (
            chip::CHIP_FILL_QUIET.with_alpha(0.28),
            nova_ui::theme::PHOSPHOR.with_alpha(0.16),
            nova_ui::theme::PHOSPHOR_MUTED.with_alpha(0.55),
        ),
        DockChipState::Available => (
            chip::CHIP_FILL,
            nova_ui::theme::PHOSPHOR.with_alpha(0.4),
            nova_ui::theme::PHOSPHOR,
        ),
        DockChipState::Hot => (
            nova_ui::theme::PHOSPHOR,
            nova_ui::theme::PHOSPHOR,
            nova_ui::theme::INK,
        ),
    }
}

/// The keycap image tint for `state` - Bevy UI has no grayscale filter, so the
/// dim state is alpha-dimming (spike 20260728-175742 note); the dark keycaps
/// with white glyphs read well over the phosphor HUD either way.
fn glyph_tint(state: DockChipState) -> Color {
    match state {
        DockChipState::Dim => Color::WHITE.with_alpha(0.45),
        _ => Color::WHITE,
    }
}

/// UI bundle for the keybind dock: one row of icon chips, bottom-centre (demo 2
/// `.dock`).
///
/// The chips spawn EMPTY - no keycap, no key text, collapsed. [`update_dock`]
/// fills them, and its `Added<DockChip>` gate makes that happen on the same
/// frame they appear. Resolving the art here as well would mean guessing each
/// verb's key at spawn time, and the binding is only knowable from the live
/// [`FlightVerbHints`] the update system already reads.
pub fn keybind_dock_hud() -> impl Bundle {
    let chip_of = |index: usize| {
        let (fill, border, text) = chip_paint(DockChipState::Dim);
        (
            DockChip(index),
            DockChipState::Dim,
            Node {
                display: Display::None,
                align_items: AlignItems::Center,
                column_gap: Val::Px(6.0),
                padding: UiRect::new(Val::Px(4.0), Val::Px(8.0), Val::Px(3.0), Val::Px(3.0)),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(fill),
            BorderColor::all(border),
            Pickable::IGNORE,
            children![
                (
                    ChipGlyph,
                    ImageNode::default(),
                    Node {
                        display: Display::None,
                        width: Val::Px(GLYPH_PX),
                        height: Val::Px(GLYPH_PX),
                        ..default()
                    },
                    Pickable::IGNORE,
                ),
                (
                    ChipKeyText,
                    Text::new(""),
                    TextFont::from_font_size(CHIP_TEXT_PX),
                    TextColor(text),
                    Node {
                        display: Display::None,
                        ..default()
                    },
                ),
                (
                    ChipLabel,
                    Text::new(DOCK_VERBS[index]),
                    TextFont::from_font_size(CHIP_TEXT_PX),
                    TextColor(text),
                ),
            ],
        )
    };

    (
        Name::new("KeybindDockHUD"),
        KeybindDockMarker,
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(14.0),
            left: Val::Px(0.0),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            column_gap: Val::Px(7.0),
            ..default()
        },
        Pickable::IGNORE,
        children![
            chip_of(0),
            chip_of(1),
            chip_of(2),
            chip_of(3),
            chip_of(4),
            chip_of(5),
            chip_of(6),
        ],
    )
}

/// UI bundle for the anchored cues: one indicator layer with the orbit and goto
/// keycap chips. Each cue hugs its content ([`ScreenIndicatorSize::Content`]) so
/// a keycap + word chip is never clipped into a fixed box.
///
/// Like the dock, a cue spawns with an EMPTY key visual: its drive system fills
/// the keycap (or the text fallback) from the LIVE binding every update, so a
/// remap moves the cue with it and a cue spawned before the glyph collection
/// finished loading still picks its picture up.
pub fn verb_cues_hud() -> impl Bundle {
    let cue = |label: &'static str| {
        (
            screen_indicator_node(
                ScreenIndicatorConfig {
                    anchor: None,
                    size: ScreenIndicatorSize::Content,
                    offset: CUE_OFFSET,
                    offscreen: ScreenIndicatorOffscreen::Hide,
                },
                Node {
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(6.0),
                    padding: UiRect::new(Val::Px(4.0), Val::Px(8.0), Val::Px(3.0), Val::Px(3.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    border_radius: BorderRadius::all(Val::Px(chip::CHIP_RADIUS)),
                    ..default()
                },
            ),
            BackgroundColor(chip::CHIP_FILL),
            BorderColor::all(nova_ui::theme::PHOSPHOR.with_alpha(0.4)),
            children![
                (
                    ChipGlyph,
                    ImageNode::default(),
                    Node {
                        display: Display::None,
                        width: Val::Px(CUE_GLYPH_PX),
                        height: Val::Px(CUE_GLYPH_PX),
                        ..default()
                    },
                    Pickable::IGNORE,
                ),
                (
                    ChipKeyText,
                    Text::new(""),
                    TextFont::from_font_size(CHIP_TEXT_PX),
                    TextColor(nova_ui::theme::PHOSPHOR),
                    Node {
                        display: Display::None,
                        ..default()
                    },
                ),
                (
                    ChipLabel,
                    Text::new(label),
                    TextFont::from_font_size(CHIP_TEXT_PX),
                    TextColor(nova_ui::theme::PHOSPHOR),
                ),
            ],
        )
    };

    (
        Name::new("VerbCuesHUD"),
        VerbCuesHudMarker,
        screen_indicator_layer(),
        children![
            (Name::new("OrbitCueUI"), OrbitCueUIMarker, cue("ORBIT")),
            (Name::new("GotoCueUI"), GotoCueUIMarker, cue("GOTO")),
        ],
    )
}

/// Drives the contextual keybind surfaces: the bottom-centre dock (with the
/// scenario-driven emphasis pulse) and the orbit/goto anchored cues, all a dumb
/// view over the input layer's [`FlightVerbHints`].
/// Inits/registers [`HintEmphasis`] and runs `update_dock`,
/// `pulse_emphasized_chips` (after it), `drive_orbit_cue` and `drive_goto_cue`
/// in Update within [`super::NovaHudSystems`].
#[derive(Default)]
pub struct KeybindDockPlugin;

impl Plugin for KeybindDockPlugin {
    fn build(&self, app: &mut App) {
        debug!("KeybindDockPlugin: build");

        app.init_resource::<HintEmphasis>();
        app.register_type::<HintEmphasis>();
        app.register_type::<DockChip>();
        app.register_type::<DockChipState>();

        app.add_systems(
            Update,
            (
                update_dock,
                // The pulse layers OVER the availability paint, so it must run
                // downstream of the writer inside the frame.
                pulse_emphasized_chips.after(update_dock),
                (drive_orbit_cue, drive_goto_cue),
            )
                .in_set(super::NovaHudSystems),
        );
    }
}

/// The hint behind dock chip `index`.
fn verb_hint(hints: &FlightVerbHints, index: usize) -> &VerbHint {
    match index {
        0 => &hints.stop,
        1 => &hints.goto,
        2 => &hints.orbit,
        3 => &hints.cancel,
        4 => &hints.radar,
        5 => &hints.component_cycle,
        _ => &hints.rcs,
    }
}

/// The state chip `index` should render.
///
/// `Hot` is deliberately thin here: the only live "the ship is doing this"
/// truth the input layer publishes is `engaged`, which makes CANCEL hot while a
/// maneuver runs. Task 20260728-175747 layers the situational emphasis (a hot
/// chip the moment its verb becomes the thing to press) on top of this exact
/// component - the look is wired and proven, the driving is that task's.
fn chip_state(hints: &FlightVerbHints, index: usize) -> DockChipState {
    let hint = verb_hint(hints, index);
    if !hint.available {
        return DockChipState::Dim;
    }
    if DOCK_VERBS[index] == "CANCEL" && hints.engaged {
        return DockChipState::Hot;
    }
    DockChipState::Available
}

/// Point a chip's KEY visual at `key`: the keycap picture when the bound key has
/// art, the bare key text when it does not - so a rebind to an exotic key
/// degrades to a readable chip instead of rendering an empty box.
///
/// Shared by the dock and the anchored cues (generic over their query filters,
/// which differ only in the `Without` terms each system needs to stay disjoint
/// from its own root query).
fn paint_key_visual<FG, FT>(
    key: &str,
    glyph: Option<Handle<Image>>,
    tint: Color,
    children: &Children,
    q_glyph: &mut Query<(&mut ImageNode, &mut Node), FG>,
    q_key_text: &mut Query<(&mut Text, &mut Node), FT>,
) where
    FG: bevy::ecs::query::QueryFilter,
    FT: bevy::ecs::query::QueryFilter,
{
    for &child in children {
        if let Ok((mut image, mut node)) = q_glyph.get_mut(child) {
            let display = if glyph.is_some() {
                Display::Flex
            } else {
                Display::None
            };
            if node.display != display {
                node.display = display;
            }
            if let Some(handle) = glyph.clone() {
                if image.image != handle {
                    image.image = handle;
                }
            }
            image.color = tint;
        }
        if let Ok((mut text, mut node)) = q_key_text.get_mut(child) {
            let display = if glyph.is_some() {
                Display::None
            } else {
                Display::Flex
            };
            if node.display != display {
                node.display = display;
            }
            if glyph.is_none() && **text != key {
                **text = key.to_string();
            }
        }
    }
}

/// Paint every dock chip from availability. Unlike the text cluster it replaced,
/// a dim chip STAYS on screen: an icon row is small enough to read as one
/// instrument, and the constant positions are what make it glanceable (the old
/// cluster hid unavailable rows because seven greyed text lines were noise).
/// A chip whose key is empty - no flight rig - is still hidden entirely: no rig,
/// no keys, no dock, as always.
#[expect(clippy::type_complexity, reason = "one query per chip part")]
fn update_dock(
    hints: Res<FlightVerbHints>,
    assets: Option<Res<NovaHudAssets>>,
    mut q_chip: Query<(
        &DockChip,
        &mut DockChipState,
        &mut Node,
        &mut BackgroundColor,
        &mut BorderColor,
        &Children,
    )>,
    mut q_glyph: Query<(&mut ImageNode, &mut Node), (With<ChipGlyph>, Without<DockChip>)>,
    mut q_key_text: Query<
        (&mut Text, &mut Node),
        (With<ChipKeyText>, Without<DockChip>, Without<ChipGlyph>),
    >,
    mut q_text_color: Query<&mut TextColor, Or<(With<ChipLabel>, With<ChipKeyText>)>>,
    q_added: Query<(), Added<DockChip>>,
) {
    // Skip quiet frames, but never skip freshly spawned chips (a respawned HUD
    // may appear while the resource is unchanged), nor the frame the glyph
    // collection finishes loading (chips spawned before it must pick it up).
    let assets_changed = assets.as_ref().is_some_and(|a| a.is_changed());
    if !hints.is_changed() && !assets_changed && q_added.is_empty() {
        return;
    }
    let glyphs = assets.as_ref().map(|a| &a.key_glyphs);

    for (chip, mut state, mut node, mut fill, mut border, children) in &mut q_chip {
        let hint = verb_hint(&hints, **chip);
        let shown = !hint.key.is_empty();
        let display = if shown { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
        let next = chip_state(&hints, **chip);
        if !shown {
            // A hidden chip still tracks its state, so a pulse that was
            // running when the rig despawned restores the RIGHT base.
            state.set_if_neq(DockChipState::Dim);
            continue;
        }

        state.set_if_neq(next);
        let (fill_color, border_color, text_color) = chip_paint(next);
        fill.set_if_neq(BackgroundColor(fill_color));
        border.set_if_neq(BorderColor::all(border_color));

        let glyph = glyphs.and_then(|glyphs| glyphs.get(&hint.key));
        paint_key_visual(
            &hint.key,
            glyph,
            glyph_tint(next),
            children,
            &mut q_glyph,
            &mut q_key_text,
        );
        for &child in children {
            if let Ok(mut color) = q_text_color.get_mut(child) {
                color.set_if_neq(TextColor(text_color));
            }
        }
    }
}

/// The availability border and label colors [`update_dock`] gives a chip - the
/// base the emphasis pulse departs from and returns to.
fn base_colors(state: DockChipState) -> (Color, Color) {
    let (_, border, text) = chip_paint(state);
    (border, text)
}

/// The emphasized chip's color at `wave` in [0,1]: pure gold hue, alpha swept
/// over the availability band - never a cross-hue blend (see the EMPHASIS_*
/// docs).
fn emphasis_color(available: bool, wave: f32) -> Color {
    let (lo, hi) = if available {
        EMPHASIS_ALPHA_AVAILABLE
    } else {
        EMPHASIS_ALPHA_UNAVAILABLE
    };
    OBJECTIVE_GOLD.with_alpha(lo + (hi - lo) * wave)
}

/// Pulse the emphasized chips in objective gold (~1 Hz alpha breath) - the
/// chip's border and its label both, so the spotlight is visible whether the
/// player's eye lands on the keycap or the word. Runs after [`update_dock`] so
/// the availability paint stays the base; on any emphasis change every chip's
/// base is restored first, so a cleared emphasis cannot leave a chip stuck
/// mid-pulse.
#[expect(clippy::type_complexity, reason = "chip and its label text")]
fn pulse_emphasized_chips(
    time: Res<Time>,
    emphasis: Res<HintEmphasis>,
    hints: Res<FlightVerbHints>,
    mut q_chip: Query<(&DockChip, &DockChipState, &mut BorderColor, &Children)>,
    mut q_label: Query<&mut TextColor, With<ChipLabel>>,
) {
    if emphasis.is_empty() && !emphasis.is_changed() {
        return;
    }

    let t = time.elapsed_secs() * std::f32::consts::TAU / EMPHASIS_PERIOD_SECS;
    let wave = 0.5 + 0.5 * t.sin();

    for (chip, state, mut border, children) in &mut q_chip {
        let hint = verb_hint(&hints, **chip);
        let verb = DOCK_VERBS[(**chip).min(DOCK_VERBS.len() - 1)];
        // Key-empty chips (no flight rig) are hidden and never pulse, but they
        // still take the restore below - a chip whose key empties MID-pulse
        // (rig despawn) must not keep its gold, and the key emptying is a HINTS
        // change, not an emphasis change (review R1.4 of 20260712-093831).
        let (next_border, next_label) = if !hint.key.is_empty() && emphasis.contains(verb) {
            let gold = emphasis_color(hint.available, wave);
            (gold, gold)
        } else if emphasis.is_changed() || hints.is_changed() {
            // Restore the base exactly once per change; steady state leaves
            // unemphasized chips to update_dock (whose own write is identical,
            // so the diffed writes below are no-ops).
            base_colors(*state)
        } else {
            continue;
        };
        border.set_if_neq(BorderColor::all(next_border));
        for &child in children {
            if let Ok(mut color) = q_label.get_mut(child) {
                color.set_if_neq(TextColor(next_label));
            }
        }
    }
}

/// The ORBIT keycap on the dominant well while parking is on offer (the
/// resolver already retires the offer while orbiting).
#[expect(clippy::type_complexity, reason = "the cue root and its two key parts")]
fn drive_orbit_cue(
    hints: Res<FlightVerbHints>,
    assets: Option<Res<NovaHudAssets>>,
    mut q_ui: Query<(&mut ScreenIndicatorAnchor, &mut Node, &Children), With<OrbitCueUIMarker>>,
    mut q_glyph: Query<(&mut ImageNode, &mut Node), (With<ChipGlyph>, Without<OrbitCueUIMarker>)>,
    mut q_key_text: Query<
        (&mut Text, &mut Node),
        (
            With<ChipKeyText>,
            Without<ChipGlyph>,
            Without<OrbitCueUIMarker>,
        ),
    >,
    q_added: Query<(), Added<OrbitCueUIMarker>>,
) {
    // Same guard shape as the dock: quiet frames write nothing, but a freshly
    // spawned cue - or the frame the glyph collection lands - always runs.
    let assets_changed = assets.as_ref().is_some_and(|a| a.is_changed());
    if !hints.is_changed() && !assets_changed && q_added.is_empty() {
        return;
    }
    let glyph = assets
        .as_ref()
        .and_then(|assets| assets.key_glyphs.get(&hints.orbit.key));
    for (mut anchor, mut node, children) in &mut q_ui {
        match (hints.orbit.available, hints.orbit.anchor) {
            (true, Some(well)) => {
                **anchor = Some(ScreenIndicatorAnchorKind::Entity(well));
                node.display = Display::Flex;
            }
            _ => {
                **anchor = None;
                node.display = Display::None;
            }
        }
        // Read from the LIVE binding, so a remap moves the cue's keycap with
        // it (the text cue this replaced formatted `hints.orbit.key` the same
        // way).
        paint_key_visual(
            &hints.orbit.key,
            glyph.clone(),
            Color::WHITE,
            children,
            &mut q_glyph,
            &mut q_key_text,
        );
    }
}

/// The GOTO keycap on the aim lock while nothing is engaged - once a maneuver
/// runs the destination marker takes over and the cue would be noise.
#[expect(clippy::type_complexity, reason = "the cue root and its two key parts")]
fn drive_goto_cue(
    hints: Res<FlightVerbHints>,
    assets: Option<Res<NovaHudAssets>>,
    mut q_ui: Query<(&mut ScreenIndicatorAnchor, &mut Node, &Children), With<GotoCueUIMarker>>,
    mut q_glyph: Query<(&mut ImageNode, &mut Node), (With<ChipGlyph>, Without<GotoCueUIMarker>)>,
    mut q_key_text: Query<
        (&mut Text, &mut Node),
        (
            With<ChipKeyText>,
            Without<ChipGlyph>,
            Without<GotoCueUIMarker>,
        ),
    >,
    q_added: Query<(), Added<GotoCueUIMarker>>,
) {
    let assets_changed = assets.as_ref().is_some_and(|a| a.is_changed());
    if !hints.is_changed() && !assets_changed && q_added.is_empty() {
        return;
    }
    let glyph = assets
        .as_ref()
        .and_then(|assets| assets.key_glyphs.get(&hints.goto.key));
    for (mut anchor, mut node, children) in &mut q_ui {
        match (hints.goto.available && !hints.engaged, hints.goto.anchor) {
            (true, Some(lock)) => {
                **anchor = Some(ScreenIndicatorAnchorKind::Entity(lock));
                node.display = Display::Flex;
            }
            _ => {
                **anchor = None;
                node.display = Display::None;
            }
        }
        paint_key_visual(
            &hints.goto.key,
            glyph.clone(),
            Color::WHITE,
            children,
            &mut q_glyph,
            &mut q_key_text,
        );
    }
}

#[cfg(test)]
mod tests {
    use bevy::{asset::AssetPlugin, ecs::system::RunSystemOnce};

    use super::*;
    use crate::hud::key_glyphs::{key_glyph_stem, KeyGlyphs, KEY_GLYPH_DIR, KEY_GLYPH_FILES};

    fn hints(orbit_available: bool, engaged: bool, well: Option<Entity>) -> FlightVerbHints {
        FlightVerbHints {
            stop: VerbHint {
                key: "X".into(),
                available: true,
                anchor: None,
            },
            goto: VerbHint {
                key: "G".into(),
                available: false,
                anchor: None,
            },
            orbit: VerbHint {
                key: "O".into(),
                available: orbit_available,
                anchor: well,
            },
            cancel: VerbHint {
                key: "Z".into(),
                available: engaged,
                anchor: None,
            },
            component_cycle: VerbHint {
                key: "SCROLL".into(),
                available: false,
                anchor: None,
            },
            radar: VerbHint {
                key: "CTRL".into(),
                available: true,
                anchor: None,
            },
            rcs: VerbHint {
                key: "SHIFT".into(),
                available: false,
                anchor: None,
            },
            engaged,
        }
    }

    /// An app with the asset machinery the keycap handles need. Copied from the
    /// nearest passing sibling rig shape (lesson `reuse-known-good-stack`): a
    /// bare `World` panics in the AssetServer the moment a handle is resolved.
    fn glyph_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Image>();
        app
    }

    /// The REAL preloaded lookup, built the way `nova_assets` builds it: every
    /// mapped stem resolved through the asset server at its real path.
    fn real_glyphs(app: &App) -> KeyGlyphs {
        let server = app.world().resource::<AssetServer>().clone();
        KeyGlyphs::from_stems(|stem| Some(server.load(format!("{KEY_GLYPH_DIR}/{stem}.png"))))
    }

    fn glyph_path(app: &App, chip: Entity) -> Option<String> {
        let children = app.world().entity(chip).get::<Children>()?.to_vec();
        children.into_iter().find_map(|child| {
            let entity = app.world().entity(child);
            let image = entity.get::<ImageNode>()?;
            entity.get::<Node>()?;
            if entity.get::<ChipGlyph>().is_none() {
                return None;
            }
            Some(image.image.path()?.to_string())
        })
    }

    fn chips(app: &App) -> Vec<Entity> {
        let dock = app
            .world()
            .iter_entities()
            .find(|e| e.contains::<KeybindDockMarker>())
            .expect("a dock");
        dock.get::<Children>().unwrap().to_vec()
    }

    /// DoD 1: the dock renders ONE chip per verb, each carrying the keycap
    /// picture its live binding maps to and a state marker driven by
    /// availability. A no-op dock (chips with no glyph, or every chip in the
    /// same state) fails here.
    #[test]
    fn dock_renders_glyph_chip_per_verb_with_availability() {
        let mut app = glyph_app();
        app.init_resource::<HintEmphasis>();
        app.insert_resource(hints(true, false, None));
        app.add_systems(Update, update_dock);
        let glyphs = real_glyphs(&app);
        // The dock spawns with the preloaded glyphs, exactly like the HUD
        // observer spawns it from NovaHudAssets.
        app.world_mut().spawn(keybind_dock_hud());
        // NovaHudAssets is what the update system re-reads each frame.
        app.insert_resource(NovaHudAssets {
            key_glyphs: glyphs,
            ..default()
        });
        app.update();

        let chips = chips(&app);
        assert_eq!(chips.len(), DOCK_VERBS.len(), "one chip per verb");

        let expected_key = ["X", "G", "O", "Z", "CTRL", "SCROLL", "SHIFT"];
        for (index, verb) in DOCK_VERBS.iter().enumerate() {
            let stem = key_glyph_stem(expected_key[index]).unwrap();
            assert_eq!(
                glyph_path(&app, chips[index]).as_deref(),
                Some(format!("{KEY_GLYPH_DIR}/{stem}.png").as_str()),
                "{verb} draws the keycap its binding maps to"
            );
            assert_eq!(
                app.world()
                    .entity(chips[index])
                    .get::<Node>()
                    .unwrap()
                    .display,
                Display::Flex,
                "{verb} is docked (dim chips stay on screen)"
            );
        }

        // Availability drives the state marker, per verb.
        let state = |app: &App, index: usize| {
            *app.world()
                .entity(chips[index])
                .get::<DockChipState>()
                .unwrap()
        };
        assert_eq!(
            state(&app, 0),
            DockChipState::Available,
            "STOP is actionable"
        );
        assert_eq!(
            state(&app, 2),
            DockChipState::Available,
            "ORBIT is on offer"
        );
        assert_eq!(state(&app, 1), DockChipState::Dim, "GOTO has no lock");
        assert_eq!(
            state(&app, 3),
            DockChipState::Dim,
            "CANCEL: nothing engaged"
        );
        assert_eq!(state(&app, 6), DockChipState::Dim, "RCS is not available");

        // Engaging a maneuver makes CANCEL the hot verb.
        app.insert_resource(hints(false, true, None));
        app.update();
        assert_eq!(
            state(&app, 3),
            DockChipState::Hot,
            "CANCEL inverts while a maneuver runs"
        );
        assert_ne!(
            app.world()
                .entity(chips[3])
                .get::<BackgroundColor>()
                .unwrap()
                .0,
            chip_paint(DockChipState::Available).0,
            "the hot chip is painted differently, not just marked"
        );
    }

    /// No rig, no keys, no dock - a HUD without a flight rig must not show a
    /// row of keycaps for verbs that do not exist.
    #[test]
    fn dock_chips_stay_hidden_without_a_flight_rig() {
        let mut app = glyph_app();
        app.init_resource::<HintEmphasis>();
        app.insert_resource(FlightVerbHints::default());
        app.add_systems(Update, update_dock);
        app.insert_resource(NovaHudAssets {
            key_glyphs: real_glyphs(&app),
            ..default()
        });
        app.world_mut().spawn(keybind_dock_hud());
        app.update();

        for chip in chips(&app) {
            assert_eq!(
                app.world().entity(chip).get::<Node>().unwrap().display,
                Display::None,
                "no rig, no chips"
            );
        }
    }

    /// A key with no keycap art falls back to its TEXT - the dock degrades on a
    /// rebind instead of rendering an empty box.
    #[test]
    fn an_unmapped_key_falls_back_to_a_text_chip() {
        let mut app = glyph_app();
        app.init_resource::<HintEmphasis>();
        let mut resource = hints(true, false, None);
        resource.stop.key = "F13".into();
        app.insert_resource(resource);
        app.add_systems(Update, update_dock);
        app.insert_resource(NovaHudAssets {
            key_glyphs: real_glyphs(&app),
            ..default()
        });
        app.world_mut().spawn(keybind_dock_hud());
        app.update();

        let stop = chips(&app)[0];
        let children = app.world().entity(stop).get::<Children>().unwrap().to_vec();
        let mut saw_text = false;
        for child in children {
            let entity = app.world().entity(child);
            if entity.contains::<ChipGlyph>() {
                assert_eq!(
                    entity.get::<Node>().unwrap().display,
                    Display::None,
                    "no art, no picture"
                );
            }
            if entity.contains::<ChipKeyText>() {
                saw_text = true;
                assert_eq!(entity.get::<Text>().unwrap().0, "F13");
                assert_eq!(entity.get::<Node>().unwrap().display, Display::Flex);
            }
        }
        assert!(saw_text, "the fallback text chip exists");
    }

    /// Only dock verbs are addressable - a scenario typo is refused loudly
    /// instead of becoming a silently dead emphasis.
    #[test]
    fn emphasis_rejects_non_dock_verbs() {
        let mut emphasis = HintEmphasis::default();
        assert!(!emphasis.set("ALT"), "ALT is not a dock chip");
        assert!(!emphasis.contains("ALT"));
        assert!(emphasis.set("GOTO"));
        assert!(emphasis.contains("GOTO"));
    }

    /// The emphasized chip renders PURE gold hue at every point of the wave -
    /// only its alpha moves; a cross-hue mix passes through a washed near-white
    /// blend that killed readability in playtest (task 20260712-152340).
    #[test]
    fn emphasized_chips_pulse_pure_gold_alpha_only() {
        let gold = OBJECTIVE_GOLD.to_srgba();
        for i in 0..=10 {
            let wave = i as f32 / 10.0;
            for available in [true, false] {
                let sample = emphasis_color(available, wave).to_srgba();
                assert_eq!(
                    (sample.red, sample.green, sample.blue),
                    (gold.red, gold.green, gold.blue),
                    "wave {wave}: hue must stay gold, never a white-ish blend"
                );
            }
            let bright = emphasis_color(true, wave).to_srgba().alpha;
            let dim = emphasis_color(false, wave).to_srgba().alpha;
            assert!(
                dim < bright,
                "wave {wave}: unavailable stays below available ({dim} vs {bright})"
            );
        }
        let (lo, hi) = EMPHASIS_ALPHA_AVAILABLE;
        assert!(
            emphasis_color(true, 1.0).to_srgba().alpha - emphasis_color(true, 0.0).to_srgba().alpha
                >= 0.9 * (hi - lo),
            "the alpha actually sweeps its band (a flat pulse is dead code)"
        );
    }

    /// DoD 3 (migrated from the row-based test): `HintEmphasisSet` on a verb
    /// still visibly marks that verb's DOCK chip - the tutorial scenarios
    /// address verbs by name and must keep pointing at a key. The unemphasized
    /// chip keeps its availability paint.
    #[test]
    fn scenario_emphasis_marks_the_dock_chip() {
        let mut app = glyph_app();
        app.insert_resource(hints(true, false, None));
        app.init_resource::<HintEmphasis>();
        app.add_systems(
            Update,
            (update_dock, pulse_emphasized_chips.after(update_dock)),
        );
        app.insert_resource(NovaHudAssets {
            key_glyphs: real_glyphs(&app),
            ..default()
        });
        app.world_mut().spawn(keybind_dock_hud());

        app.world_mut().resource_mut::<HintEmphasis>().set("GOTO");
        app.update();

        let chips = chips(&app);
        let label_color = |app: &App, chip: Entity| {
            let children = app.world().entity(chip).get::<Children>().unwrap().to_vec();
            children
                .into_iter()
                .find_map(|child| {
                    let entity = app.world().entity(child);
                    entity
                        .contains::<ChipLabel>()
                        .then(|| entity.get::<TextColor>().unwrap().0)
                })
                .unwrap()
        };
        let gold = OBJECTIVE_GOLD.to_srgba();
        let goto = label_color(&app, chips[1]).to_srgba();
        assert_eq!(
            (goto.red, goto.green, goto.blue),
            (gold.red, gold.green, gold.blue),
            "the emphasized chip is gold-hued on screen"
        );
        assert_eq!(
            label_color(&app, chips[0]),
            base_colors(DockChipState::Available).1,
            "unemphasized chips keep their availability paint"
        );

        // Clearing restores the base - a cleared chip must not stay stuck
        // mid-pulse.
        app.world_mut().resource_mut::<HintEmphasis>().clear("GOTO");
        app.update();
        assert_eq!(
            label_color(&app, chips[1]),
            base_colors(DockChipState::Dim).1,
            "the cleared chip returns to its availability paint"
        );

        // Quiet frames: nothing rewrites the chip, and it stays at base (a
        // regressed gate would resume pulsing or stick a stale color).
        app.update();
        app.update();
        assert_eq!(
            label_color(&app, chips[1]),
            base_colors(DockChipState::Dim).1
        );
    }

    /// A rig despawn (key empties) while a chip is mid-pulse must restore the
    /// base paint even though the EMPHASIS never changed - the key emptying is
    /// a hints change (review R1.4 of 20260712-093831).
    #[test]
    fn rig_despawn_mid_pulse_restores_the_base_paint() {
        let mut app = glyph_app();
        app.insert_resource(hints(true, false, None));
        app.init_resource::<HintEmphasis>();
        app.add_systems(
            Update,
            (update_dock, pulse_emphasized_chips.after(update_dock)),
        );
        app.insert_resource(NovaHudAssets {
            key_glyphs: real_glyphs(&app),
            ..default()
        });
        app.world_mut().spawn(keybind_dock_hud());
        let stop = chips(&app)[0];
        let border = |app: &App| app.world().entity(stop).get::<BorderColor>().unwrap().top;

        app.world_mut().resource_mut::<HintEmphasis>().set("STOP");
        app.update();
        assert_ne!(
            border(&app),
            chip_paint(DockChipState::Available).1,
            "delivery guard: the chip pulses"
        );

        // The rig despawns: every key label empties, emphasis untouched.
        app.insert_resource(FlightVerbHints::default());
        app.update();
        assert_eq!(
            border(&app),
            base_colors(DockChipState::Dim).0,
            "the keyless chip returns to its base instead of freezing gold"
        );
    }

    fn spawn_cues(app: &mut App) -> (Entity, Entity) {
        let glyphs = real_glyphs(app);
        app.insert_resource(NovaHudAssets {
            key_glyphs: glyphs,
            ..default()
        });
        let layer = app.world_mut().spawn(verb_cues_hud()).id();
        let children = app.world().entity(layer).get::<Children>().unwrap();
        (children[0], children[1])
    }

    fn anchor_of(app: &App, entity: Entity) -> Option<ScreenIndicatorAnchorKind> {
        **app
            .world()
            .entity(entity)
            .get::<ScreenIndicatorAnchor>()
            .unwrap()
    }

    /// The anchored cues are keycap chips now, and the ORBIT cue still follows
    /// the resolver's offer.
    #[test]
    fn orbit_cue_is_a_keycap_chip_following_the_resolvers_offer() {
        let mut app = glyph_app();
        let well = app.world_mut().spawn_empty().id();
        app.insert_resource(hints(true, false, Some(well)));
        let (orbit_cue, _) = spawn_cues(&mut app);

        app.world_mut().run_system_once(drive_orbit_cue).unwrap();

        // The cue draws the keycap for the LIVE orbit binding, not a bracketed
        // key string - and it resolves it from the binding, not from a
        // hardcoded letter (feed it a different key below and it follows).
        let stem = key_glyph_stem("O").unwrap();
        assert_eq!(
            glyph_path(&app, orbit_cue).as_deref(),
            Some(format!("{KEY_GLYPH_DIR}/{stem}.png").as_str()),
            "the cue draws the keycap of the key that engages ORBIT"
        );

        assert_eq!(
            anchor_of(&app, orbit_cue),
            Some(ScreenIndicatorAnchorKind::Entity(well))
        );
        assert_eq!(
            app.world().entity(orbit_cue).get::<Node>().unwrap().display,
            Display::Flex
        );

        // Orbiting (the resolver retires the offer): the cue hides.
        app.insert_resource(hints(false, true, Some(well)));
        app.world_mut().run_system_once(drive_orbit_cue).unwrap();
        assert_eq!(anchor_of(&app, orbit_cue), None);
        assert_eq!(
            app.world().entity(orbit_cue).get::<Node>().unwrap().display,
            Display::None
        );

        // A REMAP moves the cue's keycap with it (the text cue this replaced
        // formatted the live key the same way; a hardcoded glyph would not).
        let mut remapped = hints(true, false, Some(well));
        remapped.orbit.key = "Z".into();
        app.insert_resource(remapped);
        app.world_mut().run_system_once(drive_orbit_cue).unwrap();
        let z_stem = key_glyph_stem("Z").unwrap();
        assert_eq!(
            glyph_path(&app, orbit_cue).as_deref(),
            Some(format!("{KEY_GLYPH_DIR}/{z_stem}.png").as_str()),
            "the cue follows the live binding"
        );

        // And a key with no art degrades to the text chip rather than an
        // empty box.
        let mut exotic = hints(true, false, Some(well));
        exotic.orbit.key = "F13".into();
        app.insert_resource(exotic);
        app.world_mut().run_system_once(drive_orbit_cue).unwrap();
        let children = app
            .world()
            .entity(orbit_cue)
            .get::<Children>()
            .unwrap()
            .to_vec();
        let mut saw_text = false;
        for child in children {
            let entity = app.world().entity(child);
            if entity.contains::<ChipGlyph>() {
                assert_eq!(entity.get::<Node>().unwrap().display, Display::None);
            }
            if entity.contains::<ChipKeyText>() {
                saw_text = true;
                assert_eq!(entity.get::<Text>().unwrap().0, "F13");
                assert_eq!(entity.get::<Node>().unwrap().display, Display::Flex);
            }
        }
        assert!(saw_text, "the cue carries a key-text fallback");
    }

    #[test]
    fn goto_cue_shows_on_the_lock_and_hides_while_engaged() {
        let mut app = glyph_app();
        let lock = app.world_mut().spawn_empty().id();
        let mut resource = hints(false, false, None);
        resource.goto = VerbHint {
            key: "G".into(),
            available: true,
            anchor: Some(lock),
        };
        app.insert_resource(resource.clone());
        let (_, goto_cue) = spawn_cues(&mut app);

        app.world_mut().run_system_once(drive_goto_cue).unwrap();
        assert_eq!(
            anchor_of(&app, goto_cue),
            Some(ScreenIndicatorAnchorKind::Entity(lock))
        );

        // A maneuver engages: the destination marker takes over.
        resource.cancel.available = true;
        resource.engaged = true;
        app.insert_resource(resource);
        app.world_mut().run_system_once(drive_goto_cue).unwrap();
        assert_eq!(anchor_of(&app, goto_cue), None);
    }

    /// Every dock verb's live key resolves to a keycap - a verb that fell
    /// through to the text fallback in NORMAL play would be a silent
    /// regression of the whole point of this dock.
    #[test]
    fn every_dock_verb_has_a_keycap_for_its_live_key() {
        for key in ["X", "G", "O", "Z", "CTRL", "SCROLL", "SHIFT"] {
            assert!(
                key_glyph_stem(key).is_some(),
                "the dock's live key '{key}' must have keycap art"
            );
        }
        assert!(
            KEY_GLYPH_FILES.len() >= DOCK_VERBS.len(),
            "delivery guard: the mapping table is populated"
        );
    }
}
