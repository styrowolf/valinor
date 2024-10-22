use bit_set::BitSet;
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use serde_json::json;
use std::borrow::Cow;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::OnceLock;
use valhalla_graphtile::graph_tile::{DirectedEdge, LookupError};
use valhalla_graphtile::tile_hierarchy::STANDARD_LEVELS;
use valhalla_graphtile::tile_provider::{
    DirectoryTileProvider, GraphTileProvider, GraphTileProviderError,
};
use valhalla_graphtile::{GraphId, RoadUse};
use crate::models::EdgePointer;

static PROGRESS_STYLE: OnceLock<ProgressStyle> = OnceLock::new();

mod helpers;
mod models;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to the Valhalla graph tiles.
    ///
    /// This currently expects a tile folder,
    /// but tarball support will be added eventually.
    #[arg(env)]
    tile_path: PathBuf,

    /// Path to the output directory where files will be created.
    ///
    /// These will be newline-delimited GeoJSON,
    /// and any existing files will be overwritten.
    /// The directory will be created if necessary.
    /// NB: Any existing files will be left intact.
    #[arg(env)]
    output_dir: PathBuf,

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

impl Cli {
    fn should_skip_edge(&self, edge: &DirectedEdge, names: &Vec<Cow<str>>) -> bool {
        // TODO: Actually, visualizing the shortcuts as a separate layer COULD be quite interesting!
        (self.skip_transit && edge.is_transit_line())
            || edge.is_shortcut()
            || (self.skip_ferries && edge.edge_use() == RoadUse::Ferry)
            || (self.skip_unnamed && names.is_empty())
    }
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    // TODO: Make this configurable
    let reader = DirectoryTileProvider::new(cli.tile_path.clone(), NonZeroUsize::new(25).unwrap());

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
            match reader.get_tile_containing(&graph_id) {
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
    // TODO: Should we ever export nodes too in certain cases? Ex: a bollard on an otherwise driveable road?
    let mut processed_edges = BitSet::with_capacity(edge_count);

    let progress_bar = PROGRESS_STYLE.get().map(|style| {
        let bar = ProgressBar::new(edge_count as u64);
        bar.set_message(format!("Exporting {edge_count} edges..."));
        bar.set_style(style.clone());
        bar
    });

    std::fs::create_dir_all(cli.output_dir.clone())?;
    for (tile_id, edge_index_offset) in &tile_set {
        let tile = Rc::new(reader.get_tile_containing(&tile_id)?);
        let path = cli.output_dir.join(tile.graph_id().file_path("geojson")?);
        let parent = path.parent().expect("Unexpected path structure");
        // Create the output directory
        std::fs::create_dir_all(parent)?;

        let mut writer = BufWriter::new(File::create(path)?);
        for index in 0..tile.header.directed_edge_count() as usize {
            if processed_edges.contains(edge_index_offset + index) {
                continue;
            }

            // TODO: Some TODO about transition edges in the original source

            // Get the edge
            // TODO: Helper for rewriting the index of a graph ID?
            let edge_id = tile_id.with_index(index as u64)?;
            let edge = tile.get_directed_edge(&edge_id)?;

            // TODO: Mark the edge as seen (maybe? Weird TODO in the Valhalla source)
            processed_edges.insert(edge_index_offset + index);

            progress_bar.as_ref().inspect(|bar| bar.inc(1));

            // Skip certain edge types based on the config
            let edge_info = tile.get_edge_info(edge)?;
            let names = edge_info.get_names();
            if cli.should_skip_edge(edge, &names) {
                continue;
            }

            // Get the opposing edge

            let opposing_edge = match tile.clone().get_opp_edge_index(&edge_id) {
                Ok(opp_edge_id) => {
                    let opp_graph_id = edge_id.with_index(opp_edge_id as u64)?;
                    EdgePointer {
                        graph_id: opp_graph_id,
                        tile: tile.clone(),
                    }
                }
                Err(LookupError::InvalidIndex) => {
                    return Err(LookupError::InvalidIndex)?;
                }
                Err(LookupError::MismatchedBase) => {
                    let (opp_graph_id, tile) = reader.get_opposing_edge(&edge_id)?;
                    let tile = Rc::new(tile);
                    EdgePointer {
                        graph_id: opp_graph_id,
                        tile,
                    }
                }
            };
            progress_bar.as_ref().inspect(|bar| bar.inc(1));
            if let Some(offset) = tile_set.get(&opposing_edge.graph_id.tile_base_id()) {
                processed_edges.insert(offset + opposing_edge.graph_id.index() as usize);
            } else {
                // This happens in extracts, but shouldn't for the planet...
                eprintln!(
                    "Missing opposite edge {} in tile set",
                    opposing_edge.graph_id
                );
            }

            // Keep some state about this section of road
            // let mut edges: Vec<EdgePointer> = vec![EdgePointer {
            //     graph_id: edge_id,
            //     tile: tile.clone(),
            // }];

            // TODO: Traverse forward and backward from the edge as an optimization to coalesce segments with no change?
            // Could also be useful for MLT representation?

            // TODO: Truncate to 6 digits
            let shape: Vec<_> = edge_info
                .shape()?
                .coords()
                .map(|coord| [coord.x as f32, coord.y as f32])
                .collect();

            // Write it!
            let record = json!({
                "type": "Feature",
                "tippecanoe": {
                    "layer": STANDARD_LEVELS[tile_id.level() as usize].name,
                    "minzoom": STANDARD_LEVELS[tile_id.level() as usize].tiling_system.min_zoom(),
                },
                "geometry": {
                    "type": "LineString",
                    "coordinates": shape,
                },
                "properties": {
                    // NOTE: We can't store an array in MVT
                    "names": names.join(" / "),
                    "classification": edge.classification(),
                    // TODO: Directionality (forward/reverse)
                    // I don't know what forward means
                    "forward": edge.forward(),
                    "forward_access": edge.forward_access().iter().map(|v| v.as_char()).collect::<String>(),
                    "reverse_access": edge.reverse_access().iter().map(|v| v.as_char()).collect::<String>(),
                    // TODO: Bike network
                    // TODO: Estimated speed
                    "speed_limit": edge_info.speed_limit(),
                    "use": edge.edge_use(),
                    // TODO: Cycle lane
                    // TODO: Sidewalk
                    // TODO: Use sidepath
                    // TODO: More TODOs...
                }
            });
            serde_json::to_writer(&mut writer, &record)?;
            writer.write(&['\n' as u8])?;
        }
    }

    progress_bar.inspect(ProgressBar::finish);

    Ok(())
}
