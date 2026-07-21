//! [`AssetRef`] - an asset reference that authors as a path string and resolves
//! to a live [`Handle`] at spawn time.
//!
//! Config types (ship sections, scenario objects) reference render meshes,
//! particle effects, and images. In code those are `Handle<A>`; in a
//! hand-authored RON modding file they must be a *path*. `AssetRef<A>` is both:
//! it (de)serializes as the asset path, and [`AssetRef::resolve`] turns it into a
//! `Handle<A>` through the [`AssetServer`]. Code-built configs use
//! `AssetRef::from(handle)` and resolve to that same handle.
//!
//! Resolution is non-mutating and idempotent (the `AssetServer` returns the same
//! handle for the same path), so an `AssetRef` keeps its authorable path for its
//! whole life and can be re-serialized (e.g. by the editor's scenario save).

use bevy::prelude::*;

/// Glob-import surface: `use nova_gameplay::asset_ref::prelude::*` re-exports the public API of this module.
pub mod prelude {
    pub use super::AssetRef;
}

/// A reference to an asset of type `A`: either an authorable path or a resolved
/// handle. See the module docs.
///
/// `Clone`/`Debug`/`PartialEq`/`Eq` are implemented by hand rather than derived:
/// both variants (`String` and `Handle<A>`) satisfy those traits for every asset
/// type, but a `#[derive]` would wrongly add an `A: Clone` (etc.) bound and
/// exclude asset types like `EffectAsset` that are not themselves `Debug`.
pub enum AssetRef<A: Asset> {
    /// An asset path, as written in a data file. Resolved lazily by
    /// [`AssetRef::resolve`].
    Path(String),
    /// An already-resolved handle: code-built configs, or a ref that was
    /// constructed from a live handle.
    Handle(Handle<A>),
}

impl<A: Asset> Clone for AssetRef<A> {
    fn clone(&self) -> Self {
        match self {
            AssetRef::Path(path) => AssetRef::Path(path.clone()),
            AssetRef::Handle(handle) => AssetRef::Handle(handle.clone()),
        }
    }
}

impl<A: Asset> std::fmt::Debug for AssetRef<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssetRef::Path(path) => f.debug_tuple("AssetRef::Path").field(path).finish(),
            AssetRef::Handle(handle) => f.debug_tuple("AssetRef::Handle").field(handle).finish(),
        }
    }
}

impl<A: Asset> PartialEq for AssetRef<A> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (AssetRef::Path(a), AssetRef::Path(b)) => a == b,
            (AssetRef::Handle(a), AssetRef::Handle(b)) => a == b,
            _ => false,
        }
    }
}

impl<A: Asset> Eq for AssetRef<A> {}

impl<A: Asset> Default for AssetRef<A> {
    fn default() -> Self {
        AssetRef::Handle(Handle::default())
    }
}

impl<A: Asset> From<Handle<A>> for AssetRef<A> {
    fn from(handle: Handle<A>) -> Self {
        AssetRef::Handle(handle)
    }
}

impl<A: Asset> From<String> for AssetRef<A> {
    fn from(path: String) -> Self {
        AssetRef::Path(path)
    }
}

impl<A: Asset> From<&str> for AssetRef<A> {
    fn from(path: &str) -> Self {
        AssetRef::Path(path.to_string())
    }
}

impl<A: Asset> AssetRef<A> {
    /// Resolve to a live handle. Paths load through the `AssetServer`
    /// (idempotent - the same path yields the same handle); already-resolved
    /// handles are cloned. Non-mutating, so the ref keeps its authorable path.
    pub fn resolve(&self, asset_server: &AssetServer) -> Handle<A> {
        match self {
            AssetRef::Path(path) => asset_server.load(path),
            AssetRef::Handle(handle) => handle.clone(),
        }
    }

    /// The authored path, if this ref was authored as one (`None` for a
    /// handle-backed ref).
    pub fn path(&self) -> Option<&str> {
        match self {
            AssetRef::Path(path) => Some(path.as_str()),
            AssetRef::Handle(_) => None,
        }
    }
}

#[cfg(feature = "serde")]
impl<A: Asset> serde::Serialize for AssetRef<A> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            AssetRef::Path(path) => serializer.serialize_str(path),
            AssetRef::Handle(_) => Err(serde::ser::Error::custom(
                "AssetRef::Handle has no authorable path; only path-authored asset refs serialize",
            )),
        }
    }
}

#[cfg(feature = "serde")]
impl<'de, A: Asset> serde::Deserialize<'de> for AssetRef<A> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let path = <String as serde::Deserialize>::deserialize(deserializer)?;
        Ok(AssetRef::Path(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str_is_a_path_and_from_handle_is_a_handle() {
        let by_path: AssetRef<Image> = "textures/rock.png".into();
        assert_eq!(by_path.path(), Some("textures/rock.png"));

        let by_handle: AssetRef<Image> = Handle::<Image>::default().into();
        assert_eq!(by_handle.path(), None);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn path_ref_round_trips_through_ron_as_a_bare_string() {
        let original: AssetRef<Image> = "scenarios/space.cube.png".into();
        let ron = ron::to_string(&original).expect("serialize");
        assert_eq!(ron, "\"scenarios/space.cube.png\"");
        let back: AssetRef<Image> = ron::from_str(&ron).expect("deserialize");
        assert_eq!(back, original);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn handle_ref_cannot_serialize() {
        let handle_ref: AssetRef<Image> = Handle::<Image>::default().into();
        assert!(ron::to_string(&handle_ref).is_err());
    }
}
