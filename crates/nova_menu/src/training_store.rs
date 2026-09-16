//! The persisted form of what the player has learned.
//!
//! A SECOND file beside the settings, not a field inside them. The settings
//! file holds switches the player threw - volumes, keybinds, whether the corner
//! prompt still shows. What a player has read and been proven on is not a
//! switch, and a `settings.ron` somebody deletes to fix their audio must not
//! also erase their progress.
//!
//! The two stores share a ROOT (the same profile directory, so one
//! `SettingsStoreRoot` override redirects both in a test) and the same
//! best-effort semantics from [`nova_assets::persist`]: a missing or corrupt
//! store reads as "nothing learned yet", and a failed write is logged, never
//! fatal.
//!
//! Unlike the settings store, the resource here IS the stored value, so this is
//! the plain load-on-startup / save-on-change plugin shape `persist`'s module
//! docs name as the counterpart case.

use bevy::prelude::*;
use nova_assets::persist;
use nova_gameplay::prelude::harness_env_active;
use nova_scenario::prelude::{CurrentOutcome, CurrentScenario, ScenarioOutcomeKind};
use nova_training::prelude::{LessonId, TrainingCatalog, TrainingProgress};
use serde::{Deserialize, Serialize};

use crate::settings_store::SettingsStoreRoot;

/// The store key, beside the settings store's `settings`.
pub(crate) const KEY: &str = "training";

/// The persisted record: two explicit id lists, and nothing else.
///
/// No "first run" flag and no counts. Whether a player is new is answered by
/// the settings file's `training_prompt`, and a count derived from a list
/// cannot disagree with the list.
///
/// Both fields carry a serde default, so a record written before a field
/// existed still loads.
#[derive(Serialize, Deserialize, Clone, Default, PartialEq, Eq, Debug)]
pub struct PersistedTraining {
    /// Lesson ids the player has opened.
    #[serde(default)]
    pub viewed: Vec<LessonId>,
    /// Lesson ids a scenario outcome has PROVEN.
    #[serde(default)]
    pub completed: Vec<LessonId>,
}

impl PersistedTraining {
    /// Snapshot a live record. Both lists come out of `BTreeSet`s, so the file
    /// is ordered and a save with nothing new in it is byte-identical.
    pub fn from_progress(progress: &TrainingProgress) -> Self {
        Self {
            viewed: progress.viewed().cloned().collect(),
            completed: progress.completed().cloned().collect(),
        }
    }

    /// The live record this file describes.
    ///
    /// An id this build has no lesson for is kept, not dropped: a player who
    /// disabled a mod for one session must not lose their progress in it, and
    /// [`TrainingProgress::counts`] already refuses to count an id the catalog
    /// cannot show. Nothing here validates - a record is not content.
    pub fn into_progress(self) -> TrainingProgress {
        TrainingProgress::seeded(self.viewed, self.completed)
    }
}

/// The saved record, or `None` when nothing has been saved yet (or the store is
/// unreadable/corrupt). `None` means "nothing learned yet".
pub fn load_training(root: &SettingsStoreRoot) -> Option<PersistedTraining> {
    persist::load_from(&root.store()?, KEY)
}

/// Persist the record. Best-effort - failures are logged, not returned.
pub fn save_training(root: &SettingsStoreRoot, record: &PersistedTraining) {
    let Some(store) = root.store() else {
        warn!("persist[{KEY}]: no store available; the record will not persist");
        return;
    };
    persist::save_to(&store, KEY, record);
}

/// Whether this app's progress store reads, writes, or does neither.
///
/// The same three-state policy as the settings store, for the same reason and
/// with a sharper edge: a scripted run that saved would mark lessons read in
/// the developer's own profile because a capture pass happened to open the
/// handbook. Reading is harmless and keeps a human's frames honest; writing is
/// earned by carrying the screen a player reads lessons on.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug)]
pub enum TrainingStoreAccess {
    /// Neither direction. A scripted run.
    Inert,
    /// Load the record; never write it back.
    Read,
    /// Load the record and save it as it changes.
    ReadWrite,
}

impl TrainingStoreAccess {
    /// Whether the record is read at startup.
    pub fn reads(self) -> bool {
        !matches!(self, Self::Inert)
    }

    /// Whether a change reaches the file.
    pub fn writes(self) -> bool {
        matches!(self, Self::ReadWrite)
    }

    /// READ for a human at the keyboard, INERT under a scripted run.
    ///
    /// NATIVE only in effect: a wasm build has no process environment, so
    /// `harness_env_active` is always false there and this always reads.
    pub fn from_env() -> Self {
        if harness_env_active() {
            Self::Inert
        } else {
            Self::Read
        }
    }
}

/// Run condition on the write system, so the write direction can be granted
/// after the plugin is built.
fn the_store_writes(access: Res<TrainingStoreAccess>) -> bool {
    access.writes()
}

/// Grant this app the write direction.
///
/// Called by [`NovaMenuPlugin`](crate::NovaMenuPlugin): the handbook is where a
/// lesson is read, so it is what turns a reading store into a writing one. An
/// INERT store stays inert.
///
/// # Panics
///
/// If no [`TrainingProgressPlugin`] has been added.
pub fn allow_training_saves(app: &mut App) {
    let mut access = app.world_mut().resource_mut::<TrainingStoreAccess>();
    if *access == TrainingStoreAccess::Read {
        *access = TrainingStoreAccess::ReadWrite;
    }
}

/// Owns [`TrainingProgress`] and both directions of its store.
pub struct TrainingProgressPlugin {
    /// How far this app's store reaches; see [`TrainingStoreAccess`].
    pub access: TrainingStoreAccess,
}

impl Default for TrainingProgressPlugin {
    fn default() -> Self {
        Self {
            access: TrainingStoreAccess::from_env(),
        }
    }
}

impl Plugin for TrainingProgressPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TrainingProgress>();
        app.insert_resource(self.access);
        // OUTSIDE the access guard: completion is live state, and the access
        // policy gates the FILE, not what the player did this run. An inert
        // run still draws the right rows; it just does not keep them.
        app.add_systems(Update, complete_what_a_won_scenario_proves);
        if !self.access.reads() {
            return;
        }
        // The root is the settings store's, so one override redirects both
        // files. `init_resource` is idempotent and the default root is the
        // player's own profile, which is what a plugin added alone wants.
        app.init_resource::<SettingsStoreRoot>();
        app.add_systems(Startup, load_persisted_training);
        app.add_systems(
            Update,
            persist_training_on_change
                .run_if(the_store_writes)
                .run_if(resource_changed::<TrainingProgress>)
                // The record COMING INTO EXISTENCE is not a change to keep, and
                // neither is the startup load overwriting it: both land on the
                // first frame, and both would otherwise write a file for a boot
                // in which the player learned nothing.
                .run_if(not(resource_added::<TrainingProgress>)),
        );
    }
}

/// Load the record once at startup, over the empty default.
///
/// No debounce, unlike the settings store: progress changes when a player opens
/// a lesson or finishes a flight, which is a handful of writes an hour, not a
/// slider drag.
pub(crate) fn load_persisted_training(
    mut progress: ResMut<TrainingProgress>,
    root: Res<SettingsStoreRoot>,
) {
    let Some(saved) = load_training(&root) else {
        return;
    };
    *progress = saved.into_progress();
}

/// Mark every lesson a just-won scenario proves.
///
/// VICTORY only, and only for the scenario actually loaded. This is the one
/// writer of [`TrainingProgress::mark_completed`] in the shipped game, which is
/// what keeps "Completed" meaning a flight the player finished: opening a
/// lesson writes `viewed`, launching Practice writes nothing at all, and a
/// Defeat - including the retry-loop a lost range queues - writes nothing
/// either.
///
/// Every resource is optional because this plugin stands alone in slim rigs
/// that have no scenario loader.
pub(crate) fn complete_what_a_won_scenario_proves(
    outcome: Option<Res<CurrentOutcome>>,
    scenario: Option<Res<CurrentScenario>>,
    catalog: Option<Res<TrainingCatalog>>,
    progress: Option<ResMut<TrainingProgress>>,
) {
    let (Some(outcome), Some(scenario), Some(catalog), Some(mut progress)) =
        (outcome, scenario, catalog, progress)
    else {
        return;
    };
    // An outcome flips at most once per scenario, so the change tick is the
    // whole guard - without it this would re-mark every frame the banner is up
    // and keep `TrainingProgress` permanently dirty, which is a file write per
    // frame on the other system.
    if !outcome.is_changed() {
        return;
    }
    let Some(config) = outcome.0.as_ref() else {
        return;
    };
    if config.outcome != ScenarioOutcomeKind::Victory {
        return;
    }
    let Some(won) = scenario.0.as_ref().map(|config| config.id.as_str()) else {
        return;
    };
    for lesson in catalog.lessons() {
        let proves_it = lesson.proven_by.iter().any(|id| id == won);
        if proves_it && progress.mark_completed(&lesson.id) {
            info!("training: scenario '{won}' proved lesson '{}'", lesson.id);
        }
    }
}

/// Write the record back whenever it changes - see the plugin for what does
/// not count as a change.
pub(crate) fn persist_training_on_change(
    progress: Res<TrainingProgress>,
    root: Res<SettingsStoreRoot>,
) {
    save_training(&root, &PersistedTraining::from_progress(&progress));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn progress(viewed: &[&str], completed: &[&str]) -> TrainingProgress {
        TrainingProgress::seeded(
            viewed.iter().map(|id| (*id).to_string()),
            completed.iter().map(|id| (*id).to_string()),
        )
    }

    #[test]
    fn the_record_round_trips_through_its_persisted_form() {
        let live = progress(&["start_hud"], &["flight_stop"]);
        let back = PersistedTraining::from_progress(&live).into_progress();
        assert_eq!(back, live);
    }

    /// A proven lesson is also a read one, so the file says so in both lists
    /// and a hand-edited file that disagreed is repaired on load.
    #[test]
    fn a_completed_lesson_comes_back_as_read_as_well() {
        let record = PersistedTraining {
            viewed: vec![],
            completed: vec!["flight_stop".to_string()],
        };
        let live = record.into_progress();
        assert_eq!(
            live.viewed().map(String::as_str).collect::<Vec<_>>(),
            ["flight_stop"]
        );
    }

    /// The player disabled a mod for one session. Their progress in it survives
    /// the round trip, because a record is not content and nothing here
    /// validates ids.
    #[test]
    fn an_id_this_build_has_no_lesson_for_survives_a_save() {
        let live = progress(&["from_a_mod"], &[]);
        let record = PersistedTraining::from_progress(&live);
        assert_eq!(record.viewed, ["from_a_mod"]);
        assert_eq!(record.into_progress(), live);
    }

    #[test]
    fn an_empty_record_is_a_player_who_has_done_nothing() {
        assert_eq!(
            PersistedTraining::default().into_progress(),
            TrainingProgress::default()
        );
    }

    /// A unique temp root per test, so nothing here can reach the developer's
    /// own profile. The test cleans it up.
    #[cfg(not(target_arch = "wasm32"))]
    fn temp_root(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("nova_training_store_{name}"))
    }

    /// An app rooted at `root`, the way the shipped game is rooted at the
    /// player's profile.
    #[cfg(not(target_arch = "wasm32"))]
    fn app_at(root: &std::path::Path, access: TrainingStoreAccess) -> App {
        let mut app = App::new();
        app.insert_resource(SettingsStoreRoot(Some(root.to_path_buf())));
        app.add_plugins(TrainingProgressPlugin { access });
        app
    }

    /// The Phase 3 claim end to end: what one run learned, the next run knows.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn what_one_run_learned_the_next_run_loads() {
        use nova_training::prelude::LessonStatus;

        let root = temp_root("restart");
        let _ = std::fs::remove_dir_all(&root);

        let mut first = app_at(&root, TrainingStoreAccess::ReadWrite);
        first.update();
        first
            .world_mut()
            .resource_mut::<TrainingProgress>()
            .mark_viewed("start_hud");
        first
            .world_mut()
            .resource_mut::<TrainingProgress>()
            .mark_completed("flight_stop");
        first.update();

        let mut second = app_at(&root, TrainingStoreAccess::Read);
        second.update();
        let progress = second.world().resource::<TrainingProgress>();
        assert_eq!(progress.status("start_hud"), LessonStatus::Viewed);
        assert_eq!(progress.status("flight_stop"), LessonStatus::Completed);
        assert_eq!(progress.status("combat_turrets"), LessonStatus::New);

        let _ = std::fs::remove_dir_all(&root);
    }

    /// A boot nobody learned anything in leaves no file behind. The record
    /// coming into existence is not something to keep.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn a_boot_that_learned_nothing_writes_no_file() {
        let root = temp_root("quiet_boot");
        let _ = std::fs::remove_dir_all(&root);

        let mut app = app_at(&root, TrainingStoreAccess::ReadWrite);
        app.update();
        app.update();

        assert_eq!(
            load_training(&SettingsStoreRoot(Some(root.clone()))),
            None,
            "a boot that changed nothing wrote a record"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A READING store leaves the file exactly as it found it, so a capture
    /// pass or a bench run cannot write progress into a real profile.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn a_reading_store_never_writes_what_the_run_did() {
        let root = temp_root("read_only");
        let _ = std::fs::remove_dir_all(&root);

        let mut app = app_at(&root, TrainingStoreAccess::Read);
        app.update();
        app.world_mut()
            .resource_mut::<TrainingProgress>()
            .mark_viewed("start_hud");
        app.update();

        assert_eq!(
            load_training(&SettingsStoreRoot(Some(root.clone()))),
            None,
            "a reading store wrote a file"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// An INERT store does not even read, so a scripted run starts from the
    /// same empty record on every machine.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn an_inert_store_ignores_a_record_that_is_already_there() {
        use nova_training::prelude::LessonStatus;

        let root = temp_root("inert");
        let _ = std::fs::remove_dir_all(&root);
        save_training(
            &SettingsStoreRoot(Some(root.clone())),
            &PersistedTraining {
                viewed: vec!["start_hud".to_string()],
                completed: vec![],
            },
        );

        let mut app = app_at(&root, TrainingStoreAccess::Inert);
        app.update();
        assert_eq!(
            app.world()
                .resource::<TrainingProgress>()
                .status("start_hud"),
            LessonStatus::New
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Corrupt, truncated or hand-mangled: the record falls back to "nothing
    /// learned yet" rather than refusing to boot the screen.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn a_corrupt_record_reads_as_a_fresh_one() {
        use nova_assets::storage::Storage;

        let root = temp_root("corrupt");
        let _ = std::fs::remove_dir_all(&root);
        let store = nova_assets::storage::platform_at(Some(&root)).expect("a native store");
        store
            .write(KEY, b"(viewed: [\"start_hud\"], completed: ")
            .expect("write the broken file");

        let mut app = app_at(&root, TrainingStoreAccess::Read);
        app.update();
        assert_eq!(
            *app.world().resource::<TrainingProgress>(),
            TrainingProgress::default()
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The rig the completion tests drive: the plugin, plus the two scenario
    /// resources it reads, plus a catalog of two lessons only one of which the
    /// won scenario proves.
    fn completion_app(proven_by: &[&str]) -> App {
        use nova_gameplay::prelude::AssetRef;
        use nova_training::prelude::{Lesson, LessonCategory, LessonMedia};

        let lesson = |id: &str, proven_by: &[&str]| Lesson {
            id: id.to_string(),
            category: LessonCategory::Flight,
            order: 10,
            title: id.to_string(),
            media: LessonMedia::Image {
                image: AssetRef::from("self://training/x.png"),
                alt: "a still".to_string(),
            },
            body: "body".to_string(),
            actions: vec![],
            wiki_path: "wiki/flight-autopilot".to_string(),
            practice: None,
            proven_by: proven_by.iter().map(|id| (*id).to_string()).collect(),
            field_notes: vec![],
        };

        let mut app = App::new();
        app.add_plugins(TrainingProgressPlugin {
            access: TrainingStoreAccess::Inert,
        });
        app.init_resource::<CurrentOutcome>();
        app.init_resource::<CurrentScenario>();
        app.insert_resource(TrainingCatalog::new([
            lesson("flight_stop", proven_by),
            lesson("combat_turrets", &["something_else"]),
        ]));
        app
    }

    /// Put `scenario` in play and declare `kind` on it, the way the scenario
    /// script's `Outcome` action does.
    fn finish(app: &mut App, scenario: &str, kind: ScenarioOutcomeKind) {
        use nova_gameplay::prelude::AssetRef;
        use nova_scenario::prelude::ScenarioConfig;

        app.insert_resource(CurrentScenario(Some(ScenarioConfig::new(
            scenario.to_string(),
            "Test".to_string(),
            AssetRef::default(),
        ))));
        app.insert_resource(CurrentOutcome(Some(
            nova_scenario::prelude::OutcomeActionConfig::new(kind, "done"),
        )));
        app.update();
    }

    fn status(app: &App, id: &str) -> nova_training::prelude::LessonStatus {
        app.world().resource::<TrainingProgress>().status(id)
    }

    /// The whole point of `proven_by`: a won flight completes the lessons it
    /// proves and leaves every other lesson alone.
    #[test]
    fn a_won_scenario_completes_only_the_lessons_it_proves() {
        use nova_training::prelude::LessonStatus;

        let mut app = completion_app(&["drill_stop"]);
        finish(&mut app, "drill_stop", ScenarioOutcomeKind::Victory);
        assert_eq!(status(&app, "flight_stop"), LessonStatus::Completed);
        assert_eq!(status(&app, "combat_turrets"), LessonStatus::New);
    }

    /// Basic Training is a chapter, and it proves several lessons at once.
    #[test]
    fn basic_training_completes_every_lesson_its_flow_proves() {
        use nova_training::prelude::LessonStatus;

        let mut app = completion_app(&["tutorial", "drill_stop"]);
        finish(&mut app, "tutorial", ScenarioOutcomeKind::Victory);
        assert_eq!(status(&app, "flight_stop"), LessonStatus::Completed);
    }

    /// Losing is not learning - and a lost range QUEUES ITSELF, so a player who
    /// wrecks repeatedly must not accumulate completions.
    #[test]
    fn a_lost_scenario_proves_nothing() {
        use nova_training::prelude::LessonStatus;

        let mut app = completion_app(&["drill_stop"]);
        finish(&mut app, "drill_stop", ScenarioOutcomeKind::Defeat);
        assert_eq!(status(&app, "flight_stop"), LessonStatus::New);
    }

    /// Merely LOADING a range claims nothing: the outcome is what completes a
    /// lesson, and a scenario in play has none.
    #[test]
    fn launching_practice_does_not_complete_the_lesson() {
        use nova_gameplay::prelude::AssetRef;
        use nova_scenario::prelude::ScenarioConfig;
        use nova_training::prelude::LessonStatus;

        let mut app = completion_app(&["drill_stop"]);
        app.insert_resource(CurrentScenario(Some(ScenarioConfig::new(
            "drill_stop".to_string(),
            "Test".to_string(),
            AssetRef::default(),
        ))));
        app.update();
        assert_eq!(status(&app, "flight_stop"), LessonStatus::New);
    }

    /// A won scenario nothing names proves nothing, which is every chapter in
    /// the campaign.
    #[test]
    fn winning_a_scenario_no_lesson_names_completes_nothing() {
        use nova_training::prelude::LessonStatus;

        let mut app = completion_app(&["drill_stop"]);
        finish(
            &mut app,
            "ledger_01_drift_run",
            ScenarioOutcomeKind::Victory,
        );
        assert_eq!(status(&app, "flight_stop"), LessonStatus::New);
    }

    /// The write direction is EARNED. A scripted run that opened the handbook
    /// would otherwise mark lessons read in the profile of whoever launched it.
    #[test]
    fn an_inert_store_cannot_be_granted_the_write_direction() {
        let mut app = App::new();
        app.add_plugins(TrainingProgressPlugin {
            access: TrainingStoreAccess::Inert,
        });
        allow_training_saves(&mut app);
        assert_eq!(
            *app.world().resource::<TrainingStoreAccess>(),
            TrainingStoreAccess::Inert
        );
    }

    #[test]
    fn a_reading_store_becomes_a_writing_one_when_asked() {
        let mut app = App::new();
        app.add_plugins(TrainingProgressPlugin {
            access: TrainingStoreAccess::Read,
        });
        allow_training_saves(&mut app);
        assert_eq!(
            *app.world().resource::<TrainingStoreAccess>(),
            TrainingStoreAccess::ReadWrite
        );
    }
}
