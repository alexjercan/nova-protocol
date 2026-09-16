//! What resolution owes its callers: the layers, their order, and the errors
//! a bad reference reports instead of silently doing nothing.

use nova_ship::prelude::{
    BaseSectionConfig, GameSections, HullSectionConfig, SectionConfigPatch, SectionKind,
    SectionKindPatch, ThrusterSectionConfig, ThrusterSectionConfigPatch,
};

use super::*;
use crate::objects::spaceship::prelude::SectionSource;

/// A catalog holding one hull prototype and one thruster prototype.
fn catalog() -> GameSections {
    GameSections(vec![
        SectionConfig {
            base: BaseSectionConfig {
                id: "plate".to_string(),
                health: 100.0,
                ..default()
            },
            kind: SectionKind::Hull(HullSectionConfig::default()),
        },
        SectionConfig {
            base: BaseSectionConfig {
                id: "engine".to_string(),
                health: 100.0,
                ..default()
            },
            kind: SectionKind::Thruster(ThrusterSectionConfig {
                magnitude: 10.0,
                ..default()
            }),
        },
    ])
}

/// A design mounting `sections`, registered under `id`.
fn designs(id: &str, sections: Vec<SpaceshipSectionConfig>) -> GameShipDesigns {
    GameShipDesigns(vec![ShipDesignPrototype {
        id: id.to_string(),
        name: "Test".to_string(),
        design: ShipDesign {
            sections,
            ..default()
        },
    }])
}

fn placed(id: &str, source: SectionSource) -> SpaceshipSectionConfig {
    SpaceshipSectionConfig {
        id: id.to_string(),
        position: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        source,
    }
}

/// The plain case: a prototype design of prototype sections, nothing patched,
/// resolves to the catalog's own values.
#[test]
fn a_prototype_design_resolves_its_sections_from_the_catalog() {
    let designs = designs(
        "corvette",
        vec![placed("nose", SectionSource::prototype("plate"))],
    );

    let (resolved, errors) = resolve_ship_design(
        &ShipDesignSource::prototype("corvette"),
        &designs,
        &catalog(),
    );

    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(resolved.sections.len(), 1);
    assert_eq!(resolved.sections[0].config.base.health, 100.0);
}

/// The ledger's shape, and the gap this refactor closes: a DESIGN tunes a
/// catalog section it mounts, without inlining the whole part. 22 of the
/// ledger's sections are exactly this.
#[test]
fn a_designs_own_section_patch_applies() {
    let designs = designs(
        "hauler",
        vec![placed(
            "engine_starboard",
            SectionSource::Prototype {
                id: "engine".to_string(),
                patch: SectionConfigPatch {
                    health: Some(90.0),
                    ..default()
                },
            },
        )],
    );

    let (resolved, errors) =
        resolve_ship_design(&ShipDesignSource::prototype("hauler"), &designs, &catalog());

    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(resolved.sections[0].config.base.health, 90.0);
}

/// Two layers, and the outer one wins - the precedence the spawn
/// modifications already had. The design says 90, this one spawn says 50.
#[test]
fn the_spawns_patch_wins_over_the_designs_own() {
    let designs = designs(
        "hauler",
        vec![placed(
            "engine_starboard",
            SectionSource::Prototype {
                id: "engine".to_string(),
                patch: SectionConfigPatch {
                    health: Some(90.0),
                    ..default()
                },
            },
        )],
    );
    let source = ShipDesignSource::Prototype {
        id: "hauler".to_string(),
        section_patches: [(
            "engine_starboard".to_string(),
            SpaceshipSectionConfigPatch {
                position: Some(Vec3::new(0.0, 0.0, 3.0)),
                config: SectionConfigPatch {
                    health: Some(50.0),
                    ..default()
                },
                ..default()
            },
        )]
        .into_iter()
        .collect(),
    };

    let (resolved, errors) = resolve_ship_design(&source, &designs, &catalog());

    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(resolved.sections[0].config.base.health, 50.0);
    assert_eq!(
        resolved.sections[0].position,
        Vec3::new(0.0, 0.0, 3.0),
        "position is patched on the placement, not inside the section config"
    );
}

/// A field the outer layer does not mention keeps the inner layer's value, so
/// the two layers compose instead of the outer one resetting the section.
#[test]
fn an_outer_patch_leaves_the_fields_it_does_not_mention() {
    let designs = designs(
        "hauler",
        vec![placed(
            "engine_starboard",
            SectionSource::Prototype {
                id: "engine".to_string(),
                patch: SectionConfigPatch {
                    kind: Some(SectionKindPatch::Thruster(ThrusterSectionConfigPatch {
                        magnitude: Some(25.0),
                    })),
                    ..default()
                },
            },
        )],
    );
    let source = ShipDesignSource::Prototype {
        id: "hauler".to_string(),
        section_patches: [(
            "engine_starboard".to_string(),
            SpaceshipSectionConfigPatch {
                config: SectionConfigPatch {
                    health: Some(50.0),
                    ..default()
                },
                ..default()
            },
        )]
        .into_iter()
        .collect(),
    };

    let (resolved, _) = resolve_ship_design(&source, &designs, &catalog());

    let SectionKind::Thruster(thruster) = &resolved.sections[0].config.kind else {
        panic!("the kind is preserved");
    };
    assert_eq!(thruster.magnitude, 25.0, "the design's patch survives");
    assert_eq!(resolved.sections[0].config.base.health, 50.0);
}

/// An inline design needs no catalog entry of its own, and an inline section
/// needs no section catalog - which is what keeps an example's two-section rig
/// spawnable with nothing loaded.
#[test]
fn an_inline_design_of_inline_sections_needs_no_catalog_at_all() {
    let source = ShipDesignSource::Inline(ShipDesign {
        sections: vec![placed(
            "nose",
            SectionSource::Inline(SectionConfig {
                base: BaseSectionConfig {
                    id: "bespoke".to_string(),
                    health: 42.0,
                    ..default()
                },
                kind: SectionKind::Hull(HullSectionConfig::default()),
            }),
        )],
        ..default()
    });

    let (resolved, errors) = resolve_ship_design(
        &source,
        &GameShipDesigns::default(),
        &GameSections::default(),
    );

    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(resolved.sections[0].config.base.health, 42.0);
}

/// A design nobody authored is an error and an empty root, not a panic and not
/// a load failure: a mod overlay that dropped a ship must not take the whole
/// scenario with it.
#[test]
fn an_unknown_design_reports_and_resolves_to_nothing() {
    let (resolved, errors) = resolve_ship_design(
        &ShipDesignSource::prototype("nothing"),
        &GameShipDesigns::default(),
        &catalog(),
    );

    assert!(resolved.sections.is_empty());
    assert_eq!(
        errors,
        vec![ShipDesignError::UnknownDesign("nothing".to_string())]
    );
}

/// One missing part does not ground the ship: the section is reported and
/// skipped, and everything else still spawns.
#[test]
fn an_unknown_section_prototype_is_skipped_and_the_rest_still_resolves() {
    let designs = designs(
        "corvette",
        vec![
            placed("nose", SectionSource::prototype("plate")),
            placed("ghost", SectionSource::prototype("no_such_part")),
        ],
    );

    let (resolved, errors) = resolve_ship_design(
        &ShipDesignSource::prototype("corvette"),
        &designs,
        &catalog(),
    );

    assert_eq!(resolved.sections.len(), 1);
    assert_eq!(resolved.sections[0].id, "nose");
    assert_eq!(
        errors,
        vec![ShipDesignError::UnknownSectionPrototype {
            section: "ghost".to_string(),
            prototype: "no_such_part".to_string(),
        }]
    );
}

/// A patch aimed at a section the design does not carry changes nothing at
/// all, which is precisely why it has to be reported: the author's intent went
/// nowhere and nothing else would ever say so.
#[test]
fn a_patch_naming_no_section_is_an_error() {
    let designs = designs(
        "corvette",
        vec![placed("nose", SectionSource::prototype("plate"))],
    );
    let source = ShipDesignSource::Prototype {
        id: "corvette".to_string(),
        section_patches: [(
            "noze".to_string(),
            SpaceshipSectionConfigPatch {
                config: SectionConfigPatch {
                    health: Some(10.0),
                    ..default()
                },
                ..default()
            },
        )]
        .into_iter()
        .collect(),
    };

    let (_, errors) = resolve_ship_design(&source, &designs, &catalog());

    assert_eq!(
        errors,
        vec![ShipDesignError::UnknownPatchTarget("noze".to_string())]
    );
}

/// A kind mismatch is reported against the SECTION, so the lint can name the
/// place in the design rather than just the shape of the patch.
#[test]
fn a_kind_mismatch_is_reported_against_its_section() {
    let designs = designs(
        "corvette",
        vec![placed(
            "nose",
            SectionSource::Prototype {
                id: "plate".to_string(),
                patch: SectionConfigPatch {
                    kind: Some(SectionKindPatch::Thruster(ThrusterSectionConfigPatch {
                        magnitude: Some(1.0),
                    })),
                    ..default()
                },
            },
        )],
    );

    let (resolved, errors) = resolve_ship_design(
        &ShipDesignSource::prototype("corvette"),
        &designs,
        &catalog(),
    );

    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(matches!(
        errors[0],
        ShipDesignError::Patch {
            ref section,
            error: SectionPatchError::KindMismatch { .. }
        } if section == "nose"
    ));
    assert_eq!(
        resolved.sections.len(),
        1,
        "the section still spawns, unpatched, rather than vanishing"
    );
}

/// Where a section SITS is patched on the placement, not inside the section
/// config - so a spawn can move and turn one part of a shared design without
/// inlining the part or the design.
#[test]
fn a_spawn_patch_moves_and_turns_the_placement_it_names() {
    let turned = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
    let designs = designs(
        "hauler",
        vec![
            SpaceshipSectionConfig {
                position: Vec3::new(0.0, 0.0, 1.0),
                rotation: Quat::IDENTITY,
                ..placed("engine_port", SectionSource::prototype("engine"))
            },
            SpaceshipSectionConfig {
                position: Vec3::new(0.0, 0.0, 2.0),
                rotation: Quat::IDENTITY,
                ..placed("engine_starboard", SectionSource::prototype("engine"))
            },
        ],
    );
    let source = ShipDesignSource::Prototype {
        id: "hauler".to_string(),
        section_patches: [(
            "engine_port".to_string(),
            SpaceshipSectionConfigPatch {
                position: Some(Vec3::new(4.0, 0.0, 0.0)),
                rotation: Some(turned),
                ..default()
            },
        )]
        .into_iter()
        .collect(),
    };

    let (resolved, errors) = resolve_ship_design(&source, &designs, &catalog());

    assert!(errors.is_empty(), "{errors:?}");
    let port = &resolved.sections[0];
    assert_eq!(port.position, Vec3::new(4.0, 0.0, 0.0));
    assert_eq!(port.rotation, turned);
    let starboard = &resolved.sections[1];
    assert_eq!(
        (starboard.position, starboard.rotation),
        (Vec3::new(0.0, 0.0, 2.0), Quat::IDENTITY),
        "a placement the patch does not name keeps the design's own"
    );
}

/// The base voice names every cue, and every path it names is a file the
/// base content ships: a typo here is silence at run time, logged once by
/// the asset server and noticed by nobody.
#[test]
fn the_base_voice_names_every_cue_at_a_file_that_exists() {
    let voice = ShipPresentationConfig::base_voice();
    let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    for (cue, sound) in [
        ("collapse", &voice.collapse_sound),
        ("lock on", &voice.lock_on_sound),
        ("lock off", &voice.lock_off_sound),
        ("radar deny", &voice.radar_deny_sound),
        ("radar retarget", &voice.radar_retarget_sound),
        ("safety on", &voice.safety_on_sound),
        ("warn lock", &voice.warn_lock_sound),
        ("ammo dry", &voice.ammo_dry_sound),
        ("warn hull", &voice.warn_hull_sound),
        ("rcs loop", &voice.rcs_loop_sound),
    ] {
        let path = sound
            .as_ref()
            .and_then(AssetRef::path)
            .unwrap_or_else(|| panic!("the base voice leaves {cue} silent"));
        assert!(
            assets.join(path).is_file(),
            "{cue}: no file at assets/{path}"
        );
    }
    assert!(!voice.skin, "the voice is not a look");
    assert_eq!(voice.style, None);
}
