//! A generic, extensible event system for Bevy games.
//!
//! This module provides traits and components to define, filter, and handle game events
//! in a flexible way. Events are queued and processed in a world-specific context,
//! allowing complex systems to react to game state changes.
//!
//! Nova owns this engine. The event vocabulary in this crate's root and the
//! machinery that dispatches it are one design, so keeping the dispatcher in a
//! separate shared crate made every change to the vocabulary a two-repository
//! change. The data-driven handler registry that shipped beside it upstream had
//! no callers here and did not come across.

use std::{
    collections::{HashMap, VecDeque},
    marker::PhantomData,
    sync::Arc,
};

use bevy::{ecs::component::Mutable, prelude::*};

/// Glob-import surface for the event engine: the traits a game world and its
/// handlers implement, the fired-event types, and the dispatch plugin.
pub mod prelude {
    pub use super::{
        CommandsGameEventExt, EventAction, EventFilter, EventHandler, EventHandlerIndex, EventKind,
        EventWorld, GameEvent, GameEventInfo, GameEventsPlugin,
    };
}

/// A trait representing a game world that can synchronize its state to and from systems.
///
/// Implement this trait for your game state resource to integrate with `GameEventsPlugin`.
pub trait EventWorld: Resource<Mutability = Mutable> + Send + Sync {
    /// System to update the world from a saved or external state.
    fn world_to_state_system(world: &mut World);

    /// System to update the state back to the world after processing events.
    fn state_to_world_system(world: &mut World);

    /// True while the bevy world this event world feeds is still being BUILT.
    ///
    /// An event world whose [`state_to_world_system`](Self::state_to_world_system)
    /// applies its queued work over SEVERAL frames reports true until that work
    /// is done. Dispatch is held while it is true, so no handler ever runs
    /// against a half-populated world - "the world is not yet live" rather than
    /// "the world is briefly inconsistent". An event world that applies
    /// everything in one call returns false forever.
    fn is_settling(&self) -> bool;
}

/// A trait representing a kind of game event.
///
/// Each event kind defines its data type (`Info`) and a unique name.
///
/// Usually derived with `#[derive(EventKind)]`. Without attributes the
/// derive defaults `Info` to `()` (no payload) and the name to the
/// lowercased struct name; override them with `#[event_info(MyPayload)]`
/// and `#[event_name("my_name")]`.
pub trait EventKind: Clone + Send + Sync + 'static {
    /// The type of event data associated with this event kind.
    type Info: serde::Serialize + Default + Clone + std::fmt::Debug + Send + Sync + 'static;

    /// Returns a unique name for this event type.
    fn name() -> &'static str;
}

/// A trait representing an action to perform in response to an event.
pub trait EventAction<W: EventWorld>: Send + Sync {
    /// Execute the action on the given world using the event info.
    fn action(&self, world: &mut W, info: &GameEventInfo);

    /// Returns the name of the action (defaults to the Rust type name).
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

/// A trait representing a filter that determines if an event should trigger an action.
pub trait EventFilter<W: EventWorld>: Send + Sync {
    /// Returns true if the event passes the filter.
    fn filter(&self, world: &W, info: &GameEventInfo) -> bool;

    /// Returns the name of the filter (defaults to the Rust type name).
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

/// Component that handles game events by applying filters and executing actions.
#[derive(Component, Reflect)]
pub struct EventHandler<W: EventWorld> {
    pub(super) name: &'static str,
    pub(super) filters: Vec<Arc<dyn EventFilter<W>>>,
    pub(super) actions: Vec<Arc<dyn EventAction<W>>>,
}

// NOTE: hand-written, not derived -- a derive would add a `W: Clone` bound the
// fields (a `&'static str` and `Vec`s of `Arc` trait objects) do not need, and
// `EventHandlerIndex` requires this impl to snapshot handlers.
impl<W: EventWorld> Clone for EventHandler<W> {
    fn clone(&self) -> Self {
        Self {
            name: self.name,
            filters: self.filters.clone(),
            actions: self.actions.clone(),
        }
    }
}

impl<W> EventHandler<W>
where
    W: EventWorld,
{
    /// Create a new handler for a given event kind.
    pub fn new<E: EventKind>() -> Self {
        Self::from_event_name(E::name())
    }

    /// Create a handler bound to an already-resolved event name.
    ///
    /// Prefer [`EventHandler::new`] with an [`EventKind`]; this lower-level
    /// constructor exists for data-driven builders (the registry) that only
    /// have the event name as a `&'static str` resolved from a string.
    pub fn from_event_name(name: &'static str) -> Self {
        Self {
            name,
            filters: Vec::new(),
            actions: Vec::new(),
        }
    }

    /// Push an already-shared filter (used by data-driven construction).
    pub fn add_filter_arc(&mut self, f: Arc<dyn EventFilter<W>>) {
        self.filters.push(f);
    }

    /// Push an already-shared action (used by data-driven construction).
    pub fn add_action_arc(&mut self, a: Arc<dyn EventAction<W>>) {
        self.actions.push(a);
    }

    /// Add a filter to the handler (builder-style).
    pub fn with_filter<F: EventFilter<W> + 'static>(mut self, f: F) -> Self {
        self.filters.push(Arc::new(f));
        self
    }

    /// Add a filter to the handler.
    pub fn add_filter<F: EventFilter<W> + 'static>(&mut self, f: F) {
        self.filters.push(Arc::new(f));
    }

    /// Add an action to the handler (builder-style).
    pub fn with_action<A: EventAction<W> + 'static>(mut self, a: A) -> Self {
        self.actions.push(Arc::new(a));
        self
    }

    /// Add an action to the handler.
    pub fn add_action<A: EventAction<W> + 'static>(&mut self, a: A) {
        self.actions.push(Arc::new(a));
    }

    /// Checks if the event passes all filters.
    pub fn filter(&self, world: &W, info: &GameEventInfo) -> bool {
        self.filters.iter().all(|f| f.filter(world, info))
    }
}

/// Event data wrapper.
#[derive(Debug, Clone, Default)]
pub struct GameEventInfo {
    /// Optional serialized data for the event.
    pub data: Option<serde_json::Value>,
}

impl GameEventInfo {
    /// Create an event info from serializable data.
    ///
    /// A serialization failure yields `data: None`, which every entity filter
    /// reads as "does not match" - so the handler stops firing PERMANENTLY
    /// rather than erroring. Loud, because nothing downstream can tell the two
    /// apart.
    pub fn from_data<T: serde::Serialize>(data: T) -> Self {
        let json_value = match serde_json::to_value(data) {
            Ok(value) => Some(value),
            Err(e) => {
                error!(
                    "GameEventInfo::from_data: {e} - entity-filtered handlers for this event \
                     will stop matching"
                );
                None
            }
        };
        Self { data: json_value }
    }
}

impl<T: serde::Serialize> From<T> for GameEventInfo {
    fn from(value: T) -> Self {
        GameEventInfo::from_data(value)
    }
}

/// Represents a fired game event in the Bevy event system.
#[derive(Event, Debug, Clone)]
pub struct GameEvent {
    pub(super) name: &'static str,
    pub(super) info: GameEventInfo,
}

impl GameEvent {
    /// Create a new game event with the given name and info.
    pub fn new(name: &'static str, info: GameEventInfo) -> Self {
        Self { name, info }
    }

    /// The event's kind name (the `EventKind::name()` it was fired as).
    ///
    /// Public so an external observer on `On<GameEvent>` - a run recorder, a
    /// debug overlay - can SEE which event passed by without being able to
    /// forge or mutate it; construction stays [`GameEvent::new`]-only.
    ///
    /// ```
    /// use nova_events::engine::{GameEvent, GameEventInfo};
    ///
    /// let event = GameEvent::new("ondestroyed", GameEventInfo::default());
    /// assert_eq!(event.name(), "ondestroyed");
    /// assert!(event.info().data.is_none());
    /// ```
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// The event's payload wrapper (its optional serialized `data`).
    ///
    /// Same contract as [`name`](Self::name): read access for observers,
    /// no mutation.
    pub fn info(&self) -> &GameEventInfo {
        &self.info
    }
}

/// Extension trait for `Commands` to fire game events easily.
pub trait CommandsGameEventExt {
    /// Trigger a [`GameEvent`] of kind `E` carrying `info`.
    fn fire<E: EventKind>(&mut self, info: E::Info);
}

impl<'w, 's> CommandsGameEventExt for Commands<'w, 's> {
    fn fire<E: EventKind>(&mut self, info: E::Info) {
        self.trigger(GameEvent::new(E::name(), info.into()));
    }
}

/// Resource holding a queue of pending game events for a specific world type.
#[derive(Resource, Debug, Clone, Default)]
pub struct GameEventQueue<W> {
    /// Fired events awaiting dispatch, oldest first.
    pub events: VecDeque<GameEvent>,
    _marker: std::marker::PhantomData<W>,
}

/// Plugin that processes game events for a specific world type.
pub struct GameEventsPlugin<W> {
    _marker: std::marker::PhantomData<W>,
}

// Hand-written rather than derived: `#[derive(Default)]` would add a
// `W: Default` bound, and `W` only ever appears behind `PhantomData`.
impl<W> Default for GameEventsPlugin<W> {
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

impl<W> Plugin for GameEventsPlugin<W>
where
    W: EventWorld + Default,
{
    fn build(&self, app: &mut bevy::prelude::App) {
        debug!("GameEventsPlugin: build");
        app.init_resource::<GameEventQueue<W>>();
        app.init_resource::<EventHandlerIndex<W>>();
        app.add_observer(on_game_event::<W>);

        app.init_resource::<W>();
        // NOTE: ungated and every frame, unlike the dispatch chain below, so a
        // handler spawned or despawned on a quiet frame still lands before the next
        // event; `.before` pins it ahead of the dispatch that reads it.
        app.add_systems(
            PostUpdate,
            maintain_handler_index::<W>.before(queue_system::<W>),
        );
        app.add_systems(
            PostUpdate,
            (
                W::world_to_state_system,
                // HELD while the world is settling: a handler that counts
                // objects must not read a world whose spawns are still
                // landing. Fired events keep queueing and dispatch in order
                // once the world is live. `state_to_world_system` deliberately
                // stays ungated - it is what applies the rest.
                queue_system::<W>.run_if(not(is_settling::<W>)),
                W::state_to_world_system,
            )
                .chain()
                .run_if(
                    not(is_queue_empty::<W>)
                        .or_else(resource_changed::<W>)
                        .or_else(is_settling::<W>),
                ),
        );
    }
}

/// Returns true if the event queue is empty.
fn is_queue_empty<W>(queue: Res<GameEventQueue<W>>) -> bool
where
    W: Send + Sync + 'static,
{
    queue.events.is_empty()
}

/// Returns true while the event world is still building the bevy world
/// (see [`EventWorld::is_settling`]).
fn is_settling<W: EventWorld>(world: Res<W>) -> bool {
    world.is_settling()
}

/// Observer that pushes fired events into the queue.
fn on_game_event<W>(event: On<GameEvent>, mut queue: ResMut<GameEventQueue<W>>)
where
    W: Send + Sync + 'static,
{
    let event = event.event();
    trace!(
        "on_game_event: event {:?}, info {:?}",
        event.name,
        event.info
    );

    queue.events.push_back(event.clone());
}

/// Index of spawned handlers keyed by their event name, holding contiguous
/// handler snapshots the dispatcher iterates directly.
///
/// The dispatcher (the private `queue_system`) used to scan *every* handler in
/// the world for *every* fired event, matching on `handler.name == event.name`.
/// That is `O(handlers)` per event regardless of how many actually react - fine
/// for the handful a first-party scenario spawns, but wasteful once a large
/// community mod brings hundreds of handlers most of which are for other event
/// names.
///
/// A first attempt indexed *entity ids* and looked each up with `Query::get`
/// during dispatch. Benchmarking showed that lost most of the win at scale: it
/// trades Bevy's fast linear archetype iteration for random-access component
/// lookups that thrash cache as the handler count grows.
/// So the index instead stores cheap **clones** of the handlers themselves
/// (an [`EventHandler`] is just a `&'static str` plus two `Vec`s of `Arc` trait
/// objects), grouped by event name. Dispatch then walks a single contiguous
/// `Vec` for the fired event, touching neither the ECS nor scattered memory.
///
/// The snapshot is valid because a handler is built, spawned once, and never
/// mutated in place; the private `maintain_handler_index` refreshes it on
/// add/despawn.
#[derive(Resource)]
pub struct EventHandlerIndex<W: EventWorld> {
    by_name: HashMap<&'static str, Vec<(Entity, EventHandler<W>)>>,
    names: HashMap<Entity, &'static str>,
    _marker: PhantomData<W>,
}

// Hand-written rather than derived: `#[derive(Default)]` would add a
// `W: Default` bound, and `W` only ever appears behind `PhantomData`.
impl<W: EventWorld> Default for EventHandlerIndex<W> {
    fn default() -> Self {
        Self {
            by_name: HashMap::new(),
            names: HashMap::new(),
            _marker: PhantomData,
        }
    }
}

impl<W: EventWorld> EventHandlerIndex<W> {
    /// Handlers registered for `name`, in spawn order. Empty slice when nothing
    /// reacts to that event.
    pub fn handlers(&self, name: &str) -> &[(Entity, EventHandler<W>)] {
        self.by_name.get(name).map(Vec::as_slice).unwrap_or(&[])
    }
}

/// Keeps [`EventHandlerIndex`] in sync with the spawned handler entities.
///
/// Runs every frame *unconditionally* - unlike the gated dispatch chain - so an
/// added or despawned handler is reflected before the next event is processed,
/// and `RemovedComponents` is drained each frame rather than overflowing its
/// double-buffer. A handler entity is spawned once and never changes its event
/// name, so the `Added` filter is sufficient; the reverse map guards against a
/// (theoretical) double insert.
fn maintain_handler_index<W: EventWorld>(
    mut index: ResMut<EventHandlerIndex<W>>,
    added: Query<(Entity, &EventHandler<W>), Added<EventHandler<W>>>,
    mut removed: RemovedComponents<EventHandler<W>>,
) {
    for entity in removed.read() {
        if let Some(name) = index.names.remove(&entity) {
            if let Some(bucket) = index.by_name.get_mut(name) {
                bucket.retain(|(e, _)| *e != entity);
            }
        }
    }
    for (entity, handler) in &added {
        if index.names.insert(entity, handler.name).is_none() {
            index
                .by_name
                .entry(handler.name)
                .or_default()
                .push((entity, handler.clone()));
        }
    }
}

/// Processes the event queue by applying handlers and executing actions.
fn queue_system<W: EventWorld>(
    mut queue: ResMut<GameEventQueue<W>>,
    mut world: ResMut<W>,
    index: Res<EventHandlerIndex<W>>,
) {
    while let Some(event) = queue.events.pop_front() {
        trace!(
            "queue_system: processing event {:?}, info {:?}",
            event.name,
            event.info
        );

        for (_entity, handler) in index.handlers(event.name) {
            if handler.filter(&*world, &event.info) {
                trace!("queue_system: handler {:?} passed filters", handler.name);

                for action in &handler.actions {
                    trace!("queue_system: executing action {:?}", action.name());
                    action.action(&mut *world, &event.info);
                }
            }
        }

        // A handler that queued world work has made the world INCOMPLETE from
        // here on, so the pass STOPS and the remaining events wait for the
        // frame it is live again. Without this the scenario's OnStart burst and
        // the same frame's OnUpdate pulse dispatch back to back, and the pulse
        // reads a world whose every object is still sitting in the queue.
        if world.is_settling() {
            trace!("queue_system: world is settling; holding the rest of the queue");
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap as StdHashMap;

    // NOTE: `bevy::prelude::*` (App, World, Resource, ...) arrives through this
    // glob, which re-exports the parent module's own prelude import.
    use super::*;

    /// The attribute-less `#[derive(EventKind)]`: name defaults to the lowercased
    /// struct name and `Info` to `()`. This test exists to keep that default
    /// COMPILING - it is the path AGENTS.md once told everyone to avoid, because
    /// the original default named a type that neither resolved nor implemented
    /// `Serialize`. Nothing else in the repo derives without `#[event_info(...)]`.
    #[test]
    fn attribute_less_derive_defaults_to_no_payload() {
        #[derive(Clone, nova_events_macros::EventKind)]
        struct OnQuiet;

        assert_eq!(OnQuiet::name(), "onquiet");

        // NOTE: this call is the real assertion -- the `Info = ()` bound fails to
        // COMPILE, not to run, if the default payload type ever changes back to
        // something that does not resolve, which is the original defect.
        fn requires_unit_payload<E: EventKind<Info = ()>>() {}
        requires_unit_payload::<OnQuiet>();
    }

    /// Minimal event world that just counts action fires by tag. `settling`
    /// stands in for a world whose spawns are still landing.
    #[derive(Resource, Default)]
    struct Counts {
        fires: StdHashMap<&'static str, u32>,
        settling: bool,
    }

    impl EventWorld for Counts {
        fn world_to_state_system(_: &mut World) {}
        fn state_to_world_system(_: &mut World) {}
        fn is_settling(&self) -> bool {
            self.settling
        }
    }

    /// Action that bumps the counter for its tag when it runs.
    struct Bump(&'static str);
    impl EventAction<Counts> for Bump {
        fn action(&self, world: &mut Counts, _: &GameEventInfo) {
            *world.fires.entry(self.0).or_default() += 1;
        }
    }

    /// Action that queues world work - the stand-in for a spawn.
    struct Settle;
    impl EventAction<Counts> for Settle {
        fn action(&self, world: &mut Counts, _: &GameEventInfo) {
            world.settling = true;
        }
    }

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(GameEventsPlugin::<Counts>::default());
        app
    }

    fn spawn_handler(app: &mut App, event: &'static str, tag: &'static str) -> Entity {
        let mut handler = EventHandler::<Counts>::from_event_name(event);
        handler.add_action(Bump(tag));
        app.world_mut().spawn(handler).id()
    }

    /// Push one event of `name` and run a frame so the index dispatches it.
    fn fire(app: &mut App, name: &'static str) {
        app.world_mut()
            .resource_mut::<GameEventQueue<Counts>>()
            .events
            .push_back(GameEvent::new(name, GameEventInfo::default()));
        app.update();
    }

    fn count(app: &App, tag: &str) -> u32 {
        app.world()
            .resource::<Counts>()
            .fires
            .get(tag)
            .copied()
            .unwrap_or(0)
    }

    /// A SETTLING world holds dispatch: nothing runs against a world whose
    /// spawns are still landing. The held events are not dropped - they
    /// dispatch, in order, on the first frame the world reports live.
    #[test]
    fn a_settling_world_holds_dispatch_until_it_is_live() {
        let mut app = app();
        spawn_handler(&mut app, "alpha", "a1");
        app.update();

        app.world_mut().resource_mut::<Counts>().settling = true;
        fire(&mut app, "alpha");
        fire(&mut app, "alpha");
        assert_eq!(
            count(&app, "a1"),
            0,
            "a settling world must not dispatch (the pre-gate engine fires here)"
        );

        app.world_mut().resource_mut::<Counts>().settling = false;
        app.update();
        assert_eq!(
            count(&app, "a1"),
            2,
            "both held events dispatch once the world is live"
        );
    }

    /// A handler that queues world work STOPS the pass: the events behind it
    /// wait for the frame the world is live again. This is the scenario's
    /// OnStart-then-OnUpdate case - without the break the pulse dispatches in
    /// the same pass as the spawn burst and reads an empty world.
    #[test]
    fn queueing_world_work_holds_the_rest_of_the_pass() {
        let mut app = app();
        let mut spawner = EventHandler::<Counts>::from_event_name("alpha");
        spawner.add_action(Bump("a1"));
        spawner.add_action(Settle);
        app.world_mut().spawn(spawner);
        spawn_handler(&mut app, "beta", "b1");
        app.update();

        let queue = &mut app
            .world_mut()
            .resource_mut::<GameEventQueue<Counts>>()
            .events;
        queue.push_back(GameEvent::new("alpha", GameEventInfo::default()));
        queue.push_back(GameEvent::new("beta", GameEventInfo::default()));
        app.update();

        assert_eq!(count(&app, "a1"), 1, "the first event dispatched");
        assert_eq!(
            count(&app, "b1"),
            0,
            "the event behind the spawn must wait for a live world"
        );

        app.world_mut().resource_mut::<Counts>().settling = false;
        app.update();
        assert_eq!(count(&app, "b1"), 1, "it dispatches once the world is live");
    }

    #[test]
    fn dispatch_routes_by_event_name_via_index() {
        let mut app = app();
        spawn_handler(&mut app, "alpha", "a1");
        spawn_handler(&mut app, "alpha", "a2");
        spawn_handler(&mut app, "beta", "b1");
        app.update(); // maintain_handler_index picks up the added handlers

        fire(&mut app, "alpha");
        assert_eq!(count(&app, "a1"), 1);
        assert_eq!(count(&app, "a2"), 1);
        assert_eq!(count(&app, "b1"), 0, "beta handler must not run on alpha");

        fire(&mut app, "beta");
        assert_eq!(count(&app, "b1"), 1);
        assert_eq!(
            count(&app, "a1"),
            1,
            "alpha handlers must not re-run on beta"
        );

        // NOTE: gamma has no registered handler at all -- firing it must be a
        // harmless no-op, not a panic or a stray dispatch.
        fire(&mut app, "gamma");
        assert_eq!(count(&app, "a1"), 1);
    }

    #[test]
    fn despawned_handler_is_pruned_from_the_index() {
        let mut app = app();
        let a1 = spawn_handler(&mut app, "alpha", "a1");
        spawn_handler(&mut app, "alpha", "a2");
        app.update();

        fire(&mut app, "alpha");
        assert_eq!(count(&app, "a1"), 1);
        assert_eq!(count(&app, "a2"), 1);

        // NOTE: the dispatch chain is idle on this frame, so pruning can only come
        // from the ungated maintenance system -- that is what this step exercises.
        app.world_mut().entity_mut(a1).despawn();
        app.update();

        fire(&mut app, "alpha");
        assert_eq!(count(&app, "a1"), 1, "despawned handler must not run");
        assert_eq!(count(&app, "a2"), 2);
    }

    /// The read-accessor contract: a plain observer on `On<GameEvent>` (a run
    /// recorder, a debug overlay) sees each event's kind name and payload as it
    /// passes by, without draining the dispatch queue.
    #[test]
    fn observers_read_a_fired_events_name_and_payload() {
        #[derive(Resource, Default)]
        struct Seen(Vec<(String, Option<serde_json::Value>)>);

        let mut app = app();
        app.init_resource::<Seen>();
        app.add_observer(|event: On<GameEvent>, mut seen: ResMut<Seen>| {
            seen.0
                .push((event.name().to_string(), event.info().data.clone()));
        });

        app.world_mut().trigger(GameEvent::new(
            "ondestroyed",
            GameEventInfo::from_data(serde_json::json!({ "id": "prey" })),
        ));
        app.world_mut()
            .trigger(GameEvent::new("onstart", GameEventInfo::default()));

        let seen = app.world().resource::<Seen>();
        assert_eq!(seen.0.len(), 2);
        assert_eq!(seen.0[0].0, "ondestroyed");
        assert_eq!(
            seen.0[0].1.as_ref().and_then(|d| d["id"].as_str()),
            Some("prey")
        );
        assert_eq!(seen.0[1], ("onstart".to_string(), None));

        // NOTE: the point of this assertion is that observing did NOT starve the
        // real dispatch path -- the queue observer ran too, so handlers still see both.
        let queue = app.world().resource::<GameEventQueue<Counts>>();
        assert_eq!(queue.events.len(), 2);
    }
}
