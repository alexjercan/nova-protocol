//! The two slider-driven tracks and the dB taper every slider goes through.
//!
//! Owns the per-bus volume resources. The level of a sound is its
//! [`SoundCategoryType`](super::SoundCategoryType) row; this module only
//! decides how far the player's sliders turn a whole track down.

use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    category::{db_to_gain, SoundBus},
    master::{HarnessMute, MasterVolume},
};

/// The bottom of every slider's range, in dB: the gain one notch above zero.
///
/// A linear slider spends its whole top half inside the last 6 dB, so the
/// player hears nothing move until the knob is nearly at the bottom. A dB
/// taper spreads the change evenly over the travel.
pub const SLIDER_FLOOR_DB: f32 = -40.0;

/// The gain a slider setting applies: zero is silent, and above zero the
/// setting maps linearly onto dB from [`SLIDER_FLOOR_DB`] at the bottom to
/// 0 dB at the top. The setting is clamped to `0.0..=1.0` first, so a corrupt
/// persisted value can never drive the mixer past unity.
pub fn slider_gain(setting: f32) -> f32 {
    let setting = setting.clamp(0.0, 1.0);
    if setting <= 0.0 {
        0.0
    } else {
        db_to_gain(SLIDER_FLOOR_DB * (1.0 - setting))
    }
}

/// The route of a legacy caller-levelled voice. Only the interface is left on
/// it: the interface producers still pass their own volume through
/// [`PlaySfx`](super::PlaySfx) and [`SfxVoice`](super::SfxVoice), and move to
/// [`PlaySound`](super::PlaySound) with the rest of the interface.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum AudioRoute {
    /// UI chrome, on [`SoundBus::Ui`]. Non-positional.
    #[default]
    Interface,
}

impl AudioRoute {
    /// The track this route is scaled by.
    pub fn bus(self) -> SoundBus {
        match self {
            Self::Interface => SoundBus::Ui,
        }
    }
}

/// The Interface slider setting, `0.0..=1.0`. Scales [`SoundBus::Ui`].
#[derive(Resource, Clone, Copy, PartialEq, Debug, Reflect)]
#[reflect(Resource)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InterfaceVolume(pub f32);

impl Default for InterfaceVolume {
    fn default() -> Self {
        Self(1.0)
    }
}

impl InterfaceVolume {
    /// The clamped setting: what the menu shows and the store saves.
    pub fn factor(self) -> f32 {
        self.0.clamp(0.0, 1.0)
    }

    /// The gain the setting applies, through [`slider_gain`].
    pub fn gain(self) -> f32 {
        slider_gain(self.0)
    }
}

/// The Effects slider setting, `0.0..=1.0`. Scales [`SoundBus::Effect`]. The
/// name stays `World` because the saved settings store it under that name.
#[derive(Resource, Clone, Copy, PartialEq, Debug, Reflect)]
#[reflect(Resource)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WorldVolume(pub f32);

impl Default for WorldVolume {
    fn default() -> Self {
        Self(1.0)
    }
}

impl WorldVolume {
    /// The clamped setting: what the menu shows and the store saves.
    pub fn factor(self) -> f32 {
        self.0.clamp(0.0, 1.0)
    }

    /// The gain the setting applies, through [`slider_gain`].
    pub fn gain(self) -> f32 {
        slider_gain(self.0)
    }
}

/// Read access to every knob that scales a voice: the two bus sliders and the
/// master.
///
/// Every resource is optional so an audio-only test rig that never adds
/// [`NovaSoundPlugin`](super::NovaSoundPlugin) runs at full volume instead of
/// panicking on a missing resource.
#[derive(SystemParam)]
pub struct Mixer<'w> {
    interface: Option<Res<'w, InterfaceVolume>>,
    world: Option<Res<'w, WorldVolume>>,
    master: Option<Res<'w, MasterVolume>>,
    mute: Option<Res<'w, HarnessMute>>,
}

impl Mixer<'_> {
    /// The gain `bus`'s slider applies, WITHOUT the master.
    ///
    /// This is what a freshly spawned sink is given, because bevy's
    /// `audio_output` multiplies the master in for us through `GlobalVolume`.
    pub fn bus_gain(&self, bus: SoundBus) -> f32 {
        match bus {
            SoundBus::Ui => self
                .interface
                .as_deref()
                .copied()
                .unwrap_or_default()
                .gain(),
            SoundBus::Effect => self.world.as_deref().copied().unwrap_or_default().gain(),
        }
    }

    /// The master output gain, which a PER-FRAME sink write must fold in by
    /// hand.
    ///
    /// Writing a sink's volume bypasses the `GlobalVolume` path bevy applies
    /// once at sink creation, so the master is re-applied here - through
    /// [`MasterVolume::output_gain`], which is what a [`HarnessMute`]d run
    /// silences.
    pub fn master_gain(&self) -> f32 {
        let mute = self.mute.as_deref().copied().unwrap_or_default();
        self.master.as_deref().map_or(1.0, |m| m.output_gain(mute))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_slider_at_zero_is_silent_and_at_full_is_unity() {
        assert_eq!(slider_gain(0.0), 0.0);
        assert_eq!(slider_gain(1.0), 1.0);
    }

    #[test]
    fn a_slider_moves_the_level_by_equal_db_steps() {
        let db = |setting: f32| 20.0 * slider_gain(setting).log10();
        assert!((db(0.5) - SLIDER_FLOOR_DB / 2.0).abs() < 1e-3);
        assert!((db(0.25) - db(0.5) - (db(0.5) - db(0.75))).abs() < 1e-3);
        assert!((db(f32::MIN_POSITIVE) - SLIDER_FLOOR_DB).abs() < 1e-3);
    }

    #[test]
    fn a_corrupt_persisted_volume_cannot_push_the_mixer_out_of_range() {
        assert_eq!(InterfaceVolume(4.0).gain(), 1.0);
        assert_eq!(WorldVolume(-1.0).gain(), 0.0);
        assert_eq!(InterfaceVolume(4.0).factor(), 1.0);
    }
}
