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

/// A station recipe: 2 iron ore make 1 rail slug in 10 s for a 10 cr fee,
/// at a station of advancement 0.5 or more.
fn station_slug() -> RecipeDesign {
    RecipeDesign {
        id: RecipeDesignId::from("test/rail_slug"),
        inputs: vec![(ItemDesignId::from(ItemRoleType::IronOre), 2)],
        output: (ItemDesignId::from(ItemRoleType::RailSlug), 1),
        duration_secs: 10.0,
        site: CraftSiteType::Station {
            fee_cr: 10,
            advancement: 0.5,
        },
    }
}

/// Docked at a Craft station below a station recipe's advancement, the
/// recipe has no tile; at the floor and undocked it shows.
#[test]
fn a_station_recipe_the_docked_station_cannot_craft_is_hidden() {
    let items = test_items();
    let recipes = GameRecipes::new([ship_rounds(), station_slug()]);
    let site = |station| CraftSite {
        hold: ShipInventory::new(&items, 400_000, []),
        own_cr: 100,
        station,
        running: false,
    };
    let ids = |site: &CraftSite| -> Vec<RecipeDesignId> {
        shown_recipes(&recipes, &items, site, None)
            .into_iter()
            .map(|recipe| recipe.id.clone())
            .collect()
    };
    assert_eq!(ids(&site(Some(0.25))), [ship_rounds().id]);
    assert_eq!(ids(&site(Some(0.5))), [ship_rounds().id, station_slug().id]);
    assert_eq!(ids(&site(None)), [ship_rounds().id, station_slug().id]);
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
/// muted under its `STATION` tag; the first tile is picked.
#[test]
fn an_undocked_ship_sees_its_ship_recipe_live_and_a_station_recipe_greyed() {
    let mut app = craft_app();
    spawn_player(app.world_mut());
    app.update();
    let world = app.world_mut();
    let refused = |world: &mut World, recipe: &RecipeDesignId| {
        let tiles = world
            .query::<&CraftTileGrid>()
            .single(world)
            .expect("one grid");
        tiles
            .0
            .as_ref()
            .and_then(|tiles| tiles.iter().find(|tile| tile.recipe == *recipe))
            .map(|tile| tile.refused)
            .expect("the recipe's tile")
    };
    assert!(
        !refused(world, &ship_rounds().id),
        "the ship recipe runs in flight"
    );
    assert!(
        refused(world, &station_slug().id),
        "the station recipe is greyed undocked"
    );
    assert_eq!(
        world.resource::<CraftRuntime>().selected(),
        Some(&ship_rounds().id),
        "ship recipes sort first and the first tile is picked"
    );
}

/// Craft on a ship recipe inserts the job; a station recipe started
/// undocked is refused and takes nothing.
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

    app.world_mut().write_message(CraftStartCommand {
        recipe: ship_rounds().id,
        batches: 2,
    });
    app.update();
    assert_eq!(
        app.world().get::<ShipCraftJob>(ship),
        Some(&ShipCraftJob::new(ship_rounds().id, 2))
    );
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
/// count, the card's totals and the batches Craft starts, and a value past
/// the top is held at it.
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
    for line in ["2", "of 2", "PDC round x200", "4 / 2", "2 / 2", "16 s"] {
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
