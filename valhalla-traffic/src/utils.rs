use std::path::Path;

use valhalla_graphtile::GraphId;
use anyhow::{anyhow};

pub trait GraphIdExt {
    /// Converts a GraphId to a hierarchical string format.
    fn to_hierarchical_string(&self) -> String;
    fn from_hierarchical_string(id_str: &str) -> anyhow::Result<GraphId>;
    fn from_file_path<P: AsRef<std::path::Path>>(path: P) -> anyhow::Result<GraphId>;
}

impl GraphIdExt for GraphId {
    fn to_hierarchical_string(&self) -> String {
        graph_id2hierarchical(*self)
    }

    fn from_hierarchical_string(id_str: &str) -> anyhow::Result<GraphId> {
        graph_id_from_hierarchical_str(id_str)
    }
    
    fn from_file_path<P: AsRef<std::path::Path>>(path: P) -> anyhow::Result<GraphId> {
        graph_id_from_file_path(path)
    }
}

fn graph_id2hierarchical(id: GraphId) -> String {
    format!("{}/{}/{}", id.level(), id.tile_id(), id.index())
}

fn graph_id_from_hierarchical_str(id_str: &str) -> anyhow::Result<GraphId> {
    let hierarchical_id = id_str.split('/').collect::<Vec<_>>();
    if hierarchical_id.len() != 3 {
        return Err(anyhow!("Invalid hierarchical ID format: {}", id_str));
    }
    Ok(GraphId::try_from_components(
        u8::from_str_radix(hierarchical_id[0], 10)?,
        u64::from_str_radix(hierarchical_id[1], 10)?,
        u64::from_str_radix(hierarchical_id[2], 10)?,
    )?)
}

/// Decodes a GraphId from a relative tile path (as produced by `file_path`).
/// Returns an error if the path is invalid or does not match the expected format.
pub fn graph_id_from_file_path<P: AsRef<Path>>(path: P) -> anyhow::Result<GraphId> {
    let path = path.as_ref();
    let mut components = path.components();

    // Get the level as the first component
    let level_str = components.next()
        .ok_or(anyhow!("InvalidGraphId"))?
        .as_os_str().to_str().ok_or(anyhow!("InvalidGraphId"))?;
    let level: u8 = level_str.parse().map_err(|_| anyhow!("InvalidGraphId"))?;

    // Reconstruct the tile id string from all components except the level
    let mut tile_id_parts: Vec<&str> = path.iter().skip(1).map(|os| os.to_str().unwrap()).collect();
    if let Some(last_idx) = tile_id_parts.last_mut() {
        // Remove extension from last part
        if let Some(dot_idx) = last_idx.find('.') {
            *last_idx = &last_idx[..dot_idx];
        }
    }
    let tile_id_str = tile_id_parts.concat();
    let tile_id: u64 = tile_id_str.parse().map_err(|_| anyhow!("InvalidGraphId"))?;

    // Index is always 0 for tile paths
    Ok(GraphId::try_from_components(level, tile_id, 0)?)
}