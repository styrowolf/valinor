use bit_set::BitSet;
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;
use valhalla_graphtile::tile_hierarchy::STANDARD_LEVELS;
use valhalla_graphtile::tile_provider::{
    DirectoryTileProvider, GraphTileProvider, GraphTileProviderError,
};
use valhalla_graphtile::GraphId;

static PROGRESS_STYLE: OnceLock<ProgressStyle> = OnceLock::new();

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to the Valhalla graph tiles.
    ///
    /// This currently expects a tile folder,
    /// but tarball support will be added eventually.
    #[arg(env)]
    tile_path: PathBuf,

    /// Disables progress output
    #[arg(env, long)]
    no_progress: bool,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let reader = DirectoryTileProvider::new(cli.tile_path);

    if !cli.no_progress {
        _ = PROGRESS_STYLE.set(
            ProgressStyle::with_template(
                "[{elapsed}] {bar:40.cyan/blue} {msg} {percent}% ETA {eta}",
            )?
            .progress_chars("##-"),
        );
    }

    // TODO: Almost all code below feels like it can be abstracted into a graph traversal helper...
    // We could even make processing plugins with WASM LOL
    // Enumerate edges in available tiles

    let mut tile_set = HashMap::new();
    let mut edge_count: u64 = 0;
    for level in &*STANDARD_LEVELS {
        // For each tile in that level...
        let n_tiles = level.tiling_system.n_rows * level.tiling_system.n_cols;

        let progress_bar = PROGRESS_STYLE.get().map(|style| {
            let bar = ProgressBar::new(u64::from(n_tiles));
            bar.set_message(format!(
                "Scanning {n_tiles} tiles in level {}...",
                level.level
            ));
            bar.set_style(style.clone());
            bar
        });

        for tile_id in 0..n_tiles {
            progress_bar.as_ref().inspect(|bar| bar.inc(1));
            // Get the index pointer for each tile in the level
            let graph_id = GraphId::try_from_components(level.level, u64::from(tile_id), 0)?;
            match reader.get_tile(&graph_id) {
                Ok(tile) => {
                    let tile_edge_count = u64::from(tile.header.directed_edge_count());
                    tile_set.insert(graph_id, edge_count);
                    edge_count += tile_edge_count;
                }
                Err(GraphTileProviderError::TileDoesNotExist) => {
                    // Ignore; not all tiles will exist for extracts
                }
                Err(e) => return Err(e.into()),
            }
        }

        progress_bar.inspect(ProgressBar::finish);
    }

    // An efficient way of tracking whether we've seen an edge before
    // FIXME: Only works on 64-bit (or higher?) platforms
    // TODO: Does this crate actually work for 64-bit values? I also have some doubts about efficiency.
    let mut processed_edges = BitSet::with_capacity(edge_count as usize);

    let progress_bar = PROGRESS_STYLE.get().map(|style| {
        let bar = ProgressBar::new(edge_count as u64);
        bar.set_message(format!("Exporting {edge_count} edges..."));
        bar.set_style(style.clone());
        bar
    });

    for (graph_id, edge_index_offset) in tile_set {
        let tile = reader.get_tile(&graph_id)?;
        for index in 0..tile.header.directed_edge_count() {
            // FIXME: Toy version for testing; will eventually increment as edges are processed
            progress_bar.as_ref().inspect(|bar| bar.inc(1));

            if processed_edges.contains(index as usize) {
                continue;
            }

            // TODO: Mark the edge as seen (maybe? Weird TODO in the Valhalla source)

            // TODO: Get the directed edge from the tile

            // TODO: Optionally skip transit connections, shortcuts, and nameless roads

            // TODO: Get the opposing edge

            // TODO: Traverse forward and backward from the edge

            // TODO: Build the shape from the similar edges found

            // TODO: Output!
        }
    }

    progress_bar.inspect(ProgressBar::finish);

    Ok(())
}
