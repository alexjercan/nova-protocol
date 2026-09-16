//! Static content lint: the identifier-level checks no load or publish gate
//! can make, because these references resolve at SPAWN time (a scenario naming
//! a section prototype that does not exist loads green and ships a
//! half-spawning ship).
//!
//! Pure functions over parsed config - no assets, no ECS - so one core serves
//! every consumer: the `content` author CLI's `lint` subcommand (nova_assets
//! bin), the CI gate test, and the runtime merge sweep.
//!
//! Static approximations, documented: a reference matching a `ScatterObjects`
//! id prefix counts as satisfiable (the actual `<prefix><n>` ids exist only at
//! runtime); variable set/use is checked scenario-wide, not in firing order.

use std::collections::HashMap;

use nova_ship::prelude::{GameSections, SectionConfig};

use crate::prelude::*;

#[cfg(test)]
mod fixtures;
mod scenario;
mod ship;

pub use scenario::{lint_campaign, lint_scenario};
pub use ship::{lint_section_config, lint_ship_design_config};

/// Glob-import surface: `use nova_scenario::lint::prelude::*` brings the
/// content-lint entry points and result types into scope.
pub mod prelude {
    pub use super::{
        lint_campaign, lint_scenario, lint_section_config, lint_ship_design_config, KnownSections,
        KnownShipDesigns, LintIssue, LintSeverity,
    };
}

/// Last-wins section-prototype view used by scenario lint: the whole resolved
/// config for each visible id, in the shape
/// [`resolve_ship_design`](crate::objects::ship_design::prelude::resolve_ship_design)
/// takes - so the lint resolves a design exactly the way the spawn does and
/// the two can never disagree about what a patch did.
#[derive(Clone, Debug, Default)]
pub struct KnownSections {
    catalog: GameSections,
    index: HashMap<String, usize>,
}

impl KnownSections {
    /// Resolve full section configs in iterator order; later duplicate IDs
    /// replace earlier ones.
    pub fn from_configs<'a>(configs: impl IntoIterator<Item = &'a SectionConfig>) -> Self {
        let mut known = Self::default();
        for config in configs {
            match known.index.get(&config.base.id) {
                Some(&at) => known.catalog.0[at] = config.clone(),
                None => {
                    known
                        .index
                        .insert(config.base.id.clone(), known.catalog.0.len());
                    known.catalog.0.push(config.clone());
                }
            }
        }
        known
    }

    /// Look up one resolved prototype by content ID.
    pub fn get(&self, id: &str) -> Option<&SectionConfig> {
        self.index.get(id).map(|&at| &self.catalog.0[at])
    }

    /// Whether one prototype ID resolves in this catalog.
    pub fn contains(&self, id: &str) -> bool {
        self.index.contains_key(id)
    }

    /// The same prototypes as the catalog resource the resolver reads.
    pub fn catalog(&self) -> &GameSections {
        &self.catalog
    }
}

/// Last-wins design view used by scenario lint: the design each visible id
/// resolves to, so a spawn's `Prototype` reference and its per-section patches
/// can both be checked against a real section list.
#[derive(Clone, Debug, Default)]
pub struct KnownShipDesigns {
    entries: HashMap<String, ShipDesign>,
}

impl KnownShipDesigns {
    /// Resolve full design prototypes in iterator order; later duplicate ids
    /// replace earlier ones.
    pub fn from_configs<'a>(configs: impl IntoIterator<Item = &'a ShipDesignPrototype>) -> Self {
        let mut entries = HashMap::new();
        for config in configs {
            entries.insert(config.id.clone(), config.design.clone());
        }
        Self { entries }
    }

    /// The design one id resolves to, or `None` if nothing authored it.
    pub fn get(&self, id: &str) -> Option<&ShipDesign> {
        self.entries.get(id)
    }
}

/// How bad a finding is: `Error` fails gates (the content WILL misbehave),
/// `Warn` is reported but does not fail (almost certainly an authoring bug,
/// but the scenario still runs - e.g. a fails-closed unset variable).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LintSeverity {
    /// Fails gates: the content will misbehave at runtime.
    Error,
    /// Reported but non-fatal: almost certainly an authoring bug, but the
    /// scenario still runs.
    Warn,
}

/// One finding, human-readable and self-contained ("scenario 'x': unknown
/// section prototype 'y'").
#[derive(Clone, Debug)]
pub struct LintIssue {
    /// How bad the finding is.
    pub severity: LintSeverity,
    /// The scenario the finding is about.
    pub scenario: ScenarioId,
    /// The human-readable, self-contained description of the finding.
    pub message: String,
}

impl LintIssue {
    pub(crate) fn error(scenario: &str, message: String) -> Self {
        Self {
            severity: LintSeverity::Error,
            scenario: scenario.to_string(),
            message,
        }
    }

    pub(crate) fn warn(scenario: &str, message: String) -> Self {
        Self {
            severity: LintSeverity::Warn,
            scenario: scenario.to_string(),
            message,
        }
    }
}
