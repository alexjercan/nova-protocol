//! Pure mod-dependency resolution over an `id -> dependency-ids` graph.
//! Engine-free like the rest of this crate, so the asset merge
//! (`register_bundles`), the menu's enable/install flows and their tests all
//! share ONE implementation. Ids only - no version constraints.
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

/// The deepest dependency chain [`transitive_deps`] will walk.
///
/// The walk recurses once per level, and the graph is built from UNTRUSTED
/// input (a portal `catalog.json`, a hand-edited bundle), so an unbounded
/// chain is a stack overflow - which aborts the process and cannot be caught.
/// 64 is far past any real mod stack and far short of the stack limit.
pub const MAX_DEP_DEPTH: usize = 64;

/// Why a dependency walk refused to finish. Refusing is the answer for
/// untrusted input: a truncated dependency list would read as complete.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DepError {
    /// The chain reached [`MAX_DEP_DEPTH`] at this id without terminating.
    DepthExceeded {
        /// The id the walk was standing on when it gave up.
        at: String,
    },
}

impl std::fmt::Display for DepError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DepthExceeded { at } => {
                write!(f, "dependency chain deeper than {MAX_DEP_DEPTH} at '{at}'",)
            }
        }
    }
}

/// Every TRANSITIVE dependency of `id` (NOT including `id` itself), in DFS
/// post-order so a dependency always appears before the mods that need it.
/// Cycle-tolerant: each id is visited once. Unknown ids contribute nothing.
/// Chains deeper than [`MAX_DEP_DEPTH`] are refused, never truncated.
pub fn transitive_deps(graph: &DepGraph, id: &str) -> Result<Vec<String>, DepError> {
    fn visit(
        graph: &DepGraph,
        id: &str,
        depth: usize,
        seen: &mut HashSet<String>,
        out: &mut Vec<String>,
    ) -> Result<(), DepError> {
        if depth >= MAX_DEP_DEPTH {
            return Err(DepError::DepthExceeded { at: id.to_string() });
        }
        for dep in direct(graph, id) {
            if seen.insert(dep.clone()) {
                visit(graph, dep, depth + 1, seen, out)?;
                out.push(dep.clone());
            }
        }
        Ok(())
    }
    let mut seen = HashSet::new();
    // Seeding with `id` is what makes a self-edge (or a cycle back to the
    // root) ignored and keeps the root out of its own list.
    seen.insert(id.to_string());
    let mut out = Vec::new();
    visit(graph, id, 0, &mut seen, &mut out)?;
    Ok(out)
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
    // A repeated id is emitted once, so `order.len() != ids.len()` would report
    // a cycle for a duplicate-carrying set with no dependencies at all. Dedup
    // first, keeping input order, and the length compare means what it says.
    let mut deduped: Vec<String> = Vec::with_capacity(ids.len());
    let mut distinct: HashSet<&str> = HashSet::new();
    for id in ids {
        if distinct.insert(id.as_str()) {
            deduped.push(id.clone());
        }
    }
    let ids: &[String] = &deduped;

    let in_set: HashSet<&str> = ids.iter().map(String::as_str).collect();

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
    // Re-scanning `ids` in input order each round is what preserves the
    // stable tiebreak without a priority queue (mod counts are tiny).
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
        let g = graph(&[
            ("c", &["b"]),
            ("b", &["a"]),
            ("d", &["b", "e"]),
            ("e", &["a"]),
        ]);
        assert_eq!(transitive_deps(&g, "c").unwrap(), vec!["a", "b"]);
        // Post-order, each dep once - b's subtree (a, b) then e's (e, with
        // a already seen).
        assert_eq!(transitive_deps(&g, "d").unwrap(), vec!["a", "b", "e"]);
        assert_eq!(transitive_deps(&g, "a").unwrap(), Vec::<String>::new());
    }

    #[test]
    fn transitive_deps_tolerates_a_cycle() {
        let g = graph(&[("a", &["b"]), ("b", &["a"])]);
        // B only - the `b -> a` edge back to the root is ignored.
        assert_eq!(transitive_deps(&g, "a").unwrap(), vec!["b"]);
    }

    #[test]
    fn topological_order_puts_deps_before_dependents_regardless_of_input_order() {
        let g = graph(&[("b", &["a"])]);
        let ids = vec!["b".to_string(), "a".to_string(), "c".to_string()];
        let topo = topological_order(&ids, &g);
        assert!(!topo.cycle);
        let pos = |x: &str| topo.order.iter().position(|s| s == x).unwrap();
        assert!(
            pos("a") < pos("b"),
            "dep before dependent: {:?}",
            topo.order
        );
        // With b blocked in round one, c may land before or after it
        // depending on relaxation, so only the hard constraint is asserted.
        assert_eq!(topo.order.len(), 3);
    }

    #[test]
    fn topological_order_is_stable_for_independent_ids() {
        let g = graph(&[]);
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
        assert!(topo.order.contains(&"c".to_string()));
        assert_eq!(topo.order.len(), 3, "all ids present despite the cycle");
    }

    /// F08: an untrusted catalog can declare an arbitrarily long chain, and
    /// the walk recurses once per level. Refuse past `MAX_DEP_DEPTH` rather
    /// than overflowing the stack - an overflow ABORTS the process. Remove the
    /// depth argument and this test blows the test thread's stack.
    #[test]
    fn a_chain_deeper_than_the_cap_is_refused_not_walked() {
        let chain: Vec<(String, Vec<String>)> = (0..MAX_DEP_DEPTH * 4)
            .map(|i| (format!("m{i}"), vec![format!("m{}", i + 1)]))
            .collect();
        let g: DepGraph = chain.into_iter().collect();
        assert!(
            matches!(
                transitive_deps(&g, "m0"),
                Err(DepError::DepthExceeded { .. })
            ),
            "an over-deep chain is refused"
        );

        // The cap does not fire on a chain that fits: MAX_DEP_DEPTH levels of
        // recursion below the root.
        let ok: DepGraph = (0..MAX_DEP_DEPTH - 1)
            .map(|i| (format!("m{i}"), vec![format!("m{}", i + 1)]))
            .collect();
        assert_eq!(
            transitive_deps(&ok, "m0").unwrap().len(),
            MAX_DEP_DEPTH - 1,
            "a chain within the cap still resolves in full"
        );
    }

    /// F60: two records with the same id used to make `order.len() != ids.len()`
    /// true, reporting "a dependency cycle" for a set with zero dependencies.
    #[test]
    fn duplicate_ids_are_not_a_cycle() {
        let g = graph(&[]);
        let ids = vec!["a".to_string(), "b".to_string(), "a".to_string()];
        let topo = topological_order(&ids, &g);
        assert!(!topo.cycle, "duplicates are not a cycle");
        assert_eq!(topo.order, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn dependents_lists_enabled_mods_that_need_the_id() {
        let g = graph(&[("b", &["a"]), ("c", &["a"]), ("d", &["e"])]);
        let enabled = ["a", "b", "c", "d"];
        assert_eq!(
            dependents("a", enabled.iter().copied(), &g),
            vec!["b".to_string(), "c".to_string()]
        );
        assert_eq!(
            dependents("e", enabled.iter().copied(), &g),
            vec!["d".to_string()]
        );
        assert!(dependents("d", enabled.iter().copied(), &g).is_empty());
        // A DISABLED dependent does not count - only `enabled` is scanned.
        assert!(dependents("a", ["a", "d"].iter().copied(), &g).is_empty());
    }
}

/// `DepGraph` with `transitive_deps` and `dependents`, `topological_order` and its
/// `TopoOrder`, plus `DepError` and `MAX_DEP_DEPTH`.
pub mod prelude {
    pub use super::{
        dependents, topological_order, transitive_deps, DepError, DepGraph, TopoOrder,
        MAX_DEP_DEPTH,
    };
}
