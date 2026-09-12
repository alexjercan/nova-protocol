//! Entity lifetime: [`TempEntity`] despawns an entity after a timer, and
//! [`DespawnEntity`] despawns it the moment the marker lands.
//!
//! Nova owns this because it is what every transient the game spawns rides on -
//! torpedo projectiles, muzzle flashes, impact volumes and the debris
//! [`crate::integrity::explode`] cuts all carry a `TempEntity` with a lifetime
//! that is a tuning decision, not an engine one.
//!
//! ## Usage
//!
//! ```rust
//! # use bevy::prelude::*;
//! # use nova_gameplay::prelude::*;
//! # fn demo(mut commands: Commands) {
//! commands.spawn((
//!     TempEntity(5.0), // despawns after 5 seconds
//! ));
//! commands.spawn((
//!     DespawnEntity, // entity will be despawned immediately
//! ));
//! # }
//! ```

use bevy::prelude::*;

/// The despawn message and its plugin, and `TempEntity` with `TempEntityPlugin`.
pub mod prelude {
    pub use super::{
        DespawnEntity, DespawnEntityPlugin, TempEntity, TempEntityPlugin, TempEntityState,
    };
}

/// Component indicating that the entity is temporary.
///
/// The inner value is the lifetime of the entity in seconds.
/// When the timer runs out, the entity will be automatically despawned.
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub struct TempEntity(pub f32);

/// The countdown behind a [`TempEntity`]: how much of the authored lifetime is
/// left.
///
/// Readable, never constructible from outside - the inner timer stays private,
/// so only [`TempEntityPlugin`] inserts and ticks it. Read it for the REMAINING
/// lifetime of a transient (a torpedo's flight time, a debris chunk's); the
/// `TempEntity` beside it carries the authored TOTAL and never moves.
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub struct TempEntityState(Timer);

/// Plugin that manages temporary entities.
///
/// Automatically inserts the timer state on entities with `TempEntity` and
/// updates timers each frame to despawn entities when the duration expires.
pub struct TempEntityPlugin;

impl Plugin for TempEntityPlugin {
    fn build(&self, app: &mut App) {
        trace!("TempEntityPlugin: build");

        app.add_observer(on_insert_temp_entity);

        // Update, so the timers tick with the frame delta.
        app.add_systems(Update, update_temp_entities);
    }
}

/// Initialize the internal timer when a TempEntity is added.
fn on_insert_temp_entity(
    insert: On<Insert, TempEntity>,
    mut commands: Commands,
    q_temp: Query<&TempEntity>,
) {
    let entity = insert.entity;
    trace!("on_insert_temp_entity: entity {:?}", entity);

    let Ok(temp_entity) = q_temp.get(entity) else {
        error!(
            "on_insert_temp_entity: entity {:?} not found in q_temp",
            entity
        );
        return;
    };

    commands
        .entity(entity)
        .insert(TempEntityState(Timer::from_seconds(
            **temp_entity,
            TimerMode::Once,
        )));
}

/// Update timers for temporary entities and despawn them when finished.
fn update_temp_entities(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut TempEntityState)>,
) {
    for (entity, mut temp_state) in query.iter_mut() {
        temp_state.tick(time.delta());

        if temp_state.is_finished() {
            // try_despawn: this sweep runs against entities other systems also
            // delete (a torpedo that fuzes on the frame its lifetime expires),
            // and both despawns land in the same flush. The loser of that race
            // logs a WARN (`despawn` bakes in the warn handler, see
            // `crate::test_log`), which is a failed clean pass on every probe
            // run.
            commands.entity(entity).try_despawn();
            trace!("update_temp_entities: despawn entity {:?}", entity);
        }
    }
}

/// Marker component that indicates an entity should be despawned immediately.
///
/// Adding this component to an entity triggers the plugin to remove it
/// from the world in the same frame.
#[derive(Component, Clone, Debug, Reflect)]
pub struct DespawnEntity;

/// Plugin that handles immediate despawning of entities marked with `DespawnEntity`.
pub struct DespawnEntityPlugin;

impl Plugin for DespawnEntityPlugin {
    fn build(&self, app: &mut App) {
        trace!("DespawnEntityPlugin: build");

        app.add_observer(on_insert_despawn_entity);
    }
}

/// Observer system that runs when a `DespawnEntity` component is inserted.
///
/// This system immediately despawns the entity.
///
/// try_despawn, for the same race `update_temp_entities` names above: the mark
/// is data-driven, so a scenario script can raise it on the same frame the
/// entity dies of something else, and the loser of that flush warns.
fn on_insert_despawn_entity(insert: On<Insert, DespawnEntity>, mut commands: Commands) {
    let entity = insert.entity;
    trace!("on_insert_despawn_entity: entity {:?}", entity);

    commands.entity(entity).try_despawn();
}

#[cfg(test)]
mod tests {
    use bevy::log::tracing_subscriber::{self, util::SubscriberInitExt};

    use super::*;
    use crate::test_log::CapturedLog;

    /// The mark is a public, data-driven signal, so it can have more than one
    /// answer: a mod that reaps its own entities registers a second observer
    /// on it. Observer order for one event is unspecified, so one of them
    /// finds the entity already gone - the same race `on_destroyed_entity`
    /// names for the destruction reaper, and the reason its sibling
    /// `update_temp_entities` already reaches for `try_despawn`.
    ///
    /// Asserted on the LOG: `EntityCommands::despawn` bakes in the WARN
    /// handler at queue time (see `crate::test_log`), so the loser leaves a
    /// warn line rather than a crash - and the probe's clean pass fails a run
    /// that logs it.
    #[test]
    fn a_second_reaper_on_the_same_mark_leaves_no_stale_command() {
        let log = CapturedLog::default();
        let writer = log.clone();
        let _guard = tracing_subscriber::fmt()
            .with_writer(move || writer.clone())
            .set_default();

        let mut app = App::new();
        // The mod's own reaper, registered FIRST so the plugin's is the one
        // that finds nothing left. Observer order is unspecified in general;
        // it is pinned here so the test is about the losing reaper rather
        // than about which one loses.
        app.add_observer(
            |insert: On<Insert, DespawnEntity>, mut commands: Commands| {
                commands.entity(insert.entity).try_despawn();
            },
        );
        app.add_plugins(DespawnEntityPlugin);
        let entity = app.world_mut().spawn_empty().id();

        app.world_mut().entity_mut(entity).insert(DespawnEntity);

        assert!(!app.world().entities().contains(entity));
        assert!(
            !log.contents().contains("despawn"),
            "the reaper that loses the race must leave no stale command; got: {}",
            log.contents()
        );
    }
}
