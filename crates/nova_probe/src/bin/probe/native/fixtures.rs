//! Shared test fixtures for the driver modules.

/// Owned argv for a CLI parse.
pub fn s(args: &[&str]) -> Vec<String> {
    args.iter().map(|a| a.to_string()).collect()
}

/// A stand-in example catalog covering every category the specs use.
pub fn catalog() -> Vec<nova_probe::CatalogExample> {
    [
        ("controller_section", "sections"),
        ("scenario", "gameplay"),
        ("playable", "gameplay"),
        ("screenshot_reel", "screenshots"),
        ("render_scale_shot", "screenshots"),
        ("perf_baseline", "perf"),
    ]
    .into_iter()
    .map(|(name, category)| nova_probe::CatalogExample {
        name: name.into(),
        path: format!("examples/{category}/{name}.rs"),
        category: category.into(),
    })
    .collect()
}
