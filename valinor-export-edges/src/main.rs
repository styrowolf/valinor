use anyhow::anyhow;
use bit_set::BitSet;
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::OnceLock;
use valhalla_graphtile::graph_tile::{DirectedEdge, GraphTile, LookupError};
use valhalla_graphtile::tile_hierarchy::STANDARD_LEVELS;
use valhalla_graphtile::tile_provider::{
    DirectoryTileProvider, GraphTileProvider, GraphTileProviderError,
};
use valhalla_graphtile::{GraphId, RoadUse};

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

    /// Disables progress output.
    #[arg(env, long)]
    no_progress: bool,

    /// Skips transit features.
    ///
    /// I don't think these are even correctly handled anyway.
    #[arg(env, long, default_value = "true")]
    skip_transit: bool,

    /// Skips ferries.
    #[arg(env, long)]
    skip_ferries: bool,

    /// Skips roads with no name.
    #[arg(env, long)]
    skip_unnamed: bool,
}

struct EdgePointer<'a> {
    graph_id: GraphId,
    tile: Rc<GraphTile>,
    edge: &'a DirectedEdge,
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
    let mut edge_count: usize = 0;
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
                    let tile_edge_count = tile.header.directed_edge_count() as usize;
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

    // Drop mutability
    let tile_set = tile_set;

    // An efficient way of tracking whether we've seen an edge before
    // FIXME: Only works on 64-bit (or higher?) platforms
    // TODO: Does this crate actually work for 64-bit values? I also have some doubts about efficiency.
    let mut processed_edges = BitSet::with_capacity(edge_count);

    let progress_bar = PROGRESS_STYLE.get().map(|style| {
        let bar = ProgressBar::new(edge_count as u64);
        bar.set_message(format!("Exporting {edge_count} edges..."));
        bar.set_style(style.clone());
        bar
    });

    for (graph_id, edge_index_offset) in &tile_set {
        let tile = Rc::new(reader.get_tile(&graph_id)?);
        for index in 0..tile.header.directed_edge_count() as usize {
            if processed_edges.contains(edge_index_offset + index) {
                continue;
            }

            // TODO: Some TODO about transition edges in the original source

            // Get the edge
            // TODO: Helper for rewriting the index of a graph ID?
            let edge_id =
                GraphId::try_from_components(graph_id.level(), graph_id.tile_id(), index as u64)?;
            let edge = tile.get_directed_edge(&edge_id)?;

            // TODO: Mark the edge as seen (maybe? Weird TODO in the Valhalla source)
            processed_edges.insert(edge_index_offset + index);

            progress_bar.as_ref().inspect(|bar| bar.inc(1));

            // Skip certain edge types based on the config

            if (cli.skip_transit && edge.is_transit_line())
                || edge.is_shortcut()
                || (cli.skip_ferries && edge.edge_use() == RoadUse::Ferry)
            // TODO
            // || (cli.skip_unnamed && edge.names().is_empty())
            {
                continue;
            }

            // Get the opposing edge

            // FIXME: Perf is really bad when the typical case is reading the same tile over and over...
            let (opp_id, opp_tile) = match tile.clone().get_opp_edge_id(&edge_id) {
                Ok(opp_edge_id) => {
                    // TODO: Verify that it matches the slow path
                    let opp_graph_id = GraphId::try_from_components(
                        edge_id.level(),
                        edge_id.tile_id(),
                        opp_edge_id as u64,
                    )?;
                    // TODO: Make some code like this into a property test
                    // let (slow_id, _) = reader.get_opposing_edge(&edge_id)?.unwrap();
                    // assert_eq!(slow_id, opp_graph_id);
                    (opp_graph_id, tile.clone())
                }
                Err(LookupError::InvalidIndex) => {
                    return Err(LookupError::InvalidIndex)?;
                }
                Err(LookupError::MismatchedBase) => {
                    let (opp_graph_id, tile) = reader.get_opposing_edge(&edge_id)?;
                    (opp_graph_id, Rc::new(tile))
                }
            };
            progress_bar.as_ref().inspect(|bar| bar.inc(1));
            if let Some(offset) = tile_set.get(&opp_id.tile_base_id()) {
                processed_edges.insert(offset + opp_id.tile_index() as usize);
            } else {
                // This happens in extracts, but shouldn't for the planet...
                eprintln!("Missing opposite edge {opp_id} in tile set");
            }

            // TODO: Traverse forward and backward from the edge

            // Keep some state about this section of road
            let mut edges: Vec<EdgePointer> = vec![EdgePointer {
                graph_id: edge_id,
                tile: tile.clone(),
                edge,
            }];

            // TODO: Build the shape from the similar edges found

            // TODO: Output!
        }
    }

    progress_bar.inspect(ProgressBar::finish);

    Ok(())
}
