//! Live-tree tests for `nova_menu`, one module per screen or mode. `support`
//! holds the shared app rig they all build on.

mod ambience;
#[cfg(not(target_arch = "wasm32"))]
mod leave;
#[cfg(not(target_arch = "wasm32"))]
mod load_screen;
mod menu;
mod mods;
mod outcome;
mod pause;
mod portal;
mod safe_mode;
mod scenarios;
mod settings;
mod settings_store;
mod support;
mod training;
mod world_setup;
