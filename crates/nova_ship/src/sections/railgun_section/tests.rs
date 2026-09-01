//! The lance's contract: the charge is a committed clock, the shot is one
//! shell down the hull's own line, and the recoil goes back through the hull
//! at the muzzle.

use nova_gameplay::test_support::{settle, unfinished_integrity_physics_app};

use super::*;

/// A physics app running only the lance's cycle. Real avian, because the
/// recoil is applied through `Forces` and a hand-built rig cannot stand in
/// for a mass and an inertia tensor.
fn railgun_app() -> App {
    let mut app = unfinished_integrity_physics_app();
    app.add_systems(FixedUpdate, charge_and_fire_railgun);
    app.add_observer(insert_railgun_section);
    app.finish();
    app
}

/// A ship with one lance, mounted `offset` from the root in the ship's frame.
///
/// The hull block is deliberately at the origin and the gun is placed off it,
/// so a caller can put the bore on the centre of mass or well off it and get
/// the two different recoil answers the mechanic is about.
fn spawn_lance_ship(app: &mut App, config: RailgunSectionConfig, offset: Vec3) -> (Entity, Entity) {
    let ship = app
        .world_mut()
        .spawn((
            Name::new("ship"),
            RigidBody::Dynamic,
            Transform::default(),
            SpaceshipRootMarker,
        ))
        .id();
    app.world_mut().spawn((
        ChildOf(ship),
        Name::new("hull"),
        Transform::default(),
        Collider::cuboid(1.0, 1.0, 1.0),
        ColliderDensity(1.0),
    ));
    let lance = app
        .world_mut()
        .spawn((
            ChildOf(ship),
            Name::new("lance"),
            Transform::from_translation(offset),
            Collider::cuboid(1.0, 1.0, 3.0),
            ColliderDensity(1.0),
            railgun_section(config),
        ))
        .id();
    settle(app);
    (ship, lance)
}

/// Every slug in the world, as its damage budget.
fn slugs(app: &mut App) -> Vec<ProjectileDamage> {
    let world = app.world_mut();
    let mut q = world.query_filtered::<&ProjectileDamage, With<RailgunSlugProjectileMarker>>();
    q.iter(world).copied().collect()
}

fn hold_trigger(app: &mut App, lance: Entity, held: bool) {
    **app
        .world_mut()
        .get_mut::<RailgunSectionInput>(lance)
        .expect("the lance carries its trigger") = held;
}

/// The commit, and the whole reason the charge exists: the trigger does not
/// fire the gun, it starts a clock, and the shot lands when the clock does.
#[test]
fn a_committed_lance_fires_when_its_charge_completes_and_not_before() {
    let mut app = railgun_app();
    // 1/60 s per step (the harness clock), so this is six steps of charge.
    let charge_seconds = 0.1;
    let (_ship, lance) = spawn_lance_ship(
        &mut app,
        RailgunSectionConfig {
            charge_seconds,
            ..default()
        },
        Vec3::NEG_Z * 2.0,
    );

    hold_trigger(&mut app, lance, true);
    app.update();
    assert!(
        matches!(
            app.world().get::<RailgunCharge>(lance),
            Some(RailgunCharge::Charging { .. })
        ),
        "the trigger commits the shot rather than firing it"
    );
    // Release: a charge already running is not the trigger's business.
    hold_trigger(&mut app, lance, false);
    app.update();
    assert!(
        slugs(&mut app).is_empty(),
        "nothing has left the bore before the charge completes"
    );
    assert!(
        matches!(
            app.world().get::<RailgunCharge>(lance),
            Some(RailgunCharge::Charging { .. })
        ),
        "dropping the trigger cannot abort a committed charge"
    );

    for _ in 0..8 {
        app.update();
    }
    assert_eq!(
        slugs(&mut app).len(),
        1,
        "the charge completed, so exactly one shell left"
    );
    assert_eq!(
        app.world().get::<RailgunCharge>(lance),
        Some(&RailgunCharge::Ready),
        "the gun returns to Ready after the shot"
    );
}

/// The owner's call, made structural: POWER is the only bound on how deep one
/// shell goes. A layer count here - any layer count - would be the cap the
/// railgun deliberately does not have.
#[test]
fn a_fired_slug_carries_its_authored_power_and_no_layer_cap() {
    let mut app = railgun_app();
    let (_ship, lance) = spawn_lance_ship(
        &mut app,
        RailgunSectionConfig {
            slug_damage: 90.0,
            slug_power: 4_000.0,
            ..default()
        },
        Vec3::NEG_Z * 2.0,
    );

    hold_trigger(&mut app, lance, true);
    app.update();

    let fired = slugs(&mut app);
    assert_eq!(fired.len(), 1, "one shell");
    assert_eq!(fired[0].kind, DamageType::Pierce);
    assert_eq!(fired[0].amount, 90.0);
    assert_eq!(fired[0].power, 4_000.0);
    assert_eq!(
        fired[0].layers,
        u32::MAX,
        "a lance stops when it runs out of thickness to spend, never on a layer count"
    );
}

/// Recoil is real, and it is applied AT THE MUZZLE: a lance on the ship's
/// axis shoves the hull straight back, and the same lance bolted off the axis
/// also spins it. That difference is the mechanic - where the builder put the
/// gun is part of what it costs.
#[test]
fn recoil_shoves_the_hull_back_and_spins_it_when_the_bore_is_off_axis() {
    let fire = |offset: Vec3| {
        let mut app = railgun_app();
        let (ship, lance) = spawn_lance_ship(
            &mut app,
            RailgunSectionConfig {
                recoil_impulse: 400.0,
                ..default()
            },
            offset,
        );
        hold_trigger(&mut app, lance, true);
        app.update();
        let world = app.world();
        (
            **world.get::<LinearVelocity>(ship).expect("a body"),
            **world.get::<AngularVelocity>(ship).expect("a body"),
        )
    };

    // Bore down the ship's own -Z, through the hull: pure pushback.
    let (on_axis_linear, on_axis_angular) = fire(Vec3::NEG_Z * 2.0);
    assert!(
        on_axis_linear.z > 0.0,
        "the shot leaves along -Z, so the ship is pushed along +Z: {on_axis_linear}"
    );
    assert!(
        on_axis_angular.length() < 1.0e-3,
        "a bore through the centre of mass has no lever arm: {on_axis_angular}"
    );

    // Same gun, bolted high off the axis: still pushed back, and now rolled.
    let (off_axis_linear, off_axis_angular) = fire(Vec3::new(0.0, 3.0, -2.0));
    assert!(
        off_axis_linear.z > 0.0,
        "still pushed back: {off_axis_linear}"
    );
    assert!(
        off_axis_angular.length() > on_axis_angular.length(),
        "an off-axis bore torques the hull it is bolted to: {off_axis_angular}"
    );
}

/// One shell in the tube, and the reload IS the cadence. The magazine gates
/// the COMMIT rather than the shot, so an empty gun never burns a charge it
/// cannot finish.
#[test]
fn an_empty_lance_refuses_the_commit_until_its_shell_returns() {
    let mut app = railgun_app();
    let (_ship, lance) = spawn_lance_ship(
        &mut app,
        RailgunSectionConfig {
            ammo_capacity: Some(1),
            reload: Some(SectionReloadConfig {
                delay: 5.0,
                amount: 1,
            }),
            ..default()
        },
        Vec3::NEG_Z * 2.0,
    );

    hold_trigger(&mut app, lance, true);
    app.update();
    assert_eq!(slugs(&mut app).len(), 1, "the loaded shell fires");
    assert!(
        app.world()
            .get::<SectionAmmo>(lance)
            .expect("a magazine")
            .is_empty(),
        "the shot spent the one shell"
    );

    // The trigger is still held. Without a shell there is nothing to commit,
    // so the gun must stay Ready rather than charge into an empty breech.
    for _ in 0..10 {
        app.update();
    }
    assert_eq!(slugs(&mut app).len(), 1, "no second shell appeared");
    assert_eq!(
        app.world().get::<RailgunCharge>(lance),
        Some(&RailgunCharge::Ready),
        "an empty gun refuses the commit instead of charging a shot it cannot take"
    );
}

/// The safety is live through the charge, not just at the trigger: a ship
/// that goes safe mid-charge dumps the charge and KEEPS the shell. A lance is
/// one shell per reload cycle, so firing it into a friendly because the
/// safety came on a tick late is the expensive version of the mistake.
#[test]
fn safing_a_ship_mid_charge_dumps_the_charge_and_keeps_the_shell() {
    let mut app = railgun_app();
    let (ship, lance) = spawn_lance_ship(
        &mut app,
        RailgunSectionConfig {
            charge_seconds: 0.2,
            ammo_capacity: Some(1),
            ..default()
        },
        Vec3::NEG_Z * 2.0,
    );
    app.world_mut().entity_mut(ship).insert(WeaponsHot(true));

    hold_trigger(&mut app, lance, true);
    app.update();
    assert!(matches!(
        app.world().get::<RailgunCharge>(lance),
        Some(RailgunCharge::Charging { .. })
    ));

    app.world_mut()
        .get_mut::<WeaponsHot>(ship)
        .expect("a safety")
        .0 = false;
    hold_trigger(&mut app, lance, false);
    for _ in 0..20 {
        app.update();
    }

    assert!(slugs(&mut app).is_empty(), "the safed gun fired nothing");
    assert_eq!(
        app.world().get::<RailgunCharge>(lance),
        Some(&RailgunCharge::Ready),
        "the charge was dumped"
    );
    assert!(
        !app.world()
            .get::<SectionAmmo>(lance)
            .expect("a magazine")
            .is_empty(),
        "the shell is still in the tube"
    );
}

/// The charge tell and the charge clock are the same clock. The bolt's
/// progress is written from the gameplay charge, so art can never promise a
/// shot that has not arrived (see `SectionAnimationCue::Charge`).
#[test]
fn the_charge_cue_tracks_the_gameplay_charge_and_resets_on_the_shot() {
    let mut app = railgun_app();
    let charge_seconds = 0.2;
    let (_ship, lance) = spawn_lance_ship(
        &mut app,
        RailgunSectionConfig {
            charge_seconds,
            ..default()
        },
        Vec3::NEG_Z * 2.0,
    );
    app.world_mut()
        .entity_mut(lance)
        .insert(SectionAnimations::new(vec![SectionAnimation {
            cue: SectionAnimationCue::Charge,
            node_prefix: "charge_bolt".to_string(),
            motion: SectionAnimationMotion::Translate {
                offset: Vec3::NEG_Z * 2.4,
            },
            open_seconds: 0.0,
            close_seconds: 0.0,
        }]));

    let progress = |app: &App| {
        app.world()
            .get::<SectionAnimations>(lance)
            .expect("the tracks")
            .cue_progress(SectionAnimationCue::Charge)
            .expect("a charge track")
    };

    hold_trigger(&mut app, lance, true);
    app.update();
    assert_eq!(
        progress(&app),
        0.0,
        "the commit tick leaves the bolt at the breech - no charge has run yet"
    );

    app.update();
    let early = progress(&app);
    assert!(
        early > 0.0 && early < 1.0,
        "the bolt is partway up the bore: {early}"
    );

    app.update();
    assert!(
        progress(&app) > early,
        "the bolt keeps walking while the charge runs"
    );

    hold_trigger(&mut app, lance, false);
    for _ in 0..20 {
        app.update();
    }
    assert_eq!(slugs(&mut app).len(), 1, "the shot went");
    assert_eq!(
        progress(&app),
        0.0,
        "the bolt snaps back to the breech the instant the shell leaves"
    );
}
