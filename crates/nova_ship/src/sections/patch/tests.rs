//! What a patch is allowed to move, and what it must refuse.

use super::*;

/// A turret whose barrels carry the given muzzle ids, hung off one fixed root
/// so the walk has to descend to find them.
fn turret_with_muzzles(ids: &[&str]) -> SectionConfig {
    let children = ids
        .iter()
        .map(|id| TurretJoint {
            name: None,
            offset: Vec3::ZERO,
            axis: None,
            speed: 1.0,
            min: None,
            max: None,
            render_mesh: None,
            render_mesh_transform: None,
            muzzle: Some(MuzzleConfig {
                id: (*id).to_string(),
                fire_rate: 10.0,
                muzzle_effect: None,
            }),
            children: vec![],
        })
        .collect();
    SectionConfig {
        base: BaseSectionConfig {
            id: "gun".to_string(),
            health: 100.0,
            ..default()
        },
        kind: SectionKind::Turret(TurretSectionConfig {
            root: TurretJoint {
                name: None,
                offset: Vec3::ZERO,
                axis: None,
                speed: 1.0,
                min: None,
                max: None,
                render_mesh: None,
                render_mesh_transform: None,
                muzzle: None,
                children,
            },
            ..default()
        }),
    }
}

fn hull(health: f32) -> SectionConfig {
    SectionConfig {
        base: BaseSectionConfig {
            id: "plate".to_string(),
            health,
            ..default()
        },
        kind: SectionKind::Hull(HullSectionConfig::default()),
    }
}

fn muzzle_rate(config: &SectionConfig, id: &str) -> f32 {
    let SectionKind::Turret(turret) = &config.kind else {
        panic!("not a turret");
    };
    let mut found = None;
    fn walk<'a>(joint: &'a TurretJoint, id: &str, found: &mut Option<&'a MuzzleConfig>) {
        if let Some(muzzle) = &joint.muzzle {
            if muzzle.id == id {
                *found = Some(muzzle);
            }
        }
        for child in &joint.children {
            walk(child, id, found);
        }
    }
    walk(&turret.root, id, &mut found);
    found.expect("muzzle id is in the tree").fire_rate
}

/// The value an omitted patch takes. Authored content leaves the field out,
/// the loader fills in the default, and resolution must treat the two the
/// same - otherwise every prototype reference in the shipped catalogs would
/// have to be edited to say "change nothing".
#[test]
fn an_empty_patch_leaves_the_prototype_exactly_as_it_was() {
    let mut config = hull(80.0);
    let before = format!("{config:?}");

    SectionConfigPatch::default().apply(&mut config).unwrap();

    assert!(SectionConfigPatch::default().is_empty());
    assert_eq!(format!("{config:?}"), before);
}

/// The one field every section kind shares, and the case the shipped ledger
/// ships 22 of: a design mounts a catalog section and trims its health.
#[test]
fn health_replaces_the_prototypes_value() {
    let mut config = hull(100.0);

    SectionConfigPatch {
        health: Some(90.0),
        ..default()
    }
    .apply(&mut config)
    .unwrap();

    assert_eq!(config.base.health, 90.0);
}

/// Omitted slots INHERIT. A patch that speaks to one field must not quietly
/// reset the rest of the kind config to its `Default`, which is the failure a
/// whole-struct override would have.
#[test]
fn an_omitted_slot_inherits_instead_of_resetting() {
    let mut config = SectionConfig {
        base: BaseSectionConfig {
            id: "gun".to_string(),
            health: 100.0,
            ..default()
        },
        kind: SectionKind::Turret(TurretSectionConfig {
            bullet_damage: 7.0,
            projectile_lifetime: 3.0,
            ..default()
        }),
    };

    SectionConfigPatch {
        kind: Some(SectionKindPatch::Turret(TurretSectionConfigPatch {
            bullet_damage: Some(12.0),
            ..default()
        })),
        ..default()
    }
    .apply(&mut config)
    .unwrap();

    let SectionKind::Turret(turret) = &config.kind else {
        panic!("the patch must not change the kind");
    };
    assert_eq!(turret.bullet_damage, 12.0);
    assert_eq!(
        turret.projectile_lifetime, 3.0,
        "an omitted slot inherits the prototype's value"
    );
    assert_eq!(config.base.health, 100.0);
}

/// A nullable field needs three answers, not two: inherit, set, and CLEAR.
/// `Some(None)` is the clear, and without it a raking railgun prototype could
/// never be mounted as a clean-bore gun.
#[test]
fn some_none_clears_a_nullable_field() {
    let mut config = SectionConfig {
        base: BaseSectionConfig {
            id: "lance".to_string(),
            ..default()
        },
        kind: SectionKind::Railgun(RailgunSectionConfig {
            rake_radius: Some(Meters(4.0)),
            ..default()
        }),
    };

    SectionConfigPatch {
        kind: Some(SectionKindPatch::Railgun(RailgunSectionConfigPatch {
            rake_radius: Some(None),
            ..default()
        })),
        ..default()
    }
    .apply(&mut config)
    .unwrap();

    let SectionKind::Railgun(railgun) = &config.kind else {
        panic!("the patch must not change the kind");
    };
    assert_eq!(railgun.rake_radius, None);
}

/// A patch cannot change what a section IS. The editor picks the patch struct
/// from the resolved kind, so this only fires on hand-authored content - which
/// is exactly where a silent no-op would be the expensive failure.
#[test]
fn a_patch_for_the_wrong_kind_is_an_error() {
    let mut config = hull(100.0);

    let error = SectionConfigPatch {
        kind: Some(SectionKindPatch::Thruster(ThrusterSectionConfigPatch {
            magnitude: Some(5.0),
        })),
        ..default()
    }
    .apply(&mut config)
    .unwrap_err();

    assert_eq!(
        error,
        SectionPatchError::KindMismatch {
            resolved: SectionClass::Hull,
            patch: SectionClass::Thruster,
        }
    );
}

/// Muzzles are addressed by NAME, not by position in the joint tree: this is
/// the whole reason `MuzzleConfig::id` exists. A twin's two rates must stay
/// with their own barrels.
#[test]
fn a_muzzle_patch_lands_on_the_muzzle_it_names() {
    let mut config = turret_with_muzzles(&["left", "right"]);

    SectionConfigPatch {
        kind: Some(SectionKindPatch::Turret(TurretSectionConfigPatch {
            muzzles: [(
                "right".to_string(),
                MuzzleConfigPatch {
                    fire_rate: Some(25.0),
                },
            )]
            .into_iter()
            .collect(),
            ..default()
        })),
        ..default()
    }
    .apply(&mut config)
    .unwrap();

    assert_eq!(muzzle_rate(&config, "right"), 25.0);
    assert_eq!(
        muzzle_rate(&config, "left"),
        10.0,
        "an unnamed muzzle keeps the prototype's rate"
    );
}

/// A typo in a muzzle id must not silently do nothing. The content lint reads
/// the same answer, so the error arrives when the mod loads rather than when a
/// player notices the gun fires at the wrong rate.
#[test]
fn an_unknown_muzzle_id_is_an_error() {
    let mut config = turret_with_muzzles(&["main"]);

    let error = SectionConfigPatch {
        kind: Some(SectionKindPatch::Turret(TurretSectionConfigPatch {
            muzzles: [("mian".to_string(), MuzzleConfigPatch::default())]
                .into_iter()
                .collect(),
            ..default()
        })),
        ..default()
    }
    .apply(&mut config)
    .unwrap_err();

    assert_eq!(error, SectionPatchError::UnknownMuzzle("mian".to_string()));
}

/// Two barrels sharing an id leave a patch with no single right target, so the
/// whole apply fails instead of picking whichever the walk reached first.
#[test]
fn a_duplicate_muzzle_id_is_an_error_even_when_the_patch_names_another() {
    let mut config = turret_with_muzzles(&["twin", "twin", "spotter"]);

    let error = SectionConfigPatch {
        kind: Some(SectionKindPatch::Turret(TurretSectionConfigPatch {
            muzzles: [("spotter".to_string(), MuzzleConfigPatch::default())]
                .into_iter()
                .collect(),
            ..default()
        })),
        ..default()
    }
    .apply(&mut config)
    .unwrap_err();

    assert_eq!(
        error,
        SectionPatchError::DuplicateMuzzle("twin".to_string())
    );
    assert_eq!(
        duplicate_muzzle_id(&turret_with_muzzles(&["a", "b"]).turret_root().clone()),
        None
    );
}

/// The editor labels its muzzle rows from this, so the order has to be the
/// tree's and every barrel has to appear.
#[test]
fn muzzle_ids_lists_every_barrel_in_tree_order() {
    let config = turret_with_muzzles(&["left", "right", "coax"]);

    assert_eq!(
        muzzle_ids(config.turret_root()),
        vec!["left", "right", "coax"]
    );
}

/// Test-only reach into the turret tree, so the assertions above read as
/// behavior statements rather than as three lines of unwrapping each.
trait TurretRoot {
    fn turret_root(&self) -> &TurretJoint;
}

impl TurretRoot for SectionConfig {
    fn turret_root(&self) -> &TurretJoint {
        let SectionKind::Turret(turret) = &self.kind else {
            panic!("not a turret");
        };
        &turret.root
    }
}

/// The inverse of apply, and the answer the editor writes back. A section
/// nobody touched keeps a reference with NO patch on it, so a later prototype
/// change still reaches every one of its fields.
#[test]
fn between_two_identical_configs_is_the_all_inherit_patch() {
    let prototype = turret_with_muzzles(&["left", "right"]);

    let patch = SectionConfigPatch::between(&prototype, &prototype.clone())
        .expect("nothing moved, so nothing is outside the boundary");

    assert!(patch.is_empty(), "no patch field at all: {patch:?}");
}

/// What one edit produces: the field that moved, and only that field.
#[test]
fn between_holds_the_field_that_moved_and_no_other() {
    let prototype = hull(100.0);
    let mut edited = prototype.clone();
    edited.base.health = 60.0;

    let patch = SectionConfigPatch::between(&prototype, &edited).expect("health is patchable");

    assert_eq!(patch.health, Some(60.0));
    assert_eq!(patch.kind, None, "a hull has no kind delta to write");
    let mut rebuilt = prototype.clone();
    patch.apply(&mut rebuilt).unwrap();
    assert_eq!(
        rebuilt, edited,
        "and the patch rebuilds exactly what it read"
    );
}

/// A twin PDC is the case the ids were added for: the edited barrel is named,
/// and the other one keeps inheriting.
#[test]
fn a_twin_writes_the_barrel_it_names_by_id() {
    let prototype = turret_with_muzzles(&["left", "right"]);
    let mut edited = prototype.clone();
    let SectionKind::Turret(turret) = &mut edited.kind else {
        panic!("the fixture is a turret");
    };
    turret.root.children[0].muzzle.as_mut().unwrap().fire_rate = 50.0;

    let patch = SectionConfigPatch::between(&prototype, &edited).expect("a rate is patchable");

    let Some(SectionKindPatch::Turret(turret)) = &patch.kind else {
        panic!("a turret patch: {patch:?}");
    };
    assert_eq!(
        turret.muzzles.keys().collect::<Vec<_>>(),
        vec!["left"],
        "by ID, and only the one that moved: {:?}",
        turret.muzzles
    );
    assert_eq!(turret.muzzles["left"].fire_rate, Some(50.0));
    let mut rebuilt = prototype.clone();
    patch.apply(&mut rebuilt).unwrap();
    assert_eq!(muzzle_rate(&rebuilt, "left"), 50.0);
    assert_eq!(muzzle_rate(&rebuilt, "right"), 10.0, "the twin inherits");
}

/// The boundary, checked rather than assumed. A joint that MOVED is not a
/// gameplay value, so the reference cannot carry it and the caller has to keep
/// the whole config instead.
#[test]
fn a_change_outside_the_boundary_cannot_be_said_as_a_patch() {
    let prototype = turret_with_muzzles(&["main"]);
    let mut moved = prototype.clone();
    let SectionKind::Turret(turret) = &mut moved.kind else {
        panic!("the fixture is a turret");
    };
    turret.root.children[0].offset = Vec3::new(0.0, 1.0, 0.0);

    assert_eq!(SectionConfigPatch::between(&prototype, &moved), None);

    let mut renamed = prototype.clone();
    renamed.base.name = "Point Defense".to_string();
    assert_eq!(
        SectionConfigPatch::between(&prototype, &renamed),
        None,
        "a name is the prototype's too"
    );

    let mut retyped = prototype.clone();
    retyped.kind = SectionKind::Hull(HullSectionConfig::default());
    assert_eq!(
        SectionConfigPatch::between(&prototype, &retyped),
        None,
        "and a patch can never change what a section IS"
    );
}

/// A muzzle the prototype has not got cannot be patched onto it - the id is a
/// handle on the prototype's own tree, not a way to grow one.
#[test]
fn a_muzzle_the_prototype_does_not_carry_is_outside_the_boundary() {
    let prototype = turret_with_muzzles(&["main"]);
    let grown = turret_with_muzzles(&["main", "spare"]);

    assert_eq!(SectionConfigPatch::between(&prototype, &grown), None);
}
