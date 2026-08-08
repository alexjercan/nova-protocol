//! Scenario fixture builders shared by the examples.
//!
//! Three examples were building the same two config shapes inline - a
//! [`SpaceshipConfig`] assembled out of [`GameSections`] lookups, and an
//! asteroid [`ScenarioObjectConfig`] with a health knob - before `stress/`
//! became the third caller and paid for the extraction.
//!
//! They live in `nova_probe` rather than under `examples/` because
//! `tests/catalog_drift.rs::catalog_matches_disk` scans every `.rs` directly
//! under `examples/*/` and demands a catalog block for each, so a shared
//! `examples/support/` module could only exist by hiding from that scan.
//! `nova_probe` already depends on the game stack and is already an
//! unconditional dev-dependency of the root package, so every example can reach
//! it for free.
//!
//! Deliberately small: this is the union the three callers actually share, not
//! a scenario DSL. The four other inline `SpaceshipConfig` builders in
//! `examples/` stay inline until a shape asks to join.

use bevy::prelude::*;
use nova_protocol::prelude::*;

/// One section slot on a ship built by [`ship`].
///
/// `asset` is a [`GameSections`] content id (e.g. `"basic_controller_section"`),
/// `id` the per-ship instance id the input mapping and the integrity graph bind
/// to. Rotation is not a field: every caller places sections axis-aligned, so
/// the builder pins [`Quat::IDENTITY`] rather than carrying a knob nobody turns.
pub struct SectionSpec {
    /// Per-ship instance id.
    pub id: String,
    /// Content id looked up in [`GameSections`].
    pub asset: String,
    /// Offset from the ship origin.
    pub position: Vec3,
}

impl SectionSpec {
    /// A slot at `position` filled by the `asset` section, named `id`.
    pub fn new(id: impl Into<String>, asset: impl Into<String>, position: Vec3) -> Self {
        Self {
            id: id.into(),
            asset: asset.into(),
            position,
        }
    }
}

/// A ship under `controller`, with one inline section per spec, in order.
///
/// Panics on an unknown content id: an example that silently flew a ship
/// missing its drive would fail somewhere far from the typo.
pub fn ship(
    sections: &GameSections,
    controller: SpaceshipController,
    specs: &[SectionSpec],
) -> SpaceshipConfig {
    SpaceshipConfig {
        allegiance: None,
        controller,
        sections: specs
            .iter()
            .map(|spec| SpaceshipSectionConfig {
                id: spec.id.clone(),
                position: spec.position,
                rotation: Quat::IDENTITY,
                source: SectionSource::Inline(
                    sections
                        .get_section(&spec.asset)
                        .unwrap_or_else(|| panic!("section '{}' not found", spec.asset))
                        .clone(),
                ),
                modifications: vec![],
            })
            .collect(),
    }
}

/// An asteroid target.
///
/// `health` is the knob that decides whether it survives being hit: a gate a
/// blast destroys wants a low number, a target that must outlive several
/// torpedoes wants a high one. `lock_signature` is `Some` only where the run
/// needs the rock to be radar-lockable - an unsigned rock is invisible to the
/// radar, which is exactly what most fixtures want.
pub fn asteroid(
    game_assets: &GameAssets,
    id: &str,
    name: &str,
    position: Vec3,
    radius: f32,
    health: f32,
    lock_signature: Option<f32>,
) -> ScenarioObjectConfig {
    ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
            impact_sound: Some("base/sounds/impact.wav".into()),
            destroy_sound: Some("base/sounds/explosion.wav".into()),
            radius,
            texture: game_assets.asteroid_texture.clone().into(),
            health,
            mass: None,
            invulnerable: false,
            lock_signature,
        }),
    }
}

/// The whole event list for a scenario that only has to put `objects` in the
/// world: one unfiltered `OnStart` spawning each in order.
pub fn spawn_on_start(objects: Vec<ScenarioObjectConfig>) -> Vec<ScenarioEventConfig> {
    vec![ScenarioEventConfig {
        name: EventConfig::OnStart,
        filters: vec![],
        actions: objects
            .into_iter()
            .map(EventActionConfig::SpawnScenarioObject)
            .collect(),
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A catalog of empty hull sections under the given content ids.
    fn catalog(ids: &[&str]) -> GameSections {
        GameSections(
            ids.iter()
                .map(|id| SectionConfig {
                    base: BaseSectionConfig {
                        id: id.to_string(),
                        ..default()
                    },
                    kind: SectionKind::Hull(HullSectionConfig::default()),
                })
                .collect(),
        )
    }

    #[test]
    fn ship_carries_its_sections_in_order() {
        let sections = catalog(&["hull_a", "hull_b"]);
        let specs = [
            SectionSpec::new("front", "hull_a", Vec3::new(0.0, 0.0, -1.0)),
            SectionSpec::new("back", "hull_b", Vec3::new(0.0, 0.0, 1.0)),
            SectionSpec::new("spare", "hull_a", Vec3::ZERO),
        ];

        let built = ship(&sections, SpaceshipController::None, &specs);

        let placed: Vec<_> = built
            .sections
            .iter()
            .map(|s| (s.id.as_str(), s.position))
            .collect();
        assert_eq!(
            placed,
            vec![
                ("front", Vec3::new(0.0, 0.0, -1.0)),
                ("back", Vec3::new(0.0, 0.0, 1.0)),
                ("spare", Vec3::ZERO),
            ]
        );
        // The slot id is the ship-local one; the content id it resolved is the
        // asset. Confusing the two is the failure this pins.
        let SectionSource::Inline(front) = &built.sections[0].source else {
            panic!("sections are inlined, not referenced");
        };
        assert_eq!(front.base.id, "hull_a");
    }

    #[test]
    #[should_panic(expected = "section 'nope' not found")]
    fn ship_panics_on_an_unknown_content_id() {
        ship(
            &catalog(&["hull_a"]),
            SpaceshipController::None,
            &[SectionSpec::new("front", "nope", Vec3::ZERO)],
        );
    }

    /// A marker object. Beacons, not asteroids: `GameAssets` is an
    /// `AssetCollection` with no `Default`, and `asteroid` only forwards its
    /// arguments into a struct literal - the behavior worth pinning here is
    /// `spawn_on_start`'s wrapping, which is asset-free.
    fn beacon(id: &str) -> ScenarioObjectConfig {
        ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: id.to_string(),
                name: id.to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Beacon(BeaconConfig {
                label: id.to_string(),
                radius: 1.0,
                color: Color::WHITE,
                area_radius: None,
                lock_signature: None,
            }),
        }
    }

    #[test]
    fn spawn_on_start_wraps_every_object_in_exactly_one_event() {
        let events = spawn_on_start(vec![beacon("a"), beacon("b")]);

        assert_eq!(events.len(), 1);
        assert!(matches!(events[0].name, EventConfig::OnStart));
        assert!(events[0].filters.is_empty());
        assert_eq!(events[0].actions.len(), 2);
        assert!(events[0]
            .actions
            .iter()
            .all(|a| matches!(a, EventActionConfig::SpawnScenarioObject(_))));
    }
}
