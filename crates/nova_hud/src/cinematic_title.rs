//! The cinematic title card: where you are, when it is, and one line that
//! makes it matter.
//!
//! Data path: a scenario's `CinematicTitle` action posts a card, and the
//! event-world sync (nova_scenario) writes [`CinematicTitle`] here with the
//! card's age on the scenario's own pause-frozen clock. The card fades in,
//! holds for its authored seconds, and fades out. Nothing has to take it down:
//! a scene that is cancelled, deadlined or torn down simply stops posting, and
//! the card that is already up finishes its own hold.
//!
//! Untagged by `HudTier` for the same reason the skip prompt is: not because a
//! scene drops the HUD level (nothing does - that toggle is the player's), but
//! so the player's own toggle cannot delete a shot's title while the shot keeps
//! running. Untagged means not HUD-managed, so the card outlives the level.
//!
//! The CORNER is authored, not chosen here. Every corner of a Nova screen is
//! spoken for by something at some point (the comms stack owns bottom-left, the
//! status bar the top strip), and only the author of a shot knows which one is
//! free while it plays.

use bevy::prelude::*;
use nova_ui::theme;

/// The `CinematicTitle` resource, its card and the corner it sits in.
pub mod prelude {
    pub use super::{CinematicTitle, ScreenCorner, TitleCard};
}

/// Which corner of the screen a card sits in.
///
/// Mirrors nova_scenario's `ScreenCornerConfig`, the same split `StoryFeed` and
/// `HudReadouts` make: the HUD cannot depend on the scenario crate.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
pub enum ScreenCorner {
    /// Under the status bar, above everything else. The default free corner.
    #[default]
    TopLeft,
    /// Opposite the status bar's version stamp.
    TopRight,
    /// Where the comms stack lives - free only in a scene with no dialogue.
    BottomLeft,
    /// Free unless a scene has put something there.
    BottomRight,
}

impl ScreenCorner {
    /// Whether this corner hangs its content off the LEFT edge, which is also
    /// the side the accent rule sits on.
    fn is_left(self) -> bool {
        matches!(self, ScreenCorner::TopLeft | ScreenCorner::BottomLeft)
    }

    /// Whether this corner hangs its content off the TOP edge.
    fn is_top(self) -> bool {
        matches!(self, ScreenCorner::TopLeft | ScreenCorner::TopRight)
    }
}

/// One posted title card, as the HUD needs it.
#[derive(Clone, Debug, PartialEq)]
pub struct TitleCard {
    /// Where it sits.
    pub corner: ScreenCorner,
    /// The place, drawn largest.
    pub location: String,
    /// The stamp under it.
    pub date: String,
    /// One line of context. Empty draws no line.
    pub note: String,
    /// Seconds since the card was posted, on the scenario's pause-frozen
    /// clock: a card holds through a pause rather than bleeding away behind
    /// the menu.
    pub age: f32,
    /// The authored hold, fades included.
    pub seconds: f32,
}

/// The card on screen right now, or `None`.
///
/// Written every frame by nova_scenario's event-world sync. Lives here rather
/// than in the scenario crate because the HUD cannot depend on nova_scenario -
/// the same split `StoryFeed` and `GameObjectives` make.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct CinematicTitle {
    /// The live card.
    pub card: Option<TitleCard>,
}

/// Marker for the card's root.
#[derive(Component, Debug)]
struct CinematicTitleMarker;

/// Marker for the location line.
#[derive(Component, Debug)]
struct TitleLocation;

/// Marker for the date line.
#[derive(Component, Debug)]
struct TitleDate;

/// Marker for the note line.
#[derive(Component, Debug)]
struct TitleNote;

/// How far the card sits off the screen edges it hangs from.
const CARD_INSET_X_PX: f32 = 28.0;
const CARD_INSET_TOP_PX: f32 = 48.0;
const CARD_INSET_BOTTOM_PX: f32 = 96.0;

/// The accent rule down the card's outer edge.
const CARD_RULE_PX: f32 = 2.0;

/// Seconds the card takes to arrive, and to leave. A card whose authored hold
/// is shorter than the two together never reaches full strength, which is the
/// honest reading of "show this for half a second".
const CARD_FADE_IN_SECS: f32 = 0.6;
const CARD_FADE_OUT_SECS: f32 = 0.9;

/// The cinematic title card.
pub struct CinematicTitlePlugin;

impl Plugin for CinematicTitlePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CinematicTitle>();
        app.register_type::<ScreenCorner>();
        app.add_systems(Startup, spawn_cinematic_title);
        app.add_systems(Update, sync_cinematic_title.in_set(super::NovaHudSystems));
    }
}

fn spawn_cinematic_title(mut commands: Commands) {
    commands
        .spawn((
            Name::new("CinematicTitleCard"),
            CinematicTitleMarker,
            Visibility::Hidden,
            Node {
                position_type: PositionType::Absolute,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(3.0),
                padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                border: UiRect::left(Val::Px(CARD_RULE_PX)),
                ..default()
            },
            BorderColor::all(theme::AMBER_NOVA),
            BackgroundColor(theme::SPACE.with_alpha(0.55)),
        ))
        .with_children(|card| {
            card.spawn((
                TitleLocation,
                Text::new(""),
                TextFont::from_font_size(20.0),
                TextColor(theme::AMBER_HI),
            ));
            card.spawn((
                TitleDate,
                Text::new(""),
                TextFont::from_font_size(12.0),
                TextColor(theme::AMBER_NOVA),
            ));
            card.spawn((
                TitleNote,
                Text::new(""),
                TextFont::from_font_size(11.0),
                TextColor(theme::AMBER_LO),
            ));
        });
}

/// How strongly the card draws at this point in its life: in over
/// [`CARD_FADE_IN_SECS`], out over [`CARD_FADE_OUT_SECS`], full in between.
///
/// The two ramps are taken as a MINIMUM rather than as branches, so a hold too
/// short for both simply peaks lower instead of snapping.
fn card_strength(age: f32, seconds: f32) -> f32 {
    let arriving = age / CARD_FADE_IN_SECS;
    let leaving = (seconds - age) / CARD_FADE_OUT_SECS;
    arriving.min(leaving).clamp(0.0, 1.0)
}

/// Place, fill and fade the card.
#[expect(clippy::type_complexity, reason = "one query per marked text line")]
fn sync_cinematic_title(
    title: Res<CinematicTitle>,
    mut q_card: Query<
        (
            &mut Visibility,
            &mut Node,
            &mut BorderColor,
            &mut BackgroundColor,
        ),
        With<CinematicTitleMarker>,
    >,
    mut q_location: Query<(&mut Text, &mut TextColor), With<TitleLocation>>,
    mut q_date: Query<(&mut Text, &mut TextColor), (With<TitleDate>, Without<TitleLocation>)>,
    mut q_note: Query<
        (&mut Text, &mut TextColor, &mut Node),
        (
            With<TitleNote>,
            Without<TitleLocation>,
            Without<TitleDate>,
            // The root also writes a `Node`; the markers are disjoint but the
            // access checker needs to be told so.
            Without<CinematicTitleMarker>,
        ),
    >,
) {
    let card = title.card.as_ref();
    let strength = card.map_or(0.0, |card| card_strength(card.age, card.seconds));
    for (mut visibility, mut node, mut border, mut background) in &mut q_card {
        let Some(card) = card.filter(|_| strength > 0.0) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        *visibility = Visibility::Inherited;
        let corner = card.corner;
        let inset = Val::Px(CARD_INSET_X_PX);
        (node.left, node.right) = if corner.is_left() {
            (inset, Val::Auto)
        } else {
            (Val::Auto, inset)
        };
        (node.top, node.bottom) = if corner.is_top() {
            (Val::Px(CARD_INSET_TOP_PX), Val::Auto)
        } else {
            (Val::Auto, Val::Px(CARD_INSET_BOTTOM_PX))
        };
        // The rule stays on the OUTER edge and the lines stack against it, so
        // a right-hand card reads as a mirror of a left-hand one rather than
        // as a left-hand one pushed across the screen.
        node.border = if corner.is_left() {
            UiRect::left(Val::Px(CARD_RULE_PX))
        } else {
            UiRect::right(Val::Px(CARD_RULE_PX))
        };
        node.align_items = if corner.is_left() {
            AlignItems::FlexStart
        } else {
            AlignItems::FlexEnd
        };
        *border = BorderColor::all(theme::AMBER_NOVA.with_alpha(strength));
        background.0 = theme::SPACE.with_alpha(0.55 * strength);
    }
    let Some(card) = card else {
        return;
    };
    let line = |text: &mut Text, color: &mut TextColor, wanted: &str, base: Color| {
        if text.0 != wanted {
            text.0 = wanted.to_string();
        }
        color.0 = base.with_alpha(strength);
    };
    for (mut text, mut color) in &mut q_location {
        line(&mut text, &mut color, &card.location, theme::AMBER_HI);
    }
    for (mut text, mut color) in &mut q_date {
        line(&mut text, &mut color, &card.date, theme::AMBER_NOVA);
    }
    for (mut text, mut color, mut node) in &mut q_note {
        line(&mut text, &mut color, &card.note, theme::AMBER_LO);
        // An authored empty note is "no third line", not a blank one: a text
        // node with no glyphs still takes its line height in the column.
        node.display = if card.note.is_empty() {
            Display::None
        } else {
            Display::Flex
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn title_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<CinematicTitle>();
        app.add_systems(Startup, spawn_cinematic_title);
        app.add_systems(Update, sync_cinematic_title);
        app
    }

    fn card(corner: ScreenCorner, age: f32) -> TitleCard {
        TitleCard {
            corner,
            location: "MERIDIAN, ROCK PLATE 4".to_string(),
            date: "2481.114 / 0640 SHIP".to_string(),
            note: "412 aboard. Nine years on station.".to_string(),
            age,
            seconds: 8.0,
        }
    }

    fn post(app: &mut App, card: Option<TitleCard>) {
        app.world_mut().resource_mut::<CinematicTitle>().card = card;
        app.update();
    }

    fn root(app: &mut App) -> (Visibility, Node) {
        let mut query = app
            .world_mut()
            .query_filtered::<(&Visibility, &Node), With<CinematicTitleMarker>>();
        let (visibility, node) = query.single(app.world()).expect("the card root exists");
        (*visibility, node.clone())
    }

    fn location_text(app: &mut App) -> String {
        app.world_mut()
            .query_filtered::<&Text, With<TitleLocation>>()
            .single(app.world())
            .expect("the card has a location line")
            .0
            .clone()
    }

    fn note_display(app: &mut App) -> Display {
        app.world_mut()
            .query_filtered::<&Node, With<TitleNote>>()
            .single(app.world())
            .expect("the card has a note line")
            .display
    }

    /// The card is up only while one is posted, and prints what was authored.
    #[test]
    fn the_card_follows_the_posting() {
        let mut app = title_app();
        app.update();
        assert_eq!(root(&mut app).0, Visibility::Hidden, "nothing posted");

        post(&mut app, Some(card(ScreenCorner::TopLeft, 2.0)));
        assert_eq!(root(&mut app).0, Visibility::Inherited);
        assert_eq!(location_text(&mut app), "MERIDIAN, ROCK PLATE 4");

        post(&mut app, None);
        assert_eq!(
            root(&mut app).0,
            Visibility::Hidden,
            "the card was taken down"
        );
    }

    /// Each corner hangs the card off its own two edges and puts the accent
    /// rule on the outer one. A card must never resolve BOTH edges of an axis,
    /// which would stretch it across the screen.
    #[test]
    fn every_corner_hangs_the_card_off_its_own_edges() {
        let mut app = title_app();
        app.update();
        for (corner, left, top, rule_left) in [
            (ScreenCorner::TopLeft, true, true, true),
            (ScreenCorner::TopRight, false, true, false),
            (ScreenCorner::BottomLeft, true, false, true),
            (ScreenCorner::BottomRight, false, false, false),
        ] {
            post(&mut app, Some(card(corner, 2.0)));
            let (_, node) = root(&mut app);
            assert_eq!(
                (node.left == Val::Auto, node.right == Val::Auto),
                (!left, left),
                "{corner:?} resolves the wrong horizontal edge"
            );
            assert_eq!(
                (node.top == Val::Auto, node.bottom == Val::Auto),
                (!top, top),
                "{corner:?} resolves the wrong vertical edge"
            );
            assert_eq!(
                (node.border.left, node.border.right),
                if rule_left {
                    (Val::Px(CARD_RULE_PX), Val::Px(0.0))
                } else {
                    (Val::Px(0.0), Val::Px(CARD_RULE_PX))
                },
                "{corner:?} draws its rule on the inner edge"
            );
        }
    }

    /// The card arrives, holds, and leaves. A posting whose hold has run out
    /// draws nothing rather than snapping off at full strength.
    #[test]
    fn the_card_fades_in_holds_and_fades_out() {
        assert_eq!(card_strength(0.0, 8.0), 0.0, "it arrives from nothing");
        assert!(card_strength(0.3, 8.0) > 0.0 && card_strength(0.3, 8.0) < 1.0);
        assert_eq!(card_strength(4.0, 8.0), 1.0, "it holds at full");
        assert!(card_strength(7.6, 8.0) < 1.0, "it is already leaving");
        assert_eq!(card_strength(8.0, 8.0), 0.0, "and it is gone on time");
        assert_eq!(card_strength(99.0, 8.0), 0.0, "and stays gone");
    }

    /// A hold shorter than the two fades peaks lower instead of snapping on
    /// and off at full strength.
    #[test]
    fn a_hold_too_short_for_both_fades_peaks_lower() {
        let short = 0.6_f32;
        let peak = (0..=60_u8)
            .map(|step| card_strength(short * f32::from(step) / 60.0, short))
            .fold(0.0_f32, f32::max);
        assert!(peak > 0.0, "a short card must still be visible");
        assert!(peak < 1.0, "a short card must never reach full strength");
    }

    /// A card with nothing to add is two lines, not three with a gap.
    #[test]
    fn an_empty_note_draws_no_line() {
        let mut app = title_app();
        app.update();
        post(&mut app, Some(card(ScreenCorner::TopLeft, 2.0)));
        assert_eq!(note_display(&mut app), Display::Flex);

        let mut quiet = card(ScreenCorner::TopLeft, 2.0);
        quiet.note = String::new();
        post(&mut app, Some(quiet));
        assert_eq!(note_display(&mut app), Display::None);
    }
}
