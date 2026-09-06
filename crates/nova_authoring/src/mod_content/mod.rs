//! The shipped mods' authored content: every builder-generated file under
//! `assets/mods/<id>/`.
//!
//! The base game is `base_content`. A mod here layers on top of it: it ships
//! its own art with `self://` refs and borrows base art with
//! `dep://base/<path>`, exactly like a mod written by hand.

pub(crate) mod nova_protocol;
