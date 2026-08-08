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

/// The despawn message and its plugin, and `TempEntity` with `TempEntityPlugin` and
/// `TempEntitySystems`.
pub mod prelude {
    pub use super::{
        DespawnEntity, DespawnEntityPlugin, TempEntity, TempEntityPlugin, TempEntitySystems,
    };
}

/// Component indicating that the entity is temporary.
///
/// The inner value is the lifetime of the entity in seconds.
/// When the timer runs out, the entity will be automatically despawned.
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub struct TempEntity(pub f32);

/// Internal state for temporary entities.
///
/// This component stores the timer used to track when the entity
/// should be despawned. It is automatically inserted and updated
/// by the plugin.
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
struct TempEntityState(Timer);

/// System set for the [`TempEntityPlugin`].
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum TempEntitySystems {
    /// Ticks the lifetime timers and despawns the entities that ran out.
    Sync,
}

/// Plugin that manages temporary entities.
///
/// Automatically inserts the timer state on entities with `TempEntity` and
/// updates timers each frame to despawn entities when the duration expires.
pub struct TempEntityPlugin;

impl Plugin for TempEntityPlugin {
    fn build(&self, app: &mut App) {
        debug!("TempEntityPlugin: build");

        app.add_observer(on_insert_temp_entity);

        // NOTE: Update, so the timers tick with the frame delta.
        app.add_systems(
            Update,
            (update_temp_entities,)
                .chain()
                .in_set(TempEntitySystems::Sync),
        );
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
            commands.entity(entity).despawn();
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
        debug!("DespawnEntityPlugin: build");

        app.add_observer(on_insert_despawn_entity);
    }
}

/// Observer system that runs when a `DespawnEntity` component is inserted.
///
/// This system immediately despawns the entity.
fn on_insert_despawn_entity(insert: On<Insert, DespawnEntity>, mut commands: Commands) {
    let entity = insert.entity;
    trace!("on_insert_despawn_entity: entity {:?}", entity);

    commands.entity(entity).despawn();
}
