//! Live validation of the document: the Problems tab and the Inspector's
//! validity word both read [`Problems`].
//!
//! The lint runs on the lowered document, the same config Play would load, so a
//! problem here is one the start gate would raise. It runs when the history
//! revision moves, after [`SETTLE_SECS`], and never while a mouse button is
//! held. A frame with no edit costs one comparison.

use std::collections::HashSet;

use bevy::{ecs::system::SystemParam, prelude::*};
use nova_assets::mod_refs::prelude::{resource_ref_violations, SELF_SCHEME};
use nova_gameplay::prelude::{GameItems, GameTradeProfiles};
use nova_modding::prelude::Content;
use nova_scenario::prelude::*;
use nova_ship::prelude::{GameSections, GameStyles, TargetingSettings};

use crate::{
    asset::{lower_bench, AssetNode, AssetPartType, SocketNode},
    asset_index::AssetIndex,
    event::ScriptNodes,
    history::EditHistory,
    node::{
        sections_of, EditContext, NodeId, ObjectNode, ObjectNodes, ScenarioNode, SectionNode,
        SectionNodes, ShipNode,
    },
    project::{package::asset_findings, ModProject, ProjectData},
    scenario::{
        lower_fleet, lower_ship, sandbox_scenario, ship_design, world_objects, world_script,
        world_settings, HullForm,
    },
    station::{short_fault, StationCatalogs},
};

/// How long the document must rest before the lint runs. A burst of typed edits
/// costs one lint.
const SETTLE_SECS: f64 = 0.25;

/// The source a section finding is filed under. The panel shows the message
/// only, so this never reaches the screen.
const LINT_SOURCE: &str = "editor";

/// How bad a problem is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ProblemSeverityType {
    /// The start gate refuses this document.
    Error,
    /// The document runs, and this is almost certainly a mistake.
    Warning,
}

/// One finding about the document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Problem {
    /// How bad it is.
    pub(crate) severity: ProblemSeverityType,
    /// What the lint said.
    pub(crate) message: String,
    /// The node the message names. `None` when it names no node, which is a
    /// fact about the message and not an error.
    pub(crate) node: Option<Entity>,
}

/// The findings of the last lint, worst first.
#[derive(Resource, Default)]
pub(crate) struct Problems {
    /// The findings.
    pub(crate) list: Vec<Problem>,
    /// What the last Package found. Kept apart from the document's lint, which
    /// rewrites `list`, and gone with the next edit: the mod has changed.
    project: Vec<Problem>,
    /// The history revision they describe.
    seen: Option<u64>,
    /// When the document last moved, while a lint is owed.
    moved_at: Option<f64>,
}

impl Problems {
    /// Put a project's findings beside the document's.
    #[cfg_attr(
        target_arch = "wasm32",
        expect(
            dead_code,
            reason = "only Package reads this, and the web has no Package"
        )
    )]
    pub(crate) fn set_project(&mut self, found: Vec<Problem>) {
        self.project = found;
        self.show_project();
    }

    /// Drop what was found on the last document, and lint the new one next
    /// frame. Its nodes are gone and its revision does not move on a switch.
    pub(crate) fn document_switched(&mut self) {
        self.list.clear();
        self.seen = None;
        self.moved_at = None;
    }

    /// Rebuild the list with the project's findings on it, worst first.
    fn show_project(&mut self) {
        self.list.extend(self.project.iter().cloned());
        self.list.sort_by_key(|problem| problem.severity);
        self.list.dedup();
    }

    /// How many findings are errors and how many are warnings.
    pub(crate) fn counts(&self) -> (usize, usize) {
        let errors = self
            .list
            .iter()
            .filter(|problem| problem.severity == ProblemSeverityType::Error)
            .count();
        (errors, self.list.len() - errors)
    }

    /// The same counts for one node.
    pub(crate) fn counts_of(&self, node: Entity) -> (usize, usize) {
        let named = || self.list.iter().filter(|p| p.node == Some(node));
        let errors = named()
            .filter(|problem| problem.severity == ProblemSeverityType::Error)
            .count();
        (errors, named().count() - errors)
    }
}

/// The node whose id a message names, longest id first, so `rock_12` is not
/// read as `rock_1`.
pub(crate) fn named_node(message: &str, ids: &[(String, Entity)]) -> Option<Entity> {
    let mut ids: Vec<&(String, Entity)> = ids.iter().collect();
    ids.sort_by_key(|(id, _)| std::cmp::Reverse(id.len()));
    ids.into_iter()
        .find(|(id, _)| names_id(message, id))
        .map(|(_, node)| *node)
}

/// Whether `id` stands in `message` as a whole word.
fn names_id(message: &str, id: &str) -> bool {
    let word = |letter: Option<char>| {
        letter.is_some_and(|letter| letter.is_alphanumeric() || letter == '_')
    };
    message.match_indices(id).any(|(at, _)| {
        !word(message[..at].chars().next_back()) && !word(message[at + id.len()..].chars().next())
    })
}

/// Everything the lint reads.
#[derive(SystemParam)]
pub(crate) struct ProblemSources<'w, 's> {
    context: Res<'w, EditContext>,
    q_ships: Query<
        'w,
        's,
        (
            Entity,
            &'static NodeId,
            &'static ShipNode,
            &'static Transform,
        ),
    >,
    q_settings: Query<'w, 's, &'static ScenarioNode>,
    q_objects: ObjectNodes<'w, 's>,
    q_named: Query<
        'w,
        's,
        (Entity, &'static NodeId),
        Or<(
            With<ShipNode>,
            With<ObjectNode>,
            With<SectionNode>,
            With<SocketNode>,
        )>,
    >,
    nodes: SectionNodes<'w, 's>,
    script: ScriptNodes<'w, 's>,
    assets: AssetIndex<'w>,
    project: Option<Res<'w, ModProject>>,
    catalogs: StationCatalogs<'w>,
    targeting: Option<Res<'w, TargetingSettings>>,
    sections: Option<Res<'w, GameSections>>,
    ships: Option<Res<'w, GameShipDesigns>>,
    stations: Option<Res<'w, GameStationDesigns>>,
    scenarios: Option<Res<'w, GameScenarios>>,
    items: Option<Res<'w, GameItems>>,
    styles: Option<Res<'w, GameStyles>>,
    profiles: Option<Res<'w, GameTradeProfiles>>,
    bench: Query<'w, 's, &'static AssetNode>,
    q_sockets: Query<'w, 's, (&'static NodeId, &'static Transform, &'static SocketNode)>,
    q_parts: Query<'w, 's, (Entity, &'static AssetPartType)>,
}

impl ProblemSources<'_, '_> {
    /// What the project holds, with the bench's item as it stands right now.
    fn live_data(&self, project: &ModProject) -> Option<(ProjectData, Entity)> {
        let root = self.context.scenario()?;
        let node = self.bench.get(root).ok()?;
        let open = project.data.open_bench()?;
        let mut data = project.data.as_ref().clone();
        data.replace_item(&open.id, lower_bench(node, self.q_sockets.iter()));
        Some((data, root))
    }

    /// Lint the item on the bench. A bench's root is no scenario to lint.
    fn bench_lint(&self) -> Option<Vec<Problem>> {
        let project = self.project.as_deref()?;
        let (data, root) = self.live_data(project)?;
        let own = project.id.as_deref().unwrap_or(&project.namespace);
        let ids: Vec<(String, Entity)> = self
            .q_named
            .iter()
            .map(|(node, id)| (id.0.clone(), node))
            .collect();
        let mesh = self
            .q_parts
            .iter()
            .find(|(_, part)| **part == AssetPartType::Mesh)
            .map(|(node, _)| node);
        let place = |message: &str| {
            let about_mesh = message.contains("mesh ") || message.contains(SELF_SCHEME);
            about_mesh
                .then_some(mesh)
                .flatten()
                .or_else(|| named_node(message, &ids))
                .or(Some(root))
        };
        let mut found = asset_findings(own, &data, &self.assets);
        for problem in &mut found {
            problem.node = place(&problem.message);
        }
        found.extend(
            self.ref_faults(project, &data)
                .into_iter()
                .map(|message| Problem {
                    severity: ProblemSeverityType::Error,
                    node: place(&message),
                    message,
                }),
        );
        found.sort_by_key(|problem| problem.severity);
        found.dedup();
        Some(found)
    }

    /// The refs in `data` the merge gate would refuse, and the dependencies it
    /// could not find.
    fn ref_faults(&self, project: &ModProject, data: &ProjectData) -> Vec<String> {
        let mut faults = self.assets.scoped(|scope| {
            data.items()
                .flat_map(|item| resource_ref_violations(item, scope))
                .collect::<Vec<_>>()
        });
        for dep in &project.data.meta.dependencies {
            if !self.assets.has_bundle(dep) {
                faults.push(format!("dependency {dep} is not loaded"));
            }
        }
        faults
    }

    /// Lint the ship of a ship's document as a design: its sections and their
    /// links, the station faults Play would refuse, and what the merge gate
    /// would refuse in the mod. A ship's document is no scenario, so nothing a
    /// scenario owes (a player, a world, a script) is asked of it.
    fn ship_lint(&self) -> Option<Vec<Problem>> {
        let project = self.project.as_deref()?;
        let open = project.data.open.as_ref()?;
        if !crate::project::ship::is_ship_kind(open.kind) {
            return None;
        }
        let ids: Vec<(String, Entity)> = self
            .q_named
            .iter()
            .map(|(node, id)| (id.0.clone(), node))
            .collect();
        let fleet = lower_fleet(&self.q_ships, &self.nodes);
        let mut found = Vec::new();
        for (name, faults) in self.catalogs.refused(&fleet) {
            for fault in faults {
                let message = format!("{name} cannot be a station: {}", short_fault(&fault));
                found.push(Problem {
                    severity: ProblemSeverityType::Error,
                    node: named_node(&message, &ids).or_else(|| self.ship_named(&name)),
                    message,
                });
            }
        }
        let no_sections = GameSections::default();
        let sections = self.sections.as_deref().unwrap_or(&no_sections);
        let known_sections = KnownSections::from_configs(&sections.0);
        let no_items = GameItems::default();
        let no_styles = GameStyles::default();
        let issues: Vec<LintIssue> = self
            .q_ships
            .iter()
            .flat_map(|(ship, id, node, _)| {
                let lowered = lower_ship(ship, id, node, Transform::IDENTITY, &self.nodes);
                let design = ShipDesignPrototype {
                    id: open.id.as_str().into(),
                    name: node.name.clone(),
                    design: ship_design(&lowered),
                };
                lint_ship_design_config(
                    &design,
                    &known_sections,
                    self.items.as_deref().unwrap_or(&no_items),
                    self.styles.as_deref().unwrap_or(&no_styles),
                    LINT_SOURCE,
                )
            })
            .collect();
        for issue in issues {
            found.push(Problem {
                severity: match issue.severity {
                    LintSeverity::Error => ProblemSeverityType::Error,
                    LintSeverity::Warn => ProblemSeverityType::Warning,
                },
                node: named_node(&issue.message, &ids),
                message: issue.message,
            });
        }
        let own = project.id.as_deref().unwrap_or(&project.namespace);
        found.extend(asset_findings(own, &project.data, &self.assets));
        found.extend(
            self.ref_faults(project, &project.data)
                .into_iter()
                .map(|message| Problem {
                    severity: ProblemSeverityType::Error,
                    message,
                    node: None,
                }),
        );
        found.sort_by_key(|problem| problem.severity);
        found.dedup();
        Some(found)
    }

    /// Lint the document as it stands.
    fn lint(&self) -> Vec<Problem> {
        if let Some(found) = self.bench_lint().or_else(|| self.ship_lint()) {
            return found;
        }
        let targeting = self.targeting.as_deref().cloned().unwrap_or_default();
        let fleet = lower_fleet(&self.q_ships, &self.nodes);
        let scenario = self.assets.resolved(sandbox_scenario(
            &world_settings(&self.context, &self.q_settings),
            world_objects(&self.context, &self.q_objects),
            &fleet,
            world_script(&self.context, &self.script),
            &targeting,
        ));
        let ids: Vec<(String, Entity)> = self
            .q_named
            .iter()
            .map(|(node, id)| (id.0.clone(), node))
            .collect();
        let mut found = Vec::new();

        // A block the file cannot spell: lowering would drop it, so the lint
        // below never sees it. Said here, with the block that holds the gap.
        if let Some(open) = self.context.scenario() {
            for (node, message) in self.script.gaps(open) {
                found.push(Problem {
                    severity: ProblemSeverityType::Error,
                    message,
                    node: Some(node),
                });
            }
        }

        // Faults Play refuses before the lint ever sees the hull.
        for (name, faults) in self.catalogs.refused(&fleet) {
            for fault in faults {
                let message = format!("{name} cannot be a station: {}", short_fault(&fault));
                found.push(Problem {
                    severity: ProblemSeverityType::Error,
                    node: named_node(&message, &ids).or_else(|| self.ship_named(&name)),
                    message,
                });
            }
        }

        let no_sections = GameSections::default();
        let sections = self.sections.as_deref().unwrap_or(&no_sections);
        let known_sections = KnownSections::from_configs(&sections.0);
        let known_ships = KnownShipDesigns::from_configs(
            self.ships
                .as_deref()
                .map_or(&[][..], |registry| &registry.0),
        );
        // Play registers a marked hull's station design for the session, so the
        // lint has to see those designs or it reports every station as unknown.
        let designs: Vec<StationDesignPrototype> = self
            .catalogs
            .designs(&fleet, HullForm::Inline)
            .into_iter()
            .map(|(design, _)| design)
            .collect();
        let known_stations = KnownStationDesigns::from_configs(
            self.stations
                .as_deref()
                .map_or(&[][..], |registry| &registry.0)
                .iter()
                .chain(&designs),
        );
        let mut known_scenarios: HashSet<String> = self
            .scenarios
            .as_deref()
            .map(|registry| registry.0.keys().cloned().collect())
            .unwrap_or_default();
        known_scenarios.insert(scenario.id.clone());
        let no_items = GameItems::default();
        let no_styles = GameStyles::default();
        let no_profiles = GameTradeProfiles::default();
        let issues = lint_scenario(
            &scenario,
            &known_sections,
            &known_ships,
            &known_stations,
            &known_scenarios,
            self.items.as_deref().unwrap_or(&no_items),
            self.styles.as_deref().unwrap_or(&no_styles),
            self.profiles.as_deref().unwrap_or(&no_profiles),
        );

        // Each prototype once, however many hulls carry it.
        let mut linted = HashSet::new();
        let section_issues: Vec<LintIssue> = self
            .q_ships
            .iter()
            .flat_map(|(ship, ..)| sections_of(ship, &self.nodes))
            .filter_map(|(_, _, section, _)| section.resolve(self.sections.as_deref()))
            .filter(|config| linted.insert(config.base.id.clone()))
            .flat_map(|config| lint_section_config(config, LINT_SOURCE))
            .collect();
        for issue in issues.into_iter().chain(section_issues) {
            found.push(Problem {
                severity: match issue.severity {
                    LintSeverity::Error => ProblemSeverityType::Error,
                    LintSeverity::Warn => ProblemSeverityType::Warning,
                },
                node: named_node(&issue.message, &ids).or_else(|| {
                    let open = self.context.scenario()?;
                    self.script.located(open, &issue.message)
                }),
                message: issue.message,
            });
        }
        found.extend(self.project_faults(&scenario));
        found.sort_by_key(|problem| problem.severity);
        found.dedup();
        found
    }

    /// What in the project the merge gate would refuse, with the live document
    /// standing in for what the project holds of the open scenario. Each of
    /// these stops Play as well, because a refusal after saving is too late.
    fn project_faults(&self, live: &ScenarioConfig) -> Vec<Problem> {
        let Some(project) = self.project.as_deref() else {
            return vec![];
        };
        let mut data = project.data.as_ref().clone();
        if let Some(open) = project.data.open_scenario() {
            data.replace_item(open, Content::Scenario(live.clone()));
        }
        let own = project.id.as_deref().unwrap_or(&project.namespace);
        let mut found = asset_findings(own, &data, &self.assets);
        found.extend(
            self.ref_faults(project, &data)
                .into_iter()
                .map(|message| Problem {
                    severity: ProblemSeverityType::Error,
                    message,
                    node: None,
                }),
        );
        found
    }

    /// The ship a station fault names by its display name.
    fn ship_named(&self, name: &str) -> Option<Entity> {
        self.q_ships
            .iter()
            .find(|(_, _, ship, _)| ship.name == name)
            .map(|(ship, ..)| ship)
    }
}

/// Run the lint once the document has settled.
pub(crate) fn sync_problems(
    time: Res<Time<Real>>,
    history: Res<EditHistory>,
    buttons: Option<Res<ButtonInput<MouseButton>>>,
    mut problems: ResMut<Problems>,
    sources: ProblemSources,
) {
    let revision = history.revision();
    let now = time.elapsed_secs_f64();
    if problems.seen.is_none() {
        problems.list = sources.lint();
        problems.show_project();
        problems.seen = revision.or(Some(0));
        return;
    }
    if revision.is_none() || revision == problems.seen {
        problems.moved_at = None;
        return;
    }
    let moved_at = *problems.moved_at.get_or_insert(now);
    let held = buttons.is_some_and(|buttons| buttons.get_pressed().next().is_some());
    if held || now - moved_at < SETTLE_SECS {
        return;
    }
    problems.project.clear();
    problems.list = sources.lint();
    problems.seen = revision;
    problems.moved_at = None;
}

/// Forget the last lint, so a new visit to the editor runs one at once.
pub(crate) fn reset_problems(mut problems: ResMut<Problems>) {
    *problems = Problems::default();
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use super::{named_node, Problem, ProblemSeverityType, Problems};
    use crate::history::mark_document_switched;

    /// A row left over from the asset before the switch names a node that is
    /// gone, and a finding the new asset does not have.
    #[test]
    fn a_switched_document_drops_the_findings_of_the_last() {
        let mut world = World::new();
        let gone = world.spawn_empty().id();
        world.despawn(gone);
        world.insert_resource(Problems {
            list: vec![Problem {
                severity: ProblemSeverityType::Error,
                message: "the other asset's fault".to_string(),
                node: Some(gone),
            }],
            seen: Some(4),
            ..default()
        });

        mark_document_switched(&mut world);

        let problems = world.resource::<Problems>();
        assert!(problems.list.is_empty());
        assert_eq!(problems.seen, None, "the next frame lints the new asset");
    }

    #[test]
    fn a_message_names_the_longest_matching_id() {
        let mut world = World::new();
        let (short, long) = (world.spawn_empty().id(), world.spawn_empty().id());
        let ids = [("rock_1".to_string(), short), ("rock_12".to_string(), long)];
        assert_eq!(named_node("object 'rock_12' overlaps", &ids), Some(long));
        assert_eq!(named_node("object 'rock_1' overlaps", &ids), Some(short));
    }

    #[test]
    fn an_id_inside_a_longer_word_is_not_a_mention() {
        let mut world = World::new();
        let node = world.spawn_empty().id();
        let ids = [("rock".to_string(), node)];
        assert_eq!(named_node("the bedrock_field is empty", &ids), None);
    }
}
