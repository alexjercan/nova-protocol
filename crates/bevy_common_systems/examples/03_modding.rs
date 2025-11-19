use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_common_systems::prelude::*;
use clap::Parser;

#[derive(Parser)]
#[command(name = "03_modding")]
#[command(version = "1.0.0")]
#[command(about = "A simple example showing how to create basic custom events", long_about = None)]
struct Cli;

fn main() {
    let _ = Cli::parse();
    let mut app = App::new();

    app.add_plugins(DefaultPlugins);
    app.add_plugins(PhysicsPlugins::default());

    #[cfg(feature = "debug")]
    app.add_plugins(InspectorDebugPlugin);

    app.add_plugins(custom_plugin);

    app.run();
}

fn custom_plugin(app: &mut App) {
    app.add_plugins(GameEventsPlugin::<CustomEventWorld>::default());
    app.insert_resource(Time::<Fixed>::from_hz(1.0));

    app.init_resource::<SomeCounter>();

    app.add_systems(Startup, setup_handler);
    app.add_systems(FixedUpdate, (print_counter_system, update_system));
}

#[derive(Resource, Default, Debug, Clone, Deref, DerefMut)]
pub struct SomeCounter(pub u32);

fn print_counter_system(counter: Res<SomeCounter>) {
    println!("print_counter_system: counter {}", **counter);
}

#[derive(Resource, Default, Debug, Clone)]
pub struct CustomEventWorld {
    pub counter: u32,
}

impl EventWorld for CustomEventWorld {
    fn world_to_state_system(world: &mut World) {
        let counter = **world.resource::<SomeCounter>();
        let mut resource = world.resource_mut::<Self>();
        resource.counter = counter;
    }

    fn state_to_world_system(world: &mut World) {
        let new_counter = world.resource::<Self>().counter;
        let mut counter = world.resource_mut::<SomeCounter>();
        **counter = new_counter;
    }
}

#[derive(Debug, Clone, EventKind)]
#[event_name("onupdate")]
#[event_info(OnUpdateEventInfo)]
pub struct OnUpdateEvent;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct OnUpdateEventInfo {
    value: f32,
}

#[derive(Debug, Clone)]
struct IncrementCounterAction;

impl EventAction<CustomEventWorld> for IncrementCounterAction {
    fn action(&self, world: &mut CustomEventWorld, _: &GameEventInfo) {
        world.counter += 1;
        println!("IncrementCounterAction: counter {}", world.counter);
    }
}

#[derive(Debug, Clone)]
struct MinValueFilter {
    min_value: f32,
}

impl EventFilter<CustomEventWorld> for MinValueFilter {
    fn filter(&self, _: &CustomEventWorld, info: &GameEventInfo) -> bool {
        let Some(data) = &info.data else {
            return false;
        };

        let Some(value) = data.get("value").and_then(|v| v.as_f64()) else {
            return false;
        };

        println!("MinValueFilter: value {}", value);
        (value as f32) >= self.min_value
    }
}

fn setup_handler(mut commands: Commands) {
    commands.spawn((
        Name::new("OnUpdate Handler"),
        EventHandler::<CustomEventWorld>::new::<OnUpdateEvent>()
            .with_filter(MinValueFilter { min_value: 0.5 })
            .with_action(IncrementCounterAction),
    ));
}

fn update_system(mut commands: Commands) {
    commands.fire::<OnUpdateEvent>(OnUpdateEventInfo {
        value: rand::random(),
    });
}
