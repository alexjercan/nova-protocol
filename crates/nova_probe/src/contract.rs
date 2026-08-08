//! What the EXAMPLE claims, declared by the plugins it wires.
//!
//! `probe-run.json` says what probe ARMED; `probe-contract.json` says what the
//! app can produce. The pair is what lets a report tell "makes no such claim"
//! apart from "claimed it, was armed, and produced nothing" - the second is a
//! failure, the first is not, and a single missing artifact looks identical
//! without the contract.
//!
//! Every probe plugin calls [`declare`] at the TOP of its `build`, above its
//! own arming guard: wiring is the claim, arming is probe's answer to it. The
//! first `declare` also registers the Startup system that writes the file, so
//! an app wiring no probe plugin writes nothing at all.
//!
//! Two properties worth stating:
//!
//! - **It is the BUILD's claim, not the file's.** Examples wire probe inside
//!   `#[cfg(feature = "debug")]`; probe always builds `--features debug`, so
//!   the contract reflects the binary that actually ran.
//! - **Absent is not empty.** A run dir with no `probe-contract.json` predates
//!   the contract (or is a web run, which has no filesystem). Consumers hold
//!   it as `Option` and must not read absence as "declares nothing".

use std::collections::BTreeSet;

use bevy::prelude::*;

/// Param (via [`perf_param`](crate::capabilities::frametime::perf_param), so
/// `NOVA_PERF_CONTRACT` on native) naming the path the contract is written to.
/// Unset - a hand-run example - writes nothing.
pub const CONTRACT_PARAM: &str = "contract";

/// One evidence surface, one plugin. Adding a probe plugin that produces an
/// artifact a check reads means adding a variant here and one `declare` line
/// in its `build`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Capability {
    /// `nova_timeline()` - the run-timeline recorder (`timeline.jsonl`).
    Timeline,
    /// `nova_invariants()` - the continuous invariant checks, recorded onto
    /// the timeline.
    Invariants,
    /// `nova_frametime()` - the frame-time capture (`frametime.csv`).
    FrameTime,
}

impl Capability {
    /// The stable wire name used in `probe-contract.json`.
    pub fn as_str(self) -> &'static str {
        match self {
            Capability::Timeline => "timeline",
            Capability::Invariants => "invariants",
            Capability::FrameTime => "frametime",
        }
    }

    /// Parse a wire name. `None` for anything unknown - the caller decides
    /// whether that is an error (it is, for the report).
    pub fn from_wire(name: &str) -> Option<Self> {
        match name {
            "timeline" => Some(Capability::Timeline),
            "invariants" => Some(Capability::Invariants),
            "frametime" => Some(Capability::FrameTime),
            _ => None,
        }
    }

    /// The call an example makes to claim this capability. Reports name it so
    /// a "makes no such claim" row says what to add, not just what is missing.
    pub fn wiring(self) -> &'static str {
        match self {
            Capability::Timeline => "nova_probe::nova_timeline()",
            Capability::Invariants => "nova_probe::nova_invariants()",
            Capability::FrameTime => "nova_probe::nova_frametime()",
        }
    }
}

/// The capabilities this app wired. Built up during plugin `build`, written
/// once at Startup.
#[derive(Resource, Default, Debug, Clone, PartialEq, Eq)]
pub struct ProbeContract {
    declared: BTreeSet<Capability>,
}

impl ProbeContract {
    /// A contract declaring exactly `capabilities`. For tests and for
    /// consumers building an expectation to compare against.
    pub fn of(capabilities: impl IntoIterator<Item = Capability>) -> Self {
        Self {
            declared: capabilities.into_iter().collect(),
        }
    }

    /// Whether the app wired the plugin behind `capability`.
    pub fn declares(&self, capability: Capability) -> bool {
        self.declared.contains(&capability)
    }

    /// Serialize for `probe-contract.json`.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "declared": self.declared.iter().map(|c| c.as_str()).collect::<Vec<_>>(),
            "generated_by": "nova_probe contract",
        })
    }

    /// Parse `probe-contract.json`. Loud on malformed content and on an
    /// unknown capability name: a contract read as fewer claims than it makes
    /// turns a real gap into a silent "not applicable".
    pub fn from_json(contents: &str) -> Result<Self, String> {
        let value: serde_json::Value =
            serde_json::from_str(contents).map_err(|e| format!("probe-contract.json: {e}"))?;
        let declared = value
            .get("declared")
            .and_then(|d| d.as_array())
            .ok_or("probe-contract.json: missing declared")?;
        declared
            .iter()
            .map(|entry| {
                let name = entry
                    .as_str()
                    .ok_or("probe-contract.json: declared entry is not a string")?;
                Capability::from_wire(name)
                    .ok_or_else(|| format!("probe-contract.json: unknown capability {name:?}"))
            })
            .collect::<Result<BTreeSet<_>, String>>()
            .map(|declared| Self { declared })
    }
}

/// Declare `capability`, from a plugin's `build` and ABOVE its arming guard.
/// Idempotent: repeated calls (and a plugin added twice) add one claim and
/// register one writer.
pub fn declare(app: &mut App, capability: Capability) {
    if !app.world().contains_resource::<ProbeContract>() {
        app.init_resource::<ProbeContract>();
        #[cfg(not(target_arch = "wasm32"))]
        app.add_systems(Startup, write_contract);
    }
    app.world_mut()
        .resource_mut::<ProbeContract>()
        .declared
        .insert(capability);
}

/// Write the contract out when probe named a path. Failing to write is an
/// ERROR, never silence: `log_clean` greps whole-word ERROR, so a lost
/// contract fails the run instead of reading as "claims nothing".
#[cfg(not(target_arch = "wasm32"))]
fn write_contract(contract: Res<ProbeContract>) {
    let Some(path) = crate::capabilities::frametime::perf_param(CONTRACT_PARAM) else {
        return;
    };
    let path = std::path::PathBuf::from(path);
    match write_to(&path, &contract) {
        Ok(()) => info!("nova probe: contract -> {path:?}"),
        Err(error) => error!("nova probe: contract not written to {path:?}: {error}"),
    }
}

/// Write `contract` to `path` via temp file + rename, so a reader can never
/// observe a half-written contract.
#[cfg(not(target_arch = "wasm32"))]
fn write_to(path: &std::path::Path, contract: &ProbeContract) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temp = path.with_extension("json.tmp");
    std::fs::write(&temp, contract.to_json().to_string())?;
    std::fs::rename(&temp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("nova-contract-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_contract_round_trips_through_json() {
        let contract = ProbeContract::of([Capability::FrameTime, Capability::Timeline]);
        let parsed =
            ProbeContract::from_json(&contract.to_json().to_string()).expect("round-trips");
        assert_eq!(parsed, contract);
        assert!(parsed.declares(Capability::Timeline));
        assert!(!parsed.declares(Capability::Invariants));
    }

    #[test]
    fn an_unknown_capability_is_an_error_never_a_thinner_contract() {
        let json = r#"{"declared":["timeline","warp_drive"]}"#;
        let error = ProbeContract::from_json(json).expect_err("must not parse");
        assert!(error.contains("warp_drive"), "{error}");
        // ... and malformed content is not "declares nothing" either.
        assert!(ProbeContract::from_json("{}").is_err());
        assert!(ProbeContract::from_json("not json").is_err());
    }

    #[test]
    fn declaring_is_idempotent_and_accumulates_across_plugins() {
        let mut app = App::new();
        declare(&mut app, Capability::Timeline);
        declare(&mut app, Capability::Timeline);
        declare(&mut app, Capability::Invariants);
        let contract = app.world().resource::<ProbeContract>();
        assert!(contract.declares(Capability::Timeline));
        assert!(contract.declares(Capability::Invariants));
        // Serialized in enum order, and the repeat left no duplicate.
        assert_eq!(
            contract.to_json()["declared"],
            serde_json::json!(["timeline", "invariants"])
        );
    }

    /// The point of the whole module: an UNARMED plugin still declares. Each
    /// of these three returns early without env, and the claim survives it.
    #[test]
    fn wiring_a_plugin_declares_even_when_the_run_arms_nothing() {
        let mut app = App::new();
        app.add_plugins((
            crate::nova_timeline(),
            crate::nova_invariants(),
            crate::nova_frametime(),
        ));
        let contract = app.world().resource::<ProbeContract>().clone();
        assert_eq!(
            contract,
            ProbeContract::of([
                Capability::Timeline,
                Capability::Invariants,
                Capability::FrameTime
            ])
        );
    }

    #[test]
    fn the_write_is_atomic_and_leaves_no_temp_behind() {
        let dir = scratch("write");
        let path = dir.join("probe-contract.json");
        write_to(&path, &ProbeContract::of([Capability::FrameTime])).unwrap();
        let parsed = ProbeContract::from_json(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(parsed, ProbeContract::of([Capability::FrameTime]));
        assert!(!path.with_extension("json.tmp").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
