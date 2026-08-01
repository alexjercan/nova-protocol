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

use std::collections::HashSet;

use nova_gameplay::prelude::{SectionConfig, SectionKind};

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
        lint_campaign, lint_scenario, lint_section_config, KnownSections, LintIssue, LintSeverity,
    };
}

/// The section-prototype view a caller lints against: every visible
/// prototype id, plus the subset that MOUNTS - kinds whose model has a base
/// face at local -Y that must sit flush against a neighboring section
/// (turrets and torpedo bays; the turret turntable and the bay hatch sit at
/// +Y in the GLB vertex data). Built from full configs via
/// [`KnownSections::from_configs`] so the kind classification lives in ONE
/// place for every caller (author CLI walk, CI gate, runtime merge sweep).
#[derive(Clone, Debug, Default)]
pub struct KnownSections {
    /// Every visible section-prototype id.
    pub ids: HashSet<String>,
    /// The ids whose every visible definition is a mount kind. Conservative
    /// on cross-bundle id conflicts (a mod overriding a mount id with a
    /// hull, say): a contested id is NOT treated as a mount, so the
    /// adjacency check can under-flag but never false-fail. The static
    /// walk unions every VISIBLE definition and is where a contested id
    /// can under-flag; the runtime merge gate classifies from the actual
    /// last-wins overlay, so it is the accurate one - conflicting content
    /// can pass CI yet still be refused in-game.
    pub mounts: HashSet<String>,
}

impl KnownSections {
    /// Whether a section kind mounts by its -Y base face.
    pub fn kind_mounts(kind: &SectionKind) -> bool {
        matches!(kind, SectionKind::Turret(_) | SectionKind::Torpedo(_))
    }

    /// Classify full section configs into the catalog view.
    pub fn from_configs<'a>(configs: impl IntoIterator<Item = &'a SectionConfig>) -> Self {
        let mut ids = HashSet::new();
        let mut mounts = HashSet::new();
        let mut non_mounts = HashSet::new();
        for config in configs {
            let id = &config.base.id;
            if Self::kind_mounts(&config.kind) {
                mounts.insert(id.clone());
            } else {
                non_mounts.insert(id.clone());
            }
            ids.insert(id.clone());
        }
        mounts.retain(|id| !non_mounts.contains(id));
        Self { ids, mounts }
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
