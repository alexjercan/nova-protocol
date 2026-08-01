//! Terminal model: the [`NovaOsTerminal`] resource, its scrollback row types,
//! the boot/welcome/help content builders, and the prompt-render string
//! helpers. This is the pure command-prompt state the bevy UI in
//! `nova_gameplay` reads and drives.
//!
//! Three concerns, one module each, all re-exported here so `terminal::<item>`
//! paths are unchanged: `state` holds the resource, its types and the session
//! lifecycle; `edit` holds the prompt editing, submit, completion and history;
//! `view` holds the rows and prompt strings the UI renders.

mod edit;
mod state;
mod view;

#[cfg(test)]
mod fixtures;

pub use edit::*;
pub use state::*;
pub use view::*;
