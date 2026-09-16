//! Field notes: the two-line facts the menu shows on a card and the loading
//! screens show in their fact slot, and the rotation that keeps one from coming
//! up twice in a row.

use std::collections::VecDeque;

use bevy::prelude::*;

use crate::catalog::{LessonId, TrainingCatalog};

/// The longest a note line may be. Sized so two of them fit the loading
/// screen's slot and the menu card at the supported narrow window without
/// wrapping into a paragraph.
pub const FIELD_NOTE_LINE_CHARS: usize = 64;

/// The most lines one note may carry. Two: a fact that needs three is a
/// lesson.
pub const FIELD_NOTE_LINES: usize = 2;

/// One fact.
///
/// It names an ACTION, never a key. A note that spelled the default binding
/// would be wrong for the player who rebound it, and the surfaces that draw a
/// note - a loading screen with no input registry loaded yet among them -
/// cannot all resolve a chip.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldNote {
    /// Stable id, for the rotation's memory and for a test to name one.
    pub id: String,
    /// The lesson this note came from, when it came from one. The menu card
    /// opens it; a loading-screen note is not interactive, so it ignores this.
    pub lesson: Option<LessonId>,
    /// At most [`FIELD_NOTE_LINES`] short lines.
    pub lines: Vec<String>,
}

impl FieldNote {
    /// A note with no lesson behind it.
    pub fn new(id: &str, lines: [&str; FIELD_NOTE_LINES]) -> Self {
        Self {
            id: id.to_string(),
            lesson: None,
            lines: lines.iter().map(|line| (*line).to_string()).collect(),
        }
    }

    /// The same note, pointing at the lesson it was drawn from.
    #[must_use]
    pub fn from_lesson(mut self, lesson: &str) -> Self {
        self.lesson = Some(lesson.to_string());
        self
    }
}

/// Which notes have come up lately, so the next pick is not one of them.
///
/// SESSION state, never persisted: the rule is "not the one you just read",
/// which is about this sitting at the machine and not about the save file.
#[derive(Resource, Default, Debug)]
pub struct FieldNoteRotation {
    recent: VecDeque<String>,
}

impl FieldNoteRotation {
    /// Pick the note a surface that just opened shows, and remember it.
    ///
    /// `roll` is the caller's randomness (a `GlobalRng` draw, a frame count -
    /// anything), taken as an argument so the rotation itself is pure and a
    /// test can pin the sequence. `None` only when there are no notes at all.
    ///
    /// Everything shown recently is skipped. With every note skipped the
    /// memory clears and the whole set is eligible again, so a one-note build
    /// still shows its note rather than nothing.
    pub fn pick<'a>(&mut self, notes: &'a [FieldNote], roll: usize) -> Option<&'a FieldNote> {
        if notes.is_empty() {
            return None;
        }
        let eligible: Vec<&FieldNote> = notes
            .iter()
            .filter(|note| !self.recent.contains(&note.id))
            .collect();
        let eligible = if eligible.is_empty() {
            self.recent.clear();
            notes.iter().collect()
        } else {
            eligible
        };
        let picked = eligible[roll % eligible.len()];
        self.remember(picked, notes.len());
        Some(picked)
    }

    /// Pick with a PREFERENCE: `preferred` is what the surface being opened is
    /// about, `all` is everything the build has.
    ///
    /// A preferred note nobody has just read wins. When every preferred note is
    /// one the player has just read - a range with a single note, reloaded -
    /// the pick falls back to the whole set rather than repeating, which is the
    /// "valid general fallback" half of the rule. An empty preference is simply
    /// the fallback, so a caller with no context at all can use this too.
    pub fn pick_preferred<'a>(
        &mut self,
        preferred: &'a [FieldNote],
        all: &'a [FieldNote],
        roll: usize,
    ) -> Option<&'a FieldNote> {
        let fresh: Vec<&FieldNote> = preferred
            .iter()
            .filter(|note| !self.recent.contains(&note.id))
            .collect();
        let Some(picked) = fresh.get(roll % fresh.len().max(1)).copied() else {
            return self.pick(all, roll);
        };
        self.remember(picked, all.len());
        Some(picked)
    }

    /// Record a pick and trim the memory to what the set can afford.
    fn remember(&mut self, picked: &FieldNote, set: usize) {
        self.recent.push_back(picked.id.clone());
        // Remember one fewer than the set holds, so there is always something
        // left to pick and the rotation never has to fall back to a repeat
        // while an unread note exists.
        let memory = set.saturating_sub(1).min(FIELD_NOTE_MEMORY);
        while self.recent.len() > memory {
            self.recent.pop_front();
        }
    }
}

/// Break one authored note into the lines the slot draws, or `None` when it
/// does not fit.
///
/// Greedy word wrap to at most [`FIELD_NOTE_LINES`] lines of
/// [`FIELD_NOTE_LINE_CHARS`]. Authors write a note as one sentence and the slot
/// decides where it breaks, because the break depends on the box and not on
/// the claim. `None` is an authoring error the lint reports - a note that needs
/// a third line is a lesson.
pub fn wrap_note_lines(text: &str) -> Option<Vec<String>> {
    let mut lines: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        if word.chars().count() > FIELD_NOTE_LINE_CHARS {
            return None;
        }
        match lines.last_mut() {
            Some(line)
                if line.chars().count() + 1 + word.chars().count() <= FIELD_NOTE_LINE_CHARS =>
            {
                line.push(' ');
                line.push_str(word);
            }
            _ => lines.push(word.to_string()),
        }
    }
    (!lines.is_empty() && lines.len() <= FIELD_NOTE_LINES).then_some(lines)
}

/// Every field note the authored catalog carries, in catalog order.
///
/// ONE source: a claim is written once, on the lesson it belongs to, and the
/// note that quotes it points back at that lesson so the menu card can open
/// it. A note whose text does not fit the slot is DROPPED here rather than
/// drawn over the layout - the content lint is what reports it, loudly, before
/// it ever ships.
pub fn catalog_field_notes(catalog: &TrainingCatalog) -> Vec<FieldNote> {
    let mut notes = Vec::new();
    for lesson in catalog.lessons() {
        for (nth, text) in lesson.field_notes.iter().enumerate() {
            let Some(lines) = wrap_note_lines(text) else {
                continue;
            };
            notes.push(FieldNote {
                id: format!("{}#{nth}", lesson.id),
                lesson: Some(lesson.id.clone()),
                lines,
            });
        }
    }
    notes
}

/// The notes carried by the lessons a given scenario TEACHES, in catalog order.
///
/// A lesson teaches a scenario when the scenario is its practice range or one
/// of the scenarios that may prove it. That is the destination context a
/// loading screen has: the note you read while a practice range comes up is
/// about the thing the range is going to ask you to do.
///
/// Empty is the ordinary answer for a scenario no lesson points at - a campaign
/// chapter, a mod's own arena - and the caller falls back to the whole set.
pub fn notes_for_scenario(catalog: &TrainingCatalog, scenario: &str) -> Vec<FieldNote> {
    catalog_field_notes(catalog)
        .into_iter()
        .filter(|note| {
            note.lesson.as_ref().is_some_and(|id| {
                catalog.lessons().iter().any(|lesson| {
                    &lesson.id == id
                        && (lesson.practice.as_deref() == Some(scenario)
                            || lesson.proven_by.iter().any(|proof| proof == scenario))
                })
            })
        })
        .collect()
}

/// The most picks the rotation remembers. Bounded so a large authored set does
/// not turn "not the one you just read" into "every note before you may see one
/// twice".
const FIELD_NOTE_MEMORY: usize = 6;

/// The notes available BEFORE the main asset collection has loaded: compiled
/// data, so the boot loading screen has something to show on its first frame.
///
/// Each one quotes a lesson in the base handbook and carries that lesson's id,
/// so the claim still has one owner. Every surface that draws AFTER the merge
/// reads [`catalog_field_notes`] instead, which is where an installed mod's
/// notes come from; this set is the floor under it.
pub fn boot_field_notes() -> Vec<FieldNote> {
    vec![
        FieldNote::new(
            "note_momentum",
            [
                "Nothing slows you down in space.",
                "Release the drive and your speed stays the same.",
            ],
        )
        .from_lesson("flight_momentum"),
        FieldNote::new(
            "note_stop",
            [
                "STOP is not instant.",
                "The flight computer thrusts until your speed reads zero.",
            ],
        )
        .from_lesson("flight_stop"),
        FieldNote::new(
            "note_lock",
            [
                "A torpedo needs a combat lock before it will fire.",
                "It will not arm until it is well clear of your ship.",
            ],
        )
        .from_lesson("combat_torpedoes"),
        FieldNote::new(
            "note_mass",
            [
                "Every section you add is mass the drive has to move.",
                "A heavier ship turns slower for the same thrust.",
            ],
        )
        .from_lesson("build_mass"),
        FieldNote::new(
            "note_novaos",
            [
                "NOVA OS runs while you fly.",
                "Open it to read contacts and give orders.",
            ],
        )
        .from_lesson("novaos_open"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notes(ids: &[&str]) -> Vec<FieldNote> {
        ids.iter()
            .map(|id| FieldNote::new(id, ["one", "two"]))
            .collect()
    }

    #[test]
    fn a_pick_is_never_the_note_just_shown() {
        let notes = notes(&["a", "b", "c", "d"]);
        let mut rotation = FieldNoteRotation::default();
        let mut seen = Vec::new();
        for roll in 0..4 {
            seen.push(rotation.pick(&notes, roll).expect("a note").id.clone());
        }
        for pair in seen.windows(2) {
            assert_ne!(pair[0], pair[1], "sequence repeated back to back: {seen:?}");
        }
    }

    /// Four notes, four picks: the rotation spends the whole set before it
    /// comes back around.
    #[test]
    fn the_rotation_spends_the_set_before_repeating() {
        let notes = notes(&["a", "b", "c", "d"]);
        let mut rotation = FieldNoteRotation::default();
        let mut seen: Vec<String> = (0..4)
            .map(|roll| rotation.pick(&notes, roll).expect("a note").id.clone())
            .collect();
        seen.sort();
        assert_eq!(seen, ["a", "b", "c", "d"]);
    }

    /// One note is the whole set, so it is also the only honest answer.
    #[test]
    fn a_single_note_build_still_shows_its_note() {
        let notes = notes(&["only"]);
        let mut rotation = FieldNoteRotation::default();
        assert_eq!(
            rotation.pick(&notes, 0).map(|n| n.id.as_str()),
            Some("only")
        );
        assert_eq!(
            rotation.pick(&notes, 1).map(|n| n.id.as_str()),
            Some("only")
        );
    }

    #[test]
    fn no_notes_at_all_picks_nothing() {
        let mut rotation = FieldNoteRotation::default();
        assert!(rotation.pick(&[], 0).is_none());
    }

    /// The slot is two short lines. A note that outgrew it would push the
    /// loading panel's layout around on the one screen nobody can scroll.
    #[test]
    fn every_boot_note_fits_the_slot() {
        for note in boot_field_notes() {
            assert!(
                note.lines.len() <= FIELD_NOTE_LINES,
                "`{}` has {} lines",
                note.id,
                note.lines.len()
            );
            for line in &note.lines {
                assert!(
                    line.chars().count() <= FIELD_NOTE_LINE_CHARS,
                    "`{}` has a {}-char line: {line}",
                    note.id,
                    line.chars().count()
                );
            }
        }
    }

    /// The wrap is what turns an authored sentence into the slot's two lines,
    /// so it has to break on words and refuse anything that needs a third.
    #[test]
    fn a_note_wraps_onto_the_slots_lines_and_refuses_a_third() {
        let wrapped = wrap_note_lines(
            "Cutting thrust does not cancel momentum; nothing out here slows you down at all.",
        )
        .expect("a two-line note wraps");
        assert_eq!(wrapped.len(), 2, "{wrapped:?}");
        for line in &wrapped {
            assert!(line.chars().count() <= FIELD_NOTE_LINE_CHARS, "{line}");
        }
        assert_eq!(
            wrapped.join(" "),
            "Cutting thrust does not cancel momentum; nothing out here slows you down at all.",
            "wrapping moves the breaks, never the words"
        );

        assert!(
            wrap_note_lines(&"word ".repeat(40)).is_none(),
            "a note needing a third line does not fit the slot"
        );
        assert!(wrap_note_lines("   ").is_none(), "an empty note is no note");
    }

    /// A lesson with one authored note, wired to whatever range the test needs.
    fn lesson_with_note(
        id: &str,
        note: &str,
        practice: Option<&str>,
        proven_by: &[&str],
    ) -> crate::catalog::Lesson {
        use nova_gameplay::prelude::AssetRef;

        use crate::catalog::{Lesson, LessonCategory, LessonMedia};

        Lesson {
            id: id.to_string(),
            category: LessonCategory::Flight,
            order: 20,
            title: "Title".to_string(),
            media: LessonMedia::Image {
                image: AssetRef::from("self://training/x.png"),
                alt: "a still".to_string(),
            },
            body: "body".to_string(),
            actions: vec![],
            wiki_path: "wiki/flight".to_string(),
            practice: practice.map(ToString::to_string),
            proven_by: proven_by.iter().map(|id| (*id).to_string()).collect(),
            field_notes: vec![note.to_string()],
        }
    }

    /// The destination decides the note: a range's own lessons are what the
    /// player is about to be asked to do.
    #[test]
    fn a_ranges_notes_are_the_lessons_that_teach_it() {
        let catalog = TrainingCatalog::new([
            lesson_with_note(
                "combat_stance",
                "Hold the range.",
                Some("drill_gunnery"),
                &[],
            ),
            lesson_with_note("combat_radar", "Watch the sweep.", None, &["drill_gunnery"]),
            lesson_with_note(
                "build_mass",
                "Mass costs turn rate.",
                Some("drill_build"),
                &[],
            ),
        ]);

        let mut ids: Vec<String> = notes_for_scenario(&catalog, "drill_gunnery")
            .into_iter()
            .map(|note| note.lesson.expect("a lesson"))
            .collect();
        ids.sort();
        assert_eq!(
            ids,
            ["combat_radar", "combat_stance"],
            "both the practice range and the range that proves it count"
        );
        assert!(
            notes_for_scenario(&catalog, "chapter_one").is_empty(),
            "a scenario no lesson teaches has no notes of its own"
        );
    }

    /// Destination context first, the whole build when the destination has
    /// nothing left to say.
    #[test]
    fn a_preferred_pick_falls_back_rather_than_repeating() {
        let all = notes(&["a", "b", "c", "d"]);
        let preferred = notes(&["a"]);
        let mut rotation = FieldNoteRotation::default();

        assert_eq!(
            rotation
                .pick_preferred(&preferred, &all, 0)
                .map(|n| n.id.as_str()),
            Some("a"),
            "the destination's own note comes up first"
        );
        let second = rotation
            .pick_preferred(&preferred, &all, 0)
            .expect("a note")
            .id
            .clone();
        assert_ne!(second, "a", "reloading the same range does not repeat");

        let mut empty = FieldNoteRotation::default();
        assert!(
            empty.pick_preferred(&[], &all, 2).is_some(),
            "no destination context is simply the general pick"
        );
        assert!(
            empty.pick_preferred(&[], &[], 0).is_none(),
            "nothing to say stays nothing to say"
        );
    }

    /// The catalog is the ONE place a claim is written: a derived note keeps
    /// the lesson it came from, which is what lets the menu card open it.
    #[test]
    fn catalog_notes_carry_the_lesson_they_came_from() {
        use nova_gameplay::prelude::AssetRef;

        use crate::catalog::{Lesson, LessonCategory, LessonMedia};

        let catalog = TrainingCatalog::new([Lesson {
            id: "flight_momentum".to_string(),
            category: LessonCategory::Flight,
            order: 20,
            title: "Momentum".to_string(),
            media: LessonMedia::Image {
                image: AssetRef::from("self://training/x.png"),
                alt: "a still".to_string(),
            },
            body: "body".to_string(),
            actions: vec![],
            wiki_path: "wiki/flight".to_string(),
            practice: None,
            proven_by: vec![],
            field_notes: vec!["Cutting thrust does not cancel momentum.".to_string()],
        }]);
        let notes = catalog_field_notes(&catalog);
        assert_eq!(notes.len(), 1, "{notes:?}");
        assert_eq!(notes[0].lesson.as_deref(), Some("flight_momentum"));
        assert_eq!(notes[0].lines, ["Cutting thrust does not cancel momentum."]);
    }

    /// A note names an action. Spelling a default key here would be wrong for
    /// every player who rebound it, and the loading screen cannot resolve a
    /// chip to correct it.
    #[test]
    fn no_boot_note_spells_a_default_key() {
        // Spellings that can only be keyboard talk. Bare letters are NOT on
        // the list: "A heavier hull" is not a keycap, and a check that cannot
        // tell those apart is a check nobody can keep.
        let keys = [
            "Shift", "Ctrl", "Alt", "Spacebar", "keyboard", " key", "press ", "click",
        ];
        for note in boot_field_notes() {
            let text = note.lines.join(" ");
            for key in keys {
                assert!(
                    !text.contains(key),
                    "`{}` names a key (`{key}`): {text}",
                    note.id
                );
            }
        }
    }
}
