//! What the stage puts a name on, and when.

use nova_scenario::prelude::{SpaceshipConfig, SpaceshipController};

use super::*;
use crate::node::{ObjectChoice, ScenarioNode, ShipDriver};

/// A scenario holding one ship and one rock, with the plate layer up.
fn stage() -> (App, Entity, Entity) {
    let mut app = App::new();
    app.init_resource::<UiSkin>();
    app.init_resource::<SelectedNode>();
    app.init_resource::<HoveredNode>();
    let scenario = app
        .world_mut()
        .spawn((ScenarioNode, NodeId("scenario".to_string())))
        .id();
    let ship = app
        .world_mut()
        .spawn((
            ShipNode {
                name: "Kestrel".to_string(),
                driver: ShipDriver::Player,
                ..default()
            },
            NodeId("ship_1".to_string()),
            ChildOf(scenario),
        ))
        .id();
    let rock = app
        .world_mut()
        .spawn((
            // Unnamed, so the plate has to fall back to the minted id the
            // way the tree row does.
            ObjectNode {
                name: String::new(),
                ..ObjectChoice::Asteroid.stock()
            },
            NodeId("asteroid_3".to_string()),
            ChildOf(scenario),
        ))
        .id();
    app.world_mut().spawn(plate_layer());
    app.insert_resource(EditContext {
        path: vec![scenario],
    });
    app.add_systems(Update, sync_nameplates);
    (app, ship, rock)
}

/// The labels the stage is wearing, in the order they were built.
fn plates(app: &mut App) -> Vec<String> {
    let hung: Vec<Entity> = app
        .world_mut()
        .query_filtered::<Entity, With<NamePlate>>()
        .iter(app.world())
        .collect();
    hung.into_iter()
        .filter_map(|plate| {
            let kids = app.world().get::<Children>(plate)?;
            kids.iter()
                .find_map(|kid| app.world().get::<Text>(kid).map(|text| text.0.clone()))
        })
        .collect()
}

/// The stage's own labels go UNDER the rail and the Inspector: a plate that
/// crossed a panel drew phosphor text over a phosphor list and neither could
/// be read. The rung is positive because a UI node below the camera's own rung
/// does not draw at all - the first cut of this used -20 and the plates
/// vanished.
#[test]
fn a_plate_hangs_under_the_panels() {
    let mut app = App::new();
    let hung = app.world_mut().spawn(plate_layer()).id();
    let rung = app
        .world()
        .get::<GlobalZIndex>(hung)
        .expect("the layer claims a rung")
        .0;
    assert!(rung < layer::CHROME_Z, "a plate is under the panels");
    assert!(rung > 0, "a plate still draws");
}

/// Every ship is named because there are a handful of them and which hull is
/// which is a question the stage could not otherwise answer. A rock sitting
/// there is not asking anything.
#[test]
fn a_ship_is_named_on_the_stage_and_a_rock_is_not() {
    let (mut app, _, _) = stage();
    app.update();
    assert_eq!(plates(&mut app), vec!["Kestrel".to_string()]);
}

/// The rock takes a plate for exactly as long as it is the one being worked
/// on, then gives it back.
#[test]
fn a_marked_rock_takes_a_plate_and_gives_it_back() {
    let (mut app, _, rock) = stage();
    app.world_mut().resource_mut::<SelectedNode>().0 = Some(rock);
    app.update();
    assert_eq!(
        plates(&mut app),
        vec!["Kestrel".to_string(), "asteroid 3".to_string()]
    );

    app.world_mut().resource_mut::<SelectedNode>().0 = None;
    app.update();
    assert_eq!(plates(&mut app), vec!["Kestrel".to_string()]);
}

/// The seeded hulls are objects rather than [`ShipNode`]s, and they are the
/// exact case the plates exist for: five derelicts are five identical grey
/// shapes until one of them says which one it is.
#[test]
fn a_seeded_hull_is_named_without_being_marked() {
    let (mut app, _, _) = stage();
    let scenario = app
        .world()
        .resource::<EditContext>()
        .scenario()
        .expect("the fixture has a scenario");
    app.world_mut().spawn((
        ObjectNode {
            name: "Derelict Hulk 3".to_string(),
            kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
                controller: SpaceshipController::None,
                ..default()
            }),
        },
        NodeId("spaceship_2".to_string()),
        ChildOf(scenario),
    ));
    app.update();
    assert_eq!(
        plates(&mut app),
        vec!["Kestrel".to_string(), "Derelict Hulk 3".to_string()]
    );
}

/// Hover alone is enough: finding a rock on the stage without clicking it is
/// the whole point of the hover link between the rail and the stage.
#[test]
fn hover_alone_names_a_rock() {
    let (mut app, _, rock) = stage();
    app.world_mut().resource_mut::<HoveredNode>().0 = Some(rock);
    app.update();
    assert_eq!(plates(&mut app).len(), 2);
}

/// Inside a ship the stage holds one hull, and the breadcrumb at the top of
/// the screen already names it.
#[test]
fn the_plates_come_off_inside_a_ship() {
    let (mut app, ship, _) = stage();
    app.update();
    app.world_mut().resource_mut::<EditContext>().enter(ship);
    app.update();
    assert!(plates(&mut app).is_empty());
}
