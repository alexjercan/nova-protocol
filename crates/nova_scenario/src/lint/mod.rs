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

use nova_ship::prelude::{LinkPoint, SectionConfig, SectionKind};

use crate::prelude::*;

#[cfg(test)]
mod fixtures;
mod scenario;
mod ship;

pub use scenario::{lint_campaign, lint_scenario};
pub use ship::lint_section_config;

/// Glob-import surface: `use nova_scenario::lint::prelude::*` brings the
/// content-lint entry points and result types into scope.
pub mod prelude {
    pub use super::{
        lint_campaign, lint_scenario, lint_section_config, KnownSection, KnownSections, LintIssue,
        LintSeverity,
    };
}

/// Resolved lint-relevant data for one visible section prototype.
#[derive(Clone, Debug, Default)]
pub struct KnownSection {
    /// Whether the prototype is a turret or torpedo mount.
    pub mounts: bool,
    /// Structural sockets copied from the resolved prototype.
    pub link_points: Vec<LinkPoint>,
}

/// Last-wins section-prototype view used by scenario lint.
#[derive(Clone, Debug, Default)]
pub struct KnownSections {
    entries: HashMap<String, KnownSection>,
}

impl KnownSections {
    /// Whether a section kind mounts by its -Y base face.
    pub fn kind_mounts(kind: &SectionKind) -> bool {
        matches!(kind, SectionKind::Turret(_) | SectionKind::Torpedo(_))
    }

    /// Resolve full section configs in iterator order; later duplicate IDs replace earlier ones.
    pub fn from_configs<'a>(configs: impl IntoIterator<Item = &'a SectionConfig>) -> Self {
        let mut entries = HashMap::new();
        for config in configs {
            entries.insert(
                config.base.id.clone(),
                KnownSection {
                    mounts: Self::kind_mounts(&config.kind),
                    link_points: config.base.link_points.clone(),
                },
            );
        }
        Self { entries }
    }

    /// Look up one resolved prototype by content ID.
    pub fn get(&self, id: &str) -> Option<&KnownSection> {
        self.entries.get(id)
    }

    /// Whether one prototype ID resolves in this catalog.
    pub fn contains(&self, id: &str) -> bool {
        self.entries.contains_key(id)
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
