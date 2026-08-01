//! Shared observers for the audio tests: the two resources every cue
//! module asserts through.

use bevy::prelude::*;

/// Count of `PlaySfx` triggers observed, standing in for "sounds played".
#[derive(Resource, Default)]
pub(super) struct PlayedSfx(pub(super) usize);

/// The handle of the last `PlaySfx` observed, so a test can assert WHICH
/// sound played (not just that one did) - the discriminator between a
/// section's own authored fire sound and the global bank cue.
#[derive(Resource, Default)]
pub(super) struct LastPlayed(pub(super) Option<Handle<AudioSource>>);
