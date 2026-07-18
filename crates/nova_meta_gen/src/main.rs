//! CLI for the default `.meta` sidecar generator.
//!
//! Typical use is as a Trunk `post_build` hook: Trunk exports
//! `TRUNK_STAGING_DIR` (the built site, including the copied `assets/`), and
//! this tool writes any missing `.meta` sidecars into `<staging>/assets`
//! before Trunk moves staging into `dist/`. See
//! `docs/design/wasm-asset-meta-always.md`.
//!
//! Manual runs: `cargo run -p nova_meta_gen -- --assets assets`.

use std::{path::Path, process::ExitCode};

use bevy::asset::AssetServer;
use clap::Parser;
use nova_meta_gen::{build_app, generate};

#[derive(Parser, Debug)]
#[command(
    about = "Write default .meta sidecars for assets that lack one (for AssetMetaCheck::Always on the web)."
)]
struct Args {
    /// Asset directory (root of the default Bevy asset source) to process.
    /// Repeatable. If omitted, defaults to `$TRUNK_STAGING_DIR/assets` when
    /// run as a Trunk hook, else `assets`.
    #[arg(long = "assets")]
    assets: Vec<String>,
}

fn resolve_dirs(args: &Args) -> Vec<String> {
    if !args.assets.is_empty() {
        return args.assets.clone();
    }
    if let Ok(staging) = std::env::var("TRUNK_STAGING_DIR") {
        return vec![format!("{staging}/assets")];
    }
    vec!["assets".to_string()]
}

fn main() -> ExitCode {
    let args = Args::parse();
    let dirs = resolve_dirs(&args);

    let mut failed = false;
    for dir in &dirs {
        if !Path::new(dir).is_dir() {
            eprintln!("nova_meta_gen: skipping '{dir}' (not a directory)");
            continue;
        }

        // One app per source root: AssetPlugin's `file_path` is fixed at build.
        let app = build_app(dir);
        let server = app.world().resource::<AssetServer>().clone();

        match generate(&server, dir, |rel, outcome| {
            // Only the interesting cases are worth a line; existing/no-loader
            // are summarized at the end.
            if matches!(outcome, nova_meta_gen::Outcome::Written) {
                println!("nova_meta_gen: wrote {}.meta", rel.display());
            }
        }) {
            Ok(summary) => {
                println!(
                    "nova_meta_gen: {dir}: {} written, {} kept, {} skipped (no loader)",
                    summary.written, summary.already_exists, summary.no_loader
                );
            }
            Err(errors) => {
                for e in &errors {
                    eprintln!("nova_meta_gen: error: {e}");
                }
                failed = true;
            }
        }
    }

    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
