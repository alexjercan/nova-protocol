//! Cargo a destroyed body leaves: a ship's remaining hold and a share of each
//! destroyed section's build parts, as cargo canisters.
//!
//! Both drops ride [`IntegrityDestroyMarker`], which lands once per entity:
//!
//! - a destroyed section drops [`BuildRecipe::strip_parts`] of its recipe:
//!   `floor(strip_yield * count)` of each part, so a part whose share rounds
//!   down to zero drops nothing;
//! - a destroyed ship root drops every stack still in its hold, and a
//!   destroyed station root also drops the player's [`StationStorage`] with
//!   it, so the stored items outlive the station.
//!
//! A section dies before its ship root does: a structural collapse peels every
//! section first, and the root dies only with no section standing. A section
//! that dies while its root collapses adds its parts to the root's
//! [`CollapseSalvage`] instead of dropping them, and the root's own drop
//! carries them with the hold, so a ship destroyed whole leaves one drop. Any
//! other section drops its parts on its own as it dies.
//!
//! One drop's stacks pack together into mixed canisters of whole items by
//! [`CargoCanister::pack`], each filled in item id order up to the canister
//! mass cap; separate drops never share a canister. The canisters wait in a
//! [`MinedOreDrop`](crate::mining::MinedOreDrop) at the dead body's position,
//! under the parent its ship root had, so a sector retirement freezes it. The
//! mining ejector ([`MiningPlugin`](crate::mining::MiningPlugin)) lets them
//! out one at a time, each born off the drop while the birth point is clear and
//! drifting away from it. A section's canisters leave along the line from its
//! ship's centre through the section; a ship root's leave along the root's up.
//! A drop does not move with the ship it came from, but a section's drop off
//! a hull that lives on carries the hull's velocity at that point
//! ([`MinedDropDrift`]) and hands it to every canister, so a moving hull does
//! not ram its own salvage. Each canister takes its id
//! from `CargoCanisterIdAllocator` at birth, so a waiting drop and a born
//! canister both freeze, thaw and save like mined ore.

use std::collections::{BTreeMap, HashMap};

use avian3d::prelude::{AngularVelocity, LinearVelocity};
use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;

use crate::mining::{ore_drop, MinedCanisterQueue, MinedDropDrift};

/// `WreckCargoPlugin`.
pub mod prelude {
    pub use super::WreckCargoPlugin;
}

/// The parts of sections destroyed while their ship root collapses, by root,
/// waiting for the root's own drop. A root despawned without being destroyed,
/// as a scenario unload despawns it, drops nothing and its entry is forgotten.
#[derive(Resource, Default, Debug)]
struct CollapseSalvage(HashMap<Entity, BTreeMap<ItemDesignId, u32>>);

/// Drop a destroyed section's parts share or a destroyed ship's hold, with
/// a station's player storage.
#[expect(
    clippy::type_complexity,
    reason = "the dying body's pose, owner, recipe, hold and storage"
)]
fn drop_wreck_cargo(
    add: On<Add, IntegrityDestroyMarker>,
    mut commands: Commands,
    mut salvage: ResMut<CollapseSalvage>,
    q_dying: Query<(
        &GlobalTransform,
        Option<&ChildOf>,
        Option<&SectionBuildConfig>,
        Option<&ShipInventory>,
        Option<&StationStorage>,
    )>,
    q_frames: Query<(
        &GlobalTransform,
        Option<&ChildOf>,
        Has<StructuralCollapseMarker>,
    )>,
    q_motion: Query<(&LinearVelocity, &AngularVelocity)>,
    items: Option<Res<GameItems>>,
) {
    let entity = add.entity;
    let Ok((frame, child_of, build, hold, storage)) = q_dying.get(entity) else {
        return;
    };
    let parent = child_of.map(ChildOf::parent);
    let up = frame.rotation() * Vec3::Y;
    let (stacks, owner, normal, drift) = if let Some(hold) = hold {
        let mut stacks = salvage.0.remove(&entity).unwrap_or_default();
        for (item, count) in hold
            .stacks()
            .chain(storage.into_iter().flat_map(StationStorage::stacks))
        {
            *stacks.entry(item.clone()).or_default() += count;
        }
        (stacks, parent, up, None)
    } else if let Some(SectionBuildConfig(config)) = build {
        let parts = config
            .base
            .build
            .strip_parts()
            .map(|(item, count)| (item.clone(), count));
        match parent.and_then(|root| q_frames.get(root).ok().map(|found| (root, found))) {
            Some((root, (_, _, true))) => {
                let pending = salvage.0.entry(root).or_default();
                for (item, count) in parts {
                    *pending.entry(item).or_default() += count;
                }
                return;
            }
            Some((root, (root_frame, root_parent, false))) => {
                let outward = Dir3::new(frame.translation() - root_frame.translation())
                    .map_or(root_frame.rotation() * Vec3::Y, Vec3::from);
                let drift = q_motion.get(root).ok().map(|(linear, angular)| {
                    MinedDropDrift(rigid_body_point_velocity(
                        linear.0,
                        angular.0,
                        root_frame.translation(),
                        frame.translation(),
                    ))
                });
                (
                    parts.collect(),
                    root_parent.map(ChildOf::parent),
                    outward,
                    drift,
                )
            }
            None => (parts.collect(), parent, up, None),
        }
    } else {
        return;
    };
    if stacks.is_empty() {
        return;
    }
    let items = items.expect("drop_wreck_cargo: a destroyed body holds items but no GameItems");
    let canisters = CargoCanister::pack(&items, &stacks);
    let queue = MinedCanisterQueue::leaving(canisters, Vec3::ZERO, normal)
        .expect("every stack above zero packs into a canister");
    let pose = GlobalTransform::from(Transform::from_translation(frame.translation()));
    let local = match owner.and_then(|owner| q_frames.get(owner).ok()) {
        Some((owner_frame, ..)) => pose.reparented_to(owner_frame),
        None => pose.compute_transform(),
    };
    debug!(
        "drop_wreck_cargo: {entity:?} dropped {} item stacks at {}",
        stacks.len(),
        frame.translation()
    );
    let mut drop = match owner {
        Some(owner) => commands.spawn((ore_drop(queue), local, pose, ChildOf(owner))),
        None => commands.spawn((ore_drop(queue), local, pose)),
    };
    if let Some(drift) = drift {
        drop.insert(drift);
    }
}

/// Forget the salvage of a root that is gone without being destroyed.
fn forget_vanished_salvage(mut salvage: ResMut<CollapseSalvage>, q_live: Query<()>) {
    salvage.0.retain(|root, _| q_live.contains(*root));
}

/// Adds the cargo drops of destroyed ships and sections. The drops are
/// [`MinedOreDrop`](crate::mining::MinedOreDrop)s, which `MiningPlugin`'s
/// ejector drains, so an app adds both, as `NovaScenarioPlugin` does.
#[derive(Default, Clone, Debug)]
pub struct WreckCargoPlugin;

impl Plugin for WreckCargoPlugin {
    fn build(&self, app: &mut App) {
        trace!("WreckCargoPlugin: build");

        app.init_resource::<CollapseSalvage>();
        app.add_observer(drop_wreck_cargo);
        app.add_systems(Last, forget_vanished_salvage);
    }
}

#[cfg(test)]
mod tests;
