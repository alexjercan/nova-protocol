//! The player training handbook: what a lesson IS, what the player has done
//! with it, and the field notes the menu and the loading screens draw from the
//! same material.
//!
//! A LEAF crate: the handbook UI lives in `nova_menu`, the loading-screen fact
//! slot lives in `nova_core`, and `nova_core` is what adds `nova_menu`. Two
//! surfaces on opposite sides of that edge quote the same notes, so the notes
//! live under both. Nothing here builds an `App`.
//!
//! Lessons are CONTENT, not code. The types in [`catalog`] are `serde`, a
//! lesson is one `Lesson((..))` item in an ordinary `*.content.ron` file, and
//! the base game ships its lessons through the base bundle exactly as a mod
//! would - so `nova_modding` carries the variant, `nova_assets` merges them by
//! id and publishes the [`TrainingCatalog`](catalog::TrainingCatalog), and
//! this crate stays below all of them with the data and the pure checks.
#![warn(missing_docs)]

/// Glob-import surface: `use nova_training::prelude::*`.
pub mod prelude {
    pub use super::{
        catalog::{
            body_words, Lesson, LessonCategory, LessonId, LessonMedia, TrainingCatalog,
            LESSON_BODY_MAX_WORDS,
        },
        facts::{
            boot_field_notes, catalog_field_notes, notes_for_scenario, wrap_note_lines, FieldNote,
            FieldNoteRotation,
        },
        progress::{LessonStatus, TrainingProgress},
        validate::{
            lint_lesson, lint_lessons, unused_practice_ranges, LessonIssue, LessonSeverity,
        },
        TUTORIAL_SCENARIO_ID,
    };
}

/// The scenario id Basic Training runs under. The base content authors the
/// scenario and its lessons on it, and the menu's first-launch offer starts it.
pub const TUTORIAL_SCENARIO_ID: &str = "tutorial";

pub mod catalog;
pub mod facts;
pub mod progress;
pub mod validate;
