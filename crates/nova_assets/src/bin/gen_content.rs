//! Writes the builder-backed base content files (task 20260716-155823):
//!
//! ```text
//! cargo run -p nova_assets --bin gen_content
//! ```
//!
//! The scenario/section builders in `nova_assets::scenario_generation` are the
//! single definition of each built-in; this bin serializes them into the
//! committed `assets/base/**/*.content.ron` the game actually loads. Run it
//! (and commit the result) after any builder change - the `content_ron_parity`
//! test asserts the files match and names this command when they drift.

use std::path::PathBuf;

use nova_assets::scenario_generation::content_files;

fn main() {
    // CARGO_MANIFEST_DIR is compiled in, so the paths resolve regardless of
    // the invocation directory.
    let assets = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    for (rel, contents) in content_files() {
        let path = assets.join(&rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .unwrap_or_else(|err| panic!("create {}: {err}", parent.display()));
        }
        std::fs::write(&path, contents)
            .unwrap_or_else(|err| panic!("write {}: {err}", path.display()));
        println!("wrote {}", path.display());
    }
}
