//! A named registry of loaded audio handles, keyed by a game-defined enum.
//!
//! Every game hand-rolls the same thing: a flat struct of named
//! `Handle<AudioSource>` fields, loaded inline from `sounds/<name>.wav` paths,
//! read back as `sfx.some_field.clone()`. [`SoundBank`] replaces that bag with a
//! keyed registry -- the game declares one small key enum and a list of
//! `(key, name)` pairs, and the bank owns the loading (applying the
//! `sounds/<name>.wav` convention) and the lookup.
//!
//! The sound *set* differs per game, so the bank is generic over the key type:
//! define a `Copy` enum, load a bank, and read handles with [`SoundBank::get`].
//! For loading UI, [`SoundBank::all_loaded`] (and the [`sounds_loaded`]
//! run-condition) report when every handle is ready -- an **opt-in** gate, since
//! games happily boot straight to their menu without one.
//!
//! ```rust
//! # use bevy::prelude::*;
//! # use nova_gameplay::prelude::*;
//! #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
//! enum Sfx {
//!     Click,
//!     GameOver,
//! }
//!
//! fn setup(mut commands: Commands, assets: Res<AssetServer>) {
//!     // Loads assets/sounds/click.wav and assets/sounds/game_over.wav.
//!     commands.insert_resource(SoundBank::load(
//!         &assets,
//!         [(Sfx::Click, "click"), (Sfx::GameOver, "game_over")],
//!     ));
//! }
//!
//! fn on_click(mut commands: Commands, sfx: Res<SoundBank<Sfx>>) {
//!     commands.play_sfx(sfx.get(Sfx::Click));
//! }
//! ```
//!
//! Nova owns this because the bank is what `nova_assets` fills with
//! [`crate::audio::UI_SFX_FILES`] and what every HUD and menu cue reads back
//! through `SoundBank<UiSfx>` - which sounds exist and what they are called is
//! a content decision, not an engine one.

use std::{collections::HashMap, fmt::Debug, hash::Hash};

use bevy::prelude::*;

/// A game's audio key: a small `Copy` enum naming each sound. Blanket-implemented
/// for any type with the needed bounds, so a game just derives them on its enum.
pub trait SoundKey: Copy + Eq + Hash + Debug + Send + Sync + 'static {}
impl<T: Copy + Eq + Hash + Debug + Send + Sync + 'static> SoundKey for T {}

/// A registry of loaded audio handles keyed by a game-defined `SoundKey` enum.
///
/// Build it with [`load`](Self::load) (or [`load_paths`](Self::load_paths)) and
/// read handles with [`get`](Self::get). Insert it as a resource
/// (`SoundBank<YourKey>`).
#[derive(Resource, Debug, Clone)]
pub struct SoundBank<K: SoundKey> {
    handles: HashMap<K, Handle<AudioSource>>,
}

impl<K: SoundKey> SoundBank<K> {
    /// Load a bank from `(key, name)` pairs, where `name` is a base filename
    /// under `assets/sounds/`: it is loaded as `sounds/<name>.wav`. This is the
    /// convention all the crate's example games share.
    pub fn load<'a>(assets: &AssetServer, entries: impl IntoIterator<Item = (K, &'a str)>) -> Self {
        Self::from_handles(entries, |name| assets.load(format!("sounds/{name}.wav")))
    }

    /// Load a bank from `(key, path)` pairs where `path` is the full asset path,
    /// for sounds outside the `sounds/<name>.wav` convention.
    pub fn load_paths<'a>(
        assets: &AssetServer,
        entries: impl IntoIterator<Item = (K, &'a str)>,
    ) -> Self {
        Self::from_handles(entries, |path| assets.load(path.to_string()))
    }

    fn from_handles<'a>(
        entries: impl IntoIterator<Item = (K, &'a str)>,
        mut load: impl FnMut(&'a str) -> Handle<AudioSource>,
    ) -> Self {
        Self {
            handles: entries
                .into_iter()
                .map(|(key, spec)| (key, load(spec)))
                .collect(),
        }
    }

    /// The handle for `key`. Panics if the key is not in the bank (every key the
    /// game plays should be loaded up front, so a miss is a programming error).
    pub fn get(&self, key: K) -> Handle<AudioSource> {
        self.try_get(key)
            .unwrap_or_else(|| panic!("SoundBank: sound {key:?} was not loaded"))
    }

    /// The handle for `key`, or `None` if it is not in the bank.
    pub fn try_get(&self, key: K) -> Option<Handle<AudioSource>> {
        self.handles.get(&key).cloned()
    }

    /// The number of sounds in the bank.
    pub fn len(&self) -> usize {
        self.handles.len()
    }

    /// Whether the bank holds no sounds.
    pub fn is_empty(&self) -> bool {
        self.handles.is_empty()
    }

    /// Whether every handle in the bank has finished loading (with its
    /// dependencies). Use it for an opt-in "wait for assets" gate; see
    /// [`sounds_loaded`].
    pub fn all_loaded(&self, assets: &AssetServer) -> bool {
        self.handles
            .values()
            .all(|handle| assets.is_loaded_with_dependencies(handle.id()))
    }
}

/// Run-condition that is `true` once every sound in the `SoundBank<K>` has
/// loaded (and `false` while the bank is missing).
///
/// The opt-in loading gate: hold a `Loading` state and advance to the menu once
/// the sounds are ready.
///
/// ```rust
/// # use bevy::prelude::*;
/// # use nova_gameplay::prelude::*;
/// # #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)] enum Sfx { Click }
/// # #[derive(States, Clone, PartialEq, Eq, Hash, Debug, Default)] enum Game { #[default] Loading, Menu }
/// # fn to_menu(mut next: ResMut<NextState<Game>>) { next.set(Game::Menu); }
/// # fn wire(app: &mut App) {
/// app.add_systems(
///     Update,
///     to_menu
///         .run_if(in_state(Game::Loading))
///         .run_if(sounds_loaded::<Sfx>),
/// );
/// # }
/// ```
pub fn sounds_loaded<K: SoundKey>(
    bank: Option<Res<SoundBank<K>>>,
    assets: Res<AssetServer>,
) -> bool {
    bank.is_some_and(|bank| bank.all_loaded(&assets))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
    enum Sfx {
        Click,
        GameOver,
        Absent,
    }

    fn asset_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app
    }

    #[test]
    fn load_builds_a_keyed_bank() {
        let app = asset_app();
        let assets = app.world().resource::<AssetServer>();
        let bank = SoundBank::load(
            assets,
            [(Sfx::Click, "click"), (Sfx::GameOver, "game_over")],
        );

        assert_eq!(bank.len(), 2);
        assert!(!bank.is_empty());
        assert_ne!(bank.get(Sfx::Click), bank.get(Sfx::GameOver));
        assert!(bank.try_get(Sfx::Absent).is_none());
    }

    #[test]
    #[should_panic(expected = "was not loaded")]
    fn get_panics_on_a_missing_key() {
        let app = asset_app();
        let assets = app.world().resource::<AssetServer>();
        let bank = SoundBank::load(assets, [(Sfx::Click, "click")]);
        let _ = bank.get(Sfx::Absent);
    }

    #[test]
    fn all_loaded_is_vacuously_true_for_an_empty_bank() {
        let app = asset_app();
        let assets = app.world().resource::<AssetServer>();
        let bank = SoundBank::<Sfx>::load(assets, []);
        assert!(bank.is_empty());
        assert!(bank.all_loaded(assets));
    }

    #[test]
    fn a_bank_with_pending_handles_is_not_loaded() {
        let app = asset_app();
        let assets = app.world().resource::<AssetServer>();
        // NOTE: a freshly requested handle is pending whether or not the file exists, which is
        // what makes a missing path a usable stand-in for a slow load here.
        let bank = SoundBank::load(assets, [(Sfx::Click, "does_not_exist")]);
        assert!(!bank.all_loaded(assets));
    }
}
