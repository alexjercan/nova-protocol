//! Pure mod-dependency resolution over an `id -> dependency-ids` graph (task
//! 20260715-142931). Engine-free like the rest of this crate, so the asset
//! merge (`register_bundles`), the menu's enable/install flows and their tests
//! all share ONE implementation. Ids only - no version constraints.
//!
//! `base` is an IMPLICIT dependency (see [`ModMeta::dependencies`](crate::ModMeta))
//! and never appears in a graph here; callers seed it separately. An id absent
//! from the graph is treated as having no dependencies.

use std::collections::{HashMap, HashSet};

/// The dependency graph: mod id -> the ids it DIRECTLY declares. Callers build
/// it from whatever carries `ModMeta` (loaded bundles, the catalog, the portal).
pub type DepGraph = HashMap<String, Vec<String>>;

/// A mod's direct dependency ids in `graph` (empty slice if the id is absent).
fn direct<'a>(graph: &'a DepGraph, id: &str) -> &'a [String] {
    graph.get(id).map(Vec::as_slice).unwrap_or(&[])
}

/// Every TRANSITIVE dependency of `id` (NOT including `id` itself), in DFS
/// post-order so a dependency always appears before the mods that need it.
/// Cycle-tolerant: each id is visited once. Unknown ids contribute nothing.
pub fn transitive_deps(graph: &DepGraph, id: &str) -> Vec<String> {
    fn visit(graph: &DepGraph, id: &str, seen: &mut HashSet<String>, out: &mut Vec<String>) {
        for dep in direct(graph, id) {
            if seen.insert(dep.clone()) {
                visit(graph, dep, seen, out);
                out.push(dep.clone());
            }
        }
    }
    let mut seen = HashSet::new();
    // Seed with `id` so a self-edge (or a cycle back to the root) is ignored and
    // the root never lists itself.
    seen.insert(id.to_string());
    let mut out = Vec::new();
    visit(graph, id, &mut seen, &mut out);
    out
}

/// The result of [`topological_order`]: the ordered ids, plus whether a
/// dependency CYCLE was detected (the cyclic ids are still emitted, in input
/// order, so the caller can warn but proceed).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TopoOrder {
    /// `ids` reordered so every id follows all of its in-set dependencies.
    pub order: Vec<String>,
    /// True when the in-set edges contained a cycle (some ids could not be
    /// ordered by dependency and kept their input order).
    pub cycle: bool,
}

/// Order `ids` so every id comes AFTER all of its dependencies that are also in
/// `ids` (Kahn's algorithm). The one hard guarantee is dependencies-before-
/// dependents; the tiebreak is input order among ids that become ready in the
/// SAME relaxation round (so a node blocked early can trail independent later
/// nodes - e.g. input `[b, a, c]` with `b -> a` yields `[a, c, b]`).
/// Dependencies outside `ids` are ignored (only intra-set edges order the
/// result). A cycle emits its members in input order and sets `cycle`.
/// Deterministic.
pub fn topological_order(ids: &[String], graph: &DepGraph) -> TopoOrder {
    let in_set: HashSet<&str> = ids.iter().map(String::as_str).collect();

    // Remaining in-set prerequisite count per id, and the reverse edges
    // (prerequisite -> the ids that depend on it) so we can relax on emit.
    let mut indegree: HashMap<&str, usize> = ids.iter().map(|id| (id.as_str(), 0usize)).collect();
    let mut dependents_of: HashMap<&str, Vec<&str>> = HashMap::new();
    for id in ids {
        for dep in direct(graph, id) {
            if in_set.contains(dep.as_str()) {
                *indegree.get_mut(id.as_str()).unwrap() += 1;
                dependents_of
                    .entry(dep.as_str())
                    .or_default()
                    .push(id.as_str());
            }
        }
    }

    let mut order: Vec<String> = Vec::with_capacity(ids.len());
    let mut emitted: HashSet<&str> = HashSet::new();
    // Kahn's: repeatedly emit the earliest-in-input id whose prerequisites are
    // all emitted. Re-scanning `ids` in order each round preserves the stable
    // tiebreak without a priority queue (mod counts are tiny).
    loop {
        let mut progressed = false;
        for id in ids {
            let id = id.as_str();
            if emitted.contains(id) || indegree[id] != 0 {
                continue;
            }
            emitted.insert(id);
            order.push(id.to_string());
            progressed = true;
            if let Some(deps) = dependents_of.get(id) {
                for &d in deps {
                    *indegree.get_mut(d).unwrap() -= 1;
                }
            }
        }
        if !progressed {
            break;
        }
    }

    // Anything left is in a cycle; emit in input order so the set is complete.
    let cycle = order.len() != ids.len();
    if cycle {
        for id in ids {
            if !emitted.contains(id.as_str()) {
                order.push(id.clone());
            }
        }
    }

    TopoOrder { order, cycle }
}

/// The ids among `enabled` that DIRECTLY depend on `id` (list it in their
/// dependencies). Used to BLOCK a disable: if this is non-empty, `id` cannot be
/// disabled without breaking those mods. Sorted for a deterministic message.
pub fn dependents<'a>(
    id: &str,
    enabled: impl IntoIterator<Item = &'a str>,
    graph: &DepGraph,
) -> Vec<String> {
    let mut out: Vec<String> = enabled
        .into_iter()
        .filter(|&e| e != id && direct(graph, e).iter().any(|d| d == id))
        .map(str::to_string)
        .collect();
    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph(edges: &[(&str, &[&str])]) -> DepGraph {
        edges
            .iter()
            .map(|(id, deps)| (id.to_string(), deps.iter().map(|d| d.to_string()).collect()))
            .collect()
    }

    #[test]
    fn transitive_deps_walks_a_chain_and_a_diamond() {
        // c -> b -> a ; and d -> {b, e}, e -> a (diamond on a).
        let g = graph(&[
            ("c", &["b"]),
            ("b", &["a"]),
            ("d", &["b", "e"]),
            ("e", &["a"]),
        ]);
        assert_eq!(transitive_deps(&g, "c"), vec!["a", "b"]);
        // Post-order, each dep once: b's subtree (a,b) then e's (e, a already seen).
        assert_eq!(transitive_deps(&g, "d"), vec!["a", "b", "e"]);
        assert_eq!(transitive_deps(&g, "a"), Vec::<String>::new());
    }

    #[test]
    fn transitive_deps_tolerates_a_cycle() {
        let g = graph(&[("a", &["b"]), ("b", &["a"])]);
        // a's deps: b (and b -> a is ignored as the root/seen).
        assert_eq!(transitive_deps(&g, "a"), vec!["b"]);
    }

    #[test]
    fn topological_order_puts_deps_before_dependents_regardless_of_input_order() {
        // Input order lists the dependent (b) BEFORE its dependency (a).
        let g = graph(&[("b", &["a"])]);
        let ids = vec!["b".to_string(), "a".to_string(), "c".to_string()];
        let topo = topological_order(&ids, &g);
        assert!(!topo.cycle);
        // a before b; c independent keeps its relative slot (stable tiebreak).
        let pos = |x: &str| topo.order.iter().position(|s| s == x).unwrap();
        assert!(
            pos("a") < pos("b"),
            "dep before dependent: {:?}",
            topo.order
        );
        // c had no edges; among no-prereq nodes the input order (b,a,c... but b
        // blocked) yields a, c, b or a, b, c depending on relaxation - assert the
        // only hard constraint plus determinism.
        assert_eq!(topo.order.len(), 3);
    }

    #[test]
    fn topological_order_is_stable_for_independent_ids() {
        let g = graph(&[]); // no edges
        let ids = vec!["x".to_string(), "y".to_string(), "z".to_string()];
        let topo = topological_order(&ids, &g);
        assert!(!topo.cycle);
        assert_eq!(topo.order, ids, "independent ids keep input order");
    }

    #[test]
    fn topological_order_flags_a_cycle_and_stays_complete() {
        let g = graph(&[("a", &["b"]), ("b", &["a"]), ("c", &[])]);
        let ids = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let topo = topological_order(&ids, &g);
        assert!(topo.cycle, "a<->b is a cycle");
        // c is orderable; a and b are emitted in input order after it.
        assert!(topo.order.contains(&"c".to_string()));
        assert_eq!(topo.order.len(), 3, "all ids present despite the cycle");
    }

    #[test]
    fn dependents_lists_enabled_mods_that_need_the_id() {
        // b and c depend on a; d does not.
        let g = graph(&[("b", &["a"]), ("c", &["a"]), ("d", &["e"])]);
        let enabled = ["a", "b", "c", "d"];
        assert_eq!(
            dependents("a", enabled.iter().copied(), &g),
            vec!["b".to_string(), "c".to_string()]
        );
        // d depends on e, so e has one enabled dependent.
        assert_eq!(
            dependents("e", enabled.iter().copied(), &g),
            vec!["d".to_string()]
        );
        // Nothing depends on d.
        assert!(dependents("d", enabled.iter().copied(), &g).is_empty());
        // A DISABLED dependent does not count (only `enabled` is scanned): with
        // only a and d enabled, nothing enabled depends on a.
        assert!(dependents("a", ["a", "d"].iter().copied(), &g).is_empty());
    }
}
