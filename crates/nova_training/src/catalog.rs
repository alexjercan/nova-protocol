//! What a lesson is: its identity, the category it files under, the one screen
//! it draws, and the catalog that holds them in a stable order.
//!
//! These types are the AUTHORING surface. A lesson is mod content - one
//! `Lesson((..))` item in an ordinary `*.content.ron` file - so everything here
//! is `serde` and everything a lesson needs is an explicit field. The base game
//! ships its lessons through the base bundle and an enabled mod adds or
//! overlays them by id, through the same merge every other content kind uses.

use bevy::prelude::*;
use nova_gameplay::prelude::AssetRef;
use serde::{Deserialize, Serialize};

/// A lesson's stable id. The string a progress record, a field note and a
/// practice hand-off all name it by, so it outlives any reordering of the
/// catalog.
///
/// It also denotes a LEARNING OUTCOME, not a screen: a mod that overlays a
/// base lesson id may change the presentation, but a persisted record saying
/// the player proved that id has to still mean the same thing afterwards.
pub type LessonId = String;

/// The handbook's top-level groups, in the order the list draws them.
///
/// The SET is fixed and the ORDER is this declaration: a category with no
/// lessons simply does not draw a header, so the first shipped content set can
/// be smaller than the information architecture without the list learning a
/// second ordering rule. A mod adds lessons to these groups rather than adding
/// a group, so the handbook a player learns to read stays the same shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum LessonCategory {
    /// The first thing a new pilot reads.
    StartHere,
    /// Flying the hull: thrust, momentum, the autopilot orders.
    Flight,
    /// Weapons, locks, and what a fight asks of the flight computer.
    Combat,
    /// The editor: sections, mass, and what a hull is made of.
    Shipbuilding,
    /// The cockpit computer and its apps.
    NovaOs,
    /// Everything that needs the rest first.
    Advanced,
}

impl LessonCategory {
    /// Every category, in draw order.
    pub const ORDER: [LessonCategory; 6] = [
        LessonCategory::StartHere,
        LessonCategory::Flight,
        LessonCategory::Combat,
        LessonCategory::Shipbuilding,
        LessonCategory::NovaOs,
        LessonCategory::Advanced,
    ];

    /// The header a category draws above its rows.
    pub fn title(self) -> &'static str {
        match self {
            LessonCategory::StartHere => "Start Here",
            LessonCategory::Flight => "Flight",
            LessonCategory::Combat => "Combat",
            LessonCategory::Shipbuilding => "Shipbuilding",
            LessonCategory::NovaOs => "NOVA OS",
            LessonCategory::Advanced => "Advanced",
        }
    }

    /// This category's place in [`LessonCategory::ORDER`] - the first key the
    /// catalog sorts on.
    pub fn rank(self) -> usize {
        LessonCategory::ORDER
            .iter()
            .position(|category| *category == self)
            .unwrap_or(LessonCategory::ORDER.len())
    }
}

/// The visual a lesson page carries, above its text box.
///
/// REQUIRED on every lesson, and that is a layout decision rather than an
/// authoring preference: the frame takes the width of the details pane at a
/// fixed 16:9, so a lesson without one would reflow the whole page as the
/// player walks down the list.
///
/// A loop is an animated SPRITE SHEET, not a video file: one image through
/// bevy's ordinary texture pipeline plays the same on native and on wasm, with
/// no codec and no second asset loader. The grid, the frame count and the rate
/// are all explicit - a sheet that has to be measured at load is a sheet that
/// is silently wrong when the art changes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum LessonMedia {
    /// A still image.
    Image {
        /// The picture, as a mod resource ref (`self://` / `dep://`).
        image: AssetRef<Image>,
        /// What the frame says while the image loads, and what it keeps
        /// saying if the image never arrives. Also the lesson's alt text.
        alt: String,
    },
    /// A looping demonstration, played from a sprite sheet.
    Loop {
        /// The sheet, as a mod resource ref (`self://` / `dep://`).
        sheet: AssetRef<Image>,
        /// Frame columns in the sheet.
        columns: u32,
        /// Frame rows in the sheet.
        rows: u32,
        /// How many of the `columns * rows` cells are real frames, read in
        /// row-major order. Authored rather than derived so a sheet with a
        /// partly-filled last row plays its frames and not its padding.
        frames: u32,
        /// Playback rate, in frames per second.
        frames_per_second: f32,
        /// What the frame says while the sheet loads, and what it keeps saying
        /// if the sheet never arrives. Also the lesson's alt text.
        alt: String,
    },
}

impl LessonMedia {
    /// The line the media frame draws when it has no picture to draw.
    pub fn alt(&self) -> &str {
        match self {
            LessonMedia::Image { alt, .. } | LessonMedia::Loop { alt, .. } => alt,
        }
    }

    /// The image this media loads, whichever shape it is.
    pub fn source(&self) -> &AssetRef<Image> {
        match self {
            LessonMedia::Image { image, .. } => image,
            LessonMedia::Loop { sheet, .. } => sheet,
        }
    }

    /// Whether this is a moving demonstration rather than a still.
    pub fn is_loop(&self) -> bool {
        matches!(self, LessonMedia::Loop { .. })
    }
}

/// The most words a lesson body may carry.
///
/// Not a style preference: it is what the text box holds at the 640x600 floor
/// without scrolling, under a 16:9 loop that takes the width of the pane. A tip
/// that needs more than this is two tips.
pub const LESSON_BODY_MAX_WORDS: usize = 45;

/// How many words a lesson body spells, by the same count the cap is stated in.
pub fn body_words(body: &str) -> usize {
    body.split_whitespace().count()
}

/// One lesson: one tip, on one screen.
///
/// There is no page list and no page cursor. A lesson is a demonstration and
/// the words that go with it, so anything that would have been page two is its
/// own lesson with its own id - which is also what makes a progress record mean
/// something, since Viewed now names a thing the player actually saw rather
/// than the first page of something longer.
///
/// STRICT: an unknown key is a load error. A lesson that authors one thing and
/// draws another is the failure this format exists to prevent.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lesson {
    /// Stable id. See [`LessonId`].
    pub id: LessonId,
    /// The group it files under.
    pub category: LessonCategory,
    /// Where it sits inside that group, low first.
    ///
    /// REQUIRED, and required for a reason: lessons arrive from several
    /// bundles in installed-mod order, so a catalog that fell back to arrival
    /// order would put a mod's lesson wherever the player happened to enable
    /// it. Ties break on id, so the order is total.
    pub order: i32,
    /// The row's label, and the lead of the text box.
    pub title: String,
    /// The demonstration above the text. See [`LessonMedia`].
    pub media: LessonMedia,
    /// The whole tip, at most [`LESSON_BODY_MAX_WORDS`] words.
    pub body: String,
    /// The actions this tip is about, drawn as LIVE binding chips under the
    /// body.
    ///
    /// Registry action names (`InputBindings::get`), never keys: a lesson that
    /// spelled `W` in its prose would lie to a player who rebound it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<String>,
    /// Where the complete manual continues, as a stable wiki path.
    pub wiki_path: String,
    /// The scenario the Practice action launches, when the lesson has
    /// something to fly.
    ///
    /// It must be a scenario declaring `role: Lesson` - a focused range built
    /// for this lesson. Naming a chapter is a lint Error, which is what stops
    /// a Practice button quietly handing the player the campaign start. The
    /// button's LABEL is not authored: the handbook draws one word for every
    /// lesson, because a row of differently-worded buttons reads as a row of
    /// different actions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub practice: Option<String>,
    /// The scenarios whose Victory PROVES this lesson, which is the only thing
    /// that may mark it [`Completed`](crate::progress::LessonStatus::Completed).
    ///
    /// Separate from [`Self::practice`], which is a BUTTON. The two usually
    /// name the same range, but they answer different questions and a lesson
    /// must answer both explicitly: Basic Training proves eight lessons and is
    /// nobody's Practice button, and a range may exist to be flown without
    /// being the proof of anything. Empty means nothing in the game can claim
    /// this lesson - reading it is all it will ever be.
    ///
    /// Each id must name a scenario the player can launch. A backdrop is a
    /// lint Error, the same walk [`Self::practice`] goes through.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub proven_by: Vec<String>,
    /// The field notes this lesson contributes, each at most two short lines.
    ///
    /// The menu and the post-boot loading screens draw from the lesson
    /// catalog rather than from a second list, so a claim exists once and the
    /// note can open the lesson it came from.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub field_notes: Vec<String>,
}

/// Every lesson the handbook lists, in a stable order.
///
/// Order is CATEGORY first ([`LessonCategory::ORDER`]), then [`Lesson::order`],
/// then the lesson id. A TOTAL order with no arrival component in it: the
/// merge hands lessons over in bundle order, and a catalog that inherited that
/// would rearrange the handbook when the player enabled a mod.
#[derive(Resource, Default, Debug, Clone, PartialEq)]
pub struct TrainingCatalog {
    lessons: Vec<Lesson>,
}

impl TrainingCatalog {
    /// A catalog holding these lessons, sorted into the stable order above.
    pub fn new(lessons: impl IntoIterator<Item = Lesson>) -> Self {
        let mut lessons: Vec<Lesson> = lessons.into_iter().collect();
        lessons.sort_by(|a, b| {
            (a.category.rank(), a.order, &a.id).cmp(&(b.category.rank(), b.order, &b.id))
        });
        Self { lessons }
    }

    /// Every lesson, in catalog order.
    pub fn lessons(&self) -> &[Lesson] {
        &self.lessons
    }

    /// How many lessons the catalog holds.
    pub fn len(&self) -> usize {
        self.lessons.len()
    }

    /// Whether the catalog holds nothing.
    pub fn is_empty(&self) -> bool {
        self.lessons.is_empty()
    }

    /// The lesson with this id, if the catalog has one.
    pub fn get(&self, id: &str) -> Option<&Lesson> {
        self.lessons.iter().find(|lesson| lesson.id == id)
    }

    /// The first lesson's id: what the list default-selects, and where
    /// "Open lessons" lands.
    pub fn first_id(&self) -> Option<&LessonId> {
        self.lessons.first().map(|lesson| &lesson.id)
    }

    /// The catalog grouped for the list: each NON-EMPTY category with its
    /// lessons, in draw order.
    pub fn by_category(&self) -> Vec<(LessonCategory, Vec<&Lesson>)> {
        LessonCategory::ORDER
            .iter()
            .filter_map(|category| {
                let group: Vec<&Lesson> = self
                    .lessons
                    .iter()
                    .filter(|lesson| lesson.category == *category)
                    .collect();
                (!group.is_empty()).then_some((*category, group))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn lesson(id: &str, category: LessonCategory, order: i32) -> Lesson {
        Lesson {
            id: id.to_string(),
            category,
            order,
            title: id.to_string(),
            media: LessonMedia::Image {
                image: AssetRef::from("self://training/x.png"),
                alt: "a still".to_string(),
            },
            body: "body".to_string(),
            actions: vec![],
            wiki_path: "wiki/".to_string(),
            practice: None,
            proven_by: vec![],
            field_notes: vec![],
        }
    }

    #[test]
    fn the_catalog_orders_by_category_then_order_then_id() {
        let catalog = TrainingCatalog::new([
            lesson("advanced_one", LessonCategory::Advanced, 10),
            lesson("flight_late", LessonCategory::Flight, 20),
            lesson("start", LessonCategory::StartHere, 10),
            lesson("flight_early", LessonCategory::Flight, 10),
        ]);
        let ids: Vec<&str> = catalog.lessons().iter().map(|l| l.id.as_str()).collect();
        assert_eq!(
            ids,
            ["start", "flight_early", "flight_late", "advanced_one"]
        );
    }

    /// The order is TOTAL: two lessons sharing a category and an order - which
    /// two bundles authoring independently will do - still draw the same way
    /// round every run, whatever order the merge handed them over in.
    #[test]
    fn a_tie_on_order_breaks_on_the_id_not_on_arrival() {
        let forwards = TrainingCatalog::new([
            lesson("flight_b", LessonCategory::Flight, 10),
            lesson("flight_a", LessonCategory::Flight, 10),
        ]);
        let backwards = TrainingCatalog::new([
            lesson("flight_a", LessonCategory::Flight, 10),
            lesson("flight_b", LessonCategory::Flight, 10),
        ]);
        assert_eq!(forwards, backwards);
        assert_eq!(forwards.first_id().map(String::as_str), Some("flight_a"));
    }

    #[test]
    fn an_empty_category_draws_no_group() {
        let catalog = TrainingCatalog::new([
            lesson("start", LessonCategory::StartHere, 10),
            lesson("fight", LessonCategory::Combat, 10),
        ]);
        let groups: Vec<LessonCategory> = catalog
            .by_category()
            .into_iter()
            .map(|(category, _)| category)
            .collect();
        assert_eq!(groups, [LessonCategory::StartHere, LessonCategory::Combat]);
    }

    #[test]
    fn the_first_listed_lesson_is_what_open_lessons_lands_on() {
        let catalog = TrainingCatalog::new([
            lesson("fight", LessonCategory::Combat, 10),
            lesson("start", LessonCategory::StartHere, 10),
        ]);
        assert_eq!(catalog.first_id().map(String::as_str), Some("start"));
    }

    #[test]
    fn the_body_cap_counts_the_words_a_reader_sees() {
        assert_eq!(body_words("Aim first, then burn."), 4);
        assert_eq!(body_words("  spaced   out  "), 2);
        assert_eq!(body_words(""), 0);
    }

    #[test]
    fn an_unknown_id_resolves_to_nothing() {
        let catalog = TrainingCatalog::new([lesson("start", LessonCategory::StartHere, 10)]);
        assert!(catalog.get("no_such_lesson").is_none());
    }

    /// The authored form is what a mod writes by hand, so it is parsed from a
    /// literal rather than round-tripped from our own value: the defaulted
    /// fields stay optional and the media grid arrives as authored.
    #[test]
    fn a_lesson_parses_from_authored_ron() {
        let authored = r#"(
            id: "flight_momentum",
            category: Flight,
            order: 20,
            title: "Momentum",
            media: Loop(
                sheet: "self://training/flight_momentum.png",
                columns: 6,
                rows: 4,
                frames: 24,
                frames_per_second: 12,
                alt: "A ship stops thrusting and keeps moving.",
            ),
            body: "Cutting thrust does not stop the ship.",
            actions: ["main_drive"],
            wiki_path: "wiki/flight",
            practice: Some("drill_momentum"),
        )"#;
        let lesson: Lesson = ron::from_str(authored).expect("authored lesson parses");
        assert_eq!(lesson.category, LessonCategory::Flight);
        assert_eq!(lesson.practice.as_deref(), Some("drill_momentum"));
        assert!(
            lesson.field_notes.is_empty(),
            "an absent list defaults empty"
        );
        let LessonMedia::Loop {
            sheet,
            frames,
            frames_per_second,
            ..
        } = &lesson.media
        else {
            panic!("authored a Loop, parsed {:?}", lesson.media);
        };
        assert_eq!(sheet.path(), Some("self://training/flight_momentum.png"));
        assert_eq!(*frames, 24);
        assert_eq!(*frames_per_second, 12.0);
    }

    /// A misspelled or removed key must REFUSE, not be dropped: a lesson that
    /// silently loses its practice range is a Practice button that vanishes
    /// with no message.
    #[test]
    fn a_lesson_with_an_unknown_key_refuses_to_parse() {
        let authored = r#"(
            id: "x",
            category: Flight,
            order: 10,
            title: "X",
            media: Image(image: "self://a.png", alt: "a"),
            body: "b",
            wiki_path: "wiki/",
            practise: Some("drill_momentum"),
        )"#;
        let err = ron::from_str::<Lesson>(authored).expect_err("an unknown key must not parse");
        assert!(err.to_string().contains("practise"), "{err}");
    }
}
