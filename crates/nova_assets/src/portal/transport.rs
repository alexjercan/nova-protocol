//! The portal's one-shot byte fetches: the [`PortalTransport`] seam, the
//! production `ehttp` implementation, and the [`PortalClient`] resource tests
//! swap to inject mocks and failures.

use std::sync::Arc;

use bevy::prelude::*;

/// A fetched body, or a human-readable failure (transport error or a non-2xx
/// status).
pub type FetchResult = Result<Vec<u8>, String>;

/// One-shot byte fetches, callback-completed - the whole transport surface
/// the portal client needs. Object-safe and `Send + Sync` so tests inject
/// mocks/failures through the [`PortalClient`] resource without touching the
/// state machines.
pub trait PortalTransport: Send + Sync + 'static {
    /// Fetch `url`; deliver the raw body (non-2xx statuses are `Err`) to
    /// `on_done` from whatever context the implementation completes on (a
    /// worker thread natively, the JS microtask queue on wasm - the callback
    /// must only post messages, never touch world state).
    fn fetch(&self, url: &str, on_done: Box<dyn FnOnce(FetchResult) + Send>);
}

/// The production transport: `ehttp` GETs (native: a ureq call on a spawned
/// thread; wasm: the browser `fetch` API) under the one cross-platform API.
pub struct EhttpTransport;

impl PortalTransport for EhttpTransport {
    fn fetch(&self, url: &str, on_done: Box<dyn FnOnce(FetchResult) + Send>) {
        ehttp::fetch(ehttp::Request::get(url), move |result| {
            on_done(match result {
                Ok(response) if response.ok => Ok(response.bytes),
                Ok(response) => Err(format!("HTTP {} {}", response.status, response.status_text)),
                Err(error) => Err(error),
            });
        });
    }
}

/// The swappable transport handle. Production inserts [`EhttpTransport`];
/// tests replace the resource with a mock after adding
/// [`PortalPlugin`](super::PortalPlugin).
#[derive(Resource, Clone)]
pub struct PortalClient(pub Arc<dyn PortalTransport>);
