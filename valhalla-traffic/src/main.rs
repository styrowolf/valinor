mod encoding;
mod utils;
mod traffic;

use core::arch;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Read, Seek, Write};
use std::num::NonZero;
use std::path::Path;
use std::ptr::drop_in_place;
use std::{fs::File, path::PathBuf};
use anyhow::anyhow;
use memmap2::{Mmap, MmapMut, MmapOptions};
use tar::{Builder, Header};

use encoding::*;
use traffic::*;
use valhalla_graphtile::GraphId;
use valhalla_graphtile::tile_provider::{DirectoryTileProvider, GraphTileProvider};
use zerocopy::{FromBytes, IntoBytes, TryFromBytes};
use geo::algorithm::line_measures::{Length, Geodesic};

use crate::utils::GraphIdExt;
const HELP: &str = "\
valhalla-traffic

USAGE:
  valhalla-traffic [OPTIONS] [INPUT]

FLAGS:
  -h, --help            Prints help information

OPTIONS:

ARGS:
  <INPUT>
";


fn main() -> anyhow::Result<()> {
    let mut pargs = pico_args::Arguments::from_env();

    // Help has a higher priority and should be handled separately.
    if pargs.contains(["-h", "--help"]) {
        print!("{}", HELP);
        std::process::exit(0);
    }

    let sbc = pargs.subcommand()?;

    if sbc.as_ref().map(|e| e == "decode_id").unwrap_or(false) {
        let id_str: String = pargs.free_from_str()?;
        println!("{}", GraphId::from_hierarchical_string(&id_str)?.value());
        std::process::exit(0);
    }   

    if sbc.as_ref().map(|e| e == "encode_id").unwrap_or(false) {
        let id: u64 = pargs.free_from_str()?;
        let vgraph_id = GraphId::try_from_id(id)?;
        println!("{}", vgraph_id);
        std::process::exit(0);
    }
    
    if sbc.as_ref().map(|e| e == "dct-ii").unwrap_or(false)  {
        let speeds = vec![50_f32; K_BUCKETS_PER_WEEK];
        let comp_res = compress_speed_buckets(&speeds);
        let encoded = encode_compressed_speeds(&comp_res);
        println!("{}", encoded);
        std::process::exit(0);
    } 

    if sbc.as_ref().map(|e| e == "make_traffic_dir").unwrap_or(false) {
        let path: PathBuf = pargs.value_from_str("--csv")?;
        let dir_path: PathBuf = pargs.value_from_str("--dir")?;
        let file = File::open(path)?;
        let mut rdr = csv::Reader::from_reader(file);

        let mut dir_map: HashMap<PathBuf, Vec<csv::StringRecord>> = HashMap::new();

        for record in rdr.records() {
            let record = record?;
            let id = GraphId::from_hierarchical_string(&record[0])?;
            let filepath = id.file_path("csv")?;
            
            dir_map.entry(filepath)
                .and_modify(|e| { e.push(record.clone()) })
                .or_insert(vec![record]);
        }

        for (path, records) in dir_map {
            let path = dir_path.join(path);
            if !path.exists() {
                fs::create_dir_all(&path.parent().unwrap())?;
            }
            println!("Writing to {:?}", path);
            let file = File::create(path)?;
            let mut wtr = csv::Writer::from_writer(file);
            for record in records {
                wtr.write_record(&record)?;
            }
            wtr.flush()?;
        }

        std::process::exit(0);
    }

    if sbc.as_ref().map(|e| e == "make_dummy_live_traffic_tar").unwrap_or(false) {
        let tile_dir: PathBuf = pargs.value_from_str("--tile_dir")?;
        let traffic_extract: PathBuf = pargs.value_from_str("--traffic_extract")?;
        let tile_id_str: String = pargs.value_from_str("--tile_id")?;
        let tile_id = GraphId::from_hierarchical_string(&tile_id_str)?;
        let traffic_update_timestamp: u64 = pargs.value_from_str("--traffic_update_timestamp").unwrap_or_else(|_| 0);
        build_live_traffic_data(&tile_dir, &traffic_extract, tile_id, 0, traffic_update_timestamp)?;
        std::process::exit(0);
    }

    if sbc.as_ref().map(|e| e == "walkdir_test").unwrap_or(false) {
        for entry in walkdir::WalkDir::new("../../prediction-test/custom_files/traffic_data/").into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                println!("{}", entry.path().display());
                let path = entry.path();
                let path = path.strip_prefix("../../prediction-test/custom_files/traffic_data/")?;
                if let Ok(id) = GraphId::from_file_path(path) {
                    println!("Graph ID: {}", id);
                } else {
                    println!("Invalid Graph ID in path: {}", path.display());
                }
            }
        }
        std::process::exit(0);
    }

    if sbc.as_ref().map(|e| e == "make_live_traffic_tar").unwrap_or(false) {
        let csv_path: PathBuf = pargs.value_from_str("--csv")?;
        let tile_dir: PathBuf = pargs.value_from_str("--tile_dir")?;
        let traffic_extract: PathBuf = pargs.value_from_str("--traffic_extract")?;

        let mut rdr = csv::Reader::from_path(csv_path)?;
        let mut tiles_traffic_map: HashMap<GraphId, u32> = HashMap::new();
        for record in rdr.records() {
            let record = record?;
            //let id = GraphId::from_hierarchical_string(&record[0])?;
            let id = GraphId::try_from_id(record[0].parse()?)?;
            let speed: u32 = record[1].parse()?;
            tiles_traffic_map.insert(id, speed);
        }

        build_tiles(
            HashMap::new(),
            &tile_dir,
            &traffic_extract,
            pargs.value_from_str("--traffic_update_timestamp").unwrap_or_else(|_| 0),
        )?;

        std::process::exit(0);
    }

    if sbc.as_ref().map(|e| e == "update_live_traffic_tar").unwrap_or(false) {
        let csv_path: PathBuf = pargs.value_from_str("--csv")?;
        let traffic_extract: PathBuf = pargs.value_from_str("--traffic_extract")?;
        let traffic_update_timestamp: u64 = pargs.value_from_str("--traffic_update_timestamp").unwrap_or_else(|_| 0);

        let mut rdr = csv::Reader::from_path(csv_path)?;
        let mut tiles_traffic_map: HashMap<GraphId, u32> = HashMap::new();
        for record in rdr.records() {
            let record = record?;
            //let id = GraphId::from_hierarchical_string(&record[0])?;
            let id = GraphId::try_from_id(record[0].parse()?)?;
            let speed: u32 = record[1].parse()?;
            tiles_traffic_map.insert(id, speed);
        }

        update_tiles(
            tiles_traffic_map,
            &traffic_extract,
            traffic_update_timestamp,
        )?;

        std::process::exit(0);
    }

    let vgraph_id = GraphId::try_from_id(12488291882).unwrap();
    println!("{}", vgraph_id);

    let speeds = vec![50_f32; K_BUCKETS_PER_WEEK];
    println!("{:?}", speeds);

    let comp_res = compress_speed_buckets(&speeds);
    let encoded = encode_compressed_speeds(&comp_res);
    println!("{}", encoded);

    let decoded = decode_compressed_speeds(&encoded).unwrap();
    println!("{:?}", decoded);
    let speeds_decoded: Vec<_> = (0..K_BUCKETS_PER_WEEK).map(|idx| decompress_speed_bucket(&decoded, idx)).collect();

    println!("{:?}", speeds);
    println!("{:?}", speeds_decoded);

    Ok(())
}

fn walk_tiles(root: &Path) -> impl Iterator<Item = GraphId> {
    walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(move |entry| {
            entry.path().strip_prefix(root)
                .ok()
                .and_then(|path| GraphId::from_file_path(path).ok())
        })
}

// live traffic-related


/// Build live traffic data tar, similar to the C++ implementation.
/// - `config`: HashMap with keys "mjolnir.tile_dir" and "mjolnir.traffic_extract"
/// - `tile_id`: GraphId for the tile
/// - `constant_encoded_speed`: u8 speed value
/// - `traffic_update_timestamp`: u64 timestamp
fn build_live_traffic_data(
    tile_dir: &Path,
    traffic_extract: &Path,
    tile_id: GraphId,
    constant_encoded_speed: u8,
    traffic_update_timestamp: u64,
) -> io::Result<()> {
    // Get paths from config
    let parent_dir = traffic_extract.parent().unwrap().to_path_buf();
    if !parent_dir.exists() {
        panic!("Traffic extract directory {:?} does not exist", parent_dir);
    }

    // Open tar file for writing
    let file = File::create(traffic_extract)?;
    let mut tar_builder = Builder::new(file);

    // Read the tile
    let tile_provider = DirectoryTileProvider::new(tile_dir.to_path_buf(), NonZero::new(32).unwrap());
    let tile = tile_provider.get_tile_containing(&tile_id).expect("Tile not found");

    // Prepare buffer for tile data
    let mut buffer = vec![];
    let header = TrafficTileHeader {
        tile_id: tile.header.graph_id().value(),
        last_update: traffic_update_timestamp,
        traffic_tile_version: TRAFFIC_TILE_VERSION as u32,
        directed_edge_count: tile.header.directed_edge_count(),
        spare2: 0,
        spare3: 0,
    };

    println!("Tile ID: {}", tile.header.graph_id());
    println!("Directed edge count: {}", header.directed_edge_count);
    
    buffer.write_all(&header.as_bytes())?;
    let dummy_speed = TrafficSpeed::with_values(
        constant_encoded_speed,
        constant_encoded_speed,
        constant_encoded_speed,
        constant_encoded_speed,
        255,
        255,
        1,
        2,
        3,
        false,
    );
    for i in 0..header.directed_edge_count {
        buffer.write_all(&dummy_speed.as_bytes())?;
    }
    let dummy_uint32: [u8; 4] = 0u32.to_le_bytes();
    buffer.write_all(&dummy_uint32)?;
    buffer.write_all(&dummy_uint32)?;

    // Write buffer as a file entry in the tar archive
    let filename = tile_id.file_path(".gph").unwrap();
    let mut tar_header = Header::new_gnu();
    tar_header.set_size(buffer.len() as u64);
    tar_builder.append_data(&mut tar_header, filename, &buffer[..])?;
    tar_builder.finish()?;

    Ok(())
}

fn tile_id2edge_id(
    graph_id: GraphId,
    edge_offset: u64,
) -> GraphId {
    GraphId::try_from_components(
        graph_id.level(),
        graph_id.tile_id(),
        edge_offset,
    ).unwrap()
}

fn build_tiles(
    tiles_traffic_map: HashMap<GraphId, u32>,
    tile_dir: &Path,
    traffic_extract: &Path,
    traffic_update_timestamp: u64,
) -> anyhow::Result<()> {
    // Open tar file for writing
    let file = File::create(traffic_extract)?;
    let mut tar_builder = Builder::new(file);

    let tile_provider = DirectoryTileProvider::new(tile_dir.to_path_buf(), NonZero::new(32).unwrap());

    for tile_id in walk_tiles(&tile_dir) {
        let tile = tile_provider.get_tile_containing(&tile_id)
            .expect("Tile not found");

        let mut buffer = vec![];
        let header = TrafficTileHeader {
            tile_id: tile.header.graph_id().value(),
            last_update: traffic_update_timestamp,
            traffic_tile_version: TRAFFIC_TILE_VERSION as u32,
            directed_edge_count: tile.header.directed_edge_count(),
            spare2: 0,
            spare3: 0,
        };

        buffer.write_all(header.as_bytes())?;

        let unset_speed = TrafficSpeed::with_values(
            0,
            0,
            0,
            0,
            255,
            255,
            0,
            0,
            0,
            false,
        );

        for i in 0..header.directed_edge_count {
            let edge_id = tile_id2edge_id(tile.header.graph_id(), i as u64);
            if let Some(speed) = tiles_traffic_map.get(&edge_id) {
                let speed_value = *speed as u8;
                let traffic_speed = TrafficSpeed::with_values(
                    speed_value,
                    speed_value,
                    speed_value,
                    speed_value,
                    255,
                    255,
                    0,
                    0,
                    0,
                    false,
                );
                buffer.write_all(&traffic_speed.as_bytes())?;
            } else {
                // If no speed is set, use the unset speed
                buffer.write_all(&unset_speed.as_bytes())?;
            }
        }
        
        // placeholders for some reason (idk why)
        let dummy_uint32: [u8; 4] = 0u32.to_le_bytes();
        buffer.write_all(&dummy_uint32)?;
        buffer.write_all(&dummy_uint32)?;
        
        let filename = tile_id.file_path("gph").unwrap();
        let mut tar_header = Header::new_gnu();
        tar_header.set_size(buffer.len() as u64);
        tar_builder.append_data(&mut tar_header, filename, &buffer[..])?;
    }

    tar_builder.finish()?;

    Ok(())
} 

fn update_tiles(
    tiles_traffic_map: HashMap<GraphId, u32>,
    traffic_extract: &Path,
    traffic_update_timestamp: u64,
) -> anyhow::Result<()> {
    // Open tar file for writing
    let file = std::fs::OpenOptions::new()
        .write(true)
        .read(true)
        .append(true)
        .open(traffic_extract)?;

    let mut archive = tar::Archive::new(&file);

    let mut tile_id_map = HashMap::new();

    for entry in archive.entries_with_seek()? {
        let entry = entry?;
        let graph_id = GraphId::from_file_path(entry.path()?)
            .map_err(|e| anyhow!("Invalid GraphId in entry {:?}: {}", entry.path(), e))?;
        let offset = entry.raw_file_position();
        let entry_size = entry.header().entry_size()?;
        
        tile_id_map.insert(graph_id, (offset as usize, entry_size as usize));
    }

    drop(archive);

    let unset_speed = TrafficSpeed::with_values(
        0,
        0,
        0,
        0,
        255,
        255,
        0,
        0,
        0,
        false,
    );

    let mut mmap = unsafe { MmapMut::map_mut(&file)?  };

    for (tile, (offset, _entry_size)) in tile_id_map {
        let buffer = mmap.as_mut_bytes();
        let header = TrafficTileHeader::try_mut_from_bytes(&mut buffer[offset..offset + size_of::<TrafficTileHeader>()])
            .map_err(|e| anyhow!("Failed to parse TrafficTileHeader: {}", e))?;
        header.last_update = traffic_update_timestamp;
        let directed_edge_count = header.directed_edge_count as usize;

        for i in 0..directed_edge_count {
            let edge_id = tile_id2edge_id(tile, i as u64);
            if let Some(speed) = tiles_traffic_map.get(&edge_id) {
                let speed_value = *speed as u8;
                let traffic_speed = TrafficSpeed::with_values(
                    speed_value,
                    speed_value,
                    speed_value,
                    speed_value,
                    255,
                    255,
                    0,
                    0,
                    0,
                    false,
                );
                buffer[offset + size_of::<TrafficTileHeader>() + i * size_of::<TrafficSpeed>()..offset + size_of::<TrafficTileHeader>() + (i + 1) * size_of::<TrafficSpeed>()]
                    .copy_from_slice(&traffic_speed.as_bytes());
            } else {
                // If no speed is set, use the unset speed
                buffer[offset + size_of::<TrafficTileHeader>() + i * size_of::<TrafficSpeed>()..offset + size_of::<TrafficTileHeader>() + (i + 1) * size_of::<TrafficSpeed>()]
                    .copy_from_slice(&unset_speed.as_bytes());
            }
        }

    }

    mmap.flush()?;

    Ok(())
}

/* 
pub fn recover_shortcut(
    provider: &DirectoryTileProvider, // your graph structure
    shortcut_edge_id: GraphId // your edge identifier type
) -> Option<Vec<GraphId>> {
    let tile = provider.get_tile_containing(&shortcut_edge_id)
        .expect("Tile not found for shortcut edge ID");
    let shortcut = tile.get_directed_edge(&shortcut_edge_id).expect("Unable to get shortcut edge");

    if !shortcut.is_shortcut() {
        return None;
    }

    let shortcut_info = tile.get_edge_info(shortcut).expect("Unable to get edge info for shortcut");

    let mut edges = Vec::new();
    let mut current_node = edge_start_node(shortcut_edge_id, provider);
    let end_node = shortcut.end_node_id();
    let mut accumulated_length = 0;
    let shortcut_length = shortcut_info.shape().unwrap().length::<Geodesic>();

    while current_node != end_node {
        let mut found = false;
        // Find the next outgoing edge that matches shortcut attributes and is not itself a shortcut
        for edge in graph.out_edges(current_node) {
            if is_superseded_by_shortcut(edge, shortcut) {
                edges.push(edge.id());
                accumulated_length += edge.length();
                current_node = edge.end_node();
                found = true;
                break;
            }
        }
        if !found {
            // Could not recover, return just shortcut as fallback
            return vec![shortcut_edge_id];
        }
        // Optionally check for overrun/underrun on accumulated_length (roundoff)
    }

    edges
}

fn edge_start_node(edge_id: GraphId, provider: &DirectoryTileProvider) -> GraphId {
    let tile = provider.get_tile_containing(&edge_id)
        .expect("Tile not found for edge ID");
    let edge = tile.get_directed_edge(&edge_id)
        .expect("Unable to get directed edge");
    let opp_edge_index = edge.opposing_edge_index();

    let opposing_edge_id = GraphId::try_from_components(
        edge_id.level(),
        edge_id.tile_id(),
        opp_edge_index as u64,
    ).expect("Failed to create opposing edge ID");

    let opposing_edge = tile.get_directed_edge(&opposing_edge_id)
        .expect("Unable to get opposing edge");

    return opposing_edge.end_node_id();
}

// Example attribute check (customize as needed for your data model):
fn is_superseded_by_shortcut(edge: &Edge, shortcut: &Edge) -> bool {
    // Compare access, road class, etc.
    // Avoid edge.is_shortcut()
    // ... add checks as needed ...
    true
}

/*
// Pseudocode / simplified
GraphId GraphReader::GetOpposingEdgeId(const GraphId& edgeid, graph_tile_ptr& tile) {
  const DirectedEdge* edge = tile->directededge(edgeid);
  GraphId endnode = edge->endnode();
  // Load the tile for the endnode if needed
  graph_tile_ptr end_tile = GetGraphTile(endnode);
  // Get the node at the end of the edge
  const NodeInfo* node = end_tile->node(endnode);
  // The opposing edge index
  uint32_t opp_index = edge->opp_index();
  // The GraphId for the opposing edge
  GraphId opp_edgeid = GraphId(endnode.tileid(), endnode.level(), node->edge_index() + opp_index);
  return opp_edgeid;
}
 */
*/