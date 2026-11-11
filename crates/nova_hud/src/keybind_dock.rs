//! The contextual keybind **dock** and the anchored verb cues.
//!
//! Nobody memorizes X/G/O/Z, and nobody reads a wall of bracketed-key-plus-verb
//! text while flying either. Both surfaces render from the input layer's
//! [`FlightVerbHints`] resource - which verbs apply, which are available and
//! what key each is bound to are resolved where the verbs live; this module is
//! a dumb view.
//!
//! - **Dock**: a row of icon chips along the bottom of the screen, each a real
//!   keycap PICTURE (see [`super::key_glyphs`]) plus the verb's word. A chip
//!   draws only while its verb applies to the situation
//!   ([`VerbHint::shown`]), in three groups, in [`DOCK_VERBS`] order: the
//!   core group (INTERFACE, DRIVE, STOP, RCS, RADAR) while the flight rig
//!   exists; the situational group (GOTO, ORBIT, CANCEL, DOCK, HELM, MINE)
//!   while there is a lock, a well, a maneuver, a port, a docked pair or a
//!   bound mining section to act on; the weapon group (AIM, FIRE, TORPEDO,
//!   RAILGUN) while the hull is armed and the weapons are hot. The core group
//!   is right-aligned to the screen's centre line and the other groups grow
//!   rightward from it, so a core chip never moves and a chip entering pushes
//!   only the chips right of it. A shown chip reads in three states - dim
//!   (pressing it now does nothing), available (full phosphor) and hot
//!   (inverted, the verb is what the ship is doing).
//! - **Anchored cues**: the hint sits on the thing you would act on - ORBIT
//!   projected on the dominant well, GOTO on the aim lock while no maneuver is
//!   engaged - the same keycap chip rather than `[O] ORBIT` text.
//!
//! The scenario spotlight ([`HintEmphasis`], driven by the
//! `HintEmphasisSet`/`HintEmphasisClear` actions) pulses a dock chip by verb
//! name. An emphasized verb is always drawn, dim while it is unavailable, so
//! a tutorial can point at a key BEFORE it applies: the spotlight pulses the
//! dim chip in the unavailable alpha band.

use bevy::{platform::collections::HashSet, prelude::*, ui::UiSystems};
use nova_ship::input::prelude::*;
use nova_ui::hud as chip;

use super::{
    emphasis::prelude::*,
    hud_notice::{HudNoticeKeepOut, NOTICE_BOTTOM_PX},
    key_glyphs::prelude::*,
    screen_indicator::{fit_box, prelude::*},
    situation::prelude::*,
    target_inset::{prelude::*, TargetInsetKeepOut},
    NovaHudAssets, OBJECTIVE_GOLD,
};

/// The keybind-dock and verb-cue spawners, `DockChip` with its state and emphasis, `DOCK_VERBS`
/// with `DOCK_CORE_LEN`, `DOCK_BOTTOM_PX` and `KeybindDockPlugin`.
pub mod prelude {
    pub use super::{
        keybind_dock_hud, verb_cues_hud, DockChip, DockChipState, HintEmphasis, KeybindDockMarker,
        KeybindDockPlugin, VerbCuesHudMarker, DOCK_BOTTOM_PX, DOCK_CORE_LEN, DOCK_VERBS,
    };
}

/// The keycap picture's on-screen HEIGHT (px) inside a chip - demo 2's `.ki`.
/// Height only: the PNGs share a square 128x128 canvas but the caps drawn in it
/// do not (a letter cap is 96x104, Tab/Shift/Ctrl 112x74, Space 128x68), so the
/// width follows each cap's own aspect and a wide cap draws WIDE at full legend
/// size instead of being squeezed into a square. [`KeyCap`] owns that rule.
const GLYPH_PX: f32 = 22.0;

/// The (smaller) keycap height on an anchored cue, which sits over the world and
/// must not eclipse the thing it labels.
const CUE_GLYPH_PX: f32 = 20.0;

/// Chip text size (px) - the dock is a glance surface, the word is a reminder.
const CHIP_TEXT_PX: f32 = 11.0;

/// How the cue stands off its object: below it, clear of its projected
/// silhouette, so it reads as a caption and not as a lock marker.
///
/// The floor is the 48 px the cue has always sat at - the composition on a
/// small contact. Without the clearance the same 48 px lands inside a big
/// silhouette, where a verb cue reads as something painted ON the target.
const CUE_CLEARANCE: ScreenIndicatorClearance = ScreenIndicatorClearance {
    direction: Vec2::Y,
    gap_px: 16.0,
    min_px: 48.0,
};

/// The gap (px) a cue keeps from the shown notice box and the target inset.
const CUE_NOTICE_GAP_PX: f32 = 8.0;
use nova_ui::{
    theme::{ActiveUiTheme, UiColorType},
    widget::{ThemedBorder, ThemedText},
};

/// How far off the bottom of the screen the dock row sits.
///
/// Public because the dock is the floor of the bottom-centre column: anything
/// else that wants that spot stacks ON it, measuring the row's live height
/// rather than assuming one (see `cinematic_prompt`).
pub const DOCK_BOTTOM_PX: f32 = 14.0;

/// Space between dock chips. Split across the two halves' inner padding, so
/// the gap at the centre line is the same gap.
const DOCK_CHIP_GAP_PX: f32 = 7.0;

/// The verb names, in dock display order (left to right): the core group,
/// then the situational group, then the weapon group. The first
/// [`DOCK_CORE_LEN`] sit left of the centre line; the rest grow right of it.
pub const DOCK_VERBS: [&str; 15] = [
    "INTERFACE",
    "DRIVE",
    "STOP",
    "RCS",
    "RADAR",
    "GOTO",
    "ORBIT",
    "CANCEL",
    "DOCK",
    "HELM",
    "MINE",
    "AIM",
    "FIRE",
    "TORPEDO",
    "RAILGUN",
];

/// How many of [`DOCK_VERBS`], from the front, are the core group: shown
/// while the flight rig exists, right-aligned to the centre line.
pub const DOCK_CORE_LEN: usize = 5;

/// Emphasis pulse rate and the alpha bands it sweeps. The emphasized chip
/// renders PURE OBJECTIVE_GOLD hue at all times and only its alpha pulses: the
/// first cut cross-mixed the availability color toward gold, and a lit chip's
/// phosphor->gold RGB path passes through a washed near-white blend every
/// cycle - unreadable at 11 px.
/// Availability still reads from the band: available chips sweep the bright
/// band, unavailable chips a dim one (spotlight, not a state change). ~1 Hz -
/// present in peripheral vision, not a strobe.
const EMPHASIS_PERIOD_SECS: f32 = 1.0;
const EMPHASIS_ALPHA_AVAILABLE: (f32, f32) = (0.7, 1.0);
const EMPHASIS_ALPHA_UNAVAILABLE: (f32, f32) = (0.3, 0.5);

/// Peak scale of a HOT chip - demo 2's `.vchip.hot { transform: scale(1.08) }`.
/// Small on purpose: the inversion already carries the state, the growth just
/// pulls the eye.
const HOT_CHIP_EMPHASIS: f32 = 1.08;

/// Marker for the bottom-centre keybind dock root; spawned by
/// [`keybind_dock_hud`].
#[derive(Component, Debug, Clone, Reflect)]
pub struct KeybindDockMarker;

/// One dock chip; the index picks the verb (see [`DOCK_VERBS`] and
/// `verb_hint`), and its label is that verb's name. Public so range examples
/// and screenshot rigs can assert on the dock's contents.
#[derive(Component, Debug, Clone, Copy, Deref, DerefMut, Reflect)]
pub struct DockChip(pub usize);

/// A dock chip's rendered state, written every update from availability (and,
/// for `Hot`, from what the ship is actually doing). Public so tests and the
/// situational-emphasis driver can read and drive it.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
pub enum DockChipState {
    /// Pressing it now would do nothing: the chip draws muted in its slot, and
    /// a scenario spotlight pulses it in the unavailable alpha band.
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

/// The verbs the scenario wants the player's eyes on:
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
fn chip_paint(state: DockChipState, theme: &ActiveUiTheme) -> (Color, Color, Color) {
    match state {
        DockChipState::Dim => (
            chip::CHIP_FILL_QUIET.with_alpha(0.62),
            theme.color_alpha(UiColorType::Text, 0.22),
            theme.color_alpha(UiColorType::TextMuted, 0.8),
        ),
        DockChipState::Available => (
            chip::CHIP_FILL,
            theme.color_alpha(UiColorType::Text, 0.4),
            theme.color(UiColorType::Text),
        ),
        DockChipState::Hot => (
            theme.color(UiColorType::Text),
            theme.color(UiColorType::Text),
            theme.color(UiColorType::TextInverse),
        ),
    }
}

/// The keycap image tint for `state` - Bevy UI has no grayscale filter, so the
/// dim state is alpha-dimming; the dark keycaps with white glyphs read well
/// over the phosphor HUD either way.
fn glyph_tint(state: DockChipState) -> Color {
    match state {
        DockChipState::Dim => Color::WHITE.with_alpha(0.6),
        _ => Color::WHITE,
    }
}

/// UI bundle for the keybind dock: two half-width halves on the bottom edge.
/// The left half right-aligns the core chips to the centre line, the right
/// half grows the situational and weapon chips rightward from it, so a core
/// chip never moves and a chip entering pushes only the chips right of it.
///
/// The chips spawn EMPTY - no keycap, no key text, collapsed. `update_dock`
/// fills them, and its `Added<DockChip>` gate makes that happen on the same
/// frame they appear. Resolving the art here as well would mean guessing each
/// verb's key at spawn time, and the binding is only knowable from the live
/// [`FlightVerbHints`] the update system already reads.
pub fn keybind_dock_hud() -> impl Bundle {
    let chip_of = |index: usize| {
        (
            DockChip(index),
            DockChipState::Dim,
            // A hot chip grows a little as well as inverting (demo 2's
            // `.vchip.hot`), driven by `grow_hot_chips` from the state above.
            HudEmphasis::settle(HOT_CHIP_EMPHASIS),
            Node {
                display: Display::None,
                align_items: AlignItems::Center,
                column_gap: Val::Px(6.0),
                padding: UiRect::new(Val::Px(4.0), Val::Px(8.0), Val::Px(3.0), Val::Px(3.0)),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(chip::CHIP_RADIUS)),
                ..default()
            },
            // Unpainted: `update_dock` paints every chip, and its `Added` gate
            // makes that the frame the chip appears.
            BackgroundColor(Color::NONE),
            BorderColor::all(Color::NONE),
            Pickable::IGNORE,
            children![
                (
                    ChipGlyph,
                    ImageNode::default(),
                    // No size here: the keycap's box depends on WHICH cap the
                    // live binding resolves to, and only `paint_key_visual`
                    // knows that. A hand-set square here would be both dead and
                    // wrong for every wide cap.
                    Node {
                        display: Display::None,
                        ..default()
                    },
                    Pickable::IGNORE,
                ),
                (
                    ChipKeyText,
                    Text::new(""),
                    TextFont::from_font_size(CHIP_TEXT_PX),
                    TextColor(Color::NONE),
                    Node {
                        display: Display::None,
                        ..default()
                    },
                ),
                (
                    ChipLabel,
                    Text::new(DOCK_VERBS[index]),
                    TextFont::from_font_size(CHIP_TEXT_PX),
                    TextColor(Color::NONE),
                ),
            ],
        )
    };

    let half = |name: &'static str, justify: JustifyContent, padding: UiRect| {
        (
            Name::new(name),
            Node {
                width: Val::Percent(50.0),
                justify_content: justify,
                column_gap: Val::Px(DOCK_CHIP_GAP_PX),
                padding,
                ..default()
            },
            Pickable::IGNORE,
        )
    };
    let half_gap = Val::Px(DOCK_CHIP_GAP_PX / 2.0);

    (
        Name::new("KeybindDockHUD"),
        KeybindDockMarker,
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(DOCK_BOTTOM_PX),
            left: Val::Px(0.0),
            width: Val::Percent(100.0),
            ..default()
        },
        Pickable::IGNORE,
        children![
            (
                half(
                    "KeybindDockCore",
                    JustifyContent::End,
                    UiRect::right(half_gap)
                ),
                Children::spawn(SpawnIter((0..DOCK_CORE_LEN).map(chip_of))),
            ),
            (
                half(
                    "KeybindDockSituational",
                    JustifyContent::Start,
                    UiRect::left(half_gap)
                ),
                Children::spawn(SpawnIter((DOCK_CORE_LEN..DOCK_VERBS.len()).map(chip_of))),
            ),
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
                    offset: Vec2::ZERO,
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
            CUE_CLEARANCE,
            BackgroundColor(chip::CHIP_FILL),
            BorderColor::all(Color::NONE),
            ThemedBorder::alpha(UiColorType::Text, 0.4),
            children![
                (
                    ChipGlyph,
                    ImageNode::default(),
                    // Sized by `paint_key_visual` from the live binding's cap,
                    // exactly like the dock's chips.
                    Node {
                        display: Display::None,
                        ..default()
                    },
                    Pickable::IGNORE,
                ),
                (
                    ChipKeyText,
                    Text::new(""),
                    TextFont::from_font_size(CHIP_TEXT_PX),
                    TextColor(Color::NONE),
                    ThemedText::new(UiColorType::Text),
                    Node {
                        display: Display::None,
                        ..default()
                    },
                ),
                (
                    ChipLabel,
                    Text::new(label),
                    TextFont::from_font_size(CHIP_TEXT_PX),
                    TextColor(Color::NONE),
                    ThemedText::new(UiColorType::Text),
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
        trace!("KeybindDockPlugin: build");

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
                // Same reason: it reads the state update_dock writes.
                grow_hot_chips.after(update_dock),
                (drive_orbit_cue, drive_goto_cue),
            )
                .in_set(super::NovaHudSystems),
        );
        // The fit needs the cue box the indicator pass just wrote, and must
        // land before layout reads the cue's `left`/`top`.
        app.add_systems(
            PostUpdate,
            clear_cues_of_notice
                .after(ScreenIndicatorSystems)
                .before(UiSystems::Layout),
        );
    }
}

/// Move a cue off the shown notice. The notice holds its place: a cue follows
/// a lock anywhere on the screen, so no fixed notice slot can avoid it.
///
/// Only a cue within [`CUE_NOTICE_GAP_PX`] of the box moves. Its room ends at
/// the notice's bottom edge, so the move cannot land on the keybind dock row,
/// and a move that would reach the shown target inset or a cue already placed
/// clears those too.
/// The indicator pass rewrites `left`/`top` from the anchor every frame, so
/// the shift does not accumulate and the cue returns when the notice ends.
fn clear_cues_of_notice(
    notice: HudNoticeKeepOut,
    inset: TargetInsetKeepOut,
    q_camera: Query<&Camera, With<ScreenIndicatorCamera>>,
    ui_scale: Res<UiScale>,
    mut q_cue: Query<
        (&mut Node, &ComputedNode, &Visibility),
        (
            Or<(With<GotoCueUIMarker>, With<OrbitCueUIMarker>)>,
            Without<TargetInsetHudMarker>,
        ),
    >,
) {
    let Some(view) = q_camera
        .iter()
        .next()
        .and_then(|camera| camera.logical_viewport_size())
        .map(|size| size / ui_scale.0)
    else {
        return;
    };
    let Some(keep_out) = notice.rect(view) else {
        return;
    };
    let room = Vec2::new(view.x, view.y - NOTICE_BOTTOM_PX);
    let mut taken: Vec<Rect> = inset.rect(view).into_iter().collect();
    for (mut node, computed, visibility) in &mut q_cue {
        if *visibility == Visibility::Hidden {
            continue;
        }
        let (Val::Px(left), Val::Px(top)) = (node.left, node.top) else {
            continue;
        };
        let min = Vec2::new(left, top);
        let cue = Rect::from_corners(min, min + computed.size() * computed.inverse_scale_factor());
        if cue
            .intersect(keep_out.inflate(CUE_NOTICE_GAP_PX))
            .is_empty()
        {
            taken.push(cue);
            continue;
        }
        let moved = |shift: Vec2| Rect::from_corners(cue.min + shift, cue.max + shift);
        let mut blocked = keep_out;
        let mut shift = fit_box(cue, room, 0.0, Some(blocked), CUE_NOTICE_GAP_PX);
        // A refit can land on a rect the last fit cleared, so each pass adds
        // what it hit. Every pass adds a rect, which bounds the loop.
        for _ in 0..taken.len() {
            let hit = taken
                .iter()
                .filter(|rect| {
                    !moved(shift)
                        .intersect(rect.inflate(CUE_NOTICE_GAP_PX))
                        .is_empty()
                })
                .fold(blocked, |all, rect| all.union(*rect));
            if hit == blocked {
                break;
            }
            blocked = hit;
            shift = fit_box(cue, room, 0.0, Some(blocked), CUE_NOTICE_GAP_PX);
        }
        node.left = Val::Px(left + shift.x);
        node.top = Val::Px(top + shift.y);
        taken.push(moved(shift));
    }
}

/// The hint behind dock chip `index`.
fn verb_hint(hints: &FlightVerbHints, index: usize) -> &VerbHint {
    match dock_verb(index) {
        "INTERFACE" => &hints.interface,
        "DRIVE" => &hints.drive,
        "STOP" => &hints.stop,
        "RCS" => &hints.rcs,
        "RADAR" => &hints.radar,
        "GOTO" => &hints.goto,
        "ORBIT" => &hints.orbit,
        "CANCEL" => &hints.cancel,
        "DOCK" => &hints.dock,
        "HELM" => &hints.helm,
        "MINE" => &hints.mine,
        "AIM" => &hints.aim,
        "FIRE" => &hints.fire,
        "TORPEDO" => &hints.torpedo,
        _ => &hints.railgun,
    }
}

/// The state chip `index` should render, from availability plus what the ship
/// is actually doing right now.
///
/// The `Hot` rules are the demo's, mapped onto the real verbs: the engaged
/// maneuver's own chip is hot (GOTO while a GOTO burns, STOP while stopping,
/// ORBIT while parking), CANCEL is hot whenever anything is engaged (it is the
/// live way out), RADAR is hot while a combat lock is held - the lock IS the
/// radar's product, and it is the chip you press to change it - DOCK is hot
/// while the hull is held by a docking joint, and HELM while the player holds
/// the docked pair's helm.
///
/// STOP is hot only while the maneuver its own word names flies
/// ([`FlightVerbHints::stop_flying`]: STOP while a STOP burns, MATCH while a
/// match flies and the modifier is held). A flying match leaves the plain
/// STOP chip lit as an offer and not hot.
///
/// `Hot` is checked BEFORE availability, because it means "this is what the
/// ship is doing", not "press this": the ORBIT offer is retired the moment you
/// are parked (`orbit.available` goes false), and that is precisely when the
/// ORBIT chip should read as the live maneuver.
///
/// RCS and DRIVE are hot while their key is held and the hold moves the
/// ship: the held key is a live state, the same meaning HELM's hot paint has.
/// AIM is hot while the weapons are hot and FIRE while a trigger is down.
/// INTERFACE, MINE, TORPEDO and RAILGUN are never hot: a press of each is an
/// instant, not a state the ship stays in.
fn chip_state(hints: &FlightVerbHints, situations: &HudSituations, index: usize) -> DockChipState {
    let hint = verb_hint(hints, index);
    let hot = match dock_verb(index) {
        "DRIVE" => hints.drive_held && hint.available,
        "STOP" => hints.stop_flying,
        "GOTO" => situations.maneuver == Some(ManeuverChip::Goto),
        "ORBIT" => situations.maneuver == Some(ManeuverChip::Orbit),
        "CANCEL" => hints.engaged,
        "RADAR" => situations.combat_lock,
        // DOCK is the only chip whose hot state is not a maneuver: it says the
        // hull is HELD. Its offer is retired the instant the joint exists (the
        // ports are spoken for), so without this the chip would vanish at the
        // exact moment docking becomes worth reporting.
        "DOCK" => situations.docked,
        "HELM" => hints.helm_held,
        "RCS" => hints.rcs_held && hint.available,
        "AIM" => situations.weapons_hot,
        "FIRE" => situations.firing,
        _ => false,
    };
    if hot {
        return DockChipState::Hot;
    }
    if !hint.available {
        return DockChipState::Dim;
    }
    DockChipState::Available
}

/// The verb name behind chip `index`, clamped rather than panicking - the dock
/// spawns exactly [`DOCK_VERBS`]-many chips, so an out-of-range index means a
/// caller invented a chip and the last verb is a harmless answer.
fn dock_verb(index: usize) -> &'static str {
    DOCK_VERBS[index.min(DOCK_VERBS.len() - 1)]
}

/// Point a chip's KEY visual at `key`: the keycap picture when the bound key has
/// art, the bare key text when it does not - so a rebind to an exotic key
/// degrades to a readable chip instead of rendering an empty box.
///
/// Shared by the dock and the anchored cues (generic over their query filters,
/// which differ only in the `Without` terms each system needs to stay disjoint
/// from its own root query). `height_px` is the SITE's keycap height; the width
/// comes from the cap's own aspect, via [`KeyCap::apply`].
fn paint_key_visual<FG, FT>(
    key: &str,
    cap: Option<KeyCap>,
    height_px: f32,
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
            let display = if cap.is_some() {
                Display::Flex
            } else {
                Display::None
            };
            if node.display != display {
                node.display = display;
            }
            if let Some(cap) = cap.as_ref() {
                // Guarded: `apply` writes the node's box, and dirtying a `Node`
                // costs a UI relayout even when the numbers are unchanged.
                if !cap.is_applied(&image, &node, height_px) {
                    cap.apply(height_px, &mut image, &mut node);
                }
            }
            image.color = tint;
        }
        if let Ok((mut text, mut node)) = q_key_text.get_mut(child) {
            let display = if cap.is_some() {
                Display::None
            } else {
                Display::Flex
            };
            if node.display != display {
                node.display = display;
            }
            if cap.is_none() && **text != key {
                **text = key.to_string();
            }
        }
    }
}

/// Paint every dock chip from the hints. A chip is in the layout while its
/// verb is shown ([`VerbHint::shown`]) or emphasized by a scenario, and
/// draws an unavailable verb in the muted [`DockChipState::Dim`] paint. The
/// dock is `Pickable::IGNORE` throughout, so a dim chip is no target. A
/// missing flight rig empties every key and takes every chip out of the
/// layout.
#[expect(clippy::type_complexity, reason = "one query per chip part")]
fn update_dock(
    hints: Res<FlightVerbHints>,
    theme: Res<ActiveUiTheme>,
    situations: Res<HudSituations>,
    emphasis: Res<HintEmphasis>,
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
        (
            With<ChipKeyText>,
            Without<DockChip>,
            Without<ChipGlyph>,
            Without<ChipLabel>,
        ),
    >,
    mut q_text_color: Query<&mut TextColor, Or<(With<ChipLabel>, With<ChipKeyText>)>>,
    mut q_label: Query<&mut Text, (With<ChipLabel>, Without<ChipKeyText>)>,
    q_added: Query<(), Added<DockChip>>,
) {
    // Skip quiet frames, but never skip freshly spawned chips (a respawned HUD
    // may appear while the resource is unchanged), nor the frame the glyph
    // collection finishes loading (chips spawned before it must pick it up).
    let assets_changed = assets.as_ref().is_some_and(|a| a.is_changed());
    // The theme is in the gate for the same reason `assets` is: it decides
    // what a chip is PAINTED, and this system is the only writer of that
    // paint, so a theme change that skipped here left the dock in the old look
    // until the next hint moved.
    // The emphasis is in the gate because it decides what is SHOWN: a
    // spotlight on a hidden verb brings its chip back.
    if !hints.is_changed()
        && !situations.is_changed()
        && !emphasis.is_changed()
        && !assets_changed
        && !theme.is_changed()
        && q_added.is_empty()
    {
        return;
    }
    let glyphs = assets.as_ref().map(|a| &a.key_glyphs);

    for (chip, mut state, mut node, mut fill, mut border, children) in &mut q_chip {
        let hint = verb_hint(&hints, **chip);
        let next = chip_state(&hints, &situations, **chip);
        // No key means no flight rig: the whole row leaves the layout. A
        // verb that does not apply leaves too, unless a scenario points at
        // it, which keeps the chip on the row to point at.
        let shown = !hint.key.is_empty() && (hint.shown || emphasis.contains(dock_verb(**chip)));
        let display = if shown { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }

        let cap = glyphs.and_then(|glyphs| glyphs.get(&hint.key));
        paint_key_visual(
            &hint.key,
            cap,
            GLYPH_PX,
            glyph_tint(next),
            children,
            &mut q_glyph,
            &mut q_key_text,
        );
        if !shown {
            // A hidden chip still tracks a state, so a pulse that was running
            // when it left restores the RIGHT base. Writing `Dim` rather than
            // `next` keeps a hidden chip from staying `Hot` while
            // `HudSituations` still claims a maneuver, which `grow_hot_chips`
            // would hold grown until the chip came back.
            state.set_if_neq(DockChipState::Dim);
            continue;
        }

        state.set_if_neq(next);
        let (fill_color, border_color, text_color) = chip_paint(next, &theme);
        fill.set_if_neq(BackgroundColor(fill_color));
        border.set_if_neq(BorderColor::all(border_color));
        let word = chip_word(&hints, **chip);
        for &child in children {
            if let Ok(mut color) = q_text_color.get_mut(child) {
                color.set_if_neq(TextColor(text_color));
            }
            if let Ok(mut label) = q_label.get_mut(child) {
                if label.0 != word {
                    label.0 = word.to_string();
                }
            }
        }
    }
}

/// The word chip `index` shows. STOP reads MATCH while the RCS modifier is
/// held, because the key press is then the MATCH chord; the chip index and its
/// emphasis name do not change.
fn chip_word(hints: &FlightVerbHints, index: usize) -> &'static str {
    match (dock_verb(index), hints.stop_verb) {
        ("STOP", StopVerbType::Match) => "MATCH",
        (verb, _) => verb,
    }
}

/// Grow the chip whose verb the ship is currently doing.
/// Reads the state [`update_dock`] just wrote rather than re-deriving it, so the
/// grown chip and the inverted chip can never be different chips.
fn grow_hot_chips(mut q_chip: Query<(&DockChipState, &mut HudEmphasis), With<DockChip>>) {
    for (state, mut emphasis) in &mut q_chip {
        emphasis.set_held(*state == DockChipState::Hot);
    }
}

/// The availability border and label colors [`update_dock`] gives a chip - the
/// base the emphasis pulse departs from and returns to.
fn base_colors(state: DockChipState, theme: &ActiveUiTheme) -> (Color, Color) {
    let (_, border, text) = chip_paint(state, theme);
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
fn pulse_emphasized_chips(
    time: Res<Time>,
    theme: Res<ActiveUiTheme>,
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
        let verb = dock_verb(**chip);
        // Only a chip with a key is on screen, so only it pulses; everything
        // else takes the restore below. The restore matters for a chip whose
        // key empties MID-pulse (rig despawn): it must not keep its gold, and
        // the key emptying is a HINTS change, not an emphasis change.
        let (next_border, next_label) = if !hint.key.is_empty() && emphasis.contains(verb) {
            let gold = emphasis_color(hint.available, wave);
            (gold, gold)
        } else if emphasis.is_changed() || hints.is_changed() {
            // Restore the base exactly once per change; steady state leaves
            // unemphasized chips to update_dock (whose own write is identical,
            // so the diffed writes below are no-ops).
            base_colors(*state, &theme)
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
    let cap = assets
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
            cap.clone(),
            CUE_GLYPH_PX,
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
    let cap = assets
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
            cap.clone(),
            CUE_GLYPH_PX,
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
    use crate::key_glyphs::prelude::{key_glyph_stem, KeyGlyphs, KEY_GLYPH_DIR, KEY_GLYPH_FILES};

    fn hint(key: &str, shown: bool, available: bool) -> VerbHint {
        VerbHint {
            key: key.into(),
            shown,
            available,
            anchor: None,
        }
    }

    /// The fixture dock: the core group and GOTO, ORBIT, CANCEL and DOCK on
    /// the row, with STOP, RADAR, INTERFACE and DRIVE lit; HELM, MINE and the
    /// weapon verbs off the row.
    pub(super) fn hints(
        orbit_available: bool,
        engaged: bool,
        well: Option<Entity>,
    ) -> FlightVerbHints {
        FlightVerbHints {
            interface: hint("Tab", true, true),
            drive: hint("W", true, true),
            drive_held: false,
            stop: hint("X", true, true),
            goto: hint("G", true, false),
            orbit: VerbHint {
                key: "O".into(),
                shown: true,
                available: orbit_available,
                anchor: well,
            },
            cancel: hint("Z", true, engaged),
            radar: hint("CTRL", true, true),
            rcs: hint("SHIFT", true, false),
            dock: hint("D", true, false),
            helm: hint("H", false, false),
            mine: hint("V", false, false),
            aim: hint("RMB", false, false),
            fire: hint("LMB", false, false),
            torpedo: hint("F", false, false),
            railgun: hint("R", false, false),
            helm_held: false,
            helm_fault: false,
            helm_station: false,
            dock_refusal: None,
            engaged,
            stop_verb: StopVerbType::Stop,
            stop_flying: false,
            rcs_held: false,
        }
    }

    /// Every verb bound, shown AND actionable - the fixture for assertions
    /// about a fully lit dock.
    pub(super) fn all_available_hints() -> FlightVerbHints {
        let mut resource = hints(true, false, None);
        for verb in DOCK_VERBS {
            let hint = verb_hint_mut(&mut resource, verb);
            hint.shown = true;
            hint.available = true;
        }
        resource
    }

    /// The hint behind `verb`, for a fixture to edit.
    pub(super) fn verb_hint_mut<'a>(
        hints: &'a mut FlightVerbHints,
        verb: &str,
    ) -> &'a mut VerbHint {
        match verb {
            "INTERFACE" => &mut hints.interface,
            "DRIVE" => &mut hints.drive,
            "STOP" => &mut hints.stop,
            "RCS" => &mut hints.rcs,
            "RADAR" => &mut hints.radar,
            "GOTO" => &mut hints.goto,
            "ORBIT" => &mut hints.orbit,
            "CANCEL" => &mut hints.cancel,
            "DOCK" => &mut hints.dock,
            "HELM" => &mut hints.helm,
            "MINE" => &mut hints.mine,
            "AIM" => &mut hints.aim,
            "FIRE" => &mut hints.fire,
            "TORPEDO" => &mut hints.torpedo,
            "RAILGUN" => &mut hints.railgun,
            other => panic!("{other} is not a dock verb"),
        }
    }

    /// The chip entity for `verb`, from a [`chips`] listing.
    pub(super) fn chip_of(chips: &[Entity], verb: &str) -> Entity {
        chips[DOCK_VERBS
            .iter()
            .position(|known| *known == verb)
            .unwrap_or_else(|| panic!("{verb} is not a dock verb"))]
    }

    /// An app with the asset machinery the keycap handles need. Copied from the
    /// nearest passing sibling rig shape (lesson `reuse-known-good-stack`): a
    /// bare `World` panics in the AssetServer the moment a handle is resolved.
    fn glyph_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Image>();
        // `NovaUiPlugin` owns the live theme in production; this harness adds
        // no plugins, and the paint systems below read it as a required
        // resource.
        app.init_resource::<ActiveUiTheme>();
        // The dock reads the contextual situations for its Hot states;
        // idle-cruise defaults keep these rigs testing the availability paint
        // they were written for.
        app.init_resource::<HudSituations>();
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
            entity.get::<ChipGlyph>()?;
            Some(image.image.path()?.to_string())
        })
    }

    /// Every dock chip, in [`DOCK_VERBS`] order, across both halves.
    pub(super) fn chips(app: &App) -> Vec<Entity> {
        let dock = app
            .world()
            .iter_entities()
            .find(|e| e.contains::<KeybindDockMarker>())
            .expect("a dock");
        let mut chips: Vec<Entity> = dock
            .get::<Children>()
            .unwrap()
            .iter()
            .flat_map(|half| app.world().entity(half).get::<Children>().unwrap().to_vec())
            .collect();
        chips.sort_by_key(|&chip| app.world().entity(chip).get::<DockChip>().unwrap().0);
        chips
    }

    /// Whether `chip` draws: in the layout.
    fn seen(app: &App, chip: Entity) -> bool {
        app.world().entity(chip).get::<Node>().unwrap().display == Display::Flex
    }

    /// HELM keeps one word through every helm state - the paint says which way
    /// the key goes, and the mode chip says why it cannot be taken - so its
    /// slot never changes width.
    #[test]
    fn the_helm_chip_reads_helm_and_paints_the_held_helm_hot() {
        let mut app = glyph_app();
        app.init_resource::<HintEmphasis>();
        app.add_systems(Update, update_dock);
        app.world_mut().spawn(keybind_dock_hud());
        let chips = chips(&app);
        let helm = chip_of(&chips, "HELM");
        let stop = chip_of(&chips, "STOP");
        let rcs = chip_of(&chips, "RCS");
        let label = |app: &App, chip: Entity| {
            app.world()
                .entity(chip)
                .get::<Children>()
                .unwrap()
                .iter()
                .find_map(|child| {
                    let entity = app.world().entity(child);
                    entity.get::<ChipLabel>()?;
                    Some(entity.get::<Text>()?.0.clone())
                })
                .unwrap()
        };
        let state =
            |app: &App, chip: Entity| *app.world().entity(chip).get::<DockChipState>().unwrap();

        // Docked in neutral: the helm is on offer, and the verbs it would
        // give back stay on the dock, dark. RCS is withheld and dark too.
        let mut neutral = hints(false, false, None);
        neutral.stop.available = false;
        neutral.helm.shown = true;
        neutral.helm.available = true;
        app.insert_resource(neutral.clone());
        app.insert_resource(HudSituations {
            docked: true,
            ..default()
        });
        app.update();
        assert_eq!(label(&app, helm), "HELM");
        assert_eq!(state(&app, helm), DockChipState::Available);
        assert!(seen(&app, stop), "STOP is blocked, not gone");
        assert_eq!(state(&app, stop), DockChipState::Dim);
        assert!(seen(&app, rcs), "a withheld RCS keeps its slot");
        assert_eq!(state(&app, rcs), DockChipState::Dim);

        // A pair that cannot be measured: the helm cannot be taken, and the
        // chip stays on the dock, dark, beside the verbs it would not give
        // back.
        let mut faulted = neutral.clone();
        faulted.helm.available = false;
        faulted.helm_fault = true;
        app.insert_resource(faulted);
        app.update();
        assert!(seen(&app, helm), "a faulted HELM stays on the dock");
        assert_eq!(label(&app, helm), "HELM");
        assert_eq!(state(&app, helm), DockChipState::Dim);
        assert!(seen(&app, stop), "a faulted pair's STOP keeps its slot");
        assert_eq!(state(&app, stop), DockChipState::Dim);

        // Measured again: the helm is on offer and STOP waits on it.
        app.insert_resource(neutral.clone());
        app.update();
        assert_eq!(label(&app, helm), "HELM");
        assert_eq!(state(&app, helm), DockChipState::Available);
        assert!(seen(&app, stop), "STOP is blocked again, not gone");

        let mut held = neutral;
        held.helm_held = true;
        held.stop.available = true;
        app.insert_resource(held.clone());
        app.update();
        assert_eq!(label(&app, helm), "HELM");
        assert_eq!(state(&app, helm), DockChipState::Hot);
        assert_ne!(
            app.world().entity(helm).get::<BackgroundColor>().unwrap().0,
            chip_paint(DockChipState::Available, &ActiveUiTheme::default()).0,
            "the held helm is painted hot, not just marked"
        );
        assert_eq!(state(&app, stop), DockChipState::Available);

        // Held through a fault: the helm can still be handed back.
        let mut held_faulted = held;
        held_faulted.helm_fault = true;
        held_faulted.stop.available = false;
        app.insert_resource(held_faulted);
        app.update();
        assert_eq!(label(&app, helm), "HELM");
        assert_eq!(state(&app, helm), DockChipState::Hot);
        assert_eq!(state(&app, stop), DockChipState::Dim);

        // Undocked: no helm to take.
        app.insert_resource(hints(false, false, None));
        app.insert_resource(HudSituations::default());
        app.update();
        assert!(!seen(&app, helm), "HELM leaves the dock without a pair");
        assert_eq!(state(&app, helm), DockChipState::Dim);
        for chip in &chips[..DOCK_CORE_LEN] {
            assert!(seen(&app, *chip), "the core verbs keep their slots");
        }
    }

    fn chip_word_of(app: &App, chip: Entity) -> String {
        app.world()
            .entity(chip)
            .get::<Children>()
            .unwrap()
            .iter()
            .find_map(|child| {
                let entity = app.world().entity(child);
                entity.get::<ChipLabel>()?;
                Some(entity.get::<Text>()?.0.clone())
            })
            .unwrap()
    }

    #[test]
    fn the_stop_chip_reads_match_while_the_rcs_modifier_is_held() {
        let mut app = glyph_app();
        app.init_resource::<HintEmphasis>();
        app.insert_resource(hints(false, false, None));
        app.add_systems(Update, update_dock);
        app.world_mut().spawn(keybind_dock_hud());
        app.update();
        let stop = chip_of(&chips(&app), "STOP");
        assert_eq!(chip_word_of(&app, stop), "STOP");

        let mut held = hints(false, false, None);
        held.stop_verb = StopVerbType::Match;
        held.rcs_held = true;
        app.insert_resource(held);
        app.update();
        assert_eq!(chip_word_of(&app, stop), "MATCH");

        app.insert_resource(hints(false, false, None));
        app.update();
        assert_eq!(chip_word_of(&app, stop), "STOP");
    }

    #[test]
    fn the_stop_chip_is_hot_only_while_its_own_word_is_flying() {
        let mut app = glyph_app();
        app.init_resource::<HintEmphasis>();
        app.insert_resource(hints(false, true, None));
        app.add_systems(Update, update_dock);
        app.world_mut().spawn(keybind_dock_hud());
        app.update();
        let stop = chip_of(&chips(&app), "STOP");
        let state = |app: &App| *app.world().entity(stop).get::<DockChipState>().unwrap();
        assert_eq!(state(&app), DockChipState::Available);

        // A flying match with the modifier up: the word is STOP, lit as an
        // offer, not hot.
        app.insert_resource(hints(false, true, None));
        app.update();
        assert_eq!(chip_word_of(&app, stop), "STOP");
        assert_eq!(state(&app), DockChipState::Available);

        // The same match with the modifier held: the word is MATCH and hot.
        let mut chord = hints(false, true, None);
        chord.stop_verb = StopVerbType::Match;
        chord.rcs_held = true;
        chord.stop_flying = true;
        app.insert_resource(chord);
        app.update();
        assert_eq!(chip_word_of(&app, stop), "MATCH");
        assert_eq!(state(&app), DockChipState::Hot);

        // A STOP burn with the modifier up: hot on its own word.
        let mut burning = hints(false, true, None);
        burning.stop_flying = true;
        app.insert_resource(burning);
        app.update();
        assert_eq!(chip_word_of(&app, stop), "STOP");
        assert_eq!(state(&app), DockChipState::Hot);
    }

    #[test]
    fn the_rcs_chip_is_hot_while_the_modifier_is_held() {
        let mut app = glyph_app();
        app.init_resource::<HintEmphasis>();
        let mut available = hints(false, false, None);
        available.rcs.available = true;
        app.insert_resource(available.clone());
        app.add_systems(Update, update_dock);
        app.world_mut().spawn(keybind_dock_hud());
        app.update();
        let rcs = chip_of(&chips(&app), "RCS");
        let state = |app: &App| *app.world().entity(rcs).get::<DockChipState>().unwrap();
        assert_eq!(state(&app), DockChipState::Available);

        available.rcs_held = true;
        app.insert_resource(available);
        app.update();
        assert_eq!(state(&app), DockChipState::Hot);
    }

    /// DoD 1: the dock renders ONE chip per verb, each carrying the keycap
    /// picture its live binding maps to and a state marker driven by
    /// availability. A no-op dock (chips with no glyph, or every chip in the
    /// same state) fails here.
    #[test]
    fn dock_renders_glyph_chip_per_verb_with_availability() {
        let mut app = glyph_app();
        app.init_resource::<HintEmphasis>();
        // Every verb available, so the keycap sweep below sees every chip lit;
        // the dim rule itself is `unavailable_verbs_draw_dim_in_their_slot`.
        // Docked, so HELM is on the dock too.
        app.insert_resource(all_available_hints());
        app.insert_resource(HudSituations {
            docked: true,
            ..default()
        });
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

        let expected_key = [
            "Tab", "W", "X", "SHIFT", "CTRL", "G", "O", "Z", "D", "H", "V", "RMB", "LMB", "F", "R",
        ];
        for (index, verb) in DOCK_VERBS.iter().enumerate() {
            let stem = key_glyph_stem(expected_key[index]).unwrap();
            assert_eq!(
                glyph_path(&app, chips[index]).as_deref(),
                Some(format!("{KEY_GLYPH_DIR}/{stem}.png").as_str()),
                "{verb} draws the keycap its binding maps to"
            );
            assert!(
                seen(&app, chips[index]),
                "{verb} is docked while it is actionable"
            );
        }

        // Availability drives the state marker, per verb. STOP/ORBIT/RADAR are
        // available here, the rest are not.
        app.insert_resource(hints(true, false, None));
        app.update();
        let state = |app: &App, verb: &str| {
            *app.world()
                .entity(chip_of(&chips, verb))
                .get::<DockChipState>()
                .unwrap()
        };
        assert_eq!(
            state(&app, "STOP"),
            DockChipState::Available,
            "STOP is actionable"
        );
        assert_eq!(
            state(&app, "ORBIT"),
            DockChipState::Available,
            "ORBIT is on offer"
        );
        assert_eq!(state(&app, "GOTO"), DockChipState::Dim, "GOTO has no lock");
        assert_eq!(
            state(&app, "CANCEL"),
            DockChipState::Dim,
            "CANCEL: nothing engaged"
        );
        assert_eq!(
            state(&app, "RCS"),
            DockChipState::Dim,
            "RCS is not available"
        );

        // Engaging a maneuver makes CANCEL the hot verb.
        app.insert_resource(hints(false, true, None));
        app.update();
        assert_eq!(
            state(&app, "CANCEL"),
            DockChipState::Hot,
            "CANCEL inverts while a maneuver runs"
        );
        assert_ne!(
            app.world()
                .entity(chip_of(&chips, "CANCEL"))
                .get::<BackgroundColor>()
                .unwrap()
                .0,
            chip_paint(DockChipState::Available, &ActiveUiTheme::default()).0,
            "the hot chip is painted differently, not just marked"
        );
    }

    /// The situational hot rules: the engaged maneuver's OWN chip inverts, and
    /// RADAR inverts while a combat lock is held. Each
    /// situation is asserted on and then off again, so a chip that latched hot
    /// would fail.
    #[test]
    fn the_dock_lights_the_verb_the_ship_is_doing() {
        let mut app = glyph_app();
        app.init_resource::<HintEmphasis>();
        app.insert_resource(hints(true, true, None));
        app.add_systems(Update, (update_dock, grow_hot_chips.after(update_dock)));
        app.world_mut().spawn(keybind_dock_hud());
        app.update();

        let chips = chips(&app);
        let state = |app: &App, verb: &str| {
            *app.world()
                .entity(chip_of(&chips, verb))
                .get::<DockChipState>()
                .unwrap()
        };
        let grown = |app: &App, verb: &str| {
            app.world()
                .entity(chip_of(&chips, verb))
                .get::<HudEmphasis>()
                .unwrap()
                .held()
        };

        // GOTO: hot only while a GOTO is the engaged maneuver.
        assert_eq!(
            state(&app, "GOTO"),
            DockChipState::Dim,
            "GOTO has no lock yet"
        );
        app.world_mut().resource_mut::<HudSituations>().maneuver = Some(ManeuverChip::Goto);
        app.update();
        assert_eq!(
            state(&app, "GOTO"),
            DockChipState::Hot,
            "the GOTO chip inverts while the GOTO burns"
        );
        assert!(grown(&app, "GOTO"), "and the hot chip grows");
        assert_eq!(
            state(&app, "RADAR"),
            DockChipState::Available,
            "RADAR is not hot without a lock"
        );

        // RADAR: hot while a combat lock is held; GOTO settles when the
        // maneuver ends.
        app.world_mut().resource_mut::<HudSituations>().maneuver = None;
        app.world_mut().resource_mut::<HudSituations>().combat_lock = true;
        app.update();
        assert_eq!(
            state(&app, "RADAR"),
            DockChipState::Hot,
            "RADAR inverts while the lock it produced is held"
        );
        assert_eq!(
            state(&app, "GOTO"),
            DockChipState::Dim,
            "GOTO settles back when the maneuver ends"
        );
        assert!(!grown(&app, "GOTO"), "and stops growing with it");
    }

    /// `Hot` means "this is what the ship is doing", so it beats the
    /// availability paint: the ORBIT offer is retired the moment you are
    /// parked, and that is exactly when the chip must read as the live
    /// maneuver rather than going dim.
    #[test]
    fn an_engaged_orbit_stays_hot_after_its_offer_is_retired() {
        let mut app = glyph_app();
        app.init_resource::<HintEmphasis>();
        // orbit_available = false: the resolver has retired the offer because
        // the ship is already parked.
        app.insert_resource(hints(false, true, None));
        app.add_systems(Update, update_dock);
        app.world_mut().spawn(keybind_dock_hud());
        app.world_mut().resource_mut::<HudSituations>().maneuver = Some(ManeuverChip::Orbit);
        app.update();

        let chips = chips(&app);
        assert_eq!(
            *app.world()
                .entity(chip_of(&chips, "ORBIT"))
                .get::<DockChipState>()
                .unwrap(),
            DockChipState::Hot,
            "the parked ship's ORBIT chip reads as the live maneuver"
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

    fn display(app: &App, chip: Entity) -> Display {
        app.world().entity(chip).get::<Node>().unwrap().display
    }

    /// A shown verb you cannot press right now draws muted on the row, so
    /// the player sees it is not on offer; a verb that does not apply is not
    /// on the row at all. A `Hot` chip stays hot even when its own offer has
    /// been retired, because hot means "this is what the ship is doing".
    #[test]
    fn unavailable_verbs_draw_dim_and_inapplicable_verbs_leave_the_row() {
        let mut app = glyph_app();
        app.init_resource::<HintEmphasis>();
        // STOP/ORBIT/RADAR available; GOTO/CANCEL/RCS shown but not; HELM,
        // MINE and the weapon verbs not shown.
        app.insert_resource(hints(true, false, None));
        app.add_systems(Update, update_dock);
        app.insert_resource(NovaHudAssets {
            key_glyphs: real_glyphs(&app),
            ..default()
        });
        app.world_mut().spawn(keybind_dock_hud());
        app.update();

        let chips = chips(&app);
        for verb in ["STOP", "ORBIT", "RADAR"] {
            assert!(
                seen(&app, chip_of(&chips, verb)),
                "{verb} is actionable, so it is docked"
            );
        }
        for verb in ["GOTO", "CANCEL", "RCS"] {
            let chip = chip_of(&chips, verb);
            assert!(seen(&app, chip), "{verb} applies, so it is on the row");
            assert_eq!(
                app.world().entity(chip).get::<BackgroundColor>().unwrap().0,
                chip_paint(DockChipState::Dim, &ActiveUiTheme::default()).0,
                "{verb} is not actionable, so it draws muted"
            );
        }
        for verb in ["HELM", "MINE", "AIM", "FIRE", "TORPEDO", "RAILGUN"] {
            assert_eq!(
                display(&app, chip_of(&chips, verb)),
                Display::None,
                "{verb} does not apply, so it leaves the row"
            );
        }

        // The retired-offer case: parked, so `orbit.available` is false, but
        // ORBIT is the live maneuver and must stay docked.
        app.insert_resource(hints(false, true, None));
        app.world_mut().resource_mut::<HudSituations>().maneuver = Some(ManeuverChip::Orbit);
        app.update();
        assert_eq!(
            *app.world()
                .entity(chip_of(&chips, "ORBIT"))
                .get::<DockChipState>()
                .unwrap(),
            DockChipState::Hot,
            "delivery guard: the parked ship's ORBIT chip is hot"
        );

        // ...and a verb that becomes available lights up.
        assert_ne!(
            *app.world()
                .entity(chip_of(&chips, "CANCEL"))
                .get::<DockChipState>()
                .unwrap(),
            DockChipState::Dim,
            "CANCEL lights up once something is engaged"
        );
    }

    /// A scenario that points at a verb before it applies still gets a chip
    /// to point at: the emphasized hidden verb joins the row, dim, and leaves
    /// again when the spotlight clears. The emphasis is set on an otherwise
    /// QUIET frame, which a change gate that ignores it silently drops.
    #[test]
    fn an_emphasized_hidden_verb_still_draws_dim() {
        let mut app = glyph_app();
        app.init_resource::<HintEmphasis>();
        app.insert_resource(hints(true, false, None));
        app.add_systems(
            Update,
            (update_dock, pulse_emphasized_chips.after(update_dock)),
        );
        app.insert_resource(NovaHudAssets {
            key_glyphs: real_glyphs(&app),
            ..default()
        });
        app.world_mut().spawn(keybind_dock_hud());
        app.update();
        app.update();

        let dock = chip_of(&chips(&app), "HELM");
        assert_eq!(
            display(&app, dock),
            Display::None,
            "delivery guard: undocked"
        );

        app.world_mut().resource_mut::<HintEmphasis>().set("HELM");
        app.update();
        assert_eq!(
            display(&app, dock),
            Display::Flex,
            "the spotlight brings it back"
        );
        assert_eq!(
            *app.world().entity(dock).get::<DockChipState>().unwrap(),
            DockChipState::Dim,
            "a spotlight is not a promotion"
        );

        app.world_mut().resource_mut::<HintEmphasis>().clear("HELM");
        app.update();
        assert_eq!(display(&app, dock), Display::None, "and it leaves with it");
    }

    /// DoD 2: a tutorial points at a verb BEFORE it lights up (that is what
    /// `EMPHASIS_ALPHA_UNAVAILABLE` is for), so an emphasized unavailable chip
    /// pulses in the dim band. The emphasis is set on an otherwise QUIET frame,
    /// which is the case a change gate that ignores `HintEmphasis` silently
    /// drops.
    #[test]
    fn an_emphasized_unavailable_verb_pulses_in_the_dim_band() {
        let mut app = glyph_app();
        app.init_resource::<HintEmphasis>();
        app.insert_resource(hints(true, false, None));
        app.add_systems(
            Update,
            (update_dock, pulse_emphasized_chips.after(update_dock)),
        );
        app.insert_resource(NovaHudAssets {
            key_glyphs: real_glyphs(&app),
            ..default()
        });
        app.world_mut().spawn(keybind_dock_hud());
        app.update();

        let chips = chips(&app);
        assert_eq!(
            *app.world()
                .entity(chip_of(&chips, "GOTO"))
                .get::<DockChipState>()
                .unwrap(),
            DockChipState::Dim,
            "delivery guard: GOTO has no lock, so it starts dim"
        );

        // Quiet frames: hints and situations are untouched from here on, so
        // only the emphasis can light the chip.
        app.update();
        app.world_mut().resource_mut::<HintEmphasis>().set("GOTO");
        app.update();

        let gold = OBJECTIVE_GOLD.to_srgba();
        let border = app
            .world()
            .entity(chip_of(&chips, "GOTO"))
            .get::<BorderColor>()
            .unwrap()
            .top;
        let sample = border.to_srgba();
        assert_eq!(
            (sample.red, sample.green, sample.blue),
            (gold.red, gold.green, gold.blue),
            "and it pulses gold"
        );
        let (lo, hi) = EMPHASIS_ALPHA_UNAVAILABLE;
        assert!(
            (lo - f32::EPSILON..=hi + f32::EPSILON).contains(&sample.alpha),
            "in the UNAVAILABLE band ({} not in {lo}..={hi}) - a spotlight is not \
             a promotion, and a fully transparent chip is not a spotlight",
            sample.alpha
        );

        // Clearing the spotlight must not leave the chip gold.
        app.world_mut().resource_mut::<HintEmphasis>().clear("GOTO");
        app.update();
        assert_eq!(
            app.world()
                .entity(chip_of(&chips, "GOTO"))
                .get::<BorderColor>()
                .unwrap()
                .top,
            base_colors(DockChipState::Dim, &ActiveUiTheme::default()).0,
            "the cleared chip drops its gold rather than freezing mid-pulse"
        );
    }

    /// A chip that leaves the dock must not stay marked `Hot`: `grow_hot_chips`
    /// reads that state and would hold an off-screen chip grown, so it would pop
    /// back in mid-shrink when the rig returned. The rig despawns MID-maneuver -
    /// keys empty while `HudSituations` still reports the ORBIT - which is the
    /// one input combination where the keyless branch's state write is visible.
    #[test]
    fn a_chip_that_leaves_the_dock_stops_being_hot() {
        let mut app = glyph_app();
        app.init_resource::<HintEmphasis>();
        app.insert_resource(hints(false, true, None));
        app.add_systems(Update, (update_dock, grow_hot_chips.after(update_dock)));
        app.world_mut().spawn(keybind_dock_hud());
        app.world_mut().resource_mut::<HudSituations>().maneuver = Some(ManeuverChip::Orbit);
        app.update();

        let orbit = chip_of(&chips(&app), "ORBIT");
        let held = |app: &App| {
            app.world()
                .entity(orbit)
                .get::<HudEmphasis>()
                .unwrap()
                .held()
        };
        assert_eq!(
            *app.world().entity(orbit).get::<DockChipState>().unwrap(),
            DockChipState::Hot,
            "delivery guard: the engaged ORBIT is hot and docked"
        );
        assert!(held(&app), "delivery guard: and grown");

        // The rig despawns while the ORBIT is still the reported maneuver.
        app.insert_resource(FlightVerbHints::default());
        app.update();
        assert_eq!(display(&app, orbit), Display::None, "no rig, no chip");
        assert_ne!(
            *app.world().entity(orbit).get::<DockChipState>().unwrap(),
            DockChipState::Hot,
            "an off-screen chip is not the verb the ship is doing"
        );
        assert!(!held(&app), "so it is not held grown while it is hidden");
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

        let stop = chip_of(&chips(&app), "STOP");
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
    /// blend that killed readability in playtest.
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
        let goto = label_color(&app, chip_of(&chips, "GOTO")).to_srgba();
        assert_eq!(
            (goto.red, goto.green, goto.blue),
            (gold.red, gold.green, gold.blue),
            "the emphasized chip is gold-hued on screen"
        );
        assert_eq!(
            label_color(&app, chip_of(&chips, "STOP")),
            base_colors(DockChipState::Available, &ActiveUiTheme::default()).1,
            "unemphasized chips keep their availability paint"
        );

        // Clearing restores the base - a cleared chip must not stay stuck
        // mid-pulse.
        app.world_mut().resource_mut::<HintEmphasis>().clear("GOTO");
        app.update();
        assert_eq!(
            label_color(&app, chip_of(&chips, "GOTO")),
            base_colors(DockChipState::Dim, &ActiveUiTheme::default()).1,
            "the cleared chip returns to its availability paint"
        );

        // Quiet frames: nothing rewrites the chip, and it stays at base (a
        // regressed gate would resume pulsing or stick a stale color).
        app.update();
        app.update();
        assert_eq!(
            label_color(&app, chip_of(&chips, "GOTO")),
            base_colors(DockChipState::Dim, &ActiveUiTheme::default()).1
        );
    }

    /// A rig despawn (key empties) while a chip is mid-pulse must restore the
    /// base paint even though the EMPHASIS never changed - the key emptying is
    /// a hints change.
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
        let stop = chip_of(&chips(&app), "STOP");
        let border = |app: &App| app.world().entity(stop).get::<BorderColor>().unwrap().top;

        app.world_mut().resource_mut::<HintEmphasis>().set("STOP");
        app.update();
        assert_ne!(
            border(&app),
            chip_paint(DockChipState::Available, &ActiveUiTheme::default()).1,
            "delivery guard: the chip pulses"
        );

        // The rig despawns: every key label empties, emphasis untouched.
        app.insert_resource(FlightVerbHints::default());
        app.update();
        assert_eq!(
            border(&app),
            base_colors(DockChipState::Dim, &ActiveUiTheme::default()).0,
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
            shown: true,
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
    /// regression of the whole point of this dock. The keys are the rig
    /// defaults and the editor's section defaults (`placement.rs`).
    #[test]
    fn every_dock_verb_has_a_keycap_for_its_live_key() {
        for key in [
            "Tab", "W", "X", "SHIFT", "CTRL", "G", "O", "Z", "D", "H", "V", "RMB", "LMB", "F", "R",
        ] {
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

/// Live-tree keycap SIZING rig.
///
/// The wide caps (Tab, Shift, Ctrl, Space) are drawn wide-and-short inside a
/// square 128x128 canvas, so a `width == height` node threw ~40% of their
/// height away and shrank the legend with it. These tests measure the real
/// laid-out `ComputedNode` of a chip's keycap and check it against
/// INDEPENDENTLY measured art bounds.
#[cfg(test)]
mod keycap_sizing_tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::{
        tests::{all_available_hints, chip_of, chips, hints, verb_hint_mut},
        *,
    };
    use crate::{
        chip_layout_rig::{chip_layout_app, load_png, measure, only_descendant_with, settle},
        key_glyphs::prelude::{trimmed_cap, KeyGlyphs, KEY_GLYPH_DIR, KEY_GLYPH_FILES},
    };

    /// The opaque bounding box of each keycap inside its 128x128 canvas,
    /// measured OUTSIDE this code base with
    /// `magick <file> -alpha extract -threshold 0 -format '%@' info:`
    /// (`WxH+X+Y`), then written here as `[min_x, min_y, max_x, max_y]`.
    ///
    /// Independent by construction: nothing here re-runs the production trim,
    /// so a trim that scans the wrong channel or the wrong axis fails instead
    /// of agreeing with itself (LESSONS `test-must-not-reuse-the-formula-under-test`).
    const MEASURED_CAPS: &[(&str, &str, [f32; 4])] = &[
        // 96x104+16+16 - the near-square letter caps.
        ("X", "T_X_Key_Alt", [16.0, 16.0, 112.0, 120.0]),
        ("O", "T_O_Key_Alt", [16.0, 16.0, 112.0, 120.0]),
        // 112x74+8+32 - the wide modifier caps.
        ("CTRL", "T_Crtl_Key_Alt", [8.0, 32.0, 120.0, 106.0]),
        ("SHIFT", "T_Shift_Key_Alt", [8.0, 32.0, 120.0, 106.0]),
        ("Tab", "T_Tab_Key_Alt", [8.0, 32.0, 120.0, 106.0]),
        // 128x68+0+32 - the widest cap of all, canvas-wide.
        ("Space", "T_Space_Key_Alt", [0.0, 32.0, 128.0, 100.0]),
        // 76x128+28+0 - the TALL odd one out: a mouse, not a keycap.
        (
            "SCROLL",
            "T_Mouse_Scroll_Key_Dark_Key_Alt",
            [28.0, 0.0, 104.0, 128.0],
        ),
    ];

    /// The independently measured bounds of `label`'s cap.
    fn measured_cap(label: &str) -> Rect {
        let [min_x, min_y, max_x, max_y] = MEASURED_CAPS
            .iter()
            .find(|(key, _, _)| *key == label)
            .unwrap_or_else(|| panic!("no hand-measured bounds recorded for '{label}'"))
            .2;
        Rect::new(min_x, min_y, max_x, max_y)
    }

    /// The REAL glyph lookup with REAL decoded pixels behind every handle -
    /// what the game has once the preload collection lands, and the only shape
    /// in which a trim scan has anything to scan.
    fn loaded_glyphs(app: &mut App) -> KeyGlyphs {
        let mut handles: Vec<(&'static str, Handle<Image>)> = Vec::new();
        for (_, stem) in KEY_GLYPH_FILES {
            if handles.iter().any(|(known, _)| known == stem) {
                continue;
            }
            let handle = load_png(app, &format!("{KEY_GLYPH_DIR}/{stem}.png"));
            handles.push((stem, handle));
        }
        assert!(
            !handles.is_empty(),
            "delivery guard: the mapping table names keycap files"
        );
        KeyGlyphs::from_stems(|stem| {
            handles
                .iter()
                .find(|(known, _)| *known == stem)
                .map(|(_, handle)| handle.clone())
        })
    }

    /// A dock laid out for real, with every keycap decoded and the glyph
    /// metrics scanned exactly as asset loading scans them.
    fn dock_app() -> (App, Vec<Entity>) {
        let mut app = chip_layout_app();
        app.init_resource::<HudSituations>();
        app.init_resource::<HintEmphasis>();
        app.insert_resource(all_available_hints());
        let mut glyphs = loaded_glyphs(&mut app);
        glyphs.measure_caps(app.world().resource::<Assets<Image>>());
        app.insert_resource(NovaHudAssets {
            key_glyphs: glyphs,
            ..default()
        });
        app.world_mut().spawn(keybind_dock_hud());
        app.add_systems(Update, update_dock);
        settle(&mut app);
        let chips = chips(&app);
        (app, chips)
    }

    /// The `ChipGlyph` node inside the dock chip for `verb`.
    fn chip_glyph(app: &mut App, chips: &[Entity], verb: &str) -> Entity {
        only_descendant_with::<ChipGlyph>(app, chip_of(chips, verb))
    }

    /// The core group is right-aligned to the centre line and the other
    /// groups grow rightward from it: a situational chip entering moves no
    /// core chip, sits right of the centre line in [`DOCK_VERBS`] order, and
    /// keeps the chip gap across the centre line.
    #[test]
    fn the_core_verbs_hold_still_when_situational_chips_come_and_go() {
        let (mut app, chips) = dock_app();
        // The core group alone on the row.
        let mut quiet = hints(true, false, None);
        for verb in &DOCK_VERBS[DOCK_CORE_LEN..] {
            verb_hint_mut(&mut quiet, verb).shown = false;
        }
        app.insert_resource(quiet.clone());
        settle(&mut app);
        let core_rects = |app: &App| -> Vec<Rect> {
            chips[..DOCK_CORE_LEN]
                .iter()
                .map(|&chip| measure(app, chip).rect)
                .collect()
        };
        let alone = core_rects(&app);
        // The rig lays out a 1280 px wide target.
        let centre = 1280.0 / 2.0;
        let radar = measure(&app, chip_of(&chips, "RADAR")).rect;
        assert!(
            (radar.max.x + DOCK_CHIP_GAP_PX / 2.0 - centre).abs() <= 1.0,
            "the core group ends half a gap short of the centre line: {radar:?}"
        );
        for verb in &DOCK_VERBS[DOCK_CORE_LEN..] {
            assert_eq!(
                app.world()
                    .entity(chip_of(&chips, verb))
                    .get::<Node>()
                    .unwrap()
                    .display,
                Display::None,
                "delivery guard: {verb} is off the row"
            );
        }

        // GOTO, DOCK and HELM join the row.
        let mut busy = quiet;
        for verb in ["GOTO", "DOCK", "HELM"] {
            verb_hint_mut(&mut busy, verb).shown = true;
        }
        app.insert_resource(busy);
        settle(&mut app);
        assert_eq!(
            core_rects(&app),
            alone,
            "no core chip moves when situational chips show"
        );
        let goto = measure(&app, chip_of(&chips, "GOTO")).rect;
        let dock = measure(&app, chip_of(&chips, "DOCK")).rect;
        let helm = measure(&app, chip_of(&chips, "HELM")).rect;
        assert!(
            (goto.min.x - DOCK_CHIP_GAP_PX / 2.0 - centre).abs() <= 1.0,
            "the first situational chip starts half a gap past the centre line: {goto:?}"
        );
        assert!(
            (goto.min.x - radar.max.x - DOCK_CHIP_GAP_PX).abs() <= 1.0,
            "the gap across the centre line is the chip gap: {radar:?} vs {goto:?}"
        );
        assert!(
            dock.min.x >= goto.max.x && helm.min.x >= dock.max.x,
            "the chips sit in dock order: {goto:?}, {dock:?}, {helm:?}"
        );
        assert!(helm.max.x <= 1280.0, "HELM stays on screen: {helm:?}");
        assert!(
            (helm.height() - radar.height()).abs() <= 0.5,
            "HELM matches the row height: {helm:?} vs {radar:?}"
        );
    }

    /// A wide cap renders at the aspect its ART carries, with the cap filling
    /// the box height - and a near-square cap stays near-square.
    ///
    /// The bug this pins: on the square `width == height` box every chip
    /// measured 22x22 regardless of its art.
    #[test]
    fn wide_keycaps_render_at_their_art_aspect() {
        let (mut app, chips) = dock_app();

        // RADAR's live key is CTRL (a WIDE cap), STOP's is X (a near-square
        // cap).
        for (verb, label) in [("RADAR", "CTRL"), ("STOP", "X")] {
            let cap = measured_cap(label);
            let glyph = chip_glyph(&mut app, &chips, verb);
            let size = measure(&app, glyph).size;
            let expected_width = GLYPH_PX * cap.width() / cap.height();

            assert!(
                (size.y - GLYPH_PX).abs() <= 0.5,
                "{label}: the cap is pinned to the site's height ({GLYPH_PX}), got {size:?}"
            );
            // Within a pixel: bevy_ui rounds every computed node to whole
            // PHYSICAL pixels, so an exact match is not on offer - and a pixel
            // is far tighter than the ~11 px the square box was wrong by.
            assert!(
                (size.x - expected_width).abs() <= 1.0,
                "{label}: the rendered box is {size:?}, but the art's opaque cap \
                 is {cap:?} - at a pinned height of {GLYPH_PX} that is \
                 {expected_width:.1} px wide"
            );

            let rect = app
                .world()
                .get::<ImageNode>(glyph)
                .expect("the glyph node carries an ImageNode")
                .rect;
            assert_eq!(
                rect,
                Some(cap),
                "{label}: the node must draw the CAP sub-rect, not the whole \
                 canvas - otherwise the transparent bands eat the height again"
            );
        }
    }

    /// The anchored cues carry the SAME sizing path at their OWN height - the
    /// site a fix aimed at the dock alone would miss (LESSONS
    /// `pin-each-caller-not-just-shared-core`).
    #[test]
    fn the_anchored_cue_sizes_its_keycap_at_the_cue_height() {
        let mut app = chip_layout_app();
        let mut glyphs = loaded_glyphs(&mut app);
        glyphs.measure_caps(app.world().resource::<Assets<Image>>());
        app.insert_resource(NovaHudAssets {
            key_glyphs: glyphs,
            ..default()
        });

        let well = app.world_mut().spawn_empty().id();
        let mut hints = all_available_hints();
        hints.orbit.anchor = Some(well);
        app.insert_resource(hints);

        let layer = app.world_mut().spawn(verb_cues_hud()).id();
        let orbit = app.world().entity(layer).get::<Children>().unwrap()[0];
        app.world_mut().run_system_once(drive_orbit_cue).unwrap();
        settle(&mut app);

        // ORBIT is bound to O, a near-square cap: what this pins is that the
        // cue's OWN height constant reaches the shared path, not the dock's.
        let cap = measured_cap("O");
        let glyph = only_descendant_with::<ChipGlyph>(&mut app, orbit);
        let size = measure(&app, glyph).size;
        assert!(
            (size.y - CUE_GLYPH_PX).abs() <= 0.5,
            "the cue's cap is pinned to CUE_GLYPH_PX ({CUE_GLYPH_PX}), got {size:?}"
        );
        let expected_width = CUE_GLYPH_PX * cap.width() / cap.height();
        assert!(
            (size.x - expected_width).abs() <= 1.0,
            "the cue's cap measured {size:?}, but O's art is {cap:?} - at a \
             pinned height of {CUE_GLYPH_PX} that is {expected_width:.1} px wide"
        );
        assert_eq!(
            app.world().entity(glyph).get::<ImageNode>().unwrap().rect,
            Some(cap),
            "the cue draws the cap sub-rect too"
        );
    }

    /// The trim itself, pinned against the hand-measured art: a scan that read
    /// the wrong channel, transposed the axes or was off by a row would agree
    /// with the production sizing and disagree here.
    #[test]
    fn the_trim_finds_the_hand_measured_cap_bounds() {
        let mut app = chip_layout_app();
        for (label, stem, _) in MEASURED_CAPS {
            let handle = load_png(&mut app, &format!("{KEY_GLYPH_DIR}/{stem}.png"));
            let images = app.world().resource::<Assets<Image>>();
            let image = images.get(&handle).expect("the rig decoded the PNG");
            assert_eq!(
                (image.width(), image.height()),
                (128, 128),
                "{label}: the keycap art is a 128x128 canvas"
            );
            assert_eq!(
                trimmed_cap(image),
                Some(measured_cap(label)),
                "{label}: the alpha trim disagrees with the art's measured bounds"
            );
        }
    }

    /// DoD 3: EVERY preloaded glyph resolves a cap, and a plausible one - a
    /// future glyph on an unexpected canvas (or one the scan cannot read) must
    /// fail loudly here instead of drawing a sliver in the dock.
    #[test]
    fn every_preloaded_glyph_resolves_a_plausible_cap() {
        let mut app = chip_layout_app();
        let mut glyphs = loaded_glyphs(&mut app);
        let resolved = glyphs.measure_caps(app.world().resource::<Assets<Image>>());

        assert!(!glyphs.is_empty(), "delivery guard: the rig loaded keycaps");
        assert_eq!(
            resolved,
            glyphs.len(),
            "every preloaded keycap must resolve a cap rect"
        );

        for (label, _) in KEY_GLYPH_FILES {
            let cap = glyphs.get(label).expect("a mapped label resolves").cap();
            let cap = cap.unwrap_or_else(|| panic!("{label}: no cap measured"));
            assert!(
                cap.min.x >= 0.0 && cap.min.y >= 0.0 && cap.max.x <= 128.0 && cap.max.y <= 128.0,
                "{label}: the cap {cap:?} escapes the 128x128 canvas"
            );
            // Half the canvas in each axis: every cap in this set spans at
            // least 68 of 128 px, so anything thinner is a glyph whose canvas
            // or padding changed and whose sizing wants a fresh look.
            assert!(
                cap.width() >= 64.0 && cap.height() >= 64.0,
                "{label}: the cap {cap:?} covers less than half the canvas - \
                 has the art changed shape?"
            );
        }
    }
}
