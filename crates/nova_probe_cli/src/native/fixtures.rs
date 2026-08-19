//! Shared test fixtures for the driver modules.

/// Owned argv for a CLI parse.
pub fn s(args: &[&str]) -> Vec<String> {
    args.iter().map(|a| a.to_string()).collect()
}

/// A stand-in example catalog covering every category the specs use.
///
/// Every name here is INVENTED. Spec resolution is pure - the catalog is an
/// argument - so a real example name in this table would read as a dependency
/// on the repo's examples, which is exactly what the driver must not have.
pub fn catalog() -> Vec<crate::CatalogExample> {
    [
        ("controller_section", "sections"),
        ("scenario", "gameplay"),
        ("playable", "gameplay"),
        ("shot_alpha", "screenshots"),
        ("shot_beta", "screenshots"),
        ("many_things", "stress"),
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
