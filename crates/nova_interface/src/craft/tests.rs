use bevy::ecs::system::RunSystemOnce;
use nova_gameplay::test_support::test_items;

use super::*;
use crate::icons::InterfaceIcons;

/// A ship recipe: 1 iron ore and 1 stone ore make 100 PDC rounds in 8 s.
fn ship_rounds() -> RecipeDesign {
    RecipeDesign {
        id: RecipeDesignId::from("test/pdc_round"),
        inputs: vec![
            (ItemDesignId::from(ItemRoleType::IronOre), 1),
            (ItemDesignId::from(ItemRoleType::StoneOre), 1),
        ],
        output: (ItemDesignId::from(ItemRoleType::PdcRound), 100),
        duration_secs: 8.0,
        site: CraftSiteType::Ship,
    }
}

/// A station recipe: 2 iron ore make 1 rail slug in 10 s for a 10 cr fee.
fn station_slug() -> RecipeDesign {
    RecipeDesign {
        id: RecipeDesignId::from("test/rail_slug"),
        inputs: vec![(ItemDesignId::from(ItemRoleType::IronOre), 2)],
        output: (ItemDesignId::from(ItemRoleType::RailSlug), 1),
        duration_secs: 10.0,
        site: CraftSiteType::Station { fee_cr: 10 },
    }
}

/// An undocked player ship with 4 iron ore, 2 stone ore and 100 cr.
fn spawn_player(world: &mut World) -> Entity {
    let hold = ShipInventory::new(
        &test_items(),
        400_000,
        [
            (ItemDesignId::from(ItemRoleType::IronOre), 4),
            (ItemDesignId::from(ItemRoleType::StoneOre), 2),
        ],
    );
    world
        .spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            hold,
            ShipCredits(100),
        ))
        .id()
}

fn craft_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(InterfaceIcons::blank());
    app.insert_resource(test_items());
    app.insert_resource(GameRecipes::new([ship_rounds(), station_slug()]));
    app.init_resource::<CraftRuntime>();
    app.add_message::<CraftStartCommand>();
    app.add_systems(
        Update,
        (apply_craft_start_commands, update_craft_view).chain(),
    );
    let root = app.world_mut().spawn(Node::default()).id();
    app.world_mut()
        .run_system_once_with(
            |parent: In<Entity>, mut commands: Commands| {
                commands.entity(*parent).with_children(craft_body);
            },
            root,
        )
        .expect("building the Craft view");
    app
}

/// Every text under the entity named `name`, depth first.
fn texts_under(world: &mut World, name: &str) -> Vec<String> {
    let root = world
        .query::<(Entity, &Name)>()
        .iter(world)
        .find(|(_, each)| each.as_str() == name)
        .map(|(entity, _)| entity)
        .unwrap_or_else(|| panic!("a node named {name}"));
    let mut texts = Vec::new();
    let mut stack = vec![root];
    while let Some(entity) = stack.pop() {
        if let Some(text) = world.get::<Text>(entity) {
            texts.push(text.0.clone());
        }
        if let Some(children) = world.get::<Children>(entity) {
            stack.extend(children.iter());
        }
    }
    texts
}

/// Undocked, the ship recipe's tile is live and the station recipe's tile is
/// muted and names the dock it needs; the first tile is picked.
#[test]
fn an_undocked_ship_sees_its_ship_recipe_live_and_a_station_recipe_greyed() {
    let mut app = craft_app();
    spawn_player(app.world_mut());
    app.update();
    let world = app.world_mut();
    let ship_tile = texts_under(world, &craft_tile_name(&ship_rounds().id));
    assert!(
        !ship_tile.iter().any(|text| text.starts_with("Dock")),
        "the ship recipe runs in flight: {ship_tile:?}"
    );
    let station_tile = texts_under(world, &craft_tile_name(&station_slug().id));
    assert!(
        station_tile.contains(&"x1  STATION".to_string())
            && station_tile.contains(&"Dock at a Craft station".to_string()),
        "the station recipe is greyed with its reason: {station_tile:?}"
    );
    assert_eq!(
        world.resource::<CraftRuntime>().selected(),
        Some(&ship_rounds().id),
        "ship recipes sort first and the first tile is picked"
    );
}

/// Craft on a ship recipe inserts the job; a station recipe started
/// undocked is refused, takes nothing and says why.
#[test]
fn craft_starts_a_ship_recipe_in_flight_and_refuses_a_station_recipe_undocked() {
    let mut app = craft_app();
    let ship = spawn_player(app.world_mut());
    app.update();

    app.world_mut().write_message(CraftStartCommand {
        recipe: station_slug().id,
        batches: 1,
    });
    app.update();
    assert!(app.world().get::<ShipCraftJob>(ship).is_none());
    assert_eq!(
        app.world().resource::<CraftRuntime>().notice.text(),
        Some("Dock at a Craft station")
    );

    app.world_mut().write_message(CraftStartCommand {
        recipe: ship_rounds().id,
        batches: 2,
    });
    app.update();
    assert_eq!(
        app.world().get::<ShipCraftJob>(ship),
        Some(&ShipCraftJob::new(ship_rounds().id, 2))
    );
    assert_eq!(
        app.world().resource::<CraftRuntime>().notice.text(),
        Some("Crafting 200 PDC round: 16 s on the flight clock")
    );

    // The job runs: every tile now says so.
    app.update();
    let tile = texts_under(app.world_mut(), &craft_tile_name(&ship_rounds().id));
    assert!(tile.contains(&"A craft is running".to_string()), "{tile:?}");
}

/// The entity named `name`.
fn named(world: &mut World, name: &str) -> Entity {
    world
        .query::<(Entity, &Name)>()
        .iter(world)
        .find(|(_, each)| each.as_str() == name)
        .map(|(entity, _)| entity)
        .unwrap_or_else(|| panic!("a node named {name}"))
}

/// The slider runs from 1 to the batches the hold covers; its value sets the
/// count, the Takes and Makes lines and the batches Craft starts, and a value
/// past the top is held at it.
#[test]
fn the_batch_slider_sets_the_batches_a_craft_click_starts() {
    let mut app = craft_app();
    let ship = spawn_player(app.world_mut());
    app.update();
    let slider = named(app.world_mut(), CRAFT_SLIDER);
    assert_eq!(
        app.world().get::<SliderRange>(slider),
        Some(&SliderRange::new(1.0, 2.0)),
        "2 stone ore cover 2 batches of PDC rounds"
    );

    app.world_mut().trigger(ValueChange {
        source: slider,
        value: 9.0_f32,
        is_final: true,
    });
    app.update();
    assert_eq!(app.world().resource::<CraftRuntime>().batches(), 2);
    assert_eq!(
        app.world().get::<SliderValue>(slider),
        Some(&SliderValue(2.0))
    );
    let form = texts_under(app.world_mut(), "CraftCard");
    for line in [
        "2",
        "of 2",
        "Takes 2 iron ore, 2 stone ore",
        "Makes 200 PDC round in 16 s",
    ] {
        assert!(form.contains(&line.to_string()), "{line:?} in {form:?}");
    }

    let craft = named(app.world_mut(), CRAFT_BUTTON);
    app.world_mut().trigger(Activate { entity: craft });
    app.update();
    assert_eq!(
        app.world().get::<ShipCraftJob>(ship),
        Some(&ShipCraftJob::new(ship_rounds().id, 2))
    );
}
