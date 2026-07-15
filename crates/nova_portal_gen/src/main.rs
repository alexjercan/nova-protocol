//! CLI wrapper over [`nova_portal_gen::generate`] - the deploy workflow (and
//! local dev) runs this to build the static mod portal:
//!
//! ```text
//! cargo run --release -p nova_portal_gen -- \
//!     --source webmods --shipped assets/mods.catalog.ron --out site/mods
//! ```

use std::{path::PathBuf, process::ExitCode};

use clap::Parser;

#[derive(Parser)]
#[command(name = "nova_portal_gen")]
#[command(about = "Generate the static mod portal (catalog.json + hashed file copies) from a webmods/ source tree", long_about = None)]
struct Cli {
    /// Directory of mod sources: each subdirectory is one mod (dir name = id).
    #[arg(long)]
    source: PathBuf,
    /// The game's shipped mods.catalog.ron; portal ids must not collide with it.
    #[arg(long)]
    shipped: Option<PathBuf>,
    /// Output directory for catalog.json + <id>/<version>/<files>.
    #[arg(long)]
    out: PathBuf,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match nova_portal_gen::generate(&cli.source, cli.shipped.as_deref(), &cli.out) {
        Ok(catalog) => {
            let total: u64 = catalog.entries.iter().map(|e| e.total_size).sum();
            println!(
                "portal: published {} mod(s), {} bytes -> {}",
                catalog.entries.len(),
                total,
                cli.out.display()
            );
            for entry in &catalog.entries {
                println!(
                    "  {} {} ({} files, {} bytes)",
                    entry.id,
                    entry.version,
                    entry.files.len(),
                    entry.total_size
                );
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("portal generation failed: {e}");
            ExitCode::FAILURE
        }
    }
}
