//! What the player has done with the handbook: which lessons they have READ
//! and which ones a real outcome has PROVEN.

use std::collections::BTreeSet;

use bevy::prelude::*;

use crate::catalog::{LessonId, TrainingCatalog};

/// What a lesson row draws as its state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LessonStatus {
    /// Never opened.
    New,
    /// Opened and read. Says nothing about whether the player can do it.
    Viewed,
    /// A real outcome proved it.
    Completed,
}

/// The player's handbook record.
///
/// Two SETS, not one flag with two meanings. Opening a page is the only thing
/// reading a page can claim, so it writes [`Self::viewed`]; a scenario outcome
/// or a practice assertion is what writes [`Self::completed`]. A screen that
/// collapsed them would tell a player who scrolled past a lesson that they had
/// learned it.
///
/// Sets are `BTreeSet<LessonId>`: an explicit id list, ordered, so a persisted
/// record round-trips byte-for-byte and an unknown id is a value somebody can
/// look at rather than a bit position. `nova_menu`'s `TrainingProgressPlugin`
/// owns persistence; nothing here writes a file.
///
/// What the player ANSWERED is not in here. Whether the menu still offers
/// Basic Training is a menu setting (`nova_menu`'s `TrainingPromptSetting`),
/// kept with the other things the player switched, rather than a flag riding
/// along with what they have learned.
#[derive(Resource, Default, Debug, Clone, PartialEq, Eq)]
pub struct TrainingProgress {
    viewed: BTreeSet<LessonId>,
    completed: BTreeSet<LessonId>,
}

impl TrainingProgress {
    /// A record seeded with these ids. `completed` implies viewed, the same
    /// rule [`Self::mark_completed`] enforces, so a hand-written or migrated
    /// record cannot describe a lesson proven but unread.
    pub fn seeded(
        viewed: impl IntoIterator<Item = LessonId>,
        completed: impl IntoIterator<Item = LessonId>,
    ) -> Self {
        let mut progress = Self::default();
        for id in viewed {
            progress.mark_viewed(&id);
        }
        for id in completed {
            progress.mark_completed(&id);
        }
        progress
    }

    /// Record that the player opened this lesson. `true` when that was news.
    pub fn mark_viewed(&mut self, id: &str) -> bool {
        self.viewed.insert(id.to_string())
    }

    /// Record that an outcome PROVED this lesson. `true` when that was news.
    ///
    /// Also marks it viewed: there is no way to prove a lesson without having
    /// been shown it, and a record that said otherwise would draw a completed
    /// row the progress count called unread.
    pub fn mark_completed(&mut self, id: &str) -> bool {
        self.viewed.insert(id.to_string());
        self.completed.insert(id.to_string())
    }

    /// What this lesson's row draws.
    pub fn status(&self, id: &str) -> LessonStatus {
        if self.completed.contains(id) {
            LessonStatus::Completed
        } else if self.viewed.contains(id) {
            LessonStatus::Viewed
        } else {
            LessonStatus::New
        }
    }

    /// Every viewed id, in order.
    pub fn viewed(&self) -> impl Iterator<Item = &LessonId> {
        self.viewed.iter()
    }

    /// Every completed id, in order.
    pub fn completed(&self) -> impl Iterator<Item = &LessonId> {
        self.completed.iter()
    }

    /// `(viewed, completed, total)` COUNTED AGAINST THE CATALOG.
    ///
    /// An id the catalog no longer holds is not counted. A record written by a
    /// build that had a lesson this one does not - a disabled mod, a renamed
    /// id - would otherwise report "8 of 6 lessons", and the safe reading of an
    /// id nobody can open is that the player has not done it.
    pub fn counts(&self, catalog: &TrainingCatalog) -> (usize, usize, usize) {
        let viewed = catalog
            .lessons()
            .iter()
            .filter(|lesson| self.viewed.contains(&lesson.id))
            .count();
        let completed = catalog
            .lessons()
            .iter()
            .filter(|lesson| self.completed.contains(&lesson.id))
            .count();
        (viewed, completed, catalog.len())
    }
}

#[cfg(test)]
mod tests {
    use nova_gameplay::prelude::AssetRef;

    use super::*;
    use crate::catalog::{Lesson, LessonCategory, LessonMedia};

    fn catalog(ids: &[&str]) -> TrainingCatalog {
        TrainingCatalog::new(ids.iter().enumerate().map(|(nth, id)| Lesson {
            id: (*id).to_string(),
            category: LessonCategory::StartHere,
            order: nth as i32,
            title: (*id).to_string(),
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
        }))
    }

    #[test]
    fn a_fresh_record_has_opened_nothing() {
        let progress = TrainingProgress::default();
        assert_eq!(progress.status("start"), LessonStatus::New);
        assert_eq!(progress.counts(&catalog(&["start"])), (0, 0, 1));
    }

    #[test]
    fn reading_a_lesson_never_claims_the_player_can_do_it() {
        let mut progress = TrainingProgress::default();
        assert!(progress.mark_viewed("start"));
        assert_eq!(progress.status("start"), LessonStatus::Viewed);
        assert_eq!(progress.counts(&catalog(&["start"])), (1, 0, 1));
    }

    #[test]
    fn a_proven_lesson_counts_as_read_as_well() {
        let mut progress = TrainingProgress::default();
        assert!(progress.mark_completed("start"));
        assert_eq!(progress.status("start"), LessonStatus::Completed);
        assert_eq!(progress.counts(&catalog(&["start"])), (1, 1, 1));
    }

    #[test]
    fn marking_the_same_lesson_twice_is_not_news() {
        let mut progress = TrainingProgress::default();
        assert!(progress.mark_viewed("start"));
        assert!(!progress.mark_viewed("start"));
        assert!(progress.mark_completed("start"));
        assert!(!progress.mark_completed("start"));
    }

    /// A record naming a lesson this build does not have is neither counted nor
    /// fatal: the count stays inside the catalog it is drawn beside.
    #[test]
    fn an_id_the_catalog_lost_does_not_inflate_the_count() {
        let progress = TrainingProgress::seeded(
            ["start".to_string(), "from_a_disabled_mod".to_string()],
            ["start".to_string()],
        );
        assert_eq!(progress.counts(&catalog(&["start"])), (1, 1, 1));
    }
}
