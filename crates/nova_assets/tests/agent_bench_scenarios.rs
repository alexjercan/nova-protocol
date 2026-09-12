//! The loose development scenarios must stay readable by the same loader the
//! agent bench launches through.

use std::path::PathBuf;

use nova_assets::loose::prelude::*;

/// Every checked-in bench fixture parses as one loose scenario and keeps the
/// id named by its file. This catches format drift before a long agent run.
#[test]
fn every_agent_bench_sandbox_is_a_loadable_loose_scenario() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    for (file, id) in [
        ("hunt", "bench_hunt"),
        ("range", "bench_range"),
        ("slingshot", "bench_slingshot"),
        ("arsenal", "bench_arsenal"),
    ] {
        let path = root
            .join("crates/nova_bench/scenarios")
            .join(format!("{file}.content.ron"));
        let scenarios = read_loose_scenarios(&path)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        assert_eq!(scenarios.len(), 1, "{}", path.display());
        assert_eq!(scenarios[0].id, id, "{}", path.display());
    }
}
