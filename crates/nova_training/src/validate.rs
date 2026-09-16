//! Pure validation over authored lessons: the checks that decide whether a
//! lesson is well-formed, and whether the range its Practice button names is
//! really a practice range.
//!
//! PURE, and deliberately engine-free of the scenario crate: the caller hands
//! in the scenario ids it knows and which of them declare `role: Lesson`, so
//! the same function serves the offline `content lint` walk (which reads a mod
//! tree off disk) and the runtime bundle merge (which is the only place a
//! cross-mod reference is decidable at all).

use std::collections::HashSet;

use crate::{
    catalog::{body_words, Lesson, LessonId, LessonMedia, LESSON_BODY_MAX_WORDS},
    facts::{wrap_note_lines, FIELD_NOTE_LINES, FIELD_NOTE_LINE_CHARS},
};

/// How bad a finding is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LessonSeverity {
    /// The lesson is not usable: the handbook must not offer it as authored.
    Error,
    /// Worth fixing, but the lesson still draws.
    Warn,
}

/// One finding about one lesson.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LessonIssue {
    /// How bad it is.
    pub severity: LessonSeverity,
    /// The lesson the finding is about - the key a report groups on.
    pub lesson: LessonId,
    /// The whole message, already naming the lesson.
    pub message: String,
}

impl LessonIssue {
    /// An Error-level finding.
    pub fn error(lesson: &str, message: String) -> Self {
        Self {
            severity: LessonSeverity::Error,
            lesson: lesson.to_string(),
            message,
        }
    }

    /// A Warn-level finding.
    pub fn warn(lesson: &str, message: String) -> Self {
        Self {
            severity: LessonSeverity::Warn,
            lesson: lesson.to_string(),
            message,
        }
    }
}

/// Lint one lesson against the scenarios the caller knows about.
///
/// `known_scenarios` is every registered scenario id (base plus every enabled
/// bundle); `practice_ranges` is the subset of those that declare
/// `role: Lesson`; `launchable` is every scenario a player can actually start,
/// which is everything that is not a menu backdrop. All three are the
/// caller's, because none of them is decidable from a lesson alone.
///
/// Checks:
/// - an empty id, title, body, wiki path or media alt text is an Error - a
///   required field that is present but blank draws a blank screen;
/// - a body over [`LESSON_BODY_MAX_WORDS`] is an Error: the cap is what the
///   text box holds at the supported narrow window, so prose over it is prose
///   the floor cannot show;
/// - a loop with an empty grid, no frames, more frames than cells, or a
///   non-positive rate is an Error - all four play nothing;
/// - a `practice` naming a scenario no bundle provides is an Error, the same
///   class as a campaign's dangling member;
/// - a `practice` naming a scenario that is NOT a practice range is an Error:
///   a lesson hands off to a focused range built for it, and a Practice button
///   that launched a campaign chapter would be a lesson pretending to be one;
/// - a `proven_by` naming a scenario no bundle provides is an Error, and one
///   naming a menu BACKDROP is an Error: a lesson is proven by a flight the
///   player can be sent on, and scenery nobody launches would leave a lesson
///   that can never be completed;
/// - a duplicate inside `proven_by` is an Error - the same claim twice is an
///   authoring slip, not two ways to prove the lesson;
/// - a field note that does not wrap onto the slot's lines is an Error - the
///   loading screen cannot scroll, so a note over the box is a note that
///   pushes the panel around.
pub fn lint_lesson(
    lesson: &Lesson,
    known_scenarios: &HashSet<String>,
    practice_ranges: &HashSet<String>,
    launchable: &HashSet<String>,
) -> Vec<LessonIssue> {
    let id = lesson.id.as_str();
    let mut issues = Vec::new();

    for (field, value) in [
        ("id", id),
        ("title", lesson.title.as_str()),
        ("body", lesson.body.as_str()),
        ("wiki_path", lesson.wiki_path.as_str()),
        ("media alt text", lesson.media.alt()),
    ] {
        if value.trim().is_empty() {
            issues.push(LessonIssue::error(
                id,
                format!("lesson '{id}' has an empty {field}"),
            ));
        }
    }

    let words = body_words(&lesson.body);
    if words > LESSON_BODY_MAX_WORDS {
        issues.push(LessonIssue::error(
            id,
            format!(
                "lesson '{id}' spells {words} words, over the {LESSON_BODY_MAX_WORDS}-word text \
                 box; a tip that needs more than that is two tips"
            ),
        ));
    }

    if let LessonMedia::Loop {
        columns,
        rows,
        frames,
        frames_per_second,
        ..
    } = &lesson.media
    {
        let cells = u64::from(*columns) * u64::from(*rows);
        if cells == 0 {
            issues.push(LessonIssue::error(
                id,
                format!("lesson '{id}' has a loop sheet with a {columns}x{rows} grid, which holds no frames"),
            ));
        } else if *frames == 0 {
            issues.push(LessonIssue::error(
                id,
                format!("lesson '{id}' has a loop sheet declaring 0 frames"),
            ));
        } else if u64::from(*frames) > cells {
            issues.push(LessonIssue::error(
                id,
                format!(
                    "lesson '{id}' has a loop sheet declaring {frames} frames, more than the \
                     {cells} its {columns}x{rows} grid holds"
                ),
            ));
        }
        if !(*frames_per_second > 0.0) {
            issues.push(LessonIssue::error(
                id,
                format!(
                    "lesson '{id}' has a loop sheet at {frames_per_second} frames per second, \
                     which never advances"
                ),
            ));
        }
    }

    for note in &lesson.field_notes {
        if wrap_note_lines(note).is_none() {
            issues.push(LessonIssue::error(
                id,
                format!(
                    "lesson '{id}' has a field note that does not fit {FIELD_NOTE_LINES} lines \
                     of {FIELD_NOTE_LINE_CHARS}: {note:?}"
                ),
            ));
        }
    }

    if let Some(scenario) = &lesson.practice {
        if !known_scenarios.contains(scenario) {
            issues.push(LessonIssue::error(
                id,
                format!(
                    "lesson '{id}' practises in scenario '{scenario}', which no bundle provides"
                ),
            ));
        } else if !practice_ranges.contains(scenario) {
            issues.push(LessonIssue::error(
                id,
                format!(
                    "lesson '{id}' practises in scenario '{scenario}', which is not a practice \
                     range; a lesson hands off to a focused scenario authored for it - declare \
                     that scenario `role: Lesson`"
                ),
            ));
        }
    }

    // What may claim this lesson. Unlike `practice`, a CHAPTER is welcome here:
    // Basic Training is the one flow that proves several lessons at once, and
    // it is a chapter because a player starts it from the menu.
    let mut seen: HashSet<&str> = HashSet::new();
    for scenario in &lesson.proven_by {
        if !seen.insert(scenario.as_str()) {
            issues.push(LessonIssue::error(
                id,
                format!("lesson '{id}' names scenario '{scenario}' twice in `proven_by`"),
            ));
            continue;
        }
        if !known_scenarios.contains(scenario) {
            issues.push(LessonIssue::error(
                id,
                format!(
                    "lesson '{id}' is proven by scenario '{scenario}', which no bundle provides"
                ),
            ));
        } else if !launchable.contains(scenario) {
            issues.push(LessonIssue::error(
                id,
                format!(
                    "lesson '{id}' is proven by scenario '{scenario}', which is a menu backdrop; \
                     a lesson is proven by a flight the player can be sent on, and nothing \
                     launches scenery"
                ),
            ));
        }
    }

    issues
}

/// [`lint_lesson`] over a whole set.
pub fn lint_lessons<'a>(
    lessons: impl IntoIterator<Item = &'a Lesson>,
    known_scenarios: &HashSet<String>,
    practice_ranges: &HashSet<String>,
    launchable: &HashSet<String>,
) -> Vec<LessonIssue> {
    lessons
        .into_iter()
        .flat_map(|lesson| lint_lesson(lesson, known_scenarios, practice_ranges, launchable))
        .collect()
}

/// The one thing only a SET can answer: a practice range nothing sends the
/// player to.
///
/// `owned` is the ranges the caller is answerable for - the merge judges every
/// installed range, and the offline walk judges the ones each bundle ships -
/// while `lessons` is every lesson that could reach them, because a mod's
/// lesson may practise in a base range. The two sets are separate for exactly
/// that reason: a caller that judged base's ranges against its own lessons
/// would report every base range as unreachable.
///
/// A Warn rather than an Error - an unreachable range costs a load nobody
/// pays, and a mod may ship one ahead of the lesson that uses it - but it is
/// almost always a renamed id.
pub fn unused_practice_ranges<'a>(
    lessons: impl IntoIterator<Item = &'a Lesson>,
    owned: &HashSet<String>,
) -> Vec<LessonIssue> {
    let practised: HashSet<&str> = lessons
        .into_iter()
        .filter_map(|lesson| lesson.practice.as_deref())
        .collect();
    let mut orphans: Vec<&String> = owned
        .iter()
        .filter(|range| !practised.contains(range.as_str()))
        .collect();
    orphans.sort();
    orphans
        .into_iter()
        .map(|range| {
            LessonIssue::warn(
                range,
                format!(
                    "scenario '{range}' declares `role: Lesson` but no lesson practises in it; \
                     the Scenarios picker renders no row for one, so nothing can reach it"
                ),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use nova_gameplay::prelude::AssetRef;

    use super::*;
    use crate::catalog::{LessonCategory, LessonMedia};

    fn known(ids: &[&str]) -> HashSet<String> {
        ids.iter().map(|s| (*s).to_string()).collect()
    }

    fn lesson(practice: Option<&str>) -> Lesson {
        Lesson {
            id: "flight_stop".to_string(),
            category: LessonCategory::Flight,
            order: 30,
            title: "STOP is an order".to_string(),
            media: LessonMedia::Image {
                image: AssetRef::from("self://training/flight_stop.png"),
                alt: "the flight computer burning to zero".to_string(),
            },
            body: "STOP hands the ship to the flight computer.".to_string(),
            actions: vec!["autopilot_stop".to_string()],
            wiki_path: "wiki/flight".to_string(),
            practice: practice.map(str::to_string),
            proven_by: vec![],
            field_notes: vec![],
        }
    }

    /// The same fixture, with the scenarios that may claim the lesson.
    fn lesson_proven(practice: Option<&str>, proven_by: &[&str]) -> Lesson {
        Lesson {
            proven_by: proven_by.iter().map(|id| (*id).to_string()).collect(),
            ..lesson(practice)
        }
    }

    fn errors(issues: &[LessonIssue]) -> Vec<&LessonIssue> {
        issues
            .iter()
            .filter(|issue| issue.severity == LessonSeverity::Error)
            .collect()
    }

    #[test]
    fn a_well_formed_lesson_with_a_real_range_lints_clean() {
        let issues = lint_lesson(
            &lesson(Some("drill_stop")),
            &known(&["tutorial", "drill_stop"]),
            &known(&["drill_stop"]),
            &known(&["tutorial", "drill_stop"]),
        );
        assert!(issues.is_empty(), "{issues:?}");
    }

    #[test]
    fn a_lesson_with_nothing_to_fly_lints_clean() {
        let issues = lint_lesson(
            &lesson(None),
            &known(&["tutorial"]),
            &known(&[]),
            &known(&["tutorial"]),
        );
        assert!(issues.is_empty(), "{issues:?}");
    }

    #[test]
    fn a_practice_scenario_no_bundle_provides_is_an_error() {
        let issues = lint_lesson(
            &lesson(Some("drill_ghost")),
            &known(&["tutorial"]),
            &known(&[]),
            &known(&["tutorial"]),
        );
        let errs = errors(&issues);
        assert_eq!(errs.len(), 1, "{issues:?}");
        assert!(
            errs[0].message.contains("drill_ghost"),
            "{}",
            errs[0].message
        );
    }

    /// The rule the whole practice contract rests on: a lesson may only hand
    /// off to a range authored for it. Naming the campaign start - a real,
    /// launchable, eleven-beat scenario - is exactly the mistake this refuses.
    #[test]
    fn practising_in_a_chapter_rather_than_a_range_is_an_error() {
        let issues = lint_lesson(
            &lesson(Some("tutorial")),
            &known(&["tutorial", "drill_stop"]),
            &known(&["drill_stop"]),
            &known(&["tutorial", "drill_stop"]),
        );
        let errs = errors(&issues);
        assert_eq!(errs.len(), 1, "{issues:?}");
        assert!(
            errs[0].message.contains("not a practice range")
                && errs[0].message.contains("role: Lesson"),
            "the finding says what to do about it: {}",
            errs[0].message
        );
    }

    #[test]
    fn a_body_over_the_box_is_an_error() {
        let mut long = lesson(None);
        long.body = "word ".repeat(LESSON_BODY_MAX_WORDS + 1);
        let issues = lint_lesson(&long, &known(&[]), &known(&[]), &known(&[]));
        let errs = errors(&issues);
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert!(errs[0].message.contains("text box"), "{}", errs[0].message);
    }

    #[test]
    fn a_blank_required_field_is_an_error() {
        let mut blank = lesson(None);
        blank.title = "   ".to_string();
        let issues = lint_lesson(&blank, &known(&[]), &known(&[]), &known(&[]));
        let errs = errors(&issues);
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert!(
            errs[0].message.contains("empty title"),
            "{}",
            errs[0].message
        );
    }

    #[test]
    fn a_loop_declaring_more_frames_than_its_grid_holds_is_an_error() {
        let mut bad = lesson(None);
        bad.media = LessonMedia::Loop {
            sheet: AssetRef::from("self://training/x.png"),
            columns: 4,
            rows: 2,
            frames: 12,
            frames_per_second: 12.0,
            alt: "a loop".to_string(),
        };
        let issues = lint_lesson(&bad, &known(&[]), &known(&[]), &known(&[]));
        let errs = errors(&issues);
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert!(
            errs[0].message.contains("more than the 8"),
            "{}",
            errs[0].message
        );
    }

    #[test]
    fn a_loop_that_never_advances_is_an_error() {
        let mut bad = lesson(None);
        bad.media = LessonMedia::Loop {
            sheet: AssetRef::from("self://training/x.png"),
            columns: 4,
            rows: 2,
            frames: 8,
            frames_per_second: 0.0,
            alt: "a loop".to_string(),
        };
        let issues = lint_lesson(&bad, &known(&[]), &known(&[]), &known(&[]));
        let errs = errors(&issues);
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert!(
            errs[0].message.contains("never advances"),
            "{}",
            errs[0].message
        );
    }

    /// A range nothing practises in is unreachable: the picker lists no row
    /// for it and no Practice button names it.
    #[test]
    fn a_field_note_that_outgrows_the_slot_is_an_error() {
        let mut long = lesson(None);
        long.field_notes = vec!["word ".repeat(40)];
        let issues = lint_lesson(&long, &known(&[]), &known(&[]), &known(&[]));
        let errs = errors(&issues);
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert!(
            errs[0].message.contains("field note"),
            "{}",
            errs[0].message
        );
    }

    /// The point of the second field: Basic Training proves several lessons at
    /// once and is nobody's Practice button, so a CHAPTER is legitimate here
    /// even though `practice` refuses one.
    #[test]
    fn a_chapter_may_prove_a_lesson_even_though_it_may_not_be_its_practice() {
        let issues = lint_lesson(
            &lesson_proven(Some("drill_stop"), &["tutorial", "drill_stop"]),
            &known(&["tutorial", "drill_stop"]),
            &known(&["drill_stop"]),
            &known(&["tutorial", "drill_stop"]),
        );
        assert!(issues.is_empty(), "{issues:?}");
    }

    #[test]
    fn a_proving_scenario_no_bundle_provides_is_an_error() {
        let issues = lint_lesson(
            &lesson_proven(None, &["drill_ghost"]),
            &known(&["tutorial"]),
            &known(&[]),
            &known(&["tutorial"]),
        );
        let errs = errors(&issues);
        assert_eq!(errs.len(), 1, "{issues:?}");
        assert!(
            errs[0].message.contains("drill_ghost") && errs[0].message.contains("proven by"),
            "{}",
            errs[0].message
        );
    }

    /// Scenery cannot prove anything, because nothing sends a player into it -
    /// a lesson claiming otherwise could never be completed.
    #[test]
    fn a_backdrop_cannot_prove_a_lesson() {
        let issues = lint_lesson(
            &lesson_proven(None, &["main_menu"]),
            &known(&["tutorial", "main_menu"]),
            &known(&[]),
            &known(&["tutorial"]),
        );
        let errs = errors(&issues);
        assert_eq!(errs.len(), 1, "{issues:?}");
        assert!(
            errs[0].message.contains("menu backdrop"),
            "{}",
            errs[0].message
        );
    }

    #[test]
    fn naming_the_same_prover_twice_is_an_error() {
        let issues = lint_lesson(
            &lesson_proven(None, &["tutorial", "tutorial"]),
            &known(&["tutorial"]),
            &known(&[]),
            &known(&["tutorial"]),
        );
        let errs = errors(&issues);
        assert_eq!(errs.len(), 1, "{issues:?}");
        assert!(errs[0].message.contains("twice"), "{}", errs[0].message);
    }

    #[test]
    fn a_practice_range_no_lesson_uses_warns() {
        let issues = unused_practice_ranges(
            [&lesson(Some("drill_stop"))],
            &known(&["drill_stop", "drill_orphan"]),
        );
        assert!(errors(&issues).is_empty(), "{issues:?}");
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert_eq!(issues[0].severity, LessonSeverity::Warn);
        assert!(
            issues[0].message.contains("drill_orphan"),
            "{}",
            issues[0].message
        );
    }

    /// The set a range is judged against is not the set of lessons that may
    /// reach it: a mod's lesson practising in a base range keeps that range
    /// reachable, and a caller that judged base's ranges against the mod's own
    /// lessons alone would report every one of them as an orphan.
    #[test]
    fn a_range_another_bundles_lesson_practises_in_is_not_an_orphan() {
        let issues = unused_practice_ranges([&lesson(Some("drill_stop"))], &known(&["drill_stop"]));
        assert!(issues.is_empty(), "{issues:?}");
    }
}
