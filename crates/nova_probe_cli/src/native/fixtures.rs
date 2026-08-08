//! Shared test fixtures for the driver modules.

/// Owned argv for a CLI parse.
pub fn s(args: &[&str]) -> Vec<String> {
    args.iter().map(|a| a.to_string()).collect()
}

/// A stand-in example catalog covering every category the specs use.
pub fn catalog() -> Vec<crate::CatalogExample> {
    [
        ("controller_section", "sections"),
        ("scenario", "gameplay"),
        ("playable", "gameplay"),
        ("screenshot_scene", "screenshots"),
        ("render_scale_shot", "screenshots"),
        ("scene_baseline", "stress"),
        ("many_bodies", "stress"),
        ("outcomes", "systems"),
    ]
    .into_iter()
    .map(|(name, category)| crate::CatalogExample {
        name: name.into(),
        path: format!("examples/{category}/{name}.rs"),
        category: category.into(),
    })
    .collect()
}
