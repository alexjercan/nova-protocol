//! The mod a builder works on: every item of every kind, one of them open.
//!
//! [`ModProject`] is the model and the ECS node tree is the one asset open in
//! it. The document is the truth for that asset while it is open. Switching
//! lowers it back first ([`lower_open_document`]), so what the project holds for
//! the open scenario may lag the nodes and never the other way round.
//!
//! Lowering happens only when the nodes differ from what was last lowered. A
//! scenario nobody edited is never rewritten, so a hand-written one keeps every
//! field the document has no node for.
//!
//! UNDO. There is one stack for the project, in the order things happened
//! ([`crate::history`]). A snapshot holds the nodes and the [`ProjectData`] they
//! stood beside, so restoring one puts back the asset that was open at the time
//! through the same diff an in-document step uses. Opening another asset is not
//! a step; it lowers and moves on, and Ctrl+Z after it walks back into the
//! asset the last edit was made in.

pub(crate) mod ids;
pub(crate) mod package;
pub(crate) mod store;
pub(crate) mod verbs;

use std::{
    collections::{BTreeMap, HashSet},
    sync::Arc,
};

use bevy::prelude::*;
use nova_assets::{
    mod_refs::prelude::{names_string, prototype_refs, string_leaves},
    prelude::EnabledMods,
};
use nova_events::units::prelude::Meters3;
use nova_modding::prelude::{Content, ModMeta};
use nova_scenario::prelude::{
    EventActionConfig, ScenarioConfig, ScenarioEventConfig, ScenarioObjectConfig,
};

use self::{
    ids::{namespace_of, scenario_id},
    store::{bundle_file, content_file, new_meta},
};
use crate::{bundle::is_layout, template::ScenarioTemplate};

/// What [`Content::kind`] says for a scenario.
pub(crate) const SCENARIO_KIND: &str = "scenario";

/// How the editor treats one content kind. The one table a new kind joins: a row
/// here lists it in the Assets tab, and `opens` and `add` turn its rows on.
#[derive(Debug, Clone, Copy)]
pub(crate) struct KindRow {
    /// What [`Content::kind`] says.
    pub(crate) kind: &'static str,
    /// The group heading.
    pub(crate) heading: &'static str,
    /// Whether a double click opens an item of this kind.
    pub(crate) opens: bool,
    /// Whether the Add menu can make one.
    pub(crate) add: bool,
    /// What a disabled row says instead.
    pub(crate) waits: &'static str,
}

/// Every content kind, in Assets tab order.
pub(crate) const KINDS: [KindRow; 12] = [
    KindRow::opens("scenario", "Scenarios"),
    KindRow::waits("ship", "Ships", "edit in its scenario"),
    KindRow::waits("station", "Stations", "edit in its scenario"),
    KindRow::opens("section", "Sections"),
    KindRow::opens("campaign", "Campaigns"),
    KindRow::opens("style", "Styles"),
    KindRow::waits("lesson", "Lessons", "kept as written"),
    KindRow::opens("ui theme", "UI Themes"),
    KindRow::opens("item", "Items"),
    KindRow::opens("start role", "Start Roles"),
    KindRow::opens("price table", "Price Tables"),
    KindRow::opens("trade profile", "Trade Profiles"),
];

impl KindRow {
    /// A kind the document can open.
    const fn opens(kind: &'static str, heading: &'static str) -> Self {
        Self {
            kind,
            heading,
            opens: true,
            add: true,
            waits: "",
        }
    }

    /// A kind that is kept and listed, and not edited here yet.
    const fn waits(kind: &'static str, heading: &'static str, waits: &'static str) -> Self {
        Self {
            kind,
            heading,
            opens: false,
            add: false,
            waits,
        }
    }
}

/// One content file: its path in the mod, and its items in file order.
///
/// Items are shared with every snapshot that holds them, and an unchanged item
/// stays the same `Arc`, which is how a save knows a file needs no rewrite.
#[derive(Debug, Clone)]
pub(crate) struct ContentFile {
    /// Relative to the mod's directory.
    pub(crate) path: String,
    /// The items, of any kind.
    pub(crate) items: Vec<Arc<Content>>,
}

/// Everything about a mod an undo step can change.
#[derive(Debug, Clone)]
pub(crate) struct ProjectData {
    /// The manifest's description of the mod.
    pub(crate) meta: ModMeta,
    /// The binary files the manifest declares.
    pub(crate) resources: Vec<String>,
    /// Kept for the write. The merge ignores it on any mod but the base game.
    #[cfg_attr(
        target_arch = "wasm32",
        expect(
            dead_code,
            reason = "only Package reads this, and the web has no Package"
        )
    )]
    pub(crate) new_game_scenario: Option<String>,
    /// The content, in the mod's own file grouping.
    pub(crate) files: Vec<ContentFile>,
    /// The asset open in the document. `None` in a mod that holds no scenario
    /// and has opened nothing else.
    pub(crate) open: Option<OpenAsset>,
}

/// Which item of the mod stands in the document, by kind and id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OpenAsset {
    /// What [`Content::kind`] says.
    pub(crate) kind: &'static str,
    /// What [`Content::id`] says.
    pub(crate) id: String,
}

impl OpenAsset {
    /// The scenario `id`.
    pub(crate) fn scenario(id: impl Into<String>) -> Self {
        Self {
            kind: SCENARIO_KIND,
            id: id.into(),
        }
    }

    /// Whether this is a scenario, which the document holds as a node tree. Any
    /// other kind stands on the bench ([`crate::asset`]).
    pub(crate) fn is_scenario(&self) -> bool {
        self.kind == SCENARIO_KIND
    }
}

impl ProjectData {
    /// Every item, in file order.
    pub(crate) fn items(&self) -> impl Iterator<Item = &Content> {
        self.files
            .iter()
            .flat_map(|file| file.items.iter().map(Arc::as_ref))
    }

    /// Every scenario, in file order.
    pub(crate) fn scenarios(&self) -> impl Iterator<Item = &ScenarioConfig> {
        self.items().filter_map(|item| match item {
            Content::Scenario(scenario) => Some(scenario),
            _ => None,
        })
    }

    /// The scenario `id`.
    pub(crate) fn scenario(&self, id: &str) -> Option<&ScenarioConfig> {
        self.scenarios().find(|scenario| scenario.id == id)
    }

    /// The id of the scenario open in the document. `None` with no scenario
    /// open, which is also the bench.
    pub(crate) fn open_scenario(&self) -> Option<&str> {
        self.open
            .as_ref()
            .filter(|open| open.is_scenario())
            .map(|open| open.id.as_str())
    }

    /// What the open asset is called: a scenario's name, a section's name, any
    /// other item's id.
    pub(crate) fn open_name(&self) -> Option<String> {
        let open = self.open.as_ref()?;
        Some(match self.item(open.kind, &open.id)? {
            Content::Scenario(scenario) => scenario.name.clone(),
            Content::Section(config) => config.base.name.clone(),
            other => other.id().to_string(),
        })
    }

    /// The asset on the bench: open, and not a scenario.
    pub(crate) fn open_bench(&self) -> Option<&OpenAsset> {
        self.open.as_ref().filter(|open| !open.is_scenario())
    }

    /// The item of this kind and id.
    pub(crate) fn item(&self, kind: &str, id: &str) -> Option<&Content> {
        self.items().find(|item| same_item(item, kind, id))
    }

    /// Every id held by an item of `kind`.
    pub(crate) fn ids_of(&self, kind: &str) -> HashSet<String> {
        self.items()
            .filter(|item| item.kind() == kind)
            .map(|item| item.id().to_string())
            .collect()
    }

    /// Every id the mod's items hold, of any kind.
    pub(crate) fn all_ids(&self) -> HashSet<String> {
        self.items().map(|item| item.id().to_string()).collect()
    }

    /// Put `item` where the item of its kind and `id` stands.
    pub(crate) fn replace_item(&mut self, id: &str, item: Content) {
        if let Some((file, place)) = self.locate(item.kind(), id) {
            self.files[file].items[place] = Arc::new(item);
        }
    }

    /// Add `item` at the end of the first file, or of a new `file`.
    pub(crate) fn add_item(&mut self, item: Content, file: &str) {
        if self.files.is_empty() {
            self.files.push(ContentFile {
                path: file.to_string(),
                items: vec![],
            });
        }
        self.files[0].items.push(Arc::new(item));
    }

    /// Remove the item of this kind and id.
    pub(crate) fn remove_item(&mut self, kind: &str, id: &str) {
        for file in &mut self.files {
            file.items.retain(|item| !same_item(item, kind, id));
        }
    }

    /// Every item but the one of `kind` and `id` that names `id`, as a line
    /// each.
    pub(crate) fn users_of(&self, kind: &str, id: &str) -> Vec<String> {
        let mut users: Vec<String> = self
            .items()
            .filter(|item| !same_item(item, kind, id))
            .filter(|item| names_string(item, id))
            .map(|item| format!("{} {}", item.kind(), item.id()))
            .collect();
        users.sort();
        users
    }

    /// Every scenario id.
    pub(crate) fn scenario_ids(&self) -> HashSet<String> {
        self.scenarios()
            .map(|scenario| scenario.id.clone())
            .collect()
    }

    /// How many items of each kind, in the order the kinds first appear.
    #[cfg_attr(
        target_arch = "wasm32",
        expect(dead_code, reason = "only the native hub list reads this")
    )]
    pub(crate) fn kind_counts(&self) -> Vec<(&'static str, usize)> {
        let mut counts: Vec<(&'static str, usize)> = Vec::new();
        for item in self.items() {
            match counts.iter_mut().find(|(kind, _)| *kind == item.kind()) {
                Some((_, count)) => *count += 1,
                None => counts.push((item.kind(), 1)),
            }
        }
        counts
    }

    /// Which file and place holds the item of this kind and id.
    fn locate(&self, kind: &str, id: &str) -> Option<(usize, usize)> {
        self.files.iter().enumerate().find_map(|(file, held)| {
            held.items
                .iter()
                .position(|item| item.kind() == kind && item.id() == id)
                .map(|place| (file, place))
        })
    }

    /// Whether any item but `except` holds `leaf` as a whole string.
    fn mentioned(&self, leaf: &str, except: &str) -> bool {
        self.items()
            .filter(|item| item.id() != except)
            // A design names itself: it keeps the node's id as its display name.
            .filter(|item| !matches!(item, Content::Ship(_) | Content::Station(_)))
            .any(|item| string_leaves(item).iter().any(|held| held == leaf))
    }

    /// Put `lowered`, the designs and scenario of `id`, into the mod.
    ///
    /// The scenario keeps every field a document has no node for: its role, its
    /// world, its thumbnail. Only what the document owns is replaced.
    ///
    /// A design `id` owned and no longer writes goes, unless a spawn that stands
    /// as authored still names it. So does, for an old save, the bare design a
    /// spawn named before this write renamed it, unless something else still
    /// names it. A hand-written design stays either way: a mod may publish one
    /// for other mods to spawn.
    pub(crate) fn absorb(&mut self, id: &str, lowered: Vec<Content>, migrating: bool, file: &str) {
        let mut designs = Vec::new();
        let mut written = None;
        for item in lowered {
            match item {
                Content::Scenario(scenario) => written = Some(scenario),
                other => designs.push(other),
            }
        }
        let Some(written) = written else {
            return;
        };
        let held = self.scenario(id).cloned();
        let stale = self.stale_designs(id, &designs, held.as_ref(), migrating, &written);
        for held_file in &mut self.files {
            held_file
                .items
                .retain(|item| !stale.iter().any(|(kind, id)| same_item(item, kind, id)));
        }
        let unchanged = held.clone();
        let scenario = match held {
            Some(held) => ScenarioConfig {
                name: written.name,
                description: written.description,
                cubemap: written.cubemap,
                skybox_brightness: written.skybox_brightness,
                events: merged_events(&held.events, written.events),
                ..held
            },
            None => written,
        };
        let (at_file, at_place) = match self.locate(SCENARIO_KIND, id) {
            Some(found) => found,
            None => {
                if self.files.is_empty() {
                    self.files.push(ContentFile {
                        path: file.to_string(),
                        items: vec![],
                    });
                }
                let items = &mut self.files[0].items;
                items.push(Arc::new(Content::Scenario(scenario.clone())));
                (0, items.len() - 1)
            }
        };
        // An item that came back the same keeps its `Arc`, and so its file keeps
        // its bytes: see `file_is_written`.
        if format!("{unchanged:?}") != format!("{:?}", Some(&scenario)) {
            self.files[at_file].items[at_place] = Arc::new(Content::Scenario(scenario));
        }
        let mut before = at_place;
        for design in designs {
            match self.locate(design.kind(), design.id()) {
                Some((file, place)) => {
                    if format!("{:?}", self.files[file].items[place]) != format!("{design:?}") {
                        self.files[file].items[place] = Arc::new(design);
                    }
                }
                None => {
                    self.files[at_file].items.insert(before, Arc::new(design));
                    before += 1;
                }
            }
        }
    }

    /// Which designs `absorb` removes, by kind and id.
    fn stale_designs(
        &self,
        id: &str,
        fresh: &[Content],
        held: Option<&ScenarioConfig>,
        migrating: bool,
        written: &ScenarioConfig,
    ) -> Vec<(&'static str, String)> {
        let owner = format!("{id}.");
        let fresh: HashSet<&str> = fresh.iter().map(Content::id).collect();
        let is_design = |item: &&Content| matches!(item, Content::Ship(_) | Content::Station(_));
        let still = prototype_refs(&Content::Scenario(written.clone()));
        let mut stale: Vec<(&'static str, String)> = self
            .items()
            .filter(is_design)
            .filter(|item| item.id().starts_with(&owner) && !fresh.contains(item.id()))
            .filter(|item| !still.iter().any(|leaf| leaf == item.id()))
            .map(|item| (item.kind(), item.id().to_string()))
            .collect();
        if let (true, Some(held)) = (migrating, held) {
            let named = prototype_refs(&Content::Scenario(held.clone()));
            stale.extend(
                self.items()
                    .filter(is_design)
                    .filter(|item| !item.id().starts_with(&owner))
                    .filter(|item| named.iter().any(|leaf| leaf == item.id()))
                    .filter(|item| !still.iter().any(|leaf| leaf == item.id()))
                    .filter(|item| !self.mentioned(item.id(), id))
                    .map(|item| (item.kind(), item.id().to_string())),
            );
        }
        stale
    }

    /// Remove the scenario `id` and the designs only it owned.
    pub(crate) fn remove_scenario(&mut self, id: &str) {
        let owner = format!("{id}.");
        let owned: Vec<(&'static str, String)> = self
            .items()
            .filter(|item| matches!(item, Content::Ship(_) | Content::Station(_)))
            .filter(|item| item.id().starts_with(&owner) && !self.mentioned(item.id(), id))
            .map(|item| (item.kind(), item.id().to_string()))
            .collect();
        for file in &mut self.files {
            file.items.retain(|item| {
                !same_item(item, SCENARIO_KIND, id)
                    && !owned.iter().any(|(kind, id)| same_item(item, kind, id))
            });
        }
    }

    /// Replace the scenario `id` through `change`.
    pub(crate) fn change_scenario(&mut self, id: &str, change: impl FnOnce(&mut ScenarioConfig)) {
        let Some((file, place)) = self.locate(SCENARIO_KIND, id) else {
            return;
        };
        let item = &mut self.files[file].items[place];
        if let Content::Scenario(scenario) = Arc::make_mut(item) {
            change(scenario);
        }
    }

    /// Add `items` after the scenario `after`, in the same file.
    pub(crate) fn insert_after(&mut self, after: &str, items: Vec<Content>) {
        let Some((file, place)) = self.locate(SCENARIO_KIND, after) else {
            return;
        };
        for (offset, item) in items.into_iter().enumerate() {
            self.files[file]
                .items
                .insert(place + 1 + offset, Arc::new(item));
        }
    }
}

fn same_item(item: &Content, kind: &str, id: &str) -> bool {
    item.kind() == kind && item.id() == id
}

/// The open mod. Present only while one is open.
#[derive(Resource, Debug)]
pub(crate) struct ModProject {
    /// The local mod id. `None` until a first save names it, which only a walk
    /// and the web reach: the hub writes a new mod before it opens it.
    pub(crate) id: Option<String>,
    /// What this project's scenario ids start with.
    pub(crate) namespace: String,
    /// The manifest's path in the mod.
    pub(crate) bundle: String,
    /// What undo steps through. Change it through [`Arc::make_mut`].
    pub(crate) data: Arc<ProjectData>,
    /// The data as last read or written, so a save can leave alone a file
    /// nothing touched.
    pub(crate) on_disk: Option<Arc<ProjectData>>,
    /// The scenario of an old save, until a lowering writes it the new way.
    pub(crate) migrate: Option<String>,
    /// The template the first scenario is founded from, on the way into the
    /// editor.
    pub(crate) seed: Option<ScenarioTemplate>,
}

impl ModProject {
    /// A new mod, named `id` once it is on disk, holding nothing yet.
    pub(crate) fn new(
        id: Option<String>,
        name: &str,
        author: &str,
        seed: ScenarioTemplate,
    ) -> Self {
        let namespace = match &id {
            Some(id) => namespace_of(id).to_string(),
            None => scratch_namespace(),
        };
        let bundle = bundle_file(id.as_deref().unwrap_or(&namespace));
        let data = ProjectData {
            meta: new_meta(name, author),
            resources: vec![],
            new_game_scenario: None,
            files: vec![],
            open: None,
        };
        Self {
            id,
            namespace,
            bundle,
            data: Arc::new(data),
            on_disk: None,
            migrate: None,
            seed: Some(seed),
        }
    }

    /// Change the data, making the copy snapshots need.
    pub(crate) fn edit(&mut self) -> &mut ProjectData {
        Arc::make_mut(&mut self.data)
    }

    /// The file a new scenario of this mod goes in, when it holds none.
    pub(crate) fn default_file(&self) -> String {
        content_file(self.id.as_deref().unwrap_or(&self.namespace))
    }

    /// Whether `file` is on disk as it stands.
    #[cfg_attr(
        target_arch = "wasm32",
        expect(dead_code, reason = "only a native save compares files with the disk")
    )]
    fn file_is_written(&self, file: &ContentFile) -> bool {
        let Some(disk) = &self.on_disk else {
            return false;
        };
        disk.files.iter().any(|held| {
            held.path == file.path
                && held.items.len() == file.items.len()
                && held
                    .items
                    .iter()
                    .zip(&file.items)
                    .all(|(a, b)| Arc::ptr_eq(a, b))
        })
    }
}

/// The namespace of a project with no id yet. A token, because two such projects
/// saved later must not share scenario ids.
#[cfg(not(target_arch = "wasm32"))]
fn scratch_namespace() -> String {
    let token = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());
    format!("scratch-{token:x}")
}

/// The web cannot save, so a token buys nothing there.
#[cfg(target_arch = "wasm32")]
fn scratch_namespace() -> String {
    "scratch".to_string()
}

/// What the enabled set must learn once the game leaves Playing.
///
/// Deferred because a change to [`EnabledMods`] re-runs the bundle merge, and a
/// refusal by that merge during Playing is fatal. Leaving runs the merge anyway,
/// which is also the first moment the Scenarios list can be reached.
#[derive(Resource, Debug, Default)]
pub(crate) struct PendingMods {
    /// Ids to switch on: a saved mod and what it depends on.
    pub(crate) enable: BTreeMap<String, ()>,
    /// Ids to switch off: a deleted mod.
    pub(crate) disable: BTreeMap<String, ()>,
}

impl PendingMods {
    /// Queue `id` and its dependencies.
    pub(crate) fn enable(&mut self, ids: impl IntoIterator<Item = String>) {
        for id in ids {
            self.disable.remove(&id);
            self.enable.insert(id, ());
        }
    }

    /// Queue `id` to go.
    pub(crate) fn disable(&mut self, id: &str) {
        self.enable.remove(id);
        self.disable.insert(id.to_string(), ());
    }
}

/// Apply what [`PendingMods`] holds.
pub(crate) fn apply_pending_mods(
    mut pending: ResMut<PendingMods>,
    enabled: Option<ResMut<EnabledMods>>,
) {
    let (enable, disable) = (
        std::mem::take(&mut pending.enable),
        std::mem::take(&mut pending.disable),
    );
    let Some(mut enabled) = enabled else {
        return;
    };
    if enable.is_empty() && disable.is_empty() {
        return;
    }
    enabled.0.extend(enable.into_keys());
    for id in disable.keys() {
        enabled.0.remove(id);
    }
}

/// A fresh scenario id for this project.
pub(crate) fn mint_scenario_id(project: &ModProject, name: &str) -> String {
    scenario_id(&project.namespace, name, &project.data.scenario_ids())
}

/// `written`, the derived layout and then the script, in the shape `held` gave
/// the same handlers.
///
/// The tree holds one flat set of spawns. A file may hold them in any order,
/// over several layout handlers, between its script: that is its text, so each
/// spawn stays where it stood. A new one joins the last layout handler, and a
/// handler that lost its last spawn goes.
fn merged_events(
    held: &[ScenarioEventConfig],
    mut written: Vec<ScenarioEventConfig>,
) -> Vec<ScenarioEventConfig> {
    if written.is_empty() {
        return written;
    }
    let derived = written.remove(0);
    let mut spawns: Vec<ScenarioObjectConfig> = derived
        .actions
        .iter()
        .filter_map(|action| match action {
            EventActionConfig::SpawnScenarioObject(object) => Some(object.clone()),
            _ => None,
        })
        .collect();
    let mut layouts: Vec<(usize, ScenarioEventConfig)> = held
        .iter()
        .enumerate()
        .filter(|(_, event)| is_layout(event))
        .map(|(at, event)| (at, event.clone()))
        .collect();
    if layouts.is_empty() {
        if !derived.actions.is_empty() {
            written.insert(0, derived);
        }
        return written;
    }
    let count = layouts.len();
    for (place, (at, event)) in layouts.iter_mut().enumerate() {
        let stood = !event.actions.is_empty();
        event.actions.retain_mut(|action| {
            let EventActionConfig::SpawnScenarioObject(object) = action else {
                return false;
            };
            let Some(found) = spawns.iter().position(|new| new.base.id == object.base.id) else {
                return false;
            };
            *object = settled(object, spawns.remove(found));
            true
        });
        if place + 1 == count {
            event
                .actions
                .extend(spawns.drain(..).map(EventActionConfig::SpawnScenarioObject));
        }
        if !stood || !event.actions.is_empty() {
            written.insert((*at).min(written.len()), event.clone());
        }
    }
    written
}

/// `written`, with the position `held` authored where the node only carried it.
/// A node holds engine units, and meters do not survive the trip to the last bit.
fn settled(held: &ScenarioObjectConfig, mut written: ScenarioObjectConfig) -> ScenarioObjectConfig {
    let carried = Meters3::from_engine(held.base.position.to_engine());
    if format!("{carried:?}") == format!("{:?}", written.base.position) {
        written.base.position = held.base.position;
    }
    written
}

#[cfg(test)]
pub(crate) mod tests;
